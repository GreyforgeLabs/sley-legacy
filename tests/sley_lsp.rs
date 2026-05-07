use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{ChildStdout, Command as ProcessCommand, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value as JsonValue, json};

mod support;

#[test]
fn lsp_publishes_diagnostics_formats_symbols_and_previews_code_actions() {
    let mut child = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-lsp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn sley-lsp");
    let mut stdin = child.stdin.take().expect("lsp stdin");
    let stdout = child.stdout.take().expect("lsp stdout");
    let mut reader = BufReader::new(stdout);

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": null,
                "rootUri": null,
                "capabilities": {}
            }
        }),
    );
    let initialized = read_response(&mut reader, 1);
    assert_eq!(
        initialized.pointer("/result/serverInfo/name"),
        Some(&json!("sley-lsp"))
    );
    assert_eq!(
        initialized.pointer("/result/capabilities/documentFormattingProvider"),
        Some(&json!(true))
    );
    assert_eq!(
        initialized.pointer("/result/capabilities/executeCommandProvider/commands/0"),
        Some(&json!("sley.fix.preview"))
    );
    assert_eq!(
        initialized.pointer("/result/capabilities/definitionProvider"),
        Some(&json!(true))
    );

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }),
    );

    let uri = "file:///tmp/sley-lsp-main.sley";
    let source = r#"module app.lsp

export task fetch -> Result<Text, Error> uses Network {
  take url: Text
  return call http.try_get_text(url)
}

task main -> Text {
return "hello"
}

task orphan -> Text {
return "unused"
}
"#;
    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "sley",
                    "version": 1,
                    "text": source
                }
            }
        }),
    );
    let diagnostics = read_notification(&mut reader, "textDocument/publishDiagnostics");
    assert_eq!(diagnostics.pointer("/params/uri"), Some(&json!(uri)));
    let diagnostic_codes = diagnostics
        .pointer("/params/diagnostics")
        .and_then(JsonValue::as_array)
        .expect("diagnostic array")
        .iter()
        .filter_map(|diagnostic| diagnostic.get("code").and_then(JsonValue::as_str))
        .collect::<Vec<_>>();
    assert!(
        diagnostic_codes.contains(&"unused_private_task"),
        "expected unused_private_task diagnostic, got {diagnostic_codes:?}"
    );

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/formatting",
            "params": {
                "textDocument": {
                    "uri": uri
                },
                "options": {
                    "tabSize": 2,
                    "insertSpaces": true
                }
            }
        }),
    );
    let formatting = read_response(&mut reader, 2);
    let formatted = formatting
        .pointer("/result/0/newText")
        .and_then(JsonValue::as_str)
        .expect("format edit");
    assert!(formatted.contains("  return \"hello\""));

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/documentSymbol",
            "params": {
                "textDocument": {
                    "uri": uri
                }
            }
        }),
    );
    let symbols = read_response(&mut reader, 3);
    let symbol_names = symbols
        .get("result")
        .and_then(JsonValue::as_array)
        .expect("symbol array")
        .iter()
        .filter_map(|symbol| symbol.get("name").and_then(JsonValue::as_str))
        .collect::<Vec<_>>();
    assert!(symbol_names.contains(&"app.lsp"));
    assert!(symbol_names.contains(&"fetch"));
    assert!(symbol_names.contains(&"main"));
    assert!(symbol_names.contains(&"orphan"));

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {
                    "uri": uri
                },
                "position": {
                    "line": 2,
                    "character": 5
                }
            }
        }),
    );
    let hover = read_response(&mut reader, 4);
    let hover_text = hover
        .pointer("/result/contents/value")
        .and_then(JsonValue::as_str)
        .expect("task hover text");
    assert!(hover_text.contains("export task `fetch`"));
    assert!(hover_text.contains("module: `app.lsp`"));
    assert!(hover_text.contains("returns: `Result<Text, Error>`"));
    assert!(hover_text.contains("takes: `url: Text`"));
    assert!(hover_text.contains("effects: `Network`"));
    assert!(hover_text.contains("node: `task:app.lsp.fetch`"));

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": {
                    "uri": uri
                },
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 8, "character": 0 }
                },
                "context": {
                    "diagnostics": []
                }
            }
        }),
    );
    let actions = read_response(&mut reader, 5);
    let delete_action = actions
        .get("result")
        .and_then(JsonValue::as_array)
        .expect("code actions")
        .iter()
        .find(|action| action.pointer("/data/kind") == Some(&json!("delete_unused_private_task")))
        .cloned()
        .expect("unused private task code action");

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "workspace/executeCommand",
            "params": {
                "command": delete_action.pointer("/command/command").expect("command name"),
                "arguments": delete_action.pointer("/command/arguments").expect("command args")
            }
        }),
    );
    let preview = read_response(&mut reader, 6);
    let report_bytes = serde_json::to_vec(preview.get("result").expect("preview result"))
        .expect("serialize LSP fix preview report");
    support::validate_report_schema("sley.lsp.fix_preview.v0", &report_bytes);
    assert_eq!(
        preview.pointer("/result/schema"),
        Some(&json!("sley.lsp.fix_preview.v0"))
    );
    assert_eq!(
        preview.pointer("/result/preview/kind"),
        Some(&json!("delete_unused_private_task"))
    );
    assert_eq!(preview.pointer("/result/preview/uri"), Some(&json!(uri)));
    assert_eq!(
        preview.pointer("/result/preview/operation/op"),
        Some(&json!("DeleteNode"))
    );

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "shutdown",
            "params": null
        }),
    );
    let shutdown = read_response(&mut reader, 7);
    assert!(shutdown.get("result").is_some());
    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );
    let status = child.wait().expect("wait for sley-lsp");
    assert!(status.success());
}

