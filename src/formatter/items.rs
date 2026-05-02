use super::*;

impl Formatter {
    pub(in crate::formatter) fn format_program(&mut self, program: &Program) {
        for (index, item) in program.items.iter().enumerate() {
            self.format_item(item);
            if index + 1 < program.items.len() {
                self.out.push_str("\n\n");
            }
        }
    }

    pub(in crate::formatter) fn format_item(&mut self, item: &Item) {
        if item.visibility == Visibility::Public {
            self.out.push_str("pub ");
        }
        match &item.kind {
            ItemKind::Function {
                name,
                type_params,
                type_param_bounds,
                params,
                return_type,
                body,
            } => self.format_function_item(
                name,
                type_params,
                type_param_bounds,
                params,
                return_type,
                body,
            ),
            ItemKind::Const { name, ty, value } => self.format_const_item(name, ty, value),
            ItemKind::TypeAlias {
                name,
                type_params,
                target,
            } => self.format_type_alias_item(name, type_params, target),
            ItemKind::Struct {
                name,
                type_params,
                fields,
            } => self.format_struct_item(name, type_params, fields),
            ItemKind::Enum {
                name,
                type_params,
                variants,
            } => self.format_enum_item(name, type_params, variants),
            ItemKind::Trait { name, methods } => self.format_trait_item(name, methods),
            ItemKind::Impl {
                type_params,
                trait_ref,
                target,
                methods,
            } => self.format_impl_item(type_params, trait_ref.as_ref(), target, methods),
        }
    }

    pub(in crate::formatter) fn format_const_item(
        &mut self,
        name: &str,
        ty: &TypeRef,
        value: &Expr,
    ) {
        let _ = write!(
            self.out,
            "const {name}: {} = {};",
            format_type_ref(ty),
            format_expr(value)
        );
    }

    pub(in crate::formatter) fn format_type_alias_item(
        &mut self,
        name: &str,
        type_params: &[String],
        target: &TypeRef,
    ) {
        let _ = write!(
            self.out,
            "type {name}{} = {};",
            format_type_params(type_params),
            format_type_ref(target)
        );
    }

    pub(in crate::formatter) fn format_function_item(
        &mut self,
        name: &str,
        type_params: &[String],
        type_param_bounds: &[TypeParamBound],
        params: &[Param],
        return_type: &TypeRef,
        body: &Block,
    ) {
        let _ = write!(
            self.out,
            "fn {name}{}(",
            format_function_type_params(type_params, type_param_bounds)
        );
        for (index, param) in params.iter().enumerate() {
            if index > 0 {
                self.out.push_str(", ");
            }
            let _ = write!(self.out, "{}: {}", param.name, format_type_ref(&param.ty));
        }
        let _ = write!(self.out, ") -> {} ", format_type_ref(return_type));
        self.format_block(body);
    }

    pub(in crate::formatter) fn format_struct_item(
        &mut self,
        name: &str,
        type_params: &[String],
        fields: &[StructField],
    ) {
        let params = format_type_params(type_params);
        if fields.is_empty() {
            let _ = write!(self.out, "struct {name}{params} {{}}");
            return;
        }

        let _ = writeln!(self.out, "struct {name}{params} {{");
        self.indent += 1;
        for field in fields {
            self.write_indent();
            let _ = writeln!(self.out, "{}: {},", field.name, format_type_ref(&field.ty));
        }
        self.indent -= 1;
        self.write_indent();
        self.out.push('}');
    }

    pub(in crate::formatter) fn format_impl_item(
        &mut self,
        type_params: &[String],
        trait_ref: Option<&TypeRef>,
        target: &TypeRef,
        methods: &[ImplMethod],
    ) {
        let params = format_type_params(type_params);
        if let Some(trait_ref) = trait_ref {
            let _ = writeln!(
                self.out,
                "impl{} {} for {} {{",
                params,
                format_type_ref(trait_ref),
                format_type_ref(target)
            );
        } else {
            let _ = writeln!(self.out, "impl{} {} {{", params, format_type_ref(target));
        }
        self.indent += 1;
        for (index, method) in methods.iter().enumerate() {
            self.write_indent();
            self.format_method_item(method);
            if index + 1 < methods.len() {
                self.out.push('\n');
                self.out.push('\n');
            } else {
                self.out.push('\n');
            }
        }
        self.indent -= 1;
        self.write_indent();
        self.out.push('}');
    }

    pub(in crate::formatter) fn format_trait_item(&mut self, name: &str, methods: &[TraitMethod]) {
        if methods.is_empty() {
            let _ = write!(self.out, "trait {name} {{}}");
            return;
        }

        let _ = writeln!(self.out, "trait {name} {{");
        self.indent += 1;
        for method in methods {
            self.write_indent();
            let _ = write!(self.out, "fn {}(", method.name);
            for (index, param) in method.params.iter().enumerate() {
                if index > 0 {
                    self.out.push_str(", ");
                }
                let _ = write!(self.out, "{}: {}", param.name, format_type_ref(&param.ty));
            }
            let _ = writeln!(self.out, ") -> {};", format_type_ref(&method.return_type));
        }
        self.indent -= 1;
        self.write_indent();
        self.out.push('}');
    }

    pub(in crate::formatter) fn format_method_item(&mut self, method: &ImplMethod) {
        let _ = write!(
            self.out,
            "fn {}{}(",
            method.name,
            format_type_params(&method.type_params)
        );
        for (index, param) in method.params.iter().enumerate() {
            if index > 0 {
                self.out.push_str(", ");
            }
            let _ = write!(self.out, "{}: {}", param.name, format_type_ref(&param.ty));
        }
        let _ = write!(self.out, ") -> {} ", format_type_ref(&method.return_type));
        self.format_block(&method.body);
    }

    pub(in crate::formatter) fn format_enum_item(
        &mut self,
        name: &str,
        type_params: &[String],
        variants: &[crate::ast::EnumVariant],
    ) {
        let params = format_type_params(type_params);
        if variants.is_empty() {
            let _ = write!(self.out, "enum {name}{params} {{}}");
            return;
        }

        let _ = writeln!(self.out, "enum {name}{params} {{");
        self.indent += 1;
        for variant in variants {
            self.write_indent();
            if let Some(payload) = &variant.payload {
                let _ = writeln!(self.out, "{}({}),", variant.name, format_type_ref(payload));
            } else {
                let _ = writeln!(self.out, "{},", variant.name);
            }
        }
        self.indent -= 1;
        self.write_indent();
        self.out.push('}');
    }
}
