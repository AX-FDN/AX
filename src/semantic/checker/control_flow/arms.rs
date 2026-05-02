use super::helpers::{substitute_enum_payload_type, substitute_struct_field_type};
use super::*;

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_match_arm_block(
        &mut self,
        scrutinee_type: &Type,
        pattern: &MatchPattern,
        body: &Block,
    ) {
        self.scopes.push(HashMap::new());
        self.declare_match_binding(scrutinee_type, pattern);
        self.check_block(body);
        self.scopes.pop();
    }

    pub(super) fn check_match_arm_guard(
        &mut self,
        scrutinee_type: &Type,
        pattern: &MatchPattern,
        guard: Option<&Expr>,
    ) {
        let Some(guard) = guard else {
            return;
        };

        self.scopes.push(HashMap::new());
        self.declare_match_binding(scrutinee_type, pattern);
        let guard_type = self.check_expr(guard);
        self.expect_type_match_with_kind(
            &Type::Bool,
            &guard_type,
            guard.span,
            format!(
                "match guard must be `bool`, found `{}`",
                guard_type.describe()
            ),
            DiagnosticKind::MatchGuardTypeMismatch,
        );
        self.scopes.pop();
    }

    pub(super) fn check_match_expression_arm(
        &mut self,
        scrutinee_type: &Type,
        pattern: &MatchPattern,
        value: &Expr,
        expected_type: Option<&Type>,
    ) -> Type {
        self.scopes.push(HashMap::new());
        self.declare_match_binding(scrutinee_type, pattern);
        let ty = if let Some(expected_type) = expected_type {
            self.check_expr_with_expected(value, expected_type)
        } else {
            self.check_expr(value)
        };
        self.scopes.pop();
        ty
    }

    fn declare_match_binding(&mut self, scrutinee_type: &Type, pattern: &MatchPattern) {
        match &pattern.kind {
            MatchPatternKind::Binding { name } => {
                self.declare(name, scrutinee_type.clone(), false, pattern.span.start);
            }
            MatchPatternKind::EnumVariant {
                path,
                payload: Some(EnumVariantPayloadPattern::Binding { name }),
            } => {
                if let Some(payload_type) =
                    self.resolve_enum_pattern_payload_type(path, scrutinee_type, pattern.span)
                {
                    self.declare(name, payload_type, false, pattern.span.start);
                }
            }
            MatchPatternKind::Struct { path, fields } => {
                let struct_name = self.struct_pattern_name(path, pattern.span);
                for field in fields {
                    if let Some(field_type) = struct_name.as_deref().and_then(|name| {
                        self.resolve_struct_pattern_field_type(name, &field.name, scrutinee_type)
                    }) {
                        self.declare(&field.binding, field_type, false, field.span.start);
                    }
                }
            }
            MatchPatternKind::Or { .. } => {}
            _ => {}
        }
    }

    fn struct_pattern_name(&mut self, path: &str, span: Span) -> Option<String> {
        let current_unit_path = self.current_unit_path().to_string();
        let resolved_key =
            self.info
                .resolve_named_type_key(path, &current_unit_path, span, self.diagnostics)?;
        let Type::Struct(struct_name) = self.info.named_types.get(&resolved_key).cloned()? else {
            return None;
        };
        Some(struct_name)
    }

    fn resolve_struct_pattern_field_type(
        &self,
        struct_name: &str,
        field_name: &str,
        scrutinee_type: &Type,
    ) -> Option<Type> {
        let struct_info = self.info.structs.get(struct_name)?;
        let field_type = struct_info.fields.get(field_name)?.ty.clone();
        Some(substitute_struct_field_type(
            field_type,
            struct_info.type_params.as_slice(),
            scrutinee_type,
        ))
    }

    fn resolve_enum_pattern_payload_type(
        &mut self,
        path: &str,
        scrutinee_type: &Type,
        span: Span,
    ) -> Option<Type> {
        let (enum_path, variant) = path.rsplit_once('.')?;
        let current_unit_path = self.current_unit_path().to_string();
        let resolved_key = self.info.resolve_named_type_key(
            enum_path,
            &current_unit_path,
            span,
            self.diagnostics,
        )?;
        let Type::Enum(enum_name) = self.info.named_types.get(&resolved_key).cloned()? else {
            return None;
        };
        let enum_info = self.info.enums.get(&enum_name)?;
        let payload_type = enum_info.variants.get(variant)?.payload.clone()?;
        Some(substitute_enum_payload_type(
            payload_type,
            enum_info.type_params.as_slice(),
            scrutinee_type,
        ))
    }
}
