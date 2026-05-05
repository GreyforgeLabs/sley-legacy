use std::collections::{HashMap, HashSet};

use crate::ast::{ExprKind, FunctionDecl, Program, StatementKind, TypeExpr};
use crate::diagnostics::{Diagnostic, RepairHint};

pub fn check_program(program: &Program) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let known_types = collect_known_types(program, &mut diagnostics);
    let known_effects = collect_known_effects(program, &mut diagnostics);

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

    let mut function_names = HashSet::new();
    for function in &program.functions {
        if !function_names.insert(function.name.clone()) {
            diagnostics.push(
                Diagnostic::error(
                    "DUPLICATE_FUNCTION",
                    format!("function `{}` is declared more than once", function.name),
                )
                .with_node(function.id.clone()),
            );
        }
        check_function(function, &known_types, &known_effects, &mut diagnostics);
    }

    diagnostics
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
                let inferred = infer_expr_type(expr, &locals, &function.return_type);
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
                locals.insert(name.clone(), local_type);
            }
            StatementKind::Return { expr } => {
                check_expression_effects(function, &expr.source, &declared_effects, diagnostics);
                check_fallible_expression(function, expr, diagnostics);
                if let Some(actual) = infer_expr_type(expr, &locals, &function.return_type) {
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
            }
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
    let uses_question = expr.source.ends_with('?') || expr.source.contains("? ");
    if uses_question && function.return_type.generic_name() != Some("Result") {
        diagnostics.push(
            Diagnostic::error(
                "QUESTION_REQUIRES_RESULT",
                "`?` may only be used in a function returning Result<T, E>",
            )
            .with_node(function.id.clone()),
        );
    }
}

fn infer_expr_type(
    expr: &crate::ast::Expr,
    locals: &HashMap<String, TypeExpr>,
    return_type: &TypeExpr,
) -> Option<TypeExpr> {
    match &expr.kind {
        ExprKind::StringLiteral { .. } => Some(TypeExpr::named("Text")),
        ExprKind::IntLiteral { .. } => Some(TypeExpr::named("Int")),
        ExprKind::FloatLiteral { .. } => Some(TypeExpr::named("Float")),
        ExprKind::BoolLiteral { .. } => Some(TypeExpr::named("Bool")),
        ExprKind::Identifier { name } => locals.get(name).cloned(),
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

fn types_compatible(expected: &TypeExpr, actual: &TypeExpr) -> bool {
    expected == actual
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
