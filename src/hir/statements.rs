use super::*;

impl<'a> LoweringContext<'a> {
    pub(in crate::hir) fn lower_block(&self, block: &ast::Block) -> Result<Block, Diagnostic> {
        Ok(Block {
            statements: block
                .statements
                .iter()
                .map(|statement| self.lower_statement(statement))
                .collect::<Result<Vec<_>, _>>()?,
            span: block.span,
        })
    }

    pub(in crate::hir) fn lower_statement(
        &self,
        statement: &ast::Stmt,
    ) -> Result<Stmt, Diagnostic> {
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
}
