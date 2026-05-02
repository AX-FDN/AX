use super::*;

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_to_string_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, "to_string", 1, argument_types.len()) {
            return Type::Error;
        }

        match &argument_types[0] {
            Type::Error => Type::Error,
            Type::Void => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0022",
                        "function `to_string` expects argument `value` to be a concrete runtime value, found `<void>`",
                        self.info.source,
                        expr.span,
                    )
                    .with_note(
                        "`to_string` formats an existing runtime value; `println(...)` does not produce one",
                    )
                    .with_suggestion(
                        "call `to_string` on a string, number, bool, enum, struct, array, or slice value",
                    ),
                );
                Type::Error
            }
            Type::EmptyArrayLiteral => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0022",
                        "function `to_string` expects argument `value` to have a concrete runtime type, found `[]`",
                        self.info.source,
                        expr.span,
                    )
                    .with_note(
                        "an empty array literal must first be placed in an explicit zero-length array context",
                    )
                    .with_suggestion(
                        "bind `[]` as something like `[i32; 0]` before converting it with `to_string`",
                    ),
                );
                Type::Error
            }
            _ => Type::String,
        }
    }
}
