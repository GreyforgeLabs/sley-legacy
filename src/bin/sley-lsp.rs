use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use serde_json::{Value as JsonValue, json};
use sley::ast::{
    BindingKind, EffectDecl, Expr, ExprKind, ImportDecl, Program, Statement, StatementKind,
    TaskDecl, TypeDecl,
};
use sley::authority::{host_effect_contracts, host_effects_for_callee};
use sley::checker::{check_program, has_errors};
use sley::diagnostics::{Diagnostic, Severity, SourceSpan};
use sley::formatter::format_program;
use sley::lint::{LintFinding, LintOptions, build_lint_report};
use sley::parser::parse_program;
use sley::plan::{
    EditPlanGraftTemplate, EditPlanOptions, EditPlanTransactionTemplate,
    build_edit_plan_report_with_options, module_name_from_sley_path,
};
use sley::project::{ProjectGraph, load_project_with_source_overlays};
use sley::symbols::{TaskResolution, callee_path, resolve_task};

const FIX_PREVIEW_COMMAND: &str = "sley.fix.preview";
const FIX_PREVIEW_SCHEMA: &str = "sley.lsp.fix_preview.v0";
const COMMAND_PREVIEW_COMMAND: &str = "sley.command.preview";
const COMMAND_PREVIEW_SCHEMA: &str = "sley.lsp.command_preview.v0";

#[derive(Debug, Parser)]
#[command(name = "sley-lsp")]
#[command(about = "Run the Sley stdio language server")]
struct Cli {}

#[derive(Default)]
struct ServerState {
    documents: HashMap<String, DocumentState>,
    shutdown_requested: bool,
}

struct DocumentState {
    text: String,
    version: Option<i64>,
}

struct TaskSymbolContext {
    uri: String,
    source: String,
    current_program: Program,
    project: Option<ProjectGraph>,
    target_task_id: String,
    old_name: String,
    range: JsonValue,
}

impl ServerState {
    fn source_overlays(&self) -> HashMap<PathBuf, String> {
        self.documents
            .iter()
            .filter_map(|(uri, document)| {
                uri_to_path(uri).map(|path| (path, document.text.clone()))
            })
            .collect()
    }
}

fn main() -> Result<()> {
    let _cli = Cli::parse();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());
    let mut state = ServerState::default();

    while let Some(message) = read_lsp_message(&mut reader)? {
        if !handle_message(message, &mut state, &mut writer)? {
            break;
        }
    }

    Ok(())
}

fn read_lsp_message<R: BufRead + Read>(reader: &mut R) -> Result<Option<JsonValue>> {
    let mut content_length = None;
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            return Ok(None);
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some((name, value)) = trimmed.split_once(':') {
            if name.eq_ignore_ascii_case("Content-Length") {
                content_length = Some(value.trim().parse::<usize>().with_context(|| {
                    format!("invalid LSP Content-Length header `{}`", value.trim())
                })?);
            }
        }
    }

    let content_length = content_length.context("missing LSP Content-Length header")?;
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body)?;
    Ok(Some(serde_json::from_slice(&body)?))
}

fn handle_message<W: Write>(
    message: JsonValue,
    state: &mut ServerState,
    writer: &mut W,
) -> Result<bool> {
    let id = message.get("id").cloned();
    let method = message
        .get("method")
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    let params = message.get("params").cloned().unwrap_or_else(|| json!({}));

    match method {
        "initialize" => {
            if let Some(id) = id {
                send_response(writer, id, initialize_result())?;
            }
        }
        "initialized" => {}
        "shutdown" => {
            state.shutdown_requested = true;
            if let Some(id) = id {
                send_response(writer, id, JsonValue::Null)?;
            }
        }
        "exit" => return Ok(false),
        "textDocument/didOpen" => handle_did_open(params, state, writer)?,
        "textDocument/didChange" => handle_did_change(params, state, writer)?,
        "textDocument/didClose" => handle_did_close(params, state, writer)?,
        "workspace/didChangeWatchedFiles" => {
            handle_did_change_watched_files(params, state, writer)?
        }
        "textDocument/formatting" => {
            if let Some(id) = id {
                send_response(writer, id, handle_formatting(params, state))?;
            }
        }
        "textDocument/documentSymbol" => {
            if let Some(id) = id {
                send_response(writer, id, handle_document_symbols(params, state))?;
            }
        }
        "textDocument/foldingRange" => {
            if let Some(id) = id {
                send_response(writer, id, handle_folding_ranges(params, state))?;
            }
        }
        "textDocument/selectionRange" => {
            if let Some(id) = id {
                send_response(writer, id, handle_selection_ranges(params, state))?;
            }
        }
        "textDocument/hover" => {
            if let Some(id) = id {
                send_response(writer, id, handle_hover(params, state))?;
            }
        }
        "textDocument/definition" => {
            if let Some(id) = id {
                send_response(writer, id, handle_definition(params, state))?;
            }
        }
        "textDocument/documentLink" => {
            if let Some(id) = id {
                send_response(writer, id, handle_document_links(params, state))?;
            }
        }
        "textDocument/completion" => {
            if let Some(id) = id {
                send_response(writer, id, handle_completion(params, state))?;
            }
        }
        "textDocument/signatureHelp" => {
            if let Some(id) = id {
                send_response(writer, id, handle_signature_help(params, state))?;
            }
        }
        "textDocument/inlayHint" => {
            if let Some(id) = id {
                send_response(writer, id, handle_inlay_hints(params, state))?;
            }
        }
        "textDocument/semanticTokens/full" => {
            if let Some(id) = id {
                send_response(writer, id, handle_semantic_tokens(params, state))?;
            }
        }
        "workspace/symbol" => {
            if let Some(id) = id {
                send_response(writer, id, handle_workspace_symbol(params, state))?;
            }
        }
        "textDocument/references" => {
            if let Some(id) = id {
                send_response(writer, id, handle_references(params, state))?;
            }
        }
        "textDocument/documentHighlight" => {
            if let Some(id) = id {
                send_response(writer, id, handle_document_highlight(params, state))?;
            }
        }
        "textDocument/prepareRename" => {
            if let Some(id) = id {
                send_response(writer, id, handle_prepare_rename(params, state))?;
            }
        }
        "textDocument/rename" => {
            if let Some(id) = id {
                send_response(writer, id, handle_rename(params, state))?;
            }
        }
        "textDocument/codeAction" => {
            if let Some(id) = id {
                send_response(writer, id, handle_code_actions(params, state))?;
            }
        }
        "textDocument/codeLens" => {
            if let Some(id) = id {
                send_response(writer, id, handle_code_lens(params, state))?;
            }
        }
        "workspace/executeCommand" => {
            if let Some(id) = id {
                send_response(writer, id, handle_execute_command(params))?;
            }
        }
        _ => {
            if let Some(id) = id {
                send_error(
                    writer,
                    id,
                    -32601,
                    format!("method `{method}` is not supported"),
                )?;
            }
        }
    }

    Ok(!state.shutdown_requested || method != "exit")
}

fn initialize_result() -> JsonValue {
    json!({
        "capabilities": {
            "textDocumentSync": {
                "openClose": true,
                "change": 1,
                "save": false
            },
            "documentFormattingProvider": true,
            "documentSymbolProvider": true,
            "foldingRangeProvider": true,
            "selectionRangeProvider": true,
            "hoverProvider": true,
            "definitionProvider": true,
            "documentLinkProvider": {
                "resolveProvider": false
            },
            "completionProvider": {
                "resolveProvider": false,
                "triggerCharacters": [".", " ", ":", ">"]
            },
            "signatureHelpProvider": {
                "triggerCharacters": ["(", ","]
            },
            "inlayHintProvider": {
                "resolveProvider": false
            },
            "semanticTokensProvider": {
                "legend": {
                    "tokenTypes": [
                        "namespace",
                        "type",
                        "function",
                        "parameter",
                        "variable",
                        "keyword",
                        "string",
                        "number",
                        "comment",
                        "operator"
                    ],
                    "tokenModifiers": [
                        "declaration",
                        "defaultLibrary"
                    ]
                },
                "full": true,
                "range": false
            },
            "workspaceSymbolProvider": true,
            "referencesProvider": true,
            "documentHighlightProvider": true,
            "renameProvider": {
                "prepareProvider": true
            },
            "codeActionProvider": {
                "resolveProvider": false,
                "codeActionKinds": ["quickfix", "refactor.rewrite"]
            },
            "codeLensProvider": {
                "resolveProvider": false
            },
            "executeCommandProvider": {
                "commands": [FIX_PREVIEW_COMMAND, COMMAND_PREVIEW_COMMAND]
            },
            "workspace": {
                "workspaceFolders": {
                    "supported": true,
                    "changeNotifications": true
                }
            }
        },
        "serverInfo": {
            "name": "sley-lsp",
            "version": env!("CARGO_PKG_VERSION")
        }
    })
}

fn handle_did_open<W: Write>(
    params: JsonValue,
    state: &mut ServerState,
    writer: &mut W,
) -> Result<()> {
    let Some(text_document) = params.get("textDocument") else {
        return Ok(());
    };
    let Some(uri) = text_document.get("uri").and_then(JsonValue::as_str) else {
        return Ok(());
    };
    let text = text_document
        .get("text")
        .and_then(JsonValue::as_str)
        .unwrap_or_default()
        .to_string();
    let version = text_document.get("version").and_then(JsonValue::as_i64);
    state
        .documents
        .insert(uri.to_string(), DocumentState { text, version });
    publish_all_open_document_diagnostics(writer, state)
}

fn handle_did_change<W: Write>(
    params: JsonValue,
    state: &mut ServerState,
    writer: &mut W,
) -> Result<()> {
    let Some(text_document) = params.get("textDocument") else {
        return Ok(());
    };
    let Some(uri) = text_document.get("uri").and_then(JsonValue::as_str) else {
        return Ok(());
    };
    let Some(text) = params
        .get("contentChanges")
        .and_then(JsonValue::as_array)
        .and_then(|changes| changes.last())
        .and_then(|change| change.get("text"))
        .and_then(JsonValue::as_str)
    else {
        return Ok(());
    };
    let version = text_document.get("version").and_then(JsonValue::as_i64);
    state.documents.insert(
        uri.to_string(),
        DocumentState {
            text: text.to_string(),
            version,
        },
    );
    publish_all_open_document_diagnostics(writer, state)
}

fn handle_did_close<W: Write>(
    params: JsonValue,
    state: &mut ServerState,
    writer: &mut W,
) -> Result<()> {
    let Some(uri) = text_document_uri(&params) else {
        return Ok(());
    };
    state.documents.remove(&uri);
    send_notification(
        writer,
        "textDocument/publishDiagnostics",
        json!({
            "uri": uri,
            "diagnostics": []
        }),
    )?;
    publish_all_open_document_diagnostics(writer, state)
}

fn handle_did_change_watched_files<W: Write>(
    _params: JsonValue,
    state: &ServerState,
    writer: &mut W,
) -> Result<()> {
    publish_all_open_document_diagnostics(writer, state)
}

fn handle_formatting(params: JsonValue, state: &ServerState) -> JsonValue {
    let Some(uri) = text_document_uri(&params) else {
        return json!([]);
    };
    let Some(document) = state.documents.get(&uri) else {
        return json!([]);
    };
    let Ok(program) = parse_program(&document.text) else {
        return json!([]);
    };
    json!([{
        "range": full_document_range(&document.text),
        "newText": format_program(&program)
    }])
}

fn handle_document_symbols(params: JsonValue, state: &ServerState) -> JsonValue {
    let Some(uri) = text_document_uri(&params) else {
        return json!([]);
    };
    let Some(document) = state.documents.get(&uri) else {
        return json!([]);
    };
    let Ok(program) = parse_program(&document.text) else {
        return json!([]);
    };
    json!(document_symbols(&program, &document.text))
}

fn handle_folding_ranges(params: JsonValue, state: &ServerState) -> JsonValue {
    let Some(uri) = text_document_uri(&params) else {
        return json!([]);
    };
    let Some(document) = state.documents.get(&uri) else {
        return json!([]);
    };
    let Ok(program) = parse_program(&document.text) else {
        return json!([]);
    };
    json!(folding_ranges_for_program(&document.text, &program))
}

fn handle_selection_ranges(params: JsonValue, state: &ServerState) -> JsonValue {
    let Some(uri) = text_document_uri(&params) else {
        return json!([]);
    };
    let Some(document) = state.documents.get(&uri) else {
        return json!([]);
    };
    let Ok(program) = parse_program(&document.text) else {
        return json!([]);
    };
    let positions = params
        .get("positions")
        .and_then(JsonValue::as_array)
        .cloned()
        .unwrap_or_default();
    let selection_ranges = positions
        .iter()
        .filter_map(position_line_and_character)
        .map(|(line, character)| {
            selection_range_at_position(&program, &document.text, line, character)
                .unwrap_or_else(|| selection_range_json(full_document_range(&document.text), None))
        })
        .collect::<Vec<_>>();
    json!(selection_ranges)
}

fn handle_hover(params: JsonValue, state: &ServerState) -> JsonValue {
    let Some(uri) = text_document_uri(&params) else {
        return JsonValue::Null;
    };
    let Some(document) = state.documents.get(&uri) else {
        return JsonValue::Null;
    };
    let Some(position) = params.get("position") else {
        return JsonValue::Null;
    };
    let Some((line, character)) = position_line_and_character(position) else {
        return JsonValue::Null;
    };
    let Ok(current_program) = parse_program(&document.text) else {
        return JsonValue::Null;
    };
    let project = project_graph_for_document(&uri, &current_program, state);
    let analysis_program = project
        .as_ref()
        .map(|project| &project.program)
        .unwrap_or(&current_program);
    if let Some((caller_module, callee_name, range)) =
        call_at_position(&current_program, &document.text, line, character)
    {
        match resolve_task(analysis_program, &caller_module, &callee_name) {
            TaskResolution::Resolved { index, .. } => {
                return hover_json(
                    &task_call_hover(
                        &analysis_program.tasks[index],
                        analysis_program.module_name(),
                        &callee_name,
                    ),
                    range,
                );
            }
            TaskResolution::Unknown | TaskResolution::Ambiguous(_) | TaskResolution::Private(_) => {
                if let Some(effects) = host_effects_for_callee(&callee_name) {
                    return hover_json(&host_call_hover(&callee_name, effects), range);
                }
            }
        }
    }
    hover_for_line(&current_program, &document.text, line).unwrap_or(JsonValue::Null)
}

fn handle_definition(params: JsonValue, state: &ServerState) -> JsonValue {
    let Some(uri) = text_document_uri(&params) else {
        return JsonValue::Null;
    };
    let Some(document) = state.documents.get(&uri) else {
        return JsonValue::Null;
    };
    let Some(position) = params.get("position") else {
        return JsonValue::Null;
    };
    let line = position
        .get("line")
        .and_then(JsonValue::as_u64)
        .unwrap_or_default() as usize;
    let Ok(current_program) = parse_program(&document.text) else {
        return JsonValue::Null;
    };
    let project = project_graph_for_document(&uri, &current_program, state);

    if let Some(project) = project.as_ref()
        && let Some(location) = import_definition_at_line(&current_program, line, project, state)
    {
        return json!([location]);
    }

    let analysis_program = project
        .as_ref()
        .map(|project| &project.program)
        .unwrap_or(&current_program);
    let Some((caller_module, callee_name)) = call_at_line(&current_program, line) else {
        return JsonValue::Null;
    };
    match resolve_task(analysis_program, &caller_module, &callee_name) {
        TaskResolution::Resolved { index, .. } => definition_location_for_task(
            &uri,
            &document.text,
            &current_program,
            project.as_ref(),
            &analysis_program.tasks[index],
            state,
        )
        .map(|location| json!([location]))
        .unwrap_or(JsonValue::Null),
        TaskResolution::Unknown | TaskResolution::Ambiguous(_) | TaskResolution::Private(_) => {
            JsonValue::Null
        }
    }
}

fn handle_document_links(params: JsonValue, state: &ServerState) -> JsonValue {
    let Some(uri) = text_document_uri(&params) else {
        return json!([]);
    };
    let Some(document) = state.documents.get(&uri) else {
        return json!([]);
    };
    let Ok(current_program) = parse_program(&document.text) else {
        return json!([]);
    };
    let Some(project) = project_graph_for_document(&uri, &current_program, state) else {
        return json!([]);
    };
    json!(document_links_for_imports(
        &document.text,
        &current_program,
        &project
    ))
}

fn handle_completion(params: JsonValue, state: &ServerState) -> JsonValue {
    let Some(uri) = text_document_uri(&params) else {
        return completion_list(core_completion_items());
    };
    let Some(document) = state.documents.get(&uri) else {
        return completion_list(core_completion_items());
    };
    let Ok(current_program) = parse_program(&document.text) else {
        return completion_list(core_completion_items());
    };
    let project = project_graph_for_document(&uri, &current_program, state);
    let analysis_program = project
        .as_ref()
        .map(|project| &project.program)
        .unwrap_or(&current_program);
    completion_list(completion_items_for_program(
        &current_program,
        analysis_program,
        project.as_ref(),
    ))
}

