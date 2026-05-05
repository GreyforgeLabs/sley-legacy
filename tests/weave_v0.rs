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
