use serde::Serialize;

use crate::hir::{
    BinaryOp, EnumVariant, EnumVariantPayloadPattern, StructField, Type, TypeParamBound, UnaryOp,
};
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
        locals: Vec<Local>,
        entry_block: u32,
        blocks: Vec<BasicBlock>,
    },
    Const {
        name: String,
        ty: Type,
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
    pub local: u32,
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocalKind {
    Param,
    Local,
}

#[derive(Debug, Clone, Serialize)]
pub struct Local {
    pub id: u32,
    pub kind: LocalKind,
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct BasicBlock {
    pub id: u32,
    pub span: Span,
    pub statements: Vec<Statement>,
    pub terminator: Terminator,
}

#[derive(Debug, Clone, Serialize)]
pub struct Statement {
    #[serde(flatten)]
    pub kind: StatementKind,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StatementKind {
    Let {
        local: u32,
        name: String,
        mutable: bool,
        ty: Type,
        initializer: Expr,
    },
    Assign {
        target: Place,
        value: Expr,
    },
    Eval {
        expr: Expr,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct Terminator {
    #[serde(flatten)]
    pub kind: TerminatorKind,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TerminatorKind {
    Goto {
        target: u32,
    },
    Branch {
        condition: Expr,
        then_block: u32,
        else_block: u32,
    },
    Return {
        value: Expr,
    },
    Unreachable,
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
    Local { local: u32, name: String },
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
pub struct MatchExprArm {
    pub pattern: MatchPattern,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard: Option<Expr>,
    pub value: Expr,
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
pub struct StructPatternField {
    pub name: String,
    pub binding: String,
    pub ty: Type,
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
    Local {
        local: u32,
        name: String,
    },
    Const {
        name: String,
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
    StructLiteral {
        name: String,
        fields: Vec<StructLiteralField>,
    },
    ArrayLiteral {
        elements: Vec<Expr>,
    },
    Block {
        statements: Vec<Statement>,
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

#[derive(Debug, Clone, Serialize)]
pub struct StructLiteralField {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}