fn handle_signature_help(params: JsonValue, state: &ServerState) -> JsonValue {
    let Some(uri) = text_document_uri(&params) else {
        return JsonValue::Null;
    };
    let Some(document) = state.documents.get(&uri) else {
        return JsonValue::Null;
    };
    let Some(position) = params.get("position") else {
        return JsonValue::Null;
    };
    let Some((line, character)) = position_line_and_character(position) else {
        return JsonValue::Null;
    };
    let Ok(current_program) = parse_program(&document.text) else {
        return JsonValue::Null;
    };
    let project = project_graph_for_document(&uri, &current_program, state);
    let analysis_program = project
        .as_ref()
        .map(|project| &project.program)
        .unwrap_or(&current_program);
    let Some((caller_module, callee_name, active_parameter)) =
        call_signature_at_position(&current_program, &document.text, line, character)
    else {
        return JsonValue::Null;
    };
    match resolve_task(analysis_program, &caller_module, &callee_name) {
        TaskResolution::Resolved { index, .. } => task_signature_help(
            &analysis_program.tasks[index],
            analysis_program.module_name(),
            &callee_name,
            active_parameter,
        ),
        TaskResolution::Unknown | TaskResolution::Ambiguous(_) | TaskResolution::Private(_) => {
            JsonValue::Null
        }
    }
}

fn handle_inlay_hints(params: JsonValue, state: &ServerState) -> JsonValue {
    let Some(uri) = text_document_uri(&params) else {
        return json!([]);
    };
    let Some(document) = state.documents.get(&uri) else {
        return json!([]);
    };
    let Ok(current_program) = parse_program(&document.text) else {
        return json!([]);
    };
    let Some(range) = request_range(&params, &document.text) else {
        return json!([]);
    };
    let project = project_graph_for_document(&uri, &current_program, state);
    let analysis_program = project
        .as_ref()
        .map(|project| &project.program)
        .unwrap_or(&current_program);
    json!(inlay_hints_for_program(
        &current_program,
        analysis_program,
        &document.text,
        range,
    ))
}

fn handle_semantic_tokens(params: JsonValue, state: &ServerState) -> JsonValue {
    let Some(uri) = text_document_uri(&params) else {
        return json!({ "data": [] });
    };
    let Some(document) = state.documents.get(&uri) else {
        return json!({ "data": [] });
    };
    json!({
        "data": semantic_tokens_data(&document.text)
    })
}

fn handle_workspace_symbol(params: JsonValue, state: &ServerState) -> JsonValue {
    let query = params
        .get("query")
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    let mut symbols = Vec::new();
    let mut seen = BTreeSet::new();
    for (uri, document) in &state.documents {
        let Ok(program) = parse_program(&document.text) else {
            continue;
        };
        if let Some(project) = project_graph_for_document(uri, &program, state) {
            for symbol in workspace_symbols_for_project(&project, state, query) {
                if let Some(key) = workspace_symbol_key(&symbol)
                    && seen.insert(key)
                {
                    symbols.push(symbol);
                }
            }
        } else {
            for symbol in workspace_symbols_for_document(uri, &document.text, &program, query) {
                if let Some(key) = workspace_symbol_key(&symbol)
                    && seen.insert(key)
                {
                    symbols.push(symbol);
                }
            }
        }
    }
    symbols.sort_by_key(|symbol| {
        symbol
            .get("name")
            .and_then(JsonValue::as_str)
            .unwrap_or_default()
            .to_string()
    });
    json!(symbols)
}

fn handle_references(params: JsonValue, state: &ServerState) -> JsonValue {
    let include_declaration = params
        .pointer("/context/includeDeclaration")
        .and_then(JsonValue::as_bool)
        .unwrap_or(true);
    let Some(context) = task_symbol_context_for_request(&params, state) else {
        return json!([]);
    };
    let analysis_program = context
        .project
        .as_ref()
        .map(|project| &project.program)
        .unwrap_or(&context.current_program);
    json!(reference_locations_for_task(
        &context.target_task_id,
        analysis_program,
        context.project.as_ref(),
        &context.uri,
        &context.source,
        &context.current_program,
        include_declaration,
        state,
    ))
}

fn handle_document_highlight(params: JsonValue, state: &ServerState) -> JsonValue {
    let Some(context) = task_symbol_context_for_request(&params, state) else {
        return json!([]);
    };
    let analysis_program = context
        .project
        .as_ref()
        .map(|project| &project.program)
        .unwrap_or(&context.current_program);
    let changes = rename_edits_for_task(
        &context.target_task_id,
        &context.old_name,
        &context.old_name,
        analysis_program,
        context.project.as_ref(),
        &context.uri,
        &context.source,
        &context.current_program,
        state,
    );
    let highlights = changes
        .get(&context.uri)
        .into_iter()
        .flat_map(|edits| edits.iter())
        .filter_map(|edit| edit.get("range").cloned())
        .map(|range| {
            json!({
                "range": range,
                "kind": 1
            })
        })
        .collect::<Vec<_>>();
    json!(highlights)
}

fn handle_prepare_rename(params: JsonValue, state: &ServerState) -> JsonValue {
    let Some(context) = task_symbol_context_for_request(&params, state) else {
        return JsonValue::Null;
    };
    json!({
        "range": context.range,
        "placeholder": context.old_name
    })
}

fn handle_rename(params: JsonValue, state: &ServerState) -> JsonValue {
    let new_name = params
        .get("newName")
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    if !is_sley_identifier(new_name) {
        return JsonValue::Null;
    }
    let Some(context) = task_symbol_context_for_request(&params, state) else {
        return JsonValue::Null;
    };
    let analysis_program = context
        .project
        .as_ref()
        .map(|project| &project.program)
        .unwrap_or(&context.current_program);
    let changes = rename_edits_for_task(
        &context.target_task_id,
        &context.old_name,
        new_name,
        analysis_program,
        context.project.as_ref(),
        &context.uri,
        &context.source,
        &context.current_program,
        state,
    );
    if changes.is_empty() {
        JsonValue::Null
    } else {
        json!({ "changes": changes })
    }
}

fn task_symbol_context_for_request(
    params: &JsonValue,
    state: &ServerState,
) -> Option<TaskSymbolContext> {
    let uri = text_document_uri(params)?;
    let document = state.documents.get(&uri)?;
    let (line, character) = position_line_and_character(params.get("position")?)?;
    let current_program = parse_program(&document.text).ok()?;
    let project = project_graph_for_document(&uri, &current_program, state);
    let (target_task_id, old_name, range) = {
        let analysis_program = project
            .as_ref()
            .map(|project| &project.program)
            .unwrap_or(&current_program);
        task_rename_target_at_position(
            &current_program,
            analysis_program,
            &document.text,
            line,
            character,
        )?
    };
    Some(TaskSymbolContext {
        uri,
        source: document.text.clone(),
        current_program,
        project,
        target_task_id,
        old_name,
        range,
    })
}

fn position_line_and_character(position: &JsonValue) -> Option<(usize, usize)> {
    Some((
        position.get("line")?.as_u64()? as usize,
        position.get("character")?.as_u64()? as usize,
    ))
}

fn request_range(params: &JsonValue, source: &str) -> Option<(usize, usize, usize, usize)> {
    if let Some(range) = params.get("range") {
        let (start_line, start_character) = position_line_and_character(range.get("start")?)?;
        let (end_line, end_character) = position_line_and_character(range.get("end")?)?;
        return Some((start_line, start_character, end_line, end_character));
    }
    let full_range = full_document_range(source);
    Some((
        full_range.pointer("/start/line")?.as_u64()? as usize,
        full_range.pointer("/start/character")?.as_u64()? as usize,
        full_range.pointer("/end/line")?.as_u64()? as usize,
        full_range.pointer("/end/character")?.as_u64()? as usize,
    ))
}

fn position_in_request_range(
    range: (usize, usize, usize, usize),
    line: usize,
    character: usize,
) -> bool {
    let (start_line, start_character, end_line, end_character) = range;
    if line < start_line || line > end_line {
        return false;
    }
    if line == start_line && character < start_character {
        return false;
    }
    if line == end_line && character > end_character {
        return false;
    }
    true
}

fn request_range_intersects_lsp_range(
    request: (usize, usize, usize, usize),
    surface: &JsonValue,
) -> bool {
    let Some(surface_start_line) = surface
        .pointer("/start/line")
        .and_then(JsonValue::as_u64)
        .map(|value| value as usize)
    else {
        return true;
    };
    let Some(surface_start_character) = surface
        .pointer("/start/character")
        .and_then(JsonValue::as_u64)
        .map(|value| value as usize)
    else {
        return true;
    };
    let Some(surface_end_line) = surface
        .pointer("/end/line")
        .and_then(JsonValue::as_u64)
        .map(|value| value as usize)
    else {
        return true;
    };
    let Some(surface_end_character) = surface
        .pointer("/end/character")
        .and_then(JsonValue::as_u64)
        .map(|value| value as usize)
    else {
        return true;
    };
    let (request_start_line, request_start_character, request_end_line, request_end_character) =
        request;
    position_leq(
        request_start_line,
        request_start_character,
        surface_end_line,
        surface_end_character,
    ) && position_leq(
        surface_start_line,
        surface_start_character,
        request_end_line,
        request_end_character,
    )
}

fn position_leq(
    left_line: usize,
    left_character: usize,
    right_line: usize,
    right_character: usize,
) -> bool {
    left_line < right_line || (left_line == right_line && left_character <= right_character)
}

fn handle_code_actions(params: JsonValue, state: &ServerState) -> JsonValue {
    let Some(uri) = text_document_uri(&params) else {
        return json!([]);
    };
    let Some(document) = state.documents.get(&uri) else {
        return json!([]);
    };
    let requested_range = request_range(&params, &document.text);
    let requested_kinds = requested_code_action_kinds(&params);
    let program_for_range_filter = parse_program(&document.text).ok();
    let report = build_edit_plan_report_with_options(
        target_for_uri(&uri),
        parse_program(&document.text),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: None,
            module_name_hint: module_name_hint_for_uri(&uri),
        },
    );
    let mut actions = Vec::new();
    actions.extend(
        report
            .graft_templates
            .iter()
            .filter(|template| {
                code_action_surface_in_range(
                    program_for_range_filter.as_ref(),
                    &document.text,
                    requested_range,
                    &template.surface,
                )
            })
            .filter(|template| {
                code_action_kind_allowed(&requested_kinds, code_action_kind(&template.kind))
            })
            .map(|template| code_action_for_graft_template(&uri, &report.target, template)),
    );
    actions.extend(
        report
            .transaction_templates
            .iter()
            .filter(|template| {
                code_action_surface_in_range(
                    program_for_range_filter.as_ref(),
                    &document.text,
                    requested_range,
                    &template.surface,
                )
            })
            .filter(|template| {
                code_action_kind_allowed(&requested_kinds, code_action_kind(&template.kind))
            })
            .map(|template| code_action_for_transaction_template(&uri, &report.target, template)),
    );
    actions.truncate(50);
    json!(actions)
}

fn requested_code_action_kinds(params: &JsonValue) -> Vec<String> {
    params
        .pointer("/context/only")
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(JsonValue::as_str)
        .map(str::to_string)
        .collect()
}

fn code_action_kind_allowed(requested: &[String], actual: &str) -> bool {
    requested.is_empty()
        || requested.iter().any(|kind| {
            actual == kind
                || actual
                    .strip_prefix(kind)
                    .is_some_and(|tail| tail.starts_with('.'))
        })
}

fn code_action_surface_in_range(
    program: Option<&Program>,
    source: &str,
    range: Option<(usize, usize, usize, usize)>,
    surface: &str,
) -> bool {
    let (Some(program), Some(range)) = (program, range) else {
        return true;
    };
    let Some(surface_range) = range_for_code_action_surface(program, source, surface) else {
        return true;
    };
    request_range_intersects_lsp_range(range, &surface_range)
}

fn handle_code_lens(params: JsonValue, state: &ServerState) -> JsonValue {
    let Some(uri) = text_document_uri(&params) else {
        return json!([]);
    };
    let Some(document) = state.documents.get(&uri) else {
        return json!([]);
    };
    let Ok(program) = parse_program(&document.text) else {
        return json!([]);
    };
    let target = target_for_uri(&uri);
    let module_range = first_line_range(&document.text);
    let mut lenses = vec![
        command_preview_lens(
            &module_range,
            "Sley: doctor",
            &uri,
            "doctor",
            vec!["doctor", "--json", &target],
        ),
        command_preview_lens(
            &module_range,
            "Sley: verify --deny-warnings",
            &uri,
            "verify",
            vec!["verify", "--json", "--deny-warnings", &target],
        ),
        command_preview_lens(
            &module_range,
            "Sley: deploy --dry-run",
            &uri,
            "deploy_dry_run",
            vec!["deploy", "--json", "--dry-run", &target],
        ),
    ];
    for task in &program.tasks {
        let Some(range) = range_for_task_name(&document.text, task) else {
            continue;
        };
        let slice_arg = task.id.as_str();
        lenses.push(command_preview_lens(
            &range,
            "Sley: graph slice",
            &uri,
            "graph_slice",
            vec!["graph", "--json", "--slice", slice_arg, &target],
        ));
    }
    json!(lenses)
}

fn handle_execute_command(params: JsonValue) -> JsonValue {
    let command = params
        .get("command")
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    let preview = params
        .get("arguments")
        .and_then(JsonValue::as_array)
        .and_then(|arguments| arguments.first())
        .cloned()
        .unwrap_or(JsonValue::Null);
    match command {
        FIX_PREVIEW_COMMAND => json!({
            "schema": FIX_PREVIEW_SCHEMA,
            "status": "preview",
            "preview": preview
        }),
        COMMAND_PREVIEW_COMMAND => json!({
            "schema": COMMAND_PREVIEW_SCHEMA,
            "status": "preview",
            "preview": preview
        }),
        _ => JsonValue::Null,
    }
}

fn command_preview_lens(
    range: &JsonValue,
    title: &str,
    uri: &str,
    kind: &str,
    args: Vec<&str>,
) -> JsonValue {
    json!({
        "range": range,
        "command": {
            "title": title,
            "command": COMMAND_PREVIEW_COMMAND,
            "arguments": [{
                "schema": COMMAND_PREVIEW_SCHEMA,
                "uri": uri,
                "kind": kind,
                "command": "sley",
                "args": args
            }]
        }
    })
}

fn publish_all_open_document_diagnostics<W: Write>(
    writer: &mut W,
    state: &ServerState,
) -> Result<()> {
    let mut documents = state
        .documents
        .iter()
        .map(|(uri, document)| (uri.clone(), document.text.clone(), document.version))
        .collect::<Vec<_>>();
    documents.sort_by(|left, right| left.0.cmp(&right.0));
    for (uri, text, version) in documents {
        publish_diagnostics(writer, &uri, &text, version, state)?;
    }
    Ok(())
}

fn publish_diagnostics<W: Write>(
    writer: &mut W,
    uri: &str,
    text: &str,
    version: Option<i64>,
    state: &ServerState,
) -> Result<()> {
    let mut diagnostics = Vec::new();
    match parse_program(text) {
        Ok(current_program) => {
            let (analysis_program, project_diagnostics) =
                analysis_program_for_document(uri, &current_program, state);
            diagnostics.extend(
                project_diagnostics
                    .iter()
                    .filter(|diagnostic| {
                        project_diagnostic_belongs_to_document(&current_program, diagnostic)
                    })
                    .map(|diagnostic| compiler_diagnostic_to_lsp(text, diagnostic)),
            );
            let compiler_diagnostics = check_program(&analysis_program);
            let compiler_has_errors = has_errors(&compiler_diagnostics);
            diagnostics.extend(
                compiler_diagnostics
                    .iter()
                    .filter(|diagnostic| {
                        diagnostic_belongs_to_document(&current_program, diagnostic)
                    })
                    .map(|diagnostic| compiler_diagnostic_to_lsp(text, diagnostic)),
            );
            if !compiler_has_errors && project_diagnostics.is_empty() {
                let lint_report = build_lint_report(&analysis_program, LintOptions::default());
                diagnostics.extend(
                    lint_report
                        .findings
                        .iter()
                        .filter(|finding| {
                            lint_finding_belongs_to_document(&current_program, finding)
                        })
                        .map(|finding| lint_finding_to_lsp(text, &current_program, finding)),
                );
            }
        }
        Err(parse_diagnostics) => diagnostics.extend(
            parse_diagnostics
                .iter()
                .map(|diagnostic| compiler_diagnostic_to_lsp(text, diagnostic)),
        ),
    }

    let mut params = json!({
        "uri": uri,
        "diagnostics": diagnostics
    });
    if let Some(version) = version {
        params["version"] = json!(version);
    }
    send_notification(writer, "textDocument/publishDiagnostics", params)
}

fn analysis_program_for_document(
    uri: &str,
    current_program: &Program,
    state: &ServerState,
) -> (Program, Vec<Diagnostic>) {
    match load_project_graph_for_document(uri, current_program, state) {
        Ok(Some(project)) => (project.program, Vec::new()),
        Ok(None) => (current_program.clone(), Vec::new()),
        Err(diagnostics) => (current_program.clone(), diagnostics),
    }
}

fn project_graph_for_document(
    uri: &str,
    current_program: &Program,
    state: &ServerState,
) -> Option<ProjectGraph> {
    load_project_graph_for_document(uri, current_program, state)
        .ok()
        .flatten()
}

fn load_project_graph_for_document(
    uri: &str,
    current_program: &Program,
    state: &ServerState,
) -> Result<Option<ProjectGraph>, Vec<Diagnostic>> {
    let Some(project_root) = project_root_for_uri(uri) else {
        return Ok(None);
    };
    let source_overlays = state.source_overlays();
    load_project_with_source_overlays(project_root, &source_overlays).map(|project| {
        project
            .modules
            .iter()
            .any(|module| module.module == current_program.module_name())
            .then_some(project)
    })
}

