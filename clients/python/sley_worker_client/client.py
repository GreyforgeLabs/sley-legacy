"""Synchronous stdio client for the canonical Sley worker protocol."""

from __future__ import annotations

import json
import os
import re
import secrets
import signal
import subprocess
import threading
import time
from collections import OrderedDict
from collections.abc import Callable, Mapping, Sequence
from pathlib import Path
from typing import Any, cast

from .generated.models import (
    CONTRACT_VERSIONS,
    OPERATIONS,
    PROTOCOL,
    ZERO_DIGEST,
    WorkerEvent,
    WorkerRequest,
    WorkerRequestBudgets,
    WorkerResponse,
)


DEFAULT_MAX_MESSAGE_BYTES = 20 * 1024 * 1024
DEFAULT_MAX_RETAINED_MESSAGES = 8192
DEFAULT_REQUEST_TIMEOUT = 120.0
DEFAULT_MAX_TOMBSTONES = 1024
REQUEST_ID_PATTERN = re.compile(r"^request:[A-Za-z0-9._-]+$")
NONCE_PATTERN = re.compile(r"^nonce:[A-Za-z0-9._-]+$")
DIGEST_PATTERN = re.compile(r"^sha256:[0-9a-f]{64}$")
EXPECTED_FAILURE_CLASSES = [
    "adapter_failed", "authority_denied", "budget_exhausted", "cancelled",
    "internal_error", "invalid_result", "protocol_mismatch", "source_mismatch",
    "timeout", "worker_crashed", "worker_unhealthy",
]
EXPECTED_REQUEST_STATES = ["accepted", "running", "completed", "failed", "cancelled", "timed_out"]


class WorkerClientError(RuntimeError):
    """Base error for local client transport and framing failures."""


class WorkerClientClosedError(WorkerClientError):
    """Raised when the worker transport is unavailable."""


class WorkerClientProtocolError(WorkerClientError):
    """Raised when a stream message violates the negotiated protocol."""


class WorkerClientTimeoutError(WorkerClientError):
    """Raised when a request's local response deadline expires."""


class WorkerClientCancelledError(WorkerClientError):
    """Raised when a caller cancels a local pending response."""


class _PendingRequest:
    def __init__(self, request_id: str, operation: str):
        self.request_id = request_id
        self.operation = operation
        self.response: WorkerResponse | None = None
        self.error: BaseException | None = None
        self.done = False
        self.timer: threading.Timer | None = None


class PendingResponse:
    """One in-flight request with a bounded local wait lifecycle."""

    def __init__(self, client: "WorkerClient", pending: _PendingRequest):
        self._client = client
        self._pending = pending
        self.request_id = pending.request_id

    def result(self, timeout: float | None = None) -> WorkerResponse:
        return self._client._wait_response(self._pending, timeout)

    def cancel(self) -> bool:
        """Cancel this local wait; use ``WorkerClient.cancel`` for worker cancellation."""

        return self._client._abandon_pending(
            self._pending,
            WorkerClientCancelledError(f"{self.request_id}: request wait was cancelled"),
        )


