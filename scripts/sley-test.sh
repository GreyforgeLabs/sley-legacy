#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$ROOT_DIR"

if [[ "${1:-}" == "--repo-root" ]]; then
  [[ $# -ge 2 ]] || { echo "--repo-root requires a path" >&2; exit 2; }
  REPO_ROOT="$2"
  shift 2
fi

exec python3 /dev/fd/3 "$REPO_ROOT" "$PWD" "$@" 3<<'PY'
import hashlib
import itertools
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time


REPO = Path(sys.argv[1]).resolve()
CALLER_CWD = Path(sys.argv[2]).resolve()
SLEY = REPO / "bin" / "sley"
CONTRACT = REPO / "bin" / "sley-contract"
SCHEMAS = REPO / "docs" / "schemas"
MANIFEST_SCHEMA = os.environ["SLEY_TEST_MANIFEST_SCHEMA"]
REPORT_SCHEMA = os.environ["SLEY_TEST_REPORT_SCHEMA"]
REVIEW_PACKET_SCHEMA = os.environ["SLEY_TEST_REVIEW_PACKET_SCHEMA"]
DISCOVERY_CONVENTION = os.environ["SLEY_TEST_DISCOVERY_CONVENTION"]
CASE_KINDS = json.loads(os.environ["SLEY_TEST_CASE_KINDS"])
COVERAGE_DIMENSIONS = json.loads(os.environ["SLEY_TEST_COVERAGE_DIMENSIONS"])
COVERAGE_EVIDENCE_STATES = json.loads(os.environ["SLEY_TEST_COVERAGE_EVIDENCE_STATES"])
PROPERTY_STRATEGY = os.environ["SLEY_TEST_PROPERTY_STRATEGY"]
DIFFERENTIAL_COMPARISON = os.environ["SLEY_TEST_DIFFERENTIAL_COMPARISON"]
AUTHORITY_MODE = os.environ["SLEY_TEST_AUTHORITY_MODE"]
REVIEW_BINDING_AUTHORITY_MODE = os.environ["SLEY_TEST_REVIEW_BINDING_AUTHORITY_MODE"]
EMPTY_COVERAGE = {"tasks": [], "branches": [], "effects": [], "capabilities": []}


class TestFailure(Exception):
    pass


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False)


def digest_bytes(value):
    return "sha256:" + hashlib.sha256(value).hexdigest()


def digest_json(value):
    return digest_bytes(canonical(value).encode("utf-8"))


def digest_json_line(value):
    return digest_bytes((canonical(value) + "\n").encode("utf-8"))


def content_identity_matches(value, id_field, digest_field, id_prefix, digest_fn=digest_json):
    core = {key: item for key, item in value.items() if key not in (id_field, digest_field)}
    expected_digest = digest_fn(core)
    return (
        value.get(digest_field) == expected_digest
        and value.get(id_field) == id_prefix + expected_digest.removeprefix("sha256:")
    )


def issue(code, message):
    return {"code": code, "message": message}


def load_json(path):
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:
        raise TestFailure(f"could not load JSON from {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise TestFailure(f"JSON root must be an object: {path}")
    return value


def run(command, *, input_text=None, timeout_ms=120000, allow_failure=False):
    try:
        completed = subprocess.run(
            [str(item) for item in command],
            cwd=CALLER_CWD,
            input=input_text,
            text=True,
            capture_output=True,
            timeout=max(0.001, timeout_ms / 1000.0),
            env={**os.environ, "PATH": str(REPO / "bin") + os.pathsep + os.environ.get("PATH", "")},
        )
    except subprocess.TimeoutExpired as exc:
        raise TestFailure(f"bounded command timed out: {' '.join(map(str, command))}") from exc
    if completed.returncode != 0 and not allow_failure:
        detail = completed.stderr.strip() or completed.stdout.strip() or f"exit {completed.returncode}"
        raise TestFailure(f"command failed: {' '.join(map(str, command))}: {detail[:512]}")
    return completed


def parse_stdout_json(completed, label):
    if len(completed.stdout.encode("utf-8")) > 5242880:
        raise TestFailure(f"{label} exceeded the global JSON output budget")
    try:
        value = json.loads(completed.stdout)
    except Exception as exc:
        raise TestFailure(f"{label} did not emit one JSON object") from exc
    if not isinstance(value, dict):
        raise TestFailure(f"{label} JSON root was not an object")
    return value


def validate_contract(schema, path):
    completed = run([
        CONTRACT, "validate", "--schema", schema, str(path),
        "--schemas", str(SCHEMAS), "--json",
    ], allow_failure=True)
    if completed.returncode != 0:
        raise TestFailure(f"{path} failed {schema} validation")


def validate_contract_document(schema, value):
    with tempfile.TemporaryDirectory(prefix="sley-test-contract-") as temporary:
        document_path = Path(temporary) / "document.json"
        document_path.write_text(json.dumps(value), encoding="utf-8")
        completed = subprocess.run(
            [str(CONTRACT), "validate", "--schema", schema, str(document_path), "--schemas", str(SCHEMAS), "--json"],
            cwd=CALLER_CWD,
            text=True,
            capture_output=True,
            env={**os.environ, "PATH": str(REPO / "bin") + os.pathsep + os.environ.get("PATH", "")},
        )
    if completed.returncode != 0:
        detail = completed.stdout.strip() or completed.stderr.strip()
        raise TestFailure(f"generated {schema} failed validation: {detail[:1024]}")


def relative(path, root):
    try:
        return path.resolve().relative_to(root.resolve()).as_posix()
    except ValueError:
        return str(path.resolve())


def confined(path, root, label):
    resolved = path.resolve()
    try:
        resolved.relative_to(root.resolve())
    except ValueError as exc:
        raise TestFailure(f"{label} escapes the test discovery root: {path}") from exc
    if path.is_symlink() or resolved.is_symlink():
        raise TestFailure(f"{label} must not be a symlink: {path}")
    return resolved


def discover(target):
    if not target.exists():
        raise TestFailure(f"test target was not found: {target}")
    if target.is_symlink():
        raise TestFailure(f"test target must not be a symlink: {target}")
    if target.is_file():
        root = target.parent.resolve()
        manifests = [target.resolve()]
    else:
        root = target.resolve()
        manifests = sorted(
            [
                path.resolve()
                for path in root.rglob("*.json")
                if path.name == "sley.test.json" or path.name.endswith(".sley-test.json")
            ],
            key=lambda path: relative(path, root).encode("utf-8"),
        )
    if not manifests:
        raise TestFailure(f"no sley.test.json or *.sley-test.json manifests found under {target}")
    for manifest in manifests:
        confined(manifest, root, "test manifest")
    return root, manifests


def source_files(target):
    if target.is_file():
        candidates = [target]
        source_root = target.parent.resolve()
    else:
        base = target / "src" if (target / "sley.toml").is_file() and (target / "src").is_dir() else target
        candidates = list(base.rglob("*.sley"))
        source_root = target.resolve()
    files = []
    for path in candidates:
        if path.is_symlink():
            raise TestFailure(f"Sley source must not be a symlink: {path}")
        resolved = path.resolve()
        try:
            resolved.relative_to(source_root)
        except ValueError as exc:
            raise TestFailure(f"Sley source escapes the suite source root: {path}") from exc
        if not resolved.is_file():
            raise TestFailure(f"Sley source must be a regular file: {path}")
        files.append(resolved)
    return sorted(files, key=lambda path: str(path).encode("utf-8"))


def structural_source_digest(target):
    files = source_files(target)
    if not files:
        raise TestFailure(f"source contains no Sley files: {target}")
    if len(files) == 1:
        return digest_bytes(files[0].read_bytes())
    target_root = target.resolve()
    payload = bytearray()
    for path in files:
        payload.extend(f"path:{path.resolve().relative_to(target_root).as_posix()}\n".encode("utf-8"))
        payload.extend(path.read_bytes())
        payload.extend(b"\n")
    return digest_bytes(bytes(payload))


def resolve_ref(manifest_path, raw, root, label):
    path = confined(manifest_path.parent / raw, root, label)
    if not path.exists():
        raise TestFailure(f"{label} was not found: {path}")
    return path


def query_source(target, timeout_ms):
    completed = run([SLEY, "query", "--json", "--kind", "all", target], timeout_ms=timeout_ms)
    return parse_stdout_json(completed, "sley query")


def branch_inventory(target, timeout_ms):
    branches = set()

    def walk(value):
        if isinstance(value, dict):
            node_id = value.get("id")
            if value.get("kind") == "If" and isinstance(node_id, str):
                branches.add(node_id + ":then")
                branches.add(node_id + ":else")
            if value.get("expr_kind") == "If" and isinstance(node_id, str):
                then_id = value.get("then_branch", {}).get("id") if isinstance(value.get("then_branch"), dict) else None
                else_id = value.get("else_branch", {}).get("id") if isinstance(value.get("else_branch"), dict) else None
                branches.add(then_id or node_id + ":then")
                branches.add(else_id or node_id + ":else")
            for child in value.values():
                walk(child)
        elif isinstance(value, list):
            for child in value:
                walk(child)

    for path in source_files(target):
        completed = run([SLEY, "ast", "--json", path], timeout_ms=timeout_ms)
        walk(parse_stdout_json(completed, "sley ast"))
    return sorted(branches)


def source_inventory(target, timeout_ms):
    query = query_source(target, timeout_ms)
    tasks = sorted(
        row["id"] for row in query.get("tasks", [])
        if isinstance(row, dict) and isinstance(row.get("id"), str) and not row["id"].startswith("task:sley.machine.")
    )
    effects = sorted({
        "effect:" + effect
        for row in query.get("tasks", []) if isinstance(row, dict)
        for effect in row.get("effects", []) if isinstance(effect, str)
    })
    return query, {
        "tasks": tasks,
        "branches": branch_inventory(target, timeout_ms),
        "effects": effects,
    }


def coverage_copy(value=None):
    source = value if isinstance(value, dict) else EMPTY_COVERAGE
    return {key: sorted(set(source.get(key, []))) for key in EMPTY_COVERAGE}


def machine_invoke(target, package, task, input_value, request_id, bounds):
    timeout_ms = min(bounds["max_wall_ms"], 2000)
    request = {
        "protocolVersion": "sley.machine.invoke.v0",
        "requestId": request_id,
        "operation": "invoke",
        "packageId": package["id"],
        "packageVersion": package["version"],
        "task": task,
        "input": input_value,
        "limits": {
            "requestLineBytes": 4096,
            "responseBytes": min(bounds["max_output_bytes"], 262144),
            "batch": 1,
            "evaluationSteps": 250000,
            "callDepth": 128,
            "collectionLength": 4096,
            "timeoutMs": timeout_ms,
        },
    }
    completed = run([
        SLEY, "machine", "--source", target,
        "--package-id", package["id"], "--package-version", package["version"],
    ], input_text=canonical(request) + "\n", timeout_ms=bounds["max_wall_ms"], allow_failure=True)
    response = parse_stdout_json(completed, "sley machine")
    return response


def result_record(case_id, execution_id, kind, manifest_rel, source_rel, *, task=None, input_value=None, expected=None, actual=None, subreport=None, observed=None, declared=None, issues=None):
    problems = list(issues or [])
    return {
        "case_id": case_id,
        "execution_id": execution_id,
        "kind": kind,
        "status": "passed" if not problems else "failed",
        "manifest": manifest_rel,
        "source": source_rel,
        **({"task": task} if task else {}),
        "input_digest": digest_json(input_value) if input_value is not None else None,
        "expected_digest": digest_json(expected) if expected is not None else None,
        "actual_digest": digest_json(actual) if actual is not None else None,
        "subreport": ({"schema": subreport.get("schema", "unknown"), "digest": digest_json(subreport)} if isinstance(subreport, dict) else None),
        "observed": coverage_copy(observed),
        "declared": coverage_copy(declared),
        "issues": problems,
    }


def execute_machine_case(case, manifest_path, manifest_rel, root, source, package, bounds):
    records = []
    task = case["task"]
    case_id = case["id"]
    if case["kind"] == "table":
        rows = sorted(case["rows"], key=lambda row: row["id"].encode("utf-8"))
    else:
        fields = case["generator"]["fields"]
        names = [field["name"] for field in fields]
        product = itertools.product(*(field["values"] for field in fields))
        rows = [
            {"id": f"generated-{index:04d}", "input": dict(zip(names, values)), "expected_result": True, "branch_witnesses": []}
            for index, values in enumerate(itertools.islice(product, case["generator"]["max_cases"]))
        ]
    for row in rows:
        execution_id = case_id + ":" + row["id"]
        response = machine_invoke(source, package, task, row["input"], execution_id, bounds)
        actual = response.get("result") if response.get("status") == "ok" else None
        problems = []
        if response.get("status") != "ok":
            problems.append(issue("machine_invocation_failed", f"{execution_id} did not return an ok machine response"))
        elif actual != row["expected_result"]:
            problems.append(issue("result_mismatch", f"{execution_id} result did not match the expected canonical JSON value"))
        declared = coverage_copy(case["covers"])
        declared["branches"] = sorted(set(declared["branches"]) | set(row.get("branch_witnesses", [])))
        observed = coverage_copy()
        if not problems:
            observed["tasks"] = ["task:" + task]
        records.append(result_record(
            case_id, execution_id, case["kind"], manifest_rel, relative(source, root),
            task=task, input_value=row["input"], expected=row["expected_result"], actual=actual,
            subreport=response, observed=observed, declared=declared, issues=problems,
        ))
    return records


def projected_execution_count(case):
    if case["kind"] in ("table", "differential"):
        return len(case["rows"])
    if case["kind"] == "property":
        field_product = 1
        for field in case["generator"]["fields"]:
            field_product *= len(field["values"])
        return min(field_product, case["generator"]["max_cases"])
    return 1


def execute_effect_mock(case, manifest_path, manifest_rel, root, bounds):
    target = resolve_ref(manifest_path, case["target"], root, "effect-mock target")
    command = [SLEY, "run", "--json"]
    for capability in case["capabilities"]:
        command.extend(["--cap", capability])
    flags = {
        "secrets": "--secret",
        "http_text": "--http-text",
        "shell_outputs": "--shell-output",
        "model_outputs": "--model-output",
        "deploy_results": "--deploy-result",
        "spend_results": "--spend-result",
    }
    for family, flag in flags.items():
        for mock in case["mocks"][family]:
            command.extend([flag, mock["key"], mock["text"]])
    command.append(target)
    completed = run(command, timeout_ms=bounds["max_wall_ms"], allow_failure=True)
    report = parse_stdout_json(completed, "sley run")
    actual = report.get("value")
    problems = []
    if report.get("status") != "passed":
        problems.append(issue("effect_mock_failed", f"{case['id']} did not return a passing run report"))
    elif actual != case["expected_result"]:
        problems.append(issue("result_mismatch", f"{case['id']} effect-mock result did not match"))
    observed = coverage_copy()
    if not problems:
        query = query_source(target, bounds["max_wall_ms"])
        entry = query.get("entry_module")
        if isinstance(entry, str):
            observed["tasks"] = [f"task:{entry}.main"]
        observed["effects"] = sorted({"effect:" + value.split("=", 1)[0] for value in case["capabilities"]})
        observed["capabilities"] = sorted({"capability:" + value for value in case["capabilities"]})
    return [result_record(
        case["id"], case["id"], case["kind"], manifest_rel, relative(target, root),
        expected=case["expected_result"], actual=actual, subreport=report,
        observed=observed, declared=case["covers"], issues=problems,
    )]


def execute_adapter(case, manifest_path, manifest_rel, root, bounds):
    adapter_manifest = resolve_ref(manifest_path, case["manifest"], root, "adapter replay manifest")
    completed = run([SLEY, "adapter", "replay", "--json", adapter_manifest], timeout_ms=bounds["max_wall_ms"], allow_failure=True)
    report = parse_stdout_json(completed, "sley adapter replay")
    actual = report.get("record", {}).get("record_digest") if isinstance(report.get("record"), dict) else None
    problems = []
    if report.get("status") != "passed":
        problems.append(issue("adapter_replay_failed", f"{case['id']} did not return a passing adapter report"))
    elif actual != case["expected_record_digest"]:
        problems.append(issue("adapter_record_mismatch", f"{case['id']} replay digest did not match"))
    observed = coverage_copy()
    if not problems:
        document = load_json(adapter_manifest)
        observed["effects"] = sorted({"effect:" + row["effect"] for row in document.get("capabilities", [])})
        observed["capabilities"] = sorted({
            "capability:" + row["effect"] + "=" + row["scope"]
            for row in document.get("capabilities", [])
        })
        target = resolve_ref(adapter_manifest, document["target"], root, "adapter target")
        query = query_source(target, bounds["max_wall_ms"])
        entry = query.get("entry_module")
        if isinstance(entry, str):
            observed["tasks"] = [f"task:{entry}.main"]
    return [result_record(
        case["id"], case["id"], case["kind"], manifest_rel, relative(adapter_manifest, root),
        expected=case["expected_record_digest"], actual=actual, subreport=report,
        observed=observed, declared=case["covers"], issues=problems,
    )]


def execute_diagnostic(case, manifest_path, manifest_rel, root, bounds):
    target = resolve_ref(manifest_path, case["target"], root, "diagnostic target")
    completed = run([SLEY, "check", "--json", target], timeout_ms=bounds["max_wall_ms"], allow_failure=True)
    report = parse_stdout_json(completed, "sley check")
    actual = [row.get("id") for row in report.get("diagnostics", []) if isinstance(row, dict) and isinstance(row.get("id"), str)]
    expected = case["expected_ids"]
    missing = sorted(set(expected) - set(actual))
    extras = sorted(set(actual) - set(expected))
    problems = []
    if missing:
        problems.append(issue("expected_diagnostic_missing", f"{case['id']} missed diagnostics: {', '.join(missing)}"))
    if extras and not case["allow_additional"]:
        problems.append(issue("unexpected_diagnostic", f"{case['id']} emitted additional diagnostics: {', '.join(extras)}"))
    if report.get("status") != "error":
        problems.append(issue("diagnostic_status_mismatch", f"{case['id']} expected an error diagnostic report"))
    return [result_record(
        case["id"], case["id"], case["kind"], manifest_rel, relative(target, root),
        expected=expected, actual=actual, subreport=report,
        declared=case["covers"], issues=problems,
    )]


def execute_differential(case, manifest_path, manifest_rel, root, bounds):
    records = []
    left = case["left"]
    right = case["right"]
    left_target = resolve_ref(manifest_path, left["target"], root, "differential left target")
    right_target = resolve_ref(manifest_path, right["target"], root, "differential right target")
    for row in sorted(case["rows"], key=lambda value: value["id"].encode("utf-8")):
        execution_id = case["id"] + ":" + row["id"]
        left_report = machine_invoke(left_target, left["package"], left["task"], row["input"], execution_id + ":left", bounds)
        right_report = machine_invoke(right_target, right["package"], right["task"], row["input"], execution_id + ":right", bounds)
        left_value = left_report.get("result") if left_report.get("status") == "ok" else None
        right_value = right_report.get("result") if right_report.get("status") == "ok" else None
        problems = []
        if left_report.get("status") != "ok" or right_report.get("status") != "ok":
            problems.append(issue("differential_invocation_failed", f"{execution_id} did not produce two ok machine responses"))
        elif canonical(left_value) != canonical(right_value):
            problems.append(issue("differential_mismatch", f"{execution_id} produced different canonical JSON results"))
        observed = coverage_copy()
        if not problems:
            observed["tasks"] = sorted({"task:" + left["task"], "task:" + right["task"]})
        declared = coverage_copy(case["covers"])
        declared["branches"] = sorted(set(declared["branches"]) | set(row.get("branch_witnesses", [])))
        combined = {"schema": "sley.test.differential.evidence.v1", "left": left_report, "right": right_report}
        records.append(result_record(
            case["id"], execution_id, case["kind"], manifest_rel, relative(left_target, root) + "|" + relative(right_target, root),
            task=left["task"] + "|" + right["task"], input_value=row["input"],
            expected=left_value, actual=right_value, subreport=combined,
            observed=observed, declared=declared, issues=problems,
        ))
    return records


def expand_changed_nodes(requested, query, scope):
    expanded = set(requested)
    reverse = {}
    for call in query.get("calls", []):
        if not isinstance(call, dict) or call.get("status") != "resolved":
            continue
        source = "task:" + call.get("from", "")
        target = "task:" + call.get("target", "")
        reverse.setdefault(target, set()).add(source)
    queue = []
    for node in requested:
        if node.startswith("task:"):
            queue.append(node)
        elif node.startswith("module:"):
            module = node[len("module:"):]
            queue.extend(task for task in scope["tasks"] if task.startswith("task:" + module + "."))
        else:
            match = re.search(r"task:[A-Za-z_][A-Za-z0-9_.]*", node)
            if match:
                queue.append(match.group(0))
    while queue:
        node = queue.pop(0)
        if node not in expanded:
            expanded.add(node)
        for caller in sorted(reverse.get(node, [])):
            if caller not in expanded:
                expanded.add(caller)
                queue.append(caller)
    return sorted(expanded)


def coverage_dimension(inventory, observed, declared, method):
    inventory_set = set(inventory)
    observed_set = set(observed) & inventory_set
    declared_set = (set(declared) & inventory_set) - observed_set
    credited = observed_set | declared_set
    uncovered = inventory_set - credited
    return {
        "method": method,
        "inventory": sorted(inventory_set),
        "observed": sorted(observed_set),
        "declared": sorted(declared_set),
        "unsupported": [],
        "unknown": sorted((set(observed) | set(declared)) - inventory_set),
        "credited": sorted(credited),
        "uncovered": sorted(uncovered),
        "credited_ratio": (len(credited) / len(inventory_set)) if inventory_set else 1.0,
    }


def parse_args(args):
    parsed = {"json": False, "changed_nodes": [], "mode": "run", "target": None, "review": None, "seal": None, "report": None}
    index = 0
    if index < len(args) and args[index] == "bind-review":
        parsed["mode"] = "bind"
        index += 1
    while index < len(args):
        arg = args[index]
        if arg == "--json":
            parsed["json"] = True
            index += 1
        elif arg == "--changed-node":
            if index + 1 >= len(args):
                raise TestFailure("--changed-node requires a node id")
            parsed["changed_nodes"].append(args[index + 1])
            index += 2
        elif arg in ("--review", "--seal", "--report"):
            if index + 1 >= len(args):
                raise TestFailure(f"{arg} requires a path")
            parsed[arg[2:].replace("-", "_")] = args[index + 1]
            index += 2
        elif arg.startswith("--"):
            raise TestFailure(f"unknown sley test option: {arg}")
        elif parsed["target"] is None:
            parsed["target"] = arg
            index += 1
        else:
            raise TestFailure("sley test accepts one file or directory target")
    return parsed


def bind_review(parsed):
    if not all(parsed[name] for name in ("review", "seal", "report")):
        raise TestFailure("sley test bind-review requires --review, --seal, and --report")
    paths = {name: (CALLER_CWD / parsed[name]).resolve() for name in ("review", "seal", "report")}
    validate_contract("sley.change.review.v0", paths["review"])
    validate_contract("sley.change.transaction_seal.v0", paths["seal"])
    validate_contract(REPORT_SCHEMA, paths["report"])
    review = load_json(paths["review"])
    seal = load_json(paths["seal"])
    report = load_json(paths["report"])
    review_identity_match = content_identity_matches(review, "review_id", "review_digest", "review:", digest_json_line)
    seal_identity_match = content_identity_matches(seal, "seal_id", "seal_digest", "transaction-seal:", digest_json_line)
    test_identity_match = content_identity_matches(report, "report_id", "report_digest", "test-report:")
    checks = {
        "transaction_match": review["transaction"]["transaction_id"] == seal["transaction_id"],
        "review_match": (
            review_identity_match
            and seal["review_ref"]["review_id"] == review["review_id"]
            and seal["review_ref"]["review_digest"] == review["review_digest"]
            and seal["bundle_digest"] == digest_json_line(review)
        ),
        "seal_match": seal_identity_match and seal["review_ref"]["schema"] == review["schema"],
        "source_match": review["final"]["source_digest"] == seal["source_digest"] == report["source"]["digest"],
        "test_passed": test_identity_match and report["status"] == "passed",
    }
    failed = [name for name, value in checks.items() if not value]
    if failed:
        raise TestFailure("review/test evidence binding failed: " + ", ".join(failed))
    core = {
        "schema": REVIEW_PACKET_SCHEMA,
        "status": "ready",
        "transaction_id": review["transaction"]["transaction_id"],
        "terminal_outcome": review["terminal_outcome"],
        "final_source_digest": review["final"]["source_digest"],
        "review_ref": {"schema": review["schema"], "review_id": review["review_id"], "review_digest": review["review_digest"]},
        "seal_ref": {"schema": seal["schema"], "seal_id": seal["seal_id"], "seal_digest": seal["seal_digest"]},
        "test_ref": {
            "schema": report["schema"], "report_id": report["report_id"], "report_digest": report["report_digest"],
            "suite_id": report["suite"]["id"], "suite_version": report["suite"]["version"],
            "status": report["status"], "source_digest": report["source"]["digest"],
        },
        "binding": checks,
        "authority": {"mode": REVIEW_BINDING_AUTHORITY_MODE, "mutation_authority": False},
        "issues": [],
    }
    packet_digest = digest_json(core)
    packet = {
        "schema": core["schema"], "status": core["status"],
        "packet_id": "review-packet:" + packet_digest.removeprefix("sha256:"),
        "packet_digest": packet_digest,
        **{key: value for key, value in core.items() if key not in ("schema", "status")},
    }
    validate_contract_document(REVIEW_PACKET_SCHEMA, packet)
    return packet


def run_suite(parsed):
    if not parsed["target"]:
        raise TestFailure("sley test requires a manifest file or discovery directory")
    target = (CALLER_CWD / parsed["target"]).resolve()
    root, manifest_paths = discover(target)
    manifests = []
    manifest_refs = []
    for path in manifest_paths:
        validate_contract(MANIFEST_SCHEMA, path)
        document = load_json(path)
        manifests.append((path, document))
        manifest_refs.append({"path": relative(path, root), "digest": digest_json(document)})
    first = manifests[0][1]
    if first["authority"]["mode"] != AUTHORITY_MODE:
        raise TestFailure("test authority mode does not match the Sley-owned testing contract")
    if sorted(first["coverage_scope"]) != sorted(COVERAGE_DIMENSIONS):
        raise TestFailure("coverage dimensions do not match the Sley-owned testing contract")
    if sorted(COVERAGE_EVIDENCE_STATES) != ["declared", "observed", "unknown", "unsupported"]:
        raise TestFailure("coverage evidence states do not match the supported runner contract")
    for path, document in manifests[1:]:
        for key in ("suite", "source", "authority", "bounds", "coverage_scope"):
            if document[key] != first[key]:
                raise TestFailure(f"discovered manifests disagree on suite-level {key}: {path}")
    source = resolve_ref(manifests[0][0], first["source"]["target"], root, "suite source")
    source_digest = structural_source_digest(source)
    bounds = first["bounds"]
    query, discovered = source_inventory(source, bounds["max_wall_ms"])
    scope = coverage_copy(first["coverage_scope"])
    for key in ("tasks", "branches", "effects"):
        if set(scope[key]) != set(discovered[key]):
            missing = sorted(set(discovered[key]) - set(scope[key]))
            extra = sorted(set(scope[key]) - set(discovered[key]))
            raise TestFailure(f"coverage_scope.{key} does not match source inventory; missing={missing}, extra={extra}")

    all_cases = []
    seen_case_ids = set()
    for path, document in manifests:
        for case in sorted(document["cases"], key=lambda item: item["id"].encode("utf-8")):
            if case["id"] in seen_case_ids:
                raise TestFailure(f"duplicate discovered test case id: {case['id']}")
            seen_case_ids.add(case["id"])
            if case["kind"] not in CASE_KINDS:
                raise TestFailure(f"{case['id']} uses a case kind outside the Sley-owned testing contract")
            if case["kind"] == "property" and case["generator"]["strategy"] != PROPERTY_STRATEGY:
                raise TestFailure(f"{case['id']} uses an unsupported property strategy")
            if case["kind"] == "differential" and case["comparison"] != DIFFERENTIAL_COMPARISON:
                raise TestFailure(f"{case['id']} uses an unsupported differential comparison")
            for dimension in EMPTY_COVERAGE:
                unknown = sorted(set(case["covers"][dimension]) - set(scope[dimension]))
                if unknown:
                    raise TestFailure(f"{case['id']} covers unknown {dimension}: {unknown}")
            all_cases.append((path, case))
    requested = sorted(set(parsed["changed_nodes"]))
    expanded_by_requested = {
        node: set(expand_changed_nodes([node], query, scope)) for node in requested
    }
    expanded = sorted(set().union(*expanded_by_requested.values())) if requested else []
    selected = []
    skipped = []
    for path, case in all_cases:
        case_nodes = set(case["selection_nodes"])
        for values in case["covers"].values():
            case_nodes.update(values)
        if not requested or case_nodes.intersection(expanded):
            selected.append((path, case))
        else:
            skipped.append(case["id"])
    uncovered_nodes = []
    if requested:
        selected_nodes = set()
        for _, case in selected:
            selected_nodes.update(case["selection_nodes"])
            for values in case["covers"].values():
                selected_nodes.update(values)
        uncovered_nodes = [
            node for node in requested
            if not expanded_by_requested[node].intersection(selected_nodes)
        ]

    projected_executions = sum(projected_execution_count(case) for _, case in selected)
    generated_executions = sum(
        projected_execution_count(case) for _, case in selected if case["kind"] == "property"
    )
    if projected_executions > bounds["max_cases"]:
        raise TestFailure("selected execution count exceeds bounds.max_cases")
    if generated_executions > bounds["max_generated_cases"]:
        raise TestFailure("selected property execution count exceeds bounds.max_generated_cases")

    records = []
    start = time.monotonic()
    package = first["source"]["package"]
    for manifest_path, case in selected:
        if (time.monotonic() - start) * 1000 > bounds["max_wall_ms"]:
            raise TestFailure("suite exceeded max_wall_ms before all selected cases ran")
        manifest_rel = relative(manifest_path, root)
        if case["kind"] in ("table", "property"):
            produced = execute_machine_case(case, manifest_path, manifest_rel, root, source, package, bounds)
        elif case["kind"] == "effect_mock":
            produced = execute_effect_mock(case, manifest_path, manifest_rel, root, bounds)
        elif case["kind"] == "adapter_replay":
            produced = execute_adapter(case, manifest_path, manifest_rel, root, bounds)
        elif case["kind"] == "diagnostic":
            produced = execute_diagnostic(case, manifest_path, manifest_rel, root, bounds)
        elif case["kind"] == "differential":
            produced = execute_differential(case, manifest_path, manifest_rel, root, bounds)
        else:
            raise TestFailure(f"unsupported test case kind: {case['kind']}")
        records.extend(produced)
        if len(records) > bounds["max_cases"]:
            raise TestFailure("expanded execution count exceeded bounds.max_cases")
        if (time.monotonic() - start) * 1000 > bounds["max_wall_ms"]:
            raise TestFailure("suite exceeded max_wall_ms")

    suite_issues = []
    if uncovered_nodes:
        suite_issues.append(issue("changed_node_uncovered", "changed nodes selected no test evidence: " + ", ".join(uncovered_nodes)))
    failed_count = sum(1 for record in records if record["status"] == "failed")
    if failed_count:
        suite_issues.append(issue("test_execution_failed", f"{failed_count} test executions failed"))
    observed = {key: set() for key in EMPTY_COVERAGE}
    declared = {key: set() for key in EMPTY_COVERAGE}
    for record in records:
        if record["status"] != "passed":
            continue
        for key in EMPTY_COVERAGE:
            observed[key].update(record["observed"][key])
            declared[key].update(record["declared"][key])
    coverage = {
        "tasks": coverage_dimension(scope["tasks"], observed["tasks"], declared["tasks"], "entrypoint_execution"),
        "branches": coverage_dimension(scope["branches"], observed["branches"], declared["branches"], "passing_manifest_witness"),
        "effects": coverage_dimension(scope["effects"], observed["effects"], declared["effects"], "runtime_authority"),
        "capabilities": coverage_dimension(scope["capabilities"], observed["capabilities"], declared["capabilities"], "adapter_authority"),
    }
    core = {
        "schema": REPORT_SCHEMA,
        "status": "passed" if not suite_issues else "failed",
        "suite": first["suite"],
        "source": {"target": relative(source, root), "digest": source_digest},
        "manifests": manifest_refs,
        "authority": first["authority"],
        "bounds": bounds,
        "discovery": {
            "root": relative(root, CALLER_CWD),
            "convention": DISCOVERY_CONVENTION,
            "ordered_manifests": [relative(path, root) for path in manifest_paths],
        },
        "selection": {
            "mode": "changed_nodes" if requested else "all",
            "requested_nodes": requested,
            "expanded_nodes": expanded,
            "selected_cases": [case["id"] for _, case in selected],
            "skipped_cases": sorted(skipped),
            "uncovered_nodes": uncovered_nodes,
        },
        "summary": {
            "discovered_case_count": len(all_cases),
            "selected_case_count": len(selected),
            "execution_count": len(records),
            "passed_count": len(records) - failed_count,
            "failed_count": failed_count,
            "skipped_count": len(skipped),
        },
        "cases": records,
        "coverage": coverage,
        "issues": suite_issues,
    }
    report_digest = digest_json(core)
    report = {
        "schema": core["schema"], "status": core["status"],
        "report_id": "test-report:" + report_digest.removeprefix("sha256:"),
        "report_digest": report_digest,
        **{key: value for key, value in core.items() if key not in ("schema", "status")},
    }
    if len(canonical(report).encode("utf-8")) > bounds["max_output_bytes"]:
        raise TestFailure("generated test report exceeded bounds.max_output_bytes")
    validate_contract_document(REPORT_SCHEMA, report)
    return report


def render_human(value):
    if value.get("schema") == REPORT_SCHEMA:
        summary = value["summary"]
        print(f"Sley test {value['status']}: {summary['passed_count']} passed, {summary['failed_count']} failed, {summary['skipped_count']} skipped")
        print(f"Suite: {value['suite']['id']} {value['suite']['version']}")
        print(f"Report: {value['report_digest']}")
        for dimension in ("tasks", "branches", "effects", "capabilities"):
            row = value["coverage"][dimension]
            print(f"Coverage {dimension}: {len(row['credited'])}/{len(row['inventory'])} credited via {row['method']}")
    else:
        print(f"Sley review packet ready: {value['packet_digest']}")


def main():
    parsed = parse_args(sys.argv[3:])
    value = bind_review(parsed) if parsed["mode"] == "bind" else run_suite(parsed)
    if parsed["json"]:
        print(json.dumps(value, indent=2, ensure_ascii=False))
    else:
        render_human(value)
    return 0 if value.get("status") in ("passed", "ready") else 1


try:
    raise SystemExit(main())
except TestFailure as exc:
    print(f"sley test: {exc}", file=sys.stderr)
    raise SystemExit(2)
PY
