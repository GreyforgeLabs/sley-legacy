use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};

use sley::ast::{AST_PROGRAM_SCHEMA, ExprKind, ProvenanceRecord, StatementKind};
use sley::authority::{host_effect_contracts, host_effects_for_callee};
use sley::checker::{check_program, has_errors};
use sley::diagnostics::{DIAGNOSTIC_REPORT_SCHEMA, DiagnosticReport};
use sley::doctor::{DOCTOR_REPORT_SCHEMA, build_doctor_report};
use sley::formatter::format_program;
use sley::graft::{
    GRAFT_OUTCOME_SCHEMA, GraftInput, GraftOutcome, apply_graft_input, apply_graft_program,
};
use sley::lint::{LINT_REPORT_SCHEMA, LintOptions, LintRule, build_lint_report};
use sley::parser::parse_program;
use sley::plan::{
    EDIT_PLAN_REPORT_SCHEMA, EditPlanOptions, build_edit_plan_report,
    build_edit_plan_report_with_options,
};
use sley::project::load_project;
use sley::query::{QUERY_REPORT_SCHEMA, QueryKind, QueryOptions, build_query_report};
use sley::runtime::{RuntimeGates, Value, run_main, run_main_with_gates};
use sley::scaffold::{
    PROJECT_SCAFFOLD_SCHEMA, ProjectScaffoldReport, ProjectScaffoldSummary, ScaffoldFile,
};
use sley::symbols::{SYMBOL_GRAPH_SCHEMA, SYMBOL_GRAPH_SLICE_SCHEMA, slice_symbol_graph};
use sley::trace::{
    TRACE_RECEIPT_SCHEMA, TRACE_SEAL_SCHEMA, TraceReceipt, append_trace_receipt,
    build_trace_receipt, build_trace_seal, read_trace_receipts,
};
use sley::verify::{VERIFY_REPORT_SCHEMA, build_verify_report};

#[derive(Debug, serde::Deserialize)]
struct CorpusExpectation {
    diagnostics: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct CorpusManifest {
    schema: String,
    accepted: Vec<CorpusManifestCase>,
    rejected: Vec<CorpusManifestCase>,
}

#[derive(Debug, serde::Deserialize)]
struct CorpusManifestCase {
    path: String,
    covers: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct CliSmokeManifest {
    schema: String,
    cases: Vec<CliSmokeCase>,
}

#[derive(Debug, serde::Deserialize)]
struct CliSmokeCase {
    name: String,
    #[serde(default)]
    cwd: CliSmokeCwd,
    args: Vec<String>,
    covers: Vec<String>,
    expect: CliSmokeExpectation,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum CliSmokeCwd {
    #[default]
    Repo,
    Tmp,
}

#[derive(Debug, serde::Deserialize)]
struct CliSmokeExpectation {
    success: bool,
    #[serde(default)]
    stdout_contains: Vec<String>,
    #[serde(default)]
    stderr_contains: Vec<String>,
    #[serde(default)]
    stdout_json: Vec<CliSmokeJsonExpectation>,
}

#[derive(Debug, serde::Deserialize)]
struct CliSmokeJsonExpectation {
    pointer: String,
    value: serde_json::Value,
}

#[test]
fn host_effect_contract_table_is_canonical() {
    let mut callees = BTreeSet::new();
    let mut source_needles = BTreeSet::new();

    for contract in host_effect_contracts() {
        assert!(
            callees.insert(contract.callee),
            "duplicate host callee {}",
            contract.callee
        );
        assert!(
            source_needles.insert(contract.source_needle),
            "duplicate host source needle {}",
            contract.source_needle
        );
        assert!(
            !contract.effects.is_empty(),
            "host callee {} has no authority effects",
            contract.callee
        );
        assert!(
            contract.source_needle.starts_with(contract.callee),
            "source needle {} should identify callee {}",
            contract.source_needle,
            contract.callee
        );
        assert!(
            contract.source_needle.ends_with('('),
            "source needle {} should match call syntax",
            contract.source_needle
        );
        assert_eq!(
            host_effects_for_callee(contract.callee),
            Some(contract.effects)
        );
    }

    assert_eq!(
        host_effects_for_callee("db.query_one"),
        Some(&["DatabaseRead", "DbRead"][..])
    );
    assert_eq!(
        host_effects_for_callee("deploy.try_stage"),
        Some(&["Deploy"][..])
    );
    assert_eq!(host_effects_for_callee("unknown.try_call"), None);
}

#[test]
fn profile_fixture_checks_cleanly() {
    let source = include_str!("../examples/profile_service.sley");
    let program = parse_program(source).expect("parse fixture");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
}

#[test]
fn formatter_round_trips_profile_fixture() {
    let source = include_str!("../examples/profile_service.sley");
    let program = parse_program(source).expect("parse fixture");
    let formatted = format_program(&program);
    let reparsed = parse_program(&formatted).expect("parse formatted fixture");
    assert_eq!(formatted, format_program(&reparsed));
}

#[test]
fn parser_expected_token_errors_include_repair_hints() {
    let parse_err = |source: &str| match parse_program(source) {
        Ok(program) => panic!("expected parse error, got {program:#?}"),
        Err(diagnostics) => diagnostics,
    };

    let arrow_diagnostics = parse_err("task main Int {\n  return 1\n}\n");
    let hint = find_repair_hint(
        &arrow_diagnostics,
        "PARSE_EXPECTED_ARROW",
        "insert_expected_token",
    );
    assert_eq!(hint.replacement.as_deref(), Some("->"));

    let identifier_diagnostics = parse_err("task -> Int {\n  return 1\n}\n");
    let hint = find_repair_hint(
        &identifier_diagnostics,
        "PARSE_EXPECTED_IDENTIFIER",
        "provide_identifier",
    );
    assert_eq!(hint.replacement.as_deref(), Some("identifier"));

    let symbol_diagnostics = parse_err("task main -> Int {\n  bind value 1\n}\n");
    let hint = find_repair_hint(
        &symbol_diagnostics,
        "PARSE_EXPECTED_SYMBOL",
        "insert_expected_token",
    );
    assert_eq!(hint.replacement.as_deref(), Some("="));

    let expression_diagnostics = parse_err("task main -> Int {\n  return\n}\n");
    let hint = find_repair_hint(
        &expression_diagnostics,
        "PARSE_EXPECTED_EXPRESSION",
        "provide_expression",
    );
    assert_eq!(hint.replacement.as_deref(), Some("TODO_VALUE"));
}

#[test]
fn formatter_round_trips_every_example_and_project_module() {
    let examples_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");
    let example_files = collect_sley_files(&examples_root).expect("collect examples");
    assert!(
        example_files.len() >= 7,
        "expected all examples plus project modules, got {example_files:#?}"
    );

    for file in example_files {
        let source =
            fs::read_to_string(&file).unwrap_or_else(|error| panic!("read {file:?}: {error}"));
        let program = parse_program(&source).unwrap_or_else(|diagnostics| {
            panic!("parse example {file:?}: {diagnostics:#?}");
        });
        let formatted = format_program(&program);
        let reparsed = parse_program(&formatted).unwrap_or_else(|diagnostics| {
            panic!("parse formatted example {file:?}: {diagnostics:#?}");
        });
        assert_eq!(
            formatted,
            format_program(&reparsed),
            "formatter is not stable for {file:?}"
        );
    }
}

#[test]
fn synthetic_gold_corpus_accepts_and_rejects_expected_cases() {
    let corpus_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/corpus");
    let manifest = load_corpus_manifest(&corpus_root);
    let accepted_files =
        collect_sley_files(&corpus_root.join("accepted")).expect("collect accepted corpus");
    assert!(
        !accepted_files.is_empty(),
        "accepted corpus should contain at least one fixture"
    );
    let rejected_files =
        collect_sley_files(&corpus_root.join("rejected")).expect("collect rejected corpus");
    assert!(
        !rejected_files.is_empty(),
        "rejected corpus should contain at least one fixture"
    );
    assert_corpus_manifest_matches_files(&corpus_root, &manifest, &accepted_files, &rejected_files);
    assert_corpus_manifest_has_release_coverage(&manifest);

    for file in &accepted_files {
        let source =
            fs::read_to_string(file).unwrap_or_else(|error| panic!("read {file:?}: {error}"));
        let program = parse_program(&source).unwrap_or_else(|diagnostics| {
            panic!("parse accepted corpus fixture {file:?}: {diagnostics:#?}");
        });
        let diagnostics = check_program(&program);
        assert!(
            !has_errors(&diagnostics),
            "accepted corpus fixture {file:?} produced diagnostics: {diagnostics:#?}"
        );

        let formatted = format_program(&program);
        let reparsed = parse_program(&formatted).unwrap_or_else(|diagnostics| {
            panic!("parse formatted accepted corpus fixture {file:?}: {diagnostics:#?}");
        });
        assert_eq!(
            formatted,
            format_program(&reparsed),
            "accepted corpus formatter is not stable for {file:?}"
        );
    }

    for file in &rejected_files {
        let expectation_path = file.with_extension("json");
        let expectation_source = fs::read_to_string(&expectation_path).unwrap_or_else(|error| {
            panic!("read rejected corpus expectation {expectation_path:?}: {error}")
        });
        let expectation: CorpusExpectation = serde_json::from_str(&expectation_source)
            .unwrap_or_else(|error| {
                panic!("parse rejected corpus expectation {expectation_path:?}: {error}")
            });
        let source =
            fs::read_to_string(file).unwrap_or_else(|error| panic!("read {file:?}: {error}"));
        let diagnostics = match parse_program(&source) {
            Ok(program) => check_program(&program),
            Err(diagnostics) => diagnostics,
        };

        assert!(
            has_errors(&diagnostics),
            "rejected corpus fixture {file:?} should fail"
        );
        for expected in &expectation.diagnostics {
            assert!(
                diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.id == *expected),
                "expected diagnostic {expected} for {file:?}, got {diagnostics:#?}"
            );
        }
    }
}

#[test]
fn cli_smoke_manifest_commands_match_stable_release_surface() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest_root = repo_root.join("fixtures/cli_smokes");
    let manifest = load_cli_smoke_manifest(&manifest_root);
    assert_cli_smoke_manifest_is_well_formed(&manifest);
    assert_cli_smoke_manifest_has_release_coverage(&manifest);

    let tmp_root = temp_project_dir("cli-smokes");
    fs::create_dir_all(&tmp_root).expect("create CLI smoke temp dir");

    for case in &manifest.cases {
        let args = expand_cli_smoke_args(&case.args, &repo_root, &tmp_root);
        let cwd = match case.cwd {
            CliSmokeCwd::Repo => &repo_root,
            CliSmokeCwd::Tmp => &tmp_root,
        };
        let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
            .current_dir(cwd)
            .args(&args)
            .output()
            .unwrap_or_else(|error| panic!("run CLI smoke {}: {error}", case.name));
        let stdout = String::from_utf8(output.stdout)
            .unwrap_or_else(|error| panic!("CLI smoke {} stdout utf8: {error}", case.name));
        let stderr = String::from_utf8(output.stderr)
            .unwrap_or_else(|error| panic!("CLI smoke {} stderr utf8: {error}", case.name));
        assert_eq!(
            output.status.success(),
            case.expect.success,
            "CLI smoke {} exit mismatch\nargs: {args:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
            case.name
        );

        for expected in &case.expect.stdout_contains {
            assert!(
                stdout.contains(expected),
                "CLI smoke {} stdout did not contain {expected:?}\nstdout:\n{stdout}",
                case.name
            );
        }
        for expected in &case.expect.stderr_contains {
            assert!(
                stderr.contains(expected),
                "CLI smoke {} stderr did not contain {expected:?}\nstderr:\n{stderr}",
                case.name
            );
        }
        if !case.expect.stdout_json.is_empty() {
            let json: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|error| {
                panic!("CLI smoke {} stdout JSON: {error}\n{stdout}", case.name)
            });
            for expectation in &case.expect.stdout_json {
                let actual = json.pointer(&expectation.pointer).unwrap_or_else(|| {
                    panic!(
                        "CLI smoke {} missing JSON pointer {}\njson:\n{}",
                        case.name, expectation.pointer, json
                    )
                });
                assert_eq!(
                    actual, &expectation.value,
                    "CLI smoke {} JSON pointer {} mismatch",
                    case.name, expectation.pointer
                );
            }
        }
    }

    let _ = fs::remove_dir_all(tmp_root);
}

#[test]
fn project_scaffold_creates_checked_deploy_project() {
    let root = temp_project_dir("new-deploy");
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "new",
            "--json",
            "--template",
            "deploy",
            "--name",
            "agent-app",
            "--module",
            "app.main",
        ])
        .arg(&root)
        .output()
        .expect("run sley new");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(
        output.status.success(),
        "stdout: {stdout}\nstderr: {stderr}"
    );
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("parse scaffold JSON");
    assert_eq!(
        json.pointer("/schema"),
        Some(&serde_json::json!(PROJECT_SCAFFOLD_SCHEMA))
    );
    assert_eq!(json.pointer("/status"), Some(&serde_json::json!("created")));
    assert_eq!(
        json.pointer("/project/template"),
        Some(&serde_json::json!("deploy"))
    );
    assert!(root.join("sley.toml").exists());
    assert!(root.join("README.md").exists());
    assert!(root.join("src/app/main.sley").exists());

    let project = load_project(&root).expect("load scaffolded project");
    let diagnostics = check_program(&project.program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected scaffold diagnostics: {diagnostics:#?}"
    );

    let lint_output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["lint", "--json", "--deny-warnings"])
        .arg(&root)
        .output()
        .expect("lint scaffolded project");
    assert!(
        lint_output.status.success(),
        "lint stdout: {}\nlint stderr: {}",
        String::from_utf8(lint_output.stdout).expect("lint stdout utf8"),
        String::from_utf8(lint_output.stderr).expect("lint stderr utf8")
    );

    let run_output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "run",
            "--json",
            "--cap",
            "Deploy",
            "--deploy-result",
            "staging",
            "staged",
        ])
        .arg(&root)
        .output()
        .expect("run scaffolded project");
    let run_stdout = String::from_utf8(run_output.stdout).expect("run stdout utf8");
    let run_stderr = String::from_utf8(run_output.stderr).expect("run stderr utf8");
    assert!(
        run_output.status.success(),
        "stdout: {run_stdout}\nstderr: {run_stderr}"
    );
    let value: Value = serde_json::from_str(&run_stdout).expect("parse runtime JSON");
    assert_eq!(
        value,
        Value::Ok(Box::new(Value::Text("staged".to_string())))
    );

    let overwrite = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["new", "--json", "--name", "agent-app"])
        .arg(&root)
        .output()
        .expect("rerun sley new");
    let overwrite_stdout = String::from_utf8(overwrite.stdout).expect("overwrite stdout utf8");
    assert!(
        !overwrite.status.success(),
        "second scaffold should reject existing files"
    );
    let overwrite_json: serde_json::Value =
        serde_json::from_str(&overwrite_stdout).expect("parse overwrite diagnostics");
    assert_eq!(
        overwrite_json.pointer("/schema"),
        Some(&serde_json::json!("sley.diagnostics.report.v0"))
    );
    assert!(
        overwrite_json
            .pointer("/diagnostics/0/id")
            .is_some_and(|id| id == "PROJECT_SCAFFOLD_FILE_EXISTS")
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn doctor_report_summarizes_readiness_and_lint_gates() {
    let ready = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["doctor", "--json", "examples/project"])
        .output()
        .expect("run doctor");
    let ready_stdout = String::from_utf8(ready.stdout).expect("ready stdout utf8");
    let ready_stderr = String::from_utf8(ready.stderr).expect("ready stderr utf8");
    assert!(
        ready.status.success(),
        "stdout: {ready_stdout}\nstderr: {ready_stderr}"
    );
    let ready_json: serde_json::Value =
        serde_json::from_str(&ready_stdout).expect("parse ready doctor JSON");
    assert_eq!(
        ready_json.pointer("/schema"),
        Some(&serde_json::json!(DOCTOR_REPORT_SCHEMA))
    );
    assert_eq!(
        ready_json.pointer("/status"),
        Some(&serde_json::json!("ready"))
    );
    assert_eq!(
        ready_json.pointer("/query/source_schema"),
        Some(&serde_json::json!("sley.query.report.v0"))
    );
    assert_eq!(
        ready_json.pointer("/lint/source_schema"),
        Some(&serde_json::json!("sley.lint.report.v0"))
    );

    let root = temp_project_dir("doctor-warnings");
    fs::create_dir_all(&root).expect("create doctor temp dir");
    let warning_file = root.join("warning.sley");
    fs::write(
        &warning_file,
        r#"
module app.warning

task main -> Int {
  return 1
}

task orphan -> Int {
  return 2
}
"#,
    )
    .expect("write warning source");

    let warnings = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["doctor", "--json"])
        .arg(&warning_file)
        .output()
        .expect("run warning doctor");
    let warnings_stdout = String::from_utf8(warnings.stdout).expect("warnings stdout utf8");
    assert!(
        warnings.status.success(),
        "doctor warnings should be non-blocking by default"
    );
    let warnings_json: serde_json::Value =
        serde_json::from_str(&warnings_stdout).expect("parse warning doctor JSON");
    assert_eq!(
        warnings_json.pointer("/status"),
        Some(&serde_json::json!("warnings"))
    );
    assert_eq!(
        warnings_json.pointer("/summary/lint_finding_count"),
        Some(&serde_json::json!(1))
    );

    let denied = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["doctor", "--json", "--deny-warnings"])
        .arg(&warning_file)
        .output()
        .expect("run denied warning doctor");
    let denied_stdout = String::from_utf8(denied.stdout).expect("denied stdout utf8");
    assert!(
        !denied.status.success(),
        "doctor --deny-warnings should block on lint findings"
    );
    let denied_json: serde_json::Value =
        serde_json::from_str(&denied_stdout).expect("parse denied doctor JSON");
    assert_eq!(
        denied_json.pointer("/status"),
        Some(&serde_json::json!("blocked"))
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn edit_plan_report_ranks_query_surfaces_and_carries_lint_findings() {
    let source = r#"
module app.plan

task main -> Int {
  return call used()
}

task used -> Int {
  return 1
}

task orphan -> Int {
  return 2
}
"#;
    let program = parse_program(source).expect("parse plan fixture");
    let report = build_edit_plan_report("app.plan", Ok(program.clone()), false);
    assert_eq!(report.schema, EDIT_PLAN_REPORT_SCHEMA);
    assert_eq!(report.status, "warnings");
    assert_eq!(
        report.query.as_ref().expect("query summary").source_schema,
        QUERY_REPORT_SCHEMA
    );
    assert_eq!(
        report.lint.as_ref().expect("lint summary").source_schema,
        LINT_REPORT_SCHEMA
    );
    assert_eq!(report.lint.as_ref().expect("lint summary").finding_count, 1);
    assert_eq!(
        report.task_surfaces[0].planning_notes,
        vec!["entrypoint_surface", "has_outbound_calls"]
    );
    assert_eq!(report.task_surfaces[0].id, "task:app.plan.main");
    assert_eq!(
        report.task_surfaces[0].graft_targets,
        vec!["task:app.plan.main", "module:app.plan:tasks"]
    );
    assert_eq!(
        report.next_actions[0].kind, "repair_lint_findings",
        "warning plans should lead with the lint repair surface"
    );
    assert!(
        report.graft_templates.is_empty(),
        "default plan reports should stay compact unless templates are requested"
    );

    let denied = build_edit_plan_report("app.plan", Ok(program), true);
    assert_eq!(denied.status, "blocked");
    assert_eq!(denied.summary.lint_finding_count, 1);
    assert_eq!(
        denied.lint.as_ref().expect("denied lint").findings[0].id,
        "UNUSED_PRIVATE_TASK"
    );
}

#[test]
fn edit_plan_report_can_emit_primary_surface_graft_templates() {
    let source = r#"
module app.plan

task main -> Result<Text, Error> {
  return Ok("ready")
}
"#;
    let program = parse_program(source).expect("parse template plan fixture");
    let report = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
        },
    );
    assert_eq!(report.status, "ready");
    assert!(report.graft_templates.len() >= 6);
    assert_eq!(report.graft_templates[0].kind, "replace_task_body");
    assert_eq!(
        report.graft_templates[0]
            .operation
            .pointer("/payload/statements/0"),
        Some(&serde_json::json!("return Ok(\"\")"))
    );
    assert_eq!(
        report.graft_templates[1].operation.pointer("/op"),
        Some(&serde_json::json!("RenameDeclaration"))
    );
    assert_eq!(
        report.graft_templates[2].editable_json_pointers,
        vec!["/payload/name", "/payload/type", "/payload/position"]
    );
    assert_eq!(
        report.graft_templates[3]
            .operation
            .pointer("/payload/parent"),
        Some(&serde_json::json!("module:app.plan:tasks"))
    );
    assert_eq!(report.graft_templates[4].kind, "move_statement");
    assert_eq!(
        report.graft_templates[4].operation.pointer("/target"),
        Some(&serde_json::json!("block:task:app.plan.main:stmt:0"))
    );
    assert_eq!(
        report.graft_templates[4].editable_json_pointers,
        vec!["/payload/position".to_string()]
    );
    let expression_template = report
        .graft_templates
        .iter()
        .find(|template| {
            template.kind == "replace_expression"
                && template.operation.pointer("/target")
                    == Some(&serde_json::json!("block:task:app.plan.main:stmt:0:expr"))
        })
        .expect("replace expression template");
    assert_eq!(
        expression_template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        expression_template.editable_json_pointers,
        vec!["/payload/source".to_string()]
    );
    for template in &report.graft_templates {
        serde_json::from_value::<GraftInput>(template.operation.clone())
            .expect("template should be strict graft JSON");
    }
    let replace_template: GraftInput =
        serde_json::from_value(report.graft_templates[0].operation.clone())
            .expect("parse replace template");
    let outcome = apply_graft_input(
        &program,
        replace_template,
        Some("agent:template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted");
}

#[test]
fn edit_plan_graft_templates_can_target_named_surfaces() {
    let source = r#"
module app.plan

task main -> Int {
  return call helper(1)
}

task helper -> Int {
  take value: Int

  return value
}
"#;
    let program = parse_program(source).expect("parse named surface plan fixture");
    let report = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("task:app.plan.helper".to_string()),
        },
    );
    assert_eq!(report.status, "ready");
    assert_eq!(report.graft_templates[0].surface, "task:app.plan.helper");
    assert!(
        report
            .graft_templates
            .iter()
            .any(|template| template.kind == "move_take"
                && template.operation.pointer("/target")
                    == Some(&serde_json::json!("take:task:app.plan.helper:0:value"))),
        "expected graph-slice take move template, got {:#?}",
        report.graft_templates
    );
    assert_eq!(
        report.graft_templates[0].operation.pointer("/target"),
        Some(&serde_json::json!("task:app.plan.helper"))
    );
    assert_eq!(report.transaction_templates.len(), 2);
    assert_eq!(
        report.transaction_templates[0].kind,
        "rename_and_update_call_sites"
    );
    assert_eq!(
        report.transaction_templates[0]
            .transaction
            .pointer("/ops/1/payload/from"),
        Some(&serde_json::json!("helper"))
    );
    assert_eq!(
        report.transaction_templates[0]
            .transaction
            .pointer("/ops/1/payload/replacement"),
        Some(&serde_json::json!("renamed_helper"))
    );
    let transaction_template: GraftInput =
        serde_json::from_value(report.transaction_templates[0].transaction.clone())
            .expect("parse transaction template");
    let outcome = apply_graft_input(
        &program,
        transaction_template,
        Some("agent:transaction-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted");
    assert_eq!(
        report.transaction_templates[1].kind,
        "add_take_and_update_call_args"
    );
    assert_eq!(
        report.transaction_templates[1]
            .transaction
            .pointer("/ops/1/op"),
        Some(&serde_json::json!("UpdateCallArgs"))
    );
    assert_eq!(
        report.transaction_templates[1]
            .transaction
            .pointer("/ops/1/payload/source"),
        Some(&serde_json::json!("\"\""))
    );
    let add_take_transaction: GraftInput =
        serde_json::from_value(report.transaction_templates[1].transaction.clone())
            .expect("parse add-take transaction template");
    let outcome = apply_graft_input(
        &program,
        add_take_transaction,
        Some("agent:add-take-transaction-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);

    let missing = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("task:app.plan.missing".to_string()),
        },
    );
    assert_eq!(missing.status, "blocked");
    assert!(missing.graft_templates.is_empty());
    assert_eq!(missing.diagnostics[0].id, "PLAN_SURFACE_NOT_FOUND");
    assert_eq!(missing.next_actions[0].kind, "inspect_plan_surfaces");
}

