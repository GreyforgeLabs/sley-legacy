#!/usr/bin/env bash
set -euo pipefail

python3 - "$@" <<'PY'
"""Plan and assemble the bounded S12-602 agent workflow comparison."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import subprocess
import sys
import warnings
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, RefResolver


MAX_JSON_BYTES = 16 * 1024 * 1024
MAX_EVIDENCE_FILE_BYTES = 2 * 1024 * 1024
PRIVATE_APPROVAL_SCHEMA = "greyforge.sley.operational.agent_workflow_approval.private.v1"
PRIVATE_EXECUTION_PROVENANCE_SCHEMA = "greyforge.sley.operational.agent_workflow_execution.private.v1"


class WorkflowError(RuntimeError):
    pass


def required_env(name: str) -> str:
    value = os.environ.get(name, "")
    if not value:
        raise WorkflowError(f"OPERATIONAL_VOCABULARY_MISSING: {name}")
    return value


PLAN_SCHEMA = required_env("SLEY_OPERATIONAL_PLAN_SCHEMA")
COMPARISON_SCHEMA = required_env("SLEY_OPERATIONAL_COMPARISON_SCHEMA")
WORKFLOW_ID = required_env("SLEY_OPERATIONAL_WORKFLOW_ID")
AUTHORITY_MODE = required_env("SLEY_OPERATIONAL_AUTHORITY_MODE")
MODES = json.loads(required_env("SLEY_OPERATIONAL_MODES"))
EQUAL_CONTROLS = json.loads(required_env("SLEY_OPERATIONAL_EQUAL_CONTROLS"))
REQUIRED_METRICS = json.loads(required_env("SLEY_OPERATIONAL_REQUIRED_METRICS"))
DECISIONS = json.loads(required_env("SLEY_OPERATIONAL_DECISIONS"))
ORACLE_STEPS = json.loads(required_env("SLEY_OPERATIONAL_ORACLE_STEPS"))
CONTROLLED_DIFFERENCE = json.loads(required_env("SLEY_OPERATIONAL_CONTROLLED_DIFFERENCE"))


def canonical_bytes(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def digest_bytes(value: bytes) -> str:
    return "sha256:" + hashlib.sha256(value).hexdigest()


def regular_file(path: Path, label: str, limit: int = MAX_JSON_BYTES) -> Path:
    if path.is_symlink() or not path.is_file():
        raise WorkflowError(f"INPUT_PATH_INVALID: {label} must be a regular file")
    if path.stat().st_size > limit:
        raise WorkflowError(f"INPUT_LIMIT_EXCEEDED: {label} exceeds {limit} bytes")
    return path.resolve()


def load_json(path: Path, label: str) -> Any:
    source = regular_file(path, label)
    try:
        return json.loads(source.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise WorkflowError(f"JSON_INVALID: {label}: {exc}") from exc


def file_digest(path: Path, label: str = "input") -> str:
    return digest_bytes(regular_file(path, label).read_bytes())


def tree_files(root: Path, label: str) -> dict[str, Path]:
    if root.is_symlink() or not root.is_dir():
        raise WorkflowError(f"INPUT_PATH_INVALID: {label} root is not a directory: {root}")
    files: dict[str, Path] = {}
    for path in sorted(root.rglob("*")):
        if ".git" in path.parts:
            continue
        if path.is_symlink():
            raise WorkflowError(f"INPUT_PATH_INVALID: {label} contains a symlink: {path}")
        if path.is_file():
            files[path.relative_to(root).as_posix()] = path
    return files


def digest_tree_files(files: dict[str, Path]) -> str:
    material = bytearray()
    for relative, path in sorted(files.items()):
        material.extend(relative.encode("utf-8"))
        material.append(0)
        material.extend(hashlib.sha256(path.read_bytes()).hexdigest().encode("ascii"))
        material.append(10)
    return digest_bytes(bytes(material))


def tree_digest(root: Path) -> str:
    return digest_tree_files(tree_files(root, "tree"))


def overlaid_tree_digest(baseline: Path, candidate: Path) -> str:
    files = tree_files(baseline, "baseline workspace")
    files.update(tree_files(candidate, "candidate overlay"))
    return digest_tree_files(files)


def load_schema_store(schema_dir: Path) -> dict[str, Any]:
    store: dict[str, Any] = {}
    for path in sorted(schema_dir.glob("*.json")):
        value = load_json(path, f"schema {path.name}")
        if isinstance(value, dict) and isinstance(value.get("$id"), str):
            store[value["$id"]] = value
    return store


def validate(schema_id: str, value: Any, schema_dir: Path, label: str) -> None:
    store = load_schema_store(schema_dir)
    schema = store.get(schema_id)
    if schema is None:
        raise WorkflowError(f"SCHEMA_NOT_FOUND: {schema_id}")
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", DeprecationWarning)
        resolver = RefResolver.from_schema(schema, store=store)
        errors = sorted(Draft202012Validator(schema, resolver=resolver).iter_errors(value), key=lambda item: list(item.absolute_path))
    if errors:
        detail = "; ".join(f"/{'/'.join(map(str, item.absolute_path))}: {item.message}" for item in errors[:8])
        raise WorkflowError(f"CONTRACT_INVALID: {label}: {detail}")


def git_commit(repo: Path) -> str:
    completed = subprocess.run(
        ["git", "-C", str(repo), "rev-parse", "HEAD"],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    value = completed.stdout.strip()
    if completed.returncode != 0 or not re.fullmatch(r"[a-f0-9]{40}", value):
        raise WorkflowError("GIT_STATE_INVALID: Sley commit could not be resolved")
    return value


def fixture_paths(repo: Path) -> dict[str, Path]:
    root = repo / "fixtures/operational/agent-workflow-v1"
    return {
        "root": root,
        "case": root / "case-manifest.json",
        "raw_run": root / "run-k1.json",
        "structural_run": root / "run-k3.json",
        "workspace": root / "workspace",
        "candidate": root / "candidate",
        "prompt": root / "prompt.md",
        "bootstrap": repo / "SLEY_AI.md",
    }


def require_strong_oracle(case: dict[str, Any]) -> None:
    commands = case["oracle"]["commands"]
    names = [item["name"] for item in commands]
    if names != ORACLE_STEPS:
        raise WorkflowError("ORACLE_INVALID: exact compile, structural, call-site, and behavior steps are required")
    check = commands[0]
    tasks = commands[1]
    calls = commands[2]
    behavior = commands[3]
    if check["argv"] != ["sley", "check", "--json", "."]:
        raise WorkflowError("ORACLE_INVALID: candidate check command changed")
    if tasks["argv"] != ["sley", "query", "--json", "--kind", "tasks", "--module", "agent.pipeline", "."]:
        raise WorkflowError("ORACLE_INVALID: pipeline task inventory query changed")
    if calls["argv"] != ["sley", "query", "--json", "--kind", "calls", "--module", "agent.main", "."]:
        raise WorkflowError("ORACLE_INVALID: main call inventory query changed")
    if any(item["expected_exit"] != 0 for item in commands):
        raise WorkflowError("ORACLE_INVALID: every oracle step must require a zero exit")
    if [item["expected_schema"] for item in commands] != [
        "sley.diagnostics.report.v0",
        "sley.query.report.v0",
        "sley.query.report.v0",
        "sley.test.report.v1",
    ]:
        raise WorkflowError("ORACLE_INVALID: oracle output schemas changed")
    check_expected = {item["pointer"]: item["value"] for item in check["assertions"]}
    if check_expected != {"/status": "ok"}:
        raise WorkflowError("ORACLE_INVALID: candidate check assertion changed")
    task_values = [item["value"] for item in tasks["assertions"] if item["pointer"] == "/tasks"]
    call_values = [item["value"] for item in calls["assertions"] if item["pointer"] == "/calls"]
    if len(task_values) != 1 or len(call_values) != 1:
        raise WorkflowError("ORACLE_INVALID: exact structural inventory assertions are required")
    task_names = [item.get("qualified_name") for item in task_values[0]]
    call_targets = [item.get("target") for item in call_values[0]]
    if task_names.count("agent.pipeline.compose_plan") != 1 or "agent.pipeline.draft_plan" in task_names:
        raise WorkflowError("ORACLE_INVALID: renamed task inventory is not exact")
    if call_targets.count("agent.pipeline.compose_plan") != 1 or "agent.pipeline.draft_plan" in call_targets:
        raise WorkflowError("ORACLE_INVALID: renamed call target inventory is not exact")
    if behavior["argv"] != ["sley", "test", "--json", "."]:
        raise WorkflowError("ORACLE_INVALID: locked behavior test command changed")
    expected = {item["pointer"]: item["value"] for item in behavior["assertions"]}
    if expected != {"/status": "passed", "/summary/passed_count": 1, "/summary/failed_count": 0}:
        raise WorkflowError("ORACLE_INVALID: locked behavior assertions changed")
    if case["minimality"] != {"max_changed_files": 2, "max_changed_lines": 4}:
        raise WorkflowError("ORACLE_INVALID: rename minimality boundary changed")


def load_controls(repo: Path, controller: Path) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any], dict[str, Path]]:
    paths = fixture_paths(repo)
    schema_dir = repo / "docs/schemas"
    manifest = load_json(paths["case"], "case manifest")
    raw_run = load_json(paths["raw_run"], "raw run manifest")
    structural_run = load_json(paths["structural_run"], "structural run manifest")
    validate("sley.agent_bench.case_manifest.v0", manifest, schema_dir, "case manifest")
    validate("sley.agent_bench.run_manifest.v0", raw_run, schema_dir, "raw run manifest")
    validate("sley.agent_bench.run_manifest.v0", structural_run, schema_dir, "structural run manifest")
    if len(manifest["cases"]) != 1:
        raise WorkflowError("CASE_MANIFEST_INVALID: exactly one controlled case is required")
    case = manifest["cases"][0]
    require_strong_oracle(case)
    manifest_digest = file_digest(paths["case"], "case manifest")
    if raw_run["case_manifest_digest"] != manifest_digest or structural_run["case_manifest_digest"] != manifest_digest:
        raise WorkflowError("RUN_PIN_MISMATCH: run manifests do not pin the case manifest")
    same_keys = ["suite_id", "case_manifest_path", "case_manifest_digest", "adapter", "bootstrap", "tool_allowlist", "retrieval", "retain_workspaces"]
    if any(raw_run[key] != structural_run[key] for key in same_keys):
        raise WorkflowError("CONTROL_MISMATCH: paired run controls are not equal")
    if raw_run["mode"] != "K1" or structural_run["mode"] != "K3":
        raise WorkflowError("CONTROL_MISMATCH: paired modes must be K1 and K3")
    if raw_run["run_id"] == structural_run["run_id"] or raw_run["system_prompt"] == structural_run["system_prompt"]:
        raise WorkflowError("CONTROL_MISMATCH: mode-specific run and prompt identities are required")
    if raw_run["tool_allowlist"] != ["query"] or case["allowed_tools"] != ["query"]:
        raise WorkflowError("CONTROL_MISMATCH: query must be the only structural tool")
    candidate_files = set(tree_files(paths["candidate"], "candidate overlay"))
    if candidate_files != set(case["owned_paths"]):
        raise WorkflowError("CANDIDATE_SCOPE_MISMATCH: candidate overlay must contain exactly the owned files")
    expected_candidate_digest = overlaid_tree_digest(paths["workspace"], paths["candidate"])
    if case["candidate_digest"] != expected_candidate_digest:
        raise WorkflowError("CANDIDATE_DIGEST_MISMATCH: trusted final workspace tree changed")
    if case["training_exclusion_id"] != file_digest(paths["prompt"], "prompt"):
        raise WorkflowError("PROMPT_DIGEST_MISMATCH: training exclusion does not pin the prompt")
    if raw_run["bootstrap"]["digest"] != file_digest(paths["bootstrap"], "bootstrap"):
        raise WorkflowError("BOOTSTRAP_DIGEST_MISMATCH: run bootstrap pin changed")
    regular_file(controller, "controller")
    return manifest, raw_run, structural_run, paths


def build_plan(repo: Path, controller: Path) -> dict[str, Any]:
    manifest, raw_run, structural_run, paths = load_controls(repo, controller)
    case = manifest["cases"][0]
    mode_map = {item.split("|", 1)[0]: item.split("|", 1)[1] for item in MODES}
    differences = {item.split("|", 1)[0]: item.split("|")[1:] for item in CONTROLLED_DIFFERENCE}
    if mode_map != {"raw_source": "K1", "sley_structural": "K3"} or set(differences) != {"K1", "K3"}:
        raise WorkflowError("OPERATIONAL_VOCABULARY_INVALID: arm vocabulary is incomplete")
    plan = {
        "schema": PLAN_SCHEMA,
        "status": "ready_for_approval",
        "workflow_id": WORKFLOW_ID,
        "identity": {
            "sley_commit": git_commit(repo),
            "case_manifest_digest": file_digest(paths["case"], "case manifest"),
            "workspace_digest": tree_digest(paths["workspace"]),
            "candidate_digest": overlaid_tree_digest(paths["workspace"], paths["candidate"]),
            "prompt_digest": file_digest(paths["prompt"], "prompt"),
            "bootstrap_digest": file_digest(paths["bootstrap"], "bootstrap"),
            "controller_digest": file_digest(controller, "controller"),
        },
        "task": {
            "case_id": case["id"],
            "suite_id": manifest["suite_id"],
            "owned_paths": case["owned_paths"],
            "oracle_commands": [item["argv"] for item in case["oracle"]["commands"]],
        },
        "arms": [
            {
                "id": "raw_source", "mode": raw_run["mode"], "interface": differences["K1"][0],
                "run_id": raw_run["run_id"], "run_manifest_digest": file_digest(paths["raw_run"], "raw run manifest"),
                "model": raw_run["adapter"]["model"], "thinking": raw_run["adapter"]["thinking"],
                "tool_allowlist": raw_run["tool_allowlist"],
                "spend_ceiling_usd": raw_run["adapter"]["incremental_spend_ceiling_usd"],
                "max_inference_turns": int(differences["K1"][2]), "read_only_tool_turns": int(differences["K1"][1]),
            },
            {
                "id": "sley_structural", "mode": structural_run["mode"], "interface": differences["K3"][0],
                "run_id": structural_run["run_id"], "run_manifest_digest": file_digest(paths["structural_run"], "structural run manifest"),
                "model": structural_run["adapter"]["model"], "thinking": structural_run["adapter"]["thinking"],
                "tool_allowlist": structural_run["tool_allowlist"],
                "spend_ceiling_usd": structural_run["adapter"]["incremental_spend_ceiling_usd"],
                "max_inference_turns": int(differences["K3"][2]), "read_only_tool_turns": int(differences["K3"][1]),
            },
        ],
        "equal_controls": {"verified": True, "fields": EQUAL_CONTROLS},
        "controlled_difference": {
            "raw_source_tool_turns": 0, "structural_tool_turns": 1,
            "raw_source_inference_turns": 1, "structural_inference_turns": 2,
            "same_model_context_window": True, "same_total_spend_ceiling": True,
            "observed_context_consumption_measured": True,
            "manifest_limits_fully_enforced": False,
        },
        "authority": {
            "mode": AUTHORITY_MODE, "provider_execution_authorized": False,
            "repository_mutation": False, "product_runtime_mutation": False,
            "deploy": False, "publication": False, "approval_required": True,
        },
        "disclosure": {"public_fixture": True, "contains_real_user_data": False, "contains_credentials": False, "training_eligible": False},
        "issues": [{
            "code": "MANIFEST_LIMIT_ENFORCEMENT_PARTIAL",
            "message": "The private controller enforces its fixed interface and spend ledger but does not generically enforce every declared case-manifest resource limit",
        }],
    }
    validate(PLAN_SCHEMA, plan, repo / "docs/schemas", "workflow plan")
    return plan


def approval_digest(path: Path, plan: dict[str, Any]) -> str:
    approval = load_json(path, "approval record")
    raw_arm, structural_arm = plan["arms"]
    expected = {
        "schema": PRIVATE_APPROVAL_SCHEMA,
        "status": "approved",
        "workflow_id": WORKFLOW_ID,
        "identity": {
            "sley_commit": plan["identity"]["sley_commit"],
            "case_manifest_digest": plan["identity"]["case_manifest_digest"],
            "raw_run_manifest_digest": raw_arm["run_manifest_digest"],
            "structural_run_manifest_digest": structural_arm["run_manifest_digest"],
            "controller_digest": plan["identity"]["controller_digest"],
        },
        "execution": {
            "provider": "openai",
            "model": raw_arm["model"],
            "thinking": raw_arm["thinking"],
            "run_ids": [raw_arm["run_id"], structural_arm["run_id"]],
            "max_total_spend_usd": raw_arm["spend_ceiling_usd"] + structural_arm["spend_ceiling_usd"],
            "provider_execution_authorized": True,
            "operator_authority_confirmed": True,
        },
        "denied_actions": {
            "repository_mutation": True,
            "product_runtime_mutation": True,
            "deploy": True,
            "publication": True,
        },
    }
    if approval != expected:
        raise WorkflowError("APPROVAL_RECORD_INVALID: private approval record does not exactly match the pinned plan and authority boundary")
    return file_digest(path, "approval record")


def execution_provenance_digest(
    path: Path,
    plan: dict[str, Any],
    approval_value_digest: str,
    raw_arm: dict[str, Any],
    structural_arm: dict[str, Any],
) -> str:
    provenance = load_json(path, "execution provenance")
    reviewer = provenance.get("review", {}).get("reviewer_id") if isinstance(provenance, dict) else None
    if not isinstance(reviewer, str) or not re.fullmatch(r"[A-Za-z0-9._-]{3,128}", reviewer):
        raise WorkflowError("EXECUTION_PROVENANCE_INVALID: independent reviewer identity is missing or invalid")
    expected = {
        "schema": PRIVATE_EXECUTION_PROVENANCE_SCHEMA,
        "status": "completed",
        "workflow_id": WORKFLOW_ID,
        "identity": {
            "sley_commit": plan["identity"]["sley_commit"],
            "case_manifest_digest": plan["identity"]["case_manifest_digest"],
            "raw_run_manifest_digest": plan["arms"][0]["run_manifest_digest"],
            "structural_run_manifest_digest": plan["arms"][1]["run_manifest_digest"],
            "controller_digest": plan["identity"]["controller_digest"],
            "execution_approval_digest": approval_value_digest,
        },
        "execution": {
            "provider": "openai",
            "model": plan["arms"][0]["model"],
            "thinking": plan["arms"][0]["thinking"],
            "run_ids": [plan["arms"][0]["run_id"], plan["arms"][1]["run_id"]],
            "controller_executed": True,
            "provider_execution_observed": True,
        },
        "evidence": {
            "raw_aggregate_digest": raw_arm["aggregate_digest"],
            "raw_evidence_bundle_digest": raw_arm["evidence_bundle_digest"],
            "raw_inference_attempts": raw_arm["inference_attempts"],
            "structural_aggregate_digest": structural_arm["aggregate_digest"],
            "structural_evidence_bundle_digest": structural_arm["evidence_bundle_digest"],
            "structural_inference_attempts": structural_arm["inference_attempts"],
        },
        "review": {
            "independent": True,
            "reviewer_role": "independent_evidence_reviewer",
            "reviewer_id": reviewer,
            "verdict": "passed",
            "high_findings": 0,
            "medium_findings": 0,
            "raw_evidence_inspected": True,
            "provider_origin_inspected": True,
        },
        "assurance": {
            "assertion_based": True,
            "cryptographically_verified": False,
        },
    }
    if provenance != expected:
        raise WorkflowError("EXECUTION_PROVENANCE_INVALID: provenance does not exactly bind the approved execution, source evidence, and independent review")
    return file_digest(path, "execution provenance")


def evidence_metrics(directory: Path, structural_mode: bool, tool_calls: int) -> dict[str, Any]:
    if directory.is_symlink() or not directory.is_dir():
        raise WorkflowError("EVIDENCE_PATH_INVALID: evidence root must be a directory")
    files = sorted(item for item in directory.iterdir() if item.is_file())
    if any(item.is_symlink() or item.stat().st_size > MAX_EVIDENCE_FILE_BYTES for item in files):
        raise WorkflowError("EVIDENCE_PATH_INVALID: evidence file is unsafe or exceeds its limit")
    allowed = re.compile(r"^(prompt|response|model-error)-[12]\.txt$")
    if any(not allowed.fullmatch(item.name) for item in files):
        raise WorkflowError("EVIDENCE_PATH_INVALID: unexpected private evidence file")
    prompts = [item for item in files if item.name.startswith("prompt-")]
    responses = [item for item in files if item.name.startswith("response-")]
    if not prompts:
        raise WorkflowError("EVIDENCE_INCOMPLETE: at least one prompt is required")
    structural_bytes = 0
    prompt_two = directory / "prompt-2.txt"
    require_structural_report = structural_mode and tool_calls == 1
    if prompt_two.is_file() and not require_structural_report:
        raise WorkflowError("EVIDENCE_INVALID: second-turn evidence disagrees with the aggregate tool count")
    if require_structural_report:
        if not prompt_two.is_file():
            raise WorkflowError("EVIDENCE_INCOMPLETE: exercised structural turn requires prompt-2.txt")
        text = prompt_two.read_text(encoding="utf-8")
        match = re.search(r"<interaction_history>\n(.+?)\n</interaction_history>", text, re.DOTALL)
        if not match:
            raise WorkflowError("EVIDENCE_INVALID: structural interaction history is missing")
        try:
            history = json.loads(match.group(1))
            interaction = history[-1]
            model = interaction["model"]
            report = interaction["tool_report"]
            if model.get("action") != "tool" or model.get("tool") != "query":
                raise WorkflowError("EVIDENCE_INVALID: structural history does not contain a query tool action")
            if model.get("argv") != ["--json", "--kind", "tasks", "."]:
                raise WorkflowError("EVIDENCE_INVALID: structural history does not contain the bounded task query")
            if not isinstance(report, dict) or report.get("schema") != "sley.query.report.v0":
                raise WorkflowError("EVIDENCE_INVALID: structural tool report schema is not sley.query.report.v0")
            structural_bytes = len(canonical_bytes(report))
        except WorkflowError:
            raise
        except (KeyError, IndexError, TypeError, json.JSONDecodeError) as exc:
            raise WorkflowError("EVIDENCE_INVALID: structural tool report could not be measured") from exc
    return {
        "inference_attempts": len(prompts),
        "prompt_bytes": sum(item.stat().st_size for item in prompts),
        "response_bytes": sum(item.stat().st_size for item in responses),
        "structural_response_bytes": structural_bytes,
        "bundle_digest": tree_digest(directory),
    }


def invalid_action_count(case: dict[str, Any]) -> int:
    codes = {"SLEYBENCH_MODEL_RESPONSE_INVALID", "SLEYBENCH_TOOL_NOT_ALLOWED", "SLEYBENCH_SCOPE_VIOLATION"}
    count = sum(1 for item in case.get("issues", []) if item.get("code") in codes)
    if case["measurements"].get("invalid_tool_call") and count == 0:
        count = 1
    return count


def require_bound_case_result(
    aggregate: dict[str, Any],
    result: dict[str, Any],
    contract_case: dict[str, Any],
    run: dict[str, Any],
    paths: dict[str, Path],
) -> None:
    expected_evidence = {
        "training_exclusion_id": contract_case["training_exclusion_id"],
        "initial_tree_digest": tree_digest(paths["workspace"]),
        "candidate_digest": overlaid_tree_digest(paths["workspace"], paths["candidate"]),
        "final_tree_digest": overlaid_tree_digest(paths["workspace"], paths["candidate"]),
        "prompt_digest": file_digest(paths["prompt"], "prompt"),
    }
    if any(result["evidence"].get(key) != value for key, value in expected_evidence.items()):
        raise WorkflowError(f"CASE_EVIDENCE_MISMATCH: {run['mode']} result is not bound to the controlled workspace, candidate, and prompt")
    if aggregate["evidence"]["bootstrap_digest"] != run["bootstrap"]["digest"]:
        raise WorkflowError(f"AGGREGATE_IDENTITY_MISMATCH: {run['mode']} aggregate bootstrap digest changed")

    definitions = [
        (definition, phase)
        for phase, commands in (("preflight", contract_case["preflight"]), ("oracle", contract_case["oracle"]["commands"]))
        for definition in commands
    ]
    recorded = result["steps"]
    if len(recorded) != len(definitions):
        raise WorkflowError(f"ORACLE_EVIDENCE_MISMATCH: {run['mode']} result does not contain the exact controlled step set")
    for step, (definition, phase) in zip(recorded, definitions, strict=True):
        expected_assertions = len(definition.get("assertions", []))
        if (
            step["name"] != definition["name"]
            or step["phase"] != phase
            or step["argv"] != definition["argv"]
            or step["expected_exit"] != definition["expected_exit"]
            or step["actual_exit"] != definition["expected_exit"]
            or step["status"] != "passed"
            or step["stdout_schema"] != definition.get("expected_schema")
            or step["assertion_count"] != expected_assertions
            or step["assertion_passed_count"] != expected_assertions
            or step["issues"]
        ):
            raise WorkflowError(f"ORACLE_EVIDENCE_MISMATCH: {run['mode']} controlled step evidence is missing, changed, or failed")
    if result["evidence"]["step_evidence_digest"] != digest_bytes(canonical_bytes(recorded)):
        raise WorkflowError(f"ORACLE_EVIDENCE_MISMATCH: {run['mode']} step evidence digest is invalid")
    if result["measurements"]["compiler_cycles"] != len(recorded):
        raise WorkflowError(f"ORACLE_EVIDENCE_MISMATCH: {run['mode']} compiler cycle count disagrees with its steps")

    passed = result["status"] == "passed"
    expected_counts = {"case_count": 1, "passed_count": int(passed), "failed_count": int(not passed)}
    if any(aggregate["counts"][key] != value for key, value in expected_counts.items()):
        raise WorkflowError(f"AGGREGATE_SUMMARY_MISMATCH: {run['mode']} aggregate counts disagree with its singleton result")
    expected_resources = {
        "tool_calls_total": result["measurements"]["tool_calls"],
        "compiler_cycles_total": result["measurements"]["compiler_cycles"],
        "wall_ms_total": result["measurements"]["wall_ms"],
        "input_tokens_total": result["measurements"]["input_tokens"],
        "output_tokens_total": result["measurements"]["output_tokens"],
        "estimated_cost_usd_total": result["measurements"]["estimated_cost_usd"],
    }
    if any(aggregate["resources"][key] != value for key, value in expected_resources.items()):
        raise WorkflowError(f"AGGREGATE_SUMMARY_MISMATCH: {run['mode']} aggregate resources disagree with its singleton result")
    if (
        aggregate["status"] != result["status"]
        or aggregate["adapter"] != {
            "kind": run["adapter"]["kind"],
            "id": run["adapter"]["id"],
            "model_measurement_applicable": True,
        }
        or result["adapter_id"] != run["adapter"]["id"]
        or result["family"] != contract_case["family"]
    ):
        raise WorkflowError(f"AGGREGATE_SUMMARY_MISMATCH: {run['mode']} aggregate metadata disagrees with the controlled result")


def build_arm(
    aggregate_path: Path,
    evidence_dir: Path,
    run: dict[str, Any],
    interface: str,
    contract_case: dict[str, Any],
    paths: dict[str, Path],
    structural: bool,
    schema_dir: Path,
) -> tuple[dict[str, Any], list[dict[str, str]]]:
    aggregate = load_json(aggregate_path, f"{run['mode']} aggregate")
    validate("sley.agent_bench.aggregate.v0", aggregate, schema_dir, f"{run['mode']} aggregate")
    expected_run_digest = file_digest(fixture_paths(schema_dir.parent.parent)["structural_run" if structural else "raw_run"], "run manifest")
    if aggregate["run_id"] != run["run_id"] or aggregate["mode"] != run["mode"] or aggregate["evidence"]["run_manifest_digest"] != expected_run_digest:
        raise WorkflowError(f"AGGREGATE_IDENTITY_MISMATCH: {run['mode']} aggregate disagrees with its run manifest")
    if aggregate["evidence"]["case_manifest_digest"] != run["case_manifest_digest"] or len(aggregate["cases"]) != 1:
        raise WorkflowError(f"AGGREGATE_IDENTITY_MISMATCH: {run['mode']} case evidence is not the controlled singleton")
    case = aggregate["cases"][0]
    if case["case_id"] != "sleybench-v1-f6-001" or case["mode"] != run["mode"] or case["run_id"] != run["run_id"]:
        raise WorkflowError(f"CASE_IDENTITY_MISMATCH: {run['mode']} result is not the controlled case")
    require_bound_case_result(aggregate, case, contract_case, run, paths)
    measurement = case["measurements"]
    tool_calls = measurement["tool_calls"]
    evidence = evidence_metrics(evidence_dir, structural, tool_calls)
    tool_exercised = tool_calls == 1 if structural else tool_calls == 0
    strict = bool(
        case["status"] == "passed"
        and case["scores"]["oracle_tests_passed"]
        and case["scores"]["scope_precise"]
        and case["scores"]["required_evidence_produced"]
        and not measurement["invalid_tool_call"]
        and tool_exercised
    )
    total_tokens = measurement["input_tokens"] + measurement["output_tokens"]
    workspace_files = [item for item in paths["workspace"].rglob("*") if item.is_file() and ".git" not in item.parts]
    issues: list[dict[str, str]] = []
    if structural and not tool_exercised:
        issues.append({"code": "STRUCTURAL_TOOL_NOT_EXERCISED", "message": "K3 did not complete exactly one read-only structural query; the attempt remains in the denominator"})
    arm = {
        "mode": run["mode"], "interface": interface, "run_id": run["run_id"],
        "aggregate_digest": file_digest(aggregate_path, f"{run['mode']} aggregate"),
        "case_result_digest": digest_bytes(canonical_bytes(case)),
        "evidence_bundle_digest": evidence["bundle_digest"],
        "accepted_correct_change": strict, "strict_success": strict,
        "inference_attempts": evidence["inference_attempts"],
        "input_tokens": measurement["input_tokens"], "output_tokens": measurement["output_tokens"],
        "total_tokens": total_tokens, "accepted_change_tokens": total_tokens if strict else None,
        "prompt_bytes": evidence["prompt_bytes"], "response_bytes": evidence["response_bytes"],
        "context_files_supplied": len(workspace_files), "context_bytes_supplied": sum(item.stat().st_size for item in workspace_files),
        "structural_response_bytes": evidence["structural_response_bytes"],
        "wall_ms": measurement["wall_ms"], "tool_calls": tool_calls,
        "compiler_cycles": measurement["compiler_cycles"], "invalid_actions": invalid_action_count(case),
        "repair_loops": 0, "changed_files": measurement["changed_files"], "changed_lines": measurement["changed_lines"],
        "human_review_applicable": False, "human_review_ms": None,
    }
    missing = set(REQUIRED_METRICS) - set(arm)
    if missing:
        raise WorkflowError(f"METRIC_MISSING: {run['mode']} lacks {sorted(missing)}")
    return arm, issues


def environment() -> dict[str, Any]:
    os_name = platform.freedesktop_os_release().get("PRETTY_NAME", platform.system())
    memory_bytes = 0
    try:
        for line in Path("/proc/meminfo").read_text(encoding="utf-8").splitlines():
            if line.startswith("MemTotal:"):
                memory_bytes = int(line.split()[1]) * 1024
                break
    except (OSError, ValueError, IndexError):
        pass
    if memory_bytes <= 0:
        raise WorkflowError("ENVIRONMENT_UNAVAILABLE: total memory could not be measured")
    return {
        "scope": "comparison_host",
        "os": os_name, "kernel": platform.release(), "architecture": platform.machine(),
        "logical_cpus": os.cpu_count() or 1, "memory_bytes": memory_bytes,
    }


def compare(args: argparse.Namespace) -> dict[str, Any]:
    repo = args.repo_root.resolve()
    plan = build_plan(repo, args.controller.resolve())
    manifest, raw_run, structural_run, paths = load_controls(repo, args.controller.resolve())
    execution_approval_digest = approval_digest(args.approval_record.resolve(), plan)
    contract_case = manifest["cases"][0]
    raw_arm, raw_issues = build_arm(args.raw_aggregate.resolve(), args.raw_evidence.resolve(), raw_run, "raw_workspace", contract_case, paths, False, repo / "docs/schemas")
    structural_arm, structural_issues = build_arm(args.structural_aggregate.resolve(), args.structural_evidence.resolve(), structural_run, "raw_workspace_plus_read_only_structure", contract_case, paths, True, repo / "docs/schemas")
    execution_provenance_value_digest = execution_provenance_digest(
        args.execution_provenance.resolve(), plan, execution_approval_digest, raw_arm, structural_arm
    )
    if structural_arm["strict_success"] and not raw_arm["strict_success"]:
        outcome = "structural_advantage_observed"
    elif structural_arm["strict_success"] and raw_arm["strict_success"]:
        outcome = "both_accepted"
    elif raw_arm["strict_success"]:
        outcome = "structural_disadvantage_observed"
    else:
        outcome = "both_failed"
    if outcome not in DECISIONS:
        raise WorkflowError("OPERATIONAL_VOCABULARY_INVALID: decision is not registered")
    delta_fields = [
        "inference_attempts", "input_tokens", "output_tokens", "total_tokens", "prompt_bytes", "response_bytes",
        "context_files_supplied", "context_bytes_supplied", "structural_response_bytes",
        "wall_ms", "tool_calls", "compiler_cycles", "invalid_actions", "repair_loops",
        "changed_files", "changed_lines",
    ]
    issues = raw_issues + structural_issues + [
        {"code": "SINGLE_TRIAL_ONLY", "message": "One controlled trial cannot support a general model-fluency or structural-advantage claim"},
        {"code": "POST_SUBMIT_REPAIR_UNAVAILABLE", "message": "The one-shot controller exposes no post-submit repair loop; repair_loops is reported as zero and not inferred"},
        {"code": "MANIFEST_LIMIT_ENFORCEMENT_PARTIAL", "message": "Equal task, model-context, and spend budgets are verified; generic case-manifest resource-limit enforcement is not claimed"},
        {"code": "PROVIDER_ORIGIN_ASSERTION_NOT_CRYPTOGRAPHIC", "message": "Provider origin is bound by an exact operator and independent-review provenance assertion, not by a cryptographic provider attestation"},
    ]
    if structural_arm["prompt_bytes"] > raw_arm["prompt_bytes"]:
        issues.append({"code": "STRUCTURAL_CONTEXT_EXPANDED", "message": "The structural arm consumed more prompt bytes than the raw arm; this negative context result is retained"})
    prompt_delta = structural_arm["prompt_bytes"] - raw_arm["prompt_bytes"]
    context_effect = "expansion" if prompt_delta > 0 else "saving" if prompt_delta < 0 else "equal"
    result = {
        "schema": COMPARISON_SCHEMA, "status": "passed", "workflow_id": WORKFLOW_ID,
        "identity": {
            "sley_commit": plan["identity"]["sley_commit"],
            "case_manifest_digest": plan["identity"]["case_manifest_digest"],
            "raw_run_manifest_digest": plan["arms"][0]["run_manifest_digest"],
            "structural_run_manifest_digest": plan["arms"][1]["run_manifest_digest"],
            "controller_digest": plan["identity"]["controller_digest"],
            "execution_approval_digest": execution_approval_digest,
            "execution_provenance_digest": execution_provenance_value_digest,
        },
        "controls": {
            "equal_task_context_spend_budget_verified": True, "same_model": True, "same_task": True,
            "same_workspace": True, "same_oracle": True, "same_owned_paths": True,
            "action_budget_is_controlled_difference": True,
        },
        "arms": {"raw_source": raw_arm, "sley_structural": structural_arm},
        "deltas": {field: structural_arm[field] - raw_arm[field] for field in delta_fields},
        "decision": {
            "outcome": outcome,
            "recommendation": "retain_for_bounded_followup" if structural_arm["strict_success"] else "defer_structural_interface",
            "structural_context_effect": context_effect,
            "promotion_decision": "deferred", "general_claim_supported": False, "production_claim": False,
        },
        "methodology": {
            "trial_count": 1, "failures_retained": True, "cache_state": "fresh_per_arm",
            "context_budget_basis": "same_model_context_window_and_spend_ceiling",
            "manifest_limit_enforcement": "partial_controller_bounds",
            "repair_loop_observability": "post_submit_repair_unavailable", "single_trial_generalization": False,
        },
        "environment": environment(),
        "authority": {"mode": AUTHORITY_MODE, "provider_execution_authorized": True, "repository_mutation": False, "product_runtime_mutation": False, "deploy": False, "publication": False},
        "disclosure": {"raw_prompts_embedded": False, "raw_responses_embedded": False, "contains_real_user_data": False, "contains_credentials": False, "publication_authorized": False},
        "issues": issues,
    }
    validate(COMPARISON_SCHEMA, result, repo / "docs/schemas", "workflow comparison")
    return result


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", required=True, type=Path)
    parser.add_argument("command", choices=("plan", "compare"))
    parser.add_argument("--controller", required=True, type=Path)
    parser.add_argument("--raw-aggregate", type=Path)
    parser.add_argument("--raw-evidence", type=Path)
    parser.add_argument("--structural-aggregate", type=Path)
    parser.add_argument("--structural-evidence", type=Path)
    parser.add_argument("--approval-record", type=Path)
    parser.add_argument("--execution-provenance", type=Path)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    if args.command == "compare":
        required = [args.raw_aggregate, args.raw_evidence, args.structural_aggregate, args.structural_evidence, args.approval_record, args.execution_provenance]
        if any(value is None for value in required):
            parser.error("compare requires both aggregate/evidence pairs, --approval-record, and --execution-provenance")
    return args


def main() -> int:
    args = parse_args()
    repo = args.repo_root.resolve()
    if not repo.is_dir() or repo.is_symlink():
        raise WorkflowError("REPO_ROOT_INVALID: repository root is unavailable")
    result = build_plan(repo, args.controller.resolve()) if args.command == "plan" else compare(args)
    print(json.dumps(result, indent=2))
    return 0


try:
    raise SystemExit(main())
except WorkflowError as exc:
    print(str(exc), file=sys.stderr)
    raise SystemExit(2)
PY
