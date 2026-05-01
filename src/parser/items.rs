use super::*;

impl<'a> Parser<'a> {
    pub(super) fn parse_source_unit(
        &mut self,
        path: &str,
        span: Span,
        items: &mut Vec<Item>,
    ) -> SourceUnit {
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

    pub(super) fn parse_module_decl(&mut self) -> ModuleDecl {
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

    pub(super) fn parse_import_decl(&mut self) -> ImportDecl {
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

    pub(super) fn parse_item(&mut self) -> Option<Item> {
        let start = self.peek().span.start;
        let visibility = if self.matches(&[TokenKind::PubKw]) {
            Visibility::Public
        } else {
            Visibility::Private
        };
        let start = if visibility == Visibility::Public {
            start
        } else {
            self.peek().span.start
        };
        match self.peek().kind {
            TokenKind::FnKw => {
                self.advance();
                Some(self.parse_function_item(start, visibility))
            }
            TokenKind::ConstKw => {
                self.advance();
                Some(self.parse_const_item(start, visibility))
            }
            TokenKind::TypeKw => {
                self.advance();
                Some(self.parse_type_alias_item(start, visibility))
            }
            TokenKind::StructKw => {
                self.advance();
                Some(self.parse_struct_item(start, visibility))
            }
            TokenKind::EnumKw => {
                self.advance();
                Some(self.parse_enum_item(start, visibility))
            }
            TokenKind::TraitKw => {
                self.advance();
                Some(self.parse_trait_item(start, visibility))
            }
            TokenKind::ImplKw => {
                self.advance();
                Some(self.parse_impl_item(start, visibility))
            }
            TokenKind::Eof => None,
            _ => {
                self.error_at_current(
                    "P0001",
                    "expected a top-level declaration",
                    &[
                        "`pub`", "`fn`", "`const`", "`type`", "`struct`", "`enum`", "`trait`",
                        "`impl`",
                    ],
                );
                None
            }
        }
    }

    pub(super) fn parse_const_item(&mut self, start: usize, visibility: Visibility) -> Item {
        let name = self.expect_identifier("expected a constant name");
        self.expect(
            TokenKind::Colon,
            "expected `:` after constant name",
            &["`:`"],
        );
        let ty = self.parse_type();
        self.expect(
            TokenKind::Equal,
            "expected `=` before constant value",
            &["`=`"],
        );
        let value = self.parse_expression();
        let end = self.expect(
            TokenKind::Semicolon,
            "expected `;` after constant declaration",
            &["`;`"],
        );
        Item {
            kind: ItemKind::Const {
                name: name.lexeme,
                ty,
                value,
            },
            visibility,
            span: Span::new(start, end.span.end),
        }
    }

    pub(super) fn parse_type_alias_item(&mut self, start: usize, visibility: Visibility) -> Item {
        let name = self.expect_identifier("expected a type alias name");
        let type_params = self.parse_type_params();
        self.expect(
            TokenKind::Equal,
            "expected `=` before type alias target",
            &["`=`"],
        );
        let target = self.parse_type();
        let end = self.expect(
            TokenKind::Semicolon,
            "expected `;` after type alias declaration",
            &["`;`"],
        );
        Item {
            kind: ItemKind::TypeAlias {
                name: name.lexeme,
                type_params,
                target,
            },
            visibility,
            span: Span::new(start, end.span.end),
        }
    }

    pub(super) fn parse_function_item(&mut self, start: usize, visibility: Visibility) -> Item {
        let name = self.expect_identifier("expected a function name");
        let (type_params, type_param_bounds) = self.parse_function_type_params();
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
        let mut type_param_bounds = type_param_bounds;
        type_param_bounds.extend(self.parse_where_bounds(&type_params));
        let body = self.parse_block();
        Item {
            kind: ItemKind::Function {
                name: name.lexeme,
                type_params,
                type_param_bounds,
                params,
                return_type,
                body: body.clone(),
            },
            visibility,
            span: Span::new(start, body.span.end),
        }
    }

    pub(super) fn parse_function_type_params(&mut self) -> (Vec<String>, Vec<TypeParamBound>) {
        if !self.matches(&[TokenKind::Less]) {
            return (Vec::new(), Vec::new());
        }

        let mut params = Vec::new();
        let mut bounds = Vec::new();
        loop {
            let param = self.expect_identifier("expected a generic type parameter name");
            let param_name = param.lexeme;
            if self.matches(&[TokenKind::Colon]) {
                loop {
                    let trait_ref = self.parse_type();
                    bounds.push(TypeParamBound {
                        type_param: param_name.clone(),
                        span: Span::new(param.span.start, trait_ref.span.end),
                        trait_ref,
                    });
                    if !self.matches(&[TokenKind::Plus]) {
                        break;
                    }
                }
            }
            params.push(param_name);

            if !self.matches(&[TokenKind::Comma]) {
                break;
            }
        }

        self.expect(
            TokenKind::Greater,
            "expected `>` after generic type parameters",
            &["`>`"],
        );
        (params, bounds)
    }

    pub(super) fn parse_where_bounds(&mut self, type_params: &[String]) -> Vec<TypeParamBound> {
        if !self.matches(&[TokenKind::WhereKw]) {
            return Vec::new();
        }

        let mut bounds = Vec::new();
        loop {
            let param =
                self.expect_identifier("expected a generic type parameter name after `where`");
            let param_name = param.lexeme;
            if !type_params.iter().any(|candidate| candidate == &param_name) {
                self.diagnostics.push(
                    Diagnostic::new(
                        "P0002",
                        format!("where clause references undeclared type parameter `{param_name}`"),
                        self.source,
                        param.span,
                    )
                    .with_suggestion("use a type parameter declared in the preceding `<...>` list"),
                );
            }
            self.expect(
                TokenKind::Colon,
                "expected `:` after where type parameter",
                &["`:`"],
            );
            loop {
                let trait_ref = self.parse_type();
                bounds.push(TypeParamBound {
                    type_param: param_name.clone(),
                    span: Span::new(param.span.start, trait_ref.span.end),
                    trait_ref,
                });
                if !self.matches(&[TokenKind::Plus]) {
                    break;
                }
            }

            if !self.matches(&[TokenKind::Comma]) {
                break;
            }
        }

        bounds
    }

    pub(super) fn parse_struct_item(&mut self, start: usize, visibility: Visibility) -> Item {
        let name = self.expect_identifier("expected a struct name");
        let type_params = self.parse_type_params();
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
                type_params,
                fields,
            },
            visibility,
            span: Span::new(start, end.span.end),
        }
    }

    pub(super) fn parse_type_params(&mut self) -> Vec<String> {
        if !self.matches(&[TokenKind::Less]) {
            return Vec::new();
        }

        let mut params = Vec::new();
        loop {
            let param = self.expect_identifier("expected a generic type parameter name");
            params.push(param.lexeme);

            if !self.matches(&[TokenKind::Comma]) {
                break;
            }
        }

        self.expect(
            TokenKind::Greater,
            "expected `>` after generic type parameters",
            &["`>`"],
        );
        params
    }

    pub(super) fn parse_enum_item(&mut self, start: usize, visibility: Visibility) -> Item {
        let name = self.expect_identifier("expected an enum name");
        let type_params = self.parse_type_params();
        self.expect(TokenKind::LBrace, "expected `{` after enum name", &["`{`"]);
        let mut variants = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            let variant = self.expect_identifier("expected an enum variant");
            let payload = if self.matches(&[TokenKind::LParen]) {
                let payload = self.parse_type();
                self.expect(
                    TokenKind::RParen,
                    "expected `)` after enum variant payload type",
                    &["`)`"],
                );
                Some(payload)
            } else {
                None
            };
            variants.push(EnumVariant {
                name: variant.lexeme,
                payload,
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
                type_params,
                variants,
            },
            visibility,
            span: Span::new(start, end.span.end),
        }
    }

    pub(super) fn parse_impl_item(&mut self, start: usize, visibility: Visibility) -> Item {
        let type_params = self.parse_type_params();
        let first_type = self.parse_type();
        let (trait_ref, target) = if self.matches(&[TokenKind::ForKw]) {
            (Some(first_type), self.parse_type())
        } else {
            (None, first_type)
        };
        self.expect(
            TokenKind::LBrace,
            "expected `{` after impl target",
            &["`{`"],
        );
        let mut methods = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            let method_start = self.peek().span.start;
            self.expect(
                TokenKind::FnKw,
                "expected `fn` inside impl block",
                &["`fn`"],
            );
            methods.push(self.parse_impl_method(method_start));
        }
        let end = self.expect(TokenKind::RBrace, "expected `}` after impl body", &["`}`"]);
        Item {
            kind: ItemKind::Impl {
                type_params,
                trait_ref,
                target,
                methods,
            },
            visibility,
            span: Span::new(start, end.span.end),
        }
    }

