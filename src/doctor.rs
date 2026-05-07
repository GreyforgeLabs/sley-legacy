use serde::Serialize;

use crate::Program;
use crate::checker::{check_program, has_errors};
use crate::diagnostics::Diagnostic;
use crate::lint::{LintOptions, LintReport, build_lint_report};
use crate::plan::{CheckedLintRepair, single_checked_lint_repair};
use crate::query::{QueryKind, QueryOptions, QueryReport, build_query_report};

pub const DOCTOR_REPORT_SCHEMA: &str = "sley.doctor.report.v0";

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DoctorReport {
    pub schema: String,
    pub status: String,
    pub target: String,
    pub entry_module: Option<String>,
    pub summary: DoctorSummary,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<DoctorQuerySummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lint: Option<DoctorLintSummary>,
    pub next_actions: Vec<DoctorAction>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DoctorSummary {
    pub error_count: usize,
    pub warning_count: usize,
    pub module_count: usize,
    pub task_count: usize,
    pub call_count: usize,
    pub lint_finding_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DoctorQuerySummary {
    pub source_schema: String,
    pub kind: String,
    pub module_count: usize,
    pub task_count: usize,
    pub call_count: usize,
    pub entrypoints: Vec<String>,
    pub effectful_tasks: Vec<DoctorTaskRef>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DoctorTaskRef {
    pub id: String,
    pub qualified_name: String,
    pub effects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DoctorLintSummary {
    pub source_schema: String,
    pub status: String,
    pub rules: Vec<String>,
    pub finding_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DoctorAction {
    pub kind: String,
    pub reason: String,
    pub command: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_command: Option<Vec<String>>,
}

pub fn build_doctor_report(
    target: impl Into<String>,
    program_result: Result<Program, Vec<Diagnostic>>,
    deny_warnings: bool,
) -> DoctorReport {
    let target = target.into();
    let program = match program_result {
        Ok(program) => program,
        Err(diagnostics) => {
            let summary = diagnostic_summary(&diagnostics);
            return DoctorReport {
                schema: DOCTOR_REPORT_SCHEMA.to_string(),
                status: "blocked".to_string(),
                target: target.clone(),
                entry_module: None,
                summary,
                diagnostics,
                query: None,
                lint: None,
                next_actions: blocked_actions(&target),
            };
        }
    };

    let diagnostics = check_program(&program);
    if has_errors(&diagnostics) {
        let summary = diagnostic_summary(&diagnostics);
        return DoctorReport {
            schema: DOCTOR_REPORT_SCHEMA.to_string(),
            status: "blocked".to_string(),
            target: target.clone(),
            entry_module: Some(program.module_name().to_string()),
            summary,
            diagnostics,
            query: None,
            lint: None,
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
    let lint_finding_count = lint_report.findings.len();
    let status = if lint_finding_count == 0 {
        "ready"
    } else if deny_warnings {
        "blocked"
    } else {
        "warnings"
    };
    let query = summarize_query(&query_report);
    let lint = summarize_lint(&lint_report);
    let summary = DoctorSummary {
        error_count: diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.is_error())
            .count(),
        warning_count: lint_finding_count,
        module_count: query.module_count,
        task_count: query.task_count,
        call_count: query.call_count,
        lint_finding_count,
    };
    let checked_lint_repair = single_checked_lint_repair(&target, &program, &lint_report);
    let next_actions = if lint_finding_count > 0 {
        lint_actions(&target, checked_lint_repair.as_ref(), &query_report)
    } else {
        ready_actions(&target, &query_report)
    };

    DoctorReport {
        schema: DOCTOR_REPORT_SCHEMA.to_string(),
        status: status.to_string(),
        target,
        entry_module: Some(program.module_name().to_string()),
        summary,
        diagnostics,
        query: Some(query),
        lint: Some(lint),
        next_actions,
    }
}

fn diagnostic_summary(diagnostics: &[Diagnostic]) -> DoctorSummary {
    DoctorSummary {
        error_count: diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.is_error())
            .count(),
        warning_count: diagnostics.len()
            - diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.is_error())
                .count(),
        module_count: 0,
        task_count: 0,
        call_count: 0,
        lint_finding_count: 0,
    }
}

fn summarize_query(report: &QueryReport) -> DoctorQuerySummary {
    let entry_task = format!("{}.main", report.entry_module);
    let entrypoints = report
        .tasks
        .iter()
        .filter(|task| task.qualified_name == entry_task || task.exported)
        .map(|task| task.qualified_name.clone())
        .collect();
    let effectful_tasks = report
        .tasks
        .iter()
        .filter(|task| !task.effects.is_empty())
        .map(|task| DoctorTaskRef {
            id: task.id.clone(),
            qualified_name: task.qualified_name.clone(),
            effects: task.effects.clone(),
        })
        .collect();

    DoctorQuerySummary {
        source_schema: report.schema.clone(),
        kind: report.kind.clone(),
        module_count: report.modules.len(),
        task_count: report.tasks.len(),
        call_count: report.calls.len(),
        entrypoints,
        effectful_tasks,
    }
}

fn summarize_lint(report: &LintReport) -> DoctorLintSummary {
    DoctorLintSummary {
        source_schema: report.schema.clone(),
        status: report.status.clone(),
        rules: report.filters.rules.clone(),
        finding_count: report.findings.len(),
    }
}

fn blocked_actions(target: &str) -> Vec<DoctorAction> {
    vec![DoctorAction {
        kind: "repair_diagnostics".to_string(),
        reason: "the target must pass strict checking before query, lint, graft, or run planning"
            .to_string(),
        command: command(["sley", "check", "--json", target]),
        write_command: None,
    }]
}

fn lint_actions(
    target: &str,
    checked_repair: Option<&CheckedLintRepair>,
    query: &QueryReport,
) -> Vec<DoctorAction> {
    let mut actions = vec![
        DoctorAction {
            kind: "repair_lint_findings".to_string(),
            reason: "warning-grade hygiene findings should be resolved or deliberately accepted"
                .to_string(),
            command: command(["sley", "lint", "--json", target]),
            write_command: None,
        },
        DoctorAction {
            kind: "plan_lint_repairs".to_string(),
            reason: "checked graft templates show which lint findings can be repaired structurally"
                .to_string(),
            command: command(["sley", "plan", "--json", "--graft-templates", target]),
            write_command: None,
        },
    ];
    if let Some(repair) = checked_repair {
        actions.push(DoctorAction {
            kind: "preview_lint_repair".to_string(),
            reason: "exactly one checked lint repair is available; preview it before writing"
                .to_string(),
            command: fix_command(target, repair, false),
            write_command: Some(fix_command(target, repair, true)),
        });
    }
    actions.push(DoctorAction {
        kind: "inspect_tasks".to_string(),
        reason: "task and call facts usually identify the narrowest edit surface".to_string(),
        command: command(["sley", "query", "--json", "--kind", "tasks", target]),
        write_command: None,
    });
    if !query.calls.is_empty() {
        actions.push(inspect_calls_action(target));
    }
    actions
}

fn fix_command(target: &str, repair: &CheckedLintRepair, write: bool) -> Vec<String> {
    vec![
        "sley".to_string(),
        "fix".to_string(),
        "--json".to_string(),
        "--kind".to_string(),
        repair.kind.clone(),
        "--template-surface".to_string(),
        repair.surface.clone(),
        if write { "--write" } else { "--dry-run" }.to_string(),
        target.to_string(),
    ]
}

fn ready_actions(target: &str, query: &QueryReport) -> Vec<DoctorAction> {
    let mut actions = vec![DoctorAction {
        kind: "inspect_tasks".to_string(),
        reason: "query task facts before planning edits".to_string(),
        command: command(["sley", "query", "--json", "--kind", "tasks", target]),
        write_command: None,
    }];
    if !query.calls.is_empty() {
        actions.push(inspect_calls_action(target));
    }
    actions.push(DoctorAction {
        kind: "lint_gate".to_string(),
        reason: "preserve the warning-grade lint gate after edits".to_string(),
        command: command(["sley", "lint", "--json", "--deny-warnings", target]),
        write_command: None,
    });

    let entry_task = format!("{}.main", query.entry_module);
    if let Some(task) = query
        .tasks
        .iter()
        .find(|task| task.qualified_name == entry_task)
    {
        let mut verify_command = vec![
            "sley".to_string(),
            "verify".to_string(),
            "--json".to_string(),
            "--deny-warnings".to_string(),
        ];
        let mut run_command = vec!["sley".to_string(), "run".to_string(), "--json".to_string()];
        for effect in &task.effects {
            verify_command.push("--cap".to_string());
            verify_command.push(effect.clone());
            run_command.push("--cap".to_string());
            run_command.push(effect.clone());
        }
        verify_command.push(target.to_string());
        run_command.push(target.to_string());
        actions.push(DoctorAction {
            kind: "verify_gate".to_string(),
            reason: if task.effects.is_empty() {
                "strict verification should pass before deploy, seal, or package handoff"
                    .to_string()
            } else {
                "strict verification should pass with explicit runtime gates before deploy, seal, or package handoff"
                    .to_string()
            },
            command: verify_command,
            write_command: None,
        });
        actions.push(DoctorAction {
            kind: if task.effects.is_empty() {
                "run_entrypoint".to_string()
            } else {
                "run_entrypoint_with_gates".to_string()
            },
            reason: if task.effects.is_empty() {
                "main is pure and can run without runtime gates".to_string()
            } else {
                "main declares effects, so runtime gates must be supplied explicitly".to_string()
            },
            command: run_command,
            write_command: None,
        });
    }

    actions
}

fn inspect_calls_action(target: &str) -> DoctorAction {
    DoctorAction {
        kind: "inspect_calls".to_string(),
        reason:
            "strict call rows show caller/callee edges before rename, arity, or authority edits"
                .to_string(),
        command: command(["sley", "query", "--json", "--kind", "calls", target]),
        write_command: None,
    }
}

fn command<const N: usize>(items: [&str; N]) -> Vec<String> {
    items.into_iter().map(str::to_string).collect()
}