fn project_diagnostic_belongs_to_document(program: &Program, diagnostic: &Diagnostic) -> bool {
    match diagnostic.node.as_deref() {
        Some(node) if !node.is_empty() => node_belongs_to_program(program, node),
        _ => !diagnostic.id.starts_with("PARSE_"),
    }
}

fn diagnostic_belongs_to_document(program: &Program, diagnostic: &Diagnostic) -> bool {
    match diagnostic.node.as_deref() {
        Some(node) if !node.is_empty() => node_belongs_to_program(program, node),
        _ => true,
    }
}

fn lint_finding_belongs_to_document(program: &Program, finding: &LintFinding) -> bool {
    node_belongs_to_program(program, &finding.node)
}

fn node_belongs_to_program(program: &Program, node: &str) -> bool {
    span_for_node(program, node).is_some()
        || program.module.as_deref().is_some_and(|module| {
            node == format!("module:{module}")
                || node.contains(&format!("{module}."))
                || node.contains(&format!(":{module}:"))
        })
}

fn compiler_diagnostic_to_lsp(text: &str, diagnostic: &Diagnostic) -> JsonValue {
    json!({
        "range": diagnostic
            .span
            .as_ref()
            .map(|span| range_for_span(text, span))
            .unwrap_or_else(|| full_document_range(text)),
        "severity": severity_number(&diagnostic.severity),
        "code": diagnostic.id,
        "source": "sley",
        "message": diagnostic.message,
        "data": {
            "node": diagnostic.node,
            "repairHints": diagnostic.repair_hints
        }
    })
}

fn lint_finding_to_lsp(text: &str, program: &Program, finding: &LintFinding) -> JsonValue {
    let range = span_for_node(program, &finding.node)
        .as_ref()
        .map(|span| range_for_span(text, span))
        .unwrap_or_else(|| full_document_range(text));
    json!({
        "range": range,
        "severity": 2,
        "code": finding.rule,
        "source": "sley-lint",
        "message": finding.message,
        "data": {
            "id": finding.id,
            "node": finding.node,
            "module": finding.module,
            "hint": finding.hint
        }
    })
}

fn severity_number(severity: &Severity) -> u8 {
    match severity {
        Severity::Error => 1,
        Severity::Warning => 2,
        Severity::Info => 3,
    }
}

fn document_symbols(program: &Program, text: &str) -> Vec<JsonValue> {
    let mut symbols = Vec::new();
    if let Some(module) = &program.module {
        symbols.push(json!({
            "name": module,
            "kind": 2,
            "detail": "module",
            "range": first_line_range(text),
            "selectionRange": first_line_range(text)
        }));
    }
    for import in &program.imports {
        symbols.push(symbol_json(
            import.alias.as_deref().unwrap_or(&import.module),
            4,
            "import",
            text,
            import.span.as_ref(),
        ));
    }
    for ty in &program.types {
        symbols.push(symbol_json(&ty.name, 23, "type", text, ty.span.as_ref()));
    }
    for effect in &program.effects {
        symbols.push(symbol_json(
            &effect.name,
            24,
            "effect",
            text,
            effect.span.as_ref(),
        ));
    }
    for task in &program.tasks {
        symbols.push(symbol_json(
            &task.name,
            12,
            "task",
            text,
            task.span.as_ref(),
        ));
    }
    symbols
}

fn document_links_for_imports(
    source: &str,
    program: &Program,
    project: &ProjectGraph,
) -> Vec<JsonValue> {
    program
        .imports
        .iter()
        .filter_map(|import| {
            let module = project
                .modules
                .iter()
                .find(|module| module.module == import.module)?;
            let range = range_for_import_module(source, import)?;
            Some(json!({
                "range": range,
                "target": file_uri_for_path(&module.path),
                "tooltip": format!("Open module {}", import.module)
            }))
        })
        .collect()
}

fn folding_ranges_for_program(source: &str, program: &Program) -> Vec<JsonValue> {
    let mut ranges = Vec::new();
    for ty in &program.types {
        push_folding_range_for_span(source, ty.span.as_ref(), &mut ranges);
    }
    for task in &program.tasks {
        push_folding_range_for_span(source, task.span.as_ref(), &mut ranges);
        collect_statement_folding_ranges(source, &task.body.statements, &mut ranges);
    }
    ranges.sort_by_key(folding_range_sort_key);
    ranges
}

fn collect_statement_folding_ranges(
    source: &str,
    statements: &[Statement],
    ranges: &mut Vec<JsonValue>,
) {
    for statement in statements {
        match &statement.kind {
            StatementKind::If {
                then_block,
                else_block,
                ..
            } => {
                push_folding_range_for_span(source, statement.span.as_ref(), ranges);
                collect_statement_folding_ranges(source, &then_block.statements, ranges);
                if let Some(else_block) = else_block {
                    collect_statement_folding_ranges(source, &else_block.statements, ranges);
                }
            }
            StatementKind::While { body, .. } => {
                push_folding_range_for_span(source, statement.span.as_ref(), ranges);
                collect_statement_folding_ranges(source, &body.statements, ranges);
            }
            StatementKind::For { body, .. } => {
                push_folding_range_for_span(source, statement.span.as_ref(), ranges);
                collect_statement_folding_ranges(source, &body.statements, ranges);
            }
            StatementKind::Forge { body } => {
                push_folding_range_for_span(source, statement.span.as_ref(), ranges);
                collect_statement_folding_ranges(source, &body.statements, ranges);
            }
            StatementKind::Binding { .. }
            | StatementKind::Set { .. }
            | StatementKind::Return { .. }
            | StatementKind::Expr { .. } => {}
        }
    }
}

fn push_folding_range_for_span(
    source: &str,
    span: Option<&SourceSpan>,
    ranges: &mut Vec<JsonValue>,
) {
    if let Some(span) = span
        && let Some(range) = folding_range_for_braced_region(source, span)
    {
        ranges.push(range);
    }
}

fn folding_range_sort_key(range: &JsonValue) -> (u64, u64) {
    (
        range
            .get("startLine")
            .and_then(JsonValue::as_u64)
            .unwrap_or_default(),
        range
            .get("endLine")
            .and_then(JsonValue::as_u64)
            .unwrap_or_default(),
    )
}

fn selection_range_at_position(
    program: &Program,
    source: &str,
    line: usize,
    character: usize,
) -> Option<JsonValue> {
    let task_parent = enclosing_task_selection_range(program, source, line, character);
    if let Some((leaf_range, expression_range)) =
        call_selection_ranges_at_position(program, source, line, character)
    {
        let parent = expression_range
            .filter(|range| range != &leaf_range)
            .map(|range| selection_range_json(range, task_parent.clone()))
            .or(task_parent);
        return Some(selection_range_json(leaf_range, parent));
    }
    for task in &program.tasks {
        if let Some(name_range) = range_for_task_name(source, task)
            && position_in_range(&name_range, line, character)
        {
            let parent = task
                .span
                .as_ref()
                .and_then(|span| range_for_braced_span(source, span))
                .map(|range| selection_range_json(range, None));
            return Some(selection_range_json(name_range, parent));
        }
    }
    for import in &program.imports {
        if let Some(module_range) = range_for_import_module(source, import)
            && position_in_range(&module_range, line, character)
        {
            let parent = import
                .span
                .as_ref()
                .map(|span| line_range_for_span(source, span))
                .map(|range| selection_range_json(range, None));
            return Some(selection_range_json(module_range, parent));
        }
    }
    task_parent
}

fn enclosing_task_selection_range(
    program: &Program,
    source: &str,
    line: usize,
    character: usize,
) -> Option<JsonValue> {
    for task in &program.tasks {
        let Some(range) = task
            .span
            .as_ref()
            .and_then(|span| range_for_braced_span(source, span))
        else {
            continue;
        };
        if position_in_range(&range, line, character) {
            return Some(selection_range_json(range, None));
        }
    }
    None
}

fn call_selection_ranges_at_position(
    program: &Program,
    source: &str,
    line: usize,
    character: usize,
) -> Option<(JsonValue, Option<JsonValue>)> {
    for task in &program.tasks {
        if let Some(selection) =
            call_selection_ranges_in_statements(&task.body.statements, source, line, character)
        {
            return Some(selection);
        }
    }
    None
}

fn call_selection_ranges_in_statements(
    statements: &[Statement],
    source: &str,
    line: usize,
    character: usize,
) -> Option<(JsonValue, Option<JsonValue>)> {
    for statement in statements {
        match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                if let Some(selection) =
                    call_selection_ranges_in_expr(expr, source, line, character)
                {
                    return Some(selection);
                }
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                if let Some(selection) =
                    call_selection_ranges_in_expr(condition, source, line, character)
                {
                    return Some(selection);
                }
                if let Some(selection) = call_selection_ranges_in_statements(
                    &then_block.statements,
                    source,
                    line,
                    character,
                ) {
                    return Some(selection);
                }
                if let Some(else_block) = else_block
                    && let Some(selection) = call_selection_ranges_in_statements(
                        &else_block.statements,
                        source,
                        line,
                        character,
                    )
                {
                    return Some(selection);
                }
            }
            StatementKind::While { condition, body } => {
                if let Some(selection) =
                    call_selection_ranges_in_expr(condition, source, line, character)
                {
                    return Some(selection);
                }
                if let Some(selection) =
                    call_selection_ranges_in_statements(&body.statements, source, line, character)
                {
                    return Some(selection);
                }
            }
            StatementKind::For {
                collection, body, ..
            } => {
                if let Some(selection) =
                    call_selection_ranges_in_expr(collection, source, line, character)
                {
                    return Some(selection);
                }
                if let Some(selection) =
                    call_selection_ranges_in_statements(&body.statements, source, line, character)
                {
                    return Some(selection);
                }
            }
            StatementKind::Forge { body } => {
                if let Some(selection) =
                    call_selection_ranges_in_statements(&body.statements, source, line, character)
                {
                    return Some(selection);
                }
            }
        }
    }
    None
}

fn call_selection_ranges_in_expr(
    expr: &Expr,
    source: &str,
    line: usize,
    character: usize,
) -> Option<(JsonValue, Option<JsonValue>)> {
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            if let Some(selection) = call_selection_ranges_in_expr(callee, source, line, character)
            {
                return Some(selection);
            }
            for arg in args {
                if let Some(selection) = call_selection_ranges_in_expr(arg, source, line, character)
                {
                    return Some(selection);
                }
            }
            let expression_range =
                range_for_expr_source_at_position(source, expr, line, character)?;
            if let Some(callee_name) = callee_path(callee) {
                let leaf_name = callee_leaf_name(&callee_name);
                if let Some(leaf_range) =
                    range_for_callee_leaf(source, expr, callee, &callee_name, leaf_name)
                    && position_in_range(&leaf_range, line, character)
                {
                    return Some((leaf_range, Some(expression_range)));
                }
            }
            Some((expression_range, None))
        }
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            call_selection_ranges_in_expr(expr, source, line, character)
        }
        ExprKind::Binary { left, right, .. } => {
            call_selection_ranges_in_expr(left, source, line, character)
                .or_else(|| call_selection_ranges_in_expr(right, source, line, character))
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => call_selection_ranges_in_expr(condition, source, line, character)
            .or_else(|| call_selection_ranges_in_expr(then_branch, source, line, character))
            .or_else(|| call_selection_ranges_in_expr(else_branch, source, line, character)),
        ExprKind::ListLiteral { items } => items
            .iter()
            .find_map(|item| call_selection_ranges_in_expr(item, source, line, character)),
        ExprKind::MapLiteral { entries } => entries.iter().find_map(|entry| {
            call_selection_ranges_in_expr(&entry.key, source, line, character)
                .or_else(|| call_selection_ranges_in_expr(&entry.value, source, line, character))
        }),
        ExprKind::Index { collection, index } => {
            call_selection_ranges_in_expr(collection, source, line, character)
                .or_else(|| call_selection_ranges_in_expr(index, source, line, character))
        }
        ExprKind::FieldAccess { receiver, .. } => {
            call_selection_ranges_in_expr(receiver, source, line, character)
        }
        ExprKind::RecordLiteral { fields, .. } => fields
            .iter()
            .find_map(|field| call_selection_ranges_in_expr(&field.expr, source, line, character)),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => None,
    }
}

fn selection_range_json(range: JsonValue, parent: Option<JsonValue>) -> JsonValue {
    let mut selection = json!({ "range": range });
    if let Some(parent) = parent {
        selection["parent"] = parent;
    }
    selection
}

fn symbol_json(
    name: &str,
    kind: u8,
    detail: &str,
    text: &str,
    span: Option<&SourceSpan>,
) -> JsonValue {
    let range = span
        .map(|span| range_for_span(text, span))
        .unwrap_or_else(|| full_document_range(text));
    json!({
        "name": name,
        "kind": kind,
        "detail": detail,
        "range": range,
        "selectionRange": range
    })
}

fn hover_for_line(program: &Program, text: &str, line: usize) -> Option<JsonValue> {
    if line == 0 {
        if let Some(module) = &program.module {
            return Some(hover_json(&module_hover(module), first_line_range(text)));
        }
    }
    for import in &program.imports {
        if span_line_matches(import.span.as_ref(), line) {
            return Some(hover_json(
                &import_hover(import),
                import
                    .span
                    .as_ref()
                    .map(|span| range_for_span(text, span))?,
            ));
        }
    }
    for ty in &program.types {
        if span_line_matches(ty.span.as_ref(), line) {
            return Some(hover_json(
                &type_hover(ty, program.module_name()),
                ty.span.as_ref().map(|span| range_for_span(text, span))?,
            ));
        }
    }
    for effect in &program.effects {
        if span_line_matches(effect.span.as_ref(), line) {
            return Some(hover_json(
                &effect_hover(effect, program.module_name()),
                effect
                    .span
                    .as_ref()
                    .map(|span| range_for_span(text, span))?,
            ));
        }
    }
    for task in &program.tasks {
        if span_line_matches(task.span.as_ref(), line) {
            return Some(hover_json(
                &task_hover(task, program.module_name()),
                task.span.as_ref().map(|span| range_for_span(text, span))?,
            ));
        }
    }
    None
}

fn module_hover(module: &str) -> String {
    format!("module `{module}`")
}

fn import_hover(import: &ImportDecl) -> String {
    let alias = import
        .alias
        .as_ref()
        .map(|alias| format!("\n\nalias: `{alias}`"))
        .unwrap_or_default();
    format!("import `{}`{alias}\n\nnode: `{}`", import.module, import.id)
}

fn type_hover(ty: &TypeDecl, default_module: &str) -> String {
    let export = if ty.exported { "export " } else { "" };
    format!(
        "{export}type `{}` = `{}`\n\nmodule: `{}`\nnode: `{}`",
        ty.name,
        ty.value.display(),
        ty.module.as_deref().unwrap_or(default_module),
        ty.id
    )
}

fn effect_hover(effect: &EffectDecl, default_module: &str) -> String {
    let export = if effect.exported { "export " } else { "" };
    format!(
        "{export}effect `{}`\n\nmodule: `{}`\nnode: `{}`",
        effect.name,
        effect.module.as_deref().unwrap_or(default_module),
        effect.id
    )
}

