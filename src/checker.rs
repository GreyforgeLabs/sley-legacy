use std::collections::{HashMap, HashSet};

use crate::ast::{
    BinaryOp, BindingKind, Block, Expr, ExprKind, ExprMapEntry, Program, RecordField,
    StatementKind, TaskDecl, TypeExpr, UnaryOp,
};
use crate::diagnostics::{Diagnostic, RepairHint};
use crate::symbols::{
    EffectResolution, TaskResolution, TypeResolution, callee_path, effect_module,
    is_module_qualified_callee, resolve_effect, resolve_task, resolve_type, task_module,
    type_fq_name, type_module,
};

pub fn check_program(program: &Program) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    collect_known_types(program, &mut diagnostics);
    collect_known_effects(program, &mut diagnostics);
    let known_tasks = collect_known_tasks(program, &mut diagnostics);
    let record_types = collect_record_types(program);

    for import in &program.imports {
        if import.module.trim().is_empty() {
            diagnostics.push(
                Diagnostic::error("EMPTY_IMPORT", "import module path cannot be empty")
                    .with_node(import.id.clone()),
            );
        }
    }

    for ty in &program.types {
        validate_type_expr(
            &ty.value,
            program,
            &type_module(ty),
            &mut diagnostics,
            &ty.id,
        );
    }

    for task in &program.tasks {
        check_task(program, task, &known_tasks, &record_types, &mut diagnostics);
    }

    diagnostics
}

#[derive(Debug, Clone)]
struct TaskSignature {
    takes: Vec<TypeExpr>,
    return_type: TypeExpr,
    effects: Vec<String>,
}

type TaskSignatures = HashMap<usize, TaskSignature>;

fn collect_known_tasks(program: &Program, diagnostics: &mut Vec<Diagnostic>) -> TaskSignatures {
    let mut known = HashMap::new();
    let mut seen = HashSet::new();
    for (index, task) in program.tasks.iter().enumerate() {
        if !seen.insert((task_module(task), task.name.clone())) {
            diagnostics.push(
                Diagnostic::error(
                    "DUPLICATE_TASK",
                    format!(
                        "task `{}` is declared more than once in module `{}`",
                        task.name,
                        task_module(task)
                    ),
                )
                .with_node(task.id.clone()),
            );
        }
        known.insert(
            index,
            TaskSignature {
                takes: task
                    .takes
                    .iter()
                    .filter(|take| take.binding_kind != BindingKind::Gate)
                    .map(|take| normalize_type_expr_silent(program, &task_module(task), &take.ty))
                    .collect(),
                return_type: normalize_type_expr_silent(
                    program,
                    &task_module(task),
                    &task.return_type,
                ),
                effects: task
                    .effects
                    .iter()
                    .map(|effect| normalize_effect_name_silent(program, &task_module(task), effect))
                    .collect(),
            },
        );
    }
    known
}

fn collect_record_types(program: &Program) -> HashMap<String, Vec<RecordField>> {
    program
        .types
        .iter()
        .filter_map(|ty| match &ty.value {
            TypeExpr::Record { fields } => Some((
                type_fq_name(ty),
                fields
                    .iter()
                    .map(|field| RecordField {
                        name: field.name.clone(),
                        ty: normalize_type_expr_silent(program, &type_module(ty), &field.ty),
                    })
                    .collect(),
            )),
            _ => None,
        })
        .collect()
}

fn collect_known_types(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    let mut local_types = HashSet::new();
    for ty in &program.types {
        if !local_types.insert((type_module(ty), ty.name.clone())) {
            diagnostics.push(
                Diagnostic::error(
                    "DUPLICATE_TYPE",
                    format!(
                        "type `{}` is already declared in module `{}`",
                        ty.name,
                        type_module(ty)
                    ),
                )
                .with_node(ty.id.clone()),
            );
        }
    }
}

fn collect_known_effects(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    let mut local_effects = HashSet::new();
    for effect in &program.effects {
        if !local_effects.insert((effect_module(effect), effect.name.clone())) {
            diagnostics.push(
                Diagnostic::error(
                    "DUPLICATE_EFFECT",
                    format!(
                        "effect `{}` is already declared in module `{}`",
                        effect.name,
                        effect_module(effect)
                    ),
                )
                .with_node(effect.id.clone()),
            );
        }
    }
}

fn check_task(
    program: &Program,
    task: &TaskDecl,
    known_tasks: &TaskSignatures,
    record_types: &HashMap<String, Vec<RecordField>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut takes = HashSet::new();
    let mut locals = HashMap::new();
    let mut local_bindings = HashMap::new();
    let declared_effects = task
        .effects
        .iter()
        .map(|effect| normalize_effect_name_silent(program, &task_module(task), effect))
        .collect::<HashSet<_>>();

    for take in &task.takes {
        if !takes.insert(take.name.clone()) {
            diagnostics.push(
                Diagnostic::error(
                    "DUPLICATE_TAKE",
                    format!(
                        "take `{}` appears more than once in `{}`",
                        take.name, task.name
                    ),
                )
                .with_node(task.id.clone()),
            );
        }
        if take.binding_kind == BindingKind::Gate {
            validate_gate_take(
                take,
                program,
                &task_module(task),
                &declared_effects,
                task,
                diagnostics,
            );
        } else {
            validate_type_expr(&take.ty, program, &task_module(task), diagnostics, &take.id);
        }
        locals.insert(
            take.name.clone(),
            normalize_type_expr_silent(program, &task_module(task), &take.ty),
        );
        local_bindings.insert(take.name.clone(), take.binding_kind.clone());
    }

    validate_type_expr(
        &task.return_type,
        program,
        &task_module(task),
        diagnostics,
        &task.id,
    );

    for effect in &task.effects {
        validate_effect_name(effect, program, &task_module(task), task, diagnostics);
    }

    check_block(
        program,
        task,
        &task.body,
        &mut locals,
        &mut local_bindings,
        &declared_effects,
        known_tasks,
        record_types,
        diagnostics,
    );
}

