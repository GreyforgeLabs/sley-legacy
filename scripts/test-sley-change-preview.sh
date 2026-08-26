#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
SLEY="$ROOT_DIR/bin/sley"
CONTRACT="$ROOT_DIR/bin/sley-contract"
SCHEMA_DIR="$ROOT_DIR/docs/schemas"
TARGET="$ROOT_DIR/examples/project"
OPERATION="$ROOT_DIR/fixtures/transactions/cross_module_preview.json"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

fail() {
  echo "change preview contract failed: $*" >&2
  exit 1
}

tree_digest() {
  local target="$1"
  find "$target" -type f -print0 | sort -z | xargs -0 sha256sum | sha256sum | awk '{print $1}'
}

file_digest_or_absent() {
  local path="$1"
  if [[ -f "$path" ]]; then sha256sum "$path" | awk '{print $1}'; else printf 'absent\n'; fi
}

expect_diagnostic() {
  local expected_id="$1"
  shift
  local output="$WORK_DIR/error.json"
  if "$SLEY" "$@" >"$output" 2>/dev/null; then
    fail "command unexpectedly succeeded for $expected_id"
  fi
  jq -e --arg id "$expected_id" '
    .schema == "sley.diagnostics.report.v0"
    and .status == "error"
    and any(.diagnostics[]; .id == $id and .severity == "error")
  ' "$output" >/dev/null || fail "missing typed diagnostic $expected_id"
}

git_index="$(git -C "$ROOT_DIR" rev-parse --git-path index)"
source_before="$(tree_digest "$TARGET")"
index_before="$(file_digest_or_absent "$git_index")"

"$SLEY" change inspect --json \
  --goal "preview cross-module structural edit" \
  --actor "operator:test" \
  --nonce "w4-preview-001" \
  --created-at "2026-08-25T00:00:00Z" \
  --expires-at "2099-01-01T00:00:00Z" \
  "$TARGET" >"$WORK_DIR/inspect.json"

"$SLEY" change plan --json \
  --repository "$ROOT_DIR" \
  --inspection "$WORK_DIR/inspect.json" \
  --operation "$OPERATION" \
  --requested-outcome "rename the exported task and add a factor parameter across its caller" \
  --assumption "the caller remains in app.main" \
  --non-goal "do not apply candidate source to the repository" \
  >"$WORK_DIR/plan.json"

"$SLEY" change plan --json \
  --repository "$ROOT_DIR" \
  --inspection "$WORK_DIR/inspect.json" \
  --operation "$OPERATION" \
  --requested-outcome "rename the exported task and add a factor parameter across its caller" \
  --assumption "the caller remains in app.main" \
  --non-goal "do not apply candidate source to the repository" \
  >"$WORK_DIR/plan-repeat.json"
cmp -s "$WORK_DIR/plan.json" "$WORK_DIR/plan-repeat.json" || fail "identical plan inputs were not deterministic"

"$SLEY" change preview --json --repository "$ROOT_DIR" --plan "$WORK_DIR/plan.json" >"$WORK_DIR/preview.json"
"$SLEY" change preview --json --repository "$ROOT_DIR" --plan "$WORK_DIR/plan.json" >"$WORK_DIR/preview-repeat.json"
cmp -s "$WORK_DIR/preview.json" "$WORK_DIR/preview-repeat.json" || fail "candidate preview leaked temporary workspace identity"

"$SLEY" change approval-request --json --repository "$ROOT_DIR" \
  --preview "$WORK_DIR/preview.json" >"$WORK_DIR/approval-request.json"
"$SLEY" change approval-request --json --repository "$ROOT_DIR" \
  --preview "$WORK_DIR/preview.json" >"$WORK_DIR/approval-request-repeat.json"
cmp -s "$WORK_DIR/approval-request.json" "$WORK_DIR/approval-request-repeat.json" \
  || fail "identical approval request inputs were not deterministic"

