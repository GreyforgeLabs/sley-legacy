#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKDIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR"' EXIT

python3 - "$ROOT_DIR" "$WORKDIR/transcript.jsonl" <<'PY'
from __future__ import annotations

import copy
import hashlib
import json
import os
import select
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

from jsonschema import Draft202012Validator, RefResolver


root = Path(sys.argv[1])
transcript_path = Path(sys.argv[2])
zero = "sha256:" + ("0" * 64)
runtime_digest = "sha256:5b36de7470d015d2090552587d4f163dadbd062d87b7508320b1f1b638ad2361"
base = json.loads((root / "fixtures/contracts/worker_request_invoke.json").read_text())
transcript: list[dict] = []

env = os.environ.copy()
env["SLEY_ALLOW_TEST_HOOKS"] = "1"
env["SLEY_WORKER_TEST_CRASH_NONCE"] = "nonce:crash"
env["SLEY_WORKER_TEST_OUTPUT_NONCE"] = "nonce:output-flood"
env["SLEY_WORKER_TEST_SNAPSHOT_NONCE"] = "nonce:snapshot-race"
process = subprocess.Popen(
    [str(root / "bin/sley"), "worker", "start", "--json", "--max-requests", "8", "--idle-timeout-ms", "120000"],
    cwd=root,
    env=env,
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
    bufsize=1,
)
assert process.stdin is not None and process.stdout is not None and process.stderr is not None


def receive(timeout: float = 45.0) -> dict:
    ready, _, _ = select.select([process.stdout], [], [], timeout)
    if not ready:
        raise AssertionError("timed out waiting for worker output")
    line = process.stdout.readline()
    if line == "":
        stderr = process.stderr.read()
        raise AssertionError(f"worker output closed unexpectedly: {stderr}")
    value = json.loads(line)
    transcript.append(value)
    return value


def receive_until(predicate, timeout: float = 120.0) -> dict:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        value = receive(max(0.1, deadline - time.monotonic()))
        if predicate(value):
            return value
    raise AssertionError("worker output predicate was not satisfied")


