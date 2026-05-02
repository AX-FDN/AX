use super::*;

impl<'a> Interpreter<'a> {
    pub(in crate::interpreter) fn exec_block(
        &mut self,
        block: &Block,
        frame: &mut Frame,
    ) -> Result<ControlFlow, Diagnostic> {
        frame.scopes.push(HashMap::new());
        for statement in &block.statements {
            match self.exec_statement(statement, frame)? {
                ControlFlow::Continue => {}
                ControlFlow::Break => {
                    frame.scopes.pop();
                    return Ok(ControlFlow::Break);
                }
                ControlFlow::LoopContinue => {
                    frame.scopes.pop();
                    return Ok(ControlFlow::LoopContinue);
                }
                ControlFlow::Return(value) => {
                    frame.scopes.pop();
                    return Ok(ControlFlow::Return(value));
                }
            }
        }
        frame.scopes.pop();
        Ok(ControlFlow::Continue)
    }

    pub(in crate::interpreter) fn exec_statement(
        &mut self,
        statement: &Stmt,
        frame: &mut Frame,
    ) -> Result<ControlFlow, Diagnostic> {
        match &statement.kind {
            StmtKind::Let {
                mutable,
                name,
                initializer,
                ..
            } => {
                let value = match self.eval_expr(initializer, frame)? {
                    EvalFlow::Value(value) => value,
                    EvalFlow::Return(value) => return Ok(ControlFlow::Return(value)),
                };
                frame.scopes.last_mut().expect("scope should exist").insert(
                    name.clone(),
                    Slot {
                        mutable: *mutable,
                        value,
                    },
                );
                Ok(ControlFlow::Continue)
            }
            StmtKind::Assign { target, value } => {
                let next_value = match self.eval_expr(value, frame)? {
                    EvalFlow::Value(value) => value,
                    EvalFlow::Return(value) => return Ok(ControlFlow::Return(value)),
                };
                self.assign_target(frame, target, next_value)?;
                Ok(ControlFlow::Continue)
            }
            StmtKind::Break => Ok(ControlFlow::Break),
            StmtKind::Continue => Ok(ControlFlow::LoopContinue),
            StmtKind::Expr { expr } => match self.eval_expr(expr, frame)? {
                EvalFlow::Value(_) => Ok(ControlFlow::Continue),
                EvalFlow::Return(value) => Ok(ControlFlow::Return(value)),
            },
            StmtKind::Return { value } => {
                let value = match self.eval_expr(value, frame)? {
                    EvalFlow::Value(value) => value,
                    EvalFlow::Return(value) => value,
                };
                Ok(ControlFlow::Return(value))
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => match self.eval_condition(condition, frame)? {
                ConditionFlow::Return(value) => Ok(ControlFlow::Return(value)),
                ConditionFlow::Value(true) => self.exec_block(then_branch, frame),
                ConditionFlow::Value(false) => {
                    if let Some(block) = else_branch {
                        self.exec_block(block, frame)
                    } else {
                        Ok(ControlFlow::Continue)
                    }
                }
            },
            StmtKind::While { condition, body } => {
                loop {
                    match self.eval_condition(condition, frame)? {
                        ConditionFlow::Return(value) => return Ok(ControlFlow::Return(value)),
                        ConditionFlow::Value(true) => {}
                        ConditionFlow::Value(false) => break,
                    }
                    match self.exec_block(body, frame)? {
                        ControlFlow::Continue => {}
                        ControlFlow::Break => break,
                        ControlFlow::LoopContinue => continue,
                        ControlFlow::Return(value) => {
                            return Ok(ControlFlow::Return(value));
                        }
                    }
                }
                Ok(ControlFlow::Continue)
            }
            StmtKind::Block { block } => self.exec_block(block, frame),
        }
    }

    pub(in crate::interpreter) fn eval_condition(
        &mut self,
        expr: &Expr,
        frame: &mut Frame,
    ) -> Result<ConditionFlow, Diagnostic> {
        match self.eval_expr(expr, frame)? {
            EvalFlow::Return(value) => Ok(ConditionFlow::Return(value)),
            EvalFlow::Value(Value::Bool(value)) => Ok(ConditionFlow::Value(value)),
            EvalFlow::Value(other) => Err(self.runtime_error(
                "R0009",
                format!(
                    "condition must evaluate to `bool`, got `{}`",
                    other.display()
                ),
                expr.span,
            )),
        }
    }
}
