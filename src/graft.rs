use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::ast::{
    BindingKind, Block, EffectDecl, ImportDecl, Program, ProvenanceRecord, TakeDecl, TaskDecl,
    TypeDecl,
};
use crate::checker::{check_program, has_errors};
use crate::diagnostics::Diagnostic;
use crate::formatter::format_program;
use crate::parser::{parse_block_source, parse_program, parse_type_expr_source};

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum GraftInput {
    Transaction(GraftTransaction),
    Operation(GraftOperation),
}

#[derive(Debug, Clone, Deserialize)]
pub struct GraftTransaction {
    pub transaction: String,
    #[serde(default)]
    pub actor: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    pub ops: Vec<GraftOperation>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "op")]
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
    },
    InsertStatement {
        target: String,
    },
    ReplaceExpression {
        target: String,
    },
    MoveNode {
        target: String,
    },
    DeleteNode {
        target: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct AddTakePayload {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(default)]
    pub position: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BodyPayload {
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub statements: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SourcePayload {
    pub source: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NamedPayload {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RenamePayload {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImportPayload {
    pub module: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraftOutcome {
    pub status: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provenance: Vec<ProvenanceRecord>,
}

pub fn apply_graft_input(
    program: &Program,
    input: GraftInput,
    actor: Option<String>,
) -> GraftOutcome {
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
        GraftOutcome {
            status: "rejected".to_string(),
            diagnostics,
            source: None,
            provenance: Vec::new(),
        }
    } else {
        candidate.provenance.extend(provenance.clone());
        GraftOutcome {
            status: "accepted".to_string(),
            diagnostics,
            source: Some(format_program(&candidate)),
            provenance,
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
            if program.tasks.iter().any(|item| item.name == task.name) {
                return Err(vec![Diagnostic::error(
                    "GRAFT_TASK_EXISTS",
                    format!("task `{}` already exists", task.name),
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
            if program.types.iter().any(|item| item.name == ty.name) {
                return Err(vec![Diagnostic::error(
                    "GRAFT_TYPE_EXISTS",
                    format!("type `{}` already exists", ty.name),
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
        GraftOperation::UpdateCallSites { target }
        | GraftOperation::InsertStatement { target }
        | GraftOperation::ReplaceExpression { target }
        | GraftOperation::MoveNode { target }
        | GraftOperation::DeleteNode { target } => Err(vec![Diagnostic::error(
            "GRAFT_OPERATION_UNSUPPORTED",
            format!(
                "graft operation for `{target}` is declared in the spec but not implemented yet"
            ),
        )]),
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
