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
        initialized.pointer("/result/capabilities/foldingRangeProvider"),
        Some(&json!(true))
    );
    assert_eq!(
        initialized.pointer("/result/capabilities/selectionRangeProvider"),
        Some(&json!(true))
    );
    assert_eq!(
        initialized.pointer("/result/capabilities/executeCommandProvider/commands/0"),
        Some(&json!("sley.fix.preview"))
    );
    assert_eq!(
        initialized.pointer("/result/capabilities/executeCommandProvider/commands/1"),
        Some(&json!("sley.command.preview"))
    );
    assert_eq!(
        initialized.pointer("/result/capabilities/codeLensProvider/resolveProvider"),
        Some(&json!(false))
    );
    assert_eq!(
        initialized.pointer("/result/capabilities/definitionProvider"),
        Some(&json!(true))
    );
    assert_eq!(
        initialized.pointer("/result/capabilities/documentLinkProvider/resolveProvider"),
        Some(&json!(false))
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
        initialized.pointer("/result/capabilities/inlayHintProvider/resolveProvider"),
        Some(&json!(false))
    );
    assert_eq!(
        initialized.pointer("/result/capabilities/semanticTokensProvider/full"),
        Some(&json!(true))
    );
    assert_eq!(
        initialized.pointer("/result/capabilities/semanticTokensProvider/legend/tokenTypes/2"),
        Some(&json!("function"))
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
    assert_eq!(
        initialized.pointer("/result/capabilities/workspace/workspaceFolders/supported"),
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
if true {
return "hello"
}
return "fallback"
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
            "id": 20,
            "method": "textDocument/foldingRange",
            "params": {
                "textDocument": {
                    "uri": uri
                }
            }
        }),
    );
    let folding_ranges = read_response(&mut reader, 20);
    let fold_lines = folding_range_lines_and_kinds(&folding_ranges);
    assert!(
        fold_lines.contains(&(2, 5, "region")),
        "folding ranges should include fetch task body, got {fold_lines:?}"
    );
    assert!(
        fold_lines.contains(&(7, 12, "region")),
        "folding ranges should include main task body, got {fold_lines:?}"
    );
    assert!(
        fold_lines.contains(&(8, 10, "region")),
        "folding ranges should include nested if block, got {fold_lines:?}"
    );
    assert!(
        fold_lines.contains(&(14, 16, "region")),
        "folding ranges should include orphan task body, got {fold_lines:?}"
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

    let host_call_line = source
        .lines()
        .position(|line| line.contains("http.try_get_text"))
        .expect("host call line");
    let host_call_character = source
        .lines()
        .nth(host_call_line)
        .and_then(|line| line.find("try_get_text"))
        .map(|character| character + 1)
        .expect("host call character");
    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 21,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {
                    "uri": uri
                },
                "position": {
                    "line": host_call_line,
                    "character": host_call_character
                }
            }
        }),
    );
    let host_hover = read_response(&mut reader, 21);
    let host_hover_text = host_hover
        .pointer("/result/contents/value")
        .and_then(JsonValue::as_str)
        .expect("host call hover text");
    assert!(host_hover_text.contains("host call `http.try_get_text`"));
    assert!(host_hover_text.contains("required capabilities: `Network`"));
    assert!(host_hover_text.contains("`--cap Network[=SCOPE]`"));
    assert!(host_hover_text.contains("recoverable host failures return `Result` values"));

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "textDocument/codeLens",
            "params": {
                "textDocument": {
                    "uri": uri
                }
            }
        }),
    );
    let code_lenses = read_response(&mut reader, 5);
    let doctor_lens = code_lenses
        .get("result")
        .and_then(JsonValue::as_array)
        .expect("code lens array")
        .iter()
        .find(|lens| lens.pointer("/command/title") == Some(&json!("Sley: doctor")))
        .cloned()
        .expect("doctor code lens");
    assert_eq!(
        doctor_lens.pointer("/command/command"),
        Some(&json!("sley.command.preview"))
    );
    assert_eq!(
        doctor_lens.pointer("/command/arguments/0/args/0"),
        Some(&json!("doctor"))
    );
    assert_eq!(
        doctor_lens.pointer("/command/arguments/0/args/1"),
        Some(&json!("--json"))
    );
    let graph_lens_count = code_lenses
        .get("result")
        .and_then(JsonValue::as_array)
        .expect("code lens array")
        .iter()
        .filter(|lens| lens.pointer("/command/title") == Some(&json!("Sley: graph slice")))
        .count();
    assert_eq!(graph_lens_count, 3);

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "workspace/executeCommand",
            "params": {
                "command": doctor_lens.pointer("/command/command").expect("command name"),
                "arguments": doctor_lens.pointer("/command/arguments").expect("command args")
            }
        }),
    );
    let command_preview = read_response(&mut reader, 6);
    let command_report_bytes = serde_json::to_vec(
        command_preview
            .get("result")
            .expect("command preview result"),
    )
    .expect("serialize LSP command preview report");
    support::validate_report_schema("sley.lsp.command_preview.v0", &command_report_bytes);
    assert_eq!(
        command_preview.pointer("/result/schema"),
        Some(&json!("sley.lsp.command_preview.v0"))
    );
    assert_eq!(
        command_preview.pointer("/result/preview/kind"),
        Some(&json!("doctor"))
    );
    assert_eq!(
        command_preview.pointer("/result/preview/uri"),
        Some(&json!(uri))
    );

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": {
                    "uri": uri
                },
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 17, "character": 0 }
                },
                "context": {
                    "diagnostics": []
                }
            }
        }),
    );
    let actions = read_response(&mut reader, 7);
    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 22,
            "method": "textDocument/codeAction",
            "params": {
                "textDocument": {
                    "uri": uri
                },
                "range": {
                    "start": { "line": 8, "character": 4 },
                    "end": { "line": 8, "character": 4 }
                },
                "context": {
                    "diagnostics": []
                }
            }
        }),
    );
    let scoped_actions = read_response(&mut reader, 22);
    let scoped_action_kinds = scoped_actions
        .get("result")
        .and_then(JsonValue::as_array)
        .expect("scoped code actions")
        .iter()
        .filter_map(|action| action.pointer("/data/kind").and_then(JsonValue::as_str))
        .collect::<Vec<_>>();
    assert!(
        scoped_action_kinds.contains(&"simplify_constant_if_statement"),
        "scoped code action should include the if-statement repair, got {scoped_action_kinds:?}"
    );
    assert!(
        !scoped_action_kinds.contains(&"delete_unused_private_task"),
        "scoped code action should exclude unrelated private-task repair, got {scoped_action_kinds:?}"
    );
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
            "id": 8,
            "method": "workspace/executeCommand",
            "params": {
                "command": delete_action.pointer("/command/command").expect("command name"),
                "arguments": delete_action.pointer("/command/arguments").expect("command args")
            }
        }),
    );
    let preview = read_response(&mut reader, 8);
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
    assert_eq!(
        preview.pointer("/result/preview/dryRun/args/5"),
        Some(&json!("--template-surface"))
    );
    assert_eq!(
        preview.pointer("/result/preview/dryRun/args/6"),
        preview.pointer("/result/preview/surface")
    );

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "shutdown",
            "params": null
        }),
    );
    let shutdown = read_response(&mut reader, 9);
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
    let pipeline_source = r#"module app.pipeline

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
"#;
    fs::write(&pipeline_path, pipeline_source).expect("write LSP pipeline module");

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
    let diagnostics = read_diagnostics_for_uri(&mut reader, &main_uri);
    assert_eq!(diagnostics.pointer("/params/uri"), Some(&json!(main_uri)));
    assert!(
        !diagnostic_codes(&diagnostics).contains(&"UNKNOWN_TASK"),
        "valid imported task should resolve with project context: {diagnostics:#}"
    );

    let disk_changed_pipeline_source =
        pipeline_source.replace("export task message", "export task absent");
    fs::write(&pipeline_path, &disk_changed_pipeline_source)
        .expect("write changed LSP pipeline module");
    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "workspace/didChangeWatchedFiles",
            "params": {
                "changes": [{
                    "uri": pipeline_uri,
                    "type": 2
                }]
            }
        }),
    );
    let watched_diagnostics = read_diagnostics_for_uri(&mut reader, &main_uri);
    assert!(
        diagnostic_codes(&watched_diagnostics).contains(&"UNKNOWN_TASK"),
        "watched project-file changes should refresh dependent diagnostics: {watched_diagnostics:#}"
    );

    fs::write(&pipeline_path, pipeline_source).expect("restore LSP pipeline module");
    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "workspace/didChangeWatchedFiles",
            "params": {
                "changes": [{
                    "uri": pipeline_uri,
                    "type": 2
                }]
            }
        }),
    );
    let restored_diagnostics = read_diagnostics_for_uri(&mut reader, &main_uri);
    assert!(
        !diagnostic_codes(&restored_diagnostics).contains(&"UNKNOWN_TASK"),
        "restored watched project-file changes should clear dependent diagnostics: {restored_diagnostics:#}"
    );

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": pipeline_uri,
                    "languageId": "sley",
                    "version": 1,
                    "text": pipeline_source
                }
            }
        }),
    );
    let _ = read_diagnostics_for_uri(&mut reader, &main_uri);
    let pipeline_diagnostics = read_diagnostics_for_uri(&mut reader, &pipeline_uri);
    assert!(
        !diagnostic_codes(&pipeline_diagnostics).contains(&"UNKNOWN_TASK"),
        "valid imported module should diagnose cleanly when opened: {pipeline_diagnostics:#}"
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

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "textDocument/documentLink",
            "params": {
                "textDocument": {
                    "uri": main_uri
                }
            }
        }),
    );
    let document_links = read_response(&mut reader, 11);
    assert_eq!(
        document_links.pointer("/result/0/target"),
        Some(&json!(pipeline_uri))
    );
    assert_eq!(
        document_links.pointer("/result/0/range/start/line"),
        Some(&json!(2))
    );
    assert_eq!(
        document_links.pointer("/result/0/range/start/character"),
        Some(&json!(7))
    );
    assert_eq!(
        document_links.pointer("/result/0/tooltip"),
        Some(&json!("Open module app.pipeline"))
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
    let hover_character = main_source
        .lines()
        .nth(signature_line)
        .and_then(|line| line.find("join"))
        .map(|character| character + 1)
        .expect("signature call hover character");
    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 12,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {
                    "uri": main_uri
                },
                "position": {
                    "line": signature_line,
                    "character": hover_character
                }
            }
        }),
    );
    let call_hover = read_response(&mut reader, 12);
    let call_hover_text = call_hover
        .pointer("/result/contents/value")
        .and_then(JsonValue::as_str)
        .expect("task call hover text");
    assert!(call_hover_text.contains("call `pipe.join`"));
    assert!(call_hover_text.contains("resolved task: `app.pipeline.join`"));
    assert!(call_hover_text.contains("signature: `pipe.join(left: Text, right: Text) -> Text`"));
    assert!(call_hover_text.contains("effects: none"));
    assert!(call_hover_text.contains("node: `task:app.pipeline.join`"));
    assert_eq!(
        call_hover.pointer("/result/range/start/line"),
        Some(&json!(signature_line))
    );
    assert_eq!(
        call_hover.pointer("/result/range/start/character"),
        Some(&json!(14))
    );
    assert_eq!(
        call_hover.pointer("/result/range/end/character"),
        Some(&json!(18))
    );

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 13,
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
    let signature_help = read_response(&mut reader, 13);
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

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 14,
            "method": "textDocument/inlayHint",
            "params": {
                "textDocument": {
                    "uri": main_uri
                },
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 100, "character": 0 }
                }
            }
        }),
    );
    let inlay_hints = read_response(&mut reader, 14);
    let inlay_hint_rows = inlay_hint_labels_lines_and_chars(&inlay_hints);
    assert!(
        inlay_hint_rows.contains(&("left:", signature_line as u64, 19)),
        "inlay hints should include first imported task parameter, got {inlay_hint_rows:?}"
    );
    assert!(
        inlay_hint_rows.contains(&("right:", signature_line as u64, 24)),
        "inlay hints should include second imported task parameter, got {inlay_hint_rows:?}"
    );
    assert_eq!(inlay_hints.pointer("/result/0/kind"), Some(&json!(2)));

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 15,
            "method": "textDocument/semanticTokens/full",
            "params": {
                "textDocument": {
                    "uri": main_uri
                }
            }
        }),
    );
    let semantic_tokens = read_response(&mut reader, 15);
    let semantic_rows = semantic_token_rows(&semantic_tokens);
    let signature_decl_line = main_source
        .lines()
        .position(|line| line.starts_with("task signature"))
        .expect("signature declaration line");
    assert!(
        semantic_rows.contains(&(0, 0, 6, 5, 0)),
        "semantic tokens should mark `module` as a keyword, got {semantic_rows:?}"
    );
    assert!(
        semantic_rows.contains(&(0, 7, 3, 0, 1)),
        "semantic tokens should mark module path segments as declaration namespaces, got {semantic_rows:?}"
    );
    assert!(
        semantic_rows.contains(&(signature_decl_line as u64, 5, 9, 2, 1)),
        "semantic tokens should mark task declarations as functions, got {semantic_rows:?}"
    );
    assert!(
        semantic_rows.contains(&(signature_line as u64, 14, 4, 2, 0)),
        "semantic tokens should mark project call leaves as functions, got {semantic_rows:?}"
    );
    assert!(
        semantic_rows.contains(&(signature_line as u64, 19, 3, 6, 0)),
        "semantic tokens should mark string literals, got {semantic_rows:?}"
    );

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 16,
            "method": "textDocument/selectionRange",
            "params": {
                "textDocument": {
                    "uri": main_uri
                },
                "positions": [{
                    "line": 6,
                    "character": 20
                }]
            }
        }),
    );
    let selection_range = read_response(&mut reader, 16);
    assert_eq!(
        selection_range.pointer("/result/0/range/start/line"),
        Some(&json!(6))
    );
    assert_eq!(
        selection_range.pointer("/result/0/range/start/character"),
        Some(&json!(19))
    );
    assert_eq!(
        selection_range.pointer("/result/0/parent/range/start/character"),
        Some(&json!(9))
    );
    assert_eq!(
        selection_range.pointer("/result/0/parent/parent/range/start/line"),
        Some(&json!(4))
    );

    let changed_pipeline_source =
        pipeline_source.replace("export task message", "export task absent");
    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": pipeline_uri,
                    "version": 2
                },
                "contentChanges": [{
                    "text": changed_pipeline_source
                }]
            }
        }),
    );
    let dependent_diagnostics = read_diagnostics_for_uri(&mut reader, &main_uri);
    assert!(
        diagnostic_codes(&dependent_diagnostics).contains(&"UNKNOWN_TASK"),
        "unsaved imported-module changes should refresh dependent main diagnostics: {dependent_diagnostics:#}"
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
    let diagnostics = read_diagnostics_for_uri(&mut reader, &main_uri);
    assert!(
        diagnostic_codes(&diagnostics).contains(&"UNKNOWN_TASK"),
        "unsaved missing imported task should be diagnosed: {diagnostics:#}"
    );

    write_lsp(
        &mut stdin,
        json!({
            "jsonrpc": "2.0",
            "id": 17,
            "method": "shutdown",
            "params": null
        }),
    );
    let shutdown = read_response(&mut reader, 17);
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

fn folding_range_lines_and_kinds(message: &JsonValue) -> Vec<(u64, u64, &str)> {
    message
        .get("result")
        .and_then(JsonValue::as_array)
        .expect("folding range array")
        .iter()
        .filter_map(|range| {
            Some((
                range.get("startLine")?.as_u64()?,
                range.get("endLine")?.as_u64()?,
                range.get("kind")?.as_str()?,
            ))
        })
        .collect()
}

fn inlay_hint_labels_lines_and_chars(message: &JsonValue) -> Vec<(&str, u64, u64)> {
    message
        .get("result")
        .and_then(JsonValue::as_array)
        .expect("inlay hint array")
        .iter()
        .filter_map(|hint| {
            Some((
                hint.get("label")?.as_str()?,
                hint.pointer("/position/line")?.as_u64()?,
                hint.pointer("/position/character")?.as_u64()?,
            ))
        })
        .collect()
}

fn semantic_token_rows(message: &JsonValue) -> Vec<(u64, u64, u64, u64, u64)> {
    let data = message
        .pointer("/result/data")
        .and_then(JsonValue::as_array)
        .expect("semantic token data");
    let mut rows = Vec::new();
    let mut line = 0u64;
    let mut character = 0u64;
    for chunk in data.chunks(5) {
        if chunk.len() != 5 {
            continue;
        }
        let delta_line = chunk[0].as_u64().expect("delta line");
        let delta_character = chunk[1].as_u64().expect("delta character");
        let length = chunk[2].as_u64().expect("length");
        let token_type = chunk[3].as_u64().expect("token type");
        let modifiers = chunk[4].as_u64().expect("modifiers");
        line += delta_line;
        if delta_line == 0 {
            character += delta_character;
        } else {
            character = delta_character;
        }
        rows.push((line, character, length, token_type, modifiers));
    }
    rows
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

fn read_diagnostics_for_uri(reader: &mut BufReader<ChildStdout>, uri: &str) -> JsonValue {
    for _ in 0..40 {
        let message = read_lsp(reader);
        if message.get("method").and_then(JsonValue::as_str)
            == Some("textDocument/publishDiagnostics")
            && message.pointer("/params/uri").and_then(JsonValue::as_str) == Some(uri)
        {
            return message;
        }
    }
    panic!("diagnostics for {uri} not received");
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
