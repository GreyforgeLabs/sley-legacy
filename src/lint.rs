use std::collections::BTreeSet;

use serde::Serialize;

use crate::ast::Program;
use crate::query::{QueryKind, QueryOptions, build_query_report};

pub const LINT_REPORT_SCHEMA: &str = "sley.lint.report.v0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LintRule {
    UnusedPrivateTask,
}

impl LintRule {
    pub fn all() -> Vec<Self> {
        vec![Self::UnusedPrivateTask]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnusedPrivateTask => "unused_private_task",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LintOptions {
    pub rules: Vec<LintRule>,
    pub module: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LintReport {
    pub schema: String,
    pub status: String,
    pub entry_module: String,
    pub filters: LintFilters,
    pub findings: Vec<LintFinding>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LintFilters {
    pub module: Option<String>,
    pub rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LintFinding {
    pub id: String,
    pub rule: String,
    pub severity: String,
    pub message: String,
    pub node: String,
    pub module: String,
    pub hint: String,
}

pub fn build_lint_report(program: &Program, options: LintOptions) -> LintReport {
    let rules = selected_rules(options.rules);
    let mut findings = Vec::new();

    if rules.contains(&LintRule::UnusedPrivateTask) {
        findings.extend(lint_unused_private_tasks(
            program,
            options.module.as_deref(),
        ));
    }

    findings.sort_by(|left, right| {
        left.rule
            .cmp(&right.rule)
            .then_with(|| left.node.cmp(&right.node))
    });
    let status = if findings.is_empty() {
        "ok"
    } else {
        "findings"
    };

    LintReport {
        schema: LINT_REPORT_SCHEMA.to_string(),
        status: status.to_string(),
        entry_module: program.module_name().to_string(),
        filters: LintFilters {
            module: options.module,
            rules: rules
                .into_iter()
                .map(LintRule::as_str)
                .map(str::to_string)
                .collect(),
        },
        findings,
    }
}

fn selected_rules(rules: Vec<LintRule>) -> Vec<LintRule> {
    if rules.is_empty() {
        return LintRule::all();
    }
    rules
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn lint_unused_private_tasks(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let report = build_query_report(
        program,
        QueryOptions {
            kind: QueryKind::Tasks,
            module: module.map(str::to_string),
            exported_only: false,
        },
    );
    let entry_task = format!("{}.main", program.module_name());

    report
        .tasks
        .into_iter()
        .filter(|task| !task.exported)
        .filter(|task| task.inbound_call_count == 0)
        .filter(|task| task.qualified_name != entry_task)
        .map(|task| LintFinding {
            id: "UNUSED_PRIVATE_TASK".to_string(),
            rule: LintRule::UnusedPrivateTask.as_str().to_string(),
            severity: "warning".to_string(),
            message: format!(
                "private task `{}` is not called by any checked task",
                task.qualified_name
            ),
            node: task.id,
            module: task.module,
            hint: "call it, export it, or delete it".to_string(),
        })
        .collect()
}
