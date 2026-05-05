use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use crate::ast::{
    BinaryOp, Expr, ExprKind, FunctionDecl, Program, StatementKind, TypeExpr, UnaryOp,
};
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
            StatementKind::Expr { expr } => {
                eval_expr(program, expr, &locals, &function.return_type)?;
            }
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
        ExprKind::Unary { op, expr: inner } => {
            let value = eval_expr(program, inner, locals, return_type)?;
            eval_unary(op, value, expr)
        }
        ExprKind::Binary { op, left, right } => match op {
            BinaryOp::And => {
                let left_value = eval_expr(program, left, locals, return_type)?;
                let left_value = expect_bool(left_value, left)?;
                if !left_value {
                    return Ok(Value::Bool(false));
                }
                let right_value = eval_expr(program, right, locals, return_type)?;
                Ok(Value::Bool(expect_bool(right_value, right)?))
            }
            BinaryOp::Or => {
                let left_value = eval_expr(program, left, locals, return_type)?;
                let left_value = expect_bool(left_value, left)?;
                if left_value {
                    return Ok(Value::Bool(true));
                }
                let right_value = eval_expr(program, right, locals, return_type)?;
                Ok(Value::Bool(expect_bool(right_value, right)?))
            }
            _ => {
                let left = eval_expr(program, left, locals, return_type)?;
                let right = eval_expr(program, right, locals, return_type)?;
                eval_binary(op, left, right, expr)
            }
        },
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition_value = eval_expr(program, condition, locals, return_type)?;
            if expect_bool(condition_value, condition)? {
                eval_expr(program, then_branch, locals, return_type)
            } else {
                eval_expr(program, else_branch, locals, return_type)
            }
        }
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

fn eval_unary(op: &UnaryOp, value: Value, expr: &Expr) -> Result<Value, Vec<Diagnostic>> {
    match (op, value) {
        (UnaryOp::Not, Value::Bool(value)) => Ok(Value::Bool(!value)),
        (UnaryOp::Negate, Value::Int(value)) => Ok(Value::Int(-value)),
        (UnaryOp::Negate, Value::Float(value)) => Ok(Value::Float(-value)),
        _ => runtime_type_error(
            expr,
            format!("operator `{}` cannot be applied at runtime", op.as_str()),
        ),
    }
}

