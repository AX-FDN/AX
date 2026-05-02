use super::*;

impl<'a> Interpreter<'a> {
    pub(in crate::interpreter) fn eval_match_expression(
        &mut self,
        scrutinee: Value,
        arms: &[MatchExprArm],
        span: Span,
        frame: &mut Frame,
    ) -> Result<EvalFlow, Diagnostic> {
        for arm in arms {
            if self.match_pattern_matches_value(&arm.pattern, &scrutinee, span)? {
                if let Some(value) = self.eval_match_expression_arm_value(
                    &arm.pattern,
                    arm.guard.as_ref(),
                    &scrutinee,
                    &arm.value,
                    frame,
                )? {
                    return Ok(value);
                }
            }
        }

        Err(self.runtime_error(
            "R0036",
            "non-exhaustive match expression reached runtime without a matching arm",
            span,
        ))
    }

    pub(in crate::interpreter) fn match_pattern_matches_value(
        &self,
        pattern: &MatchPattern,
        scrutinee: &Value,
        span: Span,
    ) -> Result<bool, Diagnostic> {
        match &pattern.kind {
            MatchPatternKind::Wildcard => Ok(true),
            MatchPatternKind::Binding { .. } => Ok(true),
            MatchPatternKind::Bool { value } => match scrutinee {
                Value::Bool(actual) => Ok(actual == value),
                other => Err(self.runtime_error(
                    "R0037",
                    format!(
                        "match pattern `bool` cannot be applied to runtime value `{}`",
                        other.display()
                    ),
                    span,
                )),
            },
            MatchPatternKind::Int { value } => match scrutinee {
                Value::I32(actual) => Ok(actual == value),
                other => Err(self.runtime_error(
                    "R0037",
                    format!(
                        "match pattern `i32` cannot be applied to runtime value `{}`",
                        other.display()
                    ),
                    span,
                )),
            },
            MatchPatternKind::IntRange { start, end } => match scrutinee {
                Value::I32(actual) => Ok(actual >= start && actual <= end),
                other => Err(self.runtime_error(
                    "R0037",
                    format!(
                        "match pattern `i32` range cannot be applied to runtime value `{}`",
                        other.display()
                    ),
                    span,
                )),
            },
            MatchPatternKind::String { value } => match scrutinee {
                Value::String(actual) => Ok(actual == value),
                other => Err(self.runtime_error(
                    "R0037",
                    format!(
                        "match pattern `string` cannot be applied to runtime value `{}`",
                        other.display()
                    ),
                    span,
                )),
            },
            MatchPatternKind::EnumVariant {
                enum_name,
                variant,
                payload,
                ..
            } => match scrutinee {
                Value::Enum {
                    name,
                    variant: actual_variant,
                    payload: actual_payload,
                } => {
                    if name != enum_name || actual_variant != variant {
                        return Ok(false);
                    }

                    match (payload, actual_payload.as_ref()) {
                        (None, _) => Ok(true),
                        (Some(MatchPatternPayload::Wildcard), Some(_)) => Ok(true),
                        (Some(MatchPatternPayload::Binding { .. }), Some(_)) => Ok(true),
                        (Some(_), None) => Err(self.runtime_error(
                            "R0037",
                            format!(
                                "match enum pattern `{}` expects a payload value",
                                Self::match_pattern_label(pattern)
                            ),
                            span,
                        )),
                    }
                }
                other => Err(self.runtime_error(
                    "R0037",
                    format!(
                        "match enum pattern cannot be applied to runtime value `{}`",
                        other.display()
                    ),
                    span,
                )),
            },
            MatchPatternKind::Struct {
                struct_name,
                fields,
            } => match scrutinee {
                Value::Struct {
                    name,
                    fields: values,
                } => {
                    if name != struct_name {
                        return Ok(false);
                    }
                    Ok(fields.iter().all(|field| values.contains_key(&field.name)))
                }
                other => Err(self.runtime_error(
                    "R0037",
                    format!(
                        "match struct pattern cannot be applied to runtime value `{}`",
                        other.display()
                    ),
                    span,
                )),
            },
            MatchPatternKind::Or { alternatives } => {
                for alternative in alternatives {
                    if self.match_pattern_matches_value(alternative, scrutinee, span)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            MatchPatternKind::Error => {
                Err(self.runtime_error("R0038", "invalid match pattern reached the runtime", span))
            }
        }
    }

    pub(in crate::interpreter) fn eval_match_expression_arm_value(
        &mut self,
        pattern: &MatchPattern,
        guard: Option<&Expr>,
        scrutinee: &Value,
        value: &Expr,
        frame: &mut Frame,
    ) -> Result<Option<EvalFlow>, Diagnostic> {
        frame.scopes.push(HashMap::new());
        if let Err(error) = self.bind_match_pattern_locals(pattern, scrutinee, frame) {
            frame.scopes.pop();
            return Err(error);
        }
        if let Some(guard) = guard {
            let guard_matches = self.eval_condition(guard, frame);
            match guard_matches {
                Ok(ConditionFlow::Value(true)) => {}
                Ok(ConditionFlow::Value(false)) => {
                    frame.scopes.pop();
                    return Ok(None);
                }
                Ok(ConditionFlow::Return(value)) => {
                    frame.scopes.pop();
                    return Ok(Some(EvalFlow::Return(value)));
                }
                Err(error) => {
                    frame.scopes.pop();
                    return Err(error);
                }
            }
        }
        let result = self.eval_expr(value, frame).map(Some);
        frame.scopes.pop();
        result
    }

    pub(in crate::interpreter) fn bind_match_pattern_locals(
        &self,
        pattern: &MatchPattern,
        scrutinee: &Value,
        frame: &mut Frame,
    ) -> Result<(), Diagnostic> {
        match &pattern.kind {
            MatchPatternKind::Binding { name } => {
                frame.scopes.last_mut().expect("scope should exist").insert(
                    name.clone(),
                    Slot {
                        mutable: false,
                        value: scrutinee.clone(),
                    },
                );
            }
            MatchPatternKind::EnumVariant {
                payload: Some(MatchPatternPayload::Binding { name }),
                ..
            } => {
                let Value::Enum {
                    payload: Some(payload),
                    ..
                } = scrutinee
                else {
                    return Err(self.runtime_error(
                        "R0042",
                        format!("payload binding `{}` requires a payload enum value", name),
                        pattern.span,
                    ));
                };
                frame.scopes.last_mut().expect("scope should exist").insert(
                    name.clone(),
                    Slot {
                        mutable: false,
                        value: (**payload).clone(),
                    },
                );
            }
            MatchPatternKind::Struct { fields, .. } => {
                let Value::Struct { fields: values, .. } = scrutinee else {
                    return Err(self.runtime_error(
                        "R0043",
                        "struct pattern binding requires a struct value",
                        pattern.span,
                    ));
                };
                for field in fields {
                    let Some(value) = values.get(&field.name) else {
                        return Err(self.runtime_error(
                            "R0043",
                            format!(
                                "struct pattern binding `{}` requires field `{}`",
                                field.binding, field.name
                            ),
                            field.span,
                        ));
                    };
                    frame.scopes.last_mut().expect("scope should exist").insert(
                        field.binding.clone(),
                        Slot {
                            mutable: false,
                            value: value.clone(),
                        },
                    );
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub(in crate::interpreter) fn match_pattern_label(pattern: &MatchPattern) -> String {
        match &pattern.kind {
            MatchPatternKind::Wildcard => "_".to_string(),
            MatchPatternKind::Binding { name } => name.clone(),
            MatchPatternKind::Bool { value } => value.to_string(),
            MatchPatternKind::Int { value } => value.to_string(),
            MatchPatternKind::IntRange { start, end } => format!("{start}..={end}"),
            MatchPatternKind::String { value } => format!("{value:?}"),
            MatchPatternKind::EnumVariant {
                enum_name,
                variant,
                payload: Some(MatchPatternPayload::Wildcard),
                ..
            } => format!("{enum_name}.{variant}(_)"),
            MatchPatternKind::EnumVariant {
                enum_name,
                variant,
                payload: Some(MatchPatternPayload::Binding { name }),
                ..
            } => format!("{enum_name}.{variant}({name})"),
            MatchPatternKind::EnumVariant {
                enum_name, variant, ..
            } => format!("{enum_name}.{variant}"),
            MatchPatternKind::Struct {
                struct_name,
                fields,
            } => {
                let fields = fields
                    .iter()
                    .map(|field| field.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{struct_name} {{ {fields} }}")
            }
            MatchPatternKind::Or { alternatives } => alternatives
                .iter()
                .map(Self::match_pattern_label)
                .collect::<Vec<_>>()
                .join(" | "),
            MatchPatternKind::Error => "<invalid-pattern>".to_string(),
        }
    }
}
