use crate::source::Span;

pub(super) struct ParsedIntLiteral {
    pub(super) value: i64,
    pub(super) span: Span,
}

pub(super) fn parse_signed_i64_literal(lexeme: &str, negative: bool) -> i64 {
    let value = lexeme.parse::<i64>().unwrap_or(0);
    if negative { -value } else { value }
}
