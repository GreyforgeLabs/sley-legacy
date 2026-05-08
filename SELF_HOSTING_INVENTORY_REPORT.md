# Self-Hosting Migration Inventory

Generated: 2026-05-08T18:26:45Z

## Foreign-language surface scan

| Extension | Count |
|---|---:|
| *.rs | 0 |
| *.c | 0 |
| *.cc | 0 |
| *.cpp | 0 |
| *.h | 0 |
| *.hpp | 0 |
| *.m | 0 |
| *.mm | 0 |
| *.swift | 0 |
| *.go | 0 |
| *.java | 0 |
| *.kt | 0 |
| *.cs | 0 |
| *.rb | 0 |
| *.php | 0 |
| *.py | 0 |
| *.js | 0 |
| *.mjs | 0 |
| *.ts | 0 |
| *.tsx | 0 |

## Stage-1/2 executable and source surfaces

- bin/sley
- bin/sley-agent-bench
- bin/sley-ci
- bin/sley-conformance
- bin/sley-contract
- bin/sley-docgen
- bin/sley-lsp
- bin/sley-migrate
- bin/sley-sandbox-runner
- bin/sley-shadow
- bin/sley-workbench
- bin/sley-zjx
- self-hosted/src/loom/bootstrap.sley
- self-hosted/src/loom/checker.sley
- self-hosted/src/loom/lint.sley
- self-hosted/src/loom/parser.sley
- self-hosted/src/loom/reports.sley
- self-hosted/src/loom/runtime.sley

## Reported CLI commands in llms.txt

1. `sley check --json <file-or-project>`
2. `sley ast --json <file-or-project>`
3. `sley ast --json --node <node-id> <file-or-project>`
4. `sley graph --json <file-or-project>`
5. `sley graph --json --slice <node-id> <file-or-project>`
6. `sley query --json --kind calls <file-or-project>`
7. `sley lint --json <file-or-project>`
8. `sley plan --json --graft-templates <file-or-project>`
9. `sley new --json --template agent-project --name agent-app --module agent.main agent-app`
10. `sley doctor --json <file-or-project>`
11. `sley fix --json --kind add_module_declaration --dry-run <file-or-project>`
12. `sley fix --json --kind delete_unused_private_declarations --dry-run <file-or-project>`
13. `sley verify --json <file-or-project>`
14. `sley deploy --json --dry-run --artifacts-dir .sley/deploy <file-or-project>`
15. `sley trace --json <file-or-project>`
16. `sley seal --json <file-or-project>`
17. `sley zjx --json <file-or-project>`
18. `sley graft --json --dry-run <file> <graft.json>`
19. `sley format <file>`
20. `sley run --json <file-or-project>`
21. `sley run --json self-hosted`
22. `sley self-hosting-status --json`
23. `sley-ci lint --json --deny-warnings <file-or-project>`
24. `sley-ci doctor --json --deny-warnings <file-or-project>`
25. `sley-ci plan --json --graft-templates <file-or-project>`
26. `sley-ci run --json <file-or-project>`
27. `sley-ci deploy --json --dry-run --artifacts-dir .sley/ci-deploy <file-or-project>`
28. `sley-ci corpus --json fixtures/corpus/manifest.json`
29. `sley-ci examples --json examples`
30. `sley-ci smoke --json fixtures/ci_smoke_probe/manifest.json`
31. `sley-conformance report --json`
32. `sley-conformance coverage --json --require-tag <tag>`
33. `sley-contract inventory --json`
34. `sley-contract check-fixtures fixtures/contracts --schemas docs/schemas --json`
35. `sley-contract validate --schema sley.conformance.manifest.v0 fixtures/corpus/manifest.json --schemas docs/schemas --json`
36. `sley-contract inspect-deploy-artifacts .sley/deploy --schemas docs/schemas --json`
37. `sley-lsp --validate-editor-shims`
38. `sley-workbench --json --html .sley/workbench.html <file-or-project>`
39. `sley-docgen reference --json --markdown .sley/reference.md <file-or-project>`
40. `sley-agent-bench run --json`
41. `sley-migrate report --json --schemas docs/schemas --fixtures fixtures/contracts <file-or-project>`
42. `sley-sandbox-runner run --json <manifest.json>`
43. `sley-zjx inspect --json <zjx-envelope.json>`
44. `sley-zjx verify-digest --json <zjx-envelope.json>`