fn task_hover(task: &TaskDecl, default_module: &str) -> String {
    let export = if task.exported { "export " } else { "" };
    let takes = if task.takes.is_empty() {
        "none".to_string()
    } else {
        task.takes
            .iter()
            .map(|take| format!("`{}: {}`", take.name, take.ty.display()))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let effects = if task.effects.is_empty() {
        "none".to_string()
    } else {
        task.effects
            .iter()
            .map(|effect| format!("`{effect}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "{export}task `{}`\n\nmodule: `{}`\nreturns: `{}`\ntakes: {takes}\neffects: {effects}\nnode: `{}`",
        task.name,
        task.module.as_deref().unwrap_or(default_module),
        task.return_type.display(),
        task.id
    )
}

fn task_call_hover(task: &TaskDecl, default_module: &str, callee_name: &str) -> String {
    let parameter_labels = task
        .takes
        .iter()
        .map(|take| format!("{}: {}", take.name, take.ty.display()))
        .collect::<Vec<_>>();
    let signature = format!(
        "{callee_name}({}) -> {}",
        parameter_labels.join(", "),
        task.return_type.display()
    );
    let effects = if task.effects.is_empty() {
        "none".to_string()
    } else {
        task.effects
            .iter()
            .map(|effect| format!("`{effect}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let visibility = if task.exported { "exported" } else { "private" };
    let task_module = task.module.as_deref().unwrap_or(default_module);
    format!(
        "call `{callee_name}`\n\nresolved task: `{}.{}`\nsignature: `{signature}`\nvisibility: `{visibility}`\neffects: {effects}\nnode: `{}`",
        task_module, task.name, task.id
    )
}

fn host_call_hover(callee_name: &str, effects: &[&str]) -> String {
    let effects_text = effects
        .iter()
        .map(|effect| format!("`{effect}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let cap_hint = effects
        .first()
        .map(|effect| format!("`--cap {effect}[=SCOPE]`"))
        .unwrap_or_else(|| "`--cap EFFECT[=SCOPE]`".to_string());
    format!(
        "host call `{callee_name}`\n\nrequired capabilities: {effects_text}\nseeded run flag: {cap_hint}\nauthority failures are diagnostics; recoverable host failures return `Result` values"
    )
}

fn task_signature_help(
    task: &TaskDecl,
    default_module: &str,
    callee_name: &str,
    active_parameter: usize,
) -> JsonValue {
    let parameter_labels = task
        .takes
        .iter()
        .map(|take| format!("{}: {}", take.name, take.ty.display()))
        .collect::<Vec<_>>();
    let parameters = parameter_labels
        .iter()
        .map(|label| {
            json!({
                "label": label,
                "documentation": {
                    "kind": "markdown",
                    "value": format!("`{label}`")
                }
            })
        })
        .collect::<Vec<_>>();
    let bounded_active_parameter = if task.takes.is_empty() {
        0
    } else {
        active_parameter.min(task.takes.len().saturating_sub(1))
    };
    let takes = if task.takes.is_empty() {
        "none".to_string()
    } else {
        parameter_labels
            .iter()
            .map(|label| format!("`{label}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let effects = if task.effects.is_empty() {
        "none".to_string()
    } else {
        task.effects
            .iter()
            .map(|effect| format!("`{effect}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let label = format!(
        "{callee_name}({}) -> {}",
        parameter_labels.join(", "),
        task.return_type.display()
    );
    json!({
        "signatures": [{
            "label": label,
            "documentation": {
                "kind": "markdown",
                "value": format!(
                    "task `{}`\n\nmodule: `{}`\nreturns: `{}`\ntakes: {takes}\neffects: {effects}\nnode: `{}`",
                    task.name,
                    task.module.as_deref().unwrap_or(default_module),
                    task.return_type.display(),
                    task.id
                )
            },
            "parameters": parameters
        }],
        "activeSignature": 0,
        "activeParameter": bounded_active_parameter
    })
}

fn inlay_hints_for_program(
    current_program: &Program,
    analysis_program: &Program,
    source: &str,
    range: (usize, usize, usize, usize),
) -> Vec<JsonValue> {
    let mut hints = Vec::new();
    for task in &current_program.tasks {
        let caller_module = task
            .module
            .clone()
            .unwrap_or_else(|| current_program.module_name().to_string());
        collect_inlay_hints_from_statements(
            analysis_program,
            &caller_module,
            source,
            &task.body.statements,
            range,
            &mut hints,
        );
    }
    hints.sort_by_key(inlay_hint_sort_key);
    hints
}

fn collect_inlay_hints_from_statements(
    analysis_program: &Program,
    caller_module: &str,
    source: &str,
    statements: &[Statement],
    range: (usize, usize, usize, usize),
    hints: &mut Vec<JsonValue>,
) {
    for statement in statements {
        match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => collect_inlay_hints_from_expr(
                analysis_program,
                caller_module,
                source,
                expr,
                range,
                hints,
            ),
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                collect_inlay_hints_from_expr(
                    analysis_program,
                    caller_module,
                    source,
                    condition,
                    range,
                    hints,
                );
                collect_inlay_hints_from_statements(
                    analysis_program,
                    caller_module,
                    source,
                    &then_block.statements,
                    range,
                    hints,
                );
                if let Some(else_block) = else_block {
                    collect_inlay_hints_from_statements(
                        analysis_program,
                        caller_module,
                        source,
                        &else_block.statements,
                        range,
                        hints,
                    );
                }
            }
            StatementKind::While { condition, body } => {
                collect_inlay_hints_from_expr(
                    analysis_program,
                    caller_module,
                    source,
                    condition,
                    range,
                    hints,
                );
                collect_inlay_hints_from_statements(
                    analysis_program,
                    caller_module,
                    source,
                    &body.statements,
                    range,
                    hints,
                );
            }
            StatementKind::For {
                collection, body, ..
            } => {
                collect_inlay_hints_from_expr(
                    analysis_program,
                    caller_module,
                    source,
                    collection,
                    range,
                    hints,
                );
                collect_inlay_hints_from_statements(
                    analysis_program,
                    caller_module,
                    source,
                    &body.statements,
                    range,
                    hints,
                );
            }
            StatementKind::Forge { body } => collect_inlay_hints_from_statements(
                analysis_program,
                caller_module,
                source,
                &body.statements,
                range,
                hints,
            ),
        }
    }
}

fn collect_inlay_hints_from_expr(
    analysis_program: &Program,
    caller_module: &str,
    source: &str,
    expr: &Expr,
    range: (usize, usize, usize, usize),
    hints: &mut Vec<JsonValue>,
) {
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            if let Some(callee_name) = callee_path(callee)
                && let TaskResolution::Resolved { index, .. } =
                    resolve_task(analysis_program, caller_module, &callee_name)
            {
                let task = &analysis_program.tasks[index];
                let parameter_takes = task
                    .takes
                    .iter()
                    .filter(|take| take.binding_kind != BindingKind::Gate)
                    .collect::<Vec<_>>();
                for (arg, take) in args.iter().zip(parameter_takes) {
                    if let Some(hint) = inlay_hint_for_arg(source, arg, take, range) {
                        hints.push(hint);
                    }
                }
            }
            collect_inlay_hints_from_expr(
                analysis_program,
                caller_module,
                source,
                callee,
                range,
                hints,
            );
            for arg in args {
                collect_inlay_hints_from_expr(
                    analysis_program,
                    caller_module,
                    source,
                    arg,
                    range,
                    hints,
                );
            }
        }
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            collect_inlay_hints_from_expr(
                analysis_program,
                caller_module,
                source,
                expr,
                range,
                hints,
            );
        }
        ExprKind::Binary { left, right, .. } => {
            collect_inlay_hints_from_expr(
                analysis_program,
                caller_module,
                source,
                left,
                range,
                hints,
            );
            collect_inlay_hints_from_expr(
                analysis_program,
                caller_module,
                source,
                right,
                range,
                hints,
            );
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_inlay_hints_from_expr(
                analysis_program,
                caller_module,
                source,
                condition,
                range,
                hints,
            );
            collect_inlay_hints_from_expr(
                analysis_program,
                caller_module,
                source,
                then_branch,
                range,
                hints,
            );
            collect_inlay_hints_from_expr(
                analysis_program,
                caller_module,
                source,
                else_branch,
                range,
                hints,
            );
        }
        ExprKind::ListLiteral { items } => {
            for item in items {
                collect_inlay_hints_from_expr(
                    analysis_program,
                    caller_module,
                    source,
                    item,
                    range,
                    hints,
                );
            }
        }
        ExprKind::MapLiteral { entries } => {
            for entry in entries {
                collect_inlay_hints_from_expr(
                    analysis_program,
                    caller_module,
                    source,
                    &entry.key,
                    range,
                    hints,
                );
                collect_inlay_hints_from_expr(
                    analysis_program,
                    caller_module,
                    source,
                    &entry.value,
                    range,
                    hints,
                );
            }
        }
        ExprKind::Index { collection, index } => {
            collect_inlay_hints_from_expr(
                analysis_program,
                caller_module,
                source,
                collection,
                range,
                hints,
            );
            collect_inlay_hints_from_expr(
                analysis_program,
                caller_module,
                source,
                index,
                range,
                hints,
            );
        }
        ExprKind::FieldAccess { receiver, .. } => collect_inlay_hints_from_expr(
            analysis_program,
            caller_module,
            source,
            receiver,
            range,
            hints,
        ),
        ExprKind::RecordLiteral { fields, .. } => {
            for field in fields {
                collect_inlay_hints_from_expr(
                    analysis_program,
                    caller_module,
                    source,
                    &field.expr,
                    range,
                    hints,
                );
            }
        }
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => {}
    }
}

fn inlay_hint_for_arg(
    source: &str,
    arg: &Expr,
    take: &sley::ast::TakeDecl,
    range: (usize, usize, usize, usize),
) -> Option<JsonValue> {
    let span = arg.span.as_ref()?;
    let line = span.line.saturating_sub(1);
    let character = span.column.saturating_sub(1);
    if !position_in_request_range(range, line, character) {
        return None;
    }
    let line_width = source_line_width(source, line);
    if character > line_width {
        return None;
    }
    Some(json!({
        "position": {
            "line": line,
            "character": character
        },
        "label": format!("{}:", take.name),
        "kind": 2,
        "tooltip": format!("{}: {}", take.name, take.ty.display()),
        "paddingRight": true
    }))
}

fn inlay_hint_sort_key(hint: &JsonValue) -> (u64, u64, String) {
    (
        hint.pointer("/position/line")
            .and_then(JsonValue::as_u64)
            .unwrap_or_default(),
        hint.pointer("/position/character")
            .and_then(JsonValue::as_u64)
            .unwrap_or_default(),
        hint.get("label")
            .and_then(JsonValue::as_str)
            .unwrap_or_default()
            .to_string(),
    )
}

const SEMANTIC_NAMESPACE: u32 = 0;
const SEMANTIC_TYPE: u32 = 1;
const SEMANTIC_FUNCTION: u32 = 2;
const SEMANTIC_PARAMETER: u32 = 3;
const SEMANTIC_VARIABLE: u32 = 4;
const SEMANTIC_KEYWORD: u32 = 5;
const SEMANTIC_STRING: u32 = 6;
const SEMANTIC_NUMBER: u32 = 7;
const SEMANTIC_COMMENT: u32 = 8;
const SEMANTIC_OPERATOR: u32 = 9;

const SEMANTIC_MOD_DECLARATION: u32 = 1 << 0;
const SEMANTIC_MOD_DEFAULT_LIBRARY: u32 = 1 << 1;

#[derive(Debug, Clone)]
struct SemanticToken {
    line: u32,
    character: u32,
    length: u32,
    token_type: u32,
    modifiers: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SemanticContext {
    NamespaceLine,
    FunctionDeclaration,
    TypeDeclaration,
    EffectDeclaration,
    ParameterDeclaration,
    VariableDeclaration,
    ForItem,
}

fn semantic_tokens_data(source: &str) -> Vec<u32> {
    let mut tokens = semantic_tokens_for_source(source);
    tokens.sort_by_key(|token| (token.line, token.character));
    let mut data = Vec::with_capacity(tokens.len() * 5);
    let mut previous_line = 0u32;
    let mut previous_character = 0u32;
    for token in tokens {
        let delta_line = token.line.saturating_sub(previous_line);
        let delta_character = if delta_line == 0 {
            token.character.saturating_sub(previous_character)
        } else {
            token.character
        };
        data.extend([
            delta_line,
            delta_character,
            token.length,
            token.token_type,
            token.modifiers,
        ]);
        previous_line = token.line;
        previous_character = token.character;
    }
    data
}

fn semantic_tokens_for_source(source: &str) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();
    for (line_index, line) in source.lines().enumerate() {
        collect_semantic_tokens_for_line(line, line_index as u32, &mut tokens);
    }
    tokens
}

fn collect_semantic_tokens_for_line(line: &str, line_index: u32, tokens: &mut Vec<SemanticToken>) {
    let mut byte = 0usize;
    let mut context: Option<SemanticContext> = None;
    let mut namespace_context = false;

    while byte < line.len() {
        let Some(ch) = line.get(byte..).and_then(|text| text.chars().next()) else {
            break;
        };
        if ch.is_whitespace() {
            byte += ch.len_utf8();
            continue;
        }
        if line.get(byte..).is_some_and(|text| text.starts_with("//")) || ch == '#' {
            push_semantic_token(
                tokens,
                line,
                line_index,
                byte,
                line.len(),
                SEMANTIC_COMMENT,
                0,
            );
            break;
        }
        if ch == '"' {
            let end = semantic_string_end(line, byte);
            push_semantic_token(tokens, line, line_index, byte, end, SEMANTIC_STRING, 0);
            byte = end;
            continue;
        }
        if ch.is_ascii_digit() {
            let end = semantic_number_end(line, byte);
            push_semantic_token(tokens, line, line_index, byte, end, SEMANTIC_NUMBER, 0);
            byte = end;
            continue;
        }
        if is_identifier_start(ch) {
            let end = semantic_identifier_end(line, byte);
            let identifier = &line[byte..end];
            let next_byte = next_non_whitespace_byte(line, end);
            let previous_byte = previous_non_whitespace_byte(line, byte);
            let (token_type, modifiers) = semantic_identifier_kind(
                identifier,
                context,
                namespace_context,
                previous_byte
                    .and_then(|previous| line.get(previous..).and_then(|text| text.chars().next())),
                next_byte.and_then(|next| line.get(next..).and_then(|text| text.chars().next())),
            );
            push_semantic_token(tokens, line, line_index, byte, end, token_type, modifiers);

            if is_semantic_keyword(identifier) {
                context = semantic_context_after_keyword(identifier);
                namespace_context = matches!(
                    context,
                    Some(SemanticContext::NamespaceLine) | Some(SemanticContext::EffectDeclaration)
                );
            } else if !namespace_context {
                context = None;
            } else if next_byte
                .and_then(|next| line.get(next..).and_then(|text| text.chars().next()))
                != Some('.')
            {
                context = None;
                namespace_context = false;
            }
            byte = end;
            continue;
        }
        let end = byte + ch.len_utf8();
        if is_semantic_operator(ch)
            || ch == '-' && line.get(end..).is_some_and(|text| text.starts_with('>'))
        {
            let token_end =
                if ch == '-' && line.get(end..).is_some_and(|text| text.starts_with('>')) {
                    end + 1
                } else {
                    end
                };
            push_semantic_token(
                tokens,
                line,
                line_index,
                byte,
                token_end,
                SEMANTIC_OPERATOR,
                0,
            );
            byte = token_end;
            continue;
        }
        if ch == ',' || ch == '{' || ch == '}' || ch == '(' || ch == ')' || ch == ':' {
            context = None;
            namespace_context = false;
        }
        byte = end;
    }
}

fn semantic_identifier_kind(
    identifier: &str,
    context: Option<SemanticContext>,
    namespace_context: bool,
    previous_char: Option<char>,
    next_char: Option<char>,
) -> (u32, u32) {
    if is_semantic_keyword(identifier) {
        return (SEMANTIC_KEYWORD, 0);
    }
    match context {
        Some(SemanticContext::NamespaceLine) if namespace_context => {
            (SEMANTIC_NAMESPACE, SEMANTIC_MOD_DECLARATION)
        }
        Some(SemanticContext::FunctionDeclaration) => (SEMANTIC_FUNCTION, SEMANTIC_MOD_DECLARATION),
        Some(SemanticContext::TypeDeclaration) => (SEMANTIC_TYPE, SEMANTIC_MOD_DECLARATION),
        Some(SemanticContext::EffectDeclaration) => (
            SEMANTIC_TYPE,
            SEMANTIC_MOD_DECLARATION | SEMANTIC_MOD_DEFAULT_LIBRARY,
        ),
        Some(SemanticContext::ParameterDeclaration) => {
            (SEMANTIC_PARAMETER, SEMANTIC_MOD_DECLARATION)
        }
        Some(SemanticContext::VariableDeclaration) | Some(SemanticContext::ForItem) => {
            (SEMANTIC_VARIABLE, SEMANTIC_MOD_DECLARATION)
        }
        None if previous_char == Some('.') && next_char == Some('(') => (SEMANTIC_FUNCTION, 0),
        None if next_char == Some('(') => (SEMANTIC_FUNCTION, 0),
        None if identifier
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase()) =>
        {
            (SEMANTIC_TYPE, SEMANTIC_MOD_DEFAULT_LIBRARY)
        }
        _ => (SEMANTIC_VARIABLE, 0),
    }
}

fn semantic_context_after_keyword(keyword: &str) -> Option<SemanticContext> {
    match keyword {
        "module" | "import" | "as" => Some(SemanticContext::NamespaceLine),
        "task" => Some(SemanticContext::FunctionDeclaration),
        "type" => Some(SemanticContext::TypeDeclaration),
        "effect" => Some(SemanticContext::EffectDeclaration),
        "take" => Some(SemanticContext::ParameterDeclaration),
        "bind" | "state" | "cell" | "knot" | "slot" | "gate" | "lease" | "veil" | "dial"
        | "flag" | "memo" | "cache" | "derive" | "flow" | "port" | "tally" | "hole" | "draft"
        | "taint" | "witness" | "seal" | "anchor" | "view" | "cursor" => {
            Some(SemanticContext::VariableDeclaration)
        }
        "for" => Some(SemanticContext::ForItem),
        _ => None,
    }
}

fn push_semantic_token(
    tokens: &mut Vec<SemanticToken>,
    line: &str,
    line_index: u32,
    start_byte: usize,
    end_byte: usize,
    token_type: u32,
    modifiers: u32,
) {
    if end_byte <= start_byte {
        return;
    }
    tokens.push(SemanticToken {
        line: line_index,
        character: line[..start_byte].chars().count() as u32,
        length: line[start_byte..end_byte].chars().count() as u32,
        token_type,
        modifiers,
    });
}

fn semantic_string_end(line: &str, start_byte: usize) -> usize {
    let mut escaped = false;
    for (offset, ch) in line[start_byte + 1..].char_indices() {
        let byte = start_byte + 1 + offset + ch.len_utf8();
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return byte;
        }
    }
    line.len()
}

fn semantic_number_end(line: &str, start_byte: usize) -> usize {
    let mut seen_dot = false;
    let mut end = start_byte;
    for (offset, ch) in line[start_byte..].char_indices() {
        if ch.is_ascii_digit() {
            end = start_byte + offset + ch.len_utf8();
        } else if ch == '.' && !seen_dot {
            seen_dot = true;
            end = start_byte + offset + ch.len_utf8();
        } else {
            break;
        }
    }
    end
}

fn semantic_identifier_end(line: &str, start_byte: usize) -> usize {
    let mut end = start_byte;
    for (offset, ch) in line[start_byte..].char_indices() {
        if is_identifier_part(ch) {
            end = start_byte + offset + ch.len_utf8();
        } else {
            break;
        }
    }
    end
}

fn next_non_whitespace_byte(line: &str, start_byte: usize) -> Option<usize> {
    line.get(start_byte..)?
        .char_indices()
        .find_map(|(offset, ch)| (!ch.is_whitespace()).then_some(start_byte + offset))
}

fn previous_non_whitespace_byte(line: &str, start_byte: usize) -> Option<usize> {
    line.get(..start_byte)?
        .char_indices()
        .rev()
        .find_map(|(byte, ch)| (!ch.is_whitespace()).then_some(byte))
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_identifier_part(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn is_semantic_keyword(value: &str) -> bool {
    matches!(
        value,
        "module"
            | "import"
            | "as"
            | "export"
            | "type"
            | "effect"
            | "task"
            | "take"
            | "uses"
            | "bind"
            | "state"
            | "cell"
            | "knot"
            | "slot"
            | "gate"
            | "lease"
            | "veil"
            | "dial"
            | "flag"
            | "memo"
            | "cache"
            | "derive"
            | "flow"
            | "port"
            | "tally"
            | "hole"
            | "draft"
            | "taint"
            | "witness"
            | "seal"
            | "anchor"
            | "view"
            | "cursor"
            | "return"
            | "call"
            | "if"
            | "else"
            | "while"
            | "for"
            | "in"
            | "forge"
            | "set"
            | "true"
            | "false"
    )
}

fn is_semantic_operator(ch: char) -> bool {
    matches!(
        ch,
        '=' | '+' | '-' | '*' | '/' | '%' | '<' | '>' | '!' | '&' | '|' | '?' | '.'
    )
}

fn completion_items_for_program(
    current_program: &Program,
    analysis_program: &Program,
    project: Option<&ProjectGraph>,
) -> Vec<JsonValue> {
    let mut items = BTreeMap::new();
    for item in core_completion_items() {
        if let Some(label) = item.get("label").and_then(JsonValue::as_str) {
            items.insert(label.to_string(), item);
        }
    }
    for contract in host_effect_contracts() {
        push_completion_item(
            &mut items,
            completion_item(
                contract.callee,
                3,
                &format!("host call uses {}", contract.effects.join(", ")),
                contract.callee,
            ),
        );
    }
    if let Some(project) = project {
        for module in &project.modules {
            push_completion_item(
                &mut items,
                completion_item(&module.module, 9, "project module", &module.module),
            );
        }
    }
    for task in &analysis_program.tasks {
        if let Some((label, detail)) = task_completion(current_program, task, analysis_program) {
            push_completion_item(&mut items, completion_item(&label, 3, &detail, &label));
        }
    }
    for ty in &analysis_program.types {
        if let Some((label, detail)) =
            declaration_completion(current_program, ty.module.as_deref(), &ty.name, ty.exported)
        {
            push_completion_item(&mut items, completion_item(&label, 7, &detail, &label));
        }
    }
    for effect in &analysis_program.effects {
        if let Some((label, detail)) = declaration_completion(
            current_program,
            effect.module.as_deref(),
            &effect.name,
            effect.exported,
        ) {
            push_completion_item(&mut items, completion_item(&label, 14, &detail, &label));
        }
    }
    items.into_values().collect()
}

fn core_completion_items() -> Vec<JsonValue> {
    [
        "module", "import", "as", "export", "type", "effect", "task", "take", "uses", "bind",
        "set", "return", "call", "if", "else", "while", "for", "each", "forge", "true", "false",
        "Ok", "Err", "Result", "Text", "Int", "Float", "Bool", "Error", "Unit",
    ]
    .into_iter()
    .map(|label| completion_item(label, 14, "Sley keyword or built-in", label))
    .collect()
}

fn task_completion(
    current_program: &Program,
    task: &TaskDecl,
    analysis_program: &Program,
) -> Option<(String, String)> {
    let current_module = current_program.module_name();
    let task_module = task
        .module
        .as_deref()
        .unwrap_or_else(|| analysis_program.module_name());
    if task_module == current_module {
        return Some((
            task.name.clone(),
            format!(
                "task {task_module}.{} -> {}",
                task.name,
                task.return_type.display()
            ),
        ));
    }
    let import = visible_import_for_module(current_program, task_module)?;
    if !task.exported {
        return None;
    }
    let qualifier = import_qualifier(import);
    Some((
        format!("{qualifier}.{}", task.name),
        format!(
            "imported task {task_module}.{} -> {}",
            task.name,
            task.return_type.display()
        ),
    ))
}

fn declaration_completion(
    current_program: &Program,
    declaration_module: Option<&str>,
    name: &str,
    exported: bool,
) -> Option<(String, String)> {
    let module = declaration_module.unwrap_or_else(|| current_program.module_name());
    if module == current_program.module_name() {
        return Some((name.to_string(), format!("declaration {module}.{name}")));
    }
    let import = visible_import_for_module(current_program, module)?;
    if !exported {
        return None;
    }
    let qualifier = import_qualifier(import);
    Some((
        format!("{qualifier}.{name}"),
        format!("imported declaration {module}.{name}"),
    ))
}

fn visible_import_for_module<'a>(
    current_program: &'a Program,
    module: &str,
) -> Option<&'a ImportDecl> {
    current_program
        .imports
        .iter()
        .find(|import| import.module == module)
}

fn import_qualifier(import: &ImportDecl) -> String {
    import
        .alias
        .clone()
        .or_else(|| import.module.rsplit('.').next().map(str::to_string))
        .unwrap_or_else(|| import.module.clone())
}

fn completion_item(label: &str, kind: u8, detail: &str, insert_text: &str) -> JsonValue {
    json!({
        "label": label,
        "kind": kind,
        "detail": detail,
        "insertText": insert_text
    })
}

fn push_completion_item(items: &mut BTreeMap<String, JsonValue>, item: JsonValue) {
    if let Some(label) = item.get("label").and_then(JsonValue::as_str) {
        items.entry(label.to_string()).or_insert(item);
    }
}

fn completion_list(items: Vec<JsonValue>) -> JsonValue {
    json!({
        "isIncomplete": false,
        "items": items
    })
}

fn workspace_symbols_for_project(
    project: &ProjectGraph,
    state: &ServerState,
    query: &str,
) -> Vec<JsonValue> {
    let mut symbols = Vec::new();
    for module in &project.modules {
        let uri = file_uri_for_path(&module.path);
        let source = source_text_for_path(&module.path, state);
        let module_name = module.module.as_str();
        if symbol_matches(query, module_name) {
            symbols.push(workspace_symbol_json(
                module_name,
                2,
                None,
                &uri,
                first_line_range(&source),
            ));
        }
        symbols.extend(workspace_symbols_for_program(
            &uri,
            &source,
            &module.program,
            query,
        ));
    }
    symbols
}

fn workspace_symbols_for_document(
    uri: &str,
    source: &str,
    program: &Program,
    query: &str,
) -> Vec<JsonValue> {
    let mut symbols = Vec::new();
    let module_name = program.module_name();
    if symbol_matches(query, module_name) {
        symbols.push(workspace_symbol_json(
            module_name,
            2,
            None,
            uri,
            first_line_range(source),
        ));
    }
    symbols.extend(workspace_symbols_for_program(uri, source, program, query));
    symbols
}

fn workspace_symbols_for_program(
    uri: &str,
    source: &str,
    program: &Program,
    query: &str,
) -> Vec<JsonValue> {
    let module = program.module_name();
    let mut symbols = Vec::new();
    for ty in &program.types {
        let name = format!("{module}.{}", ty.name);
        if symbol_matches(query, &name) {
            symbols.push(workspace_symbol_json(
                &name,
                23,
                Some(module),
                uri,
                ty.span
                    .as_ref()
                    .map(|span| range_for_span(source, span))
                    .unwrap_or_else(|| first_line_range(source)),
            ));
        }
    }
    for effect in &program.effects {
        let name = format!("{module}.{}", effect.name);
        if symbol_matches(query, &name) {
            symbols.push(workspace_symbol_json(
                &name,
                24,
                Some(module),
                uri,
                effect
                    .span
                    .as_ref()
                    .map(|span| range_for_span(source, span))
                    .unwrap_or_else(|| first_line_range(source)),
            ));
        }
    }
    for task in &program.tasks {
        let task_module = task.module.as_deref().unwrap_or(module);
        let name = format!("{task_module}.{}", task.name);
        if symbol_matches(query, &name) {
            symbols.push(workspace_symbol_json(
                &name,
                12,
                Some(task_module),
                uri,
                task.span
                    .as_ref()
                    .map(|span| range_for_span(source, span))
                    .unwrap_or_else(|| first_line_range(source)),
            ));
        }
    }
    symbols
}

fn workspace_symbol_json(
    name: &str,
    kind: u8,
    container_name: Option<&str>,
    uri: &str,
    range: JsonValue,
) -> JsonValue {
    let mut symbol = json!({
        "name": name,
        "kind": kind,
        "location": location_json(uri, range)
    });
    if let Some(container_name) = container_name {
        symbol["containerName"] = json!(container_name);
    }
    symbol
}

fn workspace_symbol_key(symbol: &JsonValue) -> Option<String> {
    Some(format!(
        "{}:{}:{}",
        symbol.get("name")?.as_str()?,
        symbol.get("kind")?.as_u64()?,
        symbol.pointer("/location/uri")?.as_str()?
    ))
}

fn symbol_matches(query: &str, name: &str) -> bool {
    query.trim().is_empty() || name.to_lowercase().contains(&query.to_lowercase())
}

fn task_rename_target_at_position(
    current_program: &Program,
    analysis_program: &Program,
    source: &str,
    line: usize,
    character: usize,
) -> Option<(String, String, JsonValue)> {
    for task in &current_program.tasks {
        if let Some(range) = range_for_task_name(source, task)
            && position_in_range(&range, line, character)
        {
            return Some((task.id.clone(), task.name.clone(), range));
        }
    }
    let (caller_module, callee_name, range) =
        call_at_position(current_program, source, line, character)?;
    match resolve_task(analysis_program, &caller_module, &callee_name) {
        TaskResolution::Resolved { index, .. } => {
            let task = &analysis_program.tasks[index];
            Some((task.id.clone(), task.name.clone(), range))
        }
        TaskResolution::Unknown | TaskResolution::Ambiguous(_) | TaskResolution::Private(_) => None,
    }
}

fn reference_locations_for_task(
    target_task_id: &str,
    analysis_program: &Program,
    project: Option<&ProjectGraph>,
    current_uri: &str,
    current_source: &str,
    current_program: &Program,
    include_declaration: bool,
    state: &ServerState,
) -> Vec<JsonValue> {
    let mut locations = Vec::new();
    let mut seen = BTreeSet::new();
    if let Some(project) = project {
        for module in &project.modules {
            let uri = file_uri_for_path(&module.path);
            let source = source_text_for_path(&module.path, state);
            collect_task_references_for_program(
                target_task_id,
                analysis_program,
                &uri,
                &source,
                &module.program,
                include_declaration,
                &mut locations,
                &mut seen,
            );
        }
    } else {
        collect_task_references_for_program(
            target_task_id,
            analysis_program,
            current_uri,
            current_source,
            current_program,
            include_declaration,
            &mut locations,
            &mut seen,
        );
    }
    locations.sort_by_key(location_sort_key);
    locations
}

fn collect_task_references_for_program(
    target_task_id: &str,
    analysis_program: &Program,
    uri: &str,
    source: &str,
    program: &Program,
    include_declaration: bool,
    locations: &mut Vec<JsonValue>,
    seen: &mut BTreeSet<String>,
) {
    if include_declaration {
        if let Some(task) = program.tasks.iter().find(|task| task.id == target_task_id) {
            let range = range_for_task_name(source, task)
                .or_else(|| task.span.as_ref().map(|span| range_for_span(source, span)))
                .unwrap_or_else(|| first_line_range(source));
            push_reference_location(locations, seen, uri, range);
        }
    }
    for task in &program.tasks {
        let caller_module = task
            .module
            .clone()
            .unwrap_or_else(|| program.module_name().to_string());
        collect_task_reference_calls_from_statements(
            target_task_id,
            analysis_program,
            &caller_module,
            &task.body.statements,
            uri,
            source,
            locations,
            seen,
        );
    }
}

fn collect_task_reference_calls_from_statements(
    target_task_id: &str,
    analysis_program: &Program,
    caller_module: &str,
    statements: &[Statement],
    uri: &str,
    source: &str,
    locations: &mut Vec<JsonValue>,
    seen: &mut BTreeSet<String>,
) {
    for statement in statements {
        match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => collect_task_reference_calls_from_expr(
                target_task_id,
                analysis_program,
                caller_module,
                expr,
                uri,
                source,
                locations,
                seen,
            ),
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                collect_task_reference_calls_from_expr(
                    target_task_id,
                    analysis_program,
                    caller_module,
                    condition,
                    uri,
                    source,
                    locations,
                    seen,
                );
                collect_task_reference_calls_from_statements(
                    target_task_id,
                    analysis_program,
                    caller_module,
                    &then_block.statements,
                    uri,
                    source,
                    locations,
                    seen,
                );
                if let Some(else_block) = else_block {
                    collect_task_reference_calls_from_statements(
                        target_task_id,
                        analysis_program,
                        caller_module,
                        &else_block.statements,
                        uri,
                        source,
                        locations,
                        seen,
                    );
                }
            }
            StatementKind::While { condition, body } => {
                collect_task_reference_calls_from_expr(
                    target_task_id,
                    analysis_program,
                    caller_module,
                    condition,
                    uri,
                    source,
                    locations,
                    seen,
                );
                collect_task_reference_calls_from_statements(
                    target_task_id,
                    analysis_program,
                    caller_module,
                    &body.statements,
                    uri,
                    source,
                    locations,
                    seen,
                );
            }
            StatementKind::For {
                collection, body, ..
            } => {
                collect_task_reference_calls_from_expr(
                    target_task_id,
                    analysis_program,
                    caller_module,
                    collection,
                    uri,
                    source,
                    locations,
                    seen,
                );
                collect_task_reference_calls_from_statements(
                    target_task_id,
                    analysis_program,
                    caller_module,
                    &body.statements,
                    uri,
                    source,
                    locations,
                    seen,
                );
            }
            StatementKind::Forge { body } => collect_task_reference_calls_from_statements(
                target_task_id,
                analysis_program,
                caller_module,
                &body.statements,
                uri,
                source,
                locations,
                seen,
            ),
        }
    }
}

fn collect_task_reference_calls_from_expr(
    target_task_id: &str,
    analysis_program: &Program,
    caller_module: &str,
    expr: &Expr,
    uri: &str,
    source: &str,
    locations: &mut Vec<JsonValue>,
    seen: &mut BTreeSet<String>,
) {
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            if let Some(callee_name) = callee_path(callee)
                && let TaskResolution::Resolved { index, .. } =
                    resolve_task(analysis_program, caller_module, &callee_name)
                && analysis_program.tasks[index].id == target_task_id
            {
                let range = range_for_callee_leaf(
                    source,
                    expr,
                    callee,
                    &callee_name,
                    callee_leaf_name(&callee_name),
                )
                .or_else(|| expr.span.as_ref().map(|span| range_for_span(source, span)))
                .unwrap_or_else(|| first_line_range(source));
                push_reference_location(locations, seen, uri, range);
            }
            collect_task_reference_calls_from_expr(
                target_task_id,
                analysis_program,
                caller_module,
                callee,
                uri,
                source,
                locations,
                seen,
            );
            for arg in args {
                collect_task_reference_calls_from_expr(
                    target_task_id,
                    analysis_program,
                    caller_module,
                    arg,
                    uri,
                    source,
                    locations,
                    seen,
                );
            }
        }
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            collect_task_reference_calls_from_expr(
                target_task_id,
                analysis_program,
                caller_module,
                expr,
                uri,
                source,
                locations,
                seen,
            );
        }
        ExprKind::Binary { left, right, .. } => {
            collect_task_reference_calls_from_expr(
                target_task_id,
                analysis_program,
                caller_module,
                left,
                uri,
                source,
                locations,
                seen,
            );
            collect_task_reference_calls_from_expr(
                target_task_id,
                analysis_program,
                caller_module,
                right,
                uri,
                source,
                locations,
                seen,
            );
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_task_reference_calls_from_expr(
                target_task_id,
                analysis_program,
                caller_module,
                condition,
                uri,
                source,
                locations,
                seen,
            );
            collect_task_reference_calls_from_expr(
                target_task_id,
                analysis_program,
                caller_module,
                then_branch,
                uri,
                source,
                locations,
                seen,
            );
            collect_task_reference_calls_from_expr(
                target_task_id,
                analysis_program,
                caller_module,
                else_branch,
                uri,
                source,
                locations,
                seen,
            );
        }
        ExprKind::ListLiteral { items } => {
            for item in items {
                collect_task_reference_calls_from_expr(
                    target_task_id,
                    analysis_program,
                    caller_module,
                    item,
                    uri,
                    source,
                    locations,
                    seen,
                );
            }
        }
        ExprKind::MapLiteral { entries } => {
            for entry in entries {
                collect_task_reference_calls_from_expr(
                    target_task_id,
                    analysis_program,
                    caller_module,
                    &entry.key,
                    uri,
                    source,
                    locations,
                    seen,
                );
                collect_task_reference_calls_from_expr(
                    target_task_id,
                    analysis_program,
                    caller_module,
                    &entry.value,
                    uri,
                    source,
                    locations,
                    seen,
                );
            }
        }
        ExprKind::Index { collection, index } => {
            collect_task_reference_calls_from_expr(
                target_task_id,
                analysis_program,
                caller_module,
                collection,
                uri,
                source,
                locations,
                seen,
            );
            collect_task_reference_calls_from_expr(
                target_task_id,
                analysis_program,
                caller_module,
                index,
                uri,
                source,
                locations,
                seen,
            );
        }
        ExprKind::FieldAccess { receiver, .. } => collect_task_reference_calls_from_expr(
            target_task_id,
            analysis_program,
            caller_module,
            receiver,
            uri,
            source,
            locations,
            seen,
        ),
        ExprKind::RecordLiteral { fields, .. } => {
            for field in fields {
                collect_task_reference_calls_from_expr(
                    target_task_id,
                    analysis_program,
                    caller_module,
                    &field.expr,
                    uri,
                    source,
                    locations,
                    seen,
                );
            }
        }
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => {}
    }
}

