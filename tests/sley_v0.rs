use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use sley::ast::{ExprKind, ProvenanceRecord, StatementKind};
use sley::checker::{check_program, has_errors};
use sley::formatter::format_program;
use sley::graft::{GraftInput, apply_graft_input};
use sley::parser::parse_program;
use sley::project::load_project;
use sley::runtime::{Value, run_main};
use sley::symbols::slice_symbol_graph;
use sley::trace::{append_trace_receipt, build_trace_receipt, read_trace_receipts};

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
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "GRAFT_PRECONDITION_FAILED")
    );
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
    assert_eq!(slice.outbound_calls.len(), 1);
    assert_eq!(slice.outbound_calls[0].callee, "math.double");
    assert_eq!(
        slice.outbound_calls[0].target.as_deref(),
        Some("app.math.double")
    );
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

fn temp_project_dir(name: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("sley-{name}-{}-{timestamp}", std::process::id()))
}
