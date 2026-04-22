use std::fmt::Write;

use serde::Serialize;

use crate::source::{SourceFile, Span};

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub file: String,
    pub span: Span,
    pub notes: Vec<String>,
    pub expected: Vec<String>,
    pub suggestion: Option<String>,
}

impl Diagnostic {
    pub fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        source: &SourceFile,
        span: Span,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            file: source.display_path(),
            span,
            notes: Vec::new(),
            expected: Vec::new(),
            suggestion: None,
        }
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn with_expected(mut self, expected: impl Into<String>) -> Self {
        self.expected.push(expected.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

pub fn render_diagnostics(source: &SourceFile, diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| render_diagnostic(source, diagnostic))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn render_diagnostic(source: &SourceFile, diagnostic: &Diagnostic) -> String {
    let (line, column) = source.line_col(diagnostic.span.start);
    let line_text = source.line_text(line);
    let underline_width = diagnostic
        .span
        .end
        .saturating_sub(diagnostic.span.start)
        .max(1)
        .min(
            line_text
                .len()
                .saturating_sub(column.saturating_sub(1))
                .max(1),
        );

    let mut out = String::new();
    let _ = writeln!(out, "{}: {}", diagnostic.code, diagnostic.message);
    let _ = writeln!(out, " --> {}:{}:{}", diagnostic.file, line, column);
    let _ = writeln!(out, "  |");
    let _ = writeln!(out, "{line:>2} | {line_text}");
    let _ = writeln!(
        out,
        "  | {}{}",
        " ".repeat(column.saturating_sub(1)),
        "^".repeat(underline_width)
    );

    if !diagnostic.expected.is_empty() {
        let _ = writeln!(out, "  = expected: {}", diagnostic.expected.join(", "));
    }

    for note in &diagnostic.notes {
        let _ = writeln!(out, "  = note: {note}");
    }

    if let Some(suggestion) = &diagnostic.suggestion {
        let _ = writeln!(out, "  = help: {suggestion}");
    }

    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::{Diagnostic, render_diagnostics};
    use crate::source::{SourceFile, Span};

    #[test]
    fn renders_span_and_message() {
        let source = SourceFile::anonymous("let value: i32 = 1;\n");
        let diagnostic =
            Diagnostic::new("P0001", "expected `;`", &source, Span::new(4, 9)).with_expected("`;`");
        let rendered = render_diagnostics(&source, &[diagnostic]);
        assert!(rendered.contains("P0001"));
        assert!(rendered.contains("--> <memory>:1:5"));
        assert!(rendered.contains("expected: `;`"));
    }
}
