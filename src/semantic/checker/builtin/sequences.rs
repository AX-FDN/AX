use super::*;

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_len_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, "len", 1, argument_types.len()) {
            return Type::Error;
        }

        match &argument_types[0] {
            Type::String | Type::StringList | Type::Array { .. } | Type::Slice { .. } => Type::I32,
            actual => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0022",
                        format!(
                            "function `len` expects argument `value` to be `string`, `string_list`, array, or slice, found `{}`",
                            actual.describe()
                        ),
                        self.info.source,
                        expr.span,
                    )
                    .with_kind(DiagnosticKind::LenBuiltinTypeMismatch)
                    .with_note(
                        "`len` is the general traversal-length builtin for strings, string lists, fixed-size arrays, and slices",
                    )
                    .with_suggestion(
                        "call `len` with a string, string list, array, or slice value like `len(values)`",
                    ),
                );
                Type::Error
            }
        }
    }
}
