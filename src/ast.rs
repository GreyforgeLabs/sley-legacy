use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::diagnostics::SourceSpan;

pub const AST_PROGRAM_SCHEMA: &str = "sley.ast.program.v0";
pub const AST_NODE_SCHEMA: &str = "sley.ast.node.v0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Program {
    #[serde(default = "ast_program_schema")]
    pub schema: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    #[serde(default)]
    pub imports: Vec<ImportDecl>,
    #[serde(default)]
    pub types: Vec<TypeDecl>,
    #[serde(default)]
    pub effects: Vec<EffectDecl>,
    #[serde(default)]
    pub tasks: Vec<TaskDecl>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provenance: Vec<ProvenanceRecord>,
}

impl Program {
    pub fn new() -> Self {
        Self {
            schema: AST_PROGRAM_SCHEMA.to_string(),
            module: None,
            imports: Vec::new(),
            types: Vec::new(),
            effects: Vec::new(),
            tasks: Vec::new(),
            provenance: Vec::new(),
        }
    }

    pub fn module_name(&self) -> &str {
        self.module.as_deref().unwrap_or("main")
    }

    pub fn assign_ids(&mut self) {
        let default_module = self.module.clone().unwrap_or_else(|| "main".to_string());
        for import in &mut self.imports {
            let owner_module = import
                .owner_module
                .clone()
                .unwrap_or_else(|| default_module.clone());
            import.owner_module = Some(owner_module.clone());
            import.id = format!("import:{owner_module}:{}", import.module);
        }
        for ty in &mut self.types {
            let module = ty.module.clone().unwrap_or_else(|| default_module.clone());
            ty.module = Some(module.clone());
            ty.id = format!("type:{module}.{}", ty.name);
        }
        for effect in &mut self.effects {
            let module = effect
                .module
                .clone()
                .unwrap_or_else(|| default_module.clone());
            effect.module = Some(module.clone());
            effect.id = format!("effect:{module}.{}", effect.name);
        }
        for task in &mut self.tasks {
            let module = task
                .module
                .clone()
                .unwrap_or_else(|| default_module.clone());
            task.module = Some(module.clone());
            task.id = format!("task:{module}.{}", task.name);
            for (take_index, take) in task.takes.iter_mut().enumerate() {
                take.id = format!("take:{}:{take_index}:{}", task.id, take.name);
            }
            task.body.assign_ids(format!("block:{}", task.id));
        }
    }

    pub fn find_task_index(&self, target: &str) -> Option<usize> {
        let needle = target.strip_prefix("task:").unwrap_or(target);
        self.tasks.iter().position(|task| {
            task.id == target
                || task.id == format!("task:{needle}")
                || task.name == needle
                || task.id.ends_with(&format!(".{needle}"))
        })
    }

    pub fn ast_node_report(&self, target: &str) -> Option<AstNodeReport> {
        find_ast_node_report(self, target)
    }
}

