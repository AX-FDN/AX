use super::*;

impl FunctionLowerer {
    pub(in crate::mir) fn lower_block(
        &mut self,
        block: &hir::Block,
        current: u32,
    ) -> Result<u32, String> {
        self.push_scope();
        let mut current = current;
        for statement in &block.statements {
            current = self.lower_statement(statement, current)?;
        }
        self.pop_scope();
        Ok(current)
    }

    pub(in crate::mir) fn lower_statement(
        &mut self,
        statement: &hir::Stmt,
        current: u32,
    ) -> Result<u32, String> {
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
                let target = self.lower_place(target)?;
                let value = self.lower_expr(value)?;
                self.push_statement(
                    current,
                    Statement {
                        kind: StatementKind::Assign { target, value },
                        span: statement.span,
                    },
                );
                Ok(current)
            }
            hir::StmtKind::Break => {
                let Some(loop_targets) = self.loop_stack.last().copied() else {
                    return Err("internal MIR lowering error: `break` used outside loop".into());
                };
                self.set_terminator(current, goto(loop_targets.break_target, statement.span));
                Ok(self.new_block(statement.span))
            }
            hir::StmtKind::Continue => {
                let Some(loop_targets) = self.loop_stack.last().copied() else {
                    return Err("internal MIR lowering error: `continue` used outside loop".into());
                };
                self.set_terminator(current, goto(loop_targets.continue_target, statement.span));
                Ok(self.new_block(statement.span))
            }
            hir::StmtKind::Expr { expr } => {
                let expr = self.lower_expr(expr)?;
                self.push_statement(
                    current,
                    Statement {
                        kind: StatementKind::Eval { expr },
                        span: statement.span,
                    },
                );
                Ok(current)
            }
            hir::StmtKind::Return { value } => {
                let value = self.lower_expr(value)?;
                self.set_terminator(
                    current,
                    Terminator {
                        kind: TerminatorKind::Return { value },
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
                let condition = self.lower_expr(condition)?;

                self.set_terminator(
                    current,
                    Terminator {
                        kind: TerminatorKind::Branch {
                            condition,
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
                let condition_span = condition.span;
                let condition_block = self.new_block(condition_span);
                let body_block = self.new_block(body.span);
                let exit_block = self.new_block(statement.span);
                let lowered_condition = self.lower_expr(condition)?;

                self.set_terminator(current, goto(condition_block, statement.span));
                self.set_terminator(
                    condition_block,
                    Terminator {
                        kind: TerminatorKind::Branch {
                            condition: lowered_condition,
                            then_block: body_block,
                            else_block: exit_block,
                        },
                        span: condition_span,
                    },
                );

                self.loop_stack.push(LoopTargets {
                    break_target: exit_block,
                    continue_target: condition_block,
                });
                let body_exit = self.lower_block(body, body_block)?;
                self.loop_stack.pop();
                if !self.block_is_terminated(body_exit) {
                    self.set_terminator(body_exit, goto(condition_block, body.span));
                }

                Ok(exit_block)
            }
            hir::StmtKind::Block { block } => self.lower_block(block, current),
        }
    }
}