#[test]
fn edit_plan_graft_templates_include_move_destinations() {
    let source = r#"
module app.plan

export task helper -> Int {
  take unused: Int

  return 1
}

task main -> Int {
  tally total = 1
  if true {
    set total = total + 1
  }
  return total
}
"#;
    let program = parse_program(source).expect("parse destination template fixture");
    let main_report = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("task:app.plan.main".to_string()),
        },
    );
    assert_eq!(main_report.status, "warnings");
    let statement_destination = main_report
        .graft_templates
        .iter()
        .find(|template| {
            template.kind == "move_statement_destination"
                && template.operation.pointer("/target")
                    == Some(&serde_json::json!(
                        "block:task:app.plan.main:stmt:1:then:stmt:0"
                    ))
                && template.operation.pointer("/payload/destination")
                    == Some(&serde_json::json!("block:task:app.plan.main"))
        })
        .expect("statement destination template");
    assert_eq!(
        statement_destination.operation.pointer("/payload/parent"),
        Some(&serde_json::json!("block:task:app.plan.main:stmt:1:then"))
    );
    assert_eq!(
        statement_destination.operation.pointer("/payload/position"),
        Some(&serde_json::json!(3))
    );
    assert_eq!(
        statement_destination.editable_json_pointers,
        vec!["/payload/position".to_string()]
    );
    let statement_graft: GraftInput =
        serde_json::from_value(statement_destination.operation.clone())
            .expect("parse statement destination template");
    let statement_outcome = apply_graft_input(
        &program,
        statement_graft,
        Some("agent:statement-destination-template-test".to_string()),
    );
    assert_eq!(
        statement_outcome.status, "accepted",
        "{:#?}",
        statement_outcome.diagnostics
    );

    let helper_report = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("task:app.plan.helper".to_string()),
        },
    );
    assert_eq!(helper_report.status, "warnings");
    let take_destination = helper_report
        .graft_templates
        .iter()
        .find(|template| {
            template.kind == "move_take_destination"
                && template.operation.pointer("/target")
                    == Some(&serde_json::json!("take:task:app.plan.helper:0:unused"))
                && template.operation.pointer("/payload/destination")
                    == Some(&serde_json::json!("task:app.plan.main:takes"))
        })
        .expect("take destination template");
    assert_eq!(
        take_destination.operation.pointer("/payload/parent"),
        Some(&serde_json::json!("task:app.plan.helper:takes"))
    );
    assert_eq!(
        take_destination.operation.pointer("/payload/position"),
        Some(&serde_json::json!(0))
    );
    let take_graft: GraftInput = serde_json::from_value(take_destination.operation.clone())
        .expect("parse take destination template");
    let take_outcome = apply_graft_input(
        &program,
        take_graft,
        Some("agent:take-destination-template-test".to_string()),
    );
    assert_eq!(
        take_outcome.status, "accepted",
        "{:#?}",
        take_outcome.diagnostics
    );
}

#[test]
fn edit_plan_graft_templates_include_checked_delete_affordances() {
    let source = r#"
module app.plan

export task helper -> Int {
  take unused: Int

  return 1
}

task main -> Int {
  bind debug = 1
  return 42
}
"#;
    let program = parse_program(source).expect("parse delete template fixture");
    let main_report = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("task:app.plan.main".to_string()),
        },
    );
    assert_eq!(main_report.status, "warnings");
    let statement_delete = main_report
        .graft_templates
        .iter()
        .find(|template| {
            template.kind == "delete_statement"
                && template.operation.pointer("/target")
                    == Some(&serde_json::json!("block:task:app.plan.main:stmt:0"))
        })
        .expect("checked statement delete template");
    assert_eq!(
        statement_delete.operation.pointer("/op"),
        Some(&serde_json::json!("DeleteNode"))
    );
    assert!(statement_delete.editable_json_pointers.is_empty());
    let statement_graft: GraftInput = serde_json::from_value(statement_delete.operation.clone())
        .expect("parse statement delete template");
    let statement_outcome = apply_graft_input(
        &program,
        statement_graft,
        Some("agent:statement-delete-template-test".to_string()),
    );
    assert_eq!(
        statement_outcome.status, "accepted",
        "{:#?}",
        statement_outcome.diagnostics
    );
    assert!(
        !main_report.graft_templates.iter().any(|template| {
            template.kind == "delete_statement"
                && template.operation.pointer("/target")
                    == Some(&serde_json::json!("block:task:app.plan.main:stmt:1"))
        }),
        "return deletion should be filtered because the checked graft rejects"
    );

    let helper_report = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("task:app.plan.helper".to_string()),
        },
    );
    assert_eq!(helper_report.status, "warnings");
    let take_delete = helper_report
        .graft_templates
        .iter()
        .find(|template| {
            template.kind == "delete_take"
                && template.operation.pointer("/target")
                    == Some(&serde_json::json!("take:task:app.plan.helper:0:unused"))
        })
        .expect("checked take delete template");
    let take_graft: GraftInput =
        serde_json::from_value(take_delete.operation.clone()).expect("parse take delete template");
    let take_outcome = apply_graft_input(
        &program,
        take_graft,
        Some("agent:take-delete-template-test".to_string()),
    );
    assert_eq!(
        take_outcome.status, "accepted",
        "{:#?}",
        take_outcome.diagnostics
    );
}

#[test]
fn edit_plan_graft_templates_include_lint_declaration_deletes() {
    let source = r#"
module app.plan

type Used = {
  slot name: Text
}

type Orphan = {
  slot id: Int
}

effect OrphanEffect

task main -> Used {
  return Used { name: "Ada" }
}
"#;
    let program = parse_program(source).expect("parse lint declaration delete template fixture");
    let report = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 2);

    let type_delete = report
        .graft_templates
        .iter()
        .find(|template| {
            template.kind == "delete_unused_private_type"
                && template.surface == "type:app.plan.Orphan"
                && template.operation.pointer("/target")
                    == Some(&serde_json::json!("type:app.plan.Orphan"))
        })
        .expect("unused private type delete template");
    assert_eq!(
        type_delete.operation.pointer("/op"),
        Some(&serde_json::json!("DeleteNode"))
    );
    assert!(type_delete.editable_json_pointers.is_empty());
    let type_graft: GraftInput =
        serde_json::from_value(type_delete.operation.clone()).expect("parse type delete template");
    let type_outcome = apply_graft_input(
        &program,
        type_graft,
        Some("agent:type-delete-template-test".to_string()),
    );
    assert_eq!(
        type_outcome.status, "accepted",
        "{:#?}",
        type_outcome.diagnostics
    );

    let effect_delete = report
        .graft_templates
        .iter()
        .find(|template| {
            template.kind == "delete_unused_private_effect"
                && template.surface == "effect:app.plan.OrphanEffect"
                && template.operation.pointer("/target")
                    == Some(&serde_json::json!("effect:app.plan.OrphanEffect"))
        })
        .expect("unused private effect delete template");
    let effect_graft: GraftInput = serde_json::from_value(effect_delete.operation.clone())
        .expect("parse effect delete template");
    let effect_outcome = apply_graft_input(
        &program,
        effect_graft,
        Some("agent:effect-delete-template-test".to_string()),
    );
    assert_eq!(
        effect_outcome.status, "accepted",
        "{:#?}",
        effect_outcome.diagnostics
    );

    assert!(
        !report.graft_templates.iter().any(|template| {
            template.kind == "delete_unused_private_type"
                && template.operation.pointer("/target")
                    == Some(&serde_json::json!("type:app.plan.Used"))
        }),
        "used private type should not receive a lint-driven delete template"
    );
    assert_eq!(report.transaction_templates.len(), 1);
    assert_eq!(
        report.transaction_templates[0].kind,
        "delete_unused_private_declarations"
    );
    assert_eq!(
        report.transaction_templates[0].surface,
        "lint:unused_private_declarations"
    );
    assert_eq!(
        report.transaction_templates[0].transaction.pointer("/mode"),
        Some(&serde_json::json!("all_or_nothing"))
    );
    assert_eq!(
        report.transaction_templates[0]
            .transaction
            .pointer("/ops/0/target"),
        Some(&serde_json::json!("effect:app.plan.OrphanEffect"))
    );
    assert_eq!(
        report.transaction_templates[0]
            .transaction
            .pointer("/ops/1/target"),
        Some(&serde_json::json!("type:app.plan.Orphan"))
    );
    assert!(
        report.transaction_templates[0]
            .editable_json_pointers
            .is_empty()
    );
    let cleanup_transaction: GraftInput =
        serde_json::from_value(report.transaction_templates[0].transaction.clone())
            .expect("parse declaration cleanup transaction template");
    let cleanup_outcome = apply_graft_input(
        &program,
        cleanup_transaction,
        Some("agent:declaration-cleanup-transaction-test".to_string()),
    );
    assert_eq!(
        cleanup_outcome.status, "accepted",
        "{:#?}",
        cleanup_outcome.diagnostics
    );

    let targeted_type_report = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("type:app.plan.Orphan".to_string()),
        },
    );
    assert_eq!(targeted_type_report.status, "warnings");
    assert!(targeted_type_report.diagnostics.is_empty());
    assert_eq!(targeted_type_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_type_report.graft_templates[0].kind,
        "delete_unused_private_type"
    );
    assert_eq!(
        targeted_type_report.graft_templates[0]
            .operation
            .pointer("/target"),
        Some(&serde_json::json!("type:app.plan.Orphan"))
    );
    assert!(targeted_type_report.transaction_templates.is_empty());

    let targeted_effect_report = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("effect:app.plan.OrphanEffect".to_string()),
        },
    );
    assert_eq!(targeted_effect_report.status, "warnings");
    assert!(targeted_effect_report.diagnostics.is_empty());
    assert_eq!(targeted_effect_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_effect_report.graft_templates[0].kind,
        "delete_unused_private_effect"
    );
    assert_eq!(
        targeted_effect_report.graft_templates[0]
            .operation
            .pointer("/target"),
        Some(&serde_json::json!("effect:app.plan.OrphanEffect"))
    );
}

#[test]
fn edit_plan_remove_take_transaction_requires_unused_take() {
    let source = r#"
module app.plan

task main -> Int {
  return call helper(41, 1)
}

task helper -> Int {
  take value: Int
  take unused: Int

  return value
}
"#;
    let program = parse_program(source).expect("parse remove-take plan fixture");
    let report = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("task:app.plan.helper".to_string()),
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.transaction_templates.len(), 3);
    assert_eq!(
        report.transaction_templates[2].kind,
        "remove_take_and_remove_call_arg"
    );
    assert_eq!(
        report.transaction_templates[2]
            .transaction
            .pointer("/ops/0/payload/name"),
        Some(&serde_json::json!("unused"))
    );
    assert_eq!(
        report.transaction_templates[2]
            .transaction
            .pointer("/ops/1/op"),
        Some(&serde_json::json!("RemoveCallArg"))
    );
    assert_eq!(
        report.transaction_templates[2]
            .transaction
            .pointer("/ops/1/payload/position"),
        Some(&serde_json::json!(1))
    );
    let remove_take_transaction: GraftInput =
        serde_json::from_value(report.transaction_templates[2].transaction.clone())
            .expect("parse remove-take transaction template");
    let outcome = apply_graft_input(
        &program,
        remove_take_transaction,
        Some("agent:remove-take-transaction-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(!grafted_source.contains("take unused: Int"));
    assert!(grafted_source.contains("return call helper(41)"));
    let grafted = parse_program(&grafted_source).expect("parse grafted source");
    assert_eq!(run_main(&grafted), Ok(Value::Int(41)));
}

#[test]
fn add_take_graft_is_structural_and_checked() {
    let source = include_str!("../examples/profile_service.sley");
    let graft_source = include_str!("../fixtures/grafts/add_tenant_take.json");
    let program = parse_program(source).expect("parse fixture");
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));
    assert_eq!(outcome.status, "accepted");
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("take tenant_id: Text"));
    assert!(grafted_source.contains("take id: Text"));
    assert_eq!(outcome.provenance.len(), 1);
}

#[test]
fn stale_precondition_rejects_graft() {
    let source = include_str!("../examples/profile_service.sley");
    let graft_source = include_str!("../fixtures/grafts/stale_add_tenant_take.json");
    let program = parse_program(source).expect("parse fixture");
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));
    assert_eq!(outcome.status, "rejected");
    let diagnostic = outcome
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "GRAFT_PRECONDITION_FAILED")
        .expect("expected stale graft precondition diagnostic");
    assert_eq!(
        diagnostic.node.as_deref(),
        Some("task:app.profile.get_user")
    );
    let hint = diagnostic
        .repair_hints
        .iter()
        .find(|hint| hint.kind == "refresh_graft_precondition")
        .expect("expected stale graft refresh hint");
    assert_eq!(hint.target.as_deref(), Some("task:app.profile.get_user"));
    assert!(
        hint.replacement
            .as_deref()
            .is_some_and(|replacement| replacement.contains("rebuild the graft")),
        "expected refresh hint replacement guidance, got {hint:#?}"
    );
}

#[test]
fn graft_namespace_conflicts_include_repair_hints() {
    let source = r#"
module app.main
import app.shared

effect Audit

type User = {
  slot id: Int
}

task helper -> Int {
  return 1
}

task main -> Int {
  return call helper()
}
"#;
    let program = parse_program(source).expect("parse namespace conflict source");
    let cases = [
        (
            r#"{ "op": "AddEffectDeclaration", "payload": { "name": "Audit" } }"#,
            "GRAFT_EFFECT_EXISTS",
        ),
        (
            r#"{ "op": "AddImport", "payload": { "module": "app.shared" } }"#,
            "GRAFT_IMPORT_EXISTS",
        ),
        (
            r#"{ "op": "AddTask", "payload": { "source": "module app.main\n\ntask helper -> Int {\n  return 2\n}" } }"#,
            "GRAFT_TASK_EXISTS",
        ),
        (
            r#"{ "op": "AddTypeDeclaration", "payload": { "source": "module app.main\n\ntype User = {\n  slot id: Text\n}" } }"#,
            "GRAFT_TYPE_EXISTS",
        ),
        (
            r#"{ "op": "RenameDeclaration", "target": "module:app.main", "payload": { "name": "app.shared" } }"#,
            "GRAFT_MODULE_EXISTS",
        ),
    ];

    for (graft_source, diagnostic_id) in cases {
        let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
        let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));
        assert_eq!(outcome.status, "rejected", "{diagnostic_id}: {outcome:#?}");
        assert_has_repair_hint(
            &outcome.diagnostics,
            diagnostic_id,
            "resolve_namespace_conflict",
        );
    }
}

#[test]
fn update_call_sites_graft_rewrites_renamed_task_calls() {
    let source = r#"
task double -> Int {
  take value: Int

  return value + 1
}

task main -> Int {
  return call double(41)
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = include_str!("../fixtures/grafts/rename_update_call_sites.json");
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("task increment -> Int"));
    assert!(grafted_source.contains("return call increment(41)"));
    let grafted = parse_program(&grafted_source).expect("parse grafted source");
    assert_eq!(run_main(&grafted), Ok(Value::Int(42)));
}

#[test]
fn update_call_sites_graft_rewrites_resolved_task_target() {
    let source = r#"
task double -> Int {
  take value: Int

  return value * 2
}

task increment -> Int {
  take value: Int

  return value + 1
}

task main -> Int {
  return call double(41)
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = r#"
{
  "op": "UpdateCallSites",
  "target": "task:main.double",
  "payload": { "replacement": "increment" }
}
"#;
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return call increment(41)"));
    let grafted = parse_program(&grafted_source).expect("parse grafted source");
    assert_eq!(run_main(&grafted), Ok(Value::Int(42)));
}

#[test]
fn update_call_args_graft_inserts_checked_call_arguments() {
    let source = r#"
task double -> Int {
  take value: Int
  take scale: Int

  return value * scale
}

task main -> Int {
  return call double(21)
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = r#"
{
  "op": "UpdateCallArgs",
  "target": "task:main.double",
  "payload": { "source": "2", "position": 1 }
}
"#;
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return call double(21, 2)"));
    let grafted = parse_program(&grafted_source).expect("parse grafted source");
    assert_eq!(run_main(&grafted), Ok(Value::Int(42)));
}

#[test]
fn update_call_args_rejects_out_of_range_positions() {
    let source = r#"
task double -> Int {
  take value: Int
  take scale: Int

  return value * scale
}

task main -> Int {
  return call double(21)
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = r#"
{
  "op": "UpdateCallArgs",
  "target": "task:main.double",
  "payload": { "source": "2", "position": 3 }
}
"#;
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "rejected");
    assert_eq!(outcome.diagnostics[0].id, "GRAFT_POSITION_OUT_OF_RANGE");
}

#[test]
fn replace_call_arg_graft_replaces_checked_call_arguments() {
    let source = r#"
task double -> Int {
  take value: Int
  take scale: Int

  return value * scale
}

task main -> Int {
  return call double(21, 1)
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = r#"
{
  "op": "ReplaceCallArg",
  "target": "task:main.double",
  "payload": { "source": "2", "position": 1 }
}
"#;
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return call double(21, 2)"));
    let grafted = parse_program(&grafted_source).expect("parse grafted source");
    assert_eq!(run_main(&grafted), Ok(Value::Int(42)));
}

#[test]
fn replace_call_arg_rejects_out_of_range_positions() {
    let source = r#"
task double -> Int {
  take value: Int

  return value * 2
}

task main -> Int {
  return call double(21)
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = r#"
{
  "op": "ReplaceCallArg",
  "target": "task:main.double",
  "payload": { "source": "2", "position": 1 }
}
"#;
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "rejected");
    assert_eq!(outcome.diagnostics[0].id, "GRAFT_POSITION_OUT_OF_RANGE");
}

#[test]
fn remove_call_arg_graft_removes_checked_call_arguments() {
    let source = r#"
task double -> Int {
  take value: Int

  return value * 2
}

task main -> Int {
  return call double(21, 2)
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = r#"
{
  "op": "RemoveCallArg",
  "target": "task:main.double",
  "payload": { "position": 1 }
}
"#;
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return call double(21)"));
    let grafted = parse_program(&grafted_source).expect("parse grafted source");
    assert_eq!(run_main(&grafted), Ok(Value::Int(42)));
}

#[test]
fn remove_call_arg_rejects_out_of_range_positions() {
    let source = r#"
task double -> Int {
  take value: Int

  return value * 2
}

task main -> Int {
  return call double(21)
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = r#"
{
  "op": "RemoveCallArg",
  "target": "task:main.double",
  "payload": { "position": 1 }
}
"#;
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "rejected");
    assert_eq!(outcome.diagnostics[0].id, "GRAFT_POSITION_OUT_OF_RANGE");
}

#[test]
fn insert_statement_graft_adds_checked_task_body_statement() {
    let source = r#"
task main -> Int {
  tally total = 1
  return total
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = include_str!("../fixtures/grafts/insert_total_increment_statement.json");
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("set total = total + 4"));
    let grafted = parse_program(&grafted_source).expect("parse grafted source");
    assert_eq!(run_main(&grafted), Ok(Value::Int(5)));
}

#[test]
fn move_node_graft_reorders_checked_task_body_statement() {
    let source = r#"
task main -> Int {
  bind left = 1
  bind right = 41
  return left + right
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = include_str!("../fixtures/grafts/move_bind_before_bind.json");
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    assert_eq!(outcome.provenance[0].operation, "MoveNode");
    let grafted_source = outcome.source.as_deref().expect("grafted source");
    assert!(
        grafted_source.find("bind right = 41").expect("right bind")
            < grafted_source.find("bind left = 1").expect("left bind"),
        "{grafted_source}"
    );
    let grafted = parse_program(grafted_source).expect("parse grafted source");
    assert_eq!(run_main(&grafted), Ok(Value::Int(42)));
}

#[test]
fn move_node_graft_reorders_top_level_declarations() {
    let source = r#"
task first -> Int {
  return 1
}

task second -> Int {
  return 2
}

task main -> Int {
  bind left = call first()
  bind right = call second()
  return left + right
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = r#"
{
  "op": "MoveNode",
  "target": "task:main.second",
  "payload": { "parent": "program.tasks", "position": 0 }
}
"#;
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.as_deref().expect("grafted source");
    assert!(
        grafted_source.find("task second").expect("second task")
            < grafted_source.find("task first").expect("first task"),
        "{grafted_source}"
    );
    let grafted = parse_program(grafted_source).expect("parse grafted source");
    assert_eq!(run_main(&grafted), Ok(Value::Int(3)));
}

#[test]
fn move_node_graft_moves_import_between_modules() {
    let source = r#"
module app.main

import app.extra
import app.shared

task main -> Int {
  return 1
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = r#"
{
  "op": "MoveNode",
  "target": "import:app.main:app.shared",
  "payload": { "parent": "module:app.extra:imports", "position": 0 }
}
"#;
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let applied = apply_graft_program(&program, graft, Some("agent:test".to_string()));

    assert_eq!(
        applied.outcome.status, "accepted",
        "{:#?}",
        applied.outcome.diagnostics
    );
    let grafted = applied.program.expect("grafted program");
    let shared_import = grafted
        .imports
        .iter()
        .find(|import| import.module == "app.shared")
        .expect("shared import");
    assert_eq!(shared_import.owner_module.as_deref(), Some("app.extra"));
    assert_eq!(shared_import.id, "import:app.extra:app.shared");
}

#[test]
fn move_node_rejects_unknown_destination_module() {
    let source = r#"
module app.main

task helper -> Int {
  return 1
}

task main -> Int {
  return 2
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = r#"
{
  "op": "MoveNode",
  "target": "task:app.main.helper",
  "payload": { "parent": "module:app.extra:tasks", "position": 0 }
}
"#;
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "rejected");
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "GRAFT_MODULE_MISSING"),
        "expected missing module diagnostic, got {:#?}",
        outcome.diagnostics
    );
}

#[test]
fn move_node_graft_moves_statement_across_blocks() {
    let source = r#"
task main -> Int {
  tally total = 1
  if true {
    set total = total + 1
  }
  return total
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = r#"
{
  "op": "MoveNode",
  "target": "block:task:main.main:stmt:1:then:stmt:0",
  "payload": {
    "parent": "block:task:main.main:stmt:1:then",
    "destination": "block:task:main.main",
    "position": 1
  }
}
"#;
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.as_deref().expect("grafted source");
    assert!(
        grafted_source
            .find("set total = total + 1")
            .expect("moved set")
            < grafted_source.find("if true").expect("if statement"),
        "{grafted_source}"
    );
    let grafted = parse_program(grafted_source).expect("parse grafted source");
    assert_eq!(run_main(&grafted), Ok(Value::Int(2)));
}

#[test]
fn move_node_graft_reorders_takes_within_task() {
    let source = r#"
task helper -> Int {
  take left: Int
  take right: Int

  return left - right
}

task main -> Int {
  return call helper(10, 3)
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = r#"
{
  "op": "MoveNode",
  "target": "take:task:main.helper:right",
  "payload": { "parent": "task:main.helper:takes", "position": 0 }
}
"#;
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.as_deref().expect("grafted source");
    assert!(
        grafted_source.find("take right: Int").expect("right take")
            < grafted_source.find("take left: Int").expect("left take"),
        "{grafted_source}"
    );
    let grafted = parse_program(grafted_source).expect("parse grafted source");
    assert_eq!(run_main(&grafted), Ok(Value::Int(-7)));
}

#[test]
fn move_node_rejects_statement_move_into_own_child_block() {
    let source = r#"
task main -> Int {
  if true {
    return 1
  }
  return 2
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = r#"
{
  "op": "MoveNode",
  "target": "block:task:main.main:stmt:0",
  "payload": {
    "destination": "block:task:main.main:stmt:0:then",
    "position": 0
  }
}
"#;
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "rejected");
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "GRAFT_MOVE_UNSUPPORTED"),
        "expected move unsupported diagnostic, got {:#?}",
        outcome.diagnostics
    );
    assert_has_repair_hint(
        &outcome.diagnostics,
        "GRAFT_MOVE_UNSUPPORTED",
        "use_supported_graft_operation",
    );
}

#[test]
fn move_node_graft_moves_take_across_tasks() {
    let source = r#"
task helper -> Int {
  take tenant: Text

  return 1
}

task main -> Int {
  return call helper()
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = r#"
{
  "op": "MoveNode",
  "target": "take:task:main.helper:tenant",
  "payload": {
    "parent": "task:main.helper:takes",
    "destination": "task:main.main:takes",
    "position": 0
  }
}
"#;
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.as_deref().expect("grafted source");
    assert!(
        !grafted_source.contains("task helper -> Int {\n  take tenant: Text"),
        "{grafted_source}"
    );
    assert!(
        grafted_source.contains("task main -> Int {\n  take tenant: Text"),
        "{grafted_source}"
    );
    let grafted = parse_program(grafted_source).expect("parse grafted source");
    assert!(
        !has_errors(&check_program(&grafted)),
        "cross-task take move should check cleanly"
    );
}

#[test]
fn move_node_rejects_unknown_take_destination() {
    let source = r#"
task helper -> Int {
  take value: Int

  return value
}

task main -> Int {
  return 1
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = r#"
{
  "op": "MoveNode",
  "target": "take:task:main.helper:value",
  "payload": {
    "parent": "task:main.helper:takes",
    "destination": "task:main.missing:takes",
    "position": 0
  }
}
"#;
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "rejected");
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "GRAFT_TARGET_MISSING"),
        "expected missing destination diagnostic, got {:#?}",
        outcome.diagnostics
    );
}

#[test]
fn move_node_rejects_destination_for_declaration_moves() {
    let source = r#"
task helper -> Int {
  return 1
}

task main -> Int {
  return call helper()
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = r#"
{
  "op": "MoveNode",
  "target": "task:main.helper",
  "payload": {
    "parent": "program.tasks",
    "destination": "block:task:main.main",
    "position": 0
  }
}
"#;
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "rejected");
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "GRAFT_MOVE_UNSUPPORTED"),
        "expected move unsupported diagnostic, got {:#?}",
        outcome.diagnostics
    );
    assert_has_repair_hint(
        &outcome.diagnostics,
        "GRAFT_MOVE_UNSUPPORTED",
        "use_supported_graft_operation",
    );
}

#[test]
fn move_node_rejects_expression_targets() {
    let source = "task main -> Int {\n  return 1 + 41\n}\n";
    let program = parse_program(source).expect("parse source");
    let graft_source = include_str!("../fixtures/grafts/move_expression_unsupported.json");
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "rejected");
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "GRAFT_MOVE_UNSUPPORTED"),
        "expected move unsupported diagnostic, got {:#?}",
        outcome.diagnostics
    );
    assert_has_repair_hint(
        &outcome.diagnostics,
        "GRAFT_MOVE_UNSUPPORTED",
        "replace_expression",
    );
    assert_has_repair_hint(
        &outcome.diagnostics,
        "GRAFT_MOVE_UNSUPPORTED",
        "use_supported_graft_operation",
    );
}

