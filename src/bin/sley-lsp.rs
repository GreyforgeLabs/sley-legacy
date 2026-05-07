use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use serde_json::{Value as JsonValue, json};
use sley::ast::{
    EffectDecl, Expr, ExprKind, ImportDecl, Program, Statement, StatementKind, TaskDecl, TypeDecl,
};
use sley::authority::host_effect_contracts;
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
        "textDocument/completion" => {
            if let Some(id) = id {
                send_response(writer, id, handle_completion(params, state))?;
            }
        }
        "workspace/symbol" => {
            if let Some(id) = id {
                send_response(writer, id, handle_workspace_symbol(params, state))?;
            }
        }
        "textDocument/codeAction" => {
            if let Some(id) = id {
                send_response(writer, id, handle_code_actions(params, state))?;
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
            "hoverProvider": true,
            "definitionProvider": true,
            "completionProvider": {
                "resolveProvider": false,
                "triggerCharacters": [".", " ", ":", ">"]
            },
            "workspaceSymbolProvider": true,
            "codeActionProvider": {
                "resolveProvider": false,
                "codeActionKinds": ["quickfix", "refactor.rewrite"]
            },
            "executeCommandProvider": {
                "commands": [FIX_PREVIEW_COMMAND]
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
        .insert(uri.to_string(), DocumentState { text: text.clone() });
    publish_diagnostics(writer, uri, &text, version, state)
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
        },
    );
    publish_diagnostics(writer, uri, text, version, state)
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
    )
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
    let line = position
        .get("line")
        .and_then(JsonValue::as_u64)
        .unwrap_or_default() as usize;
    let Ok(program) = parse_program(&document.text) else {
        return JsonValue::Null;
    };
    hover_for_line(&program, &document.text, line).unwrap_or(JsonValue::Null)
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

fn handle_code_actions(params: JsonValue, state: &ServerState) -> JsonValue {
    let Some(uri) = text_document_uri(&params) else {
        return json!([]);
    };
    let Some(document) = state.documents.get(&uri) else {
        return json!([]);
    };
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
            .map(|template| code_action_for_graft_template(&uri, &report.target, template)),
    );
    actions.extend(
        report
            .transaction_templates
            .iter()
            .map(|template| code_action_for_transaction_template(&uri, &report.target, template)),
    );
    actions.truncate(50);
    json!(actions)
}

fn handle_execute_command(params: JsonValue) -> JsonValue {
    let command = params
        .get("command")
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    if command != FIX_PREVIEW_COMMAND {
        return JsonValue::Null;
    }
    let preview = params
        .get("arguments")
        .and_then(JsonValue::as_array)
        .and_then(|arguments| arguments.first())
        .cloned()
        .unwrap_or(JsonValue::Null);
    json!({
        "schema": FIX_PREVIEW_SCHEMA,
        "status": "preview",
        "preview": preview
    })
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
            "args": ["fix", "--json", "--dry-run", "--kind", template.kind, target]
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
            "args": ["fix", "--json", "--dry-run", "--kind", template.kind, target]
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
