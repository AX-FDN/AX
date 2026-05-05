use serde::Serialize;

use crate::ast::{BinaryOp, UnaryOp};
use crate::source::Span;

#[derive(Debug, Clone, Serialize)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    #[serde(flatten)]
    pub kind: ItemKind,
    #[serde(default, skip_serializing_if = "crate::ast::Visibility::is_private")]
    pub visibility: crate::ast::Visibility,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ItemKind {
    Function {
        name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        type_params: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        type_param_bounds: Vec<TypeParamBound>,
        params: Vec<Param>,
        return_type: Type,
        body: Block,
    },
    Const {
        name: String,
        ty: Type,
        value: Expr,
    },
    Struct {
        name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        type_params: Vec<String>,
        fields: Vec<StructField>,
    },
    Enum {
        name: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        type_params: Vec<String>,
        variants: Vec<EnumVariant>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeParamBound {
    pub type_param: String,
    pub trait_name: String,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Type>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Type {
    Bool,
    I32,
    F32,
    String,
    Bytes,
    StringList,
    Slice { element: Box<Type> },
    Array { element: Box<Type>, length: usize },
    Struct { name: String },
    StructInstance { name: String, args: Vec<Type> },
    Enum { name: String },
    EnumInstance { name: String, args: Vec<Type> },
    TypeParam { name: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatchExprArm {
    pub pattern: MatchPattern,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard: Option<Expr>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EnumVariantPayloadPattern {
    Wildcard,
    Binding { name: String },
}

#[derive(Debug, Clone, Serialize)]
pub struct StructPatternField {
    pub name: String,
    pub binding: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatchPattern {
    #[serde(flatten)]
    pub kind: MatchPatternKind,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MatchPatternKind {
    Wildcard,
    Binding {
        name: String,
    },
    Bool {
        value: bool,
    },
    Int {
        value: i32,
    },
    IntRange {
        start: i32,
        end: i32,
    },
    String {
        value: String,
    },
    EnumVariant {
        enum_name: String,
        variant: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<EnumVariantPayloadPattern>,
        #[serde(skip_serializing_if = "Option::is_none")]
        payload_type: Option<Type>,
    },
    Struct {
        struct_name: String,
        fields: Vec<StructPatternField>,
    },
    Or {
        alternatives: Vec<MatchPattern>,
    },
    Error,
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
        ty: Type,
        initializer: Expr,
    },
    Assign {
        target: Place,
        value: Expr,
    },
    Break,
    Continue,
    Expr {
        expr: Expr,
    },
    Return {
        value: Expr,
    },
    If {
        condition: Expr,
        then_branch: Block,
        else_branch: Option<Block>,
    },
    While {
        condition: Expr,
        body: Block,
    },
    Block {
        block: Block,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct Place {
    #[serde(flatten)]
    pub kind: PlaceKind,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlaceKind {
    Local { name: String },
    Field { base: Box<Place>, field: String },
    Index { base: Box<Place>, index: Expr },
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
        value: i32,
    },
    Float {
        value: f32,
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
    Try {
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        function: String,
        arguments: Vec<Expr>,
    },
    MethodCall {
        receiver: Box<Expr>,
        method: String,
        arguments: Vec<Expr>,
    },
    StructLiteral {
        name: String,
        fields: Vec<StructLiteralField>,
    },
    ArrayLiteral {
        elements: Vec<Expr>,
    },
    Block {
        statements: Vec<Stmt>,
        value: Box<Expr>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchExprArm>,
    },
    EnumVariant {
        enum_name: String,
        variant: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<Box<Expr>>,
    },
    MatchTest {
        scrutinee: Box<Expr>,
        pattern: MatchPattern,
    },
    EnumPayload {
        value: Box<Expr>,
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
}
