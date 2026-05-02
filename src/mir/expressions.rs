use super::*;

impl FunctionLowerer {
    pub(in crate::mir) fn lower_place(&mut self, place: &hir::Place) -> Result<Place, String> {
        let kind = match &place.kind {
            hir::PlaceKind::Local { name } => PlaceKind::Local {
                local: self.lookup(name)?,
                name: name.clone(),
            },
            hir::PlaceKind::Field { base, field } => PlaceKind::Field {
                base: Box::new(self.lower_place(base)?),
                field: field.clone(),
            },
            hir::PlaceKind::Index { base, index } => PlaceKind::Index {
                base: Box::new(self.lower_place(base)?),
                index: self.lower_expr(index)?,
            },
        };

        Ok(Place {
            kind,
            span: place.span,
        })
    }

    pub(in crate::mir) fn lower_expr(&mut self, expr: &hir::Expr) -> Result<Expr, String> {
        let kind = match &expr.kind {
            hir::ExprKind::Int { value } => ExprKind::Int { value: *value },
            hir::ExprKind::Float { value } => ExprKind::Float { value: *value },
            hir::ExprKind::Bool { value } => ExprKind::Bool { value: *value },
            hir::ExprKind::String { value } => ExprKind::String {
                value: value.clone(),
            },
            hir::ExprKind::Name { value } => match self.lookup(value) {
                Ok(local) => ExprKind::Local {
                    local,
                    name: value.clone(),
                },
                Err(_) => ExprKind::Const {
                    name: value.clone(),
                },
            },
            hir::ExprKind::Unary { op, expr } => ExprKind::Unary {
                op: *op,
                expr: Box::new(self.lower_expr(expr)?),
            },
            hir::ExprKind::Try { expr } => ExprKind::Try {
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
            hir::ExprKind::MethodCall {
                receiver,
                method,
                arguments,
            } => {
                let mut lowered_arguments = Vec::with_capacity(arguments.len() + 1);
                lowered_arguments.push(self.lower_expr(receiver)?);
                for argument in arguments {
                    lowered_arguments.push(self.lower_expr(argument)?);
                }
                ExprKind::Call {
                    function: format!("<method>.{method}"),
                    arguments: lowered_arguments,
                }
            }
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
            hir::ExprKind::ArrayLiteral { elements } => ExprKind::ArrayLiteral {
                elements: elements
                    .iter()
                    .map(|element| self.lower_expr(element))
                    .collect::<Result<Vec<_>, _>>()?,
            },
            hir::ExprKind::Block { statements, value } => {
                self.push_scope();
                let statements = match self.lower_block_expr_statements(statements) {
                    Ok(statements) => statements,
                    Err(error) => {
                        self.pop_scope();
                        return Err(error);
                    }
                };
                let value = self.lower_expr(value);
                self.pop_scope();
                ExprKind::Block {
                    statements,
                    value: Box::new(value?),
                }
            }
            hir::ExprKind::Match { scrutinee, arms } => {
                let scrutinee_ty = self.infer_match_scrutinee_type(arms, expr.span)?;
                ExprKind::Match {
                    scrutinee: Box::new(self.lower_expr(scrutinee)?),
                    arms: arms
                        .iter()
                        .map(|arm| {
                            let pattern = self.lower_match_pattern(&arm.pattern);
                            self.push_scope();
                            for (binding_name, binding_ty, binding_span) in
                                Self::match_pattern_bindings(&arm.pattern, &scrutinee_ty)
                            {
                                let local = self.allocate_local(
                                    binding_name,
                                    &binding_ty,
                                    false,
                                    LocalKind::Local,
                                    binding_span,
                                );
                                self.declare(binding_name, local);
                            }
                            let guard = arm
                                .guard
                                .as_ref()
                                .map(|guard| self.lower_expr(guard))
                                .transpose()?;
                            let value = self.lower_expr(&arm.value);
                            self.pop_scope();

                            Ok(MatchExprArm {
                                pattern,
                                guard,
                                value: value?,
                                span: arm.span,
                            })
                        })
                        .collect::<Result<Vec<_>, String>>()?,
                }
            }
            hir::ExprKind::EnumVariant {
                enum_name,
                variant,
                payload,
            } => ExprKind::EnumVariant {
                enum_name: enum_name.clone(),
                variant: variant.clone(),
                payload: payload
                    .as_ref()
                    .map(|payload| self.lower_expr(payload))
                    .transpose()?
                    .map(Box::new),
            },
            hir::ExprKind::MatchTest { scrutinee, pattern } => ExprKind::MatchTest {
                scrutinee: Box::new(self.lower_expr(scrutinee)?),
                pattern: self.lower_match_pattern(pattern),
            },
            hir::ExprKind::EnumPayload { value } => ExprKind::EnumPayload {
                value: Box::new(self.lower_expr(value)?),
            },
            hir::ExprKind::Field { base, field } => ExprKind::Field {
                base: Box::new(self.lower_expr(base)?),
                field: field.clone(),
            },
            hir::ExprKind::Index { base, index } => ExprKind::Index {
                base: Box::new(self.lower_expr(base)?),
                index: Box::new(self.lower_expr(index)?),
            },
            hir::ExprKind::Slice { base, start, end } => ExprKind::Slice {
                base: Box::new(self.lower_expr(base)?),
                start: Box::new(self.lower_expr(start)?),
                end: Box::new(self.lower_expr(end)?),
            },
        };

        Ok(Expr {
            kind,
            span: expr.span,
        })
    }

    pub(in crate::mir) fn lower_block_expr_statements(
        &mut self,
        statements: &[hir::Stmt],
    ) -> Result<Vec<Statement>, String> {
        let mut lowered = Vec::new();
        for statement in statements {
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
                    lowered.push(Statement {
                        kind: StatementKind::Let {
                            local,
                            name: name.clone(),
                            mutable: *mutable,
                            ty: ty.clone(),
                            initializer,
                        },
                        span: statement.span,
                    });
                }
                hir::StmtKind::Assign { target, value } => {
                    lowered.push(Statement {
                        kind: StatementKind::Assign {
                            target: self.lower_place(target)?,
                            value: self.lower_expr(value)?,
                        },
                        span: statement.span,
                    });
                }
                hir::StmtKind::Expr { expr } => {
                    lowered.push(Statement {
                        kind: StatementKind::Eval {
                            expr: self.lower_expr(expr)?,
                        },
                        span: statement.span,
                    });
                }
                hir::StmtKind::Block { block } => {
                    self.push_scope();
                    lowered.extend(self.lower_block_expr_statements(&block.statements)?);
                    self.pop_scope();
                }
                hir::StmtKind::Return { .. }
                | hir::StmtKind::Break
                | hir::StmtKind::Continue
                | hir::StmtKind::If { .. }
                | hir::StmtKind::While { .. } => {
                    return Err(format!(
                        "internal MIR lowering error: block-valued match arms currently lower only let, assignment, expression, and nested block statements at {}..{}",
                        statement.span.start, statement.span.end
                    ));
                }
            }
        }
        Ok(lowered)
    }
}
