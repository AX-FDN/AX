use crate::ast::{
    BinaryOp, Block, EnumVariant, Expr, ExprKind, ForInBinding, ImportDecl, Item, ItemKind,
    MatchArm, MatchPattern, MatchPatternKind, ModuleDecl, Param, Program, SourceUnit, Stmt,
    StmtKind, StructField, StructLiteralField, TypeRef, UnaryOp,
};
use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::source::{SourceFile, Span};
use crate::token::{Token, TokenKind};

pub struct ParseOutput {
    pub program: Program,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn parse(source: &SourceFile, tokens: Vec<Token>) -> ParseOutput {
    Parser::new(source, tokens).parse_program()
}

struct Parser<'a> {
    source: &'a SourceFile,
    tokens: Vec<Token>,
    current: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn new(source: &'a SourceFile, tokens: Vec<Token>) -> Self {
        Self {
            source,
            tokens,
            current: 0,
            diagnostics: Vec::new(),
        }
    }

    fn parse_program(mut self) -> ParseOutput {
        let mut items = Vec::new();
        let mut source_units = Vec::new();

        for segment in self.source.segments() {
            source_units.push(self.parse_source_unit(&segment.path, segment.span, &mut items));
        }

        ParseOutput {
            program: Program {
                items,
                source_units,
            },
            diagnostics: self.diagnostics,
        }
    }

    fn parse_source_unit(&mut self, path: &str, span: Span, items: &mut Vec<Item>) -> SourceUnit {
        let mut module = None;
        let mut imports = Vec::new();

        if self.token_in_span(span) && self.check(TokenKind::ModuleKw) {
            module = Some(self.parse_module_decl());
        }

        while self.token_in_span(span) && self.check(TokenKind::ImportKw) {
            imports.push(self.parse_import_decl());
        }

        while self.token_in_span(span) {
            match self.parse_item() {
                Some(item) => items.push(item),
                None => self.sync_to_item(span.end),
            }
        }

        SourceUnit {
            path: path.to_string(),
            module,
            imports,
            span,
            is_entry: path == self.source.display_path(),
        }
    }

    fn parse_module_decl(&mut self) -> ModuleDecl {
        let start = self.advance().span.start;
        let (path, path_span) = self.parse_qualified_identifier_path(
            "expected a module path after `module`",
            "expected an identifier after `.` in module path",
        );
        let end = self.expect(
            TokenKind::Semicolon,
            "expected `;` after module declaration",
            &["`;`"],
        );
        ModuleDecl {
            path,
            span: Span::new(start, end.span.end.max(path_span.end)),
        }
    }

    fn parse_import_decl(&mut self) -> ImportDecl {
        let start = self.advance().span.start;
        let (path, path_span) = self.parse_qualified_identifier_path(
            "expected a module path after `import`",
            "expected an identifier after `.` in import path",
        );
        let end = self.expect(
            TokenKind::Semicolon,
            "expected `;` after import declaration",
            &["`;`"],
        );
        ImportDecl {
            path,
            span: Span::new(start, end.span.end.max(path_span.end)),
        }
    }

    fn parse_item(&mut self) -> Option<Item> {
        let start = self.peek().span.start;
        match self.peek().kind {
            TokenKind::FnKw => {
                self.advance();
                Some(self.parse_function_item(start))
            }
            TokenKind::StructKw => {
                self.advance();
                Some(self.parse_struct_item(start))
            }
            TokenKind::EnumKw => {
                self.advance();
                Some(self.parse_enum_item(start))
            }
            TokenKind::Eof => None,
            _ => {
                self.error_at_current(
                    "P0001",
                    "expected a top-level declaration",
                    &["`fn`", "`struct`", "`enum`"],
                );
                None
            }
        }
    }

