use std::collections::HashSet;

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
        params: Vec<Param>,
        return_type: Type,
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
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Type {
    Bool,
    I32,
    F32,
    String,
    Struct { name: String },
    Enum { name: String },
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
        ty: Type,
        initializer: Expr,
    },
    Assign {
        target: Place,
        value: Expr,
    },
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
    Field { base: String, field: String },
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
    StructLiteral {
        name: String,
        fields: Vec<StructLiteralField>,
    },
    EnumVariant {
        enum_name: String,
        variant: String,
    },
    Field {
        base: Box<Expr>,
        field: String,
    },
}

pub fn lower_program(source: &SourceFile, program: &ast::Program) -> Result<Program, Diagnostic> {
    LoweringContext::new(source, program).lower_program(program)
}

struct LoweringContext<'a> {
    source: &'a SourceFile,
    struct_names: HashSet<String>,
    enum_names: HashSet<String>,
}

impl<'a> LoweringContext<'a> {
    fn new(source: &'a SourceFile, program: &ast::Program) -> Self {
        let mut struct_names = HashSet::new();
        let mut enum_names = HashSet::new();

        for item in &program.items {
            match &item.kind {
                ast::ItemKind::Struct { name, .. } => {
                    struct_names.insert(name.clone());
                }
                ast::ItemKind::Enum { name, .. } => {
                    enum_names.insert(name.clone());
                }
                ast::ItemKind::Function { .. } => {}
            }
        }

        Self {
            source,
            struct_names,
            enum_names,
        }
    }

    fn lower_program(&self, program: &ast::Program) -> Result<Program, Diagnostic> {
        Ok(Program {
            items: program
                .items
                .iter()
                .map(|item| self.lower_item(item))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    fn lower_item(&self, item: &ast::Item) -> Result<Item, Diagnostic> {
        let kind = match &item.kind {
            ast::ItemKind::Function {
                name,
                params,
                return_type,
                body,
            } => ItemKind::Function {
                name: name.clone(),
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
            ast::ItemKind::Struct { name, fields } => ItemKind::Struct {
                name: name.clone(),
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
            ast::ItemKind::Enum { name, variants } => ItemKind::Enum {
                name: name.clone(),
                variants: variants
                    .iter()
                    .map(|variant| EnumVariant {
                        name: variant.name.clone(),
                        span: variant.span,
                    })
                    .collect(),
            },
        };

        Ok(Item {
            kind,
            span: item.span,
        })
    }

    fn lower_type_ref(&self, ty: &ast::TypeRef) -> Result<Type, Diagnostic> {
        match ty.name.as_str() {
            "bool" => Ok(Type::Bool),
            "i32" => Ok(Type::I32),
            "f32" => Ok(Type::F32),
            "string" => Ok(Type::String),
            name if self.struct_names.contains(name) => Ok(Type::Struct {
                name: name.to_string(),
            }),
            name if self.enum_names.contains(name) => Ok(Type::Enum {
                name: name.to_string(),
            }),
            _ => Err(self.lowering_error(
                "H0001",
                format!("cannot lower unknown type `{}` into HIR", ty.name),
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

        let lowered_body = self.lower_block(body)?;
        let mut loop_body_statements = vec![Stmt {
            kind: StmtKind::Block {
                block: lowered_body,
            },
            span: body.span,
        }];

        if let Some(step) = step {
            loop_body_statements.push(self.lower_statement(step)?);
        }

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

    fn lower_place(&self, expr: &ast::Expr) -> Result<Place, Diagnostic> {
        let kind = match &expr.kind {
            ast::ExprKind::Name { value } => PlaceKind::Local {
                name: value.clone(),
            },
            ast::ExprKind::Field { base, field } => {
                let ast::ExprKind::Name { value } = &base.kind else {
                    return Err(self.lowering_error(
                        "H0003",
                        "HIR assignments require a direct variable or direct field target",
                        expr.span,
                    ));
                };
                PlaceKind::Field {
                    base: value.clone(),
                    field: field.clone(),
                }
            }
            _ => {
                return Err(self.lowering_error(
                    "H0003",
                    "HIR assignments require a direct variable or direct field target",
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
                let ast::ExprKind::Name { value } = &callee.kind else {
                    return Err(self.lowering_error(
                        "H0006",
                        "HIR calls require a direct function name",
                        callee.span,
                    ));
                };
                ExprKind::Call {
                    function: value.clone(),
                    arguments: arguments
                        .iter()
                        .map(|argument| self.lower_expr(argument))
                        .collect::<Result<Vec<_>, _>>()?,
                }
            }
            ast::ExprKind::StructLiteral { name, fields } => ExprKind::StructLiteral {
                name: name.clone(),
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
            ast::ExprKind::Field { base, field } => {
                if let ast::ExprKind::Name { value: enum_name } = &base.kind
                    && self.enum_names.contains(enum_name)
                {
                    ExprKind::EnumVariant {
                        enum_name: enum_name.clone(),
                        variant: field.clone(),
                    }
                } else {
                    ExprKind::Field {
                        base: Box::new(self.lower_expr(base)?),
                        field: field.clone(),
                    }
                }
            }
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

    fn lowering_error(&self, code: &str, message: impl Into<String>, span: Span) -> Diagnostic {
        Diagnostic::new(code, message.into(), self.source, span)
    }
}

#[cfg(test)]
mod tests {
    use super::{ExprKind, ItemKind, PlaceKind, StmtKind, lower_program};
    use crate::lexer::tokenize;
    use crate::parser::parse;
    use crate::semantic::check_program;
    use crate::source::SourceFile;

    fn lower(source_text: &str) -> super::Program {
        let source = SourceFile::anonymous(source_text);
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
}
