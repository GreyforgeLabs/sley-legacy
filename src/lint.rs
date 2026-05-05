use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::ast::Program;
use crate::authority::host_effects_for_callee;
use crate::query::{QueryKind, QueryOptions, build_query_report};
use crate::symbols::{
    EffectResolution, collect_task_calls, resolve_effect, task_fq_name, task_module,
};

pub const LINT_REPORT_SCHEMA: &str = "sley.lint.report.v0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LintRule {
    UnusedPrivateTask,
    UnreachablePrivateTask,
    UnusedDeclaredEffect,
    RawHostAdapter,
}

impl LintRule {
    pub fn all() -> Vec<Self> {
        vec![
            Self::UnusedPrivateTask,
            Self::UnreachablePrivateTask,
            Self::UnusedDeclaredEffect,
            Self::RawHostAdapter,
        ]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnusedPrivateTask => "unused_private_task",
            Self::UnreachablePrivateTask => "unreachable_private_task",
            Self::UnusedDeclaredEffect => "unused_declared_effect",
            Self::RawHostAdapter => "raw_host_adapter",
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
    if rules.contains(&LintRule::UnreachablePrivateTask) {
        findings.extend(lint_unreachable_private_tasks(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::UnusedDeclaredEffect) {
        findings.extend(lint_unused_declared_effects(
            program,
            options.module.as_deref(),
        ));
    }
    if rules.contains(&LintRule::RawHostAdapter) {
        findings.extend(lint_raw_host_adapters(program, options.module.as_deref()));
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

fn lint_unreachable_private_tasks(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let task_names = program
        .tasks
        .iter()
        .map(task_fq_name)
        .collect::<BTreeSet<_>>();
    let entry_task = format!("{}.main", program.module_name());
    let mut edges = BTreeMap::<String, Vec<String>>::new();
    let mut inbound_counts = BTreeMap::<String, usize>::new();

    for call in collect_task_calls(program) {
        let Some(target) = call.target else {
            continue;
        };
        if !task_names.contains(&target) {
            continue;
        }
        edges
            .entry(call.from.clone())
            .or_default()
            .push(target.clone());
        *inbound_counts.entry(target).or_default() += 1;
    }

    let mut reachable = BTreeSet::new();
    let mut pending = program
        .tasks
        .iter()
        .filter(|task| task.exported || task_fq_name(task) == entry_task)
        .map(task_fq_name)
        .collect::<Vec<_>>();

    while let Some(task) = pending.pop() {
        if !reachable.insert(task.clone()) {
            continue;
        }
        if let Some(targets) = edges.get(&task) {
            pending.extend(targets.iter().cloned());
        }
    }

    program
        .tasks
        .iter()
        .filter(|task| !task.exported)
        .map(|task| (task, task_fq_name(task)))
        .filter(|(task, _qualified_name)| module_matches(module, &task_module(task)))
        .filter(|(_task, qualified_name)| qualified_name != &entry_task)
        .filter(|(_task, qualified_name)| !reachable.contains(qualified_name))
        .filter(|(_task, qualified_name)| {
            inbound_counts
                .get(qualified_name)
                .copied()
                .unwrap_or_default()
                > 0
        })
        .map(|(task, qualified_name)| LintFinding {
            id: "UNREACHABLE_PRIVATE_TASK".to_string(),
            rule: LintRule::UnreachablePrivateTask.as_str().to_string(),
            severity: "warning".to_string(),
            message: format!(
                "private task `{qualified_name}` is not reachable from main or any exported task"
            ),
            node: task.id.clone(),
            module: task_module(task),
            hint:
                "call it from main or an exported task, export a reachable entrypoint, or delete it"
                    .to_string(),
        })
        .collect()
}

fn lint_unused_declared_effects(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    let task_effects = program
        .tasks
        .iter()
        .map(|task| {
            let task_module = task_module(task);
            (
                task_fq_name(task),
                task.effects
                    .iter()
                    .map(|effect| normalize_effect_name(program, &task_module, effect))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let calls = collect_task_calls(program);

    program
        .tasks
        .iter()
        .filter(|task| module_matches(module, &task_module(task)))
        .filter(|task| !task.effects.is_empty())
        .flat_map(|task| {
            let qualified_name = task_fq_name(task);
            let module_name = task_module(task);
            let declared = task
                .effects
                .iter()
                .map(|effect| {
                    (
                        effect.clone(),
                        normalize_effect_name(program, &module_name, effect),
                    )
                })
                .collect::<Vec<_>>();
            let declared_normalized = declared
                .iter()
                .map(|(_raw, normalized)| normalized.clone())
                .collect::<BTreeSet<_>>();
            let mut used = BTreeSet::new();
            for call in calls.iter().filter(|call| call.from == qualified_name) {
                if let Some(effects) = host_effects_for_callee(&call.callee) {
                    for effect in effects {
                        if declared_normalized.contains(*effect) {
                            used.insert((*effect).to_string());
                        }
                    }
                }
                if let Some(target) = &call.target
                    && let Some(effects) = task_effects.get(target)
                {
                    for effect in effects {
                        if declared_normalized.contains(effect) {
                            used.insert(effect.clone());
                        }
                    }
                }
            }
            declared
                .into_iter()
                .filter(move |(_effect, normalized)| !used.contains(normalized))
                .map(move |(effect, _normalized)| LintFinding {
                    id: "UNUSED_DECLARED_EFFECT".to_string(),
                    rule: LintRule::UnusedDeclaredEffect.as_str().to_string(),
                    severity: "warning".to_string(),
                    message: format!(
                        "task `{qualified_name}` declares effect `{effect}` but no checked call uses it"
                    ),
                    node: task.id.clone(),
                    module: module_name.clone(),
                    hint: format!(
                        "remove `{effect}` from the task uses list, or add a real checked call that requires it"
                    ),
                })
        })
        .collect()
}

fn module_matches(filter: Option<&str>, module: &str) -> bool {
    filter.is_none_or(|filter| filter == module)
}

fn lint_raw_host_adapters(program: &Program, module: Option<&str>) -> Vec<LintFinding> {
    collect_task_calls(program)
        .into_iter()
        .filter(|call| module_matches(module, &call.from_module))
        .filter_map(|call| {
            let replacement = raw_host_adapter_replacement(&call.callee)?;
            Some(LintFinding {
                id: "RAW_HOST_ADAPTER".to_string(),
                rule: LintRule::RawHostAdapter.as_str().to_string(),
                severity: "warning".to_string(),
                message: format!(
                    "host call `{}` uses a legacy diagnostic-failing adapter; prefer fallible `{replacement}`",
                    call.callee
                ),
                node: call.expr_id,
                module: call.from_module,
                hint: format!(
                    "replace `{}` with `{replacement}` and handle the Result with `?` inside a Result-returning task",
                    call.callee
                ),
            })
        })
        .collect()
}

fn raw_host_adapter_replacement(callee: &str) -> Option<&'static str> {
    match callee {
        "fs.read_text" => Some("fs.try_read_text"),
        "fs.write_text" => Some("fs.try_write_text"),
        "db.query_one" => Some("db.try_query_one"),
        "db.query" => Some("db.try_query"),
        _ => None,
    }
}

fn normalize_effect_name(program: &Program, module: &str, effect: &str) -> String {
    match resolve_effect(program, module, effect) {
        EffectResolution::Builtin(name) => name,
        EffectResolution::Resolved { fq_name, .. } => fq_name,
        EffectResolution::Unknown
        | EffectResolution::Ambiguous(_)
        | EffectResolution::Private(_) => effect.to_string(),
    }
}