approve_args=(
  change approve --json --repository "$ROOT_DIR"
  --request "$WORK_DIR/approval-request.json"
  --preview "$WORK_DIR/preview.json"
  --confirm-exact-local-grant
  --issuer "operator:test"
  --principal "agent:test"
  --audience "sley:local"
  --purpose "authorize only this immutable cross-module candidate"
  --nonce "w4-grant-001"
  --created-at "2026-08-25T00:00:01Z"
  --expires-at "2098-01-01T00:00:00Z"
  --ack-review "human_source_review"
  --ack-review "graph_diff_review"
  --ack-review "lint_review"
  --ack-review "authority_review"
  --revocation-state "not_revoked"
  --max-wall-clock-seconds "300"
)
"$SLEY" "${approve_args[@]}" >"$WORK_DIR/grant.json"
"$SLEY" "${approve_args[@]}" >"$WORK_DIR/grant-repeat.json"
cmp -s "$WORK_DIR/grant.json" "$WORK_DIR/grant-repeat.json" \
  || fail "identical explicit local grant inputs were not deterministic"

apply_authorization_args=(
  change apply-authorization --json --repository "$ROOT_DIR"
  --grant "$WORK_DIR/grant.json"
  --request "$WORK_DIR/approval-request.json"
  --preview "$WORK_DIR/preview.json"
  --principal "agent:test"
  --audience "sley:local"
)
"$SLEY" "${apply_authorization_args[@]}" >"$WORK_DIR/apply-authorization.json"
"$SLEY" "${apply_authorization_args[@]}" >"$WORK_DIR/apply-authorization-repeat.json"
cmp -s "$WORK_DIR/apply-authorization.json" "$WORK_DIR/apply-authorization-repeat.json" \
  || fail "identical apply authorization inputs were not deterministic"

"$CONTRACT" validate --schema sley.change.plan.v0 "$WORK_DIR/plan.json" --schemas "$SCHEMA_DIR" --json >/dev/null
"$CONTRACT" validate --schema sley.change.preview.v0 "$WORK_DIR/preview.json" --schemas "$SCHEMA_DIR" --json >/dev/null
"$CONTRACT" validate --schema sley.change.approval_request.v0 "$WORK_DIR/approval-request.json" --schemas "$SCHEMA_DIR" --json >/dev/null
"$CONTRACT" validate --schema sley.change.grant.v0 "$WORK_DIR/grant.json" --schemas "$SCHEMA_DIR" --json >/dev/null
"$CONTRACT" validate --schema sley.change.apply_authorization.v0 "$WORK_DIR/apply-authorization.json" --schemas "$SCHEMA_DIR" --json >/dev/null

jq -e '
  .schema == "sley.change.plan.v0"
  and .status == "planned"
  and .transaction.history == ["opened", "inspected", "planned"]
  and (.plan.operations | map(.op)) == ["RenameDeclaration", "UpdateCallSites", "AddTake", "UpdateCallArgs"]
  and .plan.affected_files == ["examples/project/src/app/main.sley", "examples/project/src/app/math.sley"]
  and .mutations == {repository_source:false,git_index:false,trace_sidecars:false,external_systems:false,provider_state:false}
' "$WORK_DIR/plan.json" >/dev/null || fail "plan facts or mutation boundary are incomplete"

jq -e '
  .schema == "sley.change.preview.v0"
  and .status == "previewed"
  and .transaction.history == ["opened", "inspected", "planned", "previewed"]
  and .candidate.workspace_kind == "isolated_ephemeral_copy"
  and .candidate.materialized == false
  and .candidate.owned_paths == ["examples/project/src/app/main.sley", "examples/project/src/app/math.sley"]
  and (.candidate.source_projection | map(.content) | join("\n") | contains("task scale"))
  and (.candidate.source_projection | map(.content) | join("\n") | contains("take factor: Int"))
  and (.candidate.source_projection | map(.content) | join("\n") | contains("call math.scale(2, 21)"))
  and .diffs.source.changed_files == .candidate.owned_paths
  and (.diffs.types | length) > 0
  and (.diffs.calls | length) > 0
  and .diffs.effects == []
  and .diffs.authority == []
  and .validation.required_passed == true
  and .mutations == {repository_source:false,git_index:false,trace_sidecars:false,external_systems:false,provider_state:false}
  and ([.review_requirements[].kind] | sort) == ["authority_review", "graph_diff_review", "human_source_review", "lint_review"]
' "$WORK_DIR/preview.json" >/dev/null || fail "immutable candidate evidence is incomplete"

