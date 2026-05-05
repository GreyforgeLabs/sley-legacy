use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ast::{
    BinaryOp, BindingKind, Expr, ExprKind, Program, StatementKind, TakeDecl, TaskDecl, TypeExpr,
    UnaryOp,
};
use crate::diagnostics::Diagnostic;
use crate::symbols::{
    EffectResolution, TaskResolution, callee_path, resolve_effect, resolve_task, task_module,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "value")]
pub enum Value {
    Unit,
    Text(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Gate(RuntimeGate),
    List(Vec<Value>),
    Map(BTreeMap<String, Value>),
    Record(BTreeMap<String, Value>),
    Raw(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeGate {
    pub effect: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<PathBuf>,
}

impl RuntimeGate {
    pub fn new(effect: impl Into<String>) -> Self {
        Self {
            effect: effect.into(),
            root: None,
        }
    }

    pub fn with_root(effect: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        Self {
            effect: effect.into(),
            root: Some(root.into()),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeGates {
    gates: BTreeMap<String, RuntimeGate>,
}

impl RuntimeGates {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allow(effect: impl Into<String>) -> Self {
        let mut gates = Self::new();
        gates.grant(RuntimeGate::new(effect));
        gates
    }

    pub fn grant(&mut self, gate: RuntimeGate) {
        self.gates.insert(gate.effect.clone(), gate);
    }

    pub fn grant_effect(&mut self, effect: impl Into<String>) {
        self.grant(RuntimeGate::new(effect));
    }

    pub fn grant_effect_root(&mut self, effect: impl Into<String>, root: impl Into<PathBuf>) {
        self.grant(RuntimeGate::with_root(effect, root));
    }

    pub fn is_empty(&self) -> bool {
        self.gates.is_empty()
    }

    pub fn allows(&self, effect: &str) -> bool {
        self.gates.contains_key(effect)
    }

    pub fn get(&self, effect: &str) -> Option<&RuntimeGate> {
        self.gates.get(effect)
    }

    fn get_any(&self, effects: &[&str]) -> Option<&RuntimeGate> {
        effects.iter().find_map(|effect| self.get(effect))
    }
}

const MAX_LOOP_ITERATIONS: usize = 1_000_000;

pub fn run_main(program: &Program) -> Result<Value, Vec<Diagnostic>> {
    let gates = RuntimeGates::new();
    run_main_with_gates(program, &gates)
}

pub fn run_main_with_gates(
    program: &Program,
    gates: &RuntimeGates,
) -> Result<Value, Vec<Diagnostic>> {
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

    if runtime_arg_count(main) != 0 {
        return Err(vec![
            Diagnostic::error("RUNTIME_MAIN_HAS_TAKES", "`main` cannot require takes")
                .with_node(main.id.clone()),
        ]);
    }

    eval_task(program, main, Vec::new(), gates)
}

fn eval_task(
    program: &Program,
    task: &TaskDecl,
    args: Vec<Value>,
    gates: &RuntimeGates,
) -> Result<Value, Vec<Diagnostic>> {
    let missing_effects = missing_task_effects(program, task, gates);
    if !missing_effects.is_empty() {
        return Err(vec![
            Diagnostic::error(
                "RUNTIME_CAPABILITY_REQUIRED",
                format!(
                    "`{}` requires runtime capabilities: {}",
                    task.name,
                    missing_effects.join(", ")
                ),
            )
            .with_node(task.id.clone()),
        ]);
    }

    let expected_args = runtime_arg_count(task);
    if args.len() != expected_args {
        return Err(vec![
            Diagnostic::error(
                "RUNTIME_ARITY_MISMATCH",
                format!(
                    "`{}` expected {} arguments but received {}",
                    task.name,
                    expected_args,
                    args.len()
                ),
            )
            .with_node(task.id.clone()),
        ]);
    }

    let mut locals = HashMap::new();
    let mut args = args.into_iter();
    for take in &task.takes {
        let value = if take.binding_kind == BindingKind::Gate {
            gate_take_value(program, task, take, gates)?
        } else {
            args.next()
                .expect("runtime arity check guarantees enough call arguments")
        };
        locals.insert(take.name.clone(), value);
    }

    if let Some(value) = eval_block(program, task, &task.body, &mut locals, gates)? {
        return Ok(value);
    }

    Ok(Value::Unit)
}

fn eval_block(
    program: &Program,
    task: &TaskDecl,
    block: &crate::ast::Block,
    locals: &mut HashMap<String, Value>,
    gates: &RuntimeGates,
) -> Result<Option<Value>, Vec<Diagnostic>> {
    for statement in &block.statements {
        if let Some(value) = eval_statement(program, task, statement, locals, gates)? {
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
    gates: &RuntimeGates,
) -> Result<Option<Value>, Vec<Diagnostic>> {
    let existing = locals.keys().cloned().collect::<HashSet<_>>();
    let result = eval_block(program, task, block, locals, gates);
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
    gates: &RuntimeGates,
) -> Result<Option<Value>, Vec<Diagnostic>> {
    let existing = locals.keys().cloned().collect::<HashSet<_>>();
    locals.insert(item.to_string(), value);
    let result = eval_block(program, task, block, locals, gates);
    locals.retain(|name, _| existing.contains(name));
    result
}

fn eval_statement(
    program: &Program,
    task: &TaskDecl,
    statement: &crate::ast::Statement,
    locals: &mut HashMap<String, Value>,
    gates: &RuntimeGates,
) -> Result<Option<Value>, Vec<Diagnostic>> {
    match &statement.kind {
        StatementKind::Binding { name, expr, .. } => {
            let value = eval_expr(program, task, expr, locals, gates)?;
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
            let value = eval_expr(program, task, expr, locals, gates)?;
            locals.insert(name.clone(), value);
            Ok(None)
        }
        StatementKind::Return { expr } => Ok(Some(eval_expr(program, task, expr, locals, gates)?)),
        StatementKind::Expr { expr } => {
            eval_expr(program, task, expr, locals, gates)?;
            Ok(None)
        }
        StatementKind::If {
            condition,
            then_block,
            else_block,
        } => {
            let condition_value = eval_expr(program, task, condition, locals, gates)?;
            if expect_bool(condition_value, condition)? {
                eval_scoped_block(program, task, then_block, locals, gates)
            } else if let Some(else_block) = else_block {
                eval_scoped_block(program, task, else_block, locals, gates)
            } else {
                Ok(None)
            }
        }
        StatementKind::While { condition, body } => {
            for _ in 0..MAX_LOOP_ITERATIONS {
                let condition_value = eval_expr(program, task, condition, locals, gates)?;
                if !expect_bool(condition_value, condition)? {
                    return Ok(None);
                }
                if let Some(value) = eval_scoped_block(program, task, body, locals, gates)? {
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
            let collection_value = eval_expr(program, task, collection, locals, gates)?;
            match collection_value {
                Value::List(items) => {
                    for value in items {
                        if let Some(return_value) =
                            eval_for_body(program, task, body, locals, item, value, gates)?
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
        StatementKind::Forge { body } => eval_scoped_block(program, task, body, locals, gates),
    }
}

fn eval_expr(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    locals: &HashMap<String, Value>,
    gates: &RuntimeGates,
) -> Result<Value, Vec<Diagnostic>> {
    match &expr.kind {
        ExprKind::StringLiteral { value } => Ok(Value::Text(value.clone())),
        ExprKind::IntLiteral { value } => Ok(Value::Int(*value)),
        ExprKind::FloatLiteral { value } => Ok(Value::Float(*value)),
        ExprKind::BoolLiteral { value } => Ok(Value::Bool(*value)),
        ExprKind::Identifier { name } => Ok(locals.get(name).cloned().unwrap_or(Value::Unit)),
        ExprKind::Unary { op, expr: inner } => {
            let value = eval_expr(program, task, inner, locals, gates)?;
            eval_unary(op, value, expr)
        }
        ExprKind::Binary { op, left, right } => match op {
            BinaryOp::And => {
                let left_value = eval_expr(program, task, left, locals, gates)?;
                let left_value = expect_bool(left_value, left)?;
                if !left_value {
                    return Ok(Value::Bool(false));
                }
                let right_value = eval_expr(program, task, right, locals, gates)?;
                Ok(Value::Bool(expect_bool(right_value, right)?))
            }
            BinaryOp::Or => {
                let left_value = eval_expr(program, task, left, locals, gates)?;
                let left_value = expect_bool(left_value, left)?;
                if left_value {
                    return Ok(Value::Bool(true));
                }
                let right_value = eval_expr(program, task, right, locals, gates)?;
                Ok(Value::Bool(expect_bool(right_value, right)?))
            }
            _ => {
                let left = eval_expr(program, task, left, locals, gates)?;
                let right = eval_expr(program, task, right, locals, gates)?;
                eval_binary(op, left, right, expr)
            }
        },
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition_value = eval_expr(program, task, condition, locals, gates)?;
            if expect_bool(condition_value, condition)? {
                eval_expr(program, task, then_branch, locals, gates)
            } else {
                eval_expr(program, task, else_branch, locals, gates)
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
                return match eval_expr(program, task, &args[0], locals, gates)? {
                    Value::List(items) => Ok(Value::Int(items.len() as i64)),
                    Value::Map(items) => Ok(Value::Int(items.len() as i64)),
                    Value::Text(value) => Ok(Value::Int(value.chars().count() as i64)),
                    _ => {
                        runtime_type_error(expr, "builtin `len` expects a list, map, or text value")
                    }
                };
            }
            if let Some(callee_name) = callee_path(callee)
                && is_host_callee_path(&callee_name)
            {
                return eval_host_call(program, task, expr, &callee_name, args, locals, gates);
            }
            if let Some(callee_name) = callee_path(callee)
                && let TaskResolution::Resolved { index, .. } =
                    resolve_task(program, &task_module(task), &callee_name)
            {
                let mut values = Vec::new();
                for arg in args {
                    values.push(eval_expr(program, task, arg, locals, gates)?);
                }
                return eval_task(program, &program.tasks[index], values, gates);
            }
            Ok(Value::Raw(expr.source.clone()))
        }
        ExprKind::ListLiteral { items } => {
            let mut values = Vec::new();
            for item in items {
                values.push(eval_expr(program, task, item, locals, gates)?);
            }
            Ok(Value::List(values))
        }
        ExprKind::MapLiteral { entries } => {
            let mut values = BTreeMap::new();
            for entry in entries {
                let key = eval_expr(program, task, &entry.key, locals, gates)?;
                let Value::Text(key) = key else {
                    return runtime_type_error(expr, "map literal keys must be text values");
                };
                values.insert(key, eval_expr(program, task, &entry.value, locals, gates)?);
            }
            Ok(Value::Map(values))
        }
        ExprKind::Index { collection, index } => {
            let collection = eval_expr(program, task, collection, locals, gates)?;
            let index = eval_expr(program, task, index, locals, gates)?;
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
            match eval_expr(program, task, receiver, locals, gates)? {
                Value::Record(fields) => Ok(fields.get(field).cloned().unwrap_or(Value::Unit)),
                _ => Ok(Value::Raw(expr.source.clone())),
            }
        }
        ExprKind::RecordLiteral { fields, .. } => {
            let mut values = BTreeMap::new();
            for field in fields {
                values.insert(
                    field.name.clone(),
                    eval_expr(program, task, &field.expr, locals, gates)?,
                );
            }
            Ok(Value::Record(values))
        }
        ExprKind::Try { expr } => eval_expr(program, task, expr, locals, gates),
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

fn runtime_arg_count(task: &TaskDecl) -> usize {
    task.takes
        .iter()
        .filter(|take| take.binding_kind != BindingKind::Gate)
        .count()
}

fn missing_task_effects(program: &Program, task: &TaskDecl, gates: &RuntimeGates) -> Vec<String> {
    task.effects
        .iter()
        .filter_map(|effect| {
            let normalized = normalize_effect_name(program, &task_module(task), effect);
            (!gates.allows(&normalized) && !gates.allows(effect)).then_some(normalized)
        })
        .collect()
}

fn gate_take_value(
    program: &Program,
    task: &TaskDecl,
    take: &TakeDecl,
    gates: &RuntimeGates,
) -> Result<Value, Vec<Diagnostic>> {
    let Some(effect) = gate_effect_from_type(program, &task_module(task), &take.ty) else {
        return Err(vec![
            Diagnostic::error(
                "RUNTIME_GATE_TYPE_INVALID",
                format!(
                    "gate take `{}` must use `Gate<Effect>` but found `{}`",
                    take.name,
                    take.ty.display()
                ),
            )
            .with_node(take.id.clone()),
        ]);
    };
    let Some(gate) = gates.get(&effect) else {
        return Err(vec![
            Diagnostic::error(
                "RUNTIME_GATE_REQUIRED",
                format!(
                    "gate take `{}` requires runtime capability `{effect}`",
                    take.name
                ),
            )
            .with_node(take.id.clone()),
        ]);
    };
    Ok(Value::Gate(gate.clone()))
}

fn gate_effect_from_type(program: &Program, module: &str, ty: &TypeExpr) -> Option<String> {
    let TypeExpr::Generic { name, args } = ty else {
        return None;
    };
    if name != "Gate" || args.len() != 1 {
        return None;
    }
    let TypeExpr::Named { name } = &args[0] else {
        return None;
    };
    Some(normalize_effect_name(program, module, name))
}

fn normalize_effect_name(program: &Program, module: &str, name: &str) -> String {
    match resolve_effect(program, module, name) {
        EffectResolution::Builtin(name) | EffectResolution::Resolved { fq_name: name, .. } => name,
        EffectResolution::Unknown
        | EffectResolution::Ambiguous(_)
        | EffectResolution::Private(_) => name.to_string(),
    }
}

fn eval_host_call(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    callee_name: &str,
    args: &[Expr],
    locals: &HashMap<String, Value>,
    gates: &RuntimeGates,
) -> Result<Value, Vec<Diagnostic>> {
    let Some(required_effects) = host_required_effects(callee_name) else {
        return Err(vec![
            Diagnostic::error(
                "RUNTIME_HOST_CALL_UNSUPPORTED",
                format!("host call `{callee_name}` has no runtime handler"),
            )
            .with_node(expr.id.clone()),
        ]);
    };
    let Some(gate) = gates.get_any(required_effects) else {
        return Err(vec![
            Diagnostic::error(
                "RUNTIME_CAPABILITY_REQUIRED",
                format!(
                    "host call `{callee_name}` requires runtime capability `{}`",
                    required_effects.join(" | ")
                ),
            )
            .with_node(expr.id.clone()),
        ]);
    };

    match callee_name {
        "fs.read_text" => {
            if args.len() != 1 {
                return host_arity_error(expr, callee_name, 1, args.len());
            }
            let path = eval_text_arg(program, task, expr, args, 0, locals, gates)?;
            let path = gate_path(gate, &path, expr)?;
            match std::fs::read_to_string(&path) {
                Ok(value) => Ok(Value::Text(value)),
                Err(error) => Err(vec![
                    Diagnostic::error(
                        "RUNTIME_HOST_IO_ERROR",
                        format!(
                            "host call `{callee_name}` failed for `{}`: {error}",
                            path.display()
                        ),
                    )
                    .with_node(expr.id.clone()),
                ]),
            }
        }
        "fs.write_text" => {
            if args.len() != 2 {
                return host_arity_error(expr, callee_name, 2, args.len());
            }
            let path = eval_text_arg(program, task, expr, args, 0, locals, gates)?;
            let contents = eval_text_arg(program, task, expr, args, 1, locals, gates)?;
            let path = gate_path(gate, &path, expr)?;
            match std::fs::write(&path, contents) {
                Ok(()) => Ok(Value::Unit),
                Err(error) => Err(vec![
                    Diagnostic::error(
                        "RUNTIME_HOST_IO_ERROR",
                        format!(
                            "host call `{callee_name}` failed for `{}`: {error}",
                            path.display()
                        ),
                    )
                    .with_node(expr.id.clone()),
                ]),
            }
        }
        _ => Err(vec![
            Diagnostic::error(
                "RUNTIME_HOST_CALL_UNSUPPORTED",
                format!("host call `{callee_name}` has no runtime handler"),
            )
            .with_node(expr.id.clone()),
        ]),
    }
}

fn host_required_effects(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "fs.read_text" => Some(&["FileRead"]),
        "fs.write_text" => Some(&["FileWrite"]),
        "db.query_one" | "db.query" => Some(&["DatabaseRead", "DbRead"]),
        _ => None,
    }
}

fn eval_text_arg(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    args: &[Expr],
    index: usize,
    locals: &HashMap<String, Value>,
    gates: &RuntimeGates,
) -> Result<String, Vec<Diagnostic>> {
    match eval_expr(program, task, &args[index], locals, gates)? {
        Value::Text(value) => Ok(value),
        _ => Err(vec![
            Diagnostic::error(
                "RUNTIME_TYPE_ERROR",
                format!("host call argument {index} must be `Text`"),
            )
            .with_node(if args[index].id.is_empty() {
                expr.id.clone()
            } else {
                args[index].id.clone()
            }),
        ]),
    }
}

fn host_arity_error(
    expr: &Expr,
    callee_name: &str,
    expected: usize,
    actual: usize,
) -> Result<Value, Vec<Diagnostic>> {
    Err(vec![
        Diagnostic::error(
            "RUNTIME_HOST_ARITY_MISMATCH",
            format!(
                "host call `{callee_name}` expected {expected} arguments but received {actual}"
            ),
        )
        .with_node(expr.id.clone()),
    ])
}

fn gate_path(gate: &RuntimeGate, path: &str, expr: &Expr) -> Result<PathBuf, Vec<Diagnostic>> {
    let path = absolute_normalized_path(Path::new(path));
    if let Some(root) = gate.root.as_deref() {
        let root = absolute_normalized_path(root);
        if !path.starts_with(&root) {
            return Err(vec![
                Diagnostic::error(
                    "RUNTIME_CAPABILITY_SCOPE_DENIED",
                    format!(
                        "runtime capability `{}` is scoped to `{}` and cannot access `{}`",
                        gate.effect,
                        root.display(),
                        path.display()
                    ),
                )
                .with_node(expr.id.clone()),
            ]);
        }
    }
    Ok(path)
}

fn absolute_normalized_path(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    };
    normalize_path(&absolute)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new("/")),
            Component::Normal(part) => normalized.push(part),
        }
    }
    normalized
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
