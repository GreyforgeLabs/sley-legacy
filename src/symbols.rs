use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::ast::{EffectDecl, Expr, ExprKind, ImportDecl, Program, TaskDecl, TypeDecl};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskResolution {
    Resolved { index: usize, fq_name: String },
    Unknown,
    Ambiguous(Vec<String>),
    Private(String),
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SymbolGraph {
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
        entry_module: program.module_name().to_string(),
        modules: module_summaries,
    }
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

fn resolved_task(program: &Program, index: usize) -> TaskResolution {
    TaskResolution::Resolved {
        index,
        fq_name: task_fq_name(&program.tasks[index]),
    }
}

fn find_task_in_module(program: &Program, module: &str, name: &str) -> Option<usize> {
    program
        .tasks
        .iter()
        .position(|task| task_module(task) == module && task.name == name)
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
