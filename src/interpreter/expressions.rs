use super::*;

impl<'a> Interpreter<'a> {
    pub(in crate::interpreter) fn eval_expr(
        &mut self,
        expr: &Expr,
        frame: &mut Frame,
    ) -> Result<EvalFlow, Diagnostic> {
        macro_rules! eval_value {
            ($inner:expr) => {
                match self.eval_expr($inner, frame)? {
                    EvalFlow::Value(value) => value,
                    early @ EvalFlow::Return(_) => return Ok(early),
                }
            };
        }

        match &expr.kind {
            ExprKind::Int { value } => Ok(EvalFlow::Value(Value::I32(*value))),
            ExprKind::Float { value } => Ok(EvalFlow::Value(Value::F32(*value))),
            ExprKind::Bool { value } => Ok(EvalFlow::Value(Value::Bool(*value))),
            ExprKind::String { value } => Ok(EvalFlow::Value(Value::String(value.clone()))),
            ExprKind::Name { value } => Ok(EvalFlow::Value(
                lookup_slot(frame, value)
                    .map(|slot| slot.value.clone())
                    .or_else(|| self.constants.get(value).cloned())
                    .ok_or_else(|| {
                        self.runtime_error(
                            "R0011",
                            format!("use of unknown variable `{value}`"),
                            expr.span,
                        )
                    })?,
            )),
            ExprKind::Unary { op, expr: inner } => {
                let inner = eval_value!(inner);
                let value = match (op, inner) {
                    (UnaryOp::Negate, Value::I32(value)) => {
                        value.checked_neg().map(Value::I32).ok_or_else(|| {
                            self.runtime_error("R0012", "integer negation overflowed", expr.span)
                        })
                    }
                    (UnaryOp::Negate, Value::F32(value)) => Ok(Value::F32(-value)),
                    (UnaryOp::Not, Value::Bool(value)) => Ok(Value::Bool(!value)),
                    (_, other) => Err(self.runtime_error(
                        "R0013",
                        format!("invalid unary operation on `{}`", other.display()),
                        expr.span,
                    )),
                }?;
                Ok(EvalFlow::Value(value))
            }
            ExprKind::Try { expr: inner } => {
                let value = eval_value!(inner);
                match value {
                    Value::Enum {
                        variant,
                        payload: Some(payload),
                        ..
                    } if variant == "Ok" => Ok(EvalFlow::Value(*payload)),
                    Value::Enum {
                        name,
                        variant,
                        payload,
                    } if variant == "Err" => Ok(EvalFlow::Return(Value::Enum {
                        name,
                        variant,
                        payload,
                    })),
                    Value::Enum { variant, .. } => Err(self.runtime_error(
                        "R0136",
                        format!(
                            "`?` expected `Result.Ok` or `Result.Err`, got variant `{variant}`"
                        ),
                        expr.span,
                    )),
                    other => Err(self.runtime_error(
                        "R0136",
                        format!("`?` expected a `Result` value, got `{}`", other.display()),
                        expr.span,
                    )),
                }
            }
            ExprKind::Binary { op, left, right } => {
                if matches!(*op, BinaryOp::LogicalAnd | BinaryOp::LogicalOr) {
                    let left_value = eval_value!(left);
                    let value = match (*op, left_value) {
                        (BinaryOp::LogicalAnd, Value::Bool(false)) => Ok(Value::Bool(false)),
                        (BinaryOp::LogicalAnd, Value::Bool(true)) => {
                            let right_value = eval_value!(right);
                            self.eval_binary(*op, Value::Bool(true), right_value, expr.span)
                        }
                        (BinaryOp::LogicalOr, Value::Bool(true)) => Ok(Value::Bool(true)),
                        (BinaryOp::LogicalOr, Value::Bool(false)) => {
                            let right_value = eval_value!(right);
                            self.eval_binary(*op, Value::Bool(false), right_value, expr.span)
                        }
                        (_, other) => Err(self.runtime_error(
                            "R0023",
                            format!(
                                "invalid binary operation for runtime values `{}` and `<unevaluated>`",
                                other.display()
                            ),
                            expr.span,
                        )),
                    }?;
                    return Ok(EvalFlow::Value(value));
                }

                let mut operands = Vec::new();
                collect_left_associative_binary_operands(expr, *op, &mut operands);
                if operands.len() > 2 {
                    let mut operands = operands.into_iter();
                    let first = operands
                        .next()
                        .expect("binary chain should contain at least one operand");
                    let mut value = eval_value!(first);
                    for operand in operands {
                        let right = eval_value!(operand);
                        value = self.eval_binary(*op, value, right, expr.span)?;
                    }
                    return Ok(EvalFlow::Value(value));
                }

                let left = eval_value!(left);
                let right = eval_value!(right);
                Ok(EvalFlow::Value(
                    self.eval_binary(*op, left, right, expr.span)?,
                ))
            }
            ExprKind::Call {
                function,
                arguments,
            } => {
                let mut argument_values = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    argument_values.push(eval_value!(argument));
                }
                let value = if self.functions.contains_key(function) {
                    self.call_declared_function(function, argument_values, expr.span)
                } else {
                    self.call_function(function, argument_values, expr.span)
                }?;
                Ok(EvalFlow::Value(value))
            }
            ExprKind::MethodCall {
                receiver,
                method,
                arguments,
            } => {
                let receiver_value = eval_value!(receiver);
                let method_function = match &receiver_value {
                    Value::Struct { name, .. } | Value::Enum { name, .. } => {
                        format!("{name}.{method}")
                    }
                    other => {
                        return Err(self.runtime_error(
                            "R0133",
                            format!(
                                "method call `{method}` requires a struct or enum receiver, got `{}`",
                                other.display()
                            ),
                            expr.span,
                        ));
                    }
                };
                let mut argument_values = Vec::with_capacity(arguments.len() + 1);
                argument_values.push(receiver_value);
                for argument in arguments {
                    argument_values.push(eval_value!(argument));
                }
                Ok(EvalFlow::Value(self.call_declared_function(
                    &method_function,
                    argument_values,
                    expr.span,
                )?))
            }
            ExprKind::StructLiteral { name, fields } => {
                let mut values = BTreeMap::new();
                for field in fields {
                    values.insert(field.name.clone(), eval_value!(&field.value));
                }
                Ok(EvalFlow::Value(Value::Struct {
                    name: name.clone(),
                    fields: values,
                }))
            }
            ExprKind::ArrayLiteral { elements } => {
                let mut values = Vec::with_capacity(elements.len());
                for element in elements {
                    values.push(eval_value!(element));
                }
                Ok(EvalFlow::Value(Value::Array(values)))
            }
            ExprKind::Block { statements, value } => {
                self.eval_block_expr(statements, value, frame, expr.span)
            }
            ExprKind::Match { scrutinee, arms } => {
                let scrutinee_value = eval_value!(scrutinee);
                self.eval_match_expression(scrutinee_value, arms, expr.span, frame)
            }
            ExprKind::EnumVariant {
                enum_name,
                variant,
                payload,
            } => Ok(EvalFlow::Value(Value::Enum {
                name: enum_name.clone(),
                variant: variant.clone(),
                payload: match payload {
                    Some(payload) => Some(Box::new(eval_value!(payload))),
                    None => None,
                },
            })),
            ExprKind::MatchTest { scrutinee, pattern } => {
                let scrutinee_value = eval_value!(scrutinee);
                Ok(EvalFlow::Value(Value::Bool(
                    self.match_pattern_matches_value(pattern, &scrutinee_value, expr.span)?,
                )))
            }
            ExprKind::EnumPayload { value } => match eval_value!(value) {
                Value::Enum {
                    payload: Some(payload),
                    ..
                } => Ok(EvalFlow::Value(*payload)),
                other => Err(self.runtime_error(
                    "R0042",
                    format!(
                        "payload extraction requires a payload enum value, got `{}`",
                        other.display()
                    ),
                    expr.span,
                )),
            },
            ExprKind::Field { base, field } => match eval_value!(base) {
                Value::Struct { fields, .. } => Ok(EvalFlow::Value(
                    fields.get(field).cloned().ok_or_else(|| {
                        self.runtime_error(
                            "R0015",
                            format!("struct value does not contain field `{field}`"),
                            expr.span,
                        )
                    })?,
                )),
                other => Err(self.runtime_error(
                    "R0016",
                    format!(
                        "field access requires a struct value, got `{}`",
                        other.display()
                    ),
                    expr.span,
                )),
            },
            ExprKind::Index { base, index } => {
                let base_value = eval_value!(base);
                let elements = self.indexable_elements(base_value, expr.span)?;

                let index_value = eval_value!(index);
                let resolved =
                    self.resolve_array_index(index_value, index.span, elements.len(), expr.span)?;
                Ok(EvalFlow::Value(elements[resolved].clone()))
            }
            ExprKind::Slice { base, start, end } => {
                let base_value = eval_value!(base);
                let elements = self.indexable_elements(base_value, expr.span)?;
                let start_value = eval_value!(start);
                let end_value = eval_value!(end);
                let start_index =
                    self.resolve_slice_bound(start_value, start.span, elements.len(), "start")?;
                let end_index =
                    self.resolve_slice_bound(end_value, end.span, elements.len(), "end")?;

                if start_index > end_index {
                    return Err(self
                        .runtime_error(
                            "R0035",
                            format!(
                                "slice start `{start_index}` cannot be greater than slice end `{end_index}`"
                            ),
                            expr.span,
                        )
                        .with_note("AX slice ranges are half-open: `values[start:end]` includes `start` and excludes `end`")
                        .with_suggestion("ensure the start bound is less than or equal to the end bound"));
                }

                Ok(EvalFlow::Value(Value::Slice(
                    elements[start_index..end_index].to_vec(),
                )))
            }
        }
    }

    pub(in crate::interpreter) fn eval_block_expr(
        &mut self,
        statements: &[Stmt],
        value: &Expr,
        frame: &mut Frame,
        span: Span,
    ) -> Result<EvalFlow, Diagnostic> {
        frame.scopes.push(HashMap::new());
        for statement in statements {
            match self.exec_statement(statement, frame)? {
                ControlFlow::Continue => {}
                ControlFlow::Return(value) => {
                    frame.scopes.pop();
                    return Ok(EvalFlow::Return(value));
                }
                ControlFlow::Break => {
                    frame.scopes.pop();
                    return Err(self.runtime_error(
                        "R0137",
                        "`break` cannot leave a block-valued expression",
                        span,
                    ));
                }
                ControlFlow::LoopContinue => {
                    frame.scopes.pop();
                    return Err(self.runtime_error(
                        "R0138",
                        "`continue` cannot leave a block-valued expression",
                        span,
                    ));
                }
            }
        }
        let result = self.eval_expr(value, frame);
        frame.scopes.pop();
        result
    }
}
