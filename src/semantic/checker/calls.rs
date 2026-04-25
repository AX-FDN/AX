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
}
