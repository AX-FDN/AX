use crate::diagnostics::Diagnostic;
use crate::source::Span;

use super::{Type, TypeChecker, type_mismatch_suggestion};

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn expect_type_match(
        &mut self,
        expected: &Type,
        actual: &Type,
        span: Span,
        message: String,
    ) {
        if expected.is_error() || actual.is_error() || actual.is_assignable_to(expected) {
            return;
        }

        if matches!(actual, Type::EmptyArrayLiteral) {
            let suggestion = match expected {
                Type::Array { .. } => {
                    "use `[]` only where the expected type is a zero-length array like `[i32; 0]`"
                }
                _ => {
                    "give the empty array a zero-length array context like `[i32; 0]` before using `[]`"
                }
            };

            self.diagnostics.push(
                Diagnostic::new(
                    "S0032",
                    format!(
                        "empty array literal `[]` requires a zero-length array context, found expected `{}`",
                        expected.describe()
                    ),
                    self.info.source,
                    span,
                )
                .with_note("AX can only type-check `[]` when the surrounding context fixes it to a length-0 array")
                .with_suggestion(suggestion),
            );
            return;
        }

        self.diagnostics.push(
            Diagnostic::new("S0022", message, self.info.source, span)
                .with_note(format!(
                    "AX does not implicitly convert `{}` to `{}`",
                    actual.describe(),
                    expected.describe()
                ))
                .with_suggestion(type_mismatch_suggestion(expected, actual)),
        );
    }
}
