# Self-Hosting Migration Inventory

Generated: 2026-05-08T07:24:25Z

## Foreign-language surface scan

| Extension | Count |
|---|---:|
| *.rs | 50 |
| *.c | 0 |
| *.cc | 0 |
| *.cpp | 0 |
| *.h | 0 |
| *.m | 0 |
| *.mm | 0 |
| *.swift | 0 |

## Rust bins present

- sley-agent-bench
- sley-ci
- sley-conformance
- sley-contract
- sley-docgen
- sley-lsp
- sley-migrate
- sley-sandbox-runner
- sley-shadow
- sley-workbench
- sley-zjx

## Reported CLI commands in llms.txt

1. `sley -- check --json <file-or-project>`
2. `sley -- ast --json <file-or-project>`
3. `sley -- ast --json --node <node-id> <file-or-project>`
4. `sley -- graph --json <file-or-project>`
5. `sley -- graph --json --slice <node-id> <file-or-project>`
6. `sley -- query --json --kind calls <file-or-project>`
7. `sley -- lint --json <file-or-project>`
8. `sley -- plan --json --graft-templates <file-or-project>`
9. `sley -- new --json --template agent-project --name agent-app --module agent.main agent-app`
10. `sley -- doctor --json <file-or-project>`
11. `sley -- fix --json --kind add_module_declaration --dry-run <file-or-project>`
12. `sley -- fix --json --kind delete_unused_private_declarations --dry-run <file-or-project>`
13. `sley -- fix --json --kind delete_unused_private_declarations --write <file-or-project>`
14. `sley -- verify --json <file-or-project>`
15. `sley -- deploy --json --dry-run --artifacts-dir .sley/deploy <file-or-project>`
16. `sley -- trace --json <file-or-project>`
17. `sley -- zjx --json <file-or-project>`
18. `sley-ci -- lint --json --deny-warnings <file-or-project>`
19. `sley-ci -- doctor --json --deny-warnings <file-or-project>`
20. `sley-ci -- plan --json --graft-templates <file-or-project>`
21. `sley-ci -- run --json <file-or-project>`
22. `sley-ci -- deploy --json --dry-run --artifacts-dir .sley/ci-deploy <file-or-project>`
23. `sley-ci -- corpus --json fixtures/corpus/manifest.json`
24. `sley-ci -- examples --json examples`
25. `sley-conformance -- report --json`
26. `sley-conformance -- coverage --json --require-tag <tag>`
27. `sley-contract -- inspect-deploy-artifacts .sley/deploy --schemas docs/schemas --json`
28. `sley-lsp`
29. `sley-workbench -- --json --html .sley/workbench.html <file-or-project>`
30. `sley-docgen -- reference --json --markdown .sley/reference.md <file-or-project>`
31. `sley-agent-bench -- run --json`
32. `sley-migrate -- report --json --schemas docs/schemas --fixtures fixtures/contracts <file-or-project>`
33. `sley-sandbox-runner -- run --json <manifest.json>`
34. `sley-zjx -- inspect --json <zjx-envelope.json>`
35. `sley-zjx -- verify-digest --json <zjx-envelope.json>`
36. `sley -- seal --json <file-or-project>`
37. `sley -- graft --json --dry-run <file> <graft.json>`
38. `sley -- graft --json --write <file> <graft.json>`
39. `sley -- format <file>`
40. `sley -- run --json --cap DatabaseRead --db-table users=examples/users.json examples/db_gate.sley`
41. `sley -- run --json --cap DatabaseRead --cap DatabaseWrite --db-table users=examples/users.json examples/db_write_gate.sley`
42. `sley -- run --json --cap Network --http-text https://example.test/profile Ada examples/network_gate.sley`
43. `sley -- run --json --cap Shell --shell-output date 2026-05-05 examples/shell_gate.sley`
44. `sley -- run --json --cap ModelCall --model-output name Ada examples/model_gate.sley`
45. `sley -- run --json --cap SecretRead --secret api_key redacted examples/secret_gate.sley`
46. `sley -- run --json --cap Deploy --deploy-result staging staged examples/deploy_gate.sley`
47. `sley -- run --json --cap Spend --spend-result ads-budget authorized examples/spend_gate.sley`

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
- v1

## Non-Rust/C non-foreign files (tracked)

