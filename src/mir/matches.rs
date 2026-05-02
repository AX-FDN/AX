use super::*;

impl FunctionLowerer {
    pub(in crate::mir) fn lower_match_pattern(&self, pattern: &hir::MatchPattern) -> MatchPattern {
        let kind = match &pattern.kind {
            hir::MatchPatternKind::Wildcard => MatchPatternKind::Wildcard,
            hir::MatchPatternKind::Binding { name } => {
                MatchPatternKind::Binding { name: name.clone() }
            }
            hir::MatchPatternKind::Bool { value } => MatchPatternKind::Bool { value: *value },
            hir::MatchPatternKind::Int { value } => MatchPatternKind::Int { value: *value },
            hir::MatchPatternKind::IntRange { start, end } => MatchPatternKind::IntRange {
                start: *start,
                end: *end,
            },
            hir::MatchPatternKind::String { value } => MatchPatternKind::String {
                value: value.clone(),
            },
            hir::MatchPatternKind::EnumVariant {
                enum_name,
                variant,
                payload,
                payload_type,
            } => MatchPatternKind::EnumVariant {
                enum_name: enum_name.clone(),
                variant: variant.clone(),
                payload: payload.clone(),
                payload_type: payload_type.clone(),
            },
            hir::MatchPatternKind::Struct {
                struct_name,
                fields,
            } => MatchPatternKind::Struct {
                struct_name: struct_name.clone(),
                fields: fields
                    .iter()
                    .map(|field| StructPatternField {
                        name: field.name.clone(),
                        binding: field.binding.clone(),
                        ty: field.ty.clone(),
                        span: field.span,
                    })
                    .collect(),
            },
            hir::MatchPatternKind::Or { alternatives } => MatchPatternKind::Or {
                alternatives: alternatives
                    .iter()
                    .map(|pattern| self.lower_match_pattern(pattern))
                    .collect(),
            },
            hir::MatchPatternKind::Error => MatchPatternKind::Error,
        };

        MatchPattern {
            kind,
            span: pattern.span,
        }
    }

    pub(in crate::mir) fn infer_match_scrutinee_type(
        &self,
        arms: &[hir::MatchExprArm],
        span: Span,
    ) -> Result<Type, String> {
        for arm in arms {
            match &arm.pattern.kind {
                hir::MatchPatternKind::Bool { .. } => return Ok(Type::Bool),
                hir::MatchPatternKind::Int { .. } => return Ok(Type::I32),
                hir::MatchPatternKind::IntRange { .. } => return Ok(Type::I32),
                hir::MatchPatternKind::String { .. } => return Ok(Type::String),
                hir::MatchPatternKind::EnumVariant { enum_name, .. } => {
                    return Ok(Type::Enum {
                        name: enum_name.clone(),
                    });
                }
                hir::MatchPatternKind::Struct { struct_name, .. } => {
                    return Ok(Type::Struct {
                        name: struct_name.clone(),
                    });
                }
                hir::MatchPatternKind::Or { alternatives } => {
                    for alternative in alternatives {
                        match &alternative.kind {
                            hir::MatchPatternKind::Bool { .. } => return Ok(Type::Bool),
                            hir::MatchPatternKind::Int { .. } => return Ok(Type::I32),
                            hir::MatchPatternKind::IntRange { .. } => return Ok(Type::I32),
                            hir::MatchPatternKind::String { .. } => return Ok(Type::String),
                            hir::MatchPatternKind::EnumVariant { enum_name, .. } => {
                                return Ok(Type::Enum {
                                    name: enum_name.clone(),
                                });
                            }
                            hir::MatchPatternKind::Struct { struct_name, .. } => {
                                return Ok(Type::Struct {
                                    name: struct_name.clone(),
                                });
                            }
                            _ => {}
                        }
                    }
                }
                hir::MatchPatternKind::Wildcard
                | hir::MatchPatternKind::Binding { .. }
                | hir::MatchPatternKind::Error => {}
            }
        }

        Err(format!(
            "internal MIR lowering error: cannot infer match input type at {}..{} without a concrete pattern",
            span.start, span.end
        ))
    }

    pub(in crate::mir) fn match_pattern_bindings<'a>(
        pattern: &'a hir::MatchPattern,
        scrutinee_ty: &'a Type,
    ) -> Vec<(&'a str, Type, Span)> {
        match &pattern.kind {
            hir::MatchPatternKind::Binding { name } => {
                vec![(name.as_str(), scrutinee_ty.clone(), pattern.span)]
            }
            hir::MatchPatternKind::EnumVariant {
                payload: Some(EnumVariantPayloadPattern::Binding { name }),
                payload_type: Some(payload_type),
                ..
            } => vec![(name.as_str(), payload_type.clone(), pattern.span)],
            hir::MatchPatternKind::Struct { fields, .. } => fields
                .iter()
                .map(|field| (field.binding.as_str(), field.ty.clone(), field.span))
                .collect(),
            _ => Vec::new(),
        }
    }
}
