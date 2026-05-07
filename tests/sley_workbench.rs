use std::fs;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value as JsonValue, json};

mod support;

#[test]
fn workbench_reports_lint_repairs_and_writes_static_html_when_requested() {
    let root = temp_project_dir("workbench");
    fs::create_dir_all(&root).expect("create workbench temp dir");
    let source_path = root.join("main.sley");
    let html_path = root.join("workbench.html");
    fs::write(
        &source_path,
        r#"module app.workbench

task main -> Text {
  return "hello"
}

task orphan -> Text {
  return "unused"
}
"#,
    )
    .expect("write workbench source");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-workbench"))
        .args([
            "--json",
            "--html",
            html_path.to_str().expect("html path"),
            "--slice",
            "task:app.workbench.orphan",
            source_path.to_str().expect("source path"),
        ])
        .output()
        .expect("run sley-workbench");
    assert!(
        output.status.success(),
        "workbench failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    support::validate_report_schema("sley.workbench.report.v0", &output.stdout);
    let report: JsonValue = serde_json::from_slice(&output.stdout).expect("parse workbench JSON");
    assert_eq!(
        report.pointer("/schema"),
        Some(&json!("sley.workbench.report.v0"))
    );
    assert_eq!(report.pointer("/status"), Some(&json!("warnings")));
    assert_eq!(report.pointer("/lint/status"), Some(&json!("findings")));
    assert_eq!(report.pointer("/summary/module_count"), Some(&json!(1)));
    assert_eq!(report.pointer("/summary/task_count"), Some(&json!(2)));
    assert_eq!(
        report.pointer("/summary/lint_finding_count"),
        Some(&json!(1))
    );
    assert_eq!(
        report.pointer("/graph_slice/focus/id"),
        Some(&json!("task:app.workbench.orphan"))
    );
    assert_eq!(
        report.pointer("/graph_slice/focus/kind"),
        Some(&json!("task"))
    );
    let template_kinds = report
        .pointer("/plan/graft_templates")
        .and_then(JsonValue::as_array)
        .expect("graft templates")
        .iter()
        .filter_map(|template| template.get("kind").and_then(JsonValue::as_str))
        .collect::<Vec<_>>();
    assert!(
        template_kinds.contains(&"delete_unused_private_task"),
        "expected unused private task template, got {template_kinds:?}"
    );
    assert_eq!(
        report.pointer("/html_path"),
        Some(&json!(html_path.to_string_lossy()))
    );

    let html = fs::read_to_string(&html_path).expect("read workbench HTML");
    assert!(html.contains("Sley Workbench"));
    assert!(html.contains("Graph Slice"));
    assert!(html.contains("app.workbench.orphan"));
    assert!(html.contains("Delete Affordances"));
    assert!(html.contains("delete_unused_private_task"));
}

fn temp_project_dir(name: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("sley-{name}-{}-{timestamp}", std::process::id()))
}
