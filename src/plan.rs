use std::path::{Component, Path};

use serde::Serialize;
use serde_json::{Value as JsonValue, json};

use crate::Program;
use crate::ast::{BindingKind, Block, Expr, ExprKind, Statement, StatementKind, TaskDecl};
use crate::checker::{check_program, has_errors};
use crate::diagnostics::{Diagnostic, RepairHint};
use crate::graft::{GraftInput, apply_graft_input};
use crate::lint::{
    LintOptions, LintReport, absorbing_boolean_expression_replacement_source, build_lint_report,
    constant_if_expression_replacement_source, constant_if_statement_replacement_source,
    double_negation_expression_replacement_source, identity_binary_expression_replacement_source,
    qualified_imported_call_replacement_source, raw_host_adapter_replacement,
    redundant_boolean_comparison_replacement_source,
    redundant_boolean_if_expression_replacement_source,
    redundant_boolean_if_statement_replacement_source,
    same_branch_if_expression_replacement_source, same_branch_if_statement_replacement_source,
    self_comparison_expression_replacement_source,
};
use crate::query::{QueryKind, QueryOptions, QueryReport, QueryTakeSummary, build_query_report};
use crate::symbols::{slice_symbol_graph, task_fq_name, task_module, type_module};

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
    pub module_name_hint: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedLintRepair {
    pub kind: String,
    pub surface: String,
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
            module_name_hint: None,
        },
    )
}

pub fn single_checked_lint_repair(
    target: impl Into<String>,
    program: &Program,
    lint_report: &LintReport,
) -> Option<CheckedLintRepair> {
    if lint_report.findings.len() != 1 {
        return None;
    }
    let target = target.into();
    let surface = lint_report.findings[0].node.clone();
    let report = build_edit_plan_report_with_options(
        target,
        Ok(program.clone()),
        EditPlanOptions {
            deny_warnings: false,
            include_graft_templates: true,
            template_surface: Some(surface.clone()),
            module_name_hint: None,
        },
    );
    if !report.diagnostics.is_empty() {
        return None;
    }
    let repairs = report
        .graft_templates
        .iter()
        .filter(|template| template.surface == surface && is_lint_repair_kind(&template.kind))
        .map(|template| CheckedLintRepair {
            kind: template.kind.clone(),
            surface: template.surface.clone(),
        })
        .chain(
            report
                .transaction_templates
                .iter()
                .filter(|template| {
                    template.surface == surface && is_lint_repair_kind(&template.kind)
                })
                .map(|template| CheckedLintRepair {
                    kind: template.kind.clone(),
                    surface: template.surface.clone(),
                }),
        )
        .collect::<Vec<_>>();
    match repairs.as_slice() {
        [repair] => Some(repair.clone()),
        _ => None,
    }
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
    let module_name_hint = options
        .module_name_hint
        .clone()
        .unwrap_or_else(|| infer_module_name_from_target(&target));
    let mut diagnostics = diagnostics;
    let graft_templates = if options.include_graft_templates {
        match build_graft_templates(
            &program,
            &task_surfaces,
            &lint_report,
            options.template_surface.as_deref(),
            &module_name_hint,
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
            &lint_report,
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
        warning_actions(&target, &query_report, &task_surfaces)
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
    module_name_hint: &str,
) -> Result<Vec<EditPlanGraftTemplate>, Diagnostic> {
    let lint_templates = lint_graft_templates(program, lint_report, module_name_hint);
    if let Some(requested_surface) = requested_surface {
        let requested_lint_templates = lint_templates
            .iter()
            .filter(|template| template.surface == requested_surface)
            .cloned()
            .collect::<Vec<_>>();
        if !requested_lint_templates.is_empty() {
            return Ok(requested_lint_templates);
        }
        if requested_surface == "program" {
            return Ok(program_surface_declaration_templates(program));
        }
        if let Some(template) = expression_surface_replace_template(program, requested_surface) {
            return Ok(vec![template]);
        }
        if let Some(template) = block_surface_insert_template(program, requested_surface) {
            return Ok(vec![template]);
        }
        if let Some(templates) =
            direct_graph_slice_graft_templates(program, surfaces, requested_surface)
        {
            return Ok(templates);
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
    if let Some(template) = task_body_insert_statement_template(program, surface) {
        templates.push(template);
    }
    Ok(templates)
}

fn lint_graft_templates(
    program: &Program,
    lint_report: &LintReport,
    module_name_hint: &str,
) -> Vec<EditPlanGraftTemplate> {
    let mut templates = lint_declaration_delete_templates(program, lint_report);
    templates.extend(lint_private_task_delete_templates(program, lint_report));
    templates.extend(lint_unused_import_templates(program, lint_report));
    templates.extend(lint_unused_take_templates(program, lint_report));
    templates.extend(lint_unused_declared_effect_templates(program, lint_report));
    templates.extend(lint_missing_module_templates(
        program,
        lint_report,
        module_name_hint,
    ));
    templates.extend(lint_raw_host_adapter_templates(program, lint_report));
    templates.extend(lint_unchecked_result_templates(program, lint_report));
    templates.extend(lint_unqualified_imported_call_templates(
        program,
        lint_report,
    ));
    templates.extend(lint_unused_pure_binding_templates(program, lint_report));
    templates.extend(lint_unused_pure_expression_statement_templates(
        program,
        lint_report,
    ));
    templates.extend(lint_constant_if_expression_templates(program, lint_report));
    templates.extend(lint_constant_if_statement_templates(program, lint_report));
    templates.extend(lint_constant_false_while_statement_templates(
        program,
        lint_report,
    ));
    templates.extend(lint_empty_if_statement_templates(program, lint_report));
    templates.extend(lint_empty_for_statement_templates(program, lint_report));
    templates.extend(lint_empty_forge_statement_templates(program, lint_report));
    templates.extend(lint_identity_binary_expression_templates(
        program,
        lint_report,
    ));
    templates.extend(lint_redundant_boolean_comparison_templates(
        program,
        lint_report,
    ));
    templates.extend(lint_double_negation_expression_templates(
        program,
        lint_report,
    ));
    templates.extend(lint_redundant_boolean_if_expression_templates(
        program,
        lint_report,
    ));
    templates.extend(lint_redundant_boolean_if_statement_templates(
        program,
        lint_report,
    ));
    templates.extend(lint_same_branch_if_expression_templates(
        program,
        lint_report,
    ));
    templates.extend(lint_same_branch_if_statement_templates(
        program,
        lint_report,
    ));
    templates.extend(lint_unreachable_statement_templates(program, lint_report));
    templates.extend(lint_absorbing_boolean_expression_templates(
        program,
        lint_report,
    ));
    templates.extend(lint_self_comparison_expression_templates(
        program,
        lint_report,
    ));
    templates
}

fn is_lint_repair_kind(kind: &str) -> bool {
    matches!(
        kind,
        "delete_unused_private_type"
            | "delete_unused_private_effect"
            | "delete_unused_private_task"
            | "delete_unused_import"
            | "remove_unused_take"
            | "remove_unused_declared_effect"
            | "add_module_declaration"
            | "migrate_raw_host_adapter"
            | "propagate_unchecked_result"
            | "qualify_imported_call"
            | "delete_unused_pure_binding"
            | "delete_unused_pure_expression_statement"
            | "simplify_constant_if_expression"
            | "simplify_constant_if_statement"
            | "delete_constant_false_while_statement"
            | "delete_empty_if_statement"
            | "delete_empty_for_statement"
            | "delete_empty_forge_statement"
            | "simplify_identity_binary_expression"
            | "simplify_redundant_boolean_comparison"
            | "simplify_double_negation_expression"
            | "simplify_redundant_boolean_if_expression"
            | "simplify_redundant_boolean_if_statement"
            | "simplify_same_branch_if_expression"
            | "simplify_same_branch_if_statement"
            | "delete_unreachable_statement"
            | "simplify_absorbing_boolean_expression"
            | "simplify_self_comparison_expression"
            | "convert_mutable_binding_to_bind"
            | "delete_unused_private_declarations"
            | "delete_dead_private_tasks"
    )
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

fn lint_private_task_delete_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "UNUSED_PRIVATE_TASK")
        .filter_map(|finding| {
            let operation = json!({
                "op": "DeleteNode",
                "target": finding.node
            });
            if !delete_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "delete_unused_private_task".to_string(),
                reason:
                    "delete this unused private task after checked lint proves no checked task calls it"
                        .to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: Vec::new(),
            })
        })
        .collect()
}

fn lint_unused_import_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "UNUSED_IMPORT")
        .filter_map(|finding| {
            let operation = json!({
                "op": "DeleteNode",
                "target": finding.node
            });
            if !delete_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "delete_unused_import".to_string(),
                reason:
                    "delete this unused import after checked lint proves no checked task, type, or effect uses it"
                        .to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: Vec::new(),
            })
        })
        .collect()
}

