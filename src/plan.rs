use serde::Serialize;
use serde_json::{Value as JsonValue, json};

use crate::Program;
use crate::ast::{BindingKind, Block, Expr, ExprKind, Statement, StatementKind, TaskDecl};
use crate::checker::{check_program, has_errors};
use crate::diagnostics::{Diagnostic, RepairHint};
use crate::graft::{GraftInput, apply_graft_input};
use crate::lint::{LintOptions, LintReport, build_lint_report};
use crate::query::{QueryKind, QueryOptions, QueryReport, QueryTakeSummary, build_query_report};
use crate::symbols::{slice_symbol_graph, task_fq_name};

pub const EDIT_PLAN_REPORT_SCHEMA: &str = "sley.edit_plan.report.v0";

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EditPlanReport {
    pub schema: String,
    pub status: String,
    pub target: String,
    pub entry_module: Option<String>,
    pub summary: EditPlanSummary,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<EditPlanQuerySummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lint: Option<EditPlanLintSummary>,
    pub task_surfaces: Vec<EditPlanTaskSurface>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub graft_templates: Vec<EditPlanGraftTemplate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transaction_templates: Vec<EditPlanTransactionTemplate>,
    pub next_actions: Vec<EditPlanAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EditPlanOptions {
    pub deny_warnings: bool,
    pub include_graft_templates: bool,
    pub template_surface: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EditPlanSummary {
    pub error_count: usize,
    pub warning_count: usize,
    pub module_count: usize,
    pub task_count: usize,
    pub call_count: usize,
    pub lint_finding_count: usize,
    pub task_surface_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EditPlanQuerySummary {
    pub source_schema: String,
    pub kind: String,
    pub module_count: usize,
    pub task_count: usize,
    pub call_count: usize,
    pub entrypoints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EditPlanLintSummary {
    pub source_schema: String,
    pub status: String,
    pub rules: Vec<String>,
    pub finding_count: usize,
    pub findings: Vec<EditPlanLintFinding>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EditPlanLintFinding {
    pub id: String,
    pub rule: String,
    pub severity: String,
    pub message: String,
    pub node: String,
    pub module: String,
    pub hint: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EditPlanTaskSurface {
    pub id: String,
    pub module: String,
    pub qualified_name: String,
    pub exported: bool,
    pub takes: Vec<EditPlanTake>,
    pub return_type: String,
    pub effects: Vec<String>,
    pub inbound_call_count: usize,
    pub outbound_call_count: usize,
    pub graft_targets: Vec<String>,
    pub planning_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EditPlanTake {
    pub name: String,
    pub binding_kind: String,
    #[serde(rename = "type")]
    pub ty: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EditPlanAction {
    pub kind: String,
    pub reason: String,
    pub command: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EditPlanGraftTemplate {
    pub kind: String,
    pub reason: String,
    pub surface: String,
    pub operation: JsonValue,
    pub editable_json_pointers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EditPlanTransactionTemplate {
    pub kind: String,
    pub reason: String,
    pub surface: String,
    pub transaction: JsonValue,
    pub editable_json_pointers: Vec<String>,
}

pub fn build_edit_plan_report(
    target: impl Into<String>,
    program_result: Result<Program, Vec<Diagnostic>>,
    deny_warnings: bool,
) -> EditPlanReport {
    build_edit_plan_report_with_options(
        target,
        program_result,
        EditPlanOptions {
            deny_warnings,
            include_graft_templates: false,
            template_surface: None,
        },
    )
}

pub fn build_edit_plan_report_with_options(
    target: impl Into<String>,
    program_result: Result<Program, Vec<Diagnostic>>,
    options: EditPlanOptions,
) -> EditPlanReport {
    let target = target.into();
    let deny_warnings = options.deny_warnings;
    let program = match program_result {
        Ok(program) => program,
        Err(diagnostics) => {
            let summary = diagnostic_summary(&diagnostics);
            return EditPlanReport {
                schema: EDIT_PLAN_REPORT_SCHEMA.to_string(),
                status: "blocked".to_string(),
                target: target.clone(),
                entry_module: None,
                summary,
                diagnostics,
                query: None,
                lint: None,
                task_surfaces: Vec::new(),
                graft_templates: Vec::new(),
                transaction_templates: Vec::new(),
                next_actions: blocked_actions(&target),
            };
        }
    };

    let diagnostics = check_program(&program);
    if has_errors(&diagnostics) {
        let summary = diagnostic_summary(&diagnostics);
        return EditPlanReport {
            schema: EDIT_PLAN_REPORT_SCHEMA.to_string(),
            status: "blocked".to_string(),
            target: target.clone(),
            entry_module: Some(program.module_name().to_string()),
            summary,
            diagnostics,
            query: None,
            lint: None,
            task_surfaces: Vec::new(),
            graft_templates: Vec::new(),
            transaction_templates: Vec::new(),
            next_actions: blocked_actions(&target),
        };
    }

    let query_report = build_query_report(
        &program,
        QueryOptions {
            kind: QueryKind::All,
            module: None,
            exported_only: false,
        },
    );
    let lint_report = build_lint_report(&program, LintOptions::default());
    let query = summarize_query(&query_report);
    let lint = summarize_lint(&lint_report);
    let task_surfaces = build_task_surfaces(&query_report);
    let mut diagnostics = diagnostics;
    let graft_templates = if options.include_graft_templates {
        match build_graft_templates(
            &program,
            &task_surfaces,
            &lint_report,
            options.template_surface.as_deref(),
        ) {
            Ok(templates) => templates,
            Err(diagnostic) => {
                diagnostics.push(diagnostic);
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };
    let transaction_templates = if options.include_graft_templates {
        build_transaction_templates(
            &task_surfaces,
            &query_report,
            &program,
            options.template_surface.as_deref(),
        )
        .unwrap_or_default()
    } else {
        Vec::new()
    };
    let error_count = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.is_error())
        .count();
    let warning_count = diagnostics.len() - error_count + lint.finding_count;
    let status = if error_count > 0 {
        "blocked"
    } else if warning_count == 0 {
        "ready"
    } else if deny_warnings {
        "blocked"
    } else {
        "warnings"
    };
    let summary = EditPlanSummary {
        error_count,
        warning_count,
        module_count: query.module_count,
        task_count: query.task_count,
        call_count: query.call_count,
        lint_finding_count: lint.finding_count,
        task_surface_count: task_surfaces.len(),
    };
    let next_actions = if error_count > 0 {
        template_surface_actions(&target)
    } else if warning_count > 0 {
        warning_actions(&target, &task_surfaces)
    } else {
        ready_actions(&target, &query_report, &task_surfaces)
    };

    EditPlanReport {
        schema: EDIT_PLAN_REPORT_SCHEMA.to_string(),
        status: status.to_string(),
        target,
        entry_module: Some(program.module_name().to_string()),
        summary,
        diagnostics,
        query: Some(query),
        lint: Some(lint),
        task_surfaces,
        graft_templates,
        transaction_templates,
        next_actions,
    }
}

fn diagnostic_summary(diagnostics: &[Diagnostic]) -> EditPlanSummary {
    let error_count = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.is_error())
        .count();
    EditPlanSummary {
        error_count,
        warning_count: diagnostics.len() - error_count,
        module_count: 0,
        task_count: 0,
        call_count: 0,
        lint_finding_count: 0,
        task_surface_count: 0,
    }
}

fn summarize_query(report: &QueryReport) -> EditPlanQuerySummary {
    let entry_task = format!("{}.main", report.entry_module);
    let entrypoints = report
        .tasks
        .iter()
        .filter(|task| task.qualified_name == entry_task || task.exported)
        .map(|task| task.qualified_name.clone())
        .collect();

    EditPlanQuerySummary {
        source_schema: report.schema.clone(),
        kind: report.kind.clone(),
        module_count: report.modules.len(),
        task_count: report.tasks.len(),
        call_count: report.calls.len(),
        entrypoints,
    }
}

fn summarize_lint(report: &LintReport) -> EditPlanLintSummary {
    EditPlanLintSummary {
        source_schema: report.schema.clone(),
        status: report.status.clone(),
        rules: report.filters.rules.clone(),
        finding_count: report.findings.len(),
        findings: report
            .findings
            .iter()
            .map(|finding| EditPlanLintFinding {
                id: finding.id.clone(),
                rule: finding.rule.clone(),
                severity: finding.severity.clone(),
                message: finding.message.clone(),
                node: finding.node.clone(),
                module: finding.module.clone(),
                hint: finding.hint.clone(),
            })
            .collect(),
    }
}

fn build_task_surfaces(report: &QueryReport) -> Vec<EditPlanTaskSurface> {
    let entry_task = format!("{}.main", report.entry_module);
    let mut surfaces = report
        .tasks
        .iter()
        .map(|task| EditPlanTaskSurface {
            id: task.id.clone(),
            module: task.module.clone(),
            qualified_name: task.qualified_name.clone(),
            exported: task.exported,
            takes: task.takes.iter().map(take_summary).collect(),
            return_type: task.return_type.clone(),
            effects: task.effects.clone(),
            inbound_call_count: task.inbound_call_count,
            outbound_call_count: task.outbound_call_count,
            graft_targets: vec![task.id.clone(), format!("module:{}:tasks", task.module)],
            planning_notes: planning_notes(task, &entry_task),
        })
        .collect::<Vec<_>>();
    surfaces.sort_by(|left, right| {
        surface_rank(left, &entry_task)
            .cmp(&surface_rank(right, &entry_task))
            .then_with(|| left.module.cmp(&right.module))
            .then_with(|| left.qualified_name.cmp(&right.qualified_name))
    });
    surfaces
}

fn take_summary(take: &QueryTakeSummary) -> EditPlanTake {
    EditPlanTake {
        name: take.name.clone(),
        binding_kind: take.binding_kind.clone(),
        ty: take.ty.clone(),
    }
}

fn planning_notes(task: &crate::query::QueryTaskSummary, entry_task: &str) -> Vec<String> {
    let mut notes = Vec::new();
    if task.qualified_name == entry_task {
        notes.push("entrypoint_surface".to_string());
    }
    if task.exported {
        notes.push("exported_surface".to_string());
    }
    if !task.effects.is_empty() {
        notes.push("requires_runtime_gates".to_string());
    }
    if task.outbound_call_count > 0 {
        notes.push("has_outbound_calls".to_string());
    }
    if task.inbound_call_count > 0 {
        notes.push("has_inbound_callers".to_string());
    }
    if notes.is_empty() {
        notes.push("private_task_surface".to_string());
    }
    notes
}

fn surface_rank(surface: &EditPlanTaskSurface, entry_task: &str) -> u8 {
    if surface.qualified_name == entry_task {
        0
    } else if surface.exported {
        1
    } else if !surface.effects.is_empty() {
        2
    } else if surface.inbound_call_count > 0 || surface.outbound_call_count > 0 {
        3
    } else {
        4
    }
}

fn build_graft_templates(
    program: &Program,
    surfaces: &[EditPlanTaskSurface],
    lint_report: &LintReport,
    requested_surface: Option<&str>,
) -> Result<Vec<EditPlanGraftTemplate>, Diagnostic> {
    let lint_templates = lint_declaration_delete_templates(program, lint_report);
    if let Some(requested_surface) = requested_surface {
        let requested_lint_templates = lint_templates
            .iter()
            .filter(|template| template.surface == requested_surface)
            .cloned()
            .collect::<Vec<_>>();
        if !requested_lint_templates.is_empty() {
            return Ok(requested_lint_templates);
        }
    }
    let Some(surface) = select_template_surface(surfaces, requested_surface)? else {
        return Ok(lint_templates);
    };

    let mut templates = vec![
        replace_task_body_template(surface),
        rename_declaration_template(surface),
        add_take_template(surface),
        move_task_template(surface),
    ];
    if !surface.exported
        && !surface
            .planning_notes
            .iter()
            .any(|note| note == "entrypoint_surface")
    {
        templates.push(delete_task_template(surface));
    }
    templates.extend(graph_slice_move_templates(program, surface));
    templates.extend(graph_slice_delete_templates(program, surface));
    templates.extend(graph_slice_replace_templates(program, surface));
    templates.extend(lint_templates);
    Ok(templates)
}

fn lint_declaration_delete_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter_map(|finding| {
            let (kind, declaration_kind) = match finding.id.as_str() {
                "UNUSED_PRIVATE_TYPE" => ("delete_unused_private_type", "type"),
                "UNUSED_PRIVATE_EFFECT" => ("delete_unused_private_effect", "effect"),
                _ => return None,
            };
            let operation = json!({
                "op": "DeleteNode",
                "target": finding.node
            });
            if !delete_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: kind.to_string(),
                reason: format!(
                    "delete this unused private {declaration_kind} after checked lint proves it is unreferenced"
                ),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: Vec::new(),
            })
        })
        .collect()
}

fn select_template_surface<'a>(
    surfaces: &'a [EditPlanTaskSurface],
    requested_surface: Option<&str>,
) -> Result<Option<&'a EditPlanTaskSurface>, Diagnostic> {
    let Some(requested_surface) = requested_surface else {
        return Ok(surfaces.first());
    };
    surfaces
        .iter()
        .find(|surface| {
            surface.id == requested_surface || surface.qualified_name == requested_surface
        })
        .map(Some)
        .ok_or_else(|| {
            Diagnostic::error(
                "PLAN_SURFACE_NOT_FOUND",
                format!(
                    "plan surface `{requested_surface}` was not found; use a task id, qualified task name, or lint finding declaration node"
                ),
            )
            .with_node(requested_surface)
            .with_repair_hint(
                RepairHint::new("inspect_task_surfaces")
                    .with_replacement("Run `sley plan --json <target>` and choose a task_surfaces id, task qualified_name, or lint.findings node"),
            )
        })
}

fn replace_task_body_template(surface: &EditPlanTaskSurface) -> EditPlanGraftTemplate {
    EditPlanGraftTemplate {
        kind: "replace_task_body".to_string(),
        reason: "replace the checked task body when the planned edit changes task logic"
            .to_string(),
        surface: surface.id.clone(),
        operation: json!({
            "op": "ReplaceTaskBody",
            "target": surface.id,
            "payload": {
                "statements": [default_return_statement(&surface.return_type)]
            }
        }),
        editable_json_pointers: vec!["/payload/statements".to_string()],
    }
}

fn rename_declaration_template(surface: &EditPlanTaskSurface) -> EditPlanGraftTemplate {
    EditPlanGraftTemplate {
        kind: "rename_declaration".to_string(),
        reason: "rename the task declaration; pair with UpdateCallSites when callers must change"
            .to_string(),
        surface: surface.id.clone(),
        operation: json!({
            "op": "RenameDeclaration",
            "target": surface.id,
            "payload": {
                "name": format!("renamed_{}", declaration_name(&surface.qualified_name))
            }
        }),
        editable_json_pointers: vec!["/payload/name".to_string()],
    }
}

fn add_take_template(surface: &EditPlanTaskSurface) -> EditPlanGraftTemplate {
    EditPlanGraftTemplate {
        kind: "add_take".to_string(),
        reason: "add one checked task parameter before updating callers".to_string(),
        surface: surface.id.clone(),
        operation: json!({
            "op": "AddTake",
            "target": surface.id,
            "payload": {
                "name": "new_value",
                "type": "Text",
                "position": surface.takes.len()
            }
        }),
        editable_json_pointers: vec![
            "/payload/name".to_string(),
            "/payload/type".to_string(),
            "/payload/position".to_string(),
        ],
    }
}

fn move_task_template(surface: &EditPlanTaskSurface) -> EditPlanGraftTemplate {
    EditPlanGraftTemplate {
        kind: "move_node".to_string(),
        reason: "move or reorder this task by editing the parent module target and position"
            .to_string(),
        surface: surface.id.clone(),
        operation: json!({
            "op": "MoveNode",
            "target": surface.id,
            "payload": {
                "parent": format!("module:{}:tasks", surface.module),
                "position": 0
            }
        }),
        editable_json_pointers: vec![
            "/payload/parent".to_string(),
            "/payload/position".to_string(),
        ],
    }
}

fn delete_task_template(surface: &EditPlanTaskSurface) -> EditPlanGraftTemplate {
    EditPlanGraftTemplate {
        kind: "delete_node".to_string(),
        reason: "delete a private non-entry task after query and lint prove it is safe".to_string(),
        surface: surface.id.clone(),
        operation: json!({
            "op": "DeleteNode",
            "target": surface.id
        }),
        editable_json_pointers: Vec::new(),
    }
}

fn graph_slice_move_templates(
    program: &Program,
    surface: &EditPlanTaskSurface,
) -> Vec<EditPlanGraftTemplate> {
    let Some(slice) = slice_symbol_graph(program, &surface.id) else {
        return Vec::new();
    };
    let statement_prefix = format!("block:{}", surface.id);
    let take_prefix = format!("take:{}:", surface.id);
    let mut templates = Vec::new();
    for affordance in slice.move_affordances.into_iter().filter(|affordance| {
        affordance.target.starts_with(&statement_prefix)
            || affordance.target.starts_with(&take_prefix)
    }) {
        let target_kind = affordance.target_kind.clone();
        templates.push(EditPlanGraftTemplate {
            kind: format!("move_{target_kind}"),
            reason: format!(
                "move or reorder this {target_kind} using graph-slice MoveNode affordance data"
            ),
            surface: surface.id.clone(),
            operation: affordance.operation,
            editable_json_pointers: affordance.editable_json_pointers,
        });
        for destination in affordance.destinations {
            templates.push(EditPlanGraftTemplate {
                kind: format!("move_{target_kind}_destination"),
                reason: format!(
                    "move this {target_kind} into a graph-slice destination parent using MoveNode affordance data"
                ),
                surface: surface.id.clone(),
                operation: destination.operation,
                editable_json_pointers: destination.editable_json_pointers,
            });
        }
    }
    templates
}

fn graph_slice_delete_templates(
    program: &Program,
    surface: &EditPlanTaskSurface,
) -> Vec<EditPlanGraftTemplate> {
    let Some(slice) = slice_symbol_graph(program, &surface.id) else {
        return Vec::new();
    };
    let statement_prefix = format!("block:{}", surface.id);
    let take_prefix = format!("take:{}:", surface.id);
    slice
        .delete_affordances
        .into_iter()
        .filter(|affordance| {
            affordance.target.starts_with(&statement_prefix)
                || affordance.target.starts_with(&take_prefix)
        })
        .filter(|affordance| delete_affordance_checks(program, &affordance.operation))
        .map(|affordance| EditPlanGraftTemplate {
            kind: format!("delete_{}", affordance.target_kind),
            reason: format!(
                "delete this {} using checked graph-slice DeleteNode affordance data",
                affordance.target_kind
            ),
            surface: surface.id.clone(),
            operation: affordance.operation,
            editable_json_pointers: affordance.editable_json_pointers,
        })
        .collect()
}

fn delete_affordance_checks(program: &Program, operation: &JsonValue) -> bool {
    let Ok(input) = serde_json::from_value::<GraftInput>(operation.clone()) else {
        return false;
    };
    apply_graft_input(
        program,
        input,
        Some("agent:plan-delete-affordance".to_string()),
    )
    .status
        == "accepted"
}

fn graph_slice_replace_templates(
    program: &Program,
    surface: &EditPlanTaskSurface,
) -> Vec<EditPlanGraftTemplate> {
    let Some(slice) = slice_symbol_graph(program, &surface.id) else {
        return Vec::new();
    };
    let expression_prefix = format!("block:{}", surface.id);
    slice
        .replace_affordances
        .into_iter()
        .filter(|affordance| affordance.target.starts_with(&expression_prefix))
        .filter(|affordance| replace_affordance_checks(program, &affordance.operation))
        .map(|affordance| EditPlanGraftTemplate {
            kind: "replace_expression".to_string(),
            reason: format!(
                "replace this {} expression using checked graph-slice ReplaceExpression affordance data",
                affordance.target_kind
            ),
            surface: surface.id.clone(),
            operation: affordance.operation,
            editable_json_pointers: affordance.editable_json_pointers,
        })
        .collect()
}

fn replace_affordance_checks(program: &Program, operation: &JsonValue) -> bool {
    let Ok(input) = serde_json::from_value::<GraftInput>(operation.clone()) else {
        return false;
    };
    apply_graft_input(
        program,
        input,
        Some("agent:plan-replace-affordance".to_string()),
    )
    .status
        == "accepted"
}

fn build_transaction_templates(
    surfaces: &[EditPlanTaskSurface],
    query: &QueryReport,
    program: &Program,
    requested_surface: Option<&str>,
) -> Result<Vec<EditPlanTransactionTemplate>, Diagnostic> {
    let Some(surface) = select_template_surface(surfaces, requested_surface)? else {
        return Ok(Vec::new());
    };
    let mut templates = Vec::new();
    if surface.inbound_call_count > 0 {
        let rename = rename_update_call_sites_template(surface, query);
        if let Some(rename) = rename {
            templates.push(rename);
        }
        let add_take = add_take_update_call_args_template(surface, query);
        if let Some(add_take) = add_take {
            templates.push(add_take);
        }
        let remove_take = remove_take_remove_call_arg_template(surface, query, program);
        if let Some(remove_take) = remove_take {
            templates.push(remove_take);
        }
    }
    Ok(templates)
}

fn rename_update_call_sites_template(
    surface: &EditPlanTaskSurface,
    query: &QueryReport,
) -> Option<EditPlanTransactionTemplate> {
    let new_name = format!("renamed_{}", declaration_name(&surface.qualified_name));
    let new_target = format!("task:{}.{}", surface.module, new_name);
    let mut ops = vec![json!({
        "op": "RenameDeclaration",
        "target": surface.id,
        "payload": {
            "name": new_name.clone()
        }
    })];
    let mut editable_json_pointers = vec!["/ops/0/payload/name".to_string()];
    let mut seen_call_rewrites = Vec::<(String, String)>::new();

    for call in query
        .calls
        .iter()
        .filter(|call| call.target.as_deref() == Some(surface.qualified_name.as_str()))
    {
        let replacement = replace_callee_leaf(&call.callee, &new_name);
        let key = (call.from_module.clone(), call.callee.clone());
        if seen_call_rewrites.contains(&key) {
            continue;
        }
        seen_call_rewrites.push(key);
        let op_index = ops.len();
        ops.push(json!({
            "op": "UpdateCallSites",
            "target": new_target.clone(),
            "payload": {
                "from": call.callee.clone(),
                "replacement": replacement,
                "scope": format!("module:{}", call.from_module)
            }
        }));
        editable_json_pointers.push(format!("/ops/{op_index}/payload/replacement"));
        editable_json_pointers.push(format!("/ops/{op_index}/payload/scope"));
    }

    if ops.len() == 1 {
        return None;
    }
    Some(EditPlanTransactionTemplate {
        kind: "rename_and_update_call_sites".to_string(),
        reason: "rename the task and update currently resolved callers in one all-or-nothing graft"
            .to_string(),
        surface: surface.id.clone(),
        transaction: json!({
            "transaction": format!(
                "txn_rename_{}",
                surface.qualified_name.replace('.', "_")
            ),
            "mode": "all_or_nothing",
            "ops": ops
        }),
        editable_json_pointers,
    })
}

fn add_take_update_call_args_template(
    surface: &EditPlanTaskSurface,
    query: &QueryReport,
) -> Option<EditPlanTransactionTemplate> {
    let new_take_name = "new_value";
    let new_take_type = "Text";
    let new_take_position = surface.takes.len();
    let new_arg_source = default_expression(new_take_type);
    let mut ops = vec![json!({
        "op": "AddTake",
        "target": surface.id,
        "payload": {
            "name": new_take_name,
            "type": new_take_type,
            "position": new_take_position
        }
    })];
    let mut editable_json_pointers = vec![
        "/ops/0/payload/name".to_string(),
        "/ops/0/payload/type".to_string(),
        "/ops/0/payload/position".to_string(),
    ];
    let mut seen_call_updates = Vec::<(String, String)>::new();

    for call in query
        .calls
        .iter()
        .filter(|call| call.target.as_deref() == Some(surface.qualified_name.as_str()))
    {
        let key = (call.from_module.clone(), call.callee.clone());
        if seen_call_updates.contains(&key) {
            continue;
        }
        seen_call_updates.push(key);
        let op_index = ops.len();
        ops.push(json!({
            "op": "UpdateCallArgs",
            "target": surface.id,
            "payload": {
                "from": call.callee.clone(),
                "source": new_arg_source,
                "position": new_take_position,
                "scope": format!("module:{}", call.from_module)
            }
        }));
        editable_json_pointers.push(format!("/ops/{op_index}/payload/source"));
        editable_json_pointers.push(format!("/ops/{op_index}/payload/position"));
        editable_json_pointers.push(format!("/ops/{op_index}/payload/scope"));
    }

    if ops.len() == 1 {
        return None;
    }
    Some(EditPlanTransactionTemplate {
        kind: "add_take_and_update_call_args".to_string(),
        reason: "add a task take and update currently resolved callers in one all-or-nothing graft"
            .to_string(),
        surface: surface.id.clone(),
        transaction: json!({
            "transaction": format!(
                "txn_add_take_{}",
                surface.qualified_name.replace('.', "_")
            ),
            "mode": "all_or_nothing",
            "ops": ops
        }),
        editable_json_pointers,
    })
}

fn remove_take_remove_call_arg_template(
    surface: &EditPlanTaskSurface,
    query: &QueryReport,
    program: &Program,
) -> Option<EditPlanTransactionTemplate> {
    let task = find_surface_task(program, surface)?;
    let (remove_take_position, remove_take_name) = removable_take(task)?;
    let mut ops = vec![json!({
        "op": "RemoveTake",
        "target": surface.id,
        "payload": {
            "name": remove_take_name
        }
    })];
    let mut editable_json_pointers = vec!["/ops/0/payload/name".to_string()];
    let mut seen_call_updates = Vec::<(String, String)>::new();

    for call in query
        .calls
        .iter()
        .filter(|call| call.target.as_deref() == Some(surface.qualified_name.as_str()))
    {
        let key = (call.from_module.clone(), call.callee.clone());
        if seen_call_updates.contains(&key) {
            continue;
        }
        seen_call_updates.push(key);
        let op_index = ops.len();
        ops.push(json!({
            "op": "RemoveCallArg",
            "target": surface.id,
            "payload": {
                "from": call.callee.clone(),
                "position": remove_take_position,
                "scope": format!("module:{}", call.from_module)
            }
        }));
        editable_json_pointers.push(format!("/ops/{op_index}/payload/position"));
        editable_json_pointers.push(format!("/ops/{op_index}/payload/scope"));
    }

    if ops.len() == 1 {
        return None;
    }
    Some(EditPlanTransactionTemplate {
        kind: "remove_take_and_remove_call_arg".to_string(),
        reason: "remove an unused normal task take and remove the matching caller argument in one all-or-nothing graft".to_string(),
        surface: surface.id.clone(),
        transaction: json!({
            "transaction": format!(
                "txn_remove_take_{}",
                surface.qualified_name.replace('.', "_")
            ),
            "mode": "all_or_nothing",
            "ops": ops
        }),
        editable_json_pointers,
    })
}

fn find_surface_task<'a>(
    program: &'a Program,
    surface: &EditPlanTaskSurface,
) -> Option<&'a TaskDecl> {
    program
        .tasks
        .iter()
        .find(|task| task.id == surface.id || task_fq_name(task) == surface.qualified_name)
}

fn removable_take(task: &TaskDecl) -> Option<(usize, String)> {
    task.takes
        .iter()
        .enumerate()
        .filter(|(_index, take)| take.binding_kind == BindingKind::Take)
        .filter(|(_index, take)| !block_references_identifier(&task.body, &take.name))
        .map(|(index, take)| {
            let call_arg_position = task
                .takes
                .iter()
                .take(index)
                .filter(|take| take.binding_kind != BindingKind::Gate)
                .count();
            (call_arg_position, take.name.clone())
        })
        .next_back()
}

fn block_references_identifier(block: &Block, name: &str) -> bool {
    block
        .statements
        .iter()
        .any(|statement| statement_references_identifier(statement, name))
}

fn statement_references_identifier(statement: &Statement, name: &str) -> bool {
    match &statement.kind {
        StatementKind::Binding { expr, .. }
        | StatementKind::Set { expr, .. }
        | StatementKind::Return { expr }
        | StatementKind::Expr { expr } => expr_references_identifier(expr, name),
        StatementKind::If {
            condition,
            then_block,
            else_block,
        } => {
            expr_references_identifier(condition, name)
                || block_references_identifier(then_block, name)
                || else_block
                    .as_ref()
                    .is_some_and(|block| block_references_identifier(block, name))
        }
        StatementKind::While { condition, body } => {
            expr_references_identifier(condition, name) || block_references_identifier(body, name)
        }
        StatementKind::For {
            item,
            collection,
            body,
        } => {
            expr_references_identifier(collection, name)
                || (item != name && block_references_identifier(body, name))
        }
        StatementKind::Forge { body } => block_references_identifier(body, name),
    }
}

fn expr_references_identifier(expr: &Expr, name: &str) -> bool {
    match &expr.kind {
        ExprKind::Identifier { name: identifier } => identifier == name,
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            expr_references_identifier(expr, name)
        }
        ExprKind::Binary { left, right, .. } => {
            expr_references_identifier(left, name) || expr_references_identifier(right, name)
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_references_identifier(condition, name)
                || expr_references_identifier(then_branch, name)
                || expr_references_identifier(else_branch, name)
        }
        ExprKind::Call { callee, args } => {
            expr_references_identifier(callee, name)
                || args.iter().any(|arg| expr_references_identifier(arg, name))
        }
        ExprKind::ListLiteral { items } => items
            .iter()
            .any(|item| expr_references_identifier(item, name)),
        ExprKind::MapLiteral { entries } => entries.iter().any(|entry| {
            expr_references_identifier(&entry.key, name)
                || expr_references_identifier(&entry.value, name)
        }),
        ExprKind::Index { collection, index } => {
            expr_references_identifier(collection, name) || expr_references_identifier(index, name)
        }
        ExprKind::FieldAccess { receiver, .. } => expr_references_identifier(receiver, name),
        ExprKind::RecordLiteral { fields, .. } => fields
            .iter()
            .any(|field| expr_references_identifier(&field.expr, name)),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. } => false,
    }
}

fn replace_callee_leaf(callee: &str, new_leaf: &str) -> String {
    callee
        .rsplit_once('.')
        .map(|(prefix, _leaf)| format!("{prefix}.{new_leaf}"))
        .unwrap_or_else(|| new_leaf.to_string())
}

fn default_return_statement(return_type: &str) -> String {
    if let Some(ok_type) = result_ok_type(return_type) {
        return format!("return Ok({})", default_expression(ok_type));
    }
    format!("return {}", default_expression(return_type))
}

fn default_expression(ty: &str) -> &'static str {
    match ty.trim() {
        "Int" => "0",
        "Text" => "\"\"",
        "Bool" => "false",
        _ => "TODO_VALUE",
    }
}

