use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use crate::ast::{Expr, ExprKind, FunctionDecl, Program, StatementKind, TypeExpr};
use crate::diagnostics::Diagnostic;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value")]
pub enum Value {
    Unit,
    Text(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Record(BTreeMap<String, Value>),
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

    eval_function(program, main, Vec::new())
}

fn eval_function(
    program: &Program,
    function: &FunctionDecl,
    args: Vec<Value>,
) -> Result<Value, Vec<Diagnostic>> {
    if !function.effects.is_empty() {
        return Err(vec![
            Diagnostic::error(
                "RUNTIME_CAPABILITY_REQUIRED",
                format!(
                    "`{}` requires runtime capabilities: {}",
                    function.name,
                    function.effects.join(", ")
                ),
            )
            .with_node(function.id.clone()),
        ]);
    }

    if args.len() != function.params.len() {
        return Err(vec![
            Diagnostic::error(
                "RUNTIME_ARITY_MISMATCH",
                format!(
                    "`{}` expected {} arguments but received {}",
                    function.name,
                    function.params.len(),
                    args.len()
                ),
            )
            .with_node(function.id.clone()),
        ]);
    }

    let mut locals = HashMap::new();
    for (param, value) in function.params.iter().zip(args) {
        locals.insert(param.name.clone(), value);
    }

    for statement in &function.body.statements {
        match &statement.kind {
            StatementKind::Let { name, expr, .. } => {
                let value = eval_expr(program, expr, &locals, &function.return_type)?;
                locals.insert(name.clone(), value);
            }
            StatementKind::Return { expr } => {
                return eval_expr(program, expr, &locals, &function.return_type);
            }
            StatementKind::Expr { .. } => {}
        }
    }

    Ok(Value::Unit)
}

fn eval_expr(
    program: &Program,
    expr: &Expr,
    locals: &HashMap<String, Value>,
    return_type: &TypeExpr,
) -> Result<Value, Vec<Diagnostic>> {
    match &expr.kind {
        ExprKind::StringLiteral { value } => Ok(Value::Text(value.clone())),
        ExprKind::IntLiteral { value } => Ok(Value::Int(*value)),
        ExprKind::FloatLiteral { value } => Ok(Value::Float(*value)),
        ExprKind::BoolLiteral { value } => Ok(Value::Bool(*value)),
        ExprKind::Identifier { name } => Ok(locals.get(name).cloned().unwrap_or(Value::Unit)),
        ExprKind::Call { callee, args } => {
            if direct_callee_name(callee).is_some_and(|name| name == "Ok" || name == "Err") {
                return Ok(Value::Raw(expr.source.clone()));
            }
            if let Some(callee_name) = direct_callee_name(callee) {
                if let Some(function) = program
                    .functions
                    .iter()
                    .find(|function| function.name == callee_name)
                {
                    let mut values = Vec::new();
                    for arg in args {
                        values.push(eval_expr(program, arg, locals, return_type)?);
                    }
                    return eval_function(program, function, values);
                }
            }
            Ok(Value::Raw(expr.source.clone()))
        }
        ExprKind::FieldAccess { receiver, field } => {
            match eval_expr(program, receiver, locals, return_type)? {
                Value::Record(fields) => Ok(fields.get(field).cloned().unwrap_or(Value::Unit)),
                _ => Ok(Value::Raw(expr.source.clone())),
            }
        }
        ExprKind::RecordLiteral { fields, .. } => {
            let mut values = BTreeMap::new();
            for field in fields {
                values.insert(
                    field.name.clone(),
                    eval_expr(program, &field.expr, locals, return_type)?,
                );
            }
            Ok(Value::Record(values))
        }
        ExprKind::Try { expr } => eval_expr(program, expr, locals, return_type),
        ExprKind::Raw { .. } => {
            let trimmed = expr.source.trim();
            if return_type.generic_name() == Some("Result") && trimmed.starts_with("Ok(") {
                Ok(Value::Raw(trimmed.to_string()))
            } else {
                Ok(Value::Raw(expr.source.clone()))
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
