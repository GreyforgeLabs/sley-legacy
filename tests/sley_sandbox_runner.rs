use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value as JsonValue, json};

mod support;

#[test]
fn sandbox_runner_replays_manifest_with_seeded_file_root() {
    let root = temp_project_dir("sandbox-runner");
    fs::create_dir_all(&root).expect("create temp root");
    let source_path = root.join("file_read.sley");
    let manifest_path = root.join("sandbox.json");
    fs::write(
        &source_path,
        r#"module app.sandbox

task main -> Result<Text, Error> uses FileRead {
  take gate files: Gate<FileRead>

  return fs.try_read_text("input.txt")
}
"#,
    )
    .expect("write source");
    fs::write(
        &manifest_path,
        format!(
            r#"{{
  "schema": "sley.sandbox.manifest.v0",
  "target": "{}",
  "deny_warnings": true,
  "capabilities": [
    {{ "effect": "FileRead", "root": "$sandbox" }}
  ],
  "files": [
    {{ "path": "input.txt", "text": "seeded file text" }}
  ]
}}
"#,
            source_path.to_string_lossy().replace('\\', "\\\\")
        ),
    )
    .expect("write manifest");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-sandbox-runner"))
        .args(["run", "--json", path_str(&manifest_path).as_str()])
        .output()
        .expect("run sley-sandbox-runner");
    assert!(
        output.status.success(),
        "sandbox runner failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    support::validate_report_schema("sley.sandbox.report.v0", &output.stdout);
    let report: JsonValue = serde_json::from_slice(&output.stdout).expect("parse sandbox report");
    assert_eq!(
        report.pointer("/schema"),
        Some(&json!("sley.sandbox.report.v0"))
    );
    assert_eq!(report.pointer("/status"), Some(&json!("passed")));
    assert_eq!(
        report.pointer("/manifest_schema"),
        Some(&json!("sley.sandbox.manifest.v0"))
    );
    assert_eq!(report.pointer("/summary/capability_count"), Some(&json!(1)));
    assert_eq!(report.pointer("/summary/file_seed_count"), Some(&json!(1)));
    assert_eq!(report.pointer("/verify/status"), Some(&json!("passed")));
    assert_eq!(
        report.pointer("/verify/runtime/value/value/value"),
        Some(&json!("seeded file text"))
    );
    assert_eq!(
        report.pointer("/sandbox_retained"),
        Some(&json!(false)),
        "successful run should clean the temp sandbox by default"
    );

    let _ = fs::remove_dir_all(root);
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