fn lint_unused_take_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "UNUSED_TAKE")
        .filter_map(|finding| {
            let (task, take_name) = find_take_target(program, &finding.node)?;
            let operation = json!({
                "op": "RemoveTake",
                "target": task.id,
                "payload": {
                    "name": take_name
                }
            });
            if !graft_operation_checks(program, &operation, Some("agent:plan-remove-unused-take"))
            {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "remove_unused_take".to_string(),
                reason:
                    "remove this unused normal take after checked lint proves the task body never reads it"
                        .to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: Vec::new(),
            })
        })
        .collect()
}

fn find_take_target<'a>(program: &'a Program, target: &str) -> Option<(&'a TaskDecl, String)> {
    program.tasks.iter().find_map(|task| {
        task.takes
            .iter()
            .find(|take| take.id == target)
            .map(|take| (task, take.name.clone()))
    })
}

fn lint_unused_declared_effect_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "UNUSED_DECLARED_EFFECT")
        .filter_map(|finding| {
            let (task, effect_name) = find_task_effect_target(program, &finding.node)?;
            let operation = json!({
                "op": "RemoveTaskEffect",
                "target": task.id,
                "payload": {
                    "name": effect_name
                }
            });
            if !graft_operation_checks(
                program,
                &operation,
                Some("agent:plan-remove-unused-declared-effect"),
            ) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "remove_unused_declared_effect".to_string(),
                reason:
                    "remove this unused declared effect after checked lint proves no checked call needs it"
                        .to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: Vec::new(),
            })
        })
        .collect()
}

fn lint_unused_pure_binding_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "UNUSED_PURE_BINDING")
        .filter_map(|finding| {
            let operation = json!({
                "op": "DeleteNode",
                "target": finding.node
            });
            if !delete_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "delete_unused_pure_binding".to_string(),
                reason:
                    "delete this unused bind after checked lint proves the initializer is pure and unread"
                        .to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: Vec::new(),
            })
        })
        .collect()
}

fn lint_unused_pure_expression_statement_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "UNUSED_PURE_EXPRESSION_STATEMENT")
        .filter_map(|finding| {
            let operation = json!({
                "op": "DeleteNode",
                "target": finding.node
            });
            if !delete_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "delete_unused_pure_expression_statement".to_string(),
                reason: "delete this no-op pure expression statement".to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: Vec::new(),
            })
        })
        .collect()
}

fn lint_constant_if_expression_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "CONSTANT_IF_EXPRESSION")
        .filter_map(|finding| {
            let replacement = constant_if_expression_replacement_source(program, &finding.node)?;
            let operation = json!({
                "op": "ReplaceExpression",
                "target": finding.node,
                "payload": {
                    "source": replacement
                }
            });
            if !replace_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "simplify_constant_if_expression".to_string(),
                reason: "replace this constant if expression with the branch that can execute"
                    .to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: vec!["/payload/source".to_string()],
            })
        })
        .collect()
}

