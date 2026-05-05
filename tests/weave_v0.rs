use weavelang::ast::{ExprKind, StatementKind};
use weavelang::checker::{check_program, has_errors};
use weavelang::formatter::format_program;
use weavelang::parser::parse_program;
use weavelang::patch::{PatchInput, apply_patch_input};
use weavelang::runtime::{Value, run_main};

#[test]
fn profile_fixture_checks_cleanly() {
    let source = include_str!("../examples/profile_service.weave");
    let program = parse_program(source).expect("parse fixture");
    let diagnostics = check_program(&program);
    assert!(
        !has_errors(&diagnostics),
        "unexpected diagnostics: {diagnostics:#?}"
    );
}

#[test]
fn formatter_round_trips_profile_fixture() {
    let source = include_str!("../examples/profile_service.weave");
    let program = parse_program(source).expect("parse fixture");
    let formatted = format_program(&program);
    let reparsed = parse_program(&formatted).expect("parse formatted fixture");
    assert_eq!(formatted, format_program(&reparsed));
}

#[test]
fn add_parameter_patch_is_structural_and_checked() {
    let source = include_str!("../examples/profile_service.weave");
    let patch_source = include_str!("../fixtures/patches/add_tenant_parameter.json");
    let program = parse_program(source).expect("parse fixture");
    let patch: PatchInput = serde_json::from_str(patch_source).expect("parse patch");
    let outcome = apply_patch_input(&program, patch, Some("agent:test".to_string()));
    assert_eq!(outcome.status, "accepted");
    let patched_source = outcome.source.expect("patched source");
    assert!(patched_source.contains("fn get_user(tenant_id: Text, id: Text)"));
    assert_eq!(outcome.provenance.len(), 1);
}

#[test]
fn stale_precondition_rejects_patch() {
    let source = include_str!("../examples/profile_service.weave");
    let patch_source = include_str!("../fixtures/patches/stale_add_tenant_parameter.json");
    let program = parse_program(source).expect("parse fixture");
    let patch: PatchInput = serde_json::from_str(patch_source).expect("parse patch");
    let outcome = apply_patch_input(&program, patch, Some("agent:test".to_string()));
    assert_eq!(outcome.status, "rejected");
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "PATCH_PRECONDITION_FAILED")
    );
}

#[test]
fn parser_builds_structured_call_and_record_expressions() {
    let source = include_str!("../examples/profile_service.weave");
    let program = parse_program(source).expect("parse fixture");
    let function = &program.functions[0];

    let StatementKind::Let {
        expr: query_expr, ..
    } = &function.body.statements[0].kind
    else {
        panic!("expected let statement");
    };
    assert!(matches!(query_expr.kind, ExprKind::Try { .. }));

    let StatementKind::Return { expr: return_expr } = &function.body.statements[1].kind else {
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
fn grade(score: Int) -> Text {
  return if score >= 90 { "A" } else { "B" }
}
"#;
    let program = parse_program(source).expect("parse source");
    let StatementKind::Return { expr } = &program.functions[0].body.statements[0].kind else {
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
fn main() -> Int {
  let values = [1, 2, 3]
  let index = 0
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
        program.functions[0].body.statements[0].kind,
        StatementKind::Let { .. }
    ));
    assert!(matches!(
        program.functions[0].body.statements[2].kind,
        StatementKind::If { .. }
    ));
    let StatementKind::Return { expr } = &program.functions[0].body.statements[3].kind else {
        panic!("expected return statement");
    };
    assert!(matches!(expr.kind, ExprKind::Index { .. }));
}

#[test]
fn parser_builds_for_loops_and_map_literals() {
    let source = r#"
fn main() -> Int {
  let scores: Map<Text, Int> = map { "ada": 3, "grace": 5 }
  let total = 0
  for name in ["ada", "grace"] {
    set total = total + scores[name]
  }
  return total
}
"#;
    let program = parse_program(source).expect("parse source");
    let StatementKind::Let { expr, .. } = &program.functions[0].body.statements[0].kind else {
        panic!("expected map let statement");
    };
    assert!(matches!(expr.kind, ExprKind::MapLiteral { .. }));
    assert!(matches!(
        program.functions[0].body.statements[2].kind,
        StatementKind::For { .. }
    ));
}

#[test]
fn checker_validates_user_function_calls() {
    let source = r#"
fn takes_text(value: Text) -> Text {
  return value
}

fn main() -> Text {
  return takes_text(42)
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
fn main() -> Int {
  let invalid = 1 + "x"
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
fn main() -> Int {
  let mixed = [1, "x"]
  let values = [1, 2]
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
fn main() -> Int {
  let scores = map { 1: 2, "two": "bad" }
  for score in scores {
    set missing = score
  }
  let valid = map { "one": 1 }
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
fn checker_propagates_called_function_effects() {
    let source = r#"
effect FileRead

fn read(path: Text) -> Result<Text, Error> uses FileRead {
  return fs.read_text(path)?
}

fn main() -> Result<Text, Error> {
  return read("x")
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
fn score(base: Int) -> Int {
  let doubled = base * 2
  return if doubled >= 10 && true { doubled + 1 } else { 0 }
}

fn main() -> Int {
  return score(5)
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
fn sum(values: List<Int>) -> Int {
  let index = 0
  let total = 0
  while index < len(values) {
    set total = total + values[index]
    set index = index + 1
  }
  return total
}

fn main() -> Int {
  let values = [2, 3, 5]
  return sum(values)
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
fn main() -> Int {
  let names = ["ada", "grace"]
  let scores: Map<Text, Int> = map { "ada": 3, "grace": 5 }
  let total = 0
  for name in names {
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
fn runtime_evaluates_pure_function_calls_and_record_fields() {
    let source = r#"
type User = {
  name: Text
}

fn make_user(name: Text) -> User {
  return User { name: name }
}

fn main() -> Text {
  let user = make_user("Ada")
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
