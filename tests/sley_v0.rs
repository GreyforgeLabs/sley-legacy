use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};

use sley::ast::{AST_NODE_SCHEMA, AST_PROGRAM_SCHEMA, ExprKind, ProvenanceRecord, StatementKind};
use sley::authority::{host_effect_contracts, host_effects_for_callee};
use sley::checker::{check_program, has_errors};
use sley::deploy::{
    DEPLOY_ARTIFACTS_SCHEMA, DEPLOY_REPORT_SCHEMA, DeployArtifacts, build_deploy_artifact_manifest,
    build_deploy_report,
};
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
use sley::runtime::{
    RUN_REPORT_SCHEMA, RunReport, RuntimeGates, Value, build_run_report, run_main,
    run_main_with_gates,
};
use sley::scaffold::{
    PROJECT_SCAFFOLD_SCHEMA, ProjectScaffoldReport, ProjectScaffoldSummary, ScaffoldFile,
    ScaffoldNextAction, ScaffoldOptions, ScaffoldTemplate, scaffold_project,
};
use sley::symbols::{
    SYMBOL_GRAPH_SCHEMA, SYMBOL_GRAPH_SLICE_SCHEMA, build_symbol_graph, slice_symbol_graph,
};
use sley::trace::{
    TRACE_RECEIPT_SCHEMA, TRACE_REPORT_SCHEMA, TRACE_SEAL_SCHEMA, TraceReceipt,
    append_trace_receipt, build_trace_receipt, build_trace_report, build_trace_seal,
    content_digest, read_trace_receipts,
};
use sley::verify::{VERIFY_REPORT_SCHEMA, build_verify_report};
use sley::zjx::build_zjx_envelope;

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
    #[serde(default)]
    setup_files: Vec<CliSmokeSetupFile>,
    args: Vec<String>,
    covers: Vec<String>,
    expect: CliSmokeExpectation,
}

#[derive(Debug, serde::Deserialize)]
struct CliSmokeSetupFile {
    path: String,
    content: String,
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

fn assert_cli_run_value(stdout: &[u8], expected: Value) {
    let report: RunReport = serde_json::from_slice(stdout).expect("parse run report");
    assert_eq!(report.schema, RUN_REPORT_SCHEMA);
    assert_eq!(report.status, "passed");
    assert!(report.diagnostics.is_empty());
    assert_eq!(report.value, expected);
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
fn ast_node_report_targets_nested_nodes() {
    let source = r#"
module app.ast

task main -> Int {
  bind value = 40

  return value + 2
}
"#;
    let program = parse_program(source).expect("parse source");

    let task = program
        .ast_node_report("task:app.ast.main")
        .expect("task AST node report");
    assert_eq!(task.schema, AST_NODE_SCHEMA);
    assert_eq!(task.node_kind, "task");
    assert_eq!(task.id, "task:app.ast.main");
    assert_eq!(task.module, "app.ast");
    assert_eq!(task.parent.as_deref(), Some("module:app.ast"));
    assert_eq!(
        task.value.pointer("/body/statements/1/id"),
        Some(&serde_json::json!("block:task:app.ast.main:stmt:1"))
    );

    let statement = program
        .ast_node_report("block:task:app.ast.main:stmt:0")
        .expect("statement AST node report");
    assert_eq!(statement.node_kind, "statement");
    assert_eq!(statement.parent.as_deref(), Some("block:task:app.ast.main"));
    assert_eq!(
        statement.value.pointer("/name"),
        Some(&serde_json::json!("value"))
    );

    let expression = program
        .ast_node_report("block:task:app.ast.main:stmt:1:expr:right")
        .expect("nested expression AST node report");
    assert_eq!(expression.node_kind, "expression");
    assert_eq!(
        expression.parent.as_deref(),
        Some("block:task:app.ast.main:stmt:1:expr")
    );
    assert_eq!(
        expression.value.pointer("/expr_kind"),
        Some(&serde_json::json!("IntLiteral"))
    );
    assert!(
        program
            .ast_node_report("block:task:app.ast.main:stmt:9")
            .is_none()
    );
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
        write_cli_smoke_setup_files(case, &repo_root, &tmp_root);
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
    let report: ProjectScaffoldReport =
        serde_json::from_str(&stdout).expect("parse scaffold report");
    assert_eq!(report.next_actions.len(), report.next_commands.len());
    assert_eq!(report.next_actions[5].kind, "verify_seeded_deploy");
    assert_eq!(report.next_actions[6].kind, "run_seeded_deploy");
    assert_eq!(report.next_actions[7].kind, "prepare_deploy_package");
    assert_eq!(report.next_actions[8].kind, "seal_project");
    assert_eq!(report.next_actions[9].kind, "package_project");
    for (action, command) in report.next_actions.iter().zip(&report.next_commands) {
        assert_eq!(
            &action.command, command,
            "next action command should mirror legacy next_commands"
        );
        assert_eq!(command.first().map(String::as_str), Some("sley"));
        let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
            .current_dir(&root)
            .args(&command[1..])
            .output()
            .unwrap_or_else(|error| panic!("run scaffold next action {}: {error}", action.kind));
        let stdout = String::from_utf8(output.stdout)
            .unwrap_or_else(|error| panic!("next action {} stdout utf8: {error}", action.kind));
        let stderr = String::from_utf8(output.stderr)
            .unwrap_or_else(|error| panic!("next action {} stderr utf8: {error}", action.kind));
        assert!(
            output.status.success(),
            "next action {} should succeed\nstdout: {stdout}\nstderr: {stderr}",
            action.kind
        );
        if action.kind == "run_seeded_deploy" {
            assert_cli_run_value(
                stdout.as_bytes(),
                Value::Ok(Box::new(Value::Text("staged".to_string()))),
            );
        }
        if action.kind == "prepare_deploy_package" {
            let value: serde_json::Value =
                serde_json::from_str(&stdout).expect("parse deploy dry-run JSON");
            assert_eq!(
                value.pointer("/schema"),
                Some(&serde_json::json!(DEPLOY_REPORT_SCHEMA))
            );
            assert_eq!(value.pointer("/status"), Some(&serde_json::json!("ready")));
            assert_eq!(
                value.pointer("/policy/live_deploy_allowed"),
                Some(&serde_json::json!(false))
            );
            assert_eq!(
                value.pointer("/artifacts/report"),
                Some(&serde_json::json!(".sley/deploy/deploy-report.json"))
            );
            assert!(root.join(".sley/deploy/deploy-report.json").exists());
            assert!(root.join(".sley/deploy/seal.json").exists());
            assert!(root.join(".sley/deploy/zjx-envelope.json").exists());
            assert!(root.join(".sley/deploy/manifest.json").exists());
        }
    }

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
fn project_scaffold_creates_checked_agent_project() {
    let root = temp_project_dir("new-agent");
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "new",
            "--json",
            "--template",
            "agent",
            "--name",
            "agent-app",
            "--module",
            "app.main",
        ])
        .arg(&root)
        .output()
        .expect("run sley new agent");
    let stdout = String::from_utf8(output.stdout).expect("agent stdout utf8");
    let stderr = String::from_utf8(output.stderr).expect("agent stderr utf8");
    assert!(
        output.status.success(),
        "stdout: {stdout}\nstderr: {stderr}"
    );
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("parse agent JSON");
    assert_eq!(
        json.pointer("/schema"),
        Some(&serde_json::json!(PROJECT_SCAFFOLD_SCHEMA))
    );
    assert_eq!(
        json.pointer("/project/template"),
        Some(&serde_json::json!("agent"))
    );
    assert!(root.join("sley.toml").exists());
    assert!(root.join("README.md").exists());
    assert!(root.join("src/app/main.sley").exists());

    let project = load_project(&root).expect("load scaffolded agent project");
    let diagnostics = check_program(&project.program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected scaffold diagnostics: {diagnostics:#?}"
    );
    let report: ProjectScaffoldReport =
        serde_json::from_str(&stdout).expect("parse agent scaffold report");
    assert_eq!(report.next_actions.len(), report.next_commands.len());
    assert_eq!(report.next_actions[5].kind, "verify_seeded_agent");
    assert_eq!(report.next_actions[6].kind, "ci_verify_seeded_agent");
    assert_eq!(report.next_actions[7].kind, "run_seeded_agent");
    assert_eq!(report.next_actions[8].kind, "ci_run_seeded_agent");
    assert_eq!(report.next_actions[9].kind, "prepare_deploy_package");
    assert_eq!(report.next_actions[10].kind, "ci_deploy_package");
    assert_eq!(report.next_actions[11].kind, "seal_project");
    assert_eq!(report.next_actions[12].kind, "package_project");

    for (action, command) in report.next_actions.iter().zip(&report.next_commands) {
        assert_eq!(
            &action.command, command,
            "next action command should mirror legacy next_commands"
        );
        let Some(binary) = command.first().map(String::as_str) else {
            panic!("next action {} command was empty", action.kind);
        };
        let executable = match binary {
            "sley" => env!("CARGO_BIN_EXE_sley"),
            "sley-ci" => env!("CARGO_BIN_EXE_sley-ci"),
            other => panic!("unexpected scaffold command binary {other:?}"),
        };
        let output = ProcessCommand::new(executable)
            .current_dir(&root)
            .args(&command[1..])
            .output()
            .unwrap_or_else(|error| panic!("run agent next action {}: {error}", action.kind));
        let stdout = String::from_utf8(output.stdout).unwrap_or_else(|error| {
            panic!("agent next action {} stdout utf8: {error}", action.kind)
        });
        let stderr = String::from_utf8(output.stderr).unwrap_or_else(|error| {
            panic!("agent next action {} stderr utf8: {error}", action.kind)
        });
        assert!(
            output.status.success(),
            "agent next action {} should succeed\nstdout: {stdout}\nstderr: {stderr}",
            action.kind
        );
        if action.kind == "ci_verify_seeded_agent" {
            let value: serde_json::Value =
                serde_json::from_str(&stdout).expect("parse sley-ci scaffold JSON");
            assert_eq!(
                value.pointer("/schema"),
                Some(&serde_json::json!("sley.ci.report.v0"))
            );
            assert_eq!(value.pointer("/status"), Some(&serde_json::json!("passed")));
        }
        if action.kind == "ci_deploy_package" {
            let value: serde_json::Value =
                serde_json::from_str(&stdout).expect("parse sley-ci deploy JSON");
            assert_eq!(
                value.pointer("/schema"),
                Some(&serde_json::json!("sley.ci.report.v0"))
            );
            assert_eq!(
                value.pointer("/command"),
                Some(&serde_json::json!("deploy"))
            );
            assert_eq!(
                value.pointer("/steps/0/stdout_schema"),
                Some(&serde_json::json!(DEPLOY_REPORT_SCHEMA))
            );
            assert!(root.join(".sley/ci-deploy/deploy-report.json").exists());
            assert!(root.join(".sley/ci-deploy/seal.json").exists());
            assert!(root.join(".sley/ci-deploy/zjx-envelope.json").exists());
            assert!(root.join(".sley/ci-deploy/manifest.json").exists());
        }
        if action.kind == "ci_run_seeded_agent" {
            let value: serde_json::Value =
                serde_json::from_str(&stdout).expect("parse sley-ci run JSON");
            assert_eq!(
                value.pointer("/schema"),
                Some(&serde_json::json!("sley.ci.report.v0"))
            );
            assert_eq!(value.pointer("/command"), Some(&serde_json::json!("run")));
            assert_eq!(
                value.pointer("/steps/0/stdout_schema"),
                Some(&serde_json::json!(RUN_REPORT_SCHEMA))
            );
        }
        if action.kind == "run_seeded_agent" {
            assert_cli_run_value(
                stdout.as_bytes(),
                Value::Ok(Box::new(Value::Text(
                    "profile ready | plan approved | staged".to_string(),
                ))),
            );
        }
        if action.kind == "prepare_deploy_package" {
            let value: serde_json::Value =
                serde_json::from_str(&stdout).expect("parse agent deploy dry-run JSON");
            assert_eq!(
                value.pointer("/schema"),
                Some(&serde_json::json!(DEPLOY_REPORT_SCHEMA))
            );
            assert_eq!(value.pointer("/status"), Some(&serde_json::json!("ready")));
            assert_eq!(
                value.pointer("/policy/provider_calls"),
                Some(&serde_json::json!(false))
            );
            assert_eq!(
                value.pointer("/artifacts/report"),
                Some(&serde_json::json!(".sley/deploy/deploy-report.json"))
            );
            assert!(root.join(".sley/deploy/deploy-report.json").exists());
            assert!(root.join(".sley/deploy/seal.json").exists());
            assert!(root.join(".sley/deploy/zjx-envelope.json").exists());
            assert!(root.join(".sley/deploy/manifest.json").exists());
        }
    }

    let _ = fs::remove_dir_all(root);
}

#[test]
fn deploy_dry_run_reports_verified_package_without_live_mutation() {
    let missing_dry_run = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["deploy", "--json", "examples/hello.sley"])
        .output()
        .expect("run deploy without dry-run");
    assert!(
        !missing_dry_run.status.success(),
        "deploy should require --dry-run in v0"
    );
    assert!(
        String::from_utf8_lossy(&missing_dry_run.stderr).contains("--dry-run"),
        "missing dry-run stderr should explain the required flag"
    );

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["deploy", "--json", "--dry-run", "examples/hello.sley"])
        .output()
        .expect("run deploy dry-run");
    let stdout = String::from_utf8(output.stdout).expect("deploy stdout utf8");
    let stderr = String::from_utf8(output.stderr).expect("deploy stderr utf8");
    assert!(
        output.status.success(),
        "stdout: {stdout}\nstderr: {stderr}"
    );
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("parse deploy JSON");
    assert_eq!(
        json.pointer("/schema"),
        Some(&serde_json::json!(DEPLOY_REPORT_SCHEMA))
    );
    assert_eq!(json.pointer("/status"), Some(&serde_json::json!("ready")));
    assert_eq!(json.pointer("/mode"), Some(&serde_json::json!("dry_run")));
    assert_eq!(
        json.pointer("/verify/schema"),
        Some(&serde_json::json!(VERIFY_REPORT_SCHEMA))
    );
    assert_eq!(
        json.pointer("/seal/schema"),
        Some(&serde_json::json!(TRACE_SEAL_SCHEMA))
    );
    assert_eq!(
        json.pointer("/package/source_schema"),
        Some(&serde_json::json!("sley.zjx.envelope.v0"))
    );
    assert_eq!(
        json.pointer("/policy/external_mutations"),
        Some(&serde_json::json!(false))
    );
    assert_eq!(
        json.pointer("/policy/requires_operator_approval"),
        Some(&serde_json::json!(true))
    );

    let artifacts_root = temp_project_dir("deploy-artifacts");
    let artifacts_dir = artifacts_root.join("package");
    let artifacts_dir_arg = sley_string(&artifacts_dir);
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "deploy",
            "--json",
            "--dry-run",
            "--artifacts-dir",
            &artifacts_dir_arg,
            "examples/hello.sley",
        ])
        .output()
        .expect("run deploy dry-run with artifacts");
    let stdout = String::from_utf8(output.stdout).expect("deploy artifacts stdout utf8");
    let stderr = String::from_utf8(output.stderr).expect("deploy artifacts stderr utf8");
    assert!(
        output.status.success(),
        "stdout: {stdout}\nstderr: {stderr}"
    );
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("parse deploy artifacts JSON");
    assert_eq!(
        json.pointer("/artifacts/report"),
        Some(&serde_json::json!(format!(
            "{}/deploy-report.json",
            artifacts_dir_arg
        )))
    );
    assert_eq!(
        json.pointer("/artifacts/manifest"),
        Some(&serde_json::json!(format!(
            "{}/manifest.json",
            artifacts_dir_arg
        )))
    );
    assert_eq!(
        json.pointer("/next_actions/2/kind"),
        Some(&serde_json::json!("inspect_deploy_artifacts"))
    );
    assert_eq!(
        json.pointer("/next_actions/2/command"),
        Some(&serde_json::json!([
            "sley-contract",
            "inspect-deploy-artifacts",
            artifacts_dir_arg.clone(),
            "--schemas",
            "docs/schemas",
            "--json"
        ]))
    );
    assert!(artifacts_dir.join("deploy-report.json").exists());
    assert!(artifacts_dir.join("seal.json").exists());
    assert!(artifacts_dir.join("zjx-envelope.json").exists());
    assert!(artifacts_dir.join("manifest.json").exists());
    let package_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(artifacts_dir.join("zjx-envelope.json")).unwrap())
            .expect("parse deploy package artifact");
    assert_eq!(
        package_json.pointer("/schema"),
        Some(&serde_json::json!("sley.zjx.envelope.v0"))
    );
    let report_json: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(artifacts_dir.join("deploy-report.json")).unwrap(),
    )
    .expect("parse deploy report artifact");
    assert_eq!(
        report_json.pointer("/schema"),
        Some(&serde_json::json!(DEPLOY_REPORT_SCHEMA))
    );
    let manifest_source =
        fs::read_to_string(artifacts_dir.join("manifest.json")).expect("read deploy manifest");
    let manifest_json: serde_json::Value =
        serde_json::from_str(&manifest_source).expect("parse deploy manifest artifact");
    assert_eq!(
        manifest_json.pointer("/schema"),
        Some(&serde_json::json!(DEPLOY_ARTIFACTS_SCHEMA))
    );
    assert_eq!(
        manifest_json.pointer("/files/report/digest"),
        Some(&serde_json::json!(content_digest(
            fs::read(artifacts_dir.join("deploy-report.json"))
                .expect("read deploy report artifact")
                .as_slice()
        )))
    );
    assert_eq!(
        manifest_json.pointer("/files/seal/digest"),
        Some(&serde_json::json!(content_digest(
            fs::read(artifacts_dir.join("seal.json"))
                .expect("read seal artifact")
                .as_slice()
        )))
    );
    assert_eq!(
        manifest_json.pointer("/files/package/digest"),
        Some(&serde_json::json!(content_digest(
            fs::read(artifacts_dir.join("zjx-envelope.json"))
                .expect("read package artifact")
                .as_slice()
        )))
    );
    let _ = fs::remove_dir_all(artifacts_root);
}

#[test]
fn agent_deploy_pipeline_artifacts_pass_contract_inspection() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let artifact_root = temp_project_dir("agent-deploy-contract-artifacts");
    let artifact_root_arg = sley_string(&artifact_root);

    let deploy = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .current_dir(&repo_root)
        .args([
            "deploy",
            "--json",
            "--dry-run",
            "--artifacts-dir",
            &artifact_root_arg,
            "--cap",
            "SecretRead",
            "--secret",
            "api_key",
            "redacted",
            "--cap",
            "Network",
            "--http-text",
            "https://example.test/profile",
            "profile ready",
            "--cap",
            "ModelCall",
            "--model-output",
            "deploy-plan",
            "plan approved",
            "--cap",
            "Deploy",
            "--deploy-result",
            "staging",
            "staged",
            "examples/agent_deploy_pipeline.sley",
        ])
        .output()
        .expect("build agent deploy pipeline artifacts");
    let deploy_stdout = String::from_utf8(deploy.stdout).expect("agent deploy stdout utf8");
    let deploy_stderr = String::from_utf8(deploy.stderr).expect("agent deploy stderr utf8");
    assert!(
        deploy.status.success(),
        "stdout: {deploy_stdout}\nstderr: {deploy_stderr}"
    );
    let deploy_json: serde_json::Value =
        serde_json::from_str(&deploy_stdout).expect("parse agent deploy JSON");
    assert_eq!(
        deploy_json.pointer("/schema"),
        Some(&serde_json::json!(DEPLOY_REPORT_SCHEMA))
    );
    assert_eq!(
        deploy_json.pointer("/status"),
        Some(&serde_json::json!("ready"))
    );
    assert_eq!(
        deploy_json.pointer("/verify/summary/call_count"),
        Some(&serde_json::json!(7))
    );
    assert_eq!(
        deploy_json.pointer("/verify/runtime/value/value/value"),
        Some(&serde_json::json!("profile ready | plan approved | staged"))
    );
    assert_eq!(
        deploy_json.pointer("/artifacts/manifest"),
        Some(&serde_json::json!(format!(
            "{}/manifest.json",
            artifact_root_arg
        )))
    );
    assert_eq!(
        deploy_json.pointer("/next_actions/2/kind"),
        Some(&serde_json::json!("inspect_deploy_artifacts"))
    );
    assert_eq!(
        deploy_json.pointer("/next_actions/2/command/2"),
        Some(&serde_json::json!(artifact_root_arg.clone()))
    );

    let artifact_check = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-contract"))
        .current_dir(&repo_root)
        .args([
            "inspect-deploy-artifacts",
            &artifact_root_arg,
            "--schemas",
            "docs/schemas",
            "--json",
        ])
        .output()
        .expect("inspect agent deploy pipeline artifacts");
    let artifact_stdout =
        String::from_utf8(artifact_check.stdout).expect("artifact check stdout utf8");
    let artifact_stderr =
        String::from_utf8(artifact_check.stderr).expect("artifact check stderr utf8");
    assert!(
        artifact_check.status.success(),
        "stdout: {artifact_stdout}\nstderr: {artifact_stderr}"
    );
    let artifact_json: serde_json::Value =
        serde_json::from_str(&artifact_stdout).expect("parse artifact check JSON");
    assert_eq!(
        artifact_json.pointer("/schema"),
        Some(&serde_json::json!("sley.deploy.artifact_check.v0"))
    );
    assert_eq!(
        artifact_json.pointer("/status"),
        Some(&serde_json::json!("passed"))
    );
    assert_eq!(
        artifact_json.pointer("/summary/file_count"),
        Some(&serde_json::json!(3))
    );
    assert_eq!(
        artifact_json.pointer("/summary/issue_count"),
        Some(&serde_json::json!(0))
    );
    assert_eq!(
        artifact_json.pointer("/files/0/actual_schema"),
        Some(&serde_json::json!(DEPLOY_REPORT_SCHEMA))
    );
    assert_eq!(
        artifact_json.pointer("/files/1/actual_schema"),
        Some(&serde_json::json!(TRACE_SEAL_SCHEMA))
    );
    assert_eq!(
        artifact_json.pointer("/files/2/actual_schema"),
        Some(&serde_json::json!("sley.zjx.envelope.v0"))
    );

    let _ = fs::remove_dir_all(artifact_root);
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
    assert_eq!(
        ready_json.pointer("/next_actions/1/kind"),
        Some(&serde_json::json!("inspect_calls"))
    );
    assert_eq!(
        ready_json.pointer("/next_actions/1/command"),
        Some(&serde_json::json!([
            "sley",
            "query",
            "--json",
            "--kind",
            "calls",
            "examples/project"
        ]))
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
    assert_eq!(
        warnings_json.pointer("/next_actions/1/kind"),
        Some(&serde_json::json!("plan_lint_repairs"))
    );
    assert_eq!(
        warnings_json.pointer("/next_actions/1/command"),
        Some(&serde_json::json!([
            "sley",
            "plan",
            "--json",
            "--graft-templates",
            warning_file.display().to_string()
        ]))
    );
    assert_eq!(
        warnings_json.pointer("/next_actions/2/kind"),
        Some(&serde_json::json!("preview_lint_repair"))
    );
    assert_eq!(
        warnings_json.pointer("/next_actions/2/command"),
        Some(&serde_json::json!([
            "sley",
            "fix",
            "--json",
            "--kind",
            "delete_unused_private_task",
            "--template-surface",
            "task:app.warning.orphan",
            "--dry-run",
            warning_file.display().to_string()
        ]))
    );
    assert_eq!(
        warnings_json.pointer("/next_actions/2/write_command"),
        Some(&serde_json::json!([
            "sley",
            "fix",
            "--json",
            "--kind",
            "delete_unused_private_task",
            "--template-surface",
            "task:app.warning.orphan",
            "--write",
            warning_file.display().to_string()
        ]))
    );
    assert!(
        warnings_json
            .pointer("/next_actions")
            .and_then(serde_json::Value::as_array)
            .expect("warning next actions")
            .iter()
            .all(|action| action.pointer("/kind") != Some(&serde_json::json!("inspect_calls"))),
        "call-free warning reports should not suggest call-row inspection"
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
    assert_eq!(
        denied_json.pointer("/next_actions/1/kind"),
        Some(&serde_json::json!("plan_lint_repairs"))
    );
    assert_eq!(
        denied_json.pointer("/next_actions/2/kind"),
        Some(&serde_json::json!("preview_lint_repair"))
    );

    let ambiguous = parse_program(
        r#"
module app.ambiguous

task main -> Int {
  return 1
}

task first -> Int {
  return 2
}

task second -> Int {
  return 3
}
"#,
    )
    .expect("parse ambiguous warnings");
    let ambiguous_report = build_doctor_report("app.ambiguous", Ok(ambiguous), false);
    assert_eq!(ambiguous_report.summary.lint_finding_count, 2);
    assert!(
        !ambiguous_report
            .next_actions
            .iter()
            .any(|action| action.kind == "preview_lint_repair"),
        "ambiguous lint repair reports should require planning first"
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
    assert_eq!(report.next_actions[2].kind, "inspect_calls");
    assert_eq!(
        report.next_actions[2].command,
        vec!["sley", "query", "--json", "--kind", "calls", "app.plan"]
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
            module_name_hint: None,
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
    let insert_template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "insert_statement")
        .expect("insert statement template");
    assert_eq!(insert_template.surface, "task:app.plan.main");
    assert_eq!(
        insert_template.operation.pointer("/op"),
        Some(&serde_json::json!("InsertStatement"))
    );
    assert_eq!(
        insert_template.operation.pointer("/target"),
        Some(&serde_json::json!("block:task:app.plan.main"))
    );
    assert_eq!(
        insert_template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("forge { }"))
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
fn edit_plan_replace_task_body_template_uses_float_default_return() {
    let source = r#"
module app.plan

task main -> Float {
  return 1.5
}
"#;
    let program = parse_program(source).expect("parse float template plan fixture");
    let report = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("task:app.plan.main".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "ready");
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "replace_task_body")
        .expect("replace task body template");
    assert_eq!(
        template.operation.pointer("/payload/statements/0"),
        Some(&serde_json::json!("return 0.0"))
    );
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse float replace template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:float-replace-task-body-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted = parse_program(&outcome.source.expect("grafted source")).expect("parse grafted");
    assert_eq!(run_main(&grafted), Ok(Value::Float(0.0)));
}

#[test]
fn edit_plan_graft_templates_can_target_program_declaration_surface() {
    let source = r#"
module app.plan

task main -> Int {
  return 1
}
"#;
    let program = parse_program(source).expect("parse program declaration surface fixture");
    let report = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("program".to_string()),
            module_name_hint: None,
        },
    );

    assert_eq!(report.status, "ready");
    assert_eq!(report.graft_templates.len(), 4);
    assert!(report.transaction_templates.is_empty());

    let add_task = &report.graft_templates[0];
    assert_eq!(add_task.kind, "add_task");
    assert_eq!(add_task.surface, "program");
    assert_eq!(
        add_task.operation.pointer("/op"),
        Some(&serde_json::json!("AddTask"))
    );
    assert_eq!(
        add_task.operation.pointer("/payload/source"),
        Some(&serde_json::json!(
            "module app.plan\n\ntask new_task -> Int {\n  return 0\n}"
        ))
    );
    assert_eq!(
        add_task.editable_json_pointers,
        vec!["/payload/source".to_string()]
    );

    let add_type = &report.graft_templates[1];
    assert_eq!(add_type.kind, "add_type_declaration");
    assert_eq!(
        add_type.operation.pointer("/op"),
        Some(&serde_json::json!("AddTypeDeclaration"))
    );
    assert_eq!(
        add_type.operation.pointer("/payload/source"),
        Some(&serde_json::json!(
            "module app.plan\n\ntype NewRecord = {\n  slot value: Text\n}"
        ))
    );

    let add_effect = &report.graft_templates[2];
    assert_eq!(add_effect.kind, "add_effect_declaration");
    assert_eq!(
        add_effect.operation.pointer("/op"),
        Some(&serde_json::json!("AddEffectDeclaration"))
    );
    assert_eq!(
        add_effect.operation.pointer("/payload/name"),
        Some(&serde_json::json!("NewEffect"))
    );
    assert_eq!(
        add_effect.editable_json_pointers,
        vec!["/payload/name".to_string()]
    );

    let add_import = &report.graft_templates[3];
    assert_eq!(add_import.kind, "add_import");
    assert_eq!(
        add_import.operation.pointer("/op"),
        Some(&serde_json::json!("AddImport"))
    );
    assert_eq!(
        add_import.operation.pointer("/payload/module"),
        Some(&serde_json::json!("app.new_module"))
    );
    assert_eq!(
        add_import.editable_json_pointers,
        vec!["/payload/module".to_string()]
    );

    for template in &report.graft_templates {
        let graft: GraftInput = serde_json::from_value(template.operation.clone())
            .expect("program declaration template should parse");
        let outcome = apply_graft_input(
            &program,
            graft,
            Some("agent:program-declaration-template-test".to_string()),
        );
        assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    }
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
            module_name_hint: None,
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
            module_name_hint: None,
        },
    );
    assert_eq!(missing.status, "blocked");
    assert!(missing.graft_templates.is_empty());
    assert_eq!(missing.diagnostics[0].id, "PLAN_SURFACE_NOT_FOUND");
    assert_eq!(missing.next_actions[0].kind, "inspect_plan_surfaces");
}

#[test]
fn edit_plan_graft_templates_can_target_expression_surfaces() {
    let source = r#"
module app.plan

task main -> Int {
  bind base = 1
  return base + 41
}
"#;
    let program = parse_program(source).expect("parse expression surface plan fixture");
    let target = "block:task:app.plan.main:stmt:1:expr:right";
    let report = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some(target.to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "ready");
    assert!(report.diagnostics.is_empty());
    assert_eq!(report.graft_templates.len(), 1);
    assert!(report.transaction_templates.is_empty());
    let template = &report.graft_templates[0];
    assert_eq!(template.kind, "replace_expression");
    assert_eq!(template.surface, target);
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/target"),
        Some(&serde_json::json!(target))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("41"))
    );
    assert_eq!(
        template.editable_json_pointers,
        vec!["/payload/source".to_string()]
    );
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse expression template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:expression-surface-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
}

#[test]
fn edit_plan_graft_templates_can_target_statement_surfaces() {
    let source = r#"
module app.plan

task main -> Int {
  tally total = 1
  set total = total + 1
  return total
}
"#;
    let program = parse_program(source).expect("parse statement surface plan fixture");
    let target = "block:task:app.plan.main:stmt:1";
    let report = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some(target.to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "ready");
    assert!(report.diagnostics.is_empty());
    assert_eq!(report.graft_templates.len(), 3);
    assert!(report.transaction_templates.is_empty());
    assert!(report.graft_templates.iter().all(|template| {
        template.surface == target
            && template.operation.pointer("/target") == Some(&serde_json::json!(target))
    }));
    assert!(
        report
            .graft_templates
            .iter()
            .any(|template| template.kind == "move_statement"
                && template.editable_json_pointers == vec!["/payload/position".to_string()])
    );
    let delete_template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "delete_statement")
        .expect("delete statement template");
    let graft: GraftInput =
        serde_json::from_value(delete_template.operation.clone()).expect("parse delete template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:statement-surface-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);

    let replace_template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "replace_statement")
        .expect("replace statement template");
    assert_eq!(
        replace_template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceStatement"))
    );
    assert_eq!(
        replace_template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("set total = total + 1"))
    );
    assert_eq!(
        replace_template.editable_json_pointers,
        vec!["/payload/source".to_string()]
    );
    let graft: GraftInput = serde_json::from_value(replace_template.operation.clone())
        .expect("parse replace statement template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:statement-surface-replace-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
}

#[test]
fn edit_plan_graft_templates_can_target_take_surfaces() {
    let source = r#"
module app.plan

task main -> Int {
  take first: Int
  take second: Int

  return first + second
}
"#;
    let program = parse_program(source).expect("parse take surface plan fixture");
    let target = "take:task:app.plan.main:1:second";
    let report = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some(target.to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "ready");
    assert!(report.diagnostics.is_empty());
    assert_eq!(report.graft_templates.len(), 1);
    assert!(report.transaction_templates.is_empty());
    let template = &report.graft_templates[0];
    assert_eq!(template.kind, "move_take");
    assert_eq!(template.surface, target);
    assert_eq!(
        template.operation.pointer("/target"),
        Some(&serde_json::json!(target))
    );
    assert_eq!(
        template.editable_json_pointers,
        vec!["/payload/position".to_string()]
    );
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse take move template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:take-surface-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
}

