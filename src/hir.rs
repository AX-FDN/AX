use std::cell::Cell;
use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::ast::{self};
pub use crate::ast::{BinaryOp, UnaryOp};
use crate::diagnostics::Diagnostic;
use crate::source::{SourceFile, Span};

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
        type_params: Vec<String>,
        params: Vec<Param>,
        return_type: Type,
        body: Block,
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
}

#[derive(Debug, Clone, Serialize)]
pub struct Param {
    pub name: String,
    pub ty: Type,
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

pub fn lower_program(source: &SourceFile, program: &ast::Program) -> Result<Program, Diagnostic> {
    LoweringContext::new(source, program).lower_program(program)
}

struct LoweringContext<'a> {
    source: &'a SourceFile,
    unit_modules: HashMap<String, String>,
    function_names: HashSet<String>,
    struct_names: HashSet<String>,
    enum_names: HashSet<String>,
    enum_variant_payloads: HashMap<String, HashMap<String, Type>>,
    next_match_temp: Cell<u32>,
    next_for_in_temp: Cell<u32>,
}

impl<'a> LoweringContext<'a> {
    fn new(source: &'a SourceFile, program: &ast::Program) -> Self {
        let unit_modules = program
            .source_units
            .iter()
            .filter_map(|unit| {
                unit.module
                    .as_ref()
                    .map(|module| (unit.path.clone(), module.path.clone()))
            })
            .collect::<HashMap<_, _>>();
        let mut function_names = HashSet::new();
        let mut struct_names = HashSet::new();
        let mut enum_names = HashSet::new();

        for item in &program.items {
            let canonical_name = canonical_item_name(source, &unit_modules, item);
            match &item.kind {
                ast::ItemKind::Function { .. } => {
                    function_names.insert(canonical_name);
                }
                ast::ItemKind::Struct { .. } => {
                    struct_names.insert(canonical_name);
                }
                ast::ItemKind::Enum { .. } => {
                    enum_names.insert(canonical_name);
                }
                ast::ItemKind::Trait { .. } => {}
                ast::ItemKind::Impl { .. } => {}
            }
        }

        let mut context = Self {
            source,
            unit_modules,
            function_names,
            struct_names,
            enum_names,
            enum_variant_payloads: HashMap::new(),
            next_match_temp: Cell::new(0),
            next_for_in_temp: Cell::new(0),
        };
        context.enum_variant_payloads = context.collect_enum_variant_payloads(program);
        context
    }

    fn collect_enum_variant_payloads(
        &self,
        program: &ast::Program,
    ) -> HashMap<String, HashMap<String, Type>> {
        let mut payloads = HashMap::new();

        for item in &program.items {
            let ast::ItemKind::Enum { variants, .. } = &item.kind else {
                continue;
            };

            let enum_name = canonical_item_name(self.source, &self.unit_modules, item);
            let mut variant_payloads = HashMap::new();
            for variant in variants {
                if let Some(payload) = &variant.payload
                    && let Ok(payload_type) = self.lower_type_ref(payload)
                {
                    variant_payloads.insert(variant.name.clone(), payload_type);
                }
            }
            payloads.insert(enum_name, variant_payloads);
        }

        payloads
    }

    fn lower_program(&self, program: &ast::Program) -> Result<Program, Diagnostic> {
        let mut items = Vec::new();
        for item in &program.items {
            items.extend(self.lower_items(item)?);
        }
        Ok(Program { items })
    }

    fn lower_items(&self, item: &ast::Item) -> Result<Vec<Item>, Diagnostic> {
        let kind = match &item.kind {
            ast::ItemKind::Function {
                name,
                type_params,
                params,
                return_type,
                body,
            } => ItemKind::Function {
                name: self.canonical_name(name, item.span),
                type_params: type_params.clone(),
                params: params
                    .iter()
                    .map(|param| {
                        Ok(Param {
                            name: param.name.clone(),
                            ty: self.lower_type_ref(&param.ty)?,
                            span: param.span,
                        })
                    })
                    .collect::<Result<Vec<_>, Diagnostic>>()?,
                return_type: self.lower_type_ref(return_type)?,
                body: self.lower_block(body)?,
            },
            ast::ItemKind::Struct {
                name,
                type_params,
                fields,
            } => ItemKind::Struct {
                name: self.canonical_name(name, item.span),
                type_params: type_params.clone(),
                fields: fields
                    .iter()
                    .map(|field| {
                        Ok(StructField {
                            name: field.name.clone(),
                            ty: self.lower_type_ref(&field.ty)?,
                            span: field.span,
                        })
                    })
                    .collect::<Result<Vec<_>, Diagnostic>>()?,
            },
            ast::ItemKind::Enum {
                name,
                type_params,
                variants,
            } => ItemKind::Enum {
                name: self.canonical_name(name, item.span),
                type_params: type_params.clone(),
                variants: variants
                    .iter()
                    .map(|variant| {
                        Ok(EnumVariant {
                            name: variant.name.clone(),
                            payload: variant
                                .payload
                                .as_ref()
                                .map(|payload| self.lower_type_ref(payload))
                                .transpose()?,
                            span: variant.span,
                        })
                    })
                    .collect::<Result<Vec<_>, Diagnostic>>()?,
            },
            ast::ItemKind::Trait { .. } => return Ok(Vec::new()),
            ast::ItemKind::Impl {
                target, methods, ..
            } => {
                let method_prefix = self.impl_method_prefix(target, item.span)?;
                return methods
                    .iter()
                    .map(|method| {
                        Ok(Item {
                            kind: ItemKind::Function {
                                name: format!("{method_prefix}.{}", method.name),
                                type_params: Vec::new(),
                                params: method
                                    .params
                                    .iter()
                                    .map(|param| {
                                        Ok(Param {
                                            name: param.name.clone(),
                                            ty: self.lower_type_ref(&param.ty)?,
                                            span: param.span,
                                        })
                                    })
                                    .collect::<Result<Vec<_>, Diagnostic>>()?,
                                return_type: self.lower_type_ref(&method.return_type)?,
                                body: self.lower_block(&method.body)?,
                            },
                            span: method.span,
                        })
                    })
                    .collect();
            }
        };

        Ok(vec![Item {
            kind,
            span: item.span,
        }])
    }

