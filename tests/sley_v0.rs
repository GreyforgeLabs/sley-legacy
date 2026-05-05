use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};

use sley::ast::{AST_PROGRAM_SCHEMA, ExprKind, ProvenanceRecord, StatementKind};
use sley::checker::{check_program, has_errors};
use sley::diagnostics::{DIAGNOSTIC_REPORT_SCHEMA, DiagnosticReport};
use sley::formatter::format_program;
use sley::graft::{GRAFT_OUTCOME_SCHEMA, GraftInput, GraftOutcome, apply_graft_input};
use sley::parser::parse_program;
use sley::project::load_project;
use sley::runtime::{RuntimeGates, Value, run_main, run_main_with_gates};
use sley::symbols::{SYMBOL_GRAPH_SCHEMA, SYMBOL_GRAPH_SLICE_SCHEMA, slice_symbol_graph};
use sley::trace::{
    TRACE_RECEIPT_SCHEMA, TRACE_SEAL_SCHEMA, TraceReceipt, append_trace_receipt,
    build_trace_receipt, build_trace_seal, read_trace_receipts,
};

#[derive(Debug, serde::Deserialize)]
struct CorpusExpectation {
    diagnostics: Vec<String>,
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
    let accepted_files =
        collect_sley_files(&corpus_root.join("accepted")).expect("collect accepted corpus");
    assert!(
        !accepted_files.is_empty(),
        "accepted corpus should contain at least one fixture"
    );

    for file in accepted_files {
        let source =
            fs::read_to_string(&file).unwrap_or_else(|error| panic!("read {file:?}: {error}"));
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

    let rejected_files =
        collect_sley_files(&corpus_root.join("rejected")).expect("collect rejected corpus");
    assert!(
        !rejected_files.is_empty(),
        "rejected corpus should contain at least one fixture"
    );

    for file in rejected_files {
        let expectation_path = file.with_extension("json");
        let expectation_source = fs::read_to_string(&expectation_path).unwrap_or_else(|error| {
            panic!("read rejected corpus expectation {expectation_path:?}: {error}")
        });
        let expectation: CorpusExpectation = serde_json::from_str(&expectation_source)
            .unwrap_or_else(|error| {
                panic!("parse rejected corpus expectation {expectation_path:?}: {error}")
            });
        let source =
            fs::read_to_string(&file).unwrap_or_else(|error| panic!("read {file:?}: {error}"));
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
        include_str!("../docs/schemas/sley.zjx.envelope.v0.schema.json"),
        "sley.zjx.envelope.v0",
    );
    assert_schema_file(
        include_str!("../docs/schemas/sley.trace.seal.v0.schema.json"),
        TRACE_SEAL_SCHEMA,
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
    assert_has_repair_hint(&diagnostics, "IF_CONDITION_NOT_BOOL", "replace_condition");
    assert_has_repair_hint(
        &diagnostics,
        "CALL_ARGUMENT_TYPE_MISMATCH",
        "replace_argument",
    );
    assert_has_repair_hint(&diagnostics, "RETURN_TYPE_MISMATCH", "change_return_type");
    assert_has_repair_hint(&diagnostics, "UNKNOWN_IDENTIFIER", "declare_binding");

    let unknown_task =
        parse_program("task main -> Int {\n  return call missing()\n}\n").expect("parse source");
    let diagnostics = check_program(&unknown_task);
    assert_has_repair_hint(&diagnostics, "UNKNOWN_TASK", "declare_or_import_task");
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
fn project_graft_write_rejects_new_module_writeback_without_mutation() {
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
  "op": "AddTask",
  "payload": {
    "source": "module app.extra\n\ntask helper -> Int {\n  return 2\n}\n"
  }
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
        "project writeback should reject new module writes"
    );
    let outcome: GraftOutcome = serde_json::from_str(&stdout).expect("parse outcome");
    assert_eq!(outcome.status, "rejected");
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "PROJECT_WRITEBACK_UNKNOWN_MODULE"),
        "expected PROJECT_WRITEBACK_UNKNOWN_MODULE, got {:#?}",
        outcome.diagnostics
    );
    assert_eq!(
        fs::read_to_string(&main_path).expect("read main"),
        main_source
    );
    assert!(!root.join("src/app/extra.sley").exists());
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

fn temp_project_dir(name: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("sley-{name}-{}-{timestamp}", std::process::id()))
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
