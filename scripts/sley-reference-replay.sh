#!/usr/bin/env bash
set -euo pipefail

python3 - "$@" <<'PY'
"""Pinned, offline Siglum numerology replay through the bounded Sley host."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import select
import signal
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator


class ReplayError(RuntimeError):
    pass


REQUIRED_ARTIFACTS = {
    "candidate_manifest",
    "candidate_main",
    "candidate_intrinsics",
    "corpus_hashes",
    "corpus_manifest",
    "normative_spec",
    "oracle_boundary",
    "oracle_engine_adapter",
    "parity_harness",
    "prior_parity_report",
    "ruleset",
}
DEFAULT_MEMORY_BYTES = 34_359_738_368
DEFAULT_OVERALL_WALL_MS = 1_800_000
DEFAULT_PROBE_WALL_MS = 300_000
MAX_CAPTURE_BYTES = 8 * 1_024 * 1_024


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def replay_environment() -> dict[str, str]:
    return {
        "PATH": os.environ.get("PATH", "/usr/bin:/bin"),
        "HOME": os.environ.get("HOME", "/tmp"),
        "TMPDIR": os.environ.get("TMPDIR", "/tmp"),
        "LANG": "C.UTF-8",
        "LC_ALL": "C.UTF-8",
        "TZ": "America/New_York",
    }


def sha256_bytes(value: bytes) -> str:
    return "sha256:" + hashlib.sha256(value).hexdigest()


def file_digest(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def load_json(path: Path, limit: int = MAX_CAPTURE_BYTES) -> Any:
    if not path.is_file() or path.is_symlink():
        raise ReplayError(f"PIN_PATH_INVALID: expected a regular file: {path}")
    if path.stat().st_size > limit:
        raise ReplayError(f"INPUT_LIMIT_EXCEEDED: {path} exceeds {limit} bytes")
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise ReplayError(f"JSON_INVALID: {path}: {exc}") from exc


def validate(schema_path: Path, value: Any, label: str) -> None:
    schema = load_json(schema_path)
    errors = sorted(Draft202012Validator(schema).iter_errors(value), key=lambda item: list(item.absolute_path))
    if errors:
        details = "; ".join(f"/{'/'.join(map(str, item.absolute_path))}: {item.message}" for item in errors[:8])
        raise ReplayError(f"CONTRACT_INVALID: {label}: {details}")


def confined(root: Path, relative: str, *, directory: bool = False) -> Path:
    candidate = Path(relative)
    if candidate.is_absolute() or ".." in candidate.parts:
        raise ReplayError(f"PIN_PATH_INVALID: path is not confined: {relative}")
    raw = root / candidate
    if raw.is_symlink():
        raise ReplayError(f"PIN_PATH_INVALID: symlink is not accepted: {relative}")
    try:
        resolved = raw.resolve(strict=True)
        resolved.relative_to(root)
    except (OSError, ValueError) as exc:
        raise ReplayError(f"PIN_PATH_INVALID: path is outside the Siglum root: {relative}") from exc
    if directory and not resolved.is_dir():
        raise ReplayError(f"PIN_PATH_INVALID: expected directory: {relative}")
    if not directory and not resolved.is_file():
        raise ReplayError(f"PIN_PATH_INVALID: expected file: {relative}")
    return resolved


def git(root: Path, *args: str) -> str:
    completed = subprocess.run(
        ["git", "-C", str(root), *args],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if completed.returncode != 0:
        raise ReplayError(f"GIT_STATE_INVALID: {' '.join(args)}: {completed.stderr.strip()}")
    return completed.stdout.strip()


def require_clean_tracked(root: Path, label: str) -> None:
    if git(root, "status", "--short", "--untracked-files=no"):
        raise ReplayError(f"SOURCE_DIRTY: {label} has tracked changes")


def verify_corpus(corpus_root: Path, manifest: dict[str, Any], expected_digest: str) -> None:
    artifacts = manifest.get("artifacts")
    if not isinstance(artifacts, dict):
        raise ReplayError("CORPUS_MANIFEST_INVALID: artifacts is missing")
    if artifacts.get("caseCount") != 10_000 or artifacts.get("corpusDigest") != expected_digest:
        raise ReplayError("CORPUS_MANIFEST_INVALID: case count or frozen digest disagrees with pins")
    shards = artifacts.get("shards")
    if not isinstance(shards, list) or len(shards) != 20:
        raise ReplayError("CORPUS_MANIFEST_INVALID: exactly 20 shards are required")
    total_bytes = 0
    total_cases = 0
    for shard in shards:
        if not isinstance(shard, dict):
            raise ReplayError("CORPUS_MANIFEST_INVALID: shard entry is not an object")
        path = confined(corpus_root, str(shard.get("path", "")))
        observed_bytes = path.stat().st_size
        observed_digest = file_digest(path)
        if observed_bytes != shard.get("bytes") or observed_digest != shard.get("digest"):
            raise ReplayError(f"CORPUS_SHARD_MISMATCH: {shard.get('path')}")
        line_count = sum(1 for line in path.open("rb") if line.rstrip(b"\r\n"))
        if line_count != shard.get("caseCount"):
            raise ReplayError(f"CORPUS_SHARD_MISMATCH: line count for {shard.get('path')}")
        total_bytes += observed_bytes
        total_cases += line_count
    if total_bytes != artifacts.get("totalBytes") or total_cases != 10_000:
        raise ReplayError("CORPUS_MANIFEST_INVALID: aggregate bytes or cases disagree with shards")


def verify_prior_report(report: dict[str, Any], expected: dict[str, Any]) -> None:
    runs = report.get("runs")
    if (
        report.get("schemaVersion") != "siglum.sley.numerology.parity-report.v1"
        or report.get("deterministic") is not True
        or not isinstance(runs, list)
        or len(runs) != expected["retained_repeat_count"]
    ):
        raise ReplayError("RETAINED_EVIDENCE_INVALID: prior parity report identity or repeat count is wrong")
    for run in runs:
        if (
            run.get("caseCount") != expected["case_count"]
            or run.get("exactMatchCount") != expected["retained_exact_match_count"]
            or run.get("mismatchCount") != expected["retained_mismatch_count"]
            or run.get("candidateCorpusDigest") != expected["candidate_corpus_digest"]
            or run.get("sourceDigest") != expected["candidate_source_digest"]
            or run.get("runtimeDigest") != expected["retained_runtime_digest"]
        ):
            raise ReplayError("RETAINED_EVIDENCE_INVALID: prior parity run disagrees with pins")


def verify_inputs(repo_root: Path, siglum_root: Path, pins_path: Path, pins: dict[str, Any]) -> dict[str, Path]:
    validate(repo_root / "docs/schemas/sley.operational.reference_pins.v1.schema.json", pins, "reference pins")
    if git(siglum_root, "rev-parse", "HEAD") != pins["siglum_commit"]:
        raise ReplayError("SIGLUM_COMMIT_MISMATCH: current Siglum commit is not the pinned oracle commit")
    require_clean_tracked(siglum_root, "Siglum")
    artifacts = pins["artifacts"]
    ids = [item["id"] for item in artifacts]
    if len(ids) != len(set(ids)) or set(ids) != REQUIRED_ARTIFACTS:
        raise ReplayError("PIN_SET_INVALID: exact required artifact identities were not supplied")
    resolved: dict[str, Path] = {}
    for item in artifacts:
        path = confined(siglum_root, item["path"])
        if file_digest(path) != item["digest"]:
            raise ReplayError(f"ARTIFACT_DIGEST_MISMATCH: {item['id']}")
        resolved[item["id"]] = path
    corpus_root = confined(siglum_root, pins["paths"]["corpus_root"], directory=True)
    candidate_package = confined(siglum_root, pins["paths"]["candidate_package"], directory=True)
    parity_test = confined(siglum_root, pins["paths"]["parity_test"])
    prior_report_path = confined(siglum_root, pins["paths"]["prior_report"])
    if parity_test != resolved["parity_harness"] or prior_report_path != resolved["prior_parity_report"]:
        raise ReplayError("PIN_PATH_INVALID: path aliases disagree with pinned artifacts")
    corpus_manifest = load_json(resolved["corpus_manifest"])
    verify_corpus(corpus_root, corpus_manifest, pins["expected"]["frozen_corpus_digest"])
    verify_prior_report(load_json(prior_report_path), pins["expected"])
    return {
        **resolved,
        "corpus_root": corpus_root,
        "candidate_package": candidate_package,
        "parity_test": parity_test,
        "prior_report": prior_report_path,
    }


def process_tree_rss(root_pid: int) -> int:
    pending = [root_pid]
    seen: set[int] = set()
    total = 0
    while pending:
        pid = pending.pop()
        if pid in seen:
            continue
        seen.add(pid)
        status_path = Path(f"/proc/{pid}/status")
        children_path = Path(f"/proc/{pid}/task/{pid}/children")
        try:
            for line in status_path.read_text().splitlines():
                if line.startswith("VmRSS:"):
                    total += int(line.split()[1]) * 1024
                    break
            if children_path.exists():
                pending.extend(int(value) for value in children_path.read_text().split())
        except (OSError, ValueError):
            continue
    return total


def machine_request(case: dict[str, Any], index: int) -> dict[str, Any]:
    return {
        "protocolVersion": "sley.machine.invoke.v0",
        "requestId": f"w6-probe-{index}",
        "operation": "invoke",
        "packageId": "siglum.numerology",
        "packageVersion": "siglum.numerology.reference-v1",
        "task": "siglum.numerology.main.compute",
        "input": case["input"],
        "limits": {
            "requestLineBytes": 4096,
            "responseBytes": 262144,
            "batch": 500,
            "evaluationSteps": 250000,
            "callDepth": 128,
            "collectionLength": 4096,
            "timeoutMs": 2000,
        },
    }


def read_line(stream: Any, deadline: float) -> str:
    remaining = deadline - time.monotonic()
    if remaining <= 0:
        raise ReplayError("PROBE_TIMEOUT: persistent host produced no response")
    ready, _, _ = select.select([stream], [], [], remaining)
    if not ready:
        raise ReplayError("PROBE_TIMEOUT: persistent host produced no response")
    line = stream.readline()
    if line == "":
        raise ReplayError("PROBE_FAILED: persistent host closed its output")
    return line


def run_probe(
    repo_root: Path,
    candidate_package: Path,
    corpus_root: Path,
    request_count: int,
    memory_bytes: int,
    wall_ms: int,
) -> dict[str, Any]:
    first_shard = sorted(corpus_root.glob("cases-*.jsonl"))[0]
    with first_shard.open(encoding="utf-8") as handle:
        case = json.loads(handle.readline())
    command = [
        "/usr/bin/prlimit",
        f"--as={memory_bytes}:{memory_bytes}",
        "--nofile=128:128",
        "--",
        str(repo_root / "bin/sley"),
        "machine",
        "--source", str(candidate_package),
        "--package-id", "siglum.numerology",
        "--package-version", "siglum.numerology.reference-v1",
    ]
    process = subprocess.Popen(
        command,
        cwd=repo_root,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
        start_new_session=True,
        env=replay_environment(),
    )
    assert process.stdin is not None and process.stdout is not None and process.stderr is not None
    peak_rss = [0]
    sampling = threading.Event()
    sampling.set()

    def sample() -> None:
        while sampling.is_set():
            peak_rss[0] = max(peak_rss[0], process_tree_rss(process.pid))
            time.sleep(0.02)

    sampler = threading.Thread(target=sample, daemon=True)
    sampler.start()
    responses: list[dict[str, Any]] = []
    latencies: list[int] = []
    deadline = time.monotonic() + (wall_ms / 1000)
    try:
        for index in range(request_count):
            request = machine_request(case, index)
            started = time.monotonic_ns()
            process.stdin.write(json.dumps(request, sort_keys=True, separators=(",", ":")) + "\n")
            process.stdin.flush()
            response = json.loads(read_line(process.stdout, deadline))
            latencies.append(max(0, (time.monotonic_ns() - started) // 1_000_000))
            if response.get("status") != "ok" or response.get("requestId") != request["requestId"]:
                raise ReplayError(f"PROBE_FAILED: {response.get('error', response)}")
            responses.append(response)
        process.stdin.close()
        remaining = max(0.1, deadline - time.monotonic())
        process.wait(timeout=remaining)
        if process.returncode != 0:
            raise ReplayError(f"PROBE_FAILED: machine exited {process.returncode}: {process.stderr.read()[:2048]}")
    except Exception:
        if process.poll() is None:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait()
        raise
    finally:
        sampling.clear()
        sampler.join(timeout=1)
        peak_rss[0] = max(peak_rss[0], process_tree_rss(process.pid))
    source_digests = {item["source"]["digest"] for item in responses}
    runtime_digests = {item["runtime"]["digest"] for item in responses}
    result_digests = [item["digests"]["result"] for item in responses]
    if len(source_digests) != 1 or len(runtime_digests) != 1 or len(set(result_digests)) != 1:
        raise ReplayError("PROBE_NONDETERMINISTIC: persistent responses disagree")
    return {
        "schema": "sley.operational.reference_probe.v1",
        "request_count": request_count,
        "cold_first_response_ms": latencies[0],
        "warm_response_ms": latencies[1:],
        "peak_process_tree_rss_bytes": peak_rss[0],
        "result_digests": result_digests,
        "source_digest": next(iter(source_digests)),
        "runtime_digest": next(iter(runtime_digests)),
        "deterministic": True,
    }


def run_parity(
    repo_root: Path,
    siglum_root: Path,
    result_path: Path,
    concurrency: int,
    memory_bytes: int,
    wall_ms: int,
) -> tuple[dict[str, Any], dict[str, int]]:
    vitest = siglum_root / "node_modules/.bin/vitest"
    if not vitest.is_file() or not os.access(vitest, os.X_OK):
        raise ReplayError("DEPENDENCY_MISSING: Siglum's local Vitest executable is unavailable")
    cpu_seconds = max(1, (wall_ms // 1000) + 10)
    command = [
        "/usr/bin/prlimit",
        f"--as={memory_bytes}:{memory_bytes}",
        f"--cpu={cpu_seconds}:{cpu_seconds}",
        "--nofile=128:128",
        "--",
        str(vitest),
        "run",
        "--config", "vitest.corpus.config.ts",
        "tests/corpus/sley-numerology-parity.test.ts",
    ]
    env = {
        **replay_environment(),
        "CI": "1",
        "NO_COLOR": "1",
        "SIGLUM_SLEY_FULL_PARITY": "1",
        "SIGLUM_SLEY_DETERMINISM": "1",
        "SIGLUM_SLEY_PARITY_RESULT_PATH": str(result_path),
        "SIGLUM_SLEY_PARITY_CONCURRENCY": str(concurrency),
        "SIGLUM_SLEY_ROOT": str(repo_root),
    }
    started = time.monotonic_ns()
    process = subprocess.Popen(
        command,
        cwd=siglum_root,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        start_new_session=True,
    )
    peak_rss = [0]
    sampling = threading.Event()
    sampling.set()

    def sample() -> None:
        while sampling.is_set():
            peak_rss[0] = max(peak_rss[0], process_tree_rss(process.pid))
            time.sleep(0.02)

    sampler = threading.Thread(target=sample, daemon=True)
    sampler.start()
    try:
        stdout, stderr = process.communicate(timeout=wall_ms / 1000)
    except subprocess.TimeoutExpired as exc:
        os.killpg(process.pid, signal.SIGKILL)
        process.communicate()
        raise ReplayError(f"PARITY_TIMEOUT: controller exceeded {wall_ms} ms") from exc
    finally:
        sampling.clear()
        sampler.join(timeout=1)
        peak_rss[0] = max(peak_rss[0], process_tree_rss(process.pid))
    captured = stdout + stderr
    if len(captured) > MAX_CAPTURE_BYTES:
        raise ReplayError("PARITY_OUTPUT_LIMIT_EXCEEDED: parity controller exceeded 8 MiB")
    if process.returncode != 0:
        tail = captured[-4096:].decode("utf-8", errors="replace")
        raise ReplayError(f"PARITY_FAILED: exit {process.returncode}: {tail}")
    return load_json(result_path), {
        "wall_ms": max(1, (time.monotonic_ns() - started) // 1_000_000),
        "peak_process_tree_rss_bytes": peak_rss[0],
    }


def verify_fresh_parity(parity: dict[str, Any], probe: dict[str, Any], expected: dict[str, Any]) -> list[dict[str, Any]]:
    runs = parity.get("runs")
    if (
        parity.get("schemaVersion") != "siglum.sley.numerology.parity-report.v1"
        or parity.get("deterministic") is not True
        or not isinstance(runs, list)
        or len(runs) != 2
    ):
        raise ReplayError("FRESH_PARITY_INVALID: two deterministic runs are required")
    stable_fields = [
        "caseCount", "exactMatchCount", "mismatchCount", "mismatchReportDigest",
        "candidateCorpusDigest", "shardDigests", "sourceDigest", "runtimeDigest",
        "maximumSteps", "maximumCalls", "maximumCallDepth", "maximumResponseBytes",
    ]
    for run in runs:
        if (
            run.get("caseCount") != expected["case_count"]
            or run.get("exactMatchCount") != expected["case_count"]
            or run.get("mismatchCount") != 0
            or run.get("candidateCorpusDigest") != expected["candidate_corpus_digest"]
            or run.get("sourceDigest") != expected["candidate_source_digest"]
            or run.get("runtimeDigest") != probe.get("runtime_digest")
            or len(run.get("shardDigests", [])) != 20
        ):
            raise ReplayError("FRESH_PARITY_INVALID: correctness or identity disagrees with pins and probe")
    if any(runs[0].get(field) != runs[1].get(field) for field in stable_fields):
        raise ReplayError("FRESH_PARITY_NONDETERMINISTIC: stable run fields disagree")
    if probe.get("source_digest") != expected["candidate_source_digest"] or probe.get("deterministic") is not True:
        raise ReplayError("PROBE_INVALID: source identity or determinism disagrees with pins")
    return runs


def artifact_digest(pins: dict[str, Any], artifact_id: str) -> str:
    return next(item["digest"] for item in pins["artifacts"] if item["id"] == artifact_id)


def build_report(
    repo_root: Path,
    siglum_root: Path,
    pins_path: Path,
    pins: dict[str, Any],
    parity: dict[str, Any],
    probe: dict[str, Any],
    parity_controller: dict[str, int] | None,
    overall_wall_ms: int,
    memory_bytes: int,
) -> dict[str, Any]:
    runs = verify_fresh_parity(parity, probe, pins["expected"])
    configuration = parity.get("configuration", {})
    performance: dict[str, Any] = {
        "measurement_class": "local_observation_not_benchmark",
        "probe": {
            "request_count": int(probe["request_count"]),
            "cold_first_response_ms": int(probe["cold_first_response_ms"]),
            "warm_response_ms": [int(value) for value in probe["warm_response_ms"]],
            "peak_process_tree_rss_bytes": int(probe["peak_process_tree_rss_bytes"]),
            "result_digests": probe["result_digests"],
            "deterministic": True,
        },
        "full_runs": [
            {
                "run_index": int(run["runIndex"]),
                "elapsed_ms": max(1, round(float(run["elapsedMs"]))),
                "cases_per_second": float(run["casesPerSecond"]),
                "maximum_steps": int(run["maximumSteps"]),
                "maximum_calls": int(run["maximumCalls"]),
                "maximum_call_depth": int(run["maximumCallDepth"]),
                "maximum_response_bytes": int(run["maximumResponseBytes"]),
            }
            for run in runs
        ],
    }
    if parity_controller is not None:
        performance["full_replay_controller"] = {
            "wall_ms": int(parity_controller["wall_ms"]),
            "peak_process_tree_rss_bytes": int(parity_controller["peak_process_tree_rss_bytes"]),
        }
    report = {
        "schema": "sley.operational.reference_replay.v1",
        "status": "passed",
        "workload": {
            "id": "siglum.numerology",
            "version": "siglum.numerology.reference-v1",
            "scope": "complete_frozen_reference_corpus",
            "case_count": 10_000,
        },
        "authority": {
            "mode": "local_replay_only",
            "incumbent_authoritative": True,
            "candidate_authoritative": False,
            "repository_mutation": False,
            "product_runtime_mutation": False,
            "network_authority": False,
            "provider_calls": False,
            "deploy": False,
            "spend": False,
        },
        "identity": {
            "sley_commit": git(repo_root, "rev-parse", "HEAD"),
            "siglum_commit": git(siglum_root, "rev-parse", "HEAD"),
            "candidate_source_digest": runs[0]["sourceDigest"],
            "runtime_digest": runs[0]["runtimeDigest"],
            "candidate_corpus_digest": runs[0]["candidateCorpusDigest"],
            "frozen_corpus_digest": pins["expected"]["frozen_corpus_digest"],
            "corpus_manifest_digest": artifact_digest(pins, "corpus_manifest"),
            "prior_parity_report_digest": artifact_digest(pins, "prior_parity_report"),
            "pins_digest": file_digest(pins_path),
        },
        "oracle": {
            "implementation": "ReferenceTypeScriptSignatureEngine.compute",
            "boundary": "src/lib/signature/reference-adapter.ts",
            "contract": "siglum.numerology.input.v1 -> siglum.numerology.result.v1",
            "independent": True,
            "frozen_outputs": True,
        },
        "host_boundary": {
            "kind": "bounded_persistent_machine_host",
            "protocol": "sley.machine.invoke.v0",
            "persistent_probe": True,
            "fresh_process_per_full_shard": True,
            "immutable_input_pins": True,
            "process_limits_enforced": True,
            "os_network_isolation_enforced": False,
            "credential_environment_scrubbed": True,
            "limits": {
                "overall_wall_ms": overall_wall_ms,
                "shard_wall_ms": int(configuration.get("shardTimeoutMs", 120_000)),
                "evaluation_steps": 250_000,
                "call_depth": 128,
                "collection_length": 4_096,
                "response_bytes": 262_144,
                "shard_output_bytes": int(configuration.get("shardOutputLimitBytes", 33_554_432)),
                "per_process_address_space_bytes": memory_bytes,
                "open_files": 128,
            },
        },
        "correctness": {
            "repeat_count": 2,
            "deterministic": True,
            "exact_match_count": 10_000,
            "mismatch_count": 0,
            "runs": [
                {
                    "run_index": int(run["runIndex"]),
                    "case_count": int(run["caseCount"]),
                    "exact_match_count": int(run["exactMatchCount"]),
                    "mismatch_count": int(run["mismatchCount"]),
                    "candidate_corpus_digest": run["candidateCorpusDigest"],
                    "mismatch_report_digest": run["mismatchReportDigest"],
                    "shard_digests": run["shardDigests"],
                }
                for run in runs
            ],
        },
        "performance": performance,
        "negative_evidence": [
            {
                "code": "MULTILINE_RECORD_LITERAL_RAW_REGRESSION",
                "status": "observed",
                "disposition": "fixed",
                "summary": "The first W6 replay failed closed because multiline typed record constructors were Raw; b97fba8 made them structural RecordLiteral nodes before parity was rerun.",
            },
            {
                "code": "PRODUCTION_PROMOTION_EVIDENCE_MISSING",
                "status": "retained",
                "disposition": "deferred",
                "summary": "Offline parity does not provide production shadow, latency, fallback, rollback, or promotion authority evidence.",
            },
            {
                "code": "PARITY_CONTROLLER_ADDRESS_SPACE_LIMIT",
                "status": "observed",
                "disposition": "fixed",
                "summary": "Full replay attempts at 4 GiB and 8 GiB failed before corpus execution during Node/V8 WebAssembly reservation. Bounded startup probes below 32 GiB also failed during WebAssembly reservation or V8 heap commit; 32 GiB is retained as a virtual-address ceiling, while sampled RSS records actual memory use.",
            },
        ],
        "promotion": {
            "decision": "deferred",
            "production_claim": False,
            "missing_evidence": [
                "production_shadow", "production_latency", "fallback",
                "promotion_authority", "rollback",
            ],
        },
        "disclosure": {
            "contains_real_user_data": False,
            "contains_credentials": False,
            "publication_authorized": False,
            "raw_corpus_embedded": False,
        },
        "issues": [],
    }
    validate(repo_root / "docs/schemas/sley.operational.reference_replay.v1.schema.json", report, "reference replay")
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", required=True)
    parser.add_argument("--siglum-root", required=True)
    parser.add_argument("--pins", required=True)
    parser.add_argument("--execute", action="store_true")
    parser.add_argument("--assemble", action="store_true")
    parser.add_argument("--parity-report")
    parser.add_argument("--probe-report")
    parser.add_argument("--concurrency", type=int, default=4)
    parser.add_argument("--probe-requests", type=int, default=3)
    parser.add_argument("--overall-wall-ms", type=int, default=DEFAULT_OVERALL_WALL_MS)
    parser.add_argument("--probe-wall-ms", type=int, default=DEFAULT_PROBE_WALL_MS)
    parser.add_argument("--memory-bytes", type=int, default=DEFAULT_MEMORY_BYTES)
    parser.add_argument("--json", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.execute == args.assemble:
        raise ReplayError("MODE_REQUIRED: choose exactly one of --execute or --assemble")
    if not 1 <= args.concurrency <= 8:
        raise ReplayError("BOUND_INVALID: concurrency must be between 1 and 8")
    if not 3 <= args.probe_requests <= 32:
        raise ReplayError("BOUND_INVALID: probe requests must be between 3 and 32")
    if args.overall_wall_ms < 1 or args.probe_wall_ms < 1 or args.memory_bytes < 67_108_864:
        raise ReplayError("BOUND_INVALID: wall and memory limits must be positive bounded values")
    repo_root = Path(args.repo_root).resolve(strict=True)
    siglum_root = Path(args.siglum_root).resolve(strict=True)
    pins_path = Path(args.pins).resolve(strict=True)
    try:
        pins_path.relative_to(repo_root)
    except ValueError as exc:
        raise ReplayError("PIN_PATH_INVALID: pins must be owned by the Sley repository") from exc
    pins = load_json(pins_path)
    resolved = verify_inputs(repo_root, siglum_root, pins_path, pins)
    parity_controller = None
    if args.execute:
        require_clean_tracked(repo_root, "Sley")
        with tempfile.TemporaryDirectory(prefix="sley-reference-replay-") as temporary:
            temporary_root = Path(temporary)
            probe = run_probe(
                repo_root,
                resolved["candidate_package"],
                resolved["corpus_root"],
                args.probe_requests,
                args.memory_bytes,
                args.probe_wall_ms,
            )
            parity, parity_controller = run_parity(
                repo_root,
                siglum_root,
                temporary_root / "parity.json",
                args.concurrency,
                args.memory_bytes,
                args.overall_wall_ms,
            )
    else:
        if not args.parity_report or not args.probe_report:
            raise ReplayError("ASSEMBLY_INPUT_REQUIRED: --parity-report and --probe-report are required")
        parity = load_json(Path(args.parity_report).resolve(strict=True))
        probe = load_json(Path(args.probe_report).resolve(strict=True))
    report = build_report(
        repo_root,
        siglum_root,
        pins_path,
        pins,
        parity,
        probe,
        parity_controller,
        args.overall_wall_ms,
        args.memory_bytes,
    )
    rendered = json.dumps(report, indent=2, ensure_ascii=False) + "\n"
    sys.stdout.write(rendered)
    return 0


try:
    raise SystemExit(main())
except ReplayError as exc:
    print(str(exc), file=sys.stderr)
    raise SystemExit(1)
PY
