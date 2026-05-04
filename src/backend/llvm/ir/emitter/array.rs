use std::fmt::Write;

use super::*;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_array_literal(
        &mut self,
        elements: &[Expr],
        expected_ax_ty: Option<&Type>,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if elements.is_empty() {
            let Some(expected_ax_ty @ Type::Array { element, length: 0 }) = expected_ax_ty else {
                return Err(
                    "empty array literals need explicit native array type propagation before LLVM AOT can lower them"
                        .into(),
                );
            };
            let array_ty =
                llvm_type(expected_ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                    format!(
                        "array type {} is outside LLVM AOT v0",
                        ax_type_name(expected_ax_ty)
                    )
                })?;
            llvm_type(element, self.layouts, self.enum_layouts).ok_or_else(|| {
                format!(
                    "array element type {} is outside LLVM AOT v0",
                    ax_type_name(element)
                )
            })?;
            return Ok(LlvmValue {
                ty: array_ty,
                repr: "zeroinitializer".to_string(),
                ax_ty: Some(expected_ax_ty.clone()),
            });
        }

        let mut rendered_elements = Vec::new();
        let mut element_ty: Option<String> = None;
        let mut element_ax_ty: Option<Type> = None;
        for element in elements {
            let value = self.emit_expr(element, out)?;
            if let Some(expected_ty) = &element_ty {
                ensure_same_type(expected_ty, &value.ty)?;
            } else {
                element_ty = Some(value.ty.clone());
            }
            if let Some(expected_ax_ty) = &element_ax_ty {
                if value.ax_ty.as_ref() != Some(expected_ax_ty) {
                    return Err("array literal elements must have one LLVM AOT type".to_string());
                }
            } else {
                element_ax_ty = value.ax_ty.clone();
            }
            rendered_elements.push((value.ty, value.repr));
        }

        let element_ty = element_ty.expect("non-empty array should have an element type");
        let element_ax_ty = element_ax_ty.ok_or_else(|| {
            "array literal element type is not representable in LLVM AOT v0".to_string()
        })?;
        let length = elements.len();
        let array_ty = format!("[{length} x {element_ty}]");
        let repr = self.emit_aggregate_value(&array_ty, &rendered_elements, out);
        Ok(LlvmValue {
            ty: array_ty,
            repr,
            ax_ty: Some(Type::Array {
                element: Box::new(element_ax_ty),
                length,
            }),
        })
    }

    pub(super) fn emit_index(
        &mut self,
        base: &Expr,
        index: &Expr,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let (_, base_ax_ty) = self.infer_expr_value_type(base)?;
        if matches!(base_ax_ty, Type::Slice { .. }) {
            return self.emit_slice_index(base, index, out);
        }

        let (array_ptr, array_ty, element_ty, length, element_ax_ty) =
            self.emit_array_base_ptr(base, out)?;
        let index = self.emit_expr(index, out)?;
        ensure_same_type("i32", &index.ty)?;
        self.emit_fixed_bounds_check(&index.repr, length, out);
        let element_ptr = self.next_temp();
        writeln!(
            out,
            "  {element_ptr} = getelementptr {array_ty}, ptr {array_ptr}, i32 0, i32 {}",
            index.repr
        )
        .expect("writing to string cannot fail");
        let loaded = self.next_temp();
        writeln!(out, "  {loaded} = load {element_ty}, ptr {element_ptr}")
            .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: element_ty,
            repr: loaded,
            ax_ty: Some(element_ax_ty),
        })
    }

    pub(super) fn emit_slice_from_expr(
        &mut self,
        expr: &Expr,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if let ExprKind::Local { local, .. } = &expr.kind {
            let slot = self.local_slot(*local)?.clone();
            match &slot.ax_ty {
                Type::Array { .. } => {
                    return self.emit_slice_from_array_ptr(&slot.ptr, &slot.ax_ty, out);
                }
                Type::Slice { .. } => return self.emit_expr(expr, out),
                _ => {}
            }
        }

        let value = self.emit_expr(expr, out)?;
        match value.ax_ty.as_ref() {
            Some(Type::Array { .. }) => self.emit_slice_from_array_value(value, out),
            Some(Type::Slice { .. }) => Ok(value),
            Some(other) => Err(format!(
                "cannot coerce {} to a read-only slice in LLVM AOT v0",
                ax_type_name(other)
            )),
            None => Err("slice source value is not representable in LLVM AOT v0".to_string()),
        }
    }

    pub(super) fn emit_slice_from_array_value(
        &mut self,
        value: LlvmValue,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let Some(ax_ty) = value.ax_ty.clone() else {
            return Err(
                "array-to-slice coercion needs an AX array type in LLVM AOT v0".to_string(),
            );
        };
        let (array_ty, _, _, _) = array_type_parts(&ax_ty, self.layouts, self.enum_layouts)?;
        ensure_same_type(&array_ty, &value.ty)?;
        let temp_array = self.next_temp();
        writeln!(out, "  {temp_array} = alloca {array_ty}").expect("writing to string cannot fail");
        writeln!(out, "  store {array_ty} {}, ptr {temp_array}", value.repr)
            .expect("writing to string cannot fail");
        self.emit_slice_from_array_ptr(&temp_array, &ax_ty, out)
    }

    pub(super) fn emit_slice_from_array_ptr(
        &mut self,
        array_ptr: &str,
        ax_ty: &Type,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let (array_ty, element_ty, length, element_ax_ty) =
            array_type_parts(ax_ty, self.layouts, self.enum_layouts)?;
        let data_ptr = self.next_temp();
        writeln!(
            out,
            "  {data_ptr} = getelementptr {array_ty}, ptr {array_ptr}, i32 0, i32 0"
        )
        .expect("writing to string cannot fail");
        let with_ptr = self.next_temp();
        writeln!(
            out,
            "  {with_ptr} = insertvalue {} undef, ptr {data_ptr}, 0",
            slice_llvm_type()
        )
        .expect("writing to string cannot fail");
        let repr = self.next_temp();
        writeln!(
            out,
            "  {repr} = insertvalue {} {with_ptr}, i32 {length}, 1",
            slice_llvm_type()
        )
        .expect("writing to string cannot fail");
        let slice_ax_ty = Type::Slice {
            element: Box::new(element_ax_ty),
        };
        let expected_slice_ty = llvm_type(&slice_ax_ty, self.layouts, self.enum_layouts)
            .ok_or_else(|| format!("slice element type {element_ty} is outside LLVM AOT v0"))?;
        Ok(LlvmValue {
            ty: expected_slice_ty,
            repr,
            ax_ty: Some(slice_ax_ty),
        })
    }

    pub(super) fn emit_slice_index(
        &mut self,
        base: &Expr,
        index: &Expr,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let slice = self.emit_expr(base, out)?;
        let Some(Type::Slice { element }) = slice.ax_ty.clone() else {
            return Err("slice index base is not a slice in LLVM AOT v0".to_string());
        };
        ensure_same_type(&slice_llvm_type(), &slice.ty)?;
        let element_ty = llvm_type(&element, self.layouts, self.enum_layouts).ok_or_else(|| {
            format!(
                "slice element type {} is outside LLVM AOT v0",
                ax_type_name(&element)
            )
        })?;
        let index = self.emit_expr(index, out)?;
        ensure_same_type("i32", &index.ty)?;
        let data_ptr = self.next_temp();
        writeln!(
            out,
            "  {data_ptr} = extractvalue {} {}, 0",
            slice.ty, slice.repr
        )
        .expect("writing to string cannot fail");
        let len = self.next_temp();
        writeln!(out, "  {len} = extractvalue {} {}, 1", slice.ty, slice.repr)
            .expect("writing to string cannot fail");
        self.emit_dynamic_bounds_check(&index.repr, &len, out);
        let element_ptr = self.next_temp();
        writeln!(
            out,
            "  {element_ptr} = getelementptr {element_ty}, ptr {data_ptr}, i32 {}",
            index.repr
        )
        .expect("writing to string cannot fail");
        let loaded = self.next_temp();
        writeln!(out, "  {loaded} = load {element_ty}, ptr {element_ptr}")
            .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: element_ty,
            repr: loaded,
            ax_ty: Some(*element),
        })
    }

    pub(super) fn emit_slice_range(
        &mut self,
        base: &Expr,
        start: &Expr,
        end: &Expr,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let slice = self.emit_slice_from_expr(base, out)?;
        let Some(Type::Slice { element }) = slice.ax_ty.clone() else {
            return Err("slice expression base is not slice-compatible in LLVM AOT v0".to_string());
        };
        ensure_same_type(&slice_llvm_type(), &slice.ty)?;
        let element_ax_ty = *element;
        let element_ty =
            llvm_type(&element_ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                format!(
                    "slice element type {} is outside LLVM AOT v0",
                    ax_type_name(&element_ax_ty)
                )
            })?;

        let start = self.emit_expr(start, out)?;
        ensure_same_type("i32", &start.ty)?;
        let end = self.emit_expr(end, out)?;
        ensure_same_type("i32", &end.ty)?;

        let base_ptr = self.next_temp();
        writeln!(
            out,
            "  {base_ptr} = extractvalue {} {}, 0",
            slice.ty, slice.repr
        )
        .expect("writing to string cannot fail");
        let base_len = self.next_temp();
        writeln!(
            out,
            "  {base_len} = extractvalue {} {}, 1",
            slice.ty, slice.repr
        )
        .expect("writing to string cannot fail");

        self.emit_slice_bound_check(&start.repr, &base_len, out);
        self.emit_slice_bound_check(&end.repr, &base_len, out);
        self.emit_slice_order_check(&start.repr, &end.repr, out);

        let data_ptr = self.next_temp();
        writeln!(
            out,
            "  {data_ptr} = getelementptr {element_ty}, ptr {base_ptr}, i32 {}",
            start.repr
        )
        .expect("writing to string cannot fail");
        let len = self.next_temp();
        writeln!(out, "  {len} = sub i32 {}, {}", end.repr, start.repr)
            .expect("writing to string cannot fail");
        let element_size = llvm_alloc_size(&element_ax_ty, self.layouts, self.enum_layouts)
            .ok_or_else(|| {
                format!(
                    "slice element type {} needs a native size contract before LLVM AOT can copy it",
                    ax_type_name(&element_ax_ty)
                )
            })?;
        let len_i64 = self.next_temp();
        writeln!(out, "  {len_i64} = sext i32 {len} to i64")
            .expect("writing to string cannot fail");
        let byte_len = self.next_temp();
        writeln!(out, "  {byte_len} = mul i64 {len_i64}, {element_size}")
            .expect("writing to string cannot fail");
        let copy_ptr = self.next_temp();
        writeln!(out, "  {copy_ptr} = call ptr @malloc(i64 {byte_len})")
            .expect("writing to string cannot fail");
        let _copy = self.next_temp();
        writeln!(
            out,
            "  {_copy} = call ptr @memcpy(ptr {copy_ptr}, ptr {data_ptr}, i64 {byte_len})"
        )
        .expect("writing to string cannot fail");
        let with_ptr = self.next_temp();
        writeln!(
            out,
            "  {with_ptr} = insertvalue {} undef, ptr {copy_ptr}, 0",
            slice_llvm_type()
        )
        .expect("writing to string cannot fail");
        let repr = self.next_temp();
        writeln!(
            out,
            "  {repr} = insertvalue {} {with_ptr}, i32 {len}, 1",
            slice_llvm_type()
        )
        .expect("writing to string cannot fail");

        Ok(LlvmValue {
            ty: slice_llvm_type(),
            repr,
            ax_ty: Some(Type::Slice {
                element: Box::new(element_ax_ty),
            }),
        })
    }

    pub(super) fn emit_array_base_ptr(
        &mut self,
        base: &Expr,
        out: &mut String,
    ) -> Result<(String, String, String, usize, Type), String> {
        if let ExprKind::Local { local, .. } = &base.kind {
            let slot = self.local_slot(*local)?.clone();
            let (array_ty, element_ty, length, element_ax_ty) =
                array_type_parts(&slot.ax_ty, self.layouts, self.enum_layouts)?;
            ensure_same_type(&array_ty, &slot.ty)?;
            return Ok((slot.ptr, array_ty, element_ty, length, element_ax_ty));
        }

        let value = self.emit_expr(base, out)?;
        let Some(ax_ty) = value.ax_ty.clone() else {
            return Err("array index base is not a fixed-size array in LLVM AOT v0".to_string());
        };
        let (array_ty, element_ty, length, element_ax_ty) =
            array_type_parts(&ax_ty, self.layouts, self.enum_layouts)?;
        ensure_same_type(&array_ty, &value.ty)?;
        let temp_array = self.next_temp();
        writeln!(out, "  {temp_array} = alloca {array_ty}").expect("writing to string cannot fail");
        writeln!(out, "  store {array_ty} {}, ptr {temp_array}", value.repr)
            .expect("writing to string cannot fail");
        Ok((temp_array, array_ty, element_ty, length, element_ax_ty))
    }

    pub(super) fn emit_fixed_bounds_check(&mut self, index: &str, length: usize, out: &mut String) {
        self.emit_dynamic_bounds_check(index, &length.to_string(), out);
    }

    pub(super) fn emit_dynamic_bounds_check(
        &mut self,
        index: &str,
        length: &str,
        out: &mut String,
    ) {
        let negative = self.next_temp();
        let too_high = self.next_temp();
        let out_of_bounds = self.next_temp();
        let fail_label = self.next_label("array_oob");
        let ok_label = self.next_label("array_ok");
        writeln!(out, "  {negative} = icmp slt i32 {index}, 0")
            .expect("writing to string cannot fail");
        writeln!(out, "  {too_high} = icmp sge i32 {index}, {length}")
            .expect("writing to string cannot fail");
        writeln!(out, "  {out_of_bounds} = or i1 {negative}, {too_high}")
            .expect("writing to string cannot fail");
        writeln!(
            out,
            "  br i1 {out_of_bounds}, label %{fail_label}, label %{ok_label}"
        )
        .expect("writing to string cannot fail");
        writeln!(out, "{fail_label}:").expect("writing to string cannot fail");
        writeln!(out, "  call void @ax_runtime_error(ptr @.ax_rt_index_oob)")
            .expect("writing to string cannot fail");
        writeln!(out, "  unreachable").expect("writing to string cannot fail");
        writeln!(out, "{ok_label}:").expect("writing to string cannot fail");
    }

    pub(super) fn emit_slice_bound_check(&mut self, bound: &str, length: &str, out: &mut String) {
        let negative = self.next_temp();
        let too_high = self.next_temp();
        let out_of_bounds = self.next_temp();
        let fail_label = self.next_label("slice_bound_oob");
        let ok_label = self.next_label("slice_bound_ok");
        writeln!(out, "  {negative} = icmp slt i32 {bound}, 0")
            .expect("writing to string cannot fail");
        writeln!(out, "  {too_high} = icmp sgt i32 {bound}, {length}")
            .expect("writing to string cannot fail");
        writeln!(out, "  {out_of_bounds} = or i1 {negative}, {too_high}")
            .expect("writing to string cannot fail");
        writeln!(
            out,
            "  br i1 {out_of_bounds}, label %{fail_label}, label %{ok_label}"
        )
        .expect("writing to string cannot fail");
        writeln!(out, "{fail_label}:").expect("writing to string cannot fail");
        writeln!(
            out,
            "  call void @ax_runtime_error(ptr @.ax_rt_slice_bound_oob)"
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  unreachable").expect("writing to string cannot fail");
        writeln!(out, "{ok_label}:").expect("writing to string cannot fail");
    }

    pub(super) fn emit_slice_order_check(&mut self, start: &str, end: &str, out: &mut String) {
        let invalid = self.next_temp();
        let fail_label = self.next_label("slice_order_invalid");
        let ok_label = self.next_label("slice_order_ok");
        writeln!(out, "  {invalid} = icmp sgt i32 {start}, {end}")
            .expect("writing to string cannot fail");
        writeln!(
            out,
            "  br i1 {invalid}, label %{fail_label}, label %{ok_label}"
        )
        .expect("writing to string cannot fail");
        writeln!(out, "{fail_label}:").expect("writing to string cannot fail");
        writeln!(
            out,
            "  call void @ax_runtime_error(ptr @.ax_rt_slice_order_invalid)"
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  unreachable").expect("writing to string cannot fail");
        writeln!(out, "{ok_label}:").expect("writing to string cannot fail");
    }
}