    fn lower_type_ref(&self, ty: &ast::TypeRef) -> Result<Type, Diagnostic> {
        match (&ty.name, &ty.type_args[..], &ty.element, ty.length) {
            (Some(name), [], None, None) => match name.as_str() {
                "bool" => Ok(Type::Bool),
                "i32" => Ok(Type::I32),
                "f32" => Ok(Type::F32),
                "string" => Ok(Type::String),
                "string_list" => Ok(Type::StringList),
                _ => {
                    if let Some(name) =
                        self.resolve_canonical_name(name, ty.span, &self.struct_names)
                    {
                        return Ok(Type::Struct { name });
                    }
                    if let Some(name) = self.resolve_canonical_name(name, ty.span, &self.enum_names)
                    {
                        return Ok(Type::Enum { name });
                    }
                    if looks_like_type_param(name) {
                        return Ok(Type::TypeParam { name: name.clone() });
                    }
                    Err(self.lowering_error(
                        "H0001",
                        format!("cannot lower unknown type `{}` into HIR", name),
                        ty.span,
                    ))
                }
            },
            (Some(name), args, None, None) => {
                if let Some(name) = self.resolve_canonical_name(name, ty.span, &self.struct_names) {
                    return Ok(Type::StructInstance {
                        name,
                        args: args
                            .iter()
                            .map(|arg| self.lower_type_ref(arg))
                            .collect::<Result<Vec<_>, _>>()?,
                    });
                }
                if let Some(name) = self.resolve_canonical_name(name, ty.span, &self.enum_names) {
                    return Ok(Type::EnumInstance {
                        name,
                        args: args
                            .iter()
                            .map(|arg| self.lower_type_ref(arg))
                            .collect::<Result<Vec<_>, _>>()?,
                    });
                }
                Err(self.lowering_error(
                    "H0001",
                    format!("cannot lower unknown generic type `{}` into HIR", name),
                    ty.span,
                ))
            }
            (None, [], Some(element), None) => Ok(Type::Slice {
                element: Box::new(self.lower_type_ref(element)?),
            }),
            (None, [], Some(element), Some(length)) => Ok(Type::Array {
                element: Box::new(self.lower_type_ref(element)?),
                length,
            }),
            _ => Err(self.lowering_error(
                "H0001",
                "cannot lower invalid type syntax into HIR",
                ty.span,
            )),
        }
    }

    fn lower_block(&self, block: &ast::Block) -> Result<Block, Diagnostic> {
        Ok(Block {
            statements: block
                .statements
                .iter()
                .map(|statement| self.lower_statement(statement))
                .collect::<Result<Vec<_>, _>>()?,
            span: block.span,
        })
    }

    fn lower_statement(&self, statement: &ast::Stmt) -> Result<Stmt, Diagnostic> {
        let kind = match &statement.kind {
            ast::StmtKind::Let {
                mutable,
                name,
                ty,
                initializer,
            } => StmtKind::Let {
                mutable: *mutable,
                name: name.clone(),
                ty: self.lower_type_ref(ty)?,
                initializer: self.lower_expr(initializer)?,
            },
            ast::StmtKind::Assign { target, value } => StmtKind::Assign {
                target: self.lower_place(target)?,
                value: self.lower_expr(value)?,
            },
            ast::StmtKind::Break => StmtKind::Break,
            ast::StmtKind::Continue => StmtKind::Continue,
            ast::StmtKind::Match { scrutinee, arms } => {
                return self.lower_match_statement(statement.span, scrutinee, arms);
            }
            ast::StmtKind::Expr { expr } => StmtKind::Expr {
                expr: self.lower_expr(expr)?,
            },
            ast::StmtKind::Return { value } => {
                let Some(value) = value else {
                    return Err(self.lowering_error(
                        "H0002",
                        "cannot lower `return;` into value-returning HIR",
                        statement.span,
                    ));
                };
                StmtKind::Return {
                    value: self.lower_expr(value)?,
                }
            }
            ast::StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => StmtKind::If {
                condition: self.lower_expr(condition)?,
                then_branch: self.lower_block(then_branch)?,
                else_branch: else_branch
                    .as_ref()
                    .map(|block| self.lower_block(block))
                    .transpose()?,
            },
            ast::StmtKind::While { condition, body } => StmtKind::While {
                condition: self.lower_expr(condition)?,
                body: self.lower_block(body)?,
            },
            ast::StmtKind::For {
                initializer,
                condition,
                step,
                body,
            } => {
                return self.lower_for_statement(
                    statement.span,
                    initializer.as_deref(),
                    condition.as_ref(),
                    step.as_deref(),
                    body,
                );
            }
            ast::StmtKind::ForIn {
                binding,
                iterable,
                body,
            } => {
                return self.lower_for_in_statement(statement.span, binding, iterable, body);
            }
            ast::StmtKind::Block { block } => StmtKind::Block {
                block: self.lower_block(block)?,
            },
        };

        Ok(Stmt {
            kind,
            span: statement.span,
        })
    }

    fn lower_for_statement(
        &self,
        span: Span,
        initializer: Option<&ast::Stmt>,
        condition: Option<&ast::Expr>,
        step: Option<&ast::Stmt>,
        body: &ast::Block,
    ) -> Result<Stmt, Diagnostic> {
        let mut block_statements = Vec::new();

        if let Some(initializer) = initializer {
            block_statements.push(self.lower_statement(initializer)?);
        }

        let mut lowered_body = self.lower_block(body)?;

        if let Some(step) = step {
            let lowered_step = self.lower_statement(step)?;
            Self::rewrite_for_continues(&mut lowered_body, &lowered_step);
            let mut loop_body_statements = vec![Stmt {
                kind: StmtKind::Block {
                    block: lowered_body,
                },
                span: body.span,
            }];
            loop_body_statements.push(lowered_step);
            return self.finish_lowered_for_block(
                span,
                condition,
                block_statements,
                loop_body_statements,
            );
        }

        let loop_body_statements = vec![Stmt {
            kind: StmtKind::Block {
                block: lowered_body,
            },
            span: body.span,
        }];

        self.finish_lowered_for_block(span, condition, block_statements, loop_body_statements)
    }

