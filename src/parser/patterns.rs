use super::diagnostics::enrich_parse_error;
use super::literals::{ParsedIntLiteral, parse_signed_i64_literal};
use super::*;

impl<'a> Parser<'a> {
    pub(super) fn parse_match_statement(&mut self, start: usize) -> Stmt {
        self.expect(TokenKind::LParen, "expected `(` after `match`", &["`(`"]);
        let scrutinee = self.parse_expression();
        self.expect(
            TokenKind::RParen,
            "expected `)` after match input",
            &["`)`"],
        );

        let open = self.expect(
            TokenKind::LBrace,
            "expected `{` to start match arms",
            &["`{`"],
        );
        let mut arms = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            arms.push(self.parse_match_arm());
            if self.matches(&[TokenKind::Comma]) && self.check(TokenKind::RBrace) {
                break;
            }
        }
        let close = self.expect(TokenKind::RBrace, "expected `}` after match arms", &["`}`"]);

        Stmt {
            span: Span::new(start, close.span.end),
            kind: StmtKind::Match {
                scrutinee,
                arms: if arms.is_empty() && open.span.end == close.span.start {
                    Vec::new()
                } else {
                    arms
                },
            },
        }
    }

    pub(super) fn parse_match_expression(&mut self, start: usize) -> Expr {
        self.expect(TokenKind::LParen, "expected `(` after `match`", &["`(`"]);
        let scrutinee = self.parse_expression();
        self.expect(
            TokenKind::RParen,
            "expected `)` after match input",
            &["`)`"],
        );

        let open = self.expect(
            TokenKind::LBrace,
            "expected `{` to start match arms",
            &["`{`"],
        );
        let mut arms = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            arms.push(self.parse_match_expression_arm());
            if self.matches(&[TokenKind::Comma]) && self.check(TokenKind::RBrace) {
                break;
            }
        }
        let close = self.expect(TokenKind::RBrace, "expected `}` after match arms", &["`}`"]);

        Expr {
            span: Span::new(start, close.span.end),
            kind: ExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms: if arms.is_empty() && open.span.end == close.span.start {
                    Vec::new()
                } else {
                    arms
                },
            },
        }
    }

    pub(super) fn parse_match_arm(&mut self) -> MatchArm {
        let pattern = self.parse_match_pattern();
        let guard = self.parse_match_guard();
        self.expect(
            TokenKind::FatArrow,
            "expected `=>` after match pattern",
            &["`=>`"],
        );
        let body = self.parse_block();
        MatchArm {
            span: Span::new(pattern.span.start, body.span.end),
            pattern,
            guard,
            body,
        }
    }

    pub(super) fn parse_match_expression_arm(&mut self) -> MatchExprArm {
        let pattern = self.parse_match_pattern();
        let guard = self.parse_match_guard();
        self.expect(
            TokenKind::FatArrow,
            "expected `=>` after match pattern",
            &["`=>`"],
        );
        let value = if self.check(TokenKind::LBrace) {
            self.parse_match_expression_block_value()
        } else {
            self.parse_expression()
        };
        MatchExprArm {
            span: Span::new(pattern.span.start, value.span.end),
            pattern,
            guard,
            value,
        }
    }

    pub(super) fn parse_match_expression_block_value(&mut self) -> Expr {
        let open = self.expect(
            TokenKind::LBrace,
            "expected `{` to start match expression block arm",
            &["`{`"],
        );
        let mut statements = Vec::new();

        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            if self.starts_forced_statement_in_block_expr() {
                if let Some(statement) = self.parse_statement() {
                    statements.push(statement);
                }
                continue;
            }

            let expr = self.parse_expression();
            if self.matches(&[TokenKind::Equal]) {
                let value = self.parse_expression();
                let end = self.expect(
                    TokenKind::Semicolon,
                    "expected `;` after assignment in match expression block",
                    &["`;`"],
                );
                statements.push(Stmt {
                    span: Span::new(expr.span.start, end.span.end),
                    kind: StmtKind::Assign {
                        target: expr,
                        value,
                    },
                });
                continue;
            }

            if self.matches(&[TokenKind::Semicolon]) {
                let semicolon = self.previous();
                statements.push(Stmt {
                    span: Span::new(expr.span.start, semicolon.span.end),
                    kind: StmtKind::Expr { expr },
                });
                continue;
            }

            let close = self.expect(
                TokenKind::RBrace,
                "expected `}` after final match expression block value",
                &["`}`"],
            );
            return Expr {
                span: Span::new(open.span.start, close.span.end),
                kind: ExprKind::Block {
                    statements,
                    value: Box::new(expr),
                },
            };
        }

        self.error_at_current(
            "P0001",
            "match expression block arms must end with a value expression",
            &["final expression"],
        );
        let close = self.expect(
            TokenKind::RBrace,
            "expected a final expression before `}` in match expression block arm",
            &["expression"],
        );
        Expr {
            span: Span::new(open.span.start, close.span.end),
            kind: ExprKind::Block {
                statements,
                value: Box::new(Expr {
                    span: close.span,
                    kind: ExprKind::Error,
                }),
            },
        }
    }

    pub(super) fn starts_forced_statement_in_block_expr(&self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::LetKw
                | TokenKind::ReturnKw
                | TokenKind::BreakKw
                | TokenKind::ContinueKw
                | TokenKind::IfKw
                | TokenKind::WhileKw
                | TokenKind::ForKw
                | TokenKind::LBrace
        )
    }

    pub(super) fn parse_match_guard(&mut self) -> Option<Expr> {
        if !self.matches(&[TokenKind::IfKw]) {
            return None;
        }

        Some(self.parse_expression())
    }

    pub(super) fn parse_match_pattern(&mut self) -> MatchPattern {
        let first = self.parse_single_match_pattern();
        if !self.matches(&[TokenKind::Pipe]) {
            return first;
        }

        let start = first.span.start;
        let mut alternatives = vec![first];
        loop {
            alternatives.push(self.parse_single_match_pattern());
            if !self.matches(&[TokenKind::Pipe]) {
                break;
            }
        }
        let end = alternatives
            .last()
            .map(|pattern| pattern.span.end)
            .unwrap_or(start);
        MatchPattern {
            span: Span::new(start, end),
            kind: MatchPatternKind::Or { alternatives },
        }
    }

    pub(super) fn parse_single_match_pattern(&mut self) -> MatchPattern {
        let token = self.advance();
        match token.kind {
            TokenKind::TrueKw => MatchPattern {
                span: token.span,
                kind: MatchPatternKind::Bool { value: true },
            },
            TokenKind::FalseKw => MatchPattern {
                span: token.span,
                kind: MatchPatternKind::Bool { value: false },
            },
            TokenKind::IntLiteral => self.parse_int_match_pattern(token, false),
            TokenKind::StringLiteral => MatchPattern {
                span: token.span,
                kind: MatchPatternKind::String {
                    value: token.lexeme,
                },
            },
            TokenKind::Minus => {
                let literal = self.expect(
                    TokenKind::IntLiteral,
                    "expected an integer literal after `-` in match pattern",
                    &["integer literal"],
                );
                self.parse_int_match_pattern(literal, true)
            }
            TokenKind::Identifier => {
                if token.lexeme == "_" && !self.check(TokenKind::Dot) {
                    return MatchPattern {
                        span: token.span,
                        kind: MatchPatternKind::Wildcard,
                    };
                }

                if self.check(TokenKind::LBrace) {
                    return self.finish_struct_match_pattern(token.lexeme, token.span);
                }

                if !self.check(TokenKind::Dot) {
                    return MatchPattern {
                        span: token.span,
                        kind: MatchPatternKind::Binding { name: token.lexeme },
                    };
                }

                let (path, span) = self.finish_qualified_identifier_path(
                    token,
                    "expected an identifier after `.` in match pattern",
                );
                if self.check(TokenKind::LBrace) {
                    return self.finish_struct_match_pattern(path, span);
                }
                let payload = if self.matches(&[TokenKind::LParen]) {
                    Some(self.parse_enum_variant_pattern_payload())
                } else {
                    None
                };
                MatchPattern {
                    span,
                    kind: MatchPatternKind::EnumVariant { path, payload },
                }
            }
            _ => {
                let diagnostic = enrich_parse_error(
                    Diagnostic::new("P0003", "expected a match pattern", self.source, token.span)
                        .with_expected("match pattern"),
                    &token,
                    "expected a match pattern",
                );
                self.diagnostics.push(diagnostic);
                MatchPattern {
                    span: token.span,
                    kind: MatchPatternKind::Error,
                }
            }
        }
    }

    pub(super) fn finish_struct_match_pattern(
        &mut self,
        path: String,
        path_span: Span,
    ) -> MatchPattern {
        self.expect(
            TokenKind::LBrace,
            "expected `{` after struct pattern name",
            &["`{`"],
        );
        let mut fields = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            let field =
                self.expect_identifier("expected a field binding name in struct match pattern");
            let span = field.span;
            if self.matches(&[TokenKind::Colon]) {
                let alias =
                    self.expect_identifier("expected a binding name after `:` in struct pattern");
                self.diagnostics.push(
                    Diagnostic::new(
                        "P0003",
                        "struct match patterns currently use shorthand fields only",
                        self.source,
                        Span::new(span.start, alias.span.end),
                    )
                    .with_kind(DiagnosticKind::MatchStructPatternShapeMismatch)
                    .with_expected("shorthand struct pattern field")
                    .with_suggestion(format!("rewrite this field as `{}`", field.lexeme)),
                );
            }
            fields.push(StructPatternField {
                name: field.lexeme.clone(),
                binding: field.lexeme,
                span,
            });
            if !self.matches(&[TokenKind::Comma]) {
                break;
            }
        }
        let end = self.expect(
            TokenKind::RBrace,
            "expected `}` after struct match pattern",
            &["`}`"],
        );
        MatchPattern {
            span: Span::new(path_span.start, end.span.end),
            kind: MatchPatternKind::Struct { path, fields },
        }
    }

    pub(super) fn parse_int_match_pattern(&mut self, token: Token, negative: bool) -> MatchPattern {
        let start_value = parse_signed_i64_literal(&token.lexeme, negative);
        let start_span = if negative {
            Span::new(token.span.start.saturating_sub(1), token.span.end)
        } else {
            token.span
        };

        if self.matches(&[TokenKind::DotDotEqual]) {
            let end = self.parse_match_range_bound();
            return MatchPattern {
                span: Span::new(start_span.start, end.span.end),
                kind: MatchPatternKind::IntRange {
                    start: start_value,
                    end: end.value,
                },
            };
        }

        MatchPattern {
            span: start_span,
            kind: MatchPatternKind::Int { value: start_value },
        }
    }

    pub(super) fn parse_match_range_bound(&mut self) -> ParsedIntLiteral {
        let negative = self.matches(&[TokenKind::Minus]);
        let value = self.expect(
            TokenKind::IntLiteral,
            "expected an integer literal after `..=` in match range pattern",
            &["integer literal"],
        );
        ParsedIntLiteral {
            value: parse_signed_i64_literal(&value.lexeme, negative),
            span: value.span,
        }
    }

    pub(super) fn parse_enum_variant_pattern_payload(&mut self) -> EnumVariantPayloadPattern {
        let token =
            self.expect_identifier("expected `_` or a binding name in enum payload pattern");
        let payload = if token.lexeme == "_" {
            EnumVariantPayloadPattern::Wildcard
        } else {
            EnumVariantPayloadPattern::Binding { name: token.lexeme }
        };
        self.expect(
            TokenKind::RParen,
            "expected `)` after enum payload pattern",
            &["`)`"],
        );
        payload
    }
}