#[test]
fn lsp_uses_project_context_for_imported_tasks() {
    let root = temp_lsp_project_dir("project-context");
    fs::create_dir_all(root.join("src/app")).expect("create LSP project dirs");
    fs::write(
        root.join("sley.toml"),
        r#"[project]
name = "lsp-project"
root = "src"
entry = "app.main"
"#,
    )
    .expect("write LSP project manifest");
    let main_path = root.join("src/app/main.sley");
    let main_uri = file_uri(&main_path);
    let pipeline_path = root.join("src/app/pipeline.sley");
    let pipeline_uri = file_uri(&pipeline_path);
    let main_source = r#"module app.main

import app.pipeline as pipe

task main -> Text {
  return call pipe.message()
}
    "#;
    fs::write(&main_path, main_source).expect("write LSP main module");
    fs::write(
        &pipeline_path,
        r#"module app.pipeline

export task message -> Text {
  return "ready"
}
"#,
    )
    .expect("write LSP pipeline module");

    let mut child = ProcessCommand::new(env!("CARGO_BIN_EXE_sley-lsp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn sley-lsp");
    let mut stdin = child.stdin.take().expect("lsp stdin");
    let stdout = child.stdout.take().expect("lsp stdout");
    let mut reader = BufReader::new(stdout);

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": null,
                "rootUri": file_uri(&root),
                "capabilities": {}
            }
        }),
    );
    let initialized = read_response(&mut reader, 1);
    assert_eq!(
        initialized.pointer("/result/serverInfo/name"),
        Some(&json!("sley-lsp"))
    );
    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "initialized",
            "params": {}
        }),
    );

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": main_uri,
                    "languageId": "sley",
                    "version": 1,
                    "text": main_source
                }
            }
        }),
    );
    let diagnostics = read_notification(&mut reader, "textDocument/publishDiagnostics");
    assert_eq!(diagnostics.pointer("/params/uri"), Some(&json!(main_uri)));
    assert!(
        !diagnostic_codes(&diagnostics).contains(&"UNKNOWN_TASK"),
        "valid imported task should resolve with project context: {diagnostics:#}"
    );

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/definition",
            "params": {
                "textDocument": {
                    "uri": main_uri
                },
                "position": {
                    "line": 5,
                    "character": 20
                }
            }
        }),
    );
    let definition = read_response(&mut reader, 2);
    assert_eq!(
        definition.pointer("/result/0/uri"),
        Some(&json!(pipeline_uri))
    );
    assert_eq!(
        definition.pointer("/result/0/range/start/line"),
        Some(&json!(2))
    );

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/definition",
            "params": {
                "textDocument": {
                    "uri": main_uri
                },
                "position": {
                    "line": 2,
                    "character": 8
                }
            }
        }),
    );
    let import_definition = read_response(&mut reader, 3);
    assert_eq!(
        import_definition.pointer("/result/0/uri"),
        Some(&json!(pipeline_uri))
    );
    assert_eq!(
        import_definition.pointer("/result/0/range/start/line"),
        Some(&json!(0))
    );

    let changed_source = main_source.replace("pipe.message()", "pipe.missing()");
    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": main_uri,
                    "version": 2
                },
                "contentChanges": [{
                    "text": changed_source
                }]
            }
        }),
    );
    let diagnostics = read_notification(&mut reader, "textDocument/publishDiagnostics");
    assert!(
        diagnostic_codes(&diagnostics).contains(&"UNKNOWN_TASK"),
        "unsaved missing imported task should be diagnosed: {diagnostics:#}"
    );

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "shutdown",
            "params": null
        }),
    );
    let shutdown = read_response(&mut reader, 4);
    assert!(shutdown.get("result").is_some());
    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "exit",
            "params": null
        }),
    );
    let status = child.wait().expect("wait for sley-lsp");
    assert!(status.success());
    let _ = fs::remove_dir_all(root);
}