fn eval_binary(
    op: &BinaryOp,
    left: Value,
    right: Value,
    expr: &Expr,
) -> Result<Value, Vec<Diagnostic>> {
    match op {
        BinaryOp::Add => match (left, right) {
            (Value::Text(left), Value::Text(right)) => Ok(Value::Text(format!("{left}{right}"))),
            (Value::Int(left), Value::Int(right)) => Ok(Value::Int(left + right)),
            (Value::Int(left), Value::Float(right)) => Ok(Value::Float(left as f64 + right)),
            (Value::Float(left), Value::Int(right)) => Ok(Value::Float(left + right as f64)),
            (Value::Float(left), Value::Float(right)) => Ok(Value::Float(left + right)),
            _ => runtime_type_error(expr, "invalid operands for `+`"),
        },
        BinaryOp::Subtract => match (left, right) {
            (Value::Int(left), Value::Int(right)) => Ok(Value::Int(left - right)),
            (Value::Int(left), Value::Float(right)) => Ok(Value::Float(left as f64 - right)),
            (Value::Float(left), Value::Int(right)) => Ok(Value::Float(left - right as f64)),
            (Value::Float(left), Value::Float(right)) => Ok(Value::Float(left - right)),
            _ => runtime_type_error(expr, "invalid operands for `-`"),
        },
        BinaryOp::Multiply => match (left, right) {
            (Value::Int(left), Value::Int(right)) => Ok(Value::Int(left * right)),
            (Value::Int(left), Value::Float(right)) => Ok(Value::Float(left as f64 * right)),
            (Value::Float(left), Value::Int(right)) => Ok(Value::Float(left * right as f64)),
            (Value::Float(left), Value::Float(right)) => Ok(Value::Float(left * right)),
            _ => runtime_type_error(expr, "invalid operands for `*`"),
        },
        BinaryOp::Divide => match (left, right) {
            (_, Value::Int(0)) | (_, Value::Float(0.0)) => Err(vec![
                Diagnostic::error("RUNTIME_DIVIDE_BY_ZERO", "division by zero")
                    .with_node(expr.id.clone()),
            ]),
            (Value::Int(left), Value::Int(right)) => Ok(Value::Int(left / right)),
            (Value::Int(left), Value::Float(right)) => Ok(Value::Float(left as f64 / right)),
            (Value::Float(left), Value::Int(right)) => Ok(Value::Float(left / right as f64)),
            (Value::Float(left), Value::Float(right)) => Ok(Value::Float(left / right)),
            _ => runtime_type_error(expr, "invalid operands for `/`"),
        },
        BinaryOp::Remainder => match (left, right) {
            (_, Value::Int(0)) | (_, Value::Float(0.0)) => Err(vec![
                Diagnostic::error("RUNTIME_DIVIDE_BY_ZERO", "remainder by zero")
                    .with_node(expr.id.clone()),
            ]),
            (Value::Int(left), Value::Int(right)) => Ok(Value::Int(left % right)),
            (Value::Int(left), Value::Float(right)) => Ok(Value::Float(left as f64 % right)),
            (Value::Float(left), Value::Int(right)) => Ok(Value::Float(left % right as f64)),
            (Value::Float(left), Value::Float(right)) => Ok(Value::Float(left % right)),
            _ => runtime_type_error(expr, "invalid operands for `%`"),
        },
        BinaryOp::Equal => Ok(Value::Bool(values_equal(&left, &right))),
        BinaryOp::NotEqual => Ok(Value::Bool(!values_equal(&left, &right))),
        BinaryOp::Less => compare_numeric(left, right, expr, |left, right| left < right),
        BinaryOp::LessEqual => compare_numeric(left, right, expr, |left, right| left <= right),
        BinaryOp::Greater => compare_numeric(left, right, expr, |left, right| left > right),
        BinaryOp::GreaterEqual => compare_numeric(left, right, expr, |left, right| left >= right),
        BinaryOp::And | BinaryOp::Or => unreachable!("logical operators short-circuit earlier"),
    }
}

fn expect_bool(value: Value, expr: &Expr) -> Result<bool, Vec<Diagnostic>> {
    match value {
        Value::Bool(value) => Ok(value),
        _ => Err(vec![
            Diagnostic::error("RUNTIME_TYPE_ERROR", "expected `Bool` at runtime")
                .with_node(expr.id.clone()),
        ]),
    }
}

fn compare_numeric(
    left: Value,
    right: Value,
    expr: &Expr,
    compare: impl FnOnce(f64, f64) -> bool,
) -> Result<Value, Vec<Diagnostic>> {
    match (number_as_f64(left), number_as_f64(right)) {
        (Some(left), Some(right)) => Ok(Value::Bool(compare(left, right))),
        _ => runtime_type_error(expr, "comparison operands must be numeric"),
    }
}

fn number_as_f64(value: Value) -> Option<f64> {
    match value {
        Value::Int(value) => Some(value as f64),
        Value::Float(value) => Some(value),
        _ => None,
    }
}

fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Int(left), Value::Float(right)) => *left as f64 == *right,
        (Value::Float(left), Value::Int(right)) => *left == *right as f64,
        _ => left == right,
    }
}

fn runtime_type_error(expr: &Expr, message: impl Into<String>) -> Result<Value, Vec<Diagnostic>> {
    Err(vec![
        Diagnostic::error("RUNTIME_TYPE_ERROR", message).with_node(expr.id.clone()),
    ])
}

fn direct_callee_name(expr: &Expr) -> Option<&str> {
    match &expr.kind {
        ExprKind::Identifier { name } => Some(name.as_str()),
        _ => None,
    }
}