## Make targets in Makefile

- fmt
- diff-check
- build-cli
- build-bins
- test
- contracts
- conformance
- public-release-check
- corpus
- examples
- smoke
- lsp
- editor-shims
- workbench
- agent-bench
- migrate
- docgen
- sandbox-runner
- shadow
- zjx-tools
- syntax
- self-hosted-cli
- v1

## Current tracked files

.github/actions/sley-v1/action.yml
.github/workflows/v1.yml
.gitignore
.pre-commit-config.yaml
CHANGELOG.md
LICENSE
Makefile
May8Sley.md
NOTICE
PongAfterAudit.md
README.md
SELF_HOSTING_INVENTORY_REPORT.md
SELF_HOSTING_MIGRATION_PLAN.md
SELF_HOSTING_PHASE1_PORT_PLAN.md
assets/branding/canonical/sley_loom_graph_banner_1500x500.png
assets/branding/canonical/sley_loom_graph_board.png
assets/branding/canonical/sley_loom_graph_post_1200x675.png
assets/branding/canonical/sley_loom_graph_profile_1024.png
bin/sley
bin/sley-agent-bench
bin/sley-ci
bin/sley-conformance
bin/sley-contract
bin/sley-docgen
bin/sley-lsp
bin/sley-migrate
bin/sley-sandbox-runner
bin/sley-shadow
bin/sley-workbench
bin/sley-zjx
docs/AgentQuickstart.md
docs/BrandingAssets.md
docs/PublicReleaseChecklist.md
docs/SleyDeveloperUtilitiesRoadmap.md
docs/SleyLanguageSpec.md
docs/contracts.md
docs/schemas/sley.agent_bench.report.v0.schema.json
docs/schemas/sley.ast.node.v0.schema.json
docs/schemas/sley.ast.program.v0.schema.json
docs/schemas/sley.ci.report.v0.schema.json
docs/schemas/sley.cli_smoke.manifest.v0.schema.json
docs/schemas/sley.conformance.coverage.v0.schema.json
docs/schemas/sley.conformance.manifest.v0.schema.json
docs/schemas/sley.conformance.report.v0.schema.json
docs/schemas/sley.contract.fixture_check.v0.schema.json
docs/schemas/sley.contract.inventory.v0.schema.json
docs/schemas/sley.contract.validate.v0.schema.json
docs/schemas/sley.deploy.artifact_check.v0.schema.json
docs/schemas/sley.deploy.artifacts.v0.schema.json
docs/schemas/sley.deploy.report.v0.schema.json
docs/schemas/sley.diagnostics.report.v0.schema.json
docs/schemas/sley.docgen.report.v0.schema.json
docs/schemas/sley.doctor.report.v0.schema.json
docs/schemas/sley.edit_plan.report.v0.schema.json
docs/schemas/sley.graft.outcome.v0.schema.json
docs/schemas/sley.lint.report.v0.schema.json
docs/schemas/sley.lsp.command_preview.v0.schema.json
docs/schemas/sley.lsp.fix_preview.v0.schema.json
docs/schemas/sley.migrate.report.v0.schema.json
docs/schemas/sley.project.scaffold.v0.schema.json
docs/schemas/sley.query.report.v0.schema.json
docs/schemas/sley.run.report.v0.schema.json
docs/schemas/sley.sandbox.manifest.v0.schema.json
docs/schemas/sley.sandbox.report.v0.schema.json
docs/schemas/sley.self_hosting.status.v0.schema.json
docs/schemas/sley.shadow.report.v0.schema.json
docs/schemas/sley.symbol_graph.slice.v0.schema.json
docs/schemas/sley.symbol_graph.v0.schema.json
docs/schemas/sley.trace.receipt.v0.schema.json
docs/schemas/sley.trace.report.v0.schema.json
docs/schemas/sley.trace.seal.v0.schema.json
docs/schemas/sley.verify.report.v0.schema.json
docs/schemas/sley.workbench.report.v0.schema.json
docs/schemas/sley.zjx.envelope.v0.schema.json
docs/schemas/sley.zjx.tool.report.v0.schema.json
examples/absorbing_arithmetic_expression.sley
examples/absorbing_boolean_expression.sley
examples/agent_deploy_pipeline.sley
examples/agent_project/sley.toml
examples/agent_project/src/agent/main.sley
examples/agent_project/src/agent/pipeline.sley
examples/collections.sley
examples/compute.sley
examples/constant_arithmetic_expression.sley
examples/constant_boolean_comparison_expression.sley
examples/constant_comparison_expression.sley
examples/constant_false_if_statement.sley
examples/constant_false_while_statement.sley
examples/constant_if_expression.sley
examples/constant_if_statement.sley
examples/constant_len_expression.sley
examples/constant_list_index_expression.sley
examples/constant_map_index_expression.sley
examples/constant_not_expression.sley
examples/constant_record_field_access_expression.sley
examples/constant_text_concatenation_expression.sley
examples/db_gate.sley
examples/db_write_gate.sley
examples/dead_private_tasks.sley
examples/declaration_hygiene.sley
examples/deploy_gate.sley
examples/double_negation_expression.sley
examples/duplicate_import_project/sley.toml
examples/duplicate_import_project/src/app/main.sley
examples/duplicate_import_project/src/app/shared.sley
examples/empty_else_statement.sley
examples/empty_for_statement.sley
examples/empty_forge_statement.sley
examples/empty_if_statement.sley
examples/empty_while_statement.sley
examples/file_gate.sley
examples/file_write_gate.sley
examples/hello.sley
examples/host_fallibility.sley
examples/idempotent_boolean_expression.sley
examples/identity_binary_expression.sley
examples/maps_for.sley
examples/model_gate.sley
examples/mutable_binding_style.sley
examples/negated_comparison_expression.sley
examples/network_gate.sley
examples/overwritten_set_statement.sley
examples/profile_service.sley
examples/project/sley.toml
examples/project/src/app/main.sley
examples/project/src/app/math.sley
examples/raw_host_migration.sley
examples/redundant_boolean_comparison.sley
examples/redundant_boolean_if_expression.sley
examples/redundant_boolean_if_statement.sley
examples/redundant_initial_set_statement.sley
examples/redundant_initial_set_to_bind.sley
examples/result_flow.sley
examples/same_branch_if_expression.sley
examples/same_branch_if_statement.sley
examples/secret_gate.sley
examples/self_assignment_statement.sley
examples/self_comparison_expression.sley
examples/shell_gate.sley
examples/spend_gate.sley
examples/unchecked_result.sley
examples/unchecked_result_binding.sley
examples/unqualified_import_call_project/sley.toml
examples/unqualified_import_call_project/src/app/main.sley
examples/unqualified_import_call_project/src/app/math.sley
examples/unreachable_statement.sley
examples/unused_declared_effect.sley
examples/unused_import_project/sley.toml
examples/unused_import_project/src/app/main.sley
examples/unused_import_project/src/app/stale.sley
examples/unused_import_project/src/app/used.sley
examples/unused_private_task.sley
examples/unused_pure_binding.sley
examples/unused_pure_expression_statement.sley
examples/unused_take.sley
examples/users.json
fixtures/ci_smoke_probe/graft_target.sley
fixtures/ci_smoke_probe/insert_statement.json
fixtures/ci_smoke_probe/manifest.json
fixtures/cli_smokes/manifest.json
fixtures/contract_probe/query_project_tasks.json
fixtures/contracts/agent_bench_probe.json
fixtures/contracts/ast_minimal_program.json
fixtures/contracts/ast_node_return_expression.json
fixtures/contracts/ci_check_project_ready.json
fixtures/contracts/ci_corpus_manifest_ready.json
fixtures/contracts/ci_deploy_blocked_runtime_gates.json
fixtures/contracts/ci_deploy_denied_empty_for_statement.json
fixtures/contracts/ci_deploy_project_ready.json
fixtures/contracts/ci_deploy_requires_dry_run.json
fixtures/contracts/ci_doctor_denied_empty_for_statement.json
fixtures/contracts/ci_doctor_project_ready.json
fixtures/contracts/ci_examples_ready.json
fixtures/contracts/ci_lint_denied_empty_for_statement.json
fixtures/contracts/ci_lint_project_ready.json
fixtures/contracts/ci_lint_unknown_project_module.json
fixtures/contracts/ci_plan_denied_empty_for_statement.json
fixtures/contracts/ci_plan_project_ready.json
fixtures/contracts/ci_run_project_ready.json
fixtures/contracts/ci_smoke_probe_ready.json
fixtures/contracts/ci_verify_denied_empty_for_statement.json
fixtures/contracts/ci_verify_project_ready.json
fixtures/contracts/conformance_coverage_probe.json
fixtures/contracts/conformance_manifest_probe.json
fixtures/contracts/conformance_report_probe.json
fixtures/contracts/conformance_report_public_release_blocked.json
fixtures/contracts/contract_fixture_check_probe.json
fixtures/contracts/contract_inventory_schemas.json
fixtures/contracts/contract_validate_query_report.json
fixtures/contracts/deploy_artifact_check_hello_passed.json
fixtures/contracts/deploy_artifacts_hello.json
fixtures/contracts/deploy_hello_ready.json
fixtures/contracts/diagnostic_report_deploy_requires_dry_run.json
fixtures/contracts/diagnostic_report_unknown_identifier.json
fixtures/contracts/docgen_agent_project_pipeline_module.json
fixtures/contracts/docgen_agent_reference.json
fixtures/contracts/doctor_denied_empty_for_statement.json
fixtures/contracts/doctor_project_ready.json
fixtures/contracts/edit_plan_project_graft_templates.json
fixtures/contracts/edit_plan_project_ready.json
fixtures/contracts/edit_plan_project_targeted_graft_templates.json
fixtures/contracts/graft_outcome_insert_statement.json
fixtures/contracts/graph_slice_minimal_task.json
fixtures/contracts/graph_slice_project_module.json
fixtures/contracts/graph_slice_project_task.json
fixtures/contracts/lint_absorbing_arithmetic_expression.json
fixtures/contracts/lint_absorbing_boolean_expression.json
fixtures/contracts/lint_constant_arithmetic_expression.json
fixtures/contracts/lint_constant_boolean_comparison_expression.json
fixtures/contracts/lint_constant_comparison_expression.json
fixtures/contracts/lint_constant_false_if_statement.json
fixtures/contracts/lint_constant_false_while_statement.json
fixtures/contracts/lint_constant_if_expression.json
fixtures/contracts/lint_constant_if_statement.json
fixtures/contracts/lint_constant_len_expression.json
fixtures/contracts/lint_constant_list_index_expression.json
fixtures/contracts/lint_constant_map_index_expression.json
fixtures/contracts/lint_constant_not_expression.json
fixtures/contracts/lint_constant_record_field_access_expression.json
fixtures/contracts/lint_constant_text_concatenation_expression.json
fixtures/contracts/lint_double_negation_expression.json
fixtures/contracts/lint_duplicate_import.json
fixtures/contracts/lint_empty_else_statement.json
fixtures/contracts/lint_empty_for_statement.json
fixtures/contracts/lint_empty_forge_statement.json
fixtures/contracts/lint_empty_if_statement.json
fixtures/contracts/lint_empty_while_statement.json
fixtures/contracts/lint_idempotent_boolean_expression.json
fixtures/contracts/lint_identity_binary_expression.json
fixtures/contracts/lint_missing_module_declaration.json
fixtures/contracts/lint_mutable_binding_never_set.json
fixtures/contracts/lint_negated_comparison_expression.json
fixtures/contracts/lint_overwritten_set_statement.json
fixtures/contracts/lint_raw_host_adapter.json
fixtures/contracts/lint_redundant_boolean_comparison.json
fixtures/contracts/lint_redundant_boolean_if_expression.json
fixtures/contracts/lint_redundant_boolean_if_statement.json
fixtures/contracts/lint_redundant_initial_set_statement.json
fixtures/contracts/lint_same_branch_if_expression.json
fixtures/contracts/lint_same_branch_if_statement.json
fixtures/contracts/lint_self_assignment_statement.json
fixtures/contracts/lint_self_comparison_expression.json
fixtures/contracts/lint_unchecked_result.json
fixtures/contracts/lint_unchecked_result_binding.json
fixtures/contracts/lint_unknown_project_module.json
fixtures/contracts/lint_unqualified_imported_call.json
fixtures/contracts/lint_unreachable_private_task.json
fixtures/contracts/lint_unreachable_statement.json
fixtures/contracts/lint_unused_declared_effect.json
fixtures/contracts/lint_unused_effectful_binding.json
fixtures/contracts/lint_unused_import.json
fixtures/contracts/lint_unused_private_effect.json
fixtures/contracts/lint_unused_private_task.json
fixtures/contracts/lint_unused_private_type.json
fixtures/contracts/lint_unused_pure_binding.json
fixtures/contracts/lint_unused_pure_expression_statement.json
fixtures/contracts/lint_unused_take.json
fixtures/contracts/lsp_command_preview_doctor.json
fixtures/contracts/lsp_fix_preview_unused_private_task.json
fixtures/contracts/migrate_legacy_source.json
fixtures/contracts/migrate_unchecked_result.json
fixtures/contracts/migrate_unchecked_result_binding.json
fixtures/contracts/migrate_unqualified_imported_call.json
fixtures/contracts/project_scaffold_agent.json
fixtures/contracts/project_scaffold_agent_project.json
fixtures/contracts/project_scaffold_deploy.json
fixtures/contracts/query_project_tasks.json
fixtures/contracts/query_unknown_project_module.json
fixtures/contracts/run_hello_ready.json
fixtures/contracts/sandbox_manifest_agent_pipeline.json
fixtures/contracts/sandbox_report_agent_pipeline.json
fixtures/contracts/self_hosting_status_stage2_source.json
fixtures/contracts/shadow_agent_deploy_pipeline.json
fixtures/contracts/shadow_agent_project.json
fixtures/contracts/shadow_agent_project_pipeline_module.json
fixtures/contracts/shadow_empty_while_statement_rule.json
fixtures/contracts/shadow_unused_private_task_rule.json
fixtures/contracts/symbol_graph_hello.json
fixtures/contracts/trace_receipt_hello.json
fixtures/contracts/trace_report_hello_receipt.json
fixtures/contracts/trace_seal_hello.json
fixtures/contracts/verify_project_ready.json
fixtures/contracts/workbench_unused_private_task.json
fixtures/contracts/zjx_hello_ready.json
fixtures/contracts/zjx_tool_inspect_hello.json
fixtures/contracts/zjx_tool_validate_hello.json
fixtures/corpus/accepted/agent_data_authority.sley
fixtures/corpus/accepted/agent_deploy_pipeline.sley
fixtures/corpus/accepted/agent_spend_authority.sley
fixtures/corpus/accepted/agent_split_authority.sley
fixtures/corpus/accepted/authority/database_aliases.sley
fixtures/corpus/accepted/authority/database_read.sley
fixtures/corpus/accepted/authority/database_write.sley
fixtures/corpus/accepted/authority/deploy_stage.sley
fixtures/corpus/accepted/authority/file_read.sley
fixtures/corpus/accepted/authority/file_write.sley
fixtures/corpus/accepted/authority/gate_take_database_read.sley
fixtures/corpus/accepted/authority/model_complete.sley
fixtures/corpus/accepted/authority/network_get_text.sley
fixtures/corpus/accepted/authority/secret_read.sley
fixtures/corpus/accepted/authority/shell_run.sley
fixtures/corpus/accepted/authority/spend_authorize.sley
fixtures/corpus/accepted/collections_indexing.sley
fixtures/corpus/accepted/module_namespace.sley
fixtures/corpus/accepted/mutable_sum.sley
fixtures/corpus/accepted/pure_main.sley
fixtures/corpus/accepted/records_and_calls.sley
fixtures/corpus/accepted/result_flow.sley
fixtures/corpus/accepted/type_alias_transparency.sley
fixtures/corpus/manifest.json
fixtures/corpus/rejected/authority/missing_database_alias_write_effect.json
fixtures/corpus/rejected/authority/missing_database_alias_write_effect.sley
fixtures/corpus/rejected/authority/missing_database_read_effect.json
fixtures/corpus/rejected/authority/missing_database_read_effect.sley
fixtures/corpus/rejected/authority/missing_database_write_effect.json
fixtures/corpus/rejected/authority/missing_database_write_effect.sley
fixtures/corpus/rejected/authority/missing_deploy_effect.json
fixtures/corpus/rejected/authority/missing_deploy_effect.sley
fixtures/corpus/rejected/authority/missing_file_read_effect.json
fixtures/corpus/rejected/authority/missing_file_read_effect.sley
fixtures/corpus/rejected/authority/missing_file_write_effect.json
fixtures/corpus/rejected/authority/missing_file_write_effect.sley
fixtures/corpus/rejected/authority/missing_gate_take_effect.json
fixtures/corpus/rejected/authority/missing_gate_take_effect.sley
fixtures/corpus/rejected/authority/missing_model_effect.json
fixtures/corpus/rejected/authority/missing_model_effect.sley
fixtures/corpus/rejected/authority/missing_network_effect.json
fixtures/corpus/rejected/authority/missing_network_effect.sley
fixtures/corpus/rejected/authority/missing_secret_effect.json
fixtures/corpus/rejected/authority/missing_secret_effect.sley
fixtures/corpus/rejected/authority/missing_shell_effect.json
fixtures/corpus/rejected/authority/missing_shell_effect.sley
fixtures/corpus/rejected/authority/missing_spend_effect.json
fixtures/corpus/rejected/authority/missing_spend_effect.sley
fixtures/corpus/rejected/authority/missing_transitive_agent_effect.json
fixtures/corpus/rejected/authority/missing_transitive_agent_effect.sley
fixtures/corpus/rejected/authority/missing_transitive_data_write_effect.json
fixtures/corpus/rejected/authority/missing_transitive_data_write_effect.sley
fixtures/corpus/rejected/authority/missing_transitive_spend_effect.json
fixtures/corpus/rejected/authority/missing_transitive_spend_effect.sley
fixtures/corpus/rejected/call_argument_type_mismatch.json
fixtures/corpus/rejected/call_argument_type_mismatch.sley
fixtures/corpus/rejected/call_arity_mismatch.json
fixtures/corpus/rejected/call_arity_mismatch.sley
fixtures/corpus/rejected/duplicate_map_key.json
fixtures/corpus/rejected/duplicate_map_key.sley
fixtures/corpus/rejected/duplicate_record_field.json
fixtures/corpus/rejected/duplicate_record_field.sley
fixtures/corpus/rejected/duplicate_record_literal_field.json
fixtures/corpus/rejected/duplicate_record_literal_field.sley
fixtures/corpus/rejected/duplicate_take.json
fixtures/corpus/rejected/duplicate_take.sley
fixtures/corpus/rejected/gate_effect_undeclared.json
fixtures/corpus/rejected/gate_effect_undeclared.sley
fixtures/corpus/rejected/gate_take_type_mismatch.json
fixtures/corpus/rejected/gate_take_type_mismatch.sley
fixtures/corpus/rejected/list_element_type_mismatch.json
fixtures/corpus/rejected/list_element_type_mismatch.sley
fixtures/corpus/rejected/list_index_non_int.json
fixtures/corpus/rejected/list_index_non_int.sley
fixtures/corpus/rejected/map_index_non_text.json
fixtures/corpus/rejected/map_index_non_text.sley
fixtures/corpus/rejected/map_key_type_mismatch.json
fixtures/corpus/rejected/map_key_type_mismatch.sley
fixtures/corpus/rejected/map_value_type_mismatch.json
fixtures/corpus/rejected/map_value_type_mismatch.sley
fixtures/corpus/rejected/missing_return.json
fixtures/corpus/rejected/missing_return.sley
fixtures/corpus/rejected/module_namespace_conflict.json
fixtures/corpus/rejected/module_namespace_conflict.sley
fixtures/corpus/rejected/question_requires_result.json
fixtures/corpus/rejected/question_requires_result.sley
fixtures/corpus/rejected/record_field_missing.json
fixtures/corpus/rejected/record_field_missing.sley
fixtures/corpus/rejected/record_field_type_mismatch.json
fixtures/corpus/rejected/record_field_type_mismatch.sley
fixtures/corpus/rejected/record_field_unknown.json
fixtures/corpus/rejected/record_field_unknown.sley
fixtures/corpus/rejected/record_literal_non_record_type.json
fixtures/corpus/rejected/record_literal_non_record_type.sley
fixtures/corpus/rejected/type_alias_mismatch.json
fixtures/corpus/rejected/type_alias_mismatch.sley
fixtures/corpus/rejected/type_mismatch.json
fixtures/corpus/rejected/type_mismatch.sley
fixtures/corpus/rejected/unauthorized_effect.json
fixtures/corpus/rejected/unauthorized_effect.sley
fixtures/corpus/rejected/unknown_effect.json
fixtures/corpus/rejected/unknown_effect.sley
fixtures/corpus/rejected/unknown_identifier.json
fixtures/corpus/rejected/unknown_identifier.sley
fixtures/corpus/rejected/unknown_record_field.json
fixtures/corpus/rejected/unknown_record_field.sley
fixtures/corpus/rejected/unknown_task.json
fixtures/corpus/rejected/unknown_task.sley
fixtures/corpus/rejected/unknown_type.json
fixtures/corpus/rejected/unknown_type.sley
fixtures/grafts/add_tenant_take.json
fixtures/grafts/delete_debug_statement.json
fixtures/grafts/insert_multiple_statements.json
fixtures/grafts/insert_out_of_range_statement.json
fixtures/grafts/insert_total_increment_statement.json
fixtures/grafts/insert_unknown_identifier_statement.json
fixtures/grafts/invalid_update_call_sites_callee.json
fixtures/grafts/missing_update_call_sites.json
fixtures/grafts/move_bind_before_bind.json
fixtures/grafts/move_expression_unsupported.json
fixtures/grafts/move_statement_out_of_range.json
fixtures/grafts/remove_call_arg_out_of_range.json
fixtures/grafts/rename_update_call_sites.json
fixtures/grafts/replace_call_arg_out_of_range.json
fixtures/grafts/replace_expression_right.json
fixtures/grafts/replace_missing_expression.json
fixtures/grafts/stale_add_tenant_take.json
llms.txt
scripts/check-self-hosted-code.sh
scripts/self-hosted-test.sh
scripts/self-hosting-inventory.sh
self-hosted/sley.toml
self-hosted/src/loom/bootstrap.sley
self-hosted/src/loom/checker.sley
self-hosted/src/loom/lint.sley
self-hosted/src/loom/parser.sley
self-hosted/src/loom/reports.sley
self-hosted/src/loom/runtime.sley

## Notes

- This report is generated with `./scripts/self-hosting-inventory.sh`.
- The extension scan is a release blocker, not a full semantic parity proof.
- Runtime parity evidence lives in `scripts/self-hosted-test.sh` and `make v1`.