    fn lower_for_in_statement(
        &self,
        span: Span,
        binding: &ast::ForInBinding,
        iterable: &ast::Expr,
        body: &ast::Block,
    ) -> Result<Stmt, Diagnostic> {
        let iterable_name = self.fresh_for_in_temp_name("values");
        let index_name = self.fresh_for_in_temp_name("index");
        let element_type = self.lower_type_ref(&binding.ty)?;

        let iterable_binding = Stmt {
            kind: StmtKind::Let {
                mutable: false,
                name: iterable_name.clone(),
                ty: Type::Slice {
                    element: Box::new(element_type.clone()),
                },
                initializer: self.lower_expr(iterable)?,
            },
            span: iterable.span,
        };
        let index_binding = Stmt {
            kind: StmtKind::Let {
                mutable: true,
                name: index_name.clone(),
                ty: Type::I32,
                initializer: Expr {
                    kind: ExprKind::Int { value: 0 },
                    span: binding.span,
                },
            },
            span: binding.span,
        };

        let element_binding = Stmt {
            kind: StmtKind::Let {
                mutable: binding.mutable,
                name: binding.name.clone(),
                ty: element_type,
                initializer: Expr {
                    kind: ExprKind::Index {
                        base: Box::new(Expr {
                            kind: ExprKind::Name {
                                value: iterable_name.clone(),
                            },
                            span: iterable.span,
                        }),
                        index: Box::new(Expr {
                            kind: ExprKind::Name {
                                value: index_name.clone(),
                            },
                            span: binding.span,
                        }),
                    },
                    span: Span::new(iterable.span.start, binding.span.end.max(iterable.span.end)),
                },
            },
            span: binding.span,
        };

        let step = Stmt {
            kind: StmtKind::Assign {
                target: Place {
                    kind: PlaceKind::Local {
                        name: index_name.clone(),
                    },
                    span: binding.span,
                },
                value: Expr {
                    kind: ExprKind::Binary {
                        op: BinaryOp::Add,
                        left: Box::new(Expr {
                            kind: ExprKind::Name {
                                value: index_name.clone(),
                            },
                            span: binding.span,
                        }),
                        right: Box::new(Expr {
                            kind: ExprKind::Int { value: 1 },
                            span: binding.span,
                        }),
                    },
                    span: binding.span,
                },
            },
            span: binding.span,
        };

        let mut lowered_body = self.lower_block(body)?;
        lowered_body.statements.insert(0, element_binding);
        Self::rewrite_for_continues(&mut lowered_body, &step);

        let while_condition = Expr {
            kind: ExprKind::Binary {
                op: BinaryOp::Less,
                left: Box::new(Expr {
                    kind: ExprKind::Name {
                        value: index_name.clone(),
                    },
                    span: binding.span,
                }),
                right: Box::new(Expr {
                    kind: ExprKind::Call {
                        function: "len".to_string(),
                        arguments: vec![Expr {
                            kind: ExprKind::Name {
                                value: iterable_name.clone(),
                            },
                            span: iterable.span,
                        }],
                    },
                    span: iterable.span,
                }),
            },
            span,
        };

        Ok(Stmt {
            kind: StmtKind::Block {
                block: Block {
                    statements: vec![
                        iterable_binding,
                        index_binding,
                        Stmt {
                            kind: StmtKind::While {
                                condition: while_condition,
                                body: Block {
                                    statements: vec![
                                        Stmt {
                                            kind: StmtKind::Block {
                                                block: lowered_body,
                                            },
                                            span: body.span,
                                        },
                                        step,
                                    ],
                                    span,
                                },
                            },
                            span,
                        },
                    ],
                    span,
                },
            },
            span,
        })
    }

    fn lower_match_statement(
        &self,
        span: Span,
        scrutinee: &ast::Expr,
        arms: &[ast::MatchArm],
    ) -> Result<Stmt, Diagnostic> {
        let temp_name = self.fresh_match_temp_name();
        let temp_type = self.infer_match_scrutinee_type(arms, span)?;
        let mut statements = vec![Stmt {
            kind: StmtKind::Let {
                mutable: false,
                name: temp_name.clone(),
                ty: temp_type.clone(),
                initializer: self.lower_expr(scrutinee)?,
            },
            span: scrutinee.span,
        }];
        statements.push(self.lower_match_arm_chain(&temp_name, &temp_type, arms, span)?);
        Ok(Stmt {
            kind: StmtKind::Block {
                block: Block { statements, span },
            },
            span,
        })
    }

    fn finish_lowered_for_block(
        &self,
        span: Span,
        condition: Option<&ast::Expr>,
        mut block_statements: Vec<Stmt>,
        loop_body_statements: Vec<Stmt>,
    ) -> Result<Stmt, Diagnostic> {
        let while_condition = match condition {
            Some(condition) => self.lower_expr(condition)?,
            None => Expr {
                kind: ExprKind::Bool { value: true },
                span,
            },
        };

        block_statements.push(Stmt {
            kind: StmtKind::While {
                condition: while_condition,
                body: Block {
                    statements: loop_body_statements,
                    span,
                },
            },
            span,
        });

        Ok(Stmt {
            kind: StmtKind::Block {
                block: Block {
                    statements: block_statements,
                    span,
                },
            },
            span,
        })
    }

    fn rewrite_for_continues(block: &mut Block, step: &Stmt) {
        let mut rewritten = Vec::with_capacity(block.statements.len());
        for mut statement in std::mem::take(&mut block.statements) {
            match &mut statement.kind {
                StmtKind::Continue => {
                    rewritten.push(step.clone());
                    rewritten.push(statement);
                }
                StmtKind::If {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    Self::rewrite_for_continues(then_branch, step);
                    if let Some(else_branch) = else_branch {
                        Self::rewrite_for_continues(else_branch, step);
                    }
                    rewritten.push(statement);
                }
                StmtKind::Block { block } => {
                    Self::rewrite_for_continues(block, step);
                    rewritten.push(statement);
                }
                StmtKind::While { .. } => {
                    rewritten.push(statement);
                }
                _ => rewritten.push(statement),
            }
        }
        block.statements = rewritten;
    }

    fn lower_match_arm_chain(
        &self,
        temp_name: &str,
        temp_type: &Type,
        arms: &[ast::MatchArm],
        span: Span,
    ) -> Result<Stmt, Diagnostic> {
        let Some((first, rest)) = arms.split_first() else {
            return Ok(Stmt {
                kind: StmtKind::Block {
                    block: Block {
                        statements: Vec::new(),
                        span,
                    },
                },
                span,
            });
        };

        if matches!(
            first.pattern.kind,
            ast::MatchPatternKind::Wildcard | ast::MatchPatternKind::Binding { .. }
        ) {
            return Ok(Stmt {
                kind: StmtKind::Block {
                    block: self.lower_match_arm_block(
                        temp_name,
                        temp_type,
                        &first.pattern,
                        &first.body,
                    )?,
                },
                span: first.span,
            });
        }

        let condition = self.lower_match_pattern_condition(temp_name, &first.pattern)?;
        let then_branch =
            self.lower_match_arm_block(temp_name, temp_type, &first.pattern, &first.body)?;
        let else_branch = if rest.is_empty() {
            Some(Block {
                statements: Vec::new(),
                span: first.span,
            })
        } else {
            let nested = self.lower_match_arm_chain(temp_name, temp_type, rest, rest[0].span)?;
            Some(Block {
                span: nested.span,
                statements: vec![nested],
            })
        };

        Ok(Stmt {
            kind: StmtKind::If {
                condition,
                then_branch,
                else_branch,
            },
            span: first.span,
        })
    }

    fn lower_match_pattern_condition(
        &self,
        temp_name: &str,
        pattern: &ast::MatchPattern,
    ) -> Result<Expr, Diagnostic> {
        Ok(Expr {
            kind: ExprKind::MatchTest {
                scrutinee: Box::new(Expr {
                    kind: ExprKind::Name {
                        value: temp_name.to_string(),
                    },
                    span: pattern.span,
                }),
                pattern: self.lower_match_pattern(pattern)?,
            },
            span: pattern.span,
        })
    }

