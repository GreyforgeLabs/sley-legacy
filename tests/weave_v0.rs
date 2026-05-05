use weavelang::checker::{check_program, has_errors};
use weavelang::formatter::format_program;
use weavelang::parser::parse_program;
use weavelang::patch::{PatchInput, apply_patch_input};

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
