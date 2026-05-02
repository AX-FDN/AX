use super::*;

impl<'a> LoweringContext<'a> {
    pub(in crate::hir) fn lower_match_statement(
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

    pub(in crate::hir) fn lower_match_arm_chain(
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

        if first.guard.is_none()
            && matches!(
                first.pattern.kind,
                ast::MatchPatternKind::Wildcard | ast::MatchPatternKind::Binding { .. }
            )
        {
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
        let condition = self.lower_match_pattern_condition(temp_name, &first.pattern)?;
        let then_branch = if let Some(guard) = &first.guard {
            self.lower_guarded_match_arm_block(
                temp_name,
                temp_type,
                first,
                guard,
                else_branch.clone(),
            )?
        } else {
            self.lower_match_arm_block(temp_name, temp_type, &first.pattern, &first.body)?
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

    pub(in crate::hir) fn lower_match_pattern_condition(
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

    pub(in crate::hir) fn lower_match_pattern(
        &self,
        pattern: &ast::MatchPattern,
    ) -> Result<MatchPattern, Diagnostic> {
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
            ast::MatchPatternKind::IntRange { start, end } => MatchPatternKind::IntRange {
                start: i32::try_from(*start).map_err(|_| {
                    self.lowering_error(
                        "H0017",
                        "match range pattern start must fit in i32",
                        pattern.span,
                    )
                })?,
                end: i32::try_from(*end).map_err(|_| {
                    self.lowering_error(
                        "H0018",
                        "match range pattern end must fit in i32",
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
            ast::MatchPatternKind::Struct { path, fields } => {
                let struct_name = self
                    .resolve_canonical_name(path, pattern.span, &self.struct_names)
                    .unwrap_or_else(|| path.clone());
                MatchPatternKind::Struct {
                    struct_name: struct_name.clone(),
                    fields: fields
                        .iter()
                        .map(|field| {
                            Ok(StructPatternField {
                                name: field.name.clone(),
                                binding: field.binding.clone(),
                                ty: self.resolve_struct_pattern_field_type(
                                    &struct_name,
                                    &field.name,
                                    pattern.span,
                                    None,
                                )?,
                                span: field.span,
                            })
                        })
                        .collect::<Result<Vec<_>, Diagnostic>>()?,
                }
            }
            ast::MatchPatternKind::Or { alternatives } => MatchPatternKind::Or {
                alternatives: alternatives
                    .iter()
                    .map(|pattern| self.lower_match_pattern(pattern))
                    .collect::<Result<Vec<_>, Diagnostic>>()?,
            },
            ast::MatchPatternKind::Error => MatchPatternKind::Error,
        };

        Ok(MatchPattern {
            kind,
            span: pattern.span,
        })
    }

    pub(in crate::hir) fn infer_match_scrutinee_type(
        &self,
        arms: &[ast::MatchArm],
        span: Span,
    ) -> Result<Type, Diagnostic> {
        for arm in arms {
            match &arm.pattern.kind {
                ast::MatchPatternKind::Bool { .. } => return Ok(Type::Bool),
                ast::MatchPatternKind::Int { .. } => return Ok(Type::I32),
                ast::MatchPatternKind::IntRange { .. } => return Ok(Type::I32),
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
                ast::MatchPatternKind::Struct { path, .. } => {
                    let struct_name = self
                        .resolve_canonical_name(path, arm.pattern.span, &self.struct_names)
                        .unwrap_or_else(|| path.clone());
                    return Ok(Type::Struct { name: struct_name });
                }
                ast::MatchPatternKind::Or { alternatives } => {
                    for alternative in alternatives {
                        match &alternative.kind {
                            ast::MatchPatternKind::Bool { .. } => return Ok(Type::Bool),
                            ast::MatchPatternKind::Int { .. } => return Ok(Type::I32),
                            ast::MatchPatternKind::IntRange { .. } => return Ok(Type::I32),
                            ast::MatchPatternKind::String { .. } => return Ok(Type::String),
                            ast::MatchPatternKind::EnumVariant { path, .. } => {
                                let Some((enum_path, _)) = path.rsplit_once('.') else {
                                    return Err(self.lowering_error(
                                        "H0012",
                                        "match enum pattern must use `EnumName.Variant`",
                                        alternative.span,
                                    ));
                                };
                                let enum_name = self
                                    .resolve_canonical_name(
                                        enum_path,
                                        alternative.span,
                                        &self.enum_names,
                                    )
                                    .unwrap_or_else(|| enum_path.to_string());
                                return Ok(Type::Enum { name: enum_name });
                            }
                            ast::MatchPatternKind::Struct { path, .. } => {
                                let struct_name = self
                                    .resolve_canonical_name(
                                        path,
                                        alternative.span,
                                        &self.struct_names,
                                    )
                                    .unwrap_or_else(|| path.clone());
                                return Ok(Type::Struct { name: struct_name });
                            }
                            _ => {}
                        }
                    }
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

    pub(in crate::hir) fn lower_match_arm_block(
        &self,
        temp_name: &str,
        temp_type: &Type,
        pattern: &ast::MatchPattern,
        body: &ast::Block,
    ) -> Result<Block, Diagnostic> {
        let body_block = self.lower_block(body)?;
        let binding_statements =
            self.lower_match_binding_statements(temp_name, temp_type, pattern)?;
        if !binding_statements.is_empty() {
            return Ok(Block {
                statements: binding_statements
                    .into_iter()
                    .chain(std::iter::once(Stmt {
                        kind: StmtKind::Block { block: body_block },
                        span: body.span,
                    }))
                    .collect(),
                span: body.span,
            });
        }
        Ok(body_block)
    }

    pub(in crate::hir) fn lower_guarded_match_arm_block(
        &self,
        temp_name: &str,
        temp_type: &Type,
        arm: &ast::MatchArm,
        guard: &ast::Expr,
        fallback: Option<Block>,
    ) -> Result<Block, Diagnostic> {
        let mut statements =
            self.lower_match_binding_statements(temp_name, temp_type, &arm.pattern)?;
        let guard = self.lower_expr(guard)?;
        let then_branch = self.lower_block(&arm.body)?;
        let else_branch = fallback.or_else(|| {
            Some(Block {
                statements: Vec::new(),
                span: arm.span,
            })
        });
        statements.push(Stmt {
            kind: StmtKind::If {
                condition: guard,
                then_branch,
                else_branch,
            },
            span: arm.span,
        });
        Ok(Block {
            statements,
            span: arm.body.span,
        })
    }

    pub(in crate::hir) fn lower_match_binding_statements(
        &self,
        temp_name: &str,
        temp_type: &Type,
        pattern: &ast::MatchPattern,
    ) -> Result<Vec<Stmt>, Diagnostic> {
        match &pattern.kind {
            ast::MatchPatternKind::Binding { name } => Ok(vec![Stmt {
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
            }]),
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
                Ok(vec![Stmt {
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
                }])
            }
            ast::MatchPatternKind::Struct { path, fields } => {
                let struct_name = self
                    .resolve_canonical_name(path, pattern.span, &self.struct_names)
                    .unwrap_or_else(|| path.clone());
                fields
                    .iter()
                    .map(|field| {
                        Ok(Stmt {
                            kind: StmtKind::Let {
                                mutable: false,
                                name: field.binding.clone(),
                                ty: self.resolve_struct_pattern_field_type(
                                    &struct_name,
                                    &field.name,
                                    pattern.span,
                                    Some(temp_type),
                                )?,
                                initializer: Expr {
                                    kind: ExprKind::Field {
                                        base: Box::new(Expr {
                                            kind: ExprKind::Name {
                                                value: temp_name.to_string(),
                                            },
                                            span: pattern.span,
                                        }),
                                        field: field.name.clone(),
                                    },
                                    span: field.span,
                                },
                            },
                            span: field.span,
                        })
                    })
                    .collect()
            }
            _ => Ok(Vec::new()),
        }
    }

    pub(in crate::hir) fn resolve_enum_variant_payload_type(
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

    pub(in crate::hir) fn resolve_struct_pattern_field_type(
        &self,
        struct_name: &str,
        field_name: &str,
        span: Span,
        scrutinee_type: Option<&Type>,
    ) -> Result<Type, Diagnostic> {
        let Some((type_params, fields)) = self.struct_fields.get(struct_name) else {
            return Err(self.lowering_error(
                "H0019",
                format!("cannot find field metadata for struct `{struct_name}`"),
                span,
            ));
        };
        let Some(field_type) = fields.get(field_name) else {
            return Err(self.lowering_error(
                "H0020",
                format!("struct `{struct_name}` does not contain field `{field_name}`"),
                span,
            ));
        };
        Ok(substitute_struct_field_type(
            field_type.clone(),
            type_params.as_slice(),
            scrutinee_type,
        ))
    }
}
