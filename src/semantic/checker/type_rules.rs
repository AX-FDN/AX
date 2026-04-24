use crate::diagnostics::Diagnostic;
use crate::source::Span;

use super::{type_mismatch_suggestion, Type, TypeChecker};

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