fn diagnostic_codes(message: &JsonValue) -> Vec<&str> {
    message
        .pointer("/params/diagnostics")
        .and_then(JsonValue::as_array)
        .expect("diagnostic array")
        .iter()
        .filter_map(|diagnostic| diagnostic.get("code").and_then(JsonValue::as_str))
        .collect()
}

fn temp_lsp_project_dir(name: &str) -> PathBuf {
    let mut root = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before Unix epoch")
        .as_nanos();
    root.push(format!("sley-lsp-{name}-{}-{nanos}", std::process::id()));
    root
}

fn file_uri(path: &Path) -> String {
    format!("file://{}", path.display())
}

fn write_lsp(stdin: &mut impl Write, message: JsonValue) {
    let body = serde_json::to_vec(&message).expect("serialize LSP message");
    write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).expect("write LSP header");
    stdin.write_all(&body).expect("write LSP body");
    stdin.flush().expect("flush LSP message");
}

fn read_response(reader: &mut BufReader<ChildStdout>, id: i64) -> JsonValue {
    for _ in 0..20 {
        let message = read_lsp(reader);
        if message.get("id").and_then(JsonValue::as_i64) == Some(id) {
            return message;
        }
    }
    panic!("response id {id} not received");
}

fn read_notification(reader: &mut BufReader<ChildStdout>, method: &str) -> JsonValue {
    for _ in 0..20 {
        let message = read_lsp(reader);
        if message.get("method").and_then(JsonValue::as_str) == Some(method) {
            return message;
        }
    }
    panic!("notification {method} not received");
}

fn read_lsp(reader: &mut BufReader<ChildStdout>) -> JsonValue {
    let mut content_length = None;
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).expect("read LSP header");
        assert!(bytes > 0, "unexpected LSP EOF before headers");
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some((name, value)) = trimmed.split_once(':') {
            if name.eq_ignore_ascii_case("Content-Length") {
                content_length = Some(value.trim().parse::<usize>().expect("content length"));
            }
        }
    }
    let content_length = content_length.expect("Content-Length header");
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).expect("read LSP body");
    serde_json::from_slice(&body).expect("parse LSP JSON")
}