fn lint_constant_if_statement_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "CONSTANT_IF_STATEMENT")
        .filter_map(|finding| {
            let replacement = constant_if_statement_replacement_source(program, &finding.node)?;
            let operation = json!({
                "op": "ReplaceStatement",
                "target": finding.node,
                "payload": {
                    "source": replacement
                }
            });
            if !replace_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "simplify_constant_if_statement".to_string(),
                reason:
                    "replace this constant if statement with its single executing branch statement"
                        .to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: vec!["/payload/source".to_string()],
            })
        })
        .collect()
}

fn lint_constant_false_while_statement_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "CONSTANT_FALSE_WHILE_STATEMENT")
        .filter_map(|finding| {
            let operation = json!({
                "op": "DeleteNode",
                "target": finding.node
            });
            if !delete_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "delete_constant_false_while_statement".to_string(),
                reason: "delete this never-executed while statement".to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: Vec::new(),
            })
        })
        .collect()
}

fn lint_empty_if_statement_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "EMPTY_IF_STATEMENT")
        .filter_map(|finding| {
            let operation = json!({
                "op": "DeleteNode",
                "target": finding.node
            });
            if !delete_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "delete_empty_if_statement".to_string(),
                reason: "delete this no-op if statement".to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: Vec::new(),
            })
        })
        .collect()
}

fn lint_empty_for_statement_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "EMPTY_FOR_STATEMENT")
        .filter_map(|finding| {
            let operation = json!({
                "op": "DeleteNode",
                "target": finding.node
            });
            if !delete_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "delete_empty_for_statement".to_string(),
                reason: "delete this never-executed for statement".to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: Vec::new(),
            })
        })
        .collect()
}

fn lint_empty_forge_statement_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "EMPTY_FORGE_STATEMENT")
        .filter_map(|finding| {
            let operation = json!({
                "op": "DeleteNode",
                "target": finding.node
            });
            if !delete_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "delete_empty_forge_statement".to_string(),
                reason: "delete this no-op forge statement".to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: Vec::new(),
            })
        })
        .collect()
}

fn lint_identity_binary_expression_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "IDENTITY_BINARY_EXPRESSION")
        .filter_map(|finding| {
            let replacement =
                identity_binary_expression_replacement_source(program, &finding.node)?;
            let operation = json!({
                "op": "ReplaceExpression",
                "target": finding.node,
                "payload": {
                    "source": replacement
                }
            });
            if !replace_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "simplify_identity_binary_expression".to_string(),
                reason: "replace this identity binary expression with the non-identity side"
                    .to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: vec!["/payload/source".to_string()],
            })
        })
        .collect()
}

fn lint_redundant_boolean_comparison_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "REDUNDANT_BOOLEAN_COMPARISON")
        .filter_map(|finding| {
            let replacement =
                redundant_boolean_comparison_replacement_source(program, &finding.node)?;
            let operation = json!({
                "op": "ReplaceExpression",
                "target": finding.node,
                "payload": {
                    "source": replacement
                }
            });
            if !replace_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "simplify_redundant_boolean_comparison".to_string(),
                reason: "replace this redundant boolean comparison with the boolean expression"
                    .to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: vec!["/payload/source".to_string()],
            })
        })
        .collect()
}

fn lint_absorbing_boolean_expression_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "ABSORBING_BOOLEAN_EXPRESSION")
        .filter_map(|finding| {
            let replacement =
                absorbing_boolean_expression_replacement_source(program, &finding.node)?;
            let operation = json!({
                "op": "ReplaceExpression",
                "target": finding.node,
                "payload": {
                    "source": replacement
                }
            });
            if !replace_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "simplify_absorbing_boolean_expression".to_string(),
                reason: "replace this absorbing boolean expression with the absorbing literal"
                    .to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: vec!["/payload/source".to_string()],
            })
        })
        .collect()
}

fn lint_self_comparison_expression_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "SELF_COMPARISON_EXPRESSION")
        .filter_map(|finding| {
            let replacement =
                self_comparison_expression_replacement_source(program, &finding.node)?;
            let operation = json!({
                "op": "ReplaceExpression",
                "target": finding.node,
                "payload": {
                    "source": replacement
                }
            });
            if !replace_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "simplify_self_comparison_expression".to_string(),
                reason: "replace this self-comparison expression with the constant boolean result"
                    .to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: vec!["/payload/source".to_string()],
            })
        })
        .collect()
}

fn lint_double_negation_expression_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "DOUBLE_NEGATION_EXPRESSION")
        .filter_map(|finding| {
            let replacement =
                double_negation_expression_replacement_source(program, &finding.node)?;
            let operation = json!({
                "op": "ReplaceExpression",
                "target": finding.node,
                "payload": {
                    "source": replacement
                }
            });
            if !replace_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "simplify_double_negation_expression".to_string(),
                reason: "replace this double negation expression with the inner boolean expression"
                    .to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: vec!["/payload/source".to_string()],
            })
        })
        .collect()
}

fn lint_redundant_boolean_if_expression_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "REDUNDANT_BOOLEAN_IF_EXPRESSION")
        .filter_map(|finding| {
            let replacement =
                redundant_boolean_if_expression_replacement_source(program, &finding.node)?;
            let operation = json!({
                "op": "ReplaceExpression",
                "target": finding.node,
                "payload": {
                    "source": replacement
                }
            });
            if !replace_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "simplify_redundant_boolean_if_expression".to_string(),
                reason: "replace this redundant boolean if expression with the condition"
                    .to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: vec!["/payload/source".to_string()],
            })
        })
        .collect()
}

fn lint_redundant_boolean_if_statement_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "REDUNDANT_BOOLEAN_IF_STATEMENT")
        .filter_map(|finding| {
            let replacement =
                redundant_boolean_if_statement_replacement_source(program, &finding.node)?;
            let operation = json!({
                "op": "ReplaceStatement",
                "target": finding.node,
                "payload": {
                    "source": replacement
                }
            });
            if !replace_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "simplify_redundant_boolean_if_statement".to_string(),
                reason: "replace this redundant boolean if statement with a direct return"
                    .to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: vec!["/payload/source".to_string()],
            })
        })
        .collect()
}

