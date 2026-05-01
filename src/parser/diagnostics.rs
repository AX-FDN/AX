use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::token::{Token, TokenKind};

pub(super) fn parse_error_kind(
    code: &str,
    message: &str,
    expected: &[&str],
) -> Option<DiagnosticKind> {
    match code {
        "P0001" => {
            if message == "expected a top-level declaration" {
                return Some(DiagnosticKind::TopLevelDeclarationRequired);
            }

            if expected.contains(&"`;`") {
                return Some(DiagnosticKind::MissingSemicolon);
            }

            if expected.contains(&"`)`") {
                return Some(DiagnosticKind::MissingRightParen);
            }

            if expected.contains(&"`]`") {
                return Some(DiagnosticKind::MissingRightBracket);
            }

            if expected.contains(&"`}`") {
                return Some(DiagnosticKind::MissingRightBrace);
            }

            None
        }
        "P0002" if message == "expected a type name" => Some(DiagnosticKind::TypeNameRequired),
        "P0003" if message == "expected an expression" => Some(DiagnosticKind::ExpressionRequired),
        _ => None,
    }
}

pub(super) fn enrich_parse_error(
    mut diagnostic: Diagnostic,
    token: &Token,
    message: &str,
) -> Diagnostic {
    diagnostic = diagnostic.with_note(format!("found {} instead", describe_token(token)));

    if let Some(kind) = diagnostic.kind() {
        match kind {
            DiagnosticKind::MissingSemicolon => {
                return diagnostic
                    .with_note(
                        "AX uses explicit semicolons after `let`, assignments, expression statements, and `return`.",
                    )
                    .with_suggestion("insert `;` before the next statement or closing `}`");
            }
            DiagnosticKind::MissingRightParen => {
                return diagnostic
                    .with_note(
                        "this usually means a condition, grouped expression, call, or `for` header was left open",
                    )
                    .with_suggestion("insert `)` to close the current parenthesized construct");
            }
            DiagnosticKind::MissingRightBracket => {
                return diagnostic
                    .with_note(
                        "AX closes array literals, slice types, array types, index expressions, and slice expressions with `]`",
                    )
                    .with_suggestion("insert `]` to close the current bracketed construct");
            }
            DiagnosticKind::MissingRightBrace => {
                return diagnostic
                    .with_note("AX closes blocks and struct literals with `}`")
                    .with_suggestion("insert `}` to close the current block or literal");
            }
            DiagnosticKind::TopLevelDeclarationRequired => {
                return diagnostic.with_suggestion(
                    "start a top-level item with `pub`, `fn`, `const`, `struct`, `enum`, `trait`, or `impl`",
                );
            }
            DiagnosticKind::TypeNameRequired => {
                return diagnostic.with_suggestion(
                    "use `bool`, `i32`, `f32`, `string`, `[Type]`, `[Type; N]`, or a previously declared type name",
                );
            }
            DiagnosticKind::ExpressionRequired => {
                return diagnostic.with_suggestion(
                    "insert a runtime expression such as a literal, array literal, name, call, or parenthesized expression",
                );
            }
            _ => {}
        }
    }

    if message.contains("expected `;`") {
        return diagnostic
            .with_note("AX uses explicit semicolons after `let`, assignments, expression statements, and `return`.")
            .with_suggestion("insert `;` before the next statement or closing `}`");
    }

    if message.contains("expected `)`") {
        return diagnostic
            .with_note("this usually means a condition, grouped expression, call, or `for` header was left open")
            .with_suggestion("insert `)` to close the current parenthesized construct");
    }

    if message.contains("expected `]`") {
        return diagnostic
            .with_note(
                "AX closes array literals, slice types, array types, index expressions, and slice expressions with `]`",
            )
            .with_suggestion("insert `]` to close the current bracketed construct");
    }

    if message.contains("expected `}`") {
        return diagnostic
            .with_note("AX closes blocks and struct literals with `}`")
            .with_suggestion("insert `}` to close the current block or literal");
    }

    if message == "expected a top-level declaration" {
        return diagnostic.with_suggestion(
            "start a top-level item with `pub`, `fn`, `const`, `type`, `struct`, `enum`, `trait`, or `impl`",
        );
    }

    if message == "expected a type name" {
        return diagnostic.with_suggestion(
            "use `bool`, `i32`, `f32`, `string`, `[Type]`, `[Type; N]`, or a previously declared type name",
        );
    }

    if message == "expected an expression" {
        return diagnostic.with_suggestion(
            "insert a runtime expression such as a literal, array literal, name, call, or parenthesized expression",
        );
    }

    diagnostic
}

fn describe_token(token: &Token) -> String {
    match token.kind {
        TokenKind::Eof => "the end of file".to_string(),
        TokenKind::Semicolon => "`;`".to_string(),
        TokenKind::RParen => "`)`".to_string(),
        TokenKind::LBracket => "`[`".to_string(),
        TokenKind::RBracket => "`]`".to_string(),
        TokenKind::RBrace => "`}`".to_string(),
        TokenKind::LParen => "`(`".to_string(),
        TokenKind::LBrace => "`{`".to_string(),
        TokenKind::Identifier => format!("identifier `{}`", token.lexeme),
        TokenKind::IntLiteral => format!("integer literal `{}`", token.lexeme),
        TokenKind::FloatLiteral => format!("float literal `{}`", token.lexeme),
        TokenKind::StringLiteral => format!("string literal \"{}\"", token.lexeme),
        _ if token.lexeme.is_empty() => format!("token `{:?}`", token.kind),
        _ => format!("token `{}`", token.lexeme),
    }
}
