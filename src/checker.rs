use std::collections::{HashMap, HashSet};

use crate::ast::{
    BinaryOp, Expr, ExprKind, FunctionDecl, Program, RecordField, StatementKind, TypeExpr, UnaryOp,
};
use crate::diagnostics::{Diagnostic, RepairHint};

pub fn check_program(program: &Program) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let known_types = collect_known_types(program, &mut diagnostics);
    let known_effects = collect_known_effects(program, &mut diagnostics);
    let known_functions = collect_known_functions(program, &mut diagnostics);
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
        validate_type_expr(&ty.value, &known_types, &mut diagnostics, &ty.id);
    }

    for function in &program.functions {
        check_function(
            function,
            &known_types,
            &known_effects,
            &known_functions,
            &record_types,
            &mut diagnostics,
        );
    }

    diagnostics
}

#[derive(Debug, Clone)]
struct FunctionSignature {
    params: Vec<TypeExpr>,
    return_type: TypeExpr,
    effects: Vec<String>,
}

fn collect_known_functions(
    program: &Program,
    diagnostics: &mut Vec<Diagnostic>,
) -> HashMap<String, FunctionSignature> {
    let mut known = HashMap::new();
    for function in &program.functions {
        if known
            .insert(
                function.name.clone(),
                FunctionSignature {
                    params: function
                        .params
                        .iter()
                        .map(|param| param.ty.clone())
                        .collect(),
                    return_type: function.return_type.clone(),
                    effects: function.effects.clone(),
                },
            )
            .is_some()
        {
            diagnostics.push(
                Diagnostic::error(
                    "DUPLICATE_FUNCTION",
                    format!("function `{}` is declared more than once", function.name),
                )
                .with_node(function.id.clone()),
            );
        }
    }
    known
}

fn collect_record_types(program: &Program) -> HashMap<String, Vec<RecordField>> {
    program
        .types
        .iter()
        .filter_map(|ty| match &ty.value {
            TypeExpr::Record { fields } => Some((ty.name.clone(), fields.clone())),
            _ => None,
        })
        .collect()
}

