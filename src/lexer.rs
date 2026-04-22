use crate::diagnostics::Diagnostic;
use crate::source::{SourceFile, Span};
use crate::token::{Token, TokenKind};

pub struct LexerOutput {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn tokenize(source: &SourceFile) -> LexerOutput {
    Lexer::new(source).lex()
}

struct Lexer<'a> {
    source: &'a SourceFile,
    cursor: usize,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a SourceFile) -> Self {
        Self {
            source,
            cursor: 0,
            tokens: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn lex(mut self) -> LexerOutput {
        while let Some(ch) = self.peek_char() {
            let start = self.cursor;
            match ch {
                ' ' | '\t' | '\r' | '\n' => {
                    self.advance_char();
                }
                '/' if self.peek_next_char() == Some('/') => {
                    self.skip_comment();
                }
                '(' => self.simple_token(TokenKind::LParen, start),
                ')' => self.simple_token(TokenKind::RParen, start),
                '{' => self.simple_token(TokenKind::LBrace, start),
                '}' => self.simple_token(TokenKind::RBrace, start),
                ',' => self.simple_token(TokenKind::Comma, start),
                '.' => self.simple_token(TokenKind::Dot, start),
                ':' => self.simple_token(TokenKind::Colon, start),
                ';' => self.simple_token(TokenKind::Semicolon, start),
                '+' => self.simple_token(TokenKind::Plus, start),
                '*' => self.simple_token(TokenKind::Star, start),
                '-' if self.peek_next_char() == Some('>') => {
                    self.advance_char();
                    self.advance_char();
                    self.push_token(TokenKind::Arrow, Span::new(start, self.cursor));
                }
                '-' => self.simple_token(TokenKind::Minus, start),
                '!' if self.peek_next_char() == Some('=') => {
                    self.advance_char();
                    self.advance_char();
                    self.push_token(TokenKind::BangEqual, Span::new(start, self.cursor));
                }
                '!' => self.simple_token(TokenKind::Bang, start),
                '=' if self.peek_next_char() == Some('=') => {
                    self.advance_char();
                    self.advance_char();
                    self.push_token(TokenKind::EqualEqual, Span::new(start, self.cursor));
                }
                '=' => self.simple_token(TokenKind::Equal, start),
                '<' if self.peek_next_char() == Some('=') => {
                    self.advance_char();
                    self.advance_char();
                    self.push_token(TokenKind::LessEqual, Span::new(start, self.cursor));
                }
                '<' => self.simple_token(TokenKind::Less, start),
                '>' if self.peek_next_char() == Some('=') => {
                    self.advance_char();
                    self.advance_char();
                    self.push_token(TokenKind::GreaterEqual, Span::new(start, self.cursor));
                }
                '>' => self.simple_token(TokenKind::Greater, start),
                '"' => self.lex_string(start),
                c if c.is_ascii_digit() => self.lex_number(start),
                c if is_ident_start(c) => self.lex_identifier(start),
                _ => {
                    let span = Span::new(start, start + ch.len_utf8());
                    self.diagnostics.push(
                        Diagnostic::new(
                            "L0001",
                            format!("unexpected character `{ch}`"),
                            self.source,
                            span,
                        )
                        .with_suggestion("remove the character or replace it with valid AX syntax"),
                    );
                    self.advance_char();
                }
            }
        }

        let eof = Span::new(self.cursor, self.cursor);
        self.tokens.push(Token::new(TokenKind::Eof, eof, ""));
        LexerOutput {
            tokens: self.tokens,
            diagnostics: self.diagnostics,
        }
    }

    fn simple_token(&mut self, kind: TokenKind, start: usize) {
        self.advance_char();
        self.push_token(kind, Span::new(start, self.cursor));
    }

    fn push_token(&mut self, kind: TokenKind, span: Span) {
        self.tokens.push(Token::new(kind, span, self.source.slice(span)));
    }

    fn lex_identifier(&mut self, start: usize) {
        self.advance_char();
        while let Some(ch) = self.peek_char() {
            if is_ident_continue(ch) {
                self.advance_char();
            } else {
                break;
            }
        }

        let span = Span::new(start, self.cursor);
        let lexeme = self.source.slice(span);
        let kind = match lexeme {
            "fn" => TokenKind::FnKw,
            "struct" => TokenKind::StructKw,
            "enum" => TokenKind::EnumKw,
            "let" => TokenKind::LetKw,
            "mut" => TokenKind::MutKw,
            "return" => TokenKind::ReturnKw,
            "if" => TokenKind::IfKw,
            "else" => TokenKind::ElseKw,
            "while" => TokenKind::WhileKw,
            "true" => TokenKind::TrueKw,
            "false" => TokenKind::FalseKw,
            _ => TokenKind::Identifier,
        };
        self.tokens.push(Token::new(kind, span, lexeme));
    }

    fn lex_number(&mut self, start: usize) {
        self.advance_char();
        while matches!(self.peek_char(), Some(ch) if ch.is_ascii_digit()) {
            self.advance_char();
        }

        let mut kind = TokenKind::IntLiteral;
        if self.peek_char() == Some('.') && matches!(self.peek_next_char(), Some(ch) if ch.is_ascii_digit()) {
            kind = TokenKind::FloatLiteral;
            self.advance_char();
            while matches!(self.peek_char(), Some(ch) if ch.is_ascii_digit()) {
                self.advance_char();
            }
        }

        let span = Span::new(start, self.cursor);
        let lexeme = self.source.slice(span);
        if kind == TokenKind::IntLiteral && lexeme.parse::<i64>().is_err() {
            self.diagnostics.push(
                Diagnostic::new("L0003", "invalid integer literal", self.source, span)
                    .with_suggestion("use a smaller integer literal"),
            );
        }
        if kind == TokenKind::FloatLiteral && lexeme.parse::<f64>().is_err() {
            self.diagnostics.push(
                Diagnostic::new("L0004", "invalid float literal", self.source, span)
                    .with_suggestion("use a valid decimal number"),
            );
        }

        self.tokens.push(Token::new(kind, span, lexeme));
    }

    fn lex_string(&mut self, start: usize) {
        self.advance_char();
        let mut value = String::new();

        while let Some(ch) = self.peek_char() {
            if ch == '"' {
                self.advance_char();
                let span = Span::new(start, self.cursor);
                self.tokens.push(Token::new(TokenKind::StringLiteral, span, value));
                return;
            }

            if ch == '\\' {
                self.advance_char();
                match self.peek_char() {
                    Some('"') => {
                        value.push('"');
                        self.advance_char();
                    }
                    Some('\\') => {
                        value.push('\\');
                        self.advance_char();
                    }
                    Some('n') => {
                        value.push('\n');
                        self.advance_char();
                    }
                    Some('t') => {
                        value.push('\t');
                        self.advance_char();
                    }
                    Some(other) => {
                        let span = Span::new(self.cursor, self.cursor + other.len_utf8());
                        self.diagnostics.push(
                            Diagnostic::new(
                                "L0005",
                                format!("unsupported escape sequence `\\{other}`"),
                                self.source,
                                span,
                            )
                            .with_suggestion("use \\\\, \\\" , \\n, or \\t"),
                        );
                        value.push(other);
                        self.advance_char();
                    }
                    None => break,
                }
                continue;
            }

            value.push(ch);
            self.advance_char();
        }

        let span = Span::new(start, self.cursor);
        self.diagnostics.push(
            Diagnostic::new("L0002", "unterminated string literal", self.source, span)
                .with_suggestion("close the string with `\"`"),
        );
    }

    fn skip_comment(&mut self) {
        while let Some(ch) = self.peek_char() {
            self.advance_char();
            if ch == '\n' {
                break;
            }
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.source.text()[self.cursor..].chars().next()
    }

    fn peek_next_char(&self) -> Option<char> {
        let mut chars = self.source.text()[self.cursor..].chars();
        chars.next()?;
        chars.next()
    }

    fn advance_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.cursor += ch.len_utf8();
        Some(ch)
    }
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    is_ident_start(ch) || ch.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::tokenize;
    use crate::source::SourceFile;
    use crate::token::TokenKind;

    #[test]
    fn tokenizes_basic_items() {
        let source = SourceFile::anonymous("fn main() -> i32 { let mut value: i32 = 1; }");
        let output = tokenize(&source);
        let kinds = output.tokens.iter().map(|token| token.kind).collect::<Vec<_>>();
        assert!(output.diagnostics.is_empty());
        assert!(kinds.contains(&TokenKind::FnKw));
        assert!(kinds.contains(&TokenKind::LetKw));
        assert!(kinds.contains(&TokenKind::MutKw));
        assert!(kinds.contains(&TokenKind::Arrow));
    }

    #[test]
    fn reports_unterminated_string() {
        let source = SourceFile::anonymous("\"oops");
        let output = tokenize(&source);
        assert_eq!(output.diagnostics.len(), 1);
        assert_eq!(output.diagnostics[0].code, "L0002");
    }
}