fn push_reference_location(
    locations: &mut Vec<JsonValue>,
    seen: &mut BTreeSet<String>,
    uri: &str,
    range: JsonValue,
) {
    let location = location_json(uri, range);
    if let Some(key) = location_key(&location)
        && seen.insert(key)
    {
        locations.push(location);
    }
}

fn location_key(location: &JsonValue) -> Option<String> {
    Some(format!(
        "{}:{}:{}",
        location.get("uri")?.as_str()?,
        location.pointer("/range/start/line")?.as_u64()?,
        location.pointer("/range/start/character")?.as_u64()?
    ))
}

fn location_sort_key(location: &JsonValue) -> String {
    format!(
        "{}:{:08}:{:08}",
        location
            .get("uri")
            .and_then(JsonValue::as_str)
            .unwrap_or_default(),
        location
            .pointer("/range/start/line")
            .and_then(JsonValue::as_u64)
            .unwrap_or_default(),
        location
            .pointer("/range/start/character")
            .and_then(JsonValue::as_u64)
            .unwrap_or_default()
    )
}

fn rename_edits_for_task(
    target_task_id: &str,
    old_name: &str,
    new_name: &str,
    analysis_program: &Program,
    project: Option<&ProjectGraph>,
    current_uri: &str,
    current_source: &str,
    current_program: &Program,
    state: &ServerState,
) -> BTreeMap<String, Vec<JsonValue>> {
    let mut changes = BTreeMap::new();
    let mut seen = BTreeSet::new();
    if let Some(project) = project {
        for module in &project.modules {
            let uri = file_uri_for_path(&module.path);
            let source = source_text_for_path(&module.path, state);
            collect_task_rename_edits_for_program(
                target_task_id,
                old_name,
                new_name,
                analysis_program,
                &uri,
                &source,
                &module.program,
                &mut changes,
                &mut seen,
            );
        }
    } else {
        collect_task_rename_edits_for_program(
            target_task_id,
            old_name,
            new_name,
            analysis_program,
            current_uri,
            current_source,
            current_program,
            &mut changes,
            &mut seen,
        );
    }
    changes
}