fn ast_program_schema() -> String {
    AST_PROGRAM_SCHEMA.to_string()
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AstNodeReport {
    pub schema: String,
    pub target: String,
    pub id: String,
    pub node_kind: String,
    pub module: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    pub value: JsonValue,
}

impl AstNodeReport {
    fn new(
        target: &str,
        id: impl Into<String>,
        node_kind: impl Into<String>,
        module: impl Into<String>,
        parent: Option<String>,
        value: JsonValue,
    ) -> Self {
        Self {
            schema: AST_NODE_SCHEMA.to_string(),
            target: target.to_string(),
            id: id.into(),
            node_kind: node_kind.into(),
            module: module.into(),
            parent,
            value,
        }
    }
}

fn find_ast_node_report(program: &Program, target: &str) -> Option<AstNodeReport> {
    if target == "program" {
        return Some(AstNodeReport::new(
            target,
            "program",
            "program",
            program.module_name(),
            None,
            serde_json::to_value(program).ok()?,
        ));
    }
    if let Some(import) = program.imports.iter().find(|import| import.id == target) {
        return Some(AstNodeReport::new(
            target,
            import.id.clone(),
            "import",
            import
                .owner_module
                .as_deref()
                .unwrap_or(program.module_name()),
            Some(format!(
                "module:{}",
                import
                    .owner_module
                    .as_deref()
                    .unwrap_or(program.module_name())
            )),
            serde_json::to_value(import).ok()?,
        ));
    }
    if let Some(ty) = program.types.iter().find(|ty| ty.id == target) {
        return Some(AstNodeReport::new(
            target,
            ty.id.clone(),
            "type",
            ty.module.as_deref().unwrap_or(program.module_name()),
            Some(format!(
                "module:{}",
                ty.module.as_deref().unwrap_or(program.module_name())
            )),
            serde_json::to_value(ty).ok()?,
        ));
    }
    if let Some(effect) = program.effects.iter().find(|effect| effect.id == target) {
        return Some(AstNodeReport::new(
            target,
            effect.id.clone(),
            "effect",
            effect.module.as_deref().unwrap_or(program.module_name()),
            Some(format!(
                "module:{}",
                effect.module.as_deref().unwrap_or(program.module_name())
            )),
            serde_json::to_value(effect).ok()?,
        ));
    }
    for task in &program.tasks {
        if task.id == target {
            return Some(AstNodeReport::new(
                target,
                task.id.clone(),
                "task",
                task.module.as_deref().unwrap_or(program.module_name()),
                Some(format!(
                    "module:{}",
                    task.module.as_deref().unwrap_or(program.module_name())
                )),
                serde_json::to_value(task).ok()?,
            ));
        }
        if let Some(take) = task.takes.iter().find(|take| take.id == target) {
            return Some(AstNodeReport::new(
                target,
                take.id.clone(),
                "take",
                task.module.as_deref().unwrap_or(program.module_name()),
                Some(task.id.clone()),
                serde_json::to_value(take).ok()?,
            ));
        }
        let block_id = format!("block:{}", task.id);
        if let Some(report) = find_ast_node_report_in_block(
            program,
            target,
            task.module.as_deref().unwrap_or(program.module_name()),
            &task.body,
            &block_id,
            Some(&task.id),
        ) {
            return Some(report);
        }
    }
    program.find_task_index(target).and_then(|index| {
        let task = &program.tasks[index];
        Some(AstNodeReport::new(
            target,
            task.id.clone(),
            "task",
            task.module.as_deref().unwrap_or(program.module_name()),
            Some(format!(
                "module:{}",
                task.module.as_deref().unwrap_or(program.module_name())
            )),
            serde_json::to_value(task).ok()?,
        ))
    })
}

fn find_ast_node_report_in_block(
    program: &Program,
    target: &str,
    module: &str,
    block: &Block,
    block_id: &str,
    parent: Option<&str>,
) -> Option<AstNodeReport> {
    if target == block_id {
        return Some(AstNodeReport::new(
            target,
            block_id.to_string(),
            "block",
            module,
            parent.map(str::to_string),
            serde_json::to_value(block).ok()?,
        ));
    }
    for statement in &block.statements {
        if statement.id == target {
            return Some(AstNodeReport::new(
                target,
                statement.id.clone(),
                "statement",
                module,
                Some(block_id.to_string()),
                serde_json::to_value(statement).ok()?,
            ));
        }
        if let Some(report) = find_ast_node_report_in_statement(program, target, module, statement)
        {
            return Some(report);
        }
    }
    None
}

fn find_ast_node_report_in_statement(
    program: &Program,
    target: &str,
    module: &str,
    statement: &Statement,
) -> Option<AstNodeReport> {
    match &statement.kind {
        StatementKind::Binding { expr, .. }
        | StatementKind::Set { expr, .. }
        | StatementKind::Return { expr }
        | StatementKind::Expr { expr } => {
            find_ast_node_report_in_expr(program, target, module, expr, Some(&statement.id))
        }
        StatementKind::If {
            condition,
            then_block,
            else_block,
        } => find_ast_node_report_in_expr(program, target, module, condition, Some(&statement.id))
            .or_else(|| {
                let then_id = format!("{}:then", statement.id);
                find_ast_node_report_in_block(
                    program,
                    target,
                    module,
                    then_block,
                    &then_id,
                    Some(&statement.id),
                )
            })
            .or_else(|| {
                let else_block = else_block.as_ref()?;
                let else_id = format!("{}:else", statement.id);
                find_ast_node_report_in_block(
                    program,
                    target,
                    module,
                    else_block,
                    &else_id,
                    Some(&statement.id),
                )
            }),
        StatementKind::While { condition, body } => {
            find_ast_node_report_in_expr(program, target, module, condition, Some(&statement.id))
                .or_else(|| {
                    let body_id = format!("{}:body", statement.id);
                    find_ast_node_report_in_block(
                        program,
                        target,
                        module,
                        body,
                        &body_id,
                        Some(&statement.id),
                    )
                })
        }
        StatementKind::For {
            collection, body, ..
        } => find_ast_node_report_in_expr(program, target, module, collection, Some(&statement.id))
            .or_else(|| {
                let body_id = format!("{}:body", statement.id);
                find_ast_node_report_in_block(
                    program,
                    target,
                    module,
                    body,
                    &body_id,
                    Some(&statement.id),
                )
            }),
        StatementKind::Forge { body } => {
            let forge_id = format!("{}:forge", statement.id);
            find_ast_node_report_in_block(
                program,
                target,
                module,
                body,
                &forge_id,
                Some(&statement.id),
            )
        }
    }
}

fn find_ast_node_report_in_expr(
    program: &Program,
    target: &str,
    module: &str,
    expr: &Expr,
    parent: Option<&str>,
) -> Option<AstNodeReport> {
    if expr.id == target {
        return Some(AstNodeReport::new(
            target,
            expr.id.clone(),
            "expression",
            module,
            parent.map(str::to_string),
            serde_json::to_value(expr).ok()?,
        ));
    }
    let expr_id = expr.id.as_str();
    match &expr.kind {
        ExprKind::Unary { expr: inner, .. } | ExprKind::Try { expr: inner } => {
            find_ast_node_report_in_expr(program, target, module, inner, Some(expr_id))
        }
        ExprKind::Binary { left, right, .. } => {
            find_ast_node_report_in_expr(program, target, module, left, Some(expr_id)).or_else(
                || find_ast_node_report_in_expr(program, target, module, right, Some(expr_id)),
            )
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => find_ast_node_report_in_expr(program, target, module, condition, Some(expr_id))
            .or_else(|| {
                find_ast_node_report_in_expr(program, target, module, then_branch, Some(expr_id))
            })
            .or_else(|| {
                find_ast_node_report_in_expr(program, target, module, else_branch, Some(expr_id))
            }),
        ExprKind::Call { callee, args } => {
            find_ast_node_report_in_expr(program, target, module, callee, Some(expr_id)).or_else(
                || {
                    args.iter().find_map(|arg| {
                        find_ast_node_report_in_expr(program, target, module, arg, Some(expr_id))
                    })
                },
            )
        }
        ExprKind::ListLiteral { items } => items.iter().find_map(|item| {
            find_ast_node_report_in_expr(program, target, module, item, Some(expr_id))
        }),
        ExprKind::MapLiteral { entries } => entries.iter().find_map(|entry| {
            find_ast_node_report_in_expr(program, target, module, &entry.key, Some(expr_id))
                .or_else(|| {
                    find_ast_node_report_in_expr(
                        program,
                        target,
                        module,
                        &entry.value,
                        Some(expr_id),
                    )
                })
        }),
        ExprKind::Index { collection, index } => {
            find_ast_node_report_in_expr(program, target, module, collection, Some(expr_id))
                .or_else(|| {
                    find_ast_node_report_in_expr(program, target, module, index, Some(expr_id))
                })
        }
        ExprKind::FieldAccess { receiver, .. } => {
            find_ast_node_report_in_expr(program, target, module, receiver, Some(expr_id))
        }
        ExprKind::RecordLiteral { fields, .. } => fields.iter().find_map(|field| {
            find_ast_node_report_in_expr(program, target, module, &field.expr, Some(expr_id))
        }),
        ExprKind::Raw { .. }
        | ExprKind::StringLiteral { .. }
        | ExprKind::IntLiteral { .. }
        | ExprKind::FloatLiteral { .. }
        | ExprKind::BoolLiteral { .. }
        | ExprKind::Identifier { .. } => None,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportDecl {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_module: Option<String>,
    pub module: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TypeDecl {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub exported: bool,
    pub name: String,
    pub value: TypeExpr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EffectDecl {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub exported: bool,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskDecl {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub exported: bool,
    pub name: String,
    #[serde(default)]
    pub takes: Vec<TakeDecl>,
    pub return_type: TypeExpr,
    #[serde(default)]
    pub effects: Vec<String>,
    pub body: Block,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TakeDecl {
    pub id: String,
    pub name: String,
    #[serde(default = "BindingKind::take")]
    pub binding_kind: BindingKind,
    #[serde(rename = "type")]
    pub ty: TypeExpr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BindingKind {
    Take,
    Bind,
    State,
    Cell,
    Knot,
    Slot,
    Gate,
    Lease,
    Veil,
    Dial,
    Flag,
    Memo,
    Cache,
    Derive,
    Flow,
    Port,
    Tally,
    Hole,
    Draft,
    Taint,
    Witness,
    Seal,
    Anchor,
    View,
    Cursor,
}

impl BindingKind {
    pub fn take() -> Self {
        Self::Take
    }

    pub fn from_source_keyword(keyword: &str) -> Option<Self> {
        match keyword {
            "bind" => Some(Self::Bind),
            "state" => Some(Self::State),
            "cell" => Some(Self::Cell),
            "knot" => Some(Self::Knot),
            "slot" => Some(Self::Slot),
            "gate" => Some(Self::Gate),
            "lease" => Some(Self::Lease),
            "veil" => Some(Self::Veil),
            "dial" => Some(Self::Dial),
            "flag" => Some(Self::Flag),
            "memo" => Some(Self::Memo),
            "cache" => Some(Self::Cache),
            "derive" => Some(Self::Derive),
            "flow" => Some(Self::Flow),
            "port" => Some(Self::Port),
            "tally" => Some(Self::Tally),
            "hole" => Some(Self::Hole),
            "draft" => Some(Self::Draft),
            "taint" => Some(Self::Taint),
            "witness" => Some(Self::Witness),
            "seal" => Some(Self::Seal),
            "anchor" => Some(Self::Anchor),
            "view" => Some(Self::View),
            "cursor" => Some(Self::Cursor),
            _ => None,
        }
    }

    pub fn as_source_keyword(&self) -> &'static str {
        match self {
            Self::Take => "take",
            Self::Bind => "bind",
            Self::State => "state",
            Self::Cell => "cell",
            Self::Knot => "knot",
            Self::Slot => "slot",
            Self::Gate => "gate",
            Self::Lease => "lease",
            Self::Veil => "veil",
            Self::Dial => "dial",
            Self::Flag => "flag",
            Self::Memo => "memo",
            Self::Cache => "cache",
            Self::Derive => "derive",
            Self::Flow => "flow",
            Self::Port => "port",
            Self::Tally => "tally",
            Self::Hole => "hole",
            Self::Draft => "draft",
            Self::Taint => "taint",
            Self::Witness => "witness",
            Self::Seal => "seal",
            Self::Anchor => "anchor",
            Self::View => "view",
            Self::Cursor => "cursor",
        }
    }

    pub fn is_mutable_local(&self) -> bool {
        matches!(
            self,
            Self::State
                | Self::Cell
                | Self::Knot
                | Self::Lease
                | Self::Cache
                | Self::Flow
                | Self::Port
                | Self::Tally
                | Self::Draft
                | Self::Cursor
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Block {
    #[serde(default)]
    pub statements: Vec<Statement>,
}

impl Block {
    pub fn assign_ids(&mut self, root: impl Into<String>) {
        let root = root.into();
        for (stmt_index, stmt) in self.statements.iter_mut().enumerate() {
            stmt.assign_ids(format!("{root}:stmt:{stmt_index}"));
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Statement {
    pub id: String,
    #[serde(flatten)]
    pub kind: StatementKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
}

impl Statement {
    pub fn assign_ids(&mut self, id: impl Into<String>) {
        let id = id.into();
        self.id = id.clone();
        match &mut self.kind {
            StatementKind::Binding { expr, .. }
            | StatementKind::Set { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => {
                expr.assign_ids(format!("{id}:expr"));
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                condition.assign_ids(format!("{id}:condition"));
                then_block.assign_ids(format!("{id}:then"));
                if let Some(else_block) = else_block {
                    else_block.assign_ids(format!("{id}:else"));
                }
            }
            StatementKind::While { condition, body } => {
                condition.assign_ids(format!("{id}:condition"));
                body.assign_ids(format!("{id}:body"));
            }
            StatementKind::For {
                collection, body, ..
            } => {
                collection.assign_ids(format!("{id}:collection"));
                body.assign_ids(format!("{id}:body"));
            }
            StatementKind::Forge { body } => {
                body.assign_ids(format!("{id}:forge"));
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum StatementKind {
    Binding {
        binding_kind: BindingKind,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        type_ann: Option<TypeExpr>,
        expr: Expr,
    },
    Set {
        name: String,
        expr: Expr,
    },
    Return {
        expr: Expr,
    },
    Expr {
        expr: Expr,
    },
    If {
        condition: Expr,
        then_block: Block,
        #[serde(skip_serializing_if = "Option::is_none")]
        else_block: Option<Block>,
    },
    While {
        condition: Expr,
        body: Block,
    },
    For {
        item: String,
        collection: Expr,
        body: Block,
    },
    Forge {
        body: Block,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Expr {
    pub id: String,
    pub source: String,
    #[serde(flatten)]
    pub kind: ExprKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
}

impl Expr {
    pub fn assign_ids(&mut self, root: impl Into<String>) {
        self.assign_ids_at(root.into());
    }

    fn assign_ids_at(&mut self, id: String) {
        self.id = id.clone();
        match &mut self.kind {
            ExprKind::Unary { expr, .. } => {
                expr.assign_ids_at(format!("{id}:operand"));
            }
            ExprKind::Binary { left, right, .. } => {
                left.assign_ids_at(format!("{id}:left"));
                right.assign_ids_at(format!("{id}:right"));
            }
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                condition.assign_ids_at(format!("{id}:condition"));
                then_branch.assign_ids_at(format!("{id}:then"));
                else_branch.assign_ids_at(format!("{id}:else"));
            }
            ExprKind::Call { callee, args } => {
                callee.assign_ids_at(format!("{id}:callee"));
                for (index, arg) in args.iter_mut().enumerate() {
                    arg.assign_ids_at(format!("{id}:arg:{index}"));
                }
            }
            ExprKind::ListLiteral { items } => {
                for (index, item) in items.iter_mut().enumerate() {
                    item.assign_ids_at(format!("{id}:item:{index}"));
                }
            }
            ExprKind::MapLiteral { entries } => {
                for (index, entry) in entries.iter_mut().enumerate() {
                    entry.key.assign_ids_at(format!("{id}:entry:{index}:key"));
                    entry
                        .value
                        .assign_ids_at(format!("{id}:entry:{index}:value"));
                }
            }
            ExprKind::Index { collection, index } => {
                collection.assign_ids_at(format!("{id}:collection"));
                index.assign_ids_at(format!("{id}:index"));
            }
            ExprKind::FieldAccess { receiver, .. } => {
                receiver.assign_ids_at(format!("{id}:receiver"));
            }
            ExprKind::RecordLiteral { fields, .. } => {
                for (index, field) in fields.iter_mut().enumerate() {
                    field
                        .expr
                        .assign_ids_at(format!("{id}:field:{index}:{}", field.name));
                }
            }
            ExprKind::Try { expr } => {
                expr.assign_ids_at(format!("{id}:try"));
            }
            ExprKind::Raw { .. }
            | ExprKind::StringLiteral { .. }
            | ExprKind::IntLiteral { .. }
            | ExprKind::FloatLiteral { .. }
            | ExprKind::BoolLiteral { .. }
            | ExprKind::Identifier { .. } => {}
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "expr_kind")]
pub enum ExprKind {
    Raw {
        fallible: bool,
    },
    StringLiteral {
        value: String,
    },
    IntLiteral {
        value: i64,
    },
    FloatLiteral {
        value: f64,
    },
    BoolLiteral {
        value: bool,
    },
    Identifier {
        name: String,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    ListLiteral {
        items: Vec<Expr>,
    },
    MapLiteral {
        entries: Vec<ExprMapEntry>,
    },
    Index {
        collection: Box<Expr>,
        index: Box<Expr>,
    },
    FieldAccess {
        receiver: Box<Expr>,
        field: String,
    },
    RecordLiteral {
        #[serde(skip_serializing_if = "Option::is_none")]
        type_name: Option<String>,
        fields: Vec<ExprField>,
    },
    Try {
        expr: Box<Expr>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExprField {
    pub name: String,
    pub expr: Expr,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExprMapEntry {
    pub key: Expr,
    pub value: Expr,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Negate,
}

impl UnaryOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            UnaryOp::Not => "!",
            UnaryOp::Negate => "-",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BinaryOp {
    Or,
    And,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

impl BinaryOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            BinaryOp::Or => "||",
            BinaryOp::And => "&&",
            BinaryOp::Equal => "==",
            BinaryOp::NotEqual => "!=",
            BinaryOp::Less => "<",
            BinaryOp::LessEqual => "<=",
            BinaryOp::Greater => ">",
            BinaryOp::GreaterEqual => ">=",
            BinaryOp::Add => "+",
            BinaryOp::Subtract => "-",
            BinaryOp::Multiply => "*",
            BinaryOp::Divide => "/",
            BinaryOp::Remainder => "%",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "kind")]
pub enum TypeExpr {
    Named { name: String },
    Generic { name: String, args: Vec<TypeExpr> },
    Record { fields: Vec<RecordField> },
}

impl TypeExpr {
    pub fn named(name: impl Into<String>) -> Self {
        Self::Named { name: name.into() }
    }

    pub fn display(&self) -> String {
        match self {
            TypeExpr::Named { name } => name.clone(),
            TypeExpr::Generic { name, args } => {
                let args = args
                    .iter()
                    .map(TypeExpr::display)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{name}<{args}>")
            }
            TypeExpr::Record { fields } => {
                let fields = fields
                    .iter()
                    .map(|field| format!("{}: {}", field.name, field.ty.display()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{ {fields} }}")
            }
        }
    }

    pub fn generic_name(&self) -> Option<&str> {
        match self {
            TypeExpr::Generic { name, .. } => Some(name),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RecordField {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: TypeExpr,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProvenanceRecord {
    pub graft_id: String,
    pub actor: String,
    pub timestamp: String,
    pub operation: String,
    pub targets: Vec<String>,
    pub result: String,
}

fn is_false(value: &bool) -> bool {
    !*value
}
