use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ast::{
    BinaryOp, BindingKind, Expr, ExprKind, Program, StatementKind, TakeDecl, TaskDecl, TypeExpr,
    UnaryOp,
};
use crate::authority::{host_effects_for_callee, is_host_callee_path};
use crate::diagnostics::Diagnostic;
use crate::symbols::{
    EffectResolution, TaskResolution, callee_path, resolve_effect, resolve_task, task_module,
};

pub type DbRow = BTreeMap<String, Value>;
pub type DbRows = Vec<DbRow>;

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
    Ok(Box<Value>),
    Err(Box<Value>),
    Raw(String),
}

impl TryFrom<serde_json::Value> for Value {
    type Error = String;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        match value {
            serde_json::Value::Null => Ok(Value::Unit),
            serde_json::Value::Bool(value) => Ok(Value::Bool(value)),
            serde_json::Value::Number(value) => {
                if let Some(value) = value.as_i64() {
                    Ok(Value::Int(value))
                } else if let Some(value) = value.as_f64() {
                    Ok(Value::Float(value))
                } else {
                    Err("JSON number cannot be represented as a Sley value".to_string())
                }
            }
            serde_json::Value::String(value) => Ok(Value::Text(value)),
            serde_json::Value::Array(items) => items
                .into_iter()
                .map(Value::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map(Value::List),
            serde_json::Value::Object(fields) => fields
                .into_iter()
                .map(|(name, value)| Value::try_from(value).map(|value| (name, value)))
                .collect::<Result<BTreeMap<_, _>, _>>()
                .map(Value::Record),
        }
    }
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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RuntimeGates {
    gates: BTreeMap<String, RuntimeGate>,
    db_tables: BTreeMap<String, DbRows>,
    secret_values: BTreeMap<String, String>,
    deploy_results: BTreeMap<String, String>,
    spend_results: BTreeMap<String, String>,
    http_text_responses: BTreeMap<String, String>,
    shell_outputs: BTreeMap<String, String>,
    model_outputs: BTreeMap<String, String>,
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
            && self.db_tables.is_empty()
            && self.secret_values.is_empty()
            && self.deploy_results.is_empty()
            && self.spend_results.is_empty()
            && self.http_text_responses.is_empty()
            && self.shell_outputs.is_empty()
            && self.model_outputs.is_empty()
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

    pub fn grant_db_rows(&mut self, table: impl Into<String>, rows: DbRows) {
        self.db_tables.insert(normalize_db_table(table), rows);
    }

    pub fn insert_db_row(&mut self, table: impl Into<String>, row: DbRow) {
        self.db_tables
            .entry(normalize_db_table(table))
            .or_default()
            .push(row);
    }

    pub fn db_rows(&self, table: &str) -> Option<&[DbRow]> {
        self.db_tables
            .get(&normalize_db_table(table))
            .map(Vec::as_slice)
    }

    pub fn grant_secret(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.secret_values.insert(name.into(), value.into());
    }

    pub fn secret(&self, name: &str) -> Option<&str> {
        self.secret_values.get(name).map(String::as_str)
    }

    pub fn grant_deploy_result(&mut self, target: impl Into<String>, result: impl Into<String>) {
        self.deploy_results.insert(target.into(), result.into());
    }

    pub fn deploy_result(&self, target: &str) -> Option<&str> {
        self.deploy_results.get(target).map(String::as_str)
    }

    pub fn grant_spend_result(&mut self, request: impl Into<String>, result: impl Into<String>) {
        self.spend_results.insert(request.into(), result.into());
    }

    pub fn spend_result(&self, request: &str) -> Option<&str> {
        self.spend_results.get(request).map(String::as_str)
    }

    pub fn grant_http_text(&mut self, url: impl Into<String>, body: impl Into<String>) {
        self.http_text_responses.insert(url.into(), body.into());
    }

    pub fn http_text(&self, url: &str) -> Option<&str> {
        self.http_text_responses.get(url).map(String::as_str)
    }

    pub fn grant_shell_output(&mut self, command: impl Into<String>, output: impl Into<String>) {
        self.shell_outputs.insert(command.into(), output.into());
    }

    pub fn shell_output(&self, command: &str) -> Option<&str> {
        self.shell_outputs.get(command).map(String::as_str)
    }

    pub fn grant_model_output(&mut self, prompt: impl Into<String>, output: impl Into<String>) {
        self.model_outputs.insert(prompt.into(), output.into());
    }

    pub fn model_output(&self, prompt: &str) -> Option<&str> {
        self.model_outputs.get(prompt).map(String::as_str)
    }
}

const MAX_LOOP_ITERATIONS: usize = 1_000_000;

#[derive(Debug, Clone, PartialEq)]
enum EvalOutcome {
    Value(Value),
    Propagate(Value),
}

impl EvalOutcome {
    fn value(value: Value) -> Self {
        Self::Value(value)
    }

