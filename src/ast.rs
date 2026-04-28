use serde::Serialize;

use crate::source::Span;

#[derive(Debug, Clone, Serialize)]
pub struct Program {
    pub items: Vec<Item>,
    #[serde(skip)]
    pub source_units: Vec<SourceUnit>,
}

#[derive(Debug, Clone, Default)]
pub struct SourceUnit {
    pub path: String,
    pub module: Option<ModuleDecl>,
    pub imports: Vec<ImportDecl>,
    pub span: Span,
    pub is_entry: bool,
}

#[derive(Debug, Clone)]
pub struct ModuleDecl {
    pub path: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub path: String,
    pub span: Span,
}

impl Program {
    pub fn source_unit_for_path(&self, path: &str) -> Option<&SourceUnit> {
        self.source_units.iter().find(|unit| unit.path == path)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    #[serde(flatten)]
    pub kind: ItemKind,
    #[serde(default, skip_serializing_if = "Visibility::is_private")]
    pub visibility: Visibility,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    #[default]
    Private,
    Public,
}

impl Visibility {
    pub fn is_private(visibility: &Self) -> bool {
        *visibility == Self::Private
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ItemKind {
    Function {
        name: String,
        type_params: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        type_param_bounds: Vec<TypeParamBound>,
        params: Vec<Param>,
        return_type: TypeRef,
        body: Block,
    },
    Const {
        name: String,
        ty: TypeRef,
        value: Expr,
    },
    Struct {
        name: String,
        type_params: Vec<String>,
        fields: Vec<StructField>,
    },
    Enum {
        name: String,
        type_params: Vec<String>,
        variants: Vec<EnumVariant>,
    },
    Trait {
        name: String,
        methods: Vec<TraitMethod>,
    },
    Impl {
        trait_ref: Option<TypeRef>,
        target: TypeRef,
        methods: Vec<ImplMethod>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct Param {
    pub name: String,
    pub ty: TypeRef,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeParamBound {
    pub type_param: String,
    pub trait_ref: TypeRef,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImplMethod {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: TypeRef,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraitMethod {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: TypeRef,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<TypeRef>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeRef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub type_args: Vec<TypeRef>,
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
            type_args: Vec::new(),
            element: None,
            length: None,
            span,
        }
    }

    pub fn named_with_args(name: impl Into<String>, type_args: Vec<TypeRef>, span: Span) -> Self {
        Self {
            name: Some(name.into()),
            type_args,
            element: None,
            length: None,
            span,
        }
    }

    pub fn array(element: TypeRef, length: usize, span: Span) -> Self {
        Self {
            name: None,
            type_args: Vec::new(),
            element: Some(Box::new(element)),
            length: Some(length),
            span,
        }
    }

    pub fn slice(element: TypeRef, span: Span) -> Self {
        Self {
            name: None,
            type_args: Vec::new(),
            element: Some(Box::new(element)),
            length: None,
            span,
        }
    }

    pub fn direct_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn describe(&self) -> String {
        match (&self.name, &self.type_args[..], &self.element, self.length) {
            (Some(name), [], None, None) => name.clone(),
            (Some(name), args, None, None) => {
                let args = args
                    .iter()
                    .map(TypeRef::describe)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{name}<{args}>")
            }
            (None, [], Some(element), None) => format!("[{}]", element.describe()),
            (None, [], Some(element), Some(length)) => {
                format!("[{}; {}]", element.describe(), length)
            }
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
pub struct MatchArm {
    pub pattern: MatchPattern,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard: Option<Expr>,
    pub body: Block,
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
pub struct ForInBinding {
    pub mutable: bool,
    pub name: String,
    pub ty: TypeRef,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EnumVariantPayloadPattern {
    Wildcard,
    Binding { name: String },
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
        value: i64,
    },
    String {
        value: String,
    },
    EnumVariant {
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<EnumVariantPayloadPattern>,
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
    Continue,
    Match {
        scrutinee: Expr,
        arms: Vec<MatchArm>,
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
    For {
        initializer: Option<Box<Stmt>>,
        condition: Option<Expr>,
        step: Option<Box<Stmt>>,
        body: Block,
    },
    ForIn {
        binding: ForInBinding,
        iterable: Expr,
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

impl Expr {
    pub fn qualified_name(&self) -> Option<String> {
        match &self.kind {
            ExprKind::Name { value } => Some(value.clone()),
            ExprKind::Field { base, field } => base.qualified_name().map(|base| {
                let mut path = base;
                path.push('.');
                path.push_str(field);
                path
            }),
            _ => None,
        }
    }
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
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchExprArm>,
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
    LogicalOr,
    LogicalAnd,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}
