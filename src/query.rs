use std::collections::BTreeMap;

use serde::Serialize;

use crate::ast::{Program, TaskDecl, TypeExpr};
use crate::symbols::{
    ModuleSymbolSummary, TaskCallSummary, build_symbol_graph, collect_task_calls, task_fq_name,
    task_module,
};

pub const QUERY_REPORT_SCHEMA: &str = "sley.query.report.v0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryKind {
    All,
    Modules,
    Tasks,
    Calls,
}

impl QueryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Modules => "modules",
            Self::Tasks => "tasks",
            Self::Calls => "calls",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryOptions {
    pub kind: QueryKind,
    pub module: Option<String>,
    pub exported_only: bool,
}

impl Default for QueryOptions {
    fn default() -> Self {
        Self {
            kind: QueryKind::All,
            module: None,
            exported_only: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct QueryReport {
    pub schema: String,
    pub kind: String,
    pub entry_module: String,
    pub filters: QueryFilters,
    pub modules: Vec<ModuleSymbolSummary>,
    pub tasks: Vec<QueryTaskSummary>,
    pub calls: Vec<TaskCallSummary>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct QueryFilters {
    pub module: Option<String>,
    pub exported_only: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct QueryTaskSummary {
    pub id: String,
    pub name: String,
    pub module: String,
    pub qualified_name: String,
    pub exported: bool,
    pub takes: Vec<QueryTakeSummary>,
    pub return_type: String,
    pub effects: Vec<String>,
    pub outbound_call_count: usize,
    pub inbound_call_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct QueryTakeSummary {
    pub name: String,
    pub binding_kind: String,
    #[serde(rename = "type")]
    pub ty: String,
}

pub fn build_query_report(program: &Program, options: QueryOptions) -> QueryReport {
    let graph = build_symbol_graph(program);
    let calls = collect_task_calls(program);
    let outbound_counts = count_outbound_calls(&calls);
    let inbound_counts = count_inbound_calls(&calls);
    let module_filter = options.module.as_deref();

    let modules = if matches!(options.kind, QueryKind::All | QueryKind::Modules) {
        graph
            .modules
            .into_iter()
            .filter(|module| module_matches(module_filter, &module.module))
            .map(|module| filter_module_summary(module, options.exported_only))
            .collect()
    } else {
        Vec::new()
    };

    let tasks = if matches!(options.kind, QueryKind::All | QueryKind::Tasks) {
        let mut tasks = program
            .tasks
            .iter()
            .filter(|task| module_matches(module_filter, &task_module(task)))
            .filter(|task| !options.exported_only || task.exported)
            .map(|task| summarize_task(task, &outbound_counts, &inbound_counts))
            .collect::<Vec<_>>();
        tasks.sort_by(|left, right| {
            left.module
                .cmp(&right.module)
                .then_with(|| left.name.cmp(&right.name))
        });
        tasks
    } else {
        Vec::new()
    };

    let calls = if matches!(options.kind, QueryKind::All | QueryKind::Calls) {
        let mut calls = calls
            .into_iter()
            .filter(|call| call_matches_module(module_filter, call))
            .collect::<Vec<_>>();
        calls.sort_by(|left, right| {
            left.from
                .cmp(&right.from)
                .then_with(|| left.expr_id.cmp(&right.expr_id))
                .then_with(|| left.callee.cmp(&right.callee))
        });
        calls
    } else {
        Vec::new()
    };

    QueryReport {
        schema: QUERY_REPORT_SCHEMA.to_string(),
        kind: options.kind.as_str().to_string(),
        entry_module: program.module_name().to_string(),
        filters: QueryFilters {
            module: options.module,
            exported_only: options.exported_only,
        },
        modules,
        tasks,
        calls,
    }
}

fn summarize_task(
    task: &TaskDecl,
    outbound_counts: &BTreeMap<String, usize>,
    inbound_counts: &BTreeMap<String, usize>,
) -> QueryTaskSummary {
    let qualified_name = task_fq_name(task);
    QueryTaskSummary {
        id: task.id.clone(),
        name: task.name.clone(),
        module: task_module(task),
        qualified_name: qualified_name.clone(),
        exported: task.exported,
        takes: task
            .takes
            .iter()
            .map(|take| QueryTakeSummary {
                name: take.name.clone(),
                binding_kind: take.binding_kind.as_source_keyword().to_string(),
                ty: take.ty.display(),
            })
            .collect(),
        return_type: display_type(&task.return_type),
        effects: task.effects.clone(),
        outbound_call_count: outbound_counts
            .get(&qualified_name)
            .copied()
            .unwrap_or_default(),
        inbound_call_count: inbound_counts
            .get(&qualified_name)
            .copied()
            .unwrap_or_default(),
    }
}

fn display_type(ty: &TypeExpr) -> String {
    ty.display()
}

fn filter_module_summary(
    mut module: ModuleSymbolSummary,
    exported_only: bool,
) -> ModuleSymbolSummary {
    if exported_only {
        module.types.retain(|decl| decl.exported);
        module.effects.retain(|decl| decl.exported);
        module.tasks.retain(|decl| decl.exported);
    }
    module
}

fn count_outbound_calls(calls: &[TaskCallSummary]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for call in calls {
        *counts.entry(call.from.clone()).or_insert(0) += 1;
    }
    counts
}

fn count_inbound_calls(calls: &[TaskCallSummary]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for call in calls {
        if let Some(target) = &call.target {
            *counts.entry(target.clone()).or_insert(0) += 1;
        }
    }
    counts
}

fn module_matches(filter: Option<&str>, module: &str) -> bool {
    filter.is_none_or(|filter| filter == module)
}

fn call_matches_module(filter: Option<&str>, call: &TaskCallSummary) -> bool {
    let Some(filter) = filter else {
        return true;
    };
    call.from_module == filter
        || call
            .target
            .as_deref()
            .and_then(target_module)
            .is_some_and(|module| module == filter)
}

fn target_module(target: &str) -> Option<&str> {
    target.rsplit_once('.').map(|(module, _name)| module)
}
