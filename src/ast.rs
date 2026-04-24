use serde::Serialize;

use crate::source::Span;

#[derive(Debug, Clone, Serialize)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    #[serde(flatten)]
    pub kind: ItemKind,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ItemKind {
    Function {
        name: String,
        params: Vec<Param>,
        return_type: TypeRef,
        body: Block,
    },
    Struct {
        name: String,
        fields: Vec<StructField>,
    },
    Enum {
        name: String,
        variants: Vec<EnumVariant>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct Param {
    pub name: String,
    pub ty: TypeRef,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct StructField {
    pub name: String,
    pub ty: TypeRef,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct StructLiteralField {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnumVariant {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeRef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element: Option<Box<TypeRef>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<usize>,
    pub span: Span,
}

impl TypeRef {
    pub fn named(name: impl Into<String>, span: Span) -> Self {
        Self {
            name: Some(name.into()),
            element: None,
            length: None,
            span,
        }
    }

    pub fn array(element: TypeRef, length: usize, span: Span) -> Self {
        Self {
            name: None,
            element: Some(Box::new(element)),
            length: Some(length),
            span,
        }
    }

    pub fn slice(element: TypeRef, span: Span) -> Self {
        Self {
            name: None,
            element: Some(Box::new(element)),
            length: None,
            span,
        }
    }

    pub fn direct_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn describe(&self) -> String {
        match (&self.name, &self.element, self.length) {
            (Some(name), None, None) => name.clone(),
            (None, Some(element), None) => format!("[{}]", element.describe()),
            (None, Some(element), Some(length)) => format!("[{}; {}]", element.describe(), length),
            _ => "<invalid-type>".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stmt {
    #[serde(flatten)]
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StmtKind {
    Let {
        mutable: bool,
        name: String,
        ty: TypeRef,
        initializer: Expr,
    },
    Assign {
        target: Expr,
        value: Expr,
    },
    Expr {
        expr: Expr,
    },
    Return {
        value: Option<Expr>,
    },
    Break,
    If {
        condition: Expr,
        then_branch: Block,
        else_branch: Option<Block>,
    },
    While {
        condition: Expr,
        body: Block,
    },
    For {
        initializer: Option<Box<Stmt>>,
        condition: Option<Expr>,
        step: Option<Box<Stmt>>,
        body: Block,
    },
    Block {
        block: Block,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct Expr {
    #[serde(flatten)]
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExprKind {
    Int {
        value: i64,
    },
    Float {
        value: f64,
    },
    Bool {
        value: bool,
    },
    String {
        value: String,
    },
    Name {
        value: String,
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
    Call {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
    },
    StructLiteral {
        name: String,
        fields: Vec<StructLiteralField>,
    },
    ArrayLiteral {
        elements: Vec<Expr>,
    },
    Field {
        base: Box<Expr>,
        field: String,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    Slice {
        base: Box<Expr>,
        start: Box<Expr>,
        end: Box<Expr>,
    },
    Error,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnaryOp {
    Negate,
    Not,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}
