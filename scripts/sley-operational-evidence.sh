#!/usr/bin/env bash
set -euo pipefail

python3 - "$@" <<'PY'
"""Assemble the final non-authoritative W6 operational evidence registry."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
import warnings
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, RefResolver


MAX_JSON_BYTES = 16 * 1024 * 1024


class EvidenceError(RuntimeError):
    pass


def required_env(name: str) -> str:
    value = os.environ.get(name, "")
    if not value:
        raise EvidenceError(f"OPERATIONAL_VOCABULARY_MISSING: {name}")
    return value


REGISTRY_SCHEMA = required_env("SLEY_OPERATIONAL_REGISTRY_SCHEMA")
REGISTRY_RELEASE = required_env("SLEY_OPERATIONAL_REGISTRY_RELEASE")
AUTHORITY_MODE = required_env("SLEY_OPERATIONAL_REGISTRY_AUTHORITY_MODE")
ENTRY_IDS = json.loads(required_env("SLEY_OPERATIONAL_REGISTRY_ENTRY_IDS"))
DECISION_RATIONALE = json.loads(required_env("SLEY_OPERATIONAL_REGISTRY_DECISION_RATIONALE"))


def digest_bytes(value: bytes) -> str:
    return "sha256:" + hashlib.sha256(value).hexdigest()


def regular_file(path: Path, label: str) -> Path:
    if path.is_symlink() or not path.is_file():
        raise EvidenceError(f"INPUT_PATH_INVALID: {label} must be a regular non-symlink file")
    if path.stat().st_size > MAX_JSON_BYTES:
        raise EvidenceError(f"INPUT_LIMIT_EXCEEDED: {label} exceeds {MAX_JSON_BYTES} bytes")
    return path.resolve()


def repo_file(path: Path, repo: Path, label: str) -> Path:
    source = regular_file(path, label)
    try:
        source.relative_to(repo)
    except ValueError as exc:
        raise EvidenceError(f"INPUT_PATH_OUTSIDE_REPOSITORY: {label}") from exc
    return source


def load_json(path: Path, label: str) -> Any:
    source = regular_file(path, label)
    try:
        return json.loads(source.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise EvidenceError(f"JSON_INVALID: {label}: {exc}") from exc


def file_digest(path: Path, label: str) -> str:
    return digest_bytes(regular_file(path, label).read_bytes())


def tree_digest(root: Path, label: str) -> str:
    if root.is_symlink() or not root.is_dir():
        raise EvidenceError(f"INPUT_PATH_INVALID: {label} must be a directory")
    material = bytearray()
    for path in sorted(item for item in root.rglob("*") if item.is_file()):
        if path.is_symlink():
            raise EvidenceError(f"INPUT_PATH_INVALID: {label} contains a symbolic link")
        material.extend(path.relative_to(root).as_posix().encode("utf-8"))
        material.append(0)
        material.extend(hashlib.sha256(path.read_bytes()).hexdigest().encode("ascii"))
        material.append(10)
    return digest_bytes(bytes(material))


def schema_store(schema_dir: Path) -> dict[str, Any]:
    store: dict[str, Any] = {}
    for path in sorted(schema_dir.glob("*.json")):
        value = load_json(path, f"schema {path.name}")
        if isinstance(value, dict) and isinstance(value.get("$id"), str):
            store[value["$id"]] = value
    return store


def validate(schema_id: str, value: Any, schema_dir: Path, label: str) -> None:
    store = schema_store(schema_dir)
    schema = store.get(schema_id)
    if schema is None:
        raise EvidenceError(f"SCHEMA_NOT_FOUND: {schema_id}")
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", DeprecationWarning)
        resolver = RefResolver.from_schema(schema, store=store)
        errors = sorted(Draft202012Validator(schema, resolver=resolver).iter_errors(value), key=lambda item: list(item.absolute_path))
    if errors:
        detail = "; ".join(f"/{'/'.join(map(str, item.absolute_path))}: {item.message}" for item in errors[:8])
        raise EvidenceError(f"CONTRACT_INVALID: {label}: {detail}")


def git(repo: Path, *args: str) -> str:
    completed = subprocess.run(
        ["git", "-C", str(repo), *args],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if completed.returncode != 0:
        raise EvidenceError(f"GIT_STATE_INVALID: {' '.join(args)}")
    return completed.stdout.strip()


def git_commit(repo: Path) -> str:
    value = git(repo, "rev-parse", "HEAD")
    if not re.fullmatch(r"[a-f0-9]{40}", value):
        raise EvidenceError("GIT_STATE_INVALID: HEAD is not a commit")
    return value


def require_ancestor(repo: Path, commit: str, label: str) -> None:
    completed = subprocess.run(
        ["git", "-C", str(repo), "merge-base", "--is-ancestor", commit, "HEAD"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    if completed.returncode != 0:
        raise EvidenceError(f"EVIDENCE_COMMIT_NOT_ANCESTOR: {label}")


def require_reference(reference: dict[str, Any], repo: Path) -> None:
    if reference["status"] != "passed" or reference["workload"] != {
        "id": "siglum.numerology",
        "version": "siglum.numerology.reference-v1",
        "scope": "complete_frozen_reference_corpus",
        "case_count": 10000,
    }:
        raise EvidenceError("REFERENCE_EVIDENCE_INVALID: workload identity or status changed")
    correctness = reference["correctness"]
    if (
        correctness["repeat_count"] < 2
        or not correctness["deterministic"]
        or correctness["exact_match_count"] != 10000
        or correctness["mismatch_count"] != 0
        or not reference["oracle"]["independent"]
    ):
        raise EvidenceError("REFERENCE_EVIDENCE_INVALID: exact independent replay proof is absent")
    if reference["promotion"] != {
        "decision": "deferred",
        "production_claim": False,
        "missing_evidence": ["production_shadow", "production_latency", "fallback", "promotion_authority", "rollback"],
    }:
        raise EvidenceError("REFERENCE_EVIDENCE_INVALID: production deferral changed")
    authority = reference["authority"]
    denied = ["candidate_authoritative", "repository_mutation", "product_runtime_mutation", "network_authority", "provider_calls", "deploy", "spend"]
    if authority["mode"] != "local_replay_only" or not authority["incumbent_authoritative"] or any(authority[key] for key in denied):
        raise EvidenceError("REFERENCE_EVIDENCE_INVALID: authority boundary changed")
    if reference["disclosure"]["publication_authorized"]:
        raise EvidenceError("REFERENCE_EVIDENCE_INVALID: publication authority is not allowed")
    require_ancestor(repo, reference["identity"]["sley_commit"], "reference replay")


def fixture_digests(repo: Path) -> dict[str, str]:
    root = repo / "fixtures/operational/agent-workflow-v1"
    return {
        "case_manifest_digest": file_digest(root / "case-manifest.json", "case manifest"),
        "raw_run_manifest_digest": file_digest(root / "run-k1.json", "K1 run manifest"),
        "structural_run_manifest_digest": file_digest(root / "run-k3.json", "K3 run manifest"),
    }


def require_comparison(comparison: dict[str, Any], repo: Path, controller: Path) -> None:
    if any(item["code"] == "SYNTHETIC_CONTRACT_FIXTURE" for item in comparison["issues"]):
        raise EvidenceError("AGENT_COMPARISON_SYNTHETIC: contract fixtures cannot satisfy W6 evidence")
    if comparison["status"] != "passed" or comparison["workflow_id"] != ENTRY_IDS[1]:
        raise EvidenceError("AGENT_COMPARISON_INVALID: workflow identity or status changed")
    expected_identity = fixture_digests(repo)
    expected_identity["controller_digest"] = file_digest(controller, "controller")
    if any(comparison["identity"].get(key) != value for key, value in expected_identity.items()):
        raise EvidenceError("AGENT_COMPARISON_IDENTITY_MISMATCH: fixture or controller digest changed")
    require_ancestor(repo, comparison["identity"]["sley_commit"], "agent workflow")
    controls = comparison["controls"]
    if not all(controls.values()):
        raise EvidenceError("AGENT_COMPARISON_CONTROL_MISMATCH: declared controls are incomplete")
    methodology = comparison["methodology"]
    if methodology["trial_count"] != 1 or not methodology["failures_retained"] or methodology["single_trial_generalization"]:
        raise EvidenceError("AGENT_COMPARISON_METHODOLOGY_INVALID: one retained non-general trial is required")
    if comparison["decision"]["promotion_decision"] != "deferred" or comparison["decision"]["general_claim_supported"] or comparison["decision"]["production_claim"]:
        raise EvidenceError("AGENT_COMPARISON_DECISION_INVALID: comparison must remain non-promotional")
    authority = comparison["authority"]
    if (
        authority["mode"] != "local_isolated_evaluation_only"
        or not authority["provider_execution_authorized"]
        or authority["repository_mutation"]
        or authority["product_runtime_mutation"]
        or authority["deploy"]
        or authority["publication"]
    ):
        raise EvidenceError("AGENT_COMPARISON_AUTHORITY_INVALID: execution boundary changed")
    disclosure = comparison["disclosure"]
    if any(disclosure.values()):
        raise EvidenceError("AGENT_COMPARISON_DISCLOSURE_INVALID: scrubbed comparison boundary changed")
    raw = comparison["arms"]["raw_source"]
    structural = comparison["arms"]["sley_structural"]
    if raw["mode"] != "K1" or raw["run_id"] != "sley-operational-agent-workflow-v1-k1" or structural["mode"] != "K3" or structural["run_id"] != "sley-operational-agent-workflow-v1-k3":
        raise EvidenceError("AGENT_COMPARISON_ARM_INVALID: exact K1/K3 runs are required")


def reassemble_comparison(args: argparse.Namespace, repo: Path) -> dict[str, Any]:
    command = [
        str(repo / "scripts/sley-operational-workflow.sh"),
        "--repo-root", str(repo),
        "compare",
        "--controller", str(args.controller),
        "--raw-aggregate", str(args.raw_aggregate),
        "--raw-evidence", str(args.raw_evidence),
        "--structural-aggregate", str(args.structural_aggregate),
        "--structural-evidence", str(args.structural_evidence),
        "--approval-record", str(args.approval_record),
        "--execution-provenance", str(args.execution_provenance),
        "--json",
    ]
    completed = subprocess.run(
        command,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
        timeout=120,
        env=os.environ.copy(),
    )
    if completed.returncode != 0:
        raise EvidenceError("AGENT_COMPARISON_REVALIDATION_FAILED: private comparison evidence did not reassemble")
    try:
        value = json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        raise EvidenceError("AGENT_COMPARISON_REVALIDATION_FAILED: reassembled output was not JSON") from exc
    if not isinstance(value, dict):
        raise EvidenceError("AGENT_COMPARISON_REVALIDATION_FAILED: reassembled output was not an object")
    return value


def arm_summary(arm: dict[str, Any]) -> dict[str, Any]:
    return {
        "mode": arm["mode"],
        "strict_success": arm["strict_success"],
        "accepted_change_tokens": arm["accepted_change_tokens"],
        "inference_attempts": arm["inference_attempts"],
        "total_tokens": arm["total_tokens"],
        "prompt_bytes": arm["prompt_bytes"],
        "context_bytes_supplied": arm["context_bytes_supplied"],
        "structural_response_bytes": arm["structural_response_bytes"],
        "tool_calls": arm["tool_calls"],
        "invalid_actions": arm["invalid_actions"],
        "repair_loops": arm["repair_loops"],
        "wall_ms": arm["wall_ms"],
    }


def unique(values: list[str]) -> list[str]:
    return list(dict.fromkeys(values))


def build_registry(args: argparse.Namespace, repo: Path) -> dict[str, Any]:
    schema_dir = repo / "docs/schemas"
    reference_source = repo_file(args.reference_replay, repo, "reference replay")
    comparison_source = repo_file(args.agent_comparison, repo, "agent comparison")
    controller_source = regular_file(args.controller, "controller")
    reference = load_json(reference_source, "reference replay")
    comparison = load_json(comparison_source, "agent comparison")
    validate("sley.operational.reference_replay.v1", reference, schema_dir, "reference replay")
    validate("sley.operational.agent_workflow_comparison.v1", comparison, schema_dir, "agent comparison")
    require_reference(reference, repo)
    require_comparison(comparison, repo, controller_source)
    reassembled = reassemble_comparison(args, repo)
    if reassembled != comparison:
        raise EvidenceError("AGENT_COMPARISON_REVALIDATION_MISMATCH: retained comparison differs from its private source evidence")
    reference_limitations = unique(
        reference["promotion"]["missing_evidence"]
        + [item["code"] for item in reference["negative_evidence"]]
    )
    agent_limitations = unique([item["code"] for item in comparison["issues"]])
    mandatory_limitations = {
        "SINGLE_TRIAL_ONLY",
        "MANIFEST_LIMIT_ENFORCEMENT_PARTIAL",
        "PROVIDER_ORIGIN_ASSERTION_NOT_CRYPTOGRAPHIC",
    }
    if not mandatory_limitations.issubset(agent_limitations):
        raise EvidenceError("AGENT_COMPARISON_LIMITATION_MISSING: mandatory negative evidence is absent")
    promotion = {"decision": "deferred", "general_claim_supported": False, "production_claim": False}
    registry = {
        "schema": REGISTRY_SCHEMA,
        "status": "decision_recorded",
        "release": REGISTRY_RELEASE,
        "identity": {
            "registry_source_commit": git_commit(repo),
            "reference_replay_digest": file_digest(reference_source, "reference replay"),
            "agent_workflow_comparison_digest": file_digest(comparison_source, "agent comparison"),
            "agent_workflow_source_commit": comparison["identity"]["sley_commit"],
            "case_manifest_digest": comparison["identity"]["case_manifest_digest"],
            "controller_digest": comparison["identity"]["controller_digest"],
            "execution_provenance_digest": comparison["identity"]["execution_provenance_digest"],
            "assembler_digest": file_digest(repo / "scripts/sley-operational-evidence.sh", "registry assembler"),
            "comparison_assembler_digest": file_digest(repo / "scripts/sley-operational-workflow.sh", "comparison assembler"),
            "registry_schema_digest": file_digest(schema_dir / "sley.operational.evidence_registry.v1.schema.json", "registry schema"),
            "schema_set_digest": tree_digest(schema_dir, "schema set"),
            "operational_vocabulary_digest": file_digest(repo / "self-hosted/src/loom/operational.sley", "operational vocabulary"),
        },
        "entries": {
            "reference_replay": {
                "id": ENTRY_IDS[0],
                "workload_class": "deterministic_reference_replay",
                "authority_status": "non_authoritative",
                "artifact": {
                    "schema": reference["schema"],
                    "path": reference_source.relative_to(repo).as_posix(),
                    "digest": file_digest(reference_source, "reference replay"),
                    "status": reference["status"],
                },
                "correctness": {
                    "repeat_count": reference["correctness"]["repeat_count"],
                    "case_count": reference["workload"]["case_count"],
                    "exact_match_count": reference["correctness"]["exact_match_count"],
                    "mismatch_count": reference["correctness"]["mismatch_count"],
                    "deterministic": reference["correctness"]["deterministic"],
                    "independent_oracle": reference["oracle"]["independent"],
                },
                "promotion": promotion,
                "known_limitations": reference_limitations,
                "reproducibility": {
                    "command": ["bin/sley", "reference-replay", "siglum-numerology", "--siglum-root", "$SIGLUM_ROOT", "--execute", "--json"],
                    "private_inputs_required": False,
                },
                "disclosure": {"public_artifact": True, "private_evidence_required": False, "publication_authorized": False},
            },
            "agent_workflow": {
                "id": ENTRY_IDS[1],
                "workload_class": "controlled_agent_maintenance",
                "authority_status": "local_isolated_evaluation_only",
                "artifact": {
                    "schema": comparison["schema"],
                    "path": comparison_source.relative_to(repo).as_posix(),
                    "digest": file_digest(comparison_source, "agent comparison"),
                    "status": comparison["status"],
                },
                "comparison": {
                    "trial_count": comparison["methodology"]["trial_count"],
                    "failures_retained": comparison["methodology"]["failures_retained"],
                    "outcome": comparison["decision"]["outcome"],
                    "recommendation": comparison["decision"]["recommendation"],
                    "structural_context_effect": comparison["decision"]["structural_context_effect"],
                },
                "arms": {
                    "raw_source": arm_summary(comparison["arms"]["raw_source"]),
                    "sley_structural": arm_summary(comparison["arms"]["sley_structural"]),
                },
                "promotion": promotion,
                "known_limitations": agent_limitations,
                "reproducibility": {
                    "command": ["bin/sley", "operational-workflow", "compare", "--controller", "$PRIVATE_CONTROLLER", "--raw-aggregate", "$K1_AGGREGATE", "--raw-evidence", "$K1_EVIDENCE", "--structural-aggregate", "$K3_AGGREGATE", "--structural-evidence", "$K3_EVIDENCE", "--approval-record", "$PRIVATE_APPROVAL", "--execution-provenance", "$PRIVATE_PROVENANCE", "--json"],
                    "private_inputs_required": True,
                },
                "disclosure": {"public_artifact": True, "private_evidence_required": True, "publication_authorized": False},
            },
        },
        "decision": {
            "w6_evidence_complete": True,
            "release_closeout_unblocked": True,
            "production_promotion": "deferred",
            "structural_workflow_disposition": comparison["decision"]["recommendation"],
            "general_claim_supported": False,
            "production_claim": False,
            "rationale": DECISION_RATIONALE,
        },
        "authority": {
            "mode": AUTHORITY_MODE,
            "repository_mutation": False,
            "product_runtime_mutation": False,
            "provider_calls": False,
            "deploy": False,
            "publication": False,
            "promotion_authority": False,
        },
        "disclosure": {
            "raw_prompts_embedded": False,
            "raw_responses_embedded": False,
            "raw_corpus_embedded": False,
            "contains_real_user_data": False,
            "contains_credentials": False,
            "training_eligible": False,
            "retrieval_eligible": False,
            "publication_authorized": False,
        },
        "issues": [
            {"code": "PRODUCTION_PROMOTION_DEFERRED", "message": "The registry records bounded non-authoritative evidence and grants no product, provider, deploy, publication, or promotion authority."},
            {"code": "SINGLE_AGENT_TRIAL_ONLY", "message": "One controlled K1/K3 trial cannot support a general structural-context or model-fluency claim."},
        ],
    }
    validate(REGISTRY_SCHEMA, registry, schema_dir, "operational evidence registry")
    return registry


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", required=True, type=Path)
    parser.add_argument("command", choices=("assemble",))
    parser.add_argument("--reference-replay", required=True, type=Path)
    parser.add_argument("--agent-comparison", required=True, type=Path)
    parser.add_argument("--controller", required=True, type=Path)
    parser.add_argument("--raw-aggregate", required=True, type=Path)
    parser.add_argument("--raw-evidence", required=True, type=Path)
    parser.add_argument("--structural-aggregate", required=True, type=Path)
    parser.add_argument("--structural-evidence", required=True, type=Path)
    parser.add_argument("--approval-record", required=True, type=Path)
    parser.add_argument("--execution-provenance", required=True, type=Path)
    parser.add_argument("--json", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo = args.repo_root.resolve()
    if not repo.is_dir() or repo.is_symlink():
        raise EvidenceError("REPO_ROOT_INVALID: repository root is unavailable")
    registry = build_registry(args, repo)
    print(json.dumps(registry, indent=2))
    return 0


try:
    raise SystemExit(main())
except EvidenceError as exc:
    print(str(exc), file=sys.stderr)
    raise SystemExit(2)
PY
