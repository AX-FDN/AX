use super::diagnostics::enrich_parse_error;
use super::*;

impl<'a> Parser<'a> {
    pub(super) fn parse_array_literal(&mut self, start: usize) -> Expr {
        let mut elements = Vec::new();
        if !self.check(TokenKind::RBracket) {
            loop {
                elements.push(self.parse_expression());
                if !self.matches(&[TokenKind::Comma]) {
                    break;
                }
                if self.check(TokenKind::RBracket) {
                    break;
                }
            }
        }

        let close = self.expect(
            TokenKind::RBracket,
            "expected `]` after array literal",
            &["`]`"],
        );

        Expr {
            span: Span::new(start, close.span.end),
            kind: ExprKind::ArrayLiteral { elements },
        }
    }

    pub(super) fn parse_expression(&mut self) -> Expr {
        self.parse_binary_expression(0)
    }

    pub(super) fn parse_binary_expression(&mut self, min_precedence: u8) -> Expr {
        let mut expr = self.parse_unary_expression();
        while let Some((op, precedence)) = self.current_binary_op() {
            if precedence < min_precedence {
                break;
            }
            self.advance();
            let right = self.parse_binary_expression(precedence + 1);
            let span = Span::join(expr.span, right.span);
            expr = Expr {
                span,
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
            };
        }
        expr
    }

    pub(super) fn parse_unary_expression(&mut self) -> Expr {
        match self.peek().kind {
            TokenKind::Minus => {
                let operator = self.advance();
                let expr = self.parse_unary_expression();
                Expr {
                    span: Span::new(operator.span.start, expr.span.end),
                    kind: ExprKind::Unary {
                        op: UnaryOp::Negate,
                        expr: Box::new(expr),
                    },
                }
            }
            TokenKind::Bang => {
                let operator = self.advance();
                let expr = self.parse_unary_expression();
                Expr {
                    span: Span::new(operator.span.start, expr.span.end),
                    kind: ExprKind::Unary {
                        op: UnaryOp::Not,
                        expr: Box::new(expr),
                    },
                }
            }
            _ => self.parse_postfix_expression(),
        }
    }

    pub(super) fn parse_postfix_expression(&mut self) -> Expr {
        let mut expr = self.parse_primary_expression();
        loop {
            if self.matches(&[TokenKind::LParen]) {
                let mut arguments = Vec::new();
                if !self.check(TokenKind::RParen) {
                    loop {
                        arguments.push(self.parse_expression());
                        if !self.matches(&[TokenKind::Comma]) {
                            break;
                        }
                    }
                }
                let close =
                    self.expect(TokenKind::RParen, "expected `)` after arguments", &["`)`"]);
                expr = Expr {
                    span: Span::new(expr.span.start, close.span.end),
                    kind: ExprKind::Call {
                        callee: Box::new(expr),
                        arguments,
                    },
                };
                continue;
            }

            if self.matches(&[TokenKind::Dot]) {
                let field = self.expect_identifier("expected a field name after `.`");
                expr = Expr {
                    span: Span::new(expr.span.start, field.span.end),
                    kind: ExprKind::Field {
                        base: Box::new(expr),
                        field: field.lexeme,
                    },
                };
                continue;
            }

            if self.matches(&[TokenKind::LBracket]) {
                let first = self.parse_expression();
                if self.matches(&[TokenKind::Colon]) {
                    let end = self.parse_expression();
                    let close = self.expect(
                        TokenKind::RBracket,
                        "expected `]` after slice expression",
                        &["`]`"],
                    );
                    expr = Expr {
                        span: Span::new(expr.span.start, close.span.end),
                        kind: ExprKind::Slice {
                            base: Box::new(expr),
                            start: Box::new(first),
                            end: Box::new(end),
                        },
                    };
                } else {
                    let close = self.expect(
                        TokenKind::RBracket,
                        "expected `]` after array index",
                        &["`]`"],
                    );
                    expr = Expr {
                        span: Span::new(expr.span.start, close.span.end),
                        kind: ExprKind::Index {
                            base: Box::new(expr),
                            index: Box::new(first),
                        },
                    };
                }
                continue;
            }

            if self.matches(&[TokenKind::Question]) {
                expr = Expr {
                    span: Span::new(expr.span.start, self.previous().span.end),
                    kind: ExprKind::Try {
                        expr: Box::new(expr),
                    },
                };
                continue;
            }

            break;
        }
        expr
    }

