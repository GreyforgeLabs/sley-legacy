use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use serde_json::{Value as JsonValue, json};
use sley::ast::{Expr, ExprKind, Program, Statement, StatementKind};
use sley::checker::{check_program, has_errors};
use sley::diagnostics::{Diagnostic, Severity, SourceSpan};
use sley::formatter::format_program;
use sley::lint::{LintFinding, LintOptions, build_lint_report};
use sley::parser::parse_program;
use sley::plan::{
    EditPlanGraftTemplate, EditPlanOptions, EditPlanTransactionTemplate,
    build_edit_plan_report_with_options, module_name_from_sley_path,
};

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
    publish_diagnostics(writer, uri, &text, version)
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
    publish_diagnostics(writer, uri, text, version)
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
) -> Result<()> {
    let mut diagnostics = Vec::new();
    match parse_program(text) {
        Ok(program) => {
            let compiler_diagnostics = check_program(&program);
            let compiler_has_errors = has_errors(&compiler_diagnostics);
            diagnostics.extend(
                compiler_diagnostics
                    .iter()
                    .map(|diagnostic| compiler_diagnostic_to_lsp(text, diagnostic)),
            );
            if !compiler_has_errors {
                let lint_report = build_lint_report(&program, LintOptions::default());
                diagnostics.extend(
                    lint_report
                        .findings
                        .iter()
                        .map(|finding| lint_finding_to_lsp(text, &program, finding)),
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
            return Some(hover_json(
                &format!("module `{module}`"),
                first_line_range(text),
            ));
        }
    }
    for import in &program.imports {
        if span_line_matches(import.span.as_ref(), line) {
            return Some(hover_json(
                &format!("import `{}`", import.module),
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
                &format!("type `{}`", ty.name),
                ty.span.as_ref().map(|span| range_for_span(text, span))?,
            ));
        }
    }
    for effect in &program.effects {
        if span_line_matches(effect.span.as_ref(), line) {
            return Some(hover_json(
                &format!("effect `{}`", effect.name),
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
                &format!("task `{}`", task.name),
                task.span.as_ref().map(|span| range_for_span(text, span))?,
            ));
        }
    }
    None
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
