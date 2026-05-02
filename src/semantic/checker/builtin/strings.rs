use super::*;

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_string_len_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, "string_len", 1, argument_types.len()) {
            return Type::Error;
        }

        self.expect_builtin_argument_type(
            expr,
            "string_len",
            "text",
            &Type::String,
            &argument_types[0],
        );
        Type::I32
    }

    pub(super) fn check_string_contains_builtin(
        &mut self,
        expr: &Expr,
        arguments: &[Expr],
    ) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, "string_contains", 2, argument_types.len()) {
            return Type::Error;
        }

        self.expect_builtin_argument_type(
            expr,
            "string_contains",
            "text",
            &Type::String,
            &argument_types[0],
        );
        self.expect_builtin_argument_type(
            expr,
            "string_contains",
            "needle",
            &Type::String,
            &argument_types[1],
        );
        Type::Bool
    }

    pub(super) fn check_string_starts_with_builtin(
        &mut self,
        expr: &Expr,
        arguments: &[Expr],
    ) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, "string_starts_with", 2, argument_types.len()) {
            return Type::Error;
        }

        self.expect_builtin_argument_type(
            expr,
            "string_starts_with",
            "text",
            &Type::String,
            &argument_types[0],
        );
        self.expect_builtin_argument_type(
            expr,
            "string_starts_with",
            "prefix",
            &Type::String,
            &argument_types[1],
        );
        Type::Bool
    }

    pub(super) fn check_string_ends_with_builtin(
        &mut self,
        expr: &Expr,
        arguments: &[Expr],
    ) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, "string_ends_with", 2, argument_types.len()) {
            return Type::Error;
        }

        self.expect_builtin_argument_type(
            expr,
            "string_ends_with",
            "text",
            &Type::String,
            &argument_types[0],
        );
        self.expect_builtin_argument_type(
            expr,
            "string_ends_with",
            "suffix",
            &Type::String,
            &argument_types[1],
        );
        Type::Bool
    }

    pub(super) fn check_string_replace_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, "string_replace", 3, argument_types.len()) {
            return Type::Error;
        }

        self.expect_builtin_argument_type(
            expr,
            "string_replace",
            "text",
            &Type::String,
            &argument_types[0],
        );
        self.expect_builtin_argument_type(
            expr,
            "string_replace",
            "from",
            &Type::String,
            &argument_types[1],
        );
        self.expect_builtin_argument_type(
            expr,
            "string_replace",
            "to",
            &Type::String,
            &argument_types[2],
        );
        Type::String
    }

    pub(super) fn check_string_trim_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, "string_trim", 1, argument_types.len()) {
            return Type::Error;
        }

        self.expect_builtin_argument_type(
            expr,
            "string_trim",
            "text",
            &Type::String,
            &argument_types[0],
        );
        Type::String
    }

    pub(super) fn check_string_split_lines_builtin(
        &mut self,
        expr: &Expr,
        arguments: &[Expr],
    ) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, "string_split_lines", 1, argument_types.len()) {
            return Type::Error;
        }

        self.expect_builtin_argument_type(
            expr,
            "string_split_lines",
            "text",
            &Type::String,
            &argument_types[0],
        );
        Type::Slice {
            element: Box::new(Type::String),
        }
    }
}