./.github/actions/sley-v1/action.yml
./.github/workflows/v1.yml
./.pre-commit-config.yaml
./CHANGELOG.md
./PongAfterAudit.md
./README.md
./SELF_HOSTING_INVENTORY_REPORT.md
./SELF_HOSTING_MIGRATION_PLAN.md
./SELF_HOSTING_PHASE1_PORT_PLAN.md
./docs/AgentQuickstart.md
./docs/BrandingAssets.md
./docs/PublicReleaseChecklist.md
./docs/SleyDeveloperUtilitiesRoadmap.md
./docs/SleyLanguageSpec.md
./docs/contracts.md
./docs/schemas/sley.agent_bench.report.v0.schema.json
./docs/schemas/sley.ast.node.v0.schema.json
./docs/schemas/sley.ast.program.v0.schema.json
./docs/schemas/sley.ci.report.v0.schema.json
./docs/schemas/sley.cli_smoke.manifest.v0.schema.json
./docs/schemas/sley.conformance.coverage.v0.schema.json
./docs/schemas/sley.conformance.manifest.v0.schema.json
./docs/schemas/sley.conformance.report.v0.schema.json
./docs/schemas/sley.contract.fixture_check.v0.schema.json
./docs/schemas/sley.contract.inventory.v0.schema.json
./docs/schemas/sley.contract.validate.v0.schema.json
./docs/schemas/sley.deploy.artifact_check.v0.schema.json
./docs/schemas/sley.deploy.artifacts.v0.schema.json
./docs/schemas/sley.deploy.report.v0.schema.json
./docs/schemas/sley.diagnostics.report.v0.schema.json
./docs/schemas/sley.docgen.report.v0.schema.json
./docs/schemas/sley.doctor.report.v0.schema.json
./docs/schemas/sley.edit_plan.report.v0.schema.json
./docs/schemas/sley.graft.outcome.v0.schema.json
./docs/schemas/sley.lint.report.v0.schema.json
./docs/schemas/sley.lsp.command_preview.v0.schema.json
./docs/schemas/sley.lsp.fix_preview.v0.schema.json
./docs/schemas/sley.migrate.report.v0.schema.json
./docs/schemas/sley.project.scaffold.v0.schema.json
./docs/schemas/sley.query.report.v0.schema.json
./docs/schemas/sley.run.report.v0.schema.json
./docs/schemas/sley.sandbox.manifest.v0.schema.json
./docs/schemas/sley.sandbox.report.v0.schema.json
./docs/schemas/sley.shadow.report.v0.schema.json
./docs/schemas/sley.symbol_graph.slice.v0.schema.json
./docs/schemas/sley.symbol_graph.v0.schema.json
./docs/schemas/sley.trace.receipt.v0.schema.json
./docs/schemas/sley.trace.report.v0.schema.json
./docs/schemas/sley.trace.seal.v0.schema.json
./docs/schemas/sley.verify.report.v0.schema.json
./docs/schemas/sley.workbench.report.v0.schema.json
./docs/schemas/sley.zjx.envelope.v0.schema.json
./docs/schemas/sley.zjx.tool.report.v0.schema.json
./editors/vscode-sley/README.md
./editors/vscode-sley/extension.js
./editors/vscode-sley/language-configuration.json
./editors/vscode-sley/package-lock.json
./editors/vscode-sley/package.json
./editors/vscode-sley/syntaxes/sley.tmLanguage.json
./examples/users.json
./fixtures/ci_smoke_probe/insert_statement.json
./fixtures/ci_smoke_probe/manifest.json
./fixtures/cli_smokes/manifest.json
./fixtures/contract_probe/query_project_tasks.json
./fixtures/contracts/agent_bench_probe.json
./fixtures/contracts/ast_minimal_program.json
./fixtures/contracts/ast_node_return_expression.json
./fixtures/contracts/ci_check_project_ready.json
./fixtures/contracts/ci_corpus_manifest_ready.json
./fixtures/contracts/ci_deploy_blocked_runtime_gates.json
./fixtures/contracts/ci_deploy_denied_empty_for_statement.json
./fixtures/contracts/ci_deploy_project_ready.json
./fixtures/contracts/ci_deploy_requires_dry_run.json
./fixtures/contracts/ci_doctor_denied_empty_for_statement.json
./fixtures/contracts/ci_doctor_project_ready.json
./fixtures/contracts/ci_examples_ready.json
./fixtures/contracts/ci_lint_denied_empty_for_statement.json
./fixtures/contracts/ci_lint_project_ready.json
./fixtures/contracts/ci_lint_unknown_project_module.json
./fixtures/contracts/ci_plan_denied_empty_for_statement.json
./fixtures/contracts/ci_plan_project_ready.json
./fixtures/contracts/ci_run_project_ready.json
./fixtures/contracts/ci_smoke_probe_ready.json
./fixtures/contracts/ci_verify_denied_empty_for_statement.json
./fixtures/contracts/ci_verify_project_ready.json
./fixtures/contracts/conformance_coverage_probe.json
./fixtures/contracts/conformance_manifest_probe.json
./fixtures/contracts/conformance_report_probe.json
./fixtures/contracts/conformance_report_public_release_blocked.json
./fixtures/contracts/contract_fixture_check_probe.json
./fixtures/contracts/contract_inventory_schemas.json
./fixtures/contracts/contract_validate_query_report.json
./fixtures/contracts/deploy_artifact_check_hello_passed.json
./fixtures/contracts/deploy_artifacts_hello.json
./fixtures/contracts/deploy_hello_ready.json
./fixtures/contracts/diagnostic_report_deploy_requires_dry_run.json
./fixtures/contracts/diagnostic_report_unknown_identifier.json
./fixtures/contracts/docgen_agent_project_pipeline_module.json
./fixtures/contracts/docgen_agent_reference.json
./fixtures/contracts/doctor_denied_empty_for_statement.json
./fixtures/contracts/doctor_project_ready.json
./fixtures/contracts/edit_plan_project_graft_templates.json
./fixtures/contracts/edit_plan_project_ready.json
./fixtures/contracts/edit_plan_project_targeted_graft_templates.json
./fixtures/contracts/graft_outcome_insert_statement.json
./fixtures/contracts/graph_slice_minimal_task.json
./fixtures/contracts/graph_slice_project_module.json
./fixtures/contracts/graph_slice_project_task.json
./fixtures/contracts/lint_absorbing_arithmetic_expression.json
./fixtures/contracts/lint_absorbing_boolean_expression.json
./fixtures/contracts/lint_constant_arithmetic_expression.json
./fixtures/contracts/lint_constant_boolean_comparison_expression.json
./fixtures/contracts/lint_constant_comparison_expression.json
./fixtures/contracts/lint_constant_false_if_statement.json
./fixtures/contracts/lint_constant_false_while_statement.json
./fixtures/contracts/lint_constant_if_expression.json
./fixtures/contracts/lint_constant_if_statement.json
./fixtures/contracts/lint_constant_len_expression.json
./fixtures/contracts/lint_constant_list_index_expression.json
./fixtures/contracts/lint_constant_map_index_expression.json
./fixtures/contracts/lint_constant_not_expression.json
./fixtures/contracts/lint_constant_record_field_access_expression.json
./fixtures/contracts/lint_constant_text_concatenation_expression.json
./fixtures/contracts/lint_double_negation_expression.json
./fixtures/contracts/lint_duplicate_import.json
./fixtures/contracts/lint_empty_else_statement.json
./fixtures/contracts/lint_empty_for_statement.json
./fixtures/contracts/lint_empty_forge_statement.json
./fixtures/contracts/lint_empty_if_statement.json
./fixtures/contracts/lint_empty_while_statement.json
./fixtures/contracts/lint_idempotent_boolean_expression.json
./fixtures/contracts/lint_identity_binary_expression.json
./fixtures/contracts/lint_missing_module_declaration.json
./fixtures/contracts/lint_mutable_binding_never_set.json
./fixtures/contracts/lint_negated_comparison_expression.json
./fixtures/contracts/lint_overwritten_set_statement.json
./fixtures/contracts/lint_raw_host_adapter.json
./fixtures/contracts/lint_redundant_boolean_comparison.json
./fixtures/contracts/lint_redundant_boolean_if_expression.json
./fixtures/contracts/lint_redundant_boolean_if_statement.json
./fixtures/contracts/lint_redundant_initial_set_statement.json
./fixtures/contracts/lint_same_branch_if_expression.json
./fixtures/contracts/lint_same_branch_if_statement.json
./fixtures/contracts/lint_self_assignment_statement.json
./fixtures/contracts/lint_self_comparison_expression.json
./fixtures/contracts/lint_unchecked_result.json
./fixtures/contracts/lint_unchecked_result_binding.json
./fixtures/contracts/lint_unknown_project_module.json
./fixtures/contracts/lint_unqualified_imported_call.json
./fixtures/contracts/lint_unreachable_private_task.json
./fixtures/contracts/lint_unreachable_statement.json
./fixtures/contracts/lint_unused_declared_effect.json
./fixtures/contracts/lint_unused_effectful_binding.json
./fixtures/contracts/lint_unused_import.json
./fixtures/contracts/lint_unused_private_effect.json
./fixtures/contracts/lint_unused_private_task.json
./fixtures/contracts/lint_unused_private_type.json
./fixtures/contracts/lint_unused_pure_binding.json
./fixtures/contracts/lint_unused_pure_expression_statement.json
./fixtures/contracts/lint_unused_take.json
./fixtures/contracts/lsp_command_preview_doctor.json
./fixtures/contracts/lsp_fix_preview_unused_private_task.json
./fixtures/contracts/migrate_legacy_source.json
./fixtures/contracts/migrate_unchecked_result.json
./fixtures/contracts/migrate_unchecked_result_binding.json
./fixtures/contracts/migrate_unqualified_imported_call.json
./fixtures/contracts/project_scaffold_agent.json
./fixtures/contracts/project_scaffold_agent_project.json
./fixtures/contracts/project_scaffold_deploy.json
./fixtures/contracts/query_project_tasks.json
./fixtures/contracts/query_unknown_project_module.json
./fixtures/contracts/run_hello_ready.json
./fixtures/contracts/sandbox_manifest_agent_pipeline.json
./fixtures/contracts/sandbox_report_agent_pipeline.json
./fixtures/contracts/shadow_agent_deploy_pipeline.json
./fixtures/contracts/shadow_agent_project.json
./fixtures/contracts/shadow_agent_project_pipeline_module.json
./fixtures/contracts/shadow_empty_while_statement_rule.json
./fixtures/contracts/shadow_unused_private_task_rule.json
./fixtures/contracts/symbol_graph_hello.json
./fixtures/contracts/trace_receipt_hello.json
./fixtures/contracts/trace_report_hello_receipt.json
./fixtures/contracts/trace_seal_hello.json
./fixtures/contracts/verify_project_ready.json
./fixtures/contracts/workbench_unused_private_task.json
./fixtures/contracts/zjx_hello_ready.json
./fixtures/contracts/zjx_tool_inspect_hello.json
./fixtures/contracts/zjx_tool_validate_hello.json
./fixtures/corpus/manifest.json
./fixtures/corpus/rejected/authority/missing_database_alias_write_effect.json
./fixtures/corpus/rejected/authority/missing_database_read_effect.json
./fixtures/corpus/rejected/authority/missing_database_write_effect.json
./fixtures/corpus/rejected/authority/missing_deploy_effect.json
./fixtures/corpus/rejected/authority/missing_file_read_effect.json
./fixtures/corpus/rejected/authority/missing_file_write_effect.json
./fixtures/corpus/rejected/authority/missing_gate_take_effect.json
./fixtures/corpus/rejected/authority/missing_model_effect.json
./fixtures/corpus/rejected/authority/missing_network_effect.json
./fixtures/corpus/rejected/authority/missing_secret_effect.json
./fixtures/corpus/rejected/authority/missing_shell_effect.json
./fixtures/corpus/rejected/authority/missing_spend_effect.json
./fixtures/corpus/rejected/authority/missing_transitive_agent_effect.json
./fixtures/corpus/rejected/authority/missing_transitive_data_write_effect.json
./fixtures/corpus/rejected/authority/missing_transitive_spend_effect.json
./fixtures/corpus/rejected/call_argument_type_mismatch.json
./fixtures/corpus/rejected/call_arity_mismatch.json
./fixtures/corpus/rejected/duplicate_map_key.json
./fixtures/corpus/rejected/duplicate_record_field.json
./fixtures/corpus/rejected/duplicate_record_literal_field.json
./fixtures/corpus/rejected/duplicate_take.json
./fixtures/corpus/rejected/gate_effect_undeclared.json
./fixtures/corpus/rejected/gate_take_type_mismatch.json
./fixtures/corpus/rejected/list_element_type_mismatch.json
./fixtures/corpus/rejected/list_index_non_int.json
./fixtures/corpus/rejected/map_index_non_text.json
./fixtures/corpus/rejected/map_key_type_mismatch.json
./fixtures/corpus/rejected/map_value_type_mismatch.json
./fixtures/corpus/rejected/missing_return.json
./fixtures/corpus/rejected/module_namespace_conflict.json
./fixtures/corpus/rejected/question_requires_result.json
./fixtures/corpus/rejected/record_field_missing.json
./fixtures/corpus/rejected/record_field_type_mismatch.json
./fixtures/corpus/rejected/record_field_unknown.json
./fixtures/corpus/rejected/record_literal_non_record_type.json
./fixtures/corpus/rejected/type_alias_mismatch.json
./fixtures/corpus/rejected/type_mismatch.json
./fixtures/corpus/rejected/unauthorized_effect.json
./fixtures/corpus/rejected/unknown_effect.json
./fixtures/corpus/rejected/unknown_identifier.json
./fixtures/corpus/rejected/unknown_record_field.json
./fixtures/corpus/rejected/unknown_task.json
./fixtures/corpus/rejected/unknown_type.json
./fixtures/grafts/add_tenant_take.json
./fixtures/grafts/delete_debug_statement.json
./fixtures/grafts/insert_multiple_statements.json
./fixtures/grafts/insert_out_of_range_statement.json
./fixtures/grafts/insert_total_increment_statement.json
./fixtures/grafts/insert_unknown_identifier_statement.json
./fixtures/grafts/invalid_update_call_sites_callee.json
./fixtures/grafts/missing_update_call_sites.json
./fixtures/grafts/move_bind_before_bind.json
./fixtures/grafts/move_expression_unsupported.json
./fixtures/grafts/move_statement_out_of_range.json
./fixtures/grafts/remove_call_arg_out_of_range.json
./fixtures/grafts/rename_update_call_sites.json
./fixtures/grafts/replace_call_arg_out_of_range.json
./fixtures/grafts/replace_expression_right.json
./fixtures/grafts/replace_missing_expression.json
./fixtures/grafts/stale_add_tenant_take.json
./scripts/validate-editor-shims.mjs
./target/.rustc_info.json
./target/debug/.fingerprint/ahash-01443f1efbe4aa32/lib-ahash.json
./target/debug/.fingerprint/ahash-a750001ed27a20d4/build-script-build-script-build.json
./target/debug/.fingerprint/ahash-aa5a35d19f4df8b1/run-build-script-build-script-build.json
./target/debug/.fingerprint/ahash-ffda9fc007f3b002/lib-ahash.json
./target/debug/.fingerprint/aho-corasick-558ec8d8b0bc8261/lib-aho_corasick.json
./target/debug/.fingerprint/aho-corasick-7ed241a7485c8193/lib-aho_corasick.json
./target/debug/.fingerprint/allocator-api2-56f87fb3088221db/lib-allocator_api2.json
./target/debug/.fingerprint/allocator-api2-8536947cb9a235d5/lib-allocator_api2.json
./target/debug/.fingerprint/anstream-6c5a71217f2eb6f7/lib-anstream.json
./target/debug/.fingerprint/anstream-739283ec1724e1b6/lib-anstream.json
./target/debug/.fingerprint/anstyle-2ee7b6122cf6b3bb/lib-anstyle.json
./target/debug/.fingerprint/anstyle-38bd59cd3b8fb9ec/lib-anstyle.json
./target/debug/.fingerprint/anstyle-parse-30bba8111dfe77b4/lib-anstyle_parse.json
./target/debug/.fingerprint/anstyle-parse-f91ed51063a04963/lib-anstyle_parse.json
./target/debug/.fingerprint/anstyle-query-3fc67089d2e276f5/lib-anstyle_query.json
./target/debug/.fingerprint/anstyle-query-a5071c00fde86ce1/lib-anstyle_query.json
./target/debug/.fingerprint/anyhow-11323c0fee26b0cd/build-script-build-script-build.json
./target/debug/.fingerprint/anyhow-612e018f4096fbaa/lib-anyhow.json
./target/debug/.fingerprint/anyhow-92e070dabf10b811/run-build-script-build-script-build.json
./target/debug/.fingerprint/anyhow-a21853c4865c620d/lib-anyhow.json
./target/debug/.fingerprint/autocfg-0d14e8d065a140e1/lib-autocfg.json
./target/debug/.fingerprint/bit-set-7cdbfa341686cb84/lib-bit_set.json
./target/debug/.fingerprint/bit-set-f63067da68ead8a4/lib-bit_set.json
./target/debug/.fingerprint/bit-vec-aa2b2a092a94a467/lib-bit_vec.json
./target/debug/.fingerprint/bit-vec-bbd9b4c63a177a1c/lib-bit_vec.json
./target/debug/.fingerprint/block-buffer-4ab0b84da4b08110/lib-block_buffer.json
./target/debug/.fingerprint/block-buffer-8d95b47242c20ab3/lib-block_buffer.json
./target/debug/.fingerprint/borrow-or-share-e1ba049ebaa7175a/lib-borrow_or_share.json
./target/debug/.fingerprint/borrow-or-share-e9d3e6cf29d838dd/lib-borrow_or_share.json
./target/debug/.fingerprint/bytecount-01811bee50fafcb8/lib-bytecount.json
./target/debug/.fingerprint/bytecount-df2fa1affb401010/lib-bytecount.json
./target/debug/.fingerprint/cfg-if-8e8257695bad511f/lib-cfg_if.json
./target/debug/.fingerprint/cfg-if-a3662b74b62b86e1/lib-cfg_if.json
./target/debug/.fingerprint/clap-61faace09b318081/lib-clap.json
./target/debug/.fingerprint/clap-b1711cfe562a4bc0/lib-clap.json
./target/debug/.fingerprint/clap-c81579c3cf272045/lib-clap.json
./target/debug/.fingerprint/clap-f75c8e9d5d7a68d2/lib-clap.json
./target/debug/.fingerprint/clap_builder-376d0f23b3507afd/lib-clap_builder.json
./target/debug/.fingerprint/clap_builder-599fc749969607dd/lib-clap_builder.json
./target/debug/.fingerprint/clap_derive-917f48893a6fb7d9/lib-clap_derive.json
./target/debug/.fingerprint/clap_derive-c412f8a9e5cb2966/lib-clap_derive.json
./target/debug/.fingerprint/clap_lex-ad48c1b9b06560a2/lib-clap_lex.json
./target/debug/.fingerprint/clap_lex-bd2ebd396e3daf5f/lib-clap_lex.json
./target/debug/.fingerprint/colorchoice-903a60aaf9638f45/lib-colorchoice.json
./target/debug/.fingerprint/colorchoice-fe67771bd0672a1b/lib-colorchoice.json
./target/debug/.fingerprint/cpufeatures-965da2e628aa3ebd/lib-cpufeatures.json
./target/debug/.fingerprint/cpufeatures-de1c38ddd25f3a0a/lib-cpufeatures.json
./target/debug/.fingerprint/crypto-common-c50eadff88c65ebc/lib-crypto_common.json
./target/debug/.fingerprint/crypto-common-c8131f83d21ea12c/lib-crypto_common.json
./target/debug/.fingerprint/data-encoding-26930f4dfeff13f7/lib-data_encoding.json
./target/debug/.fingerprint/data-encoding-f8c59e2f1585f9d6/lib-data_encoding.json
./target/debug/.fingerprint/deranged-328105e154db735a/lib-deranged.json
./target/debug/.fingerprint/deranged-fc840279fa8b656a/lib-deranged.json
./target/debug/.fingerprint/digest-0d87e93b6412e06e/lib-digest.json
./target/debug/.fingerprint/digest-6bb565115e6ba740/lib-digest.json
./target/debug/.fingerprint/displaydoc-36e0fff0f08d7d82/lib-displaydoc.json
./target/debug/.fingerprint/email_address-6d608478a76d9a88/lib-email_address.json
./target/debug/.fingerprint/email_address-f54510012aba6090/lib-email_address.json
./target/debug/.fingerprint/equivalent-5577b81418d2ac68/lib-equivalent.json
./target/debug/.fingerprint/equivalent-af0261b18c99c40e/lib-equivalent.json
./target/debug/.fingerprint/fancy-regex-567aca3fd9b96546/lib-fancy_regex.json
./target/debug/.fingerprint/fancy-regex-64c0bf46f323ed70/lib-fancy_regex.json
./target/debug/.fingerprint/fluent-uri-11373379dc2b1f37/lib-fluent_uri.json
./target/debug/.fingerprint/fluent-uri-2053972b13417301/lib-fluent_uri.json
./target/debug/.fingerprint/foldhash-0e29617154469000/lib-foldhash.json
./target/debug/.fingerprint/foldhash-b2cdc8c59354fd97/lib-foldhash.json
./target/debug/.fingerprint/fraction-783168458625a4e7/lib-fraction.json
./target/debug/.fingerprint/fraction-f5d4b93df869730f/lib-fraction.json
./target/debug/.fingerprint/generic-array-4d6be48c6456f7df/build-script-build-script-build.json
./target/debug/.fingerprint/generic-array-9a591590a03d1b5a/run-build-script-build-script-build.json
./target/debug/.fingerprint/generic-array-ecf89d4773286dea/lib-generic_array.json
./target/debug/.fingerprint/generic-array-edd0262653754e8e/lib-generic_array.json
./target/debug/.fingerprint/getrandom-48827429e32c729c/run-build-script-build-script-build.json
./target/debug/.fingerprint/getrandom-bcf00287326b970e/lib-getrandom.json
./target/debug/.fingerprint/getrandom-c97f217532981087/lib-getrandom.json
./target/debug/.fingerprint/getrandom-cc8697f7e7c92902/build-script-build-script-build.json
./target/debug/.fingerprint/hashbrown-3dd234a2603c8220/lib-hashbrown.json
./target/debug/.fingerprint/hashbrown-cf2bc3807c726419/lib-hashbrown.json
./target/debug/.fingerprint/heck-b6dd292169dd14d6/lib-heck.json
./target/debug/.fingerprint/icu_collections-107c55c0a860fa01/lib-icu_collections.json
./target/debug/.fingerprint/icu_collections-62ac3bfb10c7b3f4/lib-icu_collections.json
./target/debug/.fingerprint/icu_locale_core-c41c5c714e2226d1/lib-icu_locale_core.json
./target/debug/.fingerprint/icu_locale_core-f6b96ae30b0456d7/lib-icu_locale_core.json
./target/debug/.fingerprint/icu_normalizer-23f08219750f2109/lib-icu_normalizer.json
./target/debug/.fingerprint/icu_normalizer-333364b5f6e7dd18/lib-icu_normalizer.json
./target/debug/.fingerprint/icu_normalizer_data-34beae5277d59338/run-build-script-build-script-build.json
./target/debug/.fingerprint/icu_normalizer_data-5fe094d446dbaeb8/lib-icu_normalizer_data.json
./target/debug/.fingerprint/icu_normalizer_data-61c1491dfb364b2e/lib-icu_normalizer_data.json
./target/debug/.fingerprint/icu_normalizer_data-f5f497e0f612061b/build-script-build-script-build.json
./target/debug/.fingerprint/icu_properties-b11233ee8d07657f/lib-icu_properties.json
./target/debug/.fingerprint/icu_properties-b5bc1a51b1f1ebe8/lib-icu_properties.json
./target/debug/.fingerprint/icu_properties_data-4464bc4bacc7977f/lib-icu_properties_data.json
./target/debug/.fingerprint/icu_properties_data-4a5d3ea7183e2090/lib-icu_properties_data.json
./target/debug/.fingerprint/icu_properties_data-6dab9bd8cdf5dfcb/run-build-script-build-script-build.json
./target/debug/.fingerprint/icu_properties_data-d87f33ce3c95319e/build-script-build-script-build.json
./target/debug/.fingerprint/icu_provider-625cf7da63d25c83/lib-icu_provider.json
./target/debug/.fingerprint/icu_provider-6b2e8cbb939b9255/lib-icu_provider.json
./target/debug/.fingerprint/idna-8f0cf8763b99b5d3/lib-idna.json
./target/debug/.fingerprint/idna-b92f20f3a410c438/lib-idna.json
./target/debug/.fingerprint/idna_adapter-ac4dca2885d703c3/lib-idna_adapter.json
./target/debug/.fingerprint/idna_adapter-bb7de1a1f237abca/lib-idna_adapter.json
./target/debug/.fingerprint/is_terminal_polyfill-337e034bf3810c35/lib-is_terminal_polyfill.json
./target/debug/.fingerprint/is_terminal_polyfill-79537160b759df96/lib-is_terminal_polyfill.json
./target/debug/.fingerprint/itoa-311cb9dda294fc54/lib-itoa.json
./target/debug/.fingerprint/itoa-9ca2f2ed0524ac66/lib-itoa.json
./target/debug/.fingerprint/jsonschema-8e5d51aafd8c80bf/lib-jsonschema.json
./target/debug/.fingerprint/jsonschema-bbc1e919bffef7a9/lib-jsonschema.json
./target/debug/.fingerprint/lazy_static-731d3a117dc8921d/lib-lazy_static.json
./target/debug/.fingerprint/lazy_static-a5e8db74bffd96ad/lib-lazy_static.json
./target/debug/.fingerprint/libc-1784a02f99458686/lib-libc.json
./target/debug/.fingerprint/libc-31c437126fe3462b/build-script-build-script-build.json
./target/debug/.fingerprint/libc-39c5232551321dc6/run-build-script-build-script-build.json
./target/debug/.fingerprint/libc-4578b7ae24a947db/lib-libc.json
./target/debug/.fingerprint/litemap-0c80b9ad0bb9b316/lib-litemap.json
./target/debug/.fingerprint/litemap-a1de011652604402/lib-litemap.json
./target/debug/.fingerprint/lock_api-b59313b71286218d/lib-lock_api.json
./target/debug/.fingerprint/lock_api-bce0d02b6173a96b/lib-lock_api.json
./target/debug/.fingerprint/memchr-1c76f5cc91307870/lib-memchr.json
./target/debug/.fingerprint/memchr-541d2da8bdd96aca/lib-memchr.json
./target/debug/.fingerprint/micromap-3f88abd04d26657b/lib-micromap.json
./target/debug/.fingerprint/micromap-6350a92632a96e8f/lib-micromap.json
./target/debug/.fingerprint/num-bigint-94ddf2a5b9c40d1b/lib-num_bigint.json
./target/debug/.fingerprint/num-bigint-996081079bf38a4a/lib-num_bigint.json
./target/debug/.fingerprint/num-cmp-28a13f0985b23d36/lib-num_cmp.json
./target/debug/.fingerprint/num-cmp-ce1c9ffb000a95ea/lib-num_cmp.json
./target/debug/.fingerprint/num-complex-3f2153ddbbdc0fb5/lib-num_complex.json
./target/debug/.fingerprint/num-complex-ddc7524c95f4554a/lib-num_complex.json
./target/debug/.fingerprint/num-conv-05fa4b1c389a3c38/lib-num_conv.json
./target/debug/.fingerprint/num-conv-3cbcfa202c489077/lib-num_conv.json
./target/debug/.fingerprint/num-df34a90f94fd4a63/lib-num.json
./target/debug/.fingerprint/num-fd5b78d55578ccba/lib-num.json
./target/debug/.fingerprint/num-integer-2e54b9f1a4a276b8/lib-num_integer.json
./target/debug/.fingerprint/num-integer-b0d2015737269b7b/lib-num_integer.json
./target/debug/.fingerprint/num-iter-06ed1cfcc849b534/lib-num_iter.json
./target/debug/.fingerprint/num-iter-98070d4c7951236b/lib-num_iter.json
./target/debug/.fingerprint/num-rational-3d5dcd785ac005af/lib-num_rational.json
./target/debug/.fingerprint/num-rational-81c0bc0b8944cc4a/lib-num_rational.json
./target/debug/.fingerprint/num-traits-0e3ffd75fd5cfac5/build-script-build-script-build.json
./target/debug/.fingerprint/num-traits-290c6d8909fae54b/run-build-script-build-script-build.json
./target/debug/.fingerprint/num-traits-bdebd2f1a5f6f6f4/lib-num_traits.json
./target/debug/.fingerprint/num-traits-d1caebbc68890878/lib-num_traits.json
./target/debug/.fingerprint/once_cell-30429f713181964e/lib-once_cell.json
./target/debug/.fingerprint/once_cell-7c0cbf9a2ab6867c/lib-once_cell.json
./target/debug/.fingerprint/outref-3f235cd01a4fe3c2/lib-outref.json
./target/debug/.fingerprint/outref-d3d38b3305b30bb4/lib-outref.json
./target/debug/.fingerprint/parking_lot-9ff5dc24e3c363db/lib-parking_lot.json
./target/debug/.fingerprint/parking_lot-d5b993ef4334e17d/lib-parking_lot.json
./target/debug/.fingerprint/parking_lot_core-48ad7f5b8ba725f5/lib-parking_lot_core.json
./target/debug/.fingerprint/parking_lot_core-59872705755d3dce/build-script-build-script-build.json
./target/debug/.fingerprint/parking_lot_core-73a649486283305e/lib-parking_lot_core.json
./target/debug/.fingerprint/parking_lot_core-775325932bb9b871/run-build-script-build-script-build.json
./target/debug/.fingerprint/percent-encoding-af6a7a9b25d275b9/lib-percent_encoding.json
./target/debug/.fingerprint/percent-encoding-f1bea15934c8259f/lib-percent_encoding.json
./target/debug/.fingerprint/potential_utf-0042cf0af8c33a00/lib-potential_utf.json
./target/debug/.fingerprint/potential_utf-ec1cecd85c35f807/lib-potential_utf.json
./target/debug/.fingerprint/powerfmt-c5a59f73d954eb4a/lib-powerfmt.json
./target/debug/.fingerprint/powerfmt-dfa5abf2011c4742/lib-powerfmt.json
./target/debug/.fingerprint/proc-macro2-7e8392b0e59a7afb/lib-proc_macro2.json
./target/debug/.fingerprint/proc-macro2-82c8367507a6ace1/run-build-script-build-script-build.json
./target/debug/.fingerprint/proc-macro2-8e985430bcbaa7b0/build-script-build-script-build.json
./target/debug/.fingerprint/quote-6f2342aa854874a4/build-script-build-script-build.json
./target/debug/.fingerprint/quote-7c4bc4c8d8113ff9/run-build-script-build-script-build.json
./target/debug/.fingerprint/quote-9ee42181adfba244/lib-quote.json
./target/debug/.fingerprint/ref-cast-29c258973d7bf354/build-script-build-script-build.json
./target/debug/.fingerprint/ref-cast-4edc14a072d2d6c3/lib-ref_cast.json
./target/debug/.fingerprint/ref-cast-71ab1dd8ef1f1d46/lib-ref_cast.json
./target/debug/.fingerprint/ref-cast-9138240838b6151b/run-build-script-build-script-build.json
./target/debug/.fingerprint/ref-cast-impl-f40092e3f3b65664/lib-ref_cast_impl.json
./target/debug/.fingerprint/referencing-3472e99cdda4adaa/lib-referencing.json
./target/debug/.fingerprint/referencing-556d878d79b86ece/lib-referencing.json
./target/debug/.fingerprint/regex-89d1923759b4fe34/lib-regex.json
./target/debug/.fingerprint/regex-automata-c2078249b64c1815/lib-regex_automata.json
./target/debug/.fingerprint/regex-automata-d7cceb7a87469995/lib-regex_automata.json
./target/debug/.fingerprint/regex-dc4e5f7fda7844f4/lib-regex.json
./target/debug/.fingerprint/regex-syntax-4e73ada459992cfc/lib-regex_syntax.json
./target/debug/.fingerprint/regex-syntax-b325ad76ad00641a/lib-regex_syntax.json
./target/debug/.fingerprint/scopeguard-5cd31bc5354011e8/lib-scopeguard.json
./target/debug/.fingerprint/scopeguard-68e526315860e0e9/lib-scopeguard.json
./target/debug/.fingerprint/serde-05af5beb34012ef4/run-build-script-build-script-build.json
./target/debug/.fingerprint/serde-07ac095c8fa9e047/lib-serde.json
./target/debug/.fingerprint/serde-1f92435d64132f02/lib-serde.json
./target/debug/.fingerprint/serde-2fd1de87d231482c/build-script-build-script-build.json
./target/debug/.fingerprint/serde-30dfd4c93d7ae6f7/run-build-script-build-script-build.json
./target/debug/.fingerprint/serde-3d99395ed56b928f/lib-serde.json
./target/debug/.fingerprint/serde-6262f69ab9a0ea24/lib-serde.json
./target/debug/.fingerprint/serde-6d6906f02852fe9d/lib-serde.json
./target/debug/.fingerprint/serde-9f950e21b1641bfa/lib-serde.json
./target/debug/.fingerprint/serde-f2949ac091c3d152/build-script-build-script-build.json
./target/debug/.fingerprint/serde_core-0f37286f728ffabd/build-script-build-script-build.json
./target/debug/.fingerprint/serde_core-494af8253f76b55c/build-script-build-script-build.json
./target/debug/.fingerprint/serde_core-51581c1704d17ce7/run-build-script-build-script-build.json
./target/debug/.fingerprint/serde_core-6b6976e8516dc994/lib-serde_core.json
./target/debug/.fingerprint/serde_core-6b718945fbcc98f3/lib-serde_core.json
./target/debug/.fingerprint/serde_core-72ac49f382cefe72/run-build-script-build-script-build.json
./target/debug/.fingerprint/serde_core-af717a76dfe81de2/lib-serde_core.json
./target/debug/.fingerprint/serde_core-f8aa7fc8f11846cf/lib-serde_core.json
./target/debug/.fingerprint/serde_derive-229e90fbaa448cd6/lib-serde_derive.json
./target/debug/.fingerprint/serde_derive-cd0702dba5c4cad1/lib-serde_derive.json
./target/debug/.fingerprint/serde_json-5099a63400619d1c/lib-serde_json.json
./target/debug/.fingerprint/serde_json-636adaac8404fdda/lib-serde_json.json
./target/debug/.fingerprint/serde_json-916ec133267106a7/lib-serde_json.json
./target/debug/.fingerprint/serde_json-c6f598bb077f279b/lib-serde_json.json
./target/debug/.fingerprint/serde_json-f1b7c519bb855f4b/run-build-script-build-script-build.json
./target/debug/.fingerprint/serde_json-f264ea96d5ba2777/build-script-build-script-build.json
./target/debug/.fingerprint/serde_spanned-0de7af76e93fc215/lib-serde_spanned.json
./target/debug/.fingerprint/serde_spanned-be99959ea791d35b/lib-serde_spanned.json
./target/debug/.fingerprint/sha2-237c7d3ca10acc86/lib-sha2.json
./target/debug/.fingerprint/sha2-b402ed9f1be72e5c/lib-sha2.json
./target/debug/.fingerprint/sley-002910f9e0ec859d/bin-sley-lsp.json
./target/debug/.fingerprint/sley-04fc743d40a3f4ac/test-bin-sley-contract.json
./target/debug/.fingerprint/sley-079d9db4a6cd515b/test-integration-test-sley_migrate.json
./target/debug/.fingerprint/sley-07d5f81f32e0d4cd/bin-sley.json
./target/debug/.fingerprint/sley-07ee328cff69d94f/test-integration-test-sley_zjx.json
./target/debug/.fingerprint/sley-0b4c45e4f4685393/test-integration-test-sley_migrate.json
./target/debug/.fingerprint/sley-0ba333acf63a4432/bin-sley.json
./target/debug/.fingerprint/sley-0e531ed9dc6249c3/bin-sley-contract.json
./target/debug/.fingerprint/sley-0f7bdb3461623793/test-integration-test-sley_workbench.json
./target/debug/.fingerprint/sley-11e0c46237049668/test-integration-test-sley_shadow.json
./target/debug/.fingerprint/sley-1757d66701f24b4c/bin-sley-agent-bench.json
./target/debug/.fingerprint/sley-1a196bb499ec8a00/test-integration-test-sley_templates.json
./target/debug/.fingerprint/sley-1ce309d50dca76fc/test-bin-sley-agent-bench.json
./target/debug/.fingerprint/sley-1d4e68d07411d160/test-integration-test-sley_sandbox_runner.json
./target/debug/.fingerprint/sley-1f2492ff27acf476/bin-sley-workbench.json
./target/debug/.fingerprint/sley-207e27c9667ee311/test-bin-sley.json
./target/debug/.fingerprint/sley-232e30cd32ef6f68/test-integration-test-sley_zjx.json
./target/debug/.fingerprint/sley-24dcb65bab3be4b7/bin-sley-conformance.json
./target/debug/.fingerprint/sley-2712f9f351329453/test-integration-test-sley_agent_bench.json
./target/debug/.fingerprint/sley-290ca2fe6cf61bf5/test-integration-test-sley_workbench.json
./target/debug/.fingerprint/sley-2b593157df13835b/test-integration-test-sley_templates.json
./target/debug/.fingerprint/sley-2da860d19f5d573e/test-integration-test-sley_lsp.json
./target/debug/.fingerprint/sley-3088805eb7a18e53/test-integration-test-sley_agent_bench.json
./target/debug/.fingerprint/sley-316aef6d98706710/test-integration-test-sley_templates.json
./target/debug/.fingerprint/sley-322501d2ca002ea2/bin-sley.json
./target/debug/.fingerprint/sley-3672d59969e8466b/test-integration-test-sley_lsp.json
./target/debug/.fingerprint/sley-37b6aa5a3f057481/test-integration-test-sley_v0.json
./target/debug/.fingerprint/sley-38adb60e76506ff7/test-bin-sley-ci.json
./target/debug/.fingerprint/sley-3b74c32ab3de1f1c/test-bin-sley-sandbox-runner.json
./target/debug/.fingerprint/sley-3e4a8ff7049eab4d/bin-sley-lsp.json
./target/debug/.fingerprint/sley-3f8e488bcc0be742/test-integration-test-sley_zjx.json
./target/debug/.fingerprint/sley-41b9d207470613e6/bin-sley.json
./target/debug/.fingerprint/sley-43abaeae5d8ad8eb/test-integration-test-sley_lsp.json
./target/debug/.fingerprint/sley-494be8f99865c3df/test-integration-test-sley_docgen.json
./target/debug/.fingerprint/sley-4aa6820e46133cac/bin-sley-conformance.json
./target/debug/.fingerprint/sley-4dd4907892077db9/test-integration-test-sley_sandbox_runner.json
./target/debug/.fingerprint/sley-4ec8ae56bd5df243/test-integration-test-sley_lsp.json
./target/debug/.fingerprint/sley-51ef5b25ee69a3de/lib-sley.json
./target/debug/.fingerprint/sley-558e22fe2332e1fb/test-bin-sley-lsp.json
./target/debug/.fingerprint/sley-5685b479bef0aea6/test-integration-test-sley_v0.json
./target/debug/.fingerprint/sley-584030ef428ab171/test-integration-test-sley_v0.json
./target/debug/.fingerprint/sley-58befe072a7f041b/test-integration-test-sley_zjx.json
./target/debug/.fingerprint/sley-5ad3b94f042b2fa0/test-bin-sley-migrate.json
./target/debug/.fingerprint/sley-5b53cdc7762919d9/test-bin-sley-shadow.json
./target/debug/.fingerprint/sley-5c04ade56f5f5909/lib-sley.json
./target/debug/.fingerprint/sley-5f75baab330deee7/lib-sley.json
./target/debug/.fingerprint/sley-614834b81ee7ce76/bin-sley-migrate.json
./target/debug/.fingerprint/sley-641b001fa14f2fb7/bin-sley.json
./target/debug/.fingerprint/sley-65fed909015ffdd9/test-lib-sley.json
./target/debug/.fingerprint/sley-6844f6db2ff19c8a/test-integration-test-sley_templates.json
./target/debug/.fingerprint/sley-68c3e5eff9db9e8e/test-integration-test-sley_lsp.json
./target/debug/.fingerprint/sley-6a89eef24572cd9c/test-bin-sley.json
./target/debug/.fingerprint/sley-6c9eb716ef2aa59d/bin-sley-migrate.json
./target/debug/.fingerprint/sley-6de6d08f0bbf6aca/bin-sley-shadow.json
./target/debug/.fingerprint/sley-6df29ded084e3dec/test-integration-test-sley_workbench.json
./target/debug/.fingerprint/sley-6f3360b3fe3fcdce/test-lib-sley.json
./target/debug/.fingerprint/sley-714be83b852da10a/test-integration-test-sley_sandbox_runner.json
./target/debug/.fingerprint/sley-718bd8b0b6acab4e/bin-sley-agent-bench.json
./target/debug/.fingerprint/sley-78c38fa5b4310493/bin-sley-workbench.json
./target/debug/.fingerprint/sley-796f023d185ddf0a/test-integration-test-sley_docgen.json
./target/debug/.fingerprint/sley-7a03f31b4af4bc6a/test-integration-test-sley_agent_bench.json
./target/debug/.fingerprint/sley-7a62e1431bfd57b3/test-integration-test-sley_workbench.json
./target/debug/.fingerprint/sley-7b439e5d3c815ec2/bin-sley-contract.json
./target/debug/.fingerprint/sley-7edfbd27c743c377/bin-sley-contract.json
./target/debug/.fingerprint/sley-80505d27365e4706/bin-sley-zjx.json
./target/debug/.fingerprint/sley-84f220819134fb85/lib-sley.json
./target/debug/.fingerprint/sley-892c8d8ef72aa7f5/test-integration-test-sley_lsp.json
./target/debug/.fingerprint/sley-8adff3e3089ece40/test-bin-sley.json
./target/debug/.fingerprint/sley-8b579fdaf51cc756/bin-sley.json
./target/debug/.fingerprint/sley-8dcab5c3743c87c8/bin-sley-docgen.json
./target/debug/.fingerprint/sley-93caa743f8c8662b/test-lib-sley.json
./target/debug/.fingerprint/sley-9549415af81b58ab/test-bin-sley-conformance.json
./target/debug/.fingerprint/sley-971e5936b597667c/test-integration-test-sley_templates.json
./target/debug/.fingerprint/sley-979ed5509846d028/lib-sley.json
./target/debug/.fingerprint/sley-9cd416708a2a01d5/test-integration-test-sley_v0.json
./target/debug/.fingerprint/sley-9d15612b7e295011/test-bin-sley-contract.json
./target/debug/.fingerprint/sley-a11307212a616a9c/test-integration-test-sley_v0.json
./target/debug/.fingerprint/sley-a25ccb4d587ecd9e/test-bin-sley.json
./target/debug/.fingerprint/sley-a67d7afa3f08ecc7/test-integration-test-sley_templates.json
./target/debug/.fingerprint/sley-a89e36141b3afa86/bin-sley-sandbox-runner.json
./target/debug/.fingerprint/sley-aa3c08d53c4d7878/test-integration-test-sley_v0.json
./target/debug/.fingerprint/sley-ab95f3ad97b7ad8c/test-integration-test-sley_migrate.json
./target/debug/.fingerprint/sley-ac7d3bc1d6d6415e/test-integration-test-sley_v0.json
./target/debug/.fingerprint/sley-ad4bf3b6f4c2297d/test-bin-sley.json
./target/debug/.fingerprint/sley-b061d133c97c19ca/test-integration-test-sley_workbench.json
./target/debug/.fingerprint/sley-b30d900f64bb503f/test-integration-test-sley_workbench.json
./target/debug/.fingerprint/sley-b58abafb9199cbe2/test-integration-test-sley_lsp.json
./target/debug/.fingerprint/sley-b5fb580242d56802/test-integration-test-sley_v0.json
./target/debug/.fingerprint/sley-b76c6c50ae44ff10/bin-sley-docgen.json
./target/debug/.fingerprint/sley-baab324b2b81e941/test-bin-sley-workbench.json
./target/debug/.fingerprint/sley-bdc53d6de9fda5ea/test-integration-test-sley_migrate.json
./target/debug/.fingerprint/sley-bead671f2cb11360/bin-sley.json
./target/debug/.fingerprint/sley-c38eeadac6229f84/test-integration-test-sley_v0.json
./target/debug/.fingerprint/sley-c3e62e5644714f33/bin-sley-shadow.json
./target/debug/.fingerprint/sley-c40f70a2bf78025a/test-integration-test-sley_templates.json
./target/debug/.fingerprint/sley-c5aae04a5705742d/bin-sley.json
./target/debug/.fingerprint/sley-c9f6923950423379/test-bin-sley-docgen.json
./target/debug/.fingerprint/sley-d1344bbe21f908db/test-lib-sley.json
./target/debug/.fingerprint/sley-d1e70dd922b4ee69/lib-sley.json
./target/debug/.fingerprint/sley-d7d358b15ba3d5fc/test-integration-test-sley_workbench.json
./target/debug/.fingerprint/sley-d93d4ca2aeb67e1e/bin-sley-sandbox-runner.json
./target/debug/.fingerprint/sley-da1f78bfa80850fe/lib-sley.json
./target/debug/.fingerprint/sley-db99c89b0d982101/test-integration-test-sley_zjx.json
./target/debug/.fingerprint/sley-dea069c5cf3271cf/test-integration-test-sley_v0.json
./target/debug/.fingerprint/sley-debd64a9185199ad/test-integration-test-sley_v0.json
./target/debug/.fingerprint/sley-e41342607d595a16/test-integration-test-sley_v0.json
./target/debug/.fingerprint/sley-ebbebb167a460e5c/test-integration-test-sley_zjx.json
./target/debug/.fingerprint/sley-ecbaa64d702f2166/bin-sley-ci.json
./target/debug/.fingerprint/sley-ed32fe814b8a5680/test-bin-sley-zjx.json
./target/debug/.fingerprint/sley-ee8e4caf77cb58e4/test-integration-test-sley_v0.json
./target/debug/.fingerprint/sley-f3d8057a2b5472e3/test-integration-test-sley_agent_bench.json
./target/debug/.fingerprint/sley-f49bc810b45ddf9c/lib-sley.json
./target/debug/.fingerprint/sley-f4fcdd708f72a1d4/test-integration-test-sley_agent_bench.json
./target/debug/.fingerprint/sley-f503304073074dce/bin-sley-zjx.json
./target/debug/.fingerprint/sley-f5f1773a334b2d1b/test-integration-test-sley_lsp.json
./target/debug/.fingerprint/sley-f82458b0a657ecbb/bin-sley-ci.json
./target/debug/.fingerprint/sley-fa96792765456979/test-lib-sley.json
./target/debug/.fingerprint/sley-fe5c05bc0a6f6352/test-integration-test-sley_v0.json
./target/debug/.fingerprint/sley-fe8ca0cd3c641488/bin-sley-contract.json
./target/debug/.fingerprint/sley-ff927b2ad9f7e50f/test-integration-test-sley_v0.json
./target/debug/.fingerprint/sley-ffb2f6c91610ff1f/test-integration-test-sley_v0.json
./target/debug/.fingerprint/smallvec-072d30207717aa4d/lib-smallvec.json
./target/debug/.fingerprint/smallvec-7d52498a394f2dd3/lib-smallvec.json
./target/debug/.fingerprint/stable_deref_trait-9904e5659a5b8ab0/lib-stable_deref_trait.json
./target/debug/.fingerprint/stable_deref_trait-c1d6929326a6b1a9/lib-stable_deref_trait.json
./target/debug/.fingerprint/strsim-817aa286a9f20c0c/lib-strsim.json
./target/debug/.fingerprint/strsim-84c9e45cc50be838/lib-strsim.json
./target/debug/.fingerprint/syn-91e73aeae2f025d2/lib-syn.json
./target/debug/.fingerprint/syn-951e202364614414/lib-syn.json
./target/debug/.fingerprint/synstructure-9ff513407303b820/lib-synstructure.json
./target/debug/.fingerprint/time-0e93e4d08f4cee27/lib-time.json
./target/debug/.fingerprint/time-507d3091ea8f707e/lib-time.json
./target/debug/.fingerprint/time-core-05523f43e90c145f/lib-time_core.json
./target/debug/.fingerprint/time-core-212e940e31e04bcb/lib-time_core.json
./target/debug/.fingerprint/tinystr-923097809ba95387/lib-tinystr.json
./target/debug/.fingerprint/tinystr-9d8241dbc745a9ab/lib-tinystr.json
./target/debug/.fingerprint/toml-559ce7d0b7b010c3/lib-toml.json
./target/debug/.fingerprint/toml-dcc2a51eb68fef9d/lib-toml.json
./target/debug/.fingerprint/toml_datetime-6ddeda8cc02dd3aa/lib-toml_datetime.json
./target/debug/.fingerprint/toml_datetime-dc0d3e4a75816ccd/lib-toml_datetime.json
./target/debug/.fingerprint/toml_parser-a7a99c306ed8faaa/lib-toml_parser.json
./target/debug/.fingerprint/toml_parser-c436416571061de3/lib-toml_parser.json
./target/debug/.fingerprint/toml_writer-a6628090821deef1/lib-toml_writer.json
./target/debug/.fingerprint/toml_writer-d9adbfa2667f813a/lib-toml_writer.json
./target/debug/.fingerprint/typenum-8429a56cbeb3dcc8/lib-typenum.json
./target/debug/.fingerprint/typenum-aa9b4271e041a3f4/lib-typenum.json
./target/debug/.fingerprint/unicode-general-category-1fe9f76060025286/build-script-build-script-build.json
./target/debug/.fingerprint/unicode-general-category-bfc5fb54dfb63fa7/lib-unicode_general_category.json
./target/debug/.fingerprint/unicode-general-category-c438ad9ff12db904/run-build-script-build-script-build.json
./target/debug/.fingerprint/unicode-general-category-e8c819bea7904406/lib-unicode_general_category.json
./target/debug/.fingerprint/unicode-ident-6c344b3cd407a301/lib-unicode_ident.json
./target/debug/.fingerprint/utf8_iter-0c73a6391eb09350/lib-utf8_iter.json
./target/debug/.fingerprint/utf8_iter-657435fd4aa9af70/lib-utf8_iter.json
./target/debug/.fingerprint/utf8parse-46120df9309f3ced/lib-utf8parse.json
./target/debug/.fingerprint/utf8parse-73bed9f9a2974e49/lib-utf8parse.json
./target/debug/.fingerprint/uuid-simd-27fe028a975ed02f/lib-uuid_simd.json
./target/debug/.fingerprint/uuid-simd-d50c422e596d2f12/lib-uuid_simd.json
./target/debug/.fingerprint/version_check-53da2be791d0fddd/lib-version_check.json
./target/debug/.fingerprint/vsimd-4f10e8f109fe969a/lib-vsimd.json
./target/debug/.fingerprint/vsimd-7dd94f797cf9276e/lib-vsimd.json
./target/debug/.fingerprint/weavelang-003f0206bea541c9/lib-weavelang.json
./target/debug/.fingerprint/weavelang-05e0564d436ca6f1/test-bin-weavelang.json
./target/debug/.fingerprint/weavelang-080138ff8c4ac6ff/bin-weavelang.json
./target/debug/.fingerprint/weavelang-1202f0a8ff8e81a2/test-integration-test-weave_v0.json
./target/debug/.fingerprint/weavelang-1c56328373fcd919/test-bin-weavelang.json
./target/debug/.fingerprint/weavelang-3116c2dd6a7b145e/bin-weavelang.json
./target/debug/.fingerprint/weavelang-31721b4e64fc6c1e/lib-weavelang.json
./target/debug/.fingerprint/weavelang-3a9498a91c0b214b/test-bin-weavelang.json
./target/debug/.fingerprint/weavelang-3f222531d5ac384f/test-lib-weavelang.json
./target/debug/.fingerprint/weavelang-5ce7e89b28770bbf/lib-weavelang.json
./target/debug/.fingerprint/weavelang-5d6a9db6e42500d3/test-integration-test-weave_v0.json
./target/debug/.fingerprint/weavelang-77d16e11d7477a01/bin-weavelang.json
./target/debug/.fingerprint/weavelang-833a0fe155aba4b4/test-lib-weavelang.json
./target/debug/.fingerprint/weavelang-a67c386abcd72712/test-bin-weavelang.json
./target/debug/.fingerprint/weavelang-b4dd67cf75b369d0/bin-weavelang.json
./target/debug/.fingerprint/weavelang-c311c86098b8d619/test-lib-weavelang.json
./target/debug/.fingerprint/weavelang-c7444baf6b7f3d8a/lib-weavelang.json
./target/debug/.fingerprint/weavelang-cccdd2c045233b00/test-integration-test-weave_v0.json
./target/debug/.fingerprint/weavelang-d7e5d0edc0b9de47/test-lib-weavelang.json
./target/debug/.fingerprint/weavelang-db7eb7775b2a7792/bin-weavelang.json
./target/debug/.fingerprint/weavelang-ded40b6dbe6a8269/test-integration-test-weave_v0.json
./target/debug/.fingerprint/weavelang-e0c44edb11c0f533/lib-weavelang.json
./target/debug/.fingerprint/winnow-3148e2c3526050f1/lib-winnow.json
./target/debug/.fingerprint/winnow-8885125b01534fea/lib-winnow.json
./target/debug/.fingerprint/writeable-02602fa919826d75/lib-writeable.json
./target/debug/.fingerprint/writeable-8f0dcb7dd50fe6d9/lib-writeable.json
./target/debug/.fingerprint/yoke-7f0dab759591d44a/lib-yoke.json
./target/debug/.fingerprint/yoke-da2922f2f6918072/lib-yoke.json
./target/debug/.fingerprint/yoke-derive-fadc1def9bace5c5/lib-yoke_derive.json
./target/debug/.fingerprint/zerocopy-138e4deea02cc524/lib-zerocopy.json
./target/debug/.fingerprint/zerocopy-3e4c106aef611585/run-build-script-build-script-build.json
./target/debug/.fingerprint/zerocopy-bbad1e0682c1bd67/build-script-build-script-build.json
./target/debug/.fingerprint/zerocopy-f806c24807f719fb/lib-zerocopy.json
./target/debug/.fingerprint/zerofrom-acca26badf951864/lib-zerofrom.json
./target/debug/.fingerprint/zerofrom-cee16d513aa14890/lib-zerofrom.json
./target/debug/.fingerprint/zerofrom-derive-dd5b8bc3899347de/lib-zerofrom_derive.json
./target/debug/.fingerprint/zerotrie-7ccdcdea0ee7fa4c/lib-zerotrie.json
./target/debug/.fingerprint/zerotrie-e3aad84b367bcc01/lib-zerotrie.json
./target/debug/.fingerprint/zerovec-4f7efec4b0fc8124/lib-zerovec.json
./target/debug/.fingerprint/zerovec-derive-18bc5f4e550270ff/lib-zerovec_derive.json
./target/debug/.fingerprint/zerovec-eda3cda9b3054d66/lib-zerovec.json
./target/debug/.fingerprint/zmij-913b8a75377514b0/lib-zmij.json
./target/debug/.fingerprint/zmij-936c2e1741b0d707/lib-zmij.json
./target/debug/.fingerprint/zmij-a6d8e8a8dc8466df/run-build-script-build-script-build.json
./target/debug/.fingerprint/zmij-d413051177c71f35/build-script-build-script-build.json
./tree-sitter-sley/README.md
./tree-sitter-sley/grammar.js
./tree-sitter-sley/node_modules/.package-lock.json
./tree-sitter-sley/node_modules/tree-sitter-cli/README.md
./tree-sitter-sley/node_modules/tree-sitter-cli/cli.js
./tree-sitter-sley/node_modules/tree-sitter-cli/dsl.d.ts
./tree-sitter-sley/node_modules/tree-sitter-cli/install.js
./tree-sitter-sley/node_modules/tree-sitter-cli/package.json
./tree-sitter-sley/package-lock.json
./tree-sitter-sley/package.json
./tree-sitter-sley/scripts/parse-fixtures.mjs
./tree-sitter-sley/src/grammar.json
./tree-sitter-sley/src/node-types.json
./tree-sitter-sley/tree-sitter.json

## Notes

- This report is generated with `./scripts/self-hosting-inventory.sh`.
- Treat as evidence for phase planning, not proof of runtime parity.
