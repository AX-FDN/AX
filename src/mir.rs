use std::collections::HashMap;

use serde::Serialize;

use crate::hir::{self};
pub use crate::hir::{BinaryOp, EnumVariant, StructField, Type, UnaryOp};
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
        return_type: Type,
        locals: Vec<Local>,
        entry_block: u32,
        blocks: Vec<BasicBlock>,
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
    Local {
        local: u32,
        name: String,
    },
    Field {
        local: u32,
        name: String,
        field: String,
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

#[derive(Debug, Clone, Serialize)]
pub struct StructLiteralField {
    pub name: String,
    pub value: Expr,
    pub span: Span,
}

pub fn lower_program(program: &hir::Program) -> Result<Program, String> {
    Ok(Program {
        items: program
            .items
            .iter()
            .map(lower_item)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn lower_item(item: &hir::Item) -> Result<Item, String> {
    let kind = match &item.kind {
        hir::ItemKind::Function {
            name,
            params,
            return_type,
            body,
        } => {
            let mut lowerer = FunctionLowerer::new();
            let params = lowerer.lower_params(params);
            let entry_block = lowerer.new_block(body.span);
            let exit_block = lowerer.lower_block(body, entry_block)?;
            if !lowerer.block_is_terminated(exit_block) {
                lowerer.set_terminator(
                    exit_block,
                    Terminator {
                        kind: TerminatorKind::Unreachable,
                        span: body.span,
                    },
                );
            }
            let (locals, blocks) = lowerer.finish();

            ItemKind::Function {
                name: name.clone(),
                params,
                return_type: return_type.clone(),
                locals,
                entry_block,
                blocks,
            }
        }
        hir::ItemKind::Struct { name, fields } => ItemKind::Struct {
            name: name.clone(),
            fields: fields.clone(),
        },
        hir::ItemKind::Enum { name, variants } => ItemKind::Enum {
            name: name.clone(),
            variants: variants.clone(),
        },
    };

    Ok(Item {
        kind,
        span: item.span,
    })
}

struct FunctionLowerer {
    locals: Vec<Local>,
    scopes: Vec<HashMap<String, u32>>,
    blocks: Vec<BasicBlockBuilder>,
}

struct BasicBlockBuilder {
    id: u32,
    span: Span,
    statements: Vec<Statement>,
    terminator: Option<Terminator>,
}

impl FunctionLowerer {
    fn new() -> Self {
        Self {
            locals: Vec::new(),
            scopes: vec![HashMap::new()],
            blocks: Vec::new(),
        }
    }

    fn lower_params(&mut self, params: &[hir::Param]) -> Vec<Param> {
        params
            .iter()
            .map(|param| {
                let local = self.allocate_local(
                    &param.name,
                    &param.ty,
                    false,
                    LocalKind::Param,
                    param.span,
                );
                self.declare(&param.name, local);
                Param {
                    local,
                    name: param.name.clone(),
                    ty: param.ty.clone(),
                    span: param.span,
                }
            })
            .collect()
    }

    fn lower_block(&mut self, block: &hir::Block, current: u32) -> Result<u32, String> {
        self.push_scope();
        let mut current = current;
        for statement in &block.statements {
            current = self.lower_statement(statement, current)?;
        }
        self.pop_scope();
        Ok(current)
    }

    fn lower_statement(&mut self, statement: &hir::Stmt, current: u32) -> Result<u32, String> {
        match &statement.kind {
            hir::StmtKind::Let {
                mutable,
                name,
                ty,
                initializer,
            } => {
                let initializer = self.lower_expr(initializer)?;
                let local =
                    self.allocate_local(name, ty, *mutable, LocalKind::Local, statement.span);
                self.declare(name, local);
                self.push_statement(
                    current,
                    Statement {
                        kind: StatementKind::Let {
                            local,
                            name: name.clone(),
                            mutable: *mutable,
                            ty: ty.clone(),
                            initializer,
                        },
                        span: statement.span,
                    },
                );
                Ok(current)
            }
            hir::StmtKind::Assign { target, value } => {
                self.push_statement(
                    current,
                    Statement {
                        kind: StatementKind::Assign {
                            target: self.lower_place(target)?,
                            value: self.lower_expr(value)?,
                        },
                        span: statement.span,
                    },
                );
                Ok(current)
            }
            hir::StmtKind::Expr { expr } => {
                self.push_statement(
                    current,
                    Statement {
                        kind: StatementKind::Eval {
                            expr: self.lower_expr(expr)?,
                        },
                        span: statement.span,
                    },
                );
                Ok(current)
            }
            hir::StmtKind::Return { value } => {
                self.set_terminator(
                    current,
                    Terminator {
                        kind: TerminatorKind::Return {
                            value: self.lower_expr(value)?,
                        },
                        span: statement.span,
                    },
                );
                Ok(self.new_block(statement.span))
            }
            hir::StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let then_block = self.new_block(then_branch.span);
                let else_block = self.new_block(
                    else_branch
                        .as_ref()
                        .map_or(statement.span, |block| block.span),
                );
                let join_block = self.new_block(statement.span);

                self.set_terminator(
                    current,
                    Terminator {
                        kind: TerminatorKind::Branch {
                            condition: self.lower_expr(condition)?,
                            then_block,
                            else_block,
                        },
                        span: statement.span,
                    },
                );

                let then_exit = self.lower_block(then_branch, then_block)?;
                if !self.block_is_terminated(then_exit) {
                    self.set_terminator(then_exit, goto(join_block, then_branch.span));
                }

                if let Some(else_branch) = else_branch {
                    let else_exit = self.lower_block(else_branch, else_block)?;
                    if !self.block_is_terminated(else_exit) {
                        self.set_terminator(else_exit, goto(join_block, else_branch.span));
                    }
                } else {
                    self.set_terminator(else_block, goto(join_block, statement.span));
                }

                Ok(join_block)
            }
            hir::StmtKind::While { condition, body } => {
                let condition_block = self.new_block(condition.span);
                let body_block = self.new_block(body.span);
                let exit_block = self.new_block(statement.span);

                self.set_terminator(current, goto(condition_block, statement.span));
                self.set_terminator(
                    condition_block,
                    Terminator {
                        kind: TerminatorKind::Branch {
                            condition: self.lower_expr(condition)?,
                            then_block: body_block,
                            else_block: exit_block,
                        },
                        span: condition.span,
                    },
                );

                let body_exit = self.lower_block(body, body_block)?;
                if !self.block_is_terminated(body_exit) {
                    self.set_terminator(body_exit, goto(condition_block, body.span));
                }

                Ok(exit_block)
            }
            hir::StmtKind::Block { block } => self.lower_block(block, current),
        }
    }

    fn lower_place(&self, place: &hir::Place) -> Result<Place, String> {
        let kind = match &place.kind {
            hir::PlaceKind::Local { name } => PlaceKind::Local {
                local: self.lookup(name)?,
                name: name.clone(),
            },
            hir::PlaceKind::Field { base, field } => PlaceKind::Field {
                local: self.lookup(base)?,
                name: base.clone(),
                field: field.clone(),
            },
        };

        Ok(Place {
            kind,
            span: place.span,
        })
    }

    fn lower_expr(&self, expr: &hir::Expr) -> Result<Expr, String> {
        let kind = match &expr.kind {
            hir::ExprKind::Int { value } => ExprKind::Int { value: *value },
            hir::ExprKind::Float { value } => ExprKind::Float { value: *value },
            hir::ExprKind::Bool { value } => ExprKind::Bool { value: *value },
            hir::ExprKind::String { value } => ExprKind::String {
                value: value.clone(),
            },
            hir::ExprKind::Name { value } => ExprKind::Local {
                local: self.lookup(value)?,
                name: value.clone(),
            },
            hir::ExprKind::Unary { op, expr } => ExprKind::Unary {
                op: *op,
                expr: Box::new(self.lower_expr(expr)?),
            },
            hir::ExprKind::Binary { op, left, right } => ExprKind::Binary {
                op: *op,
                left: Box::new(self.lower_expr(left)?),
                right: Box::new(self.lower_expr(right)?),
            },
            hir::ExprKind::Call {
                function,
                arguments,
            } => ExprKind::Call {
                function: function.clone(),
                arguments: arguments
                    .iter()
                    .map(|argument| self.lower_expr(argument))
                    .collect::<Result<Vec<_>, _>>()?,
            },
            hir::ExprKind::StructLiteral { name, fields } => ExprKind::StructLiteral {
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
                    .collect::<Result<Vec<_>, String>>()?,
            },
            hir::ExprKind::EnumVariant { enum_name, variant } => ExprKind::EnumVariant {
                enum_name: enum_name.clone(),
                variant: variant.clone(),
            },
            hir::ExprKind::Field { base, field } => ExprKind::Field {
                base: Box::new(self.lower_expr(base)?),
                field: field.clone(),
            },
        };

        Ok(Expr {
            kind,
            span: expr.span,
        })
    }

    fn allocate_local(
        &mut self,
        name: &str,
        ty: &Type,
        mutable: bool,
        kind: LocalKind,
        span: Span,
    ) -> u32 {
        let id = self.locals.len() as u32;
        self.locals.push(Local {
            id,
            kind,
            name: name.to_string(),
            ty: ty.clone(),
            mutable,
            span,
        });
        id
    }

    fn declare(&mut self, name: &str, local: u32) {
        self.scopes
            .last_mut()
            .expect("scope must exist")
            .insert(name.to_string(), local);
    }

    fn lookup(&self, name: &str) -> Result<u32, String> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
            .ok_or_else(|| format!("internal MIR lowering error: unresolved local `{name}`"))
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn new_block(&mut self, span: Span) -> u32 {
        let id = self.blocks.len() as u32;
        self.blocks.push(BasicBlockBuilder {
            id,
            span,
            statements: Vec::new(),
            terminator: None,
        });
        id
    }

    fn push_statement(&mut self, block: u32, statement: Statement) {
        self.blocks[block as usize].statements.push(statement);
    }

    fn set_terminator(&mut self, block: u32, terminator: Terminator) {
        let block = &mut self.blocks[block as usize];
        debug_assert!(
            block.terminator.is_none(),
            "basic block terminator already set"
        );
        block.terminator = Some(terminator);
    }

    fn block_is_terminated(&self, block: u32) -> bool {
        self.blocks[block as usize].terminator.is_some()
    }

    fn finish(self) -> (Vec<Local>, Vec<BasicBlock>) {
        let blocks = self
            .blocks
            .into_iter()
            .map(|block| BasicBlock {
                id: block.id,
                span: block.span,
                statements: block.statements,
                terminator: block.terminator.unwrap_or(Terminator {
                    kind: TerminatorKind::Unreachable,
                    span: block.span,
                }),
            })
            .collect();

        (self.locals, blocks)
    }
}

fn goto(target: u32, span: Span) -> Terminator {
    Terminator {
        kind: TerminatorKind::Goto { target },
        span,
    }
}

#[cfg(test)]
mod tests {
    use super::{ExprKind, ItemKind, LocalKind, StatementKind, TerminatorKind, lower_program};
    use crate::hir::lower_program as lower_hir_program;
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
            "semantic diagnostics must be empty before MIR lowering: {:?}",
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>()
        );

        let hir = lower_hir_program(&source, &parsed.program).expect("HIR lowering should succeed");
        lower_program(&hir).expect("MIR lowering should succeed")
    }

    #[test]
    fn lowers_for_loop_into_basic_block_cfg() {
        let program = lower(
            "\
fn main() -> i32 {
    let mut total: i32 = 0;
    for (let mut i: i32 = 0; i < 3; i = i + 1) {
        total = total + i;
    }
    println(total);
    return total;
}
",
        );

        let ItemKind::Function {
            entry_block,
            locals,
            blocks,
            ..
        } = &program.items[0].kind
        else {
            panic!("expected function item");
        };

        assert_eq!(*entry_block, 0);
        assert_eq!(locals.len(), 2);
        assert!(
            locals
                .iter()
                .any(|local| local.name == "total" && local.kind == LocalKind::Local)
        );
        assert!(
            locals
                .iter()
                .any(|local| local.name == "i" && local.kind == LocalKind::Local)
        );

        assert!(matches!(
            blocks[0].statements[0].kind,
            StatementKind::Let { .. }
        ));
        assert!(matches!(
            blocks[0].statements[1].kind,
            StatementKind::Let { .. }
        ));
        assert!(matches!(
            blocks[0].terminator.kind,
            TerminatorKind::Goto { target: 1 }
        ));
        assert!(matches!(
            blocks[1].terminator.kind,
            TerminatorKind::Branch {
                then_block: 2,
                else_block: 3,
                ..
            }
        ));
        assert!(matches!(
            blocks[2].terminator.kind,
            TerminatorKind::Goto { target: 1 }
        ));
        assert!(
            blocks
                .iter()
                .any(|block| matches!(block.terminator.kind, TerminatorKind::Return { .. }))
        );
    }

    #[test]
    fn resolves_shadowed_bindings_to_distinct_locals() {
        let program = lower(
            "\
fn main() -> i32 {
    let value: i32 = 1;
    {
        let value: i32 = 2;
        println(value);
    }
    println(value);
    return value;
}
",
        );

        let ItemKind::Function { blocks, .. } = &program.items[0].kind else {
            panic!("expected function item");
        };

        let mut printed_locals = Vec::new();
        for block in blocks {
            for statement in &block.statements {
                let StatementKind::Eval { expr } = &statement.kind else {
                    continue;
                };
                let ExprKind::Call {
                    function,
                    arguments,
                } = &expr.kind
                else {
                    continue;
                };
                if function != "println" {
                    continue;
                }
                let ExprKind::Local { local, .. } = arguments[0].kind else {
                    panic!("println should lower to a local argument");
                };
                printed_locals.push(local);
            }
        }

        assert_eq!(printed_locals.len(), 2);
        assert_ne!(printed_locals[0], printed_locals[1]);

        let returned_local = blocks
            .iter()
            .find_map(|block| match &block.terminator.kind {
                TerminatorKind::Return { value } => match value.kind {
                    ExprKind::Local { local, .. } => Some(local),
                    _ => None,
                },
                _ => None,
            })
            .expect("return terminator should exist");

        assert_eq!(printed_locals[1], returned_local);
    }
}
