use std::collections::HashSet;

use crate::diagnostics::Diagnostic;
use crate::source::Span;

use super::{Type, TypeChecker};

#[derive(Debug, Clone)]
pub(super) struct Binding {
    pub(super) mutable: bool,
    pub(super) ty: Type,
    pub(super) start: usize,
}

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(crate) fn declare(&mut self, name: &str, ty: Type, mutable: bool, start: usize) {
        let current_scope = self.scopes.last_mut().expect("scope must exist");
        if let Some(previous) =
            current_scope.insert(name.to_string(), Binding { mutable, ty, start })
        {
            let (line, column) = self.info.source.line_col(previous.start);
            self.diagnostics.push(
                Diagnostic::new(
                    "S0001",
                    format!("duplicate definition of `{name}`"),
                    self.info.source,
                    Span::new(start, start + name.len()),
                )
                .with_note(format!("previous definition was at {line}:{column}")),
            );
        }
    }

    pub(super) fn lookup(&self, name: &str) -> Option<Binding> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    pub(super) fn undefined_variable_diagnostic(
        &self,
        name: &str,
        span: Span,
        suggestion: String,
    ) -> Diagnostic {
        let mut diagnostic = Diagnostic::new(
            "S0002",
            format!("use of undefined variable `{name}`"),
            self.info.source,
            span,
        )
        .with_note("AX variables are block-scoped and must be declared before use")
        .with_suggestion(suggestion);

        let visible = self.visible_binding_names();
        if !visible.is_empty() {
            diagnostic =
                diagnostic.with_note(format!("visible variables here: {}", visible.join(", ")));
        }

        diagnostic
    }

    fn visible_binding_names(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut names = Vec::new();

        for scope in self.scopes.iter().rev() {
            let mut scope_names = scope.keys().cloned().collect::<Vec<_>>();
            scope_names.sort();
            for name in scope_names {
                if seen.insert(name.clone()) {
                    names.push(name);
                }
            }
        }

        names
    }
}
