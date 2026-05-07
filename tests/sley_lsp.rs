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
    assert_eq!(
        initialized.pointer("/result/capabilities/completionProvider/resolveProvider"),
        Some(&json!(false))
    );
    assert_eq!(
        initialized.pointer("/result/capabilities/signatureHelpProvider/triggerCharacters/0"),
        Some(&json!("("))
    );
    assert_eq!(
        initialized.pointer("/result/capabilities/workspaceSymbolProvider"),
        Some(&json!(true))
    );
    assert_eq!(
        initialized.pointer("/result/capabilities/referencesProvider"),
        Some(&json!(true))
    );
    assert_eq!(
        initialized.pointer("/result/capabilities/documentHighlightProvider"),
        Some(&json!(true))
    );
    assert_eq!(
        initialized.pointer("/result/capabilities/renameProvider/prepareProvider"),
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
  bind direct = pipe.message()
  return call pipe.message()
}

task pair -> Text {
  return pipe.message() + pipe.message()
}

task mixed -> Text {
  return pipe.message() + pipe.other()
}

task signature -> Text {
  return pipe.join("a", "b")
}
    "#;
    fs::write(&main_path, main_source).expect("write LSP main module");
    fs::write(
        &pipeline_path,
        r#"module app.pipeline

export task message -> Text {
  return "ready"
}

export task join -> Text {
  take left: Text
  take right: Text

  return left + right
}

export task other -> Text {
  return "other"
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
            "method": "textDocument/completion",
            "params": {
                "textDocument": {
                    "uri": main_uri
                },
                "position": {
                    "line": 6,
                    "character": 14
                }
            }
        }),
    );
    let completion = read_response(&mut reader, 2);
    let labels = completion_labels(&completion);
    assert!(
        labels.contains(&"pipe.message"),
        "project completions should include imported task label, got {labels:?}"
    );
    assert!(
        labels.contains(&"http.try_get_text"),
        "project completions should include host call labels, got {labels:?}"
    );
    assert!(
        labels.contains(&"task"),
        "project completions should include Sley keywords, got {labels:?}"
    );
    assert!(
        labels.contains(&"app.pipeline"),
        "project completions should include project modules, got {labels:?}"
    );

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "workspace/symbol",
            "params": {
                "query": "message"
            }
        }),
    );
    let workspace_symbols = read_response(&mut reader, 3);
    let symbols = workspace_symbol_names_and_uris(&workspace_symbols);
    assert!(
        symbols.contains(&("app.pipeline.message", pipeline_uri.as_str())),
        "workspace symbols should include imported project task, got {symbols:?}"
    );

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/references",
            "params": {
                "textDocument": {
                    "uri": main_uri
                },
                "position": {
                    "line": 6,
                    "character": 20
                },
                "context": {
                    "includeDeclaration": true
                }
            }
        }),
    );
    let references = read_response(&mut reader, 4);
    let reference_locations = location_uris_and_lines(&references);
    assert!(
        reference_locations.contains(&(pipeline_uri.as_str(), 2)),
        "references should include imported task declaration, got {reference_locations:?}"
    );
    assert!(
        reference_locations.contains(&(main_uri.as_str(), 6)),
        "references should include project call site, got {reference_locations:?}"
    );
    let reference_ranges = location_uri_lines_and_chars(&references);
    assert!(
        reference_ranges.contains(&(pipeline_uri.as_str(), 2, 12)),
        "references should use exact imported task declaration range, got {reference_ranges:?}"
    );
    assert!(
        reference_ranges.contains(&(main_uri.as_str(), 6, 19)),
        "references should use exact call-keyword leaf range, got {reference_ranges:?}"
    );

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "textDocument/documentHighlight",
            "params": {
                "textDocument": {
                    "uri": main_uri
                },
                "position": {
                    "line": 6,
                    "character": 20
                }
            }
        }),
    );
    let highlights = read_response(&mut reader, 5);
    let highlight_ranges = document_highlight_lines_and_chars(&highlights);
    assert!(
        highlight_ranges.contains(&(5, 21)),
        "document highlights should include bare project call site, got {highlight_ranges:?}"
    );
    assert!(
        highlight_ranges.contains(&(6, 19)),
        "document highlights should include call-keyword project call site, got {highlight_ranges:?}"
    );
    assert!(
        highlight_ranges.contains(&(10, 14)),
        "document highlights should include first same-line project call site, got {highlight_ranges:?}"
    );
    assert!(
        highlight_ranges.contains(&(10, 31)),
        "document highlights should include second same-line project call site, got {highlight_ranges:?}"
    );
    assert!(
        highlight_ranges.contains(&(14, 14)),
        "document highlights should include mixed-line matching call site, got {highlight_ranges:?}"
    );
    assert!(
        !highlight_ranges.contains(&(14, 31)),
        "document highlights should not include a different task on the same line, got {highlight_ranges:?}"
    );

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "textDocument/prepareRename",
            "params": {
                "textDocument": {
                    "uri": main_uri
                },
                "position": {
                    "line": 6,
                    "character": 20
                }
            }
        }),
    );
    let prepare_rename = read_response(&mut reader, 6);
    assert_eq!(
        prepare_rename.pointer("/result/placeholder"),
        Some(&json!("message"))
    );
    assert_eq!(
        prepare_rename.pointer("/result/range/start/line"),
        Some(&json!(6))
    );
    assert_eq!(
        prepare_rename.pointer("/result/range/start/character"),
        Some(&json!(19))
    );
    assert_eq!(
        prepare_rename.pointer("/result/range/end/character"),
        Some(&json!(26))
    );

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "textDocument/rename",
            "params": {
                "textDocument": {
                    "uri": main_uri
                },
                "position": {
                    "line": 6,
                    "character": 20
                },
                "newName": "compose"
            }
        }),
    );
    let rename = read_response(&mut reader, 7);
    let edits = workspace_edit_uri_lines_and_text(&rename);
    assert!(
        edits.contains(&(pipeline_uri.as_str(), 2, 12, "compose")),
        "rename should edit imported task declaration, got {edits:?}"
    );
    assert!(
        edits.contains(&(main_uri.as_str(), 5, 21, "compose")),
        "rename should edit bare project call site, got {edits:?}"
    );
    assert!(
        edits.contains(&(main_uri.as_str(), 6, 19, "compose")),
        "rename should edit call-keyword project call site, got {edits:?}"
    );
    assert!(
        edits.contains(&(main_uri.as_str(), 10, 14, "compose")),
        "rename should edit first same-line project call site, got {edits:?}"
    );
    assert!(
        edits.contains(&(main_uri.as_str(), 10, 31, "compose")),
        "rename should edit second same-line project call site, got {edits:?}"
    );
    assert!(
        edits.contains(&(main_uri.as_str(), 14, 14, "compose")),
        "rename should edit mixed-line project call site, got {edits:?}"
    );
    assert!(
        !edits.contains(&(main_uri.as_str(), 14, 31, "compose")),
        "rename should not edit a different task on the same line, got {edits:?}"
    );

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "textDocument/prepareRename",
            "params": {
                "textDocument": {
                    "uri": main_uri
                },
                "position": {
                    "line": 14,
                    "character": 33
                }
            }
        }),
    );
    let prepare_other_rename = read_response(&mut reader, 8);
    assert_eq!(
        prepare_other_rename.pointer("/result/placeholder"),
        Some(&json!("other"))
    );
    assert_eq!(
        prepare_other_rename.pointer("/result/range/start/line"),
        Some(&json!(14))
    );
    assert_eq!(
        prepare_other_rename.pointer("/result/range/start/character"),
        Some(&json!(31))
    );

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "textDocument/definition",
            "params": {
                "textDocument": {
                    "uri": main_uri
                },
                "position": {
                    "line": 6,
                    "character": 20
                }
            }
        }),
    );
    let definition = read_response(&mut reader, 9);
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
            "id": 10,
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
    let import_definition = read_response(&mut reader, 10);
    assert_eq!(
        import_definition.pointer("/result/0/uri"),
        Some(&json!(pipeline_uri))
    );
    assert_eq!(
        import_definition.pointer("/result/0/range/start/line"),
        Some(&json!(0))
    );

    let signature_line = main_source
        .lines()
        .position(|line| line.contains("pipe.join"))
        .expect("signature call line");
    let signature_character = main_source
        .lines()
        .nth(signature_line)
        .and_then(|line| line.find("\"b\""))
        .map(|character| character + 1)
        .expect("signature second argument character");
    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "textDocument/signatureHelp",
            "params": {
                "textDocument": {
                    "uri": main_uri
                },
                "position": {
                    "line": signature_line,
                    "character": signature_character
                }
            }
        }),
    );
    let signature_help = read_response(&mut reader, 11);
    assert_eq!(
        signature_help.pointer("/result/signatures/0/label"),
        Some(&json!("pipe.join(left: Text, right: Text) -> Text"))
    );
    assert_eq!(
        signature_help.pointer("/result/signatures/0/parameters/0/label"),
        Some(&json!("left: Text"))
    );
    assert_eq!(
        signature_help.pointer("/result/signatures/0/parameters/1/label"),
        Some(&json!("right: Text"))
    );
    assert_eq!(
        signature_help.pointer("/result/activeParameter"),
        Some(&json!(1))
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
            "id": 12,
            "method": "shutdown",
            "params": null
        }),
    );
    let shutdown = read_response(&mut reader, 12);
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

