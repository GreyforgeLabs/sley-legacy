"""Clean-environment lifecycle proof for the Python worker client."""

from __future__ import annotations

import json
import time
import sys
import typing
from pathlib import Path

from sley_worker_client import (
    CONTRACT_VERSIONS,
    OPERATIONS,
    PROTOCOL,
    ZERO_DIGEST,
    WorkerClient,
    WorkerClientCancelledError,
    WorkerClientClosedError,
    WorkerClientProtocolError,
    WorkerClientTimeoutError,
    WorkerRequest,
    WorkerSession,
)


root = Path(sys.argv[1]).resolve()
fixture = json.loads((root / "fixtures/contracts/worker_request_invoke.json").read_text(encoding="utf-8"))
budgets = fixture["budgets"]
adapter = {
    "manifest": fixture["payload"]["manifest"],
    "record": fixture["payload"]["record"],
    "package_digest": fixture["bindings"]["package_digest"],
    "source_digest": fixture["bindings"]["source_digest"],
}

assert typing.get_type_hints(WorkerRequest)["protocol"] is not None
assert typing.get_type_hints(WorkerSession)["isolation"] is not None

client = WorkerClient(
    command=(str(root / "bin/sley"),),
    cwd=root,
    env={
        "SLEY_ALLOW_TEST_HOOKS": "1",
        "SLEY_WORKER_TEST_SNAPSHOT_NONCE": "nonce:python-cancel",
        "SLEY_WORKER_TEST_CRASH_NONCE": "nonce:python-crash",
    },
    max_requests=8,
    idle_timeout_ms=120_000,
)

try:
    ready = client.start()
    assert ready["protocol"] == PROTOCOL and ready["code"] == "WORKER_READY"

    handshake = client.handshake(budgets)
    assert handshake["status"] == "passed"
    assert client.worker_digest == handshake["worker"]["worker_digest"]

    loaded = client.load(budgets, **adapter)
    assert loaded["status"] == "passed"
    assert loaded["result"]["stored_request_state"] is False

    capabilities = client.capabilities(budgets, **adapter)
    assert capabilities["status"] == "passed"
    assert capabilities["result"]["authority"] == {"mode": "local_replay_only", "external_provider_calls": False}

    passed = client.invoke(budgets, idempotency_key="idempotency:python-happy", **adapter)
    assert passed["status"] == "passed"
    assert passed["result"]["request_local_state_retained"] is False

    mismatch_request = client.build_adapter_request(
        "invoke",
        budgets,
        idempotency_key="idempotency:python-mismatch",
        source_digest=ZERO_DIGEST,
        **{key: value for key, value in adapter.items() if key != "source_digest"},
    )
    mismatch = client.send(mismatch_request)
    assert mismatch["failure_class"] == "source_mismatch"
    assert mismatch["retryable"] is False

    cancel_request = client.build_adapter_request(
        "invoke",
        budgets,
        request_id="request:python-cancel",
        nonce="nonce:python-cancel",
        idempotency_key="idempotency:python-cancel",
        **adapter,
    )
    pending_cancel = client.send_async(cancel_request)
    client.wait_for_event(code="WORKER_PROCESS_STARTED", request_id=cancel_request["request_id"], timeout=15)
    cancel_response = client.cancel(cancel_request["request_id"], budgets)
    cancelled = pending_cancel.result(timeout=30)
    assert cancel_response["status"] == "passed"
    assert cancelled["failure_class"] == "cancelled"
    assert cancelled["retryable"] is False

    crash_request = client.build_adapter_request(
        "invoke",
        budgets,
        request_id="request:python-crash",
        nonce="nonce:python-crash",
        idempotency_key="idempotency:python-crash",
        **adapter,
    )
    crashed = client.send(crash_request)
    assert crashed["failure_class"] == "worker_crashed"
    assert crashed["retryable"] is True

    health = client.health(budgets)
    assert health["status"] == "passed"
    assert health["worker"]["request_count"] == 3
    assert health["result"] == {"healthy": True, "state": "ready"}

    shutdown = client.shutdown(budgets)
    assert shutdown["status"] == "passed"
finally:
    client.close()

assert any(message.get("failure_class") == "worker_crashed" for message in client.messages)
assert any(message.get("failure_class") == "cancelled" for message in client.messages)
assert "plan approved" not in json.dumps(client.messages, sort_keys=True)
print("python worker client clean-environment lifecycle passed")

