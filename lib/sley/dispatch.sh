# shellcheck shell=bash
# Public command-name-to-family dispatch only.

main_sley() {
  need_jq
  local cmd="${1:-}"
  case "$cmd" in
    ""|--help|-h|help) usage ;;
    --version|-V|version) printf '%s\n' "$VERSION" ;;
    parse|ast) shift; command_ast "$@" ;;
    check) shift; command_check "$@" ;;
    explain) shift; command_explain "$@" ;;
    doctor) shift; command_doctor "$@" ;;
    graph) shift; command_graph "$@" ;;
    graph-diff) shift; command_graph_diff "$@" ;;
    query) shift; command_query "$@" ;;
    lint) shift; command_lint "$@" ;;
    plan) shift; command_plan "$@" ;;
    adapter) shift; command_adapter "$@" ;;
    worker) shift; command_worker "$@" ;;
    test) shift; command_test "$@" ;;
    validate) shift; command_validate "$@" ;;
    release) shift; command_release "$@" ;;
    reference-replay) shift; command_reference_replay "$@" ;;
    operational-workflow) shift; command_operational_workflow "$@" ;;
    operational-evidence) shift; command_operational_evidence "$@" ;;
    change) shift; command_change "$@" ;;
    verify) shift; command_verify "$@" ;;
    claim-verify|claim-audit) shift; command_claim_verify "$@" ;;
    run) shift; command_run "$@" ;;
    machine) shift; command_machine "$@" ;;
    deploy) shift; command_deploy "$@" ;;
    trace) shift; command_trace "$@" ;;
    seal) shift; command_seal "$@" ;;
    zjx) shift; command_zjx "$@" ;;
    arena) shift; command_arena "$@" ;;
    graft) shift; command_graft "$@" ;;
    fix) shift; command_graft "$@" ;;
    self-hosting|self-hosting-status) shift; self_hosting_status_json "$@" ;;
    format) shift; command_format "$@" ;;
    new) shift; command_new "$@" ;;
    sley-ci) shift; tool_ci "$@" ;;
    sley-conformance) shift; tool_conformance "$@" ;;
    sley-contract) shift; tool_contract "$@" ;;
    sley-docgen) shift; command_docgen "$@" ;;
    sley-lsp) shift; command_lsp "$@" ;;
    sley-workbench) shift; command_workbench "$@" ;;
    sley-agent-bench) shift; command_agent_bench "$@" ;;
    sley-migrate) shift; command_migrate "$@" ;;
    sley-sandbox-runner) shift; command_sandbox "$@" ;;
    sley-shadow) shift; command_shadow "$@" ;;
    sley-zjx) shift; command_zjx_tool "$@" ;;
    *) echo "unknown sley command: $cmd" >&2; return 2 ;;
  esac
}