#[test]
fn move_node_rejects_out_of_range_statement_positions() {
    let source = "task main -> Int {\n  return 1\n}\n";
    let program = parse_program(source).expect("parse source");
    let graft_source = include_str!("../fixtures/grafts/move_statement_out_of_range.json");
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "rejected");
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "GRAFT_POSITION_OUT_OF_RANGE"),
        "expected position diagnostic, got {:#?}",
        outcome.diagnostics
    );
}

#[test]
fn delete_node_graft_removes_checked_task_body_statement() {
    let source = r#"
task main -> Int {
  bind debug = 1
  return 42
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = include_str!("../fixtures/grafts/delete_debug_statement.json");
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    assert_eq!(outcome.provenance[0].operation, "DeleteNode");
    let grafted_source = outcome.source.as_deref().expect("grafted source");
    assert!(!grafted_source.contains("bind debug = 1"));
    let grafted = parse_program(grafted_source).expect("parse grafted source");
    assert_eq!(run_main(&grafted), Ok(Value::Int(42)));
}

#[test]
fn delete_node_rejects_removing_required_return_statement() {
    let source = r#"
task main -> Int {
  bind debug = 1
  return 42
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = r#"
{
  "op": "DeleteNode",
  "target": "block:task:main.main:stmt:1"
}
"#;
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "rejected");
    assert!(outcome.source.is_none());
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "MISSING_RETURN"),
        "expected missing return diagnostic, got {:#?}",
        outcome.diagnostics
    );
    assert_has_repair_hint(&outcome.diagnostics, "MISSING_RETURN", "insert_return");
}

#[test]
fn delete_node_graft_removes_declarations_and_takes() {
    let source = r#"
type Scratch = {
  slot id: Int
}

effect ScratchEffect

task helper -> Int {
  return 1
}

task main -> Int {
  take unused: Int

  return 42
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = r#"
{
  "transaction": "graft_delete_decl_nodes",
  "mode": "all_or_nothing",
  "ops": [
    { "op": "DeleteNode", "target": "type:main.Scratch" },
    { "op": "DeleteNode", "target": "effect:ScratchEffect" },
    { "op": "DeleteNode", "target": "task:main.helper" },
    { "op": "DeleteNode", "target": "take:task:main.main:0:unused" }
  ]
}
"#;
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.as_deref().expect("grafted source");
    assert!(!grafted_source.contains("type Scratch"));
    assert!(!grafted_source.contains("effect ScratchEffect"));
    assert!(!grafted_source.contains("task helper"));
    assert!(!grafted_source.contains("take unused"));
    let grafted = parse_program(grafted_source).expect("parse grafted source");
    assert_eq!(run_main(&grafted), Ok(Value::Int(42)));
}

#[test]
fn delete_node_rejects_expression_targets_without_replacement() {
    let source = "task main -> Int {\n  return 1 + 41\n}\n";
    let program = parse_program(source).expect("parse source");
    let graft_source = r#"
{
  "op": "DeleteNode",
  "target": "block:task:main.main:stmt:0:expr:left"
}
"#;
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "rejected");
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "GRAFT_DELETE_UNSUPPORTED"),
        "expected delete unsupported diagnostic, got {:#?}",
        outcome.diagnostics
    );
    assert_has_repair_hint(
        &outcome.diagnostics,
        "GRAFT_DELETE_UNSUPPORTED",
        "replace_expression",
    );
    assert_has_repair_hint(
        &outcome.diagnostics,
        "GRAFT_DELETE_UNSUPPORTED",
        "use_supported_graft_operation",
    );
}

#[test]
fn replace_expression_graft_updates_nested_expression_source() {
    let source = r#"
task main -> Int {
  return 1 + 2
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = include_str!("../fixtures/grafts/replace_expression_right.json");
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return 1 + 41"));
    let grafted = parse_program(&grafted_source).expect("parse grafted source");
    assert_eq!(run_main(&grafted), Ok(Value::Int(42)));
}

#[test]
fn graft_contract_outputs_are_versioned_and_strict() {
    let source = r#"
task main -> Int {
  return 1
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = r#"
{
  "op": "InsertStatement",
  "target": "task:main.main",
  "payload": { "source": "bind extra = 41", "position": 0 }
}
"#;
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    assert_eq!(outcome.schema, GRAFT_OUTCOME_SCHEMA);

    let report = DiagnosticReport::from_diagnostics(Vec::new());
    assert_eq!(report.schema, DIAGNOSTIC_REPORT_SCHEMA);

    let slice = slice_symbol_graph(&program, "task:main.main").expect("slice main");
    assert_eq!(slice.schema, SYMBOL_GRAPH_SLICE_SCHEMA);

    let unknown_contract_field = r#"
{
  "op": "InsertStatement",
  "target": "task:main.main",
  "payload": { "source": "bind extra = 1" },
  "surprise": true
}
"#;
    assert!(serde_json::from_str::<GraftInput>(unknown_contract_field).is_err());
}

#[test]
fn json_contract_snapshots_are_locked() {
    let source = "task main -> Int {\n  return 1\n}\n";
    let program = parse_program(source).expect("parse source");
    assert_eq!(program.schema, AST_PROGRAM_SCHEMA);
    assert_json_snapshot(
        &program,
        include_str!("../fixtures/contracts/ast_minimal_program.json"),
    );

    let diagnostics_program =
        parse_program("task main -> Int {\n  return missing\n}\n").expect("parse source");
    let report = DiagnosticReport::from_diagnostics(check_program(&diagnostics_program));
    assert_json_snapshot(
        &report,
        include_str!("../fixtures/contracts/diagnostic_report_unknown_identifier.json"),
    );

    let slice = slice_symbol_graph(&program, "task:main.main").expect("slice main");
    assert_json_snapshot(
        &slice,
        include_str!("../fixtures/contracts/graph_slice_minimal_task.json"),
    );

    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/project");
    let project = load_project(&project_root).expect("load project");
    let project_slice =
        slice_symbol_graph(&project.program, "task:app.main.main").expect("slice project main");
    assert_json_snapshot(
        &project_slice,
        include_str!("../fixtures/contracts/graph_slice_project_task.json"),
    );

    let query = build_query_report(
        &project.program,
        QueryOptions {
            kind: QueryKind::Tasks,
            module: Some("app.main".to_string()),
            exported_only: false,
        },
    );
    assert_json_snapshot(
        &query,
        include_str!("../fixtures/contracts/query_project_tasks.json"),
    );

    let lint_source = r#"
module app.lint

task main -> Int {
  return call used()
}

task used -> Int {
  return 1
}

task orphan -> Int {
  return 2
}
"#;
    let lint_program = parse_program(lint_source).expect("parse lint fixture");
    let lint = build_lint_report(
        &lint_program,
        LintOptions {
            rules: vec![LintRule::UnusedPrivateTask],
            module: None,
        },
    );
    assert_json_snapshot(
        &lint,
        include_str!("../fixtures/contracts/lint_unused_private_task.json"),
    );

    let reachability_source = r#"
module app.reach

task main -> Int {
  return 0
}

export task public -> Int {
  return call public_helper()
}

task public_helper -> Int {
  return 1
}

task cycle_a -> Int {
  return call cycle_b()
}

task cycle_b -> Int {
  return call cycle_a()
}

task orphan -> Int {
  return 2
}
"#;
    let reachability_program =
        parse_program(reachability_source).expect("parse reachability lint fixture");
    let reachability_lint = build_lint_report(
        &reachability_program,
        LintOptions {
            rules: vec![LintRule::UnreachablePrivateTask],
            module: None,
        },
    );
    assert_json_snapshot(
        &reachability_lint,
        include_str!("../fixtures/contracts/lint_unreachable_private_task.json"),
    );

    let effect_lint_source = r#"
module app.effects

task main -> Text uses Network {
  return "ready"
}
"#;
    let effect_lint_program =
        parse_program(effect_lint_source).expect("parse unused effect lint fixture");
    let effect_lint = build_lint_report(
        &effect_lint_program,
        LintOptions {
            rules: vec![LintRule::UnusedDeclaredEffect],
            module: None,
        },
    );
    assert_json_snapshot(
        &effect_lint,
        include_str!("../fixtures/contracts/lint_unused_declared_effect.json"),
    );

    let raw_host_source = include_str!("../examples/file_gate.sley");
    let raw_host_program = parse_program(raw_host_source).expect("parse raw host lint fixture");
    let raw_host_lint = build_lint_report(
        &raw_host_program,
        LintOptions {
            rules: vec![LintRule::RawHostAdapter],
            module: None,
        },
    );
    assert_json_snapshot(
        &raw_host_lint,
        include_str!("../fixtures/contracts/lint_raw_host_adapter.json"),
    );

    let unused_import_root = temp_project_dir("unused-import-contract");
    write_unused_import_project(&unused_import_root);
    let unused_import_project =
        load_project(&unused_import_root).expect("load unused import project");
    let unused_import_lint = build_lint_report(
        &unused_import_project.program,
        LintOptions {
            rules: vec![LintRule::UnusedImport],
            module: None,
        },
    );
    assert_json_snapshot(
        &unused_import_lint,
        include_str!("../fixtures/contracts/lint_unused_import.json"),
    );
    let _ = fs::remove_dir_all(unused_import_root);

    let unused_take_source = r#"
module app.takes

task main -> Int {
  return call helper(21, 0)
}

task helper -> Int {
  take value: Int
  take unused: Int

  return value
}
"#;
    let unused_take_program =
        parse_program(unused_take_source).expect("parse unused take lint fixture");
    let unused_take_lint = build_lint_report(
        &unused_take_program,
        LintOptions {
            rules: vec![LintRule::UnusedTake],
            module: None,
        },
    );
    assert_json_snapshot(
        &unused_take_lint,
        include_str!("../fixtures/contracts/lint_unused_take.json"),
    );

    let unused_type_effect_source = r#"
module app.decls

type Used = {
  slot name: Text
}

type Orphan = {
  slot id: Int
}

export type Public = {
  slot value: Text
}

effect UsedEffect
effect OrphanEffect
export effect PublicEffect

task main -> Used uses UsedEffect {
  return Used { name: "Ada" }
}
"#;
    let unused_type_effect_program =
        parse_program(unused_type_effect_source).expect("parse unused declarations fixture");
    let unused_type_lint = build_lint_report(
        &unused_type_effect_program,
        LintOptions {
            rules: vec![LintRule::UnusedPrivateType],
            module: None,
        },
    );
    assert_json_snapshot(
        &unused_type_lint,
        include_str!("../fixtures/contracts/lint_unused_private_type.json"),
    );
    let unused_effect_lint = build_lint_report(
        &unused_type_effect_program,
        LintOptions {
            rules: vec![LintRule::UnusedPrivateEffect],
            module: None,
        },
    );
    assert_json_snapshot(
        &unused_effect_lint,
        include_str!("../fixtures/contracts/lint_unused_private_effect.json"),
    );

    let hello_source = include_str!("../examples/hello.sley");
    let hello_program = parse_program(hello_source).expect("parse hello fixture");
    let seal = build_trace_seal(
        "examples/hello.sley",
        hello_source.as_bytes(),
        &hello_program,
        &[],
    )
    .expect("build trace seal");
    assert_json_snapshot(
        &seal,
        include_str!("../fixtures/contracts/trace_seal_hello.json"),
    );

    let scaffold = ProjectScaffoldReport {
        schema: PROJECT_SCAFFOLD_SCHEMA.to_string(),
        status: "created".to_string(),
        project: ProjectScaffoldSummary {
            name: "agent-app".to_string(),
            root: "agent-app".to_string(),
            source_root: "src".to_string(),
            entry_module: "app.main".to_string(),
            template: "deploy".to_string(),
        },
        files: vec![
            ScaffoldFile {
                path: "sley.toml".to_string(),
                kind: "manifest".to_string(),
            },
            ScaffoldFile {
                path: "README.md".to_string(),
                kind: "guide".to_string(),
            },
            ScaffoldFile {
                path: "src/app/main.sley".to_string(),
                kind: "source".to_string(),
            },
        ],
        next_commands: vec![
            vec![
                "sley".to_string(),
                "check".to_string(),
                "--json".to_string(),
                ".".to_string(),
            ],
            vec![
                "sley".to_string(),
                "query".to_string(),
                "--json".to_string(),
                "--kind".to_string(),
                "tasks".to_string(),
                ".".to_string(),
            ],
            vec![
                "sley".to_string(),
                "plan".to_string(),
                "--json".to_string(),
                ".".to_string(),
            ],
            vec![
                "sley".to_string(),
                "lint".to_string(),
                "--json".to_string(),
                "--deny-warnings".to_string(),
                ".".to_string(),
            ],
            vec![
                "sley".to_string(),
                "verify".to_string(),
                "--json".to_string(),
                "--cap".to_string(),
                "Deploy".to_string(),
                "--deploy-result".to_string(),
                "staging".to_string(),
                "staged".to_string(),
                ".".to_string(),
            ],
            vec![
                "sley".to_string(),
                "run".to_string(),
                "--json".to_string(),
                "--cap".to_string(),
                "Deploy".to_string(),
                "--deploy-result".to_string(),
                "staging".to_string(),
                "staged".to_string(),
                ".".to_string(),
            ],
        ],
    };
    assert_json_snapshot(
        &scaffold,
        include_str!("../fixtures/contracts/project_scaffold_deploy.json"),
    );

    let doctor = build_doctor_report("examples/project", Ok(project.program.clone()), false);
    assert_json_snapshot(
        &doctor,
        include_str!("../fixtures/contracts/doctor_project_ready.json"),
    );

    let edit_plan = build_edit_plan_report("examples/project", Ok(project.program.clone()), false);
    assert_json_snapshot(
        &edit_plan,
        include_str!("../fixtures/contracts/edit_plan_project_ready.json"),
    );

    let edit_plan_with_templates = build_edit_plan_report_with_options(
        "examples/project",
        Ok(project.program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
        },
    );
    assert_json_snapshot(
        &edit_plan_with_templates,
        include_str!("../fixtures/contracts/edit_plan_project_graft_templates.json"),
    );

    let edit_plan_with_targeted_templates = build_edit_plan_report_with_options(
        "examples/project",
        Ok(project.program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("task:app.math.double".to_string()),
        },
    );
    assert_json_snapshot(
        &edit_plan_with_targeted_templates,
        include_str!("../fixtures/contracts/edit_plan_project_targeted_graft_templates.json"),
    );

    let verify = build_verify_report(
        "examples/project",
        Ok(project.program.clone()),
        RuntimeGates::new(),
        false,
    );
    assert_json_snapshot(
        &verify,
        include_str!("../fixtures/contracts/verify_project_ready.json"),
    );

    assert_schema_file(
        include_str!("../docs/schemas/sley.ast.program.v0.schema.json"),
        AST_PROGRAM_SCHEMA,
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.diagnostics.report.v0.schema.json"),
        DIAGNOSTIC_REPORT_SCHEMA,
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.graft.outcome.v0.schema.json"),
        GRAFT_OUTCOME_SCHEMA,
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.symbol_graph.v0.schema.json"),
        SYMBOL_GRAPH_SCHEMA,
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.symbol_graph.slice.v0.schema.json"),
        SYMBOL_GRAPH_SLICE_SCHEMA,
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.query.report.v0.schema.json"),
        QUERY_REPORT_SCHEMA,
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.lint.report.v0.schema.json"),
        LINT_REPORT_SCHEMA,
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.zjx.envelope.v0.schema.json"),
        "sley.zjx.envelope.v0",
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.trace.seal.v0.schema.json"),
        TRACE_SEAL_SCHEMA,
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.cli_smoke.manifest.v0.schema.json"),
        "sley.cli_smoke.manifest.v0",
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.project.scaffold.v0.schema.json"),
        PROJECT_SCAFFOLD_SCHEMA,
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.doctor.report.v0.schema.json"),
        DOCTOR_REPORT_SCHEMA,
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.edit_plan.report.v0.schema.json"),
        EDIT_PLAN_REPORT_SCHEMA,
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.verify.report.v0.schema.json"),
        VERIFY_REPORT_SCHEMA,
    );
}

#[test]
fn verify_report_blocks_on_missing_runtime_gate() {
    let source = include_str!("../examples/deploy_gate.sley");
    let program = parse_program(source).expect("parse deploy gate fixture");
    let report = build_verify_report(
        "examples/deploy_gate.sley",
        Ok(program),
        RuntimeGates::new(),
        false,
    );

    assert_eq!(report.schema, VERIFY_REPORT_SCHEMA);
    assert_eq!(report.status, "blocked");
    assert_eq!(report.summary.runtime_status, "failed");
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "RUNTIME_CAPABILITY_REQUIRED"),
        "expected missing capability diagnostic, got {report:#?}"
    );
}

#[test]
fn verify_report_blocks_when_warnings_are_denied() {
    let source = r#"
module app.effects

task main -> Text uses Network {
  return "ready"
}
"#;
    let program = parse_program(source).expect("parse unused effect fixture");
    let report = build_verify_report("app.effects", Ok(program), RuntimeGates::new(), true);

    assert_eq!(report.schema, VERIFY_REPORT_SCHEMA);
    assert_eq!(report.status, "blocked");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(report.summary.runtime_status, "skipped");
    assert_eq!(
        report
            .lint
            .as_ref()
            .expect("lint summary")
            .findings
            .first()
            .map(|finding| finding.rule.as_str()),
        Some("unused_declared_effect")
    );
}

#[test]
fn ast_schema_covers_nested_contract_variants() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../docs/schemas/sley.ast.program.v0.schema.json"
    ))
    .expect("parse AST schema");
    let defs = schema
        .get("$defs")
        .and_then(serde_json::Value::as_object)
        .expect("schema $defs");

    for key in [
        "importDecl",
        "typeDecl",
        "effectDecl",
        "taskDecl",
        "takeDecl",
        "block",
        "statement",
        "expr",
        "typeExpr",
        "sourceSpan",
        "provenanceRecord",
    ] {
        assert!(
            defs.contains_key(key),
            "missing AST schema definition {key}"
        );
    }
    assert_schema_one_of_refs(
        defs,
        "statement",
        &[
            "bindingStatement",
            "setStatement",
            "returnStatement",
            "exprStatement",
            "ifStatement",
            "whileStatement",
            "forStatement",
            "forgeStatement",
        ],
    );
    assert_schema_one_of_refs(
        defs,
        "expr",
        &[
            "rawExpr",
            "stringLiteralExpr",
            "intLiteralExpr",
            "floatLiteralExpr",
            "boolLiteralExpr",
            "identifierExpr",
            "unaryExpr",
            "binaryExpr",
            "ifExpr",
            "callExpr",
            "listLiteralExpr",
            "mapLiteralExpr",
            "indexExpr",
            "fieldAccessExpr",
            "recordLiteralExpr",
            "tryExpr",
        ],
    );
    assert_schema_one_of_refs(
        defs,
        "typeExpr",
        &["namedTypeExpr", "genericTypeExpr", "recordTypeExpr"],
    );
    assert_schema_enum(
        defs,
        "binaryOp",
        &[
            "Or",
            "And",
            "Equal",
            "NotEqual",
            "Less",
            "LessEqual",
            "Greater",
            "GreaterEqual",
            "Add",
            "Subtract",
            "Multiply",
            "Divide",
            "Remainder",
        ],
    );
    assert_schema_enum(
        defs,
        "bindingKind",
        &[
            "Take", "Bind", "State", "Cell", "Knot", "Slot", "Gate", "Lease", "Veil", "Dial",
            "Flag", "Memo", "Cache", "Derive", "Flow", "Port", "Tally", "Hole", "Draft", "Taint",
            "Witness", "Seal", "Anchor", "View", "Cursor",
        ],
    );
}

#[test]
fn checker_diagnostics_include_actionable_repair_hints() {
    let source = r#"
task takes_text -> Text {
  take value: Text

  return value
}

task main -> Int {
  bind label: Text = 1
  if 1 {
    return call takes_text(42)
  }
  return missing
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);

    assert_has_repair_hint(&diagnostics, "TYPE_MISMATCH", "change_binding_type");
    assert_has_repair_hint(&diagnostics, "TYPE_MISMATCH", "replace_initializer");
    assert_has_repair_hint(&diagnostics, "TYPE_MISMATCH", "replace_expression");
    let type_mismatch_diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "TYPE_MISMATCH")
        .expect("type mismatch diagnostic");
    let replace_expression_hint = type_mismatch_diagnostic
        .repair_hints
        .iter()
        .find(|hint| hint.kind == "replace_expression")
        .expect("binding replace expression hint");
    let target = replace_expression_hint
        .target
        .as_deref()
        .expect("binding replace expression target");
    assert!(
        target.starts_with("block:task:main.main"),
        "expected binding expression target, got {target}"
    );
    let replacement: serde_json::Value = serde_json::from_str(
        replace_expression_hint
            .replacement
            .as_deref()
            .expect("binding replace expression replacement"),
    )
    .expect("binding replace expression replacement is JSON");
    assert_eq!(
        replacement.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        replacement.pointer("/target"),
        Some(&serde_json::json!(target))
    );
    assert_eq!(
        replacement.pointer("/payload/source"),
        Some(&serde_json::json!("\"\""))
    );
    assert_has_repair_hint(&diagnostics, "IF_CONDITION_NOT_BOOL", "replace_condition");
    assert_has_repair_hint(
        &diagnostics,
        "CALL_ARGUMENT_TYPE_MISMATCH",
        "replace_argument",
    );
    assert_has_repair_hint(
        &diagnostics,
        "CALL_ARGUMENT_TYPE_MISMATCH",
        "replace_call_arg",
    );
    let call_argument_diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "CALL_ARGUMENT_TYPE_MISMATCH")
        .expect("call argument diagnostic");
    let replace_call_arg_hint = call_argument_diagnostic
        .repair_hints
        .iter()
        .find(|hint| hint.kind == "replace_call_arg")
        .expect("replace call arg hint");
    assert_eq!(
        replace_call_arg_hint.target.as_deref(),
        Some("task:main.takes_text")
    );
    let replacement: serde_json::Value = serde_json::from_str(
        replace_call_arg_hint
            .replacement
            .as_deref()
            .expect("replace call arg replacement"),
    )
    .expect("replace call arg replacement is JSON");
    assert_eq!(
        replacement.pointer("/op"),
        Some(&serde_json::json!("ReplaceCallArg"))
    );
    assert_eq!(
        replacement.pointer("/target"),
        Some(&serde_json::json!("task:main.takes_text"))
    );
    assert_eq!(
        replacement.pointer("/payload/position"),
        Some(&serde_json::json!(0))
    );
    assert_eq!(
        replacement.pointer("/payload/from"),
        Some(&serde_json::json!("takes_text"))
    );
    assert_eq!(
        replacement.pointer("/payload/scope"),
        Some(&serde_json::json!("task:main.main"))
    );
    assert_eq!(
        replacement.pointer("/payload/source"),
        Some(&serde_json::json!("\"\""))
    );
    assert_has_repair_hint(&diagnostics, "RETURN_TYPE_MISMATCH", "change_return_type");
    assert_has_repair_hint(
        &diagnostics,
        "RETURN_TYPE_MISMATCH",
        "replace_return_expression",
    );
    assert_has_repair_hint(&diagnostics, "RETURN_TYPE_MISMATCH", "replace_expression");
    let return_type_diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "RETURN_TYPE_MISMATCH")
        .expect("return type diagnostic");
    let replace_expression_hint = return_type_diagnostic
        .repair_hints
        .iter()
        .find(|hint| hint.kind == "replace_expression")
        .expect("replace expression hint");
    let target = replace_expression_hint
        .target
        .as_deref()
        .expect("replace expression target");
    assert!(
        target.starts_with("block:task:main.main"),
        "expected return expression target, got {target}"
    );
    let replacement: serde_json::Value = serde_json::from_str(
        replace_expression_hint
            .replacement
            .as_deref()
            .expect("replace expression replacement"),
    )
    .expect("replace expression replacement is JSON");
    assert_eq!(
        replacement.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        replacement.pointer("/target"),
        Some(&serde_json::json!(target))
    );
    assert_eq!(
        replacement.pointer("/payload/source"),
        Some(&serde_json::json!("0"))
    );
    assert_has_repair_hint(&diagnostics, "UNKNOWN_IDENTIFIER", "declare_binding");

    let unknown_task =
        parse_program("task main -> Int {\n  return call missing()\n}\n").expect("parse source");
    let diagnostics = check_program(&unknown_task);
    assert_has_repair_hint(&diagnostics, "UNKNOWN_TASK", "declare_or_import_task");
}