fake_script = r'''
import json
import time
ready = {
    "schema": "sley.worker.event.v1",
    "protocol": "sley.worker.v1",
    "sequence": 1,
    "request_id": None,
    "operation": "session",
    "phase": "ready",
    "worker_state": "ready",
    "code": "WORKER_READY",
    "details": {},
}
unknown = {
    "schema": "sley.worker.response.v1",
    "protocol": "sley.worker.v1",
    "request_id": "request:unsolicited",
}
print(json.dumps(ready), flush=True)
print(json.dumps(unknown), flush=True)
time.sleep(30)
'''
failed_client = WorkerClient(command=(sys.executable, "-c", fake_script))
try:
    failed_client.start()
    time.sleep(0.1)
    request = failed_client.build_control_request("handshake", budgets)
    try:
        failed_client.send_async(request)
    except Exception as exc:
        assert type(exc).__name__ == "WorkerClientClosedError"
    else:
        raise AssertionError("send after a reader failure did not reject")
finally:
    failed_client.close()
print("python worker client post-reader-error rejection passed")

fake_capabilities = {
    "protocol": PROTOCOL,
    "operations": list(OPERATIONS),
    "contract_versions": list(CONTRACT_VERSIONS),
    "failure_classes": [
        "adapter_failed", "authority_denied", "budget_exhausted", "cancelled",
        "internal_error", "invalid_result", "protocol_mismatch", "source_mismatch",
        "timeout", "worker_crashed", "worker_unhealthy",
    ],
    "request_states": ["accepted", "running", "completed", "failed", "cancelled", "timed_out"],
}
fake_worker_script = r'''
import json
import os
import sys
import threading
import time

mode = os.environ["SLEY_FAKE_MODE"]
protocol = "sley.worker.v1"
capabilities = json.loads(os.environ["SLEY_FAKE_CAPABILITIES"])
isolation = {
    "class": "fresh_subprocess_per_invoke",
    "fresh_process_per_invoke": True,
    "request_local_tempdir": True,
    "clean_environment": True,
    "closed_inherited_fds": True,
    "resource_limits": True,
    "process_group_cancellation": True,
    "namespace_isolation_enforced": False,
}
session = {
    "schema": "sley.worker.session.v1",
    "protocol": protocol,
    "worker_id": "worker:fake",
    "worker_digest": "sha256:" + ("1" * 64),
    "runtime_digest": "sha256:" + ("2" * 64),
    "state": "ready",
    "generation": 1,
    "request_count": 0,
    "active_request_id": None,
    "max_requests": 128,
    "cache": {"policy": "disabled", "shared_mutable_state": False, "hit_count": 0},
    "isolation": isolation,
}
ready = {
    "schema": "sley.worker.event.v1", "protocol": protocol, "sequence": 1,
    "request_id": None, "operation": "session", "phase": "ready",
    "worker_state": "ready", "code": "WORKER_READY", "details": {},
}

def emit(value):
    print(json.dumps(value, sort_keys=True, separators=(",", ":")), flush=True)

def response(request):
    value = {
        "schema": "sley.worker.response.v1", "protocol": protocol,
        "request_id": request["request_id"], "operation": request["operation"],
        "status": "passed", "failure_class": None, "retryable": False,
        "worker": session, "result": capabilities if request["operation"] == "handshake" else {},
        "isolation": isolation, "issues": [],
    }
    if mode == "bad_handshake":
        value["isolation"] = {**isolation, "process_group_cancellation": False}
    return value

emit(ready)
if mode == "malformed":
    threading.Timer(0.01, lambda: (sys.stdout.write("{not-json\n"), sys.stdout.flush())).start()
if mode == "oversized":
    threading.Timer(0.01, lambda: (sys.stdout.write(("x" * 4096) + "\n"), sys.stdout.flush())).start()
if mode == "exit":
    threading.Timer(0.02, lambda: os._exit(17)).start()
if mode == "exit_pending":
    threading.Timer(0.06, lambda: os._exit(18)).start()

for line in sys.stdin:
    request = json.loads(line)
    if mode in ("never", "exit_pending"):
        continue
    if mode == "late":
        threading.Timer(0.08, lambda request=request: emit(response(request))).start()
        continue
    emit(response(request))
    if mode == "duplicate":
        emit(response(request))
'''


