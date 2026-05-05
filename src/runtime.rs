use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ast::{ExprKind, Program, StatementKind, TypeExpr};
use crate::diagnostics::Diagnostic;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value")]
pub enum Value {
    Unit,
    Text(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Raw(String),
}

pub fn run_main(program: &Program) -> Result<Value, Vec<Diagnostic>> {
    let Some(main) = program
        .functions
        .iter()
        .find(|function| function.name == "main")
    else {
        return Err(vec![Diagnostic::error(
            "RUNTIME_NO_MAIN",
            "no zero-argument `main` function found",
        )]);
    };

    if !main.params.is_empty() {
        return Err(vec![
            Diagnostic::error(
                "RUNTIME_MAIN_HAS_PARAMS",
                "`main` cannot require parameters",
            )
            .with_node(main.id.clone()),
        ]);
    }

    if !main.effects.is_empty() {
        return Err(vec![
            Diagnostic::error(
                "RUNTIME_CAPABILITY_REQUIRED",
                format!(
                    "`main` requires runtime capabilities: {}",
                    main.effects.join(", ")
                ),
            )
            .with_node(main.id.clone()),
        ]);
    }

    let mut locals = HashMap::new();
    for statement in &main.body.statements {
        match &statement.kind {
            StatementKind::Let { name, expr, .. } => {
                let value = eval_expr(expr, &locals, &main.return_type);
                locals.insert(name.clone(), value);
            }
            StatementKind::Return { expr } => {
                return Ok(eval_expr(expr, &locals, &main.return_type));
            }
            StatementKind::Expr { .. } => {}
        }
    }

    Ok(Value::Unit)
}

fn eval_expr(
    expr: &crate::ast::Expr,
    locals: &HashMap<String, Value>,
    return_type: &TypeExpr,
) -> Value {
    match &expr.kind {
        ExprKind::StringLiteral { value } => Value::Text(value.clone()),
        ExprKind::IntLiteral { value } => Value::Int(*value),
        ExprKind::FloatLiteral { value } => Value::Float(*value),
        ExprKind::BoolLiteral { value } => Value::Bool(*value),
        ExprKind::Identifier { name } => locals.get(name).cloned().unwrap_or(Value::Unit),
        ExprKind::Raw { .. } => {
            let trimmed = expr.source.trim();
            if return_type.generic_name() == Some("Result") && trimmed.starts_with("Ok(") {
                Value::Raw(trimmed.to_string())
            } else {
                Value::Raw(expr.source.clone())
            }
        }
    }
}
