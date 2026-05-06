use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::ast::{
    Block, EffectDecl, Expr, ExprKind, ImportDecl, Program, Statement, StatementKind, TaskDecl,
    TypeDecl,
};

pub const SYMBOL_GRAPH_SCHEMA: &str = "sley.symbol_graph.v0";
pub const SYMBOL_GRAPH_SLICE_SCHEMA: &str = "sley.symbol_graph.slice.v0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskResolution {
    Resolved { index: usize, fq_name: String },
    Unknown,
    Ambiguous(Vec<String>),
    Private(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeResolution {
    Builtin(String),
    Resolved { index: usize, fq_name: String },
    Unknown,
    Ambiguous(Vec<String>),
    Private(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectResolution {
    Builtin(String),
    Resolved { index: usize, fq_name: String },
    Unknown,
    Ambiguous(Vec<String>),
    Private(String),
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SymbolGraph {
    pub schema: String,
    pub entry_module: String,
    pub modules: Vec<ModuleSymbolSummary>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ModuleSymbolSummary {
    pub module: String,
    pub imports: Vec<ImportSymbolSummary>,
    pub types: Vec<DeclarationSymbolSummary>,
    pub effects: Vec<DeclarationSymbolSummary>,
    pub tasks: Vec<DeclarationSymbolSummary>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ImportSymbolSummary {
    pub id: String,
    pub module: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DeclarationSymbolSummary {
    pub name: String,
    pub id: String,
    pub exported: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SymbolGraphSlice {
    pub schema: String,
    pub target: String,
    pub entry_module: String,
    pub focus: SliceFocus,
    pub imports: Vec<ImportSymbolSummary>,
    pub types: Vec<DeclarationSymbolSummary>,
    pub effects: Vec<DeclarationSymbolSummary>,
    pub tasks: Vec<DeclarationSymbolSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<TaskDecl>,
    pub move_affordances: Vec<MoveNodeAffordance>,
    pub outbound_calls: Vec<TaskCallSummary>,
    pub inbound_calls: Vec<TaskCallSummary>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SliceFocus {
    pub kind: String,
    pub id: String,
    pub module: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TaskCallSummary {
    pub from: String,
    pub from_module: String,
    pub expr_id: String,
    pub source: String,
    pub callee: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MoveNodeAffordance {
    pub target: String,
    pub target_kind: String,
    pub parent: String,
    pub position: usize,
    pub max_position: usize,
    pub destinations: Vec<MoveNodeDestination>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MoveNodeDestination {
    pub parent: String,
    pub max_position: usize,
}

pub fn build_symbol_graph(program: &Program) -> SymbolGraph {
    let mut modules = BTreeSet::new();
    modules.insert(program.module_name().to_string());
    for import in &program.imports {
        modules.insert(import_owner_module(import));
        modules.insert(import.module.clone());
    }
    for ty in &program.types {
        modules.insert(type_module(ty));
    }
    for effect in &program.effects {
        modules.insert(effect_module(effect));
    }
    for task in &program.tasks {
        modules.insert(task_module(task));
    }

    let mut module_summaries = modules
        .into_iter()
        .map(|module| ModuleSymbolSummary {
            module,
            imports: Vec::new(),
            types: Vec::new(),
            effects: Vec::new(),
            tasks: Vec::new(),
        })
        .collect::<Vec<_>>();
    let mut indexes = module_summaries
        .iter()
        .enumerate()
        .map(|(index, module)| (module.module.clone(), index))
        .collect::<BTreeMap<_, _>>();

    for import in &program.imports {
        let owner = import_owner_module(import);
        if let Some(index) = indexes.get(&owner).copied() {
            module_summaries[index].imports.push(ImportSymbolSummary {
                id: import.id.clone(),
                module: import.module.clone(),
                alias: import.alias.clone(),
            });
        }
    }
    for ty in &program.types {
        push_decl(
            &mut module_summaries,
            &mut indexes,
            &type_module(ty),
            DeclarationSymbolSummary {
                name: ty.name.clone(),
                id: ty.id.clone(),
                exported: ty.exported,
            },
            DeclarationKind::Type,
        );
    }
    for effect in &program.effects {
        push_decl(
            &mut module_summaries,
            &mut indexes,
            &effect_module(effect),
            DeclarationSymbolSummary {
                name: effect.name.clone(),
                id: effect.id.clone(),
                exported: effect.exported,
            },
            DeclarationKind::Effect,
        );
    }
    for task in &program.tasks {
        push_decl(
            &mut module_summaries,
            &mut indexes,
            &task_module(task),
            DeclarationSymbolSummary {
                name: task.name.clone(),
                id: task.id.clone(),
                exported: task.exported,
            },
            DeclarationKind::Task,
        );
    }

    for module in &mut module_summaries {
        module.imports.sort_by(|left, right| {
            left.module
                .cmp(&right.module)
                .then_with(|| left.alias.cmp(&right.alias))
                .then_with(|| left.id.cmp(&right.id))
        });
        module
            .types
            .sort_by(|left, right| left.name.cmp(&right.name));
        module
            .effects
            .sort_by(|left, right| left.name.cmp(&right.name));
        module
            .tasks
            .sort_by(|left, right| left.name.cmp(&right.name));
    }

    SymbolGraph {
        schema: SYMBOL_GRAPH_SCHEMA.to_string(),
        entry_module: program.module_name().to_string(),
        modules: module_summaries,
    }
}

pub fn slice_symbol_graph(program: &Program, target: &str) -> Option<SymbolGraphSlice> {
    if let Some(index) = program.find_task_index(target) {
        let task = &program.tasks[index];
        return Some(build_slice(
            program,
            target,
            SliceFocus {
                kind: "task".to_string(),
                id: task.id.clone(),
                module: task_module(task),
                name: task.name.clone(),
            },
            Some(index),
        ));
    }

    let module = target.strip_prefix("module:").unwrap_or(target);
    if module_exists(program, module) {
        return Some(build_slice(
            program,
            target,
            SliceFocus {
                kind: "module".to_string(),
                id: format!("module:{module}"),
                module: module.to_string(),
                name: module.to_string(),
            },
            None,
        ));
    }

    if let Some(index) = find_type_index(program, target) {
        let ty = &program.types[index];
        return Some(build_slice(
            program,
            target,
            SliceFocus {
                kind: "type".to_string(),
                id: ty.id.clone(),
                module: type_module(ty),
                name: ty.name.clone(),
            },
            None,
        ));
    }

    if let Some(index) = find_effect_index(program, target) {
        let effect = &program.effects[index];
        return Some(build_slice(
            program,
            target,
            SliceFocus {
                kind: "effect".to_string(),
                id: effect.id.clone(),
                module: effect_module(effect),
                name: effect.name.clone(),
            },
            None,
        ));
    }

    if let Some(index) = find_import_index(program, target) {
        let import = &program.imports[index];
        return Some(build_slice(
            program,
            target,
            SliceFocus {
                kind: "import".to_string(),
                id: import.id.clone(),
                module: import_owner_module(import),
                name: import.module.clone(),
            },
            None,
        ));
    }

    None
}

pub fn collect_task_calls(program: &Program) -> Vec<TaskCallSummary> {
    let mut calls = Vec::new();
    for task in &program.tasks {
        collect_task_calls_from_block(program, task, &task.body, &mut calls);
    }
    calls
}

pub fn resolve_task(program: &Program, caller_module: &str, path: &str) -> TaskResolution {
    let parts = path
        .split('.')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let Some(last) = parts.last().copied() else {
        return TaskResolution::Unknown;
    };

    if parts.len() == 1 {
        if let Some(index) = find_task_in_module(program, caller_module, last) {
            return resolved_task(program, index);
        }

        let mut matches = Vec::new();
        for import in imports_for_module(program, caller_module) {
            if let Some(index) = find_task_in_module(program, &import.module, last)
                && program.tasks[index].exported
            {
                matches.push(index);
            }
        }
        matches.sort_unstable();
        matches.dedup();
        return match matches.as_slice() {
            [index] => resolved_task(program, *index),
            [] => TaskResolution::Unknown,
            _ => TaskResolution::Ambiguous(
                matches
                    .into_iter()
                    .map(|index| task_fq_name(&program.tasks[index]))
                    .collect(),
            ),
        };
    }

    let module_path = parts[..parts.len() - 1].join(".");
    if module_exists(program, &module_path) {
        return resolve_task_in_module(program, caller_module, &module_path, last);
    }

    if parts.len() == 2 {
        let qualifier = parts[0];
        for import in imports_for_module(program, caller_module) {
            if import_matches_qualifier(import, qualifier) {
                return resolve_task_in_module(program, caller_module, &import.module, last);
            }
        }
    }

    TaskResolution::Unknown
}

pub fn resolve_type(program: &Program, caller_module: &str, path: &str) -> TypeResolution {
    let parts = path
        .split('.')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let Some(last) = parts.last().copied() else {
        return TypeResolution::Unknown;
    };

    if parts.len() == 1 {
        if is_builtin_type(last) {
            return TypeResolution::Builtin(last.to_string());
        }
        if let Some(index) = find_type_in_module(program, caller_module, last) {
            return resolved_type(program, index);
        }

        let mut matches = Vec::new();
        for import in imports_for_module(program, caller_module) {
            if let Some(index) = find_type_in_module(program, &import.module, last)
                && program.types[index].exported
            {
                matches.push(index);
            }
        }
        matches.sort_unstable();
        matches.dedup();
        return match matches.as_slice() {
            [index] => resolved_type(program, *index),
            [] => TypeResolution::Unknown,
            _ => TypeResolution::Ambiguous(
                matches
                    .into_iter()
                    .map(|index| type_fq_name(&program.types[index]))
                    .collect(),
            ),
        };
    }

    let module_path = parts[..parts.len() - 1].join(".");
    if module_exists(program, &module_path) {
        return resolve_type_in_module(program, caller_module, &module_path, last);
    }

    if parts.len() == 2 {
        let qualifier = parts[0];
        for import in imports_for_module(program, caller_module) {
            if import_matches_qualifier(import, qualifier) {
                return resolve_type_in_module(program, caller_module, &import.module, last);
            }
        }
    }

    TypeResolution::Unknown
}

pub fn resolve_effect(program: &Program, caller_module: &str, path: &str) -> EffectResolution {
    let parts = path
        .split('.')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    let Some(last) = parts.last().copied() else {
        return EffectResolution::Unknown;
    };

    if parts.len() == 1 {
        if is_builtin_effect(last) {
            return EffectResolution::Builtin(last.to_string());
        }
        if let Some(index) = find_effect_in_module(program, caller_module, last) {
            return resolved_effect(program, index);
        }

        let mut matches = Vec::new();
        for import in imports_for_module(program, caller_module) {
            if let Some(index) = find_effect_in_module(program, &import.module, last)
                && program.effects[index].exported
            {
                matches.push(index);
            }
        }
        matches.sort_unstable();
        matches.dedup();
        return match matches.as_slice() {
            [index] => resolved_effect(program, *index),
            [] => EffectResolution::Unknown,
            _ => EffectResolution::Ambiguous(
                matches
                    .into_iter()
                    .map(|index| effect_fq_name(&program.effects[index]))
                    .collect(),
            ),
        };
    }

    let module_path = parts[..parts.len() - 1].join(".");
    if module_exists(program, &module_path) {
        return resolve_effect_in_module(program, caller_module, &module_path, last);
    }

    if parts.len() == 2 {
        let qualifier = parts[0];
        for import in imports_for_module(program, caller_module) {
            if import_matches_qualifier(import, qualifier) {
                return resolve_effect_in_module(program, caller_module, &import.module, last);
            }
        }
    }

    EffectResolution::Unknown
}

pub fn callee_path(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Identifier { name } => Some(name.clone()),
        ExprKind::FieldAccess { receiver, field } => {
            let mut path = callee_path(receiver)?;
            path.push('.');
            path.push_str(field);
            Some(path)
        }
        _ => None,
    }
}

pub fn task_module(task: &TaskDecl) -> String {
    task.module
        .clone()
        .or_else(|| declaration_module_from_id(&task.id, "task"))
        .unwrap_or_else(|| "main".to_string())
}

pub fn type_module(ty: &TypeDecl) -> String {
    ty.module
        .clone()
        .or_else(|| declaration_module_from_id(&ty.id, "type"))
        .unwrap_or_else(|| "main".to_string())
}

pub fn effect_module(effect: &EffectDecl) -> String {
    effect
        .module
        .clone()
        .or_else(|| declaration_module_from_id(&effect.id, "effect"))
        .unwrap_or_else(|| "main".to_string())
}

pub fn import_owner_module(import: &ImportDecl) -> String {
    import
        .owner_module
        .clone()
        .or_else(|| {
            import.id.strip_prefix("import:").and_then(|value| {
                value
                    .split_once(':')
                    .map(|(owner, _target)| owner.to_string())
            })
        })
        .unwrap_or_else(|| "main".to_string())
}

pub fn task_fq_name(task: &TaskDecl) -> String {
    format!("{}.{}", task_module(task), task.name)
}

pub fn type_fq_name(ty: &TypeDecl) -> String {
    format!("{}.{}", type_module(ty), ty.name)
}

pub fn effect_fq_name(effect: &EffectDecl) -> String {
    format!("{}.{}", effect_module(effect), effect.name)
}

pub fn is_module_qualified_callee(program: &Program, caller_module: &str, path: &str) -> bool {
    let parts = path
        .split('.')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() < 2 {
        return false;
    }
    let module_path = parts[..parts.len() - 1].join(".");
    module_exists(program, &module_path)
        || imports_for_module(program, caller_module)
            .iter()
            .any(|import| import_matches_qualifier(import, parts[0]))
}

fn build_slice(
    program: &Program,
    target: &str,
    focus: SliceFocus,
    focus_task_index: Option<usize>,
) -> SymbolGraphSlice {
    let module = module_summary(program, &focus.module);
    let module_task_indexes = program
        .tasks
        .iter()
        .enumerate()
        .filter_map(|(index, task)| (task_module(task) == focus.module).then_some(index))
        .collect::<Vec<_>>();
    let outbound_task_ids = focus_task_index
        .map(|index| vec![task_fq_name(&program.tasks[index])])
        .unwrap_or_else(|| {
            module_task_indexes
                .iter()
                .map(|index| task_fq_name(&program.tasks[*index]))
                .collect()
        });
    let inbound_target_ids = focus_task_index
        .map(|index| vec![task_fq_name(&program.tasks[index])])
        .unwrap_or_else(|| {
            module_task_indexes
                .iter()
                .map(|index| task_fq_name(&program.tasks[*index]))
                .collect()
        });
    let calls = collect_task_calls(program);
    let outbound_calls = calls
        .iter()
        .filter(|call| outbound_task_ids.contains(&call.from))
        .cloned()
        .collect::<Vec<_>>();
    let inbound_calls = calls
        .iter()
        .filter(|call| {
            !outbound_task_ids.contains(&call.from)
                && call
                    .target
                    .as_ref()
                    .is_some_and(|target| inbound_target_ids.contains(target))
        })
        .cloned()
        .collect::<Vec<_>>();

    SymbolGraphSlice {
        schema: SYMBOL_GRAPH_SLICE_SCHEMA.to_string(),
        target: target.to_string(),
        entry_module: program.module_name().to_string(),
        focus,
        imports: module.imports,
        types: module.types,
        effects: module.effects,
        tasks: module.tasks,
        task: focus_task_index.map(|index| program.tasks[index].clone()),
        move_affordances: build_move_affordances(program, focus_task_index, &module_task_indexes),
        outbound_calls,
        inbound_calls,
    }
}

fn build_move_affordances(
    program: &Program,
    focus_task_index: Option<usize>,
    module_task_indexes: &[usize],
) -> Vec<MoveNodeAffordance> {
    let task_indexes = focus_task_index
        .map(|index| vec![index])
        .unwrap_or_else(|| module_task_indexes.to_vec());
    let mut affordances = Vec::new();
    for task_index in task_indexes {
        let task = &program.tasks[task_index];
        collect_take_move_affordances(task, module_task_indexes, program, &mut affordances);
        collect_statement_move_affordances(task, &mut affordances);
    }
    affordances
}

fn collect_take_move_affordances(
    task: &TaskDecl,
    destination_task_indexes: &[usize],
    program: &Program,
    affordances: &mut Vec<MoveNodeAffordance>,
) {
    let parent = format!("{}:takes", task.id);
    let max_position = task.takes.len().saturating_sub(1);
    for (position, take) in task.takes.iter().enumerate() {
        let destinations = destination_task_indexes
            .iter()
            .filter_map(|index| {
                let destination = &program.tasks[*index];
                (destination.id != task.id).then(|| MoveNodeDestination {
                    parent: format!("{}:takes", destination.id),
                    max_position: destination.takes.len(),
                })
            })
            .collect();
        affordances.push(MoveNodeAffordance {
            target: take.id.clone(),
            target_kind: "take".to_string(),
            parent: parent.clone(),
            position,
            max_position,
            destinations,
        });
    }
}

#[derive(Debug, Clone)]
struct BlockMoveDestination {
    parent: String,
    max_position: usize,
}

fn collect_statement_move_affordances(task: &TaskDecl, affordances: &mut Vec<MoveNodeAffordance>) {
    let mut destinations = Vec::new();
    collect_block_move_destinations(&task.body, format!("block:{}", task.id), &mut destinations);
    collect_statement_move_affordances_in_block(
        &task.body,
        &format!("block:{}", task.id),
        &destinations,
        affordances,
    );
}

fn collect_block_move_destinations(
    block: &Block,
    parent: String,
    destinations: &mut Vec<BlockMoveDestination>,
) {
    destinations.push(BlockMoveDestination {
        parent: parent.clone(),
        max_position: block.statements.len(),
    });
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::If {
                then_block,
                else_block,
                ..
            } => {
                collect_block_move_destinations(
                    then_block,
                    format!("{}:then", statement.id),
                    destinations,
                );
                if let Some(else_block) = else_block {
                    collect_block_move_destinations(
                        else_block,
                        format!("{}:else", statement.id),
                        destinations,
                    );
                }
            }
            StatementKind::While { body, .. } => {
                collect_block_move_destinations(
                    body,
                    format!("{}:body", statement.id),
                    destinations,
                );
            }
            StatementKind::For { body, .. } => {
                collect_block_move_destinations(
                    body,
                    format!("{}:body", statement.id),
                    destinations,
                );
            }
            StatementKind::Forge { body } => {
                collect_block_move_destinations(
                    body,
                    format!("{}:forge", statement.id),
                    destinations,
                );
            }
            StatementKind::Binding { .. }
            | StatementKind::Set { .. }
            | StatementKind::Return { .. }
            | StatementKind::Expr { .. } => {}
        }
    }
}

fn collect_statement_move_affordances_in_block(
    block: &Block,
    parent: &str,
    destinations: &[BlockMoveDestination],
    affordances: &mut Vec<MoveNodeAffordance>,
) {
    let max_position = block.statements.len().saturating_sub(1);
    for (position, statement) in block.statements.iter().enumerate() {
        let destination_parents = destinations
            .iter()
            .filter(|destination| {
                destination.parent != parent
                    && !destination
                        .parent
                        .strip_prefix(&statement.id)
                        .is_some_and(|suffix| suffix.starts_with(':'))
            })
            .map(|destination| MoveNodeDestination {
                parent: destination.parent.clone(),
                max_position: destination.max_position,
            })
            .collect();
        affordances.push(MoveNodeAffordance {
            target: statement.id.clone(),
            target_kind: "statement".to_string(),
            parent: parent.to_string(),
            position,
            max_position,
            destinations: destination_parents,
        });
        match &statement.kind {
            StatementKind::If {
                then_block,
                else_block,
                ..
            } => {
                collect_statement_move_affordances_in_block(
                    then_block,
                    &format!("{}:then", statement.id),
                    destinations,
                    affordances,
                );
                if let Some(else_block) = else_block {
                    collect_statement_move_affordances_in_block(
                        else_block,
                        &format!("{}:else", statement.id),
                        destinations,
                        affordances,
                    );
                }
            }
            StatementKind::While { body, .. } => {
                collect_statement_move_affordances_in_block(
                    body,
                    &format!("{}:body", statement.id),
                    destinations,
                    affordances,
                );
            }
            StatementKind::For { body, .. } => {
                collect_statement_move_affordances_in_block(
                    body,
                    &format!("{}:body", statement.id),
                    destinations,
                    affordances,
                );
            }
            StatementKind::Forge { body } => {
                collect_statement_move_affordances_in_block(
                    body,
                    &format!("{}:forge", statement.id),
                    destinations,
                    affordances,
                );
            }
            StatementKind::Binding { .. }
            | StatementKind::Set { .. }
            | StatementKind::Return { .. }
            | StatementKind::Expr { .. } => {}
        }
    }
}

fn module_summary(program: &Program, module: &str) -> ModuleSymbolSummary {
    build_symbol_graph(program)
        .modules
        .into_iter()
        .find(|summary| summary.module == module)
        .unwrap_or_else(|| ModuleSymbolSummary {
            module: module.to_string(),
            imports: Vec::new(),
            types: Vec::new(),
            effects: Vec::new(),
            tasks: Vec::new(),
        })
}

fn collect_task_calls_from_block(
    program: &Program,
    task: &TaskDecl,
    block: &Block,
    calls: &mut Vec<TaskCallSummary>,
) {
    for statement in &block.statements {
        collect_task_calls_from_statement(program, task, statement, calls);
    }
}

fn collect_task_calls_from_statement(
    program: &Program,
    task: &TaskDecl,
    statement: &Statement,
    calls: &mut Vec<TaskCallSummary>,
) {
    match &statement.kind {
        StatementKind::Binding { expr, .. }
        | StatementKind::Set { expr, .. }
        | StatementKind::Return { expr }
        | StatementKind::Expr { expr } => collect_task_calls_from_expr(program, task, expr, calls),
        StatementKind::If {
            condition,
            then_block,
            else_block,
        } => {
            collect_task_calls_from_expr(program, task, condition, calls);
            collect_task_calls_from_block(program, task, then_block, calls);
            if let Some(else_block) = else_block {
                collect_task_calls_from_block(program, task, else_block, calls);
            }
        }
        StatementKind::While { condition, body } => {
            collect_task_calls_from_expr(program, task, condition, calls);
            collect_task_calls_from_block(program, task, body, calls);
        }
        StatementKind::For {
            collection, body, ..
        } => {
            collect_task_calls_from_expr(program, task, collection, calls);
            collect_task_calls_from_block(program, task, body, calls);
        }
        StatementKind::Forge { body } => collect_task_calls_from_block(program, task, body, calls),
    }
}

fn collect_task_calls_from_expr(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    calls: &mut Vec<TaskCallSummary>,
) {
    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            collect_task_calls_from_expr(program, task, expr, calls);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_task_calls_from_expr(program, task, left, calls);
            collect_task_calls_from_expr(program, task, right, calls);
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_task_calls_from_expr(program, task, condition, calls);
            collect_task_calls_from_expr(program, task, then_branch, calls);
            collect_task_calls_from_expr(program, task, else_branch, calls);
        }
        ExprKind::Call { callee, args } => {
            if let Some(path) = callee_path(callee) {
                calls.push(call_summary(program, task, expr, path));
            }
            collect_task_calls_from_expr(program, task, callee, calls);
            for arg in args {
                collect_task_calls_from_expr(program, task, arg, calls);
            }
        }
        ExprKind::ListLiteral { items } => {
            for item in items {
                collect_task_calls_from_expr(program, task, item, calls);
            }
        }
        ExprKind::MapLiteral { entries } => {
            for entry in entries {
                collect_task_calls_from_expr(program, task, &entry.key, calls);
                collect_task_calls_from_expr(program, task, &entry.value, calls);
            }
        }
        ExprKind::Index { collection, index } => {
            collect_task_calls_from_expr(program, task, collection, calls);
            collect_task_calls_from_expr(program, task, index, calls);
        }
        ExprKind::FieldAccess { receiver, .. } => {
            collect_task_calls_from_expr(program, task, receiver, calls);
        }
        ExprKind::RecordLiteral { fields, .. } => {
            for field in fields {
                collect_task_calls_from_expr(program, task, &field.expr, calls);
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

fn call_summary(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    callee: String,
) -> TaskCallSummary {
    let from_module = task_module(task);
    if let Some(target) = intrinsic_call_target(&callee) {
        return TaskCallSummary {
            from: task_fq_name(task),
            from_module,
            expr_id: expr.id.clone(),
            source: expr.source.clone(),
            callee,
            status: "intrinsic".to_string(),
            target: Some(target.to_string()),
            candidates: Vec::new(),
        };
    }

    let resolution = resolve_task(program, &from_module, &callee);
    let (status, target, candidates) = match resolution {
        TaskResolution::Resolved { fq_name, .. } => {
            ("resolved".to_string(), Some(fq_name), Vec::new())
        }
        TaskResolution::Unknown => ("unknown".to_string(), None, Vec::new()),
        TaskResolution::Ambiguous(candidates) => ("ambiguous".to_string(), None, candidates),
        TaskResolution::Private(target) => ("private".to_string(), Some(target), Vec::new()),
    };

    TaskCallSummary {
        from: task_fq_name(task),
        from_module,
        expr_id: expr.id.clone(),
        source: expr.source.clone(),
        callee,
        status,
        target,
        candidates,
    }
}

fn intrinsic_call_target(callee: &str) -> Option<&'static str> {
    match callee {
        "len" => Some("builtin:len"),
        "Ok" => Some("constructor:Ok"),
        "Err" => Some("constructor:Err"),
        _ => None,
    }
}

fn push_decl(
    modules: &mut [ModuleSymbolSummary],
    indexes: &mut BTreeMap<String, usize>,
    module: &str,
    decl: DeclarationSymbolSummary,
    kind: DeclarationKind,
) {
    let Some(index) = indexes.get(module).copied() else {
        return;
    };
    match kind {
        DeclarationKind::Type => modules[index].types.push(decl),
        DeclarationKind::Effect => modules[index].effects.push(decl),
        DeclarationKind::Task => modules[index].tasks.push(decl),
    }
}

fn resolve_task_in_module(
    program: &Program,
    caller_module: &str,
    module: &str,
    name: &str,
) -> TaskResolution {
    if module != caller_module && !module_imported_by(program, caller_module, module) {
        return TaskResolution::Unknown;
    }
    let Some(index) = find_task_in_module(program, module, name) else {
        return TaskResolution::Unknown;
    };
    if module == caller_module || program.tasks[index].exported {
        return resolved_task(program, index);
    }
    TaskResolution::Private(task_fq_name(&program.tasks[index]))
}

fn resolve_type_in_module(
    program: &Program,
    caller_module: &str,
    module: &str,
    name: &str,
) -> TypeResolution {
    if module != caller_module && !module_imported_by(program, caller_module, module) {
        return TypeResolution::Unknown;
    }
    let Some(index) = find_type_in_module(program, module, name) else {
        return TypeResolution::Unknown;
    };
    if module == caller_module || program.types[index].exported {
        return resolved_type(program, index);
    }
    TypeResolution::Private(type_fq_name(&program.types[index]))
}

fn resolve_effect_in_module(
    program: &Program,
    caller_module: &str,
    module: &str,
    name: &str,
) -> EffectResolution {
    if module != caller_module && !module_imported_by(program, caller_module, module) {
        return EffectResolution::Unknown;
    }
    let Some(index) = find_effect_in_module(program, module, name) else {
        return EffectResolution::Unknown;
    };
    if module == caller_module || program.effects[index].exported {
        return resolved_effect(program, index);
    }
    EffectResolution::Private(effect_fq_name(&program.effects[index]))
}

fn resolved_task(program: &Program, index: usize) -> TaskResolution {
    TaskResolution::Resolved {
        index,
        fq_name: task_fq_name(&program.tasks[index]),
    }
}

fn resolved_type(program: &Program, index: usize) -> TypeResolution {
    TypeResolution::Resolved {
        index,
        fq_name: type_fq_name(&program.types[index]),
    }
}

fn resolved_effect(program: &Program, index: usize) -> EffectResolution {
    EffectResolution::Resolved {
        index,
        fq_name: effect_fq_name(&program.effects[index]),
    }
}

fn find_task_in_module(program: &Program, module: &str, name: &str) -> Option<usize> {
    program
        .tasks
        .iter()
        .position(|task| task_module(task) == module && task.name == name)
}

fn find_type_in_module(program: &Program, module: &str, name: &str) -> Option<usize> {
    program
        .types
        .iter()
        .position(|ty| type_module(ty) == module && ty.name == name)
}

fn find_effect_in_module(program: &Program, module: &str, name: &str) -> Option<usize> {
    program
        .effects
        .iter()
        .position(|effect| effect_module(effect) == module && effect.name == name)
}

fn find_type_index(program: &Program, target: &str) -> Option<usize> {
    let needle = target.strip_prefix("type:").unwrap_or(target);
    program.types.iter().position(|ty| {
        ty.id == target
            || ty.id == format!("type:{needle}")
            || ty.name == needle
            || ty.id.ends_with(&format!(".{needle}"))
    })
}

fn find_effect_index(program: &Program, target: &str) -> Option<usize> {
    let needle = target.strip_prefix("effect:").unwrap_or(target);
    program.effects.iter().position(|effect| {
        effect.id == target
            || effect.id == format!("effect:{needle}")
            || effect.name == needle
            || effect.id.ends_with(&format!(".{needle}"))
    })
}

fn find_import_index(program: &Program, target: &str) -> Option<usize> {
    let needle = target.strip_prefix("import:").unwrap_or(target);
    program.imports.iter().position(|import| {
        import.id == target || import.id == format!("import:{needle}") || import.module == needle
    })
}

fn imports_for_module<'a>(program: &'a Program, module: &str) -> Vec<&'a ImportDecl> {
    program
        .imports
        .iter()
        .filter(|import| import_owner_module(import) == module)
        .collect()
}

fn import_matches_qualifier(import: &ImportDecl, qualifier: &str) -> bool {
    import.alias.as_deref() == Some(qualifier)
        || import
            .module
            .rsplit('.')
            .next()
            .is_some_and(|segment| segment == qualifier)
}

fn module_imported_by(program: &Program, caller_module: &str, module: &str) -> bool {
    imports_for_module(program, caller_module)
        .iter()
        .any(|import| import.module == module)
}

fn module_exists(program: &Program, module: &str) -> bool {
    program.module_name() == module
        || program.tasks.iter().any(|task| task_module(task) == module)
        || program.types.iter().any(|ty| type_module(ty) == module)
        || program
            .effects
            .iter()
            .any(|effect| effect_module(effect) == module)
        || program
            .imports
            .iter()
            .any(|import| import.module == module || import_owner_module(import) == module)
}

fn is_builtin_type(name: &str) -> bool {
    matches!(
        name,
        "Int"
            | "Float"
            | "Bool"
            | "Text"
            | "Unit"
            | "List"
            | "Map"
            | "Optional"
            | "Result"
            | "Error"
            | "Gate"
            | "DbRow"
    )
}

fn is_builtin_effect(name: &str) -> bool {
    matches!(
        name,
        "FileRead"
            | "FileWrite"
            | "Network"
            | "Shell"
            | "ModelCall"
            | "SecretRead"
            | "Spend"
            | "Deploy"
            | "DatabaseRead"
            | "DatabaseWrite"
            | "DbRead"
            | "DbWrite"
    )
}

fn declaration_module_from_id(id: &str, prefix: &str) -> Option<String> {
    let value = id.strip_prefix(&format!("{prefix}:"))?;
    let (module, _name) = value.rsplit_once('.')?;
    Some(module.to_string())
}

enum DeclarationKind {
    Type,
    Effect,
    Task,
}
