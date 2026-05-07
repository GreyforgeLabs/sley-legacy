use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::Program;
use crate::ast::{Block, Expr, ExprKind, Statement, StatementKind, TaskDecl};
use crate::checker::{check_program, has_errors};
use crate::diagnostics::Diagnostic;
use crate::lint::{LintFinding, LintOptions, LintReport, build_lint_report};
use crate::plan::{CheckedLintRepair, single_checked_lint_repair};
use crate::query::{QueryKind, QueryOptions, QueryReport, build_query_report};
use crate::symbols::{TaskResolution, callee_path, resolve_task};

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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub findings: Vec<LintFinding>,
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
        ready_actions(&target, &query_report, &program)
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
        findings: report.findings.clone(),
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

fn ready_actions(target: &str, query: &QueryReport, program: &Program) -> Vec<DoctorAction> {
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
        let seed_args = inferred_runtime_seed_args(program, &entry_task);
        verify_command.extend(seed_args.iter().cloned());
        run_command.extend(seed_args);
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeSeedArg {
    flag: &'static str,
    key: String,
    value: String,
}

fn inferred_runtime_seed_args(program: &Program, entry_task: &str) -> Vec<String> {
    let Some(task) = find_task_by_qualified_name(program, entry_task) else {
        return Vec::new();
    };
    let mut seeds = Vec::new();
    let mut seen = BTreeSet::new();
    let mut visited = BTreeSet::new();
    collect_task_runtime_seeds(
        program,
        task,
        &BTreeMap::new(),
        &mut seeds,
        &mut seen,
        &mut visited,
    );
    seeds
        .into_iter()
        .flat_map(|seed: RuntimeSeedArg| {
            [seed.flag.to_string(), seed.key, seed.value]
                .into_iter()
                .collect::<Vec<_>>()
        })
        .collect()
}

fn collect_task_runtime_seeds(
    program: &Program,
    task: &TaskDecl,
    env: &BTreeMap<String, String>,
    seeds: &mut Vec<RuntimeSeedArg>,
    seen: &mut BTreeSet<(String, String)>,
    visited: &mut BTreeSet<String>,
) {
    let task_name = task_qualified_name(program, task);
    let visit_key = task_visit_key(&task_name, env);
    if !visited.insert(visit_key) {
        return;
    }
    collect_block_runtime_seeds(program, task, &task.body, env, seeds, seen, visited);
}

fn collect_block_runtime_seeds(
    program: &Program,
    task: &TaskDecl,
    block: &Block,
    env: &BTreeMap<String, String>,
    seeds: &mut Vec<RuntimeSeedArg>,
    seen: &mut BTreeSet<(String, String)>,
    visited: &mut BTreeSet<String>,
) {
    for statement in &block.statements {
        collect_statement_runtime_seeds(program, task, statement, env, seeds, seen, visited);
    }
}

fn collect_statement_runtime_seeds(
    program: &Program,
    task: &TaskDecl,
    statement: &Statement,
    env: &BTreeMap<String, String>,
    seeds: &mut Vec<RuntimeSeedArg>,
    seen: &mut BTreeSet<(String, String)>,
    visited: &mut BTreeSet<String>,
) {
    match &statement.kind {
        StatementKind::Binding { expr, .. }
        | StatementKind::Set { expr, .. }
        | StatementKind::Return { expr }
        | StatementKind::Expr { expr } => {
            collect_expr_runtime_seeds(program, task, expr, env, seeds, seen, visited);
        }
        StatementKind::If {
            condition,
            then_block,
            else_block,
        } => {
            collect_expr_runtime_seeds(program, task, condition, env, seeds, seen, visited);
            collect_block_runtime_seeds(program, task, then_block, env, seeds, seen, visited);
            if let Some(else_block) = else_block {
                collect_block_runtime_seeds(program, task, else_block, env, seeds, seen, visited);
            }
        }
        StatementKind::While { condition, body } => {
            collect_expr_runtime_seeds(program, task, condition, env, seeds, seen, visited);
            collect_block_runtime_seeds(program, task, body, env, seeds, seen, visited);
        }
        StatementKind::For {
            collection, body, ..
        } => {
            collect_expr_runtime_seeds(program, task, collection, env, seeds, seen, visited);
            collect_block_runtime_seeds(program, task, body, env, seeds, seen, visited);
        }
        StatementKind::Forge { body } => {
            collect_block_runtime_seeds(program, task, body, env, seeds, seen, visited);
        }
    }
}

fn collect_expr_runtime_seeds(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    env: &BTreeMap<String, String>,
    seeds: &mut Vec<RuntimeSeedArg>,
    seen: &mut BTreeSet<(String, String)>,
    visited: &mut BTreeSet<String>,
) {
    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            collect_expr_runtime_seeds(program, task, expr, env, seeds, seen, visited);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_expr_runtime_seeds(program, task, left, env, seeds, seen, visited);
            collect_expr_runtime_seeds(program, task, right, env, seeds, seen, visited);
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_expr_runtime_seeds(program, task, condition, env, seeds, seen, visited);
            collect_expr_runtime_seeds(program, task, then_branch, env, seeds, seen, visited);
            collect_expr_runtime_seeds(program, task, else_branch, env, seeds, seen, visited);
        }
        ExprKind::Call { callee, args } => {
            if let Some(callee_name) = callee_path(callee).as_deref() {
                if let Some(seed) = host_seed_arg(callee_name, args, env) {
                    push_runtime_seed(seed, seeds, seen);
                } else if let Some(target_task) = resolve_internal_task(program, task, callee_name)
                {
                    let call_env = call_text_env(target_task, args, env);
                    collect_task_runtime_seeds(
                        program,
                        target_task,
                        &call_env,
                        seeds,
                        seen,
                        visited,
                    );
                }
            }
            collect_expr_runtime_seeds(program, task, callee, env, seeds, seen, visited);
            for arg in args {
                collect_expr_runtime_seeds(program, task, arg, env, seeds, seen, visited);
            }
        }
        ExprKind::ListLiteral { items } => {
            for item in items {
                collect_expr_runtime_seeds(program, task, item, env, seeds, seen, visited);
            }
        }
        ExprKind::MapLiteral { entries } => {
            for entry in entries {
                collect_expr_runtime_seeds(program, task, &entry.key, env, seeds, seen, visited);
                collect_expr_runtime_seeds(program, task, &entry.value, env, seeds, seen, visited);
            }
        }
        ExprKind::Index { collection, index } => {
            collect_expr_runtime_seeds(program, task, collection, env, seeds, seen, visited);
            collect_expr_runtime_seeds(program, task, index, env, seeds, seen, visited);
        }
        ExprKind::FieldAccess { receiver, .. } => {
            collect_expr_runtime_seeds(program, task, receiver, env, seeds, seen, visited);
        }
        ExprKind::RecordLiteral { fields, .. } => {
            for field in fields {
                collect_expr_runtime_seeds(program, task, &field.expr, env, seeds, seen, visited);
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

fn host_seed_arg(
    callee_name: &str,
    args: &[Expr],
    env: &BTreeMap<String, String>,
) -> Option<RuntimeSeedArg> {
    let key = text_seed_key(args.first()?, env)?;
    let (flag, value) = match callee_name {
        "secrets.try_get" => ("--secret", "redacted".to_string()),
        "http.try_get_text" => ("--http-text", default_http_seed_value(&key)),
        "shell.try_run" => ("--shell-output", "ok".to_string()),
        "model.try_complete" => ("--model-output", default_model_seed_value(&key)),
        "deploy.try_stage" => ("--deploy-result", "staged".to_string()),
        "spend.try_authorize" => ("--spend-result", "authorized".to_string()),
        _ => return None,
    };
    Some(RuntimeSeedArg { flag, key, value })
}

fn default_http_seed_value(url: &str) -> String {
    if url.contains("health") {
        "service ready".to_string()
    } else if url.contains("profile") {
        "profile ready".to_string()
    } else {
        "response ready".to_string()
    }
}

fn default_model_seed_value(prompt: &str) -> String {
    if prompt == "deploy-plan" {
        "plan approved".to_string()
    } else {
        "model ready".to_string()
    }
}

fn push_runtime_seed(
    seed: RuntimeSeedArg,
    seeds: &mut Vec<RuntimeSeedArg>,
    seen: &mut BTreeSet<(String, String)>,
) {
    if seen.insert((seed.flag.to_string(), seed.key.clone())) {
        seeds.push(seed);
    }
}

fn resolve_internal_task<'a>(
    program: &'a Program,
    caller: &TaskDecl,
    callee_name: &str,
) -> Option<&'a TaskDecl> {
    let caller_module = caller
        .module
        .as_deref()
        .unwrap_or_else(|| program.module_name());
    let TaskResolution::Resolved { fq_name, .. } =
        resolve_task(program, caller_module, callee_name)
    else {
        return None;
    };
    find_task_by_qualified_name(program, &fq_name)
}

fn call_text_env(
    task: &TaskDecl,
    args: &[Expr],
    parent_env: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    task.takes
        .iter()
        .zip(args)
        .filter_map(|(take, arg)| {
            text_seed_key(arg, parent_env).map(|value| (take.name.clone(), value))
        })
        .collect()
}

fn text_seed_key(expr: &Expr, env: &BTreeMap<String, String>) -> Option<String> {
    match &expr.kind {
        ExprKind::StringLiteral { value } => Some(value.clone()),
        ExprKind::Identifier { name } => env.get(name).cloned(),
        _ => None,
    }
}

fn find_task_by_qualified_name<'a>(
    program: &'a Program,
    qualified_name: &str,
) -> Option<&'a TaskDecl> {
    program
        .tasks
        .iter()
        .find(|task| task_qualified_name(program, task) == qualified_name)
}

fn task_qualified_name(program: &Program, task: &TaskDecl) -> String {
    format!(
        "{}.{}",
        task.module
            .as_deref()
            .unwrap_or_else(|| program.module_name()),
        task.name
    )
}

fn task_visit_key(task_name: &str, env: &BTreeMap<String, String>) -> String {
    let env_key = env
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join(",");
    format!("{task_name}|{env_key}")
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