def fake_client(mode: str, **options) -> WorkerClient:
    options.setdefault("request_timeout", 1)
    return WorkerClient(
        command=(sys.executable, "-c", fake_worker_script),
        env={
            "SLEY_FAKE_MODE": mode,
            "SLEY_FAKE_CAPABILITIES": json.dumps(fake_capabilities, sort_keys=True, separators=(",", ":")),
        },
        **options,
    )


timeout_client = fake_client("late", request_timeout=0.02)
try:
    timeout_client.start()
    first = timeout_client.build_control_request(
        "handshake", budgets, request_id="request:late", nonce="nonce:late"
    )
    try:
        timeout_client.send(first)
    except WorkerClientTimeoutError:
        pass
    else:
        raise AssertionError("never-ending Python request did not time out")
    assert timeout_client.pending_request_count == 0
    time.sleep(0.12)
    assert timeout_client.late_response_count == 1
    second = timeout_client.build_control_request(
        "handshake", budgets, request_id="request:after-late", nonce="nonce:after-late"
    )
    assert timeout_client.send(second, timeout=0.2)["status"] == "passed"
finally:
    timeout_client.close()

cancel_client = fake_client("never")
try:
    cancel_client.start()
    pending = cancel_client.send_async(cancel_client.build_control_request("handshake", budgets))
    assert pending.cancel() is True
    try:
        pending.result()
    except WorkerClientCancelledError:
        pass
    else:
        raise AssertionError("cancelled Python wait returned a response")
    assert cancel_client.pending_request_count == 0
finally:
    cancel_client.close()

bounded_client = fake_client("never", request_timeout=0.005, max_tombstones=32)
try:
    bounded_client.start()
    waits = [
        bounded_client.send_async(
            bounded_client.build_control_request(
                "handshake", budgets,
                request_id=f"request:bounded-{index}", nonce=f"nonce:bounded-{index}",
            )
        )
        for index in range(300)
    ]
    for pending in waits:
        try:
            pending.result()
        except WorkerClientTimeoutError:
            pass
        else:
            raise AssertionError("bounded Python request did not time out")
    assert bounded_client.pending_request_count == 0
    assert bounded_client.tombstone_count == 32
finally:
    bounded_client.close()

for mode in ("malformed", "oversized", "exit"):
    options = {"max_message_bytes": 2048} if mode == "oversized" else {}
    corrupt_client = fake_client(mode, **options)
    try:
        corrupt_client.start()
        time.sleep(0.1)
        try:
            corrupt_client.send_async(corrupt_client.build_control_request("handshake", budgets))
        except WorkerClientClosedError:
            pass
        else:
            raise AssertionError(f"{mode} Python worker remained usable")
    finally:
        corrupt_client.close()

duplicate_client = fake_client("duplicate")
try:
    duplicate_client.start()
    assert duplicate_client.send(duplicate_client.build_control_request("handshake", budgets))["status"] == "passed"
    time.sleep(0.1)
    try:
        duplicate_client.send_async(duplicate_client.build_control_request("handshake", budgets))
    except WorkerClientClosedError:
        pass
    else:
        raise AssertionError("duplicate Python response was not fatal")
finally:
    duplicate_client.close()

invalid_handshake_client = fake_client("bad_handshake")
try:
    invalid_handshake_client.start()
    try:
        invalid_handshake_client.handshake(budgets)
    except WorkerClientClosedError:
        pass
    else:
        raise AssertionError("invalid Python handshake capabilities were accepted")
finally:
    invalid_handshake_client.close()

exit_client = fake_client("exit_pending")
try:
    exit_client.start()
    waits = [
        exit_client.send_async(
            exit_client.build_control_request(
                "handshake", budgets, request_id=f"request:exit-{index}", nonce=f"nonce:exit-{index}"
            )
        )
        for index in range(2)
    ]
    for pending in waits:
        try:
            pending.result()
        except WorkerClientClosedError:
            pass
        else:
            raise AssertionError("child exit did not reject all Python requests")
    assert exit_client.pending_request_count == 0
finally:
    exit_client.close()

outbound_client = fake_client("never", max_message_bytes=2048)
try:
    outbound_client.start()
    oversized_request = outbound_client.build_control_request("handshake", budgets)
    oversized_request["payload"] = {"oversized": "x" * 4096}
    try:
        outbound_client.send_async(oversized_request)
    except WorkerClientProtocolError:
        pass
    else:
        raise AssertionError("oversized Python request was accepted")
finally:
    outbound_client.close()

print("python worker client adversarial lifecycle matrix passed")
