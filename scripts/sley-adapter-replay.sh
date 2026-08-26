#!/usr/bin/env bash
set -euo pipefail

python3 - "$@" <<'PY'
"""Bounded deterministic adapter record/replay for the Sley stage-1 host."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator


ZERO_DIGEST = "sha256:" + ("0" * 64)
MAX_MANIFEST_BYTES = 1_048_576
MAX_RECORD_BYTES = 2_097_152


def required_env(name: str) -> str:
    value = os.environ.get(name)
    if value is None or value == "":
        raise SystemExit(f"missing required adapter source value: {name}")
    return value


MANIFEST_SCHEMA = required_env("SLEY_ADAPTER_MANIFEST_SCHEMA")
REPLAY_SCHEMA = required_env("SLEY_ADAPTER_REPLAY_SCHEMA")
REPORT_SCHEMA = required_env("SLEY_ADAPTER_REPORT_SCHEMA")
LIFECYCLE_STATES = json.loads(required_env("SLEY_ADAPTER_LIFECYCLE_STATES"))
EVENT_PHASES = json.loads(required_env("SLEY_ADAPTER_REPLAY_EVENT_PHASES"))
FAILURE_CODES = set(json.loads(required_env("SLEY_ADAPTER_FAILURE_CODES")))
SEED_FAMILIES = json.loads(required_env("SLEY_ADAPTER_SEED_FAMILIES"))
SUPPORTED_EFFECTS = set(json.loads(required_env("SLEY_ADAPTER_SUPPORTED_EFFECTS")))
AUTHORITY_MODE = required_env("SLEY_ADAPTER_AUTHORITY_MODE")
REPLAY_POLICY = required_env("SLEY_ADAPTER_REPLAY_POLICY")
ADAPTER_KIND = required_env("SLEY_ADAPTER_KIND")
LIVE_EXECUTION = required_env("SLEY_ADAPTER_LIVE_EXECUTION") == "true"


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def digest(value: Any) -> str:
    return "sha256:" + hashlib.sha256(canonical_bytes(value)).hexdigest()


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
    files = [path] if path.is_file() else sorted(item for item in path.rglob("*.sley") if item.is_file())
    step_count = 0
    task_names: set[str] = set()
    edges: dict[str, set[str]] = {}
    current: str | None = None
    depth = 0
    import re

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


def identity_digest(value: dict[str, Any], field: str = "digest") -> str:
    projected = copy.deepcopy(value)
    projected[field] = ZERO_DIGEST
    return digest(projected)


def manifest_identity_digest(value: dict[str, Any]) -> str:
    projected = copy.deepcopy(value)
    projected["manifest_digest"] = ZERO_DIGEST
    return digest(projected)


def record_identity_digest(value: dict[str, Any]) -> str:
    projected = copy.deepcopy(value)
    projected["record_digest"] = ZERO_DIGEST
    return digest(projected)


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


def reject_nested_symlinks(path: Path) -> None:
    if not path.is_dir():
        return
    for directory, names, files in os.walk(path, followlinks=False):
        parent = Path(directory)
        for name in names + files:
            candidate = parent / name
            if candidate.is_symlink():
                raise ValueError(f"directory target contains a symlink: {candidate.relative_to(path)}")


def issue(code: str, message: str, path: str | None = None) -> dict[str, Any]:
    if code not in FAILURE_CODES:
        code = "ADAPTER_MANIFEST_INVALID"
    value: dict[str, Any] = {"code": code, "message": message}
    if path is not None:
        value["path"] = path
    return value


def base_report(manifest_path: str) -> dict[str, Any]:
    return {
        "schema": REPORT_SCHEMA,
        "status": "failed",
        "manifest": {"path": manifest_path, "schema": None, "id": None, "digest": None},
        "adapter": {"id": None, "version": None, "digest": None, "kind": None, "live_execution": None},
        "runtime": {"id": None, "version": None, "digest": None},
        "authority": {
            "mode": None,
            "bound": False,
            "capability_count": 0,
            "mutation_authority": None,
        },
        "replay": {
            "policy": None,
            "key": None,
            "expected_record_digest": None,
            "actual_record_digest": None,
            "matched": False,
        },
        "record": None,
        "summary": {
            "lifecycle": ["opened"],
            "event_count": 0,
            "seed_count": 0,
            "observed_call_count": 0,
            "diagnostic_count": 0,
        },
        "isolation": {
            "class": "deterministic_seed_replay",
            "os_isolation_enforced": False,
            "persistent_worker": False,
            "owner": "S12-502",
        },
        "issues": [],
    }


def finish(report: dict[str, Any], status: str, problem: dict[str, Any] | None = None) -> int:
    report["status"] = status
    if problem is not None:
        report["issues"].append(problem)
    terminal = "completed" if status == "passed" else status
    if terminal not in LIFECYCLE_STATES:
        terminal = "failed"
    if report["summary"]["lifecycle"][-1] != terminal:
        report["summary"]["lifecycle"].append(terminal)
    print(json.dumps(report, indent=2, sort_keys=False))
    return 0 if status == "passed" else 1


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(add_help=True)
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--runtime-root")
    parser.add_argument("--logical-target")
    parser.add_argument("--json", action="store_true")
    parser.add_argument("--record")
    parser.add_argument("manifest")
    return parser.parse_args()


def load_json(path: Path, byte_limit: int) -> Any:
    size = path.stat().st_size
    if size > byte_limit:
        raise ValueError(f"JSON input exceeds {byte_limit} bytes")
    return json.loads(path.read_text(encoding="utf-8"))


def replace_path_prefix(value: Any, physical: str, logical: str) -> Any:
    if isinstance(value, dict):
        return {key: replace_path_prefix(item, physical, logical) for key, item in value.items()}
    if isinstance(value, list):
        return [replace_path_prefix(item, physical, logical) for item in value]
    if isinstance(value, str) and (value == physical or value.startswith(physical + os.sep)):
        return logical + value[len(physical):]
    return value


def schema_errors(schema: dict[str, Any], value: Any) -> list[str]:
    validator = Draft202012Validator(schema)
    errors = sorted(validator.iter_errors(value), key=lambda err: list(err.absolute_path))
    rendered = []
    for error in errors[:8]:
        path = "/" + "/".join(str(part) for part in error.absolute_path)
        rendered.append(f"{path}: {error.message}")
    return rendered


def seed_resource(family: str, seed: dict[str, Any]) -> str:
    fields = {
        "files": "path",
        "db_tables": "table",
        "secrets": "name",
        "http_text": "url",
        "shell_outputs": "command",
        "model_outputs": "prompt",
        "deploy_results": "target",
        "spend_results": "request",
    }
    return str(seed[fields[family]])


def family_for_effect(effect: str) -> str | None:
    return {
        "FileRead": "files",
        "DatabaseRead": "db_tables",
        "DatabaseWrite": None,
        "SecretRead": "secrets",
        "Network": "http_text",
        "Shell": "shell_outputs",
        "ModelCall": "model_outputs",
        "Deploy": "deploy_results",
        "Spend": "spend_results",
    }.get(effect)


def event(sequence: int, phase: str, adapter_id: str, effect: str, code: str, **fields: Any) -> dict[str, Any]:
    value = {
        "sequence": sequence,
        "phase": phase,
        "adapter_id": adapter_id,
        "effect": effect,
        "code": code,
    }
    value.update({key: item for key, item in fields.items() if item is not None})
    return value


def seed_cli_args(seeds: dict[str, list[dict[str, Any]]], workdir: Path) -> list[str]:
    args: list[str] = []
    for index, seed in enumerate(seeds["db_tables"]):
        seed_path = workdir / "db" / f"{index}.json"
        seed_path.parent.mkdir(parents=True, exist_ok=True)
        seed_path.write_text(json.dumps(seed["rows"], sort_keys=True), encoding="utf-8")
        args.extend(["--db-table", f'{seed["table"]}={seed_path}'])
    for family, flag, key in (
        ("secrets", "--secret", "name"),
        ("http_text", "--http-text", "url"),
        ("shell_outputs", "--shell-output", "command"),
        ("model_outputs", "--model-output", "prompt"),
        ("deploy_results", "--deploy-result", "target"),
        ("spend_results", "--spend-result", "request"),
    ):
        for seed in seeds[family]:
            # Replay executes a deterministic redacted projection. Raw seeded
            # text never enters a child process argument or public report.
            args.extend([flag, str(seed[key]), digest(seed["text"])])
    return args


def materialize_file_seeds(seeds: dict[str, list[dict[str, Any]]], workdir: Path) -> None:
    for seed in seeds["files"]:
        relative = Path(seed["path"])
        if relative.is_absolute() or ".." in relative.parts:
            raise ValueError("file seed paths must be confined relative paths")
        output = workdir / relative
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(seed["text"], encoding="utf-8")


def build_record(
    manifest: dict[str, Any],
    target_digest: str,
    verify: dict[str, Any] | None,
    events: list[dict[str, Any]],
    status: str,
    code: str,
    observed_steps: int,
    observed_call_depth: int,
) -> dict[str, Any]:
    seeds = manifest["seeds"]
    capabilities_digest = digest(manifest["capabilities"])
    seed_projection = {family: seeds[family] for family in SEED_FAMILIES}
    output_digest = digest(verify) if verify is not None else None
    diagnostics = []
    if verify is not None:
        diagnostics = [
            item.get("id")
            for item in (verify.get("diagnostics", []) + verify.get("runtime", {}).get("diagnostics", []))
            if item.get("id")
        ]
    record = {
        "schema": REPLAY_SCHEMA,
        "record_digest": ZERO_DIGEST,
        "status": status,
        "manifest_id": manifest["manifest_id"],
        "adapter": {key: manifest["adapter"][key] for key in ("id", "version", "digest")},
        "runtime": {key: manifest["runtime"][key] for key in ("id", "version", "digest")},
        "replay_key": manifest["replay"]["replay_key"],
        "authority": {
            "mode": manifest["authority"]["mode"],
            "capabilities_digest": capabilities_digest,
            "mutation_authority": manifest["authority"]["mutation_authority"],
        },
        "redaction": {
            "policy": manifest["adapter"]["redaction_policy"],
            "seed_families": [
                {"family": family, "count": len(seeds[family]), "values_included": False}
                for family in SEED_FAMILIES
            ],
            "public_projection": "digests_and_counts_only",
            "private_projection": "manifest_owned_seed_values",
        },
        "request": {
            "target_digest": target_digest,
            "capabilities_digest": capabilities_digest,
            "seeds_digest": digest(seed_projection),
        },
        "result": {
            "status": verify.get("status", status) if verify is not None else status,
            "output_digest": output_digest,
            "diagnostic_codes": diagnostics,
        },
        "measurements": {
            "limits": manifest["bounds"],
            "observed_call_count": int((verify or {}).get("summary", {}).get("call_count", 0)),
            "observed_step_count": observed_steps,
            "observed_call_depth": observed_call_depth,
            "output_bytes": len(canonical_bytes(verify)) if verify is not None else 0,
        },
        "events": events,
        "terminal_code": code,
    }
    record["record_digest"] = record_identity_digest(record)
    return record


def main() -> int:
    args = parse_args()
    root = Path(args.repo_root).resolve(strict=True)
    runtime_root = Path(args.runtime_root or args.repo_root).resolve(strict=True)
    report = base_report(args.manifest)

    try:
        raw_manifest_path = Path(args.manifest)
        if not raw_manifest_path.is_absolute():
            raw_manifest_path = Path.cwd() / raw_manifest_path
        manifest_path = confined_regular(raw_manifest_path, root)
        manifest = load_json(manifest_path, MAX_MANIFEST_BYTES)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        return finish(report, "failed", issue("ADAPTER_MANIFEST_INVALID", str(exc), "/manifest"))

    if isinstance(manifest, dict):
        report["manifest"].update({
            "schema": manifest.get("schema"),
            "id": manifest.get("manifest_id"),
            "digest": manifest.get("manifest_digest"),
        })
        for key in ("adapter", "runtime"):
            if isinstance(manifest.get(key), dict):
                report[key].update({field: manifest[key].get(field) for field in report[key]})
        if isinstance(manifest.get("authority"), dict):
            report["authority"].update({
                "mode": manifest["authority"].get("mode"),
                "capability_count": len(manifest.get("capabilities", [])),
                "mutation_authority": manifest["authority"].get("mutation_authority"),
            })
        if isinstance(manifest.get("replay"), dict):
            report["replay"].update({
                "policy": manifest["replay"].get("policy"),
                "key": manifest["replay"].get("replay_key"),
                "expected_record_digest": manifest["replay"].get("record_digest"),
            })

    live_requested = bool(
        isinstance(manifest, dict)
        and (
            manifest.get("adapter", {}).get("live_execution") is True
            or manifest.get("authority", {}).get("external_provider_calls") is True
            or manifest.get("authority", {}).get("mutation_authority", {}).get("external") is True
            or manifest.get("authority", {}).get("mutation_authority", {}).get("provider") is True
            or manifest.get("authority", {}).get("mutation_authority", {}).get("spend") is True
            or manifest.get("authority", {}).get("mutation_authority", {}).get("deploy") is True
        )
    )
    if live_requested:
        report["summary"]["lifecycle"].append("blocked")
        return finish(report, "blocked", issue(
            "ADAPTER_LIVE_PROVIDER_DENIED",
            "S12-501 accepts deterministic local replay only; live provider, deploy, spend, and external mutation are denied",
            "/adapter/live_execution",
        ))

    try:
        schema = load_json(runtime_root / "docs/schemas/sley.adapter.manifest.v0.schema.json", MAX_MANIFEST_BYTES)
        errors = schema_errors(schema, manifest)
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        errors = [str(exc)]
    if errors:
        report["summary"]["lifecycle"].append("failed")
        return finish(report, "failed", issue("ADAPTER_MANIFEST_INVALID", "; ".join(errors), "/"))

    report["summary"]["lifecycle"].append("manifest_validated")
    if manifest["schema"] != MANIFEST_SCHEMA:
        return finish(report, "failed", issue("ADAPTER_MANIFEST_INVALID", "manifest schema identity does not match the Sley-owned adapter contract", "/schema"))
    if manifest_identity_digest(manifest) != manifest["manifest_digest"]:
        return finish(report, "failed", issue("ADAPTER_MANIFEST_INVALID", "manifest digest does not match canonical manifest content", "/manifest_digest"))
    if identity_digest(manifest["adapter"]) != manifest["adapter"]["digest"]:
        return finish(report, "failed", issue("ADAPTER_MANIFEST_INVALID", "adapter digest does not match its descriptor", "/adapter/digest"))
    if identity_digest(manifest["runtime"]) != manifest["runtime"]["digest"]:
        return finish(report, "failed", issue("ADAPTER_MANIFEST_INVALID", "runtime digest does not match its descriptor", "/runtime/digest"))
    if manifest["adapter"]["kind"] != ADAPTER_KIND or manifest["adapter"]["live_execution"] != LIVE_EXECUTION:
        return finish(report, "blocked", issue("ADAPTER_LIVE_PROVIDER_DENIED", "adapter kind is not the local deterministic seed adapter", "/adapter"))
    if manifest["authority"]["mode"] != AUTHORITY_MODE or manifest["replay"]["policy"] != REPLAY_POLICY:
        return finish(report, "blocked", issue("ADAPTER_AUTHORITY_DENIED", "authority mode or replay policy is not the Sley-owned local boundary", "/authority"))

    mutations = manifest["authority"]["mutation_authority"]
    forbidden_mutation = any(mutations[key] for key in ("repository", "index", "trace", "external", "provider", "deploy", "spend"))
    if forbidden_mutation or not mutations["ephemeral_runtime_state"]:
        return finish(report, "blocked", issue("ADAPTER_AUTHORITY_DENIED", "manifest requests authority outside ephemeral local runtime state", "/authority/mutation_authority"))

    effects = set(manifest["adapter"]["supported_effects"])
    if not effects.issubset(SUPPORTED_EFFECTS):
        return finish(report, "blocked", issue("ADAPTER_AUTHORITY_DENIED", "adapter declares an unsupported effect", "/adapter/supported_effects"))
    for capability in manifest["capabilities"]:
        if capability["effect"] not in effects:
            return finish(report, "blocked", issue("ADAPTER_AUTHORITY_DENIED", "capability effect is not declared by the adapter", "/capabilities"))
        if capability["operation"] not in manifest["adapter"]["supported_operations"]:
            return finish(report, "blocked", issue("ADAPTER_AUTHORITY_DENIED", "capability operation is not declared by the adapter", "/capabilities"))

    report["authority"]["bound"] = True
    report["summary"]["lifecycle"].append("authority_bound")
    seeds = manifest["seeds"]
    report["summary"]["seed_count"] = sum(len(seeds[family]) for family in SEED_FAMILIES)
    for capability in manifest["capabilities"]:
        family = family_for_effect(capability["effect"])
        if family is None:
            continue
        family_seeds = seeds[family]
        if not family_seeds:
            return finish(report, "blocked", issue("ADAPTER_SEED_NOT_FOUND", f'{family} has no seed for {capability["effect"]}', f"/seeds/{family}"))
        resources = {seed_resource(family, seed) for seed in family_seeds}
        if capability["scope"] not in resources:
            return finish(report, "blocked", issue("ADAPTER_SCOPE_DENIED", f'capability scope {capability["scope"]!r} is not backed by an exact deterministic seed', "/capabilities"))

    target_raw = Path(manifest["target"])
    if not target_raw.is_absolute():
        target_raw = manifest_path.parent / target_raw
    try:
        target = confined_regular(target_raw, root, directory_ok=True)
        reject_nested_symlinks(target)
    except (OSError, ValueError) as exc:
        return finish(report, "blocked", issue("ADAPTER_SCOPE_DENIED", str(exc), "/target"))
    logical_target = target.relative_to(root).as_posix()
    if args.logical_target:
        try:
            requested_logical_target = Path(args.logical_target)
            if not requested_logical_target.is_absolute():
                requested_logical_target = runtime_root / requested_logical_target
            logical_target = confined_regular(
                requested_logical_target, runtime_root, directory_ok=True
            ).relative_to(runtime_root).as_posix()
        except (OSError, ValueError) as exc:
            return finish(report, "blocked", issue("ADAPTER_SCOPE_DENIED", str(exc), "/logical_target"))
    target_size = sum(path.stat().st_size for path in ([target] if target.is_file() else target.rglob("*.sley")) if path.is_file())
    input_size = target_size + len(canonical_bytes(manifest))
    if input_size > manifest["bounds"]["max_input_bytes"]:
        return finish(report, "blocked", issue("ADAPTER_BUDGET_EXCEEDED", "manifest and target source exceed max_input_bytes", "/bounds/max_input_bytes"))
    observed_steps, observed_call_depth, call_cycle = static_source_metrics(target)
    if observed_steps > manifest["bounds"]["max_steps"]:
        return finish(report, "blocked", issue("ADAPTER_BUDGET_EXCEEDED", "static source step count exceeds max_steps", "/bounds/max_steps"))
    if call_cycle or observed_call_depth > manifest["bounds"]["max_call_depth"]:
        return finish(report, "blocked", issue("ADAPTER_BUDGET_EXCEEDED", "static call depth exceeds max_call_depth or contains a cycle", "/bounds/max_call_depth"))

    adapter_id = manifest["adapter"]["id"]
    effect = ",".join(capability["effect"] for capability in manifest["capabilities"]) or "Pure"
    common = {"input_digest": digest({"target": file_digest(target), "replay_key": manifest["replay"]["replay_key"]})}
    events = [
        event(0, "opened", adapter_id, effect, "ADAPTER_REPLAY_OPENED"),
        event(1, "manifest_validated", adapter_id, effect, "ADAPTER_MANIFEST_VALIDATED"),
        event(2, "authority_bound", adapter_id, effect, "ADAPTER_AUTHORITY_BOUND"),
        event(3, "seed_bound", adapter_id, effect, "ADAPTER_SEED_BOUND", input_digest=digest(seeds)),
        event(4, "runtime_started", adapter_id, effect, "ADAPTER_RUNTIME_STARTED", **common),
    ]

    if manifest.get("cancellation", {}).get("requested_at_phase") == "before_adapter_call":
        events.append(event(5, "cancellation_observed", adapter_id, effect, "ADAPTER_CANCELLED"))
        record = build_record(manifest, file_digest(target), None, events, "cancelled", "ADAPTER_CANCELLED", observed_steps, observed_call_depth)
        report["record"] = record
        report["replay"]["actual_record_digest"] = record["record_digest"]
        report["summary"]["event_count"] = len(events)
        return finish(report, "cancelled", issue("ADAPTER_CANCELLED", "cancellation was observed before the adapter call", "/cancellation"))

    verify_args = [str(runtime_root / "bin/sley"), "verify", "--json", "--deny-warnings"]
    for capability in manifest["capabilities"]:
        verify_args.extend(["--cap", f'{capability["effect"]}={capability["scope"]}'])

    verify: dict[str, Any]
    elapsed_ms = 0
    try:
        with tempfile.TemporaryDirectory(prefix="sley-adapter-") as temporary:
            workdir = Path(temporary)
            materialize_file_seeds(seeds, workdir)
            verify_args.extend(seed_cli_args(seeds, workdir))
            verify_args.append(str(target))
            import time

            started = time.monotonic_ns()
            completed = subprocess.run(
                verify_args,
                cwd=workdir,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                timeout=manifest["bounds"]["max_wall_ms"] / 1000,
                check=False,
                env={**os.environ, "SLEY_MAX_SOURCE_BYTES": str(manifest["bounds"]["max_input_bytes"]), "SLEY_MAX_JSON_BYTES": str(manifest["bounds"]["max_output_bytes"])},
            )
            elapsed_ms = max(1, (time.monotonic_ns() - started) // 1_000_000)
        if len(completed.stdout.encode("utf-8")) > manifest["bounds"]["max_output_bytes"]:
            return finish(report, "blocked", issue("ADAPTER_BUDGET_EXCEEDED", "runtime output exceeds max_output_bytes", "/bounds/max_output_bytes"))
        verify = json.loads(completed.stdout)
        verify = replace_path_prefix(verify, str(target), logical_target)
    except subprocess.TimeoutExpired:
        return finish(report, "blocked", issue("ADAPTER_BUDGET_EXCEEDED", "runtime exceeded max_wall_ms", "/bounds/max_wall_ms"))
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        return finish(report, "failed", issue("ADAPTER_MANIFEST_INVALID", f"runtime adapter failed to produce a JSON report: {exc}", "/runtime"))

    events.append(event(5, "adapter_called", adapter_id, effect, "ADAPTER_CALLED", **common))
    events.append(event(6, "adapter_returned", adapter_id, effect, "ADAPTER_RETURNED", output_digest=digest(verify)))
    events.append(event(7, "runtime_finished", adapter_id, effect, "ADAPTER_RUNTIME_FINISHED", output_digest=digest(verify)))
    if manifest.get("cancellation", {}).get("requested_at_phase") == "after_adapter_call":
        events.append(event(8, "cancellation_observed", adapter_id, effect, "ADAPTER_CANCELLED"))
        record = build_record(manifest, file_digest(target), verify, events, "cancelled", "ADAPTER_CANCELLED", observed_steps, observed_call_depth)
        report["record"] = record
        report["replay"]["actual_record_digest"] = record["record_digest"]
        report["summary"]["event_count"] = len(events)
        report["summary"]["observed_call_count"] = int(verify.get("summary", {}).get("call_count", 0))
        return finish(report, "cancelled", issue("ADAPTER_CANCELLED", "cancellation was observed after the adapter call", "/cancellation"))

    diagnostic_codes = [
        item.get("id")
        for item in (verify.get("diagnostics", []) + verify.get("runtime", {}).get("diagnostics", []))
        if item.get("id")
    ]
    report["summary"]["diagnostic_count"] = len(diagnostic_codes)
    observed_calls = int(verify.get("summary", {}).get("call_count", 0))
    report["summary"]["observed_call_count"] = observed_calls
    if observed_calls > manifest["bounds"]["max_calls"]:
        return finish(report, "blocked", issue("ADAPTER_BUDGET_EXCEEDED", "observed structural call count exceeds max_calls", "/bounds/max_calls"))
    if verify.get("status") != "passed":
        code = "ADAPTER_SCOPE_DENIED" if "RUNTIME_CAPABILITY_SCOPE_DENIED" in diagnostic_codes else "ADAPTER_AUTHORITY_DENIED"
        events.append(event(8, "replay_verified", adapter_id, effect, code))
        record = build_record(manifest, file_digest(target), verify, events, "blocked", code, observed_steps, observed_call_depth)
        report["record"] = record
        report["replay"]["actual_record_digest"] = record["record_digest"]
        report["summary"]["event_count"] = len(events)
        return finish(report, "blocked", issue(code, "the seeded runtime rejected the declared adapter authority", "/capabilities"))

    events.append(event(8, "replay_verified", adapter_id, effect, "ADAPTER_REPLAY_VERIFIED"))
    record = build_record(manifest, file_digest(target), verify, events, "passed", "ADAPTER_REPLAY_VERIFIED", observed_steps, observed_call_depth)
    expected_digest = manifest["replay"]["record_digest"]
    record_match = record["record_digest"] == expected_digest
    if args.record:
        try:
            raw_record_path = Path(args.record)
            if not raw_record_path.is_absolute():
                raw_record_path = Path.cwd() / raw_record_path
            record_path = confined_regular(raw_record_path, root)
            expected_record = load_json(record_path, MAX_RECORD_BYTES)
            record_schema = load_json(runtime_root / "docs/schemas/sley.adapter.replay.v0.schema.json", MAX_MANIFEST_BYTES)
            record_errors = schema_errors(record_schema, expected_record)
            record_match = record_match and not record_errors and canonical_bytes(expected_record) == canonical_bytes(record)
        except (OSError, ValueError, json.JSONDecodeError):
            record_match = False

    if not record_match:
        observed_record_digest = record["record_digest"]
        events[-1] = event(8, "replay_verified", adapter_id, effect, "ADAPTER_REPLAY_MISMATCH")
        record = build_record(manifest, file_digest(target), verify, events, "failed", "ADAPTER_REPLAY_MISMATCH", observed_steps, observed_call_depth)
        report["record"] = record
        report["replay"]["actual_record_digest"] = observed_record_digest
        report["summary"]["event_count"] = len(events)
        return finish(report, "failed", issue("ADAPTER_REPLAY_MISMATCH", "observed replay record does not match the pinned deterministic record", "/replay/record_digest"))

    report["record"] = record
    report["replay"]["actual_record_digest"] = record["record_digest"]
    report["replay"]["matched"] = True
    report["summary"]["event_count"] = len(events)
    report["summary"]["lifecycle"].append("replaying")
    report["runtime"].update({"status": "passed", "elapsed_ms": elapsed_ms})
    return finish(report, "passed")


if __name__ == "__main__":
    raise SystemExit(main())
PY