#[test]
fn edit_plan_graft_templates_can_target_block_surfaces() {
    let source = r#"
module app.plan

task main -> Int {
  if call threshold() > 1 {
    return 1
  } else {
    return 0
  }
}

task threshold -> Int {
  return 2
}
"#;
    let program = parse_program(source).expect("parse block surface plan fixture");
    let top_block = "block:task:app.plan.main";
    let report = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some(top_block.to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "ready");
    assert!(report.diagnostics.is_empty());
    assert_eq!(report.graft_templates.len(), 1);
    let template = &report.graft_templates[0];
    assert_eq!(template.kind, "insert_statement");
    assert_eq!(template.surface, top_block);
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("InsertStatement"))
    );
    assert_eq!(
        template.operation.pointer("/target"),
        Some(&serde_json::json!(top_block))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("forge { }"))
    );
    assert_eq!(
        template.operation.pointer("/payload/position"),
        Some(&serde_json::json!(0))
    );
    assert_eq!(
        template.editable_json_pointers,
        vec![
            "/payload/source".to_string(),
            "/payload/position".to_string()
        ]
    );
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse block insert template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:block-surface-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);

    let nested_block = "block:task:app.plan.main:stmt:0:then";
    let nested_report = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some(nested_block.to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(nested_report.status, "ready");
    assert_eq!(nested_report.graft_templates.len(), 1);
    assert_eq!(nested_report.graft_templates[0].kind, "insert_statement");
    assert_eq!(nested_report.graft_templates[0].surface, nested_block);
    assert_eq!(
        nested_report.graft_templates[0]
            .operation
            .pointer("/target"),
        Some(&serde_json::json!(nested_block))
    );
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
            module_name_hint: None,
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
            module_name_hint: None,
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
            module_name_hint: None,
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
            module_name_hint: None,
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
            module_name_hint: None,
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
            module_name_hint: None,
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
            module_name_hint: None,
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
fn edit_plan_graft_templates_include_unused_private_task_delete() {
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
    let program = parse_program(source).expect("parse unused private task plan fixture");
    let report = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "UNUSED_PRIVATE_TASK"
    );

    let template = report
        .graft_templates
        .iter()
        .find(|template| {
            template.kind == "delete_unused_private_task"
                && template.surface == "task:app.plan.orphan"
        })
        .expect("unused private task delete template");
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("DeleteNode"))
    );
    assert_eq!(
        template.operation.pointer("/target"),
        Some(&serde_json::json!("task:app.plan.orphan"))
    );
    assert!(template.editable_json_pointers.is_empty());
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse task delete template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:unused-task-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    assert!(
        !report.graft_templates.iter().any(|template| {
            template.kind == "delete_unused_private_task"
                && template.operation.pointer("/target")
                    == Some(&serde_json::json!("task:app.plan.used"))
        }),
        "used private task should not receive a lint-driven delete template"
    );

    let targeted_report = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("task:app.plan.orphan".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "delete_unused_private_task"
    );
    assert_eq!(
        targeted_report.graft_templates[0]
            .operation
            .pointer("/target"),
        Some(&serde_json::json!("task:app.plan.orphan"))
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_transaction_templates_include_dead_private_task_cleanup() {
    let source = r#"
module app.dead

task main -> Int {
  return call used()
}

task used -> Int {
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
    let program = parse_program(source).expect("parse dead private task cleanup fixture");
    let report = build_edit_plan_report_with_options(
        "app.dead",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    let lint = report.lint.as_ref().expect("lint summary");
    assert_eq!(lint.finding_count, 3);
    assert_eq!(lint.findings[0].id, "UNREACHABLE_PRIVATE_TASK");
    assert_eq!(lint.findings[0].node, "task:app.dead.cycle_a");
    assert_eq!(lint.findings[1].id, "UNREACHABLE_PRIVATE_TASK");
    assert_eq!(lint.findings[1].node, "task:app.dead.cycle_b");
    assert_eq!(lint.findings[2].id, "UNUSED_PRIVATE_TASK");
    assert_eq!(lint.findings[2].node, "task:app.dead.orphan");

    let cleanup = report
        .transaction_templates
        .iter()
        .find(|template| template.kind == "delete_dead_private_tasks")
        .expect("dead private task cleanup transaction template");
    assert_eq!(cleanup.surface, "lint:dead_private_tasks");
    assert_eq!(
        cleanup.transaction.pointer("/mode"),
        Some(&serde_json::json!("all_or_nothing"))
    );
    assert_eq!(
        cleanup.transaction.pointer("/ops/0/target"),
        Some(&serde_json::json!("task:app.dead.cycle_a"))
    );
    assert_eq!(
        cleanup.transaction.pointer("/ops/1/target"),
        Some(&serde_json::json!("task:app.dead.cycle_b"))
    );
    assert_eq!(
        cleanup.transaction.pointer("/ops/2/target"),
        Some(&serde_json::json!("task:app.dead.orphan"))
    );
    assert!(cleanup.editable_json_pointers.is_empty());
    let graft: GraftInput = serde_json::from_value(cleanup.transaction.clone())
        .expect("parse dead private task cleanup transaction");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:dead-private-task-cleanup-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);

    let targeted_report = build_edit_plan_report_with_options(
        "app.dead",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("task:app.dead.orphan".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_unused_take_remove() {
    let source = r#"
module app.takes

task main -> Int {
  take value: Int
  take unused: Int

  return value
}
"#;
    let program = parse_program(source).expect("parse unused take plan fixture");
    let report = build_edit_plan_report_with_options(
        "app.takes",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "UNUSED_TAKE"
    );

    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "remove_unused_take")
        .expect("unused take remove template");
    assert_eq!(template.surface, "take:task:app.takes.main:1:unused");
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("RemoveTake"))
    );
    assert_eq!(
        template.operation.pointer("/target"),
        Some(&serde_json::json!("task:app.takes.main"))
    );
    assert_eq!(
        template.operation.pointer("/payload/name"),
        Some(&serde_json::json!("unused"))
    );
    assert!(template.editable_json_pointers.is_empty());
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse unused take template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:unused-take-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    assert!(
        !outcome
            .source
            .expect("grafted source")
            .contains("take unused")
    );

    let targeted_report = build_edit_plan_report_with_options(
        "app.takes",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("take:task:app.takes.main:1:unused".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "remove_unused_take"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_unused_declared_effect_remove() {
    let source = r#"
module app.effects

task main -> Text uses Network {
  return "ready"
}
"#;
    let program = parse_program(source).expect("parse unused declared effect plan fixture");
    let report = build_edit_plan_report_with_options(
        "app.effects",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "UNUSED_DECLARED_EFFECT"
    );
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].node,
        "effect-use:task:app.effects.main:0:Network"
    );

    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "remove_unused_declared_effect")
        .expect("unused declared effect remove template");
    assert_eq!(
        template.surface,
        "effect-use:task:app.effects.main:0:Network"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("RemoveTaskEffect"))
    );
    assert_eq!(
        template.operation.pointer("/target"),
        Some(&serde_json::json!("task:app.effects.main"))
    );
    assert_eq!(
        template.operation.pointer("/payload/name"),
        Some(&serde_json::json!("Network"))
    );
    assert!(template.editable_json_pointers.is_empty());
    let graft: GraftInput = serde_json::from_value(template.operation.clone())
        .expect("parse unused declared effect template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:unused-effect-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    assert!(
        !outcome
            .source
            .expect("grafted source")
            .contains("uses Network")
    );

    let targeted_report = build_edit_plan_report_with_options(
        "app.effects",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("effect-use:task:app.effects.main:0:Network".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "remove_unused_declared_effect"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_unused_import_delete() {
    let root = temp_project_dir("unused-import-plan-template");
    write_unused_import_project(&root);
    let project = load_project(&root).expect("load unused import project");
    let program = project.program;

    let report = build_edit_plan_report_with_options(
        root.display().to_string(),
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert!(
        report
            .lint
            .as_ref()
            .expect("lint summary")
            .findings
            .iter()
            .any(|finding| {
                finding.id == "UNUSED_IMPORT" && finding.node == "import:app.main:app.stale"
            })
    );

    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "delete_unused_import")
        .expect("unused import delete template");
    assert_eq!(template.surface, "import:app.main:app.stale");
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("DeleteNode"))
    );
    assert_eq!(
        template.operation.pointer("/target"),
        Some(&serde_json::json!("import:app.main:app.stale"))
    );
    assert!(template.editable_json_pointers.is_empty());
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse unused import template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:unused-import-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);

    let targeted_report = build_edit_plan_report_with_options(
        root.display().to_string(),
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("import:app.main:app.stale".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "delete_unused_import"
    );
    assert!(targeted_report.transaction_templates.is_empty());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn edit_plan_graft_templates_include_missing_module_fix() {
    let source = r#"task main -> Text {
  return "hello"
}
"#;
    let program = parse_program(source).expect("parse missing module plan fixture");
    let report = build_edit_plan_report_with_options(
        "missing_module.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "MISSING_MODULE_DECLARATION"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "add_module_declaration")
        .expect("missing module graft template");
    assert_eq!(template.surface, "program");
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("AddModuleDeclaration"))
    );
    assert_eq!(
        template.operation.pointer("/payload/name"),
        Some(&serde_json::json!("missing_module"))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/name"]);
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse module template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:module-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);

    let targeted_report = build_edit_plan_report_with_options(
        "missing_module.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("program".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "add_module_declaration"
    );
}

#[test]
fn edit_plan_graft_templates_include_raw_host_adapter_migration() {
    let source = r#"
module app.plan

task main -> Result<Text, Error> uses FileRead {
  bind text = fs.read_text("examples/hello.sley")

  return Ok(text)
}
"#;
    let program = parse_program(source).expect("parse raw host migration plan fixture");
    let report = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "RAW_HOST_ADAPTER"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "migrate_raw_host_adapter")
        .expect("raw host migration template");
    assert_eq!(template.surface, "block:task:app.plan.main:stmt:0:expr");
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!(
            "fs.try_read_text(\"examples/hello.sley\")?"
        ))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse raw host template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:raw-host-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);

    let targeted_report = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.plan.main:stmt:0:expr".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "migrate_raw_host_adapter"
    );
}

#[test]
fn edit_plan_graft_templates_include_unchecked_result_propagation() {
    let source = r#"
module app.plan

task main -> Result<Text, Error> uses FileWrite {
  fs.try_write_text("out.txt", "ok")

  return Ok("ok")
}
"#;
    let program = parse_program(source).expect("parse unchecked result plan fixture");
    let report = build_edit_plan_report_with_options(
        "app.plan",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "UNCHECKED_RESULT"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "propagate_unchecked_result")
        .expect("unchecked result propagation template");
    assert_eq!(template.surface, "block:task:app.plan.main:stmt:0:expr");
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!(
            "fs.try_write_text(\"out.txt\", \"ok\")?"
        ))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse propagation template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:unchecked-result-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
}

#[test]
fn edit_plan_graft_templates_include_unqualified_import_call_qualification() {
    let project = load_project("examples/unqualified_import_call_project")
        .expect("load import style project");
    let program = project.program;
    let report = build_edit_plan_report_with_options(
        "examples/unqualified_import_call_project",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "UNQUALIFIED_IMPORTED_CALL"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "qualify_imported_call")
        .expect("qualified imported call template");
    assert_eq!(template.surface, "block:task:app.main.main:stmt:0:expr");
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("call math.double(21)"))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse import call template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:qualified-import-call-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);

    let targeted_report = build_edit_plan_report_with_options(
        "examples/unqualified_import_call_project",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.main.main:stmt:0:expr".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "qualify_imported_call"
    );
}