fn lint_same_branch_if_expression_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "SAME_BRANCH_IF_EXPRESSION")
        .filter_map(|finding| {
            let replacement = same_branch_if_expression_replacement_source(program, &finding.node)?;
            let operation = json!({
                "op": "ReplaceExpression",
                "target": finding.node,
                "payload": {
                    "source": replacement
                }
            });
            if !replace_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "simplify_same_branch_if_expression".to_string(),
                reason: "replace this same-branch if expression with either branch".to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: vec!["/payload/source".to_string()],
            })
        })
        .collect()
}

fn lint_same_branch_if_statement_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "SAME_BRANCH_IF_STATEMENT")
        .filter_map(|finding| {
            let replacement = same_branch_if_statement_replacement_source(program, &finding.node)?;
            let operation = json!({
                "op": "ReplaceStatement",
                "target": finding.node,
                "payload": {
                    "source": replacement
                }
            });
            if !replace_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "simplify_same_branch_if_statement".to_string(),
                reason: "replace this same-branch if statement with either branch".to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: vec!["/payload/source".to_string()],
            })
        })
        .collect()
}

fn lint_unreachable_statement_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "UNREACHABLE_STATEMENT")
        .filter_map(|finding| {
            let operation = json!({
                "op": "DeleteNode",
                "target": finding.node
            });
            if !delete_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "delete_unreachable_statement".to_string(),
                reason: "delete this unreachable statement".to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: Vec::new(),
            })
        })
        .collect()
}

fn find_task_effect_target<'a>(
    program: &'a Program,
    target: &str,
) -> Option<(&'a TaskDecl, String)> {
    program.tasks.iter().find_map(|task| {
        task.effects.iter().enumerate().find_map(|(index, effect)| {
            let id = format!("effect-use:{}:{index}:{effect}", task.id);
            (id == target).then(|| (task, effect.clone()))
        })
    })
}

fn lint_missing_module_templates(
    program: &Program,
    lint_report: &LintReport,
    module_name_hint: &str,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "MISSING_MODULE_DECLARATION")
        .filter_map(|finding| {
            let operation = json!({
                "op": "AddModuleDeclaration",
                "payload": {
                    "name": module_name_hint
                }
            });
            if !graft_operation_checks(
                program,
                &operation,
                Some("agent:plan-add-module-declaration"),
            ) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "add_module_declaration".to_string(),
                reason:
                    "add an explicit module declaration so graph ids and project writeback stay stable"
                        .to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: vec!["/payload/name".to_string()],
            })
        })
        .collect()
}

fn lint_raw_host_adapter_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "RAW_HOST_ADAPTER")
        .filter_map(|finding| {
            let source = find_expr_source(program, &finding.node)?;
            let replacement = raw_host_adapter_replacement_source(&source)?;
            let operation = json!({
                "op": "ReplaceExpression",
                "target": finding.node,
                "payload": {
                    "source": replacement
                }
            });
            if !replace_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "migrate_raw_host_adapter".to_string(),
                reason:
                    "replace the diagnostic-failing host adapter with a fallible try_ adapter and `?` propagation"
                        .to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: vec!["/payload/source".to_string()],
            })
        })
        .collect()
}

fn raw_host_adapter_replacement_source(source: &str) -> Option<String> {
    let source = source.trim();
    for raw in ["fs.read_text", "fs.write_text", "db.query_one", "db.query"] {
        let Some(rest) = source.strip_prefix(raw) else {
            continue;
        };
        if !rest.starts_with('(') {
            continue;
        }
        let replacement = raw_host_adapter_replacement(raw)?;
        return Some(format!("{replacement}{rest}?"));
    }
    None
}

fn lint_unchecked_result_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "UNCHECKED_RESULT")
        .filter_map(|finding| {
            let source = find_expr_source(program, &finding.node)?;
            let replacement = format!("{}?", source.trim());
            let operation = json!({
                "op": "ReplaceExpression",
                "target": finding.node,
                "payload": {
                    "source": replacement
                }
            });
            if !replace_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "propagate_unchecked_result".to_string(),
                reason:
                    "propagate the discarded Result with `?` when the owning task can return Result"
                        .to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: vec!["/payload/source".to_string()],
            })
        })
        .collect()
}