    pub(super) fn parse_primary_expression(&mut self) -> Expr {
        let token = self.advance();
        match token.kind {
            TokenKind::Identifier => self.parse_name_or_struct_literal(token),
            TokenKind::IntLiteral => Expr {
                span: token.span,
                kind: ExprKind::Int {
                    value: token.lexeme.parse().unwrap_or(0),
                },
            },
            TokenKind::FloatLiteral => Expr {
                span: token.span,
                kind: ExprKind::Float {
                    value: token.lexeme.parse().unwrap_or(0.0),
                },
            },
            TokenKind::StringLiteral => Expr {
                span: token.span,
                kind: ExprKind::String {
                    value: token.lexeme,
                },
            },
            TokenKind::TrueKw => Expr {
                span: token.span,
                kind: ExprKind::Bool { value: true },
            },
            TokenKind::FalseKw => Expr {
                span: token.span,
                kind: ExprKind::Bool { value: false },
            },
            TokenKind::MatchKw => self.parse_match_expression(token.span.start),
            TokenKind::LParen => {
                let mut expr = self.parse_expression();
                let close =
                    self.expect(TokenKind::RParen, "expected `)` after expression", &["`)`"]);
                expr.span = Span::new(token.span.start, close.span.end);
                expr
            }
            TokenKind::LBracket => self.parse_array_literal(token.span.start),
            _ => {
                let diagnostic = enrich_parse_error(
                    Diagnostic::new("P0003", "expected an expression", self.source, token.span)
                        .with_kind(DiagnosticKind::ExpressionRequired)
                        .with_expected("expression"),
                    &token,
                    "expected an expression",
                );
                self.diagnostics.push(diagnostic);
                Expr {
                    span: token.span,
                    kind: ExprKind::Error,
                }
            }
        }
    }

    pub(super) fn parse_name_or_struct_literal(&mut self, name: Token) -> Expr {
        if !self.check(TokenKind::LBrace) && !self.qualified_path_followed_by(TokenKind::LBrace) {
            return Expr {
                span: name.span,
                kind: ExprKind::Name { value: name.lexeme },
            };
        }

        let (name, name_span) = self.finish_qualified_identifier_path(
            name,
            "expected an identifier after `.` in struct literal path",
        );
        self.advance();
        let mut fields = Vec::new();
        if !self.check(TokenKind::RBrace) {
            loop {
                let field_name = self.expect_identifier("expected a field name in struct literal");
                self.expect(
                    TokenKind::Colon,
                    "expected `:` after struct literal field name",
                    &["`:`"],
                );
                let value = self.parse_expression();
                let span = Span::new(field_name.span.start, value.span.end);
                fields.push(StructLiteralField {
                    name: field_name.lexeme,
                    value,
                    span,
                });

                if !self.matches(&[TokenKind::Comma]) {
                    break;
                }
                if self.check(TokenKind::RBrace) {
                    break;
                }
            }
        }

        let close = self.expect(
            TokenKind::RBrace,
            "expected `}` after struct literal",
            &["`}`"],
        );
        Expr {
            span: Span::new(name_span.start, close.span.end),
            kind: ExprKind::StructLiteral { name, fields },
        }
    }

    pub(super) fn current_binary_op(&self) -> Option<(BinaryOp, u8)> {
        match self.peek().kind {
            TokenKind::PipePipe => Some((BinaryOp::LogicalOr, 5)),
            TokenKind::AmpAmp => Some((BinaryOp::LogicalAnd, 6)),
            TokenKind::EqualEqual => Some((BinaryOp::Equal, 10)),
            TokenKind::BangEqual => Some((BinaryOp::NotEqual, 10)),
            TokenKind::Less => Some((BinaryOp::Less, 20)),
            TokenKind::LessEqual => Some((BinaryOp::LessEqual, 20)),
            TokenKind::Greater => Some((BinaryOp::Greater, 20)),
            TokenKind::GreaterEqual => Some((BinaryOp::GreaterEqual, 20)),
            TokenKind::Plus => Some((BinaryOp::Add, 30)),
            TokenKind::Minus => Some((BinaryOp::Subtract, 30)),
            TokenKind::Star => Some((BinaryOp::Multiply, 40)),
            TokenKind::Slash => Some((BinaryOp::Divide, 40)),
            TokenKind::Percent => Some((BinaryOp::Remainder, 40)),
            _ => None,
        }
    }
}
