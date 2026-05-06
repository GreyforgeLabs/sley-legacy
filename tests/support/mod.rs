use std::fs;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn validate_report_schema(schema_id: &str, report_json: &[u8]) {
    let report_path = temp_report_path(schema_id);
    fs::write(&report_path, report_json).expect("write report for schema validation");
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-contract"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            "validate",
            "--schema",
            schema_id,
            report_path.to_str().expect("report path"),
            "--schemas",
            "docs/schemas",
            "--json",
        ])
        .output()
        .expect("validate report schema");
    let _ = fs::remove_file(&report_path);
    assert!(
        output.status.success(),
        "{schema_id} schema validation failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn temp_report_path(schema_id: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is before unix epoch")
        .as_nanos();
    let safe_schema = schema_id.replace(['.', ':', '/', '\\'], "_");
    std::env::temp_dir().join(format!(
        "sley-schema-check-{safe_schema}-{}-{timestamp}.json",
        std::process::id()
    ))
}