    fn into_task_return(self) -> Value {
        match self {
            Self::Value(value) => value,
            Self::Propagate(error) => Value::Err(Box::new(error)),
        }
    }
}

macro_rules! value_or_propagate {
    ($outcome:expr) => {
        match $outcome? {
            EvalOutcome::Value(value) => value,
            EvalOutcome::Propagate(error) => return Ok(EvalOutcome::Propagate(error)),
        }
    };
}

macro_rules! arg_or_propagate {
    ($arg:expr) => {
        match $arg? {
            Ok(value) => value,
            Err(error) => return Ok(EvalOutcome::Propagate(error)),
        }
    };
}

pub fn run_main(program: &Program) -> Result<Value, Vec<Diagnostic>> {
    let gates = RuntimeGates::new();
    run_main_with_gates(program, &gates)
}

pub fn run_main_with_gates(
    program: &Program,
    gates: &RuntimeGates,
) -> Result<Value, Vec<Diagnostic>> {
    let mut gates = gates.clone();
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

    eval_task(program, main, Vec::new(), &mut gates)
}

fn eval_task(
    program: &Program,
    task: &TaskDecl,
    args: Vec<Value>,
    gates: &mut RuntimeGates,
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
    gates: &mut RuntimeGates,
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
    gates: &mut RuntimeGates,
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
    gates: &mut RuntimeGates,
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
    gates: &mut RuntimeGates,
) -> Result<Option<Value>, Vec<Diagnostic>> {
    match &statement.kind {
        StatementKind::Binding { name, expr, .. } => {
            let value = match eval_expr(program, task, expr, locals, gates)? {
                EvalOutcome::Value(value) => value,
                EvalOutcome::Propagate(error) => {
                    return Ok(Some(Value::Err(Box::new(error))));
                }
            };
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
            let value = match eval_expr(program, task, expr, locals, gates)? {
                EvalOutcome::Value(value) => value,
                EvalOutcome::Propagate(error) => {
                    return Ok(Some(Value::Err(Box::new(error))));
                }
            };
            locals.insert(name.clone(), value);
            Ok(None)
        }
        StatementKind::Return { expr } => Ok(Some(
            eval_expr(program, task, expr, locals, gates)?.into_task_return(),
        )),
        StatementKind::Expr { expr } => {
            if let EvalOutcome::Propagate(error) = eval_expr(program, task, expr, locals, gates)? {
                return Ok(Some(Value::Err(Box::new(error))));
            }
            Ok(None)
        }
        StatementKind::If {
            condition,
            then_block,
            else_block,
        } => {
            let condition_value = match eval_expr(program, task, condition, locals, gates)? {
                EvalOutcome::Value(value) => value,
                EvalOutcome::Propagate(error) => {
                    return Ok(Some(Value::Err(Box::new(error))));
                }
            };
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
                let condition_value = match eval_expr(program, task, condition, locals, gates)? {
                    EvalOutcome::Value(value) => value,
                    EvalOutcome::Propagate(error) => {
                        return Ok(Some(Value::Err(Box::new(error))));
                    }
                };
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
            let collection_value = match eval_expr(program, task, collection, locals, gates)? {
                EvalOutcome::Value(value) => value,
                EvalOutcome::Propagate(error) => {
                    return Ok(Some(Value::Err(Box::new(error))));
                }
            };
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
    gates: &mut RuntimeGates,
) -> Result<EvalOutcome, Vec<Diagnostic>> {
    match &expr.kind {
        ExprKind::StringLiteral { value } => Ok(EvalOutcome::value(Value::Text(value.clone()))),
        ExprKind::IntLiteral { value } => Ok(EvalOutcome::value(Value::Int(*value))),
        ExprKind::FloatLiteral { value } => Ok(EvalOutcome::value(Value::Float(*value))),
        ExprKind::BoolLiteral { value } => Ok(EvalOutcome::value(Value::Bool(*value))),
        ExprKind::Identifier { name } => Ok(EvalOutcome::value(
            locals.get(name).cloned().unwrap_or(Value::Unit),
        )),
        ExprKind::Unary { op, expr: inner } => {
            let value = value_or_propagate!(eval_expr(program, task, inner, locals, gates));
            eval_unary(op, value, expr).map(EvalOutcome::value)
        }
        ExprKind::Binary { op, left, right } => match op {
            BinaryOp::And => {
                let left_value = value_or_propagate!(eval_expr(program, task, left, locals, gates));
                let left_value = expect_bool(left_value, left)?;
                if !left_value {
                    return Ok(EvalOutcome::value(Value::Bool(false)));
                }
                let right_value =
                    value_or_propagate!(eval_expr(program, task, right, locals, gates));
                Ok(EvalOutcome::value(Value::Bool(expect_bool(
                    right_value,
                    right,
                )?)))
            }
            BinaryOp::Or => {
                let left_value = value_or_propagate!(eval_expr(program, task, left, locals, gates));
                let left_value = expect_bool(left_value, left)?;
                if left_value {
                    return Ok(EvalOutcome::value(Value::Bool(true)));
                }
                let right_value =
                    value_or_propagate!(eval_expr(program, task, right, locals, gates));
                Ok(EvalOutcome::value(Value::Bool(expect_bool(
                    right_value,
                    right,
                )?)))
            }
            _ => {
                let left = value_or_propagate!(eval_expr(program, task, left, locals, gates));
                let right = value_or_propagate!(eval_expr(program, task, right, locals, gates));
                eval_binary(op, left, right, expr).map(EvalOutcome::value)
            }
        },
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let condition_value =
                value_or_propagate!(eval_expr(program, task, condition, locals, gates));
            if expect_bool(condition_value, condition)? {
                eval_expr(program, task, then_branch, locals, gates)
            } else {
                eval_expr(program, task, else_branch, locals, gates)
            }
        }
        ExprKind::Call { callee, args } => {
            if let Some(value) =
                eval_value_method_call(program, task, expr, callee, args, locals, gates)
            {
                return value;
            }
            if callee_path(callee).is_some_and(|name| name == "Ok" || name == "Err") {
                return eval_result_constructor(program, task, expr, callee, args, locals, gates);
            }
            if callee_path(callee).is_some_and(|name| name == "len") {
                if args.len() != 1 {
                    return runtime_type_error(expr, "builtin `len` expects one argument")
                        .map(EvalOutcome::value);
                }
                let value = value_or_propagate!(eval_expr(program, task, &args[0], locals, gates));
                return match value {
                    Value::List(items) => Ok(EvalOutcome::value(Value::Int(items.len() as i64))),
                    Value::Map(items) => Ok(EvalOutcome::value(Value::Int(items.len() as i64))),
                    Value::Text(value) => {
                        Ok(EvalOutcome::value(Value::Int(value.chars().count() as i64)))
                    }
                    _ => {
                        runtime_type_error(expr, "builtin `len` expects a list, map, or text value")
                            .map(EvalOutcome::value)
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
                    values.push(value_or_propagate!(eval_expr(
                        program, task, arg, locals, gates
                    )));
                }
                return eval_task(program, &program.tasks[index], values, gates)
                    .map(EvalOutcome::value);
            }
            Ok(EvalOutcome::value(Value::Raw(expr.source.clone())))
        }
        ExprKind::ListLiteral { items } => {
            let mut values = Vec::new();
            for item in items {
                values.push(value_or_propagate!(eval_expr(
                    program, task, item, locals, gates
                )));
            }
            Ok(EvalOutcome::value(Value::List(values)))
        }
        ExprKind::MapLiteral { entries } => {
            let mut values = BTreeMap::new();
            for entry in entries {
                let key = value_or_propagate!(eval_expr(program, task, &entry.key, locals, gates));
                let Value::Text(key) = key else {
                    return runtime_type_error(expr, "map literal keys must be text values")
                        .map(EvalOutcome::value);
                };
                values.insert(
                    key,
                    value_or_propagate!(eval_expr(program, task, &entry.value, locals, gates)),
                );
            }
            Ok(EvalOutcome::value(Value::Map(values)))
        }
        ExprKind::Index { collection, index } => {
            let collection =
                value_or_propagate!(eval_expr(program, task, collection, locals, gates));
            let index = value_or_propagate!(eval_expr(program, task, index, locals, gates));
            match (collection, index) {
                (Value::List(items), Value::Int(index)) if index >= 0 => items
                    .get(index as usize)
                    .cloned()
                    .map(EvalOutcome::value)
                    .ok_or_else(|| {
                        vec![
                            Diagnostic::error(
                                "RUNTIME_INDEX_OUT_OF_BOUNDS",
                                format!("list index {index} is out of bounds"),
                            )
                            .with_node(expr.id.clone()),
                        ]
                    }),
                (Value::List(_), Value::Int(index)) => Err(vec![
                    Diagnostic::error(
                        "RUNTIME_INDEX_OUT_OF_BOUNDS",
                        format!("list index {index} is out of bounds"),
                    )
                    .with_node(expr.id.clone()),
                ]),
                (Value::Map(items), Value::Text(key)) => items
                    .get(&key)
                    .cloned()
                    .map(EvalOutcome::value)
                    .ok_or_else(|| {
                        vec![
                            Diagnostic::error(
                                "RUNTIME_MAP_KEY_NOT_FOUND",
                                format!("map key `{key}` was not found"),
                            )
                            .with_node(expr.id.clone()),
                        ]
                    }),
                _ => runtime_type_error(
                    expr,
                    "indexing expects `List<T>` with `Int` or `Map<Text, T>` with `Text`",
                )
                .map(EvalOutcome::value),
            }
        }
        ExprKind::FieldAccess { receiver, field } => {
            match value_or_propagate!(eval_expr(program, task, receiver, locals, gates)) {
                Value::Record(fields) => Ok(EvalOutcome::value(
                    fields.get(field).cloned().unwrap_or(Value::Unit),
                )),
                _ => Ok(EvalOutcome::value(Value::Raw(expr.source.clone()))),
            }
        }
        ExprKind::RecordLiteral { fields, .. } => {
            let mut values = BTreeMap::new();
            for field in fields {
                values.insert(
                    field.name.clone(),
                    value_or_propagate!(eval_expr(program, task, &field.expr, locals, gates)),
                );
            }
            Ok(EvalOutcome::value(Value::Record(values)))
        }
        ExprKind::Try { expr } => match eval_expr(program, task, expr, locals, gates)? {
            EvalOutcome::Value(Value::Ok(value)) => Ok(EvalOutcome::value(*value)),
            EvalOutcome::Value(Value::Err(error)) => Ok(EvalOutcome::Propagate(*error)),
            EvalOutcome::Value(value) => Ok(EvalOutcome::value(value)),
            EvalOutcome::Propagate(error) => Ok(EvalOutcome::Propagate(error)),
        },
        ExprKind::Raw { .. } => Ok(EvalOutcome::value(Value::Raw(expr.source.clone()))),
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

fn eval_result_constructor(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    callee: &Expr,
    args: &[Expr],
    locals: &HashMap<String, Value>,
    gates: &mut RuntimeGates,
) -> Result<EvalOutcome, Vec<Diagnostic>> {
    if args.len() != 1 {
        return Err(vec![
            Diagnostic::error(
                "RUNTIME_RESULT_ARITY_MISMATCH",
                format!(
                    "result constructor `{}` expected 1 argument but received {}",
                    callee.source,
                    args.len()
                ),
            )
            .with_node(expr.id.clone()),
        ]);
    }
    let value = value_or_propagate!(eval_expr(program, task, &args[0], locals, gates));
    match callee_path(callee).as_deref() {
        Some("Ok") => Ok(EvalOutcome::value(Value::Ok(Box::new(value)))),
        Some("Err") => Ok(EvalOutcome::value(Value::Err(Box::new(value)))),
        _ => Ok(EvalOutcome::value(Value::Raw(expr.source.clone()))),
    }
}

fn eval_host_call(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    callee_name: &str,
    args: &[Expr],
    locals: &HashMap<String, Value>,
    gates: &mut RuntimeGates,
) -> Result<EvalOutcome, Vec<Diagnostic>> {
    let Some(required_effects) = host_effects_for_callee(callee_name) else {
        return Err(vec![
            Diagnostic::error(
                "RUNTIME_HOST_CALL_UNSUPPORTED",
                format!("host call `{callee_name}` has no runtime handler"),
            )
            .with_node(expr.id.clone()),
        ]);
    };
    let Some(gate) = gates.get_any(required_effects).cloned() else {
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
                return host_arity_error(expr, callee_name, 1, args.len()).map(EvalOutcome::value);
            }
            let path =
                arg_or_propagate!(eval_text_arg(program, task, expr, args, 0, locals, gates));
            let path = gate_path(&gate, &path, expr)?;
            match std::fs::read_to_string(&path) {
                Ok(value) => Ok(EvalOutcome::value(Value::Text(value))),
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
        "fs.try_read_text" => {
            if args.len() != 1 {
                return host_arity_error(expr, callee_name, 1, args.len()).map(EvalOutcome::value);
            }
            let path =
                arg_or_propagate!(eval_text_arg(program, task, expr, args, 0, locals, gates));
            let path = gate_path(&gate, &path, expr)?;
            match std::fs::read_to_string(&path) {
                Ok(value) => Ok(host_ok(Value::Text(value))),
                Err(error) => Ok(host_err(
                    "RUNTIME_HOST_IO_ERROR",
                    format!(
                        "host call `{callee_name}` failed for `{}`: {error}",
                        path.display()
                    ),
                )),
            }
        }
        "fs.write_text" => {
            if args.len() != 2 {
                return host_arity_error(expr, callee_name, 2, args.len()).map(EvalOutcome::value);
            }
            let path =
                arg_or_propagate!(eval_text_arg(program, task, expr, args, 0, locals, gates));
            let contents =
                arg_or_propagate!(eval_text_arg(program, task, expr, args, 1, locals, gates));
            let path = gate_path(&gate, &path, expr)?;
            match std::fs::write(&path, contents) {
                Ok(()) => Ok(EvalOutcome::value(Value::Unit)),
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
        "fs.try_write_text" => {
            if args.len() != 2 {
                return host_arity_error(expr, callee_name, 2, args.len()).map(EvalOutcome::value);
            }
            let path =
                arg_or_propagate!(eval_text_arg(program, task, expr, args, 0, locals, gates));
            let contents =
                arg_or_propagate!(eval_text_arg(program, task, expr, args, 1, locals, gates));
            let path = gate_path(&gate, &path, expr)?;
            match std::fs::write(&path, contents) {
                Ok(()) => Ok(host_ok(Value::Unit)),
                Err(error) => Ok(host_err(
                    "RUNTIME_HOST_IO_ERROR",
                    format!(
                        "host call `{callee_name}` failed for `{}`: {error}",
                        path.display()
                    ),
                )),
            }
        }
        "db.query_one" => eval_db_query(
            program,
            task,
            expr,
            callee_name,
            args,
            locals,
            gates,
            DbQueryMode::RawOne,
        ),
        "db.query" => eval_db_query(
            program,
            task,
            expr,
            callee_name,
            args,
            locals,
            gates,
            DbQueryMode::RawMany,
        ),
        "db.try_query_one" => eval_db_query(
            program,
            task,
            expr,
            callee_name,
            args,
            locals,
            gates,
            DbQueryMode::ResultOne,
        ),
        "db.try_query" => eval_db_query(
            program,
            task,
            expr,
            callee_name,
            args,
            locals,
            gates,
            DbQueryMode::ResultMany,
        ),
        "db.try_insert" => eval_db_insert(program, task, expr, callee_name, args, locals, gates),
        "secrets.try_get" => eval_secret_get(program, task, expr, callee_name, args, locals, gates),
        "deploy.try_stage" => {
            eval_deploy_stage(program, task, expr, callee_name, args, locals, gates)
        }
        "spend.try_authorize" => {
            eval_spend_authorize(program, task, expr, callee_name, args, locals, gates)
        }
        "http.try_get_text" => {
            eval_http_get_text(program, task, expr, callee_name, args, locals, gates)
        }
        "shell.try_run" => eval_shell_run(program, task, expr, callee_name, args, locals, gates),
        "model.try_complete" => {
            eval_model_complete(program, task, expr, callee_name, args, locals, gates)
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

fn eval_value_method_call(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    callee: &Expr,
    args: &[Expr],
    locals: &HashMap<String, Value>,
    gates: &mut RuntimeGates,
) -> Option<Result<EvalOutcome, Vec<Diagnostic>>> {
    let ExprKind::FieldAccess { receiver, field } = &callee.kind else {
        return None;
    };
    if !matches!(field.as_str(), "get" | "text" | "int" | "float" | "bool") {
        return None;
    }
    Some(eval_row_method(
        program, task, expr, receiver, field, args, locals, gates,
    ))
}

#[allow(clippy::too_many_arguments)]
fn eval_row_method(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    receiver: &Expr,
    field: &str,
    args: &[Expr],
    locals: &HashMap<String, Value>,
    gates: &mut RuntimeGates,
) -> Result<EvalOutcome, Vec<Diagnostic>> {
    if args.len() != 1 {
        return Err(vec![
            Diagnostic::error(
                "RUNTIME_ROW_ARITY_MISMATCH",
                format!(
                    "row method `{field}` expects 1 argument but received {}",
                    args.len()
                ),
            )
            .with_node(expr.id.clone()),
        ]);
    }
    let key = arg_or_propagate!(eval_text_arg(program, task, expr, args, 0, locals, gates));
    let value = match value_or_propagate!(eval_expr(program, task, receiver, locals, gates)) {
        Value::Record(fields) | Value::Map(fields) => {
            fields.get(&key).cloned().ok_or_else(|| {
                vec![
                    Diagnostic::error(
                        "RUNTIME_ROW_FIELD_NOT_FOUND",
                        format!("row field `{key}` was not found"),
                    )
                    .with_node(args[0].id.clone()),
                ]
            })?
        }
        _ => {
            return runtime_type_error(
                expr,
                format!("row method `{field}` expects a record or map receiver"),
            )
            .map(EvalOutcome::value);
        }
    };

    match (field, value) {
        ("get", value) => Ok(EvalOutcome::value(value)),
        ("text", Value::Text(value)) => Ok(EvalOutcome::value(Value::Text(value))),
        ("int", Value::Int(value)) => Ok(EvalOutcome::value(Value::Int(value))),
        ("float", Value::Float(value)) => Ok(EvalOutcome::value(Value::Float(value))),
        ("float", Value::Int(value)) => Ok(EvalOutcome::value(Value::Float(value as f64))),
        ("bool", Value::Bool(value)) => Ok(EvalOutcome::value(Value::Bool(value))),
        (method, value) => runtime_type_error(
            expr,
            format!(
                "row method `{method}` cannot read `{key}` as requested from `{}`",
                value_kind(&value)
            ),
        )
        .map(EvalOutcome::value),
    }
}

#[allow(clippy::too_many_arguments)]
fn eval_db_query(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    callee_name: &str,
    args: &[Expr],
    locals: &HashMap<String, Value>,
    gates: &mut RuntimeGates,
    mode: DbQueryMode,
) -> Result<EvalOutcome, Vec<Diagnostic>> {
    if args.is_empty() {
        return host_arity_error(expr, callee_name, 1, args.len()).map(EvalOutcome::value);
    }

    let query = arg_or_propagate!(eval_text_arg(program, task, expr, args, 0, locals, gates));
    let parsed = match parse_db_query(&query, expr) {
        Ok(parsed) => parsed,
        Err(diagnostics) if mode.returns_result() => {
            return Ok(diagnostics_to_host_err(diagnostics));
        }
        Err(diagnostics) => return Err(diagnostics),
    };
    let expected_args = if parsed.filter.is_some() { 2 } else { 1 };
    if args.len() != expected_args {
        return host_arity_error(expr, callee_name, expected_args, args.len())
            .map(EvalOutcome::value);
    }

    let filter_value = if parsed.filter.is_some() {
        Some(value_or_propagate!(eval_expr(
            program, task, &args[1], locals, gates
        )))
    } else {
        None
    };
    let rows = match gates.db_rows(&parsed.table) {
        Some(rows) => rows,
        None if mode.returns_result() => {
            return Ok(host_err(
                "RUNTIME_DB_TABLE_NOT_FOUND",
                format!("database table `{}` was not seeded", parsed.table),
            ));
        }
        None => {
            return Err(vec![
                Diagnostic::error(
                    "RUNTIME_DB_TABLE_NOT_FOUND",
                    format!("database table `{}` was not seeded", parsed.table),
                )
                .with_node(expr.id.clone()),
            ]);
        }
    };
    let filtered = rows
        .iter()
        .filter(|row| {
            parsed
                .filter
                .as_ref()
                .zip(filter_value.as_ref())
                .is_none_or(|(field, value)| {
                    row.get(field)
                        .is_some_and(|row_value| values_equal(row_value, value))
                })
        })
        .cloned()
        .collect::<Vec<_>>();

    match mode {
        DbQueryMode::RawOne => filtered
            .into_iter()
            .next()
            .map(Value::Record)
            .map(EvalOutcome::value)
            .ok_or_else(|| {
                vec![
                    Diagnostic::error(
                        "RUNTIME_DB_ROW_NOT_FOUND",
                        format!("database query `{query}` returned no rows"),
                    )
                    .with_node(expr.id.clone()),
                ]
            }),
        DbQueryMode::RawMany => Ok(EvalOutcome::value(Value::List(
            filtered.into_iter().map(Value::Record).collect(),
        ))),
        DbQueryMode::ResultOne => match filtered.into_iter().next() {
            Some(row) => Ok(host_ok(Value::Record(row))),
            None => Ok(host_err(
                "RUNTIME_DB_ROW_NOT_FOUND",
                format!("database query `{query}` returned no rows"),
            )),
        },
        DbQueryMode::ResultMany => Ok(host_ok(Value::List(
            filtered.into_iter().map(Value::Record).collect(),
        ))),
    }
}

#[allow(clippy::too_many_arguments)]
fn eval_db_insert(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    callee_name: &str,
    args: &[Expr],
    locals: &HashMap<String, Value>,
    gates: &mut RuntimeGates,
) -> Result<EvalOutcome, Vec<Diagnostic>> {
    if args.len() != 2 {
        return host_arity_error(expr, callee_name, 2, args.len()).map(EvalOutcome::value);
    }

    let table = arg_or_propagate!(eval_text_arg(program, task, expr, args, 0, locals, gates));
    let table = normalize_db_table(table.as_str());
    if table.is_empty() {
        return Ok(host_err(
            "RUNTIME_DB_TABLE_INVALID",
            "database table name cannot be empty",
        ));
    }

    let row = value_or_propagate!(eval_expr(program, task, &args[1], locals, gates));
    let row = match row {
        Value::Record(fields) | Value::Map(fields) => fields,
        _ => {
            return runtime_type_error(
                &args[1],
                "database insert row must be a record or map value",
            )
            .map(EvalOutcome::value);
        }
    };

    gates.insert_db_row(table, row.clone());
    Ok(host_ok(Value::Record(row)))
}

#[allow(clippy::too_many_arguments)]
fn eval_http_get_text(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    callee_name: &str,
    args: &[Expr],
    locals: &HashMap<String, Value>,
    gates: &mut RuntimeGates,
) -> Result<EvalOutcome, Vec<Diagnostic>> {
    if args.len() != 1 {
        return host_arity_error(expr, callee_name, 1, args.len()).map(EvalOutcome::value);
    }

    let url = arg_or_propagate!(eval_text_arg(program, task, expr, args, 0, locals, gates));
    if url.trim().is_empty() {
        return Ok(host_err(
            "RUNTIME_HTTP_URL_INVALID",
            "http URL cannot be empty",
        ));
    }

    match gates.http_text(&url) {
        Some(body) => Ok(host_ok(Value::Text(body.to_string()))),
        None => Ok(host_err(
            "RUNTIME_HTTP_RESPONSE_NOT_FOUND",
            format!("HTTP text response for `{url}` was not seeded"),
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn eval_secret_get(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    callee_name: &str,
    args: &[Expr],
    locals: &HashMap<String, Value>,
    gates: &mut RuntimeGates,
) -> Result<EvalOutcome, Vec<Diagnostic>> {
    if args.len() != 1 {
        return host_arity_error(expr, callee_name, 1, args.len()).map(EvalOutcome::value);
    }

    let name = arg_or_propagate!(eval_text_arg(program, task, expr, args, 0, locals, gates));
    if name.trim().is_empty() {
        return Ok(host_err(
            "RUNTIME_SECRET_NAME_INVALID",
            "secret name cannot be empty",
        ));
    }

    match gates.secret(&name) {
        Some(value) => Ok(host_ok(Value::Text(value.to_string()))),
        None => Ok(host_err(
            "RUNTIME_SECRET_NOT_FOUND",
            format!("secret `{name}` was not seeded"),
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn eval_deploy_stage(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    callee_name: &str,
    args: &[Expr],
    locals: &HashMap<String, Value>,
    gates: &mut RuntimeGates,
) -> Result<EvalOutcome, Vec<Diagnostic>> {
    if args.len() != 1 {
        return host_arity_error(expr, callee_name, 1, args.len()).map(EvalOutcome::value);
    }

    let target = arg_or_propagate!(eval_text_arg(program, task, expr, args, 0, locals, gates));
    if target.trim().is_empty() {
        return Ok(host_err(
            "RUNTIME_DEPLOY_TARGET_INVALID",
            "deploy target cannot be empty",
        ));
    }

    match gates.deploy_result(&target) {
        Some(result) => Ok(host_ok(Value::Text(result.to_string()))),
        None => Ok(host_err(
            "RUNTIME_DEPLOY_RESULT_NOT_FOUND",
            format!("deploy result for target `{target}` was not seeded"),
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn eval_spend_authorize(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    callee_name: &str,
    args: &[Expr],
    locals: &HashMap<String, Value>,
    gates: &mut RuntimeGates,
) -> Result<EvalOutcome, Vec<Diagnostic>> {
    if args.len() != 1 {
        return host_arity_error(expr, callee_name, 1, args.len()).map(EvalOutcome::value);
    }

    let request = arg_or_propagate!(eval_text_arg(program, task, expr, args, 0, locals, gates));
    if request.trim().is_empty() {
        return Ok(host_err(
            "RUNTIME_SPEND_REQUEST_INVALID",
            "spend request cannot be empty",
        ));
    }

    match gates.spend_result(&request) {
        Some(result) => Ok(host_ok(Value::Text(result.to_string()))),
        None => Ok(host_err(
            "RUNTIME_SPEND_RESULT_NOT_FOUND",
            format!("spend result for request `{request}` was not seeded"),
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn eval_shell_run(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    callee_name: &str,
    args: &[Expr],
    locals: &HashMap<String, Value>,
    gates: &mut RuntimeGates,
) -> Result<EvalOutcome, Vec<Diagnostic>> {
    if args.len() != 1 {
        return host_arity_error(expr, callee_name, 1, args.len()).map(EvalOutcome::value);
    }

    let command = arg_or_propagate!(eval_text_arg(program, task, expr, args, 0, locals, gates));
    if command.trim().is_empty() {
        return Ok(host_err(
            "RUNTIME_SHELL_COMMAND_INVALID",
            "shell command cannot be empty",
        ));
    }

    match gates.shell_output(&command) {
        Some(output) => Ok(host_ok(Value::Text(output.to_string()))),
        None => Ok(host_err(
            "RUNTIME_SHELL_OUTPUT_NOT_FOUND",
            format!("shell output for `{command}` was not seeded"),
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn eval_model_complete(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    callee_name: &str,
    args: &[Expr],
    locals: &HashMap<String, Value>,
    gates: &mut RuntimeGates,
) -> Result<EvalOutcome, Vec<Diagnostic>> {
    if args.len() != 1 {
        return host_arity_error(expr, callee_name, 1, args.len()).map(EvalOutcome::value);
    }

    let prompt = arg_or_propagate!(eval_text_arg(program, task, expr, args, 0, locals, gates));
    if prompt.trim().is_empty() {
        return Ok(host_err(
            "RUNTIME_MODEL_PROMPT_INVALID",
            "model prompt cannot be empty",
        ));
    }

    match gates.model_output(&prompt) {
        Some(output) => Ok(host_ok(Value::Text(output.to_string()))),
        None => Ok(host_err(
            "RUNTIME_MODEL_OUTPUT_NOT_FOUND",
            format!("model output for prompt `{prompt}` was not seeded"),
        )),
    }
}

#[derive(Debug, Clone, Copy)]
enum DbQueryMode {
    RawOne,
    RawMany,
    ResultOne,
    ResultMany,
}

impl DbQueryMode {
    fn returns_result(self) -> bool {
        matches!(self, Self::ResultOne | Self::ResultMany)
    }
}

#[derive(Debug)]
struct ParsedDbQuery {
    table: String,
    filter: Option<String>,
}

fn parse_db_query(query: &str, expr: &Expr) -> Result<ParsedDbQuery, Vec<Diagnostic>> {
    let query = query.trim().trim_end_matches(';').trim();
    let lowercase = query.to_ascii_lowercase();
    let table = if let Some(index) = lowercase.find(" from ") {
        query[index + " from ".len()..]
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim_matches(|ch: char| ch == '"' || ch == '`' || ch == '\'' || ch == ';')
            .to_string()
    } else {
        query.to_string()
    };
    if table.is_empty() {
        return Err(vec![
            Diagnostic::error(
                "RUNTIME_DB_QUERY_UNSUPPORTED",
                format!("database query `{query}` does not name a table"),
            )
            .with_node(expr.id.clone()),
        ]);
    }

    let filter = if let Some(index) = lowercase.find(" where ") {
        let condition = query[index + " where ".len()..].trim();
        let Some((field, rhs)) = condition.split_once('=') else {
            return Err(vec![
                Diagnostic::error(
                    "RUNTIME_DB_QUERY_UNSUPPORTED",
                    format!("database query `{query}` only supports `where field = ?` filters"),
                )
                .with_node(expr.id.clone()),
            ]);
        };
        if rhs.trim() != "?" {
            return Err(vec![
                Diagnostic::error(
                    "RUNTIME_DB_QUERY_UNSUPPORTED",
                    format!("database query `{query}` only supports placeholder filters"),
                )
                .with_node(expr.id.clone()),
            ]);
        }
        Some(
            field
                .trim()
                .trim_matches(|ch: char| ch == '"' || ch == '`' || ch == '\'')
                .to_string(),
        )
    } else {
        None
    };

    Ok(ParsedDbQuery { table, filter })
}

fn normalize_db_table(table: impl Into<String>) -> String {
    table.into().trim().to_ascii_lowercase()
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Unit => "Unit",
        Value::Text(_) => "Text",
        Value::Int(_) => "Int",
        Value::Float(_) => "Float",
        Value::Bool(_) => "Bool",
        Value::Gate(_) => "Gate",
        Value::List(_) => "List",
        Value::Map(_) => "Map",
        Value::Record(_) => "Record",
        Value::Ok(_) => "Ok",
        Value::Err(_) => "Err",
        Value::Raw(_) => "Raw",
    }
}

fn host_ok(value: Value) -> EvalOutcome {
    EvalOutcome::value(Value::Ok(Box::new(value)))
}

fn host_err(code: &str, message: impl Into<String>) -> EvalOutcome {
    EvalOutcome::value(Value::Err(Box::new(host_error_value(code, message))))
}

fn diagnostics_to_host_err(diagnostics: Vec<Diagnostic>) -> EvalOutcome {
    let Some(diagnostic) = diagnostics.into_iter().next() else {
        return host_err(
            "RUNTIME_HOST_ERROR",
            "host adapter failed without a diagnostic",
        );
    };
    host_err(&diagnostic.id, diagnostic.message)
}

fn host_error_value(code: &str, message: impl Into<String>) -> Value {
    let mut fields = BTreeMap::new();
    fields.insert("code".to_string(), Value::Text(code.to_string()));
    fields.insert("message".to_string(), Value::Text(message.into()));
    Value::Record(fields)
}

fn eval_text_arg(
    program: &Program,
    task: &TaskDecl,
    expr: &Expr,
    args: &[Expr],
    index: usize,
    locals: &HashMap<String, Value>,
    gates: &mut RuntimeGates,
) -> Result<Result<String, Value>, Vec<Diagnostic>> {
    match eval_expr(program, task, &args[index], locals, gates)? {
        EvalOutcome::Value(Value::Text(value)) => Ok(Ok(value)),
        EvalOutcome::Value(_) => Err(vec![
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
        EvalOutcome::Propagate(error) => Ok(Err(error)),
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
