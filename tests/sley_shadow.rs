use std::path::PathBuf;
use std::process::Command as ProcessCommand;

use serde_json::{Value as JsonValue, json};

mod support;

#[test]
fn shadow_reports_query_lint_links_and_authority_seeds() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-shadow"))
        .current_dir(&repo_root)
        .args(["report", "--json", "examples/agent_deploy_pipeline.sley"])
        .output()
        .expect("run sley-shadow");
    assert!(
        output.status.success(),
        "shadow failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    support::validate_report_schema("sley.shadow.report.v0", &output.stdout);
    let report: JsonValue = serde_json::from_slice(&output.stdout).expect("parse shadow report");
    assert_eq!(
        report.pointer("/schema"),
        Some(&json!("sley.shadow.report.v0"))
    );
    assert_eq!(report.pointer("/status"), Some(&json!("clean")));
    assert_eq!(report.pointer("/filters/module"), Some(&json!(null)));
    assert_eq!(report.pointer("/filters/rules"), Some(&json!([])));
    assert_eq!(
        report.pointer("/source_schemas/query"),
        Some(&json!("sley.query.report.v0"))
    );
    assert_eq!(
        report.pointer("/source_schemas/lint"),
        Some(&json!("sley.lint.report.v0"))
    );
    assert_eq!(report.pointer("/summary/module_count"), Some(&json!(1)));
    assert_eq!(report.pointer("/summary/task_count"), Some(&json!(1)));
    assert_eq!(
        report.pointer("/summary/effectful_task_count"),
        Some(&json!(1))
    );
    assert_eq!(
        report.pointer("/summary/authority_seed_count"),
        Some(&json!(1))
    );
    assert_eq!(
        report.pointer("/authority_seeds/0/task_id"),
        Some(&json!("task:app.agent_deploy_pipeline.main"))
    );
    assert_eq!(
        report.pointer("/authority_seeds/0/command_args"),
        Some(&json!([
            "--cap",
            "SecretRead",
            "--cap",
            "Network",
            "--cap",
            "ModelCall",
            "--cap",
            "Deploy",
            "--secret",
            "api_key",
            "redacted",
            "--http-text",
            "https://example.test/profile",
            "profile ready",
            "--model-output",
            "deploy-plan",
            "plan approved",
            "--deploy-result",
            "staging",
            "staged"
        ]))
    );
}

#[test]
fn shadow_reports_multi_module_project_authority_seeds() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-shadow"))
        .current_dir(&repo_root)
        .args(["report", "--json", "examples/agent_project"])
        .output()
        .expect("run sley-shadow");
    assert!(
        output.status.success(),
        "shadow failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    support::validate_report_schema("sley.shadow.report.v0", &output.stdout);
    let report: JsonValue = serde_json::from_slice(&output.stdout).expect("parse shadow report");
    assert_eq!(report.pointer("/status"), Some(&json!("clean")));
    assert_eq!(
        report.pointer("/target"),
        Some(&json!("examples/agent_project"))
    );
    assert_eq!(report.pointer("/filters/module"), Some(&json!(null)));
    assert_eq!(report.pointer("/filters/rules"), Some(&json!([])));
    assert_eq!(report.pointer("/summary/module_count"), Some(&json!(2)));
    assert_eq!(report.pointer("/summary/task_count"), Some(&json!(4)));
    assert_eq!(
        report.pointer("/summary/effectful_task_count"),
        Some(&json!(4))
    );
    assert_eq!(report.pointer("/summary/call_count"), Some(&json!(11)));
    assert_eq!(
        report.pointer("/summary/authority_seed_count"),
        Some(&json!(4))
    );
    assert_eq!(
        report.pointer("/authority_seeds/0/qualified_name"),
        Some(&json!("agent.main.main"))
    );
    assert_eq!(
        report.pointer("/authority_seeds/1/qualified_name"),
        Some(&json!("agent.pipeline.collect_profile"))
    );
    assert_eq!(
        report.pointer("/authority_seeds/0/command_args/8"),
        Some(&json!("--secret"))
    );
    assert_eq!(
        report.pointer("/authority_seeds/0/command_args/13"),
        Some(&json!("profile ready"))
    );
    assert_eq!(
        report.pointer("/authority_seeds/3/command_args"),
        Some(&json!(["--cap", "Deploy"]))
    );
}

#[test]
fn shadow_module_filter_reports_project_slice() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-shadow"))
        .current_dir(&repo_root)
        .args([
            "report",
            "--json",
            "--module",
            "agent.pipeline",
            "examples/agent_project",
        ])
        .output()
        .expect("run sley-shadow");
    assert!(
        output.status.success(),
        "shadow failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    support::validate_report_schema("sley.shadow.report.v0", &output.stdout);
    let report: JsonValue = serde_json::from_slice(&output.stdout).expect("parse shadow report");
    assert_eq!(report.pointer("/status"), Some(&json!("clean")));
    assert_eq!(
        report.pointer("/filters/module"),
        Some(&json!("agent.pipeline"))
    );
    assert_eq!(report.pointer("/filters/rules"), Some(&json!([])));
    assert_eq!(report.pointer("/summary/module_count"), Some(&json!(1)));
    assert_eq!(report.pointer("/summary/task_count"), Some(&json!(3)));
    assert_eq!(
        report.pointer("/summary/effectful_task_count"),
        Some(&json!(3))
    );
    assert_eq!(report.pointer("/summary/call_count"), Some(&json!(10)));
    assert_eq!(
        report.pointer("/summary/authority_seed_count"),
        Some(&json!(3))
    );
    assert_eq!(
        report.pointer("/authority_seeds/0/qualified_name"),
        Some(&json!("agent.pipeline.collect_profile"))
    );
    assert_eq!(
        report.pointer("/authority_seeds/2/command_args"),
        Some(&json!(["--cap", "Deploy"]))
    );
}

