use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value as JsonValue, json};

mod support;

#[test]
fn docgen_generates_reference_report_and_markdown() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = temp_project_dir("docgen");
    fs::create_dir_all(&root).expect("create temp root");
    let markdown_path = root.join("reference.md");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-docgen"))
        .current_dir(&repo_root)
        .args([
            "reference",
            "--json",
            "--markdown",
            path_str(&markdown_path).as_str(),
            "examples/agent_deploy_pipeline.sley",
        ])
        .output()
        .expect("run sley-docgen");
    assert!(
        output.status.success(),
        "docgen failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    support::validate_report_schema("sley.docgen.report.v0", &output.stdout);
    let report: JsonValue = serde_json::from_slice(&output.stdout).expect("parse docgen report");
    assert_eq!(
        report.pointer("/schema"),
        Some(&json!("sley.docgen.report.v0"))
    );
    assert_eq!(report.pointer("/status"), Some(&json!("generated")));
    assert_eq!(
        report.pointer("/source_schema"),
        Some(&json!("sley.query.report.v0"))
    );
    assert_eq!(report.pointer("/summary/module_count"), Some(&json!(1)));
    assert_eq!(report.pointer("/summary/task_count"), Some(&json!(1)));
    assert_eq!(
        report.pointer("/summary/capability_count"),
        Some(&json!(10))
    );
    assert_eq!(
        report.pointer("/documents/0/path"),
        Some(&json!(path_str(&markdown_path)))
    );
    assert_eq!(
        report.pointer("/tasks/0/qualified_name"),
        Some(&json!("app.agent_deploy_pipeline.main"))
    );
    assert_eq!(
        report.pointer("/capabilities/0/seed_capability_args"),
        Some(&json!(["--cap", "DatabaseRead"]))
    );
    assert_eq!(
        report.pointer("/capabilities/2/seed_capability_args"),
        Some(&json!(["--cap", "Deploy"]))
    );

    let markdown = fs::read_to_string(&markdown_path).expect("read generated markdown");
    assert!(markdown.contains("# Sley Reference: app.agent_deploy_pipeline"));
    assert!(markdown.contains("## Tasks"));
    assert!(markdown.contains("`SecretRead, Network, ModelCall, Deploy`"));
    assert!(markdown.contains("## Capabilities"));
    assert!(markdown.contains("Seed capability args: `--cap Deploy`"));
    assert!(markdown.contains("`http.try_get_text`"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn docgen_filters_project_module_reference() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-docgen"))
        .current_dir(&repo_root)
        .args([
            "reference",
            "--json",
            "--module",
            "agent.pipeline",
            "examples/agent_project",
        ])
        .output()
        .expect("run sley-docgen");
    assert!(
        output.status.success(),
        "docgen failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    support::validate_report_schema("sley.docgen.report.v0", &output.stdout);
    let report: JsonValue = serde_json::from_slice(&output.stdout).expect("parse docgen report");
    assert_eq!(report.pointer("/status"), Some(&json!("generated")));
    assert_eq!(
        report.pointer("/target"),
        Some(&json!("examples/agent_project"))
    );
    assert_eq!(
        report.pointer("/filters/module"),
        Some(&json!("agent.pipeline"))
    );
    assert_eq!(
        report.pointer("/filters/exported_only"),
        Some(&json!(false))
    );
    assert_eq!(
        report.pointer("/documents/0/title"),
        Some(&json!("Sley Reference: agent.pipeline"))
    );
    assert_eq!(report.pointer("/summary/module_count"), Some(&json!(1)));
    assert_eq!(report.pointer("/summary/task_count"), Some(&json!(3)));
    assert_eq!(
        report.pointer("/modules/0/module"),
        Some(&json!("agent.pipeline"))
    );
    assert_eq!(
        report.pointer("/tasks/0/qualified_name"),
        Some(&json!("agent.pipeline.collect_profile"))
    );
    assert_eq!(
        report.pointer("/tasks/2/qualified_name"),
        Some(&json!("agent.pipeline.stage_release"))
    );
}

#[test]
fn docgen_blocks_unknown_project_module_filter() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-docgen"))
        .current_dir(&repo_root)
        .args([
            "reference",
            "--json",
            "--module",
            "agent.typo",
            "examples/agent_project",
        ])
        .output()
        .expect("run sley-docgen");
    assert!(
        !output.status.success(),
        "docgen unexpectedly passed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    support::validate_report_schema("sley.docgen.report.v0", &output.stdout);
    let report: JsonValue = serde_json::from_slice(&output.stdout).expect("parse docgen report");
    assert_eq!(report.pointer("/status"), Some(&json!("blocked")));
    assert_eq!(
        report.pointer("/filters/module"),
        Some(&json!("agent.typo"))
    );
    assert_eq!(report.pointer("/summary/module_count"), Some(&json!(0)));
    assert_eq!(report.pointer("/summary/task_count"), Some(&json!(0)));
    assert_eq!(report.pointer("/summary/issue_count"), Some(&json!(1)));
    assert_eq!(
        report.pointer("/issues/0/code"),
        Some(&json!("DOCGEN_MODULE_FILTER_NOT_FOUND"))
    );
}

fn path_str(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn temp_project_dir(name: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("sley-{name}-{}-{timestamp}", std::process::id()))
}