class WorkerClient:
    """Thin request builder and ordered JSONL transport for ``sley.worker.v1``.

    The client never retries a request automatically. A local deadline or
    cancellation removes its pending entry and retains a bounded tombstone so
    a late worker response is discarded deterministically. Canonical worker
    cancellation remains a separate ``cancel`` control request.
    """

    def __init__(
        self,
        *,
        command: Sequence[str] = ("sley",),
        cwd: str | os.PathLike[str] | None = None,
        env: Mapping[str, str] | None = None,
        max_requests: int = 128,
        idle_timeout_ms: int = 300_000,
        max_message_bytes: int = DEFAULT_MAX_MESSAGE_BYTES,
        max_retained_messages: int = DEFAULT_MAX_RETAINED_MESSAGES,
        request_timeout: float = DEFAULT_REQUEST_TIMEOUT,
        max_tombstones: int = DEFAULT_MAX_TOMBSTONES,
    ):
        if not command:
            raise ValueError("command must not be empty")
        if max_requests < 1 or max_requests > 1_000_000:
            raise ValueError("max_requests must be between 1 and 1000000")
        if idle_timeout_ms < 0 or idle_timeout_ms > 86_400_000:
            raise ValueError("idle_timeout_ms must be between 0 and 86400000")
        if max_message_bytes < 1 or max_retained_messages < 1:
            raise ValueError("client stream bounds must be positive")
        if request_timeout <= 0 or request_timeout > 86_400:
            raise ValueError("request_timeout must be between 0 and 86400 seconds")
        if max_tombstones < 1 or max_tombstones > 1_000_000:
            raise ValueError("max_tombstones must be between 1 and 1000000")
        self.command = tuple(command)
        self.cwd = Path(cwd).resolve() if cwd is not None else None
        self.extra_env = dict(env or {})
        self.max_requests = max_requests
        self.idle_timeout_ms = idle_timeout_ms
        self.max_message_bytes = max_message_bytes
        self.max_retained_messages = max_retained_messages
        self.request_timeout = float(request_timeout)
        self.max_tombstones = max_tombstones
        self.process: subprocess.Popen[bytes] | None = None
        self.worker_digest: str | None = None
        self.runtime_digest: str | None = None
        self._condition = threading.Condition()
        self._write_lock = threading.Lock()
        self._pending: dict[str, _PendingRequest] = {}
        self._tombstones: OrderedDict[str, str] = OrderedDict()
        self._late_response_count = 0
        self._messages: list[WorkerEvent | WorkerResponse] = []
        self._reader_error: BaseException | None = None
        self._last_event_sequence = -1
        self._stderr = bytearray()
        self._closing = False
        self._reaper_started = False

    @property
    def messages(self) -> tuple[WorkerEvent | WorkerResponse, ...]:
        with self._condition:
            return tuple(self._messages)

    @property
    def stderr(self) -> str:
        with self._condition:
            return bytes(self._stderr).decode("utf-8", errors="replace")

    @property
    def pending_request_count(self) -> int:
        with self._condition:
            return len(self._pending)

    @property
    def tombstone_count(self) -> int:
        with self._condition:
            return len(self._tombstones)

    @property
    def late_response_count(self) -> int:
        with self._condition:
            return self._late_response_count

    def start(self, timeout: float = 15.0) -> WorkerEvent:
        if self.process is not None:
            raise WorkerClientError("worker client is already started")
        environment = os.environ.copy()
        environment.update(self.extra_env)
        argv = [
            *self.command,
            "worker",
            "start",
            "--json",
            "--max-requests",
            str(self.max_requests),
            "--idle-timeout-ms",
            str(self.idle_timeout_ms),
        ]
        self.process = subprocess.Popen(
            argv,
            cwd=self.cwd,
            env=environment,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            bufsize=0,
            close_fds=True,
            start_new_session=True,
        )
        threading.Thread(target=self._read_stdout, name="sley-worker-stdout", daemon=True).start()
        threading.Thread(target=self._read_stderr, name="sley-worker-stderr", daemon=True).start()
        threading.Thread(target=self._watch_process, name="sley-worker-watch", daemon=True).start()
        return self.wait_for_event(code="WORKER_READY", timeout=timeout)

    def _read_stdout(self) -> None:
        assert self.process is not None and self.process.stdout is not None
        try:
            while True:
                line = self.process.stdout.readline(self.max_message_bytes + 2)
                if line == b"":
                    break
                if len(line) > self.max_message_bytes or not line.endswith(b"\n"):
                    raise WorkerClientProtocolError("worker message is incomplete or exceeds the client byte bound")
                try:
                    value = json.loads(line.decode("utf-8"))
                except (UnicodeDecodeError, json.JSONDecodeError) as exc:
                    raise WorkerClientProtocolError("worker emitted invalid UTF-8 JSON") from exc
                self._accept_message(value)
        except BaseException as exc:
            self._fail_reader(exc)
        finally:
            if not self._closing:
                self._fail_reader(WorkerClientClosedError("worker output closed unexpectedly"))

    def _watch_process(self) -> None:
        assert self.process is not None
        return_code = self.process.wait()
        if not self._closing:
            self._fail_reader(WorkerClientClosedError(f"worker exited unexpectedly with status {return_code}"))

    def _read_stderr(self) -> None:
        assert self.process is not None and self.process.stderr is not None
        while True:
            chunk = self.process.stderr.read(8192)
            if not chunk:
                break
            with self._condition:
                remaining = max(0, 65_536 - len(self._stderr))
                if remaining:
                    self._stderr.extend(chunk[:remaining])

    def _accept_message(self, value: Any) -> None:
        if not isinstance(value, dict) or value.get("protocol") != PROTOCOL:
            raise WorkerClientProtocolError("worker emitted a message outside sley.worker.v1")
        schema = value.get("schema")
        if schema not in ("sley.worker.event.v1", "sley.worker.response.v1"):
            raise WorkerClientProtocolError("worker emitted an unknown stream schema")
        with self._condition:
            if schema == "sley.worker.event.v1":
                sequence = value.get("sequence")
                if not isinstance(sequence, int) or sequence <= self._last_event_sequence:
                    raise WorkerClientProtocolError("worker event sequence is not strictly increasing")
                self._last_event_sequence = sequence
                message = cast(WorkerEvent, value)
            else:
                message = cast(WorkerResponse, value)
                request_id = message.get("request_id")
                if isinstance(request_id, str) and request_id in self._tombstones:
                    self._validate_response(message, self._tombstones[request_id])
                    self._late_response_count += 1
                    return
                if not isinstance(request_id, str) or request_id not in self._pending:
                    raise WorkerClientProtocolError("worker response does not match one pending request")
                pending = self._pending[request_id]
                if pending.done:
                    raise WorkerClientProtocolError("worker emitted more than one response for a request")
                self._validate_response(message, pending.operation)
                if len(self._messages) >= self.max_retained_messages:
                    raise WorkerClientProtocolError("worker stream exceeded the configured retained-message bound")
                pending.response = message
                pending.done = True
                if pending.timer is not None:
                    pending.timer.cancel()
                self._pending.pop(request_id)
            if schema == "sley.worker.event.v1" and len(self._messages) >= self.max_retained_messages:
                raise WorkerClientProtocolError("worker stream exceeded the configured retained-message bound")
            self._messages.append(message)
            self._condition.notify_all()

    def _validate_response(self, value: WorkerResponse, operation: str) -> None:
        if value.get("operation") != operation or value.get("status") not in ("passed", "failed", "cancelled"):
            raise WorkerClientProtocolError("worker response operation or status disagrees with its pending request")
        worker = value.get("worker")
        isolation = value.get("isolation")
        if not isinstance(worker, dict):
            raise WorkerClientProtocolError("worker response does not contain a session")
        cache = worker.get("cache")
        if (
            worker.get("schema") != "sley.worker.session.v1"
            or worker.get("protocol") != PROTOCOL
            or not isinstance(worker.get("worker_id"), str)
            or re.fullmatch(r"worker:[A-Za-z0-9._-]+", worker["worker_id"]) is None
            or not isinstance(worker.get("worker_digest"), str)
            or DIGEST_PATTERN.fullmatch(worker["worker_digest"]) is None
            or not isinstance(worker.get("runtime_digest"), str)
            or DIGEST_PATTERN.fullmatch(worker["runtime_digest"]) is None
            or not isinstance(worker.get("generation"), int)
            or worker["generation"] < 1
            or not isinstance(worker.get("request_count"), int)
            or worker["request_count"] < 0
            or worker.get("max_requests") != self.max_requests
            or cache != {"policy": "disabled", "shared_mutable_state": False, "hit_count": 0}
        ):
            raise WorkerClientProtocolError("worker response contains an invalid session identity or cache contract")
        expected_isolation = {
            "class": "fresh_subprocess_per_invoke",
            "fresh_process_per_invoke": True,
            "request_local_tempdir": True,
            "clean_environment": True,
            "closed_inherited_fds": True,
            "resource_limits": True,
            "process_group_cancellation": True,
            "namespace_isolation_enforced": False,
        }
        if isolation != expected_isolation or worker.get("isolation") != expected_isolation:
            raise WorkerClientProtocolError("worker response does not provide the required isolation capabilities")
        if operation == "handshake" and value.get("status") == "passed":
            result = value.get("result")
            if not isinstance(result, dict) or (
                result.get("protocol") != PROTOCOL
                or result.get("operations") != list(OPERATIONS)
                or result.get("contract_versions") != list(CONTRACT_VERSIONS)
                or result.get("failure_classes") != EXPECTED_FAILURE_CLASSES
                or result.get("request_states") != EXPECTED_REQUEST_STATES
            ):
                raise WorkerClientProtocolError("handshake response disagrees with canonical worker capabilities")

    def _remember_tombstone(self, request_id: str, operation: str) -> None:
        self._tombstones.pop(request_id, None)
        self._tombstones[request_id] = operation
        while len(self._tombstones) > self.max_tombstones:
            self._tombstones.popitem(last=False)

    def _signal_process(self, sig: signal.Signals) -> None:
        process = self.process
        if process is None or process.poll() is not None:
            return
        try:
            os.killpg(process.pid, sig)
        except ProcessLookupError:
            pass

    def _start_reaper(self) -> None:
        with self._condition:
            if self._reaper_started:
                return
            self._reaper_started = True
        process = self.process
        if process is None or process.poll() is not None:
            return
        try:
            if process.stdin is not None:
                process.stdin.close()
        except OSError:
            pass
        self._signal_process(signal.SIGTERM)

        def reap() -> None:
            try:
                process.wait(timeout=1)
            except subprocess.TimeoutExpired:
                self._signal_process(signal.SIGKILL)
                process.wait(timeout=5)

        threading.Thread(target=reap, name="sley-worker-reaper", daemon=True).start()

    def _fail_reader(self, error: BaseException) -> None:
        with self._condition:
            if self._reader_error is not None:
                return
            self._reader_error = error
            for pending in self._pending.values():
                if pending.timer is not None:
                    pending.timer.cancel()
                pending.error = WorkerClientClosedError(f"{pending.request_id}: {error}")
                pending.done = True
            self._pending.clear()
            self._condition.notify_all()
        self._start_reaper()

    def _transport(self) -> tuple[subprocess.Popen[bytes], Any]:
        process = self.process
        if process is None or process.stdin is None or process.poll() is not None:
            raise WorkerClientClosedError("worker process is not running")
        return process, process.stdin

    def send_async(self, request: WorkerRequest, timeout: float | None = None) -> PendingResponse:
        if request.get("schema") != "sley.worker.request.v1" or request.get("protocol") != PROTOCOL:
            raise WorkerClientProtocolError("request does not use the canonical worker envelope")
        request_id = request.get("request_id")
        if not isinstance(request_id, str) or REQUEST_ID_PATTERN.fullmatch(request_id) is None:
            raise WorkerClientProtocolError("request_id is not a canonical worker request identity")
        nonce = request.get("nonce")
        if not isinstance(nonce, str) or NONCE_PATTERN.fullmatch(nonce) is None:
            raise WorkerClientProtocolError("nonce is not a canonical worker request nonce")
        if request.get("operation") not in OPERATIONS:
            raise WorkerClientProtocolError("request operation is outside the canonical worker contract")
        encoded = json.dumps(request, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8") + b"\n"
        if len(encoded) > self.max_message_bytes:
            raise WorkerClientProtocolError("request exceeds the client outbound byte bound")
        effective_timeout = self.request_timeout if timeout is None else timeout
        if effective_timeout <= 0 or effective_timeout > 86_400:
            raise ValueError("timeout must be between 0 and 86400 seconds")
        with self._condition:
            if self._reader_error is not None:
                raise WorkerClientClosedError(str(self._reader_error)) from self._reader_error
            if request_id in self._pending or request_id in self._tombstones:
                raise WorkerClientProtocolError("request_id has already been used in this client")
            pending = _PendingRequest(request_id, cast(str, request["operation"]))
            pending.timer = threading.Timer(
                effective_timeout,
                lambda: self._abandon_pending(
                    pending,
                    WorkerClientTimeoutError(f"{request_id}: timed out after {effective_timeout:g}s"),
                ),
            )
            pending.timer.daemon = True
            self._pending[request_id] = pending
            pending.timer.start()
        try:
            _, stdin = self._transport()
            with self._write_lock:
                stdin.write(encoded)
                stdin.flush()
        except BaseException:
            self._fail_reader(WorkerClientClosedError(f"{request_id}: worker request write failed"))
            raise
        return PendingResponse(self, pending)

    def send(self, request: WorkerRequest, timeout: float | None = None) -> WorkerResponse:
        return self.send_async(request, timeout).result()

    def _abandon_pending(self, pending: _PendingRequest, error: BaseException) -> bool:
        with self._condition:
            if pending.done:
                return False
            pending.done = True
            pending.error = error
            if pending.timer is not None:
                pending.timer.cancel()
            self._pending.pop(pending.request_id, None)
            self._remember_tombstone(pending.request_id, pending.operation)
            self._condition.notify_all()
            return True

    def _wait_response(self, pending: _PendingRequest, timeout: float | None) -> WorkerResponse:
        deadline = None if timeout is None else time.monotonic() + timeout
        with self._condition:
            while True:
                if pending.done:
                    if pending.error is not None:
                        raise pending.error
                    assert pending.response is not None
                    return pending.response
                remaining = None if deadline is None else deadline - time.monotonic()
                if remaining is not None and remaining <= 0:
                    self._abandon_pending(
                        pending,
                        WorkerClientTimeoutError(f"{pending.request_id}: local result wait timed out"),
                    )
                    continue
                self._condition.wait(remaining)

    def wait_for_event(
        self,
        *,
        code: str | None = None,
        request_id: str | None = None,
        predicate: Callable[[WorkerEvent], bool] | None = None,
        timeout: float = 15.0,
    ) -> WorkerEvent:
        deadline = time.monotonic() + timeout
        index = 0
        with self._condition:
            while True:
                for message in self._messages[index:]:
                    index += 1
                    if message.get("schema") != "sley.worker.event.v1":
                        continue
                    event = cast(WorkerEvent, message)
                    if code is not None and event.get("code") != code:
                        continue
                    if request_id is not None and event.get("request_id") != request_id:
                        continue
                    if predicate is not None and not predicate(event):
                        continue
                    return event
                if self._reader_error is not None:
                    raise WorkerClientClosedError(str(self._reader_error)) from self._reader_error
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise TimeoutError("timed out waiting for a worker event")
                self._condition.wait(remaining)

    @staticmethod
    def _identity(prefix: str) -> str:
        return f"{prefix}:{secrets.token_hex(12)}"

    def _bound_digests(self) -> tuple[str, str]:
        if self.worker_digest is None or self.runtime_digest is None:
            raise WorkerClientProtocolError("handshake must pass before bound worker requests")
        return self.worker_digest, self.runtime_digest

    def build_control_request(
        self,
        operation: str,
        budgets: WorkerRequestBudgets,
        *,
        target_request_id: str | None = None,
        request_id: str | None = None,
        nonce: str | None = None,
    ) -> WorkerRequest:
        if operation not in ("handshake", "cancel", "reset", "drain", "health", "shutdown"):
            raise ValueError("operation is not a worker control operation")
        if operation == "handshake":
            worker_digest, runtime_digest = ZERO_DIGEST, ZERO_DIGEST
        else:
            worker_digest, runtime_digest = self._bound_digests()
        if operation == "cancel":
            if target_request_id is None:
                raise ValueError("cancel requires target_request_id")
            payload = {"target_request_id": target_request_id}
        else:
            if target_request_id is not None:
                raise ValueError("target_request_id is accepted only by cancel")
            payload = {}
        return cast(WorkerRequest, {
            "schema": "sley.worker.request.v1",
            "protocol": PROTOCOL,
            "request_id": request_id or self._identity("request"),
            "operation": operation,
            "nonce": nonce or self._identity("nonce"),
            "bindings": {
                "worker_digest": worker_digest,
                "runtime_digest": runtime_digest,
                "package_digest": ZERO_DIGEST,
                "source_digest": ZERO_DIGEST,
                "contract_versions": list(CONTRACT_VERSIONS),
                "entry_point": f"worker.{operation}",
                "authority": {"mode": "absent", "grant_digest": None, "idempotency_key": None},
            },
            "budgets": dict(budgets),
            "payload": payload,
        })

    def build_adapter_request(
        self,
        operation: str,
        budgets: WorkerRequestBudgets,
        *,
        manifest: str,
        package_digest: str,
        source_digest: str,
        record: str | None = None,
        idempotency_key: str | None = None,
        request_id: str | None = None,
        nonce: str | None = None,
    ) -> WorkerRequest:
        if operation not in ("load", "capabilities", "invoke"):
            raise ValueError("operation is not an adapter worker operation")
        worker_digest, runtime_digest = self._bound_digests()
        if operation == "invoke":
            idempotency_key = idempotency_key or self._identity("idempotency")
        elif idempotency_key is not None:
            raise ValueError("idempotency_key is accepted only by invoke")
        return cast(WorkerRequest, {
            "schema": "sley.worker.request.v1",
            "protocol": PROTOCOL,
            "request_id": request_id or self._identity("request"),
            "operation": operation,
            "nonce": nonce or self._identity("nonce"),
            "bindings": {
                "worker_digest": worker_digest,
                "runtime_digest": runtime_digest,
                "package_digest": package_digest,
                "source_digest": source_digest,
                "contract_versions": list(CONTRACT_VERSIONS),
                "entry_point": "adapter.replay",
                "authority": {
                    "mode": "local_replay_only",
                    "grant_digest": None,
                    "idempotency_key": idempotency_key,
                },
            },
            "budgets": dict(budgets),
            "payload": {"manifest": manifest, "record": record, "target_request_id": None},
        })

    def handshake(self, budgets: WorkerRequestBudgets, timeout: float = 15.0) -> WorkerResponse:
        response = self.send(self.build_control_request("handshake", budgets), timeout)
        if response["status"] == "passed":
            self.worker_digest = response["worker"]["worker_digest"]
            self.runtime_digest = response["worker"]["runtime_digest"]
        return response

    def load(self, budgets: WorkerRequestBudgets, **bindings: Any) -> WorkerResponse:
        return self.send(self.build_adapter_request("load", budgets, **bindings))

    def capabilities(self, budgets: WorkerRequestBudgets, **bindings: Any) -> WorkerResponse:
        return self.send(self.build_adapter_request("capabilities", budgets, **bindings))

    def invoke(self, budgets: WorkerRequestBudgets, **bindings: Any) -> WorkerResponse:
        return self.send(self.build_adapter_request("invoke", budgets, **bindings))

    def cancel(self, target_request_id: str, budgets: WorkerRequestBudgets) -> WorkerResponse:
        return self.send(self.build_control_request("cancel", budgets, target_request_id=target_request_id))

    def reset(self, budgets: WorkerRequestBudgets) -> WorkerResponse:
        return self.send(self.build_control_request("reset", budgets))

    def drain(self, budgets: WorkerRequestBudgets) -> WorkerResponse:
        return self.send(self.build_control_request("drain", budgets))

    def health(self, budgets: WorkerRequestBudgets) -> WorkerResponse:
        return self.send(self.build_control_request("health", budgets))

    def shutdown(self, budgets: WorkerRequestBudgets, timeout: float = 15.0) -> WorkerResponse:
        response = self.send(self.build_control_request("shutdown", budgets), timeout)
        self._closing = True
        self.wait_for_event(code="WORKER_STOPPED", timeout=timeout)
        if self.process is not None:
            self.process.wait(timeout=timeout)
        return response

    def close(self) -> None:
        self._closing = True
        process = self.process
        if process is None or process.poll() is not None:
            return
        self._fail_reader(WorkerClientClosedError("worker client was closed"))
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self._signal_process(signal.SIGKILL)
            process.wait(timeout=5)

    def __enter__(self) -> "WorkerClient":
        self.start()
        return self

    def __exit__(self, exc_type: Any, exc: Any, traceback: Any) -> None:
        self.close()
