use crate::ast::{Expr, ExprKind};
use crate::diagnostics::Diagnostic;

use super::{Type, TypeChecker};

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_call_expr(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        arguments: &[Expr],
    ) -> Type {
        match &callee.kind {
            ExprKind::Name { value } if value == "println" => {
                for argument in arguments {
                    self.check_expr(argument);
                }
                Type::Void
            }
            ExprKind::Name { value } => {
                let signature = self.info.functions.get(value).cloned();
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
                                    "function `{value}` expects {} argument(s), found {}",
                                    signature.params.len(),
                                    argument_types.len()
                                ),
                                self.info.source,
                                expr.span,
                            ));
                        }

                        for (argument, parameter) in
                            argument_types.iter().zip(signature.params.iter())
                        {
                            self.expect_type_match(
                                &parameter.ty,
                                argument,
                                expr.span,
                                format!(
                                    "function `{value}` expects argument `{}` to be `{}`, found `{}`",
                                    parameter.name,
                                    parameter.ty.describe(),
                                    argument.describe()
                                ),
                            );
                        }

                        signature.return_type
                    }
                    None if self.lookup(value).is_some() => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0018",
                                format!("variable `{value}` is not callable"),
                                self.info.source,
                                callee.span,
                            )
                            .with_suggestion(
                                "only function names and builtin functions can be called",
                            ),
                        );
                        Type::Error
                    }
                    None => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0007",
                                format!("call to undefined function `{value}`"),
                                self.info.source,
                                callee.span,
                            )
                            .with_suggestion(format!("declare `{value}` or fix the call target")),
                        );
                        Type::Error
                    }
                }
            }
            _ => {
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
                Type::Error
            }
        }
    }
}