    pub(super) fn parse_trait_item(&mut self, start: usize, visibility: Visibility) -> Item {
        let name = self.expect_identifier("expected a trait name");
        self.expect(TokenKind::LBrace, "expected `{` after trait name", &["`{`"]);
        let mut methods = Vec::new();
        while !self.check(TokenKind::RBrace) && !self.is_at_end() {
            let method_start = self.peek().span.start;
            self.expect(
                TokenKind::FnKw,
                "expected `fn` inside trait block",
                &["`fn`"],
            );
            methods.push(self.parse_trait_method(method_start));
        }
        let end = self.expect(TokenKind::RBrace, "expected `}` after trait body", &["`}`"]);
        Item {
            kind: ItemKind::Trait {
                name: name.lexeme,
                methods,
            },
            visibility,
            span: Span::new(start, end.span.end),
        }
    }

    pub(super) fn parse_trait_method(&mut self, start: usize) -> TraitMethod {
        let name = self.expect_identifier("expected a trait method name");
        self.expect(
            TokenKind::LParen,
            "expected `(` after trait method name",
            &["`(`"],
        );
        let params = self.parse_params();
        self.expect(TokenKind::RParen, "expected `)` after parameters", &["`)`"]);
        self.expect(
            TokenKind::Arrow,
            "expected `->` before trait method return type",
            &["`->`"],
        );
        let return_type = self.parse_type();
        let end = self.expect(
            TokenKind::Semicolon,
            "expected `;` after trait method signature",
            &["`;`"],
        );
        TraitMethod {
            name: name.lexeme,
            params,
            return_type,
            span: Span::new(start, end.span.end),
        }
    }

