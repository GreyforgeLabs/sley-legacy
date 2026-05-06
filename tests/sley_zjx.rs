use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value as JsonValue, json};

mod support;

#[test]
fn zjx_tool_inspects_extracts_diffs_and_rejects_tampered_digest() {
    let root = temp_project_dir("zjx-tool");
    fs::create_dir_all(&root).expect("create temp root");
    let hello_envelope = root.join("hello-envelope.json");
    let project_envelope = root.join("project-envelope.json");
    write_zjx_envelope("examples/hello.sley", &hello_envelope);
    write_zjx_envelope("examples/project", &project_envelope);

    let inspect = run_zjx_success(&["inspect", "--json", path_str(&hello_envelope).as_str()]);
    assert_eq!(
        inspect.pointer("/schema"),
        Some(&json!("sley.zjx.tool.report.v0"))
    );
    assert_eq!(inspect.pointer("/command"), Some(&json!("inspect")));
    assert_eq!(
        inspect.pointer("/envelopes/0/graph_digest_match"),
        Some(&json!(true))
    );
    assert_eq!(
        inspect.pointer("/envelopes/0/module_count"),
        Some(&json!(1))
    );
    assert_eq!(inspect.pointer("/envelopes/0/task_count"), Some(&json!(1)));

    let validate = run_zjx_success(&["validate", "--json", path_str(&hello_envelope).as_str()]);
    assert_eq!(validate.pointer("/command"), Some(&json!("validate")));
    assert_eq!(validate.pointer("/status"), Some(&json!("passed")));
    assert_eq!(validate.pointer("/digest/matches"), Some(&json!(true)));

    let digest = run_zjx_success(&[
        "verify-digest",
        "--json",
        path_str(&hello_envelope).as_str(),
    ]);
    assert_eq!(digest.pointer("/digest/matches"), Some(&json!(true)));

    let graph_path = root.join("graph.json");
    let extract = run_zjx_success(&[
        "extract-graph",
        "--json",
        "--output",
        path_str(&graph_path).as_str(),
        path_str(&hello_envelope).as_str(),
    ]);
    assert_eq!(extract.pointer("/command"), Some(&json!("extract-graph")));
    assert_eq!(
        extract.pointer("/graph/schema"),
        Some(&json!("sley.symbol_graph.v0"))
    );
    let graph: JsonValue =
        serde_json::from_str(&fs::read_to_string(&graph_path).expect("read graph output"))
            .expect("parse graph output");
    support::validate_report_schema(
        "sley.symbol_graph.v0",
        fs::read(&graph_path)
            .expect("read graph output bytes")
            .as_slice(),
    );
    assert_eq!(
        graph.pointer("/schema"),
        Some(&json!("sley.symbol_graph.v0"))
    );

    let diff = run_zjx_success(&[
        "diff-envelope",
        "--json",
        path_str(&hello_envelope).as_str(),
        path_str(&project_envelope).as_str(),
    ]);
    assert_eq!(diff.pointer("/command"), Some(&json!("diff-envelope")));
    assert_eq!(diff.pointer("/diff/changed"), Some(&json!(true)));
    assert_eq!(
        diff.pointer("/diff/graph_digest_changed"),
        Some(&json!(true))
    );

    let tampered_envelope = root.join("tampered-envelope.json");
    let mut tampered: JsonValue =
        serde_json::from_str(&fs::read_to_string(&hello_envelope).expect("read hello envelope"))
            .expect("parse hello envelope");
    tampered["graph_digest"] =
        json!("sha256:0000000000000000000000000000000000000000000000000000000000000000");
    fs::write(
        &tampered_envelope,
        serde_json::to_string_pretty(&tampered).expect("render tampered envelope"),
    )
    .expect("write tampered envelope");
    let failed = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-zjx"))
        .args([
            "verify-digest",
            "--json",
            path_str(&tampered_envelope).as_str(),
        ])
        .output()
        .expect("run tampered digest check");
    assert!(
        !failed.status.success(),
        "tampered digest should fail: {}",
        String::from_utf8_lossy(&failed.stdout)
    );
    let failed_json: JsonValue =
        serde_json::from_slice(&failed.stdout).expect("parse failed digest report");
    support::validate_report_schema("sley.zjx.tool.report.v0", &failed.stdout);
    assert_eq!(failed_json.pointer("/status"), Some(&json!("failed")));
    assert_eq!(
        failed_json.pointer("/issues/0/code"),
        Some(&json!("ZJX_GRAPH_DIGEST_MISMATCH"))
    );

    let _ = fs::remove_dir_all(root);
}

fn write_zjx_envelope(target: &str, output_path: &Path) {
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley"))
        .args(["zjx", "--json", target])
        .output()
        .unwrap_or_else(|error| panic!("run sley zjx {target}: {error}"));
    assert!(
        output.status.success(),
        "sley zjx {target} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    support::validate_report_schema("sley.zjx.envelope.v0", &output.stdout);
    fs::write(output_path, output.stdout)
        .unwrap_or_else(|error| panic!("write {}: {error}", output_path.display()));
}

fn run_zjx_success(args: &[&str]) -> JsonValue {
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-zjx"))
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("run sley-zjx {}: {error}", args.join(" ")));
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        output.status.success(),
        "sley-zjx {} failed\nstdout: {stdout}\nstderr: {stderr}",
        args.join(" ")
    );
    support::validate_report_schema("sley.zjx.tool.report.v0", stdout.as_bytes());
    serde_json::from_str(&stdout)
        .unwrap_or_else(|error| panic!("parse sley-zjx JSON: {error}\n{stdout}"))
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
