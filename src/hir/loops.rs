use super::*;

impl<'a> LoweringContext<'a> {
    pub(in crate::hir) fn lower_for_statement(
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

    pub(in crate::hir) fn lower_for_in_statement(
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

    pub(in crate::hir) fn finish_lowered_for_block(
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

    pub(in crate::hir) fn rewrite_for_continues(block: &mut Block, step: &Stmt) {
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
}
