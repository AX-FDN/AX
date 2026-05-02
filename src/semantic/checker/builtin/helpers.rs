use super::*;

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_builtin_argument_types(&mut self, arguments: &[Expr]) -> Vec<Type> {
        arguments
            .iter()
            .map(|argument| self.check_expr(argument))
            .collect::<Vec<_>>()
    }

    pub(super) fn check_builtin_arity(
        &mut self,
        expr: &Expr,
        name: &str,
        expected: usize,
        actual: usize,
    ) -> bool {
        if actual == expected {
            return true;
        }

        self.diagnostics.push(Diagnostic::new(
            "S0017",
            format!("function `{name}` expects {expected} argument(s), found {actual}"),
            self.info.source,
            expr.span,
        ));
        false
    }

    pub(super) fn check_builtin_no_arguments(
        &mut self,
        expr: &Expr,
        name: &str,
        arguments: &[Expr],
    ) -> bool {
        for argument in arguments {
            self.check_expr(argument);
        }

        self.check_builtin_arity(expr, name, 0, arguments.len())
    }

    pub(super) fn expect_builtin_argument_type(
        &mut self,
        expr: &Expr,
        name: &str,
        argument_name: &str,
        expected: &Type,
        actual: &Type,
    ) {
        self.expect_type_match(
            expected,
            actual,
            expr.span,
            format!(
                "function `{name}` expects argument `{argument_name}` to be `{}`, found `{}`",
                expected.describe(),
                actual.describe()
            ),
        );
    }
}