fn completion_labels(message: &JsonValue) -> Vec<&str> {
    message
        .pointer("/result/items")
        .and_then(JsonValue::as_array)
        .expect("completion item array")
        .iter()
        .filter_map(|item| item.get("label").and_then(JsonValue::as_str))
        .collect()
}

fn workspace_symbol_names_and_uris(message: &JsonValue) -> Vec<(&str, &str)> {
    message
        .get("result")
        .and_then(JsonValue::as_array)
        .expect("workspace symbol array")
        .iter()
        .filter_map(|symbol| {
            Some((
                symbol.get("name")?.as_str()?,
                symbol.pointer("/location/uri")?.as_str()?,
            ))
        })
        .collect()
}

fn location_uris_and_lines(message: &JsonValue) -> Vec<(&str, u64)> {
    message
        .get("result")
        .and_then(JsonValue::as_array)
        .expect("location array")
        .iter()
        .filter_map(|location| {
            Some((
                location.get("uri")?.as_str()?,
                location.pointer("/range/start/line")?.as_u64()?,
            ))
        })
        .collect()
}

fn location_uri_lines_and_chars(message: &JsonValue) -> Vec<(&str, u64, u64)> {
    message
        .get("result")
        .and_then(JsonValue::as_array)
        .expect("location array")
        .iter()
        .filter_map(|location| {
            Some((
                location.get("uri")?.as_str()?,
                location.pointer("/range/start/line")?.as_u64()?,
                location.pointer("/range/start/character")?.as_u64()?,
            ))
        })
        .collect()
}

fn document_highlight_lines_and_chars(message: &JsonValue) -> Vec<(u64, u64)> {
    message
        .get("result")
        .and_then(JsonValue::as_array)
        .expect("document highlight array")
        .iter()
        .filter_map(|highlight| {
            Some((
                highlight.pointer("/range/start/line")?.as_u64()?,
                highlight.pointer("/range/start/character")?.as_u64()?,
            ))
        })
        .collect()
}

fn workspace_edit_uri_lines_and_text(message: &JsonValue) -> Vec<(&str, u64, u64, &str)> {
    let changes = message
        .pointer("/result/changes")
        .and_then(JsonValue::as_object)
        .expect("workspace edit changes");
    let mut edits = Vec::new();
    for (uri, uri_edits) in changes {
        let Some(uri_edits) = uri_edits.as_array() else {
            continue;
        };
        for edit in uri_edits {
            if let (Some(line), Some(character), Some(new_text)) = (
                edit.pointer("/range/start/line")
                    .and_then(JsonValue::as_u64),
                edit.pointer("/range/start/character")
                    .and_then(JsonValue::as_u64),
                edit.get("newText").and_then(JsonValue::as_str),
            ) {
                edits.push((uri.as_str(), line, character, new_text));
            }
        }
    }
    edits
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