jq -e '
  .schema == "sley.change.approval_request.v0"
  and .status == "awaiting_approval"
  and .request_id == ("approval-request:" + (.request_digest | sub("^sha256:"; "")))
  and .transaction.history == ["opened", "inspected", "planned", "previewed", "awaiting_approval"]
  and .preview_ref.candidate_digest == .candidate_ref.candidate_digest
  and .derived_scope.operation_classes == ["AddTake", "RenameDeclaration", "UpdateCallArgs", "UpdateCallSites"]
  and .derived_scope.allowed_paths == ["examples/project/src/app/main.sley", "examples/project/src/app/math.sley"]
  and .derived_scope.resource_scopes == [{kind:"repository_source", paths:.derived_scope.allowed_paths}]
  and .derived_scope.max_changed_files == 2
  and .derived_scope.max_source_bytes > 0
  and .required_operator_fields == [
    "confirm_exact_local_grant", "issuer", "principal", "audience", "purpose", "nonce",
    "created_at", "expires_at", "review_acknowledgements", "revocation_state", "max_wall_clock_seconds"
  ]
  and .mutations == {repository_source:false,git_index:false,trace_sidecars:false,external_systems:false,provider_state:false}
' "$WORK_DIR/approval-request.json" >/dev/null || fail "compiler-derived approval request is incomplete"

jq -e '
  .schema == "sley.change.grant.v0"
  and .status == "approved"
  and .grant_id == ("grant:" + (.grant_digest | sub("^sha256:"; "")))
  and .transaction.history == ["opened", "inspected", "planned", "previewed", "awaiting_approval", "approved"]
  and .issuer == {id:"operator:test",source:"explicit_cli_assertion"}
  and .principal == {id:"agent:test"}
  and .audience == {id:"sley:local"}
' "$WORK_DIR/grant.json" >/dev/null || fail "explicit local grant principals are incomplete"
jq -e --slurpfile request "$WORK_DIR/approval-request.json" '
  .approval_request_ref.request_digest == $request[0].request_digest
  and .candidate.candidate_digest == $request[0].candidate_ref.candidate_digest
  and .repository == $request[0].transaction.repository
  and .base == $request[0].transaction.base
  and .allowed.operation_classes == $request[0].derived_scope.operation_classes
  and .allowed.nodes == $request[0].derived_scope.affected_nodes
  and .allowed.paths == $request[0].derived_scope.allowed_paths
  and .allowed.resource_scopes == $request[0].derived_scope.resource_scopes
  and .allowed.review_requirements_acknowledged == $request[0].derived_scope.review_requirements
  and .bounds.max_changed_files == $request[0].derived_scope.max_changed_files
  and .bounds.max_source_bytes == $request[0].derived_scope.max_source_bytes
  and .bounds.max_candidate_source_bytes == $request[0].derived_scope.max_source_bytes
  and .bounds.max_apply_attempts == 1
  and .bounds.max_wall_clock_seconds == 300
  and .replay.policy == "single_use_at_apply"
  and .replay.enforcement == "deferred_until_s12_404"
  and (.replay.replay_key | test("^sha256:[a-f0-9]{64}$"))
  and .revocation == {state:"not_revoked",checked_at:"2026-08-25T00:00:01Z",source:{kind:"operator_assertion",ref:"operator:test"}}
  and .proofs == []
  and .mutations == {repository_source:false,git_index:false,trace_sidecars:false,external_systems:false,provider_state:false}
' "$WORK_DIR/grant.json" >/dev/null || fail "exact local grant scope or deferred enforcement boundary is incomplete"