    fn lower_match_pattern(&self, pattern: &ast::MatchPattern) -> Result<MatchPattern, Diagnostic> {
        let kind = match &pattern.kind {
            ast::MatchPatternKind::Wildcard => MatchPatternKind::Wildcard,
            ast::MatchPatternKind::Binding { name } => {
                MatchPatternKind::Binding { name: name.clone() }
            }
            ast::MatchPatternKind::Bool { value } => MatchPatternKind::Bool { value: *value },
            ast::MatchPatternKind::Int { value } => MatchPatternKind::Int {
                value: i32::try_from(*value).map_err(|_| {
                    self.lowering_error(
                        "H0008",
                        "match integer pattern is out of HIR `i32` range",
                        pattern.span,
                    )
                })?,
            },
            ast::MatchPatternKind::String { value } => MatchPatternKind::String {
                value: value.clone(),
            },
            ast::MatchPatternKind::EnumVariant { path, payload } => {
                let Some((enum_path, variant)) = path.rsplit_once('.') else {
                    return Err(self.lowering_error(
                        "H0009",
                        "match enum pattern must use `EnumName.Variant`",
                        pattern.span,
                    ));
                };
                let enum_name = self
                    .resolve_canonical_name(enum_path, pattern.span, &self.enum_names)
                    .unwrap_or_else(|| enum_path.to_string());
                MatchPatternKind::EnumVariant {
                    enum_name,
                    variant: variant.to_string(),
                    payload: payload.as_ref().map(|payload| match payload {
                        ast::EnumVariantPayloadPattern::Wildcard => {
                            EnumVariantPayloadPattern::Wildcard
                        }
                        ast::EnumVariantPayloadPattern::Binding { name } => {
                            EnumVariantPayloadPattern::Binding { name: name.clone() }
                        }
                    }),
                    payload_type: self.resolve_enum_variant_payload_type(path, pattern.span)?,
                }
            }
            ast::MatchPatternKind::Error => MatchPatternKind::Error,
        };

        Ok(MatchPattern {
            kind,
            span: pattern.span,
        })
    }

    fn infer_match_scrutinee_type(
        &self,
        arms: &[ast::MatchArm],
        span: Span,
    ) -> Result<Type, Diagnostic> {
        for arm in arms {
            match &arm.pattern.kind {
                ast::MatchPatternKind::Bool { .. } => return Ok(Type::Bool),
                ast::MatchPatternKind::Int { .. } => return Ok(Type::I32),
                ast::MatchPatternKind::String { .. } => return Ok(Type::String),
                ast::MatchPatternKind::EnumVariant { path, .. } => {
                    let Some((enum_path, _)) = path.rsplit_once('.') else {
                        return Err(self.lowering_error(
                            "H0012",
                            "match enum pattern must use `EnumName.Variant`",
                            arm.pattern.span,
                        ));
                    };
                    let enum_name = self
                        .resolve_canonical_name(enum_path, arm.pattern.span, &self.enum_names)
                        .unwrap_or_else(|| enum_path.to_string());
                    return Ok(Type::Enum { name: enum_name });
                }
                ast::MatchPatternKind::Wildcard
                | ast::MatchPatternKind::Binding { .. }
                | ast::MatchPatternKind::Error => {}
            }
        }

        Err(self.lowering_error(
            "H0013",
            "cannot infer the lowered match input type without a concrete match pattern",
            span,
        ))
    }

    fn lower_match_arm_block(
        &self,
        temp_name: &str,
        temp_type: &Type,
        pattern: &ast::MatchPattern,
        body: &ast::Block,
    ) -> Result<Block, Diagnostic> {
        let body_block = self.lower_block(body)?;
        match &pattern.kind {
            ast::MatchPatternKind::Binding { name } => Ok(Block {
                statements: vec![
                    Stmt {
                        kind: StmtKind::Let {
                            mutable: false,
                            name: name.clone(),
                            ty: temp_type.clone(),
                            initializer: Expr {
                                kind: ExprKind::Name {
                                    value: temp_name.to_string(),
                                },
                                span: pattern.span,
                            },
                        },
                        span: pattern.span,
                    },
                    Stmt {
                        kind: StmtKind::Block { block: body_block },
                        span: body.span,
                    },
                ],
                span: body.span,
            }),
            ast::MatchPatternKind::EnumVariant {
                path,
                payload: Some(ast::EnumVariantPayloadPattern::Binding { name }),
            } => {
                let payload_type =
                    match self.resolve_enum_variant_payload_type(path, pattern.span)? {
                        Some(payload_type) => payload_type,
                        None => {
                            return Err(self.lowering_error(
                                "H0016",
                                format!("enum variant `{path}` does not carry a payload"),
                                pattern.span,
                            ));
                        }
                    };
                Ok(Block {
                    statements: vec![
                        Stmt {
                            kind: StmtKind::Let {
                                mutable: false,
                                name: name.clone(),
                                ty: payload_type,
                                initializer: Expr {
                                    kind: ExprKind::EnumPayload {
                                        value: Box::new(Expr {
                                            kind: ExprKind::Name {
                                                value: temp_name.to_string(),
                                            },
                                            span: pattern.span,
                                        }),
                                    },
                                    span: pattern.span,
                                },
                            },
                            span: pattern.span,
                        },
                        Stmt {
                            kind: StmtKind::Block { block: body_block },
                            span: body.span,
                        },
                    ],
                    span: body.span,
                })
            }
            _ => Ok(body_block),
        }
    }

    fn resolve_enum_variant_payload_type(
        &self,
        path: &str,
        span: Span,
    ) -> Result<Option<Type>, Diagnostic> {
        let Some((enum_path, variant)) = path.rsplit_once('.') else {
            return Err(self.lowering_error(
                "H0009",
                "match enum pattern must use `EnumName.Variant`",
                span,
            ));
        };
        let enum_name = self
            .resolve_canonical_name(enum_path, span, &self.enum_names)
            .unwrap_or_else(|| enum_path.to_string());
        let Some(variant_payloads) = self.enum_variant_payloads.get(&enum_name) else {
            return Err(self.lowering_error(
                "H0015",
                format!("cannot find payload metadata for enum `{enum_name}`"),
                span,
            ));
        };
        Ok(variant_payloads.get(variant).cloned())
    }

    fn lower_place(&self, expr: &ast::Expr) -> Result<Place, Diagnostic> {
        let kind = match &expr.kind {
            ast::ExprKind::Name { value } => PlaceKind::Local {
                name: value.clone(),
            },
            ast::ExprKind::Field { base, field } => PlaceKind::Field {
                base: Box::new(self.lower_place(base)?),
                field: field.clone(),
            },
            ast::ExprKind::Index { base, index } => PlaceKind::Index {
                base: Box::new(self.lower_place(base)?),
                index: self.lower_expr(index)?,
            },
            _ => {
                return Err(self.lowering_error(
                    "H0003",
                    "HIR assignments require writable place targets built from variables, fields, and indexes",
                    expr.span,
                ));
            }
        };

        Ok(Place {
            kind,
            span: expr.span,
        })
    }