fn collect_task_rename_edits_for_program(
    target_task_id: &str,
    old_name: &str,
    new_name: &str,
    analysis_program: &Program,
    uri: &str,
    source: &str,
    program: &Program,
    changes: &mut BTreeMap<String, Vec<JsonValue>>,
    seen: &mut BTreeSet<String>,
) {
    if let Some(task) = program.tasks.iter().find(|task| task.id == target_task_id)
        && let Some(range) = range_for_task_name(source, task)
    {
        push_rename_edit(changes, seen, uri, range, new_name);
    }
    for task in &program.tasks {
        let caller_module = task
            .module
            .clone()
            .unwrap_or_else(|| program.module_name().to_string());
        collect_task_rename_edits_from_statements(
            target_task_id,
            old_name,
            new_name,
            analysis_program,
            &caller_module,
            &task.body.statements,
            uri,
            source,
            changes,
            seen,
        );
    }
}

fn collect_task_rename_edits_from_statements(
    target_task_id: &str,
    old_name: &str,
    new_name: &str,
    analysis_program: &Program,
    caller_module: &str,
    statements: &[Statement],
    uri: &str,
    source: &str,
    changes: &mut BTreeMap<String, Vec<JsonValue>>,
    seen: &mut BTreeSet<String>,
) {
    for statement in statements {
        match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => collect_task_rename_edits_from_expr(
                target_task_id,
                old_name,
                new_name,
                analysis_program,
                caller_module,
                expr,
                uri,
                source,
                changes,
                seen,
            ),
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                collect_task_rename_edits_from_expr(
                    target_task_id,
                    old_name,
                    new_name,
                    analysis_program,
                    caller_module,
                    condition,
                    uri,
                    source,
                    changes,
                    seen,
                );
                collect_task_rename_edits_from_statements(
                    target_task_id,
                    old_name,
                    new_name,
                    analysis_program,
                    caller_module,
                    &then_block.statements,
                    uri,
                    source,
                    changes,
                    seen,
                );
                if let Some(else_block) = else_block {
                    collect_task_rename_edits_from_statements(
                        target_task_id,
                        old_name,
                        new_name,
                        analysis_program,
                        caller_module,
                        &else_block.statements,
                        uri,
                        source,
                        changes,
                        seen,
                    );
                }
            }
            StatementKind::While { condition, body } => {
                collect_task_rename_edits_from_expr(
                    target_task_id,
                    old_name,
                    new_name,
                    analysis_program,
                    caller_module,
                    condition,
                    uri,
                    source,
                    changes,
                    seen,
                );
                collect_task_rename_edits_from_statements(
                    target_task_id,
                    old_name,
                    new_name,
                    analysis_program,
                    caller_module,
                    &body.statements,
                    uri,
                    source,
                    changes,
                    seen,
                );
            }
            StatementKind::For {
                collection, body, ..
            } => {
                collect_task_rename_edits_from_expr(
                    target_task_id,
                    old_name,
                    new_name,
                    analysis_program,
                    caller_module,
                    collection,
                    uri,
                    source,
                    changes,
                    seen,
                );
                collect_task_rename_edits_from_statements(
                    target_task_id,
                    old_name,
                    new_name,
                    analysis_program,
                    caller_module,
                    &body.statements,
                    uri,
                    source,
                    changes,
                    seen,
                );
            }
            StatementKind::Forge { body } => collect_task_rename_edits_from_statements(
                target_task_id,
                old_name,
                new_name,
                analysis_program,
                caller_module,
                &body.statements,
                uri,
                source,
                changes,
                seen,
            ),
        }
    }
}

fn collect_task_rename_edits_from_expr(
    target_task_id: &str,
    old_name: &str,
    new_name: &str,
    analysis_program: &Program,
    caller_module: &str,
    expr: &Expr,
    uri: &str,
    source: &str,
    changes: &mut BTreeMap<String, Vec<JsonValue>>,
    seen: &mut BTreeSet<String>,
) {
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            if let Some(callee_name) = callee_path(callee)
                && let TaskResolution::Resolved { index, .. } =
                    resolve_task(analysis_program, caller_module, &callee_name)
                && analysis_program.tasks[index].id == target_task_id
                && let Some(range) =
                    range_for_callee_leaf(source, expr, callee, &callee_name, old_name)
            {
                push_rename_edit(changes, seen, uri, range, new_name);
            }
            collect_task_rename_edits_from_expr(
                target_task_id,
                old_name,
                new_name,
                analysis_program,
                caller_module,
                callee,
                uri,
                source,
                changes,
                seen,
            );
            for arg in args {
                collect_task_rename_edits_from_expr(
                    target_task_id,
                    old_name,
                    new_name,
                    analysis_program,
                    caller_module,
                    arg,
                    uri,
                    source,
                    changes,
                    seen,
                );
            }
        }
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            collect_task_rename_edits_from_expr(
                target_task_id,
                old_name,
                new_name,
                analysis_program,
                caller_module,
                expr,
                uri,
                source,
                changes,
                seen,
            );
        }
        ExprKind::Binary { left, right, .. } => {
            collect_task_rename_edits_from_expr(
                target_task_id,
                old_name,
                new_name,
                analysis_program,
                caller_module,
                left,
                uri,
                source,
                changes,
                seen,
            );
            collect_task_rename_edits_from_expr(
                target_task_id,
                old_name,
                new_name,
                analysis_program,
                caller_module,
                right,
                uri,
                source,
                changes,
                seen,
            );
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_task_rename_edits_from_expr(
                target_task_id,
                old_name,
                new_name,
                analysis_program,
                caller_module,
                condition,
                uri,
                source,
                changes,
                seen,
            );
            collect_task_rename_edits_from_expr(
                target_task_id,
                old_name,
                new_name,
                analysis_program,
                caller_module,
                then_branch,
                uri,
                source,
                changes,
                seen,
            );
            collect_task_rename_edits_from_expr(
                target_task_id,
                old_name,
                new_name,
                analysis_program,
                caller_module,
                else_branch,
                uri,
                source,
                changes,
                seen,
            );
        }
        ExprKind::ListLiteral { items } => {
            for item in items {
                collect_task_rename_edits_from_expr(
                    target_task_id,
                    old_name,
                    new_name,
                    analysis_program,
                    caller_module,
                    item,
                    uri,
                    source,
                    changes,
                    seen,
                );
            }
        }
        ExprKind::MapLiteral { entries } => {
            for entry in entries {
                collect_task_rename_edits_from_expr(
                    target_task_id,
                    old_name,
                    new_name,
                    analysis_program,
                    caller_module,
                    &entry.key,
                    uri,
                    source,
                    changes,
                    seen,
                );
                collect_task_rename_edits_from_expr(
                    target_task_id,
                    old_name,
                    new_name,
                    analysis_program,
                    caller_module,
                    &entry.value,
                    uri,
                    source,
                    changes,
                    seen,
                );
            }
        }
        ExprKind::Index { collection, index } => {
            collect_task_rename_edits_from_expr(
                target_task_id,
                old_name,
                new_name,
                analysis_program,
                caller_module,
                collection,
                uri,
                source,
                changes,
                seen,
            );
            collect_task_rename_edits_from_expr(
                target_task_id,
                old_name,
                new_name,
                analysis_program,
                caller_module,
                index,
                uri,
                source,
                changes,
                seen,
            );
        }
        ExprKind::FieldAccess { receiver, .. } => collect_task_rename_edits_from_expr(
            target_task_id,
            old_name,
            new_name,
            analysis_program,
            caller_module,
            receiver,
            uri,
            source,
            changes,
            seen,
        ),
        ExprKind::RecordLiteral { fields, .. } => {
            for field in fields {
                collect_task_rename_edits_from_expr(
                    target_task_id,
                    old_name,
                    new_name,
                    analysis_program,
                    caller_module,
                    &field.expr,
                    uri,
                    source,
                    changes,
                    seen,
                );
            }
        }
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => {}
    }
}

fn range_for_task_name(source: &str, task: &TaskDecl) -> Option<JsonValue> {
    let line = task.span.as_ref()?.line.saturating_sub(1);
    range_for_text_on_line(source, line, &task.name)
}

fn range_for_import_module(source: &str, import: &ImportDecl) -> Option<JsonValue> {
    let line = import.span.as_ref()?.line.saturating_sub(1);
    range_for_text_on_line(source, line, &import.module)
}

fn folding_range_for_braced_region(source: &str, span: &SourceSpan) -> Option<JsonValue> {
    let start_line = span.line.saturating_sub(1);
    let start_character = span.column.saturating_sub(1);
    let start_offset = source_offset_for_line_and_character(source, start_line, start_character)?;
    let mut depth = 0usize;
    let mut current_line = start_line;
    let mut open_line = None;
    let mut in_string = false;
    let mut escaped = false;

    for ch in source.get(start_offset..)?.chars() {
        if ch == '\n' {
            current_line += 1;
            continue;
        }
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    open_line = Some(current_line);
                }
                depth += 1;
            }
            '}' => {
                if depth == 1 {
                    let start_line = open_line?;
                    if current_line > start_line {
                        return Some(json!({
                            "startLine": start_line,
                            "endLine": current_line,
                            "kind": "region"
                        }));
                    }
                    return None;
                }
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
    }
    None
}

fn range_for_braced_span(source: &str, span: &SourceSpan) -> Option<JsonValue> {
    let folding_range = folding_range_for_braced_region(source, span)?;
    let end_line = folding_range
        .get("endLine")
        .and_then(JsonValue::as_u64)
        .map(|line| line as usize)?;
    Some(json!({
        "start": {
            "line": span.line.saturating_sub(1),
            "character": span.column.saturating_sub(1)
        },
        "end": {
            "line": end_line,
            "character": source_line_width(source, end_line)
        }
    }))
}

fn line_range_for_span(source: &str, span: &SourceSpan) -> JsonValue {
    let line = span.line.saturating_sub(1);
    json!({
        "start": {
            "line": line,
            "character": span.column.saturating_sub(1)
        },
        "end": {
            "line": line,
            "character": source_line_width(source, line)
        }
    })
}

fn range_for_callee_leaf(
    source: &str,
    expr: &Expr,
    callee: &Expr,
    callee_name: &str,
    old_name: &str,
) -> Option<JsonValue> {
    let span = leading_span_for_expr(callee).or_else(|| expr.span.as_ref())?;
    let line = span.line.saturating_sub(1);
    let line_text = source.lines().nth(line)?;
    let hint_start_byte = byte_offset_for_character(line_text, span.column.saturating_sub(1));
    if let Some(path_start_byte) = line_text
        .get(hint_start_byte..)
        .and_then(|text| text.find(callee_name))
        .map(|offset| hint_start_byte + offset)
    {
        let leaf_start_in_path = callee_name
            .rsplit_once('.')
            .map(|(prefix, _)| prefix.len() + 1)
            .unwrap_or(0);
        let leaf_start_byte = path_start_byte + leaf_start_in_path;
        let leaf_end_byte = leaf_start_byte + old_name.len();
        if line_text.get(leaf_start_byte..leaf_end_byte) == Some(old_name) {
            return Some(range_for_byte_offsets(
                line_text,
                line,
                leaf_start_byte,
                leaf_end_byte,
            ));
        }
    }
    range_for_text_on_line_after(source, line, old_name, hint_start_byte)
        .or_else(|| range_for_text_on_line(source, line, old_name))
}

fn range_for_text_on_line(source: &str, line: usize, needle: &str) -> Option<JsonValue> {
    let line_text = source.lines().nth(line)?;
    let start = line_text.find(needle)?;
    Some(range_for_byte_offsets(
        line_text,
        line,
        start,
        start + needle.len(),
    ))
}

fn range_for_text_on_line_after(
    source: &str,
    line: usize,
    needle: &str,
    start_byte: usize,
) -> Option<JsonValue> {
    let line_text = source.lines().nth(line)?;
    let relative_start = line_text.get(start_byte..)?.find(needle)?;
    let start = start_byte + relative_start;
    Some(range_for_byte_offsets(
        line_text,
        line,
        start,
        start + needle.len(),
    ))
}

fn range_for_byte_offsets(
    line_text: &str,
    line: usize,
    start_byte: usize,
    end_byte: usize,
) -> JsonValue {
    let start_character = line_text[..start_byte].chars().count();
    let end_character = line_text[..end_byte].chars().count();
    json!({
        "start": {
            "line": line,
            "character": start_character
        },
        "end": {
            "line": line,
            "character": end_character
        }
    })
}

fn position_in_range(range: &JsonValue, line: usize, character: usize) -> bool {
    let Some(start_line) = range
        .pointer("/start/line")
        .and_then(JsonValue::as_u64)
        .map(|value| value as usize)
    else {
        return false;
    };
    let Some(start_character) = range
        .pointer("/start/character")
        .and_then(JsonValue::as_u64)
        .map(|value| value as usize)
    else {
        return false;
    };
    let Some(end_line) = range
        .pointer("/end/line")
        .and_then(JsonValue::as_u64)
        .map(|value| value as usize)
    else {
        return false;
    };
    let Some(end_character) = range
        .pointer("/end/character")
        .and_then(JsonValue::as_u64)
        .map(|value| value as usize)
    else {
        return false;
    };
    if line < start_line || line > end_line {
        return false;
    }
    if line == start_line && character < start_character {
        return false;
    }
    if line == end_line && character > end_character {
        return false;
    }
    true
}

fn byte_offset_for_character(line_text: &str, character: usize) -> usize {
    line_text
        .char_indices()
        .nth(character)
        .map(|(byte, _)| byte)
        .unwrap_or_else(|| line_text.len())
}

fn source_offset_for_line_and_character(
    source: &str,
    line: usize,
    character: usize,
) -> Option<usize> {
    let line_start = source_line_start_byte(source, line)?;
    let line_text = source
        .get(line_start..)?
        .split_once('\n')
        .map(|(line_text, _)| line_text)
        .unwrap_or_else(|| &source[line_start..]);
    Some(line_start + byte_offset_for_character(line_text, character))
}

