"""Clean-environment lifecycle proof for the Python worker client."""

from __future__ import annotations

import json
import time
import sys
import typing
from pathlib import Path

from sley_worker_client import PROTOCOL, ZERO_DIGEST, WorkerClient, WorkerRequest, WorkerSession


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
