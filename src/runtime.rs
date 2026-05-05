use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::ast::{BinaryOp, Expr, ExprKind, Program, StatementKind, TaskDecl, UnaryOp};
use crate::diagnostics::Diagnostic;
use crate::symbols::{TaskResolution, callee_path, resolve_task, task_module};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value")]
pub enum Value {
    Unit,
    Text(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<Value>),
    Map(BTreeMap<String, Value>),
    Record(BTreeMap<String, Value>),
    Raw(String),
}

const MAX_LOOP_ITERATIONS: usize = 1_000_000;

pub fn run_main(program: &Program) -> Result<Value, Vec<Diagnostic>> {
    let main_index = match resolve_task(program, program.module_name(), "main") {
        TaskResolution::Resolved { index, .. } => index,
        _ => {
            return Err(vec![Diagnostic::error(
                "RUNTIME_NO_MAIN",
                "no zero-take entry-module `main` task found",
            )]);
        }
    };
    let main = &program.tasks[main_index];

    if !main.takes.is_empty() {
        return Err(vec![
            Diagnostic::error("RUNTIME_MAIN_HAS_TAKES", "`main` cannot require takes")
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

    eval_task(program, main, Vec::new())
}

fn eval_task(
    program: &Program,
    task: &TaskDecl,
    args: Vec<Value>,
) -> Result<Value, Vec<Diagnostic>> {
    if !task.effects.is_empty() {
        return Err(vec![
            Diagnostic::error(
                "RUNTIME_CAPABILITY_REQUIRED",
                format!(
                    "`{}` requires runtime capabilities: {}",
                    task.name,
                    task.effects.join(", ")
                ),
            )
            .with_node(task.id.clone()),
        ]);
    }

    if args.len() != task.takes.len() {
        return Err(vec![
            Diagnostic::error(
                "RUNTIME_ARITY_MISMATCH",
                format!(
                    "`{}` expected {} arguments but received {}",
                    task.name,
                    task.takes.len(),
                    args.len()
                ),
            )
            .with_node(task.id.clone()),
        ]);
    }

    let mut locals = HashMap::new();
    for (take, value) in task.takes.iter().zip(args) {
        locals.insert(take.name.clone(), value);
    }

    if let Some(value) = eval_block(program, task, &task.body, &mut locals)? {
        return Ok(value);
    }

    Ok(Value::Unit)
}

fn eval_block(
    program: &Program,
    task: &TaskDecl,
    block: &crate::ast::Block,
    locals: &mut HashMap<String, Value>,
) -> Result<Option<Value>, Vec<Diagnostic>> {
    for statement in &block.statements {
        if let Some(value) = eval_statement(program, task, statement, locals)? {
            return Ok(Some(value));
        }
    }
    Ok(None)
}

fn eval_scoped_block(
    program: &Program,
    task: &TaskDecl,
    block: &crate::ast::Block,
    locals: &mut HashMap<String, Value>,
) -> Result<Option<Value>, Vec<Diagnostic>> {
    let existing = locals.keys().cloned().collect::<HashSet<_>>();
    let result = eval_block(program, task, block, locals);
    locals.retain(|name, _| existing.contains(name));
    result
}

fn eval_for_body(
    program: &Program,
    task: &TaskDecl,
    block: &crate::ast::Block,
    locals: &mut HashMap<String, Value>,
    item: &str,
    value: Value,
) -> Result<Option<Value>, Vec<Diagnostic>> {
    let existing = locals.keys().cloned().collect::<HashSet<_>>();
    locals.insert(item.to_string(), value);
    let result = eval_block(program, task, block, locals);
    locals.retain(|name, _| existing.contains(name));
    result
}

fn eval_statement(
    program: &Program,
    task: &TaskDecl,
    statement: &crate::ast::Statement,
    locals: &mut HashMap<String, Value>,
) -> Result<Option<Value>, Vec<Diagnostic>> {
    match &statement.kind {
        StatementKind::Binding { name, expr, .. } => {
            let value = eval_expr(program, task, expr, locals)?;
            locals.insert(name.clone(), value);
            Ok(None)
        }
        StatementKind::Set { name, expr } => {
            if !locals.contains_key(name) {
                return Err(vec![
                    Diagnostic::error(
                        "RUNTIME_UNKNOWN_LOCAL",
                        format!("cannot set unknown local binding `{name}`"),
                    )
                    .with_node(statement.id.clone()),
                ]);
            }
            let value = eval_expr(program, task, expr, locals)?;
            locals.insert(name.clone(), value);
            Ok(None)
        }
        StatementKind::Return { expr } => Ok(Some(eval_expr(program, task, expr, locals)?)),
        StatementKind::Expr { expr } => {
            eval_expr(program, task, expr, locals)?;
            Ok(None)
        }
        StatementKind::If {
            condition,
            then_block,
            else_block,
        } => {
            let condition_value = eval_expr(program, task, condition, locals)?;
            if expect_bool(condition_value, condition)? {
                eval_scoped_block(program, task, then_block, locals)
            } else if let Some(else_block) = else_block {
                eval_scoped_block(program, task, else_block, locals)
            } else {
                Ok(None)
            }
        }
        StatementKind::While { condition, body } => {
            for _ in 0..MAX_LOOP_ITERATIONS {
                let condition_value = eval_expr(program, task, condition, locals)?;
                if !expect_bool(condition_value, condition)? {
                    return Ok(None);
                }
                if let Some(value) = eval_scoped_block(program, task, body, locals)? {
                    return Ok(Some(value));
                }
            }
            Err(vec![
                Diagnostic::error(
                    "RUNTIME_LOOP_LIMIT_EXCEEDED",
                    format!("while loop exceeded {MAX_LOOP_ITERATIONS} iterations"),
                )
                .with_node(statement.id.clone()),
            ])
        }
        StatementKind::For {
            item,
            collection,
            body,
        } => {
            let collection_value = eval_expr(program, task, collection, locals)?;
            match collection_value {
                Value::List(items) => {
                    for value in items {
                        if let Some(return_value) =
                            eval_for_body(program, task, body, locals, item, value)?
                        {
                            return Ok(Some(return_value));
                        }
                    }
                    Ok(None)
                }
                _ => Err(vec![
                    Diagnostic::error("RUNTIME_TYPE_ERROR", "for loop expects a list value")
                        .with_node(statement.id.clone()),
                ]),
            }
        }
        StatementKind::Forge { body } => eval_scoped_block(program, task, body, locals),
    }
}

fn eval_expr(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    locals: &HashMap<String, Value>,
) -> Result<Value, Vec<Diagnostic>> {
    match &expr.kind {
        ExprKind::StringLiteral { value } => Ok(Value::Text(value.clone())),
        ExprKind::IntLiteral { value } => Ok(Value::Int(*value)),
        ExprKind::FloatLiteral { value } => Ok(Value::Float(*value)),
        ExprKind::BoolLiteral { value } => Ok(Value::Bool(*value)),
        ExprKind::Identifier { name } => Ok(locals.get(name).cloned().unwrap_or(Value::Unit)),
        ExprKind::Unary { op, expr: inner } => {
            let value = eval_expr(program, task, inner, locals)?;
            eval_unary(op, value, expr)
        }
        ExprKind::Binary { op, left, right } => match op {
            BinaryOp::And => {
                let left_value = eval_expr(program, task, left, locals)?;
                let left_value = expect_bool(left_value, left)?;
                if !left_value {
                    return Ok(Value::Bool(false));
                }
                let right_value = eval_expr(program, task, right, locals)?;
                Ok(Value::Bool(expect_bool(right_value, right)?))
            }
            BinaryOp::Or => {
                let left_value = eval_expr(program, task, left, locals)?;
                let left_value = expect_bool(left_value, left)?;
                if left_value {
                    return Ok(Value::Bool(true));
                }
                let right_value = eval_expr(program, task, right, locals)?;
                Ok(Value::Bool(expect_bool(right_value, right)?))
            }
            _ => {
                let left = eval_expr(program, task, left, locals)?;
                let right = eval_expr(program, task, right, locals)?;
                eval_binary(op, left, right, expr)
            }
        },
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition_value = eval_expr(program, task, condition, locals)?;
            if expect_bool(condition_value, condition)? {
                eval_expr(program, task, then_branch, locals)
            } else {
                eval_expr(program, task, else_branch, locals)
            }
        }
        ExprKind::Call { callee, args } => {
            if callee_path(callee).is_some_and(|name| name == "Ok" || name == "Err") {
                return Ok(Value::Raw(expr.source.clone()));
            }
            if callee_path(callee).is_some_and(|name| name == "len") {
                if args.len() != 1 {
                    return runtime_type_error(expr, "builtin `len` expects one argument");
                }
                return match eval_expr(program, task, &args[0], locals)? {
                    Value::List(items) => Ok(Value::Int(items.len() as i64)),
                    Value::Map(items) => Ok(Value::Int(items.len() as i64)),
                    Value::Text(value) => Ok(Value::Int(value.chars().count() as i64)),
                    _ => {
                        runtime_type_error(expr, "builtin `len` expects a list, map, or text value")
                    }
                };
            }
            if let Some(callee_name) = callee_path(callee)
                && !is_host_callee_path(&callee_name)
                && let TaskResolution::Resolved { index, .. } =
                    resolve_task(program, &task_module(task), &callee_name)
            {
                let mut values = Vec::new();
                for arg in args {
                    values.push(eval_expr(program, task, arg, locals)?);
                }
                return eval_task(program, &program.tasks[index], values);
            }
            Ok(Value::Raw(expr.source.clone()))
        }
        ExprKind::ListLiteral { items } => {
            let mut values = Vec::new();
            for item in items {
                values.push(eval_expr(program, task, item, locals)?);
            }
            Ok(Value::List(values))
        }
        ExprKind::MapLiteral { entries } => {
            let mut values = BTreeMap::new();
            for entry in entries {
                let key = eval_expr(program, task, &entry.key, locals)?;
                let Value::Text(key) = key else {
                    return runtime_type_error(expr, "map literal keys must be text values");
                };
                values.insert(key, eval_expr(program, task, &entry.value, locals)?);
            }
            Ok(Value::Map(values))
        }
        ExprKind::Index { collection, index } => {
            let collection = eval_expr(program, task, collection, locals)?;
            let index = eval_expr(program, task, index, locals)?;
            match (collection, index) {
                (Value::List(items), Value::Int(index)) if index >= 0 => {
                    items.get(index as usize).cloned().ok_or_else(|| {
                        vec![
                            Diagnostic::error(
                                "RUNTIME_INDEX_OUT_OF_BOUNDS",
                                format!("list index {index} is out of bounds"),
                            )
                            .with_node(expr.id.clone()),
                        ]
                    })
                }
                (Value::List(_), Value::Int(index)) => Err(vec![
                    Diagnostic::error(
                        "RUNTIME_INDEX_OUT_OF_BOUNDS",
                        format!("list index {index} is out of bounds"),
                    )
                    .with_node(expr.id.clone()),
                ]),
                (Value::Map(items), Value::Text(key)) => {
                    items.get(&key).cloned().ok_or_else(|| {
                        vec![
                            Diagnostic::error(
                                "RUNTIME_MAP_KEY_NOT_FOUND",
                                format!("map key `{key}` was not found"),
                            )
                            .with_node(expr.id.clone()),
                        ]
                    })
                }
                _ => runtime_type_error(
                    expr,
                    "indexing expects `List<T>` with `Int` or `Map<Text, T>` with `Text`",
                ),
            }
        }
        ExprKind::FieldAccess { receiver, field } => {
            match eval_expr(program, task, receiver, locals)? {
                Value::Record(fields) => Ok(fields.get(field).cloned().unwrap_or(Value::Unit)),
                _ => Ok(Value::Raw(expr.source.clone())),
            }
        }
        ExprKind::RecordLiteral { fields, .. } => {
            let mut values = BTreeMap::new();
            for field in fields {
                values.insert(
                    field.name.clone(),
                    eval_expr(program, task, &field.expr, locals)?,
                );
            }
            Ok(Value::Record(values))
        }
        ExprKind::Try { expr } => eval_expr(program, task, expr, locals),
        ExprKind::Raw { .. } => {
            let trimmed = expr.source.trim();
            if task.return_type.generic_name() == Some("Result") && trimmed.starts_with("Ok(") {
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

fn is_host_callee_path(name: &str) -> bool {
    matches!(
        name.split('.').next(),
        Some("db" | "fs" | "http" | "shell" | "model" | "secrets" | "deploy")
    )
}