fn lint_unqualified_imported_call_templates(
    program: &Program,
    lint_report: &LintReport,
) -> Vec<EditPlanGraftTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "UNQUALIFIED_IMPORTED_CALL")
        .filter_map(|finding| {
            let replacement = qualified_imported_call_replacement_source(program, &finding.node)?;
            let operation = json!({
                "op": "ReplaceExpression",
                "target": finding.node,
                "payload": {
                    "source": replacement
                }
            });
            if !replace_affordance_checks(program, &operation) {
                return None;
            }
            Some(EditPlanGraftTemplate {
                kind: "qualify_imported_call".to_string(),
                reason:
                    "qualify this imported task call so future imports cannot change simple-name resolution"
                        .to_string(),
                surface: finding.node.clone(),
                operation,
                editable_json_pointers: vec!["/payload/source".to_string()],
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExpressionSurface {
    source: String,
    kind: &'static str,
}

fn expression_surface_replace_template(
    program: &Program,
    requested_surface: &str,
) -> Option<EditPlanGraftTemplate> {
    let expression = find_expr_surface(program, requested_surface)?;
    let operation = json!({
        "op": "ReplaceExpression",
        "target": requested_surface,
        "payload": {
            "source": expression.source
        }
    });
    if !replace_affordance_checks(program, &operation) {
        return None;
    }
    Some(EditPlanGraftTemplate {
        kind: "replace_expression".to_string(),
        reason: format!(
            "replace this {} expression using a checked no-op starter graft; edit /payload/source before applying",
            expression.kind
        ),
        surface: requested_surface.to_string(),
        operation,
        editable_json_pointers: vec!["/payload/source".to_string()],
    })
}

fn block_surface_insert_template(
    program: &Program,
    requested_surface: &str,
) -> Option<EditPlanGraftTemplate> {
    if !requested_surface.starts_with("block:") {
        return None;
    }
    let operation = json!({
        "op": "InsertStatement",
        "target": requested_surface,
        "payload": {
            "source": "forge { }",
            "position": 0
        }
    });
    if !insert_statement_checks(program, &operation) {
        return None;
    }
    Some(EditPlanGraftTemplate {
        kind: "insert_statement".to_string(),
        reason:
            "insert one checked statement into this block; edit /payload/source before applying"
                .to_string(),
        surface: requested_surface.to_string(),
        operation,
        editable_json_pointers: vec![
            "/payload/source".to_string(),
            "/payload/position".to_string(),
        ],
    })
}

fn insert_statement_checks(program: &Program, operation: &JsonValue) -> bool {
    graft_operation_checks(program, operation, Some("agent:plan-insert-affordance"))
}

fn direct_graph_slice_graft_templates(
    program: &Program,
    surfaces: &[EditPlanTaskSurface],
    requested_surface: &str,
) -> Option<Vec<EditPlanGraftTemplate>> {
    let owner_task = owning_task_surface_id(requested_surface)?;
    let surface = surfaces.iter().find(|surface| surface.id == owner_task)?;
    let mut templates = Vec::new();
    templates.extend(graph_slice_move_templates(program, surface));
    templates.extend(graph_slice_delete_templates(program, surface));
    templates.extend(graph_slice_replace_templates(program, surface));
    let templates = templates
        .into_iter()
        .filter(|template| operation_target(&template.operation) == Some(requested_surface))
        .map(|mut template| {
            template.surface = requested_surface.to_string();
            template
        })
        .collect::<Vec<_>>();
    if templates.is_empty() {
        None
    } else {
        Some(templates)
    }
}

fn owning_task_surface_id(surface: &str) -> Option<&str> {
    if let Some(rest) = surface.strip_prefix("block:") {
        return rest
            .split_once(":stmt:")
            .map(|(task_surface, _statement_path)| task_surface);
    }
    if let Some(rest) = surface.strip_prefix("take:") {
        let mut parts = rest.rsplitn(3, ':');
        parts.next()?;
        parts.next()?;
        return parts.next();
    }
    None
}

fn operation_target(operation: &JsonValue) -> Option<&str> {
    operation.pointer("/target").and_then(JsonValue::as_str)
}

fn find_expr_source(program: &Program, target: &str) -> Option<String> {
    find_expr_surface(program, target).map(|surface| surface.source)
}

fn find_expr_surface(program: &Program, target: &str) -> Option<ExpressionSurface> {
    program
        .tasks
        .iter()
        .find_map(|task| find_expr_surface_in_block(&task.body, target))
}

fn find_expr_surface_in_block(block: &Block, target: &str) -> Option<ExpressionSurface> {
    block
        .statements
        .iter()
        .find_map(|statement| find_expr_surface_in_statement(statement, target))
}

fn find_expr_surface_in_statement(
    statement: &Statement,
    target: &str,
) -> Option<ExpressionSurface> {
    match &statement.kind {
        StatementKind::Binding { expr, .. }
        | StatementKind::Set { expr, .. }
        | StatementKind::Return { expr }
        | StatementKind::Expr { expr } => find_expr_surface_in_expr(expr, target),
        StatementKind::If {
            condition,
            then_block,
            else_block,
        } => find_expr_surface_in_expr(condition, target)
            .or_else(|| find_expr_surface_in_block(then_block, target))
            .or_else(|| {
                else_block
                    .as_ref()
                    .and_then(|block| find_expr_surface_in_block(block, target))
            }),
        StatementKind::While { condition, body } => find_expr_surface_in_expr(condition, target)
            .or_else(|| find_expr_surface_in_block(body, target)),
        StatementKind::For {
            collection, body, ..
        } => find_expr_surface_in_expr(collection, target)
            .or_else(|| find_expr_surface_in_block(body, target)),
        StatementKind::Forge { body } => find_expr_surface_in_block(body, target),
    }
}

fn find_expr_surface_in_expr(expr: &Expr, target: &str) -> Option<ExpressionSurface> {
    if expr.id == target {
        return Some(ExpressionSurface {
            source: expr.source.clone(),
            kind: expr_kind_label(&expr.kind),
        });
    }
    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            find_expr_surface_in_expr(expr, target)
        }
        ExprKind::Binary { left, right, .. } => find_expr_surface_in_expr(left, target)
            .or_else(|| find_expr_surface_in_expr(right, target)),
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => find_expr_surface_in_expr(condition, target)
            .or_else(|| find_expr_surface_in_expr(then_branch, target))
            .or_else(|| find_expr_surface_in_expr(else_branch, target)),
        ExprKind::Call { callee, args } => {
            find_expr_surface_in_expr(callee, target).or_else(|| {
                args.iter()
                    .find_map(|arg| find_expr_surface_in_expr(arg, target))
            })
        }
        ExprKind::ListLiteral { items } => items
            .iter()
            .find_map(|item| find_expr_surface_in_expr(item, target)),
        ExprKind::MapLiteral { entries } => entries.iter().find_map(|entry| {
            find_expr_surface_in_expr(&entry.key, target)
                .or_else(|| find_expr_surface_in_expr(&entry.value, target))
        }),
        ExprKind::Index { collection, index } => find_expr_surface_in_expr(collection, target)
            .or_else(|| find_expr_surface_in_expr(index, target)),
        ExprKind::FieldAccess { receiver, .. } => find_expr_surface_in_expr(receiver, target),
        ExprKind::RecordLiteral { fields, .. } => fields
            .iter()
            .find_map(|field| find_expr_surface_in_expr(&field.expr, target)),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => None,
    }
}

fn expr_kind_label(kind: &ExprKind) -> &'static str {
    match kind {
        ExprKind::Raw { .. } => "Raw",
        ExprKind::StringLiteral { .. } => "StringLiteral",
        ExprKind::IntLiteral { .. } => "IntLiteral",
        ExprKind::FloatLiteral { .. } => "FloatLiteral",
        ExprKind::BoolLiteral { .. } => "BoolLiteral",
        ExprKind::Identifier { .. } => "Identifier",
        ExprKind::Unary { .. } => "Unary",
        ExprKind::Binary { .. } => "Binary",
        ExprKind::If { .. } => "If",
        ExprKind::Call { .. } => "Call",
        ExprKind::ListLiteral { .. } => "ListLiteral",
        ExprKind::MapLiteral { .. } => "MapLiteral",
        ExprKind::Index { .. } => "Index",
        ExprKind::FieldAccess { .. } => "FieldAccess",
        ExprKind::RecordLiteral { .. } => "RecordLiteral",
        ExprKind::Try { .. } => "Try",
    }
}

pub fn infer_module_name_from_target(target: &str) -> String {
    module_name_from_sley_path(Path::new(target)).unwrap_or_else(|| "app.main".to_string())
}

pub fn module_name_from_sley_path(path: &Path) -> Option<String> {
    if path.extension().and_then(|extension| extension.to_str()) != Some("sley") {
        return None;
    }
    let mut components = path_module_components(path)?;
    if let Some(src_index) = components.iter().rposition(|component| component == "src") {
        components = components.split_off(src_index + 1);
    } else {
        components = components
            .last()
            .cloned()
            .map(|component| vec![component])
            .unwrap_or_default();
    }
    if components.is_empty() {
        return None;
    }
    Some(components.join("."))
}

pub fn module_name_from_project_relative_sley_path(path: &Path) -> Option<String> {
    if path.extension().and_then(|extension| extension.to_str()) != Some("sley") {
        return None;
    }
    let components = path_module_components(path)?;
    if components.is_empty() {
        return None;
    }
    Some(components.join("."))
}

fn path_module_components(path: &Path) -> Option<Vec<String>> {
    let mut components = Vec::new();
    for component in path.components() {
        let Component::Normal(value) = component else {
            continue;
        };
        components.push(value.to_str()?.to_string());
    }
    if components.is_empty() {
        return None;
    }
    if let Some(last) = components.last_mut() {
        let stem = Path::new(last).file_stem()?.to_str()?;
        *last = stem.to_string();
    }
    components
        .into_iter()
        .map(|component| module_segment_from_path(&component))
        .collect()
}

fn module_segment_from_path(segment: &str) -> Option<String> {
    let mut normalized = String::new();
    let mut previous_was_underscore = false;
    for ch in segment.chars().flat_map(char::to_lowercase) {
        let next = if ch.is_ascii_alphanumeric() || ch == '_' {
            ch
        } else {
            '_'
        };
        if next == '_' {
            if !previous_was_underscore {
                normalized.push(next);
            }
            previous_was_underscore = true;
        } else {
            normalized.push(next);
            previous_was_underscore = false;
        }
    }
    let normalized = normalized.trim_matches('_');
    if normalized.is_empty() {
        return None;
    }
    if normalized
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_digit())
    {
        Some(format!("mod_{normalized}"))
    } else {
        Some(normalized.to_string())
    }
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
                    "plan surface `{requested_surface}` was not found; use `program`, a task id, qualified task name, block node id, statement node id, take node id, expression node id, or lint finding node"
                ),
            )
            .with_node(requested_surface)
            .with_repair_hint(
                RepairHint::new("inspect_task_surfaces")
                    .with_replacement("Run `sley ast --json <target>` or `sley plan --json <target>` and choose `program`, a block, statement, take, or expression node id, task_surfaces id, task qualified_name, or lint.findings node"),
            )
        })
}

