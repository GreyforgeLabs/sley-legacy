use std::process::Command as ProcessCommand;

use serde_json::{Value as JsonValue, json};

mod support;

#[test]
fn agent_bench_runs_checked_repair_loop_and_records_evidence() {
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-agent-bench"))
        .args(["run", "--json", "--sley-bin", env!("CARGO_BIN_EXE_sley")])
        .output()
        .expect("run sley-agent-bench");
    assert!(
        output.status.success(),
        "agent bench failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    support::validate_report_schema("sley.agent_bench.report.v0", &output.stdout);
    let report: JsonValue = serde_json::from_slice(&output.stdout).expect("parse agent bench JSON");
    assert_eq!(
        report.pointer("/schema"),
        Some(&json!("sley.agent_bench.report.v0"))
    );
    assert_eq!(report.pointer("/status"), Some(&json!("passed")));
    assert_eq!(report.pointer("/case_count"), Some(&json!(1)));
    assert_eq!(
        report.pointer("/cases/0/name"),
        Some(&json!("unused-private-task-repair"))
    );
    assert_eq!(
        report.pointer("/cases/0/selected_repair/kind"),
        Some(&json!("delete_unused_private_task"))
    );
    assert_eq!(
        report.pointer("/cases/0/selected_repair/surface"),
        Some(&json!("task:app.bench.orphan"))
    );
    assert_eq!(
        report.pointer("/cases/0/trace_receipt_count"),
        Some(&json!(1))
    );
    assert_eq!(
        report.pointer("/cases/0/evidence/trace_receipt_count"),
        Some(&json!(1))
    );

    let step_names = report
        .pointer("/cases/0/steps")
        .and_then(JsonValue::as_array)
        .expect("case steps")
        .iter()
        .filter_map(|step| step.get("name").and_then(JsonValue::as_str))
        .collect::<Vec<_>>();
    assert_eq!(
        step_names,
        vec![
            "check_initial",
            "query_tasks_initial",
            "lint_initial_findings",
            "plan_repair",
            "fix_write",
            "lint_after_fix",
            "verify_after_fix",
            "seal_after_fix",
            "zjx_after_fix"
        ]
    );
    assert_eq!(
        report.pointer("/cases/0/steps/2/actual_success"),
        Some(&json!(false)),
        "initial lint should fail under --deny-warnings before the checked repair"
    );
    assert_eq!(
        report.pointer("/cases/0/steps/4/stdout_status"),
        Some(&json!("accepted"))
    );
}
