use std::fmt::Write;

use super::*;

mod array;
mod call;
mod composite;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_statement(
        &mut self,
        statement: &Statement,
        out: &mut String,
    ) -> Result<(), String> {
        match &statement.kind {
            StatementKind::Let {
                local, initializer, ..
            } => {
                let slot = self.local_slot(*local)?.clone();
                let value = match &initializer.kind {
                    ExprKind::EnumPayload { value } => {
                        self.emit_enum_payload(value, Some(&slot.ax_ty), out)?
                    }
                    _ if matches!(slot.ax_ty, Type::Slice { .. }) => {
                        self.emit_slice_from_expr(initializer, out)?
                    }
                    ExprKind::EnumVariant {
                        enum_name,
                        variant,
                        payload,
                    } => self.emit_enum_variant(
                        enum_name,
                        variant,
                        payload.as_deref(),
                        Some(&slot.ax_ty),
                        out,
                    )?,
                    ExprKind::StructLiteral { name, fields } => {
                        self.emit_struct_literal(name, fields, Some(&slot.ax_ty), out)?
                    }
                    ExprKind::ArrayLiteral { elements } => {
                        self.emit_array_literal(elements, Some(&slot.ax_ty), out)?
                    }
                    _ => self.emit_expr_with_expected(initializer, Some(&slot.ax_ty), out)?,
                };
                ensure_same_type(&slot.ty, &value.ty)?;
                writeln!(out, "  store {} {}, ptr {}", value.ty, value.repr, slot.ptr)
                    .expect("writing to string cannot fail");
                Ok(())
            }
            StatementKind::Assign { target, value } => {
                let slot = self.emit_place_ptr(target, out)?;
                let value = match &value.kind {
                    ExprKind::StructLiteral { name, fields } => {
                        self.emit_struct_literal(name, fields, Some(&slot.ax_ty), out)?
                    }
                    ExprKind::ArrayLiteral { elements } => {
                        self.emit_array_literal(elements, Some(&slot.ax_ty), out)?
                    }
                    _ => self.emit_expr_with_expected(value, Some(&slot.ax_ty), out)?,
                };
                ensure_same_type(&slot.ty, &value.ty)?;
                writeln!(out, "  store {} {}, ptr {}", value.ty, value.repr, slot.ptr)
                    .expect("writing to string cannot fail");
                Ok(())
            }
            StatementKind::Eval { expr } => {
                self.emit_expr(expr, out)?;
                Ok(())
            }
        }
    }

    pub(super) fn emit_terminator(
        &mut self,
        block: &BasicBlock,
        out: &mut String,
    ) -> Result<(), String> {
        match &block.terminator.kind {
            TerminatorKind::Goto { target } => {
                writeln!(out, "  br label %bb{target}").expect("writing to string cannot fail");
                Ok(())
            }
            TerminatorKind::Branch {
                condition,
                then_block,
                else_block,
            } => {
                let condition = self.emit_expr(condition, out)?;
                ensure_same_type("i1", &condition.ty)?;
                writeln!(
                    out,
                    "  br i1 {}, label %bb{}, label %bb{}",
                    condition.repr, then_block, else_block
                )
                .expect("writing to string cannot fail");
                Ok(())
            }
            TerminatorKind::Return { value } => {
                let value = match &value.kind {
                    ExprKind::EnumVariant {
                        enum_name,
                        variant,
                        payload,
                    } => {
                        let return_ax_ty = self.return_ax_ty.clone();
                        self.emit_enum_variant(
                            enum_name,
                            variant,
                            payload.as_deref(),
                            Some(&return_ax_ty),
                            out,
                        )?
                    }
                    ExprKind::StructLiteral { name, fields } => {
                        let return_ax_ty = self.return_ax_ty.clone();
                        self.emit_struct_literal(name, fields, Some(&return_ax_ty), out)?
                    }
                    ExprKind::ArrayLiteral { elements } => {
                        let return_ax_ty = self.return_ax_ty.clone();
                        self.emit_array_literal(elements, Some(&return_ax_ty), out)?
                    }
                    _ => {
                        let return_ax_ty = self.return_ax_ty.clone();
                        self.emit_expr_with_expected(value, Some(&return_ax_ty), out)?
                    }
                };
                writeln!(out, "  ret {} {}", value.ty, value.repr)
                    .expect("writing to string cannot fail");
                Ok(())
            }
            TerminatorKind::Unreachable => {
                writeln!(out, "  unreachable").expect("writing to string cannot fail");
                Ok(())
            }
        }
    }

    fn emit_expr(&mut self, expr: &Expr, out: &mut String) -> Result<LlvmValue, String> {
        match &expr.kind {
            ExprKind::Int { value } => Ok(LlvmValue {
                ty: "i32".to_string(),
                repr: value.to_string(),
                ax_ty: Some(Type::I32),
            }),
            ExprKind::Float { value } => Ok(LlvmValue {
                ty: "float".to_string(),
                repr: llvm_float_literal(*value),
                ax_ty: Some(Type::F32),
            }),
            ExprKind::Bool { value } => Ok(LlvmValue {
                ty: "i1".to_string(),
                repr: if *value { "1" } else { "0" }.to_string(),
                ax_ty: Some(Type::Bool),
            }),
            ExprKind::Local { local, .. } => {
                let slot = self.local_slot(*local)?.clone();
                let temp = self.next_temp();
                writeln!(out, "  {temp} = load {}, ptr {}", slot.ty, slot.ptr)
                    .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: slot.ty,
                    repr: temp,
                    ax_ty: Some(slot.ax_ty),
                })
            }
            ExprKind::Unary { op, expr } => {
                let value = self.emit_expr(expr, out)?;
                match op {
                    UnaryOp::Negate => {
                        let temp = self.next_temp();
                        match value.ty.as_str() {
                            "i32" => {
                                let overflow = self.next_temp();
                                writeln!(
                                    out,
                                    "  {overflow} = icmp eq i32 {}, -2147483648",
                                    value.repr
                                )
                                .expect("writing to string cannot fail");
                                self.emit_runtime_error_if(
                                    &overflow,
                                    "i32_neg_overflow",
                                    "@.ax_rt_neg_overflow",
                                    out,
                                );
                                writeln!(out, "  {temp} = sub i32 0, {}", value.repr)
                                    .expect("writing to string cannot fail");
                                Ok(LlvmValue {
                                    ty: "i32".to_string(),
                                    repr: temp,
                                    ax_ty: Some(Type::I32),
                                })
                            }
                            "float" => {
                                writeln!(out, "  {temp} = fneg float {}", value.repr)
                                    .expect("writing to string cannot fail");
                                Ok(LlvmValue {
                                    ty: "float".to_string(),
                                    repr: temp,
                                    ax_ty: Some(Type::F32),
                                })
                            }
                            other => Err(format!(
                                "unary `-` expects i32 or f32 in LLVM AOT v0, found {other}"
                            )),
                        }
                    }
                    UnaryOp::Not => {
                        ensure_same_type("i1", &value.ty)?;
                        let temp = self.next_temp();
                        writeln!(out, "  {temp} = xor i1 {}, 1", value.repr)
                            .expect("writing to string cannot fail");
                        Ok(LlvmValue {
                            ty: "i1".to_string(),
                            repr: temp,
                            ax_ty: Some(Type::Bool),
                        })
                    }
                }
            }
            ExprKind::Binary { op, left, right } => {
                if matches!(op, BinaryOp::LogicalAnd | BinaryOp::LogicalOr) {
                    return self.emit_short_circuit_logical(*op, left, right, out);
                }
                let left = self.emit_expr(left, out)?;
                let right = self.emit_expr(right, out)?;
                self.emit_binary(*op, left, right, out)
            }
            ExprKind::Call {
                function,
                arguments,
            } => self.emit_call(function, arguments, None, out),
            ExprKind::String { value } => {
                let literal = self
                    .strings
                    .get(value)
                    .ok_or_else(|| "internal LLVM AOT error: missing string literal".to_string())?;
                Ok(LlvmValue {
                    ty: "ptr".to_string(),
                    repr: literal.symbol.clone(),
                    ax_ty: Some(Type::String),
                })
            }
            ExprKind::Const { name } => self.emit_const(name, out),
            ExprKind::Try { expr } => self.emit_try(expr, out),
            ExprKind::ArrayLiteral { elements } => self.emit_array_literal(elements, None, out),
            ExprKind::Index { base, index } => self.emit_index(base, index, out),
            ExprKind::Slice { base, start, end } => self.emit_slice_range(base, start, end, out),
            ExprKind::StructLiteral { name, fields } => {
                self.emit_struct_literal(name, fields, None, out)
            }
            ExprKind::Field { base, field } => self.emit_field(base, field, out),
            ExprKind::EnumVariant {
                enum_name,
                variant,
                payload,
            } => self.emit_enum_variant(enum_name, variant, payload.as_deref(), None, out),
            ExprKind::MatchTest { scrutinee, pattern } => {
                self.emit_match_test(scrutinee, pattern, out)
            }
            ExprKind::EnumPayload { value } => self.emit_enum_payload(value, None, out),
            ExprKind::Block { statements, value } => self.emit_block_expr(statements, value, out),
            ExprKind::Match { scrutinee, arms } => self.emit_match_expr(scrutinee, arms, out),
        }
    }

    fn emit_expr_with_expected(
        &mut self,
        expr: &Expr,
        expected_ax_ty: Option<&Type>,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        match &expr.kind {
            ExprKind::Call {
                function,
                arguments,
            } => self.emit_call(function, arguments, expected_ax_ty, out),
            ExprKind::EnumPayload { value } => self.emit_enum_payload(value, expected_ax_ty, out),
            ExprKind::EnumVariant {
                enum_name,
                variant,
                payload,
            } => {
                self.emit_enum_variant(enum_name, variant, payload.as_deref(), expected_ax_ty, out)
            }
            ExprKind::StructLiteral { name, fields } => {
                self.emit_struct_literal(name, fields, expected_ax_ty, out)
            }
            ExprKind::ArrayLiteral { elements } => {
                self.emit_array_literal(elements, expected_ax_ty, out)
            }
            ExprKind::Block { statements, value } => {
                self.emit_block_expr_with_expected(statements, value, expected_ax_ty, out)
            }
            ExprKind::Match { scrutinee, arms } => {
                self.emit_match_expr_with_expected(scrutinee, arms, expected_ax_ty, out)
            }
            _ => self.emit_expr(expr, out),
        }
    }

    fn emit_short_circuit_logical(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let left = self.emit_expr(left, out)?;
        ensure_same_type("i1", &left.ty)?;

        let result_slot = self.next_temp();
        let rhs_label = self.next_label("logical_rhs");
        let done_label = self.next_label("logical_done");
        let default_value = if matches!(op, BinaryOp::LogicalAnd) {
            "0"
        } else {
            "1"
        };

        writeln!(out, "  {result_slot} = alloca i1").expect("writing to string cannot fail");
        writeln!(out, "  store i1 {default_value}, ptr {result_slot}")
            .expect("writing to string cannot fail");
        match op {
            BinaryOp::LogicalAnd => {
                writeln!(
                    out,
                    "  br i1 {}, label %{rhs_label}, label %{done_label}",
                    left.repr
                )
                .expect("writing to string cannot fail");
            }
            BinaryOp::LogicalOr => {
                writeln!(
                    out,
                    "  br i1 {}, label %{done_label}, label %{rhs_label}",
                    left.repr
                )
                .expect("writing to string cannot fail");
            }
            _ => unreachable!("short-circuit lowering only handles logical operators"),
        }

        writeln!(out, "{rhs_label}:").expect("writing to string cannot fail");
        let right = self.emit_expr(right, out)?;
        ensure_same_type("i1", &right.ty)?;
        writeln!(out, "  store i1 {}, ptr {result_slot}", right.repr)
            .expect("writing to string cannot fail");
        writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");

        writeln!(out, "{done_label}:").expect("writing to string cannot fail");
        let result = self.next_temp();
        writeln!(out, "  {result} = load i1, ptr {result_slot}")
            .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "i1".to_string(),
            repr: result,
            ax_ty: Some(Type::Bool),
        })
    }

    fn emit_block_expr(
        &mut self,
        statements: &[Statement],
        value: &Expr,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        self.emit_block_expr_with_expected(statements, value, None, out)
    }

    fn emit_block_expr_with_expected(
        &mut self,
        statements: &[Statement],
        value: &Expr,
        expected_ax_ty: Option<&Type>,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        for statement in statements {
            self.emit_statement(statement, out)?;
        }
        self.emit_expr_with_expected(value, expected_ax_ty, out)
    }

    fn emit_const(&mut self, name: &str, out: &mut String) -> Result<LlvmValue, String> {
        let binding = self
            .consts
            .get(name)
            .cloned()
            .ok_or_else(|| format!("const `{name}` was not found for LLVM AOT lowering"))?;
        if self.const_stack.iter().any(|active| active == name) {
            return Err(format!(
                "const `{name}` recursively references itself during LLVM AOT lowering"
            ));
        }

        self.const_stack.push(name.to_string());
        let value = self.emit_expr_with_expected(&binding.value, Some(&binding.ty), out);
        self.const_stack.pop();

        let mut value = value?;
        let Some(expected_ty) = llvm_type(&binding.ty, self.layouts, self.enum_layouts) else {
            return Err(format!(
                "const `{name}` uses unsupported type {}",
                ax_type_name(&binding.ty)
            ));
        };
        ensure_same_type(&expected_ty, &value.ty)?;
        value.ax_ty = Some(binding.ty);
        Ok(value)
    }

    fn emit_match_expr(
        &mut self,
        scrutinee: &Expr,
        arms: &[MatchExprArm],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        self.emit_match_expr_with_expected(scrutinee, arms, None, out)
    }

    fn emit_match_expr_with_expected(
        &mut self,
        scrutinee: &Expr,
        arms: &[MatchExprArm],
        expected_ax_ty: Option<&Type>,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arms.is_empty() {
            return Err("match expression has no arms in LLVM AOT v0".to_string());
        }
        let (result_ty, result_ax_ty) = if let Some(expected_ax_ty) = expected_ax_ty {
            let ty =
                llvm_type(expected_ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                    format!(
                        "match expression expected type {} is outside LLVM AOT v0",
                        ax_type_name(expected_ax_ty)
                    )
                })?;
            (ty, expected_ax_ty.clone())
        } else {
            let (result_ty, result_ax_ty) = self.infer_expr_value_type(&arms[0].value)?;
            for arm in &arms[1..] {
                let (arm_ty, _) = self.infer_expr_value_type(&arm.value)?;
                ensure_same_type(&result_ty, &arm_ty)?;
            }
            (result_ty, result_ax_ty)
        };

        let scrutinee_value = self.emit_expr(scrutinee, out)?;
        let result_slot = self.next_temp();
        let done_label = self.next_label("match_done");
        let no_match_label = self.next_label("match_no_match");
        writeln!(out, "  {result_slot} = alloca {result_ty}")
            .expect("writing to string cannot fail");

        let arm_labels = (0..arms.len())
            .map(|index| self.next_label(&format!("match_arm_{index}")))
            .collect::<Vec<_>>();
        let next_labels = (0..arms.len())
            .map(|index| {
                if index + 1 == arms.len() {
                    no_match_label.clone()
                } else {
                    self.next_label(&format!("match_next_{index}"))
                }
            })
            .collect::<Vec<_>>();

        for (index, arm) in arms.iter().enumerate() {
            let matched = self.emit_match_test_value(&scrutinee_value, &arm.pattern, out)?;
            ensure_same_type("i1", &matched.ty)?;
            if let Some(guard) = &arm.guard {
                let guard_label = self.next_label(&format!("match_guard_{index}"));
                writeln!(
                    out,
                    "  br i1 {}, label %{}, label %{}",
                    matched.repr, guard_label, next_labels[index]
                )
                .expect("writing to string cannot fail");
                writeln!(out, "{guard_label}:").expect("writing to string cannot fail");
                self.emit_match_bindings(&scrutinee_value, arm, out)?;
                let guard_value = self.emit_expr(guard, out)?;
                ensure_same_type("i1", &guard_value.ty)?;
                writeln!(
                    out,
                    "  br i1 {}, label %{}, label %{}",
                    guard_value.repr, arm_labels[index], next_labels[index]
                )
                .expect("writing to string cannot fail");
            } else {
                writeln!(
                    out,
                    "  br i1 {}, label %{}, label %{}",
                    matched.repr, arm_labels[index], next_labels[index]
                )
                .expect("writing to string cannot fail");
            }
            writeln!(out, "{}:", arm_labels[index]).expect("writing to string cannot fail");
            if arm.guard.is_none() {
                self.emit_match_bindings(&scrutinee_value, arm, out)?;
            }
            let value = self.emit_expr_with_expected(&arm.value, Some(&result_ax_ty), out)?;
            ensure_same_type(&result_ty, &value.ty)?;
            writeln!(
                out,
                "  store {} {}, ptr {result_slot}",
                value.ty, value.repr
            )
            .expect("writing to string cannot fail");
            writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");
            if index + 1 < arms.len() {
                writeln!(out, "{}:", next_labels[index]).expect("writing to string cannot fail");
            }
        }

        writeln!(out, "{no_match_label}:").expect("writing to string cannot fail");
        writeln!(out, "  call void @exit(i32 1)").expect("writing to string cannot fail");
        writeln!(out, "  unreachable").expect("writing to string cannot fail");
        writeln!(out, "{done_label}:").expect("writing to string cannot fail");
        let loaded = self.next_temp();
        writeln!(out, "  {loaded} = load {result_ty}, ptr {result_slot}")
            .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: result_ty,
            repr: loaded,
            ax_ty: Some(result_ax_ty),
        })
    }

    fn infer_expr_value_type(&self, expr: &Expr) -> Result<(String, Type), String> {
        match &expr.kind {
            ExprKind::Int { .. } => Ok(("i32".to_string(), Type::I32)),
            ExprKind::Bool { .. } => Ok(("i1".to_string(), Type::Bool)),
            ExprKind::String { .. } => Ok(("ptr".to_string(), Type::String)),
            ExprKind::Local { local, .. } => {
                let slot = self.local_slot(*local)?;
                Ok((slot.ty.clone(), slot.ax_ty.clone()))
            }
            ExprKind::Unary { op, expr } => match op {
                UnaryOp::Negate => {
                    let (ty, _) = self.infer_expr_value_type(expr)?;
                    ensure_same_type("i32", &ty)?;
                    Ok(("i32".to_string(), Type::I32))
                }
                UnaryOp::Not => {
                    let (ty, _) = self.infer_expr_value_type(expr)?;
                    ensure_same_type("i1", &ty)?;
                    Ok(("i1".to_string(), Type::Bool))
                }
            },
            ExprKind::Binary { op, left, right } => {
                let (left_ty, left_ax_ty) = self.infer_expr_value_type(left)?;
                let (right_ty, _right_ax_ty) = self.infer_expr_value_type(right)?;
                ensure_same_type(&left_ty, &right_ty)?;
                match op {
                    BinaryOp::LogicalOr
                    | BinaryOp::LogicalAnd
                    | BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => Ok(("i1".to_string(), Type::Bool)),
                    BinaryOp::Add if left_ax_ty == Type::String => {
                        Ok(("ptr".to_string(), Type::String))
                    }
                    BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Remainder => Ok((left_ty, left_ax_ty)),
                }
            }
            ExprKind::Call {
                function,
                arguments,
            } => {
                if function == "println" {
                    return Err("println returns <void>, which cannot be a match value".to_string());
                }
                if function == "string_len" || function == "len" {
                    return Ok(("i32".to_string(), Type::I32));
                }
                if function == "env_has" {
                    return Ok(("i1".to_string(), Type::Bool));
                }
                if function == "env_get" {
                    return Ok(("ptr".to_string(), Type::String));
                }
                if matches!(function.as_str(), "fs_exists" | "fs_is_file" | "fs_is_dir") {
                    return Ok(("i1".to_string(), Type::Bool));
                }
                if function == "fs_file_size" {
                    return Ok(("i32".to_string(), Type::I32));
                }
                if function == "fs_read_to_string" {
                    return Ok(("ptr".to_string(), Type::String));
                }
                if function == "fs_read_dir" {
                    let ax_ty = Type::Slice {
                        element: Box::new(Type::String),
                    };
                    let ty =
                        llvm_type(&ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                            "fs_read_dir return slice is outside LLVM AOT v0".to_string()
                        })?;
                    return Ok((ty, ax_ty));
                }
                if function == "fs_copy_file" {
                    return Ok(("i32".to_string(), Type::I32));
                }
                if matches!(
                    function.as_str(),
                    "string_contains" | "string_starts_with" | "string_ends_with"
                ) {
                    return Ok(("i1".to_string(), Type::Bool));
                }
                if function == "string_replace" {
                    return Ok(("ptr".to_string(), Type::String));
                }
                if function == "string_split_lines" {
                    let ax_ty = Type::Slice {
                        element: Box::new(Type::String),
                    };
                    let ty =
                        llvm_type(&ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                            "string_split_lines return slice is outside LLVM AOT v0".to_string()
                        })?;
                    return Ok((ty, ax_ty));
                }
                if function == "string_trim" {
                    return Ok(("ptr".to_string(), Type::String));
                }
                if function == "string_list_new" {
                    return Ok(("ptr".to_string(), Type::StringList));
                }
                if function == "string_list_push" {
                    return Ok(("ptr".to_string(), Type::StringList));
                }
                if function == "string_list_get" || function == "string_list_join" {
                    return Ok(("ptr".to_string(), Type::String));
                }
                if function == "to_string" {
                    return Ok(("ptr".to_string(), Type::String));
                }
                let resolved = self.resolve_call_signature(function, arguments, None)?;
                let signature = resolved.signature;
                if signature.params.len() != arguments.len() {
                    return Err(format!(
                        "call to `{}` has {} argument(s), but LLVM AOT expected {}",
                        resolved.name,
                        arguments.len(),
                        signature.params.len()
                    ));
                }
                Ok((
                    signature.return_type.clone(),
                    signature.return_ax_type.clone(),
                ))
            }
            ExprKind::StructLiteral { name, .. } => {
                let ax_ty = Type::Struct { name: name.clone() };
                let ty = llvm_type(&ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                    format!("struct `{name}` is outside the LLVM AOT v0 layout subset")
                })?;
                Ok((ty, ax_ty))
            }
            ExprKind::ArrayLiteral { elements } => {
                let Some(first) = elements.first() else {
                    return Err(
                        "empty array literals need explicit native array type propagation before LLVM AOT can lower them"
                            .to_string(),
                    );
                };
                let (element_ty, element_ax_ty) = self.infer_expr_value_type(first)?;
                for element in &elements[1..] {
                    let (next_ty, _next_ax_ty) = self.infer_expr_value_type(element)?;
                    ensure_same_type(&element_ty, &next_ty)?;
                }
                let ax_ty = Type::Array {
                    element: Box::new(element_ax_ty),
                    length: elements.len(),
                };
                let ty = llvm_type(&ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                    format!("array type {} is outside LLVM AOT v0", ax_type_name(&ax_ty))
                })?;
                Ok((ty, ax_ty))
            }
            ExprKind::Block { value, .. } => self.infer_expr_value_type(value),
            ExprKind::Match { arms, .. } => {
                let Some(first) = arms.first() else {
                    return Err("match expression has no arms in LLVM AOT v0".to_string());
                };
                let (ty, ax_ty) = self.infer_expr_value_type(&first.value)?;
                for arm in &arms[1..] {
                    let (arm_ty, _) = self.infer_expr_value_type(&arm.value)?;
                    ensure_same_type(&ty, &arm_ty)?;
                }
                Ok((ty, ax_ty))
            }
            ExprKind::EnumVariant { enum_name, .. } => {
                let ax_ty = Type::Enum {
                    name: enum_name.clone(),
                };
                let ty = llvm_type(&ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                    format!("enum `{enum_name}` is outside the LLVM AOT v0 layout subset")
                })?;
                Ok((ty, ax_ty))
            }
            ExprKind::EnumPayload { value } => self.infer_enum_payload_type(value),
            ExprKind::Field { base, field } => {
                let (_, base_ax_ty) = self.infer_expr_value_type(base)?;
                let layout = self.struct_layout_for_type(&base_ax_ty)?;
                let field_layout = layout
                    .fields
                    .iter()
                    .find(|candidate| candidate.name == *field)
                    .ok_or_else(|| {
                        format!(
                            "internal LLVM AOT error: struct `{}` has no field `{field}`",
                            layout.name
                        )
                    })?;
                Ok((field_layout.ty.clone(), field_layout.ax_ty.clone()))
            }
            ExprKind::Index { base, .. } => {
                let (_, base_ax_ty) = self.infer_expr_value_type(base)?;
                match base_ax_ty {
                    Type::Slice { element } => {
                        let element_ty = llvm_type(&element, self.layouts, self.enum_layouts)
                            .ok_or_else(|| {
                                format!(
                                    "slice element type {} is outside LLVM AOT v0",
                                    ax_type_name(&element)
                                )
                            })?;
                        Ok((element_ty, *element))
                    }
                    _ => {
                        let (_, element_ty, _, element_ax_ty) =
                            array_type_parts(&base_ax_ty, self.layouts, self.enum_layouts)?;
                        Ok((element_ty, element_ax_ty))
                    }
                }
            }
            ExprKind::Float { .. } => Ok(("float".to_string(), Type::F32)),
            ExprKind::Const { name } => {
                let binding = self
                    .consts
                    .get(name)
                    .ok_or_else(|| format!("const `{name}` was not found for LLVM AOT lowering"))?;
                let ty =
                    llvm_type(&binding.ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                        format!(
                            "const `{name}` type {} is outside LLVM AOT v0",
                            ax_type_name(&binding.ty)
                        )
                    })?;
                Ok((ty, binding.ty.clone()))
            }
            ExprKind::Try { expr } => {
                let (_, result_ax_ty) = self.infer_expr_value_type(expr)?;
                let (success_ax_ty, _) = self.result_success_error_types(&result_ax_ty)?;
                let success_ty = llvm_type(&success_ax_ty, self.layouts, self.enum_layouts)
                    .ok_or_else(|| {
                        format!(
                            "`?` success type {} is outside LLVM AOT v0",
                            ax_type_name(&success_ax_ty)
                        )
                    })?;
                Ok((success_ty, success_ax_ty))
            }
            ExprKind::MatchTest { .. } => Ok(("i1".to_string(), Type::Bool)),
            ExprKind::Slice { base, .. } => {
                let (_, base_ax_ty) = self.infer_expr_value_type(base)?;
                let element = match base_ax_ty {
                    Type::Array { element, .. } | Type::Slice { element } => element,
                    other => {
                        return Err(format!(
                            "slice expression base type {} is outside LLVM AOT v0",
                            ax_type_name(&other)
                        ));
                    }
                };
                let ax_ty = Type::Slice { element };
                let ty = llvm_type(&ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                    format!("slice type {} is outside LLVM AOT v0", ax_type_name(&ax_ty))
                })?;
                Ok((ty, ax_ty))
            }
        }
    }

    fn infer_enum_payload_type(&self, value: &Expr) -> Result<(String, Type), String> {
        let (_, value_ax_ty) = self.infer_expr_value_type(value)?;
        let enum_ax_ty @ (Type::Enum { .. } | Type::EnumInstance { .. }) = value_ax_ty else {
            return Err(
                "enum payload extraction requires an enum value in LLVM AOT v0".to_string(),
            );
        };
        let enum_name = enum_base_name(&enum_ax_ty).to_string();
        let layout = self.enum_layout_for_type(&enum_ax_ty)?;
        let payload_types = layout
            .variants
            .iter()
            .filter_map(|variant| variant.payload_ax_ty.clone())
            .collect::<Vec<_>>();
        let payload_ax_ty = payload_types
            .first()
            .cloned()
            .ok_or_else(|| format!("enum `{enum_name}` has no payload variant in LLVM AOT v0"))?;
        if payload_types
            .iter()
            .any(|payload| payload != &payload_ax_ty)
        {
            return Err(format!(
                "enum `{enum_name}` payload extraction needs an expected type in LLVM AOT v0"
            ));
        }
        let payload_ty =
            llvm_type(&payload_ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                format!(
                    "enum `{enum_name}` payload type {} is outside LLVM AOT v0",
                    ax_type_name(&payload_ax_ty)
                )
            })?;
        Ok((payload_ty, payload_ax_ty))
    }

    fn result_success_error_types(&self, ty: &Type) -> Result<(Type, Type), String> {
        let Type::EnumInstance { .. } = ty else {
            return Err(format!(
                "`?` result propagation requires a concrete Result<T, E> enum instance, found {}",
                ax_type_name(ty)
            ));
        };
        let layout = self.enum_layout_for_type(ty)?;
        Ok((
            self.enum_variant_payload_type(layout, "Ok")?,
            self.enum_variant_payload_type(layout, "Err")?,
        ))
    }

    fn enum_variant_payload_type(
        &self,
        layout: &EnumLayout,
        variant: &str,
    ) -> Result<Type, String> {
        layout
            .variants
            .iter()
            .find(|candidate| candidate.name == variant)
            .and_then(|candidate| candidate.payload_ax_ty.clone())
            .ok_or_else(|| {
                format!(
                    "enum `{}` must have payload variant `{variant}` for `?` lowering in LLVM AOT v0",
                    layout.name
                )
            })
    }

    fn enum_variant_tag(&self, layout: &EnumLayout, variant: &str) -> Result<i32, String> {
        layout
            .variants
            .iter()
            .find(|candidate| candidate.name == variant)
            .map(|candidate| candidate.tag)
            .ok_or_else(|| {
                format!(
                    "enum `{}` must have variant `{variant}` for `?` lowering in LLVM AOT v0",
                    layout.name
                )
            })
    }

    pub(super) fn local_slot(&self, local: u32) -> Result<&LocalSlot, String> {
        self.locals
            .get(&local)
            .ok_or_else(|| format!("internal LLVM AOT error: missing local slot {local}"))
    }

    fn emit_place_ptr(&mut self, place: &Place, out: &mut String) -> Result<LocalSlot, String> {
        match &place.kind {
            PlaceKind::Local { local, .. } => self.local_slot(*local).cloned(),
            PlaceKind::Field { base, field } => {
                let base_slot = self.emit_place_ptr(base, out)?;
                let layout = self.struct_layout_for_type(&base_slot.ax_ty)?.clone();
                ensure_same_type(&layout.ty, &base_slot.ty)?;
                let field_layout = layout
                    .fields
                    .iter()
                    .find(|candidate| candidate.name == *field)
                    .cloned()
                    .ok_or_else(|| {
                        format!(
                            "internal LLVM AOT error: struct `{}` has no field `{field}`",
                            layout.name
                        )
                    })?;
                let field_ptr = self.next_temp();
                writeln!(
                    out,
                    "  {field_ptr} = getelementptr {}, ptr {}, i32 0, i32 {}",
                    layout.ty, base_slot.ptr, field_layout.index
                )
                .expect("writing to string cannot fail");
                Ok(LocalSlot {
                    ptr: field_ptr,
                    ty: field_layout.ty,
                    ax_ty: field_layout.ax_ty,
                })
            }
            PlaceKind::Index { base, index } => {
                let base_slot = self.emit_place_ptr(base, out)?;
                if let Type::Slice { element } = &base_slot.ax_ty {
                    ensure_same_type(&slice_llvm_type(), &base_slot.ty)?;
                    let index = self.emit_expr(index, out)?;
                    ensure_same_type("i32", &index.ty)?;
                    let slice_value = self.next_temp();
                    writeln!(
                        out,
                        "  {slice_value} = load {}, ptr {}",
                        base_slot.ty, base_slot.ptr
                    )
                    .expect("writing to string cannot fail");
                    let data_ptr = self.next_temp();
                    writeln!(
                        out,
                        "  {data_ptr} = extractvalue {} {slice_value}, 0",
                        base_slot.ty
                    )
                    .expect("writing to string cannot fail");
                    let len = self.next_temp();
                    writeln!(
                        out,
                        "  {len} = extractvalue {} {slice_value}, 1",
                        base_slot.ty
                    )
                    .expect("writing to string cannot fail");
                    self.emit_dynamic_bounds_check(&index.repr, &len, out);
                    let element_ty = llvm_type(element, self.layouts, self.enum_layouts)
                        .ok_or_else(|| {
                            format!(
                                "slice element type {} is outside LLVM AOT v0",
                                ax_type_name(element)
                            )
                        })?;
                    let element_ptr = self.next_temp();
                    writeln!(
                        out,
                        "  {element_ptr} = getelementptr {element_ty}, ptr {data_ptr}, i32 {}",
                        index.repr
                    )
                    .expect("writing to string cannot fail");
                    return Ok(LocalSlot {
                        ptr: element_ptr,
                        ty: element_ty,
                        ax_ty: element.as_ref().clone(),
                    });
                }
                let (array_ty, element_ty, length, element_ax_ty) =
                    array_type_parts(&base_slot.ax_ty, self.layouts, self.enum_layouts)?;
                ensure_same_type(&array_ty, &base_slot.ty)?;
                let index = self.emit_expr(index, out)?;
                ensure_same_type("i32", &index.ty)?;
                self.emit_fixed_bounds_check(&index.repr, length, out);
                let element_ptr = self.next_temp();
                writeln!(
                    out,
                    "  {element_ptr} = getelementptr {array_ty}, ptr {}, i32 0, i32 {}",
                    base_slot.ptr, index.repr
                )
                .expect("writing to string cannot fail");
                Ok(LocalSlot {
                    ptr: element_ptr,
                    ty: element_ty,
                    ax_ty: element_ax_ty,
                })
            }
        }
    }

    fn struct_layout_by_name(&self, name: &str) -> Result<&StructLayout, String> {
        self.layouts
            .get(name)
            .ok_or_else(|| format!("struct `{name}` is outside the LLVM AOT v0 layout subset"))
    }

    fn struct_layout_for_type(&self, ty: &Type) -> Result<&StructLayout, String> {
        match ty {
            Type::Struct { name } => self.struct_layout_by_name(name),
            Type::StructInstance { .. } => self.layouts.get(&struct_layout_key(ty)).ok_or_else(|| {
                format!(
                    "generic struct instance `{}` needs monomorphized native layout before LLVM AOT can lower it",
                    ax_type_name(ty)
                )
            }),
            _ => Err(format!(
                "field access base type {} is not a struct in LLVM AOT v0",
                ax_type_name(ty)
            )),
        }
    }

    fn struct_literal_layout(
        &self,
        name: &str,
        expected_ax_ty: Option<&Type>,
    ) -> Result<&StructLayout, String> {
        if let Some(
            expected @ (Type::Struct {
                name: expected_name,
            }
            | Type::StructInstance {
                name: expected_name,
                ..
            }),
        ) = expected_ax_ty
            && expected_name == name
        {
            return self.struct_layout_for_type(expected);
        }
        self.struct_layout_by_name(name)
    }

    fn enum_layout_for_type(&self, ty: &Type) -> Result<&EnumLayout, String> {
        self.enum_layouts.get(&enum_layout_key(ty)).ok_or_else(|| {
            format!(
                "enum `{}` is outside the LLVM AOT v0 layout subset",
                ax_type_name(ty)
            )
        })
    }

    fn enum_layout_for_value(&self, value: &LlvmValue) -> Result<&EnumLayout, String> {
        let Some(enum_ax_ty @ (Type::Enum { .. } | Type::EnumInstance { .. })) =
            value.ax_ty.as_ref()
        else {
            return Err("enum operation requires an enum value in LLVM AOT v0".to_string());
        };
        self.enum_layout_for_type(enum_ax_ty)
    }

    fn enum_layout_for_constructor(
        &self,
        enum_name: &str,
        expected_ax_ty: Option<&Type>,
    ) -> Result<&EnumLayout, String> {
        if let Some(expected @ (Type::Enum { .. } | Type::EnumInstance { .. })) = expected_ax_ty {
            if enum_base_name(expected) != enum_name {
                return Err(format!(
                    "enum constructor `{enum_name}` does not match expected type {} in LLVM AOT v0",
                    ax_type_name(expected)
                ));
            }
            return self.enum_layout_for_type(expected);
        }

        let mut candidates = self
            .enum_layouts
            .values()
            .filter(|layout| layout.name == enum_name);
        let Some(first) = candidates.next() else {
            return Err(format!(
                "enum `{enum_name}` is outside the LLVM AOT v0 layout subset"
            ));
        };
        if candidates.next().is_some() {
            return Err(format!(
                "generic enum constructor `{enum_name}` needs an expected concrete enum type before LLVM AOT can lower it"
            ));
        }
        Ok(first)
    }

    fn enum_value_type_for_layout(
        &self,
        layout: &EnumLayout,
        expected_ax_ty: Option<&Type>,
    ) -> Result<Type, String> {
        if let Some(expected @ (Type::Enum { .. } | Type::EnumInstance { .. })) = expected_ax_ty {
            return Ok(expected.clone());
        }

        self.enum_layouts
            .values()
            .find_map(|candidate| {
                if candidate.ty == layout.ty && candidate.name == layout.name {
                    Some(candidate.ax_ty.clone())
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                format!(
                    "internal LLVM AOT error: missing enum value type for layout `{}`",
                    layout.ty
                )
            })
    }

    fn string_literal_symbol(&self, value: &str) -> Result<String, String> {
        self.strings
            .get(value)
            .map(|literal| literal.symbol.clone())
            .ok_or_else(|| {
                format!("internal LLVM AOT error: missing generated string literal `{value}`")
            })
    }

    fn next_temp(&mut self) -> String {
        let temp = format!("%t{}", self.temp_index);
        self.temp_index += 1;
        temp
    }

    fn next_label(&mut self, prefix: &str) -> String {
        let label = format!("{}_{}", prefix, self.temp_index);
        self.temp_index += 1;
        label
    }
}
