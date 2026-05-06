use serde::Serialize;

use crate::Program;
use crate::checker::{check_program, has_errors};
use crate::diagnostics::Diagnostic;
use crate::lint::{LintOptions, LintReport, build_lint_report};
use crate::plan::{CheckedLintRepair, single_checked_lint_repair};
use crate::query::{QueryKind, QueryOptions, QueryReport, build_query_report};
use crate::runtime::{RuntimeGates, Value, run_main, run_main_with_gates};

pub const VERIFY_REPORT_SCHEMA: &str = "sley.verify.report.v0";

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VerifyReport {
    pub schema: String,
    pub status: String,
    pub target: String,
    pub entry_module: Option<String>,
    pub summary: VerifySummary,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<VerifyQuerySummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lint: Option<VerifyLintSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<VerifyRuntimeSummary>,
    pub next_actions: Vec<VerifyAction>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VerifySummary {
    pub error_count: usize,
    pub warning_count: usize,
    pub module_count: usize,
    pub task_count: usize,
    pub call_count: usize,
    pub lint_finding_count: usize,
    pub runtime_status: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VerifyQuerySummary {
    pub source_schema: String,
    pub kind: String,
    pub module_count: usize,
    pub task_count: usize,
    pub call_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VerifyLintSummary {
    pub source_schema: String,
    pub status: String,
    pub rules: Vec<String>,
    pub finding_count: usize,
    pub findings: Vec<VerifyLintFinding>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VerifyLintFinding {
    pub id: String,
    pub rule: String,
    pub severity: String,
    pub message: String,
    pub node: String,
    pub module: String,
    pub hint: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct VerifyRuntimeSummary {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VerifyAction {
    pub kind: String,
    pub reason: String,
    pub command: Vec<String>,
}

pub fn build_verify_report(
    target: impl Into<String>,
    program_result: Result<Program, Vec<Diagnostic>>,
    gates: RuntimeGates,
    deny_warnings: bool,
) -> VerifyReport {
    let target = target.into();
    let program = match program_result {
        Ok(program) => program,
        Err(diagnostics) => {
            let actions = check_actions(&target);
            return blocked_report(
                target,
                None,
                diagnostics,
                None,
                None,
                VerifyRuntimeSummary::skipped(),
                actions,
            );
        }
    };

    let diagnostics = check_program(&program);
    if has_errors(&diagnostics) {
        let actions = check_actions(&target);
        return blocked_report(
            target,
            Some(program.module_name().to_string()),
            diagnostics,
            None,
            None,
            VerifyRuntimeSummary::skipped(),
            actions,
        );
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
    let lint_finding_count = lint.finding_count;
    let checked_lint_repair = single_checked_lint_repair(&target, &program, &lint_report);

    if deny_warnings && lint_finding_count > 0 {
        let actions = lint_actions(&target, checked_lint_repair.as_ref());
        return blocked_report(
            target,
            Some(program.module_name().to_string()),
            Vec::new(),
            Some(query),
            Some(lint),
            VerifyRuntimeSummary::skipped(),
            actions,
        );
    }

    let runtime = match run_entrypoint(&program, &gates) {
        Ok(value) => VerifyRuntimeSummary {
            status: "passed".to_string(),
            value: Some(value),
            diagnostics: Vec::new(),
        },
        Err(diagnostics) => {
            let actions = runtime_actions(&target);
            return blocked_report(
                target,
                Some(program.module_name().to_string()),
                diagnostics.clone(),
                Some(query),
                Some(lint),
                VerifyRuntimeSummary {
                    status: "failed".to_string(),
                    value: None,
                    diagnostics,
                },
                actions,
            );
        }
    };

    let status = if lint_finding_count > 0 {
        "warnings"
    } else {
        "passed"
    };
    let actions = if lint_finding_count > 0 {
        lint_actions(&target, checked_lint_repair.as_ref())
    } else {
        passed_actions(&target)
    };
    let summary = build_summary(&[], Some(&query), Some(&lint), &runtime);

    VerifyReport {
        schema: VERIFY_REPORT_SCHEMA.to_string(),
        status: status.to_string(),
        target,
        entry_module: Some(program.module_name().to_string()),
        summary,
        diagnostics: Vec::new(),
        query: Some(query),
        lint: Some(lint),
        runtime: Some(runtime),
        next_actions: actions,
    }
}

impl VerifyRuntimeSummary {
    fn skipped() -> Self {
        Self {
            status: "skipped".to_string(),
            value: None,
            diagnostics: Vec::new(),
        }
    }
}

fn run_entrypoint(program: &Program, gates: &RuntimeGates) -> Result<Value, Vec<Diagnostic>> {
    if gates.is_empty() {
        run_main(program)
    } else {
        run_main_with_gates(program, gates)
    }
}

fn blocked_report(
    target: String,
    entry_module: Option<String>,
    diagnostics: Vec<Diagnostic>,
    query: Option<VerifyQuerySummary>,
    lint: Option<VerifyLintSummary>,
    runtime: VerifyRuntimeSummary,
    actions: Vec<VerifyAction>,
) -> VerifyReport {
    let summary = build_summary(&diagnostics, query.as_ref(), lint.as_ref(), &runtime);
    VerifyReport {
        schema: VERIFY_REPORT_SCHEMA.to_string(),
        status: "blocked".to_string(),
        target: target.clone(),
        entry_module,
        summary,
        diagnostics,
        query,
        lint,
        runtime: Some(runtime),
        next_actions: actions,
    }
}

fn build_summary(
    diagnostics: &[Diagnostic],
    query: Option<&VerifyQuerySummary>,
    lint: Option<&VerifyLintSummary>,
    runtime: &VerifyRuntimeSummary,
) -> VerifySummary {
    let error_count = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.is_error())
        .count();
    let diagnostic_warning_count = diagnostics.len() - error_count;
    VerifySummary {
        error_count,
        warning_count: diagnostic_warning_count + lint.map_or(0, |lint| lint.finding_count),
        module_count: query.map_or(0, |query| query.module_count),
        task_count: query.map_or(0, |query| query.task_count),
        call_count: query.map_or(0, |query| query.call_count),
        lint_finding_count: lint.map_or(0, |lint| lint.finding_count),
        runtime_status: runtime.status.clone(),
    }
}

fn summarize_query(report: &QueryReport) -> VerifyQuerySummary {
    VerifyQuerySummary {
        source_schema: report.schema.clone(),
        kind: report.kind.clone(),
        module_count: report.modules.len(),
        task_count: report.tasks.len(),
        call_count: report.calls.len(),
    }
}

fn summarize_lint(report: &LintReport) -> VerifyLintSummary {
    VerifyLintSummary {
        source_schema: report.schema.clone(),
        status: report.status.clone(),
        rules: report.filters.rules.clone(),
        finding_count: report.findings.len(),
        findings: report
            .findings
            .iter()
            .map(|finding| VerifyLintFinding {
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

fn check_actions(target: &str) -> Vec<VerifyAction> {
    vec![VerifyAction {
        kind: "repair_diagnostics".to_string(),
        reason: "strict checking must pass before lint or runtime verification".to_string(),
        command: command(["sley", "check", "--json", target]),
    }]
}

fn lint_actions(target: &str, checked_repair: Option<&CheckedLintRepair>) -> Vec<VerifyAction> {
    let mut actions = vec![
        VerifyAction {
            kind: "repair_lint_findings".to_string(),
            reason: "warning-grade lint findings should be resolved before deployment review"
                .to_string(),
            command: command(["sley", "lint", "--json", target]),
        },
        VerifyAction {
            kind: "plan_lint_repairs".to_string(),
            reason: "checked graft templates show which lint findings can be repaired structurally"
                .to_string(),
            command: command(["sley", "plan", "--json", "--graft-templates", target]),
        },
    ];
    if let Some(repair) = checked_repair {
        actions.push(VerifyAction {
            kind: "preview_lint_repair".to_string(),
            reason: "exactly one checked lint repair is available; preview it before writing"
                .to_string(),
            command: fix_command(target, repair),
        });
    }
    actions.push(VerifyAction {
        kind: "inspect_tasks".to_string(),
        reason: "query task and call facts before planning a repair".to_string(),
        command: command(["sley", "query", "--json", "--kind", "tasks", target]),
    });
    actions
}

fn fix_command(target: &str, repair: &CheckedLintRepair) -> Vec<String> {
    vec![
        "sley".to_string(),
        "fix".to_string(),
        "--json".to_string(),
        "--kind".to_string(),
        repair.kind.clone(),
        "--template-surface".to_string(),
        repair.surface.clone(),
        "--dry-run".to_string(),
        target.to_string(),
    ]
}

fn runtime_actions(target: &str) -> Vec<VerifyAction> {
    vec![VerifyAction {
        kind: "repair_runtime_gate".to_string(),
        reason: "rerun with the required deterministic runtime gates and seeds".to_string(),
        command: command(["sley", "run", "--json", target]),
    }]
}

fn passed_actions(target: &str) -> Vec<VerifyAction> {
    vec![VerifyAction {
        kind: "seal_verified_target".to_string(),
        reason: "create a content-addressed review artifact after verification".to_string(),
        command: command(["sley", "seal", "--json", target]),
    }]
}

fn command<const N: usize>(items: [&str; N]) -> Vec<String> {
    items.into_iter().map(str::to_string).collect()
}