fn program_surface_declaration_templates(program: &Program) -> Vec<EditPlanGraftTemplate> {
    [
        add_task_declaration_template(program),
        add_type_declaration_template(program),
        add_effect_declaration_template(program),
        add_import_template(program),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn add_task_declaration_template(program: &Program) -> Option<EditPlanGraftTemplate> {
    let module = program.module_name();
    let name = unique_name(
        "new_task",
        program
            .tasks
            .iter()
            .filter(|task| task_module(task) == module)
            .map(|task| task.name.as_str()),
    );
    let source = format!("module {module}\n\ntask {name} -> Int {{\n  return 0\n}}");
    let operation = json!({
        "op": "AddTask",
        "payload": {
            "source": source
        }
    });
    if !graft_operation_checks(program, &operation, Some("agent:plan-add-task")) {
        return None;
    }
    Some(EditPlanGraftTemplate {
        kind: "add_task".to_string(),
        reason: "add a checked task declaration to the current program module".to_string(),
        surface: "program".to_string(),
        operation,
        editable_json_pointers: vec!["/payload/source".to_string()],
    })
}

fn add_type_declaration_template(program: &Program) -> Option<EditPlanGraftTemplate> {
    let module = program.module_name();
    let name = unique_name(
        "NewRecord",
        program
            .types
            .iter()
            .filter(|ty| type_module(ty) == module)
            .map(|ty| ty.name.as_str()),
    );
    let source = format!("module {module}\n\ntype {name} = {{\n  slot value: Text\n}}");
    let operation = json!({
        "op": "AddTypeDeclaration",
        "payload": {
            "source": source
        }
    });
    if !graft_operation_checks(program, &operation, Some("agent:plan-add-type-declaration")) {
        return None;
    }
    Some(EditPlanGraftTemplate {
        kind: "add_type_declaration".to_string(),
        reason: "add a checked record type declaration to the current program module".to_string(),
        surface: "program".to_string(),
        operation,
        editable_json_pointers: vec!["/payload/source".to_string()],
    })
}

fn add_effect_declaration_template(program: &Program) -> Option<EditPlanGraftTemplate> {
    let name = unique_name(
        "NewEffect",
        program.effects.iter().map(|effect| effect.name.as_str()),
    );
    let operation = json!({
        "op": "AddEffectDeclaration",
        "payload": {
            "name": name
        }
    });
    if !graft_operation_checks(
        program,
        &operation,
        Some("agent:plan-add-effect-declaration"),
    ) {
        return None;
    }
    Some(EditPlanGraftTemplate {
        kind: "add_effect_declaration".to_string(),
        reason: "add a checked effect declaration to the current program module".to_string(),
        surface: "program".to_string(),
        operation,
        editable_json_pointers: vec!["/payload/name".to_string()],
    })
}

fn add_import_template(program: &Program) -> Option<EditPlanGraftTemplate> {
    let module = unique_name(
        "app.new_module",
        program.imports.iter().map(|import| import.module.as_str()),
    );
    let operation = json!({
        "op": "AddImport",
        "payload": {
            "module": module
        }
    });
    if !graft_operation_checks(program, &operation, Some("agent:plan-add-import")) {
        return None;
    }
    Some(EditPlanGraftTemplate {
        kind: "add_import".to_string(),
        reason: "add a checked import to the current program module".to_string(),
        surface: "program".to_string(),
        operation,
        editable_json_pointers: vec!["/payload/module".to_string()],
    })
}

fn unique_name<'a>(base: &str, existing: impl Iterator<Item = &'a str>) -> String {
    let existing = existing.collect::<Vec<_>>();
    if !existing.contains(&base) {
        return base.to_string();
    }
    for index in 2.. {
        let candidate = format!("{base}{index}");
        if !existing.iter().any(|name| *name == candidate) {
            return candidate;
        }
    }
    unreachable!("unbounded unique name search should return")
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

fn task_body_insert_statement_template(
    program: &Program,
    surface: &EditPlanTaskSurface,
) -> Option<EditPlanGraftTemplate> {
    let target = format!("block:{}", surface.id);
    let operation = json!({
        "op": "InsertStatement",
        "target": target,
        "payload": {
            "source": "forge { }",
            "position": 0
        }
    });
    if !insert_statement_checks(program, &operation) {
        return None;
    }
    Some(EditPlanGraftTemplate {
        kind: "insert_statement".to_string(),
        reason:
            "insert one checked statement into the selected task body; edit /payload/source before applying"
                .to_string(),
        surface: surface.id.clone(),
        operation,
        editable_json_pointers: vec![
            "/payload/source".to_string(),
            "/payload/position".to_string(),
        ],
    })
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
    graft_operation_checks(program, operation, Some("agent:plan-delete-affordance"))
}

fn graft_operation_checks(program: &Program, operation: &JsonValue, actor: Option<&str>) -> bool {
    let Ok(input) = serde_json::from_value::<GraftInput>(operation.clone()) else {
        return false;
    };
    apply_graft_input(program, input, actor.map(str::to_string)).status == "accepted"
}

fn graph_slice_replace_templates(
    program: &Program,
    surface: &EditPlanTaskSurface,
) -> Vec<EditPlanGraftTemplate> {
    let Some(slice) = slice_symbol_graph(program, &surface.id) else {
        return Vec::new();
    };
    let block_prefix = format!("block:{}", surface.id);
    slice
        .replace_affordances
        .into_iter()
        .filter(|affordance| affordance.target.starts_with(&block_prefix))
        .filter(|affordance| replace_affordance_checks(program, &affordance.operation))
        .map(|affordance| EditPlanGraftTemplate {
            kind: if affordance.target_kind == "statement" {
                "replace_statement".to_string()
            } else {
                "replace_expression".to_string()
            },
            reason: if affordance.target_kind == "statement" {
                "replace this statement using checked graph-slice ReplaceStatement affordance data"
                    .to_string()
            } else {
                format!(
                    "replace this {} expression using checked graph-slice ReplaceExpression affordance data",
                    affordance.target_kind
                )
            },
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
    lint_report: &LintReport,
    requested_surface: Option<&str>,
) -> Result<Vec<EditPlanTransactionTemplate>, Diagnostic> {
    let mut templates =
        lint_declaration_delete_transaction_templates(program, lint_report, requested_surface);
    templates.extend(lint_dead_private_task_transaction_templates(
        program,
        lint_report,
        requested_surface,
    ));
    templates.extend(lint_mutable_binding_to_bind_transaction_templates(
        program,
        lint_report,
        requested_surface,
    ));
    if requested_surface.is_some() && !templates.is_empty() {
        return Ok(templates);
    }
    let Some(surface) = select_template_surface(surfaces, requested_surface)? else {
        return Ok(templates);
    };
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

fn lint_mutable_binding_to_bind_transaction_templates(
    program: &Program,
    lint_report: &LintReport,
    requested_surface: Option<&str>,
) -> Vec<EditPlanTransactionTemplate> {
    lint_report
        .findings
        .iter()
        .filter(|finding| finding.id == "MUTABLE_BINDING_NEVER_SET")
        .filter(|finding| requested_surface.is_none_or(|surface| surface == finding.node))
        .filter_map(|finding| {
            let location = find_statement_location(program, &finding.node)?;
            let replacement_source = binding_statement_as_bind_source(location.statement)?;
            let transaction = json!({
                "transaction": format!(
                    "txn_convert_mutable_binding_{}",
                    finding.node.replace([':', '.'], "_")
                ),
                "mode": "all_or_nothing",
                "ops": [
                    {
                        "op": "DeleteNode",
                        "target": finding.node
                    },
                    {
                        "op": "InsertStatement",
                        "target": location.parent,
                        "payload": {
                            "source": replacement_source,
                            "position": location.position
                        }
                    }
                ]
            });
            let Ok(input) = serde_json::from_value::<GraftInput>(transaction.clone()) else {
                return None;
            };
            if apply_graft_input(
                program,
                input,
                Some("agent:plan-convert-mutable-binding".to_string()),
            )
            .status
                != "accepted"
            {
                return None;
            }
            Some(EditPlanTransactionTemplate {
                kind: "convert_mutable_binding_to_bind".to_string(),
                reason:
                    "convert this never-set mutable local into an immutable bind in one checked graft"
                        .to_string(),
                surface: finding.node.clone(),
                transaction,
                editable_json_pointers: vec!["/ops/1/payload/source".to_string()],
            })
        })
        .collect()
}

struct StatementLocation<'a> {
    parent: String,
    position: usize,
    statement: &'a Statement,
}

fn find_statement_location<'a>(
    program: &'a Program,
    target: &str,
) -> Option<StatementLocation<'a>> {
    program.tasks.iter().find_map(|task| {
        find_statement_location_in_block(&task.body, format!("block:{}", task.id), target)
    })
}

fn find_statement_location_in_block<'a>(
    block: &'a Block,
    parent: String,
    target: &str,
) -> Option<StatementLocation<'a>> {
    for (position, statement) in block.statements.iter().enumerate() {
        if statement.id == target {
            return Some(StatementLocation {
                parent,
                position,
                statement,
            });
        }
        if let Some(location) = find_statement_location_in_statement(statement, target) {
            return Some(location);
        }
    }
    None
}