#[test]
fn checker_requires_non_unit_tasks_to_return_on_every_path() {
    let source = r#"
task missing -> Int {
  bind value = 1
}

task partial -> Int {
  if true {
    return 1
  }
  bind fallback = 2
}

task complete -> Int {
  if true {
    return 1
  } else {
    return 2
  }
}

task side_effect_only -> Unit {
  bind done = true
}
"#;
    let program = parse_program(source).expect("parse missing return source");
    let diagnostics = check_program(&program);
    let missing_return_nodes = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "MISSING_RETURN")
        .map(|diagnostic| diagnostic.node.as_deref().expect("diagnostic node"))
        .collect::<Vec<_>>();

    assert_eq!(
        missing_return_nodes,
        vec!["task:main.missing", "task:main.partial"]
    );
    assert_has_repair_hint(&diagnostics, "MISSING_RETURN", "insert_return");
    assert_has_repair_hint(&diagnostics, "MISSING_RETURN", "replace_task_body");
}

#[test]
fn checker_namespace_conflicts_include_repair_hints() {
    let source = r#"
effect Audit
effect Audit

type User = {
  slot id: Int
}

type User = {
  slot id: Text
}

task main -> Int {
  return 1
}

task main -> Int {
  return 2
}
"#;
    let program = parse_program(source).expect("parse namespace conflict source");
    let diagnostics = check_program(&program);

    assert_has_repair_hint(
        &diagnostics,
        "DUPLICATE_EFFECT",
        "resolve_namespace_conflict",
    );
    assert_has_repair_hint(&diagnostics, "DUPLICATE_TYPE", "resolve_namespace_conflict");
    assert_has_repair_hint(&diagnostics, "DUPLICATE_TASK", "resolve_namespace_conflict");
}

#[test]
fn checker_call_argument_replace_call_arg_hint_is_scoped_to_calling_task() {
    let source = r#"
task takes_text -> Text {
  take value: Text

  return value
}

task helper -> Text {
  return call takes_text("ready")
}

task main -> Text {
  return call takes_text(42)
}
"#;
    let program = parse_program(source).expect("parse call argument source");
    let diagnostics = check_program(&program);
    let hint = find_repair_hint(
        &diagnostics,
        "CALL_ARGUMENT_TYPE_MISMATCH",
        "replace_call_arg",
    );
    let graft: GraftInput = serde_json::from_str(
        hint.replacement
            .as_deref()
            .expect("replace call arg replacement"),
    )
    .expect("replace call arg hint parses as graft");

    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(
        grafted_source.contains("return call takes_text(\"ready\")"),
        "scoped hint should leave helper call alone:\n{grafted_source}"
    );
    assert!(
        grafted_source.contains("return call takes_text(\"\")"),
        "scoped hint should repair main call:\n{grafted_source}"
    );
    assert!(
        !grafted_source.contains("return call takes_text(42)"),
        "scoped hint should replace the bad main argument:\n{grafted_source}"
    );
}

#[test]
fn checker_call_arity_hints_include_structural_grafts() {
    let missing_arg = r#"
task scale -> Int {
  take value: Int
  take factor: Int

  return value * factor
}

task main -> Int {
  return call scale(21)
}
"#;
    let program = parse_program(missing_arg).expect("parse missing arg source");
    let diagnostics = check_program(&program);
    assert_has_repair_hint(&diagnostics, "CALL_ARITY_MISMATCH", "match_task_arity");
    assert_has_repair_hint(&diagnostics, "CALL_ARITY_MISMATCH", "update_call_args");
    let update_hint = find_repair_hint(&diagnostics, "CALL_ARITY_MISMATCH", "update_call_args");
    assert_eq!(update_hint.target.as_deref(), Some("task:main.scale"));
    let replacement: serde_json::Value = serde_json::from_str(
        update_hint
            .replacement
            .as_deref()
            .expect("update replacement"),
    )
    .expect("update replacement json");
    assert_eq!(
        replacement.pointer("/op"),
        Some(&serde_json::json!("UpdateCallArgs"))
    );
    assert_eq!(
        replacement.pointer("/target"),
        Some(&serde_json::json!("task:main.scale"))
    );
    assert_eq!(
        replacement.pointer("/payload/from"),
        Some(&serde_json::json!("scale"))
    );
    assert_eq!(
        replacement.pointer("/payload/position"),
        Some(&serde_json::json!(1))
    );
    assert_eq!(
        replacement.pointer("/payload/scope"),
        Some(&serde_json::json!("task:main.main"))
    );
    assert_eq!(
        replacement.pointer("/payload/source"),
        Some(&serde_json::json!("0"))
    );

    let extra_arg = r#"
task double -> Int {
  take value: Int

  return value * 2
}

task main -> Int {
  return call double(21, 1)
}
"#;
    let program = parse_program(extra_arg).expect("parse extra arg source");
    let diagnostics = check_program(&program);
    assert_has_repair_hint(&diagnostics, "CALL_ARITY_MISMATCH", "remove_call_arg");
    let remove_hint = find_repair_hint(&diagnostics, "CALL_ARITY_MISMATCH", "remove_call_arg");
    assert_eq!(remove_hint.target.as_deref(), Some("task:main.double"));
    let replacement: serde_json::Value = serde_json::from_str(
        remove_hint
            .replacement
            .as_deref()
            .expect("remove replacement"),
    )
    .expect("remove replacement json");
    assert_eq!(
        replacement.pointer("/op"),
        Some(&serde_json::json!("RemoveCallArg"))
    );
    assert_eq!(
        replacement.pointer("/target"),
        Some(&serde_json::json!("task:main.double"))
    );
    assert_eq!(
        replacement.pointer("/payload/from"),
        Some(&serde_json::json!("double"))
    );
    assert_eq!(
        replacement.pointer("/payload/position"),
        Some(&serde_json::json!(1))
    );
    assert_eq!(
        replacement.pointer("/payload/scope"),
        Some(&serde_json::json!("task:main.main"))
    );
}

#[test]
fn checker_assignment_type_hints_include_replace_expression_graft() {
    let source = r#"
task main -> Int {
  tally total: Int = 1
  set total = "oops"
  return total
}
"#;
    let program = parse_program(source).expect("parse assignment source");
    let diagnostics = check_program(&program);
    assert_has_repair_hint(
        &diagnostics,
        "SET_TYPE_MISMATCH",
        "replace_assignment_expression",
    );
    assert_has_repair_hint(&diagnostics, "SET_TYPE_MISMATCH", "replace_expression");
    let hint = find_repair_hint(&diagnostics, "SET_TYPE_MISMATCH", "replace_expression");
    let target = hint.target.as_deref().expect("set replacement target");
    assert!(
        target.starts_with("block:task:main.main"),
        "expected assignment expression target, got {target}"
    );
    let replacement: serde_json::Value =
        serde_json::from_str(hint.replacement.as_deref().expect("set replacement"))
            .expect("set replacement json");
    assert_eq!(
        replacement.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        replacement.pointer("/target"),
        Some(&serde_json::json!(target))
    );
    assert_eq!(
        replacement.pointer("/payload/source"),
        Some(&serde_json::json!("0"))
    );
}

#[test]
fn checker_condition_hints_include_replace_expression_grafts() {
    let source = r#"
task main -> Int {
  if 1 {
    bind branch = 1
  }
  while "again" {
    bind step = 1
  }
  return if 1 { 1 } else { 2 }
}
"#;
    let program = parse_program(source).expect("parse condition source");
    let diagnostics = check_program(&program);

    let if_condition_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "IF_CONDITION_NOT_BOOL")
        .collect::<Vec<_>>();
    assert_eq!(
        if_condition_diagnostics.len(),
        2,
        "expected statement and expression if condition diagnostics, got {diagnostics:#?}"
    );
    for diagnostic in if_condition_diagnostics {
        assert!(
            diagnostic
                .repair_hints
                .iter()
                .any(|hint| hint.kind == "replace_condition"),
            "expected replace_condition hint, got {diagnostic:#?}"
        );
        let hint = diagnostic
            .repair_hints
            .iter()
            .find(|hint| hint.kind == "replace_expression")
            .unwrap_or_else(|| panic!("missing replace_expression hint on {diagnostic:#?}"));
        let target = assert_replace_expression_hint_json(hint, "false");
        assert!(
            target.starts_with("block:task:main.main"),
            "expected condition expression target, got {target}"
        );
    }

    assert_has_repair_hint(
        &diagnostics,
        "WHILE_CONDITION_NOT_BOOL",
        "replace_condition",
    );
    let target =
        assert_replace_expression_hint_source(&diagnostics, "WHILE_CONDITION_NOT_BOOL", "false");
    assert!(
        target.starts_with("block:task:main.main"),
        "expected while condition target, got {target}"
    );
}

#[test]
fn checker_if_branch_hints_include_alternative_replace_expression_grafts() {
    let source = r#"
task main -> Int {
  return if true { 1 } else { "x" }
}
"#;
    let program = parse_program(source).expect("parse branch mismatch source");
    let diagnostics = check_program(&program);
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "IF_BRANCH_TYPE_MISMATCH")
        .unwrap_or_else(|| panic!("missing branch mismatch diagnostic: {diagnostics:#?}"));
    let hints = diagnostic
        .repair_hints
        .iter()
        .filter(|hint| hint.kind == "replace_expression")
        .collect::<Vec<_>>();
    assert_eq!(
        hints.len(),
        2,
        "expected one replace_expression hint per branch, got {diagnostic:#?}"
    );

    let then_target = assert_replace_expression_hint_json(hints[0], "\"\"");
    assert!(
        then_target.ends_with(":then"),
        "expected then branch replacement target, got {then_target}"
    );
    let else_target = assert_replace_expression_hint_json(hints[1], "0");
    assert!(
        else_target.ends_with(":else"),
        "expected else branch replacement target, got {else_target}"
    );
}

#[test]
fn checker_collection_index_and_record_hints_include_replace_expression_grafts() {
    let source = r#"
type User = {
  slot name: Text
  slot age: Int
}

task main -> Int {
  bind mixed = [1, "x"]
  bind scores = map { 1: 2, "two": "bad" }
  bind valid = map { "one": 1 }
  bind user = User { name: 1, age: 2 }
  bind values = [1, 2]
  bind bad_list_lookup = values["bad"]
  return valid[1]
}
"#;
    let program = parse_program(source).expect("parse collection repair source");
    let diagnostics = check_program(&program);

    assert_replace_expression_hint_source(&diagnostics, "LIST_ELEMENT_TYPE_MISMATCH", "0");
    assert_replace_expression_hint_source(&diagnostics, "MAP_KEY_TYPE_MISMATCH", "\"\"");
    assert_replace_expression_hint_source(&diagnostics, "MAP_VALUE_TYPE_MISMATCH", "0");
    assert_replace_expression_hint_source(&diagnostics, "INDEX_NOT_INT", "0");
    assert_replace_expression_hint_source(&diagnostics, "INDEX_KEY_TYPE_MISMATCH", "\"\"");
    assert_replace_expression_hint_source(&diagnostics, "RECORD_FIELD_TYPE_MISMATCH", "\"\"");
}

#[test]
fn checker_record_shape_hints_apply_as_expression_grafts() {
    let source = r#"
type User = {
  slot name: Text
  slot age: Int
}

task main -> User {
  return User { name: "Ada", extra: false }
}
"#;
    let program = parse_program(source).expect("parse record repair source");
    let diagnostics = check_program(&program);

    let missing_hint = find_repair_hint(&diagnostics, "RECORD_FIELD_MISSING", "replace_expression");
    let target =
        assert_replace_expression_hint_json(missing_hint, "User { name: \"Ada\", age: 0 }");
    assert!(
        target.ends_with(":expr"),
        "expected whole record literal target, got {target}"
    );
    let unknown_hint = find_repair_hint(&diagnostics, "RECORD_FIELD_UNKNOWN", "replace_expression");
    assert_replace_expression_hint_json(unknown_hint, "User { name: \"Ada\", age: 0 }");

    let graft: GraftInput = serde_json::from_str(
        missing_hint
            .replacement
            .as_deref()
            .expect("record repair replacement"),
    )
    .expect("record repair hint parses as graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("record graft source");
    assert!(
        grafted_source.contains("return User { name: \"Ada\", age: 0 }"),
        "record repair should replace shape:\n{grafted_source}"
    );
    let grafted = parse_program(&grafted_source).expect("parse grafted record source");
    assert!(
        !has_errors(&check_program(&grafted)),
        "record repair graft should check cleanly"
    );
}

#[test]
fn checker_iterable_and_builtin_argument_hints_apply_as_expression_grafts() {
    let for_source = r#"
task main -> Int {
  each item in 1 {
    bind skipped = 0
  }
  return 0
}
"#;
    let for_program = parse_program(for_source).expect("parse for repair source");
    let for_diagnostics = check_program(&for_program);
    let for_hint = find_repair_hint(
        &for_diagnostics,
        "FOR_COLLECTION_NOT_ITERABLE",
        "replace_expression",
    );
    assert_replace_expression_hint_json(for_hint, "[]");
    let for_graft: GraftInput = serde_json::from_str(
        for_hint
            .replacement
            .as_deref()
            .expect("for repair replacement"),
    )
    .expect("for repair hint parses as graft");
    let for_outcome = apply_graft_input(&for_program, for_graft, Some("agent:test".to_string()));
    assert_eq!(
        for_outcome.status, "accepted",
        "{:#?}",
        for_outcome.diagnostics
    );
    let for_grafted = parse_program(&for_outcome.source.expect("for graft source"))
        .expect("parse for grafted source");
    assert!(
        !has_errors(&check_program(&for_grafted)),
        "for repair graft should check cleanly"
    );

    let len_source = r#"
task main -> Int {
  return len(1)
}
"#;
    let len_program = parse_program(len_source).expect("parse len repair source");
    let len_diagnostics = check_program(&len_program);
    assert_has_repair_hint(
        &len_diagnostics,
        "BUILTIN_ARGUMENT_TYPE_MISMATCH",
        "replace_builtin_argument",
    );
    let len_hint = find_repair_hint(
        &len_diagnostics,
        "BUILTIN_ARGUMENT_TYPE_MISMATCH",
        "replace_expression",
    );
    assert_replace_expression_hint_json(len_hint, "\"\"");
    let len_graft: GraftInput = serde_json::from_str(
        len_hint
            .replacement
            .as_deref()
            .expect("len repair replacement"),
    )
    .expect("len repair hint parses as graft");
    let len_outcome = apply_graft_input(&len_program, len_graft, Some("agent:test".to_string()));
    assert_eq!(
        len_outcome.status, "accepted",
        "{:#?}",
        len_outcome.diagnostics
    );
    let len_grafted = parse_program(&len_outcome.source.expect("len graft source"))
        .expect("parse len grafted source");
    assert!(
        !has_errors(&check_program(&len_grafted)),
        "len repair graft should check cleanly"
    );
}

#[test]
fn checker_index_collection_hints_apply_as_expression_grafts() {
    let list_source = r#"
task main -> Int {
  bind item = 1[0]
  return 0
}
"#;
    let list_program = parse_program(list_source).expect("parse list index repair source");
    let list_diagnostics = check_program(&list_program);
    let list_hint = find_repair_hint(
        &list_diagnostics,
        "INDEX_COLLECTION_NOT_INDEXABLE",
        "replace_expression",
    );
    assert_replace_expression_hint_json(list_hint, "[]");
    let list_graft: GraftInput = serde_json::from_str(
        list_hint
            .replacement
            .as_deref()
            .expect("list index repair replacement"),
    )
    .expect("list index repair hint parses as graft");
    let list_outcome = apply_graft_input(&list_program, list_graft, Some("agent:test".to_string()));
    assert_eq!(
        list_outcome.status, "accepted",
        "{:#?}",
        list_outcome.diagnostics
    );
    let list_grafted = parse_program(&list_outcome.source.expect("list graft source"))
        .expect("parse list grafted source");
    assert!(
        !has_errors(&check_program(&list_grafted)),
        "list index repair graft should check cleanly"
    );

    let map_source = r#"
task main -> Int {
  bind item = 1["key"]
  return 0
}
"#;
    let map_program = parse_program(map_source).expect("parse map index repair source");
    let map_diagnostics = check_program(&map_program);
    let map_hint = find_repair_hint(
        &map_diagnostics,
        "INDEX_COLLECTION_NOT_INDEXABLE",
        "replace_expression",
    );
    assert_replace_expression_hint_json(map_hint, "map { }");
    let map_graft: GraftInput = serde_json::from_str(
        map_hint
            .replacement
            .as_deref()
            .expect("map index repair replacement"),
    )
    .expect("map index repair hint parses as graft");
    let map_outcome = apply_graft_input(&map_program, map_graft, Some("agent:test".to_string()));
    assert_eq!(
        map_outcome.status, "accepted",
        "{:#?}",
        map_outcome.diagnostics
    );
    let map_grafted = parse_program(&map_outcome.source.expect("map graft source"))
        .expect("parse map grafted source");
    assert!(
        !has_errors(&check_program(&map_grafted)),
        "map index repair graft should check cleanly"
    );
}

#[test]
fn checker_operator_hints_include_replace_expression_grafts() {
    let unary = parse_program("task main -> Bool {\n  return !1\n}\n")
        .expect("parse unary operator source");
    let unary_diagnostics = check_program(&unary);
    assert_has_repair_hint(
        &unary_diagnostics,
        "UNARY_OPERATOR_TYPE_MISMATCH",
        "replace_operand",
    );
    assert_replace_expression_hint_source(
        &unary_diagnostics,
        "UNARY_OPERATOR_TYPE_MISMATCH",
        "false",
    );

    let bool_binary = parse_program("task main -> Bool {\n  return 1 && \"x\"\n}\n")
        .expect("parse bool binary operator source");
    let bool_diagnostics = check_program(&bool_binary);
    let bool_operator = bool_diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "BINARY_OPERATOR_TYPE_MISMATCH")
        .unwrap_or_else(|| panic!("missing bool operator diagnostic: {bool_diagnostics:#?}"));
    let bool_sources = replace_expression_hint_sources(bool_operator);
    assert_eq!(bool_sources, vec!["false".to_string(), "false".to_string()]);

    let int_add =
        parse_program("task main -> Int {\n  return 1 + false\n}\n").expect("parse int add source");
    let int_add_diagnostics = check_program(&int_add);
    assert_replace_expression_hint_source(
        &int_add_diagnostics,
        "BINARY_OPERATOR_TYPE_MISMATCH",
        "0",
    );

    let text_add = parse_program("task main -> Text {\n  return \"x\" + false\n}\n")
        .expect("parse text add source");
    let text_add_diagnostics = check_program(&text_add);
    assert_replace_expression_hint_source(
        &text_add_diagnostics,
        "BINARY_OPERATOR_TYPE_MISMATCH",
        "\"\"",
    );

    let float_compare = parse_program("task main -> Bool {\n  return 1.5 < \"x\"\n}\n")
        .expect("parse float comparison source");
    let float_compare_diagnostics = check_program(&float_compare);
    assert_replace_expression_hint_source(
        &float_compare_diagnostics,
        "BINARY_OPERATOR_TYPE_MISMATCH",
        "0.0",
    );
}

#[test]
fn rejected_graft_fixtures_report_stable_diagnostic_ids() {
    let call_source = r#"
task double -> Int {
  take value: Int

  return value * 2
}

task main -> Int {
  return call double(21)
}
"#;
    assert_rejected_graft(
        call_source,
        include_str!("../fixtures/grafts/invalid_update_call_sites_callee.json"),
        "GRAFT_INVALID_CALLEE",
    );
    assert_rejected_graft(
        call_source,
        include_str!("../fixtures/grafts/replace_call_arg_out_of_range.json"),
        "GRAFT_POSITION_OUT_OF_RANGE",
    );
    assert_rejected_graft(
        call_source,
        include_str!("../fixtures/grafts/remove_call_arg_out_of_range.json"),
        "GRAFT_POSITION_OUT_OF_RANGE",
    );

    let no_call_source = r#"
task double -> Int {
  take value: Int

  return value * 2
}

task main -> Int {
  return 1
}
"#;
    assert_rejected_graft(
        no_call_source,
        include_str!("../fixtures/grafts/missing_update_call_sites.json"),
        "GRAFT_CALLSITE_MISSING",
    );

    let main_source = r#"
task main -> Int {
  return 1
}
"#;
    assert_rejected_graft(
        main_source,
        include_str!("../fixtures/grafts/insert_multiple_statements.json"),
        "GRAFT_EXPECTED_ONE_STATEMENT",
    );
    assert_rejected_graft(
        main_source,
        include_str!("../fixtures/grafts/insert_out_of_range_statement.json"),
        "GRAFT_POSITION_OUT_OF_RANGE",
    );
    assert_rejected_graft(
        main_source,
        include_str!("../fixtures/grafts/replace_missing_expression.json"),
        "GRAFT_TARGET_MISSING",
    );

    let outcome = assert_rejected_graft(
        main_source,
        include_str!("../fixtures/grafts/insert_unknown_identifier_statement.json"),
        "UNKNOWN_IDENTIFIER",
    );
    assert!(outcome.source.is_none());
    assert!(outcome.provenance.is_empty());
}

#[test]
fn parser_builds_structured_call_and_record_expressions() {
    let source = include_str!("../examples/profile_service.sley");
    let program = parse_program(source).expect("parse fixture");
    let task = &program.tasks[0];

    let StatementKind::Binding {
        expr: query_expr, ..
    } = &task.body.statements[0].kind
    else {
        panic!("expected binding statement");
    };
    assert!(matches!(query_expr.kind, ExprKind::Try { .. }));

    let StatementKind::Return { expr: return_expr } = &task.body.statements[1].kind else {
        panic!("expected return statement");
    };
    let ExprKind::Call { args, .. } = &return_expr.kind else {
        panic!("expected Ok(...) call");
    };
    assert!(matches!(args[0].kind, ExprKind::RecordLiteral { .. }));
}

#[test]
fn parser_builds_structured_operator_and_if_expressions() {
    let source = r#"
task grade -> Text {
  take score: Int

  return if score >= 90 { "A" } else { "B" }
}
"#;
    let program = parse_program(source).expect("parse source");
    let StatementKind::Return { expr } = &program.tasks[0].body.statements[0].kind else {
        panic!("expected return statement");
    };
    let ExprKind::If { condition, .. } = &expr.kind else {
        panic!("expected if expression");
    };
    assert!(matches!(condition.kind, ExprKind::Binary { .. }));
}

#[test]
fn parser_builds_statement_control_flow_and_list_index_expressions() {
    let source = r#"
task main -> Int {
  bind values = [1, 2, 3]
  state index = 0
  if len(values) > 2 {
    set index = 1
  } else {
    set index = 0
  }
  return values[index]
}
"#;
    let program = parse_program(source).expect("parse source");
    assert!(matches!(
        program.tasks[0].body.statements[0].kind,
        StatementKind::Binding { .. }
    ));
    assert!(matches!(
        program.tasks[0].body.statements[2].kind,
        StatementKind::If { .. }
    ));
    let StatementKind::Return { expr } = &program.tasks[0].body.statements[3].kind else {
        panic!("expected return statement");
    };
    assert!(matches!(expr.kind, ExprKind::Index { .. }));
}

#[test]
fn parser_builds_for_loops_and_map_literals() {
    let source = r#"
task main -> Int {
  bind scores: Map<Text, Int> = map { "ada": 3, "grace": 5 }
  tally total = 0
  each name in ["ada", "grace"] {
    set total = total + scores[name]
  }
  return total
}
"#;
    let program = parse_program(source).expect("parse source");
    let StatementKind::Binding { expr, .. } = &program.tasks[0].body.statements[0].kind else {
        panic!("expected map binding statement");
    };
    assert!(matches!(expr.kind, ExprKind::MapLiteral { .. }));
    assert!(matches!(
        program.tasks[0].body.statements[2].kind,
        StatementKind::For { .. }
    ));
}