    fn lower_expr(&self, expr: &ast::Expr) -> Result<Expr, Diagnostic> {
        let kind = match &expr.kind {
            ast::ExprKind::Int { value } => {
                let value = i32::try_from(*value).map_err(|_| {
                    self.lowering_error(
                        "H0004",
                        "integer literal is out of HIR `i32` range",
                        expr.span,
                    )
                })?;
                ExprKind::Int { value }
            }
            ast::ExprKind::Float { value } => {
                let narrowed = *value as f32;
                if value.is_finite() && !narrowed.is_finite() {
                    return Err(self.lowering_error(
                        "H0005",
                        "float literal is out of HIR `f32` range",
                        expr.span,
                    ));
                }
                ExprKind::Float { value: narrowed }
            }
            ast::ExprKind::Bool { value } => ExprKind::Bool { value: *value },
            ast::ExprKind::String { value } => ExprKind::String {
                value: value.clone(),
            },
            ast::ExprKind::Name { value } => ExprKind::Name {
                value: value.clone(),
            },
            ast::ExprKind::Unary { op, expr: inner } => ExprKind::Unary {
                op: *op,
                expr: Box::new(self.lower_expr(inner)?),
            },
            ast::ExprKind::Binary { op, left, right } => ExprKind::Binary {
                op: *op,
                left: Box::new(self.lower_expr(left)?),
                right: Box::new(self.lower_expr(right)?),
            },
            ast::ExprKind::Call { callee, arguments } => {
                if let Some(enum_variant) =
                    self.try_lower_enum_variant_constructor(callee, arguments)?
                {
                    enum_variant
                } else {
                    let Some(function) = callee.qualified_name() else {
                        return Err(self.lowering_error(
                            "H0006",
                            "HIR calls require a direct function name",
                            callee.span,
                        ));
                    };
                    if let Some(resolved_function) =
                        self.resolve_canonical_name(&function, callee.span, &self.function_names)
                    {
                        ExprKind::Call {
                            function: resolved_function,
                            arguments: arguments
                                .iter()
                                .map(|argument| self.lower_expr(argument))
                                .collect::<Result<Vec<_>, _>>()?,
                        }
                    } else if let ast::ExprKind::Field { base, field } = &callee.kind
                        && !self.field_base_names_type(base)
                    {
                        ExprKind::MethodCall {
                            receiver: Box::new(self.lower_expr(base)?),
                            method: field.clone(),
                            arguments: arguments
                                .iter()
                                .map(|argument| self.lower_expr(argument))
                                .collect::<Result<Vec<_>, _>>()?,
                        }
                    } else {
                        ExprKind::Call {
                            function: self.resolve_function_name(&function, callee.span),
                            arguments: arguments
                                .iter()
                                .map(|argument| self.lower_expr(argument))
                                .collect::<Result<Vec<_>, _>>()?,
                        }
                    }
                }
            }
            ast::ExprKind::StructLiteral { name, fields } => ExprKind::StructLiteral {
                name: self
                    .resolve_canonical_name(name, expr.span, &self.struct_names)
                    .unwrap_or_else(|| name.clone()),
                fields: fields
                    .iter()
                    .map(|field| {
                        Ok(StructLiteralField {
                            name: field.name.clone(),
                            value: self.lower_expr(&field.value)?,
                            span: field.span,
                        })
                    })
                    .collect::<Result<Vec<_>, Diagnostic>>()?,
            },
            ast::ExprKind::ArrayLiteral { elements } => ExprKind::ArrayLiteral {
                elements: elements
                    .iter()
                    .map(|element| self.lower_expr(element))
                    .collect::<Result<Vec<_>, _>>()?,
            },
            ast::ExprKind::Match { scrutinee, arms } => ExprKind::Match {
                scrutinee: Box::new(self.lower_expr(scrutinee)?),
                arms: arms
                    .iter()
                    .map(|arm| {
                        Ok(MatchExprArm {
                            pattern: self.lower_match_pattern(&arm.pattern)?,
                            value: self.lower_expr(&arm.value)?,
                            span: arm.span,
                        })
                    })
                    .collect::<Result<Vec<_>, Diagnostic>>()?,
            },
            ast::ExprKind::Field { base, field } => {
                if let Some(enum_name) = base.qualified_name().and_then(|name| {
                    self.resolve_canonical_name(&name, base.span, &self.enum_names)
                }) {
                    ExprKind::EnumVariant {
                        enum_name,
                        variant: field.clone(),
                        payload: None,
                    }
                } else {
                    ExprKind::Field {
                        base: Box::new(self.lower_expr(base)?),
                        field: field.clone(),
                    }
                }
            }
            ast::ExprKind::Index { base, index } => ExprKind::Index {
                base: Box::new(self.lower_expr(base)?),
                index: Box::new(self.lower_expr(index)?),
            },
            ast::ExprKind::Slice { base, start, end } => ExprKind::Slice {
                base: Box::new(self.lower_expr(base)?),
                start: Box::new(self.lower_expr(start)?),
                end: Box::new(self.lower_expr(end)?),
            },
            ast::ExprKind::Error => {
                return Err(self.lowering_error(
                    "H0007",
                    "cannot lower invalid AST expression into HIR",
                    expr.span,
                ));
            }
        };

        Ok(Expr {
            kind,
            span: expr.span,
        })
    }

    fn try_lower_enum_variant_constructor(
        &self,
        callee: &ast::Expr,
        arguments: &[ast::Expr],
    ) -> Result<Option<ExprKind>, Diagnostic> {
        let Some(path) = callee.qualified_name() else {
            return Ok(None);
        };
        let Some((enum_path, variant)) = path.rsplit_once('.') else {
            return Ok(None);
        };
        let Some(enum_name) = self.resolve_canonical_name(enum_path, callee.span, &self.enum_names)
        else {
            return Ok(None);
        };

        if arguments.len() != 1 {
            return Ok(None);
        }
        let argument = &arguments[0];
        Ok(Some(ExprKind::EnumVariant {
            enum_name,
            variant: variant.to_string(),
            payload: Some(Box::new(self.lower_expr(argument)?)),
        }))
    }

    fn lowering_error(&self, code: &str, message: impl Into<String>, span: Span) -> Diagnostic {
        Diagnostic::new(code, message.into(), self.source, span)
    }

    fn fresh_match_temp_name(&self) -> String {
        let next = self.next_match_temp.get();
        self.next_match_temp.set(next + 1);
        format!("__match_scrutinee_{next}")
    }

    fn fresh_for_in_temp_name(&self, suffix: &str) -> String {
        let next = self.next_for_in_temp.get();
        self.next_for_in_temp.set(next + 1);
        format!("__for_in_{suffix}_{next}")
    }

    fn canonical_name(&self, name: &str, span: Span) -> String {
        self.resolve_same_unit_name(name, span)
            .unwrap_or_else(|| name.to_string())
    }

    fn resolve_function_name(&self, name: &str, span: Span) -> String {
        self.resolve_canonical_name(name, span, &self.function_names)
            .unwrap_or_else(|| name.to_string())
    }

    fn impl_method_prefix(&self, target: &ast::TypeRef, span: Span) -> Result<String, Diagnostic> {
        let Some(name) = target.direct_name() else {
            return Err(self.lowering_error("H0017", "impl target must be a named type", span));
        };
        self.resolve_canonical_name(name, span, &self.struct_names)
            .or_else(|| self.resolve_canonical_name(name, span, &self.enum_names))
            .ok_or_else(|| self.lowering_error("H0017", "impl target must resolve to a type", span))
    }

