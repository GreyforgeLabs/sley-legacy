use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value as JsonValue, json};

mod support;

#[test]
fn migrate_reports_checked_source_migrations_and_schema_drift() {
    let root = temp_project_dir("migrate");
    let schema_dir = root.join("schemas");
    let fixture_dir = root.join("fixtures");
    fs::create_dir_all(&schema_dir).expect("create schema dir");
    fs::create_dir_all(&fixture_dir).expect("create fixture dir");
    let source_path = root.join("legacy_file.sley");
    fs::write(
        &source_path,
        r#"task main -> Result<Text, Error> uses FileRead {
  take gate files: Gate<FileRead>
  bind text = fs.read_text("examples/hello.sley")

  return Ok(text)
}
"#,
    )
    .expect("write legacy source");
    fs::write(
        schema_dir.join("sley.present.report.v0.schema.json"),
        r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "sley.present.report.v0",
  "type": "object"
}
"#,
    )
    .expect("write schema");
    fs::write(
        fixture_dir.join("missing-schema.json"),
        r#"{
  "schema": "sley.missing.report.v0"
}
"#,
    )
    .expect("write fixture");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-migrate"))
        .args([
            "report",
            "--json",
            "--schemas",
            path_str(&schema_dir).as_str(),
            "--fixtures",
            path_str(&fixture_dir).as_str(),
            path_str(&source_path).as_str(),
        ])
        .output()
        .expect("run sley-migrate");
    assert!(
        output.status.success(),
        "sley-migrate failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    support::validate_report_schema("sley.migrate.report.v0", &output.stdout);
    let report: JsonValue = serde_json::from_slice(&output.stdout).expect("parse migrate JSON");
    assert_eq!(
        report.pointer("/schema"),
        Some(&json!("sley.migrate.report.v0"))
    );
    assert_eq!(report.pointer("/status"), Some(&json!("migrations")));
    assert_eq!(report.pointer("/summary/migration_count"), Some(&json!(2)));
    assert_eq!(
        report.pointer("/summary/raw_host_adapter_count"),
        Some(&json!(1))
    );
    assert_eq!(
        report.pointer("/summary/module_declaration_count"),
        Some(&json!(1))
    );
    assert_eq!(
        report.pointer("/summary/schema_drift_count"),
        Some(&json!(2))
    );

    let kinds = report
        .pointer("/migrations")
        .and_then(JsonValue::as_array)
        .expect("migrations")
        .iter()
        .filter_map(|migration| migration.get("kind").and_then(JsonValue::as_str))
        .collect::<Vec<_>>();
    assert!(kinds.contains(&"add_module_declaration"));
    assert!(kinds.contains(&"migrate_raw_host_adapter"));
    assert_eq!(
        report.pointer("/migrations/1/operation/payload/source"),
        Some(&json!("fs.try_read_text(\"examples/hello.sley\")?"))
    );

    let drift_kinds = report
        .pointer("/schema_drift")
        .and_then(JsonValue::as_array)
        .expect("schema drift")
        .iter()
        .filter_map(|drift| drift.get("kind").and_then(JsonValue::as_str))
        .collect::<Vec<_>>();
    assert!(drift_kinds.contains(&"fixture_without_schema"));
    assert!(drift_kinds.contains(&"schema_without_fixture"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn migrate_reports_unchecked_result_binding_propagation() {
    let root = temp_project_dir("migrate-result-binding");
    fs::create_dir_all(&root).expect("create migrate result binding dir");
    let source_path = root.join("unchecked_binding.sley");
    fs::write(
        &source_path,
        r#"module app.migrate_result

task main -> Result<Text, Error> uses FileRead {
  bind ignored = fs.try_read_text("examples/hello.sley")
  bind used = fs.try_read_text("examples/hello.sley")?

  return Ok(used)
}
"#,
    )
    .expect("write unchecked result binding source");

    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-migrate"))
        .args(["report", "--json", path_str(&source_path).as_str()])
        .output()
        .expect("run sley-migrate");
    assert!(
        output.status.success(),
        "sley-migrate failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    support::validate_report_schema("sley.migrate.report.v0", &output.stdout);
    let report: JsonValue = serde_json::from_slice(&output.stdout).expect("parse migrate JSON");
    assert_eq!(report.pointer("/status"), Some(&json!("migrations")));
    assert_eq!(report.pointer("/summary/migration_count"), Some(&json!(1)));
    assert_eq!(
        report.pointer("/summary/result_propagation_count"),
        Some(&json!(1))
    );
    assert_eq!(
        report.pointer("/migrations/0/kind"),
        Some(&json!("propagate_unchecked_result_binding"))
    );
    assert_eq!(
        report.pointer("/migrations/0/category"),
        Some(&json!("result_propagation"))
    );
    assert_eq!(
        report.pointer("/migrations/0/operation/op"),
        Some(&json!("ReplaceStatement"))
    );
    assert_eq!(
        report.pointer("/migrations/0/operation/payload/source"),
        Some(&json!("fs.try_read_text(\"examples/hello.sley\")?"))
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