#[test]
fn shadow_blocks_unknown_project_module_filter() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-shadow"))
        .current_dir(&repo_root)
        .args([
            "report",
            "--json",
            "--module",
            "agent.typo",
            "examples/agent_project",
        ])
        .output()
        .expect("run sley-shadow");
    assert!(
        !output.status.success(),
        "shadow unexpectedly passed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    support::validate_report_schema("sley.shadow.report.v0", &output.stdout);
    let report: JsonValue = serde_json::from_slice(&output.stdout).expect("parse shadow report");
    assert_eq!(report.pointer("/status"), Some(&json!("blocked")));
    assert_eq!(
        report.pointer("/filters/module"),
        Some(&json!("agent.typo"))
    );
    assert_eq!(report.pointer("/summary/module_count"), Some(&json!(0)));
    assert_eq!(report.pointer("/summary/task_count"), Some(&json!(0)));
    assert_eq!(
        report.pointer("/summary/lint_finding_count"),
        Some(&json!(0))
    );
    assert_eq!(report.pointer("/summary/issue_count"), Some(&json!(1)));
    assert_eq!(
        report.pointer("/issues/0/code"),
        Some(&json!("SHADOW_MODULE_FILTER_NOT_FOUND"))
    );
}

#[test]
fn shadow_links_lint_findings_to_query_rows() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-shadow"))
        .current_dir(&repo_root)
        .args([
            "report",
            "--json",
            "--rule",
            "unused_private_task",
            "examples/unused_private_task.sley",
        ])
        .output()
        .expect("run sley-shadow");
    assert!(
        output.status.success(),
        "shadow failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    support::validate_report_schema("sley.shadow.report.v0", &output.stdout);
    let report: JsonValue = serde_json::from_slice(&output.stdout).expect("parse shadow report");
    assert_eq!(report.pointer("/status"), Some(&json!("findings")));
    assert_eq!(report.pointer("/filters/module"), Some(&json!(null)));
    assert_eq!(
        report.pointer("/filters/rules"),
        Some(&json!(["unused_private_task"]))
    );
    assert_eq!(
        report.pointer("/summary/lint_finding_count"),
        Some(&json!(1))
    );
    assert_eq!(
        report.pointer("/summary/linked_lint_finding_count"),
        Some(&json!(1))
    );
    assert_eq!(
        report.pointer("/summary/unlinked_lint_finding_count"),
        Some(&json!(0))
    );
    assert_eq!(report.pointer("/lint_links/0/linked"), Some(&json!(true)));
    assert_eq!(
        report.pointer("/lint_links/0/target_kind"),
        Some(&json!("task"))
    );
    assert_eq!(
        report.pointer("/lint_links/0/qualified_name"),
        Some(&json!("app.tasks.orphan"))
    );
    assert_eq!(
        report.pointer("/lint_links/0/node"),
        Some(&json!("task:app.tasks.orphan"))
    );
    assert_eq!(
        report.pointer("/lint_links/0/plan_command"),
        Some(&json!([
            "sley",
            "plan",
            "--json",
            "--graft-templates",
            "--template-surface",
            "task:app.tasks.orphan",
            "examples/unused_private_task.sley"
        ]))
    );
}

#[test]
fn shadow_links_statement_lint_findings_to_owner_task() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-shadow"))
        .current_dir(&repo_root)
        .args([
            "report",
            "--json",
            "--rule",
            "empty_while_statement",
            "examples/empty_while_statement.sley",
        ])
        .output()
        .expect("run sley-shadow");
    assert!(
        output.status.success(),
        "shadow failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    support::validate_report_schema("sley.shadow.report.v0", &output.stdout);
    let report: JsonValue = serde_json::from_slice(&output.stdout).expect("parse shadow report");
    assert_eq!(report.pointer("/status"), Some(&json!("findings")));
    assert_eq!(
        report.pointer("/filters/rules"),
        Some(&json!(["empty_while_statement"]))
    );
    assert_eq!(
        report.pointer("/summary/linked_lint_finding_count"),
        Some(&json!(1))
    );
    assert_eq!(
        report.pointer("/summary/unlinked_lint_finding_count"),
        Some(&json!(0))
    );
    assert_eq!(
        report.pointer("/lint_links/0/finding_id"),
        Some(&json!("EMPTY_WHILE_STATEMENT"))
    );
    assert_eq!(
        report.pointer("/lint_links/0/target_kind"),
        Some(&json!("task"))
    );
    assert_eq!(
        report.pointer("/lint_links/0/qualified_name"),
        Some(&json!("app.empty_while.main"))
    );
    assert_eq!(
        report.pointer("/lint_links/0/node"),
        Some(&json!("block:task:app.empty_while.main:stmt:2"))
    );
    assert_eq!(
        report.pointer("/lint_links/0/plan_command"),
        Some(&json!([
            "sley",
            "plan",
            "--json",
            "--graft-templates",
            "--template-surface",
            "block:task:app.empty_while.main:stmt:2",
            "examples/empty_while_statement.sley"
        ]))
    );
}