    pub(super) fn parse_impl_method(&mut self, start: usize) -> ImplMethod {
        let name = self.expect_identifier("expected a method name");
        let type_params = self.parse_type_params();
        self.expect(
            TokenKind::LParen,
            "expected `(` after method name",
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
        ImplMethod {
            name: name.lexeme,
            type_params,
            params,
            return_type,
            body: body.clone(),
            span: Span::new(start, body.span.end),
        }
    }

    pub(super) fn parse_params(&mut self) -> Vec<Param> {
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

    pub(super) fn parse_type(&mut self) -> TypeRef {
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

        let (name, mut span) = self.finish_qualified_identifier_path(
            token,
            "expected an identifier after `.` in type path",
        );

        let type_args = self.parse_type_args();
        if let Some(last_arg) = type_args.last() {
            span = Span::new(span.start, last_arg.span.end);
        }

        TypeRef::named_with_args(name, type_args, span)
    }

    pub(super) fn parse_type_args(&mut self) -> Vec<TypeRef> {
        if !self.matches(&[TokenKind::Less]) {
            return Vec::new();
        }

        let mut args = Vec::new();
        loop {
            args.push(self.parse_type());
            if !self.matches(&[TokenKind::Comma]) {
                break;
            }
        }

        self.expect(
            TokenKind::Greater,
            "expected `>` after generic type arguments",
            &["`>`"],
        );
        args
    }
}
