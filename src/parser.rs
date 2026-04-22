use crate::ast::{
    BinaryOp, Block, EnumVariant, Expr, ExprKind, Item, ItemKind, Param, Program, Stmt, StmtKind,
    StructField, StructLiteralField, TypeRef, UnaryOp,
};
use crate::diagnostics::Diagnostic;
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
        while !self.is_at_end() {
            match self.parse_item() {
                Some(item) => items.push(item),
                None => self.sync_to_item(),
            }
        }

        ParseOutput {
            program: Program { items },
            diagnostics: self.diagnostics,
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
        let token = if self.check(TokenKind::Identifier) {
            self.advance()
        } else {
            self.error_at_current("P0002", "expected a type name", &["type name"]);
            self.advance()
        };

        TypeRef {
            name: token.lexeme,
            span: token.span,
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
            _ => {
                self.diagnostics.push(
                    Diagnostic::new("P0003", "expected an expression", self.source, token.span)
                        .with_expected("expression"),
                );
                Expr {
                    span: token.span,
                    kind: ExprKind::Error,
                }
            }
        }
    }

    fn parse_name_or_struct_literal(&mut self, name: Token) -> Expr {
        if !self.check(TokenKind::LBrace) {
            return Expr {
                span: name.span,
                kind: ExprKind::Name { value: name.lexeme },
            };
        }

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
            span: Span::new(name.span.start, close.span.end),
            kind: ExprKind::StructLiteral {
                name: name.lexeme,
                fields,
            },
        }
    }

    fn current_binary_op(&self) -> Option<(BinaryOp, u8)> {
        match self.peek().kind {
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
            _ => None,
        }
    }

    fn sync_to_item(&mut self) {
        while !self.is_at_end() {
            match self.peek().kind {
                TokenKind::FnKw | TokenKind::StructKw | TokenKind::EnumKw => break,
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

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }
}

fn enrich_parse_error(mut diagnostic: Diagnostic, token: &Token, message: &str) -> Diagnostic {
    diagnostic = diagnostic.with_note(format!("found {} instead", describe_token(token)));

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
            "use `bool`, `i32`, `f32`, `string`, or a previously declared type name",
        );
    }

    if message == "expected an expression" {
        return diagnostic.with_suggestion(
            "insert a runtime expression such as a literal, name, call, or parenthesized expression",
        );
    }

    diagnostic
}

fn describe_token(token: &Token) -> String {
    match token.kind {
        TokenKind::Eof => "the end of file".to_string(),
        TokenKind::Semicolon => "`;`".to_string(),
        TokenKind::RParen => "`)`".to_string(),
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
    use super::parse;
    use crate::ast::{ExprKind, ItemKind, StmtKind};
    use crate::lexer::tokenize;
    use crate::source::SourceFile;

    #[test]
    fn parses_minimal_main() {
        let source = SourceFile::anonymous("fn main() -> i32 { return 0; }");
        let tokens = tokenize(&source).tokens;
        let output = parse(&source, tokens);
        assert!(output.diagnostics.is_empty());
        assert_eq!(output.program.items.len(), 1);
        match &output.program.items[0].kind {
            ItemKind::Function { name, body, .. } => {
                assert_eq!(name, "main");
                assert_eq!(body.statements.len(), 1);
            }
            _ => panic!("expected function"),
        }
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
        assert_eq!(
            diagnostic.suggestion.as_deref(),
            Some("insert `)` to close the current parenthesized construct")
        );
    }
}