#[test]
fn checker_validates_user_task_calls() {
    let source = r#"
task takes_text -> Text {
  take value: Text

  return value
}

task main -> Text {
  return call takes_text(42)
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "CALL_ARGUMENT_TYPE_MISMATCH"),
        "expected call argument diagnostic, got {diagnostics:#?}"
    );
}

#[test]
fn checker_validates_operator_and_if_semantics() {
    let source = r#"
task main -> Int {
  bind invalid = 1 + "x"
  return if invalid { 1 } else { missing }
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "BINARY_OPERATOR_TYPE_MISMATCH"),
        "expected operator diagnostic, got {diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "IF_CONDITION_NOT_BOOL"),
        "expected if-condition diagnostic, got {diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "UNKNOWN_IDENTIFIER"),
        "expected unknown identifier diagnostic, got {diagnostics:#?}"
    );
}

#[test]
fn checker_validates_collections_indexes_and_statement_control_flow() {
    let source = r#"
task main -> Int {
  bind mixed = [1, "x"]
  bind values = [1, 2]
  while values {
    set missing = 1
  }
  return values["bad"]
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "LIST_ELEMENT_TYPE_MISMATCH"),
        "expected list element diagnostic, got {diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "WHILE_CONDITION_NOT_BOOL"),
        "expected while-condition diagnostic, got {diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "UNKNOWN_IDENTIFIER"),
        "expected unknown set diagnostic, got {diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "INDEX_NOT_INT"),
        "expected index diagnostic, got {diagnostics:#?}"
    );
}

#[test]
fn checker_validates_maps_and_for_loops() {
    let source = r#"
task main -> Int {
  bind scores = map { 1: 2, "two": "bad" }
  each score in scores {
    set missing = score
  }
  bind valid = map { "one": 1 }
  return valid[1]
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "MAP_KEY_TYPE_MISMATCH"),
        "expected map key diagnostic, got {diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "MAP_VALUE_TYPE_MISMATCH"),
        "expected map value diagnostic, got {diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "FOR_COLLECTION_NOT_ITERABLE"),
        "expected for-loop collection diagnostic, got {diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "INDEX_KEY_TYPE_MISMATCH"),
        "expected map index diagnostic, got {diagnostics:#?}"
    );
}

#[test]
fn checker_propagates_called_task_effects() {
    let source = r#"
effect FileRead

task read -> Result<Text, Error> uses FileRead {
  take path: Text

  return fs.read_text(path)?
}

task main -> Result<Text, Error> {
  return call read("x")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "EFFECT_UNAUTHORIZED"),
        "expected effect diagnostic, got {diagnostics:#?}"
    );
}

#[test]
fn checker_requires_gate_takes_to_match_declared_effects() {
    let source = r#"
task main -> Int {
  take gate files: Gate<FileRead>

  return 1
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "GATE_EFFECT_UNDECLARED"),
        "expected gate effect diagnostic, got {diagnostics:#?}"
    );
}