fn result_ok_type(return_type: &str) -> Option<&str> {
    let trimmed = return_type.trim();
    let inner = trimmed.strip_prefix("Result<")?.strip_suffix('>')?.trim();
    let mut depth = 0usize;
    for (index, ch) in inner.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => return Some(inner[..index].trim()),
            _ => {}
        }
    }
    None
}

fn declaration_name(qualified_name: &str) -> &str {
    qualified_name
        .rsplit_once('.')
        .map(|(_module, name)| name)
        .unwrap_or(qualified_name)
}

fn blocked_actions(target: &str) -> Vec<EditPlanAction> {
    vec![EditPlanAction {
        kind: "repair_diagnostics".to_string(),
        reason: "strict checking must pass before edit surfaces can be ranked".to_string(),
        command: command(["sley", "check", "--json", target]),
    }]
}

fn template_surface_actions(target: &str) -> Vec<EditPlanAction> {
    vec![EditPlanAction {
        kind: "inspect_plan_surfaces".to_string(),
        reason: "choose a valid task surface before requesting graft templates".to_string(),
        command: command(["sley", "plan", "--json", target]),
    }]
}

fn warning_actions(target: &str, surfaces: &[EditPlanTaskSurface]) -> Vec<EditPlanAction> {
    let mut actions = vec![
        EditPlanAction {
            kind: "repair_lint_findings".to_string(),
            reason: "lint findings should be resolved or deliberately accepted before grafting"
                .to_string(),
            command: command(["sley", "lint", "--json", target]),
        },
        EditPlanAction {
            kind: "inspect_tasks".to_string(),
            reason: "query task facts identify the narrowest edit surface".to_string(),
            command: command(["sley", "query", "--json", "--kind", "tasks", target]),
        },
    ];
    if let Some(surface) = surfaces.first() {
        actions.push(inspect_surface_action(target, &surface.id));
    }
    actions
}