    fn parse_function_item(&mut self, start: usize) -> Item {
        let name = self.expect_identifier("expected a function name");
        self.expect(
            TokenKind::LParen,
            "expected `(` after function name",
            &["`(`"],
        );
        let params = self.parse_params();
        self.expect(TokenKind::RParen, "expected `)` after parameters", &["`)`"]);
        self.expect(
            TokenKind::Arrow,
            "expected `->` before return type",
            &["`->`"],
        );
        let return_type = self.parse_type();
        let body = self.parse_block();
        Item {
            kind: ItemKind::Function {
                name: name.lexeme,
                params,
                return_type,
                body: body.clone(),
            },
            span: Span::new(start, body.span.end),
        }
    }

    fn parse_struct_item(&mut self, start: usize) -> Item {
        let name = self.expect_identifier("expected a struct name");
        self.expect(
            TokenKind::LBrace,
            "expected `{` after struct name",
            &["`{`"],
        );
        let mut fields = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            let field_name = self.expect_identifier("expected a field name");
            self.expect(TokenKind::Colon, "expected `:` after field name", &["`:`"]);
            let ty = self.parse_type();
            let span = Span::new(field_name.span.start, ty.span.end);
            fields.push(StructField {
                name: field_name.lexeme,
                ty,
                span,
            });

            if !self.matches(&[TokenKind::Comma]) {
                break;
            }
        }
        let end = self.expect(
            TokenKind::RBrace,
            "expected `}` after struct body",
            &["`}`"],
        );
        Item {
            kind: ItemKind::Struct {
                name: name.lexeme,
                fields,
            },
            span: Span::new(start, end.span.end),
        }
    }

    fn parse_enum_item(&mut self, start: usize) -> Item {
        let name = self.expect_identifier("expected an enum name");
        self.expect(TokenKind::LBrace, "expected `{` after enum name", &["`{`"]);
        let mut variants = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            let variant = self.expect_identifier("expected an enum variant");
            variants.push(EnumVariant {
                name: variant.lexeme,
                span: variant.span,
            });
            if !self.matches(&[TokenKind::Comma]) {
                break;
            }
        }
        let end = self.expect(TokenKind::RBrace, "expected `}` after enum body", &["`}`"]);
        Item {
            kind: ItemKind::Enum {
                name: name.lexeme,
                variants,
            },
            span: Span::new(start, end.span.end),
        }
    }

    fn parse_params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        if self.check(TokenKind::RParen) {
            return params;
        }

        loop {
            let name = self.expect_identifier("expected a parameter name");
            self.expect(
                TokenKind::Colon,
                "expected `:` after parameter name",
                &["`:`"],
            );
            let ty = self.parse_type();
            let span = Span::new(name.span.start, ty.span.end);
            params.push(Param {
                name: name.lexeme,
                ty,
                span,
            });

            if !self.matches(&[TokenKind::Comma]) {
                break;
            }
        }

        params
    }

    fn parse_type(&mut self) -> TypeRef {
        if self.matches(&[TokenKind::LBracket]) {
            let start = self.previous().span.start;
            let element = self.parse_type();
            if self.matches(&[TokenKind::Semicolon]) {
                let length_token = self.expect(
                    TokenKind::IntLiteral,
                    "expected an integer array length",
                    &["integer literal"],
                );
                let length = length_token.lexeme.parse::<usize>().unwrap_or_else(|_| {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "P0002",
                            "expected a valid non-negative array length",
                            self.source,
                            length_token.span,
                        )
                        .with_expected("non-negative integer literal")
                        .with_suggestion("use an array length like `[i32; 3]`"),
                    );
                    0
                });
                let close = self.expect(
                    TokenKind::RBracket,
                    "expected `]` after array type",
                    &["`]`"],
                );
                return TypeRef::array(element, length, Span::new(start, close.span.end));
            }

            let close = self.expect(
                TokenKind::RBracket,
                "expected `]` after slice type",
                &["`]`"],
            );
            return TypeRef::slice(element, Span::new(start, close.span.end));
        }

        let token = if self.check(TokenKind::Identifier) {
            self.advance()
        } else {
            self.error_at_current("P0002", "expected a type name", &["type name"]);
            self.advance()
        };

        let (name, span) = self.finish_qualified_identifier_path(
            token,
            "expected an identifier after `.` in type path",
        );

        TypeRef::named(name, span)
    }

    fn parse_array_literal(&mut self, start: usize) -> Expr {
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

    fn parse_block(&mut self) -> Block {
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

    fn parse_statement(&mut self) -> Option<Stmt> {
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

    fn parse_let_statement(&mut self, start: usize) -> Stmt {
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

    fn parse_return_statement(&mut self, start: usize) -> Stmt {
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

    fn parse_break_statement(&mut self, start: usize) -> Stmt {
        let end = self.expect(TokenKind::Semicolon, "expected `;` after `break`", &["`;`"]);
        Stmt {
            span: Span::new(start, end.span.end),
            kind: StmtKind::Break,
        }
    }

    fn parse_continue_statement(&mut self, start: usize) -> Stmt {
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

    fn parse_match_statement(&mut self, start: usize) -> Stmt {
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

    fn parse_match_arm(&mut self) -> MatchArm {
        let pattern = self.parse_match_pattern();
        self.expect(
            TokenKind::FatArrow,
            "expected `=>` after match pattern",
            &["`=>`"],
        );
        let body = self.parse_block();
        MatchArm {
            span: Span::new(pattern.span.start, body.span.end),
            pattern,
            body,
        }
    }

    fn parse_match_pattern(&mut self) -> MatchPattern {
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
            TokenKind::IntLiteral => MatchPattern {
                span: token.span,
                kind: MatchPatternKind::Int {
                    value: token.lexeme.parse().unwrap_or(0),
                },
            },
            TokenKind::Minus => {
                let literal = self.expect(
                    TokenKind::IntLiteral,
                    "expected an integer literal after `-` in match pattern",
                    &["integer literal"],
                );
                MatchPattern {
                    span: Span::new(token.span.start, literal.span.end),
                    kind: MatchPatternKind::Int {
                        value: -literal.lexeme.parse::<i64>().unwrap_or(0),
                    },
                }
            }
            TokenKind::Identifier => {
                if token.lexeme == "_" && !self.check(TokenKind::Dot) {
                    return MatchPattern {
                        span: token.span,
                        kind: MatchPatternKind::Wildcard,
                    };
                }

                let (path, span) = self.finish_qualified_identifier_path(
                    token,
                    "expected an identifier after `.` in match pattern",
                );
                MatchPattern {
                    span,
                    kind: MatchPatternKind::EnumVariant { path },
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

    fn parse_if_statement(&mut self, start: usize) -> Stmt {
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

    fn parse_while_statement(&mut self, start: usize) -> Stmt {
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

    fn parse_for_statement(&mut self, start: usize) -> Stmt {
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

    fn try_parse_for_in_statement(&mut self, start: usize) -> Option<Stmt> {
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

    fn parse_for_in_binding(&mut self) -> ForInBinding {
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

    fn parse_for_initializer_statement(&mut self) -> Option<Box<Stmt>> {
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

    fn parse_for_header_statement(&mut self, missing_end_message: &str) -> Stmt {
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

    fn parse_expr_or_assignment_statement(&mut self) -> Stmt {
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

    fn parse_expression(&mut self) -> Expr {
        self.parse_binary_expression(0)
    }

    fn parse_binary_expression(&mut self, min_precedence: u8) -> Expr {
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

    fn parse_unary_expression(&mut self) -> Expr {
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

    fn parse_postfix_expression(&mut self) -> Expr {
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

            break;
        }
        expr
    }

    fn parse_primary_expression(&mut self) -> Expr {
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

    fn parse_name_or_struct_literal(&mut self, name: Token) -> Expr {
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

    fn current_binary_op(&self) -> Option<(BinaryOp, u8)> {
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

    fn sync_to_item(&mut self, segment_end: usize) {
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

    fn sync_to_statement_boundary(&mut self) {
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

    fn expect(&mut self, kind: TokenKind, message: &str, expected: &[&str]) -> Token {
        if self.check(kind) {
            self.advance()
        } else {
            self.error_at_current("P0001", message, expected);
            self.advance()
        }
    }

    fn expect_identifier(&mut self, message: &str) -> Token {
        if self.check(TokenKind::Identifier) {
            self.advance()
        } else {
            self.error_at_current("P0002", message, &["identifier"]);
            self.advance()
        }
    }

    fn error_at_current(&mut self, code: &str, message: &str, expected: &[&str]) {
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

    fn matches(&mut self, kinds: &[TokenKind]) -> bool {
        if kinds.iter().any(|kind| self.check(*kind)) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check(&self, kind: TokenKind) -> bool {
        !self.is_at_end() && self.peek().kind == kind
            || (kind == TokenKind::Eof && self.peek().kind == TokenKind::Eof)
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.current].clone();
        if !self.is_at_end() {
            self.current += 1;
        }
        token
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous_kind(&self) -> Option<TokenKind> {
        self.current
            .checked_sub(1)
            .map(|index| self.tokens[index].kind)
    }

    fn previous(&self) -> &Token {
        let index = self.current.saturating_sub(1);
        &self.tokens[index]
    }

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    fn token_in_span(&self, span: Span) -> bool {
        !self.is_at_end() && self.peek().span.start < span.end
    }

    fn parse_qualified_identifier_path(
        &mut self,
        expected_path_message: &str,
        expected_segment_message: &str,
    ) -> (String, Span) {
        let first = self.expect_identifier(expected_path_message);
        self.finish_qualified_identifier_path(first, expected_segment_message)
    }

    fn finish_qualified_identifier_path(
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

    fn qualified_path_followed_by(&self, terminator: TokenKind) -> bool {
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

fn parse_error_kind(code: &str, message: &str, expected: &[&str]) -> Option<DiagnosticKind> {
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

fn enrich_parse_error(mut diagnostic: Diagnostic, token: &Token, message: &str) -> Diagnostic {
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
                return diagnostic
                    .with_suggestion("start a top-level item with `fn`, `struct`, or `enum`");
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
        return diagnostic.with_suggestion("start a top-level item with `fn`, `struct`, or `enum`");
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

#[cfg(test)]
mod tests {
    use super::{enrich_parse_error, parse};
    use crate::ast::{ExprKind, ItemKind, MatchPatternKind, StmtKind};
    use crate::diagnostics::{Diagnostic, DiagnosticKind};
    use crate::lexer::tokenize;
    use crate::source::{SourceFile, Span};
    use crate::token::{Token, TokenKind};
    use std::path::PathBuf;

    #[test]
    fn parses_minimal_main() {
        let source = SourceFile::anonymous("fn main() -> i32 { return 0; }");
        let tokens = tokenize(&source).tokens;
        let output = parse(&source, tokens);
        assert!(output.diagnostics.is_empty());
        assert_eq!(output.program.items.len(), 1);
        assert_eq!(output.program.source_units.len(), 1);
        match &output.program.items[0].kind {
            ItemKind::Function { name, body, .. } => {
                assert_eq!(name, "main");
                assert_eq!(body.statements.len(), 1);
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_module_headers_per_source_segment() {
        let source = SourceFile::from_segments(
            "src/main.ax",
            vec![
                (
                    PathBuf::from("foundation/search.ax"),
                    "module foundation.search;\nfn helper() -> i32 { return 1; }\n".to_string(),
                ),
                (
                    PathBuf::from("src/main.ax"),
                    "import foundation.search;\nfn main() -> i32 { return foundation.search.helper(); }\n"
                        .to_string(),
                ),
            ],
        );
        let tokens = tokenize(&source).tokens;
        let output = parse(&source, tokens);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        assert_eq!(output.program.source_units.len(), 2);

        let support = &output.program.source_units[0];
        assert_eq!(support.path, "foundation/search.ax");
        assert_eq!(
            support.module.as_ref().map(|module| module.path.as_str()),
            Some("foundation.search")
        );
        assert!(support.imports.is_empty());
        assert!(!support.is_entry);

        let entry = &output.program.source_units[1];
        assert_eq!(entry.path, "src/main.ax");
        assert_eq!(entry.imports.len(), 1);
        assert_eq!(entry.imports[0].path, "foundation.search");
        assert!(entry.is_entry);
    }

    #[test]
    fn parses_qualified_type_path() {
        let source = SourceFile::anonymous(
            "fn main() -> i32 { let value: foundation.search.SearchStats = foundation.search.SearchStats { match_count: 0 }; return 0; }",
        );
        let tokens = tokenize(&source).tokens;
        let output = parse(&source, tokens);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

        let ItemKind::Function { body, .. } = &output.program.items[0].kind else {
            panic!("expected function");
        };
        let StmtKind::Let {
            ty, initializer, ..
        } = &body.statements[0].kind
        else {
            panic!("expected let statement");
        };

        assert_eq!(ty.describe(), "foundation.search.SearchStats");
        assert!(matches!(initializer.kind, ExprKind::StructLiteral { .. }));
    }

    #[test]
    fn respects_operator_precedence() {
        let source = SourceFile::anonymous("fn main() -> i32 { return 1 + 2 * 3; }");
        let tokens = tokenize(&source).tokens;
        let output = parse(&source, tokens);
        let function = &output.program.items[0];
        match &function.kind {
            ItemKind::Function { body, .. } => match &body.statements[0].kind {
                StmtKind::Return { value: Some(expr) } => match &expr.kind {
                    ExprKind::Binary { op, right, .. } => {
                        assert!(matches!(op, crate::ast::BinaryOp::Add));
                        assert!(matches!(right.kind, ExprKind::Binary { .. }));
                    }
                    _ => panic!("expected binary expr"),
                },
                _ => panic!("expected return"),
            },
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn respects_logical_operator_precedence() {
        let source = SourceFile::anonymous("fn main() -> i32 { return true || false && false; }");
        let tokens = tokenize(&source).tokens;
        let output = parse(&source, tokens);
        let function = &output.program.items[0];
        match &function.kind {
            ItemKind::Function { body, .. } => match &body.statements[0].kind {
                StmtKind::Return { value: Some(expr) } => match &expr.kind {
                    ExprKind::Binary { op, right, .. } => {
                        assert!(matches!(op, crate::ast::BinaryOp::LogicalOr));
                        assert!(matches!(
                            right.kind,
                            ExprKind::Binary {
                                op: crate::ast::BinaryOp::LogicalAnd,
                                ..
                            }
                        ));
                    }
                    _ => panic!("expected logical binary expr"),
                },
                _ => panic!("expected return"),
            },
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn respects_modulo_precedence() {
        let source = SourceFile::anonymous("fn main() -> i32 { return 8 % 3 * 2; }");
        let tokens = tokenize(&source).tokens;
        let output = parse(&source, tokens);
        let function = &output.program.items[0];
        match &function.kind {
            ItemKind::Function { body, .. } => match &body.statements[0].kind {
                StmtKind::Return { value: Some(expr) } => match &expr.kind {
                    ExprKind::Binary { op, left, .. } => {
                        assert!(matches!(op, crate::ast::BinaryOp::Multiply));
                        assert!(matches!(
                            left.kind,
                            ExprKind::Binary {
                                op: crate::ast::BinaryOp::Remainder,
                                ..
                            }
                        ));
                    }
                    _ => panic!("expected multiplicative binary expr"),
                },
                _ => panic!("expected return"),
            },
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_struct_literal_and_field_access() {
        let source = SourceFile::anonymous(
            "fn main() -> i32 { let point: Point = Point { x: 1, y: 2 }; return point.x; }",
        );
        let tokens = tokenize(&source).tokens;
        let output = parse(&source, tokens);
        assert!(output.diagnostics.is_empty());
        match &output.program.items[0].kind {
            ItemKind::Function { body, .. } => match &body.statements[0].kind {
                StmtKind::Let { initializer, .. } => {
                    assert!(matches!(initializer.kind, ExprKind::StructLiteral { .. }));
                }
                _ => panic!("expected let statement"),
            },
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_for_statement() {
        let source = SourceFile::anonymous(
            "\
fn main() -> i32 {
    for (let mut i: i32 = 0; i < 3; i = i + 1) {
        println(i);
    }
    return 0;
}
",
        );
        let tokens = tokenize(&source).tokens;
        let output = parse(&source, tokens);
        assert!(
            output.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            output.diagnostics
        );
        match &output.program.items[0].kind {
            ItemKind::Function { body, .. } => match &body.statements[0].kind {
                StmtKind::For {
                    initializer,
                    condition,
                    step,
                    ..
                } => {
                    assert!(initializer.is_some());
                    assert!(condition.is_some());
                    assert!(step.is_some());
                }
                _ => panic!("expected for statement"),
            },
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_for_in_statement() {
        let source = SourceFile::anonymous(
            "\
fn main() -> i32 {
    let values: [i32; 3] = [1, 2, 3];
    for (let value: i32 in values) {
        println(value);
    }
    return 0;
}
",
        );
        let tokens = tokenize(&source).tokens;
        let output = parse(&source, tokens);
        assert!(
            output.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            output.diagnostics
        );
        match &output.program.items[0].kind {
            ItemKind::Function { body, .. } => match &body.statements[1].kind {
                StmtKind::ForIn {
                    binding, iterable, ..
                } => {
                    assert_eq!(binding.name, "value");
                    assert_eq!(binding.ty.describe(), "i32");
                    assert!(matches!(iterable.kind, ExprKind::Name { ref value } if value == "values"));
                }
                _ => panic!("expected for-in statement"),
            },
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_break_statement() {
        let source = SourceFile::anonymous(
            "\
fn main() -> i32 {
    while (true) {
        break;
    }
    return 0;
}
",
        );
        let tokens = tokenize(&source).tokens;
        let output = parse(&source, tokens);
        assert!(
            output.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            output.diagnostics
        );
        match &output.program.items[0].kind {
            ItemKind::Function { body, .. } => match &body.statements[0].kind {
                StmtKind::While { body, .. } => {
                    assert!(matches!(body.statements[0].kind, StmtKind::Break));
                }
                _ => panic!("expected while statement"),
            },
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_continue_statement() {
        let source = SourceFile::anonymous(
            "\
fn main() -> i32 {
    while (true) {
        continue;
    }
    return 0;
}
",
        );
        let tokens = tokenize(&source).tokens;
        let output = parse(&source, tokens);
        assert!(
            output.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            output.diagnostics
        );
        match &output.program.items[0].kind {
            ItemKind::Function { body, .. } => match &body.statements[0].kind {
                StmtKind::While { body, .. } => {
                    assert!(matches!(body.statements[0].kind, StmtKind::Continue));
                }
                _ => panic!("expected while statement"),
            },
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parses_match_statement() {
        let source = SourceFile::anonymous(
            "\
enum Flag { On, Off }
fn main() -> i32 {
    match (Flag.On) {
        Flag.On => {
            return 1;
        }
        Flag.Off => {
            return 0;
        }
    }
}
",
        );
        let tokens = tokenize(&source).tokens;
        let output = parse(&source, tokens);
        assert!(
            output.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            output.diagnostics
        );

        let ItemKind::Function { body, .. } = &output.program.items[1].kind else {
            panic!("expected main function");
        };
        let StmtKind::Match { scrutinee, arms } = &body.statements[0].kind else {
            panic!("expected match statement");
        };
        assert!(matches!(scrutinee.kind, ExprKind::Field { .. }));
        assert_eq!(arms.len(), 2);
        assert!(matches!(
            arms[0].pattern.kind,
            MatchPatternKind::EnumVariant { ref path } if path == "Flag.On"
        ));
        assert!(matches!(
            arms[1].pattern.kind,
            MatchPatternKind::EnumVariant { ref path } if path == "Flag.Off"
        ));
    }

    #[test]
    fn parses_array_types_literals_and_indexing() {
        let source = SourceFile::anonymous(
            "fn main() -> i32 { let values: [i32; 3] = [1, 2, 3]; return values[1]; }",
        );
        let tokens = tokenize(&source).tokens;
        let output = parse(&source, tokens);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

        let ItemKind::Function { body, .. } = &output.program.items[0].kind else {
            panic!("expected function");
        };

        let StmtKind::Let {
            ty, initializer, ..
        } = &body.statements[0].kind
        else {
            panic!("expected let statement");
        };
        assert_eq!(ty.describe(), "[i32; 3]");
        assert!(matches!(initializer.kind, ExprKind::ArrayLiteral { .. }));

        let StmtKind::Return { value: Some(expr) } = &body.statements[1].kind else {
            panic!("expected return statement");
        };
        assert!(matches!(expr.kind, ExprKind::Index { .. }));
    }

    #[test]
    fn parses_slice_types_and_expressions() {
        let source = SourceFile::anonymous(
            "fn read(values: [i32]) -> i32 { let head: [i32] = values[0:2]; return head[1]; }",
        );
        let tokens = tokenize(&source).tokens;
        let output = parse(&source, tokens);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

        let ItemKind::Function {
            params,
            return_type,
            body,
            ..
        } = &output.program.items[0].kind
        else {
            panic!("expected function");
        };

        assert_eq!(params[0].ty.describe(), "[i32]");
        assert_eq!(return_type.describe(), "i32");

        let StmtKind::Let {
            ty, initializer, ..
        } = &body.statements[0].kind
        else {
            panic!("expected let statement");
        };
        assert_eq!(ty.describe(), "[i32]");
        assert!(matches!(initializer.kind, ExprKind::Slice { .. }));

        let StmtKind::Return { value: Some(expr) } = &body.statements[1].kind else {
            panic!("expected return");
        };
        assert!(matches!(expr.kind, ExprKind::Index { .. }));
    }

    #[test]
    fn enriches_missing_semicolon_diagnostic() {
        let source = SourceFile::anonymous("fn main() -> i32 { let value: i32 = 1 return value; }");
        let tokens = tokenize(&source).tokens;
        let output = parse(&source, tokens);
        let diagnostic = output
            .diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic
                    .message
                    .contains("expected `;` after variable declaration")
            })
            .expect("missing semicolon diagnostic should exist");

        assert!(
            diagnostic
                .notes
                .iter()
                .any(|note| note.contains("explicit semicolons"))
        );
        assert_eq!(diagnostic.kind(), Some(DiagnosticKind::MissingSemicolon));
        assert_eq!(
            diagnostic.suggestion.as_deref(),
            Some("insert `;` before the next statement or closing `}`")
        );
    }

    #[test]
    fn enriches_missing_right_paren_diagnostic() {
        let source = SourceFile::anonymous("fn main() -> i32 { if (true { return 1; } return 0; }");
        let tokens = tokenize(&source).tokens;
        let output = parse(&source, tokens);
        let diagnostic = output
            .diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic
                    .message
                    .contains("expected `)` after if condition")
            })
            .expect("missing right paren diagnostic should exist");

        assert!(
            diagnostic
                .notes
                .iter()
                .any(|note| note.contains("left open"))
        );
        assert_eq!(diagnostic.kind(), Some(DiagnosticKind::MissingRightParen));
        assert_eq!(
            diagnostic.suggestion.as_deref(),
            Some("insert `)` to close the current parenthesized construct")
        );
    }

    #[test]
    fn enriches_missing_right_brace_diagnostic_with_stable_kind() {
        let source = SourceFile::anonymous("fn main() -> i32 { if (true) { return 1; }");
        let tokens = tokenize(&source).tokens;
        let output = parse(&source, tokens);
        let diagnostic = output
            .diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic
                    .message
                    .contains("expected `}` to close the block")
            })
            .expect("missing right brace diagnostic should exist");

        assert_eq!(diagnostic.kind(), Some(DiagnosticKind::MissingRightBrace));
        assert_eq!(
            diagnostic.suggestion.as_deref(),
            Some("insert `}` to close the current block or literal")
        );
    }

    #[test]
    fn enriches_missing_right_bracket_diagnostic() {
        let source =
            SourceFile::anonymous("fn main() -> i32 { let values: [i32; 2 = [1, 2]; return 0; }");
        let tokens = tokenize(&source).tokens;
        let output = parse(&source, tokens);
        let diagnostic = output
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.message.contains("expected `]` after array type"))
            .expect("missing right bracket diagnostic should exist");

        assert!(
            diagnostic
                .notes
                .iter()
                .any(|note| note.contains("slice types"))
        );
        assert_eq!(diagnostic.kind(), Some(DiagnosticKind::MissingRightBracket));
        assert_eq!(
            diagnostic.suggestion.as_deref(),
            Some("insert `]` to close the current bracketed construct")
        );
    }

    #[test]
    fn classifies_top_level_declaration_error_with_stable_kind() {
        let source = SourceFile::anonymous("let value: i32 = 1;");
        let tokens = tokenize(&source).tokens;
        let output = parse(&source, tokens);
        let diagnostic = output
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.message == "expected a top-level declaration")
            .expect("top-level declaration diagnostic should exist");

        assert_eq!(
            diagnostic.kind(),
            Some(DiagnosticKind::TopLevelDeclarationRequired)
        );
        assert_eq!(
            diagnostic.suggestion.as_deref(),
            Some("start a top-level item with `fn`, `struct`, or `enum`")
        );
    }

    #[test]
    fn stable_kind_keeps_parse_help_even_if_message_text_changes() {
        let source = SourceFile::anonymous("fn main() -> i32 { return 0 }");
        let token = Token {
            kind: TokenKind::RBrace,
            lexeme: "}".to_string(),
            span: Span::new(27, 28),
        };
        let diagnostic =
            Diagnostic::new("P0001", "placeholder parser wording", &source, token.span)
                .with_kind(DiagnosticKind::MissingSemicolon);

        let enriched = enrich_parse_error(diagnostic, &token, "placeholder parser wording");

        assert!(
            enriched
                .notes
                .iter()
                .any(|note| note.contains("explicit semicolons"))
        );
        assert_eq!(
            enriched.suggestion.as_deref(),
            Some("insert `;` before the next statement or closing `}`")
        );
    }

    #[test]
    fn classifies_type_name_error_with_stable_kind() {
        let source = SourceFile::anonymous("fn main() -> i32 { let value: = 1; return 0; }");
        let tokens = tokenize(&source).tokens;
        let output = parse(&source, tokens);
        let diagnostic = output
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.message == "expected a type name")
            .expect("type name diagnostic should exist");

        assert_eq!(diagnostic.kind(), Some(DiagnosticKind::TypeNameRequired));
        assert_eq!(
            diagnostic.suggestion.as_deref(),
            Some(
                "use `bool`, `i32`, `f32`, `string`, `[Type]`, `[Type; N]`, or a previously declared type name"
            )
        );
    }

    #[test]
    fn classifies_expression_error_with_stable_kind() {
        let source = SourceFile::anonymous("fn main() -> i32 { let value: i32 = ; return 0; }");
        let tokens = tokenize(&source).tokens;
        let output = parse(&source, tokens);
        let diagnostic = output
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.message == "expected an expression")
            .expect("expression diagnostic should exist");

        assert_eq!(diagnostic.kind(), Some(DiagnosticKind::ExpressionRequired));
        assert_eq!(
            diagnostic.suggestion.as_deref(),
            Some(
                "insert a runtime expression such as a literal, array literal, name, call, or parenthesized expression"
            )
        );
    }
}
