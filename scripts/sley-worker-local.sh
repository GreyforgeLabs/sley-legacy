#!/usr/bin/env bash
set -euo pipefail

exec 3<&0
python3 - "$@" <<'PY'
"""Persistent bounded local worker for deterministic Sley adapter replay."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
import os
import select
import signal
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator


ZERO_DIGEST = "sha256:" + ("0" * 64)
MAX_LINE_BYTES = 1_048_576
MAX_MANIFEST_BYTES = 1_048_576


def required_env(name: str) -> str:
    value = os.environ.get(name)
    if value is None or value == "":
        raise SystemExit(f"missing required worker source value: {name}")
    return value


PROTOCOL = required_env("SLEY_WORKER_PROTOCOL")
REQUEST_SCHEMA = required_env("SLEY_WORKER_REQUEST_SCHEMA")
RESPONSE_SCHEMA = required_env("SLEY_WORKER_RESPONSE_SCHEMA")
EVENT_SCHEMA = required_env("SLEY_WORKER_EVENT_SCHEMA")
SESSION_SCHEMA = required_env("SLEY_WORKER_SESSION_SCHEMA")
RUNTIME_DIGEST = required_env("SLEY_WORKER_RUNTIME_DIGEST")
OPERATIONS = json.loads(required_env("SLEY_WORKER_OPERATIONS"))
LIFECYCLE_STATES = json.loads(required_env("SLEY_WORKER_LIFECYCLE_STATES"))
REQUEST_STATES = json.loads(required_env("SLEY_WORKER_REQUEST_STATES"))
FAILURE_CLASSES = set(json.loads(required_env("SLEY_WORKER_FAILURE_CLASSES")))
CONTRACT_VERSIONS = json.loads(required_env("SLEY_WORKER_CONTRACT_VERSIONS"))
ISOLATION_CLASS = required_env("SLEY_WORKER_ISOLATION_CLASS")
CACHE_POLICY = required_env("SLEY_WORKER_CACHE_POLICY")
INPUT = os.fdopen(3, "rb", buffering=0)
ADAPTER_ENV_NAMES = (
    "SLEY_ADAPTER_MANIFEST_SCHEMA",
    "SLEY_ADAPTER_REPLAY_SCHEMA",
    "SLEY_ADAPTER_REPORT_SCHEMA",
    "SLEY_ADAPTER_LIFECYCLE_STATES",
    "SLEY_ADAPTER_REPLAY_EVENT_PHASES",
    "SLEY_ADAPTER_FAILURE_CODES",
    "SLEY_ADAPTER_SEED_FAMILIES",
    "SLEY_ADAPTER_SUPPORTED_EFFECTS",
    "SLEY_ADAPTER_AUTHORITY_MODE",
    "SLEY_ADAPTER_REPLAY_POLICY",
    "SLEY_ADAPTER_KIND",
    "SLEY_ADAPTER_LIVE_EXECUTION",
)
ADAPTER_ENV = {name: required_env(name) for name in ADAPTER_ENV_NAMES}


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def digest(value: Any) -> str:
    return "sha256:" + hashlib.sha256(canonical_bytes(value)).hexdigest()


def identity_digest(value: dict[str, Any], field: str) -> str:
    projected = copy.deepcopy(value)
    projected[field] = ZERO_DIGEST
    return digest(projected)


def file_digest(path: Path) -> str:
    hasher = hashlib.sha256()
    if path.is_file():
        hasher.update(path.read_bytes())
    else:
        for child in sorted(item for item in path.rglob("*.sley") if item.is_file() and not item.is_symlink()):
            hasher.update(child.relative_to(path).as_posix().encode("utf-8"))
            hasher.update(b"\0")
            hasher.update(child.read_bytes())
            hasher.update(b"\0")
    return "sha256:" + hasher.hexdigest()


def static_source_metrics(path: Path) -> tuple[int, int, bool]:
    import re

    files = [path] if path.is_file() else sorted(item for item in path.rglob("*.sley") if item.is_file())
    step_count = 0
    task_names: set[str] = set()
    edges: dict[str, set[str]] = {}
    current: str | None = None
    depth = 0
    for source in files:
        module = "main"
        for raw in source.read_text(encoding="utf-8", errors="replace").splitlines():
            stripped = raw.strip()
            if stripped and not stripped.startswith(("#", "//")):
                step_count += 1
            module_match = re.match(r"^module\s+([A-Za-z_][A-Za-z0-9_.]*)$", stripped)
            if module_match and current is None:
                module = module_match.group(1)
                continue
            if current is None:
                task_match = re.match(r"^(?:export\s+)?task\s+([A-Za-z_][A-Za-z0-9_]*)\b", stripped)
                if task_match:
                    current = f"{module}.{task_match.group(1)}"
                    task_names.add(current)
                    edges.setdefault(current, set())
                    depth = raw.count("{") - raw.count("}")
                continue
            for call in re.finditer(r"\bcall\s+([A-Za-z_][A-Za-z0-9_.]*)\s*\(", stripped):
                callee = call.group(1)
                if "." not in callee:
                    callee = f"{module}.{callee}"
                edges[current].add(callee)
            depth += raw.count("{") - raw.count("}")
            if depth <= 0:
                current = None

    cycle = False

    def visit(task: str, active: set[str], memo: dict[str, int]) -> int:
        nonlocal cycle
        if task in memo:
            return memo[task]
        if task in active:
            cycle = True
            return len(task_names) + 1
        active.add(task)
        children = [child for child in edges.get(task, set()) if child in task_names]
        value = 1 + max((visit(child, active, memo) for child in children), default=0)
        active.remove(task)
        memo[task] = value
        return value

    roots = [task for task in task_names if task.endswith(".main")] or sorted(task_names)
    memo: dict[str, int] = {}
    call_depth = max((visit(task, set(), memo) for task in roots), default=0)
    return step_count, call_depth, cycle


def reject_nested_symlinks(path: Path) -> None:
    if not path.is_dir():
        return
    for directory, names, files in os.walk(path, followlinks=False):
        parent = Path(directory)
        for name in names + files:
            if (parent / name).is_symlink():
                raise ValueError("target directory contains a nested symlink")


def confined_regular(path: Path, root: Path, *, directory_ok: bool = False) -> Path:
    if path.is_symlink():
        raise ValueError("symlink paths are not accepted")
    resolved = path.resolve(strict=True)
    try:
        resolved.relative_to(root)
    except ValueError as exc:
        raise ValueError("path is outside the repository root") from exc
    if directory_ok:
        if not (resolved.is_file() or resolved.is_dir()):
            raise ValueError("path is not a regular file or directory")
    elif not resolved.is_file():
        raise ValueError("path is not a regular file")
    return resolved


def load_json(path: Path, byte_limit: int) -> dict[str, Any]:
    data = path.read_bytes()
    if len(data) > byte_limit:
        raise ValueError("JSON input exceeds its byte limit")
    value = json.loads(data.decode("utf-8"))
    if not isinstance(value, dict):
        raise ValueError("JSON input must be an object")
    return value


def collection_count(value: Any) -> int:
    if isinstance(value, dict):
        return len(value) + sum(collection_count(item) for item in value.values())
    if isinstance(value, list):
        return len(value) + sum(collection_count(item) for item in value)
    return 0


def copy_snapshot_path(source: Path, root: Path, snapshot_root: Path, *, source_tree: bool = False) -> Path:
    relative = source.relative_to(root)
    destination = snapshot_root / relative
    if source.is_file():
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
        destination.chmod(0o400)
        return destination
    destination.mkdir(parents=True, exist_ok=True)
    for child in sorted(source.rglob("*.sley") if source_tree else source.rglob("*")):
        if child.is_symlink():
            raise ValueError("snapshot source contains a symlink")
        target = destination / child.relative_to(source)
        if child.is_dir():
            target.mkdir(parents=True, exist_ok=True)
        elif child.is_file():
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(child, target)
            target.chmod(0o400)
    return destination


def bounded_communicate(process: subprocess.Popen[bytes], timeout: float, byte_limit: int) -> tuple[bytes, bytes, bool, bool]:
    buffers = {"stdout": bytearray(), "stderr": bytearray()}
    total = 0
    total_lock = threading.Lock()
    exceeded = threading.Event()

    def kill_group() -> None:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass

    def reader(name: str, pipe: Any) -> None:
        nonlocal total
        while True:
            chunk = pipe.read(65536)
            if not chunk:
                break
            with total_lock:
                stored = len(buffers["stdout"]) + len(buffers["stderr"])
                remaining = max(0, byte_limit + 1 - stored)
                if remaining:
                    buffers[name].extend(chunk[:remaining])
                total += len(chunk)
                if total > byte_limit and not exceeded.is_set():
                    exceeded.set()
                    kill_group()

    readers = [
        threading.Thread(target=reader, args=("stdout", process.stdout), daemon=True),
        threading.Thread(target=reader, args=("stderr", process.stderr), daemon=True),
    ]
    for thread in readers:
        thread.start()
    deadline = time.monotonic() + timeout
    timed_out = False
    while process.poll() is None:
        if time.monotonic() >= deadline:
            timed_out = True
            kill_group()
            break
        time.sleep(0.01)
    try:
        process.wait(timeout=2)
    except subprocess.TimeoutExpired:
        kill_group()
        process.wait(timeout=2)
    for thread in readers:
        thread.join(timeout=2)
    return bytes(buffers["stdout"]), bytes(buffers["stderr"]), timed_out, exceeded.is_set()


class WorkerProblem(Exception):
    def __init__(self, failure_class: str, code: str, message: str, path: str = "/", retryable: bool = False):
        super().__init__(message)
        self.failure_class = failure_class if failure_class in FAILURE_CLASSES else "internal_error"
        self.code = code
        self.message = message
        self.path = path
        self.retryable = retryable


class Worker:
    def __init__(self, root: Path, max_requests: int, idle_timeout_ms: int):
        self.root = root
        self.max_requests = max_requests
        self.idle_timeout_ms = idle_timeout_ms
        self.state = "starting"
        self.generation = 1
        self.request_count = 0
        self.active: dict[str, Any] | None = None
        self.seen_request_ids: set[str] = set()
        self.seen_nonces: set[str] = set()
        self.seen_idempotency_keys: set[str] = set()
        self.draining = False
        self.shutdown_requested = False
        self.stop_requested = False
        self.sequence = 0
        self.lock = threading.RLock()
        self.output_lock = threading.Lock()
        self.schema = load_json(root / "docs/schemas/sley.worker.request.v1.schema.json", MAX_MANIFEST_BYTES)
        self.validator = Draft202012Validator(self.schema)
        self.worker_digest = self._worker_digest()
        self.worker_id = "worker:" + self.worker_digest.removeprefix("sha256:")[:16]
        self.isolation = {
            "class": ISOLATION_CLASS,
            "fresh_process_per_invoke": True,
            "request_local_tempdir": True,
            "clean_environment": True,
            "closed_inherited_fds": True,
            "resource_limits": True,
            "process_group_cancellation": True,
            "namespace_isolation_enforced": False,
        }
        self.state = "ready"

    def _worker_digest(self) -> str:
        hasher = hashlib.sha256()
        for relative in ("scripts/sley-worker-local.sh", "self-hosted/src/loom/worker.sley", "bin/sley"):
            path = self.root / relative
            hasher.update(relative.encode("utf-8"))
            hasher.update(b"\0")
            hasher.update(path.read_bytes())
            hasher.update(b"\0")
        return "sha256:" + hasher.hexdigest()

    def session(self) -> dict[str, Any]:
        with self.lock:
            return {
                "schema": SESSION_SCHEMA,
                "protocol": PROTOCOL,
                "worker_id": self.worker_id,
                "worker_digest": self.worker_digest,
                "runtime_digest": RUNTIME_DIGEST,
                "state": self.state,
                "generation": self.generation,
                "request_count": self.request_count,
                "active_request_id": self.active["request_id"] if self.active is not None else None,
                "max_requests": self.max_requests,
                "cache": {"policy": CACHE_POLICY, "shared_mutable_state": False, "hit_count": 0},
                "isolation": self.isolation,
            }

    def emit(self, value: dict[str, Any]) -> None:
        encoded = json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
        with self.output_lock:
            print(encoded, flush=True)

    def event(self, request_id: str | None, operation: str, phase: str, code: str, details: dict[str, Any] | None = None) -> None:
        with self.lock:
            self.sequence += 1
            worker_state = self.state
            sequence = self.sequence
        self.emit({
            "schema": EVENT_SCHEMA,
            "protocol": PROTOCOL,
            "sequence": sequence,
            "request_id": request_id,
            "operation": operation,
            "phase": phase,
            "worker_state": worker_state,
            "code": code,
            "details": details or {},
        })

    def response(
        self,
        request: dict[str, Any] | None,
        status: str,
        *,
        result: dict[str, Any] | None = None,
        problem: WorkerProblem | None = None,
        operation: str | None = None,
    ) -> None:
        request_id = request.get("request_id") if isinstance(request, dict) else None
        request_operation = operation or (request.get("operation") if isinstance(request, dict) else "unknown")
        self.emit({
            "schema": RESPONSE_SCHEMA,
            "protocol": PROTOCOL,
            "request_id": request_id if isinstance(request_id, str) and request_id.startswith("request:") else None,
            "operation": request_operation if request_operation in OPERATIONS else "unknown",
            "status": status,
            "failure_class": problem.failure_class if problem else None,
            "retryable": problem.retryable if problem else False,
            "worker": self.session(),
            "result": result,
            "isolation": self.isolation,
            "issues": [] if problem is None else [{"code": problem.code, "message": problem.message, "path": problem.path}],
        })

    def fail(self, request: dict[str, Any] | None, problem: WorkerProblem, operation: str | None = None) -> None:
        self.response(request, "cancelled" if problem.failure_class == "cancelled" else "failed", problem=problem, operation=operation)

    def validate_envelope(self, request: Any) -> dict[str, Any]:
        if not isinstance(request, dict):
            raise WorkerProblem("protocol_mismatch", "WORKER_REQUEST_INVALID", "request must be a JSON object")
        errors = sorted(self.validator.iter_errors(request), key=lambda item: list(item.absolute_path))
        if errors:
            first = errors[0]
            path = "/" + "/".join(str(part) for part in first.absolute_path)
            raise WorkerProblem("protocol_mismatch", "WORKER_REQUEST_INVALID", first.message, path or "/")
        request_id = request["request_id"]
        nonce = request["nonce"]
        with self.lock:
            if request_id in self.seen_request_ids or nonce in self.seen_nonces:
                raise WorkerProblem("protocol_mismatch", "WORKER_REPLAY_DENIED", "request_id and nonce must be unique within a worker generation", "/nonce")
            self.seen_request_ids.add(request_id)
            self.seen_nonces.add(nonce)
        bindings = request["bindings"]
        if bindings["contract_versions"] != CONTRACT_VERSIONS:
            raise WorkerProblem("protocol_mismatch", "WORKER_CONTRACT_MISMATCH", "contract versions do not match the worker protocol", "/bindings/contract_versions")
        operation = request["operation"]
        expected_entry = "adapter.replay" if operation in ("load", "capabilities", "invoke") else f"worker.{operation}"
        if bindings["entry_point"] != expected_entry:
            raise WorkerProblem("protocol_mismatch", "WORKER_ENTRY_POINT_MISMATCH", "entry point does not match the requested operation", "/bindings/entry_point")
        if operation == "handshake":
            if bindings["worker_digest"] not in (ZERO_DIGEST, self.worker_digest) or bindings["runtime_digest"] not in (ZERO_DIGEST, RUNTIME_DIGEST):
                raise WorkerProblem("protocol_mismatch", "WORKER_VERSION_MISMATCH", "handshake digest binding is not absent or current", "/bindings")
        elif bindings["worker_digest"] != self.worker_digest or bindings["runtime_digest"] != RUNTIME_DIGEST:
            raise WorkerProblem("protocol_mismatch", "WORKER_VERSION_MISMATCH", "worker or runtime digest does not match", "/bindings")
        return request

    def validate_control(self, request: dict[str, Any]) -> None:
        bindings = request["bindings"]
        payload = request["payload"]
        if bindings["package_digest"] != ZERO_DIGEST or bindings["source_digest"] != ZERO_DIGEST:
            raise WorkerProblem("source_mismatch", "WORKER_SOURCE_MISMATCH", "control requests must bind explicit absence for package and source", "/bindings")
        authority = bindings["authority"]
        if authority != {"mode": "absent", "grant_digest": None, "idempotency_key": None}:
            raise WorkerProblem("authority_denied", "WORKER_AUTHORITY_DENIED", "control requests must bind explicit absence of authority", "/bindings/authority")
        operation = request["operation"]
        if operation == "cancel":
            if set(payload) != {"target_request_id"} or payload["target_request_id"] is None:
                raise WorkerProblem("protocol_mismatch", "WORKER_REQUEST_INVALID", "cancel requires only target_request_id", "/payload")
        elif payload:
            raise WorkerProblem("protocol_mismatch", "WORKER_REQUEST_INVALID", "control operation payload must be empty", "/payload")

    def preflight_manifest(self, request: dict[str, Any]) -> tuple[Path, Path | None, dict[str, Any], Path, int, int]:
        payload = request["payload"]
        if "manifest" not in payload or set(payload) - {"manifest", "record", "target_request_id"}:
            raise WorkerProblem("protocol_mismatch", "WORKER_REQUEST_INVALID", "adapter operations require a manifest payload", "/payload")
        if payload.get("target_request_id") is not None:
            raise WorkerProblem("protocol_mismatch", "WORKER_REQUEST_INVALID", "adapter operations do not accept target_request_id", "/payload/target_request_id")
        try:
            manifest = confined_regular(self.root / payload["manifest"], self.root)
            manifest_value = load_json(manifest, MAX_MANIFEST_BYTES)
        except (OSError, ValueError, json.JSONDecodeError) as exc:
            raise WorkerProblem("source_mismatch", "WORKER_SOURCE_MISMATCH", str(exc), "/payload/manifest") from exc
        if manifest_value.get("schema") != "sley.adapter.manifest.v0" or identity_digest(manifest_value, "manifest_digest") != manifest_value.get("manifest_digest"):
            raise WorkerProblem("source_mismatch", "WORKER_SOURCE_MISMATCH", "manifest identity is invalid", "/payload/manifest")
        target_raw = Path(manifest_value.get("target", ""))
        if not target_raw.is_absolute():
            target_raw = manifest.parent / target_raw
        try:
            target = confined_regular(target_raw, self.root, directory_ok=True)
            reject_nested_symlinks(target)
        except (OSError, ValueError) as exc:
            raise WorkerProblem("source_mismatch", "WORKER_SOURCE_MISMATCH", str(exc), "/payload/manifest") from exc
        record: Path | None = None
        if payload.get("record") is not None:
            try:
                record = confined_regular(self.root / payload["record"], self.root)
            except (OSError, ValueError) as exc:
                raise WorkerProblem("source_mismatch", "WORKER_SOURCE_MISMATCH", str(exc), "/payload/record") from exc
        bindings = request["bindings"]
        if bindings["package_digest"] != manifest_value["manifest_digest"] or bindings["source_digest"] != file_digest(target):
            raise WorkerProblem("source_mismatch", "WORKER_SOURCE_MISMATCH", "package or source digest does not match current content", "/bindings")
        if bindings["runtime_digest"] != manifest_value.get("runtime", {}).get("digest"):
            raise WorkerProblem("protocol_mismatch", "WORKER_VERSION_MISMATCH", "manifest runtime digest does not match the worker runtime", "/bindings/runtime_digest")
        authority = bindings["authority"]
        if authority["mode"] != "local_replay_only" or authority["grant_digest"] is not None:
            raise WorkerProblem("authority_denied", "WORKER_AUTHORITY_DENIED", "adapter requests require local replay authority and no external grant", "/bindings/authority")
        if request["operation"] == "invoke" and authority["idempotency_key"] is None:
            raise WorkerProblem("authority_denied", "WORKER_IDEMPOTENCY_REQUIRED", "invoke requires an idempotency key", "/bindings/authority/idempotency_key")
        bounds = manifest_value.get("bounds", {})
        budgets = request["budgets"]
        steps, depth, cycle = static_source_metrics(target)
        if steps > budgets["max_steps"] or cycle or depth > budgets["max_call_depth"]:
            raise WorkerProblem("budget_exhausted", "WORKER_BUDGET_EXHAUSTED", "source exceeds the request step or call-depth budget", "/budgets")
        if int(bounds.get("max_output_bytes", 0)) > budgets["max_output_bytes"]:
            raise WorkerProblem("budget_exhausted", "WORKER_BUDGET_EXHAUSTED", "manifest output bound exceeds the request output budget", "/budgets/max_output_bytes")
        redacted_manifest = {
            "manifest_digest": manifest_value["manifest_digest"],
            "runtime": manifest_value["runtime"],
            "authority": {
                "mode": manifest_value["authority"]["mode"],
                "external_provider_calls": manifest_value["authority"]["external_provider_calls"],
            },
            "capabilities": manifest_value["capabilities"],
            "bounds": manifest_value["bounds"],
        }
        return manifest, record, redacted_manifest, target, steps, depth

    def handle_control(self, request: dict[str, Any]) -> None:
        self.validate_control(request)
        operation = request["operation"]
        if operation == "handshake":
            self.response(request, "passed", result={
                "protocol": PROTOCOL,
                "operations": OPERATIONS,
                "contract_versions": CONTRACT_VERSIONS,
                "failure_classes": sorted(FAILURE_CLASSES),
                "request_states": REQUEST_STATES,
            })
        elif operation == "health":
            self.response(request, "passed", result={"healthy": self.state in ("ready", "busy", "draining"), "state": self.state})
        elif operation == "reset":
            with self.lock:
                if self.active is not None:
                    raise WorkerProblem("worker_unhealthy", "WORKER_BUSY", "cannot reset while a request is active", retryable=True)
                self.draining = False
                self.state = "ready"
            self.response(request, "passed", result={"request_local_state": "empty", "cancellation_state": "fresh"})
        elif operation == "drain":
            with self.lock:
                self.draining = True
                self.state = "draining"
                active_request = self.active["request_id"] if self.active else None
            self.event(request["request_id"], operation, "draining", "WORKER_DRAINING", {"active_request_id": active_request})
            self.response(request, "passed", result={"accepting_invocations": False, "active_request_id": active_request})
        elif operation == "shutdown":
            with self.lock:
                self.shutdown_requested = True
                active_request = self.active["request_id"] if self.active else None
                self.state = "shutting_down"
                if active_request is None:
                    self.stop_requested = True
            self.response(request, "passed", result={"graceful": True, "active_request_id": active_request})
        elif operation == "cancel":
            target = request["payload"]["target_request_id"]
            with self.lock:
                active = self.active
                if active is None or active["request_id"] != target:
                    raise WorkerProblem("worker_unhealthy", "WORKER_REQUEST_NOT_ACTIVE", "target request is not active", "/payload/target_request_id", retryable=False)
                active["cancel"].set()
                process = active.get("process")
            if process is not None and process.poll() is None:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
            self.response(request, "passed", result={"target_request_id": target, "cancellation_requested": True})
        else:
            raise WorkerProblem("protocol_mismatch", "WORKER_REQUEST_INVALID", "operation is not a control operation")

    def handle_adapter_metadata(self, request: dict[str, Any]) -> None:
        manifest, _, value, target, steps, depth = self.preflight_manifest(request)
        result = {
            "manifest": str(manifest.relative_to(self.root)),
            "manifest_digest": value["manifest_digest"],
            "source_digest": file_digest(target),
            "runtime_digest": value["runtime"]["digest"],
            "entry_point": "adapter.replay",
            "capabilities": value["capabilities"],
            "authority": {"mode": value["authority"]["mode"], "external_provider_calls": False},
            "static": {"steps": steps, "call_depth": depth},
            "stored_request_state": False,
        }
        self.response(request, "passed", result=result)

    def start_invoke(self, request: dict[str, Any]) -> None:
        manifest, record, _, target, _, _ = self.preflight_manifest(request)
        with self.lock:
            if self.draining or self.shutdown_requested:
                raise WorkerProblem("worker_unhealthy", "WORKER_DRAINING", "worker is not accepting new invocations", retryable=True)
            if self.active is not None:
                raise WorkerProblem("worker_unhealthy", "WORKER_BUSY", "worker already has an active invocation", retryable=True)
            if self.request_count >= self.max_requests:
                self.state = "unhealthy"
                raise WorkerProblem("worker_unhealthy", "WORKER_REQUEST_LIMIT", "worker request lifecycle limit is exhausted", retryable=True)
            idempotency_key = request["bindings"]["authority"]["idempotency_key"]
            if idempotency_key in self.seen_idempotency_keys:
                raise WorkerProblem("authority_denied", "WORKER_IDEMPOTENCY_REPLAY_DENIED", "idempotency key was already accepted in this worker generation", "/bindings/authority/idempotency_key")
            self.seen_idempotency_keys.add(idempotency_key)
            self.request_count += 1
            self.state = "busy"
            active = {
                "request_id": request["request_id"],
                "cancel": threading.Event(),
                "process": None,
                "idempotency_key": idempotency_key,
            }
            self.active = active
        self.event(request["request_id"], "invoke", "accepted", "WORKER_REQUEST_ACCEPTED", {"request_count": self.request_count})
        thread = threading.Thread(target=self._invoke_thread, args=(request, manifest, record, target, active), daemon=True)
        thread.start()

    def _invoke_thread(
        self,
        request: dict[str, Any],
        manifest: Path,
        record: Path | None,
        target: Path,
        active: dict[str, Any],
    ) -> None:
        self.event(request["request_id"], "invoke", "running", "WORKER_REQUEST_RUNNING", {"fresh_process": True})
        problem: WorkerProblem | None = None
        result: dict[str, Any] | None = None
        phase = "failed"
        status = "failed"
        process: subprocess.Popen[bytes] | None = None
        try:
            with tempfile.TemporaryDirectory(prefix="sley-worker-") as private_dir:
                os.chmod(private_dir, 0o700)
                if os.environ.get("SLEY_ALLOW_TEST_HOOKS") == "1" and os.environ.get("SLEY_WORKER_TEST_SNAPSHOT_NONCE") == request["nonce"]:
                    self.event(request["request_id"], "invoke", "progress", "WORKER_SNAPSHOT_PENDING", {"test_hook": True})
                    time.sleep(3)
                snapshot_root = Path(private_dir) / "inputs"
                snapshot_root.mkdir(mode=0o700)
                snapshot_manifest = copy_snapshot_path(manifest, self.root, snapshot_root)
                snapshot_target = copy_snapshot_path(target, self.root, snapshot_root, source_tree=target.is_dir())
                snapshot_record = copy_snapshot_path(record, self.root, snapshot_root) if record is not None else None
                snapshot_manifest_value = load_json(snapshot_manifest, MAX_MANIFEST_BYTES)
                if identity_digest(snapshot_manifest_value, "manifest_digest") != request["bindings"]["package_digest"]:
                    raise WorkerProblem("source_mismatch", "WORKER_SOURCE_MISMATCH", "copied manifest does not match the request package digest", "/bindings/package_digest")
                if file_digest(snapshot_target) != request["bindings"]["source_digest"]:
                    raise WorkerProblem("source_mismatch", "WORKER_SOURCE_MISMATCH", "copied source does not match the request source digest", "/bindings/source_digest")
                adapter_command = [
                    str(self.root / "scripts/sley-adapter-replay.sh"),
                    "--repo-root", str(snapshot_root),
                    "--runtime-root", str(self.root),
                    "--logical-target", str(target),
                    "--json",
                ]
                if snapshot_record is not None:
                    adapter_command.extend(["--record", str(snapshot_record)])
                adapter_command.append(str(snapshot_manifest))
                clean_env = {
                    "PATH": "/usr/bin:/bin",
                    "LANG": "C.UTF-8",
                    "LC_ALL": "C.UTF-8",
                    "HOME": private_dir,
                    "TMPDIR": private_dir,
                    "PYTHONDONTWRITEBYTECODE": "1",
                    **ADAPTER_ENV,
                }
                budgets = request["budgets"]
                wall_seconds = budgets["wall_clock_ms"] / 1000.0
                cpu_seconds = max(1, int(math.ceil(wall_seconds)) + 1)
                if os.environ.get("SLEY_ALLOW_TEST_HOOKS") == "1" and os.environ.get("SLEY_WORKER_TEST_CRASH_NONCE") == request["nonce"]:
                    adapter_command = ["/usr/bin/env", "bash", "-c", "kill -KILL $$"]
                elif os.environ.get("SLEY_ALLOW_TEST_HOOKS") == "1" and os.environ.get("SLEY_WORKER_TEST_OUTPUT_NONCE") == request["nonce"]:
                    adapter_command = ["/usr/bin/python3", "-c", "import sys; sys.stdout.write('x' * 2097152)"]
                command = [
                    "/usr/bin/prlimit",
                    f"--as={budgets['max_memory_bytes']}:{budgets['max_memory_bytes']}",
                    f"--cpu={cpu_seconds}:{cpu_seconds}",
                    "--nofile=64:64",
                    "--",
                    *adapter_command,
                ]

                process = subprocess.Popen(
                    command,
                    cwd=self.root,
                    env=clean_env,
                    stdin=subprocess.DEVNULL,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    close_fds=True,
                    start_new_session=True,
                )
                with self.lock:
                    active["process"] = process
                self.event(request["request_id"], "invoke", "progress", "WORKER_PROCESS_STARTED", {"pid_exposed": False})
                stdout, stderr, timed_out, output_exceeded = bounded_communicate(
                    process, wall_seconds, budgets["max_output_bytes"]
                )
                if timed_out:
                    problem = WorkerProblem("timeout", "WORKER_TIMEOUT", "invocation exceeded wall_clock_ms", "/budgets/wall_clock_ms", retryable=True)
                    phase = "timed_out"
                if active["cancel"].is_set():
                    problem = WorkerProblem("cancelled", "WORKER_CANCELLED", "active invocation was cancelled preemptively", retryable=False)
                    phase = "cancelled"
                    status = "cancelled"
                elif output_exceeded:
                    problem = WorkerProblem("budget_exhausted", "WORKER_BUDGET_EXHAUSTED", "combined worker stdout and stderr exceeded max_output_bytes", "/budgets/max_output_bytes")
                elif problem is None and process.returncode is not None and process.returncode < 0:
                    problem = WorkerProblem("worker_crashed", "WORKER_CHILD_CRASHED", "isolated invocation process terminated by signal", retryable=True)
                elif problem is None:
                    try:
                        adapter_report = json.loads(stdout.decode("utf-8"))
                    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
                        raise WorkerProblem("invalid_result", "WORKER_INVALID_RESULT", "adapter process did not return one valid JSON report", retryable=False) from exc
                    if not isinstance(adapter_report, dict) or adapter_report.get("schema") != "sley.adapter.report.v0":
                        raise WorkerProblem("invalid_result", "WORKER_INVALID_RESULT", "adapter process returned the wrong report schema")
                    observed_manifest = adapter_report.get("manifest", {}).get("digest")
                    observed_source = adapter_report.get("record", {}).get("request", {}).get("target_digest")
                    if observed_manifest != request["bindings"]["package_digest"]:
                        raise WorkerProblem("source_mismatch", "WORKER_SOURCE_MISMATCH", "immutable execution snapshot does not match request bindings", "/bindings")
                    if observed_source is not None and observed_source != request["bindings"]["source_digest"]:
                        raise WorkerProblem("source_mismatch", "WORKER_SOURCE_MISMATCH", "adapter result source digest does not match the immutable execution snapshot", "/bindings/source_digest")
                    if process.returncode == 0 and observed_source is None:
                        raise WorkerProblem("invalid_result", "WORKER_INVALID_RESULT", "successful adapter result omitted its source digest")
                    adapter_report["manifest"]["path"] = str(manifest.relative_to(self.root))
                    if collection_count(adapter_report) > budgets["max_collection_items"]:
                        raise WorkerProblem("budget_exhausted", "WORKER_BUDGET_EXHAUSTED", "worker result exceeded max_collection_items", "/budgets/max_collection_items")
                    if process.returncode == 0 and adapter_report.get("status") == "passed":
                        result = {
                            "adapter_report": adapter_report,
                            "idempotency_key": active["idempotency_key"],
                            "request_local_state_retained": False,
                        }
                        status = "passed"
                        phase = "completed"
                    else:
                        code = ((adapter_report.get("issues") or [{}])[0]).get("code", "")
                        failure_class = "adapter_failed"
                        if code in ("ADAPTER_AUTHORITY_DENIED", "ADAPTER_SCOPE_DENIED", "ADAPTER_LIVE_PROVIDER_DENIED"):
                            failure_class = "authority_denied"
                        elif code == "ADAPTER_BUDGET_EXCEEDED":
                            failure_class = "budget_exhausted"
                        elif code == "ADAPTER_CANCELLED":
                            failure_class = "cancelled"
                            status = "cancelled"
                            phase = "cancelled"
                        problem = WorkerProblem(failure_class, "WORKER_ADAPTER_FAILED", "adapter replay returned a typed failure", retryable=False)
                        result = {"adapter_report": adapter_report, "request_local_state_retained": False}
        except WorkerProblem as exc:
            problem = exc
        except Exception as exc:
            problem = WorkerProblem("internal_error", "WORKER_INTERNAL_ERROR", f"worker invocation failed: {type(exc).__name__}", retryable=False)
        finally:
            with self.lock:
                active["process"] = None
                self.active = None
                if self.shutdown_requested:
                    self.state = "stopped"
                    self.stop_requested = True
                elif self.draining:
                    self.state = "draining"
                else:
                    self.state = "ready"
            if problem is None:
                self.response(request, status, result=result)
            else:
                self.response(request, "cancelled" if problem.failure_class == "cancelled" else "failed", result=result, problem=problem)
            self.event(request["request_id"], "invoke", phase, "WORKER_REQUEST_" + phase.upper(), {"request_local_state_retained": False})

    def handle(self, raw: Any) -> None:
        request: dict[str, Any] | None = raw if isinstance(raw, dict) else None
        try:
            request = self.validate_envelope(raw)
            operation = request["operation"]
            if operation in ("handshake", "cancel", "reset", "drain", "health", "shutdown"):
                self.handle_control(request)
            elif operation in ("load", "capabilities"):
                self.handle_adapter_metadata(request)
            elif operation == "invoke":
                self.start_invoke(request)
            else:
                raise WorkerProblem("protocol_mismatch", "WORKER_REQUEST_INVALID", "unsupported worker operation")
        except WorkerProblem as exc:
            self.fail(request, exc)
        except Exception as exc:
            self.fail(request, WorkerProblem("internal_error", "WORKER_INTERNAL_ERROR", f"worker request failed: {type(exc).__name__}"))

    def run(self) -> int:
        self.event(None, "session", "ready", "WORKER_READY", {"max_requests": self.max_requests})
        last_activity = time.monotonic()
        while not self.stop_requested:
            timeout = 0.1
            if self.idle_timeout_ms > 0 and self.active is None:
                elapsed = (time.monotonic() - last_activity) * 1000
                remaining = self.idle_timeout_ms - elapsed
                if remaining <= 0:
                    with self.lock:
                        self.state = "stopped"
                        self.stop_requested = True
                    self.event(None, "session", "stopped", "WORKER_IDLE_TIMEOUT", {"idle_timeout_ms": self.idle_timeout_ms})
                    break
                timeout = min(timeout, remaining / 1000)
            ready, _, _ = select.select([INPUT], [], [], timeout)
            if not ready:
                continue
            raw_line = INPUT.readline(MAX_LINE_BYTES + 2)
            if raw_line == b"":
                with self.lock:
                    if self.active is None:
                        self.state = "stopped"
                        self.stop_requested = True
                    else:
                        self.shutdown_requested = True
                        self.state = "shutting_down"
                continue
            last_activity = time.monotonic()
            if len(raw_line) > MAX_LINE_BYTES or not raw_line.endswith(b"\n"):
                self.fail(None, WorkerProblem("protocol_mismatch", "WORKER_LINE_LIMIT", "request line is incomplete or exceeds the byte limit"))
                continue
            try:
                value = json.loads(raw_line.decode("utf-8"))
            except (UnicodeDecodeError, json.JSONDecodeError):
                self.fail(None, WorkerProblem("protocol_mismatch", "WORKER_JSON_INVALID", "request line is not valid UTF-8 JSON"))
                continue
            self.handle(value)
        if self.state != "stopped":
            with self.lock:
                self.state = "stopped"
        self.event(None, "session", "stopped", "WORKER_STOPPED", {"request_count": self.request_count})
        return 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(prog="sley worker")
    parser.add_argument("--repo-root", required=True)
    subparsers = parser.add_subparsers(dest="command", required=True)
    start = subparsers.add_parser("start")
    start.add_argument("--json", action="store_true")
    start.add_argument("--max-requests", type=int, default=128)
    start.add_argument("--idle-timeout-ms", type=int, default=300000)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = Path(args.repo_root).resolve(strict=True)
    if args.max_requests < 1 or args.max_requests > 1_000_000:
        raise SystemExit("--max-requests must be between 1 and 1000000")
    if args.idle_timeout_ms < 0 or args.idle_timeout_ms > 86_400_000:
        raise SystemExit("--idle-timeout-ms must be between 0 and 86400000")
    worker = Worker(root, args.max_requests, args.idle_timeout_ms)
    return worker.run()


raise SystemExit(main())
PY