fn source_line_start_byte(source: &str, target_line: usize) -> Option<usize> {
    if target_line == 0 {
        return Some(0);
    }
    let mut line = 0usize;
    for (byte, ch) in source.char_indices() {
        if ch == '\n' {
            line += 1;
            if line == target_line {
                return Some(byte + 1);
            }
        }
    }
    None
}

fn source_line_width(source: &str, line: usize) -> usize {
    source
        .lines()
        .nth(line)
        .map(|line| line.chars().count())
        .unwrap_or_default()
}

fn leading_span_for_expr(expr: &Expr) -> Option<&SourceSpan> {
    if let Some(span) = expr.span.as_ref() {
        return Some(span);
    }
    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => leading_span_for_expr(expr),
        ExprKind::Binary { left, right, .. } => {
            leading_span_for_expr(left).or_else(|| leading_span_for_expr(right))
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => leading_span_for_expr(condition)
            .or_else(|| leading_span_for_expr(then_branch))
            .or_else(|| leading_span_for_expr(else_branch)),
        ExprKind::Call { callee, args } => {
            leading_span_for_expr(callee).or_else(|| args.iter().find_map(leading_span_for_expr))
        }
        ExprKind::ListLiteral { items } => items.iter().find_map(leading_span_for_expr),
        ExprKind::MapLiteral { entries } => entries.iter().find_map(|entry| {
            leading_span_for_expr(&entry.key).or_else(|| leading_span_for_expr(&entry.value))
        }),
        ExprKind::Index { collection, index } => {
            leading_span_for_expr(collection).or_else(|| leading_span_for_expr(index))
        }
        ExprKind::FieldAccess { receiver, .. } => leading_span_for_expr(receiver),
        ExprKind::RecordLiteral { fields, .. } => fields
            .iter()
            .find_map(|field| leading_span_for_expr(&field.expr)),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => None,
    }
}

fn callee_leaf_name(callee_name: &str) -> &str {
    callee_name
        .rsplit_once('.')
        .map(|(_, leaf)| leaf)
        .unwrap_or(callee_name)
}

fn push_rename_edit(
    changes: &mut BTreeMap<String, Vec<JsonValue>>,
    seen: &mut BTreeSet<String>,
    uri: &str,
    range: JsonValue,
    new_name: &str,
) {
    let edit = json!({
        "range": range,
        "newText": new_name
    });
    if let Some(key) = text_edit_key(uri, &edit)
        && seen.insert(key)
    {
        changes.entry(uri.to_string()).or_default().push(edit);
    }
}

fn text_edit_key(uri: &str, edit: &JsonValue) -> Option<String> {
    Some(format!(
        "{}:{}:{}",
        uri,
        edit.pointer("/range/start/line")?.as_u64()?,
        edit.pointer("/range/start/character")?.as_u64()?
    ))
}

fn is_sley_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn hover_json(value: &str, range: JsonValue) -> JsonValue {
    json!({
        "contents": {
            "kind": "markdown",
            "value": value
        },
        "range": range
    })
}

fn import_definition_at_line(
    program: &Program,
    line: usize,
    project: &ProjectGraph,
    state: &ServerState,
) -> Option<JsonValue> {
    let import = program
        .imports
        .iter()
        .find(|import| span_line_matches(import.span.as_ref(), line))?;
    let module = project
        .modules
        .iter()
        .find(|module| module.module == import.module)?;
    let source = source_text_for_path(&module.path, state);
    Some(location_json(
        &file_uri_for_path(&module.path),
        first_line_range(&source),
    ))
}

fn definition_location_for_task(
    uri: &str,
    current_text: &str,
    current_program: &Program,
    project: Option<&ProjectGraph>,
    task: &TaskDecl,
    state: &ServerState,
) -> Option<JsonValue> {
    if let Some(project) = project {
        for module in &project.modules {
            if let Some(target) = module
                .program
                .tasks
                .iter()
                .find(|candidate| candidate.id == task.id)
            {
                let source = source_text_for_path(&module.path, state);
                let range = target
                    .span
                    .as_ref()
                    .map(|span| range_for_span(&source, span))
                    .unwrap_or_else(|| first_line_range(&source));
                return Some(location_json(&file_uri_for_path(&module.path), range));
            }
        }
    }
    current_program
        .tasks
        .iter()
        .find(|candidate| candidate.id == task.id)
        .map(|target| {
            let range = target
                .span
                .as_ref()
                .map(|span| range_for_span(current_text, span))
                .unwrap_or_else(|| first_line_range(current_text));
            location_json(uri, range)
        })
}

fn call_at_line(program: &Program, line: usize) -> Option<(String, String)> {
    for task in &program.tasks {
        if let Some(callee_name) = call_in_statements_at_line(&task.body.statements, line) {
            let caller_module = task
                .module
                .clone()
                .unwrap_or_else(|| program.module_name().to_string());
            return Some((caller_module, callee_name));
        }
    }
    None
}

fn call_at_position(
    program: &Program,
    source: &str,
    line: usize,
    character: usize,
) -> Option<(String, String, JsonValue)> {
    for task in &program.tasks {
        if let Some((callee_name, range)) =
            call_in_statements_at_position(&task.body.statements, source, line, character)
        {
            let caller_module = task
                .module
                .clone()
                .unwrap_or_else(|| program.module_name().to_string());
            return Some((caller_module, callee_name, range));
        }
    }
    None
}

fn call_signature_at_position(
    program: &Program,
    source: &str,
    line: usize,
    character: usize,
) -> Option<(String, String, usize)> {
    for task in &program.tasks {
        if let Some((callee_name, active_parameter)) =
            call_signature_in_statements_at_position(&task.body.statements, source, line, character)
        {
            let caller_module = task
                .module
                .clone()
                .unwrap_or_else(|| program.module_name().to_string());
            return Some((caller_module, callee_name, active_parameter));
        }
    }
    None
}

fn call_in_statements_at_line(statements: &[Statement], line: usize) -> Option<String> {
    for statement in statements {
        match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                if let Some(callee_name) = call_in_expr_at_line(expr, line) {
                    return Some(callee_name);
                }
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                if let Some(callee_name) = call_in_expr_at_line(condition, line) {
                    return Some(callee_name);
                }
                if let Some(callee_name) = call_in_statements_at_line(&then_block.statements, line)
                {
                    return Some(callee_name);
                }
                if let Some(else_block) = else_block
                    && let Some(callee_name) =
                        call_in_statements_at_line(&else_block.statements, line)
                {
                    return Some(callee_name);
                }
            }
            StatementKind::While { condition, body } => {
                if let Some(callee_name) = call_in_expr_at_line(condition, line) {
                    return Some(callee_name);
                }
                if let Some(callee_name) = call_in_statements_at_line(&body.statements, line) {
                    return Some(callee_name);
                }
            }
            StatementKind::For {
                collection, body, ..
            } => {
                if let Some(callee_name) = call_in_expr_at_line(collection, line) {
                    return Some(callee_name);
                }
                if let Some(callee_name) = call_in_statements_at_line(&body.statements, line) {
                    return Some(callee_name);
                }
            }
            StatementKind::Forge { body } => {
                if let Some(callee_name) = call_in_statements_at_line(&body.statements, line) {
                    return Some(callee_name);
                }
            }
        }
    }
    None
}

fn call_in_statements_at_position(
    statements: &[Statement],
    source: &str,
    line: usize,
    character: usize,
) -> Option<(String, JsonValue)> {
    for statement in statements {
        match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                if let Some(call) = call_in_expr_at_position(expr, source, line, character) {
                    return Some(call);
                }
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                if let Some(call) = call_in_expr_at_position(condition, source, line, character) {
                    return Some(call);
                }
                if let Some(call) =
                    call_in_statements_at_position(&then_block.statements, source, line, character)
                {
                    return Some(call);
                }
                if let Some(else_block) = else_block
                    && let Some(call) = call_in_statements_at_position(
                        &else_block.statements,
                        source,
                        line,
                        character,
                    )
                {
                    return Some(call);
                }
            }
            StatementKind::While { condition, body } => {
                if let Some(call) = call_in_expr_at_position(condition, source, line, character) {
                    return Some(call);
                }
                if let Some(call) =
                    call_in_statements_at_position(&body.statements, source, line, character)
                {
                    return Some(call);
                }
            }
            StatementKind::For {
                collection, body, ..
            } => {
                if let Some(call) = call_in_expr_at_position(collection, source, line, character) {
                    return Some(call);
                }
                if let Some(call) =
                    call_in_statements_at_position(&body.statements, source, line, character)
                {
                    return Some(call);
                }
            }
            StatementKind::Forge { body } => {
                if let Some(call) =
                    call_in_statements_at_position(&body.statements, source, line, character)
                {
                    return Some(call);
                }
            }
        }
    }
    None
}

fn call_signature_in_statements_at_position(
    statements: &[Statement],
    source: &str,
    line: usize,
    character: usize,
) -> Option<(String, usize)> {
    for statement in statements {
        match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                if let Some(call) =
                    call_signature_in_expr_at_position(expr, source, line, character)
                {
                    return Some(call);
                }
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                if let Some(call) =
                    call_signature_in_expr_at_position(condition, source, line, character)
                {
                    return Some(call);
                }
                if let Some(call) = call_signature_in_statements_at_position(
                    &then_block.statements,
                    source,
                    line,
                    character,
                ) {
                    return Some(call);
                }
                if let Some(else_block) = else_block
                    && let Some(call) = call_signature_in_statements_at_position(
                        &else_block.statements,
                        source,
                        line,
                        character,
                    )
                {
                    return Some(call);
                }
            }
            StatementKind::While { condition, body } => {
                if let Some(call) =
                    call_signature_in_expr_at_position(condition, source, line, character)
                {
                    return Some(call);
                }
                if let Some(call) = call_signature_in_statements_at_position(
                    &body.statements,
                    source,
                    line,
                    character,
                ) {
                    return Some(call);
                }
            }
            StatementKind::For {
                collection, body, ..
            } => {
                if let Some(call) =
                    call_signature_in_expr_at_position(collection, source, line, character)
                {
                    return Some(call);
                }
                if let Some(call) = call_signature_in_statements_at_position(
                    &body.statements,
                    source,
                    line,
                    character,
                ) {
                    return Some(call);
                }
            }
            StatementKind::Forge { body } => {
                if let Some(call) = call_signature_in_statements_at_position(
                    &body.statements,
                    source,
                    line,
                    character,
                ) {
                    return Some(call);
                }
            }
        }
    }
    None
}

fn call_in_expr_at_line(expr: &Expr, line: usize) -> Option<String> {
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            if expr_contains_line(expr, line)
                && let Some(callee_name) = callee_path(callee)
            {
                return Some(callee_name);
            }
            args.iter()
                .find_map(|arg| call_in_expr_at_line(arg, line))
                .or_else(|| call_in_expr_at_line(callee, line))
        }
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => call_in_expr_at_line(expr, line),
        ExprKind::Binary { left, right, .. } => {
            call_in_expr_at_line(left, line).or_else(|| call_in_expr_at_line(right, line))
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => call_in_expr_at_line(condition, line)
            .or_else(|| call_in_expr_at_line(then_branch, line))
            .or_else(|| call_in_expr_at_line(else_branch, line)),
        ExprKind::ListLiteral { items } => items
            .iter()
            .find_map(|item| call_in_expr_at_line(item, line)),
        ExprKind::MapLiteral { entries } => entries.iter().find_map(|entry| {
            call_in_expr_at_line(&entry.key, line)
                .or_else(|| call_in_expr_at_line(&entry.value, line))
        }),
        ExprKind::Index { collection, index } => {
            call_in_expr_at_line(collection, line).or_else(|| call_in_expr_at_line(index, line))
        }
        ExprKind::FieldAccess { receiver, .. } => call_in_expr_at_line(receiver, line),
        ExprKind::RecordLiteral { fields, .. } => fields
            .iter()
            .find_map(|field| call_in_expr_at_line(&field.expr, line)),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => None,
    }
}

fn call_in_expr_at_position(
    expr: &Expr,
    source: &str,
    line: usize,
    character: usize,
) -> Option<(String, JsonValue)> {
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            if let Some(callee_name) = callee_path(callee) {
                let leaf_name = callee_leaf_name(&callee_name);
                if let Some(range) =
                    range_for_callee_leaf(source, expr, callee, &callee_name, leaf_name)
                    && position_in_range(&range, line, character)
                {
                    return Some((callee_name, range));
                }
            }
            args.iter()
                .find_map(|arg| call_in_expr_at_position(arg, source, line, character))
                .or_else(|| call_in_expr_at_position(callee, source, line, character))
        }
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            call_in_expr_at_position(expr, source, line, character)
        }
        ExprKind::Binary { left, right, .. } => {
            call_in_expr_at_position(left, source, line, character)
                .or_else(|| call_in_expr_at_position(right, source, line, character))
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => call_in_expr_at_position(condition, source, line, character)
            .or_else(|| call_in_expr_at_position(then_branch, source, line, character))
            .or_else(|| call_in_expr_at_position(else_branch, source, line, character)),
        ExprKind::ListLiteral { items } => items
            .iter()
            .find_map(|item| call_in_expr_at_position(item, source, line, character)),
        ExprKind::MapLiteral { entries } => entries.iter().find_map(|entry| {
            call_in_expr_at_position(&entry.key, source, line, character)
                .or_else(|| call_in_expr_at_position(&entry.value, source, line, character))
        }),
        ExprKind::Index { collection, index } => {
            call_in_expr_at_position(collection, source, line, character)
                .or_else(|| call_in_expr_at_position(index, source, line, character))
        }
        ExprKind::FieldAccess { receiver, .. } => {
            call_in_expr_at_position(receiver, source, line, character)
        }
        ExprKind::RecordLiteral { fields, .. } => fields
            .iter()
            .find_map(|field| call_in_expr_at_position(&field.expr, source, line, character)),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => None,
    }
}

fn call_signature_in_expr_at_position(
    expr: &Expr,
    source: &str,
    line: usize,
    character: usize,
) -> Option<(String, usize)> {
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            if let Some(call) = call_signature_in_expr_at_position(callee, source, line, character)
            {
                return Some(call);
            }
            for arg in args {
                if let Some(call) = call_signature_in_expr_at_position(arg, source, line, character)
                {
                    return Some(call);
                }
            }
            let callee_name = callee_path(callee)?;
            let active_parameter =
                active_parameter_for_call_at_position(source, expr, line, character)?;
            Some((callee_name, active_parameter))
        }
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            call_signature_in_expr_at_position(expr, source, line, character)
        }
        ExprKind::Binary { left, right, .. } => {
            call_signature_in_expr_at_position(left, source, line, character)
                .or_else(|| call_signature_in_expr_at_position(right, source, line, character))
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => call_signature_in_expr_at_position(condition, source, line, character)
            .or_else(|| call_signature_in_expr_at_position(then_branch, source, line, character))
            .or_else(|| call_signature_in_expr_at_position(else_branch, source, line, character)),
        ExprKind::ListLiteral { items } => items
            .iter()
            .find_map(|item| call_signature_in_expr_at_position(item, source, line, character)),
        ExprKind::MapLiteral { entries } => entries.iter().find_map(|entry| {
            call_signature_in_expr_at_position(&entry.key, source, line, character).or_else(|| {
                call_signature_in_expr_at_position(&entry.value, source, line, character)
            })
        }),
        ExprKind::Index { collection, index } => {
            call_signature_in_expr_at_position(collection, source, line, character)
                .or_else(|| call_signature_in_expr_at_position(index, source, line, character))
        }
        ExprKind::FieldAccess { receiver, .. } => {
            call_signature_in_expr_at_position(receiver, source, line, character)
        }
        ExprKind::RecordLiteral { fields, .. } => fields.iter().find_map(|field| {
            call_signature_in_expr_at_position(&field.expr, source, line, character)
        }),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => None,
    }
}

fn active_parameter_for_call_at_position(
    source: &str,
    expr: &Expr,
    line: usize,
    character: usize,
) -> Option<usize> {
    let (line_text, expression_start_byte, _) =
        expr_source_offsets_at_position(source, expr, line, character)?;
    let cursor_byte = byte_offset_for_character(line_text, character);
    active_parameter_for_call_source(&expr.source, cursor_byte - expression_start_byte)
}

fn range_for_expr_source_at_position(
    source: &str,
    expr: &Expr,
    line: usize,
    character: usize,
) -> Option<JsonValue> {
    let (line_text, expression_start_byte, expression_end_byte) =
        expr_source_offsets_at_position(source, expr, line, character)?;
    Some(range_for_byte_offsets(
        line_text,
        line,
        expression_start_byte,
        expression_end_byte,
    ))
}

fn expr_source_offsets_at_position<'a>(
    source: &'a str,
    expr: &Expr,
    line: usize,
    character: usize,
) -> Option<(&'a str, usize, usize)> {
    if expr.source.contains('\n') {
        return None;
    }
    let line_text = source.lines().nth(line)?;
    let span = leading_span_for_expr(expr)?;
    if span.line.saturating_sub(1) != line {
        return None;
    }
    let cursor_byte = byte_offset_for_character(line_text, character);
    let mut search_start_byte = 0usize;
    while search_start_byte <= line_text.len() {
        let Some(search_text) = line_text.get(search_start_byte..) else {
            break;
        };
        let Some(relative_start_byte) = search_text.find(&expr.source) else {
            break;
        };
        let expression_start_byte = search_start_byte + relative_start_byte;
        let expression_end_byte = expression_start_byte + expr.source.len();
        if cursor_byte >= expression_start_byte && cursor_byte <= expression_end_byte {
            return Some((line_text, expression_start_byte, expression_end_byte));
        }
        search_start_byte = expression_end_byte.max(expression_start_byte + 1);
    }
    None
}

