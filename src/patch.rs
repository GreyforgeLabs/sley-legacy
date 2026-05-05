use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::ast::{
    Block, EffectDecl, FunctionDecl, ImportDecl, Param, Program, ProvenanceRecord, TypeDecl,
};
use crate::checker::{check_program, has_errors};
use crate::diagnostics::Diagnostic;
use crate::formatter::format_program;
use crate::parser::{parse_block_source, parse_program, parse_type_expr_source};

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum PatchInput {
    Transaction(PatchTransaction),
    Operation(PatchOperation),
}

#[derive(Debug, Clone, Deserialize)]
pub struct PatchTransaction {
    pub transaction: String,
    #[serde(default)]
    pub actor: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    pub ops: Vec<PatchOperation>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "op")]
pub enum PatchOperation {
    AddFunction {
        #[serde(default)]
        precondition: Option<JsonValue>,
        payload: SourcePayload,
    },
    ReplaceFunctionBody {
        target: String,
        #[serde(default)]
        precondition: Option<JsonValue>,
        payload: BodyPayload,
    },
    AddParameter {
        target: String,
        #[serde(default)]
        precondition: Option<JsonValue>,
        payload: AddParameterPayload,
    },
    RemoveParameter {
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
pub struct AddParameterPayload {
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
pub struct PatchOutcome {
    pub status: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provenance: Vec<ProvenanceRecord>,
}

pub fn apply_patch_input(
    program: &Program,
    input: PatchInput,
    actor: Option<String>,
) -> PatchOutcome {
    let actor = actor.unwrap_or_else(|| "agent:unknown".to_string());
    let mut candidate = program.clone();
    let mut provenance = Vec::new();
    let mut diagnostics = Vec::new();

    match input {
        PatchInput::Transaction(transaction) => {
            let patch_id = transaction.transaction;
            let actor = transaction.actor.unwrap_or(actor);
            if transaction
                .mode
                .as_deref()
                .is_some_and(|mode| mode != "all_or_nothing")
            {
                diagnostics.push(Diagnostic::error(
                    "PATCH_UNSUPPORTED_MODE",
                    "only `all_or_nothing` transaction mode is supported",
                ));
            } else {
                for op in transaction.ops {
                    match apply_one(&mut candidate, op, &patch_id, &actor) {
                        Ok(record) => provenance.push(record),
                        Err(mut op_diagnostics) => {
                            diagnostics.append(&mut op_diagnostics);
                            break;
                        }
                    }
                }
            }
        }
        PatchInput::Operation(op) => {
            let patch_id = format!("patch_{}", unix_timestamp());
            match apply_one(&mut candidate, op, &patch_id, &actor) {
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
        PatchOutcome {
            status: "rejected".to_string(),
            diagnostics,
            source: None,
            provenance: Vec::new(),
        }
    } else {
        candidate.provenance.extend(provenance.clone());
        PatchOutcome {
            status: "accepted".to_string(),
            diagnostics,
            source: Some(format_program(&candidate)),
            provenance,
        }
    }
}

fn apply_one(
    program: &mut Program,
    op: PatchOperation,
    patch_id: &str,
    actor: &str,
) -> Result<ProvenanceRecord, Vec<Diagnostic>> {
    match op {
        PatchOperation::AddParameter {
            target,
            precondition,
            payload,
        } => {
            let index = find_function_or_reject(program, &target)?;
            check_preconditions(program, Some(index), precondition.as_ref())?;
            let ty = parse_type_expr_source(&payload.ty)?;
            let function = &mut program.functions[index];
            if function
                .params
                .iter()
                .any(|param| param.name == payload.name)
            {
                return Err(vec![
                    Diagnostic::error(
                        "PATCH_PARAMETER_EXISTS",
                        format!("parameter `{}` already exists", payload.name),
                    )
                    .with_node(function.id.clone()),
                ]);
            }
            let position = payload.position.unwrap_or(function.params.len());
            let param = Param {
                id: String::new(),
                name: payload.name,
                ty,
                span: None,
            };
            if position >= function.params.len() {
                function.params.push(param);
            } else {
                function.params.insert(position, param);
            }
            program.assign_ids();
            Ok(record(patch_id, actor, "AddParameter", vec![target]))
        }
        PatchOperation::RemoveParameter {
            target,
            precondition,
            payload,
        } => {
            let index = find_function_or_reject(program, &target)?;
            check_preconditions(program, Some(index), precondition.as_ref())?;
            let function = &mut program.functions[index];
            let before = function.params.len();
            function.params.retain(|param| param.name != payload.name);
            if before == function.params.len() {
                return Err(vec![
                    Diagnostic::error(
                        "PATCH_PARAMETER_MISSING",
                        format!("parameter `{}` is absent", payload.name),
                    )
                    .with_node(function.id.clone()),
                ]);
            }
            program.assign_ids();
            Ok(record(patch_id, actor, "RemoveParameter", vec![target]))
        }
        PatchOperation::ReplaceFunctionBody {
            target,
            precondition,
            payload,
        } => {
            let index = find_function_or_reject(program, &target)?;
            check_preconditions(program, Some(index), precondition.as_ref())?;
            let source = payload
                .source
                .or_else(|| payload.statements.map(|statements| statements.join("\n")))
                .ok_or_else(|| {
                    vec![Diagnostic::error(
                        "PATCH_MISSING_BODY",
                        "ReplaceFunctionBody requires payload.source or payload.statements",
                    )]
                })?;
            let body: Block = parse_block_source(&source)?;
            program.functions[index].body = body;
            program.assign_ids();
            Ok(record(patch_id, actor, "ReplaceFunctionBody", vec![target]))
        }
        PatchOperation::AddEffectDeclaration {
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
                    "PATCH_EFFECT_EXISTS",
                    format!("effect `{}` already exists", payload.name),
                )]);
            }
            program.effects.push(EffectDecl {
                id: String::new(),
                name: payload.name.clone(),
                span: None,
            });
            program.assign_ids();
            Ok(record(
                patch_id,
                actor,
                "AddEffectDeclaration",
                vec![format!("effect:{}", payload.name)],
            ))
        }
        PatchOperation::AddImport {
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
                    "PATCH_IMPORT_EXISTS",
                    format!("import `{}` already exists", payload.module),
                )]);
            }
            program.imports.push(ImportDecl {
                id: String::new(),
                module: payload.module.clone(),
                span: None,
            });
            program.assign_ids();
            Ok(record(
                patch_id,
                actor,
                "AddImport",
                vec![format!("import:{}", payload.module)],
            ))
        }
        PatchOperation::AddFunction {
            precondition,
            payload,
        } => {
            check_preconditions(program, None, precondition.as_ref())?;
            let parsed = parse_program(&payload.source)?;
            let Some(function) = parsed.functions.into_iter().next() else {
                return Err(vec![Diagnostic::error(
                    "PATCH_EXPECTED_FUNCTION",
                    "AddFunction payload.source must contain a function",
                )]);
            };
            if program
                .functions
                .iter()
                .any(|item| item.name == function.name)
            {
                return Err(vec![Diagnostic::error(
                    "PATCH_FUNCTION_EXISTS",
                    format!("function `{}` already exists", function.name),
                )]);
            }
            let target = function.name.clone();
            program.functions.push(FunctionDecl { ..function });
            program.assign_ids();
            Ok(record(
                patch_id,
                actor,
                "AddFunction",
                vec![format!("function:{target}")],
            ))
        }
        PatchOperation::AddTypeDeclaration {
            precondition,
            payload,
        } => {
            check_preconditions(program, None, precondition.as_ref())?;
            let parsed = parse_program(&payload.source)?;
            let Some(ty) = parsed.types.into_iter().next() else {
                return Err(vec![Diagnostic::error(
                    "PATCH_EXPECTED_TYPE",
                    "AddTypeDeclaration payload.source must contain a type declaration",
                )]);
            };
            if program.types.iter().any(|item| item.name == ty.name) {
                return Err(vec![Diagnostic::error(
                    "PATCH_TYPE_EXISTS",
                    format!("type `{}` already exists", ty.name),
                )]);
            }
            let target = ty.name.clone();
            program.types.push(TypeDecl { ..ty });
            program.assign_ids();
            Ok(record(
                patch_id,
                actor,
                "AddTypeDeclaration",
                vec![format!("type:{target}")],
            ))
        }
        PatchOperation::RenameDeclaration {
            target,
            precondition,
            payload,
        } => {
            check_preconditions(program, None, precondition.as_ref())?;
            if let Some(index) = program.find_function_index(&target) {
                program.functions[index].name = payload.name.clone();
                program.assign_ids();
                return Ok(record(patch_id, actor, "RenameDeclaration", vec![target]));
            }
            if let Some(name) = target.strip_prefix("type:")
                && let Some(ty) = program
                    .types
                    .iter_mut()
                    .find(|ty| ty.name == name || ty.id == target)
            {
                ty.name = payload.name.clone();
                program.assign_ids();
                return Ok(record(patch_id, actor, "RenameDeclaration", vec![target]));
            }
            if let Some(name) = target.strip_prefix("effect:")
                && let Some(effect) = program
                    .effects
                    .iter_mut()
                    .find(|effect| effect.name == name || effect.id == target)
            {
                effect.name = payload.name.clone();
                program.assign_ids();
                return Ok(record(patch_id, actor, "RenameDeclaration", vec![target]));
            }
            Err(vec![Diagnostic::error(
                "PATCH_TARGET_MISSING",
                format!("target `{target}` does not exist"),
            )])
        }
        PatchOperation::UpdateCallSites { target }
        | PatchOperation::InsertStatement { target }
        | PatchOperation::ReplaceExpression { target }
        | PatchOperation::MoveNode { target }
        | PatchOperation::DeleteNode { target } => Err(vec![Diagnostic::error(
            "PATCH_OPERATION_UNSUPPORTED",
            format!(
                "patch operation for `{target}` is declared in the spec but not implemented yet"
            ),
        )]),
    }
}