    fn field_base_names_type(&self, base: &ast::Expr) -> bool {
        base.qualified_name()
            .and_then(|name| {
                self.resolve_canonical_name(&name, base.span, &self.struct_names)
                    .or_else(|| self.resolve_canonical_name(&name, base.span, &self.enum_names))
            })
            .is_some()
    }

    fn resolve_canonical_name(
        &self,
        name: &str,
        span: Span,
        known_names: &HashSet<String>,
    ) -> Option<String> {
        if known_names.contains(name) {
            return Some(name.to_string());
        }

        let local_name = self.resolve_same_unit_name(name, span)?;
        known_names.contains(&local_name).then_some(local_name)
    }

    fn resolve_same_unit_name(&self, name: &str, span: Span) -> Option<String> {
        if name.contains('.') {
            return None;
        }

        let unit_path = self.source.display_path_for_offset(span.start);
        let module_path = self.unit_modules.get(unit_path)?;
        Some(format!("{module_path}.{name}"))
    }
}

fn canonical_item_name(
    source: &SourceFile,
    unit_modules: &HashMap<String, String>,
    item: &ast::Item,
) -> String {
    let unit_path = source.display_path_for_offset(item.span.start);
    match &item.kind {
        ast::ItemKind::Function { name, .. }
        | ast::ItemKind::Struct { name, .. }
        | ast::ItemKind::Enum { name, .. }
        | ast::ItemKind::Trait { name, .. } => unit_modules
            .get(unit_path)
            .map(|module_path| format!("{module_path}.{name}"))
            .unwrap_or_else(|| name.clone()),
        ast::ItemKind::Impl { target, .. } => target
            .direct_name()
            .map(|name| {
                unit_modules
                    .get(unit_path)
                    .map(|module_path| format!("{module_path}.{name}"))
                    .unwrap_or_else(|| name.to_string())
            })
            .unwrap_or_else(|| "<impl>".to_string()),
    }
}

