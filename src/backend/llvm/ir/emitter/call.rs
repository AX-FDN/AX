use std::fmt::Write;

use super::*;

impl<'a> FunctionEmitter<'a> {
    pub(super) fn emit_call(
        &mut self,
        function: &str,
        arguments: &[Expr],
        expected_return_ax_ty: Option<&Type>,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if function == "println" {
            return self.emit_println(arguments, out);
        }
        if function == "argv_len" {
            return self.emit_argv_len(function, arguments, out);
        }
        if function == "argv_get" {
            return self.emit_argv_get(arguments, out);
        }
        if function == "env_has" {
            return self.emit_env_has(arguments, out);
        }
        if function == "env_get" {
            return self.emit_env_get(arguments, out);
        }
        if function == "std.env.try_get" {
            return self.emit_env_try_get(arguments, expected_return_ax_ty, out);
        }
        if function == "std.fs.try_read_to_string" {
            return self.emit_fs_try_read_to_string(arguments, expected_return_ax_ty, out);
        }
        if function == "std.fs.try_read_dir" {
            return self.emit_fs_try_read_dir(arguments, expected_return_ax_ty, out);
        }
        if function == "std.fs.try_file_size" {
            return self.emit_fs_try_file_size(arguments, expected_return_ax_ty, out);
        }
        if function == "std.process.try_run" {
            return self.emit_process_try_run(arguments, expected_return_ax_ty, out);
        }
        if function == "std.process.try_status" {
            return self.emit_process_try_status(arguments, expected_return_ax_ty, out);
        }
        if function == "process_cwd" {
            return self.emit_process_cwd(arguments, out);
        }
        if function == "process_run" {
            return self.emit_process_run(arguments, out);
        }
        if function == "process_capture" {
            return self.emit_process_capture(arguments, out);
        }
        if function == "process_run_in" {
            return self.emit_process_run_in(arguments, out);
        }
        if function == "process_capture_in" {
            return self.emit_process_capture_in(arguments, out);
        }
        if matches!(function, "fs_exists" | "fs_is_file" | "fs_is_dir") {
            return self.emit_fs_predicate(function, arguments, out);
        }
        if function == "fs_file_size" {
            return self.emit_fs_file_size(arguments, out);
        }
        if function == "fs_read_to_string" {
            return self.emit_fs_read_to_string(arguments, out);
        }
        if function == "fs_read_dir" {
            return self.emit_fs_read_dir(arguments, out);
        }
        if function == "fs_write_string" {
            return self.emit_fs_write_string(arguments, out);
        }
        if function == "fs_remove_file" {
            return self.emit_fs_remove_file(arguments, out);
        }
        if function == "fs_remove_dir_all" {
            return self.emit_fs_remove_dir_all(arguments, out);
        }
        if function == "fs_rename" {
            return self.emit_fs_rename(arguments, out);
        }
        if function == "fs_copy_file" {
            return self.emit_fs_copy_file(arguments, out);
        }
        if function == "fs_create_dir_all" {
            return self.emit_fs_create_dir_all(arguments, out);
        }
        if function == "string_len" || function == "len" {
            return self.emit_string_len(function, arguments, out);
        }
        if function == "std.bytes.empty" {
            return self.emit_bytes_empty(arguments, out);
        }
        if function == "std.bytes.from_string" {
            return self.emit_bytes_from_string(arguments, out);
        }
        if function == "std.bytes.to_string_lossy" {
            return self.emit_bytes_to_string_lossy(arguments, out);
        }
        if function == "std.bytes.to_hex" {
            return self.emit_bytes_to_hex(arguments, out);
        }
        if function == "std.bytes.push" {
            return self.emit_bytes_push(arguments, out);
        }
        if function == "std.bytes.get" {
            return self.emit_bytes_get(arguments, out);
        }
        if function == "std.bytes.length" || function == "std.bytes.len" {
            return self.emit_string_len("len", arguments, out);
        }
        if function == "bytes_empty" {
            return self.emit_bytes_empty(arguments, out);
        }
        if function == "bytes_from_string" {
            return self.emit_bytes_from_string(arguments, out);
        }
        if function == "bytes_to_string_lossy" {
            return self.emit_bytes_to_string_lossy(arguments, out);
        }
        if function == "bytes_to_hex" {
            return self.emit_bytes_to_hex(arguments, out);
        }
        if function == "bytes_push" {
            return self.emit_bytes_push(arguments, out);
        }
        if function == "bytes_get" {
            return self.emit_bytes_get(arguments, out);
        }
        if matches!(
            function,
            "string_contains" | "string_starts_with" | "string_ends_with"
        ) {
            return self.emit_string_predicate(function, arguments, out);
        }
        if function == "string_replace" {
            return self.emit_string_replace(arguments, out);
        }
        if function == "string_split_lines" {
            return self.emit_string_split_lines(arguments, out);
        }
        if function == "string_trim" {
            return self.emit_string_trim(arguments, out);
        }
        if function == "string_list_new" {
            return self.emit_string_list_new(arguments, out);
        }
        if function == "string_list_push" {
            return self.emit_string_list_push(arguments, out);
        }
        if function == "string_list_get" {
            return self.emit_string_list_get(arguments, out);
        }
        if function == "string_list_join" {
            return self.emit_string_list_join(arguments, out);
        }
        if function == "to_string" {
            return self.emit_to_string(arguments, out);
        }
        if function == "path_join" {
            return self.emit_path_join(arguments, out);
        }
        if matches!(
            function,
            "path_parent" | "path_resolve" | "path_file_name" | "path_stem" | "path_extension"
        ) {
            return self.emit_path_string_function(function, arguments, out);
        }
        if function == "path_is_absolute" {
            return self.emit_path_is_absolute(arguments, out);
        }

        let resolved = self.resolve_call_signature(function, arguments, expected_return_ax_ty)?;
        let signature = resolved.signature.clone();
        if signature.params.len() != arguments.len() {
            return Err(format!(
                "call to `{}` has {} argument(s), but LLVM AOT expected {}",
                resolved.name,
                arguments.len(),
                signature.params.len()
            ));
        }

        let mut rendered_args = Vec::new();
        for ((argument, expected_ty), expected_ax_ty) in arguments
            .iter()
            .zip(&signature.params)
            .zip(&signature.param_ax_types)
        {
            let value = match expected_ax_ty {
                Type::Slice { .. } => self.emit_slice_from_expr(argument, out)?,
                Type::Array { .. } => match &argument.kind {
                    ExprKind::ArrayLiteral { elements } => {
                        self.emit_array_literal(elements, Some(expected_ax_ty), out)?
                    }
                    _ => self.emit_expr(argument, out)?,
                },
                _ => self.emit_expr(argument, out)?,
            };
            ensure_same_type(expected_ty, &value.ty)?;
            rendered_args.push(format!("{} {}", value.ty, value.repr));
        }

        let temp = self.next_temp();
        writeln!(
            out,
            "  {temp} = call {} @{}({})",
            signature.return_type,
            signature.symbol,
            rendered_args.join(", ")
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: signature.return_type.clone(),
            repr: temp,
            ax_ty: Some(signature.return_ax_type.clone()),
        })
    }

    pub(super) fn resolve_call_signature(
        &self,
        function: &str,
        arguments: &[Expr],
        expected_return_ax_ty: Option<&Type>,
    ) -> Result<ResolvedCallSignature<'_>, String> {
        if let Some(method) = function.strip_prefix("<method>.") {
            let receiver = arguments.first().ok_or_else(|| {
                format!("method call `{method}` is missing its receiver in LLVM AOT v0")
            })?;
            let (_, receiver_ax_ty) = self.infer_expr_value_type(receiver)?;
            let method_function = method_function_name(method, &receiver_ax_ty).ok_or_else(|| {
                format!(
                    "method `{method}` receiver type {} needs a struct or enum before LLVM AOT can lower it",
                    ax_type_name(&receiver_ax_ty)
                )
            })?;
            if let Some(signature) = self.signatures.get(&method_function) {
                return Ok(ResolvedCallSignature {
                    name: method_function,
                    signature,
                });
            }

            let mut argument_ax_types = Vec::new();
            for argument in arguments {
                let (_, argument_ax_ty) = self.infer_expr_value_type(argument)?;
                argument_ax_types.push(argument_ax_ty);
            }
            let prefix = format!("{method_function}<");
            let mut candidates = self.signatures.iter().filter(|(name, signature)| {
                name.starts_with(&prefix)
                    && signature.param_ax_types == argument_ax_types
                    && expected_return_ax_ty
                        .map(|expected| &signature.return_ax_type == expected)
                        .unwrap_or(true)
            });
            let Some((specialized_name, signature)) = candidates.next() else {
                return Err(format!(
                    "method call `{method_function}` is outside LLVM AOT v0; only same-file impl methods with inferable native type arguments are currently lowered"
                ));
            };
            if candidates.next().is_some() {
                return Err(format!(
                    "method call `{method_function}` matched multiple LLVM AOT specializations"
                ));
            }
            return Ok(ResolvedCallSignature {
                name: specialized_name.clone(),
                signature,
            });
        }

        if let Some(signature) = self.signatures.get(function) {
            return Ok(ResolvedCallSignature {
                name: function.to_string(),
                signature,
            });
        }

        let mut argument_ax_types = Vec::new();
        for argument in arguments {
            let (_, argument_ax_ty) = self.infer_expr_value_type(argument)?;
            argument_ax_types.push(argument_ax_ty);
        }
        let prefix = format!("{function}<");
        let mut candidates = self.signatures.iter().filter(|(name, signature)| {
            name.starts_with(&prefix)
                && signature.param_ax_types == argument_ax_types
                && expected_return_ax_ty
                    .map(|expected| &signature.return_ax_type == expected)
                    .unwrap_or(true)
        });
        let Some((specialized_name, signature)) = candidates.next() else {
            return Err(format!(
                "call to `{function}` is outside LLVM AOT v0; only same-file functions with inferable native type arguments are currently lowered"
            ));
        };
        if candidates.next().is_some() {
            return Err(format!(
                "call to `{function}` matched multiple LLVM AOT specializations"
            ));
        }
        Ok(ResolvedCallSignature {
            name: specialized_name.clone(),
            signature,
        })
    }

    fn emit_string_len(
        &mut self,
        function: &str,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `{function}` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let value = self.emit_expr(&arguments[0], out)?;
        if function == "len" {
            if let Some(Type::Array { length, .. }) = value.ax_ty.as_ref() {
                return Ok(LlvmValue {
                    ty: "i32".to_string(),
                    repr: length.to_string(),
                    ax_ty: Some(Type::I32),
                });
            }
            if matches!(value.ax_ty.as_ref(), Some(Type::Slice { .. })) {
                ensure_same_type(&slice_llvm_type(), &value.ty)?;
                let len = self.next_temp();
                writeln!(out, "  {len} = extractvalue {} {}, 1", value.ty, value.repr)
                    .expect("writing to string cannot fail");
                return Ok(LlvmValue {
                    ty: "i32".to_string(),
                    repr: len,
                    ax_ty: Some(Type::I32),
                });
            }
            if matches!(value.ax_ty.as_ref(), Some(Type::Bytes)) {
                ensure_bytes_argument(function, "data", &value)?;
                let temp = self.next_temp();
                writeln!(out, "  {temp} = call i32 @ax_bytes_len(ptr {})", value.repr)
                    .expect("writing to string cannot fail");
                return Ok(LlvmValue {
                    ty: "i32".to_string(),
                    repr: temp,
                    ax_ty: Some(Type::I32),
                });
            }
            if matches!(value.ax_ty.as_ref(), Some(Type::StringList)) {
                ensure_string_list_argument(function, "items", &value)?;
                let temp = self.next_temp();
                writeln!(
                    out,
                    "  {temp} = call i32 @ax_string_list_len(ptr {})",
                    value.repr
                )
                .expect("writing to string cannot fail");
                return Ok(LlvmValue {
                    ty: "i32".to_string(),
                    repr: temp,
                    ax_ty: Some(Type::I32),
                });
            }
        }
        ensure_string_argument(function, "text", &value)?;
        let temp = self.next_temp();
        writeln!(
            out,
            "  {temp} = call i32 @ax_string_len(ptr {})",
            value.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "i32".to_string(),
            repr: temp,
            ax_ty: Some(Type::I32),
        })
    }

    fn emit_string_list_new(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if !arguments.is_empty() {
            return Err(format!(
                "call to `string_list_new` has {} argument(s), but LLVM AOT expected 0",
                arguments.len()
            ));
        }
        let list = self.next_temp();
        writeln!(out, "  {list} = call ptr @ax_string_list_new()")
            .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: list,
            ax_ty: Some(Type::StringList),
        })
    }

    fn emit_string_list_push(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 2 {
            return Err(format!(
                "call to `string_list_push` has {} argument(s), but LLVM AOT expected 2",
                arguments.len()
            ));
        }
        let items = self.emit_expr(&arguments[0], out)?;
        ensure_string_list_argument("string_list_push", "items", &items)?;
        let value = self.emit_expr(&arguments[1], out)?;
        ensure_string_argument("string_list_push", "value", &value)?;
        let pushed = self.next_temp();
        writeln!(
            out,
            "  {pushed} = call ptr @ax_string_list_push(ptr {}, ptr {})",
            items.repr, value.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: pushed,
            ax_ty: Some(Type::StringList),
        })
    }

    fn emit_string_list_get(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 2 {
            return Err(format!(
                "call to `string_list_get` has {} argument(s), but LLVM AOT expected 2",
                arguments.len()
            ));
        }
        let items = self.emit_expr(&arguments[0], out)?;
        ensure_string_list_argument("string_list_get", "items", &items)?;
        let index = self.emit_expr(&arguments[1], out)?;
        ensure_same_type("i32", &index.ty)?;
        let value = self.next_temp();
        writeln!(
            out,
            "  {value} = call ptr @ax_string_list_get(ptr {}, i32 {})",
            items.repr, index.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: value,
            ax_ty: Some(Type::String),
        })
    }

    fn emit_string_list_join(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 2 {
            return Err(format!(
                "call to `string_list_join` has {} argument(s), but LLVM AOT expected 2",
                arguments.len()
            ));
        }
        let items = self.emit_expr(&arguments[0], out)?;
        ensure_string_list_argument("string_list_join", "items", &items)?;
        let separator = self.emit_expr(&arguments[1], out)?;
        ensure_string_argument("string_list_join", "separator", &separator)?;
        let joined = self.next_temp();
        writeln!(
            out,
            "  {joined} = call ptr @ax_string_list_join(ptr {}, ptr {})",
            items.repr, separator.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: joined,
            ax_ty: Some(Type::String),
        })
    }

    fn emit_bytes_empty(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if !arguments.is_empty() {
            return Err(format!(
                "call to `bytes_empty` has {} argument(s), but LLVM AOT expected 0",
                arguments.len()
            ));
        }
        let bytes = self.next_temp();
        writeln!(out, "  {bytes} = call ptr @ax_bytes_empty()")
            .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: bytes,
            ax_ty: Some(Type::Bytes),
        })
    }

    fn emit_bytes_from_string(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `bytes_from_string` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let text = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("bytes_from_string", "text", &text)?;
        let bytes = self.next_temp();
        writeln!(
            out,
            "  {bytes} = call ptr @ax_bytes_from_string(ptr {})",
            text.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: bytes,
            ax_ty: Some(Type::Bytes),
        })
    }

    fn emit_bytes_to_string_lossy(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `bytes_to_string_lossy` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let bytes = self.emit_expr(&arguments[0], out)?;
        ensure_bytes_argument("bytes_to_string_lossy", "data", &bytes)?;
        let text = self.next_temp();
        writeln!(
            out,
            "  {text} = call ptr @ax_bytes_to_string_lossy(ptr {})",
            bytes.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: text,
            ax_ty: Some(Type::String),
        })
    }

    fn emit_bytes_to_hex(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `bytes_to_hex` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let bytes = self.emit_expr(&arguments[0], out)?;
        ensure_bytes_argument("bytes_to_hex", "data", &bytes)?;
        let text = self.next_temp();
        writeln!(
            out,
            "  {text} = call ptr @ax_bytes_to_hex(ptr {})",
            bytes.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: text,
            ax_ty: Some(Type::String),
        })
    }

    fn emit_bytes_push(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 2 {
            return Err(format!(
                "call to `bytes_push` has {} argument(s), but LLVM AOT expected 2",
                arguments.len()
            ));
        }
        let bytes = self.emit_expr(&arguments[0], out)?;
        ensure_bytes_argument("bytes_push", "data", &bytes)?;
        let value = self.emit_expr(&arguments[1], out)?;
        ensure_same_type("i32", &value.ty)?;
        let pushed = self.next_temp();
        writeln!(
            out,
            "  {pushed} = call ptr @ax_bytes_push(ptr {}, i32 {})",
            bytes.repr, value.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: pushed,
            ax_ty: Some(Type::Bytes),
        })
    }

    fn emit_bytes_get(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 2 {
            return Err(format!(
                "call to `bytes_get` has {} argument(s), but LLVM AOT expected 2",
                arguments.len()
            ));
        }
        let bytes = self.emit_expr(&arguments[0], out)?;
        ensure_bytes_argument("bytes_get", "data", &bytes)?;
        let index = self.emit_expr(&arguments[1], out)?;
        ensure_same_type("i32", &index.ty)?;
        let value = self.next_temp();
        writeln!(
            out,
            "  {value} = call i32 @ax_bytes_get(ptr {}, i32 {})",
            bytes.repr, index.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "i32".to_string(),
            repr: value,
            ax_ty: Some(Type::I32),
        })
    }

    fn emit_argv_len(
        &mut self,
        function: &str,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if !arguments.is_empty() {
            return Err(format!(
                "call to `{function}` has {} argument(s), but LLVM AOT expected 0",
                arguments.len()
            ));
        }
        let len = self.emit_argv_len_value(out);
        Ok(LlvmValue {
            ty: "i32".to_string(),
            repr: len,
            ax_ty: Some(Type::I32),
        })
    }

    fn emit_argv_get(&mut self, arguments: &[Expr], out: &mut String) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `argv_get` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let index = self.emit_expr(&arguments[0], out)?;
        ensure_same_type("i32", &index.ty)?;
        let len = self.emit_argv_len_value(out);
        let below_zero = self.next_temp();
        let past_end = self.next_temp();
        let out_of_bounds = self.next_temp();
        let fail_label = self.next_label("argv_oob");
        let ok_label = self.next_label("argv_ok");
        writeln!(out, "  {below_zero} = icmp slt i32 {}, 0", index.repr)
            .expect("writing to string cannot fail");
        writeln!(out, "  {past_end} = icmp sge i32 {}, {len}", index.repr)
            .expect("writing to string cannot fail");
        writeln!(out, "  {out_of_bounds} = or i1 {below_zero}, {past_end}")
            .expect("writing to string cannot fail");
        writeln!(
            out,
            "  br i1 {out_of_bounds}, label %{fail_label}, label %{ok_label}"
        )
        .expect("writing to string cannot fail");
        writeln!(out, "{fail_label}:").expect("writing to string cannot fail");
        writeln!(out, "  call void @ax_runtime_error(ptr @.ax_rt_argv_oob)")
            .expect("writing to string cannot fail");
        writeln!(out, "  unreachable").expect("writing to string cannot fail");
        writeln!(out, "{ok_label}:").expect("writing to string cannot fail");
        let native_index = self.next_temp();
        let native_index64 = self.next_temp();
        let argv = self.next_temp();
        let slot = self.next_temp();
        let value = self.next_temp();
        writeln!(out, "  {native_index} = add i32 {}, 1", index.repr)
            .expect("writing to string cannot fail");
        writeln!(out, "  {native_index64} = sext i32 {native_index} to i64")
            .expect("writing to string cannot fail");
        writeln!(out, "  {argv} = load ptr, ptr @.ax_argv").expect("writing to string cannot fail");
        writeln!(
            out,
            "  {slot} = getelementptr ptr, ptr {argv}, i64 {native_index64}"
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  {value} = load ptr, ptr {slot}").expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: value,
            ax_ty: Some(Type::String),
        })
    }

    fn emit_argv_len_value(&mut self, out: &mut String) -> String {
        let argc = self.next_temp();
        let raw_len = self.next_temp();
        let negative = self.next_temp();
        let len = self.next_temp();
        writeln!(out, "  {argc} = load i32, ptr @.ax_argc").expect("writing to string cannot fail");
        writeln!(out, "  {raw_len} = sub i32 {argc}, 1").expect("writing to string cannot fail");
        writeln!(out, "  {negative} = icmp slt i32 {raw_len}, 0")
            .expect("writing to string cannot fail");
        writeln!(out, "  {len} = select i1 {negative}, i32 0, i32 {raw_len}")
            .expect("writing to string cannot fail");
        len
    }

    fn emit_env_has(&mut self, arguments: &[Expr], out: &mut String) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `env_has` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let name = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("env_has", "name", &name)?;
        let value = self.next_temp();
        let present = self.next_temp();
        writeln!(out, "  {value} = call ptr @getenv(ptr {})", name.repr)
            .expect("writing to string cannot fail");
        writeln!(out, "  {present} = icmp ne ptr {value}, null")
            .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "i1".to_string(),
            repr: present,
            ax_ty: Some(Type::Bool),
        })
    }

    fn emit_env_get(&mut self, arguments: &[Expr], out: &mut String) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `env_get` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let name = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("env_get", "name", &name)?;
        let value = self.next_temp();
        let missing = self.next_temp();
        let fail_label = self.next_label("env_missing");
        let ok_label = self.next_label("env_ok");
        writeln!(out, "  {value} = call ptr @getenv(ptr {})", name.repr)
            .expect("writing to string cannot fail");
        writeln!(out, "  {missing} = icmp eq ptr {value}, null")
            .expect("writing to string cannot fail");
        writeln!(
            out,
            "  br i1 {missing}, label %{fail_label}, label %{ok_label}"
        )
        .expect("writing to string cannot fail");
        writeln!(out, "{fail_label}:").expect("writing to string cannot fail");
        writeln!(
            out,
            "  call void @ax_runtime_error(ptr @.ax_rt_env_missing)"
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  unreachable").expect("writing to string cannot fail");
        writeln!(out, "{ok_label}:").expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: value,
            ax_ty: Some(Type::String),
        })
    }

    fn emit_env_try_get(
        &mut self,
        arguments: &[Expr],
        expected_return_ax_ty: Option<&Type>,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `std.env.try_get` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let result_ax_ty = expected_return_ax_ty
            .cloned()
            .or_else(|| {
                self.signatures
                    .get("std.env.try_get")
                    .map(|signature| signature.return_ax_type.clone())
            })
            .ok_or_else(|| {
                "`std.env.try_get` needs `Result<string, string>` return type metadata in LLVM AOT v0"
                    .to_string()
            })?;
        let (success_ax_ty, error_ax_ty) = self.result_success_error_types(&result_ax_ty)?;
        if success_ax_ty != Type::String || error_ax_ty != Type::String {
            return Err(format!(
                "`std.env.try_get` lowers to Result<string, string>, found {}",
                ax_type_name(&result_ax_ty)
            ));
        }

        let name = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("std.env.try_get", "name", &name)?;
        let value = self.next_temp();
        writeln!(out, "  {value} = call ptr @getenv(ptr {})", name.repr)
            .expect("writing to string cannot fail");
        let missing = self.next_temp();
        writeln!(out, "  {missing} = icmp eq ptr {value}, null")
            .expect("writing to string cannot fail");
        let host_error_slot = self.next_temp();
        let ok_label = self.next_label("env_try_get_ok");
        let fail_label = self.next_label("env_try_get_err");
        let done_label = self.next_label("env_try_get_done");
        writeln!(
            out,
            "  {host_error_slot} = alloca {}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  br i1 {missing}, label %{fail_label}, label %{ok_label}"
        )
        .expect("writing to string cannot fail");

        writeln!(out, "{ok_label}:").expect("writing to string cannot fail");
        let ok_error = self.next_temp();
        writeln!(
            out,
            "  {ok_error} = call {} @{}()",
            abi::HOST_ERROR_LLVM_TYPE,
            abi::HOST_ERROR_OK_HELPER
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  store {} {ok_error}, ptr {host_error_slot}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");

        writeln!(out, "{fail_label}:").expect("writing to string cannot fail");
        let missing_message_prefix =
            self.string_literal_symbol("missing environment variable: ")?;
        let missing_message = self.emit_string_concat(&missing_message_prefix, &name.repr, out);
        let failed_error = self.next_temp();
        writeln!(
            out,
            "  {failed_error} = call {} @{}(i32 1, ptr {})",
            abi::HOST_ERROR_LLVM_TYPE,
            abi::HOST_ERROR_NEW_HELPER,
            missing_message
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  store {} {failed_error}, ptr {host_error_slot}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");

        writeln!(out, "{done_label}:").expect("writing to string cannot fail");
        let host_error = self.next_temp();
        writeln!(
            out,
            "  {host_error} = load {}, ptr {host_error_slot}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        self.emit_host_result_from_error(value, host_error, &result_ax_ty, out)
    }

    fn emit_process_cwd(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if !arguments.is_empty() {
            return Err(format!(
                "call to `process_cwd` has {} argument(s), but LLVM AOT expected 0",
                arguments.len()
            ));
        }
        let value = self.next_temp();
        writeln!(out, "  {value} = call ptr @ax_process_cwd()")
            .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: value,
            ax_ty: Some(Type::String),
        })
    }

    fn emit_process_run(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `process_run` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let command = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("process_run", "command", &command)?;
        let status = self.next_temp();
        writeln!(
            out,
            "  {status} = call i32 @ax_process_run(ptr {})",
            command.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "i32".to_string(),
            repr: status,
            ax_ty: Some(Type::I32),
        })
    }

    fn emit_process_try_run(
        &mut self,
        arguments: &[Expr],
        expected_return_ax_ty: Option<&Type>,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `std.process.try_run` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let result_ax_ty = expected_return_ax_ty
            .cloned()
            .or_else(|| {
                self.signatures
                    .get("std.process.try_run")
                    .map(|signature| signature.return_ax_type.clone())
            })
            .ok_or_else(|| {
                "`std.process.try_run` needs `Result<i32, string>` return type metadata in LLVM AOT v0"
                    .to_string()
            })?;
        let (success_ax_ty, error_ax_ty) = self.result_success_error_types(&result_ax_ty)?;
        if success_ax_ty != Type::I32 || error_ax_ty != Type::String {
            return Err(format!(
                "`std.process.try_run` lowers to Result<i32, string>, found {}",
                ax_type_name(&result_ax_ty)
            ));
        }

        let command = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("std.process.try_run", "command", &command)?;
        let length = self.next_temp();
        writeln!(
            out,
            "  {length} = call i32 @ax_string_len(ptr {})",
            command.repr
        )
        .expect("writing to string cannot fail");
        let empty = self.next_temp();
        writeln!(out, "  {empty} = icmp eq i32 {length}, 0")
            .expect("writing to string cannot fail");
        let host_error_slot = self.next_temp();
        let value_slot = self.next_temp();
        let ok_label = self.next_label("process_try_run_ok");
        let fail_label = self.next_label("process_try_run_err");
        let done_label = self.next_label("process_try_run_done");
        writeln!(
            out,
            "  {host_error_slot} = alloca {}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  {value_slot} = alloca i32").expect("writing to string cannot fail");
        writeln!(
            out,
            "  br i1 {empty}, label %{fail_label}, label %{ok_label}"
        )
        .expect("writing to string cannot fail");

        writeln!(out, "{ok_label}:").expect("writing to string cannot fail");
        let status = self.next_temp();
        writeln!(
            out,
            "  {status} = call i32 @ax_process_run(ptr {})",
            command.repr
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  store i32 {status}, ptr {value_slot}")
            .expect("writing to string cannot fail");
        let ok_error = self.next_temp();
        writeln!(
            out,
            "  {ok_error} = call {} @{}()",
            abi::HOST_ERROR_LLVM_TYPE,
            abi::HOST_ERROR_OK_HELPER
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  store {} {ok_error}, ptr {host_error_slot}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");

        writeln!(out, "{fail_label}:").expect("writing to string cannot fail");
        let message = self.string_literal_symbol("command must not be empty")?;
        writeln!(out, "  store i32 0, ptr {value_slot}").expect("writing to string cannot fail");
        let failed_error = self.next_temp();
        writeln!(
            out,
            "  {failed_error} = call {} @{}(i32 1, ptr {})",
            abi::HOST_ERROR_LLVM_TYPE,
            abi::HOST_ERROR_NEW_HELPER,
            message
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  store {} {failed_error}, ptr {host_error_slot}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");

        writeln!(out, "{done_label}:").expect("writing to string cannot fail");
        let value = self.next_temp();
        writeln!(out, "  {value} = load i32, ptr {value_slot}")
            .expect("writing to string cannot fail");
        let host_error = self.next_temp();
        writeln!(
            out,
            "  {host_error} = load {}, ptr {host_error_slot}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        self.emit_host_result_from_error(value, host_error, &result_ax_ty, out)
    }

    fn emit_process_try_status(
        &mut self,
        arguments: &[Expr],
        expected_return_ax_ty: Option<&Type>,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `std.process.try_status` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let result_ax_ty = expected_return_ax_ty
            .cloned()
            .or_else(|| {
                self.signatures
                    .get("std.process.try_status")
                    .map(|signature| signature.return_ax_type.clone())
            })
            .ok_or_else(|| {
                "`std.process.try_status` needs `Result<ProcessStatus, string>` return type metadata in LLVM AOT v0"
                    .to_string()
            })?;
        let (success_ax_ty, error_ax_ty) = self.result_success_error_types(&result_ax_ty)?;
        if error_ax_ty != Type::String {
            return Err(format!(
                "`std.process.try_status` lowers to Result<ProcessStatus, string>, found {}",
                ax_type_name(&result_ax_ty)
            ));
        }
        let Type::Struct {
            name: status_struct,
        } = &success_ax_ty
        else {
            return Err(format!(
                "`std.process.try_status` lowers to Result<ProcessStatus, string>, found {}",
                ax_type_name(&result_ax_ty)
            ));
        };
        if status_struct != "std.process.ProcessStatus" {
            return Err(format!(
                "`std.process.try_status` lowers to Result<std.process.ProcessStatus, string>, found {}",
                ax_type_name(&result_ax_ty)
            ));
        }
        let status_layout = self.struct_layout_for_type(&success_ax_ty)?.clone();

        let command = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("std.process.try_status", "command", &command)?;
        let length = self.next_temp();
        writeln!(
            out,
            "  {length} = call i32 @ax_string_len(ptr {})",
            command.repr
        )
        .expect("writing to string cannot fail");
        let empty = self.next_temp();
        writeln!(out, "  {empty} = icmp eq i32 {length}, 0")
            .expect("writing to string cannot fail");
        let host_error_slot = self.next_temp();
        let value_slot = self.next_temp();
        let ok_label = self.next_label("process_try_status_ok");
        let fail_label = self.next_label("process_try_status_err");
        let done_label = self.next_label("process_try_status_done");
        writeln!(
            out,
            "  {host_error_slot} = alloca {}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  {value_slot} = alloca {}", status_layout.ty)
            .expect("writing to string cannot fail");
        writeln!(
            out,
            "  br i1 {empty}, label %{fail_label}, label %{ok_label}"
        )
        .expect("writing to string cannot fail");

        writeln!(out, "{ok_label}:").expect("writing to string cannot fail");
        let code = self.next_temp();
        writeln!(
            out,
            "  {code} = call i32 @ax_process_run(ptr {})",
            command.repr
        )
        .expect("writing to string cannot fail");
        let success = self.next_temp();
        writeln!(out, "  {success} = icmp eq i32 {code}, 0")
            .expect("writing to string cannot fail");
        let status_value = self.emit_process_status_value(&status_layout, &code, &success, out)?;
        writeln!(
            out,
            "  store {} {status_value}, ptr {value_slot}",
            status_layout.ty
        )
        .expect("writing to string cannot fail");
        let ok_error = self.next_temp();
        writeln!(
            out,
            "  {ok_error} = call {} @{}()",
            abi::HOST_ERROR_LLVM_TYPE,
            abi::HOST_ERROR_OK_HELPER
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  store {} {ok_error}, ptr {host_error_slot}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");

        writeln!(out, "{fail_label}:").expect("writing to string cannot fail");
        let message = self.string_literal_symbol("command must not be empty")?;
        let fallback_status = self.emit_process_status_value(&status_layout, "0", "false", out)?;
        writeln!(
            out,
            "  store {} {fallback_status}, ptr {value_slot}",
            status_layout.ty
        )
        .expect("writing to string cannot fail");
        let failed_error = self.next_temp();
        writeln!(
            out,
            "  {failed_error} = call {} @{}(i32 1, ptr {})",
            abi::HOST_ERROR_LLVM_TYPE,
            abi::HOST_ERROR_NEW_HELPER,
            message
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  store {} {failed_error}, ptr {host_error_slot}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");

        writeln!(out, "{done_label}:").expect("writing to string cannot fail");
        let value = self.next_temp();
        writeln!(
            out,
            "  {value} = load {}, ptr {value_slot}",
            status_layout.ty
        )
        .expect("writing to string cannot fail");
        let host_error = self.next_temp();
        writeln!(
            out,
            "  {host_error} = load {}, ptr {host_error_slot}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        self.emit_host_result_from_error(value, host_error, &result_ax_ty, out)
    }

    fn emit_process_status_value(
        &mut self,
        layout: &StructLayout,
        code: &str,
        success: &str,
        out: &mut String,
    ) -> Result<String, String> {
        let mut fields = Vec::new();
        for field in &layout.fields {
            match field.name.as_str() {
                "code" => fields.push(("i32".to_string(), code.to_string())),
                "success" => fields.push(("i1".to_string(), success.to_string())),
                other => {
                    return Err(format!(
                        "std.process.ProcessStatus field `{other}` is outside LLVM AOT v0"
                    ));
                }
            }
        }
        Ok(self.emit_aggregate_value(&layout.ty, &fields, out))
    }

    fn emit_process_capture(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `process_capture` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let command = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("process_capture", "command", &command)?;
        let output = self.next_temp();
        writeln!(
            out,
            "  {output} = call ptr @ax_process_capture(ptr {})",
            command.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: output,
            ax_ty: Some(Type::String),
        })
    }

    fn emit_process_run_in(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 2 {
            return Err(format!(
                "call to `process_run_in` has {} argument(s), but LLVM AOT expected 2",
                arguments.len()
            ));
        }
        let working_dir = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("process_run_in", "working_dir", &working_dir)?;
        let command = self.emit_expr(&arguments[1], out)?;
        ensure_string_argument("process_run_in", "command", &command)?;
        let status = self.next_temp();
        writeln!(
            out,
            "  {status} = call i32 @ax_process_run_in(ptr {}, ptr {})",
            working_dir.repr, command.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "i32".to_string(),
            repr: status,
            ax_ty: Some(Type::I32),
        })
    }

    fn emit_process_capture_in(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 2 {
            return Err(format!(
                "call to `process_capture_in` has {} argument(s), but LLVM AOT expected 2",
                arguments.len()
            ));
        }
        let working_dir = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("process_capture_in", "working_dir", &working_dir)?;
        let command = self.emit_expr(&arguments[1], out)?;
        ensure_string_argument("process_capture_in", "command", &command)?;
        let output = self.next_temp();
        writeln!(
            out,
            "  {output} = call ptr @ax_process_capture_in(ptr {}, ptr {})",
            working_dir.repr, command.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: output,
            ax_ty: Some(Type::String),
        })
    }

    fn emit_fs_predicate(
        &mut self,
        function: &str,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `{function}` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let path = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument(function, "path", &path)?;
        let helper = match function {
            "fs_exists" => "ax_fs_exists",
            "fs_is_file" => "ax_fs_is_file",
            "fs_is_dir" => "ax_fs_is_dir",
            _ => {
                return Err(format!(
                    "call to `{function}` is outside the LLVM AOT filesystem read subset"
                ));
            }
        };
        let result = self.next_temp();
        writeln!(out, "  {result} = call i1 @{helper}(ptr {})", path.repr)
            .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "i1".to_string(),
            repr: result,
            ax_ty: Some(Type::Bool),
        })
    }

    fn emit_fs_file_size(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `fs_file_size` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let path = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("fs_file_size", "path", &path)?;
        let size = self.next_temp();
        writeln!(
            out,
            "  {size} = call i32 @ax_fs_file_size(ptr {})",
            path.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "i32".to_string(),
            repr: size,
            ax_ty: Some(Type::I32),
        })
    }

    fn emit_fs_read_to_string(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `fs_read_to_string` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let path = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("fs_read_to_string", "path", &path)?;
        let text = self.next_temp();
        writeln!(
            out,
            "  {text} = call ptr @ax_fs_read_to_string(ptr {})",
            path.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: text,
            ax_ty: Some(Type::String),
        })
    }

    fn emit_fs_try_read_to_string(
        &mut self,
        arguments: &[Expr],
        expected_return_ax_ty: Option<&Type>,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `std.fs.try_read_to_string` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let result_ax_ty = expected_return_ax_ty
            .cloned()
            .or_else(|| {
                self.signatures
                    .get("std.fs.try_read_to_string")
                    .map(|signature| signature.return_ax_type.clone())
            })
            .ok_or_else(|| {
                "`std.fs.try_read_to_string` needs `Result<string, string>` return type metadata in LLVM AOT v0"
                    .to_string()
            })?;
        let (success_ax_ty, error_ax_ty) = self.result_success_error_types(&result_ax_ty)?;
        if success_ax_ty != Type::String || error_ax_ty != Type::String {
            return Err(format!(
                "`std.fs.try_read_to_string` lowers to Result<string, string>, found {}",
                ax_type_name(&result_ax_ty)
            ));
        }

        let path = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("std.fs.try_read_to_string", "path", &path)?;
        let is_file = self.next_temp();
        writeln!(
            out,
            "  {is_file} = call i1 @ax_fs_is_file(ptr {})",
            path.repr
        )
        .expect("writing to string cannot fail");
        let host_error_slot = self.next_temp();
        let value_slot = self.next_temp();
        let ok_label = self.next_label("fs_try_read_ok");
        let fail_label = self.next_label("fs_try_read_err");
        let done_label = self.next_label("fs_try_read_done");
        writeln!(
            out,
            "  {host_error_slot} = alloca {}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  {value_slot} = alloca ptr").expect("writing to string cannot fail");
        writeln!(
            out,
            "  br i1 {is_file}, label %{ok_label}, label %{fail_label}"
        )
        .expect("writing to string cannot fail");

        writeln!(out, "{ok_label}:").expect("writing to string cannot fail");
        let text = self.next_temp();
        writeln!(
            out,
            "  {text} = call ptr @ax_fs_read_to_string(ptr {})",
            path.repr
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  store ptr {text}, ptr {value_slot}")
            .expect("writing to string cannot fail");
        let ok_error = self.next_temp();
        writeln!(
            out,
            "  {ok_error} = call {} @{}()",
            abi::HOST_ERROR_LLVM_TYPE,
            abi::HOST_ERROR_OK_HELPER
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  store {} {ok_error}, ptr {host_error_slot}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");

        writeln!(out, "{fail_label}:").expect("writing to string cannot fail");
        let missing_message_prefix =
            self.string_literal_symbol("readable file does not exist: ")?;
        let missing_message = self.emit_string_concat(&missing_message_prefix, &path.repr, out);
        writeln!(out, "  store ptr null, ptr {value_slot}").expect("writing to string cannot fail");
        let failed_error = self.next_temp();
        writeln!(
            out,
            "  {failed_error} = call {} @{}(i32 1, ptr {})",
            abi::HOST_ERROR_LLVM_TYPE,
            abi::HOST_ERROR_NEW_HELPER,
            missing_message
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  store {} {failed_error}, ptr {host_error_slot}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");

        writeln!(out, "{done_label}:").expect("writing to string cannot fail");
        let value = self.next_temp();
        writeln!(out, "  {value} = load ptr, ptr {value_slot}")
            .expect("writing to string cannot fail");
        let host_error = self.next_temp();
        writeln!(
            out,
            "  {host_error} = load {}, ptr {host_error_slot}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        self.emit_host_result_from_error(value, host_error, &result_ax_ty, out)
    }

    fn emit_fs_read_dir(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `fs_read_dir` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let path = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("fs_read_dir", "path", &path)?;
        let entries = self.next_temp();
        writeln!(
            out,
            "  {entries} = call {} @ax_fs_read_dir(ptr {})",
            slice_llvm_type(),
            path.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: slice_llvm_type(),
            repr: entries,
            ax_ty: Some(Type::Slice {
                element: Box::new(Type::String),
            }),
        })
    }

    fn emit_fs_try_read_dir(
        &mut self,
        arguments: &[Expr],
        expected_return_ax_ty: Option<&Type>,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `std.fs.try_read_dir` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let result_ax_ty = expected_return_ax_ty
            .cloned()
            .or_else(|| {
                self.signatures
                    .get("std.fs.try_read_dir")
                    .map(|signature| signature.return_ax_type.clone())
            })
            .ok_or_else(|| {
                "`std.fs.try_read_dir` needs `Result<[string], string>` return type metadata in LLVM AOT v0"
                    .to_string()
            })?;
        let (success_ax_ty, error_ax_ty) = self.result_success_error_types(&result_ax_ty)?;
        if success_ax_ty
            != (Type::Slice {
                element: Box::new(Type::String),
            })
            || error_ax_ty != Type::String
        {
            return Err(format!(
                "`std.fs.try_read_dir` lowers to Result<[string], string>, found {}",
                ax_type_name(&result_ax_ty)
            ));
        }

        let path = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("std.fs.try_read_dir", "path", &path)?;
        let is_dir = self.next_temp();
        writeln!(out, "  {is_dir} = call i1 @ax_fs_is_dir(ptr {})", path.repr)
            .expect("writing to string cannot fail");
        let host_error_slot = self.next_temp();
        let value_slot = self.next_temp();
        let ok_label = self.next_label("fs_try_read_dir_ok");
        let fail_label = self.next_label("fs_try_read_dir_err");
        let done_label = self.next_label("fs_try_read_dir_done");
        writeln!(
            out,
            "  {host_error_slot} = alloca {}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  {value_slot} = alloca {}", slice_llvm_type())
            .expect("writing to string cannot fail");
        writeln!(
            out,
            "  br i1 {is_dir}, label %{ok_label}, label %{fail_label}"
        )
        .expect("writing to string cannot fail");

        writeln!(out, "{ok_label}:").expect("writing to string cannot fail");
        let entries = self.next_temp();
        writeln!(
            out,
            "  {entries} = call {} @ax_fs_read_dir(ptr {})",
            slice_llvm_type(),
            path.repr
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  store {} {entries}, ptr {value_slot}",
            slice_llvm_type()
        )
        .expect("writing to string cannot fail");
        let ok_error = self.next_temp();
        writeln!(
            out,
            "  {ok_error} = call {} @{}()",
            abi::HOST_ERROR_LLVM_TYPE,
            abi::HOST_ERROR_OK_HELPER
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  store {} {ok_error}, ptr {host_error_slot}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");

        writeln!(out, "{fail_label}:").expect("writing to string cannot fail");
        let missing_message_prefix =
            self.string_literal_symbol("readable directory does not exist: ")?;
        let missing_message = self.emit_string_concat(&missing_message_prefix, &path.repr, out);
        let empty_with_ptr = self.next_temp();
        writeln!(
            out,
            "  {empty_with_ptr} = insertvalue {} undef, ptr null, 0",
            slice_llvm_type()
        )
        .expect("writing to string cannot fail");
        let empty_slice = self.next_temp();
        writeln!(
            out,
            "  {empty_slice} = insertvalue {} {empty_with_ptr}, i32 0, 1",
            slice_llvm_type()
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  store {} {empty_slice}, ptr {value_slot}",
            slice_llvm_type()
        )
        .expect("writing to string cannot fail");
        let failed_error = self.next_temp();
        writeln!(
            out,
            "  {failed_error} = call {} @{}(i32 1, ptr {})",
            abi::HOST_ERROR_LLVM_TYPE,
            abi::HOST_ERROR_NEW_HELPER,
            missing_message
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  store {} {failed_error}, ptr {host_error_slot}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");

        writeln!(out, "{done_label}:").expect("writing to string cannot fail");
        let value = self.next_temp();
        writeln!(
            out,
            "  {value} = load {}, ptr {value_slot}",
            slice_llvm_type()
        )
        .expect("writing to string cannot fail");
        let host_error = self.next_temp();
        writeln!(
            out,
            "  {host_error} = load {}, ptr {host_error_slot}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        self.emit_host_result_from_error(value, host_error, &result_ax_ty, out)
    }

    fn emit_fs_try_file_size(
        &mut self,
        arguments: &[Expr],
        expected_return_ax_ty: Option<&Type>,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `std.fs.try_file_size` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let result_ax_ty = expected_return_ax_ty
            .cloned()
            .or_else(|| {
                self.signatures
                    .get("std.fs.try_file_size")
                    .map(|signature| signature.return_ax_type.clone())
            })
            .ok_or_else(|| {
                "`std.fs.try_file_size` needs `Result<i32, string>` return type metadata in LLVM AOT v0"
                    .to_string()
            })?;
        let (success_ax_ty, error_ax_ty) = self.result_success_error_types(&result_ax_ty)?;
        if success_ax_ty != Type::I32 || error_ax_ty != Type::String {
            return Err(format!(
                "`std.fs.try_file_size` lowers to Result<i32, string>, found {}",
                ax_type_name(&result_ax_ty)
            ));
        }

        let path = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("std.fs.try_file_size", "path", &path)?;
        let is_file = self.next_temp();
        writeln!(
            out,
            "  {is_file} = call i1 @ax_fs_is_file(ptr {})",
            path.repr
        )
        .expect("writing to string cannot fail");
        let host_error_slot = self.next_temp();
        let value_slot = self.next_temp();
        let ok_label = self.next_label("fs_try_file_size_ok");
        let fail_label = self.next_label("fs_try_file_size_err");
        let done_label = self.next_label("fs_try_file_size_done");
        writeln!(
            out,
            "  {host_error_slot} = alloca {}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  {value_slot} = alloca i32").expect("writing to string cannot fail");
        writeln!(
            out,
            "  br i1 {is_file}, label %{ok_label}, label %{fail_label}"
        )
        .expect("writing to string cannot fail");

        writeln!(out, "{ok_label}:").expect("writing to string cannot fail");
        let size = self.next_temp();
        writeln!(
            out,
            "  {size} = call i32 @ax_fs_file_size(ptr {})",
            path.repr
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  store i32 {size}, ptr {value_slot}")
            .expect("writing to string cannot fail");
        let ok_error = self.next_temp();
        writeln!(
            out,
            "  {ok_error} = call {} @{}()",
            abi::HOST_ERROR_LLVM_TYPE,
            abi::HOST_ERROR_OK_HELPER
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  store {} {ok_error}, ptr {host_error_slot}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");

        writeln!(out, "{fail_label}:").expect("writing to string cannot fail");
        let missing_message_prefix = self.string_literal_symbol("sized file does not exist: ")?;
        let missing_message = self.emit_string_concat(&missing_message_prefix, &path.repr, out);
        writeln!(out, "  store i32 0, ptr {value_slot}").expect("writing to string cannot fail");
        let failed_error = self.next_temp();
        writeln!(
            out,
            "  {failed_error} = call {} @{}(i32 1, ptr {})",
            abi::HOST_ERROR_LLVM_TYPE,
            abi::HOST_ERROR_NEW_HELPER,
            missing_message
        )
        .expect("writing to string cannot fail");
        writeln!(
            out,
            "  store {} {failed_error}, ptr {host_error_slot}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");

        writeln!(out, "{done_label}:").expect("writing to string cannot fail");
        let value = self.next_temp();
        writeln!(out, "  {value} = load i32, ptr {value_slot}")
            .expect("writing to string cannot fail");
        let host_error = self.next_temp();
        writeln!(
            out,
            "  {host_error} = load {}, ptr {host_error_slot}",
            abi::HOST_ERROR_LLVM_TYPE
        )
        .expect("writing to string cannot fail");
        self.emit_host_result_from_error(value, host_error, &result_ax_ty, out)
    }

    fn emit_fs_write_string(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 2 {
            return Err(format!(
                "call to `fs_write_string` has {} argument(s), but LLVM AOT expected 2",
                arguments.len()
            ));
        }
        let path = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("fs_write_string", "path", &path)?;
        let text = self.emit_expr(&arguments[1], out)?;
        ensure_string_argument("fs_write_string", "text", &text)?;
        writeln!(
            out,
            "  call void @ax_fs_write_string(ptr {}, ptr {})",
            path.repr, text.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "void".to_string(),
            repr: String::new(),
            ax_ty: None,
        })
    }

    fn emit_fs_remove_file(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `fs_remove_file` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let path = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("fs_remove_file", "path", &path)?;
        writeln!(out, "  call void @ax_fs_remove_file(ptr {})", path.repr)
            .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "void".to_string(),
            repr: String::new(),
            ax_ty: None,
        })
    }

    fn emit_fs_rename(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 2 {
            return Err(format!(
                "call to `fs_rename` has {} argument(s), but LLVM AOT expected 2",
                arguments.len()
            ));
        }
        let from = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("fs_rename", "from", &from)?;
        let to = self.emit_expr(&arguments[1], out)?;
        ensure_string_argument("fs_rename", "to", &to)?;
        writeln!(
            out,
            "  call void @ax_fs_rename(ptr {}, ptr {})",
            from.repr, to.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "void".to_string(),
            repr: String::new(),
            ax_ty: None,
        })
    }

    fn emit_fs_copy_file(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 2 {
            return Err(format!(
                "call to `fs_copy_file` has {} argument(s), but LLVM AOT expected 2",
                arguments.len()
            ));
        }
        let source = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("fs_copy_file", "source", &source)?;
        let destination = self.emit_expr(&arguments[1], out)?;
        ensure_string_argument("fs_copy_file", "destination", &destination)?;
        let bytes = self.next_temp();
        writeln!(
            out,
            "  {bytes} = call i32 @ax_fs_copy_file(ptr {}, ptr {})",
            source.repr, destination.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "i32".to_string(),
            repr: bytes,
            ax_ty: Some(Type::I32),
        })
    }

    fn emit_fs_create_dir_all(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `fs_create_dir_all` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let path = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("fs_create_dir_all", "path", &path)?;
        writeln!(out, "  call void @ax_fs_create_dir_all(ptr {})", path.repr)
            .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "void".to_string(),
            repr: String::new(),
            ax_ty: None,
        })
    }

    fn emit_fs_remove_dir_all(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `fs_remove_dir_all` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let path = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("fs_remove_dir_all", "path", &path)?;
        writeln!(out, "  call void @ax_fs_remove_dir_all(ptr {})", path.repr)
            .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "void".to_string(),
            repr: String::new(),
            ax_ty: None,
        })
    }

    fn emit_string_predicate(
        &mut self,
        function: &str,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 2 {
            return Err(format!(
                "call to `{function}` has {} argument(s), but LLVM AOT expected 2",
                arguments.len()
            ));
        }

        let text = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument(function, "text", &text)?;
        let pattern = self.emit_expr(&arguments[1], out)?;
        let pattern_name = match function {
            "string_contains" => "needle",
            "string_starts_with" => "prefix",
            "string_ends_with" => "suffix",
            _ => "pattern",
        };
        ensure_string_argument(function, pattern_name, &pattern)?;

        match function {
            "string_contains" => {
                let found = self.next_temp();
                writeln!(
                    out,
                    "  {found} = call ptr @strstr(ptr {}, ptr {})",
                    text.repr, pattern.repr
                )
                .expect("writing to string cannot fail");
                let result = self.next_temp();
                writeln!(out, "  {result} = icmp ne ptr {found}, null")
                    .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: result,
                    ax_ty: Some(Type::Bool),
                })
            }
            "string_starts_with" => {
                let prefix_len = self.next_temp();
                writeln!(
                    out,
                    "  {prefix_len} = call i64 @strlen(ptr {})",
                    pattern.repr
                )
                .expect("writing to string cannot fail");
                let compare = self.next_temp();
                writeln!(
                    out,
                    "  {compare} = call i32 @strncmp(ptr {}, ptr {}, i64 {prefix_len})",
                    text.repr, pattern.repr
                )
                .expect("writing to string cannot fail");
                let result = self.next_temp();
                writeln!(out, "  {result} = icmp eq i32 {compare}, 0")
                    .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: result,
                    ax_ty: Some(Type::Bool),
                })
            }
            "string_ends_with" => {
                let text_len = self.next_temp();
                writeln!(out, "  {text_len} = call i64 @strlen(ptr {})", text.repr)
                    .expect("writing to string cannot fail");
                let suffix_len = self.next_temp();
                writeln!(
                    out,
                    "  {suffix_len} = call i64 @strlen(ptr {})",
                    pattern.repr
                )
                .expect("writing to string cannot fail");
                let too_long = self.next_temp();
                writeln!(out, "  {too_long} = icmp ugt i64 {suffix_len}, {text_len}")
                    .expect("writing to string cannot fail");
                let compare_label = self.next_label("string_suffix_compare");
                let false_label = self.next_label("string_suffix_false");
                let done_label = self.next_label("string_suffix_done");
                writeln!(
                    out,
                    "  br i1 {too_long}, label %{false_label}, label %{compare_label}"
                )
                .expect("writing to string cannot fail");
                writeln!(out, "{compare_label}:").expect("writing to string cannot fail");
                let offset = self.next_temp();
                writeln!(out, "  {offset} = sub i64 {text_len}, {suffix_len}")
                    .expect("writing to string cannot fail");
                let suffix_start = self.next_temp();
                writeln!(
                    out,
                    "  {suffix_start} = getelementptr i8, ptr {}, i64 {offset}",
                    text.repr
                )
                .expect("writing to string cannot fail");
                let compare = self.next_temp();
                writeln!(
                    out,
                    "  {compare} = call i32 @strncmp(ptr {suffix_start}, ptr {}, i64 {suffix_len})",
                    pattern.repr
                )
                .expect("writing to string cannot fail");
                let matched = self.next_temp();
                writeln!(out, "  {matched} = icmp eq i32 {compare}, 0")
                    .expect("writing to string cannot fail");
                writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");
                writeln!(out, "{false_label}:").expect("writing to string cannot fail");
                writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");
                writeln!(out, "{done_label}:").expect("writing to string cannot fail");
                let result = self.next_temp();
                writeln!(
                    out,
                    "  {result} = phi i1 [{matched}, %{compare_label}], [0, %{false_label}]"
                )
                .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "i1".to_string(),
                    repr: result,
                    ax_ty: Some(Type::Bool),
                })
            }
            _ => Err(format!(
                "call to `{function}` is outside LLVM AOT v0 string predicate lowering"
            )),
        }
    }

    fn emit_string_trim(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `string_trim` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }

        let text = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("string_trim", "text", &text)?;
        let trimmed = self.next_temp();
        writeln!(
            out,
            "  {trimmed} = call ptr @ax_string_trim(ptr {})",
            text.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: trimmed,
            ax_ty: Some(Type::String),
        })
    }

    fn emit_string_replace(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 3 {
            return Err(format!(
                "call to `string_replace` has {} argument(s), but LLVM AOT expected 3",
                arguments.len()
            ));
        }

        let text = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("string_replace", "text", &text)?;
        let from = self.emit_expr(&arguments[1], out)?;
        ensure_string_argument("string_replace", "from", &from)?;
        let to = self.emit_expr(&arguments[2], out)?;
        ensure_string_argument("string_replace", "to", &to)?;

        let replaced = self.next_temp();
        writeln!(
            out,
            "  {replaced} = call ptr @ax_string_replace(ptr {}, ptr {}, ptr {})",
            text.repr, from.repr, to.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: replaced,
            ax_ty: Some(Type::String),
        })
    }

    fn emit_path_join(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 2 {
            return Err(format!(
                "call to `path_join` has {} argument(s), but LLVM AOT expected 2",
                arguments.len()
            ));
        }
        let left = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("path_join", "left", &left)?;
        let right = self.emit_expr(&arguments[1], out)?;
        ensure_string_argument("path_join", "right", &right)?;
        let value = self.next_temp();
        writeln!(
            out,
            "  {value} = call ptr @ax_path_join(ptr {}, ptr {})",
            left.repr, right.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: value,
            ax_ty: Some(Type::String),
        })
    }

    fn emit_path_string_function(
        &mut self,
        function: &str,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `{function}` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let path = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument(function, "path", &path)?;
        let helper = match function {
            "path_parent" => "ax_path_parent",
            "path_resolve" => "ax_path_resolve",
            "path_file_name" => "ax_path_file_name",
            "path_stem" => "ax_path_stem",
            "path_extension" => "ax_path_extension",
            _ => {
                return Err(format!(
                    "call to `{function}` is outside the LLVM AOT path subset"
                ));
            }
        };
        let value = self.next_temp();
        writeln!(out, "  {value} = call ptr @{helper}(ptr {})", path.repr)
            .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: value,
            ax_ty: Some(Type::String),
        })
    }

    fn emit_path_is_absolute(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `path_is_absolute` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let path = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("path_is_absolute", "path", &path)?;
        let value = self.next_temp();
        writeln!(
            out,
            "  {value} = call i1 @ax_path_is_absolute(ptr {})",
            path.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "i1".to_string(),
            repr: value,
            ax_ty: Some(Type::Bool),
        })
    }

    fn emit_string_split_lines(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `string_split_lines` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }

        let text = self.emit_expr(&arguments[0], out)?;
        ensure_string_argument("string_split_lines", "text", &text)?;
        let lines = self.next_temp();
        writeln!(
            out,
            "  {lines} = call {} @ax_string_split_lines(ptr {})",
            slice_llvm_type(),
            text.repr
        )
        .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: slice_llvm_type(),
            repr: lines,
            ax_ty: Some(Type::Slice {
                element: Box::new(Type::String),
            }),
        })
    }

    fn emit_to_string(
        &mut self,
        arguments: &[Expr],
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if arguments.len() != 1 {
            return Err(format!(
                "call to `to_string` has {} argument(s), but LLVM AOT expected 1",
                arguments.len()
            ));
        }
        let value = self.emit_expr(&arguments[0], out)?;
        self.emit_value_to_string(value, out)
    }

    fn emit_value_to_string(
        &mut self,
        value: LlvmValue,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        if is_enum_value(&value) {
            return self.emit_enum_to_string(value, out);
        }
        match value.ax_ty.clone() {
            Some(Type::Array { element, length }) => {
                return self.emit_array_to_string(value, &element, length, out);
            }
            Some(Type::Struct { name }) => {
                return self.emit_struct_to_string(value, &name, out);
            }
            Some(Type::Slice { element }) => {
                return self.emit_slice_to_string(value, &element, out);
            }
            _ => {}
        }
        match value.ty.as_str() {
            "i32" => {
                let temp = self.next_temp();
                writeln!(
                    out,
                    "  {temp} = call ptr @ax_i32_to_string(i32 {})",
                    value.repr
                )
                .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "ptr".to_string(),
                    repr: temp,
                    ax_ty: Some(Type::String),
                })
            }
            "float" => {
                let temp = self.next_temp();
                writeln!(
                    out,
                    "  {temp} = call ptr @ax_f32_to_string(float {})",
                    value.repr
                )
                .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "ptr".to_string(),
                    repr: temp,
                    ax_ty: Some(Type::String),
                })
            }
            "i1" => {
                let temp = self.next_temp();
                writeln!(
                    out,
                    "  {temp} = select i1 {}, ptr @.ax_text_true, ptr @.ax_text_false",
                    value.repr
                )
                .expect("writing to string cannot fail");
                Ok(LlvmValue {
                    ty: "ptr".to_string(),
                    repr: temp,
                    ax_ty: Some(Type::String),
                })
            }
            "ptr" => Ok(value),
            "void" => Err("to_string cannot format a `<void>` value in LLVM AOT v0".to_string()),
            other => Err(format!(
                "to_string({other}) needs a native runtime formatter before LLVM AOT can lower it"
            )),
        }
    }

    fn emit_array_to_string(
        &mut self,
        value: LlvmValue,
        element: &Type,
        length: usize,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
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
        ensure_same_type(&array_ty, &value.ty)?;
        let element_ty = llvm_type(element, self.layouts, self.enum_layouts).ok_or_else(|| {
            format!(
                "array element type {} needs a native formatter before to_string(array) can lower in LLVM AOT v0",
                ax_type_name(element)
            )
        })?;

        let mut current = self.string_literal_symbol("[")?;
        for index in 0..length {
            if index > 0 {
                let separator = self.string_literal_symbol(", ")?;
                current = self.emit_string_concat(&current, &separator, out);
            }

            let element_value = self.next_temp();
            writeln!(
                out,
                "  {element_value} = extractvalue {array_ty} {}, {index}",
                value.repr
            )
            .expect("writing to string cannot fail");
            let element_text = self.emit_value_to_string(
                LlvmValue {
                    ty: element_ty.clone(),
                    repr: element_value,
                    ax_ty: Some(element.clone()),
                },
                out,
            )?;
            current = self.emit_string_concat(&current, &element_text.repr, out);
        }

        let close = self.string_literal_symbol("]")?;
        let result = self.emit_string_concat(&current, &close, out);
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: result,
            ax_ty: Some(Type::String),
        })
    }

    fn emit_slice_to_string(
        &mut self,
        value: LlvmValue,
        element: &Type,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        ensure_same_type(&slice_llvm_type(), &value.ty)?;
        let element_ty = llvm_type(element, self.layouts, self.enum_layouts).ok_or_else(|| {
            format!(
                "slice element type {} needs a native formatter before to_string(slice) can lower in LLVM AOT v0",
                ax_type_name(element)
            )
        })?;

        let data_ptr = self.next_temp();
        writeln!(
            out,
            "  {data_ptr} = extractvalue {} {}, 0",
            value.ty, value.repr
        )
        .expect("writing to string cannot fail");
        let len = self.next_temp();
        writeln!(out, "  {len} = extractvalue {} {}, 1", value.ty, value.repr)
            .expect("writing to string cannot fail");

        let index_slot = self.next_temp();
        let current_slot = self.next_temp();
        let open = self.string_literal_symbol("[")?;
        writeln!(out, "  {index_slot} = alloca i32").expect("writing to string cannot fail");
        writeln!(out, "  {current_slot} = alloca ptr").expect("writing to string cannot fail");
        writeln!(out, "  store i32 0, ptr {index_slot}").expect("writing to string cannot fail");
        writeln!(out, "  store ptr {open}, ptr {current_slot}")
            .expect("writing to string cannot fail");

        let loop_label = self.next_label("slice_to_string_loop");
        let body_label = self.next_label("slice_to_string_body");
        let separator_label = self.next_label("slice_to_string_separator");
        let element_label = self.next_label("slice_to_string_element");
        let done_label = self.next_label("slice_to_string_done");

        writeln!(out, "  br label %{loop_label}").expect("writing to string cannot fail");
        writeln!(out, "{loop_label}:").expect("writing to string cannot fail");
        let index = self.next_temp();
        writeln!(out, "  {index} = load i32, ptr {index_slot}")
            .expect("writing to string cannot fail");
        let has_item = self.next_temp();
        writeln!(out, "  {has_item} = icmp slt i32 {index}, {len}")
            .expect("writing to string cannot fail");
        writeln!(
            out,
            "  br i1 {has_item}, label %{body_label}, label %{done_label}"
        )
        .expect("writing to string cannot fail");

        writeln!(out, "{body_label}:").expect("writing to string cannot fail");
        let needs_separator = self.next_temp();
        writeln!(out, "  {needs_separator} = icmp ne i32 {index}, 0")
            .expect("writing to string cannot fail");
        writeln!(
            out,
            "  br i1 {needs_separator}, label %{separator_label}, label %{element_label}"
        )
        .expect("writing to string cannot fail");

        writeln!(out, "{separator_label}:").expect("writing to string cannot fail");
        let current_before_separator = self.next_temp();
        writeln!(
            out,
            "  {current_before_separator} = load ptr, ptr {current_slot}"
        )
        .expect("writing to string cannot fail");
        let separator = self.string_literal_symbol(", ")?;
        let with_separator = self.emit_string_concat(&current_before_separator, &separator, out);
        writeln!(out, "  store ptr {with_separator}, ptr {current_slot}")
            .expect("writing to string cannot fail");
        writeln!(out, "  br label %{element_label}").expect("writing to string cannot fail");

        writeln!(out, "{element_label}:").expect("writing to string cannot fail");
        let prefix = self.next_temp();
        writeln!(out, "  {prefix} = load ptr, ptr {current_slot}")
            .expect("writing to string cannot fail");
        let element_ptr = self.next_temp();
        writeln!(
            out,
            "  {element_ptr} = getelementptr {element_ty}, ptr {data_ptr}, i32 {index}"
        )
        .expect("writing to string cannot fail");
        let element_value = self.next_temp();
        writeln!(
            out,
            "  {element_value} = load {element_ty}, ptr {element_ptr}"
        )
        .expect("writing to string cannot fail");
        let element_text = self.emit_value_to_string(
            LlvmValue {
                ty: element_ty,
                repr: element_value,
                ax_ty: Some(element.clone()),
            },
            out,
        )?;
        let next_current = self.emit_string_concat(&prefix, &element_text.repr, out);
        writeln!(out, "  store ptr {next_current}, ptr {current_slot}")
            .expect("writing to string cannot fail");
        let next_index = self.next_temp();
        writeln!(out, "  {next_index} = add i32 {index}, 1")
            .expect("writing to string cannot fail");
        writeln!(out, "  store i32 {next_index}, ptr {index_slot}")
            .expect("writing to string cannot fail");
        writeln!(out, "  br label %{loop_label}").expect("writing to string cannot fail");

        writeln!(out, "{done_label}:").expect("writing to string cannot fail");
        let current = self.next_temp();
        writeln!(out, "  {current} = load ptr, ptr {current_slot}")
            .expect("writing to string cannot fail");
        let close = self.string_literal_symbol("]")?;
        let result = self.emit_string_concat(&current, &close, out);
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: result,
            ax_ty: Some(Type::String),
        })
    }

    fn emit_struct_to_string(
        &mut self,
        value: LlvmValue,
        name: &str,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let layout = self.layouts.get(name).cloned().ok_or_else(|| {
            format!("struct `{name}` is outside the LLVM AOT v0 formatter subset")
        })?;
        ensure_same_type(&layout.ty, &value.ty)?;

        let mut fields = layout.fields.clone();
        fields.sort_by(|left, right| left.name.cmp(&right.name));

        let prefix = self.string_literal_symbol(&struct_formatter_prefix(&layout.name))?;
        let mut current = prefix;
        for (field_index, field) in fields.iter().enumerate() {
            if field_index > 0 {
                let separator = self.string_literal_symbol(", ")?;
                current = self.emit_string_concat(&current, &separator, out);
            }

            let label = self.string_literal_symbol(&struct_field_formatter_label(&field.name))?;
            current = self.emit_string_concat(&current, &label, out);

            let field_value = self.next_temp();
            writeln!(
                out,
                "  {field_value} = extractvalue {} {}, {}",
                layout.ty, value.repr, field.index
            )
            .expect("writing to string cannot fail");
            let field_text = self.emit_value_to_string(
                LlvmValue {
                    ty: field.ty.clone(),
                    repr: field_value,
                    ax_ty: Some(field.ax_ty.clone()),
                },
                out,
            )?;
            current = self.emit_string_concat(&current, &field_text.repr, out);
        }

        let close = self.string_literal_symbol(" }")?;
        let result = self.emit_string_concat(&current, &close, out);
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: result,
            ax_ty: Some(Type::String),
        })
    }

    fn emit_enum_to_string(
        &mut self,
        value: LlvmValue,
        out: &mut String,
    ) -> Result<LlvmValue, String> {
        let Some(enum_ax_ty @ (Type::Enum { .. } | Type::EnumInstance { .. })) =
            value.ax_ty.clone()
        else {
            return Err("to_string(enum) requires an enum value in LLVM AOT v0".to_string());
        };
        let layout = self.enum_layout_for_type(&enum_ax_ty)?.clone();
        ensure_same_type(&layout.ty, &value.ty)?;
        if layout.variants.is_empty() {
            return Err(format!(
                "enum `{}` has no variants for native formatter lowering in LLVM AOT v0",
                layout.name
            ));
        }

        let tag = if layout.ty == "i32" {
            value.repr.clone()
        } else {
            let tag = self.next_temp();
            writeln!(
                out,
                "  {tag} = extractvalue {} {}, 0",
                layout.ty, value.repr
            )
            .expect("writing to string cannot fail");
            tag
        };

        let result_slot = self.next_temp();
        let done_label = self.next_label("enum_to_string_done");
        let variant_blocks = layout
            .variants
            .iter()
            .map(|variant| {
                (
                    variant.clone(),
                    self.next_label(&format!("enum_to_string_test_{}", variant.name)),
                    self.next_label(&format!("enum_to_string_{}", variant.name)),
                )
            })
            .collect::<Vec<_>>();

        let first_label = if variant_blocks.len() == 1 {
            &variant_blocks[0].2
        } else {
            &variant_blocks[0].1
        };
        writeln!(out, "  {result_slot} = alloca ptr").expect("writing to string cannot fail");
        writeln!(out, "  br label %{first_label}").expect("writing to string cannot fail");

        for (index, (variant, test_label, variant_label)) in variant_blocks.iter().enumerate() {
            if variant_blocks.len() > 1 {
                writeln!(out, "{test_label}:").expect("writing to string cannot fail");
                if index + 1 == variant_blocks.len() {
                    writeln!(out, "  br label %{variant_label}")
                        .expect("writing to string cannot fail");
                } else {
                    let is_variant = self.next_temp();
                    writeln!(out, "  {is_variant} = icmp eq i32 {tag}, {}", variant.tag)
                        .expect("writing to string cannot fail");
                    let next_test_label = &variant_blocks[index + 1].1;
                    writeln!(
                        out,
                        "  br i1 {is_variant}, label %{variant_label}, label %{next_test_label}"
                    )
                    .expect("writing to string cannot fail");
                }
            }

            writeln!(out, "{variant_label}:").expect("writing to string cannot fail");
            let formatted = self.emit_enum_variant_to_string(&layout, variant, &value, out)?;
            writeln!(out, "  store ptr {formatted}, ptr {result_slot}")
                .expect("writing to string cannot fail");
            writeln!(out, "  br label %{done_label}").expect("writing to string cannot fail");
        }

        writeln!(out, "{done_label}:").expect("writing to string cannot fail");
        let result = self.next_temp();
        writeln!(out, "  {result} = load ptr, ptr {result_slot}")
            .expect("writing to string cannot fail");
        Ok(LlvmValue {
            ty: "ptr".to_string(),
            repr: result,
            ax_ty: Some(Type::String),
        })
    }

    fn emit_enum_variant_to_string(
        &mut self,
        layout: &EnumLayout,
        variant: &EnumVariantLayout,
        value: &LlvmValue,
        out: &mut String,
    ) -> Result<String, String> {
        let label = enum_formatter_label(&layout.name, &variant.name);
        let label_text = self.string_literal_symbol(&label)?;
        let Some(payload_ax_ty) = &variant.payload_ax_ty else {
            return Ok(label_text);
        };
        if layout.ty == "i32" {
            return Err(format!(
                "enum `{}.{}` carries a payload but has no native payload storage in LLVM AOT v0",
                layout.name, variant.name
            ));
        }

        let payload_ty =
            llvm_type(payload_ax_ty, self.layouts, self.enum_layouts).ok_or_else(|| {
                format!(
                    "enum `{}.{}` payload type {} is outside LLVM AOT v0",
                    layout.name,
                    variant.name,
                    ax_type_name(payload_ax_ty)
                )
            })?;
        let payload_ptr = self.next_temp();
        writeln!(
            out,
            "  {payload_ptr} = extractvalue {} {}, 1",
            layout.ty, value.repr
        )
        .expect("writing to string cannot fail");
        let payload = self.next_temp();
        writeln!(out, "  {payload} = load {payload_ty}, ptr {payload_ptr}")
            .expect("writing to string cannot fail");

        let payload_text =
            self.emit_enum_payload_to_string(payload_ax_ty, &payload_ty, &payload, out)?;
        let open = self.string_literal_symbol("(")?;
        let close = self.string_literal_symbol(")")?;

        let with_open = self.next_temp();
        writeln!(
            out,
            "  {with_open} = call ptr @ax_string_concat(ptr {label_text}, ptr {open})"
        )
        .expect("writing to string cannot fail");
        let with_payload = self.next_temp();
        writeln!(
            out,
            "  {with_payload} = call ptr @ax_string_concat(ptr {with_open}, ptr {payload_text})"
        )
        .expect("writing to string cannot fail");
        let with_close = self.next_temp();
        writeln!(
            out,
            "  {with_close} = call ptr @ax_string_concat(ptr {with_payload}, ptr {close})"
        )
        .expect("writing to string cannot fail");
        Ok(with_close)
    }

    fn emit_enum_payload_to_string(
        &mut self,
        payload_ax_ty: &Type,
        payload_ty: &str,
        payload: &str,
        out: &mut String,
    ) -> Result<String, String> {
        let rendered = self.emit_value_to_string(
            LlvmValue {
                ty: payload_ty.to_string(),
                repr: payload.to_string(),
                ax_ty: Some(payload_ax_ty.clone()),
            },
            out,
        )?;
        Ok(rendered.repr)
    }

    fn emit_println(&mut self, arguments: &[Expr], out: &mut String) -> Result<LlvmValue, String> {
        for (index, argument) in arguments.iter().enumerate() {
            if index > 0 {
                self.emit_printf_text("@.ax_text_space", out);
            }
            let value = self.emit_expr(argument, out)?;
            self.emit_print_value(value, out)?;
        }
        self.emit_printf_text("@.ax_text_newline", out);
        Ok(LlvmValue {
            ty: "void".to_string(),
            repr: String::new(),
            ax_ty: None,
        })
    }

    fn emit_print_value(&mut self, value: LlvmValue, out: &mut String) -> Result<(), String> {
        if matches!(
            value.ax_ty.as_ref(),
            Some(
                Type::Array { .. }
                    | Type::Slice { .. }
                    | Type::Struct { .. }
                    | Type::Enum { .. }
                    | Type::EnumInstance { .. }
            )
        ) {
            let text = self.emit_value_to_string(value, out)?;
            writeln!(
                out,
                "  call i32 (ptr, ...) @printf(ptr @.ax_fmt_str, ptr {})",
                text.repr
            )
            .expect("writing to string cannot fail");
            return Ok(());
        }
        match value.ty.as_str() {
            "i32" => {
                writeln!(
                    out,
                    "  call i32 (ptr, ...) @printf(ptr @.ax_fmt_i32, i32 {})",
                    value.repr
                )
                .expect("writing to string cannot fail");
                Ok(())
            }
            "float" => {
                let text = self.next_temp();
                writeln!(
                    out,
                    "  {text} = call ptr @ax_f32_to_string(float {})",
                    value.repr
                )
                .expect("writing to string cannot fail");
                writeln!(
                    out,
                    "  call i32 (ptr, ...) @printf(ptr @.ax_fmt_str, ptr {text})"
                )
                .expect("writing to string cannot fail");
                Ok(())
            }
            "i1" => {
                let text = self.next_temp();
                writeln!(
                    out,
                    "  {text} = select i1 {}, ptr @.ax_text_true, ptr @.ax_text_false",
                    value.repr
                )
                .expect("writing to string cannot fail");
                writeln!(
                    out,
                    "  call i32 (ptr, ...) @printf(ptr @.ax_fmt_str, ptr {text})"
                )
                .expect("writing to string cannot fail");
                Ok(())
            }
            "ptr" => {
                writeln!(
                    out,
                    "  call i32 (ptr, ...) @printf(ptr @.ax_fmt_str, ptr {})",
                    value.repr
                )
                .expect("writing to string cannot fail");
                Ok(())
            }
            "void" => Err("println cannot print a `<void>` value in LLVM AOT v0".to_string()),
            other => Err(format!(
                "println({other}) needs a native runtime ABI before LLVM AOT can lower it"
            )),
        }
    }

    fn emit_printf_text(&mut self, global: &str, out: &mut String) {
        writeln!(
            out,
            "  call i32 (ptr, ...) @printf(ptr @.ax_fmt_str, ptr {global})"
        )
        .expect("writing to string cannot fail");
    }

    fn emit_string_concat(&mut self, left: &str, right: &str, out: &mut String) -> String {
        let temp = self.next_temp();
        writeln!(
            out,
            "  {temp} = call ptr @ax_string_concat(ptr {left}, ptr {right})"
        )
        .expect("writing to string cannot fail");
        temp
    }
}
