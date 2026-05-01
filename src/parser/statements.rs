use super::*;

impl<'a> Parser<'a> {
    pub(super) fn parse_block(&mut self) -> Block {
        let open = self.expect(TokenKind::LBrace, "expected `{` to start a block", &["`{`"]);
        let mut statements = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            match self.parse_statement() {
                Some(statement) => statements.push(statement),
                None => self.sync_to_statement_boundary(),
            }
        }
        let close = self.expect(
            TokenKind::RBrace,
            "expected `}` to close the block",
            &["`}`"],
        );
        Block {
            statements,
            span: Span::new(open.span.start, close.span.end),
        }
    }

    pub(super) fn parse_statement(&mut self) -> Option<Stmt> {
        match self.peek().kind {
            TokenKind::LetKw => {
                let start = self.advance().span.start;
                Some(self.parse_let_statement(start))
            }
            TokenKind::ReturnKw => {
                let start = self.advance().span.start;
                Some(self.parse_return_statement(start))
            }
            TokenKind::BreakKw => {
                let start = self.advance().span.start;
                Some(self.parse_break_statement(start))
            }
            TokenKind::ContinueKw => {
                let start = self.advance().span.start;
                Some(self.parse_continue_statement(start))
            }
            TokenKind::MatchKw => {
                let start = self.advance().span.start;
                Some(self.parse_match_statement(start))
            }
            TokenKind::IfKw => {
                let start = self.advance().span.start;
                Some(self.parse_if_statement(start))
            }
            TokenKind::WhileKw => {
                let start = self.advance().span.start;
                Some(self.parse_while_statement(start))
            }
            TokenKind::ForKw => {
                let start = self.advance().span.start;
                Some(self.parse_for_statement(start))
            }
            TokenKind::LBrace => {
                let block = self.parse_block();
                Some(Stmt {
                    span: block.span,
                    kind: StmtKind::Block { block },
                })
            }
            TokenKind::RBrace | TokenKind::Eof => None,
            _ => Some(self.parse_expr_or_assignment_statement()),
        }
    }

    pub(super) fn parse_let_statement(&mut self, start: usize) -> Stmt {
        let mutable = self.matches(&[TokenKind::MutKw]);
        let name = self.expect_identifier("expected a variable name");
        self.expect(
            TokenKind::Colon,
            "expected `:` after variable name",
            &["`:`"],
        );
        let ty = self.parse_type();
        self.expect(
            TokenKind::Equal,
            "expected `=` in variable declaration",
            &["`=`"],
        );
        let initializer = self.parse_expression();
        let end = self.expect(
            TokenKind::Semicolon,
            "expected `;` after variable declaration",
            &["`;`"],
        );
        Stmt {
            span: Span::new(start, end.span.end),
            kind: StmtKind::Let {
                mutable,
                name: name.lexeme,
                ty,
                initializer,
            },
        }
    }

    pub(super) fn parse_return_statement(&mut self, start: usize) -> Stmt {
        let value = if self.check(TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expression())
        };
        let end = self.expect(
            TokenKind::Semicolon,
            "expected `;` after return statement",
            &["`;`"],
        );
        Stmt {
            span: Span::new(start, end.span.end),
            kind: StmtKind::Return { value },
        }
    }

    pub(super) fn parse_break_statement(&mut self, start: usize) -> Stmt {
        let end = self.expect(TokenKind::Semicolon, "expected `;` after `break`", &["`;`"]);
        Stmt {
            span: Span::new(start, end.span.end),
            kind: StmtKind::Break,
        }
    }

    pub(super) fn parse_continue_statement(&mut self, start: usize) -> Stmt {
        let end = self.expect(
            TokenKind::Semicolon,
            "expected `;` after `continue`",
            &["`;`"],
        );
        Stmt {
            span: Span::new(start, end.span.end),
            kind: StmtKind::Continue,
        }
    }

    pub(super) fn parse_if_statement(&mut self, start: usize) -> Stmt {
        self.expect(TokenKind::LParen, "expected `(` after `if`", &["`(`"]);
        let condition = self.parse_expression();
        self.expect(
            TokenKind::RParen,
            "expected `)` after if condition",
            &["`)`"],
        );
        let then_branch = self.parse_block();
        let else_branch = if self.matches(&[TokenKind::ElseKw]) {
            if self.check(TokenKind::IfKw) {
                let else_if_start = self.advance().span.start;
                let else_if = self.parse_if_statement(else_if_start);
                let span = else_if.span;
                Some(Block {
                    statements: vec![else_if],
                    span,
                })
            } else {
                Some(self.parse_block())
            }
        } else {
            None
        };

        let end = else_branch
            .as_ref()
            .map(|block| block.span.end)
            .unwrap_or(then_branch.span.end);
        Stmt {
            span: Span::new(start, end),
            kind: StmtKind::If {
                condition,
                then_branch,
                else_branch,
            },
        }
    }

    pub(super) fn parse_while_statement(&mut self, start: usize) -> Stmt {
        self.expect(TokenKind::LParen, "expected `(` after `while`", &["`(`"]);
        let condition = self.parse_expression();
        self.expect(
            TokenKind::RParen,
            "expected `)` after while condition",
            &["`)`"],
        );
        let body = self.parse_block();
        Stmt {
            span: Span::new(start, body.span.end),
            kind: StmtKind::While { condition, body },
        }
    }

    pub(super) fn parse_for_statement(&mut self, start: usize) -> Stmt {
        self.expect(TokenKind::LParen, "expected `(` after `for`", &["`(`"]);
        let checkpoint = self.current;
        let diagnostics_checkpoint = self.diagnostics.len();
        if self.check(TokenKind::LetKw)
            && let Some(statement) = self.try_parse_for_in_statement(start)
        {
            return statement;
        }
        self.current = checkpoint;
        self.diagnostics.truncate(diagnostics_checkpoint);

        let initializer = self.parse_for_initializer_statement();

        let condition = if self.check(TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expression())
        };
        self.expect(
            TokenKind::Semicolon,
            "expected `;` after for condition",
            &["`;`"],
        );

        let step = if self.check(TokenKind::RParen) {
            None
        } else {
            Some(Box::new(
                self.parse_for_header_statement("expected `)` after for step"),
            ))
        };
        self.expect(
            TokenKind::RParen,
            "expected `)` after `for` header",
            &["`)`"],
        );
        let body = self.parse_block();
        Stmt {
            span: Span::new(start, body.span.end),
            kind: StmtKind::For {
                initializer,
                condition,
                step,
                body,
            },
        }
    }

    pub(super) fn try_parse_for_in_statement(&mut self, start: usize) -> Option<Stmt> {
        if !self.check(TokenKind::LetKw) {
            return None;
        }

        let binding = self.parse_for_in_binding();
        if !self.matches(&[TokenKind::InKw]) {
            return None;
        }

        let iterable = self.parse_expression();
        self.expect(
            TokenKind::RParen,
            "expected `)` after `for in` header",
            &["`)`"],
        );
        let body = self.parse_block();
        Some(Stmt {
            span: Span::new(start, body.span.end),
            kind: StmtKind::ForIn {
                binding,
                iterable,
                body,
            },
        })
    }

    pub(super) fn parse_for_in_binding(&mut self) -> ForInBinding {
        let start = self.advance().span.start;
        let mutable = self.matches(&[TokenKind::MutKw]);
        let name = self.expect_identifier("expected a loop variable name after `let`");
        self.expect(
            TokenKind::Colon,
            "expected `:` after loop variable name",
            &["`:`"],
        );
        let ty = self.parse_type();
        ForInBinding {
            mutable,
            name: name.lexeme,
            span: Span::new(start, ty.span.end),
            ty,
        }
    }

    pub(super) fn parse_for_initializer_statement(&mut self) -> Option<Box<Stmt>> {
        if self.check(TokenKind::Semicolon) {
            self.advance();
            return None;
        }

        if self.check(TokenKind::LetKw) {
            let start = self.advance().span.start;
            return Some(Box::new(self.parse_let_statement(start)));
        }

        let statement = self.parse_for_header_statement("expected `;` after for initializer");
        self.expect(
            TokenKind::Semicolon,
            "expected `;` after for initializer",
            &["`;`"],
        );
        Some(Box::new(statement))
    }

    pub(super) fn parse_for_header_statement(&mut self, missing_end_message: &str) -> Stmt {
        let expr = self.parse_expression();
        if self.matches(&[TokenKind::Equal]) {
            let value = self.parse_expression();
            return Stmt {
                span: Span::new(expr.span.start, value.span.end),
                kind: StmtKind::Assign {
                    target: expr,
                    value,
                },
            };
        }

        if self.check(TokenKind::Eof) {
            self.error_at_current("P0001", missing_end_message, &["for header terminator"]);
        }

        Stmt {
            span: expr.span,
            kind: StmtKind::Expr { expr },
        }
    }

    pub(super) fn parse_expr_or_assignment_statement(&mut self) -> Stmt {
        let expr = self.parse_expression();
        if self.matches(&[TokenKind::Equal]) {
            let value = self.parse_expression();
            let end = self.expect(
                TokenKind::Semicolon,
                "expected `;` after assignment",
                &["`;`"],
            );
            return Stmt {
                span: Span::new(expr.span.start, end.span.end),
                kind: StmtKind::Assign {
                    target: expr,
                    value,
                },
            };
        }

        let end = self.expect(
            TokenKind::Semicolon,
            "expected `;` after expression statement",
            &["`;`"],
        );
        Stmt {
            span: Span::new(expr.span.start, end.span.end),
            kind: StmtKind::Expr { expr },
        }
    }
}