#[allow(clippy::too_many_arguments)]
fn check_block(
    program: &Program,
    task: &TaskDecl,
    block: &Block,
    locals: &mut HashMap<String, TypeExpr>,
    local_bindings: &mut HashMap<String, BindingKind>,
    declared_effects: &HashSet<String>,
    known_tasks: &TaskSignatures,
    record_types: &HashMap<String, Vec<RecordField>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for statement in &block.statements {
        match &statement.kind {
            StatementKind::Binding {
                binding_kind,
                name,
                type_ann,
                expr,
            } => {
                check_expr_common(
                    program,
                    task,
                    expr,
                    locals,
                    declared_effects,
                    known_tasks,
                    record_types,
                    diagnostics,
                );
                if let Some(type_ann) = type_ann {
                    validate_type_expr(
                        type_ann,
                        program,
                        &task_module(task),
                        diagnostics,
                        &statement.id,
                    );
                }
                let inferred =
                    infer_expr_type(program, task, expr, locals, known_tasks, record_types);
                let expected = type_ann
                    .as_ref()
                    .map(|ty| normalize_type_expr_silent(program, &task_module(task), ty));
                if let (Some(expected), Some(actual)) = (expected.as_ref(), inferred.as_ref())
                    && !types_compatible(expected, actual)
                {
                    diagnostics.push(
                        Diagnostic::error(
                            "TYPE_MISMATCH",
                            format!(
                                "{} `{name}` expects `{}` but initializer looks like `{}`",
                                binding_kind.as_source_keyword(),
                                expected.display(),
                                actual.display()
                            ),
                        )
                        .with_node(statement.id.clone())
                        .with_repair_hint(
                            RepairHint::new("change_binding_type")
                                .with_target(statement.id.clone())
                                .with_replacement(actual.display()),
                        )
                        .with_repair_hint(
                            RepairHint::new("replace_initializer")
                                .with_target(expr.id.clone())
                                .with_replacement(expected.display()),
                        ),
                    );
                }
                let local_type = expected
                    .or(inferred)
                    .unwrap_or_else(|| TypeExpr::named("Unit"));
                if locals.contains_key(name) {
                    diagnostics.push(
                        Diagnostic::error(
                            "LOCAL_ALREADY_BOUND",
                            format!("local binding `{name}` is already in scope"),
                        )
                        .with_node(statement.id.clone()),
                    );
                }
                locals.insert(name.clone(), local_type);
                local_bindings.insert(name.clone(), binding_kind.clone());
            }
            StatementKind::Set { name, expr } => {
                check_expr_common(
                    program,
                    task,
                    expr,
                    locals,
                    declared_effects,
                    known_tasks,
                    record_types,
                    diagnostics,
                );
                let Some(expected) = locals.get(name).cloned() else {
                    diagnostics.push(
                        Diagnostic::error(
                            "UNKNOWN_IDENTIFIER",
                            format!("cannot set unknown local binding `{name}`"),
                        )
                        .with_node(statement.id.clone())
                        .with_repair_hint(declare_binding_hint(task, name, BindingKind::State)),
                    );
                    continue;
                };
                if !local_bindings
                    .get(name)
                    .is_some_and(BindingKind::is_mutable_local)
                {
                    diagnostics.push(
                        Diagnostic::error(
                            "BINDING_NOT_MUTABLE",
                            format!(
                                "binding `{name}` cannot be changed with `set`; use `state`, `tally`, `cache`, or another mutable Sley binding kind"
                            ),
                        )
                        .with_node(statement.id.clone())
                        .with_repair_hint(
                            RepairHint::new("use_mutable_binding_kind")
                                .with_target(statement.id.clone())
                                .with_replacement(format!("state {name} = <value>")),
                        ),
                    );
                }
                if let Some(actual) =
                    infer_expr_type(program, task, expr, locals, known_tasks, record_types)
                    && !types_compatible(&expected, &actual)
                {
                    diagnostics.push(
                        Diagnostic::error(
                            "SET_TYPE_MISMATCH",
                            format!(
                                "set `{name}` expects `{}` but expression looks like `{}`",
                                expected.display(),
                                actual.display()
                            ),
                        )
                        .with_node(statement.id.clone())
                        .with_repair_hint(
                            RepairHint::new("replace_assignment_expression")
                                .with_target(expr.id.clone())
                                .with_replacement(expected.display()),
                        ),
                    );
                }
            }
            StatementKind::Return { expr } => {
                check_expr_common(
                    program,
                    task,
                    expr,
                    locals,
                    declared_effects,
                    known_tasks,
                    record_types,
                    diagnostics,
                );
                let expected_return =
                    normalize_type_expr_silent(program, &task_module(task), &task.return_type);
                if let Some(actual) =
                    infer_expr_type(program, task, expr, locals, known_tasks, record_types)
                    && !return_types_compatible(&expected_return, &actual, &expr.source)
                {
                    diagnostics.push(
                        Diagnostic::error(
                            "RETURN_TYPE_MISMATCH",
                            format!(
                                "task `{}` returns `{}` but expression looks like `{}`",
                                task.name,
                                expected_return.display(),
                                actual.display()
                            ),
                        )
                        .with_node(task.id.clone())
                        .with_repair_hint(
                            RepairHint::new("change_return_type")
                                .with_target(task.id.clone())
                                .with_replacement(actual.display()),
                        )
                        .with_repair_hint(
                            RepairHint::new("replace_return_expression")
                                .with_target(expr.id.clone())
                                .with_replacement(expected_return.display()),
                        ),
                    );
                }
            }
            StatementKind::Expr { expr } => {
                check_expr_common(
                    program,
                    task,
                    expr,
                    locals,
                    declared_effects,
                    known_tasks,
                    record_types,
                    diagnostics,
                );
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                check_expr_common(
                    program,
                    task,
                    condition,
                    locals,
                    declared_effects,
                    known_tasks,
                    record_types,
                    diagnostics,
                );
                if let Some(condition_type) =
                    infer_expr_type(program, task, condition, locals, known_tasks, record_types)
                    && !is_bool_type(&condition_type)
                {
                    diagnostics.push(
                        Diagnostic::error(
                            "IF_CONDITION_NOT_BOOL",
                            format!(
                                "if condition must be `Bool`, not `{}`",
                                condition_type.display()
                            ),
                        )
                        .with_node(condition.id.clone())
                        .with_repair_hint(bool_condition_hint(condition)),
                    );
                }
                let mut then_locals = locals.clone();
                let mut then_bindings = local_bindings.clone();
                check_block(
                    program,
                    task,
                    then_block,
                    &mut then_locals,
                    &mut then_bindings,
                    declared_effects,
                    known_tasks,
                    record_types,
                    diagnostics,
                );
                if let Some(else_block) = else_block {
                    let mut else_locals = locals.clone();
                    let mut else_bindings = local_bindings.clone();
                    check_block(
                        program,
                        task,
                        else_block,
                        &mut else_locals,
                        &mut else_bindings,
                        declared_effects,
                        known_tasks,
                        record_types,
                        diagnostics,
                    );
                }
            }
            StatementKind::While { condition, body } => {
                check_expr_common(
                    program,
                    task,
                    condition,
                    locals,
                    declared_effects,
                    known_tasks,
                    record_types,
                    diagnostics,
                );
                if let Some(condition_type) =
                    infer_expr_type(program, task, condition, locals, known_tasks, record_types)
                    && !is_bool_type(&condition_type)
                {
                    diagnostics.push(
                        Diagnostic::error(
                            "WHILE_CONDITION_NOT_BOOL",
                            format!(
                                "while condition must be `Bool`, not `{}`",
                                condition_type.display()
                            ),
                        )
                        .with_node(condition.id.clone())
                        .with_repair_hint(bool_condition_hint(condition)),
                    );
                }
                let mut body_locals = locals.clone();
                let mut body_bindings = local_bindings.clone();
                check_block(
                    program,
                    task,
                    body,
                    &mut body_locals,
                    &mut body_bindings,
                    declared_effects,
                    known_tasks,
                    record_types,
                    diagnostics,
                );
            }
            StatementKind::For {
                item,
                collection,
                body,
            } => {
                check_expr_common(
                    program,
                    task,
                    collection,
                    locals,
                    declared_effects,
                    known_tasks,
                    record_types,
                    diagnostics,
                );
                let collection_type =
                    infer_expr_type(program, task, collection, locals, known_tasks, record_types);
                let item_type = collection_type.as_ref().and_then(list_element_type);
                if item_type.is_none() {
                    let collection_type = collection_type
                        .as_ref()
                        .map(TypeExpr::display)
                        .unwrap_or_else(|| "unknown".to_string());
                    diagnostics.push(
                        Diagnostic::error(
                            "FOR_COLLECTION_NOT_ITERABLE",
                            format!("for loop expects `List<T>`, not `{collection_type}`"),
                        )
                        .with_node(collection.id.clone()),
                    );
                }
                let mut body_locals = locals.clone();
                let mut body_bindings = local_bindings.clone();
                if body_locals.contains_key(item) {
                    diagnostics.push(
                        Diagnostic::error(
                            "LOCAL_ALREADY_BOUND",
                            format!("loop binding `{item}` is already in scope"),
                        )
                        .with_node(statement.id.clone()),
                    );
                } else {
                    body_locals.insert(
                        item.clone(),
                        item_type.unwrap_or_else(|| TypeExpr::named("Unit")),
                    );
                    body_bindings.insert(item.clone(), BindingKind::Bind);
                }
                check_block(
                    program,
                    task,
                    body,
                    &mut body_locals,
                    &mut body_bindings,
                    declared_effects,
                    known_tasks,
                    record_types,
                    diagnostics,
                );
            }
            StatementKind::Forge { body } => {
                let mut forge_locals = locals.clone();
                let mut forge_bindings = local_bindings.clone();
                check_block(
                    program,
                    task,
                    body,
                    &mut forge_locals,
                    &mut forge_bindings,
                    declared_effects,
                    known_tasks,
                    record_types,
                    diagnostics,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn check_expr_common(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    locals: &HashMap<String, TypeExpr>,
    declared_effects: &HashSet<String>,
    known_tasks: &TaskSignatures,
    record_types: &HashMap<String, Vec<RecordField>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    check_expression_effects(task, &expr.source, declared_effects, diagnostics);
    check_fallible_expression(task, expr, diagnostics);
    check_expr_structure(
        program,
        task,
        expr,
        locals,
        declared_effects,
        known_tasks,
        record_types,
        diagnostics,
    );
}

#[allow(clippy::too_many_arguments)]
fn check_expr_structure(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    locals: &HashMap<String, TypeExpr>,
    declared_effects: &HashSet<String>,
    known_tasks: &TaskSignatures,
    record_types: &HashMap<String, Vec<RecordField>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match &expr.kind {
        ExprKind::Unary { op, expr: inner } => {
            check_expr_structure(
                program,
                task,
                inner,
                locals,
                declared_effects,
                known_tasks,
                record_types,
                diagnostics,
            );
            check_unary_operator(
                program,
                task,
                expr,
                op,
                inner,
                locals,
                known_tasks,
                record_types,
                diagnostics,
            );
        }
        ExprKind::Binary { op, left, right } => {
            check_expr_structure(
                program,
                task,
                left,
                locals,
                declared_effects,
                known_tasks,
                record_types,
                diagnostics,
            );
            check_expr_structure(
                program,
                task,
                right,
                locals,
                declared_effects,
                known_tasks,
                record_types,
                diagnostics,
            );
            check_binary_operator(
                program,
                task,
                expr,
                op,
                left,
                right,
                locals,
                known_tasks,
                record_types,
                diagnostics,
            );
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            check_expr_structure(
                program,
                task,
                condition,
                locals,
                declared_effects,
                known_tasks,
                record_types,
                diagnostics,
            );
            check_expr_structure(
                program,
                task,
                then_branch,
                locals,
                declared_effects,
                known_tasks,
                record_types,
                diagnostics,
            );
            check_expr_structure(
                program,
                task,
                else_branch,
                locals,
                declared_effects,
                known_tasks,
                record_types,
                diagnostics,
            );
            check_if_expression(
                program,
                task,
                expr,
                condition,
                then_branch,
                else_branch,
                locals,
                known_tasks,
                record_types,
                diagnostics,
            );
        }
        ExprKind::Call { callee, args } => {
            let caller_module = task_module(task);
            let callee_path = callee_path(callee);
            let mut resolved_signature = None;
            if let Some(callee_name) = callee_path.as_deref() {
                if !is_result_constructor(callee_name)
                    && !is_builtin_task(callee_name)
                    && !is_host_callee_path(callee_name)
                {
                    if callee_name.contains('.')
                        && !is_module_qualified_callee(program, &caller_module, callee_name)
                    {
                        check_expr_structure(
                            program,
                            task,
                            callee,
                            locals,
                            declared_effects,
                            known_tasks,
                            record_types,
                            diagnostics,
                        );
                    } else {
                        match resolve_task(program, &caller_module, callee_name) {
                            TaskResolution::Resolved { index, fq_name } => {
                                resolved_signature = known_tasks
                                    .get(&index)
                                    .map(|signature| (fq_name, signature));
                            }
                            TaskResolution::Unknown => diagnostics.push(
                                Diagnostic::error(
                                    "UNKNOWN_TASK",
                                    format!("unknown task `{callee_name}`"),
                                )
                                .with_node(callee.id.clone())
                                .with_repair_hint(
                                    RepairHint::new("declare_or_import_task")
                                        .with_target(callee.id.clone())
                                        .with_replacement(format!(
                                            "import <module-with-{callee_name}>"
                                        )),
                                ),
                            ),
                            TaskResolution::Ambiguous(matches) => diagnostics.push(
                                Diagnostic::error(
                                    "AMBIGUOUS_TASK",
                                    format!(
                                        "task `{callee_name}` is ambiguous: {}",
                                        matches.join(", ")
                                    ),
                                )
                                .with_node(callee.id.clone())
                                .with_repair_hint(
                                    RepairHint::new("qualify_task_reference")
                                        .with_target(callee.id.clone())
                                        .with_replacement(matches.join(" | ")),
                                ),
                            ),
                            TaskResolution::Private(target) => diagnostics.push(
                                Diagnostic::error(
                                    "PRIVATE_TASK",
                                    format!("task `{target}` is not exported for `{}`", task.name),
                                )
                                .with_node(callee.id.clone())
                                .with_repair_hint(
                                    RepairHint::new("export_task")
                                        .with_target(format!("task:{target}"))
                                        .with_replacement(format!(
                                            "export task {}",
                                            short_name(&target)
                                        )),
                                ),
                            ),
                        }
                    }
                }
            } else {
                check_expr_structure(
                    program,
                    task,
                    callee,
                    locals,
                    declared_effects,
                    known_tasks,
                    record_types,
                    diagnostics,
                );
            }
            for arg in args {
                check_expr_structure(
                    program,
                    task,
                    arg,
                    locals,
                    declared_effects,
                    known_tasks,
                    record_types,
                    diagnostics,
                );
            }
            if let Some((callee_name, signature)) = resolved_signature {
                if args.len() != signature.takes.len() {
                    diagnostics.push(
                        Diagnostic::error(
                            "CALL_ARITY_MISMATCH",
                            format!(
                                "task `{}` calls `{callee_name}` with {} arguments but {} are required",
                                task.name,
                                args.len(),
                                signature.takes.len()
                            )
                        )
                        .with_node(expr.id.clone())
                        .with_repair_hint(
                            RepairHint::new("match_task_arity")
                                .with_target(expr.id.clone())
                                .with_replacement(format!(
                                    "{} arguments required",
                                    signature.takes.len()
                                )),
                        ),
                    );
                }
                for (index, (arg, expected)) in args.iter().zip(signature.takes.iter()).enumerate()
                {
                    if let Some(actual) =
                        infer_expr_type(program, task, arg, locals, known_tasks, record_types)
                        && !types_compatible(expected, &actual)
                    {
                        diagnostics.push(
                            Diagnostic::error(
                                "CALL_ARGUMENT_TYPE_MISMATCH",
                                format!(
                                    "argument {index} to `{callee_name}` expects `{}` but expression looks like `{}`",
                                    expected.display(),
                                    actual.display()
                                ),
                            )
                            .with_node(arg.id.clone())
                            .with_repair_hint(
                                RepairHint::new("replace_argument")
                                    .with_target(arg.id.clone())
                                    .with_replacement(expected.display()),
                            ),
                        );
                    }
                }
                for effect in &signature.effects {
                    if !declared_effects.contains(effect) {
                        diagnostics.push(
                            Diagnostic::error(
                                "EFFECT_UNAUTHORIZED",
                                format!(
                                    "task `{}` calls `{callee_name}` which requires `{effect}`",
                                    task.name
                                ),
                            )
                            .with_node(expr.id.clone())
                            .with_repair_hint(add_required_effect_hint(task, effect)),
                        );
                    }
                }
            }
            if let Some(callee_name) = callee_path.as_deref() {
                check_builtin_call(
                    program,
                    task,
                    callee_name,
                    args,
                    locals,
                    known_tasks,
                    record_types,
                    diagnostics,
                );
            }
        }
        ExprKind::ListLiteral { items } => {
            for item in items {
                check_expr_structure(
                    program,
                    task,
                    item,
                    locals,
                    declared_effects,
                    known_tasks,
                    record_types,
                    diagnostics,
                );
            }
            check_list_literal(
                program,
                task,
                expr,
                items,
                locals,
                known_tasks,
                record_types,
                diagnostics,
            );
        }
        ExprKind::MapLiteral { entries } => {
            for entry in entries {
                check_expr_structure(
                    program,
                    task,
                    &entry.key,
                    locals,
                    declared_effects,
                    known_tasks,
                    record_types,
                    diagnostics,
                );
                check_expr_structure(
                    program,
                    task,
                    &entry.value,
                    locals,
                    declared_effects,
                    known_tasks,
                    record_types,
                    diagnostics,
                );
            }
            check_map_literal(
                program,
                task,
                expr,
                entries,
                locals,
                known_tasks,
                record_types,
                diagnostics,
            );
        }
        ExprKind::Index { collection, index } => {
            check_expr_structure(
                program,
                task,
                collection,
                locals,
                declared_effects,
                known_tasks,
                record_types,
                diagnostics,
            );
            check_expr_structure(
                program,
                task,
                index,
                locals,
                declared_effects,
                known_tasks,
                record_types,
                diagnostics,
            );
            check_index_expression(
                program,
                task,
                expr,
                collection,
                index,
                locals,
                known_tasks,
                record_types,
                diagnostics,
            );
        }
        ExprKind::FieldAccess { receiver, field } => {
            check_expr_structure(
                program,
                task,
                receiver,
                locals,
                declared_effects,
                known_tasks,
                record_types,
                diagnostics,
            );
            if let Some(TypeExpr::Named { name }) =
                infer_expr_type(program, task, receiver, locals, known_tasks, record_types)
                && let Some(fields) = record_types.get(&name)
                && !fields
                    .iter()
                    .any(|record_field| record_field.name == *field)
            {
                diagnostics.push(
                    Diagnostic::error(
                        "UNKNOWN_RECORD_FIELD",
                        format!("type `{name}` has no field `{field}`"),
                    )
                    .with_node(expr.id.clone()),
                );
            }
        }
        ExprKind::RecordLiteral { type_name, fields } => {
            let mut seen = HashSet::new();
            for field in fields {
                if !seen.insert(field.name.clone()) {
                    diagnostics.push(
                        Diagnostic::error(
                            "DUPLICATE_RECORD_LITERAL_FIELD",
                            format!(
                                "record literal field `{}` appears more than once",
                                field.name
                            ),
                        )
                        .with_node(field.expr.id.clone()),
                    );
                }
                check_expr_structure(
                    program,
                    task,
                    &field.expr,
                    locals,
                    declared_effects,
                    known_tasks,
                    record_types,
                    diagnostics,
                );
            }

            if let Some(type_name) = type_name {
                match resolve_type(program, &task_module(task), type_name) {
                    TypeResolution::Builtin(name) => diagnostics.push(
                        Diagnostic::error(
                            "RECORD_LITERAL_NON_RECORD_TYPE",
                            format!("type `{name}` is not a record type"),
                        )
                        .with_node(expr.id.clone()),
                    ),
                    TypeResolution::Resolved { fq_name, .. } => {
                        if let Some(expected_fields) = record_types.get(&fq_name) {
                            check_record_literal_fields(
                                program,
                                task,
                                expr,
                                fields,
                                expected_fields,
                                locals,
                                known_tasks,
                                record_types,
                                diagnostics,
                            );
                        } else {
                            diagnostics.push(
                                Diagnostic::error(
                                    "RECORD_LITERAL_NON_RECORD_TYPE",
                                    format!("type `{fq_name}` is not a record type"),
                                )
                                .with_node(expr.id.clone()),
                            );
                        }
                    }
                    TypeResolution::Unknown => diagnostics.push(
                        Diagnostic::error(
                            "UNKNOWN_TYPE",
                            format!("unknown record literal type `{type_name}`"),
                        )
                        .with_node(expr.id.clone()),
                    ),
                    TypeResolution::Ambiguous(matches) => diagnostics.push(
                        Diagnostic::error(
                            "AMBIGUOUS_TYPE",
                            format!("type `{type_name}` is ambiguous: {}", matches.join(", ")),
                        )
                        .with_node(expr.id.clone()),
                    ),
                    TypeResolution::Private(target) => diagnostics.push(
                        Diagnostic::error(
                            "PRIVATE_TYPE",
                            format!("type `{target}` is not exported for `{}`", task.name),
                        )
                        .with_node(expr.id.clone()),
                    ),
                }
            }
        }
        ExprKind::Try { expr: inner } => {
            check_expr_structure(
                program,
                task,
                inner,
                locals,
                declared_effects,
                known_tasks,
                record_types,
                diagnostics,
            );
        }
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. } => {}
        ExprKind::Identifier { name } => {
            if !locals.contains_key(name) && !is_host_root(name) {
                diagnostics.push(
                    Diagnostic::error(
                        "UNKNOWN_IDENTIFIER",
                        format!("unknown local binding `{name}`"),
                    )
                    .with_node(expr.id.clone())
                    .with_repair_hint(declare_binding_hint(
                        task,
                        name,
                        BindingKind::Bind,
                    )),
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn check_unary_operator(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    op: &UnaryOp,
    inner: &Expr,
    locals: &HashMap<String, TypeExpr>,
    known_tasks: &TaskSignatures,
    record_types: &HashMap<String, Vec<RecordField>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(actual) = infer_expr_type(program, task, inner, locals, known_tasks, record_types)
    else {
        return;
    };

    let valid = match op {
        UnaryOp::Not => is_bool_type(&actual),
        UnaryOp::Negate => is_numeric_type(&actual),
    };

    if !valid {
        diagnostics.push(
            Diagnostic::error(
                "UNARY_OPERATOR_TYPE_MISMATCH",
                format!(
                    "operator `{}` cannot be applied to `{}`",
                    op.as_str(),
                    actual.display()
                ),
            )
            .with_node(expr.id.clone())
            .with_repair_hint(
                RepairHint::new("replace_operand")
                    .with_target(inner.id.clone())
                    .with_replacement(unary_expected_type(op)),
            ),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn check_binary_operator(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    op: &BinaryOp,
    left: &Expr,
    right: &Expr,
    locals: &HashMap<String, TypeExpr>,
    known_tasks: &TaskSignatures,
    record_types: &HashMap<String, Vec<RecordField>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let left_type = infer_expr_type(program, task, left, locals, known_tasks, record_types);
    let right_type = infer_expr_type(program, task, right, locals, known_tasks, record_types);
    let (Some(left_type), Some(right_type)) = (left_type, right_type) else {
        return;
    };

    if !binary_operands_compatible(op, &left_type, &right_type) {
        diagnostics.push(
            Diagnostic::error(
                "BINARY_OPERATOR_TYPE_MISMATCH",
                format!(
                    "operator `{}` cannot be applied to `{}` and `{}`",
                    op.as_str(),
                    left_type.display(),
                    right_type.display()
                ),
            )
            .with_node(expr.id.clone())
            .with_repair_hint(
                RepairHint::new("replace_binary_operands")
                    .with_target(expr.id.clone())
                    .with_replacement(binary_expected_types(op)),
            ),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn check_if_expression(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    condition: &Expr,
    then_branch: &Expr,
    else_branch: &Expr,
    locals: &HashMap<String, TypeExpr>,
    known_tasks: &TaskSignatures,
    record_types: &HashMap<String, Vec<RecordField>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(condition_type) =
        infer_expr_type(program, task, condition, locals, known_tasks, record_types)
        && !is_bool_type(&condition_type)
    {
        diagnostics.push(
            Diagnostic::error(
                "IF_CONDITION_NOT_BOOL",
                format!(
                    "if condition must be `Bool`, not `{}`",
                    condition_type.display()
                ),
            )
            .with_node(condition.id.clone())
            .with_repair_hint(bool_condition_hint(condition)),
        );
    }

    let then_type = infer_expr_type(
        program,
        task,
        then_branch,
        locals,
        known_tasks,
        record_types,
    );
    let else_type = infer_expr_type(
        program,
        task,
        else_branch,
        locals,
        known_tasks,
        record_types,
    );
    if let (Some(then_type), Some(else_type)) = (then_type, else_type)
        && !types_compatible(&then_type, &else_type)
    {
        diagnostics.push(
            Diagnostic::error(
                "IF_BRANCH_TYPE_MISMATCH",
                format!(
                    "if branches produce `{}` and `{}`",
                    then_type.display(),
                    else_type.display()
                ),
            )
            .with_node(expr.id.clone()),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn check_builtin_call(
    program: &Program,
    task: &TaskDecl,
    name: &str,
    args: &[Expr],
    locals: &HashMap<String, TypeExpr>,
    known_tasks: &TaskSignatures,
    record_types: &HashMap<String, Vec<RecordField>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if name != "len" {
        return;
    }

    if args.len() != 1 {
        diagnostics.push(
            Diagnostic::error(
                "BUILTIN_ARITY_MISMATCH",
                format!(
                    "builtin `len` expects 1 argument but received {}",
                    args.len()
                ),
            )
            .with_node(task.id.clone()),
        );
        return;
    }

    if let Some(actual) =
        infer_expr_type(program, task, &args[0], locals, known_tasks, record_types)
        && !is_list_type(&actual)
        && !is_map_type(&actual)
        && !is_named_type(&actual, "Text")
    {
        diagnostics.push(
            Diagnostic::error(
                "BUILTIN_ARGUMENT_TYPE_MISMATCH",
                format!(
                    "builtin `len` expects `List<T>`, `Map<Text, T>`, or `Text`, not `{}`",
                    actual.display()
                ),
            )
            .with_node(args[0].id.clone())
            .with_repair_hint(
                RepairHint::new("replace_builtin_argument")
                    .with_target(args[0].id.clone())
                    .with_replacement("List<T> | Map<Text, T> | Text"),
            ),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn check_list_literal(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    items: &[Expr],
    locals: &HashMap<String, TypeExpr>,
    known_tasks: &TaskSignatures,
    record_types: &HashMap<String, Vec<RecordField>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(first) = items.first() else {
        return;
    };
    let Some(first_type) = infer_expr_type(program, task, first, locals, known_tasks, record_types)
    else {
        return;
    };
    for item in items.iter().skip(1) {
        if let Some(actual) =
            infer_expr_type(program, task, item, locals, known_tasks, record_types)
            && !types_compatible(&first_type, &actual)
        {
            diagnostics.push(
                Diagnostic::error(
                    "LIST_ELEMENT_TYPE_MISMATCH",
                    format!(
                        "list literal mixes `{}` and `{}`",
                        first_type.display(),
                        actual.display()
                    ),
                )
                .with_node(expr.id.clone()),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn check_map_literal(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    entries: &[ExprMapEntry],
    locals: &HashMap<String, TypeExpr>,
    known_tasks: &TaskSignatures,
    record_types: &HashMap<String, Vec<RecordField>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut seen_literal_keys = HashSet::new();
    for entry in entries {
        if let Some(key_type) =
            infer_expr_type(program, task, &entry.key, locals, known_tasks, record_types)
            && !is_named_type(&key_type, "Text")
        {
            diagnostics.push(
                Diagnostic::error(
                    "MAP_KEY_TYPE_MISMATCH",
                    format!("map keys must be `Text`, not `{}`", key_type.display()),
                )
                .with_node(entry.key.id.clone()),
            );
        }
        if let ExprKind::StringLiteral { value } = &entry.key.kind
            && !seen_literal_keys.insert(value.clone())
        {
            diagnostics.push(
                Diagnostic::error(
                    "DUPLICATE_MAP_KEY",
                    format!("map key `{value}` is repeated"),
                )
                .with_node(entry.key.id.clone()),
            );
        }
    }

    let Some(first) = entries.first() else {
        return;
    };
    let Some(first_type) = infer_expr_type(
        program,
        task,
        &first.value,
        locals,
        known_tasks,
        record_types,
    ) else {
        return;
    };
    for entry in entries.iter().skip(1) {
        if let Some(actual) = infer_expr_type(
            program,
            task,
            &entry.value,
            locals,
            known_tasks,
            record_types,
        ) && !types_compatible(&first_type, &actual)
        {
            diagnostics.push(
                Diagnostic::error(
                    "MAP_VALUE_TYPE_MISMATCH",
                    format!(
                        "map literal mixes `{}` and `{}` values",
                        first_type.display(),
                        actual.display()
                    ),
                )
                .with_node(expr.id.clone()),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn check_index_expression(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    collection: &Expr,
    index: &Expr,
    locals: &HashMap<String, TypeExpr>,
    known_tasks: &TaskSignatures,
    record_types: &HashMap<String, Vec<RecordField>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(collection_type) =
        infer_expr_type(program, task, collection, locals, known_tasks, record_types)
    {
        if list_element_type(&collection_type).is_some() {
            if let Some(index_type) =
                infer_expr_type(program, task, index, locals, known_tasks, record_types)
                && !is_named_type(&index_type, "Int")
            {
                diagnostics.push(
                    Diagnostic::error(
                        "INDEX_NOT_INT",
                        format!("list index must be `Int`, not `{}`", index_type.display()),
                    )
                    .with_node(index.id.clone()),
                );
            }
            return;
        }
        if text_key_map_value_type(&collection_type).is_some() {
            if let Some(index_type) =
                infer_expr_type(program, task, index, locals, known_tasks, record_types)
                && !is_named_type(&index_type, "Text")
            {
                diagnostics.push(
                    Diagnostic::error(
                        "INDEX_KEY_TYPE_MISMATCH",
                        format!("map key must be `Text`, not `{}`", index_type.display()),
                    )
                    .with_node(index.id.clone()),
                );
            }
            return;
        }

        diagnostics.push(
            Diagnostic::error(
                "INDEX_COLLECTION_NOT_INDEXABLE",
                format!(
                    "indexing expects `List<T>` or `Map<Text, T>`, not `{}`",
                    collection_type.display()
                ),
            )
            .with_node(expr.id.clone()),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn check_record_literal_fields(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    fields: &[crate::ast::ExprField],
    expected_fields: &[RecordField],
    locals: &HashMap<String, TypeExpr>,
    known_tasks: &TaskSignatures,
    record_types: &HashMap<String, Vec<RecordField>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for expected in expected_fields {
        match fields.iter().find(|field| field.name == expected.name) {
            Some(field) => {
                if let Some(actual) = infer_expr_type(
                    program,
                    task,
                    &field.expr,
                    locals,
                    known_tasks,
                    record_types,
                ) && !types_compatible(&expected.ty, &actual)
                {
                    diagnostics.push(
                        Diagnostic::error(
                            "RECORD_FIELD_TYPE_MISMATCH",
                            format!(
                                "field `{}` expects `{}` but expression looks like `{}`",
                                expected.name,
                                expected.ty.display(),
                                actual.display()
                            ),
                        )
                        .with_node(field.expr.id.clone()),
                    );
                }
            }
            None => diagnostics.push(
                Diagnostic::error(
                    "RECORD_FIELD_MISSING",
                    format!("record literal is missing field `{}`", expected.name),
                )
                .with_node(expr.id.clone()),
            ),
        }
    }

    for field in fields {
        if !expected_fields
            .iter()
            .any(|expected| expected.name == field.name)
        {
            diagnostics.push(
                Diagnostic::error(
                    "RECORD_FIELD_UNKNOWN",
                    format!("record literal has unknown field `{}`", field.name),
                )
                .with_node(field.expr.id.clone()),
            );
        }
    }
}

fn validate_type_expr(
    ty: &TypeExpr,
    program: &Program,
    module: &str,
    diagnostics: &mut Vec<Diagnostic>,
    node: &str,
) {
    match ty {
        TypeExpr::Named { name } => {
            validate_type_name(name, program, module, diagnostics, node, "type");
            if name == "Gate" {
                diagnostics.push(
                    Diagnostic::error(
                        "GATE_TYPE_ARGUMENT_REQUIRED",
                        "`Gate` requires exactly one effect argument, such as `Gate<FileRead>`",
                    )
                    .with_node(node.to_string()),
                );
            }
        }
        TypeExpr::Generic { name, args } => {
            if name == "Gate" {
                validate_gate_type_expr(ty, program, module, diagnostics, node);
                return;
            }
            validate_type_name(name, program, module, diagnostics, node, "generic type");
            for arg in args {
                validate_type_expr(arg, program, module, diagnostics, node);
            }
        }
        TypeExpr::Record { fields } => {
            let mut names = HashSet::new();
            for field in fields {
                if !names.insert(field.name.clone()) {
                    diagnostics.push(
                        Diagnostic::error(
                            "DUPLICATE_FIELD",
                            format!("record field `{}` is declared more than once", field.name),
                        )
                        .with_node(node.to_string()),
                    );
                }
                validate_type_expr(&field.ty, program, module, diagnostics, node);
            }
        }
    }
}

fn validate_gate_take(
    take: &crate::ast::TakeDecl,
    program: &Program,
    module: &str,
    declared_effects: &HashSet<String>,
    task: &TaskDecl,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(effect) = validate_gate_type_expr(&take.ty, program, module, diagnostics, &take.id)
    else {
        diagnostics.push(
            Diagnostic::error(
                "GATE_TAKE_TYPE_MISMATCH",
                format!(
                    "gate take `{}` must use `Gate<Effect>`, not `{}`",
                    take.name,
                    take.ty.display()
                ),
            )
            .with_node(take.id.clone()),
        );
        return;
    };

    if !declared_effects.contains(&effect) {
        diagnostics.push(
            Diagnostic::error(
                "GATE_EFFECT_UNDECLARED",
                format!(
                    "gate take `{}` provides `{effect}` but task `{}` does not declare that effect",
                    take.name, task.name
                ),
            )
            .with_node(take.id.clone())
            .with_repair_hint(add_required_effect_hint(task, &effect)),
        );
    }
}

fn validate_gate_type_expr(
    ty: &TypeExpr,
    program: &Program,
    module: &str,
    diagnostics: &mut Vec<Diagnostic>,
    node: &str,
) -> Option<String> {
    let TypeExpr::Generic { name, args } = ty else {
        return None;
    };
    if name != "Gate" {
        return None;
    }
    validate_type_name(name, program, module, diagnostics, node, "generic type");
    if args.len() != 1 {
        diagnostics.push(
            Diagnostic::error(
                "GATE_TYPE_ARITY_MISMATCH",
                format!(
                    "`Gate` expects 1 effect argument but received {}",
                    args.len()
                ),
            )
            .with_node(node.to_string()),
        );
        return None;
    }
    let TypeExpr::Named { name: effect } = &args[0] else {
        diagnostics.push(
            Diagnostic::error(
                "GATE_EFFECT_TYPE_MISMATCH",
                "`Gate` expects an effect name argument",
            )
            .with_node(node.to_string()),
        );
        return None;
    };
    validate_effect_reference(effect, program, module, diagnostics, node);
    Some(normalize_effect_name_silent(program, module, effect))
}

fn validate_effect_reference(
    name: &str,
    program: &Program,
    module: &str,
    diagnostics: &mut Vec<Diagnostic>,
    node: &str,
) {
    match resolve_effect(program, module, name) {
        EffectResolution::Builtin(_) | EffectResolution::Resolved { .. } => {}
        EffectResolution::Unknown => diagnostics.push(
            Diagnostic::error("UNKNOWN_EFFECT", format!("unknown effect `{name}`"))
                .with_node(node.to_string())
                .with_repair_hint(
                    RepairHint::new("declare_or_import_effect")
                        .with_target(node.to_string())
                        .with_effect(name.to_string())
                        .with_replacement(format!("effect {name}")),
                ),
        ),
        EffectResolution::Ambiguous(matches) => diagnostics.push(
            Diagnostic::error(
                "AMBIGUOUS_EFFECT",
                format!("effect `{name}` is ambiguous: {}", matches.join(", ")),
            )
            .with_node(node.to_string())
            .with_repair_hint(
                RepairHint::new("qualify_effect_reference")
                    .with_target(node.to_string())
                    .with_replacement(matches.join(" | ")),
            ),
        ),
        EffectResolution::Private(target) => diagnostics.push(
            Diagnostic::error(
                "PRIVATE_EFFECT",
                format!("effect `{target}` is not exported for module `{module}`"),
            )
            .with_node(node.to_string())
            .with_repair_hint(
                RepairHint::new("export_effect")
                    .with_target(format!("effect:{target}"))
                    .with_effect(target.clone())
                    .with_replacement(format!("export effect {}", short_name(&target))),
            ),
        ),
    }
}

fn declare_binding_hint(task: &TaskDecl, name: &str, binding_kind: BindingKind) -> RepairHint {
    let kind = if binding_kind.is_mutable_local() {
        "declare_mutable_binding"
    } else {
        "declare_binding"
    };
    RepairHint::new(kind)
        .with_target(task.id.clone())
        .with_replacement(format!(
            "{} {name} = <value>",
            binding_kind.as_source_keyword()
        ))
}

fn add_required_effect_hint(task: &TaskDecl, effect: &str) -> RepairHint {
    RepairHint::new("add_required_effect")
        .with_target(task.id.clone())
        .with_effect(effect.to_string())
}

fn bool_condition_hint(condition: &Expr) -> RepairHint {
    RepairHint::new("replace_condition")
        .with_target(condition.id.clone())
        .with_replacement("Bool")
}

fn unary_expected_type(op: &UnaryOp) -> &'static str {
    match op {
        UnaryOp::Not => "Bool",
        UnaryOp::Negate => "Int | Float",
    }
}

fn binary_expected_types(op: &BinaryOp) -> &'static str {
    match op {
        BinaryOp::Or | BinaryOp::And => "Bool and Bool",
        BinaryOp::Add => "Int and Int | Float and Float | Text and Text",
        BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Remainder => {
            "Int and Int | Float and Float"
        }
        BinaryOp::Equal
        | BinaryOp::NotEqual
        | BinaryOp::Less
        | BinaryOp::LessEqual
        | BinaryOp::Greater
        | BinaryOp::GreaterEqual => "matching operand types",
    }
}

fn short_name(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or(name)
}

fn validate_type_name(
    name: &str,
    program: &Program,
    module: &str,
    diagnostics: &mut Vec<Diagnostic>,
    node: &str,
    noun: &str,
) {
    match resolve_type(program, module, name) {
        TypeResolution::Builtin(_) | TypeResolution::Resolved { .. } => {}
        TypeResolution::Unknown => diagnostics.push(
            Diagnostic::error("UNKNOWN_TYPE", format!("unknown {noun} `{name}`"))
                .with_node(node.to_string())
                .with_repair_hint(
                    RepairHint::new("declare_or_import_type")
                        .with_target(node.to_string())
                        .with_replacement(format!("type {name} = <definition>")),
                ),
        ),
        TypeResolution::Ambiguous(matches) => diagnostics.push(
            Diagnostic::error(
                "AMBIGUOUS_TYPE",
                format!("type `{name}` is ambiguous: {}", matches.join(", ")),
            )
            .with_node(node.to_string())
            .with_repair_hint(
                RepairHint::new("qualify_type_reference")
                    .with_target(node.to_string())
                    .with_replacement(matches.join(" | ")),
            ),
        ),
        TypeResolution::Private(target) => diagnostics.push(
            Diagnostic::error(
                "PRIVATE_TYPE",
                format!("type `{target}` is not exported for module `{module}`"),
            )
            .with_node(node.to_string())
            .with_repair_hint(
                RepairHint::new("export_type")
                    .with_target(format!("type:{target}"))
                    .with_replacement(format!("export type {}", short_name(&target))),
            ),
        ),
    }
}

fn validate_effect_name(
    name: &str,
    program: &Program,
    module: &str,
    task: &TaskDecl,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match resolve_effect(program, module, name) {
        EffectResolution::Builtin(_) | EffectResolution::Resolved { .. } => {}
        EffectResolution::Unknown => diagnostics.push(
            Diagnostic::error(
                "UNKNOWN_EFFECT",
                format!("task `{}` uses unknown effect `{name}`", task.name),
            )
            .with_node(task.id.clone())
            .with_repair_hint(
                RepairHint::new("declare_or_import_effect")
                    .with_target(task.id.clone())
                    .with_effect(name.to_string())
                    .with_replacement(format!("effect {name}")),
            ),
        ),
        EffectResolution::Ambiguous(matches) => diagnostics.push(
            Diagnostic::error(
                "AMBIGUOUS_EFFECT",
                format!("effect `{name}` is ambiguous: {}", matches.join(", ")),
            )
            .with_node(task.id.clone())
            .with_repair_hint(
                RepairHint::new("qualify_effect_reference")
                    .with_target(task.id.clone())
                    .with_replacement(matches.join(" | ")),
            ),
        ),
        EffectResolution::Private(target) => diagnostics.push(
            Diagnostic::error(
                "PRIVATE_EFFECT",
                format!("effect `{target}` is not exported for `{}`", task.name),
            )
            .with_node(task.id.clone())
            .with_repair_hint(
                RepairHint::new("export_effect")
                    .with_target(format!("effect:{target}"))
                    .with_effect(target.clone())
                    .with_replacement(format!("export effect {}", short_name(&target))),
            ),
        ),
    }
}

fn normalize_type_expr_silent(program: &Program, module: &str, ty: &TypeExpr) -> TypeExpr {
    match ty {
        TypeExpr::Named { name } => {
            TypeExpr::named(normalize_type_name_silent(program, module, name))
        }
        TypeExpr::Generic { name, args } if name == "Gate" => TypeExpr::Generic {
            name: "Gate".to_string(),
            args: args
                .iter()
                .map(|arg| normalize_gate_type_arg_silent(program, module, arg))
                .collect(),
        },
        TypeExpr::Generic { name, args } => TypeExpr::Generic {
            name: normalize_type_name_silent(program, module, name),
            args: args
                .iter()
                .map(|arg| normalize_type_expr_silent(program, module, arg))
                .collect(),
        },
        TypeExpr::Record { fields } => TypeExpr::Record {
            fields: fields
                .iter()
                .map(|field| RecordField {
                    name: field.name.clone(),
                    ty: normalize_type_expr_silent(program, module, &field.ty),
                })
                .collect(),
        },
    }
}

fn normalize_gate_type_arg_silent(program: &Program, module: &str, ty: &TypeExpr) -> TypeExpr {
    match ty {
        TypeExpr::Named { name } => {
            TypeExpr::named(normalize_effect_name_silent(program, module, name))
        }
        _ => normalize_type_expr_silent(program, module, ty),
    }
}

fn normalize_type_name_silent(program: &Program, module: &str, name: &str) -> String {
    match resolve_type(program, module, name) {
        TypeResolution::Builtin(name) | TypeResolution::Resolved { fq_name: name, .. } => name,
        TypeResolution::Unknown | TypeResolution::Ambiguous(_) | TypeResolution::Private(_) => {
            name.to_string()
        }
    }
}

fn normalize_effect_name_silent(program: &Program, module: &str, name: &str) -> String {
    match resolve_effect(program, module, name) {
        EffectResolution::Builtin(name) | EffectResolution::Resolved { fq_name: name, .. } => name,
        EffectResolution::Unknown
        | EffectResolution::Ambiguous(_)
        | EffectResolution::Private(_) => name.to_string(),
    }
}

fn check_expression_effects(
    task: &TaskDecl,
    source: &str,
    declared_effects: &HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (needle, acceptable_effects) in host_effect_patterns() {
        if source.contains(needle)
            && !acceptable_effects
                .iter()
                .any(|effect| declared_effects.contains(*effect))
        {
            let required = acceptable_effects[0];
            diagnostics.push(
                Diagnostic::error(
                    "EFFECT_UNAUTHORIZED",
                    format!(
                        "task `{}` expression `{source}` requires `{required}`",
                        task.name
                    ),
                )
                .with_node(task.id.clone())
                .with_repair_hint(add_required_effect_hint(task, required)),
            );
        }
    }
}

fn host_effect_patterns() -> Vec<(&'static str, Vec<&'static str>)> {
    vec![
        ("fs.read_text", vec!["FileRead"]),
        ("fs.write_text", vec!["FileWrite"]),
        ("model.", vec!["ModelCall"]),
        ("db.", vec!["DatabaseRead", "DbRead"]),
        ("http.", vec!["Network"]),
        ("shell.", vec!["Shell"]),
        ("secrets.", vec!["SecretRead"]),
        ("deploy.", vec!["Deploy"]),
    ]
}

fn check_fallible_expression(
    task: &TaskDecl,
    expr: &crate::ast::Expr,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if expr_uses_try(expr) && task.return_type.generic_name() != Some("Result") {
        diagnostics.push(
            Diagnostic::error(
                "QUESTION_REQUIRES_RESULT",
                "`?` may only be used in a task returning Result<T, E>",
            )
            .with_node(task.id.clone()),
        );
    }
}

fn expr_uses_try(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Try { .. } => true,
        ExprKind::Unary { expr, .. } => expr_uses_try(expr),
        ExprKind::Binary { left, right, .. } => expr_uses_try(left) || expr_uses_try(right),
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => expr_uses_try(condition) || expr_uses_try(then_branch) || expr_uses_try(else_branch),
        ExprKind::Call { callee, args } => expr_uses_try(callee) || args.iter().any(expr_uses_try),
        ExprKind::ListLiteral { items } => items.iter().any(expr_uses_try),
        ExprKind::MapLiteral { entries } => entries
            .iter()
            .any(|entry| expr_uses_try(&entry.key) || expr_uses_try(&entry.value)),
        ExprKind::Index { collection, index } => expr_uses_try(collection) || expr_uses_try(index),
        ExprKind::FieldAccess { receiver, .. } => expr_uses_try(receiver),
        ExprKind::RecordLiteral { fields, .. } => {
            fields.iter().any(|field| expr_uses_try(&field.expr))
        }
        ExprKind::Raw { fallible } => *fallible,
        ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => false,
    }
}

fn infer_expr_type(
    program: &Program,
    task: &TaskDecl,
    expr: &crate::ast::Expr,
    locals: &HashMap<String, TypeExpr>,
    known_tasks: &TaskSignatures,
    record_types: &HashMap<String, Vec<RecordField>>,
) -> Option<TypeExpr> {
    let return_type = normalize_type_expr_silent(program, &task_module(task), &task.return_type);
    match &expr.kind {
        ExprKind::StringLiteral { .. } => Some(TypeExpr::named("Text")),
        ExprKind::IntLiteral { .. } => Some(TypeExpr::named("Int")),
        ExprKind::FloatLiteral { .. } => Some(TypeExpr::named("Float")),
        ExprKind::BoolLiteral { .. } => Some(TypeExpr::named("Bool")),
        ExprKind::Identifier { name } => locals.get(name).cloned(),
        ExprKind::Unary { op, expr: inner } => {
            let inner_type =
                infer_expr_type(program, task, inner, locals, known_tasks, record_types)?;
            match op {
                UnaryOp::Not if is_bool_type(&inner_type) => Some(TypeExpr::named("Bool")),
                UnaryOp::Negate if is_numeric_type(&inner_type) => Some(inner_type),
                _ => None,
            }
        }
        ExprKind::Binary { op, left, right } => {
            let left_type =
                infer_expr_type(program, task, left, locals, known_tasks, record_types)?;
            let right_type =
                infer_expr_type(program, task, right, locals, known_tasks, record_types)?;
            infer_binary_type(op, &left_type, &right_type)
        }
        ExprKind::If {
            condition: _,
            then_branch,
            else_branch,
        } => {
            let then_type = infer_expr_type(
                program,
                task,
                then_branch,
                locals,
                known_tasks,
                record_types,
            )?;
            let else_type = infer_expr_type(
                program,
                task,
                else_branch,
                locals,
                known_tasks,
                record_types,
            )?;
            types_compatible(&then_type, &else_type).then_some(then_type)
        }
        ExprKind::Call { callee, .. } => {
            if callee_path(callee).is_some_and(|name| name == "Ok" || name == "Err")
                && return_type.generic_name() == Some("Result")
            {
                Some(return_type)
            } else if let Some(ty) =
                infer_value_method_type(program, task, callee, locals, known_tasks, record_types)
            {
                Some(ty)
            } else if let Some(callee_name) = callee_path(callee) {
                if callee_name == "len" {
                    Some(TypeExpr::named("Int"))
                } else if let Some(ty) = host_call_return_type(&callee_name) {
                    Some(ty)
                } else if is_host_callee_path(&callee_name)
                    || (callee_name.contains('.')
                        && !is_module_qualified_callee(program, &task_module(task), &callee_name))
                {
                    None
                } else if let TaskResolution::Resolved { index, .. } =
                    resolve_task(program, &task_module(task), &callee_name)
                {
                    known_tasks
                        .get(&index)
                        .map(|signature| signature.return_type.clone())
                } else {
                    None
                }
            } else {
                None
            }
        }
        ExprKind::ListLiteral { items } => {
            let Some(first) = items.first() else {
                return Some(TypeExpr::Generic {
                    name: "List".to_string(),
                    args: vec![TypeExpr::named("Unit")],
                });
            };
            let first_type =
                infer_expr_type(program, task, first, locals, known_tasks, record_types)?;
            for item in items.iter().skip(1) {
                let item_type =
                    infer_expr_type(program, task, item, locals, known_tasks, record_types)?;
                if !types_compatible(&first_type, &item_type) {
                    return None;
                }
            }
            Some(TypeExpr::Generic {
                name: "List".to_string(),
                args: vec![first_type],
            })
        }
        ExprKind::MapLiteral { entries } => {
            let Some(first) = entries.first() else {
                return Some(TypeExpr::Generic {
                    name: "Map".to_string(),
                    args: vec![TypeExpr::named("Text"), TypeExpr::named("Unit")],
                });
            };
            for entry in entries {
                let key_type =
                    infer_expr_type(program, task, &entry.key, locals, known_tasks, record_types)?;
                if !is_named_type(&key_type, "Text") {
                    return None;
                }
            }
            let first_value_type = infer_expr_type(
                program,
                task,
                &first.value,
                locals,
                known_tasks,
                record_types,
            )?;
            for entry in entries.iter().skip(1) {
                let value_type = infer_expr_type(
                    program,
                    task,
                    &entry.value,
                    locals,
                    known_tasks,
                    record_types,
                )?;
                if !types_compatible(&first_value_type, &value_type) {
                    return None;
                }
            }
            Some(TypeExpr::Generic {
                name: "Map".to_string(),
                args: vec![TypeExpr::named("Text"), first_value_type],
            })
        }
        ExprKind::Index { collection, index } => {
            let collection_type =
                infer_expr_type(program, task, collection, locals, known_tasks, record_types)?;
            let index_type =
                infer_expr_type(program, task, index, locals, known_tasks, record_types)?;
            if is_named_type(&index_type, "Int") {
                return list_element_type(&collection_type);
            }
            if is_named_type(&index_type, "Text") {
                return text_key_map_value_type(&collection_type);
            }
            None
        }
        ExprKind::FieldAccess { receiver, field } => {
            if let Some(TypeExpr::Named { name }) =
                infer_expr_type(program, task, receiver, locals, known_tasks, record_types)
            {
                record_types
                    .get(&name)
                    .and_then(|fields| fields.iter().find(|item| item.name == *field))
                    .map(|field| field.ty.clone())
            } else {
                None
            }
        }
        ExprKind::RecordLiteral { type_name, fields } => {
            if let Some(type_name) = type_name {
                Some(TypeExpr::named(normalize_type_name_silent(
                    program,
                    &task_module(task),
                    type_name,
                )))
            } else {
                let mut inferred_fields = Vec::new();
                for field in fields {
                    inferred_fields.push(RecordField {
                        name: field.name.clone(),
                        ty: infer_expr_type(
                            program,
                            task,
                            &field.expr,
                            locals,
                            known_tasks,
                            record_types,
                        )?,
                    });
                }
                Some(TypeExpr::Record {
                    fields: inferred_fields,
                })
            }
        }
        ExprKind::Try { expr } => {
            match infer_expr_type(program, task, expr, locals, known_tasks, record_types) {
                Some(TypeExpr::Generic { name, mut args }) if name == "Result" => {
                    if args.is_empty() {
                        None
                    } else {
                        Some(args.remove(0))
                    }
                }
                _ => None,
            }
        }
        ExprKind::Raw { .. } => {
            let trimmed = expr.source.trim();
            if trimmed.starts_with("Ok(") || trimmed.starts_with("Err(") {
                Some(return_type)
            } else {
                None
            }
        }
    }
}

fn infer_value_method_type(
    program: &Program,
    task: &TaskDecl,
    callee: &Expr,
    locals: &HashMap<String, TypeExpr>,
    known_tasks: &TaskSignatures,
    record_types: &HashMap<String, Vec<RecordField>>,
) -> Option<TypeExpr> {
    let ExprKind::FieldAccess { receiver, field } = &callee.kind else {
        return None;
    };
    if !matches!(field.as_str(), "get" | "text" | "int" | "float" | "bool") {
        return None;
    }
    let receiver_type =
        infer_expr_type(program, task, receiver, locals, known_tasks, record_types)?;
    if !is_named_type(&receiver_type, "DbRow") {
        return None;
    }
    match field.as_str() {
        "text" => Some(TypeExpr::named("Text")),
        "int" => Some(TypeExpr::named("Int")),
        "float" => Some(TypeExpr::named("Float")),
        "bool" => Some(TypeExpr::named("Bool")),
        "get" => None,
        _ => None,
    }
}

fn host_call_return_type(name: &str) -> Option<TypeExpr> {
    match name {
        "db.query_one" => Some(TypeExpr::named("DbRow")),
        "db.query" => Some(TypeExpr::Generic {
            name: "List".to_string(),
            args: vec![TypeExpr::named("DbRow")],
        }),
        _ => None,
    }
}

fn is_result_constructor(name: &str) -> bool {
    name == "Ok" || name == "Err"
}

fn is_builtin_task(name: &str) -> bool {
    name == "len"
}

fn is_host_root(name: &str) -> bool {
    matches!(
        name,
        "db" | "fs" | "http" | "shell" | "model" | "secrets" | "deploy"
    )
}

fn is_host_callee_path(name: &str) -> bool {
    name.split('.').next().is_some_and(is_host_root)
}

fn types_compatible(expected: &TypeExpr, actual: &TypeExpr) -> bool {
    expected == actual
}

fn is_named_type(ty: &TypeExpr, expected: &str) -> bool {
    matches!(ty, TypeExpr::Named { name } if name == expected)
}

fn is_bool_type(ty: &TypeExpr) -> bool {
    is_named_type(ty, "Bool")
}

fn is_numeric_type(ty: &TypeExpr) -> bool {
    is_named_type(ty, "Int") || is_named_type(ty, "Float")
}

fn is_list_type(ty: &TypeExpr) -> bool {
    matches!(ty, TypeExpr::Generic { name, args } if name == "List" && args.len() == 1)
}

fn list_element_type(ty: &TypeExpr) -> Option<TypeExpr> {
    match ty {
        TypeExpr::Generic { name, args } if name == "List" && args.len() == 1 => {
            Some(args[0].clone())
        }
        _ => None,
    }
}

fn is_map_type(ty: &TypeExpr) -> bool {
    matches!(ty, TypeExpr::Generic { name, args } if name == "Map" && args.len() == 2)
}

fn text_key_map_value_type(ty: &TypeExpr) -> Option<TypeExpr> {
    match ty {
        TypeExpr::Generic { name, args }
            if name == "Map" && args.len() == 2 && is_named_type(&args[0], "Text") =>
        {
            Some(args[1].clone())
        }
        _ => None,
    }
}

fn binary_operands_compatible(op: &BinaryOp, left: &TypeExpr, right: &TypeExpr) -> bool {
    match op {
        BinaryOp::Or | BinaryOp::And => is_bool_type(left) && is_bool_type(right),
        BinaryOp::Equal | BinaryOp::NotEqual => types_compatible(left, right),
        BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
            is_numeric_type(left) && is_numeric_type(right)
        }
        BinaryOp::Add => {
            (is_numeric_type(left) && is_numeric_type(right))
                || (is_named_type(left, "Text") && is_named_type(right, "Text"))
        }
        BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Remainder => {
            is_numeric_type(left) && is_numeric_type(right)
        }
    }
}

fn infer_binary_type(op: &BinaryOp, left: &TypeExpr, right: &TypeExpr) -> Option<TypeExpr> {
    if !binary_operands_compatible(op, left, right) {
        return None;
    }

    match op {
        BinaryOp::Or
        | BinaryOp::And
        | BinaryOp::Equal
        | BinaryOp::NotEqual
        | BinaryOp::Less
        | BinaryOp::LessEqual
        | BinaryOp::Greater
        | BinaryOp::GreaterEqual => Some(TypeExpr::named("Bool")),
        BinaryOp::Add if is_named_type(left, "Text") && is_named_type(right, "Text") => {
            Some(TypeExpr::named("Text"))
        }
        BinaryOp::Add
        | BinaryOp::Subtract
        | BinaryOp::Multiply
        | BinaryOp::Divide
        | BinaryOp::Remainder => {
            if is_named_type(left, "Float") || is_named_type(right, "Float") {
                Some(TypeExpr::named("Float"))
            } else {
                Some(TypeExpr::named("Int"))
            }
        }
    }
}

fn return_types_compatible(expected: &TypeExpr, actual: &TypeExpr, source: &str) -> bool {
    expected == actual
        || (expected.generic_name() == Some("Result")
            && (source.trim().starts_with("Ok(") || source.trim().starts_with("Err(")))
}

pub fn has_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(Diagnostic::is_error)
}

#[cfg(test)]
mod tests {
    use crate::parser::parse_program;

    use super::*;

    #[test]
    fn detects_missing_db_effect() {
        let source = r#"
task get_user -> Result<Text, Error> {
  take id: Text

  bind row = call db.query_one("select ?", id)?
  return Ok("x")
}
"#;
        let program = parse_program(source).expect("parse");
        let diagnostics = check_program(&program);
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.id == "EFFECT_UNAUTHORIZED")
        );
    }
}
