"""Synchronous stdio client for the canonical Sley worker protocol."""

from __future__ import annotations

import json
import os
import re
import secrets
import subprocess
import threading
import time
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
REQUEST_ID_PATTERN = re.compile(r"^request:[A-Za-z0-9._-]+$")
NONCE_PATTERN = re.compile(r"^nonce:[A-Za-z0-9._-]+$")


class WorkerClientError(RuntimeError):
    """Base error for local client transport and framing failures."""


class WorkerClientClosedError(WorkerClientError):
    """Raised when the worker transport is unavailable."""


class WorkerClientProtocolError(WorkerClientError):
    """Raised when a stream message violates the negotiated protocol."""


class PendingResponse:
    """One in-flight request whose worker response remains authoritative."""

    def __init__(self, client: "WorkerClient", request_id: str):
        self._client = client
        self.request_id = request_id
        self._response: WorkerResponse | None = None

    def result(self, timeout: float | None = None) -> WorkerResponse:
        if self._response is None:
            self._response = self._client._wait_response(self.request_id, timeout)
        return self._response


class WorkerClient:
    """Thin request builder and ordered JSONL transport for ``sley.worker.v1``.

    The client never retries a request automatically. A local wait timeout does
    not cancel or reinterpret the worker request; callers may issue a separate
    canonical ``cancel`` request and must still observe the invoke response.
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
    ):
        if not command:
            raise ValueError("command must not be empty")
        if max_requests < 1 or max_requests > 1_000_000:
            raise ValueError("max_requests must be between 1 and 1000000")
        if idle_timeout_ms < 0 or idle_timeout_ms > 86_400_000:
            raise ValueError("idle_timeout_ms must be between 0 and 86400000")
        if max_message_bytes < 1 or max_retained_messages < 1:
            raise ValueError("client stream bounds must be positive")
        self.command = tuple(command)
        self.cwd = Path(cwd).resolve() if cwd is not None else None
        self.extra_env = dict(env or {})
        self.max_requests = max_requests
        self.idle_timeout_ms = idle_timeout_ms
        self.max_message_bytes = max_message_bytes
        self.max_retained_messages = max_retained_messages
        self.process: subprocess.Popen[bytes] | None = None
        self.worker_digest: str | None = None
        self.runtime_digest: str | None = None
        self._condition = threading.Condition()
        self._write_lock = threading.Lock()
        self._pending: dict[str, WorkerResponse | None] = {}
        self._messages: list[WorkerEvent | WorkerResponse] = []
        self._reader_error: BaseException | None = None
        self._last_event_sequence = -1
        self._stderr = bytearray()

    @property
    def messages(self) -> tuple[WorkerEvent | WorkerResponse, ...]:
        with self._condition:
            return tuple(self._messages)

    @property
    def stderr(self) -> str:
        with self._condition:
            return bytes(self._stderr).decode("utf-8", errors="replace")

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
        )
        threading.Thread(target=self._read_stdout, name="sley-worker-stdout", daemon=True).start()
        threading.Thread(target=self._read_stderr, name="sley-worker-stderr", daemon=True).start()
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
            with self._condition:
                self._reader_error = exc
                self._condition.notify_all()
        finally:
            with self._condition:
                if self._reader_error is None and any(value is None for value in self._pending.values()):
                    self._reader_error = WorkerClientClosedError("worker output closed before every response arrived")
                self._condition.notify_all()

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
            if len(self._messages) >= self.max_retained_messages:
                raise WorkerClientProtocolError("worker stream exceeded the configured retained-message bound")
            if schema == "sley.worker.event.v1":
                sequence = value.get("sequence")
                if not isinstance(sequence, int) or sequence <= self._last_event_sequence:
                    raise WorkerClientProtocolError("worker event sequence is not strictly increasing")
                self._last_event_sequence = sequence
                message = cast(WorkerEvent, value)
            else:
                message = cast(WorkerResponse, value)
                request_id = message.get("request_id")
                if not isinstance(request_id, str) or request_id not in self._pending:
                    raise WorkerClientProtocolError("worker response does not match one pending request")
                if self._pending[request_id] is not None:
                    raise WorkerClientProtocolError("worker emitted more than one response for a request")
                self._pending[request_id] = message
            self._messages.append(message)
            self._condition.notify_all()

    def _transport(self) -> tuple[subprocess.Popen[bytes], Any]:
        process = self.process
        if process is None or process.stdin is None or process.poll() is not None:
            raise WorkerClientClosedError("worker process is not running")
        return process, process.stdin

    def send_async(self, request: WorkerRequest) -> PendingResponse:
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
        with self._condition:
            if self._reader_error is not None:
                raise WorkerClientClosedError(str(self._reader_error)) from self._reader_error
            if request_id in self._pending:
                raise WorkerClientProtocolError("request_id is already pending in this client")
            self._pending[request_id] = None
        try:
            _, stdin = self._transport()
            with self._write_lock:
                stdin.write(encoded)
                stdin.flush()
        except BaseException:
            with self._condition:
                self._pending.pop(request_id, None)
            raise
        return PendingResponse(self, request_id)

    def send(self, request: WorkerRequest, timeout: float | None = None) -> WorkerResponse:
        return self.send_async(request).result(timeout)

    def _wait_response(self, request_id: str, timeout: float | None) -> WorkerResponse:
        deadline = None if timeout is None else time.monotonic() + timeout
        with self._condition:
            while True:
                response = self._pending.get(request_id)
                if response is not None:
                    self._pending.pop(request_id, None)
                    return response
                if self._reader_error is not None:
                    raise WorkerClientClosedError(str(self._reader_error)) from self._reader_error
                remaining = None if deadline is None else deadline - time.monotonic()
                if remaining is not None and remaining <= 0:
                    raise TimeoutError(
                        f"timed out waiting for {request_id}; the worker request remains active until its response or explicit cancellation"
                    )
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
            result = response.get("result") or {}
            if result.get("protocol") != PROTOCOL or result.get("contract_versions") != list(CONTRACT_VERSIONS):
                raise WorkerClientProtocolError("handshake response disagrees with canonical client contracts")
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
        self.wait_for_event(code="WORKER_STOPPED", timeout=timeout)
        if self.process is not None:
            self.process.wait(timeout=timeout)
        return response

    def close(self) -> None:
        process = self.process
        if process is None or process.poll() is not None:
            return
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)

    def __enter__(self) -> "WorkerClient":
        self.start()
        return self

    def __exit__(self, exc_type: Any, exc: Any, traceback: Any) -> None:
        self.close()
