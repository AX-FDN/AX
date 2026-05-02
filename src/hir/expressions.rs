use super::*;

impl<'a> LoweringContext<'a> {
    pub(in crate::hir) fn lower_place(&self, expr: &ast::Expr) -> Result<Place, Diagnostic> {
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

    pub(in crate::hir) fn lower_expr(&self, expr: &ast::Expr) -> Result<Expr, Diagnostic> {
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
            ast::ExprKind::Try { expr: inner } => ExprKind::Try {
                expr: Box::new(self.lower_expr(inner)?),
            },
            ast::ExprKind::Binary { op, left, right } => ExprKind::Binary {
                op: *op,
                left: Box::new(self.lower_expr(left)?),
                right: Box::new(self.lower_expr(right)?),
            },
            ast::ExprKind::Call { callee, arguments } => {
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
                } else if let Some(enum_variant) =
                    self.try_lower_enum_variant_constructor(callee, arguments)?
                {
                    enum_variant
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
            ast::ExprKind::Block { statements, value } => ExprKind::Block {
                statements: statements
                    .iter()
                    .map(|statement| self.lower_statement(statement))
                    .collect::<Result<Vec<_>, _>>()?,
                value: Box::new(self.lower_expr(value)?),
            },
            ast::ExprKind::Match { scrutinee, arms } => ExprKind::Match {
                scrutinee: Box::new(self.lower_expr(scrutinee)?),
                arms: arms
                    .iter()
                    .map(|arm| {
                        Ok(MatchExprArm {
                            pattern: self.lower_match_pattern(&arm.pattern)?,
                            guard: arm
                                .guard
                                .as_ref()
                                .map(|guard| self.lower_expr(guard))
                                .transpose()?,
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

    pub(in crate::hir) fn try_lower_enum_variant_constructor(
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
}
