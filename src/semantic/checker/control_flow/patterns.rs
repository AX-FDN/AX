use super::helpers::pattern_label;
use super::*;

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn resolve_match_pattern(
        &mut self,
        pattern: &MatchPattern,
        scrutinee_type: &Type,
    ) -> Option<ResolvedMatchPattern> {
        match &pattern.kind {
            MatchPatternKind::Wildcard
            | MatchPatternKind::Binding { .. }
            | MatchPatternKind::Or { .. }
            | MatchPatternKind::Error => None,
            MatchPatternKind::Bool { value } => {
                if !matches!(scrutinee_type, Type::Bool | Type::Error) {
                    self.report_match_pattern_type_mismatch(pattern, scrutinee_type);
                    return None;
                }
                Some(ResolvedMatchPattern::Bool(*value))
            }
            MatchPatternKind::Int { value } => {
                let Ok(value) = i32::try_from(*value) else {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0009",
                            "integer literal is out of range for `i32`",
                            self.info.source,
                            pattern.span,
                        )
                        .with_suggestion("use a value that fits in the AX `i32` range"),
                    );
                    return None;
                };
                if !matches!(scrutinee_type, Type::I32 | Type::Error) {
                    self.report_match_pattern_type_mismatch(pattern, scrutinee_type);
                    return None;
                }
                Some(ResolvedMatchPattern::Int(value))
            }
            MatchPatternKind::IntRange { start, end } => {
                if i32::try_from(*start).is_err() || i32::try_from(*end).is_err() {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0009",
                            "integer range pattern bound is out of range for `i32`",
                            self.info.source,
                            pattern.span,
                        )
                        .with_suggestion("use range bounds that fit in the AX `i32` range"),
                    );
                    return None;
                }
                if start > end {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0056",
                            format!("empty match range pattern `{start}..={end}`"),
                            self.info.source,
                            pattern.span,
                        )
                        .with_kind(DiagnosticKind::MatchRangeMustBeNonEmpty)
                        .with_suggestion(
                            "make the start bound less than or equal to the end bound",
                        ),
                    );
                    return None;
                }
                if !matches!(scrutinee_type, Type::I32 | Type::Error) {
                    self.report_match_pattern_type_mismatch(pattern, scrutinee_type);
                }
                None
            }
            MatchPatternKind::String { value } => {
                if !matches!(scrutinee_type, Type::String | Type::Error) {
                    self.report_match_pattern_type_mismatch(pattern, scrutinee_type);
                    return None;
                }
                Some(ResolvedMatchPattern::String(value.clone()))
            }
            MatchPatternKind::EnumVariant { path, payload } => {
                let Some((enum_path, variant)) = path.rsplit_once('.') else {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0046",
                            format!(
                                "match pattern `{path}` must be a literal, `_`, or `EnumName.Variant`",
                            ),
                            self.info.source,
                            pattern.span,
                        )
                        .with_kind(DiagnosticKind::MatchPatternTypeMismatch)
                        .with_suggestion(
                            "rewrite this pattern as `true`, `false`, an integer literal, `_`, or `EnumName.Variant`",
                        ),
                    );
                    return None;
                };

                let current_unit_path = self.current_unit_path().to_string();
                let Some(resolved_key) = self.info.resolve_named_type_key(
                    enum_path,
                    &current_unit_path,
                    pattern.span,
                    self.diagnostics,
                ) else {
                    return None;
                };
                let Some(Type::Enum(enum_name)) = self.info.named_types.get(&resolved_key).cloned()
                else {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0046",
                            format!("match pattern `{path}` does not name an enum variant"),
                            self.info.source,
                            pattern.span,
                        )
                        .with_kind(DiagnosticKind::MatchPatternTypeMismatch)
                        .with_suggestion("use a real enum variant like `Flag.On`"),
                    );
                    return None;
                };

                let Some(enum_info) = self.info.enums.get(&enum_name) else {
                    return None;
                };
                let Some(variant_info) = enum_info.variants.get(variant) else {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0029",
                            format!("unknown enum variant `{variant}` for enum `{enum_name}`"),
                            self.info.source,
                            pattern.span,
                        )
                        .with_suggestion("use one of the declared enum variants"),
                    );
                    return None;
                };

                if !matches!(scrutinee_type, Type::Enum(name) if name == &enum_name)
                    && !matches!(scrutinee_type, Type::EnumInstance { name, .. } if name == &enum_name)
                    && !matches!(scrutinee_type, Type::Error)
                {
                    self.report_match_pattern_type_mismatch(pattern, scrutinee_type);
                    return None;
                }

                match (&variant_info.payload, payload) {
                    (Some(_), None) => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0055",
                                format!(
                                    "match pattern `{path}` must bind or ignore the payload for enum variant `{enum_name}.{variant}`"
                                ),
                                self.info.source,
                                pattern.span,
                            )
                            .with_kind(DiagnosticKind::MatchEnumVariantPayloadShapeMismatch)
                            .with_note(
                                "payload enum variants must appear in patterns as `EnumName.Variant(name)` or `EnumName.Variant(_)` in the current AX slice",
                            )
                            .with_suggestion(format!(
                                "rewrite this arm as `{path}(value) => ...` or `{path}(_) => ...`",
                            )),
                        );
                    }
                    (None, Some(_)) => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0055",
                                format!(
                                    "match pattern `{}` cannot bind a payload because enum variant `{enum_name}.{variant}` has no payload",
                                    pattern_label(pattern)
                                ),
                                self.info.source,
                                pattern.span,
                            )
                            .with_kind(DiagnosticKind::MatchEnumVariantPayloadShapeMismatch)
                            .with_suggestion(format!(
                                "rewrite this arm as `{path} => ...` without payload binding",
                            )),
                        );
                    }
                    _ => {}
                }

                Some(ResolvedMatchPattern::EnumVariant {
                    variant: variant.to_string(),
                })
            }
            MatchPatternKind::Struct { path, fields } => {
                let current_unit_path = self.current_unit_path().to_string();
                let Some(resolved_key) = self.info.resolve_named_type_key(
                    path,
                    &current_unit_path,
                    pattern.span,
                    self.diagnostics,
                ) else {
                    return None;
                };
                let Some(Type::Struct(struct_name)) =
                    self.info.named_types.get(&resolved_key).cloned()
                else {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0046",
                            format!("match pattern `{path} {{ ... }}` does not name a struct"),
                            self.info.source,
                            pattern.span,
                        )
                        .with_kind(DiagnosticKind::MatchPatternTypeMismatch)
                        .with_suggestion("use a real struct name for struct destructuring"),
                    );
                    return None;
                };

                if !matches!(scrutinee_type, Type::Struct(name) if name == &struct_name)
                    && !matches!(scrutinee_type, Type::StructInstance { name, .. } if name == &struct_name)
                    && !matches!(scrutinee_type, Type::Error)
                {
                    self.report_match_pattern_type_mismatch(pattern, scrutinee_type);
                    return None;
                }

                self.validate_struct_pattern_fields(pattern, &struct_name, fields);
                Some(ResolvedMatchPattern::Struct { name: struct_name })
            }
        }
    }

    fn validate_struct_pattern_fields(
        &mut self,
        pattern: &MatchPattern,
        struct_name: &str,
        fields: &[crate::ast::StructPatternField],
    ) {
        let Some(struct_info) = self.info.structs.get(struct_name) else {
            return;
        };
        let mut seen = HashSet::new();
        for field in fields {
            if !seen.insert(field.name.clone()) {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0060",
                        format!(
                            "struct pattern `{}` lists field `{}` more than once",
                            pattern_label(pattern),
                            field.name
                        ),
                        self.info.source,
                        field.span,
                    )
                    .with_kind(DiagnosticKind::MatchStructPatternShapeMismatch)
                    .with_suggestion("remove the duplicate field from the struct pattern"),
                );
            }
            if !struct_info.fields.contains_key(&field.name) {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0060",
                        format!(
                            "struct `{struct_name}` does not contain field `{}`",
                            field.name
                        ),
                        self.info.source,
                        field.span,
                    )
                    .with_kind(DiagnosticKind::MatchStructPatternShapeMismatch)
                    .with_suggestion("use one of the fields declared on the matched struct"),
                );
            }
        }

        let missing = struct_info
            .fields
            .keys()
            .filter(|field| !seen.contains(*field))
            .cloned()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            self.diagnostics.push(
                Diagnostic::new(
                    "S0060",
                    format!(
                        "struct pattern `{}` must list every field of `{struct_name}`; missing {}",
                        pattern_label(pattern),
                        missing.join(", ")
                    ),
                    self.info.source,
                    pattern.span,
                )
                .with_kind(DiagnosticKind::MatchStructPatternShapeMismatch)
                .with_note("AX struct destructuring v0 uses full-field shorthand patterns only")
                .with_suggestion(format!(
                    "rewrite as `{struct_name} {{ {} }}`",
                    struct_info
                        .fields
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
            );
        }
    }

    fn report_match_pattern_type_mismatch(
        &mut self,
        pattern: &MatchPattern,
        scrutinee_type: &Type,
    ) {
        self.diagnostics.push(
            Diagnostic::new(
                "S0046",
                format!(
                    "match pattern `{}` does not match input type `{}`",
                    pattern_label(pattern),
                    scrutinee_type.describe()
                ),
                self.info.source,
                pattern.span,
            )
            .with_kind(DiagnosticKind::MatchPatternTypeMismatch)
            .with_suggestion(
                "change the pattern so it uses the same type as the match input, or change the matched value",
            ),
        );
    }

    pub(super) fn report_duplicate_match_pattern(&mut self, span: Span, pattern: String) {
        self.diagnostics.push(
            Diagnostic::new(
                "S0047",
                format!("duplicate match pattern `{pattern}`"),
                self.info.source,
                span,
            )
            .with_kind(DiagnosticKind::DuplicateMatchPattern)
            .with_suggestion("remove the duplicate arm or merge its logic into the earlier arm"),
        );
    }
}
