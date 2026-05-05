use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::ast::{
    BinaryOp, BindingKind, Block, EffectDecl, Expr, ExprField, ExprKind, ExprMapEntry, ImportDecl,
    Program, ProvenanceRecord, Statement, StatementKind, TakeDecl, TaskDecl, TypeDecl,
};
use crate::checker::{check_program, has_errors};
use crate::diagnostics::Diagnostic;
use crate::formatter::format_program;
use crate::parser::{parse_block_source, parse_expr_source, parse_program, parse_type_expr_source};
use crate::symbols::{
    callee_path, collect_task_calls, effect_module, import_owner_module, task_fq_name, task_module,
    type_module,
};

pub const GRAFT_OUTCOME_SCHEMA: &str = "sley.graft.outcome.v0";

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum GraftInput {
    Transaction(GraftTransaction),
    Operation(GraftOperation),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraftTransaction {
    pub transaction: String,
    #[serde(default)]
    pub actor: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    pub ops: Vec<GraftOperation>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "op", deny_unknown_fields)]
pub enum GraftOperation {
    AddTask {
        #[serde(default)]
        precondition: Option<JsonValue>,
        payload: SourcePayload,
    },
    ReplaceTaskBody {
        target: String,
        #[serde(default)]
        precondition: Option<JsonValue>,
        payload: BodyPayload,
    },
    AddTake {
        target: String,
        #[serde(default)]
        precondition: Option<JsonValue>,
        payload: AddTakePayload,
    },
    RemoveTake {
        target: String,
        #[serde(default)]
        precondition: Option<JsonValue>,
        payload: NamedPayload,
    },
    RenameDeclaration {
        target: String,
        #[serde(default)]
        precondition: Option<JsonValue>,
        payload: RenamePayload,
    },
    AddTypeDeclaration {
        #[serde(default)]
        precondition: Option<JsonValue>,
        payload: SourcePayload,
    },
    AddEffectDeclaration {
        #[serde(default)]
        precondition: Option<JsonValue>,
        payload: NamedPayload,
    },
    AddImport {
        #[serde(default)]
        precondition: Option<JsonValue>,
        payload: ImportPayload,
    },
    UpdateCallSites {
        target: String,
        #[serde(default)]
        precondition: Option<JsonValue>,
        payload: UpdateCallSitesPayload,
    },
    InsertStatement {
        target: String,
        #[serde(default)]
        precondition: Option<JsonValue>,
        payload: InsertStatementPayload,
    },
    ReplaceExpression {
        target: String,
        #[serde(default)]
        precondition: Option<JsonValue>,
        payload: ExpressionPayload,
    },
    MoveNode {
        target: String,
        #[serde(default)]
        precondition: Option<JsonValue>,
        #[serde(default)]
        payload: Option<MoveNodePayload>,
    },
    DeleteNode {
        target: String,
        #[serde(default)]
        precondition: Option<JsonValue>,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddTakePayload {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(default)]
    pub position: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyPayload {
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub statements: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePayload {
    pub source: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedPayload {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenamePayload {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportPayload {
    pub module: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateCallSitesPayload {
    pub replacement: String,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InsertStatementPayload {
    pub source: String,
    #[serde(default)]
    pub position: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoveNodePayload {
    pub position: usize,
    #[serde(default)]
    pub parent: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpressionPayload {
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraftOutcome {
    pub schema: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provenance: Vec<ProvenanceRecord>,
}

#[derive(Debug, Clone)]
pub struct AppliedGraft {
    pub outcome: GraftOutcome,
    pub program: Option<Program>,
}

pub fn apply_graft_input(
    program: &Program,
    input: GraftInput,
    actor: Option<String>,
) -> GraftOutcome {
    apply_graft_program(program, input, actor).outcome
}

pub fn apply_graft_program(
    program: &Program,
    input: GraftInput,
    actor: Option<String>,
) -> AppliedGraft {
    let actor = actor.unwrap_or_else(|| "agent:unknown".to_string());
    let mut candidate = program.clone();
    let mut provenance = Vec::new();
    let mut diagnostics = Vec::new();

    match input {
        GraftInput::Transaction(transaction) => {
            let graft_id = transaction.transaction;
            let actor = transaction.actor.unwrap_or(actor);
            if transaction
                .mode
                .as_deref()
                .is_some_and(|mode| mode != "all_or_nothing")
            {
                diagnostics.push(Diagnostic::error(
                    "GRAFT_UNSUPPORTED_MODE",
                    "only `all_or_nothing` transaction mode is supported",
                ));
            } else {
                for op in transaction.ops {
                    match apply_one(&mut candidate, op, &graft_id, &actor) {
                        Ok(record) => provenance.push(record),
                        Err(mut op_diagnostics) => {
                            diagnostics.append(&mut op_diagnostics);
                            break;
                        }
                    }
                }
            }
        }
        GraftInput::Operation(op) => {
            let graft_id = format!("graft_{}", unix_timestamp());
            match apply_one(&mut candidate, op, &graft_id, &actor) {
                Ok(record) => provenance.push(record),
                Err(mut op_diagnostics) => diagnostics.append(&mut op_diagnostics),
            }
        }
    }

    if diagnostics.is_empty() {
        let mut validation = check_program(&candidate);
        if has_errors(&validation) {
            diagnostics.append(&mut validation);
        }
    }

    if diagnostics.iter().any(Diagnostic::is_error) {
        AppliedGraft {
            outcome: GraftOutcome {
                schema: GRAFT_OUTCOME_SCHEMA.to_string(),
                status: "rejected".to_string(),
                diagnostics,
                source: None,
                provenance: Vec::new(),
            },
            program: None,
        }
    } else {
        candidate.provenance.extend(provenance.clone());
        AppliedGraft {
            outcome: GraftOutcome {
                schema: GRAFT_OUTCOME_SCHEMA.to_string(),
                status: "accepted".to_string(),
                diagnostics,
                source: Some(format_program(&candidate)),
                provenance,
            },
            program: Some(candidate),
        }
    }
}

fn apply_one(
    program: &mut Program,
    op: GraftOperation,
    graft_id: &str,
    actor: &str,
) -> Result<ProvenanceRecord, Vec<Diagnostic>> {
    match op {
        GraftOperation::AddTake {
            target,
            precondition,
            payload,
        } => {
            let index = find_task_or_reject(program, &target)?;
            check_preconditions(program, Some(index), precondition.as_ref())?;
            let ty = parse_type_expr_source(&payload.ty)?;
            let task = &mut program.tasks[index];
            if task.takes.iter().any(|take| take.name == payload.name) {
                return Err(vec![
                    Diagnostic::error(
                        "GRAFT_TAKE_EXISTS",
                        format!("take `{}` already exists", payload.name),
                    )
                    .with_node(task.id.clone()),
                ]);
            }
            let position = payload.position.unwrap_or(task.takes.len());
            let take = TakeDecl {
                id: String::new(),
                name: payload.name,
                binding_kind: BindingKind::Take,
                ty,
                span: None,
            };
            if position >= task.takes.len() {
                task.takes.push(take);
            } else {
                task.takes.insert(position, take);
            }
            program.assign_ids();
            Ok(record(graft_id, actor, "AddTake", vec![target]))
        }
        GraftOperation::RemoveTake {
            target,
            precondition,
            payload,
        } => {
            let index = find_task_or_reject(program, &target)?;
            check_preconditions(program, Some(index), precondition.as_ref())?;
            let task = &mut program.tasks[index];
            let before = task.takes.len();
            task.takes.retain(|take| take.name != payload.name);
            if before == task.takes.len() {
                return Err(vec![
                    Diagnostic::error(
                        "GRAFT_TAKE_MISSING",
                        format!("take `{}` is absent", payload.name),
                    )
                    .with_node(task.id.clone()),
                ]);
            }
            program.assign_ids();
            Ok(record(graft_id, actor, "RemoveTake", vec![target]))
        }
        GraftOperation::ReplaceTaskBody {
            target,
            precondition,
            payload,
        } => {
            let index = find_task_or_reject(program, &target)?;
            check_preconditions(program, Some(index), precondition.as_ref())?;
            let source = payload
                .source
                .or_else(|| payload.statements.map(|statements| statements.join("\n")))
                .ok_or_else(|| {
                    vec![Diagnostic::error(
                        "GRAFT_MISSING_BODY",
                        "ReplaceTaskBody requires payload.source or payload.statements",
                    )]
                })?;
            let body: Block = parse_block_source(&source)?;
            program.tasks[index].body = body;
            program.assign_ids();
            Ok(record(graft_id, actor, "ReplaceTaskBody", vec![target]))
        }
        GraftOperation::AddEffectDeclaration {
            precondition,
            payload,
        } => {
            check_preconditions(program, None, precondition.as_ref())?;
            if program
                .effects
                .iter()
                .any(|effect| effect.name == payload.name)
            {
                return Err(vec![Diagnostic::error(
                    "GRAFT_EFFECT_EXISTS",
                    format!("effect `{}` already exists", payload.name),
                )]);
            }
            program.effects.push(EffectDecl {
                id: String::new(),
                module: None,
                exported: false,
                name: payload.name.clone(),
                span: None,
            });
            program.assign_ids();
            Ok(record(
                graft_id,
                actor,
                "AddEffectDeclaration",
                vec![format!("effect:{}", payload.name)],
            ))
        }
        GraftOperation::AddImport {
            precondition,
            payload,
        } => {
            check_preconditions(program, None, precondition.as_ref())?;
            if program
                .imports
                .iter()
                .any(|import| import.module == payload.module)
            {
                return Err(vec![Diagnostic::error(
                    "GRAFT_IMPORT_EXISTS",
                    format!("import `{}` already exists", payload.module),
                )]);
            }
            program.imports.push(ImportDecl {
                id: String::new(),
                owner_module: None,
                module: payload.module.clone(),
                alias: None,
                span: None,
            });
            program.assign_ids();
            Ok(record(
                graft_id,
                actor,
                "AddImport",
                vec![format!("import:{}", payload.module)],
            ))
        }
        GraftOperation::AddTask {
            precondition,
            payload,
        } => {
            check_preconditions(program, None, precondition.as_ref())?;
            let parsed = parse_program(&payload.source)?;
            let Some(task) = parsed.tasks.into_iter().next() else {
                return Err(vec![Diagnostic::error(
                    "GRAFT_EXPECTED_TASK",
                    "AddTask payload.source must contain a task",
                )]);
            };
            let module = task_module(&task);
            if program
                .tasks
                .iter()
                .any(|item| item.name == task.name && task_module(item) == module)
            {
                return Err(vec![Diagnostic::error(
                    "GRAFT_TASK_EXISTS",
                    format!("task `{module}.{}` already exists", task.name),
                )]);
            }
            let target = task.name.clone();
            program.tasks.push(TaskDecl { ..task });
            program.assign_ids();
            Ok(record(
                graft_id,
                actor,
                "AddTask",
                vec![format!("task:{target}")],
            ))
        }
        GraftOperation::AddTypeDeclaration {
            precondition,
            payload,
        } => {
            check_preconditions(program, None, precondition.as_ref())?;
            let parsed = parse_program(&payload.source)?;
            let Some(ty) = parsed.types.into_iter().next() else {
                return Err(vec![Diagnostic::error(
                    "GRAFT_EXPECTED_TYPE",
                    "AddTypeDeclaration payload.source must contain a type declaration",
                )]);
            };
            let module = type_module(&ty);
            if program
                .types
                .iter()
                .any(|item| item.name == ty.name && type_module(item) == module)
            {
                return Err(vec![Diagnostic::error(
                    "GRAFT_TYPE_EXISTS",
                    format!("type `{module}.{}` already exists", ty.name),
                )]);
            }
            let target = ty.name.clone();
            program.types.push(TypeDecl { ..ty });
            program.assign_ids();
            Ok(record(
                graft_id,
                actor,
                "AddTypeDeclaration",
                vec![format!("type:{target}")],
            ))
        }
        GraftOperation::RenameDeclaration {
            target,
            precondition,
            payload,
        } => {
            check_preconditions(program, None, precondition.as_ref())?;
            if let Some(module) = target.strip_prefix("module:") {
                rename_module(program, module, &payload.name)?;
                program.assign_ids();
                return Ok(record(graft_id, actor, "RenameDeclaration", vec![target]));
            }
            if let Some(index) = program.find_task_index(&target) {
                program.tasks[index].name = payload.name.clone();
                program.assign_ids();
                return Ok(record(graft_id, actor, "RenameDeclaration", vec![target]));
            }
            if let Some(name) = target.strip_prefix("type:")
                && let Some(ty) = program
                    .types
                    .iter_mut()
                    .find(|ty| ty.name == name || ty.id == target)
            {
                ty.name = payload.name.clone();
                program.assign_ids();
                return Ok(record(graft_id, actor, "RenameDeclaration", vec![target]));
            }
            if let Some(name) = target.strip_prefix("effect:")
                && let Some(effect) = program
                    .effects
                    .iter_mut()
                    .find(|effect| effect.name == name || effect.id == target)
            {
                effect.name = payload.name.clone();
                program.assign_ids();
                return Ok(record(graft_id, actor, "RenameDeclaration", vec![target]));
            }
            Err(vec![Diagnostic::error(
                "GRAFT_TARGET_MISSING",
                format!("target `{target}` does not exist"),
            )])
        }
        GraftOperation::UpdateCallSites {
            target,
            precondition,
            payload,
        } => {
            check_preconditions(program, None, precondition.as_ref())?;
            let target_index = find_task_or_reject(program, &target)?;
            let target_fq_name = task_fq_name(&program.tasks[target_index]);
            let replacement = parse_callee_source(&payload.replacement)?;
            let call_ids = matching_call_expr_ids(program, &target_fq_name, &payload);
            if call_ids.is_empty() {
                return Err(vec![
                    Diagnostic::error(
                        "GRAFT_CALLSITE_MISSING",
                        format!("no call sites matched `{target}`"),
                    )
                    .with_node(program.tasks[target_index].id.clone()),
                ]);
            }
            let updated = update_call_callees(program, &call_ids, &replacement);
            if updated == 0 {
                return Err(vec![
                    Diagnostic::error(
                        "GRAFT_CALLSITE_MISSING",
                        format!("no call-site expressions could be rewritten for `{target}`"),
                    )
                    .with_node(program.tasks[target_index].id.clone()),
                ]);
            }
            program.assign_ids();
            Ok(record(
                graft_id,
                actor,
                "UpdateCallSites",
                vec![target, format!("calls:{updated}")],
            ))
        }
        GraftOperation::InsertStatement {
            target,
            precondition,
            payload,
        } => {
            let index = find_task_or_reject(program, &target)?;
            check_preconditions(program, Some(index), precondition.as_ref())?;
            let mut parsed = parse_block_source(&payload.source)?;
            if parsed.statements.len() != 1 {
                return Err(vec![Diagnostic::error(
                    "GRAFT_EXPECTED_ONE_STATEMENT",
                    "InsertStatement payload.source must parse to exactly one statement",
                )]);
            }
            let statement = parsed.statements.remove(0);
            let task = &mut program.tasks[index];
            let position = payload.position.unwrap_or(task.body.statements.len());
            if position > task.body.statements.len() {
                return Err(vec![
                    Diagnostic::error(
                        "GRAFT_POSITION_OUT_OF_RANGE",
                        format!(
                            "position {position} is past task body length {}",
                            task.body.statements.len()
                        ),
                    )
                    .with_node(task.id.clone()),
                ]);
            }
            task.body.statements.insert(position, statement);
            program.assign_ids();
            Ok(record(graft_id, actor, "InsertStatement", vec![target]))
        }
        GraftOperation::ReplaceExpression {
            target,
            precondition,
            payload,
        } => {
            check_preconditions(program, None, precondition.as_ref())?;
            let replacement = parse_expr_source(&payload.source)?;
            if !replace_expression(program, &target, replacement) {
                return Err(vec![Diagnostic::error(
                    "GRAFT_TARGET_MISSING",
                    format!("expression target `{target}` does not exist"),
                )]);
            }
            program.assign_ids();
            Ok(record(graft_id, actor, "ReplaceExpression", vec![target]))
        }
        GraftOperation::DeleteNode {
            target,
            precondition,
        } => {
            let task_index = owning_task_index_for_node(program, &target);
            check_preconditions(program, task_index, precondition.as_ref())?;
            let deleted = delete_node(program, &target)?;
            program.assign_ids();
            Ok(record(graft_id, actor, "DeleteNode", vec![target, deleted]))
        }
        GraftOperation::MoveNode {
            target,
            precondition,
            payload,
        } => {
            let task_index = owning_task_index_for_node(program, &target);
            check_preconditions(program, task_index, precondition.as_ref())?;
            let payload = payload.ok_or_else(|| {
                vec![Diagnostic::error(
                    "GRAFT_MISSING_PAYLOAD",
                    "MoveNode requires payload.position",
                )]
            })?;
            let moved = move_node(program, &target, &payload)?;
            program.assign_ids();
            Ok(record(graft_id, actor, "MoveNode", vec![target, moved]))
        }
    }
}

fn rename_module(program: &mut Program, old: &str, new: &str) -> Result<(), Vec<Diagnostic>> {
    if let Some(diagnostic) = validate_module_name(new) {
        return Err(vec![diagnostic]);
    }
    let modules = program_modules(program);
    if !modules.contains(old) {
        return Err(vec![Diagnostic::error(
            "GRAFT_TARGET_MISSING",
            format!("module target `module:{old}` does not exist"),
        )]);
    }
    if old == new {
        return Ok(());
    }
    if modules.contains(new) {
        return Err(vec![Diagnostic::error(
            "GRAFT_MODULE_EXISTS",
            format!("module `{new}` already exists"),
        )]);
    }

    let old_leaf = old.rsplit('.').next().unwrap_or(old).to_string();
    let new_leaf = new.rsplit('.').next().unwrap_or(new);
    if program.module_name() == old {
        program.module = Some(new.to_string());
    }
    for import in &mut program.imports {
        if import.owner_module.as_deref() == Some(old) {
            import.owner_module = Some(new.to_string());
        }
        if import.module == old {
            import.module = new.to_string();
            if import.alias.is_none() && old_leaf != new_leaf {
                import.alias = Some(old_leaf.clone());
            }
        }
    }
    for ty in &mut program.types {
        if type_module(ty) == old {
            ty.module = Some(new.to_string());
        }
    }
    for effect in &mut program.effects {
        if effect_module(effect) == old {
            effect.module = Some(new.to_string());
        }
    }
    for task in &mut program.tasks {
        if task_module(task) == old {
            task.module = Some(new.to_string());
        }
    }
    Ok(())
}

fn validate_module_name(module: &str) -> Option<Diagnostic> {
    if module.trim().is_empty() {
        return Some(Diagnostic::error(
            "GRAFT_MODULE_INVALID",
            "module path cannot be empty",
        ));
    }
    for segment in module.split('.') {
        if !is_module_segment(segment) {
            return Some(
                Diagnostic::error(
                    "GRAFT_MODULE_INVALID",
                    format!("module path `{module}` contains invalid segment `{segment}`"),
                )
                .with_node(format!("module:{module}")),
            );
        }
    }
    None
}

fn is_module_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn program_modules(program: &Program) -> BTreeSet<String> {
    let mut modules = BTreeSet::new();
    modules.insert(program.module_name().to_string());
    for import in &program.imports {
        modules.insert(import.module.clone());
        modules.insert(import_owner_module(import));
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
    modules
}

fn owning_task_index_for_node(program: &Program, target: &str) -> Option<usize> {
    if let Some(index) = program.find_task_index(target) {
        return Some(index);
    }
    program.tasks.iter().position(|task| {
        task.takes
            .iter()
            .any(|take| take_matches_target(task, take, target))
            || block_contains_statement(&task.body, target)
            || block_contains_expr(&task.body, target)
    })
}

fn delete_node(program: &mut Program, target: &str) -> Result<String, Vec<Diagnostic>> {
    if let Some(index) = program.find_task_index(target) {
        let task = program.tasks.remove(index);
        return Ok(format!("task:{}", task.name));
    }

    if let Some(index) = program
        .types
        .iter()
        .position(|ty| declaration_matches_target("type", &ty.id, &ty.name, target))
    {
        let ty = program.types.remove(index);
        return Ok(format!("type:{}", ty.name));
    }

    if let Some(index) = program
        .effects
        .iter()
        .position(|effect| declaration_matches_target("effect", &effect.id, &effect.name, target))
    {
        let effect = program.effects.remove(index);
        return Ok(format!("effect:{}", effect.name));
    }

    if let Some(index) = program
        .imports
        .iter()
        .position(|import| import_matches_target(import, target))
    {
        let import = program.imports.remove(index);
        return Ok(format!("import:{}", import.module));
    }

    for task in &mut program.tasks {
        if let Some(index) = task
            .takes
            .iter()
            .position(|take| take_matches_target(task, take, target))
        {
            let take = task.takes.remove(index);
            return Ok(format!("take:{}", take.name));
        }
    }

    for task in &mut program.tasks {
        if delete_statement_from_block(&mut task.body, target) {
            return Ok("statement".to_string());
        }
    }

    if expression_exists(program, target) {
        return Err(vec![
            Diagnostic::error(
                "GRAFT_DELETE_UNSUPPORTED",
                format!("expression target `{target}` cannot be deleted without replacement"),
            )
            .with_node(target.to_string()),
        ]);
    }

    Err(vec![Diagnostic::error(
        "GRAFT_TARGET_MISSING",
        format!("target `{target}` does not exist"),
    )])
}

fn move_node(
    program: &mut Program,
    target: &str,
    payload: &MoveNodePayload,
) -> Result<String, Vec<Diagnostic>> {
    if let Some(index) = program.find_task_index(target) {
        if let Some(module) =
            declaration_move_parent_module(program, payload, target, "task", "tasks")?
        {
            return move_declaration_to_module(
                &mut program.tasks,
                index,
                &module,
                payload,
                target,
                "task",
                task_module,
                |task, module| task.module = Some(module),
            );
        }
        return move_index(
            &mut program.tasks,
            index,
            payload,
            target,
            "task",
            &["program.tasks", "program:tasks", "tasks"],
        );
    }

    if let Some(index) = program
        .types
        .iter()
        .position(|ty| declaration_matches_target("type", &ty.id, &ty.name, target))
    {
        if let Some(module) =
            declaration_move_parent_module(program, payload, target, "type", "types")?
        {
            return move_declaration_to_module(
                &mut program.types,
                index,
                &module,
                payload,
                target,
                "type",
                type_module,
                |ty, module| ty.module = Some(module),
            );
        }
        return move_index(
            &mut program.types,
            index,
            payload,
            target,
            "type",
            &["program.types", "program:types", "types"],
        );
    }

    if let Some(index) = program
        .effects
        .iter()
        .position(|effect| declaration_matches_target("effect", &effect.id, &effect.name, target))
    {
        if let Some(module) =
            declaration_move_parent_module(program, payload, target, "effect", "effects")?
        {
            return move_declaration_to_module(
                &mut program.effects,
                index,
                &module,
                payload,
                target,
                "effect",
                effect_module,
                |effect, module| effect.module = Some(module),
            );
        }
        return move_index(
            &mut program.effects,
            index,
            payload,
            target,
            "effect",
            &["program.effects", "program:effects", "effects"],
        );
    }

    if let Some(index) = program
        .imports
        .iter()
        .position(|import| import_matches_target(import, target))
    {
        return move_index(
            &mut program.imports,
            index,
            payload,
            target,
            "import",
            &["program.imports", "program:imports", "imports"],
        );
    }

    if let Some(moved) = move_statement(program, target, payload)? {
        return Ok(moved);
    }

    if take_exists(program, target) {
        return Err(vec![
            Diagnostic::error(
                "GRAFT_MOVE_UNSUPPORTED",
                format!("take target `{target}` cannot be moved by v0 MoveNode"),
            )
            .with_node(target.to_string()),
        ]);
    }

    if expression_exists(program, target) {
        return Err(vec![
            Diagnostic::error(
                "GRAFT_MOVE_UNSUPPORTED",
                format!("expression target `{target}` cannot be moved by v0 MoveNode"),
            )
            .with_node(target.to_string()),
        ]);
    }

    Err(vec![Diagnostic::error(
        "GRAFT_TARGET_MISSING",
        format!("target `{target}` does not exist"),
    )])
}

fn declaration_move_parent_module(
    program: &Program,
    payload: &MoveNodePayload,
    target: &str,
    kind: &str,
    parent_kind: &str,
) -> Result<Option<String>, Vec<Diagnostic>> {
    let Some(parent) = payload.parent.as_deref() else {
        return Ok(None);
    };
    let Some(value) = parent.strip_prefix("module:") else {
        return Ok(None);
    };
    let Some((module, suffix)) = value.rsplit_once(':') else {
        return Err(vec![
            Diagnostic::error(
                "GRAFT_MOVE_UNSUPPORTED",
                format!("MoveNode {kind} parent `{parent}` must use `module:<path>:{parent_kind}`"),
            )
            .with_node(target.to_string()),
        ]);
    };
    if suffix != parent_kind {
        return Err(vec![
            Diagnostic::error(
                "GRAFT_MOVE_UNSUPPORTED",
                format!("MoveNode {kind} parent `{parent}` must use `module:<path>:{parent_kind}`"),
            )
            .with_node(target.to_string()),
        ]);
    }
    if let Some(diagnostic) = validate_module_name(module) {
        return Err(vec![diagnostic]);
    }
    if !program_modules(program).contains(module) {
        return Err(vec![
            Diagnostic::error(
                "GRAFT_MODULE_MISSING",
                format!("MoveNode destination module `{module}` is not loaded or imported"),
            )
            .with_node(format!("module:{module}")),
        ]);
    }
    Ok(Some(module.to_string()))
}

fn move_declaration_to_module<T>(
    items: &mut Vec<T>,
    index: usize,
    destination_module: &str,
    payload: &MoveNodePayload,
    target: &str,
    kind: &str,
    module_of: impl Fn(&T) -> String,
    set_module: impl Fn(&mut T, String),
) -> Result<String, Vec<Diagnostic>> {
    let source_module = module_of(&items[index]);
    let destination_len = items
        .iter()
        .enumerate()
        .filter(|(item_index, item)| *item_index != index && module_of(item) == destination_module)
        .count();
    if payload.position > destination_len {
        return Err(position_out_of_range(
            target,
            payload.position,
            destination_len + 1,
            &format!("module:{destination_module}:{kind}s"),
        ));
    }

    let mut item = items.remove(index);
    set_module(&mut item, destination_module.to_string());
    let insert_index =
        declaration_insert_index(items, destination_module, payload.position, module_of);
    items.insert(insert_index, item);
    Ok(format!(
        "{kind}:{source_module}->{}:{}",
        destination_module, payload.position
    ))
}

fn declaration_insert_index<T>(
    items: &[T],
    destination_module: &str,
    position: usize,
    module_of: impl Fn(&T) -> String,
) -> usize {
    let mut seen = 0usize;
    for (index, item) in items.iter().enumerate() {
        if module_of(item) == destination_module {
            if seen == position {
                return index;
            }
            seen += 1;
        }
    }
    items.len()
}

fn move_index<T>(
    items: &mut Vec<T>,
    index: usize,
    payload: &MoveNodePayload,
    target: &str,
    kind: &str,
    accepted_parents: &[&str],
) -> Result<String, Vec<Diagnostic>> {
    ensure_parent_in_set(payload, accepted_parents, target)?;
    if payload.position >= items.len() {
        return Err(position_out_of_range(
            target,
            payload.position,
            items.len(),
            kind,
        ));
    }
    if index != payload.position {
        let item = items.remove(index);
        items.insert(payload.position, item);
    }
    Ok(format!("{kind}:{index}->{}", payload.position))
}

fn move_statement(
    program: &mut Program,
    target: &str,
    payload: &MoveNodePayload,
) -> Result<Option<String>, Vec<Diagnostic>> {
    for task in &mut program.tasks {
        let parent_root = format!("block:{}", task.id);
        if let Some(moved) = move_statement_in_block(&mut task.body, &parent_root, target, payload)?
        {
            return Ok(Some(moved));
        }
    }
    Ok(None)
}

fn move_statement_in_block(
    block: &mut Block,
    parent_root: &str,
    target: &str,
    payload: &MoveNodePayload,
) -> Result<Option<String>, Vec<Diagnostic>> {
    if let Some(index) = block
        .statements
        .iter()
        .position(|statement| statement.id == target)
    {
        ensure_parent_matches(payload, parent_root, target)?;
        if payload.position >= block.statements.len() {
            return Err(position_out_of_range(
                target,
                payload.position,
                block.statements.len(),
                parent_root,
            ));
        }
        if index != payload.position {
            let statement = block.statements.remove(index);
            block.statements.insert(payload.position, statement);
        }
        return Ok(Some(format!(
            "statement:{parent_root}:{index}->{}",
            payload.position
        )));
    }

    for statement in &mut block.statements {
        if let Some(moved) = move_statement_in_statement(statement, target, payload)? {
            return Ok(Some(moved));
        }
    }
    Ok(None)
}

fn move_statement_in_statement(
    statement: &mut Statement,
    target: &str,
    payload: &MoveNodePayload,
) -> Result<Option<String>, Vec<Diagnostic>> {
    match &mut statement.kind {
        StatementKind::If {
            then_block,
            else_block,
            ..
        } => {
            let then_root = format!("{}:then", statement.id);
            if let Some(moved) = move_statement_in_block(then_block, &then_root, target, payload)? {
                return Ok(Some(moved));
            }
            if let Some(else_block) = else_block {
                let else_root = format!("{}:else", statement.id);
                if let Some(moved) =
                    move_statement_in_block(else_block, &else_root, target, payload)?
                {
                    return Ok(Some(moved));
                }
            }
            Ok(None)
        }
        StatementKind::While { body, .. } | StatementKind::For { body, .. } => {
            let body_root = format!("{}:body", statement.id);
            move_statement_in_block(body, &body_root, target, payload)
        }
        StatementKind::Forge { body } => {
            let forge_root = format!("{}:forge", statement.id);
            move_statement_in_block(body, &forge_root, target, payload)
        }
        StatementKind::Binding { .. }
        | StatementKind::Set { .. }
        | StatementKind::Return { .. }
        | StatementKind::Expr { .. } => Ok(None),
    }
}

fn ensure_parent_in_set(
    payload: &MoveNodePayload,
    accepted_parents: &[&str],
    target: &str,
) -> Result<(), Vec<Diagnostic>> {
    let Some(parent) = payload.parent.as_deref() else {
        return Ok(());
    };
    if accepted_parents.contains(&parent) {
        Ok(())
    } else {
        Err(vec![
            Diagnostic::error(
                "GRAFT_MOVE_UNSUPPORTED",
                format!(
                    "MoveNode for `{target}` cannot move across parents; expected one of {} but got `{parent}`",
                    accepted_parents.join(", ")
                ),
            )
            .with_node(target.to_string()),
        ])
    }
}

fn ensure_parent_matches(
    payload: &MoveNodePayload,
    expected_parent: &str,
    target: &str,
) -> Result<(), Vec<Diagnostic>> {
    let Some(parent) = payload.parent.as_deref() else {
        return Ok(());
    };
    if parent == expected_parent {
        Ok(())
    } else {
        Err(vec![
            Diagnostic::error(
                "GRAFT_MOVE_UNSUPPORTED",
                format!(
                    "MoveNode for `{target}` cannot move across parents; expected `{expected_parent}` but got `{parent}`"
                ),
            )
            .with_node(target.to_string()),
        ])
    }
}

fn position_out_of_range(
    target: &str,
    position: usize,
    length: usize,
    parent: &str,
) -> Vec<Diagnostic> {
    vec![
        Diagnostic::error(
            "GRAFT_POSITION_OUT_OF_RANGE",
            format!("position {position} is outside `{parent}` length {length}"),
        )
        .with_node(target.to_string()),
    ]
}

fn take_exists(program: &Program, target: &str) -> bool {
    program.tasks.iter().any(|task| {
        task.takes
            .iter()
            .any(|take| take_matches_target(task, take, target))
    })
}

fn declaration_matches_target(kind: &str, id: &str, name: &str, target: &str) -> bool {
    target == id
        || target == format!("{kind}:{name}")
        || target
            .strip_prefix(&format!("{kind}:"))
            .is_some_and(|bare| bare == name || id.ends_with(&format!(".{bare}")))
}

fn import_matches_target(import: &ImportDecl, target: &str) -> bool {
    target == import.id
        || target == import.module
        || target == format!("import:{}", import.module)
        || import
            .alias
            .as_deref()
            .is_some_and(|alias| target == format!("import:{alias}") || target == alias)
}

fn take_matches_target(task: &TaskDecl, take: &TakeDecl, target: &str) -> bool {
    target == take.id || target == format!("take:{}:{}", task.id, take.name)
}

fn block_contains_statement(block: &Block, target: &str) -> bool {
    block
        .statements
        .iter()
        .any(|statement| statement.id == target || statement_contains_statement(statement, target))
}

fn statement_contains_statement(statement: &Statement, target: &str) -> bool {
    match &statement.kind {
        StatementKind::If {
            then_block,
            else_block,
            ..
        } => {
            block_contains_statement(then_block, target)
                || else_block
                    .as_ref()
                    .is_some_and(|block| block_contains_statement(block, target))
        }
        StatementKind::While { body, .. }
        | StatementKind::For { body, .. }
        | StatementKind::Forge { body } => block_contains_statement(body, target),
        StatementKind::Binding { .. }
        | StatementKind::Set { .. }
        | StatementKind::Return { .. }
        | StatementKind::Expr { .. } => false,
    }
}

fn delete_statement_from_block(block: &mut Block, target: &str) -> bool {
    if let Some(index) = block
        .statements
        .iter()
        .position(|statement| statement.id == target)
    {
        block.statements.remove(index);
        return true;
    }

    block
        .statements
        .iter_mut()
        .any(|statement| delete_statement_in_statement(statement, target))
}

fn delete_statement_in_statement(statement: &mut Statement, target: &str) -> bool {
    match &mut statement.kind {
        StatementKind::If {
            then_block,
            else_block,
            ..
        } => {
            delete_statement_from_block(then_block, target)
                || else_block
                    .as_mut()
                    .is_some_and(|block| delete_statement_from_block(block, target))
        }
        StatementKind::While { body, .. }
        | StatementKind::For { body, .. }
        | StatementKind::Forge { body } => delete_statement_from_block(body, target),
        StatementKind::Binding { .. }
        | StatementKind::Set { .. }
        | StatementKind::Return { .. }
        | StatementKind::Expr { .. } => false,
    }
}

fn block_contains_expr(block: &Block, target: &str) -> bool {
    block
        .statements
        .iter()
        .any(|statement| statement_contains_expr(statement, target))
}

fn statement_contains_expr(statement: &Statement, target: &str) -> bool {
    match &statement.kind {
        StatementKind::Binding { expr, .. }
        | StatementKind::Set { expr, .. }
        | StatementKind::Return { expr }
        | StatementKind::Expr { expr } => expr_contains_target(expr, target),
        StatementKind::If {
            condition,
            then_block,
            else_block,
        } => {
            expr_contains_target(condition, target)
                || block_contains_expr(then_block, target)
                || else_block
                    .as_ref()
                    .is_some_and(|block| block_contains_expr(block, target))
        }
        StatementKind::While { condition, body } => {
            expr_contains_target(condition, target) || block_contains_expr(body, target)
        }
        StatementKind::For {
            collection, body, ..
        } => expr_contains_target(collection, target) || block_contains_expr(body, target),
        StatementKind::Forge { body } => block_contains_expr(body, target),
    }
}

fn expression_exists(program: &Program, target: &str) -> bool {
    program
        .tasks
        .iter()
        .any(|task| block_contains_expr(&task.body, target))
}

fn expr_contains_target(expr: &Expr, target: &str) -> bool {
    if expr.id == target {
        return true;
    }

    match &expr.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => expr_contains_target(expr, target),
        ExprKind::Binary { left, right, .. } => {
            expr_contains_target(left, target) || expr_contains_target(right, target)
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_contains_target(condition, target)
                || expr_contains_target(then_branch, target)
                || expr_contains_target(else_branch, target)
        }
        ExprKind::Call { callee, args } => {
            expr_contains_target(callee, target)
                || args.iter().any(|arg| expr_contains_target(arg, target))
        }
        ExprKind::ListLiteral { items } => {
            items.iter().any(|item| expr_contains_target(item, target))
        }
        ExprKind::MapLiteral { entries } => entries.iter().any(|entry| {
            expr_contains_target(&entry.key, target) || expr_contains_target(&entry.value, target)
        }),
        ExprKind::Index { collection, index } => {
            expr_contains_target(collection, target) || expr_contains_target(index, target)
        }
        ExprKind::FieldAccess { receiver, .. } => expr_contains_target(receiver, target),
        ExprKind::RecordLiteral { fields, .. } => fields
            .iter()
            .any(|field| expr_contains_target(&field.expr, target)),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => false,
    }
}

fn find_task_or_reject(program: &Program, target: &str) -> Result<usize, Vec<Diagnostic>> {
    program.find_task_index(target).ok_or_else(|| {
        vec![Diagnostic::error(
            "GRAFT_TARGET_MISSING",
            format!("task target `{target}` does not exist"),
        )]
    })
}

fn check_preconditions(
    program: &Program,
    task_index: Option<usize>,
    precondition: Option<&JsonValue>,
) -> Result<(), Vec<Diagnostic>> {
    let Some(precondition) = precondition else {
        return Ok(());
    };
    let mut diagnostics = Vec::new();
    if precondition.get("task_exists").and_then(JsonValue::as_bool) == Some(true)
        && task_index.is_none()
    {
        diagnostics.push(Diagnostic::error(
            "GRAFT_PRECONDITION_FAILED",
            "expected task to exist",
        ));
    }
    if let (Some(task_index), Some(absent)) = (
        task_index,
        precondition.get("take_absent").and_then(JsonValue::as_str),
    ) {
        let task = &program.tasks[task_index];
        if task.takes.iter().any(|take| take.name == absent) {
            diagnostics.push(
                Diagnostic::error(
                    "GRAFT_PRECONDITION_FAILED",
                    format!("take `{absent}` is present"),
                )
                .with_node(task.id.clone()),
            );
        }
    }
    if let (Some(task_index), Some(present)) = (
        task_index,
        precondition.get("take_present").and_then(JsonValue::as_str),
    ) {
        let task = &program.tasks[task_index];
        if !task.takes.iter().any(|take| take.name == present) {
            diagnostics.push(
                Diagnostic::error(
                    "GRAFT_PRECONDITION_FAILED",
                    format!("take `{present}` is absent"),
                )
                .with_node(task.id.clone()),
            );
        }
    }

    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics)
    }
}

fn parse_callee_source(source: &str) -> Result<Expr, Vec<Diagnostic>> {
    let expr = parse_expr_source(source)?;
    if callee_path(&expr).is_some() {
        return Ok(expr);
    }
    Err(vec![Diagnostic::error(
        "GRAFT_INVALID_CALLEE",
        "UpdateCallSites payload.replacement must be an identifier or dotted path",
    )])
}

fn matching_call_expr_ids(
    program: &Program,
    target_fq_name: &str,
    payload: &UpdateCallSitesPayload,
) -> BTreeSet<String> {
    collect_task_calls(program)
        .into_iter()
        .filter(|call| call_is_in_scope(call, payload.scope.as_deref()))
        .filter(|call| {
            if let Some(from) = payload.from.as_deref() {
                call.callee == from
            } else {
                call.status == "resolved" && call.target.as_deref() == Some(target_fq_name)
            }
        })
        .map(|call| call.expr_id)
        .collect()
}

fn call_is_in_scope(call: &crate::symbols::TaskCallSummary, scope: Option<&str>) -> bool {
    let Some(scope) = scope else {
        return true;
    };
    let task_scope = scope.strip_prefix("task:").unwrap_or(scope);
    let module_scope = scope.strip_prefix("module:").unwrap_or(scope);
    call.from == task_scope || call.from_module == module_scope
}

fn update_call_callees(
    program: &mut Program,
    call_ids: &BTreeSet<String>,
    replacement: &Expr,
) -> usize {
    program
        .tasks
        .iter_mut()
        .map(|task| update_call_callees_in_block(&mut task.body, call_ids, replacement))
        .sum()
}

fn update_call_callees_in_block(
    block: &mut Block,
    call_ids: &BTreeSet<String>,
    replacement: &Expr,
) -> usize {
    block
        .statements
        .iter_mut()
        .map(|statement| update_call_callees_in_statement(statement, call_ids, replacement))
        .sum()
}

fn update_call_callees_in_statement(
    statement: &mut Statement,
    call_ids: &BTreeSet<String>,
    replacement: &Expr,
) -> usize {
    match &mut statement.kind {
        StatementKind::Binding { expr, .. }
        | StatementKind::Set { expr, .. }
        | StatementKind::Return { expr }
        | StatementKind::Expr { expr } => update_call_callees_in_expr(expr, call_ids, replacement),
        StatementKind::If {
            condition,
            then_block,
            else_block,
        } => {
            let mut updated = update_call_callees_in_expr(condition, call_ids, replacement);
            updated += update_call_callees_in_block(then_block, call_ids, replacement);
            if let Some(else_block) = else_block {
                updated += update_call_callees_in_block(else_block, call_ids, replacement);
            }
            updated
        }
        StatementKind::While { condition, body } => {
            update_call_callees_in_expr(condition, call_ids, replacement)
                + update_call_callees_in_block(body, call_ids, replacement)
        }
        StatementKind::For {
            collection, body, ..
        } => {
            update_call_callees_in_expr(collection, call_ids, replacement)
                + update_call_callees_in_block(body, call_ids, replacement)
        }
        StatementKind::Forge { body } => update_call_callees_in_block(body, call_ids, replacement),
    }
}

fn update_call_callees_in_expr(
    expr: &mut Expr,
    call_ids: &BTreeSet<String>,
    replacement: &Expr,
) -> usize {
    let current_id = expr.id.clone();
    let mut updated = 0usize;
    match &mut expr.kind {
        ExprKind::Unary { expr: inner, .. } | ExprKind::Try { expr: inner } => {
            updated += update_call_callees_in_expr(inner, call_ids, replacement);
        }
        ExprKind::Binary { left, right, .. } => {
            updated += update_call_callees_in_expr(left, call_ids, replacement);
            updated += update_call_callees_in_expr(right, call_ids, replacement);
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            updated += update_call_callees_in_expr(condition, call_ids, replacement);
            updated += update_call_callees_in_expr(then_branch, call_ids, replacement);
            updated += update_call_callees_in_expr(else_branch, call_ids, replacement);
        }
        ExprKind::Call { callee, args } => {
            if call_ids.contains(&current_id) {
                **callee = replacement.clone();
                updated += 1;
            } else {
                updated += update_call_callees_in_expr(callee, call_ids, replacement);
            }
            for arg in args {
                updated += update_call_callees_in_expr(arg, call_ids, replacement);
            }
        }
        ExprKind::ListLiteral { items } => {
            for item in items {
                updated += update_call_callees_in_expr(item, call_ids, replacement);
            }
        }
        ExprKind::MapLiteral { entries } => {
            for entry in entries {
                updated += update_call_callees_in_expr(&mut entry.key, call_ids, replacement);
                updated += update_call_callees_in_expr(&mut entry.value, call_ids, replacement);
            }
        }
        ExprKind::Index { collection, index } => {
            updated += update_call_callees_in_expr(collection, call_ids, replacement);
            updated += update_call_callees_in_expr(index, call_ids, replacement);
        }
        ExprKind::FieldAccess { receiver, .. } => {
            updated += update_call_callees_in_expr(receiver, call_ids, replacement);
        }
        ExprKind::RecordLiteral { fields, .. } => {
            for field in fields {
                updated += update_call_callees_in_expr(&mut field.expr, call_ids, replacement);
            }
        }
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => {}
    }
    if updated > 0 {
        refresh_expr_source(expr);
    }
    updated
}

fn replace_expression(program: &mut Program, target: &str, replacement: Expr) -> bool {
    for task in &mut program.tasks {
        if replace_expression_in_block(&mut task.body, target, &replacement) {
            return true;
        }
    }
    false
}

fn replace_expression_in_block(block: &mut Block, target: &str, replacement: &Expr) -> bool {
    for statement in &mut block.statements {
        if replace_expression_in_statement(statement, target, replacement) {
            return true;
        }
    }
    false
}

fn replace_expression_in_statement(
    statement: &mut Statement,
    target: &str,
    replacement: &Expr,
) -> bool {
    match &mut statement.kind {
        StatementKind::Binding { expr, .. }
        | StatementKind::Set { expr, .. }
        | StatementKind::Return { expr }
        | StatementKind::Expr { expr } => replace_expression_in_expr(expr, target, replacement),
        StatementKind::If {
            condition,
            then_block,
            else_block,
        } => {
            if replace_expression_in_expr(condition, target, replacement)
                || replace_expression_in_block(then_block, target, replacement)
            {
                return true;
            }
            else_block
                .as_mut()
                .is_some_and(|block| replace_expression_in_block(block, target, replacement))
        }
        StatementKind::While { condition, body } => {
            replace_expression_in_expr(condition, target, replacement)
                || replace_expression_in_block(body, target, replacement)
        }
        StatementKind::For {
            collection, body, ..
        } => {
            replace_expression_in_expr(collection, target, replacement)
                || replace_expression_in_block(body, target, replacement)
        }
        StatementKind::Forge { body } => replace_expression_in_block(body, target, replacement),
    }
}

fn replace_expression_in_expr(expr: &mut Expr, target: &str, replacement: &Expr) -> bool {
    if expr.id == target {
        *expr = replacement.clone();
        return true;
    }

    let replaced = match &mut expr.kind {
        ExprKind::Unary { expr: inner, .. } | ExprKind::Try { expr: inner } => {
            replace_expression_in_expr(inner, target, replacement)
        }
        ExprKind::Binary { left, right, .. } => {
            replace_expression_in_expr(left, target, replacement)
                || replace_expression_in_expr(right, target, replacement)
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            replace_expression_in_expr(condition, target, replacement)
                || replace_expression_in_expr(then_branch, target, replacement)
                || replace_expression_in_expr(else_branch, target, replacement)
        }
        ExprKind::Call { callee, args } => {
            if replace_expression_in_expr(callee, target, replacement) {
                true
            } else {
                args.iter_mut()
                    .any(|arg| replace_expression_in_expr(arg, target, replacement))
            }
        }
        ExprKind::ListLiteral { items } => items
            .iter_mut()
            .any(|item| replace_expression_in_expr(item, target, replacement)),
        ExprKind::MapLiteral { entries } => entries.iter_mut().any(|entry| {
            replace_expression_in_expr(&mut entry.key, target, replacement)
                || replace_expression_in_expr(&mut entry.value, target, replacement)
        }),
        ExprKind::Index { collection, index } => {
            replace_expression_in_expr(collection, target, replacement)
                || replace_expression_in_expr(index, target, replacement)
        }
        ExprKind::FieldAccess { receiver, .. } => {
            replace_expression_in_expr(receiver, target, replacement)
        }
        ExprKind::RecordLiteral { fields, .. } => fields
            .iter_mut()
            .any(|field| replace_expression_in_expr(&mut field.expr, target, replacement)),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => false,
    };

    if replaced {
        refresh_expr_source(expr);
    }
    replaced
}

fn refresh_expr_source(expr: &mut Expr) {
    let had_call_keyword = expr.source.trim_start().starts_with("call ");
    expr.source = match &expr.kind {
        ExprKind::Raw { .. } => expr.source.clone(),
        ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. } => expr.source.clone(),
        ExprKind::BoolLiteral { value } => value.to_string(),
        ExprKind::Identifier { name } => name.clone(),
        ExprKind::Unary { op, expr: inner } => {
            format!(
                "{}{}",
                op.as_str(),
                format_child_expr(inner, expr_precedence(expr), false)
            )
        }
        ExprKind::Binary { op, left, right } => {
            let precedence = binary_precedence(op);
            format!(
                "{} {} {}",
                format_child_expr(left, precedence, false),
                op.as_str(),
                format_child_expr(right, precedence, true)
            )
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => format!(
            "if {} {{ {} }} else {{ {} }}",
            condition.source, then_branch.source, else_branch.source
        ),
        ExprKind::Call { callee, args } => {
            let args_source = args
                .iter()
                .map(|arg| arg.source.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let call_source = format!("{}({args_source})", callee.source);
            if had_call_keyword {
                format!("call {call_source}")
            } else {
                call_source
            }
        }
        ExprKind::ListLiteral { items } => {
            let items_source = items
                .iter()
                .map(|item| item.source.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{items_source}]")
        }
        ExprKind::MapLiteral { entries } => format!("map {{ {} }}", format_map_entries(entries)),
        ExprKind::Index { collection, index } => {
            format!("{}[{}]", collection.source, index.source)
        }
        ExprKind::FieldAccess { receiver, field } => format!("{}.{}", receiver.source, field),
        ExprKind::RecordLiteral { type_name, fields } => {
            let fields_source = format_expr_fields(fields);
            if let Some(type_name) = type_name {
                format!("{type_name} {{ {fields_source} }}")
            } else {
                format!("{{ {fields_source} }}")
            }
        }
        ExprKind::Try { expr: inner } => {
            format!(
                "{}?",
                format_child_expr(inner, expr_precedence(expr), false)
            )
        }
    };
}

fn format_child_expr(expr: &Expr, parent_precedence: u8, is_right_child: bool) -> String {
    let child_precedence = expr_precedence(expr);
    let needs_parentheses = child_precedence < parent_precedence
        || (is_right_child && child_precedence == parent_precedence);
    if needs_parentheses && !is_wrapped_in_parentheses(&expr.source) {
        format!("({})", expr.source)
    } else {
        expr.source.clone()
    }
}

fn is_wrapped_in_parentheses(source: &str) -> bool {
    let source = source.trim();
    source.starts_with('(') && source.ends_with(')')
}

fn format_expr_fields(fields: &[ExprField]) -> String {
    fields
        .iter()
        .map(|field| format!("{}: {}", field.name, field.expr.source))
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_map_entries(entries: &[ExprMapEntry]) -> String {
    entries
        .iter()
        .map(|entry| format!("{}: {}", entry.key.source, entry.value.source))
        .collect::<Vec<_>>()
        .join(", ")
}

fn expr_precedence(expr: &Expr) -> u8 {
    match &expr.kind {
        ExprKind::If { .. } => 0,
        ExprKind::Binary { op, .. } => binary_precedence(op),
        ExprKind::Unary { .. } => 7,
        ExprKind::Call { .. }
        | ExprKind::Index { .. }
        | ExprKind::FieldAccess { .. }
        | ExprKind::Try { .. } => 8,
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. }
        | ExprKind::ListLiteral { .. }
        | ExprKind::MapLiteral { .. }
        | ExprKind::RecordLiteral { .. } => 9,
    }
}

fn binary_precedence(op: &BinaryOp) -> u8 {
    match op {
        BinaryOp::Or => 1,
        BinaryOp::And => 2,
        BinaryOp::Equal | BinaryOp::NotEqual => 3,
        BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => 4,
        BinaryOp::Add | BinaryOp::Subtract => 5,
        BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Remainder => 6,
    }
}

fn record(graft_id: &str, actor: &str, operation: &str, targets: Vec<String>) -> ProvenanceRecord {
    ProvenanceRecord {
        graft_id: graft_id.to_string(),
        actor: actor.to_string(),
        timestamp: OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| format!("unix:{}", unix_timestamp())),
        operation: operation.to_string(),
        targets,
        result: "accepted".to_string(),
    }
}

fn unix_timestamp() -> i128 {
    OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000_000
}