fn find_statement_location_in_statement<'a>(
    statement: &'a Statement,
    target: &str,
) -> Option<StatementLocation<'a>> {
    match &statement.kind {
        StatementKind::If {
            then_block,
            else_block,
            ..
        } => find_statement_location_in_block(then_block, format!("{}:then", statement.id), target)
            .or_else(|| {
                else_block.as_ref().and_then(|else_block| {
                    find_statement_location_in_block(
                        else_block,
                        format!("{}:else", statement.id),
                        target,
                    )
                })
            }),
        StatementKind::While { body, .. } | StatementKind::For { body, .. } => {
            find_statement_location_in_block(body, format!("{}:body", statement.id), target)
        }
        StatementKind::Forge { body } => {
            find_statement_location_in_block(body, format!("{}:forge", statement.id), target)
        }
        StatementKind::Binding { .. }
        | StatementKind::Set { .. }
        | StatementKind::Return { .. }
        | StatementKind::Expr { .. } => None,
    }
}

fn binding_statement_as_bind_source(statement: &Statement) -> Option<String> {
    let StatementKind::Binding {
        binding_kind,
        name,
        type_ann,
        expr,
    } = &statement.kind
    else {
        return None;
    };
    if !binding_kind.is_mutable_local() {
        return None;
    }
    let mut source = format!("bind {name}");
    if let Some(type_ann) = type_ann {
        source.push_str(": ");
        source.push_str(&type_ann.display());
    }
    source.push_str(" = ");
    source.push_str(expr.source.trim());
    Some(source)
}

