use crate::ast::Expr;
use crate::diagnostics::Diagnostic;

use super::{Type, TypeChecker};

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_builtin_call(
        &mut self,
        expr: &Expr,
        name: &str,
        arguments: &[Expr],
    ) -> Option<Type> {
        match name {
            "println" => {
                for argument in arguments {
                    self.check_expr(argument);
                }
                Some(Type::Void)
            }
            "string_len" => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();

                if argument_types.len() != 1 {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `string_len` expects 1 argument(s), found {}",
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                    return Some(Type::Error);
                }

                self.expect_type_match(
                    &Type::String,
                    &argument_types[0],
                    expr.span,
                    format!(
                        "function `string_len` expects argument `text` to be `string`, found `{}`",
                        argument_types[0].describe()
                    ),
                );
                Some(Type::I32)
            }
            _ => None,
        }
    }
}