fn collect_known_types(program: &Program, diagnostics: &mut Vec<Diagnostic>) -> HashSet<String> {
    let mut known = [
        "Int", "Float", "Bool", "Text", "Unit", "List", "Map", "Optional", "Result", "Error",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<HashSet<_>>();

    for ty in &program.types {
        if !known.insert(ty.name.clone()) {
            diagnostics.push(
                Diagnostic::error(
                    "DUPLICATE_TYPE",
                    format!("type `{}` is already declared", ty.name),
                )
                .with_node(ty.id.clone()),
            );
        }
    }

    known
}

fn collect_known_effects(program: &Program, diagnostics: &mut Vec<Diagnostic>) -> HashSet<String> {
    let mut known = [
        "FileRead",
        "FileWrite",
        "Network",
        "Shell",
        "ModelCall",
        "SecretRead",
        "Spend",
        "Deploy",
        "DatabaseRead",
        "DatabaseWrite",
        "DbRead",
        "DbWrite",
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<HashSet<_>>();

    let mut local_effects = HashSet::new();
    for effect in &program.effects {
        if !local_effects.insert(effect.name.clone()) {
            diagnostics.push(
                Diagnostic::error(
                    "DUPLICATE_EFFECT",
                    format!("effect `{}` is already declared", effect.name),
                )
                .with_node(effect.id.clone()),
            );
        }
        known.insert(effect.name.clone());
    }

    known
}

fn check_function(
    function: &FunctionDecl,
    known_types: &HashSet<String>,
    known_effects: &HashSet<String>,
    known_functions: &HashMap<String, FunctionSignature>,
    record_types: &HashMap<String, Vec<RecordField>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut params = HashSet::new();
    let mut locals = HashMap::new();

    for param in &function.params {
        if !params.insert(param.name.clone()) {
            diagnostics.push(
                Diagnostic::error(
                    "DUPLICATE_PARAMETER",
                    format!(
                        "parameter `{}` appears more than once in `{}`",
                        param.name, function.name
                    ),
                )
                .with_node(function.id.clone()),
            );
        }
        validate_type_expr(&param.ty, known_types, diagnostics, &param.id);
        locals.insert(param.name.clone(), param.ty.clone());
    }

    validate_type_expr(
        &function.return_type,
        known_types,
        diagnostics,
        &function.id,
    );

    let declared_effects = function.effects.iter().cloned().collect::<HashSet<_>>();
    for effect in &function.effects {
        if !known_effects.contains(effect) {
            diagnostics.push(
                Diagnostic::error(
                    "UNKNOWN_EFFECT",
                    format!(
                        "function `{}` uses unknown effect `{effect}`",
                        function.name
                    ),
                )
                .with_node(function.id.clone()),
            );
        }
    }

    for statement in &function.body.statements {
        match &statement.kind {
            StatementKind::Let {
                name,
                type_ann,
                expr,
            } => {
                if let Some(type_ann) = type_ann {
                    validate_type_expr(type_ann, known_types, diagnostics, &statement.id);
                }
                check_expression_effects(function, &expr.source, &declared_effects, diagnostics);
                check_fallible_expression(function, expr, diagnostics);
                check_expr_structure(
                    function,
                    expr,
                    &locals,
                    &declared_effects,
                    known_types,
                    known_functions,
                    record_types,
                    diagnostics,
                );
                let inferred = infer_expr_type(
                    expr,
                    &locals,
                    &function.return_type,
                    known_functions,
                    record_types,
                );
                if let (Some(expected), Some(actual)) = (type_ann, inferred.as_ref()) {
                    if !types_compatible(expected, actual) {
                        diagnostics.push(
                            Diagnostic::error(
                                "TYPE_MISMATCH",
                                format!(
                                    "let `{name}` expects `{}` but initializer looks like `{}`",
                                    expected.display(),
                                    actual.display()
                                ),
                            )
                            .with_node(statement.id.clone()),
                        );
                    }
                }
                let local_type = type_ann
                    .clone()
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
            }
            StatementKind::Return { expr } => {
                check_expression_effects(function, &expr.source, &declared_effects, diagnostics);
                check_fallible_expression(function, expr, diagnostics);
                check_expr_structure(
                    function,
                    expr,
                    &locals,
                    &declared_effects,
                    known_types,
                    known_functions,
                    record_types,
                    diagnostics,
                );
                if let Some(actual) = infer_expr_type(
                    expr,
                    &locals,
                    &function.return_type,
                    known_functions,
                    record_types,
                ) {
                    if !return_types_compatible(&function.return_type, &actual, &expr.source) {
                        diagnostics.push(
                            Diagnostic::error(
                                "RETURN_TYPE_MISMATCH",
                                format!(
                                    "function `{}` returns `{}` but expression looks like `{}`",
                                    function.name,
                                    function.return_type.display(),
                                    actual.display()
                                ),
                            )
                            .with_node(function.id.clone()),
                        );
                    }
                }
            }
            StatementKind::Expr { expr } => {
                check_expression_effects(function, &expr.source, &declared_effects, diagnostics);
                check_fallible_expression(function, expr, diagnostics);
                check_expr_structure(
                    function,
                    expr,
                    &locals,
                    &declared_effects,
                    known_types,
                    known_functions,
                    record_types,
                    diagnostics,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn check_expr_structure(
    function: &FunctionDecl,
    expr: &Expr,
    locals: &HashMap<String, TypeExpr>,
    declared_effects: &HashSet<String>,
    known_types: &HashSet<String>,
    known_functions: &HashMap<String, FunctionSignature>,
    record_types: &HashMap<String, Vec<RecordField>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match &expr.kind {
        ExprKind::Unary { op, expr: inner } => {
            check_expr_structure(
                function,
                inner,
                locals,
                declared_effects,
                known_types,
                known_functions,
                record_types,
                diagnostics,
            );
            check_unary_operator(
                function,
                expr,
                op,
                inner,
                locals,
                known_functions,
                record_types,
                diagnostics,
            );
        }
        ExprKind::Binary { op, left, right } => {
            check_expr_structure(
                function,
                left,
                locals,
                declared_effects,
                known_types,
                known_functions,
                record_types,
                diagnostics,
            );
            check_expr_structure(
                function,
                right,
                locals,
                declared_effects,
                known_types,
                known_functions,
                record_types,
                diagnostics,
            );
            check_binary_operator(
                function,
                expr,
                op,
                left,
                right,
                locals,
                known_functions,
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
                function,
                condition,
                locals,
                declared_effects,
                known_types,
                known_functions,
                record_types,
                diagnostics,
            );
            check_expr_structure(
                function,
                then_branch,
                locals,
                declared_effects,
                known_types,
                known_functions,
                record_types,
                diagnostics,
            );
            check_expr_structure(
                function,
                else_branch,
                locals,
                declared_effects,
                known_types,
                known_functions,
                record_types,
                diagnostics,
            );
            check_if_expression(
                function,
                expr,
                condition,
                then_branch,
                else_branch,
                locals,
                known_functions,
                record_types,
                diagnostics,
            );
        }
        ExprKind::Call { callee, args } => {
            if let Some(callee_name) = direct_callee_name(callee) {
                if !known_functions.contains_key(callee_name) && !is_result_constructor(callee_name)
                {
                    diagnostics.push(
                        Diagnostic::error(
                            "UNKNOWN_FUNCTION",
                            format!("unknown function `{callee_name}`"),
                        )
                        .with_node(callee.id.clone()),
                    );
                }
            } else {
                check_expr_structure(
                    function,
                    callee,
                    locals,
                    declared_effects,
                    known_types,
                    known_functions,
                    record_types,
                    diagnostics,
                );
            }
            for arg in args {
                check_expr_structure(
                    function,
                    arg,
                    locals,
                    declared_effects,
                    known_types,
                    known_functions,
                    record_types,
                    diagnostics,
                );
            }
            if let Some(callee_name) = direct_callee_name(callee) {
                if let Some(signature) = known_functions.get(callee_name) {
                    if args.len() != signature.params.len() {
                        diagnostics.push(
                            Diagnostic::error(
                                "CALL_ARITY_MISMATCH",
                                format!(
                                    "function `{}` calls `{callee_name}` with {} arguments but {} are required",
                                    function.name,
                                    args.len(),
                                    signature.params.len()
                                ),
                            )
                            .with_node(expr.id.clone()),
                        );
                    }
                    for (index, (arg, expected)) in
                        args.iter().zip(signature.params.iter()).enumerate()
                    {
                        if let Some(actual) = infer_expr_type(
                            arg,
                            locals,
                            &function.return_type,
                            known_functions,
                            record_types,
                        ) {
                            if !types_compatible(expected, &actual) {
                                diagnostics.push(
                                    Diagnostic::error(
                                        "CALL_ARGUMENT_TYPE_MISMATCH",
                                        format!(
                                            "argument {index} to `{callee_name}` expects `{}` but expression looks like `{}`",
                                            expected.display(),
                                            actual.display()
                                        ),
                                    )
                                    .with_node(arg.id.clone()),
                                );
                            }
                        }
                    }
                    for effect in &signature.effects {
                        if !declared_effects.contains(effect) {
                            diagnostics.push(
                                Diagnostic::error(
                                    "EFFECT_UNAUTHORIZED",
                                    format!(
                                        "function `{}` calls `{callee_name}` which requires `{effect}`",
                                        function.name
                                    ),
                                )
                                .with_node(expr.id.clone())
                                .with_repair_hint(RepairHint {
                                    kind: "add_required_effect".to_string(),
                                    target: Some(function.id.clone()),
                                    effect: Some(effect.clone()),
                                    replacement: None,
                                }),
                            );
                        }
                    }
                }
            }
        }
        ExprKind::FieldAccess { receiver, field } => {
            check_expr_structure(
                function,
                receiver,
                locals,
                declared_effects,
                known_types,
                known_functions,
                record_types,
                diagnostics,
            );
            if let Some(TypeExpr::Named { name }) = infer_expr_type(
                receiver,
                locals,
                &function.return_type,
                known_functions,
                record_types,
            ) {
                if let Some(fields) = record_types.get(&name) {
                    if !fields
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
                    function,
                    &field.expr,
                    locals,
                    declared_effects,
                    known_types,
                    known_functions,
                    record_types,
                    diagnostics,
                );
            }

            if let Some(type_name) = type_name {
                if !known_types.contains(type_name) {
                    diagnostics.push(
                        Diagnostic::error(
                            "UNKNOWN_TYPE",
                            format!("unknown record literal type `{type_name}`"),
                        )
                        .with_node(expr.id.clone()),
                    );
                } else if let Some(expected_fields) = record_types.get(type_name) {
                    check_record_literal_fields(
                        function,
                        expr,
                        fields,
                        expected_fields,
                        locals,
                        known_functions,
                        record_types,
                        diagnostics,
                    );
                } else {
                    diagnostics.push(
                        Diagnostic::error(
                            "RECORD_LITERAL_NON_RECORD_TYPE",
                            format!("type `{type_name}` is not a record type"),
                        )
                        .with_node(expr.id.clone()),
                    );
                }
            }
        }
        ExprKind::Try { expr: inner } => {
            check_expr_structure(
                function,
                inner,
                locals,
                declared_effects,
                known_types,
                known_functions,
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
                    .with_node(expr.id.clone()),
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn check_unary_operator(
    function: &FunctionDecl,
    expr: &Expr,
    op: &UnaryOp,
    inner: &Expr,
    locals: &HashMap<String, TypeExpr>,
    known_functions: &HashMap<String, FunctionSignature>,
    record_types: &HashMap<String, Vec<RecordField>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(actual) = infer_expr_type(
        inner,
        locals,
        &function.return_type,
        known_functions,
        record_types,
    ) else {
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
            .with_node(expr.id.clone()),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn check_binary_operator(
    function: &FunctionDecl,
    expr: &Expr,
    op: &BinaryOp,
    left: &Expr,
    right: &Expr,
    locals: &HashMap<String, TypeExpr>,
    known_functions: &HashMap<String, FunctionSignature>,
    record_types: &HashMap<String, Vec<RecordField>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let left_type = infer_expr_type(
        left,
        locals,
        &function.return_type,
        known_functions,
        record_types,
    );
    let right_type = infer_expr_type(
        right,
        locals,
        &function.return_type,
        known_functions,
        record_types,
    );
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
            .with_node(expr.id.clone()),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn check_if_expression(
    function: &FunctionDecl,
    expr: &Expr,
    condition: &Expr,
    then_branch: &Expr,
    else_branch: &Expr,
    locals: &HashMap<String, TypeExpr>,
    known_functions: &HashMap<String, FunctionSignature>,
    record_types: &HashMap<String, Vec<RecordField>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(condition_type) = infer_expr_type(
        condition,
        locals,
        &function.return_type,
        known_functions,
        record_types,
    ) {
        if !is_bool_type(&condition_type) {
            diagnostics.push(
                Diagnostic::error(
                    "IF_CONDITION_NOT_BOOL",
                    format!(
                        "if condition must be `Bool`, not `{}`",
                        condition_type.display()
                    ),
                )
                .with_node(condition.id.clone()),
            );
        }
    }

    let then_type = infer_expr_type(
        then_branch,
        locals,
        &function.return_type,
        known_functions,
        record_types,
    );
    let else_type = infer_expr_type(
        else_branch,
        locals,
        &function.return_type,
        known_functions,
        record_types,
    );
    if let (Some(then_type), Some(else_type)) = (then_type, else_type) {
        if !types_compatible(&then_type, &else_type) {
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
}

fn check_record_literal_fields(
    function: &FunctionDecl,
    expr: &Expr,
    fields: &[crate::ast::ExprField],
    expected_fields: &[RecordField],
    locals: &HashMap<String, TypeExpr>,
    known_functions: &HashMap<String, FunctionSignature>,
    record_types: &HashMap<String, Vec<RecordField>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for expected in expected_fields {
        match fields.iter().find(|field| field.name == expected.name) {
            Some(field) => {
                if let Some(actual) = infer_expr_type(
                    &field.expr,
                    locals,
                    &function.return_type,
                    known_functions,
                    record_types,
                ) {
                    if !types_compatible(&expected.ty, &actual) {
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
    known_types: &HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
    node: &str,
) {
    match ty {
        TypeExpr::Named { name } => {
            if !known_types.contains(name) {
                diagnostics.push(
                    Diagnostic::error("UNKNOWN_TYPE", format!("unknown type `{name}`"))
                        .with_node(node.to_string()),
                );
            }
        }
        TypeExpr::Generic { name, args } => {
            if !known_types.contains(name) {
                diagnostics.push(
                    Diagnostic::error("UNKNOWN_TYPE", format!("unknown generic type `{name}`"))
                        .with_node(node.to_string()),
                );
            }
            for arg in args {
                validate_type_expr(arg, known_types, diagnostics, node);
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
                validate_type_expr(&field.ty, known_types, diagnostics, node);
            }
        }
    }
}

fn check_expression_effects(
    function: &FunctionDecl,
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
                        "function `{}` expression `{source}` requires `{required}`",
                        function.name
                    ),
                )
                .with_node(function.id.clone())
                .with_repair_hint(RepairHint {
                    kind: "add_required_effect".to_string(),
                    target: Some(function.id.clone()),
                    effect: Some(required.to_string()),
                    replacement: None,
                }),
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
    function: &FunctionDecl,
    expr: &crate::ast::Expr,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if expr_uses_try(expr) && function.return_type.generic_name() != Some("Result") {
        diagnostics.push(
            Diagnostic::error(
                "QUESTION_REQUIRES_RESULT",
                "`?` may only be used in a function returning Result<T, E>",
            )
            .with_node(function.id.clone()),
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
    expr: &crate::ast::Expr,
    locals: &HashMap<String, TypeExpr>,
    return_type: &TypeExpr,
    known_functions: &HashMap<String, FunctionSignature>,
    record_types: &HashMap<String, Vec<RecordField>>,
) -> Option<TypeExpr> {
    match &expr.kind {
        ExprKind::StringLiteral { .. } => Some(TypeExpr::named("Text")),
        ExprKind::IntLiteral { .. } => Some(TypeExpr::named("Int")),
        ExprKind::FloatLiteral { .. } => Some(TypeExpr::named("Float")),
        ExprKind::BoolLiteral { .. } => Some(TypeExpr::named("Bool")),
        ExprKind::Identifier { name } => locals.get(name).cloned(),
        ExprKind::Unary { op, expr: inner } => {
            let inner_type =
                infer_expr_type(inner, locals, return_type, known_functions, record_types)?;
            match op {
                UnaryOp::Not if is_bool_type(&inner_type) => Some(TypeExpr::named("Bool")),
                UnaryOp::Negate if is_numeric_type(&inner_type) => Some(inner_type),
                _ => None,
            }
        }
        ExprKind::Binary { op, left, right } => {
            let left_type =
                infer_expr_type(left, locals, return_type, known_functions, record_types)?;
            let right_type =
                infer_expr_type(right, locals, return_type, known_functions, record_types)?;
            infer_binary_type(op, &left_type, &right_type)
        }
        ExprKind::If {
            condition: _,
            then_branch,
            else_branch,
        } => {
            let then_type = infer_expr_type(
                then_branch,
                locals,
                return_type,
                known_functions,
                record_types,
            )?;
            let else_type = infer_expr_type(
                else_branch,
                locals,
                return_type,
                known_functions,
                record_types,
            )?;
            types_compatible(&then_type, &else_type).then_some(then_type)
        }
        ExprKind::Call { callee, .. } => {
            if direct_callee_name(callee).is_some_and(|name| name == "Ok" || name == "Err")
                && return_type.generic_name() == Some("Result")
            {
                Some(return_type.clone())
            } else if let Some(callee_name) = direct_callee_name(callee) {
                known_functions
                    .get(callee_name)
                    .map(|signature| signature.return_type.clone())
            } else {
                None
            }
        }
        ExprKind::FieldAccess { receiver, field } => {
            if let Some(TypeExpr::Named { name }) =
                infer_expr_type(receiver, locals, return_type, known_functions, record_types)
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
                Some(TypeExpr::named(type_name.clone()))
            } else {
                let mut inferred_fields = Vec::new();
                for field in fields {
                    inferred_fields.push(RecordField {
                        name: field.name.clone(),
                        ty: infer_expr_type(
                            &field.expr,
                            locals,
                            return_type,
                            known_functions,
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
            match infer_expr_type(expr, locals, return_type, known_functions, record_types) {
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
                Some(return_type.clone())
            } else if trimmed.ends_with('?') {
                None
            } else {
                None
            }
        }
    }
}

fn direct_callee_name(expr: &Expr) -> Option<&str> {
    match &expr.kind {
        ExprKind::Identifier { name } => Some(name.as_str()),
        _ => None,
    }
}

fn is_result_constructor(name: &str) -> bool {
    name == "Ok" || name == "Err"
}

fn is_host_root(name: &str) -> bool {
    matches!(
        name,
        "db" | "fs" | "http" | "shell" | "model" | "secrets" | "deploy"
    )
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
fn get_user(id: Text) -> Result<Text, Error> {
  let row = db.query_one("select ?", id)?
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