#[test]
fn runtime_requires_and_accepts_capability_gates() {
    let source = r#"
task main -> Int uses FileRead {
  return 42
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let rejected = run_main(&program).expect_err("missing gate should reject");
    assert!(
        rejected
            .iter()
            .any(|diagnostic| diagnostic.id == "RUNTIME_CAPABILITY_REQUIRED"),
        "expected runtime capability diagnostic, got {rejected:#?}"
    );

    let gates = RuntimeGates::allow("FileRead");
    assert_eq!(run_main_with_gates(&program, &gates), Ok(Value::Int(42)));
}

#[test]
fn runtime_injects_gate_takes_and_reads_text_under_root() {
    let root = temp_project_dir("runtime-gate-read");
    fs::create_dir_all(&root).expect("create temp dir");
    let input = root.join("input.txt");
    fs::write(&input, "sley gate read").expect("write input");
    let source = format!(
        r#"
task read -> Text uses FileRead {{
  take gate file: Gate<FileRead>
  take path: Text

  return fs.read_text(path)
}}

task main -> Text uses FileRead {{
  return call read("{}")
}}
"#,
        sley_string(&input)
    );
    let program = parse_program(&source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect_root("FileRead", &root);
    assert_eq!(
        run_main_with_gates(&program, &gates),
        Ok(Value::Text("sley gate read".to_string()))
    );
}

#[test]
fn runtime_writes_text_under_file_write_gate() {
    let root = temp_project_dir("runtime-gate-write");
    fs::create_dir_all(&root).expect("create temp dir");
    let output = root.join("output.txt");
    let source = format!(
        r#"
task main -> Unit uses FileWrite {{
  return fs.write_text("{}", "sley gate write")
}}
"#,
        sley_string(&output)
    );
    let program = parse_program(&source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect_root("FileWrite", &root);
    assert_eq!(run_main_with_gates(&program, &gates), Ok(Value::Unit));
    assert_eq!(
        fs::read_to_string(&output).expect("read output"),
        "sley gate write"
    );
}

#[test]
fn runtime_rejects_file_access_outside_gate_root() {
    let root = temp_project_dir("runtime-gate-root");
    let outside = temp_project_dir("runtime-gate-outside");
    fs::create_dir_all(&root).expect("create root dir");
    fs::create_dir_all(&outside).expect("create outside dir");
    let input = outside.join("secret.txt");
    fs::write(&input, "outside").expect("write outside input");
    let source = format!(
        r#"
task main -> Text uses FileRead {{
  return fs.read_text("{}")
}}
"#,
        sley_string(&input)
    );
    let program = parse_program(&source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect_root("FileRead", &root);
    let rejected = run_main_with_gates(&program, &gates).expect_err("outside root should reject");
    assert!(
        rejected
            .iter()
            .any(|diagnostic| diagnostic.id == "RUNTIME_CAPABILITY_SCOPE_DENIED"),
        "expected root-scope diagnostic, got {rejected:#?}"
    );
}

#[test]
fn cli_run_accepts_runtime_capability_root() {
    let root = temp_project_dir("runtime-gate-cli");
    fs::create_dir_all(&root).expect("create temp dir");
    let input = root.join("input.txt");
    let source_path = root.join("main.sley");
    fs::write(&input, "cli gate read").expect("write input");
    fs::write(
        &source_path,
        format!(
            r#"
task main -> Text uses FileRead {{
  return fs.read_text("{}")
}}
"#,
            sley_string(&input)
        ),
    )
    .expect("write source");

    let cap = format!("FileRead={}", root.display());
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "run",
            "--json",
            "--cap",
            &cap,
            source_path.to_str().expect("utf-8 path"),
        ])
        .output()
        .expect("run sley");
    assert!(
        output.status.success(),
        "sley run failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).expect("parse value");
    assert_eq!(value, Value::Text("cli gate read".to_string()));
}

#[test]
fn checker_infers_database_row_accessors() {
    let source = r#"
task get_name -> Text uses DatabaseRead {
  take id: Text

  bind row = call db.query_one("select * from users where id = ?", id)
  return row.text("name")
}

task bad_age -> Text uses DatabaseRead {
  bind row = call db.query_one("select * from users")
  return row.int("age")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "RETURN_TYPE_MISMATCH"),
        "expected return type diagnostic from inferred row.int, got {diagnostics:#?}"
    );
}

#[test]
fn runtime_queries_seeded_database_with_gate() {
    let source = r#"
task get_name -> Text uses DatabaseRead {
  take gate db: Gate<DatabaseRead>
  take id: Text

  bind row = call db.query_one("select * from users where id = ?", id)
  return row.text("name")
}

task main -> Text uses DatabaseRead {
  return call get_name("u1")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect("DatabaseRead");
    gates.grant_db_rows(
        "users",
        vec![
            db_row([
                ("id", Value::Text("u1".to_string())),
                ("name", Value::Text("Ada".to_string())),
            ]),
            db_row([
                ("id", Value::Text("u2".to_string())),
                ("name", Value::Text("Grace".to_string())),
            ]),
        ],
    );

    assert_eq!(
        run_main_with_gates(&program, &gates),
        Ok(Value::Text("Ada".to_string()))
    );
}

#[test]
fn runtime_query_lists_seeded_database_rows() {
    let source = r#"
task main -> Int uses DatabaseRead {
  bind rows = call db.query("select * from users")
  return len(rows)
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect("DatabaseRead");
    gates.grant_db_rows(
        "users",
        vec![
            db_row([("id", Value::Text("u1".to_string()))]),
            db_row([("id", Value::Text("u2".to_string()))]),
        ],
    );

    assert_eq!(run_main_with_gates(&program, &gates), Ok(Value::Int(2)));
}

#[test]
fn runtime_database_adapter_requires_gate_and_seeded_rows() {
    let source = r#"
task main -> Text uses DatabaseRead {
  bind row = call db.query_one("select * from users where id = ?", "u1")
  return row.text("name")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_db_rows(
        "users",
        vec![db_row([
            ("id", Value::Text("u1".to_string())),
            ("name", Value::Text("Ada".to_string())),
        ])],
    );
    let rejected = run_main_with_gates(&program, &gates).expect_err("missing gate should reject");
    assert!(
        rejected
            .iter()
            .any(|diagnostic| diagnostic.id == "RUNTIME_CAPABILITY_REQUIRED"),
        "expected capability diagnostic, got {rejected:#?}"
    );

    let mut missing_table = RuntimeGates::new();
    missing_table.grant_effect("DatabaseRead");
    let rejected =
        run_main_with_gates(&program, &missing_table).expect_err("missing table should reject");
    assert!(
        rejected
            .iter()
            .any(|diagnostic| diagnostic.id == "RUNTIME_DB_TABLE_NOT_FOUND"),
        "expected table diagnostic, got {rejected:#?}"
    );
}

#[test]
fn runtime_returns_result_constructor_values() {
    let ok_source = r#"
task main -> Result<Int, Error> {
  return Ok(42)
}
"#;
    let ok_program = parse_program(ok_source).expect("parse ok source");
    let ok_diagnostics = check_program(&ok_program);
    assert!(
        !has_errors(&ok_diagnostics),
        "unexpected diagnostics: {ok_diagnostics:#?}"
    );
    assert_eq!(
        run_main(&ok_program),
        Ok(Value::Ok(Box::new(Value::Int(42))))
    );

    let err_source = r#"
task main -> Result<Int, Error> {
  return Err("bad")
}
"#;
    let err_program = parse_program(err_source).expect("parse err source");
    let err_diagnostics = check_program(&err_program);
    assert!(
        !has_errors(&err_diagnostics),
        "unexpected diagnostics: {err_diagnostics:#?}"
    );
    assert_eq!(
        run_main(&err_program),
        Ok(Value::Err(Box::new(Value::Text("bad".to_string()))))
    );
}

#[test]
fn runtime_unwraps_ok_with_try_operator() {
    let source = r#"
task main -> Result<Int, Error> {
  bind value = Ok(41)?
  return Ok(value + 1)
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    assert_eq!(run_main(&program), Ok(Value::Ok(Box::new(Value::Int(42)))));
}

#[test]
fn runtime_propagates_err_with_try_operator() {
    let source = r#"
task fail -> Result<Int, Error> {
  return Err("bad")
}

task main -> Result<Int, Error> {
  bind value = call fail()?
  return Ok(value + 1)
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    assert_eq!(
        run_main(&program),
        Ok(Value::Err(Box::new(Value::Text("bad".to_string()))))
    );
}

#[test]
fn runtime_try_read_text_unwraps_host_ok() {
    let root = temp_project_dir("runtime-host-try-read");
    fs::create_dir_all(&root).expect("create temp dir");
    let input = root.join("input.txt");
    fs::write(&input, "sley host ok").expect("write input");
    let source = format!(
        r#"
task main -> Result<Text, Error> uses FileRead {{
  bind text = fs.try_read_text("{}")?
  return Ok(text + "!")
}}
"#,
        sley_string(&input)
    );
    let program = parse_program(&source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect_root("FileRead", &root);
    assert_eq!(
        run_main_with_gates(&program, &gates),
        Ok(Value::Ok(Box::new(Value::Text(
            "sley host ok!".to_string()
        ))))
    );
}

#[test]
fn runtime_try_read_text_propagates_host_error_value() {
    let root = temp_project_dir("runtime-host-try-read-missing");
    fs::create_dir_all(&root).expect("create temp dir");
    let missing = root.join("missing.txt");
    let source = format!(
        r#"
task main -> Result<Text, Error> uses FileRead {{
  bind text = fs.try_read_text("{}")?
  return Ok(text)
}}
"#,
        sley_string(&missing)
    );
    let program = parse_program(&source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect_root("FileRead", &root);
    let result = run_main_with_gates(&program, &gates).expect("run main");
    let Value::Err(error) = result else {
        panic!("expected host error result, got {result:#?}");
    };
    assert_error_record(&error, "RUNTIME_HOST_IO_ERROR", "fs.try_read_text");
}

#[test]
fn runtime_try_write_text_unwraps_host_ok() {
    let root = temp_project_dir("runtime-host-try-write");
    fs::create_dir_all(&root).expect("create temp dir");
    let output = root.join("output.txt");
    let source = format!(
        r#"
task main -> Result<Int, Error> uses FileWrite {{
  fs.try_write_text("{}", "sley host write")?
  return Ok(1)
}}
"#,
        sley_string(&output)
    );
    let program = parse_program(&source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect_root("FileWrite", &root);
    assert_eq!(
        run_main_with_gates(&program, &gates),
        Ok(Value::Ok(Box::new(Value::Int(1))))
    );
    assert_eq!(
        fs::read_to_string(&output).expect("read output"),
        "sley host write"
    );
}

#[test]
fn runtime_try_database_query_one_unwraps_host_ok() {
    let source = r#"
task main -> Result<Text, Error> uses DatabaseRead {
  bind row = call db.try_query_one("select * from users where id = ?", "u1")?
  return Ok(row.text("name"))
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect("DatabaseRead");
    gates.grant_db_rows(
        "users",
        vec![db_row([
            ("id", Value::Text("u1".to_string())),
            ("name", Value::Text("Ada".to_string())),
        ])],
    );
    assert_eq!(
        run_main_with_gates(&program, &gates),
        Ok(Value::Ok(Box::new(Value::Text("Ada".to_string()))))
    );
}

#[test]
fn runtime_try_database_query_one_propagates_host_error_value() {
    let source = r#"
task main -> Result<Text, Error> uses DatabaseRead {
  bind row = call db.try_query_one("select * from users where id = ?", "missing")?
  return Ok(row.text("name"))
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect("DatabaseRead");
    gates.grant_db_rows(
        "users",
        vec![db_row([
            ("id", Value::Text("u1".to_string())),
            ("name", Value::Text("Ada".to_string())),
        ])],
    );
    let result = run_main_with_gates(&program, &gates).expect("run main");
    let Value::Err(error) = result else {
        panic!("expected host error result, got {result:#?}");
    };
    assert_error_record(&error, "RUNTIME_DB_ROW_NOT_FOUND", "returned no rows");
}

#[test]
fn checker_requires_database_write_for_insert_adapter() {
    let source = r#"
task main -> Result<DbRow, Error> uses DatabaseRead {
  return call db.try_insert("users", { id: "u3", name: "Lin" })
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    let effect_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "EFFECT_UNAUTHORIZED")
        .collect::<Vec<_>>();
    assert_eq!(
        effect_diagnostics.len(),
        1,
        "expected one effect diagnostic, got {diagnostics:#?}"
    );
    assert!(
        effect_diagnostics[0].message.contains("DatabaseWrite"),
        "expected DatabaseWrite diagnostic, got {diagnostics:#?}"
    );
}

#[test]
fn runtime_try_database_insert_persists_for_current_run() {
    let source = r#"
task main -> Result<Text, Error> uses DatabaseWrite, DatabaseRead {
  bind inserted = call db.try_insert("users", { id: "u3", name: "Lin" })?
  bind row = call db.query_one("select * from users where id = ?", inserted.text("id"))
  return Ok(row.text("name"))
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect("DatabaseWrite");
    gates.grant_effect("DatabaseRead");
    assert_eq!(
        run_main_with_gates(&program, &gates),
        Ok(Value::Ok(Box::new(Value::Text("Lin".to_string()))))
    );
}

#[test]
fn runtime_try_database_insert_returns_error_for_empty_table() {
    let source = r#"
task main -> Result<DbRow, Error> uses DatabaseWrite {
  return call db.try_insert("", { id: "u1" })
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect("DatabaseWrite");
    let result = run_main_with_gates(&program, &gates).expect("run main");
    let Value::Err(error) = result else {
        panic!("expected host error result, got {result:#?}");
    };
    assert_error_record(&error, "RUNTIME_DB_TABLE_INVALID", "table name");
}

#[test]
fn checker_requires_network_for_http_text_adapter() {
    let source = r#"
task main -> Result<Text, Error> {
  return call http.try_get_text("https://example.test/profile")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    let effect_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "EFFECT_UNAUTHORIZED")
        .collect::<Vec<_>>();
    assert_eq!(
        effect_diagnostics.len(),
        1,
        "expected one effect diagnostic, got {diagnostics:#?}"
    );
    assert!(
        effect_diagnostics[0].message.contains("Network"),
        "expected Network diagnostic, got {diagnostics:#?}"
    );
}

#[test]
fn runtime_try_http_get_text_reads_seeded_response() {
    let source = r#"
task main -> Result<Text, Error> uses Network {
  bind body = call http.try_get_text("https://example.test/profile")?
  return Ok(body)
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect("Network");
    gates.grant_http_text("https://example.test/profile", "Ada");
    assert_eq!(
        run_main_with_gates(&program, &gates),
        Ok(Value::Ok(Box::new(Value::Text("Ada".to_string()))))
    );
}

#[test]
fn runtime_try_http_get_text_returns_error_for_missing_seed() {
    let source = r#"
task main -> Result<Text, Error> uses Network {
  return call http.try_get_text("https://example.test/missing")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect("Network");
    let result = run_main_with_gates(&program, &gates).expect("run main");
    let Value::Err(error) = result else {
        panic!("expected host error result, got {result:#?}");
    };
    assert_error_record(&error, "RUNTIME_HTTP_RESPONSE_NOT_FOUND", "was not seeded");
}

#[test]
fn runtime_try_http_get_text_requires_network_capability() {
    let source = r#"
task main -> Result<Text, Error> uses Network {
  return call http.try_get_text("https://example.test/profile")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_http_text("https://example.test/profile", "Ada");
    let diagnostics = run_main_with_gates(&program, &gates).expect_err("missing Network gate");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "RUNTIME_CAPABILITY_REQUIRED"
                && diagnostic.message.contains("Network")),
        "expected missing Network capability diagnostic, got {diagnostics:#?}"
    );
}

#[test]
fn cli_run_accepts_http_text_seed() {
    let root = temp_project_dir("runtime-http-cli");
    fs::create_dir_all(&root).expect("create temp dir");
    let source_path = root.join("main.sley");
    fs::write(
        &source_path,
        r#"
task main -> Result<Text, Error> uses Network {
  return call http.try_get_text("https://example.test/profile")
}
"#,
    )
    .expect("write source");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "run",
            "--json",
            "--cap",
            "Network",
            "--http-text",
            "https://example.test/profile",
            "Ada",
            source_path.to_str().expect("utf-8 path"),
        ])
        .output()
        .expect("run sley");
    assert!(
        output.status.success(),
        "sley run failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).expect("parse value");
    assert_eq!(value, Value::Ok(Box::new(Value::Text("Ada".to_string()))));
}

#[test]
fn checker_requires_shell_for_shell_run_adapter() {
    let source = r#"
task main -> Result<Text, Error> {
  return call shell.try_run("date")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    let effect_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "EFFECT_UNAUTHORIZED")
        .collect::<Vec<_>>();
    assert_eq!(
        effect_diagnostics.len(),
        1,
        "expected one effect diagnostic, got {diagnostics:#?}"
    );
    assert!(
        effect_diagnostics[0].message.contains("Shell"),
        "expected Shell diagnostic, got {diagnostics:#?}"
    );
}

#[test]
fn runtime_try_shell_run_reads_seeded_output() {
    let source = r#"
task main -> Result<Text, Error> uses Shell {
  bind output = call shell.try_run("date")?
  return Ok(output)
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect("Shell");
    gates.grant_shell_output("date", "2026-05-05");
    assert_eq!(
        run_main_with_gates(&program, &gates),
        Ok(Value::Ok(Box::new(Value::Text("2026-05-05".to_string()))))
    );
}

#[test]
fn runtime_try_shell_run_returns_error_for_missing_seed() {
    let source = r#"
task main -> Result<Text, Error> uses Shell {
  return call shell.try_run("whoami")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect("Shell");
    let result = run_main_with_gates(&program, &gates).expect("run main");
    let Value::Err(error) = result else {
        panic!("expected host error result, got {result:#?}");
    };
    assert_error_record(&error, "RUNTIME_SHELL_OUTPUT_NOT_FOUND", "was not seeded");
}

#[test]
fn runtime_try_shell_run_requires_shell_capability() {
    let source = r#"
task main -> Result<Text, Error> uses Shell {
  return call shell.try_run("date")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_shell_output("date", "2026-05-05");
    let diagnostics = run_main_with_gates(&program, &gates).expect_err("missing Shell gate");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "RUNTIME_CAPABILITY_REQUIRED"
                && diagnostic.message.contains("Shell")),
        "expected missing Shell capability diagnostic, got {diagnostics:#?}"
    );
}

#[test]
fn cli_run_accepts_shell_output_seed() {
    let root = temp_project_dir("runtime-shell-cli");
    fs::create_dir_all(&root).expect("create temp dir");
    let source_path = root.join("main.sley");
    fs::write(
        &source_path,
        r#"
task main -> Result<Text, Error> uses Shell {
  return call shell.try_run("date")
}
"#,
    )
    .expect("write source");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "run",
            "--json",
            "--cap",
            "Shell",
            "--shell-output",
            "date",
            "2026-05-05",
            source_path.to_str().expect("utf-8 path"),
        ])
        .output()
        .expect("run sley");
    assert!(
        output.status.success(),
        "sley run failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).expect("parse value");
    assert_eq!(
        value,
        Value::Ok(Box::new(Value::Text("2026-05-05".to_string())))
    );
}

#[test]
fn checker_requires_model_call_for_model_adapter() {
    let source = r#"
task main -> Result<Text, Error> {
  return call model.try_complete("name")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    let effect_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "EFFECT_UNAUTHORIZED")
        .collect::<Vec<_>>();
    assert_eq!(
        effect_diagnostics.len(),
        1,
        "expected one effect diagnostic, got {diagnostics:#?}"
    );
    assert!(
        effect_diagnostics[0].message.contains("ModelCall"),
        "expected ModelCall diagnostic, got {diagnostics:#?}"
    );
}

#[test]
fn runtime_try_model_complete_reads_seeded_output() {
    let source = r#"
task main -> Result<Text, Error> uses ModelCall {
  bind answer = call model.try_complete("name")?
  return Ok(answer)
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect("ModelCall");
    gates.grant_model_output("name", "Ada");
    assert_eq!(
        run_main_with_gates(&program, &gates),
        Ok(Value::Ok(Box::new(Value::Text("Ada".to_string()))))
    );
}

#[test]
fn runtime_try_model_complete_returns_error_for_missing_seed() {
    let source = r#"
task main -> Result<Text, Error> uses ModelCall {
  return call model.try_complete("missing")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect("ModelCall");
    let result = run_main_with_gates(&program, &gates).expect("run main");
    let Value::Err(error) = result else {
        panic!("expected host error result, got {result:#?}");
    };
    assert_error_record(&error, "RUNTIME_MODEL_OUTPUT_NOT_FOUND", "was not seeded");
}

#[test]
fn runtime_try_model_complete_requires_model_call_capability() {
    let source = r#"
task main -> Result<Text, Error> uses ModelCall {
  return call model.try_complete("name")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_model_output("name", "Ada");
    let diagnostics = run_main_with_gates(&program, &gates).expect_err("missing ModelCall gate");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "RUNTIME_CAPABILITY_REQUIRED"
                && diagnostic.message.contains("ModelCall")),
        "expected missing ModelCall capability diagnostic, got {diagnostics:#?}"
    );
}

#[test]
fn cli_run_accepts_model_output_seed() {
    let root = temp_project_dir("runtime-model-cli");
    fs::create_dir_all(&root).expect("create temp dir");
    let source_path = root.join("main.sley");
    fs::write(
        &source_path,
        r#"
task main -> Result<Text, Error> uses ModelCall {
  return call model.try_complete("name")
}
"#,
    )
    .expect("write source");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "run",
            "--json",
            "--cap",
            "ModelCall",
            "--model-output",
            "name",
            "Ada",
            source_path.to_str().expect("utf-8 path"),
        ])
        .output()
        .expect("run sley");
    assert!(
        output.status.success(),
        "sley run failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).expect("parse value");
    assert_eq!(value, Value::Ok(Box::new(Value::Text("Ada".to_string()))));
}

#[test]
fn checker_requires_secret_read_for_secret_adapter() {
    let source = r#"
task main -> Result<Text, Error> {
  return call secrets.try_get("api_key")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    let effect_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "EFFECT_UNAUTHORIZED")
        .collect::<Vec<_>>();
    assert_eq!(
        effect_diagnostics.len(),
        1,
        "expected one effect diagnostic, got {diagnostics:#?}"
    );
    assert!(
        effect_diagnostics[0].message.contains("SecretRead"),
        "expected SecretRead diagnostic, got {diagnostics:#?}"
    );
}

#[test]
fn runtime_try_secret_get_reads_seeded_secret() {
    let source = r#"
task main -> Result<Text, Error> uses SecretRead {
  bind value = call secrets.try_get("api_key")?
  return Ok(value)
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect("SecretRead");
    gates.grant_secret("api_key", "redacted");
    assert_eq!(
        run_main_with_gates(&program, &gates),
        Ok(Value::Ok(Box::new(Value::Text("redacted".to_string()))))
    );
}

#[test]
fn runtime_try_secret_get_returns_error_for_missing_seed() {
    let source = r#"
task main -> Result<Text, Error> uses SecretRead {
  return call secrets.try_get("missing")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect("SecretRead");
    let result = run_main_with_gates(&program, &gates).expect("run main");
    let Value::Err(error) = result else {
        panic!("expected host error result, got {result:#?}");
    };
    assert_error_record(&error, "RUNTIME_SECRET_NOT_FOUND", "was not seeded");
}

#[test]
fn runtime_try_secret_get_returns_error_for_empty_name() {
    let source = r#"
task main -> Result<Text, Error> uses SecretRead {
  return call secrets.try_get("")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect("SecretRead");
    let result = run_main_with_gates(&program, &gates).expect("run main");
    let Value::Err(error) = result else {
        panic!("expected host error result, got {result:#?}");
    };
    assert_error_record(&error, "RUNTIME_SECRET_NAME_INVALID", "secret name");
}

#[test]
fn runtime_try_secret_get_requires_secret_read_capability() {
    let source = r#"
task main -> Result<Text, Error> uses SecretRead {
  return call secrets.try_get("api_key")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_secret("api_key", "redacted");
    let diagnostics = run_main_with_gates(&program, &gates).expect_err("missing SecretRead gate");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "RUNTIME_CAPABILITY_REQUIRED"
                && diagnostic.message.contains("SecretRead")),
        "expected missing SecretRead capability diagnostic, got {diagnostics:#?}"
    );
}

#[test]
fn cli_run_accepts_secret_seed() {
    let root = temp_project_dir("runtime-secret-cli");
    fs::create_dir_all(&root).expect("create temp dir");
    let source_path = root.join("main.sley");
    fs::write(
        &source_path,
        r#"
task main -> Result<Text, Error> uses SecretRead {
  return call secrets.try_get("api_key")
}
"#,
    )
    .expect("write source");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "run",
            "--json",
            "--cap",
            "SecretRead",
            "--secret",
            "api_key",
            "redacted",
            source_path.to_str().expect("utf-8 path"),
        ])
        .output()
        .expect("run sley");
    assert!(
        output.status.success(),
        "sley run failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).expect("parse value");
    assert_eq!(
        value,
        Value::Ok(Box::new(Value::Text("redacted".to_string())))
    );
}

#[test]
fn checker_requires_deploy_for_deploy_adapter() {
    let source = r#"
task main -> Result<Text, Error> {
  return call deploy.try_stage("staging")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    let effect_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "EFFECT_UNAUTHORIZED")
        .collect::<Vec<_>>();
    assert_eq!(
        effect_diagnostics.len(),
        1,
        "expected one effect diagnostic, got {diagnostics:#?}"
    );
    assert!(
        effect_diagnostics[0].message.contains("Deploy"),
        "expected Deploy diagnostic, got {diagnostics:#?}"
    );
}

#[test]
fn runtime_try_deploy_stage_reads_seeded_result() {
    let source = r#"
task main -> Result<Text, Error> uses Deploy {
  bind result = call deploy.try_stage("staging")?
  return Ok(result)
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect("Deploy");
    gates.grant_deploy_result("staging", "staged");
    assert_eq!(
        run_main_with_gates(&program, &gates),
        Ok(Value::Ok(Box::new(Value::Text("staged".to_string()))))
    );
}

#[test]
fn runtime_try_deploy_stage_returns_error_for_missing_seed() {
    let source = r#"
task main -> Result<Text, Error> uses Deploy {
  return call deploy.try_stage("staging")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect("Deploy");
    let result = run_main_with_gates(&program, &gates).expect("run main");
    let Value::Err(error) = result else {
        panic!("expected host error result, got {result:#?}");
    };
    assert_error_record(&error, "RUNTIME_DEPLOY_RESULT_NOT_FOUND", "was not seeded");
}

#[test]
fn runtime_try_deploy_stage_returns_error_for_empty_target() {
    let source = r#"
task main -> Result<Text, Error> uses Deploy {
  return call deploy.try_stage("")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect("Deploy");
    let result = run_main_with_gates(&program, &gates).expect("run main");
    let Value::Err(error) = result else {
        panic!("expected host error result, got {result:#?}");
    };
    assert_error_record(&error, "RUNTIME_DEPLOY_TARGET_INVALID", "deploy target");
}

#[test]
fn runtime_try_deploy_stage_requires_deploy_capability() {
    let source = r#"
task main -> Result<Text, Error> uses Deploy {
  return call deploy.try_stage("staging")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_deploy_result("staging", "staged");
    let diagnostics = run_main_with_gates(&program, &gates).expect_err("missing Deploy gate");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "RUNTIME_CAPABILITY_REQUIRED"
                && diagnostic.message.contains("Deploy")),
        "expected missing Deploy capability diagnostic, got {diagnostics:#?}"
    );
}

#[test]
fn cli_run_accepts_deploy_result_seed() {
    let root = temp_project_dir("runtime-deploy-cli");
    fs::create_dir_all(&root).expect("create temp dir");
    let source_path = root.join("main.sley");
    fs::write(
        &source_path,
        r#"
task main -> Result<Text, Error> uses Deploy {
  return call deploy.try_stage("staging")
}
"#,
    )
    .expect("write source");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "run",
            "--json",
            "--cap",
            "Deploy",
            "--deploy-result",
            "staging",
            "staged",
            source_path.to_str().expect("utf-8 path"),
        ])
        .output()
        .expect("run sley");
    assert!(
        output.status.success(),
        "sley run failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).expect("parse value");
    assert_eq!(
        value,
        Value::Ok(Box::new(Value::Text("staged".to_string())))
    );
}

#[test]
fn checker_requires_spend_for_spend_adapter() {
    let source = r#"
task main -> Result<Text, Error> {
  return call spend.try_authorize("ads-budget")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    let effect_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "EFFECT_UNAUTHORIZED")
        .collect::<Vec<_>>();
    assert_eq!(
        effect_diagnostics.len(),
        1,
        "expected one effect diagnostic, got {diagnostics:#?}"
    );
    assert!(
        effect_diagnostics[0].message.contains("Spend"),
        "expected Spend diagnostic, got {diagnostics:#?}"
    );
}

#[test]
fn runtime_try_spend_authorize_reads_seeded_result() {
    let source = r#"
task main -> Result<Text, Error> uses Spend {
  bind result = call spend.try_authorize("ads-budget")?
  return Ok(result)
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect("Spend");
    gates.grant_spend_result("ads-budget", "authorized");
    assert_eq!(
        run_main_with_gates(&program, &gates),
        Ok(Value::Ok(Box::new(Value::Text("authorized".to_string()))))
    );
}

#[test]
fn runtime_try_spend_authorize_returns_error_for_missing_seed() {
    let source = r#"
task main -> Result<Text, Error> uses Spend {
  return call spend.try_authorize("ads-budget")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect("Spend");
    let result = run_main_with_gates(&program, &gates).expect("run main");
    let Value::Err(error) = result else {
        panic!("expected host error result, got {result:#?}");
    };
    assert_error_record(&error, "RUNTIME_SPEND_RESULT_NOT_FOUND", "was not seeded");
}

#[test]
fn runtime_try_spend_authorize_returns_error_for_empty_request() {
    let source = r#"
task main -> Result<Text, Error> uses Spend {
  return call spend.try_authorize("")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_effect("Spend");
    let result = run_main_with_gates(&program, &gates).expect("run main");
    let Value::Err(error) = result else {
        panic!("expected host error result, got {result:#?}");
    };
    assert_error_record(&error, "RUNTIME_SPEND_REQUEST_INVALID", "spend request");
}

#[test]
fn runtime_try_spend_authorize_requires_spend_capability() {
    let source = r#"
task main -> Result<Text, Error> uses Spend {
  return call spend.try_authorize("ads-budget")
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let mut gates = RuntimeGates::new();
    gates.grant_spend_result("ads-budget", "authorized");
    let diagnostics = run_main_with_gates(&program, &gates).expect_err("missing Spend gate");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "RUNTIME_CAPABILITY_REQUIRED"
                && diagnostic.message.contains("Spend")),
        "expected missing Spend capability diagnostic, got {diagnostics:#?}"
    );
}

#[test]
fn cli_run_accepts_spend_result_seed() {
    let root = temp_project_dir("runtime-spend-cli");
    fs::create_dir_all(&root).expect("create temp dir");
    let source_path = root.join("main.sley");
    fs::write(
        &source_path,
        r#"
task main -> Result<Text, Error> uses Spend {
  return call spend.try_authorize("ads-budget")
}
"#,
    )
    .expect("write source");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "run",
            "--json",
            "--cap",
            "Spend",
            "--spend-result",
            "ads-budget",
            "authorized",
            source_path.to_str().expect("utf-8 path"),
        ])
        .output()
        .expect("run sley");
    assert!(
        output.status.success(),
        "sley run failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).expect("parse value");
    assert_eq!(
        value,
        Value::Ok(Box::new(Value::Text("authorized".to_string())))
    );
}

#[test]
fn cli_run_accepts_database_table_seed() {
    let root = temp_project_dir("runtime-db-cli");
    fs::create_dir_all(&root).expect("create temp dir");
    let source_path = root.join("main.sley");
    let table_path = root.join("users.json");
    fs::write(
        &source_path,
        r#"
task main -> Text uses DatabaseRead {
  bind row = call db.query_one("select * from users where id = ?", "u1")
  return row.text("name")
}
"#,
    )
    .expect("write source");
    fs::write(&table_path, r#"[{"id":"u1","name":"Ada"}]"#).expect("write table");

    let table = format!("users={}", table_path.display());
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "run",
            "--json",
            "--cap",
            "DatabaseRead",
            "--db-table",
            &table,
            source_path.to_str().expect("utf-8 path"),
        ])
        .output()
        .expect("run sley");
    assert!(
        output.status.success(),
        "sley run failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).expect("parse value");
    assert_eq!(value, Value::Text("Ada".to_string()));
}

#[test]
fn runtime_evaluates_locals_operators_and_if_expressions() {
    let source = r#"
task score -> Int {
  take base: Int

  bind doubled = base * 2
  return if doubled >= 10 && true { doubled + 1 } else { 0 }
}

task main -> Int {
  return call score(5)
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    assert_eq!(run_main(&program), Ok(Value::Int(11)));
}

#[test]
fn runtime_evaluates_while_set_lists_len_and_indexing() {
    let source = r#"
task sum -> Int {
  take values: List<Int>

  state index = 0
  tally total = 0
  while index < len(values) {
    set total = total + values[index]
    set index = index + 1
  }
  return total
}

task main -> Int {
  bind values = [2, 3, 5]
  return call sum(values)
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    assert_eq!(run_main(&program), Ok(Value::Int(10)));
}

#[test]
fn runtime_evaluates_for_loops_maps_len_and_text_indexing() {
    let source = r#"
task main -> Int {
  bind names = ["ada", "grace"]
  bind scores: Map<Text, Int> = map { "ada": 3, "grace": 5 }
  tally total = 0
  each name in names {
    set total = total + scores[name]
  }
  return total + len(scores)
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    assert_eq!(run_main(&program), Ok(Value::Int(10)));
}

#[test]
fn runtime_evaluates_pure_task_calls_and_record_fields() {
    let source = r#"
type User = {
  slot name: Text
}

task make_user -> User {
  take name: Text

  return User { name: name }
}

task main -> Text {
  bind user = call make_user("Ada")
  return user.name
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    assert_eq!(run_main(&program), Ok(Value::Text("Ada".to_string())));
}

#[test]
fn project_loader_resolves_imports_and_bundles_modules() {
    let root = temp_project_dir("resolves-imports");
    fs::create_dir_all(root.join("src/app")).expect("create project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"
[project]
name = "test-project"
root = "src"
entry = "app.main"
"#,
    )
    .expect("write manifest");
    fs::write(
        root.join("src/app/main.sley"),
        r#"
module app.main

import app.math as math

task main -> Int {
  return call math.double(21)
}
"#,
    )
    .expect("write main module");
    fs::write(
        root.join("src/app/math.sley"),
        r#"
module app.math

export task double -> Int {
  take value: Int

  return value * 2
}
"#,
    )
    .expect("write math module");

    let project = load_project(&root).expect("load project");
    assert_eq!(project.entry, "app.main");
    assert_eq!(project.modules.len(), 2);
    assert!(
        project
            .program
            .find_task_index("task:app.math.double")
            .is_some()
    );
    let diagnostics = check_program(&project.program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    assert_eq!(run_main(&project.program), Ok(Value::Int(42)));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_checker_enforces_exported_imported_tasks() {
    let root = temp_project_dir("private-imported-task");
    fs::create_dir_all(root.join("src/app")).expect("create project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"
[project]
entry = "app.main"
"#,
    )
    .expect("write manifest");
    fs::write(
        root.join("src/app/main.sley"),
        r#"
module app.main

import app.math as math

task main -> Int {
  return call math.double(21)
}
"#,
    )
    .expect("write main module");
    fs::write(
        root.join("src/app/math.sley"),
        r#"
module app.math

task double -> Int {
  take value: Int

  return value * 2
}
"#,
    )
    .expect("write math module");

    let project = load_project(&root).expect("load project");
    let diagnostics = check_program(&project.program);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "PRIVATE_TASK"),
        "expected private task diagnostic, got {diagnostics:#?}"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_checker_reports_ambiguous_imported_tasks() {
    let root = temp_project_dir("ambiguous-imported-task");
    fs::create_dir_all(root.join("src/app")).expect("create project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"
[project]
entry = "app.main"
"#,
    )
    .expect("write manifest");
    fs::write(
        root.join("src/app/main.sley"),
        r#"
module app.main

import app.one
import app.two

task main -> Int {
  return call value()
}
"#,
    )
    .expect("write main module");
    fs::write(
        root.join("src/app/one.sley"),
        r#"
module app.one

export task value -> Int {
  return 1
}
"#,
    )
    .expect("write first module");
    fs::write(
        root.join("src/app/two.sley"),
        r#"
module app.two

export task value -> Int {
  return 2
}
"#,
    )
    .expect("write second module");

    let project = load_project(&root).expect("load project");
    let diagnostics = check_program(&project.program);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "AMBIGUOUS_TASK"),
        "expected ambiguous task diagnostic, got {diagnostics:#?}"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_checker_rejects_unimported_fully_qualified_tasks() {
    let root = temp_project_dir("unimported-qualified-task");
    fs::create_dir_all(root.join("src/app")).expect("create project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"
[project]
entry = "app.main"
"#,
    )
    .expect("write manifest");
    fs::write(
        root.join("src/app/main.sley"),
        r#"
module app.main

import app.bridge

task main -> Int {
  return call app.math.double(21)
}
"#,
    )
    .expect("write main module");
    fs::write(
        root.join("src/app/bridge.sley"),
        r#"
module app.bridge

import app.math

export task bridge -> Int {
  return call math.double(21)
}
"#,
    )
    .expect("write bridge module");
    fs::write(
        root.join("src/app/math.sley"),
        r#"
module app.math

export task double -> Int {
  take value: Int

  return value * 2
}
"#,
    )
    .expect("write math module");

    let project = load_project(&root).expect("load project");
    let diagnostics = check_program(&project.program);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "UNKNOWN_TASK"),
        "expected unknown task diagnostic, got {diagnostics:#?}"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_checker_resolves_imported_types_and_record_literals() {
    let root = temp_project_dir("imported-record-types");
    fs::create_dir_all(root.join("src/app")).expect("create project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"
[project]
entry = "app.main"
"#,
    )
    .expect("write manifest");
    fs::write(
        root.join("src/app/main.sley"),
        r#"
module app.main

import app.types as t

task main -> Text {
  bind user: t.User = t.User { name: "Ada" }
  return user.name
}
"#,
    )
    .expect("write main module");
    fs::write(
        root.join("src/app/types.sley"),
        r#"
module app.types

export type User = {
  slot name: Text
}
"#,
    )
    .expect("write types module");

    let project = load_project(&root).expect("load project");
    let diagnostics = check_program(&project.program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    assert_eq!(
        run_main(&project.program),
        Ok(Value::Text("Ada".to_string()))
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_checker_enforces_exported_imported_types() {
    let root = temp_project_dir("private-imported-type");
    fs::create_dir_all(root.join("src/app")).expect("create project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"
[project]
entry = "app.main"
"#,
    )
    .expect("write manifest");
    fs::write(
        root.join("src/app/main.sley"),
        r#"
module app.main

import app.types as t

task main -> t.Secret {
  return t.Secret { value: "x" }
}
"#,
    )
    .expect("write main module");
    fs::write(
        root.join("src/app/types.sley"),
        r#"
module app.types

type Secret = {
  slot value: Text
}
"#,
    )
    .expect("write types module");

    let project = load_project(&root).expect("load project");
    let diagnostics = check_program(&project.program);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "PRIVATE_TYPE"),
        "expected private type diagnostic, got {diagnostics:#?}"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_checker_reports_ambiguous_imported_types() {
    let root = temp_project_dir("ambiguous-imported-type");
    fs::create_dir_all(root.join("src/app")).expect("create project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"
[project]
entry = "app.main"
"#,
    )
    .expect("write manifest");
    fs::write(
        root.join("src/app/main.sley"),
        r#"
module app.main

import app.one
import app.two

task main -> Token {
  return Token { value: "x" }
}
"#,
    )
    .expect("write main module");
    fs::write(
        root.join("src/app/one.sley"),
        r#"
module app.one

export type Token = {
  slot value: Text
}
"#,
    )
    .expect("write first module");
    fs::write(
        root.join("src/app/two.sley"),
        r#"
module app.two

export type Token = {
  slot value: Text
}
"#,
    )
    .expect("write second module");

    let project = load_project(&root).expect("load project");
    let diagnostics = check_program(&project.program);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "AMBIGUOUS_TYPE"),
        "expected ambiguous type diagnostic, got {diagnostics:#?}"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_checker_resolves_imported_effects_for_calls() {
    let root = temp_project_dir("imported-effects");
    fs::create_dir_all(root.join("src/app")).expect("create project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"
[project]
entry = "app.main"
"#,
    )
    .expect("write manifest");
    fs::write(
        root.join("src/app/main.sley"),
        r#"
module app.main

import app.io as io

task main -> Text uses io.Read {
  return call io.read()
}
"#,
    )
    .expect("write main module");
    fs::write(
        root.join("src/app/io.sley"),
        r#"
module app.io

export effect Read

export task read -> Text uses Read {
  return "ok"
}
"#,
    )
    .expect("write io module");

    let project = load_project(&root).expect("load project");
    let diagnostics = check_program(&project.program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let lint = build_lint_report(
        &project.program,
        LintOptions {
            rules: vec![LintRule::UnusedDeclaredEffect],
            module: Some("app.main".to_string()),
        },
    );
    assert_eq!(lint.status, "ok", "unexpected lint findings: {lint:#?}");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_checker_enforces_exported_imported_effects() {
    let root = temp_project_dir("private-imported-effect");
    fs::create_dir_all(root.join("src/app")).expect("create project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"
[project]
entry = "app.main"
"#,
    )
    .expect("write manifest");
    fs::write(
        root.join("src/app/main.sley"),
        r#"
module app.main

import app.io as io

task main -> Text uses io.Read {
  return "ok"
}
"#,
    )
    .expect("write main module");
    fs::write(
        root.join("src/app/io.sley"),
        r#"
module app.io

effect Read
"#,
    )
    .expect("write io module");

    let project = load_project(&root).expect("load project");
    let diagnostics = check_program(&project.program);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "PRIVATE_EFFECT"),
        "expected private effect diagnostic, got {diagnostics:#?}"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn graph_slice_reports_resolved_project_task_calls() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/project");
    let project = load_project(&project_root).expect("load project");
    let slice =
        slice_symbol_graph(&project.program, "task:app.main.main").expect("slice main task");

    assert_eq!(slice.focus.kind, "task");
    assert_eq!(slice.focus.module, "app.main");
    assert_eq!(slice.imports.len(), 1);
    assert_eq!(slice.imports[0].id, "import:app.main:app.math");
    assert_eq!(slice.outbound_calls.len(), 1);
    assert_eq!(slice.outbound_calls[0].callee, "math.double");
    assert_eq!(
        slice.outbound_calls[0].target.as_deref(),
        Some("app.math.double")
    );
}

#[test]
fn graph_slice_reports_move_node_affordances() {
    let source = r#"
task helper -> Int {
  take other: Int

  return other
}

task main -> Int {
  take value: Int

  tally total = value
  if true {
    set total = total + 1
  }
  return total
}
"#;
    let program = parse_program(source).expect("parse source");
    let slice = slice_symbol_graph(&program, "task:main.main").expect("slice main");

    let nested_statement = slice
        .move_affordances
        .iter()
        .find(|affordance| affordance.target == "block:task:main.main:stmt:1:then:stmt:0")
        .expect("nested statement move affordance");
    assert_eq!(nested_statement.target_kind, "statement");
    assert_eq!(nested_statement.parent, "block:task:main.main:stmt:1:then");
    assert_eq!(nested_statement.position, 0);
    assert_eq!(nested_statement.max_position, 0);
    assert_eq!(
        nested_statement.operation.pointer("/op"),
        Some(&serde_json::json!("MoveNode"))
    );
    assert_eq!(
        nested_statement.operation.pointer("/payload/parent"),
        Some(&serde_json::json!("block:task:main.main:stmt:1:then"))
    );
    assert_eq!(
        nested_statement.editable_json_pointers,
        vec!["/payload/position".to_string()]
    );
    let nested_graft: GraftInput =
        serde_json::from_value(nested_statement.operation.clone()).expect("parse nested move");
    let nested_outcome = apply_graft_input(&program, nested_graft, Some("agent:test".to_string()));
    assert_eq!(
        nested_outcome.status, "accepted",
        "{:#?}",
        nested_outcome.diagnostics
    );
    let top_destination = nested_statement
        .destinations
        .iter()
        .find(|destination| destination.parent == "block:task:main.main")
        .expect("top-level destination block");
    assert!(
        top_destination.max_position == 3,
        "expected top-level destination block insertion limit, got {top_destination:#?}"
    );
    assert_eq!(
        top_destination.operation.pointer("/payload/destination"),
        Some(&serde_json::json!("block:task:main.main"))
    );
    assert_eq!(
        top_destination.operation.pointer("/payload/position"),
        Some(&serde_json::json!(3))
    );
    let destination_graft: GraftInput =
        serde_json::from_value(top_destination.operation.clone()).expect("parse destination move");
    let destination_outcome =
        apply_graft_input(&program, destination_graft, Some("agent:test".to_string()));
    assert_eq!(
        destination_outcome.status, "accepted",
        "{:#?}",
        destination_outcome.diagnostics
    );
    assert!(
        !nested_statement
            .destinations
            .iter()
            .any(|destination| destination
                .parent
                .starts_with("block:task:main.main:stmt:1:then:stmt:0")),
        "statement move affordance should not allow moving into its own child"
    );

    let take = slice
        .move_affordances
        .iter()
        .find(|affordance| affordance.target == "take:task:main.main:0:value")
        .expect("take move affordance");
    assert_eq!(take.target_kind, "take");
    assert_eq!(take.parent, "task:main.main:takes");
    assert_eq!(take.position, 0);
    assert_eq!(take.max_position, 0);
    assert_eq!(
        take.operation.pointer("/target"),
        Some(&serde_json::json!("take:task:main.main:0:value"))
    );
    assert!(
        take.destinations
            .iter()
            .any(|destination| destination.parent == "task:main.helper:takes"
                && destination.max_position == 1),
        "expected helper take destination, got {take:#?}"
    );

    let delete_take = slice
        .delete_affordances
        .iter()
        .find(|affordance| affordance.target == "take:task:main.main:0:value")
        .expect("take delete affordance");
    assert_eq!(delete_take.target_kind, "take");
    assert_eq!(delete_take.parent, "task:main.main:takes");
    assert_eq!(delete_take.position, 0);
    assert_eq!(
        delete_take.operation.pointer("/op"),
        Some(&serde_json::json!("DeleteNode"))
    );
    assert!(delete_take.editable_json_pointers.is_empty());

    let delete_statement = slice
        .delete_affordances
        .iter()
        .find(|affordance| affordance.target == "block:task:main.main:stmt:1:then:stmt:0")
        .expect("nested statement delete affordance");
    assert_eq!(delete_statement.target_kind, "statement");
    assert_eq!(delete_statement.parent, "block:task:main.main:stmt:1:then");
    assert_eq!(
        delete_statement.operation.pointer("/target"),
        Some(&serde_json::json!(
            "block:task:main.main:stmt:1:then:stmt:0"
        ))
    );

    let replace_return = slice
        .replace_affordances
        .iter()
        .find(|affordance| affordance.target == "block:task:main.main:stmt:2:expr")
        .expect("return expression replace affordance");
    assert_eq!(replace_return.target_kind, "Identifier");
    assert_eq!(replace_return.parent, "block:task:main.main:stmt:2");
    assert_eq!(
        replace_return.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        replace_return.operation.pointer("/payload/source"),
        Some(&serde_json::json!("total"))
    );
    assert_eq!(
        replace_return.editable_json_pointers,
        vec!["/payload/source".to_string()]
    );
    let replace_graft: GraftInput = serde_json::from_value(replace_return.operation.clone())
        .expect("parse replace expression affordance");
    let replace_outcome =
        apply_graft_input(&program, replace_graft, Some("agent:test".to_string()));
    assert_eq!(
        replace_outcome.status, "accepted",
        "{:#?}",
        replace_outcome.diagnostics
    );
}

#[test]
fn graph_slice_reports_module_move_affordances() {
    let source = r#"
module app.main

import app.extra

effect Audit

type User = {
  slot name: Text
}

task helper -> Int {
  return 2
}

task main -> Int {
  return 1
}
"#;
    let program = parse_program(source).expect("parse source");
    let slice = slice_symbol_graph(&program, "module:app.main").expect("slice module");

    let import = slice
        .move_affordances
        .iter()
        .find(|affordance| affordance.target == "import:app.main:app.extra")
        .expect("import move affordance");
    assert_eq!(import.target_kind, "import");
    assert_eq!(import.parent, "module:app.main:imports");
    assert_eq!(
        import.operation.pointer("/payload/parent"),
        Some(&serde_json::json!("module:app.main:imports"))
    );

    let ty = slice
        .move_affordances
        .iter()
        .find(|affordance| affordance.target == "type:app.main.User")
        .expect("type move affordance");
    assert_eq!(ty.target_kind, "type");
    assert_eq!(ty.parent, "module:app.main:types");

    let effect = slice
        .move_affordances
        .iter()
        .find(|affordance| affordance.target == "effect:app.main.Audit")
        .expect("effect move affordance");
    assert_eq!(effect.target_kind, "effect");
    assert_eq!(effect.parent, "module:app.main:effects");

    let task = slice
        .move_affordances
        .iter()
        .find(|affordance| affordance.target == "task:app.main.helper")
        .expect("task move affordance");
    assert_eq!(task.target_kind, "task");
    assert_eq!(task.parent, "module:app.main:tasks");
    assert_eq!(task.position, 0);
    assert_eq!(task.max_position, 1);
    let destination = task
        .destinations
        .iter()
        .find(|destination| destination.parent == "module:app.extra:tasks")
        .expect("task module destination");
    assert_eq!(destination.max_position, 0);
    assert_eq!(
        destination.operation.pointer("/payload/parent"),
        Some(&serde_json::json!("module:app.extra:tasks"))
    );
    let graft: GraftInput =
        serde_json::from_value(destination.operation.clone()).expect("parse module move");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);

    let delete_import = slice
        .delete_affordances
        .iter()
        .find(|affordance| affordance.target == "import:app.main:app.extra")
        .expect("import delete affordance");
    assert_eq!(delete_import.target_kind, "import");
    assert_eq!(delete_import.parent, "module:app.main:imports");
    assert_eq!(
        delete_import.operation.pointer("/op"),
        Some(&serde_json::json!("DeleteNode"))
    );

    let delete_task = slice
        .delete_affordances
        .iter()
        .find(|affordance| affordance.target == "task:app.main.helper")
        .expect("task delete affordance");
    assert_eq!(delete_task.target_kind, "task");
    assert_eq!(delete_task.parent, "module:app.main:tasks");
    assert_eq!(delete_task.position, 0);
}

#[test]
fn query_report_lists_checked_project_tasks() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/project");
    let project = load_project(&project_root).expect("load project");
    let diagnostics = check_program(&project.program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );

    let report = build_query_report(
        &project.program,
        QueryOptions {
            kind: QueryKind::Tasks,
            module: Some("app.main".to_string()),
            exported_only: false,
        },
    );

    assert_eq!(report.schema, QUERY_REPORT_SCHEMA);
    assert_eq!(report.kind, "tasks");
    assert_eq!(report.entry_module, "app.main");
    assert_eq!(report.filters.module.as_deref(), Some("app.main"));
    assert_eq!(report.modules.len(), 0);
    assert_eq!(report.calls.len(), 0);
    assert_eq!(report.tasks.len(), 1);
    assert_eq!(report.tasks[0].id, "task:app.main.main");
    assert_eq!(report.tasks[0].qualified_name, "app.main.main");
    assert_eq!(report.tasks[0].return_type, "Int");
    assert_eq!(report.tasks[0].outbound_call_count, 1);
    assert_eq!(report.tasks[0].inbound_call_count, 0);

    let module_report = build_query_report(
        &project.program,
        QueryOptions {
            kind: QueryKind::Modules,
            module: Some("app.main".to_string()),
            exported_only: false,
        },
    );
    assert_eq!(module_report.modules.len(), 1);
    assert_eq!(module_report.modules[0].imports.len(), 1);
    assert_eq!(
        module_report.modules[0].imports[0].id,
        "import:app.main:app.math"
    );
}

#[test]
fn lint_report_flags_unused_private_tasks() {
    let source = r#"
module app.lint

task main -> Int {
  return call used()
}

task used -> Int {
  return 1
}

task orphan -> Int {
  return 2
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );

    let report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::UnusedPrivateTask],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.lint");
    assert_eq!(report.filters.rules, vec!["unused_private_task"]);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].id, "UNUSED_PRIVATE_TASK");
    assert_eq!(report.findings[0].node, "task:app.lint.orphan");
    assert_eq!(report.findings[0].module, "app.lint");
}

#[test]
fn lint_report_flags_unreachable_private_task_cycles() {
    let source = r#"
module app.reach

task main -> Int {
  return call main_helper()
}

task main_helper -> Int {
  return 0
}

export task public -> Int {
  return call public_helper()
}

task public_helper -> Int {
  return 1
}

task cycle_a -> Int {
  return call cycle_b()
}

task cycle_b -> Int {
  return call cycle_a()
}

task orphan -> Int {
  return 2
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );

    let report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::UnreachablePrivateTask],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.reach");
    assert_eq!(report.filters.rules, vec!["unreachable_private_task"]);
    assert_eq!(report.findings.len(), 2);
    assert_eq!(report.findings[0].id, "UNREACHABLE_PRIVATE_TASK");
    assert_eq!(report.findings[0].node, "task:app.reach.cycle_a");
    assert_eq!(report.findings[1].node, "task:app.reach.cycle_b");
}

#[test]
fn lint_report_flags_unused_declared_effects() {
    let source = r#"
module app.effects

task main -> Text uses FileRead {
  return call read_name("/tmp/name.txt")
}

task read_name -> Text uses FileRead {
  take path: Text

  return fs.read_text(path)
}

task stale -> Text uses Network {
  return "unused"
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );

    let report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::UnusedDeclaredEffect],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.effects");
    assert_eq!(report.filters.rules, vec!["unused_declared_effect"]);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].id, "UNUSED_DECLARED_EFFECT");
    assert_eq!(report.findings[0].node, "task:app.effects.stale");
    assert_eq!(report.findings[0].module, "app.effects");
    assert!(report.findings[0].message.contains("Network"));
}

#[test]
fn lint_report_flags_raw_host_adapters() {
    let source = include_str!("../examples/file_gate.sley");
    let program = parse_program(source).expect("parse file gate fixture");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );

    let report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::RawHostAdapter],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.filters.rules, vec!["raw_host_adapter"]);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].id, "RAW_HOST_ADAPTER");
    assert_eq!(report.findings[0].rule, "raw_host_adapter");
    assert!(report.findings[0].message.contains("fs.read_text"));
    assert!(report.findings[0].hint.contains("fs.try_read_text"));
}

#[test]
fn lint_report_flags_unused_imports() {
    let root = temp_project_dir("unused-import");
    write_unused_import_project(&root);
    let project = load_project(&root).expect("load project");
    let diagnostics = check_program(&project.program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );

    let report = build_lint_report(
        &project.program,
        LintOptions {
            rules: vec![LintRule::UnusedImport],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.main");
    assert_eq!(report.filters.rules, vec!["unused_import"]);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].id, "UNUSED_IMPORT");
    assert_eq!(report.findings[0].rule, "unused_import");
    assert_eq!(report.findings[0].node, "import:app.main:app.stale");
    assert_eq!(report.findings[0].module, "app.main");
    assert!(report.findings[0].message.contains("imports `app.stale`"));

    let used_import_report = build_lint_report(
        &project.program,
        LintOptions {
            rules: vec![LintRule::UnusedImport],
            module: Some("app.used".to_string()),
        },
    );
    assert_eq!(used_import_report.status, "ok");
    assert!(used_import_report.findings.is_empty());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn lint_report_flags_unused_takes() {
    let source = r#"
module app.takes

task main -> Int uses FileRead {
  return call helper(21, 0)
}

task helper -> Int uses FileRead {
  take value: Int
  take unused: Int
  take gate files: Gate<FileRead>

  return value
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );

    let report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::UnusedTake],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.takes");
    assert_eq!(report.filters.rules, vec!["unused_take"]);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].id, "UNUSED_TAKE");
    assert_eq!(report.findings[0].rule, "unused_take");
    assert_eq!(
        report.findings[0].node,
        "take:task:app.takes.helper:1:unused"
    );
    assert_eq!(report.findings[0].module, "app.takes");
    assert!(report.findings[0].message.contains("take `unused`"));
    assert!(report.findings[0].hint.contains("update callers"));
}

#[test]
fn lint_report_flags_unused_private_types_and_effects() {
    let source = r#"
module app.decls

type Used = {
  slot name: Text
}

type Orphan = {
  slot id: Int
}

export type Public = {
  slot value: Text
}

effect UsedEffect
effect OrphanEffect
export effect PublicEffect

task main -> Used uses UsedEffect {
  return Used { name: "Ada" }
}
"#;
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );

    let type_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::UnusedPrivateType],
            module: None,
        },
    );
    assert_eq!(type_report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(type_report.status, "findings");
    assert_eq!(type_report.filters.rules, vec!["unused_private_type"]);
    assert_eq!(type_report.findings.len(), 1);
    assert_eq!(type_report.findings[0].id, "UNUSED_PRIVATE_TYPE");
    assert_eq!(type_report.findings[0].rule, "unused_private_type");
    assert_eq!(type_report.findings[0].node, "type:app.decls.Orphan");
    assert_eq!(type_report.findings[0].module, "app.decls");

    let effect_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::UnusedPrivateEffect],
            module: None,
        },
    );
    assert_eq!(effect_report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(effect_report.status, "findings");
    assert_eq!(effect_report.filters.rules, vec!["unused_private_effect"]);
    assert_eq!(effect_report.findings.len(), 1);
    assert_eq!(effect_report.findings[0].id, "UNUSED_PRIVATE_EFFECT");
    assert_eq!(effect_report.findings[0].rule, "unused_private_effect");
    assert_eq!(
        effect_report.findings[0].node,
        "effect:app.decls.OrphanEffect"
    );
    assert_eq!(effect_report.findings[0].module, "app.decls");
}

#[test]
fn trace_receipts_round_trip_as_jsonl() {
    let root = temp_project_dir("trace-round-trip");
    let trace_path = root.join(".sley/trace.jsonl");
    let receipt = build_trace_receipt(
        root.join("src/app/main.sley"),
        vec![ProvenanceRecord {
            graft_id: "graft_test".to_string(),
            actor: "agent:test".to_string(),
            timestamp: "2026-05-05T00:00:00Z".to_string(),
            operation: "AddTake".to_string(),
            targets: vec!["task:app.main.main".to_string()],
            result: "accepted".to_string(),
        }],
    );

    append_trace_receipt(&trace_path, &receipt).expect("append receipt");
    let receipts = read_trace_receipts(&trace_path).expect("read receipts");

    assert_eq!(receipts, vec![receipt]);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn trace_seals_are_content_addressed() {
    let source = include_str!("../examples/hello.sley");
    let program = parse_program(source).expect("parse fixture");
    let receipt = TraceReceipt {
        schema: TRACE_RECEIPT_SCHEMA.to_string(),
        target: "examples/hello.sley".to_string(),
        written_at: "2026-05-05T00:00:00Z".to_string(),
        provenance: vec![ProvenanceRecord {
            graft_id: "graft_test".to_string(),
            actor: "agent:test".to_string(),
            timestamp: "2026-05-05T00:00:00Z".to_string(),
            operation: "AddTake".to_string(),
            targets: vec!["task:app.hello.main".to_string()],
            result: "accepted".to_string(),
        }],
    };
    let seal = build_trace_seal(
        "examples/hello.sley",
        source.as_bytes(),
        &program,
        std::slice::from_ref(&receipt),
    )
    .expect("build seal");
    let repeat = build_trace_seal(
        "examples/hello.sley",
        source.as_bytes(),
        &program,
        std::slice::from_ref(&receipt),
    )
    .expect("build repeat seal");
    let changed = build_trace_seal(
        "examples/hello.sley",
        b"task main -> Text {\n  return \"changed\"\n}\n",
        &program,
        &[receipt],
    )
    .expect("build changed seal");

    assert_eq!(seal.schema, TRACE_SEAL_SCHEMA);
    assert_eq!(seal.receipt_count, 1);
    assert_eq!(seal, repeat);
    assert_ne!(seal.source_digest, changed.source_digest);
    assert_ne!(seal.seal_digest, changed.seal_digest);
    assert!(seal.seal_digest.starts_with("sha256:"));
}

#[test]
fn graft_cli_dry_run_is_explicit_and_non_mutating() {
    let root = temp_project_dir("graft-dry-run");
    fs::create_dir_all(&root).expect("create temp dir");
    let file = root.join("main.sley");
    let graft = root.join("add_take.json");
    let source = r#"task main -> Int {
  return 1
}
"#;
    fs::write(&file, source).expect("write source");
    fs::write(
        &graft,
        r#"
{
  "op": "AddTake",
  "target": "task:main.main",
  "payload": { "name": "amount", "type": "Int" }
}
"#,
    )
    .expect("write graft");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .arg("graft")
        .arg("--dry-run")
        .arg(&file)
        .arg(&graft)
        .output()
        .expect("run dry-run graft");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(output.status.success(), "stderr: {stderr}");
    assert!(stdout.contains("take amount: Int"));
    assert_eq!(fs::read_to_string(&file).expect("read source"), source);
    assert!(!root.join(".sley/trace.jsonl").exists());

    let conflict = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["graft", "--dry-run", "--write"])
        .arg(&file)
        .arg(&graft)
        .output()
        .expect("run conflicting graft flags");
    assert!(!conflict.status.success());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_graft_write_updates_only_the_owning_module() {
    let root = temp_project_dir("project-graft-write");
    fs::create_dir_all(root.join("src/app")).expect("create project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"
[project]
entry = "app.main"
"#,
    )
    .expect("write manifest");
    let main_source = r#"module app.main

import app.math as math

task main -> Int {
  return call math.double(21)
}
"#;
    let math_source = r#"module app.math

export task double -> Int {
  take value: Int

  return value * 2
}
"#;
    let main_path = root.join("src/app/main.sley");
    let math_path = root.join("src/app/math.sley");
    let graft_path = root.join("insert_math_bind.json");
    fs::write(&main_path, main_source).expect("write main module");
    fs::write(&math_path, math_source).expect("write math module");
    fs::write(
        &graft_path,
        r#"
{
  "op": "InsertStatement",
  "target": "task:app.math.double",
  "payload": { "source": "bind extra = 0", "position": 0 }
}
"#,
    )
    .expect("write graft");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["graft", "--json", "--write"])
        .arg(&root)
        .arg(&graft_path)
        .output()
        .expect("run project graft");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let stderr = String::from_utf8(output.stderr).expect("stderr utf8");
    assert!(
        output.status.success(),
        "stdout: {stdout}\nstderr: {stderr}"
    );
    let outcome: GraftOutcome = serde_json::from_str(&stdout).expect("parse outcome");
    assert_eq!(outcome.status, "accepted");
    assert_eq!(
        fs::read_to_string(&main_path).expect("read main"),
        main_source
    );
    let math_written = fs::read_to_string(&math_path).expect("read math");
    assert!(math_written.contains("bind extra = 0"), "{math_written}");

    let project = load_project(&root).expect("reload project");
    let diagnostics = check_program(&project.program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    assert_eq!(run_main(&project.program), Ok(Value::Int(42)));
    let receipts = read_trace_receipts(root.join(".sley/trace.jsonl")).expect("read trace");
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].provenance[0].operation, "InsertStatement");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_graft_write_applies_declaration_cleanup_transaction() {
    let root = temp_project_dir("project-graft-declaration-cleanup");
    fs::create_dir_all(root.join("src/app")).expect("create project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"
[project]
entry = "app.main"
"#,
    )
    .expect("write manifest");
    let main_source = r#"module app.main

type Orphan = {
  slot id: Int
}

effect OrphanEffect

task main -> Unit {
}
"#;
    let main_path = root.join("src/app/main.sley");
    let graft_path = root.join("cleanup_declarations.json");
    fs::write(&main_path, main_source).expect("write main module");

    let project = load_project(&root).expect("load cleanup project");
    let plan = build_edit_plan_report_with_options(
        root.to_string_lossy(),
        Ok(project.program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
        },
    );
    let cleanup = plan
        .transaction_templates
        .iter()
        .find(|template| template.kind == "delete_unused_private_declarations")
        .expect("declaration cleanup transaction");
    fs::write(
        &graft_path,
        serde_json::to_string_pretty(&cleanup.transaction).expect("serialize cleanup transaction"),
    )
    .expect("write cleanup graft");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["graft", "--json", "--write"])
        .arg(&root)
        .arg(&graft_path)
        .output()
        .expect("run declaration cleanup graft");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        output.status.success(),
        "project writeback should apply declaration cleanup; stdout={stdout} stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let outcome: GraftOutcome = serde_json::from_str(&stdout).expect("parse outcome");
    assert_eq!(outcome.status, "accepted");
    assert_eq!(outcome.provenance.len(), 2);
    assert!(
        outcome
            .provenance
            .iter()
            .all(|record| record.operation == "DeleteNode")
    );
    assert_eq!(
        fs::read_to_string(&main_path).expect("read cleaned main"),
        "module app.main\n\ntask main -> Unit {\n}\n"
    );

    let cleaned = load_project(&root).expect("reload cleaned project");
    let diagnostics = check_program(&cleaned.program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let lint = build_lint_report(&cleaned.program, LintOptions::default());
    assert_eq!(lint.status, "ok", "unexpected lint findings: {lint:#?}");
    let receipts = read_trace_receipts(root.join(".sley/trace.jsonl")).expect("read trace");
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].provenance.len(), 2);
    assert!(
        receipts[0]
            .provenance
            .iter()
            .all(|record| record.operation == "DeleteNode")
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_graft_write_creates_checked_new_module() {
    let root = temp_project_dir("project-graft-new-module");
    fs::create_dir_all(root.join("src/app")).expect("create project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"
[project]
entry = "app.main"
"#,
    )
    .expect("write manifest");
    let main_source = r#"module app.main

task main -> Int {
  return 1
}
"#;
    let main_path = root.join("src/app/main.sley");
    let graft_path = root.join("add_extra_module_task.json");
    fs::write(&main_path, main_source).expect("write main module");
    fs::write(
        &graft_path,
        r#"
{
  "transaction": "graft_new_module",
  "mode": "all_or_nothing",
  "ops": [
    { "op": "AddImport", "payload": { "module": "app.extra" } },
    {
      "op": "AddTask",
      "payload": {
        "source": "module app.extra\n\nexport task main -> Int {\n  return 2\n}\n"
      }
    }
  ]
}
"#,
    )
    .expect("write graft");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["graft", "--json", "--write"])
        .arg(&root)
        .arg(&graft_path)
        .output()
        .expect("run project graft");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        output.status.success(),
        "project writeback should create checked new module; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let outcome: GraftOutcome = serde_json::from_str(&stdout).expect("parse outcome");
    assert_eq!(outcome.status, "accepted");
    assert_eq!(
        fs::read_to_string(&main_path).expect("read main"),
        "module app.main\n\nimport app.extra\n\ntask main -> Int {\n  return 1\n}\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("src/app/extra.sley")).expect("read extra module"),
        "module app.extra\n\nexport task main -> Int {\n  return 2\n}\n"
    );
    let check = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["check", "--json"])
        .arg(&root)
        .output()
        .expect("check project");
    assert!(
        check.status.success(),
        "created project should check; stdout={} stderr={}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    let receipts = read_trace_receipts(&root.join(".sley/trace.jsonl")).expect("read trace");
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].provenance.len(), 2);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_graft_write_deletes_removed_module_file() {
    let root = temp_project_dir("project-graft-delete-module");
    fs::create_dir_all(root.join("src/app")).expect("create project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"
[project]
entry = "app.main"
"#,
    )
    .expect("write manifest");
    let main_source = r#"module app.main

import app.extra

task main -> Int {
  return 1
}
"#;
    let extra_source = r#"module app.extra

export task helper -> Int {
  return 2
}
"#;
    let main_path = root.join("src/app/main.sley");
    let extra_path = root.join("src/app/extra.sley");
    let graft_path = root.join("delete_extra_module.json");
    fs::write(&main_path, main_source).expect("write main module");
    fs::write(&extra_path, extra_source).expect("write extra module");
    fs::write(
        &graft_path,
        r#"
{
  "transaction": "graft_delete_module",
  "mode": "all_or_nothing",
  "ops": [
    { "op": "DeleteNode", "target": "import:app.extra" },
    { "op": "DeleteNode", "target": "task:app.extra.helper" }
  ]
}
"#,
    )
    .expect("write graft");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["graft", "--json", "--write"])
        .arg(&root)
        .arg(&graft_path)
        .output()
        .expect("run project graft");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        output.status.success(),
        "project writeback should delete removed module; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let outcome: GraftOutcome = serde_json::from_str(&stdout).expect("parse outcome");
    assert_eq!(outcome.status, "accepted");
    assert_eq!(
        fs::read_to_string(&main_path).expect("read main"),
        "module app.main\n\ntask main -> Int {\n  return 1\n}\n"
    );
    assert!(
        !extra_path.exists(),
        "removed module file should be deleted"
    );
    let check = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["check", "--json"])
        .arg(&root)
        .output()
        .expect("check project");
    assert!(
        check.status.success(),
        "project should check after module deletion; stdout={} stderr={}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    let receipts = read_trace_receipts(&root.join(".sley/trace.jsonl")).expect("read trace");
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].provenance.len(), 2);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_graft_write_renames_non_entry_module_file() {
    let root = temp_project_dir("project-graft-rename-module");
    fs::create_dir_all(root.join("src/app")).expect("create project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"
[project]
entry = "app.main"
"#,
    )
    .expect("write manifest");
    let main_source = r#"module app.main

import app.extra

task main -> Int {
  return call extra.helper()
}
"#;
    let extra_source = r#"module app.extra

export task helper -> Int {
  return 2
}
"#;
    let main_path = root.join("src/app/main.sley");
    let extra_path = root.join("src/app/extra.sley");
    let lib_path = root.join("src/app/lib.sley");
    let graft_path = root.join("rename_extra_module.json");
    fs::write(&main_path, main_source).expect("write main module");
    fs::write(&extra_path, extra_source).expect("write extra module");
    fs::write(
        &graft_path,
        r#"
{
  "op": "RenameDeclaration",
  "target": "module:app.extra",
  "payload": { "name": "app.lib" }
}
"#,
    )
    .expect("write graft");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["graft", "--json", "--write"])
        .arg(&root)
        .arg(&graft_path)
        .output()
        .expect("run project graft");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        output.status.success(),
        "project writeback should rename non-entry module; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let outcome: GraftOutcome = serde_json::from_str(&stdout).expect("parse outcome");
    assert_eq!(outcome.status, "accepted");
    assert_eq!(
        fs::read_to_string(&main_path).expect("read main"),
        "module app.main\n\nimport app.lib as extra\n\ntask main -> Int {\n  return call extra.helper()\n}\n"
    );
    assert!(!extra_path.exists(), "old module file should be deleted");
    assert_eq!(
        fs::read_to_string(&lib_path).expect("read renamed module"),
        "module app.lib\n\nexport task helper -> Int {\n  return 2\n}\n"
    );
    let run = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["run", "--json"])
        .arg(&root)
        .output()
        .expect("run project");
    assert!(
        run.status.success(),
        "renamed project should run; stdout={} stderr={}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let receipts = read_trace_receipts(&root.join(".sley/trace.jsonl")).expect("read trace");
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].provenance.len(), 1);
    assert_eq!(receipts[0].provenance[0].operation, "RenameDeclaration");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_graft_write_renames_entry_module_and_manifest() {
    let root = temp_project_dir("project-graft-entry-rename");
    fs::create_dir_all(root.join("src/app")).expect("create project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"
[project]
entry = "app.main"
"#,
    )
    .expect("write manifest");
    let main_source = r#"module app.main

task main -> Int {
  return 1
}
"#;
    let main_path = root.join("src/app/main.sley");
    let core_path = root.join("src/app/core.sley");
    let graft_path = root.join("rename_entry_module.json");
    fs::write(&main_path, main_source).expect("write main module");
    fs::write(
        &graft_path,
        r#"
{
  "op": "RenameDeclaration",
  "target": "module:app.main",
  "payload": { "name": "app.core" }
}
"#,
    )
    .expect("write graft");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["graft", "--json", "--write"])
        .arg(&root)
        .arg(&graft_path)
        .output()
        .expect("run project graft");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        output.status.success(),
        "entry module rename should update manifest; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let outcome: GraftOutcome = serde_json::from_str(&stdout).expect("parse outcome");
    assert_eq!(outcome.status, "accepted");
    let manifest: toml::Value =
        toml::from_str(&fs::read_to_string(root.join("sley.toml")).expect("read manifest"))
            .expect("parse manifest");
    assert_eq!(
        manifest
            .get("project")
            .and_then(|project| project.get("entry"))
            .and_then(toml::Value::as_str),
        Some("app.core")
    );
    assert!(
        !main_path.exists(),
        "old entry module file should be deleted"
    );
    assert_eq!(
        fs::read_to_string(&core_path).expect("read renamed entry"),
        "module app.core\n\ntask main -> Int {\n  return 1\n}\n"
    );
    let project = load_project(&root).expect("reload renamed project");
    assert_eq!(project.entry, "app.core");
    let run = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["run", "--json"])
        .arg(&root)
        .output()
        .expect("run renamed project");
    assert!(
        run.status.success(),
        "renamed entry project should run; stdout={} stderr={}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let receipts = read_trace_receipts(&root.join(".sley/trace.jsonl")).expect("read trace");
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].provenance.len(), 1);
    assert_eq!(receipts[0].provenance[0].operation, "RenameDeclaration");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_graft_write_moves_task_between_modules() {
    let root = temp_project_dir("project-graft-move-task-module");
    fs::create_dir_all(root.join("src/app")).expect("create project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"
[project]
entry = "app.main"
"#,
    )
    .expect("write manifest");
    let main_source = r#"module app.main

import app.extra

export task helper -> Int {
  return 2
}

task main -> Int {
  return 1
}
"#;
    let extra_source = r#"module app.extra
"#;
    let main_path = root.join("src/app/main.sley");
    let extra_path = root.join("src/app/extra.sley");
    let graft_path = root.join("move_helper_module.json");
    fs::write(&main_path, main_source).expect("write main module");
    fs::write(&extra_path, extra_source).expect("write extra module");
    fs::write(
        &graft_path,
        r#"
{
  "op": "MoveNode",
  "target": "task:app.main.helper",
  "payload": { "parent": "module:app.extra:tasks", "position": 0 }
}
"#,
    )
    .expect("write graft");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["graft", "--json", "--write"])
        .arg(&root)
        .arg(&graft_path)
        .output()
        .expect("run project graft");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        output.status.success(),
        "project writeback should move task between modules; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let outcome: GraftOutcome = serde_json::from_str(&stdout).expect("parse outcome");
    assert_eq!(outcome.status, "accepted");
    assert_eq!(
        fs::read_to_string(&main_path).expect("read main"),
        "module app.main\n\nimport app.extra\n\ntask main -> Int {\n  return 1\n}\n"
    );
    assert_eq!(
        fs::read_to_string(&extra_path).expect("read extra"),
        "module app.extra\n\nexport task helper -> Int {\n  return 2\n}\n"
    );
    let check = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["check", "--json"])
        .arg(&root)
        .output()
        .expect("check moved project");
    assert!(
        check.status.success(),
        "moved project should check; stdout={} stderr={}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    let receipts = read_trace_receipts(&root.join(".sley/trace.jsonl")).expect("read trace");
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].provenance.len(), 1);
    assert_eq!(receipts[0].provenance[0].operation, "MoveNode");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_graft_write_moves_import_between_modules() {
    let root = temp_project_dir("project-graft-move-import-module");
    fs::create_dir_all(root.join("src/app")).expect("create project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"
[project]
entry = "app.main"
"#,
    )
    .expect("write manifest");
    let main_source = r#"module app.main

import app.shared
import app.extra

task main -> Int {
  return 1
}
"#;
    let extra_source = r#"module app.extra
"#;
    let shared_source = r#"module app.shared

export task helper -> Int {
  return 2
}
"#;
    let main_path = root.join("src/app/main.sley");
    let extra_path = root.join("src/app/extra.sley");
    let shared_path = root.join("src/app/shared.sley");
    let graft_path = root.join("move_import_module.json");
    fs::write(&main_path, main_source).expect("write main module");
    fs::write(&extra_path, extra_source).expect("write extra module");
    fs::write(&shared_path, shared_source).expect("write shared module");
    fs::write(
        &graft_path,
        r#"
{
  "op": "MoveNode",
  "target": "import:app.main:app.shared",
  "payload": { "parent": "module:app.extra:imports", "position": 0 }
}
"#,
    )
    .expect("write graft");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["graft", "--json", "--write"])
        .arg(&root)
        .arg(&graft_path)
        .output()
        .expect("run project graft");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        output.status.success(),
        "project writeback should move import between modules; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let outcome: GraftOutcome = serde_json::from_str(&stdout).expect("parse outcome");
    assert_eq!(outcome.status, "accepted");
    assert_eq!(
        fs::read_to_string(&main_path).expect("read main"),
        "module app.main\n\nimport app.extra\n\ntask main -> Int {\n  return 1\n}\n"
    );
    assert_eq!(
        fs::read_to_string(&extra_path).expect("read extra"),
        "module app.extra\n\nimport app.shared\n"
    );
    assert_eq!(
        fs::read_to_string(&shared_path).expect("read shared"),
        "module app.shared\n\nexport task helper -> Int {\n  return 2\n}\n"
    );
    let check = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["check", "--json"])
        .arg(&root)
        .output()
        .expect("check moved project");
    assert!(
        check.status.success(),
        "moved project should check; stdout={} stderr={}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    let receipts = read_trace_receipts(&root.join(".sley/trace.jsonl")).expect("read trace");
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].provenance.len(), 1);
    assert_eq!(receipts[0].provenance[0].operation, "MoveNode");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_graft_write_adds_import_and_moves_task_to_new_module() {
    let root = temp_project_dir("project-graft-import-assisted-move");
    fs::create_dir_all(root.join("src/app")).expect("create project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"
[project]
entry = "app.main"
"#,
    )
    .expect("write manifest");
    let main_source = r#"module app.main

export task helper -> Int {
  return 2
}

task main -> Int {
  return call helper()
}
"#;
    let main_path = root.join("src/app/main.sley");
    let extra_path = root.join("src/app/extra.sley");
    let graft_path = root.join("import_and_move_helper.json");
    fs::write(&main_path, main_source).expect("write main module");
    fs::write(
        &graft_path,
        r#"
{
  "transaction": "graft_import_assisted_move",
  "mode": "all_or_nothing",
  "ops": [
    { "op": "AddImport", "payload": { "module": "app.extra" } },
    {
      "op": "MoveNode",
      "target": "task:app.main.helper",
      "payload": { "parent": "module:app.extra:tasks", "position": 0 }
    }
  ]
}
"#,
    )
    .expect("write graft");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["graft", "--json", "--write"])
        .arg(&root)
        .arg(&graft_path)
        .output()
        .expect("run project graft");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        output.status.success(),
        "project writeback should add import and move task; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let outcome: GraftOutcome = serde_json::from_str(&stdout).expect("parse outcome");
    assert_eq!(outcome.status, "accepted");
    assert_eq!(
        fs::read_to_string(&main_path).expect("read main"),
        "module app.main\n\nimport app.extra\n\ntask main -> Int {\n  return call helper()\n}\n"
    );
    assert_eq!(
        fs::read_to_string(&extra_path).expect("read extra"),
        "module app.extra\n\nexport task helper -> Int {\n  return 2\n}\n"
    );
    let run = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["run", "--json"])
        .arg(&root)
        .output()
        .expect("run moved project");
    assert!(
        run.status.success(),
        "import-assisted moved project should run; stdout={} stderr={}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    let receipts = read_trace_receipts(&root.join(".sley/trace.jsonl")).expect("read trace");
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].provenance.len(), 2);
    assert_eq!(receipts[0].provenance[0].operation, "AddImport");
    assert_eq!(receipts[0].provenance[1].operation, "MoveNode");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_graft_write_rejects_import_without_new_module_declarations() {
    let root = temp_project_dir("project-graft-unknown-import");
    fs::create_dir_all(root.join("src/app")).expect("create project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"
[project]
entry = "app.main"
"#,
    )
    .expect("write manifest");
    let main_source = r#"module app.main

task main -> Int {
  return 1
}
"#;
    let main_path = root.join("src/app/main.sley");
    let graft_path = root.join("add_missing_import.json");
    fs::write(&main_path, main_source).expect("write main module");
    fs::write(
        &graft_path,
        r#"
{
  "op": "AddImport",
  "payload": { "module": "app.missing" }
}
"#,
    )
    .expect("write graft");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["graft", "--json", "--write"])
        .arg(&root)
        .arg(&graft_path)
        .output()
        .expect("run project graft");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        !output.status.success(),
        "project writeback should reject missing imported modules"
    );
    let outcome: GraftOutcome = serde_json::from_str(&stdout).expect("parse outcome");
    assert_eq!(outcome.status, "rejected");
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "PROJECT_WRITEBACK_UNKNOWN_IMPORT"),
        "expected PROJECT_WRITEBACK_UNKNOWN_IMPORT, got {:#?}",
        outcome.diagnostics
    );
    assert_eq!(
        fs::read_to_string(&main_path).expect("read main"),
        main_source
    );
    assert!(!root.join("src/app/missing.sley").exists());
    assert!(!root.join(".sley/trace.jsonl").exists());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_loader_reports_missing_imports() {
    let root = temp_project_dir("missing-imports");
    fs::create_dir_all(root.join("src/app")).expect("create project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"
[project]
entry = "app.main"
"#,
    )
    .expect("write manifest");
    fs::write(
        root.join("src/app/main.sley"),
        r#"
module app.main

import app.missing

task main -> Int {
  return 1
}
"#,
    )
    .expect("write main module");

    let diagnostics = load_project(&root).expect_err("project should not load");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "PROJECT_MODULE_NOT_FOUND"),
        "expected missing module diagnostic, got {diagnostics:#?}"
    );

    let _ = fs::remove_dir_all(root);
}

