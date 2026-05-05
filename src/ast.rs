use serde::{Deserialize, Serialize};

use crate::diagnostics::SourceSpan;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Program {
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
        let module = self.module.clone().unwrap_or_else(|| "main".to_string());
        for import in &mut self.imports {
            import.id = format!("import:{module}:{}", import.module);
        }
        for ty in &mut self.types {
            ty.id = format!("type:{module}.{}", ty.name);
        }
        for effect in &mut self.effects {
            effect.id = format!("effect:{module}.{}", effect.name);
        }
        for task in &mut self.tasks {
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
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportDecl {
    pub id: String,
    pub module: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TypeDecl {
    pub id: String,
    pub name: String,
    pub value: TypeExpr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EffectDecl {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskDecl {
    pub id: String,
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