fn looks_like_type_param(name: &str) -> bool {
    name.chars()
        .next()
        .map(|ch| ch.is_ascii_uppercase())
        .unwrap_or(false)
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

#[cfg(test)]
mod tests {
    use super::{
        EnumVariantPayloadPattern, ExprKind, ItemKind, MatchPatternKind, PlaceKind, StmtKind, Type,
        lower_program,
    };
    use crate::lexer::tokenize;
    use crate::parser::parse;
    use crate::semantic::check_program;
    use crate::source::SourceFile;
    use std::path::PathBuf;

    fn lower(source_text: &str) -> super::Program {
        let source = SourceFile::anonymous(source_text);
        lower_source(&source)
    }

    fn lower_source(source: &SourceFile) -> super::Program {
        let tokens = tokenize(&source);
        let parsed = parse(&source, tokens.tokens);
        let diagnostics = check_program(&source, &parsed.program);
        assert!(
            diagnostics.is_empty(),
            "semantic diagnostics must be empty before lowering: {:?}",
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>()
        );

        lower_program(&source, &parsed.program).expect("HIR lowering should succeed")
    }

    #[test]
    fn lowers_module_qualified_types_and_calls() {
        let source = SourceFile::from_segments(
            "src/main.ax",
            vec![
                (
                    PathBuf::from("lib/report.ax"),
                    "\
module lib.report;

struct Summary {
    count: i32,
}

fn build_summary() -> Summary {
    return Summary { count: 7 };
}
"
                    .to_string(),
                ),
                (
                    PathBuf::from("src/main.ax"),
                    "\
import lib.report;

fn main() -> i32 {
    let summary: lib.report.Summary = lib.report.build_summary();
    return summary.count;
}
"
                    .to_string(),
                ),
            ],
        );

        let program = lower_source(&source);

        assert!(matches!(
            program.items[0].kind,
            ItemKind::Struct { ref name, .. } if name == "lib.report.Summary"
        ));
        assert!(matches!(
            program.items[1].kind,
            ItemKind::Function { ref name, .. } if name == "lib.report.build_summary"
        ));

        let ItemKind::Function { body, .. } = &program.items[2].kind else {
            panic!("expected main function");
        };
        let StmtKind::Let {
            ty, initializer, ..
        } = &body.statements[0].kind
        else {
            panic!("expected let statement");
        };
        assert!(matches!(
            ty,
            Type::Struct { name } if name == "lib.report.Summary"
        ));
        assert!(matches!(
            initializer.kind,
            ExprKind::Call { ref function, .. } if function == "lib.report.build_summary"
        ));
    }

    #[test]
    fn lowers_module_qualified_enum_variants() {
        let source = SourceFile::from_segments(
            "src/main.ax",
            vec![
                (
                    PathBuf::from("lib/flag.ax"),
                    "\
module lib.flag;

enum Flag {
    On,
    Off,
}
"
                    .to_string(),
                ),
                (
                    PathBuf::from("src/main.ax"),
                    "\
import lib.flag;

fn main() -> i32 {
    let flag: lib.flag.Flag = lib.flag.Flag.On;
    println(flag);
    return 0;
}
"
                    .to_string(),
                ),
            ],
        );

        let program = lower_source(&source);

        let ItemKind::Function { body, .. } = &program.items[1].kind else {
            panic!("expected main function");
        };
        let StmtKind::Let { initializer, .. } = &body.statements[0].kind else {
            panic!("expected let statement");
        };
        assert!(matches!(
            initializer.kind,
            ExprKind::EnumVariant {
                ref enum_name,
                ref variant,
                payload: None
            }
                if enum_name == "lib.flag.Flag" && variant == "On"
        ));
    }

    #[test]
    fn lowers_for_loop_into_scoped_block_and_while() {
        let program = lower(
            "\
fn main() -> i32 {
    let mut total: i32 = 0;
    for (let mut i: i32 = 0; i < 3; i = i + 1) {
        total = total + i;
    }
    return total;
}
",
        );

        let ItemKind::Function { body, .. } = &program.items[0].kind else {
            panic!("expected function item");
        };

        assert_eq!(body.statements.len(), 3);
        let StmtKind::Block { block } = &body.statements[1].kind else {
            panic!("expected lowered for loop block");
        };
        assert_eq!(block.statements.len(), 2);
        assert!(matches!(block.statements[0].kind, StmtKind::Let { .. }));

        let StmtKind::While {
            condition,
            body: while_body,
        } = &block.statements[1].kind
        else {
            panic!("expected lowered while statement");
        };

        assert!(matches!(condition.kind, ExprKind::Binary { .. }));
        assert_eq!(while_body.statements.len(), 2);
        assert!(matches!(
            while_body.statements[0].kind,
            StmtKind::Block { .. }
        ));
        assert!(matches!(
            while_body.statements[1].kind,
            StmtKind::Assign { .. }
        ));
    }

    #[test]
    fn lowers_for_in_into_indexed_while_with_binding() {
        let program = lower(
            "\
fn main() -> i32 {
    let values: [i32; 3] = [1, 2, 3];
    for (let value: i32 in values) {
        println(value);
    }
    return 0;
}
",
        );

        let ItemKind::Function { body, .. } = &program.items[0].kind else {
            panic!("expected function item");
        };

        let StmtKind::Block { block } = &body.statements[1].kind else {
            panic!("expected lowered for-in outer block");
        };
        assert_eq!(block.statements.len(), 3);
        assert!(matches!(block.statements[0].kind, StmtKind::Let { .. }));
        assert!(matches!(block.statements[1].kind, StmtKind::Let { .. }));

        let StmtKind::While {
            body: while_body, ..
        } = &block.statements[2].kind
        else {
            panic!("expected lowered while statement");
        };
        assert_eq!(while_body.statements.len(), 2);

        let StmtKind::Block { block: loop_block } = &while_body.statements[0].kind else {
            panic!("expected lowered loop body block");
        };
        let StmtKind::Let {
            name, initializer, ..
        } = &loop_block.statements[0].kind
        else {
            panic!("expected synthesized element binding");
        };
        assert_eq!(name, "value");
        assert!(matches!(initializer.kind, ExprKind::Index { .. }));
        assert!(matches!(
            while_body.statements[1].kind,
            StmtKind::Assign { .. }
        ));
    }

    #[test]
    fn lowers_enum_variants_and_assignment_places() {
        let program = lower(
            "\
enum Flag { On, Off }
struct Point { x: i32 }

fn main() -> i32 {
    let flag: Flag = Flag.On;
    let mut point: Point = Point { x: 1 };
    point.x = 2;
    println(flag);
    return 0;
}
",
        );

        let ItemKind::Function { body, .. } = &program.items[2].kind else {
            panic!("expected main function");
        };

        let StmtKind::Let { initializer, .. } = &body.statements[0].kind else {
            panic!("expected let statement");
        };
        assert!(matches!(initializer.kind, ExprKind::EnumVariant { .. }));

        let StmtKind::Assign { target, .. } = &body.statements[2].kind else {
            panic!("expected assignment");
        };
        assert!(matches!(target.kind, PlaceKind::Field { .. }));

        let StmtKind::Expr { expr } = &body.statements[3].kind else {
            panic!("expected expression statement");
        };
        assert!(matches!(
            expr.kind,
            ExprKind::Call { ref function, .. } if function == "println"
        ));
    }

    #[test]
    fn lowers_array_types_literals_and_indexing() {
        let program = lower(
            "\
fn main() -> i32 {
    let values: [i32; 3] = [1, 2, 3];
    return values[1];
}
",
        );

        let ItemKind::Function {
            return_type, body, ..
        } = &program.items[0].kind
        else {
            panic!("expected function");
        };

        assert!(matches!(return_type, Type::I32));

        let StmtKind::Let {
            ty, initializer, ..
        } = &body.statements[0].kind
        else {
            panic!("expected let statement");
        };
        assert!(matches!(
            ty,
            Type::Array { element, length } if **element == Type::I32 && *length == 3
        ));
        assert!(matches!(initializer.kind, ExprKind::ArrayLiteral { .. }));

        let StmtKind::Return { value } = &body.statements[1].kind else {
            panic!("expected return statement");
        };
        assert!(matches!(value.kind, ExprKind::Index { .. }));
    }

    #[test]
    fn lowers_slice_types_and_expressions() {
        let program = lower(
            "\
fn window(values: [i32]) -> i32 {
    let head: [i32] = values[0:2];
    return head[1];
}

fn main() -> i32 {
    let values: [i32; 3] = [1, 2, 3];
    return window(values);
}
",
        );

        let ItemKind::Function {
            params,
            return_type,
            body,
            ..
        } = &program.items[0].kind
        else {
            panic!("expected function");
        };

        assert!(matches!(
            params[0].ty,
            Type::Slice { ref element } if **element == Type::I32
        ));
        assert!(matches!(return_type, Type::I32));

        let StmtKind::Let {
            ty, initializer, ..
        } = &body.statements[0].kind
        else {
            panic!("expected let");
        };
        assert!(matches!(
            ty,
            Type::Slice { element } if **element == Type::I32
        ));
        assert!(matches!(initializer.kind, ExprKind::Slice { .. }));
    }

    #[test]
    fn lowers_array_element_assignment_places() {
        let program = lower(
            "\
fn main() -> i32 {
    let mut values: [i32; 2] = [1, 2];
    values[0] = 3;
    return values[0];
}
",
        );

        let ItemKind::Function { body, .. } = &program.items[0].kind else {
            panic!("expected function");
        };

        let StmtKind::Assign { target, .. } = &body.statements[1].kind else {
            panic!("expected assignment statement");
        };
        assert!(matches!(target.kind, PlaceKind::Index { .. }));
    }

    #[test]
    fn lowers_nested_assignment_places() {
        let program = lower(
            "\
struct Point { x: i32 }

fn main() -> i32 {
    let mut points: [Point; 2] = [Point { x: 1 }, Point { x: 2 }];
    points[0].x = 3;
    return points[0].x;
}
",
        );

        let ItemKind::Function { body, .. } = &program.items[1].kind else {
            panic!("expected main function");
        };

        let StmtKind::Assign { target, .. } = &body.statements[1].kind else {
            panic!("expected assignment statement");
        };

        match &target.kind {
            PlaceKind::Field { base, field } => {
                assert_eq!(field, "x");
                assert!(matches!(base.kind, PlaceKind::Index { .. }));
            }
            _ => panic!("expected nested field assignment place"),
        }
    }

    #[test]
    fn lowers_break_statements() {
        let program = lower(
            "\
fn main() -> i32 {
    while (true) {
        break;
    }
    return 0;
}
",
        );

        let ItemKind::Function { body, .. } = &program.items[0].kind else {
            panic!("expected function item");
        };

        let StmtKind::While { body, .. } = &body.statements[0].kind else {
            panic!("expected while statement");
        };

        assert!(matches!(body.statements[0].kind, StmtKind::Break));
    }

    #[test]
    fn lowers_continue_statements() {
        let program = lower(
            "\
fn main() -> i32 {
    while (true) {
        continue;
    }
    return 0;
}
",
        );

        let ItemKind::Function { body, .. } = &program.items[0].kind else {
            panic!("expected function item");
        };

        let StmtKind::While { body, .. } = &body.statements[0].kind else {
            panic!("expected while statement");
        };

        assert!(matches!(body.statements[0].kind, StmtKind::Continue));
    }

    #[test]
    fn lowers_match_statements_into_temp_and_if_chain() {
        let program = lower(
            "\
fn main() -> i32 {
    let flag: bool = true;
    match (flag) {
        true => {
            println(1);
        }
        _ => {
            println(0);
        }
    }
    return 0;
}
",
        );

        let ItemKind::Function { body, .. } = &program.items[0].kind else {
            panic!("expected function item");
        };

        let StmtKind::Block { block } = &body.statements[1].kind else {
            panic!("expected lowered match outer block");
        };

        let StmtKind::Let {
            name,
            ty,
            initializer,
            ..
        } = &block.statements[0].kind
        else {
            panic!("expected synthesized match scrutinee binding");
        };
        assert_eq!(name, "__match_scrutinee_0");
        assert!(matches!(ty, Type::Bool));
        assert!(matches!(initializer.kind, ExprKind::Name { ref value } if value == "flag"));

        let StmtKind::If {
            condition,
            then_branch,
            else_branch,
        } = &block.statements[1].kind
        else {
            panic!("expected lowered match if chain");
        };

        assert!(matches!(condition.kind, ExprKind::MatchTest { .. }));
        assert!(matches!(
            then_branch.statements[0].kind,
            StmtKind::Expr { .. }
        ));

        let else_branch = else_branch
            .as_ref()
            .expect("match should lower wildcard arm into else branch");
        assert!(matches!(
            else_branch.statements[0].kind,
            StmtKind::Block { .. }
        ));
    }

    #[test]
    fn lowers_match_expressions() {
        let program = lower(
            "\
fn main() -> i32 {
    let flag: bool = true;
    let value: i32 = match (flag) { true => 1, false => 0 };
    return value;
}
",
        );

        let ItemKind::Function { body, .. } = &program.items[0].kind else {
            panic!("expected function item");
        };

        let StmtKind::Let { initializer, .. } = &body.statements[1].kind else {
            panic!("expected match-expression let");
        };
        let ExprKind::Match { scrutinee, arms } = &initializer.kind else {
            panic!("expected lowered match expression");
        };
        assert!(matches!(scrutinee.kind, ExprKind::Name { ref value } if value == "flag"));
        assert_eq!(arms.len(), 2);
        assert!(matches!(
            arms[0].pattern.kind,
            MatchPatternKind::Bool { value: true }
        ));
        assert!(matches!(arms[0].value.kind, ExprKind::Int { value: 1 }));
    }

    #[test]
    fn lowers_match_expression_binding_patterns() {
        let program = lower(
            "\
fn main() -> i32 {
    let value: i32 = match (4) { 0 => 1, other => other };
    return value;
}
",
        );

        let ItemKind::Function { body, .. } = &program.items[0].kind else {
            panic!("expected function item");
        };

        let StmtKind::Let { initializer, .. } = &body.statements[0].kind else {
            panic!("expected match-expression let");
        };
        let ExprKind::Match { arms, .. } = &initializer.kind else {
            panic!("expected lowered match expression");
        };
        assert!(matches!(
            arms[1].pattern.kind,
            MatchPatternKind::Binding { ref name } if name == "other"
        ));
        assert!(matches!(arms[1].value.kind, ExprKind::Name { ref value } if value == "other"));
    }

    #[test]
    fn lowers_payload_enum_constructors_and_patterns() {
        let program = lower(
            "\
enum Result { Ok(i32), Err(string), Empty }

fn score(result: Result) -> i32 {
    return match (result) {
        Result.Ok(value) => value,
        Result.Err(_) => 0,
        Result.Empty => -1,
    };
}

fn main() -> i32 {
    let ok: Result = Result.Ok(7);
    return score(ok);
}
",
        );

        let ItemKind::Function { body, .. } = &program.items[1].kind else {
            panic!("expected score function");
        };
        let StmtKind::Return { value: expr } = &body.statements[0].kind else {
            panic!("expected return statement");
        };
        let ExprKind::Match { arms, .. } = &expr.kind else {
            panic!("expected lowered match expression");
        };
        assert!(matches!(
            arms[0].pattern.kind,
            MatchPatternKind::EnumVariant {
                ref enum_name,
                ref variant,
                payload: Some(EnumVariantPayloadPattern::Binding { ref name }),
                payload_type: Some(Type::I32),
            } if enum_name == "Result" && variant == "Ok" && name == "value"
        ));
        assert!(matches!(
            arms[1].pattern.kind,
            MatchPatternKind::EnumVariant {
                ref enum_name,
                ref variant,
                payload: Some(EnumVariantPayloadPattern::Wildcard),
                payload_type: Some(Type::String),
            } if enum_name == "Result" && variant == "Err"
        ));
        assert!(matches!(
            arms[2].pattern.kind,
            MatchPatternKind::EnumVariant {
                ref enum_name,
                ref variant,
                payload: None,
                payload_type: None,
            } if enum_name == "Result" && variant == "Empty"
        ));

        let ItemKind::Function { body, .. } = &program.items[2].kind else {
            panic!("expected main function");
        };
        let StmtKind::Let { initializer, .. } = &body.statements[0].kind else {
            panic!("expected let statement");
        };
        assert!(matches!(
            initializer.kind,
            ExprKind::EnumVariant {
                ref enum_name,
                ref variant,
                payload: Some(_),
            } if enum_name == "Result" && variant == "Ok"
        ));
    }

    #[test]
    fn keeps_invalid_multi_argument_enum_constructor_calls_as_calls_in_hir() {
        let source = SourceFile::anonymous(
            "\
enum Result { Ok(i32) }

fn main() -> i32 {
    Result.Ok(1, 2);
    return 0;
}
",
        );
        let tokens = tokenize(&source);
        let parsed = parse(&source, tokens.tokens);
        let program =
            lower_program(&source, &parsed.program).expect("HIR lowering should stay lossless");

        let ItemKind::Function { body, .. } = &program.items[1].kind else {
            panic!("expected main function");
        };
        let StmtKind::Expr { expr } = &body.statements[0].kind else {
            panic!("expected expression statement");
        };
        assert!(matches!(
            expr.kind,
            ExprKind::Call {
                ref function,
                ref arguments,
            } if function == "Result.Ok" && arguments.len() == 2
        ));
    }

    #[test]
    fn rewrites_for_continue_to_run_step_before_loop_continue() {
        let program = lower(
            "\
fn main() -> i32 {
    let mut total: i32 = 0;
    for (let mut i: i32 = 0; i < 4; i = i + 1) {
        if (i == 1) {
            continue;
        }
        total = total + i;
    }
    return total;
}
",
        );

        let ItemKind::Function { body, .. } = &program.items[0].kind else {
            panic!("expected function item");
        };

        let StmtKind::Block { block } = &body.statements[1].kind else {
            panic!("expected lowered for loop outer block");
        };

        let StmtKind::While { body, .. } = &block.statements[1].kind else {
            panic!("expected lowered while statement");
        };

        let StmtKind::Block {
            block: lowered_body,
        } = &body.statements[0].kind
        else {
            panic!("expected original for body wrapper block");
        };

        let StmtKind::If {
            then_branch: continue_branch,
            ..
        } = &lowered_body.statements[0].kind
        else {
            panic!("expected if statement guarding continue");
        };

        assert!(
            matches!(continue_branch.statements[0].kind, StmtKind::Assign { .. }),
            "for-loop continue branch should run the step before continuing"
        );
        assert!(
            matches!(continue_branch.statements[1].kind, StmtKind::Continue),
            "for-loop continue branch should still end with continue"
        );
        assert!(
            matches!(body.statements[1].kind, StmtKind::Assign { .. }),
            "lowered for loop should keep the normal step at the end of the while body"
        );
    }
}