fn collect_sley_files(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_sley_files_into(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_sley_files_into(root: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_sley_files_into(&path, files)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("sley") {
            files.push(path);
        }
    }
    Ok(())
}

fn load_corpus_manifest(corpus_root: &Path) -> CorpusManifest {
    let path = corpus_root.join("manifest.json");
    let source = fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {path:?}: {error}"));
    serde_json::from_str(&source).unwrap_or_else(|error| panic!("parse {path:?}: {error}"))
}

fn assert_corpus_manifest_matches_files(
    corpus_root: &Path,
    manifest: &CorpusManifest,
    accepted_files: &[PathBuf],
    rejected_files: &[PathBuf],
) {
    assert_eq!(manifest.schema, "sley.conformance.manifest.v0");
    assert_eq!(
        manifest_paths("accepted", &manifest.accepted),
        relative_corpus_paths(corpus_root, accepted_files),
        "accepted corpus manifest must list exactly the accepted fixtures on disk"
    );
    assert_eq!(
        manifest_paths("rejected", &manifest.rejected),
        relative_corpus_paths(corpus_root, rejected_files),
        "rejected corpus manifest must list exactly the rejected fixtures on disk"
    );

    for case in manifest.accepted.iter().chain(manifest.rejected.iter()) {
        assert!(
            !case.covers.is_empty(),
            "corpus case {} must declare at least one coverage tag",
            case.path
        );
    }
}

fn assert_corpus_manifest_has_release_coverage(manifest: &CorpusManifest) {
    let coverage = manifest
        .accepted
        .iter()
        .chain(manifest.rejected.iter())
        .flat_map(|case| case.covers.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();
    let required = [
        "accepted:DatabaseRead",
        "accepted:DatabaseWrite",
        "accepted:Deploy",
        "accepted:FileRead",
        "accepted:FileWrite",
        "accepted:ModelCall",
        "accepted:Network",
        "accepted:SecretRead",
        "accepted:Shell",
        "accepted:Spend",
        "rejected:DatabaseRead",
        "rejected:DatabaseWrite",
        "rejected:Deploy",
        "rejected:FileRead",
        "rejected:FileWrite",
        "rejected:ModelCall",
        "rejected:Network",
        "rejected:SecretRead",
        "rejected:Shell",
        "rejected:Spend",
        "diagnostic:EFFECT_UNAUTHORIZED",
        "diagnostic:MISSING_RETURN",
        "diagnostic:TYPE_MISMATCH",
        "diagnostic:UNKNOWN_IDENTIFIER",
        "formatter:round-trip",
    ];
    for tag in required {
        assert!(
            coverage.contains(tag),
            "corpus manifest is missing required release coverage tag {tag}"
        );
    }
}

fn manifest_paths(section: &str, cases: &[CorpusManifestCase]) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    let prefix = format!("{section}/");
    for case in cases {
        assert!(
            case.path.starts_with(&prefix),
            "corpus manifest path {} must live under {section}/",
            case.path
        );
        assert!(
            paths.insert(case.path.clone()),
            "duplicate corpus manifest path {}",
            case.path
        );
    }
    paths
}

fn relative_corpus_paths(corpus_root: &Path, files: &[PathBuf]) -> BTreeSet<String> {
    files
        .iter()
        .map(|file| {
            file.strip_prefix(corpus_root)
                .unwrap_or_else(|error| panic!("strip corpus root for {file:?}: {error}"))
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect()
}

fn load_cli_smoke_manifest(manifest_root: &Path) -> CliSmokeManifest {
    let path = manifest_root.join("manifest.json");
    let source = fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {path:?}: {error}"));
    serde_json::from_str(&source).unwrap_or_else(|error| panic!("parse {path:?}: {error}"))
}

fn assert_cli_smoke_manifest_is_well_formed(manifest: &CliSmokeManifest) {
    assert_eq!(manifest.schema, "sley.cli_smoke.manifest.v0");
    assert!(
        !manifest.cases.is_empty(),
        "CLI smoke manifest must contain at least one case"
    );
    let mut names = BTreeSet::new();
    for case in &manifest.cases {
        assert!(
            names.insert(case.name.as_str()),
            "duplicate CLI smoke name {}",
            case.name
        );
        assert!(
            !case.args.is_empty(),
            "CLI smoke {} must declare command arguments",
            case.name
        );
        assert!(
            !case.covers.is_empty(),
            "CLI smoke {} must declare coverage tags",
            case.name
        );
        for expectation in &case.expect.stdout_json {
            assert!(
                expectation.pointer.is_empty() || expectation.pointer.starts_with('/'),
                "CLI smoke {} JSON pointer {} must be a valid document or absolute pointer",
                case.name,
                expectation.pointer
            );
        }
    }
}

fn assert_cli_smoke_manifest_has_release_coverage(manifest: &CliSmokeManifest) {
    let coverage = manifest
        .cases
        .iter()
        .flat_map(|case| case.covers.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();
    let required = [
        "cli:parse",
        "cli:new",
        "cli:doctor",
        "cli:plan",
        "cli:verify",
        "cli:format",
        "cli:check",
        "cli:run",
        "cli:ast",
        "cli:graph",
        "cli:graph-slice",
        "cli:query",
        "cli:lint",
        "cli:trace",
        "cli:seal",
        "cli:zjx",
        "cli:graft-dry-run",
        "diagnostic:MISSING_RETURN",
        "graft:templates:lint-declaration-delete",
        "graft:templates:lint-declaration-target",
        "graft:templates:replace-expression",
        "graft:transactions:lint-declaration-cleanup",
        "graph-slice:replace-affordances",
        "lint:unused_declared_effect",
        "lint:unused_import",
        "lint:unused_take",
        "lint:unused_private_type",
        "lint:unused_private_effect",
        "lint:raw_host_adapter",
        "host:DatabaseRead",
        "host:DatabaseWrite",
        "host:Deploy",
        "host:FileRead",
        "host:FileWrite",
        "host:ModelCall",
        "host:Network",
        "host:SecretRead",
        "host:Shell",
        "host:Spend",
        "json:sley.ast.program.v0",
        "json:sley.diagnostics.report.v0",
        "json:sley.graft.outcome.v0",
        "json:sley.symbol_graph.v0",
        "json:sley.symbol_graph.slice.v0",
        "json:sley.query.report.v0",
        "json:sley.lint.report.v0",
        "json:sley.project.scaffold.v0",
        "json:sley.doctor.report.v0",
        "json:sley.edit_plan.report.v0",
        "json:sley.verify.report.v0",
        "json:sley.trace.seal.v0",
        "json:sley.zjx.envelope.v0",
    ];
    for tag in required {
        assert!(
            coverage.contains(tag),
            "CLI smoke manifest is missing required release coverage tag {tag}"
        );
    }
}

fn expand_cli_smoke_args(args: &[String], repo_root: &Path, tmp_root: &Path) -> Vec<String> {
    let repo = repo_root.to_string_lossy();
    let tmp = tmp_root.to_string_lossy();
    args.iter()
        .map(|arg| arg.replace("{repo}", &repo).replace("{tmp}", &tmp))
        .collect()
}

fn temp_project_dir(name: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("sley-{name}-{}-{timestamp}", std::process::id()))
}

fn write_unused_import_project(root: &Path) {
    fs::create_dir_all(root.join("src/app")).expect("create project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"
[project]
name = "unused-import"
root = "src"
entry = "app.main"
"#,
    )
    .expect("write manifest");
    fs::write(
        root.join("src/app/main.sley"),
        r#"
module app.main

import app.used
import app.types as t
import app.io as io
import app.stale

task main -> t.User uses io.Read {
  bind doubled = call used.double(21)
  bind name = call io.name()

  return t.User { name: name }
}
"#,
    )
    .expect("write main module");
    fs::write(
        root.join("src/app/used.sley"),
        r#"
module app.used

export task double -> Int {
  take value: Int

  return value * 2
}
"#,
    )
    .expect("write used module");
    fs::write(
        root.join("src/app/types.sley"),
        r#"
module app.types

export type User = {
  slot name: Text
}
"#,
    )
    .expect("write types module");
    fs::write(
        root.join("src/app/io.sley"),
        r#"
module app.io

export effect Read

export task name -> Text uses Read {
  return "Ada"
}
"#,
    )
    .expect("write io module");
    fs::write(
        root.join("src/app/stale.sley"),
        r#"
module app.stale

export task noop -> Int {
  return 0
}
"#,
    )
    .expect("write stale module");
}

fn sley_string(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

fn assert_rejected_graft(source: &str, graft_source: &str, diagnostic_id: &str) -> GraftOutcome {
    let program = parse_program(source).expect("parse source");
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));
    assert_eq!(outcome.status, "rejected");
    assert_eq!(outcome.schema, GRAFT_OUTCOME_SCHEMA);
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == diagnostic_id),
        "expected {diagnostic_id}, got {:#?}",
        outcome.diagnostics
    );
    outcome
}