fn ready_actions(
    target: &str,
    query: &QueryReport,
    surfaces: &[EditPlanTaskSurface],
) -> Vec<EditPlanAction> {
    let mut actions = Vec::new();
    if let Some(surface) = surfaces.first() {
        actions.push(inspect_surface_action(target, &surface.id));
    } else {
        actions.push(EditPlanAction {
            kind: "inspect_modules".to_string(),
            reason: "query module facts before planning declaration edits".to_string(),
            command: command(["sley", "query", "--json", "--kind", "modules", target]),
        });
    }
    actions.push(EditPlanAction {
        kind: "post_edit_doctor".to_string(),
        reason: "preserve strict diagnostics and lint readiness after structural edits".to_string(),
        command: command(["sley", "doctor", "--json", "--deny-warnings", target]),
    });
    actions.push(EditPlanAction {
        kind: "post_edit_verify".to_string(),
        reason: "run deterministic verification after the planned graft is applied".to_string(),
        command: verify_command(target, query),
    });
    actions
}

fn inspect_surface_action(target: &str, surface_id: &str) -> EditPlanAction {
    EditPlanAction {
        kind: "inspect_primary_surface".to_string(),
        reason:
            "graph slice gives bounded AST and call context for the highest-ranked edit surface"
                .to_string(),
        command: vec![
            "sley".to_string(),
            "graph".to_string(),
            "--json".to_string(),
            "--slice".to_string(),
            surface_id.to_string(),
            target.to_string(),
        ],
    }
}

fn verify_command(target: &str, query: &QueryReport) -> Vec<String> {
    let entry_task = format!("{}.main", query.entry_module);
    let mut command = vec![
        "sley".to_string(),
        "verify".to_string(),
        "--json".to_string(),
    ];
    if let Some(task) = query
        .tasks
        .iter()
        .find(|task| task.qualified_name == entry_task)
    {
        for effect in &task.effects {
            command.push("--cap".to_string());
            command.push(effect.clone());
        }
    }
    command.push(target.to_string());
    command
}

fn command<const N: usize>(items: [&str; N]) -> Vec<String> {
    items.into_iter().map(str::to_string).collect()
}