def send(value: dict) -> None:
    process.stdin.write(json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n")
    process.stdin.flush()


def control(operation: str, index: str, worker_digest: str, payload: dict | None = None) -> dict:
    value = copy.deepcopy(base)
    value["request_id"] = f"request:{index}"
    value["nonce"] = f"nonce:{index}"
    value["operation"] = operation
    value["bindings"].update({
        "worker_digest": worker_digest,
        "runtime_digest": runtime_digest,
        "package_digest": zero,
        "source_digest": zero,
        "entry_point": f"worker.{operation}",
        "authority": {"mode": "absent", "grant_digest": None, "idempotency_key": None},
    })
    value["payload"] = payload or {}
    return value


def adapter(operation: str, index: str, worker_digest: str) -> dict:
    value = copy.deepcopy(base)
    value["request_id"] = f"request:{index}"
    value["nonce"] = f"nonce:{index}"
    value["operation"] = operation
    value["bindings"]["worker_digest"] = worker_digest
    value["bindings"]["runtime_digest"] = runtime_digest
    value["bindings"]["entry_point"] = "adapter.replay"
    value["bindings"]["authority"] = {
        "mode": "local_replay_only",
        "grant_digest": None,
        "idempotency_key": f"idempotency:{index}" if operation == "invoke" else None,
    }
    return value


def response(request_id: str, operation: str | None = None) -> dict:
    return receive_until(lambda item: item.get("schema") == "sley.worker.response.v1"
                         and item.get("request_id") == request_id
                         and (operation is None or item.get("operation") == operation))


ready = receive_until(lambda item: item.get("code") == "WORKER_READY")
assert ready["worker_state"] == "ready"

handshake = control("handshake", "handshake", zero)
handshake["bindings"]["runtime_digest"] = zero
send(handshake)
handshake_response = response("request:handshake")
assert handshake_response["status"] == "passed"
worker_digest = handshake_response["worker"]["worker_digest"]
assert handshake_response["result"]["operations"] == [
    "handshake", "load", "capabilities", "invoke", "cancel",
    "reset", "drain", "health", "shutdown",
]

send(adapter("load", "load", worker_digest))
load_response = response("request:load")
assert load_response["status"] == "passed"
assert load_response["result"]["stored_request_state"] is False
assert "seeds" not in load_response["result"]

send(adapter("capabilities", "capabilities", worker_digest))
capability_response = response("request:capabilities")
assert capability_response["status"] == "passed"
assert capability_response["result"]["authority"] == {"mode": "local_replay_only", "external_provider_calls": False}

send(adapter("invoke", "happy", worker_digest))
happy_response = response("request:happy", "invoke")
assert happy_response["status"] == "passed", happy_response
assert happy_response["result"]["adapter_report"]["status"] == "passed"
assert happy_response["result"]["request_local_state_retained"] is False
assert happy_response["worker"]["active_request_id"] is None

idempotency_replay = adapter("invoke", "idempotency-replay", worker_digest)
idempotency_replay["bindings"]["authority"]["idempotency_key"] = "idempotency:happy"
send(idempotency_replay)
idempotency_response = response("request:idempotency-replay")
assert idempotency_response["failure_class"] == "authority_denied"
assert idempotency_response["issues"][0]["code"] == "WORKER_IDEMPOTENCY_REPLAY_DENIED"

output_flood = adapter("invoke", "output-flood", worker_digest)
send(output_flood)
output_response = response("request:output-flood", "invoke")
assert output_response["failure_class"] == "budget_exhausted"
assert output_response["issues"][0]["code"] == "WORKER_BUDGET_EXHAUSTED"

race_dir = Path(tempfile.mkdtemp(prefix=".sley-worker-test-", dir=root))
try:
    race_target = race_dir / "target.sley"
    shutil.copyfile(root / "examples/agent_deploy_pipeline.sley", race_target)
    race_manifest = copy.deepcopy(json.loads((root / "fixtures/contracts/adapter_manifest_local_replay.json").read_text()))
    race_manifest["target"] = "target.sley"
    race_manifest["manifest_digest"] = zero
    race_manifest["manifest_digest"] = "sha256:" + hashlib.sha256(
        json.dumps(race_manifest, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    ).hexdigest()
    race_manifest_path = race_dir / "manifest.json"
    race_manifest_path.write_text(json.dumps(race_manifest, indent=2) + "\n")
    snapshot_race = adapter("invoke", "snapshot-race", worker_digest)
    snapshot_race["payload"]["manifest"] = str(race_manifest_path.relative_to(root))
    snapshot_race["bindings"]["package_digest"] = race_manifest["manifest_digest"]
    snapshot_race["bindings"]["source_digest"] = "sha256:" + hashlib.sha256(race_target.read_bytes()).hexdigest()
    send(snapshot_race)
    receive_until(lambda item: item.get("request_id") == "request:snapshot-race" and item.get("code") == "WORKER_SNAPSHOT_PENDING")
    race_target.write_text("module changed\n")
    snapshot_response = response("request:snapshot-race", "invoke")
    assert snapshot_response["failure_class"] == "source_mismatch"
finally:
    shutil.rmtree(race_dir)

source_mismatch = adapter("invoke", "source-mismatch", worker_digest)
source_mismatch["bindings"]["source_digest"] = zero
send(source_mismatch)
source_response = response("request:source-mismatch")
assert source_response["failure_class"] == "source_mismatch"

authority_denied = adapter("invoke", "authority-denied", worker_digest)
authority_denied["bindings"]["authority"] = {"mode": "absent", "grant_digest": None, "idempotency_key": None}
send(authority_denied)
authority_response = response("request:authority-denied")
assert authority_response["failure_class"] == "authority_denied"

budget = adapter("invoke", "budget", worker_digest)
budget["budgets"]["max_steps"] = 1
send(budget)
budget_response = response("request:budget")
assert budget_response["failure_class"] == "budget_exhausted"

protocol = control("health", "protocol", worker_digest)
protocol["bindings"]["runtime_digest"] = zero
send(protocol)
protocol_response = response("request:protocol")
assert protocol_response["failure_class"] == "protocol_mismatch"

cancelled = adapter("invoke", "cancelled", worker_digest)
send(cancelled)
receive_until(lambda item: item.get("request_id") == "request:cancelled" and item.get("code") == "WORKER_PROCESS_STARTED")
send(control("cancel", "cancel-command", worker_digest, {"target_request_id": "request:cancelled"}))
cancel_command_response = None
cancelled_response = None
deadline = time.monotonic() + 30
while time.monotonic() < deadline and (cancel_command_response is None or cancelled_response is None):
    item = receive(max(0.1, deadline - time.monotonic()))
    if item.get("schema") != "sley.worker.response.v1":
        continue
    if item.get("request_id") == "request:cancel-command":
        cancel_command_response = item
    elif item.get("request_id") == "request:cancelled":
        cancelled_response = item
assert cancel_command_response is not None and cancel_command_response["status"] == "passed"
assert cancelled_response is not None and cancelled_response["failure_class"] == "cancelled"
assert cancelled_response["retryable"] is False

crash = adapter("invoke", "crash", worker_digest)
send(crash)
crash_response = response("request:crash", "invoke")
assert crash_response["failure_class"] == "worker_crashed"
assert crash_response["retryable"] is True
assert crash_response["worker"]["state"] == "ready"

send(control("health", "health-after-crash", worker_digest))
health_response = response("request:health-after-crash")
assert health_response["status"] == "passed"
assert health_response["result"] == {"healthy": True, "state": "ready"}
assert health_response["worker"]["request_count"] == 5

send(control("drain", "drain", worker_digest))
drain_response = response("request:drain")
assert drain_response["status"] == "passed"
assert drain_response["worker"]["state"] == "draining"

send(adapter("invoke", "drain-denied", worker_digest))
drain_denied = response("request:drain-denied")
assert drain_denied["failure_class"] == "worker_unhealthy"

send(control("reset", "reset", worker_digest))
reset_response = response("request:reset")
assert reset_response["status"] == "passed"
assert reset_response["result"]["cancellation_state"] == "fresh"

send(control("shutdown", "shutdown", worker_digest))
shutdown_response = response("request:shutdown")
assert shutdown_response["status"] == "passed"
receive_until(lambda item: item.get("code") == "WORKER_STOPPED")
process.stdin.close()
exit_code = process.wait(timeout=15)
stderr = process.stderr.read()
assert exit_code == 0, stderr

schema_values = {}
for path in (root / "docs/schemas").glob("*.json"):
    value = json.loads(path.read_text())
    if isinstance(value, dict) and isinstance(value.get("$id"), str):
        schema_values[value["$id"]] = value
for item in transcript:
    schema_id = item["schema"]
    if schema_id not in ("sley.worker.response.v1", "sley.worker.event.v1"):
        raise AssertionError(f"unexpected worker stream schema: {schema_id}")
    resolver = RefResolver.from_schema(schema_values[schema_id], store=schema_values)
    errors = list(Draft202012Validator(schema_values[schema_id], resolver=resolver).iter_errors(item))
    if errors:
        raise AssertionError(f"worker stream schema failure: {errors[0].message}")

encoded = "\n".join(json.dumps(item, sort_keys=True) for item in transcript) + "\n"
for private_value in ("plan approved", "profile ready", "staged"):
    assert private_value not in encoded
transcript_path.write_text(encoded)
PY

grep -Fq '"failure_class": "cancelled"' "$WORKDIR/transcript.jsonl" \
  || { echo "worker transcript did not retain cancellation evidence" >&2; exit 1; }
grep -Fq '"failure_class": "worker_crashed"' "$WORKDIR/transcript.jsonl" \
  || { echo "worker transcript did not retain crash evidence" >&2; exit 1; }
grep -Fq '"namespace_isolation_enforced": false' "$WORKDIR/transcript.jsonl" \
  || { echo "worker transcript overstated namespace isolation" >&2; exit 1; }

echo "persistent worker protocol, isolation, cancellation, crash, and reset contracts passed"
