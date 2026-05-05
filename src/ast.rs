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
    pub functions: Vec<FunctionDecl>,
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
            functions: Vec::new(),
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
        for function in &mut self.functions {
            function.id = format!("function:{module}.{}", function.name);
            for (param_index, param) in function.params.iter_mut().enumerate() {
                param.id = format!("param:{}:{param_index}:{}", function.id, param.name);
            }
            for (stmt_index, stmt) in function.body.statements.iter_mut().enumerate() {
                stmt.id = format!("stmt:{}:{stmt_index}", function.id);
                stmt.expr_mut()
                    .assign_ids(format!("expr:{}:{stmt_index}", function.id));
            }
        }
    }

    pub fn find_function_index(&self, target: &str) -> Option<usize> {
        let needle = target.strip_prefix("function:").unwrap_or(target);
        self.functions.iter().position(|function| {
            function.id == target
                || function.id == format!("function:{needle}")
                || function.name == needle
                || function.id.ends_with(&format!(".{needle}"))
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
pub struct FunctionDecl {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub params: Vec<Param>,
    pub return_type: TypeExpr,
    #[serde(default)]
    pub effects: Vec<String>,
    pub body: Block,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Param {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub ty: TypeExpr,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Block {
    #[serde(default)]
    pub statements: Vec<Statement>,
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
    pub fn expr_mut(&mut self) -> &mut Expr {
        match &mut self.kind {
            StatementKind::Let { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => expr,
        }
    }

    pub fn expr(&self) -> &Expr {
        match &self.kind {
            StatementKind::Let { expr, .. }
            | StatementKind::Return { expr }
            | StatementKind::Expr { expr } => expr,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum StatementKind {
    Let {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        type_ann: Option<TypeExpr>,
        expr: Expr,
    },
    Return {
        expr: Expr,
    },
    Expr {
        expr: Expr,
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
    pub patch_id: String,
    pub actor: String,
    pub timestamp: String,
    pub operation: String,
    pub targets: Vec<String>,
    pub result: String,
}