fn assert_json_snapshot<T: serde::Serialize>(actual: &T, expected_json: &str) {
    let actual = serde_json::to_value(actual).expect("serialize actual json");
    let expected: serde_json::Value = serde_json::from_str(expected_json).expect("parse snapshot");
    assert_eq!(actual, expected);
}

fn db_row<const N: usize>(fields: [(&str, Value); N]) -> BTreeMap<String, Value> {
    fields
        .into_iter()
        .map(|(name, value)| (name.to_string(), value))
        .collect()
}

fn assert_error_record(value: &Value, code: &str, message_fragment: &str) {
    let Value::Record(fields) = value else {
        panic!("expected error record, got {value:#?}");
    };
    assert_eq!(fields.get("code"), Some(&Value::Text(code.to_string())));
    let Some(Value::Text(message)) = fields.get("message") else {
        panic!("expected error record message, got {fields:#?}");
    };
    assert!(
        message.contains(message_fragment),
        "expected error message containing {message_fragment:?}, got {message:?}"
    );
}

fn assert_schema_file(schema_json: &str, schema_id: &str) {
    let schema: serde_json::Value = serde_json::from_str(schema_json).expect("parse schema");
    assert_eq!(
        schema.get("$id").and_then(serde_json::Value::as_str),
        Some(schema_id)
    );
}

fn assert_schema_enum(
    defs: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    expected: &[&str],
) {
    let values = defs
        .get(key)
        .and_then(|schema| schema.get("enum"))
        .and_then(serde_json::Value::as_array)
        .unwrap_or_else(|| panic!("schema definition {key} does not expose enum"));
    let actual = values
        .iter()
        .map(|value| value.as_str().expect("string enum value"))
        .collect::<Vec<_>>();
    assert_eq!(actual.as_slice(), expected);
}

fn assert_schema_one_of_refs(
    defs: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    expected: &[&str],
) {
    let refs = defs
        .get(key)
        .and_then(|schema| schema.get("oneOf"))
        .and_then(serde_json::Value::as_array)
        .unwrap_or_else(|| panic!("schema definition {key} does not expose oneOf"));
    let actual = refs
        .iter()
        .map(|entry| {
            entry
                .get("$ref")
                .and_then(serde_json::Value::as_str)
                .and_then(|reference| reference.strip_prefix("#/$defs/"))
                .unwrap_or_else(|| panic!("schema definition {key} has non-local ref"))
        })
        .collect::<Vec<_>>();
    assert_eq!(actual.as_slice(), expected);
}

fn assert_has_repair_hint(diagnostics: &[sley::diagnostics::Diagnostic], id: &str, kind: &str) {
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == id)
        .unwrap_or_else(|| panic!("missing diagnostic {id}; got {diagnostics:#?}"));
    assert!(
        diagnostic.repair_hints.iter().any(|hint| hint.kind == kind),
        "expected repair hint {kind} on {id}; got {diagnostic:#?}"
    );
}

fn find_repair_hint<'a>(
    diagnostics: &'a [sley::diagnostics::Diagnostic],
    id: &str,
    kind: &str,
) -> &'a sley::diagnostics::RepairHint {
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == id)
        .unwrap_or_else(|| panic!("missing diagnostic {id}; got {diagnostics:#?}"));
    diagnostic
        .repair_hints
        .iter()
        .find(|hint| hint.kind == kind)
        .unwrap_or_else(|| panic!("missing repair hint {kind} on {id}; got {diagnostic:#?}"))
}

fn assert_replace_expression_hint_source(
    diagnostics: &[sley::diagnostics::Diagnostic],
    id: &str,
    expected_source: &str,
) -> String {
    let hint = find_repair_hint(diagnostics, id, "replace_expression");
    assert_replace_expression_hint_json(hint, expected_source)
}

fn assert_replace_expression_hint_json(
    hint: &sley::diagnostics::RepairHint,
    expected_source: &str,
) -> String {
    let target = hint.target.as_deref().expect("replace_expression target");
    let replacement: serde_json::Value = serde_json::from_str(
        hint.replacement
            .as_deref()
            .expect("replace_expression replacement"),
    )
    .expect("replace_expression replacement is JSON");
    assert_eq!(
        replacement.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        replacement.pointer("/target"),
        Some(&serde_json::json!(target))
    );
    assert_eq!(
        replacement.pointer("/payload/source"),
        Some(&serde_json::json!(expected_source))
    );
    target.to_string()
}

fn replace_expression_hint_sources(diagnostic: &sley::diagnostics::Diagnostic) -> Vec<String> {
    diagnostic
        .repair_hints
        .iter()
        .filter(|hint| hint.kind == "replace_expression")
        .map(|hint| {
            let replacement: serde_json::Value = serde_json::from_str(
                hint.replacement
                    .as_deref()
                    .expect("replace_expression replacement"),
            )
            .expect("replace_expression replacement is JSON");
            assert_eq!(
                replacement.pointer("/op"),
                Some(&serde_json::json!("ReplaceExpression"))
            );
            replacement
                .pointer("/payload/source")
                .and_then(serde_json::Value::as_str)
                .expect("replace_expression source")
                .to_string()
        })
        .collect()
}