fn find_function_or_reject(program: &Program, target: &str) -> Result<usize, Vec<Diagnostic>> {
    program.find_function_index(target).ok_or_else(|| {
        vec![Diagnostic::error(
            "PATCH_TARGET_MISSING",
            format!("function target `{target}` does not exist"),
        )]
    })
}

fn check_preconditions(
    program: &Program,
    function_index: Option<usize>,
    precondition: Option<&JsonValue>,
) -> Result<(), Vec<Diagnostic>> {
    let Some(precondition) = precondition else {
        return Ok(());
    };
    let mut diagnostics = Vec::new();
    if precondition
        .get("function_exists")
        .and_then(JsonValue::as_bool)
        == Some(true)
        && function_index.is_none()
    {
        diagnostics.push(Diagnostic::error(
            "PATCH_PRECONDITION_FAILED",
            "expected function to exist",
        ));
    }
    if let (Some(function_index), Some(absent)) = (
        function_index,
        precondition
            .get("parameter_absent")
            .and_then(JsonValue::as_str),
    ) {
        let function = &program.functions[function_index];
        if function.params.iter().any(|param| param.name == absent) {
            diagnostics.push(
                Diagnostic::error(
                    "PATCH_PRECONDITION_FAILED",
                    format!("parameter `{absent}` is present"),
                )
                .with_node(function.id.clone()),
            );
        }
    }
    if let (Some(function_index), Some(present)) = (
        function_index,
        precondition
            .get("parameter_present")
            .and_then(JsonValue::as_str),
    ) {
        let function = &program.functions[function_index];
        if !function.params.iter().any(|param| param.name == present) {
            diagnostics.push(
                Diagnostic::error(
                    "PATCH_PRECONDITION_FAILED",
                    format!("parameter `{present}` is absent"),
                )
                .with_node(function.id.clone()),
            );
        }
    }

    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(diagnostics)
    }
}

fn record(patch_id: &str, actor: &str, operation: &str, targets: Vec<String>) -> ProvenanceRecord {
    ProvenanceRecord {
        patch_id: patch_id.to_string(),
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