fn active_parameter_for_call_source(call_source: &str, cursor_byte: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut saw_arg_list = false;
    let mut active_parameter = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (byte, ch) in call_source.char_indices() {
        if byte >= cursor_byte {
            break;
        }
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '(' => {
                depth += 1;
                if depth == 1 {
                    saw_arg_list = true;
                }
            }
            ')' => {
                if depth == 1 {
                    return None;
                }
                depth = depth.saturating_sub(1);
            }
            ',' if saw_arg_list && depth == 1 => {
                active_parameter += 1;
            }
            _ => {}
        }
    }

    saw_arg_list.then_some(active_parameter)
}

fn expr_contains_line(expr: &Expr, line: usize) -> bool {
    span_line_matches(expr.span.as_ref(), line)
        || match &expr.kind {
            ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => expr_contains_line(expr, line),
            ExprKind::Binary { left, right, .. } => {
                expr_contains_line(left, line) || expr_contains_line(right, line)
            }
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                expr_contains_line(condition, line)
                    || expr_contains_line(then_branch, line)
                    || expr_contains_line(else_branch, line)
            }
            ExprKind::Call { callee, args } => {
                expr_contains_line(callee, line)
                    || args.iter().any(|arg| expr_contains_line(arg, line))
            }
            ExprKind::ListLiteral { items } => {
                items.iter().any(|item| expr_contains_line(item, line))
            }
            ExprKind::MapLiteral { entries } => entries.iter().any(|entry| {
                expr_contains_line(&entry.key, line) || expr_contains_line(&entry.value, line)
            }),
            ExprKind::Index { collection, index } => {
                expr_contains_line(collection, line) || expr_contains_line(index, line)
            }
            ExprKind::FieldAccess { receiver, .. } => expr_contains_line(receiver, line),
            ExprKind::RecordLiteral { fields, .. } => fields
                .iter()
                .any(|field| expr_contains_line(&field.expr, line)),
            ExprKind::Raw { .. }
            | ExprKind::StringLiteral { .. }
            | ExprKind::IntLiteral { .. }
            | ExprKind::FloatLiteral { .. }
            | ExprKind::BoolLiteral { .. }
            | ExprKind::Identifier { .. } => false,
        }
}

fn location_json(uri: &str, range: JsonValue) -> JsonValue {
    json!({
        "uri": uri,
        "range": range
    })
}

fn span_line_matches(span: Option<&SourceSpan>, line: usize) -> bool {
    span.map(|span| span.line.saturating_sub(1) == line)
        .unwrap_or(false)
}

fn code_action_for_graft_template(
    uri: &str,
    target: &str,
    template: &EditPlanGraftTemplate,
) -> JsonValue {
    let preview = json!({
        "schema": "sley.lsp.code_action.v0",
        "uri": uri,
        "target": target,
        "kind": template.kind,
        "surface": template.surface,
        "operation": template.operation,
        "editableJsonPointers": template.editable_json_pointers,
        "dryRun": {
            "command": "sley",
            "args": [
                "fix",
                "--json",
                "--dry-run",
                "--kind",
                template.kind,
                "--template-surface",
                template.surface,
                target
            ]
        }
    });
    json!({
        "title": format!("Sley: preview {}", human_action_name(&template.kind)),
        "kind": code_action_kind(&template.kind),
        "command": {
            "title": format!("Preview Sley {}", template.kind),
            "command": FIX_PREVIEW_COMMAND,
            "arguments": [preview]
        },
        "data": preview
    })
}

fn code_action_for_transaction_template(
    uri: &str,
    target: &str,
    template: &EditPlanTransactionTemplate,
) -> JsonValue {
    let preview = json!({
        "schema": "sley.lsp.code_action.v0",
        "uri": uri,
        "target": target,
        "kind": template.kind,
        "surface": template.surface,
        "transaction": template.transaction,
        "editableJsonPointers": template.editable_json_pointers,
        "dryRun": {
            "command": "sley",
            "args": [
                "fix",
                "--json",
                "--dry-run",
                "--kind",
                template.kind,
                "--template-surface",
                template.surface,
                target
            ]
        }
    });
    json!({
        "title": format!("Sley: preview {}", human_action_name(&template.kind)),
        "kind": code_action_kind(&template.kind),
        "command": {
            "title": format!("Preview Sley {}", template.kind),
            "command": FIX_PREVIEW_COMMAND,
            "arguments": [preview]
        },
        "data": preview
    })
}

fn human_action_name(kind: &str) -> String {
    kind.replace('_', " ")
}

fn code_action_kind(kind: &str) -> &'static str {
    if kind.starts_with("delete_")
        || kind.starts_with("remove_")
        || kind.starts_with("simplify_")
        || kind.starts_with("qualify_")
        || kind.starts_with("wrap_")
        || kind.starts_with("replace_raw_")
        || kind.starts_with("convert_")
    {
        "quickfix"
    } else {
        "refactor.rewrite"
    }
}

fn range_for_code_action_surface(program: &Program, source: &str, node: &str) -> Option<JsonValue> {
    if let Some(import) = program.imports.iter().find(|import| import.id == node) {
        return import
            .span
            .as_ref()
            .map(|span| line_range_for_span(source, span));
    }
    if let Some(ty) = program.types.iter().find(|ty| ty.id == node) {
        return ty
            .span
            .as_ref()
            .map(|span| line_range_for_span(source, span));
    }
    if let Some(effect) = program.effects.iter().find(|effect| effect.id == node) {
        return effect
            .span
            .as_ref()
            .map(|span| line_range_for_span(source, span));
    }
    for task in &program.tasks {
        if task.id == node {
            return task
                .span
                .as_ref()
                .map(|span| line_range_for_span(source, span));
        }
        if let Some(take) = task.takes.iter().find(|take| take.id == node) {
            return take
                .span
                .as_ref()
                .map(|span| line_range_for_span(source, span));
        }
        if let Some(range) = range_for_statement_node(source, &task.body.statements, node) {
            return Some(range);
        }
    }
    None
}

fn range_for_statement_node(
    source: &str,
    statements: &[Statement],
    node: &str,
) -> Option<JsonValue> {
    for statement in statements {
        if statement.id == node {
            return statement
                .span
                .as_ref()
                .map(|span| line_range_for_span(source, span));
        }
        match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                if let Some(range) = range_for_expr_node(source, expr, node) {
                    return Some(range);
                }
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                if let Some(range) = range_for_expr_node(source, condition, node) {
                    return Some(range);
                }
                if let Some(range) = range_for_statement_node(source, &then_block.statements, node)
                {
                    return Some(range);
                }
                if let Some(else_block) = else_block {
                    if let Some(range) =
                        range_for_statement_node(source, &else_block.statements, node)
                    {
                        return Some(range);
                    }
                }
            }
            StatementKind::While { condition, body } => {
                if let Some(range) = range_for_expr_node(source, condition, node) {
                    return Some(range);
                }
                if let Some(range) = range_for_statement_node(source, &body.statements, node) {
                    return Some(range);
                }
            }
            StatementKind::For {
                collection, body, ..
            } => {
                if let Some(range) = range_for_expr_node(source, collection, node) {
                    return Some(range);
                }
                if let Some(range) = range_for_statement_node(source, &body.statements, node) {
                    return Some(range);
                }
            }
            StatementKind::Forge { body } => {
                if let Some(range) = range_for_statement_node(source, &body.statements, node) {
                    return Some(range);
                }
            }
        }
    }
    None
}

fn range_for_expr_node(source: &str, expr: &Expr, node: &str) -> Option<JsonValue> {
    if expr.id == node {
        return expr_source_range(source, expr).or_else(|| {
            expr.span
                .as_ref()
                .map(|span| line_range_for_span(source, span))
        });
    }
    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            range_for_expr_node(source, expr, node)
        }
        ExprKind::Binary { left, right, .. } => range_for_expr_node(source, left, node)
            .or_else(|| range_for_expr_node(source, right, node)),
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => range_for_expr_node(source, condition, node)
            .or_else(|| range_for_expr_node(source, then_branch, node))
            .or_else(|| range_for_expr_node(source, else_branch, node)),
        ExprKind::Call { callee, args } => {
            range_for_expr_node(source, callee, node).or_else(|| {
                args.iter()
                    .find_map(|arg| range_for_expr_node(source, arg, node))
            })
        }
        ExprKind::ListLiteral { items } => items
            .iter()
            .find_map(|item| range_for_expr_node(source, item, node)),
        ExprKind::MapLiteral { entries } => entries.iter().find_map(|entry| {
            range_for_expr_node(source, &entry.key, node)
                .or_else(|| range_for_expr_node(source, &entry.value, node))
        }),
        ExprKind::Index { collection, index } => range_for_expr_node(source, collection, node)
            .or_else(|| range_for_expr_node(source, index, node)),
        ExprKind::FieldAccess { receiver, .. } => range_for_expr_node(source, receiver, node),
        ExprKind::RecordLiteral { fields, .. } => fields
            .iter()
            .find_map(|field| range_for_expr_node(source, &field.expr, node)),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => None,
    }
}

fn expr_source_range(source: &str, expr: &Expr) -> Option<JsonValue> {
    if expr.source.contains('\n') {
        return None;
    }
    let span = leading_span_for_expr(expr)?;
    let line = span.line.saturating_sub(1);
    let line_text = source.lines().nth(line)?;
    let hint_start_byte = byte_offset_for_character(line_text, span.column.saturating_sub(1));
    range_for_text_on_line_after(source, line, &expr.source, hint_start_byte)
        .or_else(|| range_for_text_on_line(source, line, &expr.source))
}

fn span_for_node(program: &Program, node: &str) -> Option<SourceSpan> {
    if let Some(import) = program.imports.iter().find(|import| import.id == node) {
        return import.span.clone();
    }
    if let Some(ty) = program.types.iter().find(|ty| ty.id == node) {
        return ty.span.clone();
    }
    if let Some(effect) = program.effects.iter().find(|effect| effect.id == node) {
        return effect.span.clone();
    }
    for task in &program.tasks {
        if task.id == node {
            return task.span.clone();
        }
        if let Some(take) = task.takes.iter().find(|take| take.id == node) {
            return take.span.clone();
        }
        if let Some(span) = span_for_statement_node(&task.body.statements, node) {
            return Some(span);
        }
    }
    None
}

fn span_for_statement_node(statements: &[Statement], node: &str) -> Option<SourceSpan> {
    for statement in statements {
        if statement.id == node {
            return statement.span.clone();
        }
        match &statement.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                if let Some(span) = span_for_expr_node(expr, node) {
                    return Some(span);
                }
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                if let Some(span) = span_for_expr_node(condition, node) {
                    return Some(span);
                }
                if let Some(span) = span_for_statement_node(&then_block.statements, node) {
                    return Some(span);
                }
                if let Some(else_block) = else_block {
                    if let Some(span) = span_for_statement_node(&else_block.statements, node) {
                        return Some(span);
                    }
                }
            }
            StatementKind::While { condition, body } => {
                if let Some(span) = span_for_expr_node(condition, node) {
                    return Some(span);
                }
                if let Some(span) = span_for_statement_node(&body.statements, node) {
                    return Some(span);
                }
            }
            StatementKind::For {
                collection, body, ..
            } => {
                if let Some(span) = span_for_expr_node(collection, node) {
                    return Some(span);
                }
                if let Some(span) = span_for_statement_node(&body.statements, node) {
                    return Some(span);
                }
            }
            StatementKind::Forge { body } => {
                if let Some(span) = span_for_statement_node(&body.statements, node) {
                    return Some(span);
                }
            }
        }
    }
    None
}

fn span_for_expr_node(expr: &Expr, node: &str) -> Option<SourceSpan> {
    if expr.id == node {
        return expr.span.clone();
    }
    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => span_for_expr_node(expr, node),
        ExprKind::Binary { left, right, .. } => {
            span_for_expr_node(left, node).or_else(|| span_for_expr_node(right, node))
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => span_for_expr_node(condition, node)
            .or_else(|| span_for_expr_node(then_branch, node))
            .or_else(|| span_for_expr_node(else_branch, node)),
        ExprKind::Call { callee, args } => span_for_expr_node(callee, node)
            .or_else(|| args.iter().find_map(|arg| span_for_expr_node(arg, node))),
        ExprKind::ListLiteral { items } => {
            items.iter().find_map(|item| span_for_expr_node(item, node))
        }
        ExprKind::MapLiteral { entries } => entries.iter().find_map(|entry| {
            span_for_expr_node(&entry.key, node).or_else(|| span_for_expr_node(&entry.value, node))
        }),
        ExprKind::Index { collection, index } => {
            span_for_expr_node(collection, node).or_else(|| span_for_expr_node(index, node))
        }
        ExprKind::FieldAccess { receiver, .. } => span_for_expr_node(receiver, node),
        ExprKind::RecordLiteral { fields, .. } => fields
            .iter()
            .find_map(|field| span_for_expr_node(&field.expr, node)),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => None,
    }
}

fn range_for_span(text: &str, span: &SourceSpan) -> JsonValue {
    let line = span.line.saturating_sub(1);
    let character = span.column.saturating_sub(1);
    let line_width = text
        .lines()
        .nth(line)
        .map(|line| line.chars().count())
        .unwrap_or(character + 1);
    let end_character = (character + 1).min(line_width.max(character + 1));
    json!({
        "start": {
            "line": line,
            "character": character
        },
        "end": {
            "line": line,
            "character": end_character
        }
    })
}

fn full_document_range(text: &str) -> JsonValue {
    let lines = text.split('\n').collect::<Vec<_>>();
    let end_line = lines.len().saturating_sub(1);
    let last_line_width = lines
        .last()
        .map(|line| line.chars().count())
        .unwrap_or_default();
    json!({
        "start": {
            "line": 0,
            "character": 0
        },
        "end": {
            "line": end_line,
            "character": last_line_width
        }
    })
}

fn first_line_range(text: &str) -> JsonValue {
    let line_width = text
        .lines()
        .next()
        .map(|line| line.chars().count())
        .unwrap_or(0);
    json!({
        "start": {
            "line": 0,
            "character": 0
        },
        "end": {
            "line": 0,
            "character": line_width
        }
    })
}

fn text_document_uri(params: &JsonValue) -> Option<String> {
    params
        .get("textDocument")
        .and_then(|text_document| text_document.get("uri"))
        .and_then(JsonValue::as_str)
        .map(str::to_string)
}

fn target_for_uri(uri: &str) -> String {
    uri_to_path(uri)
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| uri.to_string())
}

fn module_name_hint_for_uri(uri: &str) -> Option<String> {
    uri_to_path(uri).and_then(|path| module_name_from_sley_path(&path))
}

fn project_root_for_uri(uri: &str) -> Option<PathBuf> {
    let path = uri_to_path(uri)?;
    let mut directory = if path.is_dir() {
        Some(path.as_path())
    } else {
        path.parent()
    };
    while let Some(current) = directory {
        if current.join("sley.toml").exists() {
            return Some(current.to_path_buf());
        }
        directory = current.parent();
    }
    None
}

fn source_text_for_path(path: &Path, state: &ServerState) -> String {
    state
        .documents
        .iter()
        .find_map(|(uri, document)| {
            uri_to_path(uri)
                .filter(|candidate| candidate == path)
                .map(|_| document.text.clone())
        })
        .or_else(|| fs::read_to_string(path).ok())
        .unwrap_or_default()
}

fn file_uri_for_path(path: &Path) -> String {
    format!("file://{}", path.display())
}

fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let path = uri.strip_prefix("file://")?;
    let decoded = percent_decode(path);
    Some(PathBuf::from(decoded))
}

fn percent_decode(value: &str) -> String {
    let mut decoded = String::new();
    let mut bytes = value.as_bytes().iter().copied().peekable();
    while let Some(byte) = bytes.next() {
        if byte == b'%' {
            let hi = bytes.next();
            let lo = bytes.next();
            if let (Some(hi), Some(lo)) = (hi, lo) {
                if let Ok(value) = u8::from_str_radix(&String::from_utf8_lossy(&[hi, lo]), 16) {
                    decoded.push(value as char);
                    continue;
                }
                decoded.push('%');
                decoded.push(hi as char);
                decoded.push(lo as char);
                continue;
            }
            decoded.push('%');
            if let Some(hi) = hi {
                decoded.push(hi as char);
            }
            if let Some(lo) = lo {
                decoded.push(lo as char);
            }
        } else {
            decoded.push(byte as char);
        }
    }
    decoded
}

fn send_response<W: Write>(writer: &mut W, id: JsonValue, result: JsonValue) -> Result<()> {
    send_json(
        writer,
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        }),
    )
}

fn send_error<W: Write>(writer: &mut W, id: JsonValue, code: i64, message: String) -> Result<()> {
    send_json(
        writer,
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": code,
                "message": message
            }
        }),
    )
}

fn send_notification<W: Write>(writer: &mut W, method: &str, params: JsonValue) -> Result<()> {
    send_json(
        writer,
        json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }),
    )
}

fn send_json<W: Write>(writer: &mut W, value: JsonValue) -> Result<()> {
    let body = serde_json::to_vec(&value)?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
    writer.write_all(&body)?;
    writer.flush()?;
    Ok(())
}
