use std::collections::{BTreeMap, BTreeSet};

use crate::Program;
use crate::ast::{Block, Expr, ExprKind, Statement, StatementKind, TaskDecl};
use crate::symbols::{TaskResolution, callee_path, resolve_task};

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeSeedArg {
    flag: &'static str,
    key: String,
    value: String,
}

pub fn cap_args(effects: &[String]) -> Vec<String> {
    let mut args = Vec::with_capacity(effects.len() * 2);
    for effect in effects {
        args.push("--cap".to_string());
        args.push(effect.clone());
    }
    args
}

pub fn inferred_runtime_seed_args(program: &Program, entry_task: &str) -> Vec<String> {
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