jq -e --slurpfile grant "$WORK_DIR/grant.json" '
  .schema == "sley.change.apply_authorization.v0"
  and .status == "ready"
  and .authorization_id == ("apply-authorization:" + (.authorization_digest | sub("^sha256:"; "")))
  and .grant_ref.grant_digest == $grant[0].grant_digest
  and .grant_ref.replay_key == $grant[0].replay.replay_key
  and .principal == $grant[0].principal
  and .audience == $grant[0].audience
  and .source_scope == $grant[0].allowed.resource_scopes[0]
  and (.internal_scopes | map(.kind)) == [
    "transaction_state", "transaction_lock", "replay_ledger", "recovery_journal", "revocation_ledger"
  ]
  and .atomicity == {
    platform:"linux",strategy:"renameat2_exchange",topology:"same_filesystem_common_root",
    rollback_strategy:"verified_exchange",cancellation_point:"before_commit_point",
    limitations:["non_repo_root","single_common_root","no_symlinks","no_special_files","no_submodules","same_filesystem"]
  }
  and .verification == {
    profile:"compiler_apply_v0",
    required_checks:["candidate_source_digest","candidate_graph_digest","compiler_check","compiler_lint"],
    rollback_on_failure:true
  }
  and .mutation_authority.repository_source == true
  and .mutation_authority.internal_transaction_state == true
  and .mutations == {repository_source:false,git_index:false,trace_sidecars:false,external_systems:false,provider_state:false}
' "$WORK_DIR/apply-authorization.json" >/dev/null || fail "strict apply authorization scope is incomplete"

source_after="$(tree_digest "$TARGET")"
index_after="$(file_digest_or_absent "$git_index")"
[[ "$source_before" == "$source_after" ]] || fail "preview changed repository source"
[[ "$index_before" == "$index_after" ]] || fail "preview changed the Git index"

jq '.plan.requested_outcome = "tampered"' "$WORK_DIR/plan.json" >"$WORK_DIR/tampered-plan.json"
expect_diagnostic TRANSACTION_PLAN_DIGEST_MISMATCH change preview --json --repository "$ROOT_DIR" --plan "$WORK_DIR/tampered-plan.json"

jq '.ops[0].op = "ShellCommand"' "$OPERATION" >"$WORK_DIR/unsupported-operation.json"
expect_diagnostic TRANSACTION_OPERATION_INVALID change plan --json --repository "$ROOT_DIR" \
  --inspection "$WORK_DIR/inspect.json" --operation "$WORK_DIR/unsupported-operation.json" \
  --requested-outcome edit --assumption bounded --non-goal apply

ln -s "$WORK_DIR/plan.json" "$WORK_DIR/plan-link.json"
expect_diagnostic TRANSACTION_ARTIFACT_SYMLINK_DENIED change preview --json --repository "$ROOT_DIR" --plan "$WORK_DIR/plan-link.json"
expect_diagnostic TRANSACTION_ARTIFACT_REQUIRED change approve --json
expect_diagnostic TRANSACTION_ATOMIC_APPLY_CONFIRMATION_REQUIRED change apply --json
expect_diagnostic TRANSACTION_APPLY_PRINCIPAL_MISMATCH change apply-authorization --json --repository "$ROOT_DIR" \
  --grant "$WORK_DIR/grant.json" --request "$WORK_DIR/approval-request.json" --preview "$WORK_DIR/preview.json" \
  --principal "agent:other" --audience "sley:local"

jq '.allowed.paths += ["README.md"] | .allowed.paths |= unique' "$WORK_DIR/grant.json" >"$WORK_DIR/tampered-grant-core.json"
tampered_grant_digest="$(jq 'del(.grant_id,.grant_digest)' "$WORK_DIR/tampered-grant-core.json" | jq -cS . | sha256sum | awk '{print "sha256:" $1}')"
jq --arg digest "$tampered_grant_digest" \
  '.grant_digest=$digest | .grant_id=("grant:" + ($digest | sub("^sha256:"; "")))' \
  "$WORK_DIR/tampered-grant-core.json" >"$WORK_DIR/tampered-grant.json"
"$CONTRACT" validate --schema sley.change.grant.v0 "$WORK_DIR/tampered-grant.json" --schemas "$SCHEMA_DIR" --json >/dev/null
expect_diagnostic TRANSACTION_GRANT_SCOPE_MISMATCH change apply-authorization --json --repository "$ROOT_DIR" \
  --grant "$WORK_DIR/tampered-grant.json" --request "$WORK_DIR/approval-request.json" --preview "$WORK_DIR/preview.json" \
  --principal "agent:test" --audience "sley:local"

approve_without_confirmation=("${approve_args[@]}")
unset 'approve_without_confirmation[9]'
expect_diagnostic TRANSACTION_EXACT_GRANT_CONFIRMATION_REQUIRED "${approve_without_confirmation[@]}"