#[test]
fn edit_plan_graft_templates_include_unused_pure_binding_delete() {
    let source = include_str!("../examples/unused_pure_binding.sley");
    let program = parse_program(source).expect("parse unused pure binding fixture");
    let report = build_edit_plan_report_with_options(
        "examples/unused_pure_binding.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "UNUSED_PURE_BINDING"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "delete_unused_pure_binding")
        .expect("unused pure binding delete template");
    assert_eq!(template.surface, "block:task:app.bindings.main:stmt:0");
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("DeleteNode"))
    );
    assert_eq!(
        template.operation.pointer("/target"),
        Some(&serde_json::json!("block:task:app.bindings.main:stmt:0"))
    );
    assert!(template.editable_json_pointers.is_empty());
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse binding delete template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:unused-pure-binding-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    assert!(
        !outcome
            .source
            .expect("grafted source")
            .contains("bind stale")
    );

    let targeted_report = build_edit_plan_report_with_options(
        "examples/unused_pure_binding.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.bindings.main:stmt:0".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "delete_unused_pure_binding"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_unused_pure_expression_statement_delete() {
    let source = include_str!("../examples/unused_pure_expression_statement.sley");
    let program = parse_program(source).expect("parse unused pure expression statement fixture");
    let report = build_edit_plan_report_with_options(
        "examples/unused_pure_expression_statement.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "UNUSED_PURE_EXPRESSION_STATEMENT"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "delete_unused_pure_expression_statement")
        .expect("unused pure expression statement delete template");
    assert_eq!(template.surface, "block:task:app.unused_expr.main:stmt:1");
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("DeleteNode"))
    );
    assert_eq!(
        template.operation.pointer("/target"),
        Some(&serde_json::json!("block:task:app.unused_expr.main:stmt:1"))
    );
    assert!(template.editable_json_pointers.is_empty());
    let graft: GraftInput = serde_json::from_value(template.operation.clone())
        .expect("parse expression delete template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:unused-pure-expression-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    assert!(
        !outcome
            .source
            .expect("grafted source")
            .contains("value + 1")
    );

    let targeted_report = build_edit_plan_report_with_options(
        "examples/unused_pure_expression_statement.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.unused_expr.main:stmt:1".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "delete_unused_pure_expression_statement"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_self_assignment_statement_delete() {
    let source = include_str!("../examples/self_assignment_statement.sley");
    let program = parse_program(source).expect("parse self assignment fixture");
    let report = build_edit_plan_report_with_options(
        "examples/self_assignment_statement.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "SELF_ASSIGNMENT_STATEMENT"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "delete_self_assignment_statement")
        .expect("self assignment statement delete template");
    assert_eq!(
        template.surface,
        "block:task:app.self_assignment.main:stmt:1"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("DeleteNode"))
    );
    assert_eq!(
        template.operation.pointer("/target"),
        Some(&serde_json::json!(
            "block:task:app.self_assignment.main:stmt:1"
        ))
    );
    assert!(template.editable_json_pointers.is_empty());
    let graft: GraftInput = serde_json::from_value(template.operation.clone())
        .expect("parse self assignment delete template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:self-assignment-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(!grafted_source.contains("set count = count\n"));
    assert!(grafted_source.contains("set count = count + 1"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::SelfAssignmentStatement],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/self_assignment_statement.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.self_assignment.main:stmt:1".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "delete_self_assignment_statement"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_overwritten_set_statement_delete() {
    let source = include_str!("../examples/overwritten_set_statement.sley");
    let program = parse_program(source).expect("parse overwritten set fixture");
    let report = build_edit_plan_report_with_options(
        "examples/overwritten_set_statement.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "OVERWRITTEN_SET_STATEMENT"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "delete_overwritten_set_statement")
        .expect("overwritten set statement delete template");
    assert_eq!(
        template.surface,
        "block:task:app.overwritten_set.main:stmt:1"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("DeleteNode"))
    );
    assert_eq!(
        template.operation.pointer("/target"),
        Some(&serde_json::json!(
            "block:task:app.overwritten_set.main:stmt:1"
        ))
    );
    assert!(template.editable_json_pointers.is_empty());
    let graft: GraftInput = serde_json::from_value(template.operation.clone())
        .expect("parse overwritten set delete template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:overwritten-set-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(!grafted_source.contains("set count = 1\n"));
    assert!(grafted_source.contains("set count = 2"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::OverwrittenSetStatement],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/overwritten_set_statement.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.overwritten_set.main:stmt:1".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "delete_overwritten_set_statement"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_transaction_templates_include_redundant_initial_set_fold() {
    let source = include_str!("../examples/redundant_initial_set_statement.sley");
    let program = parse_program(source).expect("parse redundant initial set fixture");
    let report = build_edit_plan_report_with_options(
        "examples/redundant_initial_set_statement.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "REDUNDANT_INITIAL_SET_STATEMENT"
    );
    let template = report
        .transaction_templates
        .iter()
        .find(|template| template.kind == "fold_redundant_initial_set_into_binding")
        .expect("redundant initial set fold transaction");
    assert_eq!(
        template.surface,
        "block:task:app.redundant_initial_set.main:stmt:1"
    );
    assert_eq!(
        template.transaction.pointer("/ops/0/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.transaction.pointer("/ops/0/target"),
        Some(&serde_json::json!(
            "block:task:app.redundant_initial_set.main:stmt:0:expr"
        ))
    );
    assert_eq!(
        template.transaction.pointer("/ops/0/payload/source"),
        Some(&serde_json::json!("1"))
    );
    assert_eq!(
        template.transaction.pointer("/ops/1/op"),
        Some(&serde_json::json!("DeleteNode"))
    );
    assert_eq!(
        template.transaction.pointer("/ops/1/target"),
        Some(&serde_json::json!(
            "block:task:app.redundant_initial_set.main:stmt:1"
        ))
    );
    assert_eq!(
        template.editable_json_pointers,
        vec!["/ops/0/payload/source"]
    );
    let graft: GraftInput = serde_json::from_value(template.transaction.clone())
        .expect("parse redundant initial set transaction template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:redundant-initial-set-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("state count = 1"));
    assert!(!grafted_source.contains("set count = 1\n"));
    assert!(grafted_source.contains("set count = count + 1"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::RedundantInitialSetStatement],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/redundant_initial_set_statement.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.redundant_initial_set.main:stmt:1".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.transaction_templates.len(), 1);
    assert_eq!(
        targeted_report.transaction_templates[0].kind,
        "fold_redundant_initial_set_into_binding"
    );
}

#[test]
fn edit_plan_transaction_templates_include_redundant_initial_set_to_bind() {
    let source = include_str!("../examples/redundant_initial_set_to_bind.sley");
    let program = parse_program(source).expect("parse redundant initial set to bind fixture");
    let report = build_edit_plan_report_with_options(
        "examples/redundant_initial_set_to_bind.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "REDUNDANT_INITIAL_SET_STATEMENT"
    );
    let template = report
        .transaction_templates
        .iter()
        .find(|template| template.kind == "convert_redundant_initial_set_to_bind")
        .expect("redundant initial set to bind transaction");
    assert_eq!(
        template.surface,
        "block:task:app.redundant_initial_set_to_bind.main:stmt:1"
    );
    assert_eq!(
        template.transaction.pointer("/ops/0/op"),
        Some(&serde_json::json!("ReplaceStatement"))
    );
    assert_eq!(
        template.transaction.pointer("/ops/0/target"),
        Some(&serde_json::json!(
            "block:task:app.redundant_initial_set_to_bind.main:stmt:0"
        ))
    );
    assert_eq!(
        template.transaction.pointer("/ops/0/payload/source"),
        Some(&serde_json::json!("bind count = 1"))
    );
    assert_eq!(
        template.transaction.pointer("/ops/1/op"),
        Some(&serde_json::json!("DeleteNode"))
    );
    assert_eq!(
        template.transaction.pointer("/ops/1/target"),
        Some(&serde_json::json!(
            "block:task:app.redundant_initial_set_to_bind.main:stmt:1"
        ))
    );
    assert_eq!(
        template.editable_json_pointers,
        vec!["/ops/0/payload/source"]
    );
    let graft: GraftInput = serde_json::from_value(template.transaction.clone())
        .expect("parse redundant initial set to bind transaction template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:redundant-initial-set-to-bind-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("bind count = 1"));
    assert!(!grafted_source.contains("state count = 0"));
    assert!(!grafted_source.contains("set count = 1"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![
                LintRule::MutableBindingNeverSet,
                LintRule::RedundantInitialSetStatement,
                LintRule::UnusedPureBinding,
            ],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/redundant_initial_set_to_bind.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some(
                "block:task:app.redundant_initial_set_to_bind.main:stmt:1".to_string(),
            ),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.transaction_templates.len(), 1);
    assert_eq!(
        targeted_report.transaction_templates[0].kind,
        "convert_redundant_initial_set_to_bind"
    );
}

#[test]
fn edit_plan_graft_templates_include_unreachable_statement_delete() {
    let source = include_str!("../examples/unreachable_statement.sley");
    let program = parse_program(source).expect("parse unreachable statement fixture");
    let report = build_edit_plan_report_with_options(
        "examples/unreachable_statement.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert!(
        report
            .lint
            .as_ref()
            .expect("lint summary")
            .findings
            .iter()
            .any(|finding| finding.id == "UNREACHABLE_STATEMENT")
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "delete_unreachable_statement")
        .expect("unreachable statement delete template");
    assert_eq!(
        template.surface,
        "block:task:app.unreachable_statement.main:stmt:2"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("DeleteNode"))
    );
    assert_eq!(
        template.operation.pointer("/target"),
        Some(&serde_json::json!(
            "block:task:app.unreachable_statement.main:stmt:2"
        ))
    );
    assert!(template.editable_json_pointers.is_empty());
    let graft: GraftInput = serde_json::from_value(template.operation.clone())
        .expect("parse unreachable statement delete template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:unreachable-statement-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    assert!(!outcome.source.expect("grafted source").contains("return 0"));

    let targeted_report = build_edit_plan_report_with_options(
        "examples/unreachable_statement.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.unreachable_statement.main:stmt:2".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "delete_unreachable_statement"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_constant_if_expression_simplify() {
    let source = include_str!("../examples/constant_if_expression.sley");
    let program = parse_program(source).expect("parse constant if fixture");
    let report = build_edit_plan_report_with_options(
        "examples/constant_if_expression.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "CONSTANT_IF_EXPRESSION"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_constant_if_expression")
        .expect("constant if expression simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.constant_if.main:stmt:0:expr"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("41"))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse constant if template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:constant-if-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return 41"));
    assert!(!grafted_source.contains("if true"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::ConstantIfExpression],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/constant_if_expression.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.constant_if.main:stmt:0:expr".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "simplify_constant_if_expression"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_constant_if_statement_simplify() {
    let source = include_str!("../examples/constant_if_statement.sley");
    let program = parse_program(source).expect("parse constant if statement fixture");
    let report = build_edit_plan_report_with_options(
        "examples/constant_if_statement.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "CONSTANT_IF_STATEMENT"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_constant_if_statement")
        .expect("constant if statement simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.constant_if_statement.main:stmt:1"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceStatement"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("set value = 42"))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput = serde_json::from_value(template.operation.clone())
        .expect("parse constant if statement template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:constant-if-statement-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("set value = 42"));
    assert!(!grafted_source.contains("if true"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::ConstantIfStatement],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/constant_if_statement.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.constant_if_statement.main:stmt:1".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "simplify_constant_if_statement"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_constant_false_if_statement_delete() {
    let source = include_str!("../examples/constant_false_if_statement.sley");
    let program = parse_program(source).expect("parse constant false if fixture");
    let report = build_edit_plan_report_with_options(
        "examples/constant_false_if_statement.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "CONSTANT_FALSE_IF_STATEMENT"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "delete_constant_false_if_statement")
        .expect("constant false if delete template");
    assert_eq!(
        template.surface,
        "block:task:app.constant_false_if.main:stmt:0"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("DeleteNode"))
    );
    assert_eq!(
        template.operation.pointer("/target"),
        Some(&serde_json::json!(
            "block:task:app.constant_false_if.main:stmt:0"
        ))
    );
    assert!(template.editable_json_pointers.is_empty());
    let graft: GraftInput = serde_json::from_value(template.operation.clone())
        .expect("parse constant false if delete template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:constant-false-if-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(!grafted_source.contains("if false"));
    assert!(grafted_source.contains("return 42"));

    let targeted_report = build_edit_plan_report_with_options(
        "examples/constant_false_if_statement.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.constant_false_if.main:stmt:0".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "delete_constant_false_if_statement"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_constant_false_while_statement_delete() {
    let source = include_str!("../examples/constant_false_while_statement.sley");
    let program = parse_program(source).expect("parse constant false while fixture");
    let report = build_edit_plan_report_with_options(
        "examples/constant_false_while_statement.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "CONSTANT_FALSE_WHILE_STATEMENT"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "delete_constant_false_while_statement")
        .expect("constant false while delete template");
    assert_eq!(template.surface, "block:task:app.false_while.main:stmt:1");
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("DeleteNode"))
    );
    assert_eq!(
        template.operation.pointer("/target"),
        Some(&serde_json::json!("block:task:app.false_while.main:stmt:1"))
    );
    assert!(template.editable_json_pointers.is_empty());
    let graft: GraftInput = serde_json::from_value(template.operation.clone())
        .expect("parse constant false while delete template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:constant-false-while-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(!grafted_source.contains("while false"));
    assert!(grafted_source.contains("return value"));

    let targeted_report = build_edit_plan_report_with_options(
        "examples/constant_false_while_statement.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.false_while.main:stmt:1".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "delete_constant_false_while_statement"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_constant_comparison_expression_simplify() {
    let source = include_str!("../examples/constant_comparison_expression.sley");
    let program = parse_program(source).expect("parse constant comparison fixture");
    let report = build_edit_plan_report_with_options(
        "examples/constant_comparison_expression.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "CONSTANT_COMPARISON_EXPRESSION"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_constant_comparison_expression")
        .expect("constant comparison simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.constant_compare.main:stmt:0:expr"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("true"))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput = serde_json::from_value(template.operation.clone())
        .expect("parse constant comparison template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:constant-comparison-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return true"));
    assert!(!grafted_source.contains("1 < 2"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::ConstantComparisonExpression],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/constant_comparison_expression.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.constant_compare.main:stmt:0:expr".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "simplify_constant_comparison_expression"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_constant_arithmetic_expression_simplify() {
    let source = include_str!("../examples/constant_arithmetic_expression.sley");
    let program = parse_program(source).expect("parse constant arithmetic fixture");
    let report = build_edit_plan_report_with_options(
        "examples/constant_arithmetic_expression.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "CONSTANT_ARITHMETIC_EXPRESSION"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_constant_arithmetic_expression")
        .expect("constant arithmetic simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.constant_arithmetic.main:stmt:0:expr"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("5"))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput = serde_json::from_value(template.operation.clone())
        .expect("parse constant arithmetic template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:constant-arithmetic-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return 5"));
    assert!(!grafted_source.contains("2 + 3"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::ConstantArithmeticExpression],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/constant_arithmetic_expression.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some(
                "block:task:app.constant_arithmetic.main:stmt:0:expr".to_string(),
            ),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "simplify_constant_arithmetic_expression"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_absorbing_arithmetic_expression_simplify() {
    let source = include_str!("../examples/absorbing_arithmetic_expression.sley");
    let program = parse_program(source).expect("parse absorbing arithmetic fixture");
    let report = build_edit_plan_report_with_options(
        "examples/absorbing_arithmetic_expression.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 2);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "ABSORBING_ARITHMETIC_EXPRESSION"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_absorbing_arithmetic_expression")
        .expect("absorbing arithmetic simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.absorbing_arithmetic.main:stmt:0:expr"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("0"))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput = serde_json::from_value(template.operation.clone())
        .expect("parse absorbing arithmetic template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:absorbing-arithmetic-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return 0"));
    assert!(!grafted_source.contains("1 + 2"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::AbsorbingArithmeticExpression],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/absorbing_arithmetic_expression.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some(
                "block:task:app.absorbing_arithmetic.main:stmt:0:expr".to_string(),
            ),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "simplify_absorbing_arithmetic_expression"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_constant_text_concatenation_expression_simplify() {
    let source = include_str!("../examples/constant_text_concatenation_expression.sley");
    let program = parse_program(source).expect("parse constant text concatenation fixture");
    let report = build_edit_plan_report_with_options(
        "examples/constant_text_concatenation_expression.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "CONSTANT_TEXT_CONCATENATION_EXPRESSION"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_constant_text_concatenation_expression")
        .expect("constant text concatenation simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.constant_text_concat.main:stmt:0:expr"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("\"Sley agents\""))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput = serde_json::from_value(template.operation.clone())
        .expect("parse constant text concatenation template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:constant-text-concat-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return \"Sley agents\""));
    assert!(!grafted_source.contains("\"Sley \" + \"agents\""));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::ConstantTextConcatenationExpression],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/constant_text_concatenation_expression.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some(
                "block:task:app.constant_text_concat.main:stmt:0:expr".to_string(),
            ),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "simplify_constant_text_concatenation_expression"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_constant_list_index_expression_simplify() {
    let source = include_str!("../examples/constant_list_index_expression.sley");
    let program = parse_program(source).expect("parse constant list index fixture");
    let report = build_edit_plan_report_with_options(
        "examples/constant_list_index_expression.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "CONSTANT_LIST_INDEX_EXPRESSION"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_constant_list_index_expression")
        .expect("constant list index simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.constant_list_index.main:stmt:0:expr"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("20"))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse constant list template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:constant-list-index-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return 20"));
    assert!(!grafted_source.contains("[10, 20, 30][1]"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::ConstantListIndexExpression],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/constant_list_index_expression.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some(
                "block:task:app.constant_list_index.main:stmt:0:expr".to_string(),
            ),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "simplify_constant_list_index_expression"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_constant_map_index_expression_simplify() {
    let source = include_str!("../examples/constant_map_index_expression.sley");
    let program = parse_program(source).expect("parse constant map index fixture");
    let report = build_edit_plan_report_with_options(
        "examples/constant_map_index_expression.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "CONSTANT_MAP_INDEX_EXPRESSION"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_constant_map_index_expression")
        .expect("constant map index simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.constant_map_index.main:stmt:0:expr"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("\"done\""))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse constant map template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:constant-map-index-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return \"done\""));
    assert!(!grafted_source.contains("map { \"alpha\": \"ready\", \"beta\": \"done\" }[\"beta\"]"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::ConstantMapIndexExpression],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/constant_map_index_expression.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some(
                "block:task:app.constant_map_index.main:stmt:0:expr".to_string(),
            ),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "simplify_constant_map_index_expression"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_constant_record_field_access_expression_simplify() {
    let source = include_str!("../examples/constant_record_field_access_expression.sley");
    let program = parse_program(source).expect("parse constant record field access fixture");
    let report = build_edit_plan_report_with_options(
        "examples/constant_record_field_access_expression.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "CONSTANT_RECORD_FIELD_ACCESS_EXPRESSION"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_constant_record_field_access_expression")
        .expect("constant record field access simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.constant_record_field_access.main:stmt:0:expr"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("\"Ada\""))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput = serde_json::from_value(template.operation.clone())
        .expect("parse constant record field access template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:constant-record-field-access-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return \"Ada\""));
    assert!(!grafted_source.contains("User { name: \"Ada\", age: 37 }.name"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::ConstantRecordFieldAccessExpression],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/constant_record_field_access_expression.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some(
                "block:task:app.constant_record_field_access.main:stmt:0:expr".to_string(),
            ),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "simplify_constant_record_field_access_expression"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_constant_len_expression_simplify() {
    let source = include_str!("../examples/constant_len_expression.sley");
    let program = parse_program(source).expect("parse constant len fixture");
    let report = build_edit_plan_report_with_options(
        "examples/constant_len_expression.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "CONSTANT_LEN_EXPRESSION"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_constant_len_expression")
        .expect("constant len simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.constant_len.main:stmt:0:expr"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("3"))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse constant len template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:constant-len-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return 3"));
    assert!(!grafted_source.contains("len([\"red\", \"blue\", \"green\"])"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::ConstantLenExpression],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/constant_len_expression.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.constant_len.main:stmt:0:expr".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "simplify_constant_len_expression"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_constant_not_expression_simplify() {
    let source = include_str!("../examples/constant_not_expression.sley");
    let program = parse_program(source).expect("parse constant not fixture");
    let report = build_edit_plan_report_with_options(
        "examples/constant_not_expression.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "CONSTANT_NOT_EXPRESSION"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_constant_not_expression")
        .expect("constant not simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.constant_not.main:stmt:0:expr"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("false"))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse constant not template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:constant-not-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return false"));
    assert!(!grafted_source.contains("return !true"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::ConstantNotExpression],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/constant_not_expression.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.constant_not.main:stmt:0:expr".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "simplify_constant_not_expression"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_empty_if_statement_delete() {
    let source = include_str!("../examples/empty_if_statement.sley");
    let program = parse_program(source).expect("parse empty if fixture");
    let report = build_edit_plan_report_with_options(
        "examples/empty_if_statement.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "EMPTY_IF_STATEMENT"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "delete_empty_if_statement")
        .expect("empty if delete template");
    assert_eq!(template.surface, "block:task:app.empty_if.main:stmt:1");
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("DeleteNode"))
    );
    assert_eq!(
        template.operation.pointer("/target"),
        Some(&serde_json::json!("block:task:app.empty_if.main:stmt:1"))
    );
    assert!(template.editable_json_pointers.is_empty());
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse empty if delete template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:empty-if-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(!grafted_source.contains("if value > 0"));
    assert!(grafted_source.contains("return value"));

    let targeted_report = build_edit_plan_report_with_options(
        "examples/empty_if_statement.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.empty_if.main:stmt:1".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "delete_empty_if_statement"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_empty_else_statement_remove() {
    let source = include_str!("../examples/empty_else_statement.sley");
    let program = parse_program(source).expect("parse empty else fixture");
    let report = build_edit_plan_report_with_options(
        "examples/empty_else_statement.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "EMPTY_ELSE_STATEMENT"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "remove_empty_else_statement")
        .expect("empty else remove template");
    assert_eq!(template.surface, "block:task:app.empty_else.main:stmt:1");
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceStatement"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("if value > 0 {\n  return value\n}"))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse empty else template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:empty-else-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(!grafted_source.contains("else {"));
    assert!(grafted_source.contains("return value"));

    let targeted_report = build_edit_plan_report_with_options(
        "examples/empty_else_statement.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.empty_else.main:stmt:1".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "remove_empty_else_statement"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_empty_for_statement_delete() {
    let source = include_str!("../examples/empty_for_statement.sley");
    let program = parse_program(source).expect("parse empty for fixture");
    let report = build_edit_plan_report_with_options(
        "examples/empty_for_statement.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "EMPTY_FOR_STATEMENT"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "delete_empty_for_statement")
        .expect("empty for delete template");
    assert_eq!(template.surface, "block:task:app.empty_for.main:stmt:1");
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("DeleteNode"))
    );
    assert_eq!(
        template.operation.pointer("/target"),
        Some(&serde_json::json!("block:task:app.empty_for.main:stmt:1"))
    );
    assert!(template.editable_json_pointers.is_empty());
    let graft: GraftInput = serde_json::from_value(template.operation.clone())
        .expect("parse empty for delete template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:empty-for-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(!grafted_source.contains("for item in []"));
    assert!(grafted_source.contains("return value"));

    let targeted_report = build_edit_plan_report_with_options(
        "examples/empty_for_statement.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.empty_for.main:stmt:1".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "delete_empty_for_statement"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_empty_forge_statement_delete() {
    let source = include_str!("../examples/empty_forge_statement.sley");
    let program = parse_program(source).expect("parse empty forge fixture");
    let report = build_edit_plan_report_with_options(
        "examples/empty_forge_statement.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "EMPTY_FORGE_STATEMENT"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "delete_empty_forge_statement")
        .expect("empty forge delete template");
    assert_eq!(template.surface, "block:task:app.empty_forge.main:stmt:1");
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("DeleteNode"))
    );
    assert_eq!(
        template.operation.pointer("/target"),
        Some(&serde_json::json!("block:task:app.empty_forge.main:stmt:1"))
    );
    assert!(template.editable_json_pointers.is_empty());
    let graft: GraftInput = serde_json::from_value(template.operation.clone())
        .expect("parse empty forge delete template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:empty-forge-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(!grafted_source.contains("forge {"));
    assert!(grafted_source.contains("return value"));

    let targeted_report = build_edit_plan_report_with_options(
        "examples/empty_forge_statement.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.empty_forge.main:stmt:1".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "delete_empty_forge_statement"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_identity_binary_expression_simplify() {
    let source = include_str!("../examples/identity_binary_expression.sley");
    let program = parse_program(source).expect("parse identity binary fixture");
    let report = build_edit_plan_report_with_options(
        "examples/identity_binary_expression.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "IDENTITY_BINARY_EXPRESSION"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_identity_binary_expression")
        .expect("identity binary expression simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.identity_binary.main:stmt:1:expr"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("base"))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse identity binary template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:identity-binary-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return base"));
    assert!(!grafted_source.contains("base + 0"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::IdentityBinaryExpression],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/identity_binary_expression.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.identity_binary.main:stmt:1:expr".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "simplify_identity_binary_expression"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_text_identity_binary_expression_simplify() {
    let source = r#"
module app.identity_text

task main -> Text {
  bind label = "agent"

  return "" + label
}
"#;
    let program = parse_program(source).expect("parse identity text source");
    let report = build_edit_plan_report_with_options(
        "examples/identity_text_expression.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "IDENTITY_BINARY_EXPRESSION"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_identity_binary_expression")
        .expect("text identity binary expression simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.identity_text.main:stmt:1:expr"
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("label"))
    );
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse text identity template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:text-identity-binary-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return label"));
    assert!(!grafted_source.contains("\"\" + label"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::IdentityBinaryExpression],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");
}

#[test]
fn edit_plan_graft_templates_include_redundant_boolean_comparison_simplify() {
    let source = include_str!("../examples/redundant_boolean_comparison.sley");
    let program = parse_program(source).expect("parse redundant boolean comparison fixture");
    let report = build_edit_plan_report_with_options(
        "examples/redundant_boolean_comparison.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "REDUNDANT_BOOLEAN_COMPARISON"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_redundant_boolean_comparison")
        .expect("redundant boolean comparison simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.boolean_compare.main:stmt:1:expr"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("ready"))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput = serde_json::from_value(template.operation.clone())
        .expect("parse redundant boolean comparison template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:boolean-comparison-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return ready"));
    assert!(!grafted_source.contains("ready == true"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::RedundantBooleanComparison],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/redundant_boolean_comparison.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.boolean_compare.main:stmt:1:expr".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "simplify_redundant_boolean_comparison"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_absorbing_boolean_expression_simplify() {
    let source = include_str!("../examples/absorbing_boolean_expression.sley");
    let program = parse_program(source).expect("parse absorbing boolean fixture");
    let report = build_edit_plan_report_with_options(
        "examples/absorbing_boolean_expression.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "ABSORBING_BOOLEAN_EXPRESSION"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_absorbing_boolean_expression")
        .expect("absorbing boolean expression simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.absorbing_boolean.main:stmt:1:expr"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("false"))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput = serde_json::from_value(template.operation.clone())
        .expect("parse absorbing boolean template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:absorbing-boolean-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return false"));
    assert!(!grafted_source.contains("ready && false"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::AbsorbingBooleanExpression],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/absorbing_boolean_expression.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.absorbing_boolean.main:stmt:1:expr".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "simplify_absorbing_boolean_expression"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_idempotent_boolean_expression_simplify() {
    let source = include_str!("../examples/idempotent_boolean_expression.sley");
    let program = parse_program(source).expect("parse idempotent boolean fixture");
    let report = build_edit_plan_report_with_options(
        "examples/idempotent_boolean_expression.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "IDEMPOTENT_BOOLEAN_EXPRESSION"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_idempotent_boolean_expression")
        .expect("idempotent boolean expression simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.idempotent_boolean.main:stmt:1:expr"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("ready"))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput = serde_json::from_value(template.operation.clone())
        .expect("parse idempotent boolean template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:idempotent-boolean-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return ready"));
    assert!(!grafted_source.contains("ready && ready"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::IdempotentBooleanExpression],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/idempotent_boolean_expression.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some(
                "block:task:app.idempotent_boolean.main:stmt:1:expr".to_string(),
            ),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "simplify_idempotent_boolean_expression"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_self_comparison_expression_simplify() {
    let source = include_str!("../examples/self_comparison_expression.sley");
    let program = parse_program(source).expect("parse self-comparison fixture");
    let report = build_edit_plan_report_with_options(
        "examples/self_comparison_expression.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "SELF_COMPARISON_EXPRESSION"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_self_comparison_expression")
        .expect("self-comparison simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.self_compare.main:stmt:1:expr"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("true"))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse self-comparison template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:self-comparison-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return true"));
    assert!(!grafted_source.contains("ready == ready"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::SelfComparisonExpression],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/self_comparison_expression.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.self_compare.main:stmt:1:expr".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "simplify_self_comparison_expression"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_double_negation_expression_simplify() {
    let source = include_str!("../examples/double_negation_expression.sley");
    let program = parse_program(source).expect("parse double negation fixture");
    let report = build_edit_plan_report_with_options(
        "examples/double_negation_expression.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "DOUBLE_NEGATION_EXPRESSION"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_double_negation_expression")
        .expect("double negation expression simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.double_negation.main:stmt:1:expr"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("ready"))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse double negation template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:double-negation-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return ready"));
    assert!(!grafted_source.contains("!!ready"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::DoubleNegationExpression],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/double_negation_expression.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.double_negation.main:stmt:1:expr".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "simplify_double_negation_expression"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_negated_comparison_expression_simplify() {
    let source = include_str!("../examples/negated_comparison_expression.sley");
    let program = parse_program(source).expect("parse negated comparison fixture");
    let report = build_edit_plan_report_with_options(
        "examples/negated_comparison_expression.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "NEGATED_COMPARISON_EXPRESSION"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_negated_comparison_expression")
        .expect("negated comparison expression simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.negated_compare.main:stmt:1:expr"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("limit < 3"))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput = serde_json::from_value(template.operation.clone())
        .expect("parse negated comparison template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:negated-comparison-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return limit < 3"));
    assert!(!grafted_source.contains("!(limit >= 3)"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::NegatedComparisonExpression],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/negated_comparison_expression.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.negated_compare.main:stmt:1:expr".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "simplify_negated_comparison_expression"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_redundant_boolean_if_expression_simplify() {
    let source = include_str!("../examples/redundant_boolean_if_expression.sley");
    let program = parse_program(source).expect("parse redundant boolean if fixture");
    let report = build_edit_plan_report_with_options(
        "examples/redundant_boolean_if_expression.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "REDUNDANT_BOOLEAN_IF_EXPRESSION"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_redundant_boolean_if_expression")
        .expect("redundant boolean if expression simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.boolean_if.main:stmt:1:expr"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("ready"))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput = serde_json::from_value(template.operation.clone())
        .expect("parse redundant boolean if template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:boolean-if-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return ready"));
    assert!(!grafted_source.contains("if ready"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::RedundantBooleanIfExpression],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/redundant_boolean_if_expression.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.boolean_if.main:stmt:1:expr".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "simplify_redundant_boolean_if_expression"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_redundant_boolean_if_statement_simplify() {
    let source = include_str!("../examples/redundant_boolean_if_statement.sley");
    let program = parse_program(source).expect("parse redundant boolean if statement fixture");
    let report = build_edit_plan_report_with_options(
        "examples/redundant_boolean_if_statement.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "REDUNDANT_BOOLEAN_IF_STATEMENT"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_redundant_boolean_if_statement")
        .expect("redundant boolean if statement simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.boolean_if_statement.main:stmt:1"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceStatement"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("return ready"))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput = serde_json::from_value(template.operation.clone())
        .expect("parse redundant boolean if statement template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:boolean-if-statement-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return ready"));
    assert!(!grafted_source.contains("if ready"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::RedundantBooleanIfStatement],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/redundant_boolean_if_statement.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.boolean_if_statement.main:stmt:1".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "simplify_redundant_boolean_if_statement"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_same_branch_if_expression_simplify() {
    let source = include_str!("../examples/same_branch_if_expression.sley");
    let program = parse_program(source).expect("parse same branch if fixture");
    let report = build_edit_plan_report_with_options(
        "examples/same_branch_if_expression.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "SAME_BRANCH_IF_EXPRESSION"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_same_branch_if_expression")
        .expect("same branch if expression simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.same_branch_if.main:stmt:1:expr"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceExpression"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("42"))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput =
        serde_json::from_value(template.operation.clone()).expect("parse same branch if template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:same-branch-if-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return 42"));
    assert!(!grafted_source.contains("if use_backup"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::SameBranchIfExpression],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/same_branch_if_expression.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.same_branch_if.main:stmt:1:expr".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "simplify_same_branch_if_expression"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_graft_templates_include_same_branch_if_statement_simplify() {
    let source = include_str!("../examples/same_branch_if_statement.sley");
    let program = parse_program(source).expect("parse same branch if statement fixture");
    let report = build_edit_plan_report_with_options(
        "examples/same_branch_if_statement.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "SAME_BRANCH_IF_STATEMENT"
    );
    let template = report
        .graft_templates
        .iter()
        .find(|template| template.kind == "simplify_same_branch_if_statement")
        .expect("same branch if statement simplify template");
    assert_eq!(
        template.surface,
        "block:task:app.same_branch_if_statement.main:stmt:1"
    );
    assert_eq!(
        template.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceStatement"))
    );
    assert_eq!(
        template.operation.pointer("/payload/source"),
        Some(&serde_json::json!("return total"))
    );
    assert_eq!(template.editable_json_pointers, vec!["/payload/source"]);
    let graft: GraftInput = serde_json::from_value(template.operation.clone())
        .expect("parse same branch if statement template");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:same-branch-if-statement-template-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("return total"));
    assert!(!grafted_source.contains("if total > 0"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::SameBranchIfStatement],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/same_branch_if_statement.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some(
                "block:task:app.same_branch_if_statement.main:stmt:1".to_string(),
            ),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.graft_templates.len(), 1);
    assert_eq!(
        targeted_report.graft_templates[0].kind,
        "simplify_same_branch_if_statement"
    );
    assert!(targeted_report.transaction_templates.is_empty());
}

#[test]
fn edit_plan_transaction_templates_include_mutable_binding_conversion() {
    let source = include_str!("../examples/mutable_binding_style.sley");
    let program = parse_program(source).expect("parse mutable binding fixture");
    let report = build_edit_plan_report_with_options(
        "examples/mutable_binding_style.sley",
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: None,
        },
    );
    assert_eq!(report.status, "warnings");
    assert_eq!(report.summary.lint_finding_count, 1);
    assert_eq!(
        report.lint.as_ref().expect("lint summary").findings[0].id,
        "MUTABLE_BINDING_NEVER_SET"
    );
    let template = report
        .transaction_templates
        .iter()
        .find(|template| template.kind == "convert_mutable_binding_to_bind")
        .expect("mutable binding conversion transaction template");
    assert_eq!(template.surface, "block:task:app.mutable_style.main:stmt:0");
    assert_eq!(
        template.transaction.pointer("/ops/0/op"),
        Some(&serde_json::json!("DeleteNode"))
    );
    assert_eq!(
        template.transaction.pointer("/ops/0/target"),
        Some(&serde_json::json!(
            "block:task:app.mutable_style.main:stmt:0"
        ))
    );
    assert_eq!(
        template.transaction.pointer("/ops/1/op"),
        Some(&serde_json::json!("InsertStatement"))
    );
    assert_eq!(
        template.transaction.pointer("/ops/1/target"),
        Some(&serde_json::json!("block:task:app.mutable_style.main"))
    );
    assert_eq!(
        template.transaction.pointer("/ops/1/payload/source"),
        Some(&serde_json::json!("bind base = 21"))
    );
    assert_eq!(
        template.transaction.pointer("/ops/1/payload/position"),
        Some(&serde_json::json!(0))
    );
    assert_eq!(
        template.editable_json_pointers,
        vec!["/ops/1/payload/source".to_string()]
    );
    let graft: GraftInput = serde_json::from_value(template.transaction.clone())
        .expect("parse mutable binding conversion transaction");
    let outcome = apply_graft_input(
        &program,
        graft,
        Some("agent:mutable-binding-conversion-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("bind base = 21"));
    assert!(!grafted_source.contains("state base"));
    let grafted_program = parse_program(&grafted_source).expect("parse grafted source");
    let lint_report = build_lint_report(
        &grafted_program,
        LintOptions {
            rules: vec![LintRule::MutableBindingNeverSet],
            module: None,
        },
    );
    assert_eq!(lint_report.status, "ok");

    let targeted_report = build_edit_plan_report_with_options(
        "examples/mutable_binding_style.sley",
        Ok(program),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some("block:task:app.mutable_style.main:stmt:0".to_string()),
            module_name_hint: None,
        },
    );
    assert_eq!(targeted_report.status, "warnings");
    assert!(targeted_report.diagnostics.is_empty());
    assert_eq!(targeted_report.transaction_templates.len(), 1);
    assert_eq!(
        targeted_report.transaction_templates[0].kind,
        "convert_mutable_binding_to_bind"
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
            module_name_hint: None,
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
fn remove_task_effect_graft_is_structural_and_checked() {
    let source = r#"
module app.effects

task main -> Text uses FileRead, Network {
  return fs.read_text("examples/hello.sley")
}
"#;
    let program = parse_program(source).expect("parse effect graft fixture");
    let remove_unused_source = r#"
{
  "op": "RemoveTaskEffect",
  "target": "task:app.effects.main",
  "payload": {
    "name": "Network"
  }
}
"#;
    let remove_unused: GraftInput =
        serde_json::from_str(remove_unused_source).expect("parse remove unused effect graft");
    let outcome = apply_graft_input(
        &program,
        remove_unused,
        Some("agent:remove-effect-test".to_string()),
    );
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("uses FileRead"));
    assert!(!grafted_source.contains("Network"));
    assert_eq!(outcome.provenance[0].operation, "RemoveTaskEffect");
    assert_eq!(
        outcome.provenance[0].targets,
        vec!["task:app.effects.main", "effect:Network"]
    );

    let remove_required_source = r#"
{
  "op": "RemoveTaskEffect",
  "target": "task:app.effects.main",
  "payload": {
    "name": "FileRead"
  }
}
"#;
    let remove_required: GraftInput =
        serde_json::from_str(remove_required_source).expect("parse remove required effect graft");
    let rejected = apply_graft_input(
        &program,
        remove_required,
        Some("agent:remove-effect-test".to_string()),
    );
    assert_eq!(rejected.status, "rejected");
    assert!(
        rejected
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "EFFECT_UNAUTHORIZED"),
        "expected authority diagnostic, got {:#?}",
        rejected.diagnostics
    );
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
fn insert_statement_graft_adds_checked_nested_block_statement() {
    let source = r#"
task main -> Int {
  state total = 1
  if true {
    set total = total + 4
  }
  return total
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft_source = r#"
{
  "op": "InsertStatement",
  "target": "block:task:main.main:stmt:1:then",
  "payload": { "source": "set total = total + 37", "position": 1 }
}
"#;
    let graft: GraftInput = serde_json::from_str(graft_source).expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("set total = total + 37"));
    let grafted = parse_program(&grafted_source).expect("parse grafted source");
    assert_eq!(run_main(&grafted), Ok(Value::Int(42)));
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
fn replace_statement_graft_updates_checked_statement_source() {
    let source = r#"
task main -> Int {
  bind value = 1
  return value
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft: GraftInput = serde_json::from_str(
        r#"
{
  "op": "ReplaceStatement",
  "target": "block:task:main.main:stmt:0",
  "payload": { "source": "bind value = 42" }
}
"#,
    )
    .expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    assert_eq!(outcome.provenance[0].operation, "ReplaceStatement");
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("bind value = 42"));
    assert!(!grafted_source.contains("bind value = 1"));
    let grafted = parse_program(&grafted_source).expect("parse grafted source");
    assert_eq!(run_main(&grafted), Ok(Value::Int(42)));
}

#[test]
fn replace_statement_graft_updates_nested_statement_source() {
    let source = r#"
task main -> Int {
  tally value = 1
  if true {
    set value = 2
  }
  return value
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft: GraftInput = serde_json::from_str(
        r#"
{
  "op": "ReplaceStatement",
  "target": "block:task:main.main:stmt:1:then:stmt:0",
  "payload": { "source": "set value = 5" }
}
"#,
    )
    .expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    let grafted_source = outcome.source.expect("grafted source");
    assert!(grafted_source.contains("set value = 5"));
    assert!(!grafted_source.contains("set value = 2"));
    let grafted = parse_program(&grafted_source).expect("parse grafted source");
    assert_eq!(run_main(&grafted), Ok(Value::Int(5)));
}

#[test]
fn replace_statement_graft_rejects_multiple_statement_source() {
    let source = r#"
task main -> Int {
  bind value = 1
  return value
}
"#;
    let program = parse_program(source).expect("parse source");
    let graft: GraftInput = serde_json::from_str(
        r#"
{
  "op": "ReplaceStatement",
  "target": "block:task:main.main:stmt:0",
  "payload": { "source": "bind value = 42\nreturn value" }
}
"#,
    )
    .expect("parse graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));

    assert_eq!(outcome.status, "rejected");
    assert_eq!(outcome.diagnostics[0].id, "GRAFT_EXPECTED_ONE_STATEMENT");
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
fn add_module_declaration_graft_sets_explicit_module() {
    let source = r#"task main -> Text {
  return "hello"
}
"#;
    let program = parse_program(source).expect("parse module-less source");
    let graft: GraftInput = serde_json::from_str(
        r#"{ "op": "AddModuleDeclaration", "payload": { "name": "app.main" } }"#,
    )
    .expect("parse module declaration graft");
    let outcome = apply_graft_input(&program, graft, Some("agent:test".to_string()));
    assert_eq!(outcome.schema, GRAFT_OUTCOME_SCHEMA);
    assert_eq!(outcome.status, "accepted", "{:#?}", outcome.diagnostics);
    assert_eq!(outcome.provenance[0].operation, "AddModuleDeclaration");
    assert_eq!(
        outcome.provenance[0].targets,
        vec!["module:app.main".to_string()]
    );
    let grafted_source = outcome.source.expect("grafted module source");
    assert_eq!(
        grafted_source,
        "module app.main\n\ntask main -> Text {\n  return \"hello\"\n}\n"
    );
    let grafted = parse_program(&grafted_source).expect("parse grafted module source");
    assert_eq!(grafted.module.as_deref(), Some("app.main"));
    assert_eq!(grafted.tasks[0].id, "task:app.main.main");

    let existing = parse_program("module app.old\n\ntask main -> Unit {\n}\n")
        .expect("parse existing module source");
    let duplicate: GraftInput = serde_json::from_str(
        r#"{ "op": "AddModuleDeclaration", "payload": { "name": "app.main" } }"#,
    )
    .expect("parse duplicate module graft");
    let duplicate_outcome = apply_graft_input(&existing, duplicate, Some("agent:test".to_string()));
    assert_eq!(duplicate_outcome.status, "rejected");
    assert_has_repair_hint(
        &duplicate_outcome.diagnostics,
        "GRAFT_MODULE_EXISTS",
        "resolve_namespace_conflict",
    );
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
    let ast_node = program
        .ast_node_report("block:task:main.main:stmt:0:expr")
        .expect("minimal return expression AST node");
    assert_json_snapshot(
        &ast_node,
        include_str!("../fixtures/contracts/ast_node_return_expression.json"),
    );

    let diagnostics_program =
        parse_program("task main -> Int {\n  return missing\n}\n").expect("parse source");
    let report = DiagnosticReport::from_diagnostics(check_program(&diagnostics_program));
    assert_json_snapshot(
        &report,
        include_str!("../fixtures/contracts/diagnostic_report_unknown_identifier.json"),
    );

    let graft: GraftInput = serde_json::from_str(
        r#"
{
  "transaction": "txn_contract_insert",
  "actor": "agent:test",
  "mode": "all_or_nothing",
  "ops": [
    {
      "op": "InsertStatement",
      "target": "task:main.main",
      "payload": { "source": "bind extra = 41", "position": 0 }
    }
  ]
}
"#,
    )
    .expect("parse contract graft");
    let mut graft_outcome = apply_graft_input(&program, graft, None);
    assert_eq!(
        graft_outcome.status, "accepted",
        "{:#?}",
        graft_outcome.diagnostics
    );
    assert_eq!(graft_outcome.provenance.len(), 1);
    graft_outcome.provenance[0].timestamp = "2026-05-05T00:00:00Z".to_string();
    assert_json_snapshot(
        &graft_outcome,
        include_str!("../fixtures/contracts/graft_outcome_insert_statement.json"),
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

    let unchecked_result_source = r#"
module app.unchecked

task main -> Result<Text, Error> uses FileWrite {
  fs.try_write_text("out.txt", "ok")
  call helper()

  return Ok("ok")
}

task helper -> Result<Text, Error> {
  return Ok("helper")
}
"#;
    let unchecked_result_program =
        parse_program(unchecked_result_source).expect("parse unchecked result lint fixture");
    let unchecked_result_lint = build_lint_report(
        &unchecked_result_program,
        LintOptions {
            rules: vec![LintRule::UncheckedResult],
            module: None,
        },
    );
    assert_json_snapshot(
        &unchecked_result_lint,
        include_str!("../fixtures/contracts/lint_unchecked_result.json"),
    );

    let unqualified_import_project = load_project("examples/unqualified_import_call_project")
        .expect("load import style project");
    let unqualified_import_lint = build_lint_report(
        &unqualified_import_project.program,
        LintOptions {
            rules: vec![LintRule::UnqualifiedImportedCall],
            module: None,
        },
    );
    assert_json_snapshot(
        &unqualified_import_lint,
        include_str!("../fixtures/contracts/lint_unqualified_imported_call.json"),
    );

    let unused_pure_binding_source = include_str!("../examples/unused_pure_binding.sley");
    let unused_pure_binding_program =
        parse_program(unused_pure_binding_source).expect("parse unused pure binding fixture");
    let unused_pure_binding_lint = build_lint_report(
        &unused_pure_binding_program,
        LintOptions {
            rules: vec![LintRule::UnusedPureBinding],
            module: None,
        },
    );
    assert_json_snapshot(
        &unused_pure_binding_lint,
        include_str!("../fixtures/contracts/lint_unused_pure_binding.json"),
    );

    let unused_pure_expression_source =
        include_str!("../examples/unused_pure_expression_statement.sley");
    let unused_pure_expression_program = parse_program(unused_pure_expression_source)
        .expect("parse unused pure expression statement fixture");
    let unused_pure_expression_lint = build_lint_report(
        &unused_pure_expression_program,
        LintOptions {
            rules: vec![LintRule::UnusedPureExpressionStatement],
            module: None,
        },
    );
    assert_json_snapshot(
        &unused_pure_expression_lint,
        include_str!("../fixtures/contracts/lint_unused_pure_expression_statement.json"),
    );

    let unreachable_statement_source = include_str!("../examples/unreachable_statement.sley");
    let unreachable_statement_program =
        parse_program(unreachable_statement_source).expect("parse unreachable statement fixture");
    let unreachable_statement_lint = build_lint_report(
        &unreachable_statement_program,
        LintOptions {
            rules: vec![LintRule::UnreachableStatement],
            module: None,
        },
    );
    assert_json_snapshot(
        &unreachable_statement_lint,
        include_str!("../fixtures/contracts/lint_unreachable_statement.json"),
    );

    let mutable_binding_source = include_str!("../examples/mutable_binding_style.sley");
    let mutable_binding_program =
        parse_program(mutable_binding_source).expect("parse mutable binding fixture");
    let mutable_binding_lint = build_lint_report(
        &mutable_binding_program,
        LintOptions {
            rules: vec![LintRule::MutableBindingNeverSet],
            module: None,
        },
    );
    assert_json_snapshot(
        &mutable_binding_lint,
        include_str!("../fixtures/contracts/lint_mutable_binding_never_set.json"),
    );

    let self_assignment_source = include_str!("../examples/self_assignment_statement.sley");
    let self_assignment_program =
        parse_program(self_assignment_source).expect("parse self assignment fixture");
    let self_assignment_lint = build_lint_report(
        &self_assignment_program,
        LintOptions {
            rules: vec![LintRule::SelfAssignmentStatement],
            module: None,
        },
    );
    assert_json_snapshot(
        &self_assignment_lint,
        include_str!("../fixtures/contracts/lint_self_assignment_statement.json"),
    );

    let overwritten_set_source = include_str!("../examples/overwritten_set_statement.sley");
    let overwritten_set_program =
        parse_program(overwritten_set_source).expect("parse overwritten set fixture");
    let overwritten_set_lint = build_lint_report(
        &overwritten_set_program,
        LintOptions {
            rules: vec![LintRule::OverwrittenSetStatement],
            module: None,
        },
    );
    assert_json_snapshot(
        &overwritten_set_lint,
        include_str!("../fixtures/contracts/lint_overwritten_set_statement.json"),
    );

    let redundant_initial_set_source =
        include_str!("../examples/redundant_initial_set_statement.sley");
    let redundant_initial_set_program =
        parse_program(redundant_initial_set_source).expect("parse redundant initial set fixture");
    let redundant_initial_set_lint = build_lint_report(
        &redundant_initial_set_program,
        LintOptions {
            rules: vec![LintRule::RedundantInitialSetStatement],
            module: None,
        },
    );
    assert_json_snapshot(
        &redundant_initial_set_lint,
        include_str!("../fixtures/contracts/lint_redundant_initial_set_statement.json"),
    );

    let constant_if_source = include_str!("../examples/constant_if_expression.sley");
    let constant_if_program = parse_program(constant_if_source).expect("parse constant if fixture");
    let constant_if_lint = build_lint_report(
        &constant_if_program,
        LintOptions {
            rules: vec![LintRule::ConstantIfExpression],
            module: None,
        },
    );
    assert_json_snapshot(
        &constant_if_lint,
        include_str!("../fixtures/contracts/lint_constant_if_expression.json"),
    );

    let constant_if_statement_source = include_str!("../examples/constant_if_statement.sley");
    let constant_if_statement_program =
        parse_program(constant_if_statement_source).expect("parse constant if statement fixture");
    let constant_if_statement_lint = build_lint_report(
        &constant_if_statement_program,
        LintOptions {
            rules: vec![LintRule::ConstantIfStatement],
            module: None,
        },
    );
    assert_json_snapshot(
        &constant_if_statement_lint,
        include_str!("../fixtures/contracts/lint_constant_if_statement.json"),
    );

    let constant_false_if_source = include_str!("../examples/constant_false_if_statement.sley");
    let constant_false_if_program =
        parse_program(constant_false_if_source).expect("parse constant false if fixture");
    let constant_false_if_lint = build_lint_report(
        &constant_false_if_program,
        LintOptions {
            rules: vec![LintRule::ConstantFalseIfStatement],
            module: None,
        },
    );
    assert_json_snapshot(
        &constant_false_if_lint,
        include_str!("../fixtures/contracts/lint_constant_false_if_statement.json"),
    );

    let constant_false_while_source =
        include_str!("../examples/constant_false_while_statement.sley");
    let constant_false_while_program =
        parse_program(constant_false_while_source).expect("parse constant false while fixture");
    let constant_false_while_lint = build_lint_report(
        &constant_false_while_program,
        LintOptions {
            rules: vec![LintRule::ConstantFalseWhileStatement],
            module: None,
        },
    );
    assert_json_snapshot(
        &constant_false_while_lint,
        include_str!("../fixtures/contracts/lint_constant_false_while_statement.json"),
    );

    let constant_comparison_source =
        include_str!("../examples/constant_comparison_expression.sley");
    let constant_comparison_program =
        parse_program(constant_comparison_source).expect("parse constant comparison fixture");
    let constant_comparison_lint = build_lint_report(
        &constant_comparison_program,
        LintOptions {
            rules: vec![LintRule::ConstantComparisonExpression],
            module: None,
        },
    );
    assert_json_snapshot(
        &constant_comparison_lint,
        include_str!("../fixtures/contracts/lint_constant_comparison_expression.json"),
    );

    let constant_arithmetic_source =
        include_str!("../examples/constant_arithmetic_expression.sley");
    let constant_arithmetic_program =
        parse_program(constant_arithmetic_source).expect("parse constant arithmetic fixture");
    let constant_arithmetic_lint = build_lint_report(
        &constant_arithmetic_program,
        LintOptions {
            rules: vec![LintRule::ConstantArithmeticExpression],
            module: None,
        },
    );
    assert_json_snapshot(
        &constant_arithmetic_lint,
        include_str!("../fixtures/contracts/lint_constant_arithmetic_expression.json"),
    );

    let absorbing_arithmetic_source =
        include_str!("../examples/absorbing_arithmetic_expression.sley");
    let absorbing_arithmetic_program =
        parse_program(absorbing_arithmetic_source).expect("parse absorbing arithmetic fixture");
    let absorbing_arithmetic_lint = build_lint_report(
        &absorbing_arithmetic_program,
        LintOptions {
            rules: vec![LintRule::AbsorbingArithmeticExpression],
            module: None,
        },
    );
    assert_json_snapshot(
        &absorbing_arithmetic_lint,
        include_str!("../fixtures/contracts/lint_absorbing_arithmetic_expression.json"),
    );

    let constant_text_concatenation_source =
        include_str!("../examples/constant_text_concatenation_expression.sley");
    let constant_text_concatenation_program = parse_program(constant_text_concatenation_source)
        .expect("parse constant text concatenation fixture");
    let constant_text_concatenation_lint = build_lint_report(
        &constant_text_concatenation_program,
        LintOptions {
            rules: vec![LintRule::ConstantTextConcatenationExpression],
            module: None,
        },
    );
    assert_json_snapshot(
        &constant_text_concatenation_lint,
        include_str!("../fixtures/contracts/lint_constant_text_concatenation_expression.json"),
    );

    let constant_list_index_source =
        include_str!("../examples/constant_list_index_expression.sley");
    let constant_list_index_program =
        parse_program(constant_list_index_source).expect("parse constant list index fixture");
    let constant_list_index_lint = build_lint_report(
        &constant_list_index_program,
        LintOptions {
            rules: vec![LintRule::ConstantListIndexExpression],
            module: None,
        },
    );
    assert_json_snapshot(
        &constant_list_index_lint,
        include_str!("../fixtures/contracts/lint_constant_list_index_expression.json"),
    );

    let constant_map_index_source = include_str!("../examples/constant_map_index_expression.sley");
    let constant_map_index_program =
        parse_program(constant_map_index_source).expect("parse constant map index fixture");
    let constant_map_index_lint = build_lint_report(
        &constant_map_index_program,
        LintOptions {
            rules: vec![LintRule::ConstantMapIndexExpression],
            module: None,
        },
    );
    assert_json_snapshot(
        &constant_map_index_lint,
        include_str!("../fixtures/contracts/lint_constant_map_index_expression.json"),
    );

    let constant_record_field_access_source =
        include_str!("../examples/constant_record_field_access_expression.sley");
    let constant_record_field_access_program = parse_program(constant_record_field_access_source)
        .expect("parse constant record field access fixture");
    let constant_record_field_access_lint = build_lint_report(
        &constant_record_field_access_program,
        LintOptions {
            rules: vec![LintRule::ConstantRecordFieldAccessExpression],
            module: None,
        },
    );
    assert_json_snapshot(
        &constant_record_field_access_lint,
        include_str!("../fixtures/contracts/lint_constant_record_field_access_expression.json"),
    );

    let constant_len_source = include_str!("../examples/constant_len_expression.sley");
    let constant_len_program =
        parse_program(constant_len_source).expect("parse constant len fixture");
    let constant_len_lint = build_lint_report(
        &constant_len_program,
        LintOptions {
            rules: vec![LintRule::ConstantLenExpression],
            module: None,
        },
    );
    assert_json_snapshot(
        &constant_len_lint,
        include_str!("../fixtures/contracts/lint_constant_len_expression.json"),
    );

    let constant_not_source = include_str!("../examples/constant_not_expression.sley");
    let constant_not_program =
        parse_program(constant_not_source).expect("parse constant not fixture");
    let constant_not_lint = build_lint_report(
        &constant_not_program,
        LintOptions {
            rules: vec![LintRule::ConstantNotExpression],
            module: None,
        },
    );
    assert_json_snapshot(
        &constant_not_lint,
        include_str!("../fixtures/contracts/lint_constant_not_expression.json"),
    );

    let empty_if_source = include_str!("../examples/empty_if_statement.sley");
    let empty_if_program = parse_program(empty_if_source).expect("parse empty if fixture");
    let empty_if_lint = build_lint_report(
        &empty_if_program,
        LintOptions {
            rules: vec![LintRule::EmptyIfStatement],
            module: None,
        },
    );
    assert_json_snapshot(
        &empty_if_lint,
        include_str!("../fixtures/contracts/lint_empty_if_statement.json"),
    );

    let empty_else_source = include_str!("../examples/empty_else_statement.sley");
    let empty_else_program = parse_program(empty_else_source).expect("parse empty else fixture");
    let empty_else_lint = build_lint_report(
        &empty_else_program,
        LintOptions {
            rules: vec![LintRule::EmptyElseStatement],
            module: None,
        },
    );
    assert_json_snapshot(
        &empty_else_lint,
        include_str!("../fixtures/contracts/lint_empty_else_statement.json"),
    );

    let empty_for_source = include_str!("../examples/empty_for_statement.sley");
    let empty_for_program = parse_program(empty_for_source).expect("parse empty for fixture");
    let empty_for_lint = build_lint_report(
        &empty_for_program,
        LintOptions {
            rules: vec![LintRule::EmptyForStatement],
            module: None,
        },
    );
    assert_json_snapshot(
        &empty_for_lint,
        include_str!("../fixtures/contracts/lint_empty_for_statement.json"),
    );

    let empty_forge_source = include_str!("../examples/empty_forge_statement.sley");
    let empty_forge_program = parse_program(empty_forge_source).expect("parse empty forge fixture");
    let empty_forge_lint = build_lint_report(
        &empty_forge_program,
        LintOptions {
            rules: vec![LintRule::EmptyForgeStatement],
            module: None,
        },
    );
    assert_json_snapshot(
        &empty_forge_lint,
        include_str!("../fixtures/contracts/lint_empty_forge_statement.json"),
    );

    let identity_binary_source = include_str!("../examples/identity_binary_expression.sley");
    let identity_binary_program =
        parse_program(identity_binary_source).expect("parse identity binary fixture");
    let identity_binary_lint = build_lint_report(
        &identity_binary_program,
        LintOptions {
            rules: vec![LintRule::IdentityBinaryExpression],
            module: None,
        },
    );
    assert_json_snapshot(
        &identity_binary_lint,
        include_str!("../fixtures/contracts/lint_identity_binary_expression.json"),
    );

    let redundant_boolean_source = include_str!("../examples/redundant_boolean_comparison.sley");
    let redundant_boolean_program = parse_program(redundant_boolean_source)
        .expect("parse redundant boolean comparison fixture");
    let redundant_boolean_lint = build_lint_report(
        &redundant_boolean_program,
        LintOptions {
            rules: vec![LintRule::RedundantBooleanComparison],
            module: None,
        },
    );
    assert_json_snapshot(
        &redundant_boolean_lint,
        include_str!("../fixtures/contracts/lint_redundant_boolean_comparison.json"),
    );

    let absorbing_boolean_source = include_str!("../examples/absorbing_boolean_expression.sley");
    let absorbing_boolean_program =
        parse_program(absorbing_boolean_source).expect("parse absorbing boolean fixture");
    let absorbing_boolean_lint = build_lint_report(
        &absorbing_boolean_program,
        LintOptions {
            rules: vec![LintRule::AbsorbingBooleanExpression],
            module: None,
        },
    );
    assert_json_snapshot(
        &absorbing_boolean_lint,
        include_str!("../fixtures/contracts/lint_absorbing_boolean_expression.json"),
    );

    let idempotent_boolean_source = include_str!("../examples/idempotent_boolean_expression.sley");
    let idempotent_boolean_program =
        parse_program(idempotent_boolean_source).expect("parse idempotent boolean fixture");
    let idempotent_boolean_lint = build_lint_report(
        &idempotent_boolean_program,
        LintOptions {
            rules: vec![LintRule::IdempotentBooleanExpression],
            module: None,
        },
    );
    assert_json_snapshot(
        &idempotent_boolean_lint,
        include_str!("../fixtures/contracts/lint_idempotent_boolean_expression.json"),
    );

    let self_comparison_source = include_str!("../examples/self_comparison_expression.sley");
    let self_comparison_program =
        parse_program(self_comparison_source).expect("parse self-comparison fixture");
    let self_comparison_lint = build_lint_report(
        &self_comparison_program,
        LintOptions {
            rules: vec![LintRule::SelfComparisonExpression],
            module: None,
        },
    );
    assert_json_snapshot(
        &self_comparison_lint,
        include_str!("../fixtures/contracts/lint_self_comparison_expression.json"),
    );

    let double_negation_source = include_str!("../examples/double_negation_expression.sley");
    let double_negation_program =
        parse_program(double_negation_source).expect("parse double negation fixture");
    let double_negation_lint = build_lint_report(
        &double_negation_program,
        LintOptions {
            rules: vec![LintRule::DoubleNegationExpression],
            module: None,
        },
    );
    assert_json_snapshot(
        &double_negation_lint,
        include_str!("../fixtures/contracts/lint_double_negation_expression.json"),
    );

    let negated_comparison_source = include_str!("../examples/negated_comparison_expression.sley");
    let negated_comparison_program =
        parse_program(negated_comparison_source).expect("parse negated comparison fixture");
    let negated_comparison_lint = build_lint_report(
        &negated_comparison_program,
        LintOptions {
            rules: vec![LintRule::NegatedComparisonExpression],
            module: None,
        },
    );
    assert_json_snapshot(
        &negated_comparison_lint,
        include_str!("../fixtures/contracts/lint_negated_comparison_expression.json"),
    );

    let redundant_boolean_if_source =
        include_str!("../examples/redundant_boolean_if_expression.sley");
    let redundant_boolean_if_program =
        parse_program(redundant_boolean_if_source).expect("parse redundant boolean if fixture");
    let redundant_boolean_if_lint = build_lint_report(
        &redundant_boolean_if_program,
        LintOptions {
            rules: vec![LintRule::RedundantBooleanIfExpression],
            module: None,
        },
    );
    assert_json_snapshot(
        &redundant_boolean_if_lint,
        include_str!("../fixtures/contracts/lint_redundant_boolean_if_expression.json"),
    );

    let redundant_boolean_if_statement_source =
        include_str!("../examples/redundant_boolean_if_statement.sley");
    let redundant_boolean_if_statement_program =
        parse_program(redundant_boolean_if_statement_source)
            .expect("parse redundant boolean if statement fixture");
    let redundant_boolean_if_statement_lint = build_lint_report(
        &redundant_boolean_if_statement_program,
        LintOptions {
            rules: vec![LintRule::RedundantBooleanIfStatement],
            module: None,
        },
    );
    assert_json_snapshot(
        &redundant_boolean_if_statement_lint,
        include_str!("../fixtures/contracts/lint_redundant_boolean_if_statement.json"),
    );

    let same_branch_if_source = include_str!("../examples/same_branch_if_expression.sley");
    let same_branch_if_program =
        parse_program(same_branch_if_source).expect("parse same branch if fixture");
    let same_branch_if_lint = build_lint_report(
        &same_branch_if_program,
        LintOptions {
            rules: vec![LintRule::SameBranchIfExpression],
            module: None,
        },
    );
    assert_json_snapshot(
        &same_branch_if_lint,
        include_str!("../fixtures/contracts/lint_same_branch_if_expression.json"),
    );

    let same_branch_if_statement_source = include_str!("../examples/same_branch_if_statement.sley");
    let same_branch_if_statement_program = parse_program(same_branch_if_statement_source)
        .expect("parse same branch if statement fixture");
    let same_branch_if_statement_lint = build_lint_report(
        &same_branch_if_statement_program,
        LintOptions {
            rules: vec![LintRule::SameBranchIfStatement],
            module: None,
        },
    );
    assert_json_snapshot(
        &same_branch_if_statement_lint,
        include_str!("../fixtures/contracts/lint_same_branch_if_statement.json"),
    );

    let missing_module_source = r#"
task main -> Text {
  return "hello"
}
"#;
    let missing_module_program =
        parse_program(missing_module_source).expect("parse missing module lint fixture");
    let missing_module_lint = build_lint_report(
        &missing_module_program,
        LintOptions {
            rules: vec![LintRule::MissingModuleDeclaration],
            module: None,
        },
    );
    assert_json_snapshot(
        &missing_module_lint,
        include_str!("../fixtures/contracts/lint_missing_module_declaration.json"),
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
    let hello_graph = build_symbol_graph(&hello_program);
    assert_json_snapshot(
        &hello_graph,
        include_str!("../fixtures/contracts/symbol_graph_hello.json"),
    );
    let hello_run_value = run_main(&hello_program).expect("run hello fixture");
    let hello_run = build_run_report("examples/hello.sley", hello_run_value);
    assert_json_snapshot(
        &hello_run,
        include_str!("../fixtures/contracts/run_hello_ready.json"),
    );
    let trace_report_receipt = TraceReceipt {
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
    let trace_report = build_trace_report(
        "examples/hello.sley",
        "examples/.sley/trace.jsonl",
        vec![trace_report_receipt],
    );
    assert_json_snapshot(
        &trace_report,
        include_str!("../fixtures/contracts/trace_report_hello_receipt.json"),
    );
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
    let hello_verify = build_verify_report(
        "examples/hello.sley",
        Ok(hello_program.clone()),
        RuntimeGates::new(),
        true,
    );
    let hello_package = build_zjx_envelope("examples/hello.sley", hello_graph, None, Vec::new());
    assert_json_snapshot(
        &hello_package,
        include_str!("../fixtures/contracts/zjx_hello_ready.json"),
    );
    let deploy = build_deploy_report(
        "examples/hello.sley",
        "staging",
        hello_verify,
        Some(seal.clone()),
        Some(hello_package),
        Some(DeployArtifacts {
            directory: "artifacts".to_string(),
            report: "artifacts/deploy-report.json".to_string(),
            seal: "artifacts/seal.json".to_string(),
            package: "artifacts/zjx-envelope.json".to_string(),
            manifest: "artifacts/manifest.json".to_string(),
        }),
    );
    assert_json_snapshot(
        &deploy,
        include_str!("../fixtures/contracts/deploy_hello_ready.json"),
    );
    let deploy_artifacts = deploy.artifacts.clone().expect("deploy artifacts");
    let artifact_manifest = build_deploy_artifact_manifest(
        &deploy,
        &deploy_artifacts,
        "sha256:1111111111111111111111111111111111111111111111111111111111111111",
        "sha256:2222222222222222222222222222222222222222222222222222222222222222",
        "sha256:3333333333333333333333333333333333333333333333333333333333333333",
    );
    assert_json_snapshot(
        &artifact_manifest,
        include_str!("../fixtures/contracts/deploy_artifacts_hello.json"),
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
                "doctor".to_string(),
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
                "--deny-warnings".to_string(),
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
            vec![
                "sley".to_string(),
                "deploy".to_string(),
                "--json".to_string(),
                "--dry-run".to_string(),
                "--artifacts-dir".to_string(),
                ".sley/deploy".to_string(),
                "--cap".to_string(),
                "Deploy".to_string(),
                "--deploy-result".to_string(),
                "staging".to_string(),
                "staged".to_string(),
                ".".to_string(),
            ],
            vec![
                "sley".to_string(),
                "seal".to_string(),
                "--json".to_string(),
                ".".to_string(),
            ],
            vec![
                "sley".to_string(),
                "zjx".to_string(),
                "--json".to_string(),
                ".".to_string(),
            ],
        ],
        next_actions: vec![
            ScaffoldNextAction {
                kind: "check_project".to_string(),
                reason: "confirm the scaffold parses and passes strict checks".to_string(),
                command: vec![
                    "sley".to_string(),
                    "check".to_string(),
                    "--json".to_string(),
                    ".".to_string(),
                ],
            },
            ScaffoldNextAction {
                kind: "doctor_project".to_string(),
                reason:
                    "summarize strict checks, query facts, and lint gates in one readiness report"
                        .to_string(),
                command: vec![
                    "sley".to_string(),
                    "doctor".to_string(),
                    "--json".to_string(),
                    ".".to_string(),
                ],
            },
            ScaffoldNextAction {
                kind: "inspect_tasks".to_string(),
                reason: "read the entry task surface before editing".to_string(),
                command: vec![
                    "sley".to_string(),
                    "query".to_string(),
                    "--json".to_string(),
                    "--kind".to_string(),
                    "tasks".to_string(),
                    ".".to_string(),
                ],
            },
            ScaffoldNextAction {
                kind: "plan_first_edit".to_string(),
                reason: "ask Loom for ranked edit surfaces and post-edit gates".to_string(),
                command: vec![
                    "sley".to_string(),
                    "plan".to_string(),
                    "--json".to_string(),
                    ".".to_string(),
                ],
            },
            ScaffoldNextAction {
                kind: "lint_gate".to_string(),
                reason: "keep warning-grade hygiene strict before runtime work".to_string(),
                command: vec![
                    "sley".to_string(),
                    "lint".to_string(),
                    "--json".to_string(),
                    "--deny-warnings".to_string(),
                    ".".to_string(),
                ],
            },
            ScaffoldNextAction {
                kind: "verify_seeded_deploy".to_string(),
                reason: "verify deploy authority with a seeded provider result and denied warnings"
                    .to_string(),
                command: vec![
                    "sley".to_string(),
                    "verify".to_string(),
                    "--json".to_string(),
                    "--deny-warnings".to_string(),
                    "--cap".to_string(),
                    "Deploy".to_string(),
                    "--deploy-result".to_string(),
                    "staging".to_string(),
                    "staged".to_string(),
                    ".".to_string(),
                ],
            },
            ScaffoldNextAction {
                kind: "run_seeded_deploy".to_string(),
                reason: "execute the deploy-gated starter with deterministic seeded authority"
                    .to_string(),
                command: vec![
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
            },
            ScaffoldNextAction {
                kind: "prepare_deploy_package".to_string(),
                reason: "build the local dry-run deploy report after seeded verification"
                    .to_string(),
                command: vec![
                    "sley".to_string(),
                    "deploy".to_string(),
                    "--json".to_string(),
                    "--dry-run".to_string(),
                    "--artifacts-dir".to_string(),
                    ".sley/deploy".to_string(),
                    "--cap".to_string(),
                    "Deploy".to_string(),
                    "--deploy-result".to_string(),
                    "staging".to_string(),
                    "staged".to_string(),
                    ".".to_string(),
                ],
            },
            ScaffoldNextAction {
                kind: "seal_project".to_string(),
                reason: "create a content-addressed review artifact for the scaffold".to_string(),
                command: vec![
                    "sley".to_string(),
                    "seal".to_string(),
                    "--json".to_string(),
                    ".".to_string(),
                ],
            },
            ScaffoldNextAction {
                kind: "package_project".to_string(),
                reason: "create a ZJX preview envelope for agent handoff".to_string(),
                command: vec![
                    "sley".to_string(),
                    "zjx".to_string(),
                    "--json".to_string(),
                    ".".to_string(),
                ],
            },
        ],
    };
    assert_json_snapshot(
        &scaffold,
        include_str!("../fixtures/contracts/project_scaffold_deploy.json"),
    );
    let agent_scaffold_root = temp_project_dir("contract-agent-scaffold");
    let mut agent_scaffold = scaffold_project(
        &agent_scaffold_root,
        ScaffoldOptions {
            name: Some("agent-app".to_string()),
            module: "app.main".to_string(),
            template: ScaffoldTemplate::Agent,
        },
    )
    .unwrap_or_else(|diagnostics| panic!("agent scaffold diagnostics: {diagnostics:#?}"));
    agent_scaffold.project.root = "agent-app".to_string();
    assert_json_snapshot(
        &agent_scaffold,
        include_str!("../fixtures/contracts/project_scaffold_agent.json"),
    );
    let _ = fs::remove_dir_all(agent_scaffold_root);

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
            module_name_hint: None,
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
            module_name_hint: None,
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
    assert_eq!(verify.next_actions[0].kind, "seal_verified_target");
    assert_eq!(verify.next_actions[1].kind, "package_verified_target");
    assert_eq!(
        verify.next_actions[1].command,
        vec![
            "sley".to_string(),
            "zjx".to_string(),
            "--json".to_string(),
            "examples/project".to_string()
        ]
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
        include_str!("../docs/schemas/sley.ast.node.v0.schema.json"),
        AST_NODE_SCHEMA,
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.diagnostics.report.v0.schema.json"),
        DIAGNOSTIC_REPORT_SCHEMA,
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.run.report.v0.schema.json"),
        RUN_REPORT_SCHEMA,
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
        include_str!("../docs/schemas/sley.trace.receipt.v0.schema.json"),
        TRACE_RECEIPT_SCHEMA,
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.trace.report.v0.schema.json"),
        TRACE_REPORT_SCHEMA,
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.cli_smoke.manifest.v0.schema.json"),
        "sley.cli_smoke.manifest.v0",
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.conformance.manifest.v0.schema.json"),
        "sley.conformance.manifest.v0",
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.conformance.report.v0.schema.json"),
        "sley.conformance.report.v0",
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.conformance.coverage.v0.schema.json"),
        "sley.conformance.coverage.v0",
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.ci.report.v0.schema.json"),
        "sley.ci.report.v0",
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.contract.inventory.v0.schema.json"),
        "sley.contract.inventory.v0",
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.contract.fixture_check.v0.schema.json"),
        "sley.contract.fixture_check.v0",
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.contract.validate.v0.schema.json"),
        "sley.contract.validate.v0",
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
    assert_schema_file(
        include_str!("../docs/schemas/sley.deploy.report.v0.schema.json"),
        DEPLOY_REPORT_SCHEMA,
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.deploy.artifacts.v0.schema.json"),
        DEPLOY_ARTIFACTS_SCHEMA,
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.deploy.artifact_check.v0.schema.json"),
        "sley.deploy.artifact_check.v0",
    );
}

#[test]
fn ci_report_schema_covers_all_ci_commands() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../docs/schemas/sley.ci.report.v0.schema.json"
    ))
    .expect("parse CI report schema");
    assert_eq!(
        schema.pointer("/properties/command/enum"),
        Some(&serde_json::json!([
            "check", "lint", "doctor", "plan", "run", "verify", "deploy", "smoke", "corpus",
            "examples"
        ]))
    );
}

#[test]
fn deploy_artifact_schemas_pin_handoff_file_roles() {
    let manifest_schema: serde_json::Value = serde_json::from_str(include_str!(
        "../docs/schemas/sley.deploy.artifacts.v0.schema.json"
    ))
    .expect("parse deploy artifact manifest schema");
    assert_eq!(
        manifest_schema.pointer("/properties/files/properties/report/$ref"),
        Some(&serde_json::json!("#/$defs/reportFile"))
    );
    assert_eq!(
        manifest_schema.pointer("/$defs/reportFile/allOf/1/properties/schema/const"),
        Some(&serde_json::json!("sley.deploy.report.v0"))
    );
    assert_eq!(
        manifest_schema.pointer("/$defs/sealFile/allOf/1/properties/schema/const"),
        Some(&serde_json::json!("sley.trace.seal.v0"))
    );
    assert_eq!(
        manifest_schema.pointer("/$defs/packageFile/allOf/1/properties/schema/const"),
        Some(&serde_json::json!("sley.zjx.envelope.v0"))
    );

    let check_schema: serde_json::Value = serde_json::from_str(include_str!(
        "../docs/schemas/sley.deploy.artifact_check.v0.schema.json"
    ))
    .expect("parse deploy artifact check schema");
    assert_eq!(
        check_schema.pointer("/properties/manifest_schema/const"),
        Some(&serde_json::json!("sley.deploy.artifacts.v0"))
    );
    assert_eq!(
        check_schema.pointer("/$defs/file/properties/expected_schema/$ref"),
        Some(&serde_json::json!("#/$defs/deployArtifactSchema"))
    );
}

#[test]
fn contract_utility_inventories_schemas_and_validates_fixtures() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let inventory = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-contract"))
        .current_dir(&repo_root)
        .args(["inventory", "docs/schemas", "--json"])
        .output()
        .expect("run sley-contract inventory");
    assert!(
        inventory.status.success(),
        "inventory failed: {}",
        String::from_utf8_lossy(&inventory.stderr)
    );
    let inventory_json: serde_json::Value =
        serde_json::from_slice(&inventory.stdout).expect("parse inventory JSON");
    assert_json_snapshot(
        &inventory_json,
        include_str!("../fixtures/contracts/contract_inventory_schemas.json"),
    );
    assert_eq!(
        inventory_json.pointer("/schema"),
        Some(&serde_json::json!("sley.contract.inventory.v0"))
    );
    assert_eq!(
        inventory_json.pointer("/schema_count"),
        Some(&serde_json::json!(36))
    );
    let schema_ids = inventory_json
        .pointer("/schemas")
        .and_then(serde_json::Value::as_array)
        .expect("inventory schemas")
        .iter()
        .filter_map(|schema| schema.pointer("/id").and_then(serde_json::Value::as_str))
        .collect::<BTreeSet<_>>();
    assert!(schema_ids.contains("sley.query.report.v0"));
    assert!(schema_ids.contains("sley.ast.node.v0"));
    assert!(schema_ids.contains("sley.run.report.v0"));
    assert!(schema_ids.contains("sley.trace.report.v0"));
    assert!(schema_ids.contains("sley.ci.report.v0"));
    assert!(schema_ids.contains("sley.conformance.manifest.v0"));
    assert!(schema_ids.contains("sley.conformance.report.v0"));
    assert!(schema_ids.contains("sley.conformance.coverage.v0"));
    assert!(schema_ids.contains("sley.deploy.artifact_check.v0"));
    assert!(schema_ids.contains("sley.deploy.artifacts.v0"));
    assert!(schema_ids.contains("sley.deploy.report.v0"));
    assert!(schema_ids.contains("sley.docgen.report.v0"));
    assert!(schema_ids.contains("sley.contract.inventory.v0"));
    assert!(schema_ids.contains("sley.contract.fixture_check.v0"));
    assert!(schema_ids.contains("sley.contract.validate.v0"));
    assert!(schema_ids.contains("sley.agent_bench.report.v0"));
    assert!(schema_ids.contains("sley.migrate.report.v0"));
    assert!(schema_ids.contains("sley.sandbox.manifest.v0"));
    assert!(schema_ids.contains("sley.sandbox.report.v0"));
    assert!(schema_ids.contains("sley.lsp.fix_preview.v0"));
    assert!(schema_ids.contains("sley.workbench.report.v0"));
    assert!(schema_ids.contains("sley.zjx.tool.report.v0"));

    let fixture_check = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-contract"))
        .current_dir(&repo_root)
        .args([
            "check-fixtures",
            "fixtures/contracts",
            "--schemas",
            "docs/schemas",
            "--json",
        ])
        .output()
        .expect("run sley-contract check-fixtures");
    assert!(
        fixture_check.status.success(),
        "fixture check failed: {}",
        String::from_utf8_lossy(&fixture_check.stderr)
    );
    let fixture_json: serde_json::Value =
        serde_json::from_slice(&fixture_check.stdout).expect("parse fixture check JSON");
    assert_eq!(
        fixture_json.pointer("/schema"),
        Some(&serde_json::json!("sley.contract.fixture_check.v0"))
    );
    assert_eq!(
        fixture_json.pointer("/validation_level"),
        Some(&serde_json::json!("json_schema_draft_2020_12"))
    );
    assert_eq!(
        fixture_json.pointer("/fixture_count"),
        Some(&serde_json::json!(100))
    );
    assert_eq!(
        fixture_json.pointer("/failed_count"),
        Some(&serde_json::json!(0))
    );
    assert!(
        fixture_json
            .pointer("/fixtures")
            .and_then(serde_json::Value::as_array)
            .expect("fixture records")
            .iter()
            .all(|fixture| fixture
                .pointer("/issues")
                .and_then(serde_json::Value::as_array)
                .is_some_and(Vec::is_empty)),
        "passing fixtures should not carry validation issues"
    );

    let fixture_check_probe = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-contract"))
        .current_dir(&repo_root)
        .args([
            "check-fixtures",
            "fixtures/contract_probe",
            "--schemas",
            "docs/schemas",
            "--json",
        ])
        .output()
        .expect("run sley-contract check-fixtures probe");
    assert!(
        fixture_check_probe.status.success(),
        "fixture check probe failed: {}",
        String::from_utf8_lossy(&fixture_check_probe.stderr)
    );
    let fixture_check_probe_json: serde_json::Value =
        serde_json::from_slice(&fixture_check_probe.stdout)
            .expect("parse fixture check probe JSON");
    assert_json_snapshot(
        &fixture_check_probe_json,
        include_str!("../fixtures/contracts/contract_fixture_check_probe.json"),
    );

    let validate = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-contract"))
        .current_dir(&repo_root)
        .args([
            "validate",
            "--schema",
            "sley.query.report.v0",
            "fixtures/contracts/query_project_tasks.json",
            "--schemas",
            "docs/schemas",
            "--json",
        ])
        .output()
        .expect("run sley-contract validate");
    assert!(
        validate.status.success(),
        "validate failed: {}",
        String::from_utf8_lossy(&validate.stderr)
    );
    let validate_json: serde_json::Value =
        serde_json::from_slice(&validate.stdout).expect("parse validate JSON");
    assert_json_snapshot(
        &validate_json,
        include_str!("../fixtures/contracts/contract_validate_query_report.json"),
    );
    assert_eq!(
        validate_json.pointer("/schema"),
        Some(&serde_json::json!("sley.contract.validate.v0"))
    );
    assert_eq!(
        validate_json.pointer("/status"),
        Some(&serde_json::json!("passed"))
    );
    assert_eq!(
        validate_json.pointer("/validation_level"),
        Some(&serde_json::json!("json_schema_draft_2020_12"))
    );
    assert_eq!(
        validate_json.pointer("/issues"),
        Some(&serde_json::json!([]))
    );

    let artifact_root = temp_project_dir("contract-deploy-artifacts");
    let deploy_artifacts = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .current_dir(&repo_root)
        .args([
            "deploy",
            "--json",
            "--dry-run",
            "--artifacts-dir",
            &sley_string(&artifact_root),
            "examples/hello.sley",
        ])
        .output()
        .expect("build deploy artifacts");
    assert!(
        deploy_artifacts.status.success(),
        "deploy artifacts failed: {}",
        String::from_utf8_lossy(&deploy_artifacts.stderr)
    );
    let artifact_check = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-contract"))
        .current_dir(&repo_root)
        .args([
            "inspect-deploy-artifacts",
            &sley_string(&artifact_root),
            "--schemas",
            "docs/schemas",
            "--json",
        ])
        .output()
        .expect("inspect deploy artifacts");
    assert!(
        artifact_check.status.success(),
        "deploy artifact check failed: {}",
        String::from_utf8_lossy(&artifact_check.stderr)
    );
    let artifact_check_json: serde_json::Value =
        serde_json::from_slice(&artifact_check.stdout).expect("parse artifact check JSON");
    assert_eq!(
        artifact_check_json.pointer("/schema"),
        Some(&serde_json::json!("sley.deploy.artifact_check.v0"))
    );
    assert_eq!(
        artifact_check_json.pointer("/status"),
        Some(&serde_json::json!("passed"))
    );
    assert_eq!(
        artifact_check_json.pointer("/summary/file_count"),
        Some(&serde_json::json!(3))
    );
    assert_eq!(
        artifact_check_json.pointer("/summary/issue_count"),
        Some(&serde_json::json!(0))
    );
    assert_eq!(
        artifact_check_json.pointer("/files/0/role"),
        Some(&serde_json::json!("report"))
    );

    let seal_path = artifact_root.join("seal.json");
    let mut seal_source = fs::read_to_string(&seal_path).expect("read seal for tamper check");
    seal_source.push('\n');
    fs::write(&seal_path, seal_source).expect("tamper seal artifact digest");
    let tampered = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-contract"))
        .current_dir(&repo_root)
        .args([
            "inspect-deploy-artifacts",
            &sley_string(&artifact_root),
            "--schemas",
            "docs/schemas",
            "--json",
        ])
        .output()
        .expect("inspect tampered deploy artifacts");
    assert!(!tampered.status.success());
    let tampered_json: serde_json::Value =
        serde_json::from_slice(&tampered.stdout).expect("parse tampered artifact check JSON");
    assert_eq!(
        tampered_json.pointer("/status"),
        Some(&serde_json::json!("failed"))
    );
    assert_eq!(
        tampered_json.pointer("/files/1/issues/0/code"),
        Some(&serde_json::json!("digest_mismatch"))
    );
    let _ = fs::remove_dir_all(artifact_root);

    let invalid_report = temp_project_dir("contract-invalid-report");
    fs::create_dir_all(&invalid_report).expect("create invalid contract dir");
    let invalid = invalid_report.join("invalid_query.json");
    fs::write(
        &invalid,
        r#"{"schema":"sley.query.report.v0","kind":"tasks"}"#,
    )
    .expect("write invalid report");
    let invalid_validate = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-contract"))
        .current_dir(&repo_root)
        .args([
            "validate",
            "--schema",
            "sley.query.report.v0",
            &sley_string(&invalid),
            "--schemas",
            "docs/schemas",
            "--json",
        ])
        .output()
        .expect("run invalid sley-contract validate");
    assert!(!invalid_validate.status.success());
    let invalid_json: serde_json::Value =
        serde_json::from_slice(&invalid_validate.stdout).expect("parse invalid validate JSON");
    assert_eq!(
        invalid_json.pointer("/status"),
        Some(&serde_json::json!("failed"))
    );
    let invalid_issue_codes = invalid_json
        .pointer("/issues")
        .and_then(serde_json::Value::as_array)
        .expect("invalid report issues")
        .iter()
        .filter_map(|issue| issue.pointer("/code").and_then(serde_json::Value::as_str))
        .collect::<BTreeSet<_>>();
    assert!(invalid_issue_codes.contains("schema_validation_error"));
    let _ = fs::remove_dir_all(invalid_report);

    let malformed_root = temp_project_dir("contract-mismatch");
    fs::create_dir_all(&malformed_root).expect("create contract mismatch dir");
    let malformed = malformed_root.join("bad.json");
    fs::write(&malformed, r#"{"schema":"sley.missing.v0"}"#).expect("write mismatch report");
    let mismatch = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-contract"))
        .current_dir(&repo_root)
        .args([
            "validate",
            "--schema",
            "sley.query.report.v0",
            &sley_string(&malformed),
            "--schemas",
            "docs/schemas",
            "--json",
        ])
        .output()
        .expect("run mismatched sley-contract validate");
    assert!(!mismatch.status.success());
    let mismatch_json: serde_json::Value =
        serde_json::from_slice(&mismatch.stdout).expect("parse mismatch JSON");
    assert_eq!(
        mismatch_json.pointer("/status"),
        Some(&serde_json::json!("failed"))
    );
    assert_eq!(
        mismatch_json.pointer("/issues/0/code"),
        Some(&serde_json::json!("schema_mismatch"))
    );
    let _ = fs::remove_dir_all(malformed_root);
}

#[test]
fn conformance_report_summarizes_release_surface() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let report = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-conformance"))
        .current_dir(&repo_root)
        .args([
            "report",
            "--json",
            "--sley-contract-bin",
            env!("CARGO_BIN_EXE_sley-contract"),
        ])
        .output()
        .expect("run sley-conformance report");
    assert!(
        report.status.success(),
        "sley-conformance report failed: {}",
        String::from_utf8_lossy(&report.stderr)
    );
    let report_json: serde_json::Value =
        serde_json::from_slice(&report.stdout).expect("parse conformance report JSON");
    assert_eq!(
        report_json.pointer("/schema"),
        Some(&serde_json::json!("sley.conformance.report.v0"))
    );
    assert_eq!(
        report_json.pointer("/status"),
        Some(&serde_json::json!("passed"))
    );
    assert_eq!(
        report_json.pointer("/summary/schema_count"),
        Some(&serde_json::json!(36))
    );
    assert_eq!(
        report_json.pointer("/summary/schema_without_instance_count"),
        Some(&serde_json::json!(0))
    );
    assert_eq!(
        report_json.pointer("/summary/contract_fixture_count"),
        Some(&serde_json::json!(100))
    );
    assert_eq!(
        report_json.pointer("/summary/corpus_accepted_count"),
        Some(&serde_json::json!(17))
    );
    assert_eq!(
        report_json.pointer("/summary/corpus_rejected_count"),
        Some(&serde_json::json!(18))
    );
    assert_eq!(
        report_json.pointer("/summary/smoke_case_count"),
        Some(&serde_json::json!(377))
    );
    assert_eq!(
        report_json.pointer("/summary/example_source_count"),
        Some(&serde_json::json!(70))
    );
    assert_eq!(
        report_json.pointer("/summary/integration_test_count"),
        Some(&serde_json::json!(296))
    );
    assert_eq!(
        report_json.pointer("/summary/declared_integration_test_count"),
        Some(&serde_json::json!(296))
    );
    assert_eq!(
        report_json.pointer("/summary/test_count_matches_declared"),
        Some(&serde_json::json!(true))
    );
    assert_eq!(
        report_json.pointer("/summary/public_release_blocker_count"),
        Some(&serde_json::json!(5))
    );
    assert_eq!(
        report_json.pointer("/tests/integration_test_count"),
        Some(&serde_json::json!(296))
    );
    assert_eq!(
        report_json.pointer("/tests/declared_matches_actual"),
        Some(&serde_json::json!(true))
    );
    assert_eq!(
        report_json.pointer("/release/public_release_ready"),
        Some(&serde_json::json!(false))
    );
    assert_eq!(
        report_json.pointer("/release/blocker_count"),
        Some(&serde_json::json!(5))
    );
    assert_eq!(
        report_json.pointer("/release/cargo_package/rust_version"),
        Some(&serde_json::json!("1.85.0"))
    );
    assert_eq!(
        report_json.pointer("/release/cargo_package/publish"),
        Some(&serde_json::json!("false"))
    );
    assert_eq!(
        report_json.pointer("/release/license/operator_decision_required"),
        Some(&serde_json::json!(true))
    );
    assert_eq!(
        report_json.pointer("/release/blockers/0/code"),
        Some(&serde_json::json!("missing_license_file"))
    );
    assert_eq!(report_json.pointer("/issues"), Some(&serde_json::json!([])));

    let text_report = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-conformance"))
        .current_dir(&repo_root)
        .args([
            "report",
            "--sley-contract-bin",
            env!("CARGO_BIN_EXE_sley-contract"),
        ])
        .output()
        .expect("run text sley-conformance report");
    assert!(
        text_report.status.success(),
        "text sley-conformance report failed: {}",
        String::from_utf8_lossy(&text_report.stderr)
    );
    let text_stdout = String::from_utf8(text_report.stdout).expect("text report stdout is utf8");
    assert!(text_stdout.contains("public_release_blockers=5"));
    assert!(text_stdout.contains("release:missing_license_file"));

    let gated_report = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-conformance"))
        .current_dir(&repo_root)
        .args([
            "report",
            "--json",
            "--require-public-release-ready",
            "--sley-contract-bin",
            env!("CARGO_BIN_EXE_sley-contract"),
        ])
        .output()
        .expect("run gated sley-conformance report");
    assert!(
        !gated_report.status.success(),
        "gated sley-conformance report should fail while public release blockers remain"
    );
    let gated_json: serde_json::Value =
        serde_json::from_slice(&gated_report.stdout).expect("parse gated conformance report JSON");
    assert_eq!(
        gated_json.pointer("/status"),
        Some(&serde_json::json!("failed"))
    );
    assert_eq!(
        gated_json.pointer("/issues/0/code"),
        Some(&serde_json::json!("public_release_not_ready"))
    );
    assert_eq!(
        gated_json.pointer("/summary/issue_count"),
        Some(&serde_json::json!(1))
    );
    assert_eq!(
        gated_json.pointer("/summary/public_release_blocker_count"),
        Some(&serde_json::json!(5))
    );

    let coverage = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-conformance"))
        .current_dir(&repo_root)
        .args([
            "coverage",
            "--json",
            "--require-tag",
            "cli:check",
            "--require-tag",
            "json:sley.trace.receipt.v0",
        ])
        .output()
        .expect("run sley-conformance coverage");
    assert!(
        coverage.status.success(),
        "sley-conformance coverage failed: {}",
        String::from_utf8_lossy(&coverage.stderr)
    );
    let coverage_json: serde_json::Value =
        serde_json::from_slice(&coverage.stdout).expect("parse conformance coverage JSON");
    assert_eq!(
        coverage_json.pointer("/schema"),
        Some(&serde_json::json!("sley.conformance.coverage.v0"))
    );
    assert_eq!(
        coverage_json.pointer("/status"),
        Some(&serde_json::json!("passed"))
    );
    assert_eq!(
        coverage_json.pointer("/missing_tags"),
        Some(&serde_json::json!([]))
    );

    let directory_report = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-conformance"))
        .current_dir(&repo_root)
        .args([
            "report",
            "--json",
            "--corpus-manifest",
            "fixtures/corpus",
            "--smoke-manifest",
            "fixtures/cli_smokes",
            "--smoke-manifest",
            "fixtures/ci_smoke_probe",
            "--sley-contract-bin",
            env!("CARGO_BIN_EXE_sley-contract"),
        ])
        .output()
        .expect("run sley-conformance report with manifest directories");
    assert!(
        directory_report.status.success(),
        "directory conformance report failed: {}",
        String::from_utf8_lossy(&directory_report.stderr)
    );
    let directory_report_json: serde_json::Value = serde_json::from_slice(&directory_report.stdout)
        .expect("parse directory conformance report JSON");
    assert_eq!(
        directory_report_json.pointer("/status"),
        Some(&serde_json::json!("passed"))
    );
    assert_eq!(
        directory_report_json.pointer("/corpus/manifest"),
        Some(&serde_json::json!("fixtures/corpus/manifest.json"))
    );
    assert_eq!(
        directory_report_json.pointer("/smoke/manifests/1/path"),
        Some(&serde_json::json!("fixtures/ci_smoke_probe/manifest.json"))
    );

    let directory_coverage = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-conformance"))
        .current_dir(&repo_root)
        .args([
            "coverage",
            "--json",
            "--corpus-manifest",
            "fixtures/corpus",
            "--smoke-manifest",
            "fixtures/ci_smoke_probe",
            "--require-tag",
            "ci-smoke:multi-case",
            "--require-tag",
            "runtime:seeded-host-authority",
        ])
        .output()
        .expect("run sley-conformance coverage with manifest directories");
    assert!(
        directory_coverage.status.success(),
        "directory conformance coverage failed: {}",
        String::from_utf8_lossy(&directory_coverage.stderr)
    );
    let directory_coverage_json: serde_json::Value =
        serde_json::from_slice(&directory_coverage.stdout)
            .expect("parse directory conformance coverage JSON");
    assert_eq!(
        directory_coverage_json.pointer("/status"),
        Some(&serde_json::json!("passed"))
    );
}

#[test]
fn sley_ci_wraps_check_verify_and_smoke_manifest() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let check = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-ci"))
        .current_dir(&repo_root)
        .args(["check", "--json", "examples/project"])
        .output()
        .expect("run sley-ci check");
    assert!(
        check.status.success(),
        "sley-ci check failed: {}",
        String::from_utf8_lossy(&check.stderr)
    );
    let check_json: serde_json::Value =
        serde_json::from_slice(&check.stdout).expect("parse sley-ci check JSON");
    assert_eq!(
        check_json.pointer("/schema"),
        Some(&serde_json::json!("sley.ci.report.v0"))
    );
    assert_eq!(
        check_json.pointer("/status"),
        Some(&serde_json::json!("passed"))
    );
    assert_eq!(
        check_json.pointer("/command"),
        Some(&serde_json::json!("check"))
    );
    assert_eq!(
        check_json.pointer("/summary/step_count"),
        Some(&serde_json::json!(2))
    );
    assert_eq!(
        check_json.pointer("/steps/0/stdout_schema"),
        Some(&serde_json::json!("sley.diagnostics.report.v0"))
    );
    assert_eq!(
        check_json.pointer("/steps/1/stdout_schema"),
        Some(&serde_json::json!("sley.lint.report.v0"))
    );
    assert_json_snapshot(
        &check_json,
        include_str!("../fixtures/contracts/ci_check_project_ready.json"),
    );

    let lint = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-ci"))
        .current_dir(&repo_root)
        .args(["lint", "--json", "--deny-warnings", "examples/project"])
        .output()
        .expect("run sley-ci lint");
    assert!(
        lint.status.success(),
        "sley-ci lint failed: {}",
        String::from_utf8_lossy(&lint.stderr)
    );
    let lint_json: serde_json::Value =
        serde_json::from_slice(&lint.stdout).expect("parse sley-ci lint JSON");
    assert_eq!(
        lint_json.pointer("/schema"),
        Some(&serde_json::json!("sley.ci.report.v0"))
    );
    assert_eq!(
        lint_json.pointer("/status"),
        Some(&serde_json::json!("passed"))
    );
    assert_eq!(
        lint_json.pointer("/command"),
        Some(&serde_json::json!("lint"))
    );
    assert_eq!(
        lint_json.pointer("/steps/0/stdout_schema"),
        Some(&serde_json::json!("sley.lint.report.v0"))
    );
    assert_json_snapshot(
        &lint_json,
        include_str!("../fixtures/contracts/ci_lint_project_ready.json"),
    );

    let denied_lint = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-ci"))
        .current_dir(&repo_root)
        .args([
            "lint",
            "--json",
            "--deny-warnings",
            "--rule",
            "empty-for-statement",
            "examples/empty_for_statement.sley",
        ])
        .output()
        .expect("run denied sley-ci lint");
    assert!(
        !denied_lint.status.success(),
        "sley-ci lint with denied findings should fail"
    );
    let denied_lint_json: serde_json::Value =
        serde_json::from_slice(&denied_lint.stdout).expect("parse denied sley-ci lint JSON");
    assert_eq!(
        denied_lint_json.pointer("/schema"),
        Some(&serde_json::json!("sley.ci.report.v0"))
    );
    assert_eq!(
        denied_lint_json.pointer("/status"),
        Some(&serde_json::json!("failed"))
    );
    assert_eq!(
        denied_lint_json.pointer("/command"),
        Some(&serde_json::json!("lint"))
    );
    assert_eq!(
        denied_lint_json.pointer("/steps/0/stdout_schema"),
        Some(&serde_json::json!("sley.lint.report.v0"))
    );
    assert_eq!(
        denied_lint_json.pointer("/steps/0/issues/0/code"),
        Some(&serde_json::json!("exit_status_mismatch"))
    );
    assert_json_snapshot(
        &denied_lint_json,
        include_str!("../fixtures/contracts/ci_lint_denied_empty_for_statement.json"),
    );

    let doctor = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-ci"))
        .current_dir(&repo_root)
        .args(["doctor", "--json", "--deny-warnings", "examples/project"])
        .output()
        .expect("run sley-ci doctor");
    assert!(
        doctor.status.success(),
        "sley-ci doctor failed: {}",
        String::from_utf8_lossy(&doctor.stderr)
    );
    let doctor_json: serde_json::Value =
        serde_json::from_slice(&doctor.stdout).expect("parse sley-ci doctor JSON");
    assert_eq!(
        doctor_json.pointer("/schema"),
        Some(&serde_json::json!("sley.ci.report.v0"))
    );
    assert_eq!(
        doctor_json.pointer("/status"),
        Some(&serde_json::json!("passed"))
    );
    assert_eq!(
        doctor_json.pointer("/command"),
        Some(&serde_json::json!("doctor"))
    );
    assert_eq!(
        doctor_json.pointer("/steps/0/stdout_schema"),
        Some(&serde_json::json!("sley.doctor.report.v0"))
    );
    assert_json_snapshot(
        &doctor_json,
        include_str!("../fixtures/contracts/ci_doctor_project_ready.json"),
    );

    let denied_doctor = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-ci"))
        .current_dir(&repo_root)
        .args([
            "doctor",
            "--json",
            "--deny-warnings",
            "examples/empty_for_statement.sley",
        ])
        .output()
        .expect("run denied sley-ci doctor");
    assert!(
        !denied_doctor.status.success(),
        "sley-ci doctor with denied findings should fail"
    );
    let denied_doctor_json: serde_json::Value =
        serde_json::from_slice(&denied_doctor.stdout).expect("parse denied sley-ci doctor JSON");
    assert_eq!(
        denied_doctor_json.pointer("/schema"),
        Some(&serde_json::json!("sley.ci.report.v0"))
    );
    assert_eq!(
        denied_doctor_json.pointer("/status"),
        Some(&serde_json::json!("failed"))
    );
    assert_eq!(
        denied_doctor_json.pointer("/command"),
        Some(&serde_json::json!("doctor"))
    );
    assert_eq!(
        denied_doctor_json.pointer("/steps/0/stdout_schema"),
        Some(&serde_json::json!("sley.doctor.report.v0"))
    );
    assert_eq!(
        denied_doctor_json.pointer("/steps/0/issues/0/code"),
        Some(&serde_json::json!("exit_status_mismatch"))
    );
    assert_json_snapshot(
        &denied_doctor_json,
        include_str!("../fixtures/contracts/ci_doctor_denied_empty_for_statement.json"),
    );

    let plan = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-ci"))
        .current_dir(&repo_root)
        .args(["plan", "--json", "--graft-templates", "examples/project"])
        .output()
        .expect("run sley-ci plan");
    assert!(
        plan.status.success(),
        "sley-ci plan failed: {}",
        String::from_utf8_lossy(&plan.stderr)
    );
    let plan_json: serde_json::Value =
        serde_json::from_slice(&plan.stdout).expect("parse sley-ci plan JSON");
    assert_eq!(
        plan_json.pointer("/schema"),
        Some(&serde_json::json!("sley.ci.report.v0"))
    );
    assert_eq!(
        plan_json.pointer("/status"),
        Some(&serde_json::json!("passed"))
    );
    assert_eq!(
        plan_json.pointer("/command"),
        Some(&serde_json::json!("plan"))
    );
    assert_eq!(
        plan_json.pointer("/steps/0/stdout_schema"),
        Some(&serde_json::json!("sley.edit_plan.report.v0"))
    );
    assert_json_snapshot(
        &plan_json,
        include_str!("../fixtures/contracts/ci_plan_project_ready.json"),
    );

    let run = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-ci"))
        .current_dir(&repo_root)
        .args(["run", "--json", "examples/project"])
        .output()
        .expect("run sley-ci run");
    assert!(
        run.status.success(),
        "sley-ci run failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let run_json: serde_json::Value =
        serde_json::from_slice(&run.stdout).expect("parse sley-ci run JSON");
    assert_eq!(
        run_json.pointer("/schema"),
        Some(&serde_json::json!("sley.ci.report.v0"))
    );
    assert_eq!(
        run_json.pointer("/status"),
        Some(&serde_json::json!("passed"))
    );
    assert_eq!(
        run_json.pointer("/command"),
        Some(&serde_json::json!("run"))
    );
    assert_eq!(
        run_json.pointer("/steps/0/stdout_schema"),
        Some(&serde_json::json!(RUN_REPORT_SCHEMA))
    );
    assert_json_snapshot(
        &run_json,
        include_str!("../fixtures/contracts/ci_run_project_ready.json"),
    );

    let stable_verify = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-ci"))
        .current_dir(&repo_root)
        .args(["verify", "--json", "--deny-warnings", "examples/project"])
        .output()
        .expect("run stable sley-ci verify");
    assert!(
        stable_verify.status.success(),
        "stable sley-ci verify failed: {}",
        String::from_utf8_lossy(&stable_verify.stderr)
    );
    let stable_verify_json: serde_json::Value =
        serde_json::from_slice(&stable_verify.stdout).expect("parse stable sley-ci verify JSON");
    assert_json_snapshot(
        &stable_verify_json,
        include_str!("../fixtures/contracts/ci_verify_project_ready.json"),
    );

    let denied_verify = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-ci"))
        .current_dir(&repo_root)
        .args([
            "verify",
            "--json",
            "--deny-warnings",
            "examples/empty_for_statement.sley",
        ])
        .output()
        .expect("run denied sley-ci verify");
    assert!(
        !denied_verify.status.success(),
        "sley-ci verify with denied findings should fail"
    );
    let denied_verify_json: serde_json::Value =
        serde_json::from_slice(&denied_verify.stdout).expect("parse denied sley-ci verify JSON");
    assert_json_snapshot(
        &denied_verify_json,
        include_str!("../fixtures/contracts/ci_verify_denied_empty_for_statement.json"),
    );

    let stable_deploy = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-ci"))
        .current_dir(&repo_root)
        .args(["deploy", "--json", "--dry-run", "examples/project"])
        .output()
        .expect("run stable sley-ci deploy");
    assert!(
        stable_deploy.status.success(),
        "stable sley-ci deploy failed: {}",
        String::from_utf8_lossy(&stable_deploy.stderr)
    );
    let stable_deploy_json: serde_json::Value =
        serde_json::from_slice(&stable_deploy.stdout).expect("parse stable sley-ci deploy JSON");
    assert_json_snapshot(
        &stable_deploy_json,
        include_str!("../fixtures/contracts/ci_deploy_project_ready.json"),
    );

    let denied_plan = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-ci"))
        .current_dir(&repo_root)
        .args([
            "plan",
            "--json",
            "--deny-warnings",
            "--graft-templates",
            "examples/empty_for_statement.sley",
        ])
        .output()
        .expect("run denied sley-ci plan");
    assert!(
        !denied_plan.status.success(),
        "sley-ci plan with denied findings should fail"
    );
    let denied_plan_json: serde_json::Value =
        serde_json::from_slice(&denied_plan.stdout).expect("parse denied sley-ci plan JSON");
    assert_eq!(
        denied_plan_json.pointer("/schema"),
        Some(&serde_json::json!("sley.ci.report.v0"))
    );
    assert_eq!(
        denied_plan_json.pointer("/status"),
        Some(&serde_json::json!("failed"))
    );
    assert_eq!(
        denied_plan_json.pointer("/command"),
        Some(&serde_json::json!("plan"))
    );
    assert_eq!(
        denied_plan_json.pointer("/steps/0/stdout_schema"),
        Some(&serde_json::json!("sley.edit_plan.report.v0"))
    );
    assert_eq!(
        denied_plan_json.pointer("/steps/0/issues/0/code"),
        Some(&serde_json::json!("exit_status_mismatch"))
    );
    assert_json_snapshot(
        &denied_plan_json,
        include_str!("../fixtures/contracts/ci_plan_denied_empty_for_statement.json"),
    );

    let deploy_root = temp_project_dir("sley-ci-deploy");
    let scaffold = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .current_dir(&repo_root)
        .args([
            "new",
            "--json",
            "--template",
            "deploy",
            "--name",
            "agent-app",
            "--module",
            "app.main",
            &sley_string(&deploy_root),
        ])
        .output()
        .expect("run sley new for sley-ci");
    assert!(
        scaffold.status.success(),
        "sley new failed: {}",
        String::from_utf8_lossy(&scaffold.stderr)
    );
    let verify = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-ci"))
        .current_dir(&repo_root)
        .args([
            "verify",
            "--json",
            "--deny-warnings",
            "--cap",
            "Deploy",
            "--deploy-result",
            "staging",
            "staged",
            &sley_string(&deploy_root),
        ])
        .output()
        .expect("run sley-ci verify");
    assert!(
        verify.status.success(),
        "sley-ci verify failed: {}",
        String::from_utf8_lossy(&verify.stderr)
    );
    let verify_json: serde_json::Value =
        serde_json::from_slice(&verify.stdout).expect("parse sley-ci verify JSON");
    assert_eq!(
        verify_json.pointer("/schema"),
        Some(&serde_json::json!("sley.ci.report.v0"))
    );
    assert_eq!(
        verify_json.pointer("/status"),
        Some(&serde_json::json!("passed"))
    );
    assert_eq!(
        verify_json.pointer("/steps/0/stdout_schema"),
        Some(&serde_json::json!("sley.verify.report.v0"))
    );

    let seeded_run = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-ci"))
        .current_dir(&repo_root)
        .args([
            "run",
            "--json",
            "--cap",
            "Deploy",
            "--deploy-result",
            "staging",
            "staged",
            &sley_string(&deploy_root),
        ])
        .output()
        .expect("run seeded sley-ci run");
    assert!(
        seeded_run.status.success(),
        "seeded sley-ci run failed: {}",
        String::from_utf8_lossy(&seeded_run.stderr)
    );
    let seeded_run_json: serde_json::Value =
        serde_json::from_slice(&seeded_run.stdout).expect("parse seeded sley-ci run JSON");
    assert_eq!(
        seeded_run_json.pointer("/schema"),
        Some(&serde_json::json!("sley.ci.report.v0"))
    );
    assert_eq!(
        seeded_run_json.pointer("/status"),
        Some(&serde_json::json!("passed"))
    );
    assert_eq!(
        seeded_run_json.pointer("/command"),
        Some(&serde_json::json!("run"))
    );
    assert_eq!(
        seeded_run_json.pointer("/steps/0/stdout_schema"),
        Some(&serde_json::json!(RUN_REPORT_SCHEMA))
    );

    let ci_deploy_artifacts = deploy_root.join(".sley/ci-deploy");
    let ci_deploy_artifacts_arg = sley_string(&ci_deploy_artifacts);
    let deploy = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-ci"))
        .current_dir(&repo_root)
        .args([
            "deploy",
            "--json",
            "--dry-run",
            "--artifacts-dir",
            &ci_deploy_artifacts_arg,
            "--cap",
            "Deploy",
            "--deploy-result",
            "staging",
            "staged",
            &sley_string(&deploy_root),
        ])
        .output()
        .expect("run sley-ci deploy");
    assert!(
        deploy.status.success(),
        "sley-ci deploy failed: {}",
        String::from_utf8_lossy(&deploy.stderr)
    );
    let deploy_json: serde_json::Value =
        serde_json::from_slice(&deploy.stdout).expect("parse sley-ci deploy JSON");
    assert_eq!(
        deploy_json.pointer("/schema"),
        Some(&serde_json::json!("sley.ci.report.v0"))
    );
    assert_eq!(
        deploy_json.pointer("/status"),
        Some(&serde_json::json!("passed"))
    );
    assert_eq!(
        deploy_json.pointer("/command"),
        Some(&serde_json::json!("deploy"))
    );
    assert_eq!(
        deploy_json.pointer("/steps/0/stdout_schema"),
        Some(&serde_json::json!(DEPLOY_REPORT_SCHEMA))
    );
    assert!(ci_deploy_artifacts.join("deploy-report.json").exists());
    assert!(ci_deploy_artifacts.join("seal.json").exists());
    assert!(ci_deploy_artifacts.join("zjx-envelope.json").exists());

    let smoke_root = temp_project_dir("sley-ci-smoke");
    fs::create_dir_all(&smoke_root).expect("create sley-ci smoke root");
    let smoke_manifest = smoke_root.join("manifest.json");
    fs::write(
        &smoke_manifest,
        r#"{
  "schema": "sley.cli_smoke.manifest.v0",
  "cases": [
    {
      "name": "parse_hello_json",
      "args": ["parse", "--json", "{repo}/examples/hello.sley"],
      "covers": ["cli:parse", "json:sley.ast.program.v0"],
      "expect": {
        "success": true,
        "stdout_json": [
          { "pointer": "/schema", "value": "sley.ast.program.v0" },
          { "pointer": "/tasks/0/name", "value": "main" }
        ]
      }
    },
    {
      "name": "parse_hello_from_tmp_cwd_json",
      "cwd": "tmp",
      "args": ["parse", "--json", "{repo}/examples/hello.sley"],
      "covers": ["cli:parse", "json:sley.ast.program.v0", "cli:smoke-relative-repo-root"],
      "expect": {
        "success": true,
        "stdout_json": [
          { "pointer": "/schema", "value": "sley.ast.program.v0" },
          { "pointer": "/tasks/0/name", "value": "main" }
        ]
      }
    }
  ]
}
"#,
    )
    .expect("write sley-ci smoke manifest");
    let smoke = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-ci"))
        .current_dir(&repo_root)
        .args([
            "smoke",
            "--json",
            "--repo-root",
            ".",
            &sley_string(&smoke_manifest),
        ])
        .output()
        .expect("run sley-ci smoke");
    assert!(
        smoke.status.success(),
        "sley-ci smoke failed: {}",
        String::from_utf8_lossy(&smoke.stderr)
    );
    let smoke_json: serde_json::Value =
        serde_json::from_slice(&smoke.stdout).expect("parse sley-ci smoke JSON");
    assert_eq!(
        smoke_json.pointer("/schema"),
        Some(&serde_json::json!("sley.ci.report.v0"))
    );
    assert_eq!(
        smoke_json.pointer("/status"),
        Some(&serde_json::json!("passed"))
    );
    assert_eq!(
        smoke_json.pointer("/command"),
        Some(&serde_json::json!("smoke"))
    );
    assert_eq!(
        smoke_json.pointer("/steps/0/covers/0"),
        Some(&serde_json::json!("cli:parse"))
    );
    assert_eq!(
        smoke_json.pointer("/steps/0/stdout_schema"),
        Some(&serde_json::json!("sley.ast.program.v0"))
    );
    assert_eq!(
        smoke_json.pointer("/steps/1/name"),
        Some(&serde_json::json!("parse_hello_from_tmp_cwd_json"))
    );
    assert_eq!(
        smoke_json.pointer("/steps/1/stdout_schema"),
        Some(&serde_json::json!("sley.ast.program.v0"))
    );

    let stable_smoke = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-ci"))
        .current_dir(&repo_root)
        .args([
            "smoke",
            "--json",
            "--repo-root",
            ".",
            "fixtures/ci_smoke_probe/manifest.json",
        ])
        .output()
        .expect("run stable sley-ci smoke");
    assert!(
        stable_smoke.status.success(),
        "stable sley-ci smoke failed: {}",
        String::from_utf8_lossy(&stable_smoke.stderr)
    );
    let stable_smoke_json: serde_json::Value =
        serde_json::from_slice(&stable_smoke.stdout).expect("parse stable sley-ci smoke JSON");
    assert_json_snapshot(
        &stable_smoke_json,
        include_str!("../fixtures/contracts/ci_smoke_probe_ready.json"),
    );

    let stable_smoke_dir = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-ci"))
        .current_dir(&repo_root)
        .args([
            "smoke",
            "--json",
            "--repo-root",
            ".",
            "fixtures/ci_smoke_probe",
        ])
        .output()
        .expect("run stable sley-ci smoke directory form");
    assert!(
        stable_smoke_dir.status.success(),
        "stable sley-ci smoke directory form failed: {}",
        String::from_utf8_lossy(&stable_smoke_dir.stderr)
    );
    let stable_smoke_dir_json: serde_json::Value = serde_json::from_slice(&stable_smoke_dir.stdout)
        .expect("parse stable sley-ci smoke dir JSON");
    assert_eq!(
        stable_smoke_dir_json.pointer("/status"),
        Some(&serde_json::json!("passed"))
    );
    assert_eq!(
        stable_smoke_dir_json.pointer("/manifest"),
        Some(&serde_json::json!("fixtures/ci_smoke_probe/manifest.json"))
    );
    assert_eq!(
        stable_smoke_dir_json.pointer("/summary/step_count"),
        Some(&serde_json::json!(4))
    );

    let corpus = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-ci"))
        .current_dir(&repo_root)
        .args(["corpus", "--json", "fixtures/corpus/manifest.json"])
        .output()
        .expect("run sley-ci corpus");
    assert!(
        corpus.status.success(),
        "sley-ci corpus failed: {}",
        String::from_utf8_lossy(&corpus.stderr)
    );
    let corpus_json: serde_json::Value =
        serde_json::from_slice(&corpus.stdout).expect("parse sley-ci corpus JSON");
    assert_eq!(
        corpus_json.pointer("/schema"),
        Some(&serde_json::json!("sley.ci.report.v0"))
    );
    assert_eq!(
        corpus_json.pointer("/status"),
        Some(&serde_json::json!("passed"))
    );
    assert_eq!(
        corpus_json.pointer("/command"),
        Some(&serde_json::json!("corpus"))
    );
    assert_eq!(
        corpus_json.pointer("/summary/step_count"),
        Some(&serde_json::json!(52))
    );
    assert_eq!(
        corpus_json.pointer("/steps/0/name"),
        Some(&serde_json::json!(
            "accepted_check:accepted/authority/database_read.sley"
        ))
    );
    assert_eq!(
        corpus_json.pointer("/steps/0/stdout_schema"),
        Some(&serde_json::json!("sley.diagnostics.report.v0"))
    );
    assert_eq!(
        corpus_json.pointer("/steps/1/name"),
        Some(&serde_json::json!(
            "accepted_format_round_trip:accepted/authority/database_read.sley"
        ))
    );
    assert_eq!(
        corpus_json.pointer("/steps/34/name"),
        Some(&serde_json::json!(
            "rejected_check:rejected/authority/missing_database_read_effect.sley"
        ))
    );
    assert_eq!(
        corpus_json.pointer("/steps/34/stdout_schema"),
        Some(&serde_json::json!("sley.diagnostics.report.v0"))
    );

    let corpus_dir = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-ci"))
        .current_dir(&repo_root)
        .args(["corpus", "--json", "fixtures/corpus"])
        .output()
        .expect("run sley-ci corpus directory form");
    assert!(
        corpus_dir.status.success(),
        "sley-ci corpus directory form failed: {}",
        String::from_utf8_lossy(&corpus_dir.stderr)
    );
    let corpus_dir_json: serde_json::Value =
        serde_json::from_slice(&corpus_dir.stdout).expect("parse sley-ci corpus dir JSON");
    assert_eq!(
        corpus_dir_json.pointer("/status"),
        Some(&serde_json::json!("passed"))
    );
    assert_eq!(
        corpus_dir_json.pointer("/manifest"),
        Some(&serde_json::json!("fixtures/corpus/manifest.json"))
    );
    assert_eq!(
        corpus_dir_json.pointer("/summary/step_count"),
        Some(&serde_json::json!(52))
    );

    let examples = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-ci"))
        .current_dir(&repo_root)
        .args(["examples", "--json", "examples"])
        .output()
        .expect("run sley-ci examples");
    assert!(
        examples.status.success(),
        "sley-ci examples failed: {}",
        String::from_utf8_lossy(&examples.stderr)
    );
    let examples_json: serde_json::Value =
        serde_json::from_slice(&examples.stdout).expect("parse sley-ci examples JSON");
    assert_eq!(
        examples_json.pointer("/schema"),
        Some(&serde_json::json!("sley.ci.report.v0"))
    );
    assert_eq!(
        examples_json.pointer("/status"),
        Some(&serde_json::json!("passed"))
    );
    assert_eq!(
        examples_json.pointer("/command"),
        Some(&serde_json::json!("examples"))
    );
    assert_eq!(
        examples_json.pointer("/summary/step_count"),
        Some(&serde_json::json!(135))
    );
    assert_eq!(
        examples_json.pointer("/steps/0/name"),
        Some(&serde_json::json!("project_check:examples/agent_project"))
    );
    assert_eq!(
        examples_json.pointer("/steps/0/stdout_schema"),
        Some(&serde_json::json!("sley.diagnostics.report.v0"))
    );
    assert_eq!(
        examples_json.pointer("/steps/1/name"),
        Some(&serde_json::json!("project_check:examples/project"))
    );
    assert_eq!(
        examples_json.pointer("/steps/4/name"),
        Some(&serde_json::json!(
            "file_check:examples/absorbing_arithmetic_expression.sley"
        ))
    );
    assert_eq!(
        examples_json.pointer("/steps/65/name"),
        Some(&serde_json::json!(
            "format_round_trip:examples/absorbing_arithmetic_expression.sley"
        ))
    );

    let _ = fs::remove_dir_all(deploy_root);
    let _ = fs::remove_dir_all(smoke_root);
}

#[test]
fn trace_receipt_schema_covers_strict_provenance_records() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../docs/schemas/sley.trace.receipt.v0.schema.json"
    ))
    .expect("parse trace receipt schema");
    assert_eq!(
        schema.pointer("/required"),
        Some(&serde_json::json!([
            "schema",
            "target",
            "written_at",
            "provenance"
        ]))
    );
    assert_eq!(
        schema.pointer("/additionalProperties"),
        Some(&serde_json::json!(false))
    );
    assert_eq!(
        schema.pointer("/properties/schema/const"),
        Some(&serde_json::json!(TRACE_RECEIPT_SCHEMA))
    );
    assert_eq!(
        schema.pointer("/properties/target/minLength"),
        Some(&serde_json::json!(1))
    );
    assert_eq!(
        schema.pointer("/properties/written_at/minLength"),
        Some(&serde_json::json!(1))
    );
    assert_eq!(
        schema.pointer("/properties/provenance/items/$ref"),
        Some(&serde_json::json!(
            "sley.graft.outcome.v0#/$defs/provenanceRecord"
        ))
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
    assert_eq!(report.next_actions[1].kind, "plan_lint_repairs");
    assert_eq!(
        report.next_actions[1].command,
        vec!["sley", "plan", "--json", "--graft-templates", "app.effects"]
    );
    assert_eq!(report.next_actions[2].kind, "preview_lint_repair");
    assert_eq!(
        report.next_actions[2].command,
        vec![
            "sley",
            "fix",
            "--json",
            "--kind",
            "remove_unused_declared_effect",
            "--template-surface",
            "effect-use:task:app.effects.main:0:Network",
            "--dry-run",
            "app.effects"
        ]
    );
    assert_eq!(
        report.next_actions[2]
            .write_command
            .as_ref()
            .expect("write command"),
        &vec![
            "sley".to_string(),
            "fix".to_string(),
            "--json".to_string(),
            "--kind".to_string(),
            "remove_unused_declared_effect".to_string(),
            "--template-surface".to_string(),
            "effect-use:task:app.effects.main:0:Network".to_string(),
            "--write".to_string(),
            "app.effects".to_string()
        ]
    );

    let ambiguous = parse_program(
        r#"
module app.effects

task main -> Text uses Network, Shell {
  return "ready"
}
"#,
    )
    .expect("parse ambiguous unused effects");
    let ambiguous_report =
        build_verify_report("app.effects", Ok(ambiguous), RuntimeGates::new(), true);
    assert_eq!(ambiguous_report.summary.lint_finding_count, 2);
    assert!(
        !ambiguous_report
            .next_actions
            .iter()
            .any(|action| action.kind == "preview_lint_repair"),
        "ambiguous verify lint repair reports should require planning first"
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
fn diagnostic_schema_is_shared_by_agent_reports() {
    let diagnostics_schema: serde_json::Value = serde_json::from_str(include_str!(
        "../docs/schemas/sley.diagnostics.report.v0.schema.json"
    ))
    .expect("parse diagnostics schema");
    assert_eq!(
        diagnostics_schema.pointer("/properties/diagnostics/items/$ref"),
        Some(&serde_json::json!("#/$defs/diagnostic"))
    );
    assert_eq!(
        diagnostics_schema.pointer("/$defs/diagnostic/additionalProperties"),
        Some(&serde_json::json!(false))
    );
    assert_eq!(
        diagnostics_schema.pointer("/$defs/diagnostic/properties/id/minLength"),
        Some(&serde_json::json!(1))
    );
    assert_eq!(
        diagnostics_schema.pointer("/$defs/diagnostic/properties/severity/enum"),
        Some(&serde_json::json!(["error", "warning", "info"]))
    );
    assert_eq!(
        diagnostics_schema.pointer("/$defs/diagnostic/properties/repair_hints/items/$ref"),
        Some(&serde_json::json!("#/$defs/repair_hint"))
    );
    assert_eq!(
        diagnostics_schema.pointer("/$defs/repair_hint/properties/kind/minLength"),
        Some(&serde_json::json!(1))
    );

    for (schema_name, schema_source, pointer) in [
        (
            "graft outcome",
            include_str!("../docs/schemas/sley.graft.outcome.v0.schema.json"),
            "/properties/diagnostics/items/$ref",
        ),
        (
            "doctor report",
            include_str!("../docs/schemas/sley.doctor.report.v0.schema.json"),
            "/properties/diagnostics/items/$ref",
        ),
        (
            "edit-plan report",
            include_str!("../docs/schemas/sley.edit_plan.report.v0.schema.json"),
            "/properties/diagnostics/items/$ref",
        ),
        (
            "verify report",
            include_str!("../docs/schemas/sley.verify.report.v0.schema.json"),
            "/properties/diagnostics/items/$ref",
        ),
        (
            "verify runtime summary",
            include_str!("../docs/schemas/sley.verify.report.v0.schema.json"),
            "/$defs/runtimeSummary/properties/diagnostics/items/$ref",
        ),
    ] {
        let schema: serde_json::Value =
            serde_json::from_str(schema_source).unwrap_or_else(|error| {
                panic!("parse {schema_name} schema for diagnostic ref: {error}")
            });
        assert_eq!(
            schema.pointer(pointer),
            Some(&serde_json::json!(
                "sley.diagnostics.report.v0#/$defs/diagnostic"
            )),
            "{schema_name} should reference the shared diagnostic schema"
        );
    }
}

#[test]
fn verify_report_schema_reuses_strict_run_value_contract() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../docs/schemas/sley.verify.report.v0.schema.json"
    ))
    .expect("parse verify report schema");
    assert_eq!(
        schema.pointer("/$defs/runtimeSummary/properties/value/$ref"),
        Some(&serde_json::json!("sley.run.report.v0#/$defs/value"))
    );
}

#[test]
fn query_report_schema_exposes_strict_row_definitions() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../docs/schemas/sley.query.report.v0.schema.json"
    ))
    .expect("parse query report schema");
    assert_eq!(
        schema.pointer("/properties/entry_module/minLength"),
        Some(&serde_json::json!(1))
    );
    assert_eq!(
        schema.pointer("/properties/tasks/items/$ref"),
        Some(&serde_json::json!("#/$defs/queryTaskSummary"))
    );
    assert_eq!(
        schema.pointer("/properties/types/items/$ref"),
        Some(&serde_json::json!("#/$defs/queryTypeSummary"))
    );
    assert_eq!(
        schema.pointer("/properties/effects/items/$ref"),
        Some(&serde_json::json!("#/$defs/queryEffectSummary"))
    );
    assert_eq!(
        schema.pointer("/$defs/queryTaskSummary/additionalProperties"),
        Some(&serde_json::json!(false))
    );
    assert_eq!(
        schema.pointer("/$defs/queryTaskSummary/properties/takes/items/$ref"),
        Some(&serde_json::json!("#/$defs/queryTakeSummary"))
    );
    assert_eq!(
        schema.pointer("/$defs/queryTypeSummary/properties/fields/items/$ref"),
        Some(&serde_json::json!("#/$defs/queryTypeFieldSummary"))
    );
    assert_eq!(
        schema.pointer("/$defs/callSummary/additionalProperties"),
        Some(&serde_json::json!(false))
    );
    assert_eq!(
        schema.pointer("/$defs/callSummary/properties/status/enum"),
        Some(&serde_json::json!([
            "resolved",
            "unknown",
            "ambiguous",
            "private",
            "intrinsic"
        ]))
    );
    assert_eq!(
        schema.pointer("/$defs/callSummary/properties/expr_id/minLength"),
        Some(&serde_json::json!(1))
    );
}

#[test]
fn graph_slice_schema_reuses_strict_graft_operation_affordances() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../docs/schemas/sley.symbol_graph.slice.v0.schema.json"
    ))
    .expect("parse graph slice schema");
    let operation_ref = serde_json::json!("sley.edit_plan.report.v0#/$defs/graftOperation");
    for pointer in [
        "/$defs/insertAffordance/properties/operation/$ref",
        "/$defs/moveAffordance/properties/operation/$ref",
        "/$defs/moveDestination/properties/operation/$ref",
        "/$defs/deleteAffordance/properties/operation/$ref",
        "/$defs/replaceAffordance/properties/operation/$ref",
    ] {
        assert_eq!(
            schema.pointer(pointer),
            Some(&operation_ref),
            "graph slice operation at {pointer} should use the strict graft operation schema"
        );
    }
}

#[test]
fn graph_slice_schema_covers_focus_task_and_call_summaries() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../docs/schemas/sley.symbol_graph.slice.v0.schema.json"
    ))
    .expect("parse graph slice schema");
    assert_eq!(
        schema.pointer("/properties/focus/$ref"),
        Some(&serde_json::json!("#/$defs/sliceFocus"))
    );
    assert_eq!(
        schema.pointer("/properties/insert_affordances/items/$ref"),
        Some(&serde_json::json!("#/$defs/insertAffordance"))
    );
    assert_eq!(
        schema.pointer("/$defs/insertAffordance/properties/target_kind/enum"),
        Some(&serde_json::json!(["block"]))
    );
    assert_eq!(
        schema.pointer("/$defs/replaceAffordance/properties/target_kind/enum/0"),
        Some(&serde_json::json!("statement"))
    );
    assert_eq!(
        schema.pointer("/$defs/sliceFocus/additionalProperties"),
        Some(&serde_json::json!(false))
    );
    assert_eq!(
        schema.pointer("/$defs/sliceFocus/properties/kind/enum"),
        Some(&serde_json::json!([
            "module", "task", "type", "effect", "import"
        ]))
    );
    assert_eq!(
        schema.pointer("/properties/task/$ref"),
        Some(&serde_json::json!("sley.ast.program.v0#/$defs/taskDecl"))
    );
    assert_eq!(
        schema.pointer("/properties/outbound_calls/items/$ref"),
        Some(&serde_json::json!(
            "sley.query.report.v0#/$defs/callSummary"
        ))
    );
    assert_eq!(
        schema.pointer("/properties/inbound_calls/items/$ref"),
        Some(&serde_json::json!(
            "sley.query.report.v0#/$defs/callSummary"
        ))
    );
}

#[test]
fn edit_plan_schema_covers_strict_graft_template_payloads() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../docs/schemas/sley.edit_plan.report.v0.schema.json"
    ))
    .expect("parse edit-plan schema");
    assert_eq!(
        schema.pointer("/$defs/graftTemplate/properties/operation/$ref"),
        Some(&serde_json::json!("#/$defs/graftOperation"))
    );
    assert_eq!(
        schema.pointer("/$defs/transactionTemplate/properties/transaction/$ref"),
        Some(&serde_json::json!("#/$defs/graftTransaction"))
    );
    assert_eq!(
        schema.pointer("/$defs/graftOperation/additionalProperties"),
        Some(&serde_json::json!(false))
    );
    assert_eq!(
        schema.pointer("/$defs/graftTransaction/additionalProperties"),
        Some(&serde_json::json!(false))
    );
    assert_eq!(
        schema.pointer("/$defs/graftTransaction/properties/ops/items/$ref"),
        Some(&serde_json::json!("#/$defs/graftOperation"))
    );
    assert_eq!(
        schema.pointer("/$defs/graftTransaction/properties/mode/enum"),
        Some(&serde_json::json!(["all_or_nothing"]))
    );
    assert_eq!(
        schema.pointer("/$defs/graftOperation/properties/op/enum"),
        Some(&serde_json::json!([
            "AddModuleDeclaration",
            "AddTask",
            "ReplaceTaskBody",
            "AddTake",
            "RemoveTake",
            "RenameDeclaration",
            "AddTypeDeclaration",
            "AddEffectDeclaration",
            "RemoveTaskEffect",
            "AddImport",
            "UpdateCallSites",
            "UpdateCallArgs",
            "ReplaceCallArg",
            "RemoveCallArg",
            "InsertStatement",
            "ReplaceStatement",
            "ReplaceExpression",
            "MoveNode",
            "DeleteNode"
        ]))
    );
}

#[test]
fn lsp_fix_preview_schema_reuses_strict_edit_plan_operations() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../docs/schemas/sley.lsp.fix_preview.v0.schema.json"
    ))
    .expect("parse LSP fix-preview schema");
    assert_eq!(
        schema.pointer("/$defs/code_action_preview/properties/operation/$ref"),
        Some(&serde_json::json!(
            "sley.edit_plan.report.v0#/$defs/graftOperation"
        ))
    );
    assert_eq!(
        schema.pointer("/$defs/code_action_preview/properties/transaction/$ref"),
        Some(&serde_json::json!(
            "sley.edit_plan.report.v0#/$defs/graftTransaction"
        ))
    );
}

#[test]
fn graft_outcome_schema_covers_strict_provenance_records() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../docs/schemas/sley.graft.outcome.v0.schema.json"
    ))
    .expect("parse graft outcome schema");
    assert_eq!(
        schema.pointer("/properties/provenance/items/$ref"),
        Some(&serde_json::json!("#/$defs/provenanceRecord"))
    );
    assert_eq!(
        schema.pointer("/$defs/provenanceRecord/additionalProperties"),
        Some(&serde_json::json!(false))
    );
    assert_eq!(
        schema.pointer("/$defs/provenanceRecord/required"),
        Some(&serde_json::json!([
            "graft_id",
            "actor",
            "timestamp",
            "operation",
            "targets",
            "result"
        ]))
    );
    assert_eq!(
        schema.pointer("/$defs/provenanceRecord/properties/result/enum"),
        Some(&serde_json::json!(["accepted"]))
    );
    assert_eq!(
        schema.pointer("/$defs/provenanceRecord/properties/targets/minItems"),
        Some(&serde_json::json!(1))
    );
    assert_eq!(
        schema.pointer("/$defs/provenanceRecord/properties/operation/enum"),
        Some(&serde_json::json!([
            "AddModuleDeclaration",
            "AddTask",
            "ReplaceTaskBody",
            "AddTake",
            "RemoveTake",
            "RenameDeclaration",
            "AddTypeDeclaration",
            "AddEffectDeclaration",
            "RemoveTaskEffect",
            "AddImport",
            "UpdateCallSites",
            "UpdateCallArgs",
            "ReplaceCallArg",
            "RemoveCallArg",
            "InsertStatement",
            "ReplaceStatement",
            "ReplaceExpression",
            "MoveNode",
            "DeleteNode"
        ]))
    );
}

#[test]
fn zjx_envelope_schema_covers_handoff_contract_roots() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../docs/schemas/sley.zjx.envelope.v0.schema.json"
    ))
    .expect("parse ZJX envelope schema");
    assert_eq!(
        schema.pointer("/required"),
        Some(&serde_json::json!([
            "schema",
            "format",
            "compression",
            "target",
            "graph_digest",
            "graph"
        ]))
    );
    assert_eq!(
        schema.pointer("/properties/graph_digest/pattern"),
        Some(&serde_json::json!("^sha256:[0-9a-f]{64}$"))
    );
    assert_eq!(
        schema.pointer("/properties/graph/$ref"),
        Some(&serde_json::json!("sley.symbol_graph.v0"))
    );
    assert_eq!(
        schema.pointer("/properties/slice/$ref"),
        Some(&serde_json::json!("sley.symbol_graph.slice.v0"))
    );
    assert_eq!(
        schema.pointer("/properties/trace_receipts/items/$ref"),
        Some(&serde_json::json!("sley.trace.receipt.v0"))
    );
}

#[test]
fn zjx_envelope_carries_recomputable_graph_digest() {
    let program = parse_program(include_str!("../examples/hello.sley")).expect("parse hello");
    let graph = build_symbol_graph(&program);
    let graph_bytes = serde_json::to_vec(&graph).expect("serialize graph");
    let expected_digest = content_digest(&graph_bytes);
    let envelope = build_zjx_envelope("examples/hello.sley", graph, None, Vec::new());

    assert_eq!(envelope.schema, "sley.zjx.envelope.v0");
    assert_eq!(envelope.format, "zjx-preview-json");
    assert_eq!(envelope.compression, "none");
    assert_eq!(envelope.graph.schema, SYMBOL_GRAPH_SCHEMA);
    assert_eq!(envelope.graph_digest, expected_digest);
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
fn checker_repair_hints_use_typed_default_sources_for_result_arguments() {
    let source = r#"
task main -> Int {
  return call helper(0)
}

task helper -> Int {
  take value: Result<Int, Error>

  return 1
}
"#;
    let program = parse_program(source).expect("parse result argument hint source");
    let diagnostics = check_program(&program);
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "CALL_ARGUMENT_TYPE_MISMATCH")
        .expect("call argument type mismatch");
    let hint = diagnostic
        .repair_hints
        .iter()
        .find(|hint| hint.kind == "replace_call_arg")
        .expect("replace call arg hint");
    let replacement: serde_json::Value =
        serde_json::from_str(hint.replacement.as_deref().expect("replacement JSON"))
            .expect("replacement should parse");
    assert_eq!(
        replacement.pointer("/payload/source"),
        Some(&serde_json::json!("Ok(0)"))
    );
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
    assert_cli_run_value(&output.stdout, Value::Text("cli gate read".to_string()));
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
fn runtime_scoped_network_capability_rejects_nonmatching_url() {
    let source = r#"
task main -> Result<Text, Error> uses Network {
  return call http.try_get_text("https://other.test/profile")
}
"#;
    let mut gates = RuntimeGates::new();
    gates.grant_effect_scope("Network", "https://example.test/");
    gates.grant_http_text("https://other.test/profile", "blocked");
    assert_scope_denied(source, gates, "Network", "https://other.test/profile");
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
    assert_cli_run_value(
        &output.stdout,
        Value::Ok(Box::new(Value::Text("Ada".to_string()))),
    );
}

#[test]
fn cli_run_accepts_scoped_network_capability() {
    let root = temp_project_dir("runtime-http-scoped-cli");
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
            "Network=https://example.test/",
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
    assert_cli_run_value(
        &output.stdout,
        Value::Ok(Box::new(Value::Text("Ada".to_string()))),
    );
    let _ = fs::remove_dir_all(root);
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
    assert_cli_run_value(
        &output.stdout,
        Value::Ok(Box::new(Value::Text("2026-05-05".to_string()))),
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
    assert_cli_run_value(
        &output.stdout,
        Value::Ok(Box::new(Value::Text("Ada".to_string()))),
    );
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
    assert_cli_run_value(
        &output.stdout,
        Value::Ok(Box::new(Value::Text("redacted".to_string()))),
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
    assert_cli_run_value(
        &output.stdout,
        Value::Ok(Box::new(Value::Text("staged".to_string()))),
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
    assert_cli_run_value(
        &output.stdout,
        Value::Ok(Box::new(Value::Text("authorized".to_string()))),
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
    assert_cli_run_value(&output.stdout, Value::Text("Ada".to_string()));
}

#[test]
fn runtime_scoped_seeded_host_resources_reject_nonmatching_keys() {
    let mut database = RuntimeGates::new();
    database.grant_effect_scope("DatabaseRead", "tenant_");
    database.grant_db_rows(
        "users",
        vec![db_row([
            ("id", Value::Text("u1".to_string())),
            ("name", Value::Text("Ada".to_string())),
        ])],
    );
    assert_scope_denied(
        r#"
task main -> Result<Text, Error> uses DatabaseRead {
  bind row = call db.try_query_one("select * from users where id = ?", "u1")?
  return Ok(row.text("name"))
}
"#,
        database,
        "DatabaseRead",
        "users",
    );

    let mut secret = RuntimeGates::new();
    secret.grant_effect_scope("SecretRead", "prod/");
    secret.grant_secret("dev/api_key", "redacted");
    assert_scope_denied(
        r#"
task main -> Result<Text, Error> uses SecretRead {
  return call secrets.try_get("dev/api_key")
}
"#,
        secret,
        "SecretRead",
        "dev/api_key",
    );

    let mut shell = RuntimeGates::new();
    shell.grant_effect_scope("Shell", "safe:");
    shell.grant_shell_output("date", "2026-05-06");
    assert_scope_denied(
        r#"
task main -> Result<Text, Error> uses Shell {
  return call shell.try_run("date")
}
"#,
        shell,
        "Shell",
        "date",
    );

    let mut model = RuntimeGates::new();
    model.grant_effect_scope("ModelCall", "classification:");
    model.grant_model_output("draft a post", "no");
    assert_scope_denied(
        r#"
task main -> Result<Text, Error> uses ModelCall {
  return call model.try_complete("draft a post")
}
"#,
        model,
        "ModelCall",
        "draft a post",
    );

    let mut deploy = RuntimeGates::new();
    deploy.grant_effect_scope("Deploy", "staging/");
    deploy.grant_deploy_result("prod/app", "blocked");
    assert_scope_denied(
        r#"
task main -> Result<Text, Error> uses Deploy {
  return call deploy.try_stage("prod/app")
}
"#,
        deploy,
        "Deploy",
        "prod/app",
    );

    let mut spend = RuntimeGates::new();
    spend.grant_effect_scope("Spend", "ads/");
    spend.grant_spend_result("infra/budget", "blocked");
    assert_scope_denied(
        r#"
task main -> Result<Text, Error> uses Spend {
  return call spend.try_authorize("infra/budget")
}
"#,
        spend,
        "Spend",
        "infra/budget",
    );
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

    let insert_body = slice
        .insert_affordances
        .iter()
        .find(|affordance| affordance.target == "block:task:main.main")
        .expect("task-body insert affordance");
    assert_eq!(insert_body.target_kind, "block");
    assert_eq!(insert_body.max_position, 3);
    assert_eq!(
        insert_body.operation.pointer("/op"),
        Some(&serde_json::json!("InsertStatement"))
    );
    assert_eq!(
        insert_body.operation.pointer("/payload/source"),
        Some(&serde_json::json!("forge { }"))
    );
    assert_eq!(
        insert_body.operation.pointer("/payload/position"),
        Some(&serde_json::json!(3))
    );
    assert_eq!(
        insert_body.editable_json_pointers,
        vec![
            "/payload/source".to_string(),
            "/payload/position".to_string()
        ]
    );
    let insert_graft: GraftInput =
        serde_json::from_value(insert_body.operation.clone()).expect("parse insert affordance");
    let insert_outcome = apply_graft_input(&program, insert_graft, Some("agent:test".to_string()));
    assert_eq!(
        insert_outcome.status, "accepted",
        "{:#?}",
        insert_outcome.diagnostics
    );

    let insert_nested = slice
        .insert_affordances
        .iter()
        .find(|affordance| affordance.target == "block:task:main.main:stmt:1:then")
        .expect("nested block insert affordance");
    assert_eq!(insert_nested.max_position, 1);
    assert_eq!(
        insert_nested.operation.pointer("/target"),
        Some(&serde_json::json!("block:task:main.main:stmt:1:then"))
    );

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

    let replace_statement = slice
        .replace_affordances
        .iter()
        .find(|affordance| affordance.target == "block:task:main.main:stmt:0")
        .expect("statement replace affordance");
    assert_eq!(replace_statement.target_kind, "statement");
    assert_eq!(replace_statement.parent, "block:task:main.main");
    assert_eq!(
        replace_statement.operation.pointer("/op"),
        Some(&serde_json::json!("ReplaceStatement"))
    );
    assert_eq!(
        replace_statement.operation.pointer("/payload/source"),
        Some(&serde_json::json!("tally total = value"))
    );
    assert_eq!(
        replace_statement.editable_json_pointers,
        vec!["/payload/source".to_string()]
    );
    let replace_statement_graft: GraftInput =
        serde_json::from_value(replace_statement.operation.clone())
            .expect("parse replace statement affordance");
    let replace_statement_outcome = apply_graft_input(
        &program,
        replace_statement_graft,
        Some("agent:test".to_string()),
    );
    assert_eq!(
        replace_statement_outcome.status, "accepted",
        "{:#?}",
        replace_statement_outcome.diagnostics
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
    assert_eq!(report.types.len(), 0);
    assert_eq!(report.effects.len(), 0);
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

    let declarations = parse_program(
        r#"
module app.profile

export type User = {
  slot id: Text
  slot name: Text
}

type Draft = {
  slot name: Text
}

export effect PublicEffect
effect LocalEffect

task main -> User {
  return User { id: "1", name: "Ada" }
}
"#,
    )
    .expect("parse query declarations");
    let diagnostics = check_program(&declarations);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );

    let types = build_query_report(
        &declarations,
        QueryOptions {
            kind: QueryKind::Types,
            module: Some("app.profile".to_string()),
            exported_only: false,
        },
    );
    assert_eq!(types.kind, "types");
    assert_eq!(types.modules.len(), 0);
    assert_eq!(types.tasks.len(), 0);
    assert_eq!(types.calls.len(), 0);
    assert_eq!(types.effects.len(), 0);
    assert_eq!(types.types.len(), 2);
    assert_eq!(types.types[1].qualified_name, "app.profile.User");
    assert_eq!(types.types[1].value, "{ id: Text, name: Text }");
    assert_eq!(types.types[1].fields[0].name, "id");
    assert_eq!(types.types[1].fields[0].ty, "Text");

    let effects = build_query_report(
        &declarations,
        QueryOptions {
            kind: QueryKind::Effects,
            module: Some("app.profile".to_string()),
            exported_only: true,
        },
    );
    assert_eq!(effects.kind, "effects");
    assert_eq!(effects.types.len(), 0);
    assert_eq!(effects.effects.len(), 1);
    assert_eq!(
        effects.effects[0].qualified_name,
        "app.profile.PublicEffect"
    );
    assert!(effects.effects[0].exported);

    let calls = build_query_report(
        &project.program,
        QueryOptions {
            kind: QueryKind::Calls,
            module: None,
            exported_only: false,
        },
    );
    assert_eq!(calls.kind, "calls");
    assert_eq!(calls.modules.len(), 0);
    assert_eq!(calls.tasks.len(), 0);
    assert_eq!(calls.types.len(), 0);
    assert_eq!(calls.effects.len(), 0);
    assert_eq!(calls.calls.len(), 1);
    assert_eq!(calls.calls[0].from, "app.main.main");
    assert_eq!(calls.calls[0].from_module, "app.main");
    assert_eq!(calls.calls[0].callee, "math.double");
    assert_eq!(calls.calls[0].status, "resolved");
    assert_eq!(calls.calls[0].target.as_deref(), Some("app.math.double"));
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
    assert_eq!(
        report.findings[0].node,
        "effect-use:task:app.effects.stale:0:Network"
    );
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
fn lint_report_flags_unchecked_results() {
    let source = r#"
module app.unchecked

task main -> Result<Text, Error> uses FileWrite {
  fs.try_write_text("out.txt", "ok")
  call helper()
  fs.try_write_text("out.txt", "handled")?

  return call helper()
}

task helper -> Result<Text, Error> {
  return Ok("helper")
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
            rules: vec![LintRule::UncheckedResult],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.unchecked");
    assert_eq!(report.filters.rules, vec!["unchecked_result"]);
    assert_eq!(report.findings.len(), 2);
    assert_eq!(report.findings[0].id, "UNCHECKED_RESULT");
    assert_eq!(report.findings[0].rule, "unchecked_result");
    assert_eq!(
        report.findings[0].node,
        "block:task:app.unchecked.main:stmt:0:expr"
    );
    assert_eq!(
        report.findings[1].node,
        "block:task:app.unchecked.main:stmt:1:expr"
    );
    assert!(report.findings[0].message.contains("fs.try_write_text"));
    assert!(report.findings[1].message.contains("helper"));
    assert!(report.findings[0].hint.contains("`?`"));

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::UncheckedResult],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_unqualified_imported_calls() {
    let project = load_project("examples/unqualified_import_call_project")
        .expect("load import style project");
    let diagnostics = check_program(&project.program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );

    let report = build_lint_report(
        &project.program,
        LintOptions {
            rules: vec![LintRule::UnqualifiedImportedCall],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.main");
    assert_eq!(report.filters.rules, vec!["unqualified_imported_call"]);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].id, "UNQUALIFIED_IMPORTED_CALL");
    assert_eq!(report.findings[0].rule, "unqualified_imported_call");
    assert_eq!(
        report.findings[0].node,
        "block:task:app.main.main:stmt:0:expr"
    );
    assert_eq!(report.findings[0].module, "app.main");
    assert!(report.findings[0].hint.contains("math.double"));

    let scoped_report = build_lint_report(
        &project.program,
        LintOptions {
            rules: vec![LintRule::UnqualifiedImportedCall],
            module: Some("app.math".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_unused_pure_bindings() {
    let source = r#"
module app.bindings

task main -> Result<Int, Error> uses FileRead {
  bind stale = true
  bind used = 40
  bind host_unused = fs.read_text("examples/hello.sley")
  bind fallible_unused = fs.try_read_text("examples/hello.sley")?
  bind indexed_unused = [1, 2][0]
  bind divided_unused = 10 / 2

  return Ok(used + 2)
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
            rules: vec![LintRule::UnusedPureBinding],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.bindings");
    assert_eq!(report.filters.rules, vec!["unused_pure_binding"]);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].id, "UNUSED_PURE_BINDING");
    assert_eq!(report.findings[0].rule, "unused_pure_binding");
    assert_eq!(
        report.findings[0].node,
        "block:task:app.bindings.main:stmt:0"
    );
    assert_eq!(report.findings[0].module, "app.bindings");
    assert!(report.findings[0].message.contains("binds `stale`"));
    assert!(report.findings[0].hint.contains("delete bind `stale`"));

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::UnusedPureBinding],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_unused_pure_expression_statements() {
    let source = r#"
module app.unused_expr

task main -> Result<Int, Error> uses FileRead {
  bind used = 40

  used + 2
  used / 2
  fs.read_text("examples/hello.sley")
  fs.try_read_text("examples/hello.sley")?
  [1, 2][0]

  return Ok(used + 2)
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
            rules: vec![LintRule::UnusedPureExpressionStatement],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.unused_expr");
    assert_eq!(
        report.filters.rules,
        vec!["unused_pure_expression_statement"]
    );
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].id, "UNUSED_PURE_EXPRESSION_STATEMENT");
    assert_eq!(report.findings[0].rule, "unused_pure_expression_statement");
    assert_eq!(
        report.findings[0].node,
        "block:task:app.unused_expr.main:stmt:1"
    );
    assert_eq!(report.findings[0].module, "app.unused_expr");
    assert!(report.findings[0].message.contains("used + 2"));
    assert!(report.findings[0].hint.contains("delete"));

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::UnusedPureExpressionStatement],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_unreachable_statements() {
    let source = r#"
module app.unreachable_statement

task main -> Int {
  bind flag = true

  if flag {
    return 1
  } else {
    return 2
  }

  bind stale = 0
}

task nested -> Int {
  if true {
    return 3
    bind nested_stale = 9
  } else {
    return 4
  }

  return 5
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
            rules: vec![LintRule::UnreachableStatement],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.unreachable_statement");
    assert_eq!(report.filters.rules, vec!["unreachable_statement"]);
    assert_eq!(report.findings.len(), 3);
    assert_eq!(report.findings[0].id, "UNREACHABLE_STATEMENT");
    assert_eq!(report.findings[0].rule, "unreachable_statement");
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.unreachable_statement.main:stmt:2"));
    assert!(nodes.contains(&"block:task:app.unreachable_statement.nested:stmt:0:then:stmt:1"));
    assert!(nodes.contains(&"block:task:app.unreachable_statement.nested:stmt:1"));
    assert_eq!(report.findings[0].module, "app.unreachable_statement");
    assert!(report.findings[0].message.contains("unreachable statement"));
    assert!(report.findings[0].hint.contains("delete"));

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::UnreachableStatement],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_mutable_bindings_never_set() {
    let source = r#"
module app.mutable_style

task main -> Int {
  state base = 21
  tally total = 0
  set total = total + base

  return total
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
            rules: vec![LintRule::MutableBindingNeverSet],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.mutable_style");
    assert_eq!(report.filters.rules, vec!["mutable_binding_never_set"]);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].id, "MUTABLE_BINDING_NEVER_SET");
    assert_eq!(report.findings[0].rule, "mutable_binding_never_set");
    assert_eq!(
        report.findings[0].node,
        "block:task:app.mutable_style.main:stmt:0"
    );
    assert_eq!(report.findings[0].module, "app.mutable_style");
    assert!(report.findings[0].message.contains("state"));
    assert!(report.findings[0].hint.contains("bind base"));

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::MutableBindingNeverSet],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_self_assignment_statements() {
    let source = r#"
module app.self_assignment

task main -> Int {
  state count = 1
  set count = count
  set count = count + 1
  if count > 1 {
    set count = count
  }
  set count = count - 1

  return count
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
            rules: vec![LintRule::SelfAssignmentStatement],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.self_assignment");
    assert_eq!(report.filters.rules, vec!["self_assignment_statement"]);
    assert_eq!(report.findings.len(), 2);
    assert_eq!(report.findings[0].id, "SELF_ASSIGNMENT_STATEMENT");
    assert_eq!(report.findings[0].rule, "self_assignment_statement");
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.self_assignment.main:stmt:1"));
    assert!(nodes.contains(&"block:task:app.self_assignment.main:stmt:3:then:stmt:0"));
    assert!(!nodes.contains(&"block:task:app.self_assignment.main:stmt:2"));
    assert!(!nodes.contains(&"block:task:app.self_assignment.main:stmt:4"));
    assert_eq!(report.findings[0].module, "app.self_assignment");
    assert!(report.findings[0].message.contains("set count = count"));
    assert!(report.findings[0].hint.contains("delete"));

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::SelfAssignmentStatement],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_overwritten_set_statements() {
    let source = r#"
module app.overwritten_set

task main -> Int {
  state count = 0
  set count = 1
  set count = 2
  set count = count
  set count = 3
  set count = count + 1
  set count = [1, 2][0]
  set count = 4
  if count > 1 {
    set count = 5
    set count = 6
  }

  return count
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
            rules: vec![LintRule::OverwrittenSetStatement],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.overwritten_set");
    assert_eq!(report.filters.rules, vec!["overwritten_set_statement"]);
    assert_eq!(report.findings.len(), 2);
    assert_eq!(report.findings[0].id, "OVERWRITTEN_SET_STATEMENT");
    assert_eq!(report.findings[0].rule, "overwritten_set_statement");
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.overwritten_set.main:stmt:1"));
    assert!(nodes.contains(&"block:task:app.overwritten_set.main:stmt:8:then:stmt:0"));
    assert!(!nodes.contains(&"block:task:app.overwritten_set.main:stmt:3"));
    assert!(!nodes.contains(&"block:task:app.overwritten_set.main:stmt:4"));
    assert!(!nodes.contains(&"block:task:app.overwritten_set.main:stmt:6"));
    assert_eq!(report.findings[0].module, "app.overwritten_set");
    assert!(
        report.findings[0]
            .message
            .contains("immediately overwrites")
    );
    assert!(report.findings[0].hint.contains("set count"));

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::OverwrittenSetStatement],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_redundant_initial_set_statements() {
    let source = r#"
module app.redundant_initial_set

task main -> Int {
  state count = 0
  set count = 1
  set count = count + 1
  state kept = 0
  set kept = kept + 1
  set kept = 2
  state indexed = 0
  set indexed = [1, 2][0]
  set indexed = 3
  state once = 0
  set once = 1
  state selfed = 0
  set selfed = 1
  set selfed = selfed
  if count > 0 {
    state total = 0
    set total = 2
    set total = total + count
  }

  return count + once + selfed
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
            rules: vec![LintRule::RedundantInitialSetStatement],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.redundant_initial_set");
    assert_eq!(
        report.filters.rules,
        vec!["redundant_initial_set_statement"]
    );
    assert_eq!(report.findings.len(), 3);
    assert_eq!(report.findings[0].id, "REDUNDANT_INITIAL_SET_STATEMENT");
    assert_eq!(report.findings[0].rule, "redundant_initial_set_statement");
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.redundant_initial_set.main:stmt:1"));
    assert!(nodes.contains(&"block:task:app.redundant_initial_set.main:stmt:10"));
    assert!(nodes.contains(&"block:task:app.redundant_initial_set.main:stmt:14:then:stmt:1"));
    assert!(!nodes.contains(&"block:task:app.redundant_initial_set.main:stmt:4"));
    assert!(!nodes.contains(&"block:task:app.redundant_initial_set.main:stmt:7"));
    assert!(!nodes.contains(&"block:task:app.redundant_initial_set.main:stmt:12"));
    assert_eq!(report.findings[0].module, "app.redundant_initial_set");
    assert!(report.findings[0].message.contains("immediately replaces"));
    assert!(report.findings[0].hint.contains("initializer"));

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::RedundantInitialSetStatement],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_constant_if_expressions() {
    let source = r#"
module app.constant_if

task main -> Int {
  return if false { 0 } else { 41 }
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
            rules: vec![LintRule::ConstantIfExpression],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.constant_if");
    assert_eq!(report.filters.rules, vec!["constant_if_expression"]);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].id, "CONSTANT_IF_EXPRESSION");
    assert_eq!(report.findings[0].rule, "constant_if_expression");
    assert_eq!(
        report.findings[0].node,
        "block:task:app.constant_if.main:stmt:0:expr"
    );
    assert_eq!(report.findings[0].module, "app.constant_if");
    assert!(report.findings[0].message.contains("constant `false`"));
    assert!(report.findings[0].hint.contains("`else` branch"));

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::ConstantIfExpression],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_constant_if_statements() {
    let source = r#"
module app.constant_if_statement

task main -> Int {
  state total = 0

  if true {
    set total = 1
  } else {
    set total = 2
  }

  if false {
    set total = 3
  } else {
    set total = 4
  }

  if total > 0 {
    set total = 5
  } else {
    set total = 6
  }

  return total
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
            rules: vec![LintRule::ConstantIfStatement],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.constant_if_statement");
    assert_eq!(report.filters.rules, vec!["constant_if_statement"]);
    assert_eq!(report.findings.len(), 2);
    assert_eq!(report.findings[0].id, "CONSTANT_IF_STATEMENT");
    assert_eq!(report.findings[0].rule, "constant_if_statement");
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.constant_if_statement.main:stmt:1"));
    assert!(nodes.contains(&"block:task:app.constant_if_statement.main:stmt:2"));
    assert_eq!(report.findings[0].module, "app.constant_if_statement");
    assert!(report.findings[0].message.contains("constant"));
    assert!(report.findings[0].hint.contains("single executing branch"));

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::ConstantIfStatement],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_constant_false_if_statements() {
    let source = r#"
module app.constant_false_if

task main -> Int {
  if false {
    return 1
  }

  if false {
    bind skipped = 2
  }

  if false {
    bind skipped_with_else = 3
  } else {
    bind active = 4
  }

  if true {
    if false {
      bind nested = 5
    }
  }

  return 6
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
            rules: vec![LintRule::ConstantFalseIfStatement],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.constant_false_if");
    assert_eq!(report.filters.rules, vec!["constant_false_if_statement"]);
    assert_eq!(
        report.findings.len(),
        3,
        "constant false if should skip false if statements with else branches"
    );
    assert_eq!(report.findings[0].id, "CONSTANT_FALSE_IF_STATEMENT");
    assert_eq!(report.findings[0].rule, "constant_false_if_statement");
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.constant_false_if.main:stmt:0"));
    assert!(nodes.contains(&"block:task:app.constant_false_if.main:stmt:1"));
    assert!(nodes.contains(&"block:task:app.constant_false_if.main:stmt:3:then:stmt:0"));
    assert_eq!(report.findings[0].module, "app.constant_false_if");
    assert!(report.findings[0].message.contains("constant `false`"));
    assert!(report.findings[0].hint.contains("never-executed if"));

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::ConstantFalseIfStatement],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_constant_false_while_statements() {
    let source = r#"
module app.false_while

task main -> Int {
  state value = 41

  while false {
    set value = 0
  }

  while value < 0 {
    set value = value + 1
  }

  if value > 40 {
    while false {
      set value = 7
    }
  }

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
            rules: vec![LintRule::ConstantFalseWhileStatement],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.false_while");
    assert_eq!(report.filters.rules, vec!["constant_false_while_statement"]);
    assert_eq!(report.findings.len(), 2);
    assert_eq!(report.findings[0].id, "CONSTANT_FALSE_WHILE_STATEMENT");
    assert_eq!(report.findings[0].rule, "constant_false_while_statement");
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.false_while.main:stmt:1"));
    assert!(nodes.contains(&"block:task:app.false_while.main:stmt:3:then:stmt:0"));
    assert_eq!(report.findings[0].module, "app.false_while");
    assert!(report.findings[0].message.contains("constant `false`"));
    assert!(report.findings[0].hint.contains("delete"));

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::ConstantFalseWhileStatement],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_constant_comparison_expressions() {
    let source = r#"
module app.constant_compare

task main -> Bool {
  bind guarded = true == false
  bind same = 1 == 1
  bind text = "a" == "b"
  bind floaty = 1.5 >= 2.0

  return 1 < 2
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
            rules: vec![LintRule::ConstantComparisonExpression],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.constant_compare");
    assert_eq!(report.filters.rules, vec!["constant_comparison_expression"]);
    assert_eq!(
        report.findings.len(),
        3,
        "constant comparison should not overlap boolean-literal or self-comparison simplifications"
    );
    assert_eq!(report.findings[0].id, "CONSTANT_COMPARISON_EXPRESSION");
    assert_eq!(report.findings[0].rule, "constant_comparison_expression");
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.constant_compare.main:stmt:2:expr"));
    assert!(nodes.contains(&"block:task:app.constant_compare.main:stmt:3:expr"));
    assert!(nodes.contains(&"block:task:app.constant_compare.main:stmt:4:expr"));
    assert_eq!(report.findings[0].module, "app.constant_compare");
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.message.contains("\"a\" == \"b\""))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.message.contains("1.5 >= 2.0"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("true"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("false"))
    );

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::ConstantComparisonExpression],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_constant_arithmetic_expressions() {
    let source = r#"
module app.constant_arithmetic

task main -> Float {
  bind total = 2 + 3
  bind scaled = 4 * 5
  bind mixed = 1.5 + 2.0
  bind identity = 1 + 0
  bind unsafe = 1 / 0

  return 6 / 2.0
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
            rules: vec![LintRule::ConstantArithmeticExpression],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.constant_arithmetic");
    assert_eq!(report.filters.rules, vec!["constant_arithmetic_expression"]);
    assert_eq!(
        report.findings.len(),
        4,
        "constant arithmetic should skip identity expressions and divide-by-zero"
    );
    assert_eq!(report.findings[0].id, "CONSTANT_ARITHMETIC_EXPRESSION");
    assert_eq!(report.findings[0].rule, "constant_arithmetic_expression");
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.constant_arithmetic.main:stmt:0:expr"));
    assert!(nodes.contains(&"block:task:app.constant_arithmetic.main:stmt:1:expr"));
    assert!(nodes.contains(&"block:task:app.constant_arithmetic.main:stmt:2:expr"));
    assert!(nodes.contains(&"block:task:app.constant_arithmetic.main:stmt:5:expr"));
    assert_eq!(report.findings[0].module, "app.constant_arithmetic");
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("5"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("20"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("3.5"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("3.0"))
    );

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::ConstantArithmeticExpression],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_absorbing_arithmetic_expressions() {
    let source = r#"
module app.absorbing_arithmetic

task make_runtime -> Int {
  return 9
}

task main -> Int {
  bind left_zero = 0 * (1 + 2)
  bind right_zero = (3 + 4) * 0
  bind float_zero = 0.0 * (1.5 + 2.5)
  bind constant_product = 2 * 0
  bind runtime_product = call make_runtime() * 0

  return (5 + 6) * 0
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
            rules: vec![LintRule::AbsorbingArithmeticExpression],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.absorbing_arithmetic");
    assert_eq!(
        report.filters.rules,
        vec!["absorbing_arithmetic_expression"]
    );
    assert_eq!(
        report.findings.len(),
        4,
        "absorbing arithmetic should skip constant products and runtime calls"
    );
    assert_eq!(report.findings[0].id, "ABSORBING_ARITHMETIC_EXPRESSION");
    assert_eq!(report.findings[0].rule, "absorbing_arithmetic_expression");
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.absorbing_arithmetic.main:stmt:0:expr"));
    assert!(nodes.contains(&"block:task:app.absorbing_arithmetic.main:stmt:1:expr"));
    assert!(nodes.contains(&"block:task:app.absorbing_arithmetic.main:stmt:2:expr"));
    assert!(nodes.contains(&"block:task:app.absorbing_arithmetic.main:stmt:5:expr"));
    assert_eq!(report.findings[0].module, "app.absorbing_arithmetic");
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("0.0"))
    );
    assert!(
        report
            .findings
            .iter()
            .filter(|finding| finding.hint.contains("`0`"))
            .count()
            >= 3
    );

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::AbsorbingArithmeticExpression],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_constant_text_concatenation_expressions() {
    let source = r#"
module app.constant_text_concat

task suffix -> Text {
  return "agent"
}

task main -> Text {
  bind greeting = "Hello, " + "agent"
  bind escaped = "line\n" + "tab\t"
  bind identity = "" + "agent"
  bind runtime = "prefix " + call suffix()

  return "Sley" + " ready"
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
            rules: vec![LintRule::ConstantTextConcatenationExpression],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.constant_text_concat");
    assert_eq!(
        report.filters.rules,
        vec!["constant_text_concatenation_expression"]
    );
    assert_eq!(
        report.findings.len(),
        3,
        "constant text concatenation should skip runtime text expressions"
    );
    assert_eq!(
        report.findings[0].id,
        "CONSTANT_TEXT_CONCATENATION_EXPRESSION"
    );
    assert_eq!(
        report.findings[0].rule,
        "constant_text_concatenation_expression"
    );
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.constant_text_concat.main:stmt:0:expr"));
    assert!(nodes.contains(&"block:task:app.constant_text_concat.main:stmt:1:expr"));
    assert!(!nodes.contains(&"block:task:app.constant_text_concat.main:stmt:2:expr"));
    assert!(nodes.contains(&"block:task:app.constant_text_concat.main:stmt:4:expr"));
    assert_eq!(report.findings[0].module, "app.constant_text_concat");
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("\"Hello, agent\""))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("\"line\\ntab\\t\""))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("\"Sley ready\""))
    );

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::ConstantTextConcatenationExpression],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_constant_list_index_expressions() {
    let source = r#"
module app.constant_list_index

task make_runtime -> Int {
  return 2
}

task main -> Text {
  bind number = [10, 20, 30][1]
  bind text = ["alpha", "beta"][0]
  bind bool_value = [true, false][1]
  bind out_of_range = [1, 2][5]
  bind runtime = [1, call make_runtime()][0]

  return ["ready", "done"][1]
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
            rules: vec![LintRule::ConstantListIndexExpression],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.constant_list_index");
    assert_eq!(report.filters.rules, vec!["constant_list_index_expression"]);
    assert_eq!(
        report.findings.len(),
        4,
        "constant list index should skip out-of-range and runtime list items"
    );
    assert_eq!(report.findings[0].id, "CONSTANT_LIST_INDEX_EXPRESSION");
    assert_eq!(report.findings[0].rule, "constant_list_index_expression");
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.constant_list_index.main:stmt:0:expr"));
    assert!(nodes.contains(&"block:task:app.constant_list_index.main:stmt:1:expr"));
    assert!(nodes.contains(&"block:task:app.constant_list_index.main:stmt:2:expr"));
    assert!(nodes.contains(&"block:task:app.constant_list_index.main:stmt:5:expr"));
    assert_eq!(report.findings[0].module, "app.constant_list_index");
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("20"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("\"alpha\""))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("false"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("\"done\""))
    );

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::ConstantListIndexExpression],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_constant_map_index_expressions() {
    let source = r#"
module app.constant_map_index

task make_runtime -> Text {
  return "runtime"
}

task main -> Text {
  bind text = map { "one": "alpha", "two": "beta" }["one"]
  bind number = map { "low": 1, "high": 2 }["high"]
  bind bool_value = map { "yes": true, "no": false }["no"]
  bind missing = map { "one": "alpha" }["missing"]
  bind runtime_value = map { "one": "alpha", "two": call make_runtime() }["one"]
  bind runtime_key = map { call make_runtime(): "alpha" }["alpha"]

  return map { "ready": "go", "done": "ship" }["done"]
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
            rules: vec![LintRule::ConstantMapIndexExpression],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.constant_map_index");
    assert_eq!(report.filters.rules, vec!["constant_map_index_expression"]);
    assert_eq!(
        report.findings.len(),
        4,
        "constant map index should skip missing keys and runtime map entries"
    );
    assert_eq!(report.findings[0].id, "CONSTANT_MAP_INDEX_EXPRESSION");
    assert_eq!(report.findings[0].rule, "constant_map_index_expression");
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.constant_map_index.main:stmt:0:expr"));
    assert!(nodes.contains(&"block:task:app.constant_map_index.main:stmt:1:expr"));
    assert!(nodes.contains(&"block:task:app.constant_map_index.main:stmt:2:expr"));
    assert!(nodes.contains(&"block:task:app.constant_map_index.main:stmt:6:expr"));
    assert_eq!(report.findings[0].module, "app.constant_map_index");
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("\"alpha\""))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("2"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("false"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("\"ship\""))
    );

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::ConstantMapIndexExpression],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_constant_record_field_access_expressions() {
    let source = r#"
module app.constant_record_field_access

type User = {
  slot name: Text
  slot age: Int
  slot active: Bool
  slot label: Text
}

task make_runtime -> Text {
  return "runtime"
}

task main -> Text {
  bind text = User { name: "Ada", age: 37, active: true, label: "ready" }.name
  bind number = User { name: "Ada", age: 37, active: true, label: "ready" }.age
  bind bool_value = User { name: "Ada", age: 37, active: true, label: "ready" }.active
  bind runtime = User { name: call make_runtime(), age: 37, active: true, label: "ready" }.age

  return User { name: "Ada", age: 37, active: true, label: "done" }.label
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
            rules: vec![LintRule::ConstantRecordFieldAccessExpression],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.constant_record_field_access");
    assert_eq!(
        report.filters.rules,
        vec!["constant_record_field_access_expression"]
    );
    assert_eq!(
        report.findings.len(),
        4,
        "constant record field access should skip runtime record fields"
    );
    assert_eq!(
        report.findings[0].id,
        "CONSTANT_RECORD_FIELD_ACCESS_EXPRESSION"
    );
    assert_eq!(
        report.findings[0].rule,
        "constant_record_field_access_expression"
    );
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.constant_record_field_access.main:stmt:0:expr"));
    assert!(nodes.contains(&"block:task:app.constant_record_field_access.main:stmt:1:expr"));
    assert!(nodes.contains(&"block:task:app.constant_record_field_access.main:stmt:2:expr"));
    assert!(nodes.contains(&"block:task:app.constant_record_field_access.main:stmt:4:expr"));
    assert_eq!(
        report.findings[0].module,
        "app.constant_record_field_access"
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("\"Ada\""))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("37"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("true"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("\"done\""))
    );

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::ConstantRecordFieldAccessExpression],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_constant_len_expressions() {
    let source = r#"
module app.constant_len

task make_runtime -> Text {
  return "runtime"
}

task main -> Int {
  bind text_len = len("agent")
  bind list_len = len([1, 2, 3])
  bind map_len = len(map { "one": "ready", "two": "done" })
  bind runtime_list = len([call make_runtime()])
  bind runtime_map = len(map { "one": call make_runtime() })

  return len("done")
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
            rules: vec![LintRule::ConstantLenExpression],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.constant_len");
    assert_eq!(report.filters.rules, vec!["constant_len_expression"]);
    assert_eq!(
        report.findings.len(),
        4,
        "constant len should skip literal collections that evaluate runtime values"
    );
    assert_eq!(report.findings[0].id, "CONSTANT_LEN_EXPRESSION");
    assert_eq!(report.findings[0].rule, "constant_len_expression");
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.constant_len.main:stmt:0:expr"));
    assert!(nodes.contains(&"block:task:app.constant_len.main:stmt:1:expr"));
    assert!(nodes.contains(&"block:task:app.constant_len.main:stmt:2:expr"));
    assert!(nodes.contains(&"block:task:app.constant_len.main:stmt:5:expr"));
    assert_eq!(report.findings[0].module, "app.constant_len");
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("5"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("3"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("2"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("4"))
    );

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::ConstantLenExpression],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_constant_not_expressions() {
    let source = r#"
module app.constant_not

task runtime_flag -> Bool {
  return true
}

task main -> Bool {
  bind false_value = !true
  bind true_value = !false
  bind runtime_value = !call runtime_flag()

  return !false
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
            rules: vec![LintRule::ConstantNotExpression],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.constant_not");
    assert_eq!(report.filters.rules, vec!["constant_not_expression"]);
    assert_eq!(
        report.findings.len(),
        3,
        "constant not should skip runtime boolean expressions"
    );
    assert_eq!(report.findings[0].id, "CONSTANT_NOT_EXPRESSION");
    assert_eq!(report.findings[0].rule, "constant_not_expression");
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.constant_not.main:stmt:0:expr"));
    assert!(nodes.contains(&"block:task:app.constant_not.main:stmt:1:expr"));
    assert!(nodes.contains(&"block:task:app.constant_not.main:stmt:3:expr"));
    assert_eq!(report.findings[0].module, "app.constant_not");
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("false"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("true"))
    );

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::ConstantNotExpression],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_empty_if_statements() {
    let source = r#"
module app.empty_if

task main -> Int {
  state total = 0

  if total > 0 {
  } else {
  }

  if total == 1 {
    set total = 2
  } else {
  }

  if total == 0 {
    if total >= 0 {
    } else {
    }
  }

  return total
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
            rules: vec![LintRule::EmptyIfStatement],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.empty_if");
    assert_eq!(report.filters.rules, vec!["empty_if_statement"]);
    assert_eq!(report.findings.len(), 2);
    assert_eq!(report.findings[0].id, "EMPTY_IF_STATEMENT");
    assert_eq!(report.findings[0].rule, "empty_if_statement");
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.empty_if.main:stmt:1"));
    assert!(nodes.contains(&"block:task:app.empty_if.main:stmt:3:then:stmt:0"));
    assert_eq!(report.findings[0].module, "app.empty_if");
    assert!(report.findings[0].message.contains("empty if"));
    assert!(report.findings[0].hint.contains("delete"));

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::EmptyIfStatement],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_empty_else_statements() {
    let source = r#"
module app.empty_else

task main -> Int {
  bind value = 1

  if value > 0 {
    return value
  } else {
  }

  if value == 0 {
  } else {
  }

  if value < 0 {
    return 0
  } else {
    return value
  }

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
            rules: vec![LintRule::EmptyElseStatement],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.empty_else");
    assert_eq!(report.filters.rules, vec!["empty_else_statement"]);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].id, "EMPTY_ELSE_STATEMENT");
    assert_eq!(report.findings[0].rule, "empty_else_statement");
    assert_eq!(
        report.findings[0].node,
        "block:task:app.empty_else.main:stmt:1"
    );
    assert_eq!(report.findings[0].module, "app.empty_else");
    assert!(report.findings[0].message.contains("empty else"));
    assert!(report.findings[0].hint.contains("remove"));

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::EmptyElseStatement],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_empty_for_statements() {
    let source = r#"
module app.empty_for

task main -> Int {
  state total = 0

  for item in [] {
    set total = 1
  }

  for item in [1] {
    set total = total + item
  }

  if total > 0 {
    for item in [] {
      set total = 7
    }
  }

  return total
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
            rules: vec![LintRule::EmptyForStatement],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.empty_for");
    assert_eq!(report.filters.rules, vec!["empty_for_statement"]);
    assert_eq!(report.findings.len(), 2);
    assert_eq!(report.findings[0].id, "EMPTY_FOR_STATEMENT");
    assert_eq!(report.findings[0].rule, "empty_for_statement");
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.empty_for.main:stmt:1"));
    assert!(nodes.contains(&"block:task:app.empty_for.main:stmt:3:then:stmt:0"));
    assert_eq!(report.findings[0].module, "app.empty_for");
    assert!(report.findings[0].message.contains("empty list"));
    assert!(report.findings[0].hint.contains("delete"));

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::EmptyForStatement],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_empty_forge_statements() {
    let source = r#"
module app.empty_forge

task main -> Int {
  bind value = 0

  forge {
  }

  forge {
    bind used = value + 1
  }

  if value == 0 {
    forge {
    }
  }

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
            rules: vec![LintRule::EmptyForgeStatement],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.empty_forge");
    assert_eq!(report.filters.rules, vec!["empty_forge_statement"]);
    assert_eq!(report.findings.len(), 2);
    assert_eq!(report.findings[0].id, "EMPTY_FORGE_STATEMENT");
    assert_eq!(report.findings[0].rule, "empty_forge_statement");
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.empty_forge.main:stmt:1"));
    assert!(nodes.contains(&"block:task:app.empty_forge.main:stmt:3:then:stmt:0"));
    assert_eq!(report.findings[0].module, "app.empty_forge");
    assert!(report.findings[0].message.contains("empty forge"));
    assert!(report.findings[0].hint.contains("delete"));

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::EmptyForgeStatement],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_identity_binary_expressions() {
    let source = r#"
module app.identity_binary

task main -> Int {
  bind amount = 10
  bind flag = true
  bind label = "agent"
  bind left_text = "" + label
  bind right_text = label + ""
  bind literal_text = "" + "literal"
  bind constant_text = "Sley " + "agents"

  return if flag && true { amount * 1 } else { 0 + amount }
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
            rules: vec![LintRule::IdentityBinaryExpression],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.identity_binary");
    assert_eq!(report.filters.rules, vec!["identity_binary_expression"]);
    assert_eq!(report.findings.len(), 6);
    assert_eq!(report.findings[0].id, "IDENTITY_BINARY_EXPRESSION");
    assert_eq!(report.findings[0].rule, "identity_binary_expression");
    assert_eq!(report.findings[0].module, "app.identity_binary");
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.identity_binary.main:stmt:3:expr"));
    assert!(nodes.contains(&"block:task:app.identity_binary.main:stmt:4:expr"));
    assert!(nodes.contains(&"block:task:app.identity_binary.main:stmt:5:expr"));
    assert!(!nodes.contains(&"block:task:app.identity_binary.main:stmt:6:expr"));
    assert!(nodes.contains(&"block:task:app.identity_binary.main:stmt:7:expr:condition"));
    assert!(nodes.contains(&"block:task:app.identity_binary.main:stmt:7:expr:then"));
    assert!(nodes.contains(&"block:task:app.identity_binary.main:stmt:7:expr:else"));
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("label"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("\"literal\""))
    );
    assert!(
        report.findings.iter().any(
            |finding| finding.message.contains("flag && true") && finding.hint.contains("flag")
        )
    );

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::IdentityBinaryExpression],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_redundant_boolean_comparisons() {
    let source = r#"
module app.boolean_compare

task main -> Bool {
  bind ready = true
  bind nested = ready && true

  return if ready == true { false != ready } else { nested != true }
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
            rules: vec![LintRule::RedundantBooleanComparison],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.boolean_compare");
    assert_eq!(report.filters.rules, vec!["redundant_boolean_comparison"]);
    assert_eq!(report.findings.len(), 3);
    assert_eq!(report.findings[0].id, "REDUNDANT_BOOLEAN_COMPARISON");
    assert_eq!(report.findings[0].rule, "redundant_boolean_comparison");
    assert_eq!(
        report.findings[0].node,
        "block:task:app.boolean_compare.main:stmt:2:expr:condition"
    );
    assert_eq!(report.findings[0].module, "app.boolean_compare");
    assert!(report.findings[0].message.contains("ready == true"));
    assert!(report.findings[0].hint.contains("ready"));
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("!nested"))
    );

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::RedundantBooleanComparison],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_absorbing_boolean_expressions() {
    let source = r#"
module app.absorbing_boolean

task main -> Bool {
  bind ready = true
  bind total = 1
  bind guarded = (total / 1 == 1) && false
  bind pure_or = ready || true
  bind short_or = true || (total / 1 == 1)

  return ready && false || false && (total / 1 == 1)
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
            rules: vec![LintRule::AbsorbingBooleanExpression],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.absorbing_boolean");
    assert_eq!(report.filters.rules, vec!["absorbing_boolean_expression"]);
    assert_eq!(
        report.findings.len(),
        4,
        "right-literal absorption should not drop division-bearing left operands"
    );
    assert_eq!(report.findings[0].id, "ABSORBING_BOOLEAN_EXPRESSION");
    assert_eq!(report.findings[0].rule, "absorbing_boolean_expression");
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.absorbing_boolean.main:stmt:3:expr"));
    assert!(nodes.contains(&"block:task:app.absorbing_boolean.main:stmt:4:expr"));
    assert!(nodes.contains(&"block:task:app.absorbing_boolean.main:stmt:5:expr:left"));
    assert!(nodes.contains(&"block:task:app.absorbing_boolean.main:stmt:5:expr:right"));
    assert_eq!(report.findings[0].module, "app.absorbing_boolean");
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.message.contains("ready && false"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("false"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.message.contains("false &&"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.message.contains("ready || true"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.message.contains("true ||"))
    );

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::AbsorbingBooleanExpression],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_idempotent_boolean_expressions() {
    let source = r#"
module app.idempotent_boolean

task main -> Bool {
  bind ready = true
  bind total = 1
  bind guarded = (total / 1 == 1) && (total / 1 == 1)
  bind repeated_and = ready && ready
  bind repeated_or = ready || ready

  return repeated_and || repeated_and
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
            rules: vec![LintRule::IdempotentBooleanExpression],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.idempotent_boolean");
    assert_eq!(report.filters.rules, vec!["idempotent_boolean_expression"]);
    assert_eq!(
        report.findings.len(),
        3,
        "idempotent boolean cleanup should not drop division-bearing duplicated operands"
    );
    assert_eq!(report.findings[0].id, "IDEMPOTENT_BOOLEAN_EXPRESSION");
    assert_eq!(report.findings[0].rule, "idempotent_boolean_expression");
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.idempotent_boolean.main:stmt:3:expr"));
    assert!(nodes.contains(&"block:task:app.idempotent_boolean.main:stmt:4:expr"));
    assert!(nodes.contains(&"block:task:app.idempotent_boolean.main:stmt:5:expr"));
    assert_eq!(report.findings[0].module, "app.idempotent_boolean");
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.message.contains("ready && ready"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.message.contains("ready || ready"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("ready"))
    );

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::IdempotentBooleanExpression],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_self_comparison_expressions() {
    let source = r#"
module app.self_compare

task main -> Bool {
  bind ready = true
  bind total = 1
  bind guarded = (total / 1 == 1) == (total / 1 == 1)
  bind strict = total < total
  bind inverted = ready != ready
  bind nonstrict_low = total <= total
  bind nonstrict_high = total >= total

  return ready == ready
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
            rules: vec![LintRule::SelfComparisonExpression],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.self_compare");
    assert_eq!(report.filters.rules, vec!["self_comparison_expression"]);
    assert_eq!(
        report.findings.len(),
        5,
        "self-comparison should not drop division-bearing duplicated operands"
    );
    assert_eq!(report.findings[0].id, "SELF_COMPARISON_EXPRESSION");
    assert_eq!(report.findings[0].rule, "self_comparison_expression");
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.self_compare.main:stmt:4:expr"));
    assert!(nodes.contains(&"block:task:app.self_compare.main:stmt:5:expr"));
    assert!(nodes.contains(&"block:task:app.self_compare.main:stmt:6:expr"));
    assert!(nodes.contains(&"block:task:app.self_compare.main:stmt:7:expr"));
    assert!(nodes.contains(&"block:task:app.self_compare.main:stmt:3:expr"));
    assert_eq!(report.findings[0].module, "app.self_compare");
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.message.contains("ready != ready"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.message.contains("ready == ready"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.message.contains("total < total"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("false"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("true"))
    );

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::SelfComparisonExpression],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_double_negation_expressions() {
    let source = r#"
module app.double_negation

task main -> Bool {
  bind ready = true
  bind nested = !ready

  return if !!ready { !!nested } else { true }
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
            rules: vec![LintRule::DoubleNegationExpression],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.double_negation");
    assert_eq!(report.filters.rules, vec!["double_negation_expression"]);
    assert_eq!(report.findings.len(), 2);
    assert_eq!(report.findings[0].id, "DOUBLE_NEGATION_EXPRESSION");
    assert_eq!(report.findings[0].rule, "double_negation_expression");
    assert_eq!(
        report.findings[0].node,
        "block:task:app.double_negation.main:stmt:2:expr:condition"
    );
    assert_eq!(report.findings[0].module, "app.double_negation");
    assert!(report.findings[0].message.contains("!!ready"));
    assert!(report.findings[0].hint.contains("ready"));
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("nested"))
    );

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::DoubleNegationExpression],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_negated_comparison_expressions() {
    let source = r#"
module app.negated_compare

task main -> Bool {
  bind limit = 7
  bind exact = !(limit == 7)
  bind text = !("a" != "b")
  bind low = !(limit < 3)

  return !(limit >= 3)
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
            rules: vec![LintRule::NegatedComparisonExpression],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.negated_compare");
    assert_eq!(report.filters.rules, vec!["negated_comparison_expression"]);
    assert_eq!(report.findings.len(), 4);
    assert_eq!(report.findings[0].id, "NEGATED_COMPARISON_EXPRESSION");
    assert_eq!(report.findings[0].rule, "negated_comparison_expression");
    let nodes: Vec<&str> = report
        .findings
        .iter()
        .map(|finding| finding.node.as_str())
        .collect();
    assert!(nodes.contains(&"block:task:app.negated_compare.main:stmt:1:expr"));
    assert!(nodes.contains(&"block:task:app.negated_compare.main:stmt:2:expr"));
    assert!(nodes.contains(&"block:task:app.negated_compare.main:stmt:3:expr"));
    assert!(nodes.contains(&"block:task:app.negated_compare.main:stmt:4:expr"));
    assert_eq!(report.findings[0].module, "app.negated_compare");
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("limit != 7"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("\"a\" == \"b\""))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("limit >= 3"))
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("limit < 3"))
    );

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::NegatedComparisonExpression],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_redundant_boolean_if_expressions() {
    let source = r#"
module app.boolean_if

task main -> Bool {
  bind ready = true
  bind nested = ready && true

  return if ready { true } else { false } && if nested { false } else { true }
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
            rules: vec![LintRule::RedundantBooleanIfExpression],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.boolean_if");
    assert_eq!(
        report.filters.rules,
        vec!["redundant_boolean_if_expression"]
    );
    assert_eq!(report.findings.len(), 2);
    assert_eq!(report.findings[0].id, "REDUNDANT_BOOLEAN_IF_EXPRESSION");
    assert_eq!(report.findings[0].rule, "redundant_boolean_if_expression");
    assert_eq!(
        report.findings[0].node,
        "block:task:app.boolean_if.main:stmt:2:expr:left"
    );
    assert_eq!(report.findings[0].module, "app.boolean_if");
    assert!(report.findings[0].message.contains("if ready"));
    assert!(report.findings[0].hint.contains("ready"));
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("!nested"))
    );

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::RedundantBooleanIfExpression],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_redundant_boolean_if_statements() {
    let source = r#"
module app.boolean_if_statement

task main -> Bool {
  bind ready = true
  bind nested = ready && true

  if ready {
    return true
  } else {
    return false
  }

  if nested {
    return false
  } else {
    return true
  }
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
            rules: vec![LintRule::RedundantBooleanIfStatement],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.boolean_if_statement");
    assert_eq!(report.filters.rules, vec!["redundant_boolean_if_statement"]);
    assert_eq!(report.findings.len(), 2);
    assert_eq!(report.findings[0].id, "REDUNDANT_BOOLEAN_IF_STATEMENT");
    assert_eq!(report.findings[0].rule, "redundant_boolean_if_statement");
    assert_eq!(
        report.findings[0].node,
        "block:task:app.boolean_if_statement.main:stmt:2"
    );
    assert_eq!(report.findings[0].module, "app.boolean_if_statement");
    assert!(report.findings[0].message.contains("boolean if statement"));
    assert!(report.findings[0].hint.contains("return ready"));
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.hint.contains("return !nested"))
    );

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::RedundantBooleanIfStatement],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_same_branch_if_expressions() {
    let source = r#"
module app.same_branch_if

task main -> Int {
  bind use_backup = false
  bind total = 1
  bind guarded = if total / 1 == 1 { 5 } else { 5 }

  return if use_backup { 41 } else { 41 } + guarded
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
            rules: vec![LintRule::SameBranchIfExpression],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.same_branch_if");
    assert_eq!(report.filters.rules, vec!["same_branch_if_expression"]);
    assert_eq!(
        report.findings.len(),
        1,
        "division-bearing conditions should not be dropped as same-branch simplifications"
    );
    assert_eq!(report.findings[0].id, "SAME_BRANCH_IF_EXPRESSION");
    assert_eq!(report.findings[0].rule, "same_branch_if_expression");
    assert_eq!(
        report.findings[0].node,
        "block:task:app.same_branch_if.main:stmt:3:expr:left"
    );
    assert_eq!(report.findings[0].module, "app.same_branch_if");
    assert!(report.findings[0].message.contains("identical branches"));
    assert!(report.findings[0].hint.contains("41"));

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::SameBranchIfExpression],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_same_branch_if_statements() {
    let source = r#"
module app.same_branch_if_statement

task main -> Int {
  bind total = 1

  if total / 1 == 1 {
    return 5
  } else {
    return 5
  }

  if total > 0 {
    return total
  } else {
    return total
  }
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
            rules: vec![LintRule::SameBranchIfStatement],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "app.same_branch_if_statement");
    assert_eq!(report.filters.rules, vec!["same_branch_if_statement"]);
    assert_eq!(
        report.findings.len(),
        1,
        "division-bearing conditions should not be dropped as same-branch statement simplifications"
    );
    assert_eq!(report.findings[0].id, "SAME_BRANCH_IF_STATEMENT");
    assert_eq!(report.findings[0].rule, "same_branch_if_statement");
    assert_eq!(
        report.findings[0].node,
        "block:task:app.same_branch_if_statement.main:stmt:2"
    );
    assert_eq!(report.findings[0].module, "app.same_branch_if_statement");
    assert!(report.findings[0].message.contains("identical branches"));
    assert!(report.findings[0].hint.contains("return total"));

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::SameBranchIfStatement],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
}

#[test]
fn lint_report_flags_missing_module_declarations() {
    let source = r#"
task main -> Text {
  return "hello"
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
            rules: vec![LintRule::MissingModuleDeclaration],
            module: None,
        },
    );

    assert_eq!(report.schema, LINT_REPORT_SCHEMA);
    assert_eq!(report.status, "findings");
    assert_eq!(report.entry_module, "main");
    assert_eq!(report.filters.rules, vec!["missing_module_declaration"]);
    assert_eq!(report.findings.len(), 1);
    assert_eq!(report.findings[0].id, "MISSING_MODULE_DECLARATION");
    assert_eq!(report.findings[0].rule, "missing_module_declaration");
    assert_eq!(report.findings[0].node, "program");
    assert_eq!(report.findings[0].module, "main");
    assert!(report.findings[0].hint.contains("module app.name"));

    let scoped_report = build_lint_report(
        &program,
        LintOptions {
            rules: vec![LintRule::MissingModuleDeclaration],
            module: Some("app.other".to_string()),
        },
    );
    assert_eq!(scoped_report.status, "ok");
    assert!(scoped_report.findings.is_empty());
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

    let planned = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "plan",
            "--json",
            "--graft-templates",
            "--emit-graft",
            "delete_unused_private_declarations",
        ])
        .arg(&root)
        .output()
        .expect("emit declaration cleanup graft");
    let planned_stdout = String::from_utf8(planned.stdout).expect("planned stdout utf8");
    assert!(
        planned.status.success(),
        "plan should emit declaration cleanup graft; stdout={planned_stdout} stderr={}",
        String::from_utf8_lossy(&planned.stderr)
    );
    let planned_graft: GraftInput =
        serde_json::from_str(&planned_stdout).expect("parse emitted cleanup graft");
    match &planned_graft {
        GraftInput::Transaction(transaction) => {
            assert_eq!(transaction.mode.as_deref(), Some("all_or_nothing"));
            assert_eq!(transaction.ops.len(), 2);
        }
        GraftInput::Operation(_) => panic!("expected cleanup transaction graft"),
    }
    fs::write(&graft_path, planned_stdout).expect("write cleanup graft");

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
fn project_fix_dry_run_and_write_apply_planned_cleanup() {
    let root = temp_project_dir("project-fix-declaration-cleanup");
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
    fs::write(&main_path, main_source).expect("write main module");

    let dry_run = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "fix",
            "--json",
            "--kind",
            "delete_unused_private_declarations",
            "--dry-run",
        ])
        .arg(&root)
        .output()
        .expect("dry-run declaration cleanup fix");
    let dry_stdout = String::from_utf8(dry_run.stdout).expect("dry-run stdout utf8");
    assert!(
        dry_run.status.success(),
        "fix dry-run should accept planned cleanup; stdout={dry_stdout} stderr={}",
        String::from_utf8_lossy(&dry_run.stderr)
    );
    let dry_outcome: GraftOutcome = serde_json::from_str(&dry_stdout).expect("parse dry outcome");
    assert_eq!(dry_outcome.schema, GRAFT_OUTCOME_SCHEMA);
    assert_eq!(dry_outcome.status, "accepted");
    assert_eq!(dry_outcome.provenance.len(), 2);
    assert_eq!(
        fs::read_to_string(&main_path).expect("read after dry-run"),
        main_source
    );
    assert!(!root.join(".sley/trace.jsonl").exists());

    let write = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "fix",
            "--json",
            "--kind",
            "delete_unused_private_declarations",
            "--write",
        ])
        .arg(&root)
        .output()
        .expect("write declaration cleanup fix");
    let write_stdout = String::from_utf8(write.stdout).expect("write stdout utf8");
    assert!(
        write.status.success(),
        "fix write should apply planned cleanup; stdout={write_stdout} stderr={}",
        String::from_utf8_lossy(&write.stderr)
    );
    let write_outcome: GraftOutcome =
        serde_json::from_str(&write_stdout).expect("parse write outcome");
    assert_eq!(write_outcome.status, "accepted");
    assert_eq!(write_outcome.provenance.len(), 2);
    assert!(
        write_outcome
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

    let _ = fs::remove_dir_all(root);
}

#[test]
fn fix_template_source_and_position_overrides_dry_run_without_writing() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target = repo_root.join("examples/collections.sley");
    let original = fs::read_to_string(&target).expect("read collections example");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "fix",
            "--json",
            "--kind",
            "insert_statement",
            "--template-surface",
            "block:task:app.collections.sum:stmt:2:body",
            "--source",
            "set index = index + 2",
            "--position",
            "1",
            "--dry-run",
        ])
        .arg(&target)
        .output()
        .expect("dry-run insert override fix");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        output.status.success(),
        "fix dry-run should accept overridden insert; stdout={stdout} stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let outcome: GraftOutcome = serde_json::from_str(&stdout).expect("parse outcome");
    assert_eq!(outcome.status, "accepted");
    assert_eq!(outcome.provenance[0].operation, "InsertStatement");
    let source = outcome.source.as_deref().expect("dry-run source");
    assert!(
        source.contains("set total = total + values[index]\n    set index = index + 2\n    set index = index + 1"),
        "{source}"
    );
    assert_eq!(
        fs::read_to_string(&target).expect("read collections after dry-run"),
        original
    );

    let snippet_root = temp_project_dir("fix-source-file-override");
    fs::create_dir_all(&snippet_root).expect("create source-file temp dir");
    let snippet = snippet_root.join("statement.sleypart");
    fs::write(&snippet, "set total = total + 10").expect("write source snippet");
    let file_output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "fix",
            "--json",
            "--kind",
            "insert_statement",
            "--template-surface",
            "block:task:app.collections.sum:stmt:2:body",
            "--source-file",
        ])
        .arg(&snippet)
        .args(["--position", "1", "--dry-run"])
        .arg(&target)
        .output()
        .expect("dry-run insert source-file override fix");
    let file_stdout = String::from_utf8(file_output.stdout).expect("stdout utf8");
    assert!(
        file_output.status.success(),
        "fix dry-run should accept source-file insert; stdout={file_stdout} stderr={}",
        String::from_utf8_lossy(&file_output.stderr)
    );
    let file_outcome: GraftOutcome = serde_json::from_str(&file_stdout).expect("parse outcome");
    assert_eq!(file_outcome.status, "accepted");
    assert!(
        file_outcome
            .source
            .as_deref()
            .expect("dry-run source")
            .contains("set total = total + 10"),
        "{file_outcome:#?}"
    );
    assert_eq!(
        fs::read_to_string(&target).expect("read collections after source-file dry-run"),
        original
    );
    let _ = fs::remove_dir_all(snippet_root);
}

#[test]
fn fix_template_name_and_type_overrides_dry_run_without_writing() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target = repo_root.join("examples/project");
    let main_file = target.join("src/app/main.sley");
    let original_main = fs::read_to_string(&main_file).expect("read main example");

    let add_take = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "fix",
            "--json",
            "--kind",
            "add_take",
            "--template-surface",
            "task:app.main.main",
            "--name",
            "limit",
            "--type",
            "Int",
            "--position",
            "0",
            "--dry-run",
        ])
        .arg(&target)
        .output()
        .expect("dry-run add-take name/type override fix");
    let add_take_stdout = String::from_utf8(add_take.stdout).expect("stdout utf8");
    assert!(
        add_take.status.success(),
        "fix dry-run should accept name/type add-take override; stdout={add_take_stdout} stderr={}",
        String::from_utf8_lossy(&add_take.stderr)
    );
    let add_take_outcome: GraftOutcome =
        serde_json::from_str(&add_take_stdout).expect("parse add-take outcome");
    assert_eq!(add_take_outcome.status, "accepted");
    assert_eq!(add_take_outcome.provenance[0].operation, "AddTake");
    assert!(
        add_take_outcome
            .source
            .as_deref()
            .expect("dry-run source")
            .contains("take limit: Int"),
        "{add_take_outcome:#?}"
    );
    assert_eq!(
        fs::read_to_string(&main_file).expect("read main after add-take dry-run"),
        original_main
    );

    let add_effect = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "fix",
            "--json",
            "--kind",
            "add_effect_declaration",
            "--template-surface",
            "program",
            "--name",
            "Audit",
            "--dry-run",
        ])
        .arg(&target)
        .output()
        .expect("dry-run add-effect name override fix");
    let add_effect_stdout = String::from_utf8(add_effect.stdout).expect("stdout utf8");
    assert!(
        add_effect.status.success(),
        "fix dry-run should accept name add-effect override; stdout={add_effect_stdout} stderr={}",
        String::from_utf8_lossy(&add_effect.stderr)
    );
    let add_effect_outcome: GraftOutcome =
        serde_json::from_str(&add_effect_stdout).expect("parse add-effect outcome");
    assert_eq!(add_effect_outcome.status, "accepted");
    assert_eq!(
        add_effect_outcome.provenance[0].operation,
        "AddEffectDeclaration"
    );
    assert!(
        add_effect_outcome
            .source
            .as_deref()
            .expect("dry-run source")
            .contains("effect Audit"),
        "{add_effect_outcome:#?}"
    );
    assert_eq!(
        fs::read_to_string(&main_file).expect("read main after add-effect dry-run"),
        original_main
    );
}

#[test]
fn fix_template_module_override_dry_run_without_writing() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target = repo_root.join("examples/project");
    let main_file = target.join("src/app/main.sley");
    let original_main = fs::read_to_string(&main_file).expect("read main example");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "fix",
            "--json",
            "--kind",
            "add_import",
            "--template-surface",
            "program",
            "--module",
            "app.extra",
            "--dry-run",
        ])
        .arg(&target)
        .output()
        .expect("dry-run add-import module override fix");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        output.status.success(),
        "fix dry-run should accept module add-import override; stdout={stdout} stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let outcome: GraftOutcome = serde_json::from_str(&stdout).expect("parse outcome");
    assert_eq!(outcome.status, "accepted");
    assert_eq!(outcome.provenance[0].operation, "AddImport");
    assert!(
        outcome
            .source
            .as_deref()
            .expect("dry-run source")
            .contains("import app.extra"),
        "{outcome:#?}"
    );
    assert_eq!(
        fs::read_to_string(&main_file).expect("read main after add-import dry-run"),
        original_main
    );
}

#[test]
fn fix_template_source_override_replaces_expression_and_rejects_unsupported_fields() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target = repo_root.join("examples/project");
    let math_file = repo_root.join("examples/project/src/app/math.sley");
    let original_math = fs::read_to_string(&math_file).expect("read math example");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "fix",
            "--json",
            "--kind",
            "replace_expression",
            "--template-surface",
            "block:task:app.math.double:stmt:0:expr:right",
            "--source",
            "3",
            "--dry-run",
        ])
        .arg(&target)
        .output()
        .expect("dry-run replace expression override fix");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        output.status.success(),
        "fix dry-run should accept overridden replacement; stdout={stdout} stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let outcome: GraftOutcome = serde_json::from_str(&stdout).expect("parse outcome");
    assert_eq!(outcome.status, "accepted");
    assert_eq!(outcome.provenance[0].operation, "ReplaceExpression");
    assert!(
        outcome
            .source
            .as_deref()
            .expect("dry-run source")
            .contains("return value * 3"),
        "{outcome:#?}"
    );
    assert_eq!(
        fs::read_to_string(&math_file).expect("read math after dry-run"),
        original_math
    );

    let unsupported = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "fix",
            "--json",
            "--kind",
            "move_take",
            "--template-surface",
            "take:task:app.math.double:0:value",
            "--source",
            "0",
            "--dry-run",
        ])
        .arg(&target)
        .output()
        .expect("dry-run unsupported source override");
    let unsupported_stdout = String::from_utf8(unsupported.stdout).expect("stdout utf8");
    assert!(
        !unsupported.status.success(),
        "unsupported override should reject; stdout={unsupported_stdout} stderr={}",
        String::from_utf8_lossy(&unsupported.stderr)
    );
    let rejected: GraftOutcome =
        serde_json::from_str(&unsupported_stdout).expect("parse rejected outcome");
    assert_eq!(rejected.status, "rejected");
    assert_eq!(rejected.diagnostics[0].id, "FIX_OVERRIDE_UNSUPPORTED");
}

#[test]
fn file_fix_write_adds_module_declaration() {
    let root = temp_project_dir("file-fix-module-declaration");
    fs::create_dir_all(&root).expect("create temp dir");
    let file = root.join("main.sley");
    fs::write(
        &file,
        r#"task main -> Text {
  return "hello"
}
"#,
    )
    .expect("write module-less source");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "fix",
            "--json",
            "--kind",
            "add_module_declaration",
            "--write",
        ])
        .arg(&file)
        .output()
        .expect("write module declaration fix");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        output.status.success(),
        "module fix should write; stdout={stdout} stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let outcome: GraftOutcome = serde_json::from_str(&stdout).expect("parse fix outcome");
    assert_eq!(outcome.status, "accepted");
    assert_eq!(outcome.provenance[0].operation, "AddModuleDeclaration");
    assert_eq!(
        fs::read_to_string(&file).expect("read fixed source"),
        "module main\n\ntask main -> Text {\n  return \"hello\"\n}\n"
    );
    let fixed_source = fs::read_to_string(&file).expect("read fixed source");
    let fixed = parse_program(&fixed_source).expect("parse fixed source");
    let lint = build_lint_report(&fixed, LintOptions::default());
    assert_eq!(lint.status, "ok", "unexpected lint findings: {lint:#?}");
    let receipts = read_trace_receipts(root.join(".sley/trace.jsonl")).expect("read trace");
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].provenance[0].operation, "AddModuleDeclaration");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_fix_write_respects_explicit_trace_path() {
    let root = temp_project_dir("file-fix-explicit-trace");
    fs::create_dir_all(&root).expect("create temp dir");
    let file = root.join("main.sley");
    let trace_path = root.join("receipts/custom-trace.jsonl");
    let default_trace_path = root.join(".sley/trace.jsonl");
    let source = r#"task main -> Text {
  return "hello"
}
"#;
    fs::write(&file, source).expect("write module-less source");

    let dry_run = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "fix",
            "--json",
            "--kind",
            "add_module_declaration",
            "--dry-run",
            "--actor",
            "agent:trace-test",
            "--trace",
        ])
        .arg(&trace_path)
        .arg(&file)
        .output()
        .expect("dry-run module declaration fix with explicit trace");
    let dry_stdout = String::from_utf8(dry_run.stdout).expect("dry-run stdout utf8");
    assert!(
        dry_run.status.success(),
        "dry-run module fix should accept; stdout={dry_stdout} stderr={}",
        String::from_utf8_lossy(&dry_run.stderr)
    );
    let dry_outcome: GraftOutcome =
        serde_json::from_str(&dry_stdout).expect("parse dry-run outcome");
    assert_eq!(dry_outcome.status, "accepted");
    assert_eq!(
        fs::read_to_string(&file).expect("read after dry-run"),
        source
    );
    assert!(!trace_path.exists());
    assert!(!default_trace_path.exists());

    let write = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "fix",
            "--json",
            "--kind",
            "add_module_declaration",
            "--write",
            "--actor",
            "agent:trace-test",
            "--trace",
        ])
        .arg(&trace_path)
        .arg(&file)
        .output()
        .expect("write module declaration fix with explicit trace");
    let write_stdout = String::from_utf8(write.stdout).expect("write stdout utf8");
    assert!(
        write.status.success(),
        "write module fix should accept; stdout={write_stdout} stderr={}",
        String::from_utf8_lossy(&write.stderr)
    );
    let write_outcome: GraftOutcome =
        serde_json::from_str(&write_stdout).expect("parse write outcome");
    assert_eq!(write_outcome.status, "accepted");
    assert_eq!(write_outcome.provenance[0].actor, "agent:trace-test");
    assert_eq!(
        write_outcome.provenance[0].operation,
        "AddModuleDeclaration"
    );
    assert_eq!(
        fs::read_to_string(&file).expect("read fixed source"),
        "module main\n\ntask main -> Text {\n  return \"hello\"\n}\n"
    );

    let receipts = read_trace_receipts(&trace_path).expect("read explicit trace");
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].schema, TRACE_RECEIPT_SCHEMA);
    assert_eq!(receipts[0].target, file.display().to_string());
    assert_eq!(receipts[0].provenance[0].actor, "agent:trace-test");
    assert_eq!(receipts[0].provenance[0].operation, "AddModuleDeclaration");
    assert!(
        !default_trace_path.exists(),
        "explicit trace path should not also write the default sidecar"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_fix_write_infers_project_relative_module_declaration() {
    let root = temp_project_dir("file-fix-project-relative-module");
    fs::create_dir_all(root.join("src/app/jobs")).expect("create project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"
[project]
entry = "app.main"
"#,
    )
    .expect("write manifest");
    let file = root.join("src/app/jobs/worker.sley");
    fs::write(
        &file,
        r#"task main -> Text {
  return "hello"
}
"#,
    )
    .expect("write module-less source");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "fix",
            "--json",
            "--kind",
            "add_module_declaration",
            "--write",
        ])
        .arg(&file)
        .output()
        .expect("write project-relative module declaration fix");
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(
        output.status.success(),
        "module fix should infer project-relative name; stdout={stdout} stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let outcome: GraftOutcome = serde_json::from_str(&stdout).expect("parse fix outcome");
    assert_eq!(outcome.status, "accepted");
    assert_eq!(
        outcome.provenance[0].targets,
        vec!["module:app.jobs.worker".to_string()]
    );
    assert_eq!(
        fs::read_to_string(&file).expect("read fixed source"),
        "module app.jobs.worker\n\ntask main -> Text {\n  return \"hello\"\n}\n"
    );
    let fixed_source = fs::read_to_string(&file).expect("read fixed source");
    let fixed = parse_program(&fixed_source).expect("parse fixed source");
    let lint = build_lint_report(&fixed, LintOptions::default());
    assert_eq!(lint.status, "ok", "unexpected lint findings: {lint:#?}");

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
fn project_graft_write_adds_import_to_existing_unloaded_module_file() {
    let root = temp_project_dir("project-graft-existing-import");
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
    let extra_source = r#"module app.extra

export task helper -> Int {
  return 2
}
"#;
    let main_path = root.join("src/app/main.sley");
    let extra_path = root.join("src/app/extra.sley");
    let graft_path = root.join("add_existing_import.json");
    fs::write(&main_path, main_source).expect("write main module");
    fs::write(&extra_path, extra_source).expect("write existing extra module");
    fs::write(
        &graft_path,
        r#"
{
  "op": "AddImport",
  "payload": { "module": "app.extra" }
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
        "project writeback should add imports to existing unloaded modules; stdout={stdout} stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let outcome: GraftOutcome = serde_json::from_str(&stdout).expect("parse outcome");
    assert_eq!(outcome.status, "accepted");
    assert_eq!(outcome.provenance[0].operation, "AddImport");
    assert_eq!(
        fs::read_to_string(&main_path).expect("read main"),
        "module app.main\n\nimport app.extra\n\ntask main -> Int {\n  return 1\n}\n"
    );
    assert_eq!(
        fs::read_to_string(&extra_path).expect("read extra"),
        extra_source
    );
    let check = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["check", "--json"])
        .arg(&root)
        .output()
        .expect("check project");
    assert!(
        check.status.success(),
        "imported existing module project should check; stdout={} stderr={}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    let receipts = read_trace_receipts(&root.join(".sley/trace.jsonl")).expect("read trace");
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].provenance[0].operation, "AddImport");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn project_fix_write_adds_import_to_existing_unloaded_module_file() {
    let root = temp_project_dir("project-fix-existing-import");
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
    let extra_source = r#"module app.extra

export task helper -> Int {
  return 2
}
"#;
    let main_path = root.join("src/app/main.sley");
    let extra_path = root.join("src/app/extra.sley");
    fs::write(&main_path, main_source).expect("write main module");
    fs::write(&extra_path, extra_source).expect("write existing extra module");

    let dry_run = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "fix",
            "--json",
            "--kind",
            "add_import",
            "--template-surface",
            "program",
            "--module",
            "app.extra",
            "--dry-run",
        ])
        .arg(&root)
        .output()
        .expect("dry-run add-import fix");
    let dry_stdout = String::from_utf8(dry_run.stdout).expect("dry-run stdout utf8");
    assert!(
        dry_run.status.success(),
        "fix dry-run should accept existing-module import; stdout={dry_stdout} stderr={}",
        String::from_utf8_lossy(&dry_run.stderr)
    );
    let dry_outcome: GraftOutcome = serde_json::from_str(&dry_stdout).expect("parse dry outcome");
    assert_eq!(dry_outcome.status, "accepted");
    assert_eq!(dry_outcome.provenance[0].operation, "AddImport");
    assert_eq!(
        fs::read_to_string(&main_path).expect("read main after dry-run"),
        main_source
    );
    assert!(!root.join(".sley/trace.jsonl").exists());

    let write = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args([
            "fix",
            "--json",
            "--kind",
            "add_import",
            "--template-surface",
            "program",
            "--module",
            "app.extra",
            "--write",
        ])
        .arg(&root)
        .output()
        .expect("write add-import fix");
    let write_stdout = String::from_utf8(write.stdout).expect("write stdout utf8");
    assert!(
        write.status.success(),
        "fix write should add existing-module import; stdout={write_stdout} stderr={}",
        String::from_utf8_lossy(&write.stderr)
    );
    let write_outcome: GraftOutcome =
        serde_json::from_str(&write_stdout).expect("parse write outcome");
    assert_eq!(write_outcome.status, "accepted");
    assert_eq!(write_outcome.provenance[0].operation, "AddImport");
    assert_eq!(
        fs::read_to_string(&main_path).expect("read main"),
        "module app.main\n\nimport app.extra\n\ntask main -> Int {\n  return 1\n}\n"
    );
    assert_eq!(
        fs::read_to_string(&extra_path).expect("read extra"),
        extra_source
    );
    let check = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["check", "--json"])
        .arg(&root)
        .output()
        .expect("check project");
    assert!(
        check.status.success(),
        "imported existing module project should check; stdout={} stderr={}",
        String::from_utf8_lossy(&check.stdout),
        String::from_utf8_lossy(&check.stderr)
    );
    let receipts = read_trace_receipts(&root.join(".sley/trace.jsonl")).expect("read trace");
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].provenance[0].operation, "AddImport");

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
        "accepted:agent-spend-authority",
        "accepted:agent-split-authority",
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
        "rejected:agent-transitive-effect",
        "rejected:spend-transitive-effect",
        "diagnostic:EFFECT_UNAUTHORIZED",
        "diagnostic:DUPLICATE_TASK",
        "diagnostic:DUPLICATE_TYPE",
        "diagnostic:DUPLICATE_EFFECT",
        "diagnostic:MISSING_RETURN",
        "diagnostic:TYPE_MISMATCH",
        "diagnostic:UNKNOWN_IDENTIFIER",
        "authority:transitive-effects",
        "language:module-namespace",
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
        for file in &case.setup_files {
            assert!(
                file.path.contains("{tmp}"),
                "CLI smoke {} setup file {} must use {{tmp}} so smokes do not mutate the repo",
                case.name,
                file.path
            );
        }
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
        "cli:deploy",
        "cli:ast",
        "cli:graph",
        "cli:graph-slice",
        "cli:query",
        "cli:lint",
        "cli:plan-emit-graft",
        "cli:fix",
        "cli:fix-write",
        "cli:trace",
        "cli:seal",
        "cli:zjx",
        "cli:graft-dry-run",
        "cli:graft-write",
        "cli:smoke-setup-files",
        "diagnostic:AST_NODE_NOT_FOUND",
        "diagnostic:MISSING_RETURN",
        "diagnostic:RUNTIME_CAPABILITY_SCOPE_DENIED",
        "fix:call-transaction-write",
        "fix:lint-cleanup-write",
        "fix:remove-take-transaction-write",
        "fix:payload-override-name",
        "fix:payload-override-module",
        "fix:write-source",
        "fix:payload-override-type",
        "graft:write-source",
        "graft:operations:add-module-declaration",
        "graft:operations:add-import",
        "graft:templates:constant-if-expression",
        "graft:templates:identity-binary-expression",
        "graft:templates:redundant-boolean-comparison",
        "graft:templates:absorbing-boolean-expression",
        "graft:templates:self-comparison-expression",
        "graft:templates:double-negation-expression",
        "graft:templates:negated-comparison-expression",
        "graft:templates:redundant-boolean-if-expression",
        "graft:templates:same-branch-if-expression",
        "graft:templates:add-task",
        "graft:templates:add-import",
        "graft:operations:add-take",
        "graft:operations:remove-task-effect",
        "graft:project-existing-import-write",
        "graph-slice:insert-affordances",
        "graft:templates:lint-declaration-delete",
        "graft:templates:lint-declaration-target",
        "graft:templates:module-name-inference",
        "graft:templates:missing-module",
        "graft:templates:program-surface-declarations",
        "graft:templates:raw-host-migration",
        "graft:templates:qualified-import-call",
        "graft:templates:replace-expression",
        "graft:templates:statement-surface-replace",
        "graft:templates:unchecked-result",
        "graft:templates:unused-pure-binding-delete",
        "graft:templates:unused-pure-expression-statement-delete",
        "graft:templates:self-assignment-statement-delete",
        "graft:templates:overwritten-set-statement-delete",
        "graft:transactions:redundant-initial-set-fold",
        "graft:transactions:redundant-initial-set-to-bind",
        "graft:templates:constant-if-statement",
        "graft:templates:constant-false-if-statement-delete",
        "graft:templates:constant-false-while-statement-delete",
        "graft:templates:constant-comparison-expression",
        "graft:templates:constant-arithmetic-expression",
        "graft:templates:absorbing-arithmetic-expression",
        "graft:templates:constant-text-concatenation-expression",
        "graft:templates:constant-list-index-expression",
        "graft:templates:constant-map-index-expression",
        "graft:templates:constant-record-field-access-expression",
        "graft:templates:constant-len-expression",
        "graft:templates:constant-not-expression",
        "graft:templates:empty-if-statement-delete",
        "graft:templates:empty-else-statement-remove",
        "graft:templates:empty-for-statement-delete",
        "graft:templates:empty-forge-statement-delete",
        "graft:templates:unreachable-statement-delete",
        "graft:templates:redundant-boolean-if-statement",
        "graft:templates:same-branch-if-statement",
        "graft:templates:idempotent-boolean-expression",
        "graft:templates:unused-declared-effect-remove",
        "graft:templates:unused-import-delete",
        "graft:templates:unused-private-task-delete",
        "graft:templates:unused-take-remove",
        "graft:transactions:dead-private-task-cleanup",
        "graft:transactions:lint-declaration-cleanup",
        "graft:transactions:mutable-binding-to-bind",
        "graft:transactions:remove-take-call-arg",
        "graft:transactions:rename-call-sites",
        "graph-slice:replace-affordances",
        "lint:unused_private_task",
        "lint:unreachable_private_task",
        "lint:unused_declared_effect",
        "lint:unused_import",
        "lint:unused_take",
        "lint:unused_private_type",
        "lint:unused_private_effect",
        "lint:raw_host_adapter",
        "lint:missing_module_declaration",
        "lint:unchecked_result",
        "lint:unqualified_imported_call",
        "lint:unused_pure_binding",
        "lint:unused_pure_expression_statement",
        "lint:mutable_binding_never_set",
        "lint:self_assignment_statement",
        "lint:overwritten_set_statement",
        "lint:redundant_initial_set_statement",
        "lint:constant_if_expression",
        "lint:constant_if_statement",
        "lint:constant_false_if_statement",
        "lint:constant_false_while_statement",
        "lint:constant_comparison_expression",
        "lint:constant_arithmetic_expression",
        "lint:absorbing_arithmetic_expression",
        "lint:constant_text_concatenation_expression",
        "lint:constant_list_index_expression",
        "lint:constant_map_index_expression",
        "lint:constant_record_field_access_expression",
        "lint:constant_len_expression",
        "lint:constant_not_expression",
        "lint:empty_if_statement",
        "lint:empty_else_statement",
        "lint:empty_for_statement",
        "lint:empty_forge_statement",
        "lint:identity_binary_expression",
        "lint:redundant_boolean_comparison",
        "lint:absorbing_boolean_expression",
        "lint:idempotent_boolean_expression",
        "lint:self_comparison_expression",
        "lint:double_negation_expression",
        "lint:negated_comparison_expression",
        "lint:redundant_boolean_if_expression",
        "lint:redundant_boolean_if_statement",
        "lint:same_branch_if_expression",
        "lint:same_branch_if_statement",
        "lint:unreachable_statement",
        "query:tasks",
        "query:types",
        "query:effects",
        "query:calls",
        "readiness:call-transaction-write-verify",
        "readiness:constant-if-repair-write-verify",
        "readiness:constant-if-statement-repair-write-verify",
        "readiness:constant-false-if-repair-write-verify",
        "readiness:constant-false-while-repair-write-verify",
        "readiness:constant-comparison-repair-write-verify",
        "readiness:constant-arithmetic-repair-write-verify",
        "readiness:absorbing-arithmetic-repair-write-verify",
        "readiness:constant-text-concatenation-repair-write-verify",
        "readiness:constant-list-index-repair-write-verify",
        "readiness:constant-map-index-repair-write-verify",
        "readiness:constant-record-field-access-repair-write-verify",
        "readiness:constant-len-repair-write-verify",
        "readiness:constant-not-repair-write-verify",
        "readiness:empty-if-repair-write-verify",
        "readiness:empty-for-repair-write-verify",
        "readiness:empty-forge-repair-write-verify",
        "readiness:deploy-package-artifacts",
        "readiness:deploy-package-dry-run",
        "readiness:inspect-calls",
        "readiness:deploy-lint-repair-write-verify",
        "readiness:identity-binary-repair-write-verify",
        "readiness:identity-text-concatenation-repair-write-verify",
        "readiness:redundant-boolean-repair-write-verify",
        "readiness:absorbing-boolean-repair-write-verify",
        "readiness:idempotent-boolean-repair-write-verify",
        "readiness:self-comparison-repair-write-verify",
        "readiness:double-negation-repair-write-verify",
        "readiness:negated-comparison-repair-write-verify",
        "readiness:redundant-boolean-if-repair-write-verify",
        "readiness:redundant-boolean-if-statement-repair-write-verify",
        "readiness:same-branch-if-repair-write-verify",
        "readiness:same-branch-if-statement-repair-write-verify",
        "readiness:unreachable-statement-repair-write-verify",
        "readiness:lint-repair-plan",
        "readiness:lint-repair-preview",
        "readiness:lint-repair-write-command",
        "readiness:lint-repair-write-verify",
        "readiness:mutable-binding-repair-write-verify",
        "readiness:self-assignment-repair-write-verify",
        "readiness:overwritten-set-repair-write-verify",
        "readiness:redundant-initial-set-repair-write-verify",
        "readiness:redundant-initial-set-to-bind-repair-write-verify",
        "readiness:unused-pure-expression-repair-write-verify",
        "readiness:project-lint-repair-write-verify",
        "readiness:project-import-write-verify",
        "readiness:remove-take-transaction-write-verify",
        "readiness:verify-package-next-action",
        "scaffold:agent-quickstart",
        "scaffold:deploy-quickstart",
        "scaffold:spend-quickstart",
        "scaffold:handoff-actions",
        "scaffold:next-actions",
        "scaffold:verify-ready",
        "host:DatabaseRead",
        "host:DatabaseWrite",
        "host:Deploy",
        "host:FileRead",
        "host:FileWrite",
        "host:ModelCall",
        "host:Network",
        "host:scoped-capability",
        "host:SecretRead",
        "host:Shell",
        "host:Spend",
        "project:imported-host-authority",
        "json:sley.ast.node.v0",
        "json:sley.ast.program.v0",
        "json:sley.diagnostics.report.v0",
        "json:sley.graft.outcome.v0",
        "json:sley.symbol_graph.v0",
        "json:sley.symbol_graph.slice.v0",
        "json:sley.query.report.v0",
        "json:sley.lint.report.v0",
        "json:sley.run.report.v0",
        "json:sley.project.scaffold.v0",
        "json:sley.doctor.report.v0",
        "json:sley.edit_plan.report.v0",
        "json:sley.verify.report.v0",
        "json:sley.deploy.report.v0",
        "json:sley.trace.receipt.v0",
        "json:sley.trace.report.v0",
        "json:sley.trace.seal.v0",
        "json:sley.zjx.envelope.v0",
        "trace:explicit-path",
        "trace:seal-with-receipts",
        "zjx:graph-digest",
        "zjx:trace-receipts",
    ];
    for tag in required {
        assert!(
            coverage.contains(tag),
            "CLI smoke manifest is missing required release coverage tag {tag}"
        );
    }
}

fn write_cli_smoke_setup_files(case: &CliSmokeCase, repo_root: &Path, tmp_root: &Path) {
    for file in &case.setup_files {
        let path = PathBuf::from(expand_cli_smoke_text(&file.path, repo_root, tmp_root));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap_or_else(|error| {
                panic!(
                    "create setup directory for CLI smoke {} file {}: {error}",
                    case.name,
                    path.display()
                )
            });
        }
        fs::write(
            &path,
            expand_cli_smoke_text(&file.content, repo_root, tmp_root),
        )
        .unwrap_or_else(|error| {
            panic!(
                "write setup file for CLI smoke {} file {}: {error}",
                case.name,
                path.display()
            )
        });
    }
}

fn expand_cli_smoke_args(args: &[String], repo_root: &Path, tmp_root: &Path) -> Vec<String> {
    args.iter()
        .map(|arg| expand_cli_smoke_text(arg, repo_root, tmp_root))
        .collect()
}

fn expand_cli_smoke_text(text: &str, repo_root: &Path, tmp_root: &Path) -> String {
    let repo = repo_root.to_string_lossy();
    let tmp = tmp_root.to_string_lossy();
    text.replace("{repo}", &repo).replace("{tmp}", &tmp)
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

fn assert_scope_denied(source: &str, gates: RuntimeGates, effect: &str, resource: &str) {
    let program = parse_program(source).expect("parse source");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
    let diagnostics = run_main_with_gates(&program, &gates).expect_err("scope should reject");
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "RUNTIME_CAPABILITY_SCOPE_DENIED"
                && diagnostic.message.contains(effect)
                && diagnostic.message.contains(resource)
        }),
        "expected scope diagnostic for {effect} and {resource}, got {diagnostics:#?}"
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
