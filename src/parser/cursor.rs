use super::diagnostics::{enrich_parse_error, parse_error_kind};
use super::*;

impl<'a> Parser<'a> {
    pub(super) fn sync_to_item(&mut self, segment_end: usize) {
        while !self.is_at_end() && self.peek().span.start < segment_end {
            match self.peek().kind {
                TokenKind::FnKw
                | TokenKind::StructKw
                | TokenKind::EnumKw
                | TokenKind::ModuleKw
                | TokenKind::ImportKw => break,
                _ => {
                    self.advance();
                }
            }
        }
    }

    pub(super) fn sync_to_statement_boundary(&mut self) {
        while !self.is_at_end() {
            if self.previous_kind() == Some(TokenKind::Semicolon) {
                break;
            }
            match self.peek().kind {
                TokenKind::LetKw
                | TokenKind::ReturnKw
                | TokenKind::BreakKw
                | TokenKind::MatchKw
                | TokenKind::IfKw
                | TokenKind::WhileKw
                | TokenKind::ForKw
                | TokenKind::RBrace => break,
                _ => {
                    self.advance();
                }
            }
        }
    }

    pub(super) fn expect(&mut self, kind: TokenKind, message: &str, expected: &[&str]) -> Token {
        if self.check(kind) {
            self.advance()
        } else {
            self.error_at_current("P0001", message, expected);
            self.advance()
        }
    }

    pub(super) fn expect_identifier(&mut self, message: &str) -> Token {
        if self.check(TokenKind::Identifier) {
            self.advance()
        } else {
            self.error_at_current("P0002", message, &["identifier"]);
            self.advance()
        }
    }

    pub(super) fn error_at_current(&mut self, code: &str, message: &str, expected: &[&str]) {
        let token = self.peek().clone();
        let span = token.span;
        let mut diagnostic = Diagnostic::new(code, message, self.source, span);
        for entry in expected {
            diagnostic = diagnostic.with_expected(*entry);
        }
        if let Some(kind) = parse_error_kind(code, message, expected) {
            diagnostic = diagnostic.with_kind(kind);
        }
        diagnostic = enrich_parse_error(diagnostic, &token, message);
        self.diagnostics.push(diagnostic);
    }

    pub(super) fn matches(&mut self, kinds: &[TokenKind]) -> bool {
        if kinds.iter().any(|kind| self.check(*kind)) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub(super) fn check(&self, kind: TokenKind) -> bool {
        !self.is_at_end() && self.peek().kind == kind
            || (kind == TokenKind::Eof && self.peek().kind == TokenKind::Eof)
    }

    pub(super) fn advance(&mut self) -> Token {
        let token = self.tokens[self.current].clone();
        if !self.is_at_end() {
            self.current += 1;
        }
        token
    }

    pub(super) fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    pub(super) fn previous_kind(&self) -> Option<TokenKind> {
        self.current
            .checked_sub(1)
            .map(|index| self.tokens[index].kind)
    }

    pub(super) fn previous(&self) -> &Token {
        let index = self.current.saturating_sub(1);
        &self.tokens[index]
    }

    pub(super) fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    pub(super) fn token_in_span(&self, span: Span) -> bool {
        !self.is_at_end() && self.peek().span.start < span.end
    }

    pub(super) fn parse_qualified_identifier_path(
        &mut self,
        expected_path_message: &str,
        expected_segment_message: &str,
    ) -> (String, Span) {
        let first = self.expect_identifier(expected_path_message);
        self.finish_qualified_identifier_path(first, expected_segment_message)
    }

    pub(super) fn finish_qualified_identifier_path(
        &mut self,
        first: Token,
        expected_segment_message: &str,
    ) -> (String, Span) {
        let mut path = first.lexeme;
        let mut end = first.span.end;

        while self.matches(&[TokenKind::Dot]) {
            let segment = self.expect_identifier(expected_segment_message);
            path.push('.');
            path.push_str(&segment.lexeme);
            end = segment.span.end;
        }

        (path, Span::new(first.span.start, end))
    }

    pub(super) fn qualified_path_followed_by(&self, terminator: TokenKind) -> bool {
        let mut index = self.current;
        let mut saw_dot = false;

        while self.tokens.get(index).map(|token| token.kind) == Some(TokenKind::Dot) {
            saw_dot = true;
            index += 1;
            if self.tokens.get(index).map(|token| token.kind) != Some(TokenKind::Identifier) {
                return false;
            }
            index += 1;
        }

        saw_dot && self.tokens.get(index).map(|token| token.kind) == Some(terminator)
    }
}