approve_missing_review=(
  change approve --json --repository "$ROOT_DIR"
  --request "$WORK_DIR/approval-request.json" --preview "$WORK_DIR/preview.json"
  --confirm-exact-local-grant --issuer "operator:test" --principal "agent:test" --audience "sley:local"
  --purpose "candidate text cannot authorize itself" --nonce "w4-grant-002"
  --created-at "2026-08-25T00:00:01Z" --expires-at "2098-01-01T00:00:00Z"
  --ack-review "human_source_review" --ack-review "graph_diff_review" --ack-review "lint_review"
  --revocation-state "not_revoked" --max-wall-clock-seconds "300"
)
expect_diagnostic TRANSACTION_REVIEW_ACKNOWLEDGEMENT_REQUIRED "${approve_missing_review[@]}"

jq '.derived_scope.allowed_paths += ["README.md"] | .derived_scope.allowed_paths |= unique | .derived_scope.resource_scopes[0].paths = .derived_scope.allowed_paths' \
  "$WORK_DIR/approval-request.json" >"$WORK_DIR/tampered-request-core.json"
tampered_request_digest="$(jq 'del(.request_id,.request_digest)' "$WORK_DIR/tampered-request-core.json" | jq -cS . | sha256sum | awk '{print "sha256:" $1}')"
jq --arg digest "$tampered_request_digest" \
  '.request_digest=$digest | .request_id=("approval-request:" + ($digest | sub("^sha256:"; "")))' \
  "$WORK_DIR/tampered-request-core.json" >"$WORK_DIR/tampered-request.json"
"$CONTRACT" validate --schema sley.change.approval_request.v0 "$WORK_DIR/tampered-request.json" --schemas "$SCHEMA_DIR" --json >/dev/null
tampered_approve_args=("${approve_args[@]}")
tampered_approve_args[6]="$WORK_DIR/tampered-request.json"
expect_diagnostic TRANSACTION_APPROVAL_REQUEST_MISMATCH "${tampered_approve_args[@]}"

expect_diagnostic TRANSACTION_APPROVAL_REQUEST_INVALID change approve --json --repository "$ROOT_DIR" \
  --request "$WORK_DIR/preview.json" --preview "$WORK_DIR/preview.json"

jq -n '{
  transaction:"single-file-preview",actor:"operator:test",mode:"all_or_nothing",
  ops:[{op:"RenameDeclaration",target:"task:app.math.double",payload:{name:"scale"}}]
}' >"$WORK_DIR/file-operation.json"
"$SLEY" change inspect --json \
  --goal "preview one file" --actor "operator:test" --nonce "w4-file-preview-001" \
  --created-at "2026-08-25T00:00:00Z" --expires-at "2099-01-01T00:00:00Z" \
  "$TARGET/src/app/math.sley" >"$WORK_DIR/file-inspect.json"
"$SLEY" change plan --json --repository "$ROOT_DIR" \
  --inspection "$WORK_DIR/file-inspect.json" --operation "$WORK_DIR/file-operation.json" \
  --requested-outcome "rename one file task" --assumption "the task exists" \
  --non-goal "do not write the source file" >"$WORK_DIR/file-plan.json"
"$SLEY" change preview --json --repository "$ROOT_DIR" \
  --plan "$WORK_DIR/file-plan.json" >"$WORK_DIR/file-preview.json"
"$CONTRACT" validate --schema sley.change.preview.v0 "$WORK_DIR/file-preview.json" --schemas "$SCHEMA_DIR" --json >/dev/null
jq -e '
  .candidate.owned_paths == ["examples/project/src/app/math.sley"]
  and (.candidate.source_projection | length) == 1
  and .candidate.source_projection[0].path == "examples/project/src/app/math.sley"
  and (.candidate.source_projection[0].content | contains("task scale"))
  and .validation.required_passed == true
' "$WORK_DIR/file-preview.json" >/dev/null || fail "single-file candidate preview is incomplete"
[[ "$source_before" == "$(tree_digest "$TARGET")" ]] || fail "single-file preview changed repository source"
[[ "$index_before" == "$(file_digest_or_absent "$git_index")" ]] || fail "single-file preview changed the Git index"

echo "change plan, immutable preview, and exact local grant contracts passed"
