use crate::ast::Expr;
use crate::diagnostics::{Diagnostic, DiagnosticKind};

use super::{Type, TypeChecker};

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_call_expr(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        arguments: &[Expr],
    ) -> Type {
        let Some(callee_name) = callee.qualified_name() else {
            self.check_expr(callee);
            for argument in arguments {
                self.check_expr(argument);
            }
            self.diagnostics.push(
                Diagnostic::new(
                    "S0019",
                    "call target must be a function name",
                    self.info.source,
                    callee.span,
                )
                .with_suggestion("use a direct function call like `name(arg1, arg2)`"),
            );
            return Type::Error;
        };

        if !callee_name.contains('.')
            && let Some(result) = self.check_builtin_call(expr, &callee_name, arguments)
        {
            return result;
        }

        if let Some(result) =
            self.check_enum_variant_constructor_call(expr, &callee_name, arguments)
        {
            return result;
        }

        let current_unit_path = self.current_unit_path().to_string();
        let resolved_name = self.info.resolve_function_key(
            &callee_name,
            &current_unit_path,
            callee.span,
            self.diagnostics,
        );
        let signature = resolved_name
            .as_ref()
            .and_then(|name| self.info.functions.get(name))
            .cloned();
        let argument_types = arguments
            .iter()
            .map(|argument| self.check_expr(argument))
            .collect::<Vec<_>>();

        match signature {
            Some(signature) => {
                if signature.params.len() != argument_types.len() {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `{callee_name}` expects {} argument(s), found {}",
                            signature.params.len(),
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                }

                for (argument, parameter) in argument_types.iter().zip(signature.params.iter()) {
                    self.expect_type_match_with_kind(
                        &parameter.ty,
                        argument,
                        expr.span,
                        format!(
                            "function `{callee_name}` expects argument `{}` to be `{}`, found `{}`",
                            parameter.name,
                            parameter.ty.describe(),
                            argument.describe()
                        ),
                        DiagnosticKind::FunctionArgumentTypeMismatch,
                    );
                }

                signature.return_type
            }
            None if !callee_name.contains('.') && self.lookup(&callee_name).is_some() => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0018",
                        format!("variable `{callee_name}` is not callable"),
                        self.info.source,
                        callee.span,
                    )
                    .with_suggestion("only function names and builtin functions can be called"),
                );
                Type::Error
            }
            None => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0007",
                        format!("call to undefined function `{callee_name}`"),
                        self.info.source,
                        callee.span,
                    )
                    .with_suggestion(format!("declare `{callee_name}` or fix the call target")),
                );
                Type::Error
            }
        }
    }

    fn check_enum_variant_constructor_call(
        &mut self,
        expr: &Expr,
        callee_name: &str,
        arguments: &[Expr],
    ) -> Option<Type> {
        let (enum_path, variant) = callee_name.rsplit_once('.')?;
        let current_unit_path = self.current_unit_path().to_string();
        let local_enum_candidate = self
            .info
            .unit_context(&current_unit_path)
            .and_then(|unit| unit.module_path.as_ref())
            .map(|module_path| format!("{module_path}.{enum_path}"));
        let has_local_enum_candidate = local_enum_candidate
            .as_ref()
            .map(|candidate| self.info.named_types.contains_key(candidate))
            .unwrap_or(false);
        if !self.info.named_types.contains_key(enum_path) && !has_local_enum_candidate {
            return None;
        }
        let resolved_enum_name = self.info.resolve_named_type_key(
            enum_path,
            &current_unit_path,
            expr.span,
            self.diagnostics,
        )?;
        let Some(enum_info) = self.info.enums.get(&resolved_enum_name).cloned() else {
            return None;
        };
        let Some(variant_info) = enum_info.variants.get(variant).cloned() else {
            return None;
        };

        let argument_types = arguments
            .iter()
            .map(|argument| self.check_expr(argument))
            .collect::<Vec<_>>();

        match variant_info.payload {
            Some(payload_type) => {
                if argument_types.len() != 1 {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0053",
                            format!(
                                "enum variant `{callee_name}` expects exactly 1 payload argument, found {}",
                                argument_types.len()
                            ),
                            self.info.source,
                            expr.span,
                        )
                        .with_kind(DiagnosticKind::EnumVariantPayloadShapeMismatch)
                        .with_note(
                            "payload enum variants are constructed as `EnumName.Variant(value)` in the current AX slice",
                        )
                        .with_suggestion("pass exactly one payload value that matches the declared payload type"),
                    );
                    return Some(Type::Error);
                }

                self.expect_type_match_with_kind(
                    &payload_type,
                    &argument_types[0],
                    arguments[0].span,
                    format!(
                        "enum variant `{callee_name}` expects payload type `{}`, found `{}`",
                        payload_type.describe(),
                        argument_types[0].describe()
                    ),
                    DiagnosticKind::EnumVariantPayloadTypeMismatch,
                );

                Some(Type::Enum(resolved_enum_name))
            }
            None => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0053",
                        format!("enum variant `{callee_name}` does not accept payload arguments"),
                        self.info.source,
                        expr.span,
                    )
                    .with_kind(DiagnosticKind::EnumVariantPayloadShapeMismatch)
                    .with_note(
                        "unit enum variants are already complete values, so they are written without `(...)`",
                    )
                    .with_suggestion(format!("drop the call syntax and use `{callee_name}` directly")),
                );
                Some(Type::Error)
            }
        }
    }
}
