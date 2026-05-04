use std::fmt::Write;

use super::*;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_struct_literal(
        &mut self,
        name: &str,
        fields: &[StructLiteralField],
        expected_ax_ty: Option<&Type>,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let layout = self.struct_literal_layout(name, expected_ax_ty)?.clone();
        let mut values = BTreeMap::new();
        for field in fields {
            let value = self.emit_expr(&field.value, out)?;
            values.insert(field.name.clone(), value);
        }

        let mut rendered_fields = Vec::new();
        for field in &layout.fields {
            let value = values.remove(&field.name).ok_or_else(|| {
                format!(
                    "internal LLVM AOT error: struct `{}` literal is missing field `{}`",
                    layout.name, field.name
                )
            })?;
            ensure_same_type(&field.ty, &value.ty)?;
            rendered_fields.push((field.ty.clone(), value.repr));
        }
        if let Some(extra) = values.keys().next() {
            return Err(format!(
                "internal LLVM AOT error: struct `{}` literal has unknown field `{extra}`",
                layout.name
            ));
        }

        Ok(LlvmValue {
            ty: layout.ty.clone(),
            repr: self.emit_aggregate_value(&layout.ty, &rendered_fields, out),
            ax_ty: Some(layout.ax_ty.clone()),
        })
    }

    pub(super) fn emit_aggregate_value(
        &mut self,
        aggregate_ty: &str,
        fields: &[(String, String)],
        out: &mut String,
    ) -> String {
        let mut current = "undef".to_string();
        for (index, (field_ty, field_repr)) in fields.iter().enumerate() {
            let temp = self.next_temp();
            writeln!(
                out,
                "  {temp} = insertvalue {aggregate_ty} {current}, {field_ty} {field_repr}, {index}"
            )
            .expect("writing to string cannot fail");
            current = temp;
        }
        current
    }

    pub(super) fn emit_field(
        &mut self,
        base: &Expr,
        field: &str,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let (base_ptr, layout) = self.emit_struct_base_ptr(base, out)?;
        let field_layout = layout
            .fields
            .iter()
            .find(|candidate| candidate.name == field)
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
            "  {field_ptr} = getelementptr {}, ptr {base_ptr}, i32 0, i32 {}",
            layout.ty, field_layout.index
        )
        .expect("writing to string cannot fail");
        let loaded = self.next_temp();
        writeln!(
            out,
            "  {loaded} = load {}, ptr {field_ptr}",
            field_layout.ty
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: field_layout.ty,
            repr: loaded,
            ax_ty: Some(field_layout.ax_ty),
        })
    }

    pub(super) fn emit_struct_base_ptr(
        &mut self,
        base: &Expr,
        out: &mut String,
    ) -> Result<(String, StructLayout), String> {
        if let ExprKind::Local { local, .. } = &base.kind {
            let slot = self.local_slot(*local)?.clone();
            let layout = self.struct_layout_for_type(&slot.ax_ty)?.clone();
            ensure_same_type(&layout.ty, &slot.ty)?;
            return Ok((slot.ptr, layout));
        }

        let value = self.emit_expr(base, out)?;
        let Some(ax_ty) = value.ax_ty.clone() else {
            return Err("field access base is not a struct in LLVM AOT v0".to_string());
        };
        let layout = self.struct_layout_for_type(&ax_ty)?.clone();
        ensure_same_type(&layout.ty, &value.ty)?;
        let temp_struct = self.next_temp();
        writeln!(out, "  {temp_struct} = alloca {}", layout.ty)
            .expect("writing to string cannot fail");
        writeln!(
            out,
            "  store {} {}, ptr {temp_struct}",
            layout.ty, value.repr
        )
        .expect("writing to string cannot fail");
        Ok((temp_struct, layout))
    }

    pub(super) fn emit_enum_variant(
        &mut self,
        enum_name: &str,
        variant: &str,
        payload: Option<&Expr>,
        expected_ax_ty: Option<&Type>,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let payload_value = payload
            .map(|payload_expr| self.emit_expr(payload_expr, out))
            .transpose()?;
        self.emit_enum_variant_value(enum_name, variant, payload_value, expected_ax_ty, out)
    }

    pub(super) fn emit_enum_variant_value(
        &mut self,
        enum_name: &str,
        variant: &str,
        payload_value: Option<LlvmValue>,
        expected_ax_ty: Option<&Type>,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let layout = self
            .enum_layout_for_constructor(enum_name, expected_ax_ty)?
            .clone();
        let variant_layout = layout
            .variants
            .iter()
            .find(|candidate| candidate.name == variant)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "internal LLVM AOT error: enum `{}` has no variant `{variant}`",
                    layout.name
                )
            })?;

        if layout.ty == "i32" {
            if payload_value.is_some() {
                return Err(format!(
                    "enum `{enum_name}.{variant}` does not carry a payload in LLVM AOT v0"
                ));
            }
            return Ok(LlvmValue {
                ty: layout.ty.clone(),
                repr: variant_layout.tag.to_string(),
                ax_ty: Some(self.enum_value_type_for_layout(&layout, expected_ax_ty)?),
            });
        }

        let payload_ptr = match (&variant_layout.payload_ax_ty, payload_value) {
            (Some(payload_ax_ty), Some(payload_value)) => {
                let payload_ty = llvm_type(payload_ax_ty, self.layouts, self.enum_layouts)
                    .ok_or_else(|| {
                        format!(
                            "enum `{enum_name}.{variant}` payload type {} is outside LLVM AOT v0",
                            ax_type_name(payload_ax_ty)
                        )
                    })?;
                ensure_same_type(&payload_ty, &payload_value.ty)?;
                let payload_size = llvm_alloc_size(payload_ax_ty, self.layouts, self.enum_layouts)
                    .ok_or_else(|| {
                        format!(
                            "enum `{enum_name}.{variant}` payload type {} needs a native size contract before LLVM AOT can lower it",
                            ax_type_name(payload_ax_ty)
                        )
                    })?;
                let payload_ptr = self.next_temp();
                writeln!(
                    out,
                    "  {payload_ptr} = call ptr @malloc(i64 {payload_size})"
                )
                .expect("writing to string cannot fail");
                writeln!(
                    out,
                    "  store {payload_ty} {}, ptr {payload_ptr}",
                    payload_value.repr
                )
                .expect("writing to string cannot fail");
                payload_ptr
            }
            (Some(_), None) => {
                return Err(format!(
                    "enum `{enum_name}.{variant}` requires a payload in LLVM AOT v0"
                ));
            }
            (None, Some(_)) => {
                return Err(format!(
                    "enum `{enum_name}.{variant}` does not carry a payload in LLVM AOT v0"
                ));
            }
            (None, None) => "null".to_string(),
        };

        let repr = self.emit_aggregate_value(
            &layout.ty,
            &[
                ("i32".to_string(), variant_layout.tag.to_string()),
                ("ptr".to_string(), payload_ptr),
            ],
            out,
        );
        Ok(LlvmValue {
            ty: layout.ty.clone(),
            repr,
            ax_ty: Some(self.enum_value_type_for_layout(&layout, expected_ax_ty)?),
        })
    }

    pub(super) fn emit_try(&mut self, inner: &Expr, out: &mut String) -> Result<LlvmValue, String> {
        let result_value = self.emit_expr(inner, out)?;
        let Some(result_ax_ty @ Type::EnumInstance { .. }) = result_value.ax_ty.clone() else {
            return Err(
                "`?` result propagation requires a concrete Result<T, E> value in LLVM AOT v0"
                    .to_string(),
            );
        };
        let (success_ax_ty, error_ax_ty) = self.result_success_error_types(&result_ax_ty)?;
        let (_return_success_ax_ty, return_error_ax_ty) =
            self.result_success_error_types(&self.return_ax_ty)?;
        if error_ax_ty != return_error_ax_ty {
            return Err(format!(
                "`?` result propagation error type {} does not match current function error type {} in LLVM AOT v0",
                ax_type_name(&error_ax_ty),
                ax_type_name(&return_error_ax_ty)
            ));
        }

        let layout = self.enum_layout_for_type(&result_ax_ty)?.clone();
        ensure_same_type(&layout.ty, &result_value.ty)?;
        if layout.ty == "i32" {
            return Err(
                "`?` result propagation requires payload enum layout in LLVM AOT v0".to_string(),
            );
        }

        let ok_tag = self.enum_variant_tag(&layout, "Ok")?;
        let tag_value = self.next_temp();
        writeln!(
            out,
            "  {tag_value} = extractvalue {} {}, 0",
            layout.ty, result_value.repr
        )
        .expect("writing to string cannot fail");
        let is_ok = self.next_temp();
        writeln!(out, "  {is_ok} = icmp eq i32 {tag_value}, {ok_tag}")
            .expect("writing to string cannot fail");

        let ok_label = self.next_label("try_ok");
        let err_label = self.next_label("try_err");
        writeln!(
            out,
            "  br i1 {is_ok}, label %{ok_label}, label %{err_label}"
        )
        .expect("writing to string cannot fail");

        writeln!(out, "{err_label}:").expect("writing to string cannot fail");
        let error_value = self.emit_enum_payload_value(&result_value, Some(&error_ax_ty), out)?;
        let return_ax_ty = self.return_ax_ty.clone();
        let return_enum_name = enum_base_name(&return_ax_ty).to_string();
        let return_value = self.emit_enum_variant_value(
            &return_enum_name,
            "Err",
            Some(error_value),
            Some(&return_ax_ty),
            out,
        )?;
        let return_ty =
            llvm_type(&return_ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                format!(
                    "function return type {} is outside LLVM AOT v0",
                    ax_type_name(&return_ax_ty)
                )
            })?;
        ensure_same_type(&return_ty, &return_value.ty)?;
        writeln!(out, "  ret {} {}", return_value.ty, return_value.repr)
            .expect("writing to string cannot fail");

        writeln!(out, "{ok_label}:").expect("writing to string cannot fail");
        self.emit_enum_payload_value(&result_value, Some(&success_ax_ty), out)
    }

    pub(super) fn emit_enum_payload(
        &mut self,
        value: &Expr,
        expected_ax_ty: Option<&Type>,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let value = self.emit_expr(value, out)?;
        self.emit_enum_payload_value(&value, expected_ax_ty, out)
    }

    pub(super) fn emit_enum_payload_value(
        &mut self,
        value: &LlvmValue,
        expected_ax_ty: Option<&Type>,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let Some(enum_ax_ty @ (Type::Enum { .. } | Type::EnumInstance { .. })) =
            value.ax_ty.clone()
        else {
            return Err(
                "enum payload extraction requires an enum value in LLVM AOT v0".to_string(),
            );
        };
        let enum_name = enum_base_name(&enum_ax_ty).to_string();
        let layout = self.enum_layout_for_type(&enum_ax_ty)?.clone();
        if layout.ty == "i32" {
            return Err(format!(
                "enum `{enum_name}` has no native payload storage in LLVM AOT v0"
            ));
        }

        let payload_ax_ty = match expected_ax_ty {
            Some(expected) => {
                if !layout
                    .variants
                    .iter()
                    .any(|variant| variant.payload_ax_ty.as_ref() == Some(expected))
                {
                    return Err(format!(
                        "enum `{enum_name}` has no payload variant with type {} in LLVM AOT v0",
                        ax_type_name(expected)
                    ));
                }
                expected.clone()
            }
            None => {
                let payload_types = layout
                    .variants
                    .iter()
                    .filter_map(|variant| variant.payload_ax_ty.clone())
                    .collect::<Vec<_>>();
                let first = payload_types.first().cloned().ok_or_else(|| {
                    format!("enum `{enum_name}` has no payload variant in LLVM AOT v0")
                })?;
                if payload_types.iter().any(|payload| payload != &first) {
                    return Err(format!(
                        "enum `{enum_name}` payload extraction needs an expected type in LLVM AOT v0"
                    ));
                }
                first
            }
        };

        let payload_ty =
            llvm_type(&payload_ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                format!(
                    "enum `{enum_name}` payload type {} is outside LLVM AOT v0",
                    ax_type_name(&payload_ax_ty)
                )
            })?;
        let payload_ptr = self.next_temp();
        writeln!(
            out,
            "  {payload_ptr} = extractvalue {} {}, 1",
            layout.ty, value.repr
        )
        .expect("writing to string cannot fail");
        let loaded = self.next_temp();
        writeln!(out, "  {loaded} = load {payload_ty}, ptr {payload_ptr}")
            .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: payload_ty,
            repr: loaded,
            ax_ty: Some(payload_ax_ty),
        })
    }

    pub(super) fn emit_match_test(
        &mut self,
        scrutinee: &Expr,
        pattern: &MatchPattern,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let value = self.emit_expr(scrutinee, out)?;
        self.emit_match_test_value(&value, pattern, out)
    }

    pub(super) fn emit_match_test_value(
        &mut self,
        value: &LlvmValue,
        pattern: &MatchPattern,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        match &pattern.kind {
            MatchPatternKind::Wildcard | MatchPatternKind::Binding { .. } => Ok(LlvmValue {
                ty: "i1".to_string(),
                repr: "1".to_string(),
                ax_ty: Some(Type::Bool),
            }),
            MatchPatternKind::Bool { value: expected } => {
                ensure_same_type("i1", &value.ty)?;
                let result = self.next_temp();
                let expected = if *expected { "1" } else { "0" };
                writeln!(out, "  {result} = icmp eq i1 {}, {expected}", value.repr)
                    .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: result,
                    ax_ty: Some(Type::Bool),
                })
            }
            MatchPatternKind::Int { value: expected } => {
                ensure_same_type("i32", &value.ty)?;
                let result = self.next_temp();
                writeln!(out, "  {result} = icmp eq i32 {}, {expected}", value.repr)
                    .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: result,
                    ax_ty: Some(Type::Bool),
                })
            }
            MatchPatternKind::IntRange { start, end } => {
                ensure_same_type("i32", &value.ty)?;
                let lower = self.next_temp();
                let upper = self.next_temp();
                let result = self.next_temp();
                writeln!(out, "  {lower} = icmp sge i32 {}, {start}", value.repr)
                    .expect("writing to string cannot fail");
                writeln!(out, "  {upper} = icmp sle i32 {}, {end}", value.repr)
                    .expect("writing to string cannot fail");
                writeln!(out, "  {result} = and i1 {lower}, {upper}")
                    .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: result,
                    ax_ty: Some(Type::Bool),
                })
            }
            MatchPatternKind::String { value: expected } => {
                ensure_same_type("ptr", &value.ty)?;
                let literal = self.strings.get(expected).ok_or_else(|| {
                    "internal LLVM AOT error: missing string pattern literal".to_string()
                })?;
                let compare = self.next_temp();
                let result = self.next_temp();
                writeln!(
                    out,
                    "  {compare} = call i32 @strcmp(ptr {}, ptr {})",
                    value.repr, literal.symbol
                )
                .expect("writing to string cannot fail");
                writeln!(out, "  {result} = icmp eq i32 {compare}, 0")
                    .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: result,
                    ax_ty: Some(Type::Bool),
                })
            }
            MatchPatternKind::EnumVariant {
                enum_name,
                variant,
                payload,
                ..
            } => {
                let Some(value_ax_ty @ (Type::Enum { .. } | Type::EnumInstance { .. })) =
                    value.ax_ty.as_ref()
                else {
                    return Err(format!(
                        "match pattern `{enum_name}.{variant}` cannot be tested against {} in LLVM AOT v0",
                        value
                            .ax_ty
                            .as_ref()
                            .map(ax_type_name)
                            .unwrap_or_else(|| value.ty.clone())
                    ));
                };
                if enum_base_name(value_ax_ty) != enum_name {
                    return Err(format!(
                        "match pattern `{enum_name}.{variant}` cannot be tested against {} in LLVM AOT v0",
                        value
                            .ax_ty
                            .as_ref()
                            .map(ax_type_name)
                            .unwrap_or_else(|| value.ty.clone())
                    ));
                }
                let layout = self.enum_layout_for_type(value_ax_ty)?;
                let layout_name = layout.name.clone();
                let layout_ty = layout.ty.clone();
                let tag = layout
                    .variants
                    .iter()
                    .find(|candidate| candidate.name == *variant)
                    .map(|candidate| candidate.tag)
                    .ok_or_else(|| {
                        format!(
                            "internal LLVM AOT error: enum `{}` has no variant `{variant}`",
                            layout_name
                        )
                    })?;
                if payload.is_some()
                    && !layout.variants.iter().any(|candidate| {
                        candidate.name == *variant && candidate.payload_ax_ty.is_some()
                    })
                {
                    return Err(format!(
                        "match pattern `{enum_name}.{variant}(...)` targets a unit variant in LLVM AOT v0"
                    ));
                }
                let tag_value = if layout_ty == "i32" {
                    value.repr.clone()
                } else {
                    let tag_value = self.next_temp();
                    writeln!(
                        out,
                        "  {tag_value} = extractvalue {layout_ty} {}, 0",
                        value.repr
                    )
                    .expect("writing to string cannot fail");
                    tag_value
                };
                let result = self.next_temp();
                writeln!(out, "  {result} = icmp eq i32 {tag_value}, {tag}")
                    .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: result,
                    ax_ty: Some(Type::Bool),
                })
            }
            MatchPatternKind::Struct {
                struct_name,
                fields,
            } => {
                let Some(value_ax_ty @ Type::Struct { .. }) = value.ax_ty.as_ref() else {
                    return Err(format!(
                        "struct pattern `{struct_name} {{ ... }}` cannot be tested against {} in LLVM AOT v0",
                        value
                            .ax_ty
                            .as_ref()
                            .map(ax_type_name)
                            .unwrap_or_else(|| value.ty.clone())
                    ));
                };
                let layout = self.struct_layout_for_type(value_ax_ty)?;
                if layout.name != *struct_name {
                    return Err(format!(
                        "struct pattern `{struct_name} {{ ... }}` cannot be tested against {} in LLVM AOT v0",
                        ax_type_name(value_ax_ty)
                    ));
                }
                ensure_same_type(&layout.ty, &value.ty)?;
                for field in fields {
                    if !layout
                        .fields
                        .iter()
                        .any(|candidate| candidate.name == field.name)
                    {
                        return Err(format!(
                            "internal LLVM AOT error: struct `{struct_name}` has no field `{}`",
                            field.name
                        ));
                    }
                }
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: "1".to_string(),
                    ax_ty: Some(Type::Bool),
                })
            }
            MatchPatternKind::Error => {
                Err("match pattern requires native lowering outside LLVM AOT v0".to_string())
            }
            MatchPatternKind::Or { alternatives } => {
                if alternatives.iter().any(match_pattern_contains_binding) {
                    return Err(
                        "or patterns with bindings need binding merge semantics before LLVM AOT can lower them"
                            .to_string(),
                    );
                }
                let Some(first) = alternatives.first() else {
                    return Ok(LlvmValue {
                        ty: "i1".to_string(),
                        repr: "0".to_string(),
                        ax_ty: Some(Type::Bool),
                    });
                };
                let mut result = self.emit_match_test_value(value, first, out)?;
                ensure_same_type("i1", &result.ty)?;
                for alternative in &alternatives[1..] {
                    let next = self.emit_match_test_value(value, alternative, out)?;
                    ensure_same_type("i1", &next.ty)?;
                    let combined = self.next_temp();
                    writeln!(out, "  {combined} = or i1 {}, {}", result.repr, next.repr)
                        .expect("writing to string cannot fail");
                    result = LlvmValue {
                        ty: "i1".to_string(),
                        repr: combined,
                        ax_ty: Some(Type::Bool),
                    };
                }
                Ok(result)
            }
        }
    }

    pub(super) fn emit_match_bindings(
        &mut self,
        scrutinee: &LlvmValue,
        arm: &MatchExprArm,
        out: &mut String,
    ) -> Result<(), String> {
        match &arm.pattern.kind {
            MatchPatternKind::Binding { name } => {
                if let Some(local) = find_local_use_by_name_in_match_arm(arm, name) {
                    let slot = self.local_slot(local)?.clone();
                    if slot.ty == scrutinee.ty {
                        writeln!(
                            out,
                            "  store {} {}, ptr {}",
                            slot.ty, scrutinee.repr, slot.ptr
                        )
                        .expect("writing to string cannot fail");
                    }
                }
                Ok(())
            }
            MatchPatternKind::EnumVariant {
                variant,
                payload: Some(EnumVariantPayloadPattern::Binding { name: payload_name }),
                ..
            } => {
                if let Some(local) = find_local_use_by_name_in_match_arm(arm, payload_name) {
                    let slot = self.local_slot(local)?.clone();
                    let payload_ax_ty = self
                        .enum_layout_for_value(scrutinee)?
                        .variants
                        .iter()
                        .find(|candidate| candidate.name == *variant)
                        .and_then(|candidate| candidate.payload_ax_ty.clone())
                        .ok_or_else(|| {
                            format!(
                                "match pattern payload `{variant}` needs a native payload type in LLVM AOT v0"
                            )
                        })?;
                    let expected_ty = llvm_type(&payload_ax_ty, self.layouts, self.enum_layouts)
                        .ok_or_else(|| {
                            format!(
                                "enum payload type {} is outside LLVM AOT v0",
                                ax_type_name(&payload_ax_ty)
                            )
                        })?;
                    if slot.ty == expected_ty {
                        let value =
                            self.emit_enum_payload_value(scrutinee, Some(&payload_ax_ty), out)?;
                        ensure_same_type(&slot.ty, &value.ty)?;
                        writeln!(out, "  store {} {}, ptr {}", slot.ty, value.repr, slot.ptr)
                            .expect("writing to string cannot fail");
                    }
                }
                Ok(())
            }
            MatchPatternKind::Struct {
                struct_name,
                fields,
                ..
            } => {
                let Some(scrutinee_ax_ty @ Type::Struct { .. }) = scrutinee.ax_ty.as_ref() else {
                    return Err(format!(
                        "struct pattern `{struct_name} {{ ... }}` requires a struct value in LLVM AOT v0"
                    ));
                };
                let layout = self.struct_layout_for_type(scrutinee_ax_ty)?.clone();
                if layout.name != *struct_name {
                    return Err(format!(
                        "struct pattern `{struct_name} {{ ... }}` cannot bind fields from {} in LLVM AOT v0",
                        ax_type_name(scrutinee_ax_ty)
                    ));
                }
                ensure_same_type(&layout.ty, &scrutinee.ty)?;
                for field in fields {
                    if let Some(local) = find_local_use_by_name_in_match_arm(arm, &field.binding) {
                        let slot = self.local_slot(local)?.clone();
                        let field_layout = layout
                            .fields
                            .iter()
                            .find(|candidate| candidate.name == field.name)
                            .cloned()
                            .ok_or_else(|| {
                                format!(
                                    "internal LLVM AOT error: struct `{}` has no field `{}`",
                                    layout.name, field.name
                                )
                            })?;
                        ensure_same_type(&field_layout.ty, &slot.ty)?;
                        let value = self.next_temp();
                        writeln!(
                            out,
                            "  {value} = extractvalue {} {}, {}",
                            layout.ty, scrutinee.repr, field_layout.index
                        )
                        .expect("writing to string cannot fail");
                        writeln!(out, "  store {} {value}, ptr {}", slot.ty, slot.ptr)
                            .expect("writing to string cannot fail");
                    }
                }
                Ok(())
            }
            MatchPatternKind::Wildcard
            | MatchPatternKind::Bool { .. }
            | MatchPatternKind::Int { .. }
            | MatchPatternKind::IntRange { .. }
            | MatchPatternKind::String { .. }
            | MatchPatternKind::EnumVariant { .. }
            | MatchPatternKind::Or { .. }
            | MatchPatternKind::Error => Ok(()),
        }
    }

    pub(super) fn emit_binary(
        &mut self,
        op: BinaryOp,
        left: LlvmValue,
        right: LlvmValue,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        ensure_same_type(&left.ty, &right.ty)?;
        if is_enum_value(&left) || is_enum_value(&right) {
            if left.ax_ty != right.ax_ty {
                return Err(
                    "enum comparison in LLVM AOT v0 requires both operands to have the same enum type"
                        .to_string(),
                );
            }
            if !matches!(op, BinaryOp::Equal | BinaryOp::NotEqual) {
                return Err(
                    "enum values only support equality comparisons in LLVM AOT v0".to_string(),
                );
            }
            return self.emit_enum_equality(op, left, right, out);
        }
        let temp = self.next_temp();
        match op {
            BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Remainder => {
                if left.ty == "ptr" {
                    if matches!(op, BinaryOp::Add) {
                        writeln!(
                            out,
                            "  {temp} = call ptr @ax_string_concat(ptr {}, ptr {})",
                            left.repr, right.repr
                        )
                        .expect("writing to string cannot fail");
                        return Ok(LlvmValue {
                            ty: "ptr".to_string(),
                            repr: temp,
                            ax_ty: Some(Type::String),
                        });
                    }
                    return Err(format!(
                        "string values do not support `{}` in LLVM AOT v0",
                        llvm_binary_op_name(op)
                    ));
                }
                if left.ty == "float" {
                    let instruction = match op {
                        BinaryOp::Add => "fadd",
                        BinaryOp::Subtract => "fsub",
                        BinaryOp::Multiply => "fmul",
                        BinaryOp::Divide => "fdiv",
                        BinaryOp::Remainder => {
                            return Err("f32 values do not support `%` in LLVM AOT v0".to_string());
                        }
                        _ => unreachable!(),
                    };
                    if matches!(op, BinaryOp::Divide) {
                        self.emit_float_zero_divisor_check(&right.repr, out);
                    }
                    writeln!(
                        out,
                        "  {temp} = {instruction} float {}, {}",
                        left.repr, right.repr
                    )
                    .expect("writing to string cannot fail");
                    return Ok(LlvmValue {
                        ty: "float".to_string(),
                        repr: temp,
                        ax_ty: Some(Type::F32),
                    });
                }
                ensure_same_type("i32", &left.ty)?;
                let instruction = match op {
                    BinaryOp::Add => "add",
                    BinaryOp::Subtract => "sub",
                    BinaryOp::Multiply => "mul",
                    BinaryOp::Divide => "sdiv",
                    BinaryOp::Remainder => "srem",
                    _ => unreachable!(),
                };
                match op {
                    BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply => {
                        return Ok(self.emit_checked_i32_arithmetic(op, &left, &right, out));
                    }
                    BinaryOp::Divide => {
                        self.emit_i32_zero_divisor_check(&right.repr, "@.ax_rt_div_zero", out);
                        self.emit_i32_signed_min_overflow_check(
                            &left.repr,
                            &right.repr,
                            "i32_div_overflow",
                            "@.ax_rt_div_overflow",
                            out,
                        );
                    }
                    BinaryOp::Remainder => {
                        self.emit_i32_zero_divisor_check(&right.repr, "@.ax_rt_mod_zero", out);
                        self.emit_i32_signed_min_overflow_check(
                            &left.repr,
                            &right.repr,
                            "i32_rem_overflow",
                            "@.ax_rt_rem_overflow",
                            out,
                        );
                    }
                    _ => {}
                }
                writeln!(
                    out,
                    "  {temp} = {instruction} i32 {}, {}",
                    left.repr, right.repr
                )
                .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i32".to_string(),
                    repr: temp,
                    ax_ty: Some(Type::I32),
                })
            }
            BinaryOp::LogicalAnd | BinaryOp::LogicalOr => {
                ensure_same_type("i1", &left.ty)?;
                let instruction = if matches!(op, BinaryOp::LogicalAnd) {
                    "and"
                } else {
                    "or"
                };
                writeln!(
                    out,
                    "  {temp} = {instruction} i1 {}, {}",
                    left.repr, right.repr
                )
                .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: temp,
                    ax_ty: Some(Type::Bool),
                })
            }
            BinaryOp::Equal | BinaryOp::NotEqual => {
                let result = self.emit_value_equality(op, &left, &right, out)?;
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: result,
                    ax_ty: Some(Type::Bool),
                })
            }
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
                if left.ty == "float" {
                    let predicate = match op {
                        BinaryOp::Less => "olt",
                        BinaryOp::LessEqual => "ole",
                        BinaryOp::Greater => "ogt",
                        BinaryOp::GreaterEqual => "oge",
                        _ => unreachable!(),
                    };
                    writeln!(
                        out,
                        "  {temp} = fcmp {predicate} float {}, {}",
                        left.repr, right.repr
                    )
                    .expect("writing to string cannot fail");
                    return Ok(LlvmValue {
                        ty: "i1".to_string(),
                        repr: temp,
                        ax_ty: Some(Type::Bool),
                    });
                }
                ensure_same_type("i32", &left.ty)?;
                let predicate = match op {
                    BinaryOp::Less => "slt",
                    BinaryOp::LessEqual => "sle",
                    BinaryOp::Greater => "sgt",
                    BinaryOp::GreaterEqual => "sge",
                    _ => unreachable!(),
                };
                writeln!(
                    out,
                    "  {temp} = icmp {predicate} i32 {}, {}",
                    left.repr, right.repr
                )
                .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: temp,
                    ax_ty: Some(Type::Bool),
                })
            }
        }
    }

    fn emit_checked_i32_arithmetic(
        &mut self,
        op: BinaryOp,
        left: &LlvmValue,
        right: &LlvmValue,
        out: &mut String,
    ) -> LlvmValue {
        let (intrinsic, label_prefix, message) = match op {
            BinaryOp::Add => (
                "@llvm.sadd.with.overflow.i32",
                "i32_add_overflow",
                "@.ax_rt_add_overflow",
            ),
            BinaryOp::Subtract => (
                "@llvm.ssub.with.overflow.i32",
                "i32_sub_overflow",
                "@.ax_rt_sub_overflow",
            ),
            BinaryOp::Multiply => (
                "@llvm.smul.with.overflow.i32",
                "i32_mul_overflow",
                "@.ax_rt_mul_overflow",
            ),
            _ => unreachable!("checked i32 arithmetic only handles add/sub/mul"),
        };
        let checked = self.next_temp();
        writeln!(
            out,
            "  {checked} = call {{ i32, i1 }} {intrinsic}(i32 {}, i32 {})",
            left.repr, right.repr
        )
        .expect("writing to string cannot fail");
        let value = self.next_temp();
        writeln!(out, "  {value} = extractvalue {{ i32, i1 }} {checked}, 0")
            .expect("writing to string cannot fail");
        let overflow = self.next_temp();
        writeln!(
            out,
            "  {overflow} = extractvalue {{ i32, i1 }} {checked}, 1"
        )
        .expect("writing to string cannot fail");
        self.emit_runtime_error_if(&overflow, label_prefix, message, out);
        LlvmValue {
            ty: "i32".to_string(),
            repr: value,
            ax_ty: Some(Type::I32),
        }
    }

    fn emit_i32_zero_divisor_check(&mut self, divisor: &str, message: &str, out: &mut String) {
        let is_zero = self.next_temp();
        writeln!(out, "  {is_zero} = icmp eq i32 {divisor}, 0")
            .expect("writing to string cannot fail");
        self.emit_runtime_error_if(&is_zero, "i32_div_zero", message, out);
    }

    fn emit_i32_signed_min_overflow_check(
        &mut self,
        left: &str,
        right: &str,
        label_prefix: &str,
        message: &str,
        out: &mut String,
    ) {
        let is_min = self.next_temp();
        let is_neg_one = self.next_temp();
        let overflow = self.next_temp();
        writeln!(out, "  {is_min} = icmp eq i32 {left}, -2147483648")
            .expect("writing to string cannot fail");
        writeln!(out, "  {is_neg_one} = icmp eq i32 {right}, -1")
            .expect("writing to string cannot fail");
        writeln!(out, "  {overflow} = and i1 {is_min}, {is_neg_one}")
            .expect("writing to string cannot fail");
        self.emit_runtime_error_if(&overflow, label_prefix, message, out);
    }

    fn emit_float_zero_divisor_check(&mut self, divisor: &str, out: &mut String) {
        let is_zero = self.next_temp();
        writeln!(
            out,
            "  {is_zero} = fcmp oeq float {divisor}, {}",
            llvm_float_literal(0.0)
        )
        .expect("writing to string cannot fail");
        self.emit_runtime_error_if(&is_zero, "f32_div_zero", "@.ax_rt_div_zero", out);
    }

    pub(super) fn emit_runtime_error_if(
        &mut self,
        condition: &str,
        label_prefix: &str,
        message: &str,
        out: &mut String,
    ) {
        let fail_label = self.next_label(label_prefix);
        let ok_label = self.next_label(&format!("{label_prefix}_ok"));
        writeln!(
            out,
            "  br i1 {condition}, label %{fail_label}, label %{ok_label}"
        )
        .expect("writing to string cannot fail");
        writeln!(out, "{fail_label}:").expect("writing to string cannot fail");
        writeln!(out, "  call void @ax_runtime_error(ptr {message})")
            .expect("writing to string cannot fail");
        writeln!(out, "  unreachable").expect("writing to string cannot fail");
        writeln!(out, "{ok_label}:").expect("writing to string cannot fail");
    }

    pub(super) fn emit_enum_equality(
        &mut self,
        op: BinaryOp,
        left: LlvmValue,
        right: LlvmValue,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let enum_ax_ty = left.ax_ty.as_ref().cloned().ok_or_else(|| {
            "enum equality requires enum type metadata in LLVM AOT v0".to_string()
        })?;
        let layout = self.enum_layout_for_type(&enum_ax_ty)?.clone();
        ensure_same_type(&layout.ty, &left.ty)?;
        ensure_same_type(&layout.ty, &right.ty)?;

        if layout.ty == "i32" {
            let result = self.next_temp();
            let predicate = if matches!(op, BinaryOp::Equal) {
                "eq"
            } else {
                "ne"
            };
            writeln!(
                out,
                "  {result} = icmp {predicate} i32 {}, {}",
                left.repr, right.repr
            )
            .expect("writing to string cannot fail");
            return Ok(LlvmValue {
                ty: "i1".to_string(),
                repr: result,
                ax_ty: Some(Type::Bool),
            });
        }

        let left_tag = self.next_temp();
        let right_tag = self.next_temp();
        let tags_equal = self.next_temp();
        writeln!(
            out,
            "  {left_tag} = extractvalue {} {}, 0",
            left.ty, left.repr
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  {right_tag} = extractvalue {} {}, 0",
            right.ty, right.repr
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  {tags_equal} = icmp eq i32 {left_tag}, {right_tag}")
            .expect("writing to string cannot fail");

        let result_slot = self.next_temp();
        let tags_match_label = self.next_label("enum_eq_tags_match");
        let tags_differ_label = self.next_label("enum_eq_tags_differ");
        let done_label = self.next_label("enum_eq_done");
        let invalid_label = self.next_label("enum_eq_invalid_tag");
        writeln!(out, "  {result_slot} = alloca i1").expect("writing to string cannot fail");
        writeln!(
            out,
            "  br i1 {tags_equal}, label %{tags_match_label}, label %{tags_differ_label}"
        )
        .expect("writing to string cannot fail");

        let differ_value = if matches!(op, BinaryOp::Equal) {
            "0"
        } else {
            "1"
        };
        writeln!(out, "{tags_differ_label}:").expect("writing to string cannot fail");
        writeln!(out, "  store i1 {differ_value}, ptr {result_slot}")
            .expect("writing to string cannot fail");
        writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");

        writeln!(out, "{tags_match_label}:").expect("writing to string cannot fail");
        let variant_blocks = layout
            .variants
            .iter()
            .map(|variant| {
                (
                    variant.clone(),
                    self.next_label(&format!("enum_eq_test_{}", variant.name)),
                    self.next_label(&format!("enum_eq_{}", variant.name)),
                )
            })
            .collect::<Vec<_>>();
        let first_test_label = variant_blocks
            .first()
            .map(|(_, test_label, _)| test_label.as_str())
            .unwrap_or(invalid_label.as_str());
        writeln!(out, "  br label %{first_test_label}").expect("writing to string cannot fail");

        for (index, (variant, test_label, variant_label)) in variant_blocks.iter().enumerate() {
            let next_label = variant_blocks
                .get(index + 1)
                .map(|(_, label, _)| label.as_str())
                .unwrap_or(invalid_label.as_str());
            writeln!(out, "{test_label}:").expect("writing to string cannot fail");
            let is_variant = self.next_temp();
            writeln!(
                out,
                "  {is_variant} = icmp eq i32 {left_tag}, {}",
                variant.tag
            )
            .expect("writing to string cannot fail");
            writeln!(
                out,
                "  br i1 {is_variant}, label %{variant_label}, label %{next_label}"
            )
            .expect("writing to string cannot fail");
            writeln!(out, "{variant_label}:").expect("writing to string cannot fail");
            let result = if let Some(payload_ax_ty) = &variant.payload_ax_ty {
                self.emit_enum_payload_equality(
                    op,
                    &left,
                    &right,
                    payload_ax_ty,
                    &layout.name,
                    &variant.name,
                    out,
                )?
            } else if matches!(op, BinaryOp::Equal) {
                "1".to_string()
            } else {
                "0".to_string()
            };
            writeln!(out, "  store i1 {result}, ptr {result_slot}")
                .expect("writing to string cannot fail");
            writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");
        }

        writeln!(out, "{invalid_label}:").expect("writing to string cannot fail");
        writeln!(out, "  call void @exit(i32 1)").expect("writing to string cannot fail");
        writeln!(out, "  unreachable").expect("writing to string cannot fail");
        writeln!(out, "{done_label}:").expect("writing to string cannot fail");
        let loaded = self.next_temp();
        writeln!(out, "  {loaded} = load i1, ptr {result_slot}")
            .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "i1".to_string(),
            repr: loaded,
            ax_ty: Some(Type::Bool),
        })
    }

    pub(super) fn emit_enum_payload_equality(
        &mut self,
        op: BinaryOp,
        left: &LlvmValue,
        right: &LlvmValue,
        payload_ax_ty: &Type,
        enum_name: &str,
        variant_name: &str,
        out: &mut String,
    ) -> Result<String, String> {
        let payload_ty =
            llvm_type(payload_ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                format!(
                    "enum `{enum_name}.{variant_name}` payload type {} is outside LLVM AOT v0",
                    ax_type_name(payload_ax_ty)
                )
            })?;
        if !payload_equality_supported(payload_ax_ty, self.layouts, self.enum_layouts) {
            return Err(format!(
                "enum `{enum_name}.{variant_name}` payload type {} needs a native equality comparator before LLVM AOT can lower it",
                ax_type_name(payload_ax_ty)
            ));
        }

        let left_payload_ptr = self.next_temp();
        let right_payload_ptr = self.next_temp();
        let left_payload = self.next_temp();
        let right_payload = self.next_temp();
        writeln!(
            out,
            "  {left_payload_ptr} = extractvalue {} {}, 1",
            left.ty, left.repr
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  {right_payload_ptr} = extractvalue {} {}, 1",
            right.ty, right.repr
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  {left_payload} = load {payload_ty}, ptr {left_payload_ptr}"
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  {right_payload} = load {payload_ty}, ptr {right_payload_ptr}"
        )
        .expect("writing to string cannot fail");

        self.emit_value_equality(
            op,
            &LlvmValue {
                ty: payload_ty.clone(),
                repr: left_payload,
                ax_ty: Some(payload_ax_ty.clone()),
            },
            &LlvmValue {
                ty: payload_ty,
                repr: right_payload,
                ax_ty: Some(payload_ax_ty.clone()),
            },
            out,
        )
    }

    pub(super) fn emit_value_equality(
        &mut self,
        op: BinaryOp,
        left: &LlvmValue,
        right: &LlvmValue,
        out: &mut String,
    ) -> Result<String, String> {
        ensure_same_type(&left.ty, &right.ty)?;
        if left.ax_ty != right.ax_ty {
            return Err(
                "native equality in LLVM AOT v0 requires both operands to have the same AX type"
                    .to_string(),
            );
        }

        match left.ax_ty.as_ref() {
            Some(Type::Array { element, length }) => {
                return self.emit_array_equality(op, left, right, element, *length, out);
            }
            Some(Type::Slice { element }) => {
                return self.emit_slice_equality(op, left, right, element, out);
            }
            Some(Type::Struct { name }) => {
                return self.emit_struct_equality(op, left, right, name, out);
            }
            Some(Type::Enum { .. } | Type::EnumInstance { .. }) => {
                let result = self.emit_enum_equality(op, left.clone(), right.clone(), out)?;
                return Ok(result.repr);
            }
            _ => {}
        }

        let predicate = if matches!(op, BinaryOp::Equal) {
            "eq"
        } else {
            "ne"
        };
        if left.ty == "ptr" {
            let compare = self.next_temp();
            let result = self.next_temp();
            writeln!(
                out,
                "  {compare} = call i32 @strcmp(ptr {}, ptr {})",
                left.repr, right.repr
            )
            .expect("writing to string cannot fail");
            writeln!(out, "  {result} = icmp {predicate} i32 {compare}, 0")
                .expect("writing to string cannot fail");
            return Ok(result);
        }

        if left.ty == "float" {
            let predicate = if matches!(op, BinaryOp::Equal) {
                "oeq"
            } else {
                "one"
            };
            let result = self.next_temp();
            writeln!(
                out,
                "  {result} = fcmp {predicate} float {}, {}",
                left.repr, right.repr
            )
            .expect("writing to string cannot fail");
            return Ok(result);
        }

        if matches!(left.ty.as_str(), "i1" | "i32") {
            let result = self.next_temp();
            writeln!(
                out,
                "  {result} = icmp {predicate} {} {}, {}",
                left.ty, left.repr, right.repr
            )
            .expect("writing to string cannot fail");
            return Ok(result);
        }

        Err(format!(
            "type {} needs a native equality comparator before LLVM AOT can lower it",
            left.ax_ty
                .as_ref()
                .map(ax_type_name)
                .unwrap_or_else(|| left.ty.clone())
        ))
    }

    pub(super) fn emit_array_equality(
        &mut self,
        op: BinaryOp,
        left: &LlvmValue,
        right: &LlvmValue,
        element: &Type,
        length: usize,
        out: &mut String,
    ) -> Result<String, String> {
        let array_ax_ty = Type::Array {
            element: Box::new(element.clone()),
            length,
        };
        let array_ty =
            llvm_type(&array_ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                format!(
                    "array type {} is outside LLVM AOT v0",
                    ax_type_name(&array_ax_ty)
                )
            })?;
        ensure_same_type(&array_ty, &left.ty)?;
        let element_ty = llvm_type(element, self.layouts, self.enum_layouts).ok_or_else(|| {
            format!(
                "array element type {} needs a native equality comparator before LLVM AOT can lower it",
                ax_type_name(element)
            )
        })?;

        let mut current = if matches!(op, BinaryOp::Equal) {
            "1".to_string()
        } else {
            "0".to_string()
        };
        for index in 0..length {
            let left_element = self.next_temp();
            let right_element = self.next_temp();
            writeln!(
                out,
                "  {left_element} = extractvalue {array_ty} {}, {index}",
                left.repr
            )
            .expect("writing to string cannot fail");
            writeln!(
                out,
                "  {right_element} = extractvalue {array_ty} {}, {index}",
                right.repr
            )
            .expect("writing to string cannot fail");
            let element_result = self.emit_value_equality(
                op,
                &LlvmValue {
                    ty: element_ty.clone(),
                    repr: left_element,
                    ax_ty: Some(element.clone()),
                },
                &LlvmValue {
                    ty: element_ty.clone(),
                    repr: right_element,
                    ax_ty: Some(element.clone()),
                },
                out,
            )?;
            current = self.emit_equality_accumulator(op, &current, &element_result, out);
        }
        Ok(current)
    }

    pub(super) fn emit_slice_equality(
        &mut self,
        op: BinaryOp,
        left: &LlvmValue,
        right: &LlvmValue,
        element: &Type,
        out: &mut String,
    ) -> Result<String, String> {
        ensure_same_type(&slice_llvm_type(), &left.ty)?;
        ensure_same_type(&slice_llvm_type(), &right.ty)?;
        let element_ty = llvm_type(element, self.layouts, self.enum_layouts).ok_or_else(|| {
            format!(
                "slice element type {} needs a native equality comparator before LLVM AOT can lower it",
                ax_type_name(element)
            )
        })?;

        let left_data = self.next_temp();
        let left_len = self.next_temp();
        let right_data = self.next_temp();
        let right_len = self.next_temp();
        writeln!(
            out,
            "  {left_data} = extractvalue {} {}, 0",
            left.ty, left.repr
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  {left_len} = extractvalue {} {}, 1",
            left.ty, left.repr
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  {right_data} = extractvalue {} {}, 0",
            right.ty, right.repr
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  {right_len} = extractvalue {} {}, 1",
            right.ty, right.repr
        )
        .expect("writing to string cannot fail");

        let lengths_equal = self.next_temp();
        writeln!(
            out,
            "  {lengths_equal} = icmp eq i32 {left_len}, {right_len}"
        )
        .expect("writing to string cannot fail");

        let result_slot = self.next_temp();
        let index_slot = self.next_temp();
        let length_match_label = self.next_label("slice_eq_length_match");
        let length_differ_label = self.next_label("slice_eq_length_differ");
        let loop_label = self.next_label("slice_eq_loop");
        let body_label = self.next_label("slice_eq_body");
        let done_label = self.next_label("slice_eq_done");
        writeln!(out, "  {result_slot} = alloca i1").expect("writing to string cannot fail");
        writeln!(out, "  {index_slot} = alloca i32").expect("writing to string cannot fail");
        writeln!(
            out,
            "  br i1 {lengths_equal}, label %{length_match_label}, label %{length_differ_label}"
        )
        .expect("writing to string cannot fail");

        let length_differ_value = if matches!(op, BinaryOp::Equal) {
            "0"
        } else {
            "1"
        };
        writeln!(out, "{length_differ_label}:").expect("writing to string cannot fail");
        writeln!(out, "  store i1 {length_differ_value}, ptr {result_slot}")
            .expect("writing to string cannot fail");
        writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");

        let initial_value = if matches!(op, BinaryOp::Equal) {
            "1"
        } else {
            "0"
        };
        writeln!(out, "{length_match_label}:").expect("writing to string cannot fail");
        writeln!(out, "  store i1 {initial_value}, ptr {result_slot}")
            .expect("writing to string cannot fail");
        writeln!(out, "  store i32 0, ptr {index_slot}").expect("writing to string cannot fail");
        writeln!(out, "  br label %{loop_label}").expect("writing to string cannot fail");

        writeln!(out, "{loop_label}:").expect("writing to string cannot fail");
        let index = self.next_temp();
        writeln!(out, "  {index} = load i32, ptr {index_slot}")
            .expect("writing to string cannot fail");
        let has_item = self.next_temp();
        writeln!(out, "  {has_item} = icmp slt i32 {index}, {left_len}")
            .expect("writing to string cannot fail");
        writeln!(
            out,
            "  br i1 {has_item}, label %{body_label}, label %{done_label}"
        )
        .expect("writing to string cannot fail");

        writeln!(out, "{body_label}:").expect("writing to string cannot fail");
        let left_ptr = self.next_temp();
        let right_ptr = self.next_temp();
        let left_element = self.next_temp();
        let right_element = self.next_temp();
        writeln!(
            out,
            "  {left_ptr} = getelementptr {element_ty}, ptr {left_data}, i32 {index}"
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  {right_ptr} = getelementptr {element_ty}, ptr {right_data}, i32 {index}"
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  {left_element} = load {element_ty}, ptr {left_ptr}")
            .expect("writing to string cannot fail");
        writeln!(
            out,
            "  {right_element} = load {element_ty}, ptr {right_ptr}"
        )
        .expect("writing to string cannot fail");
        let element_result = self.emit_value_equality(
            op,
            &LlvmValue {
                ty: element_ty.clone(),
                repr: left_element,
                ax_ty: Some(element.clone()),
            },
            &LlvmValue {
                ty: element_ty,
                repr: right_element,
                ax_ty: Some(element.clone()),
            },
            out,
        )?;
        let current = self.next_temp();
        writeln!(out, "  {current} = load i1, ptr {result_slot}")
            .expect("writing to string cannot fail");
        let combined = self.emit_equality_accumulator(op, &current, &element_result, out);
        writeln!(out, "  store i1 {combined}, ptr {result_slot}")
            .expect("writing to string cannot fail");
        let next_index = self.next_temp();
        writeln!(out, "  {next_index} = add i32 {index}, 1")
            .expect("writing to string cannot fail");
        writeln!(out, "  store i32 {next_index}, ptr {index_slot}")
            .expect("writing to string cannot fail");
        writeln!(out, "  br label %{loop_label}").expect("writing to string cannot fail");

        writeln!(out, "{done_label}:").expect("writing to string cannot fail");
        let result = self.next_temp();
        writeln!(out, "  {result} = load i1, ptr {result_slot}")
            .expect("writing to string cannot fail");
        Ok(result)
    }

    pub(super) fn emit_struct_equality(
        &mut self,
        op: BinaryOp,
        left: &LlvmValue,
        right: &LlvmValue,
        name: &str,
        out: &mut String,
    ) -> Result<String, String> {
        let layout =
            self.layouts.get(name).cloned().ok_or_else(|| {
                format!("struct `{name}` is outside the LLVM AOT v0 equality subset")
            })?;
        ensure_same_type(&layout.ty, &left.ty)?;

        let mut current = if matches!(op, BinaryOp::Equal) {
            "1".to_string()
        } else {
            "0".to_string()
        };
        for field in &layout.fields {
            let left_field = self.next_temp();
            let right_field = self.next_temp();
            writeln!(
                out,
                "  {left_field} = extractvalue {} {}, {}",
                layout.ty, left.repr, field.index
            )
            .expect("writing to string cannot fail");
            writeln!(
                out,
                "  {right_field} = extractvalue {} {}, {}",
                layout.ty, right.repr, field.index
            )
            .expect("writing to string cannot fail");
            let field_result = self.emit_value_equality(
                op,
                &LlvmValue {
                    ty: field.ty.clone(),
                    repr: left_field,
                    ax_ty: Some(field.ax_ty.clone()),
                },
                &LlvmValue {
                    ty: field.ty.clone(),
                    repr: right_field,
                    ax_ty: Some(field.ax_ty.clone()),
                },
                out,
            )?;
            current = self.emit_equality_accumulator(op, &current, &field_result, out);
        }
        Ok(current)
    }

    pub(super) fn emit_equality_accumulator(
        &mut self,
        op: BinaryOp,
        current: &str,
        next: &str,
        out: &mut String,
    ) -> String {
        let combined = self.next_temp();
        let instruction = if matches!(op, BinaryOp::Equal) {
            "and"
        } else {
            "or"
        };
        writeln!(out, "  {combined} = {instruction} i1 {current}, {next}")
            .expect("writing to string cannot fail");
        combined
    }
}