fn lint_declaration_delete_transaction_templates(
    program: &Program,
    lint_report: &LintReport,
    requested_surface: Option<&str>,
) -> Vec<EditPlanTransactionTemplate> {
    if requested_surface.is_some() {
        return Vec::new();
    }
    let delete_templates = lint_declaration_delete_templates(program, lint_report);
    if delete_templates.len() < 2 {
        return Vec::new();
    }
    let ops = delete_templates
        .iter()
        .map(|template| template.operation.clone())
        .collect::<Vec<_>>();
    let transaction = json!({
        "transaction": "txn_delete_unused_private_declarations",
        "mode": "all_or_nothing",
        "ops": ops
    });
    let Ok(input) = serde_json::from_value::<GraftInput>(transaction.clone()) else {
        return Vec::new();
    };
    if apply_graft_input(
        program,
        input,
        Some("agent:plan-delete-declaration-transaction".to_string()),
    )
    .status
        != "accepted"
    {
        return Vec::new();
    }
    vec![EditPlanTransactionTemplate {
        kind: "delete_unused_private_declarations".to_string(),
        reason: "delete all unused private type/effect declarations in one all-or-nothing graft"
            .to_string(),
        surface: "lint:unused_private_declarations".to_string(),
        transaction,
        editable_json_pointers: Vec::new(),
    }]
}

fn lint_dead_private_task_transaction_templates(
    program: &Program,
    lint_report: &LintReport,
    requested_surface: Option<&str>,
) -> Vec<EditPlanTransactionTemplate> {
    if requested_surface.is_some() {
        return Vec::new();
    }
    let mut targets = Vec::<String>::new();
    for finding in lint_report.findings.iter().filter(|finding| {
        matches!(
            finding.id.as_str(),
            "UNUSED_PRIVATE_TASK" | "UNREACHABLE_PRIVATE_TASK"
        )
    }) {
        if !targets.contains(&finding.node) {
            targets.push(finding.node.clone());
        }
    }
    if targets.len() < 2 {
        return Vec::new();
    }
    let ops = targets
        .iter()
        .map(|target| {
            json!({
                "op": "DeleteNode",
                "target": target
            })
        })
        .collect::<Vec<_>>();
    let transaction = json!({
        "transaction": "txn_delete_dead_private_tasks",
        "mode": "all_or_nothing",
        "ops": ops
    });
    let Ok(input) = serde_json::from_value::<GraftInput>(transaction.clone()) else {
        return Vec::new();
    };
    if apply_graft_input(
        program,
        input,
        Some("agent:plan-delete-dead-private-task-transaction".to_string()),
    )
    .status
        != "accepted"
    {
        return Vec::new();
    }
    vec![EditPlanTransactionTemplate {
        kind: "delete_dead_private_tasks".to_string(),
        reason:
            "delete all lint-proven dead private tasks in one all-or-nothing graft so unreachable cycles clean up together"
                .to_string(),
        surface: "lint:dead_private_tasks".to_string(),
        transaction,
        editable_json_pointers: Vec::new(),
    }]
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
        reason:
            "choose a valid task, block, statement, take, expression, or lint surface before requesting graft templates"
                .to_string(),
        command: command(["sley", "plan", "--json", target]),
    }]
}

fn warning_actions(
    target: &str,
    query: &QueryReport,
    surfaces: &[EditPlanTaskSurface],
) -> Vec<EditPlanAction> {
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
    if !query.calls.is_empty() {
        actions.push(inspect_calls_action(target));
    }
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
    if !query.calls.is_empty() {
        actions.push(inspect_calls_action(target));
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

fn inspect_calls_action(target: &str) -> EditPlanAction {
    EditPlanAction {
        kind: "inspect_calls".to_string(),
        reason:
            "strict call rows show caller/callee edges before rename, arity, or authority edits"
                .to_string(),
        command: command(["sley", "query", "--json", "--kind", "calls", target]),
    }
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
