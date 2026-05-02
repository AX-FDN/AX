use super::*;

impl<'a> LoweringContext<'a> {
    pub(in crate::hir) fn lowering_error(
        &self,
        code: &str,
        message: impl Into<String>,
        span: Span,
    ) -> Diagnostic {
        Diagnostic::new(code, message.into(), self.source, span)
    }

    pub(in crate::hir) fn fresh_match_temp_name(&self) -> String {
        let next = self.next_match_temp.get();
        self.next_match_temp.set(next + 1);
        format!("__match_scrutinee_{next}")
    }

    pub(in crate::hir) fn fresh_for_in_temp_name(&self, suffix: &str) -> String {
        let next = self.next_for_in_temp.get();
        self.next_for_in_temp.set(next + 1);
        format!("__for_in_{suffix}_{next}")
    }

    pub(in crate::hir) fn canonical_name(&self, name: &str, span: Span) -> String {
        self.resolve_same_unit_name(name, span)
            .unwrap_or_else(|| name.to_string())
    }

    pub(in crate::hir) fn resolve_function_name(&self, name: &str, span: Span) -> String {
        self.resolve_canonical_name(name, span, &self.function_names)
            .unwrap_or_else(|| name.to_string())
    }

    pub(in crate::hir) fn impl_method_prefix(
        &self,
        target: &ast::TypeRef,
        span: Span,
    ) -> Result<String, Diagnostic> {
        let Some(name) = target.direct_name() else {
            return Err(self.lowering_error("H0017", "impl target must be a named type", span));
        };
        self.resolve_canonical_name(name, span, &self.struct_names)
            .or_else(|| self.resolve_canonical_name(name, span, &self.enum_names))
            .ok_or_else(|| self.lowering_error("H0017", "impl target must resolve to a type", span))
    }

    pub(in crate::hir) fn field_base_names_type(&self, base: &ast::Expr) -> bool {
        base.qualified_name()
            .and_then(|name| {
                self.resolve_canonical_name(&name, base.span, &self.struct_names)
                    .or_else(|| self.resolve_canonical_name(&name, base.span, &self.enum_names))
            })
            .is_some()
    }

    pub(in crate::hir) fn resolve_canonical_name(
        &self,
        name: &str,
        span: Span,
        known_names: &HashSet<String>,
    ) -> Option<String> {
        if known_names.contains(name) {
            return Some(name.to_string());
        }

        let local_name = self.resolve_same_unit_name(name, span)?;
        known_names.contains(&local_name).then_some(local_name)
    }

    pub(in crate::hir) fn resolve_same_unit_name(&self, name: &str, span: Span) -> Option<String> {
        if name.contains('.') {
            return None;
        }

        let unit_path = self.source.display_path_for_offset(span.start);
        let module_path = self.unit_modules.get(unit_path)?;
        Some(format!("{module_path}.{name}"))
    }
}
