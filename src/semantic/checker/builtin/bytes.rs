use super::*;

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_bytes_empty_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        if !self.check_builtin_no_arguments(expr, "bytes_empty", arguments) {
            return Type::Error;
        }
        Type::Bytes
    }

    pub(super) fn check_bytes_from_string_builtin(
        &mut self,
        expr: &Expr,
        arguments: &[Expr],
    ) -> Type {
        self.check_bytes_string_builtin(expr, "bytes_from_string", arguments, Type::Bytes)
    }

    pub(super) fn check_bytes_to_string_lossy_builtin(
        &mut self,
        expr: &Expr,
        arguments: &[Expr],
    ) -> Type {
        self.check_bytes_value_builtin(expr, "bytes_to_string_lossy", arguments, Type::String)
    }

    pub(super) fn check_bytes_to_hex_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        self.check_bytes_value_builtin(expr, "bytes_to_hex", arguments, Type::String)
    }

    pub(super) fn check_bytes_push_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, "bytes_push", 2, argument_types.len()) {
            return Type::Error;
        }

        self.expect_builtin_argument_type(
            expr,
            "bytes_push",
            "data",
            &Type::Bytes,
            &argument_types[0],
        );
        self.expect_builtin_argument_type(
            expr,
            "bytes_push",
            "value",
            &Type::I32,
            &argument_types[1],
        );
        Type::Bytes
    }

    pub(super) fn check_bytes_get_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, "bytes_get", 2, argument_types.len()) {
            return Type::Error;
        }

        self.expect_builtin_argument_type(
            expr,
            "bytes_get",
            "data",
            &Type::Bytes,
            &argument_types[0],
        );
        self.expect_builtin_argument_type(
            expr,
            "bytes_get",
            "index",
            &Type::I32,
            &argument_types[1],
        );
        Type::I32
    }

    fn check_bytes_string_builtin(
        &mut self,
        expr: &Expr,
        name: &str,
        arguments: &[Expr],
        return_type: Type,
    ) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, name, 1, argument_types.len()) {
            return Type::Error;
        }

        self.expect_builtin_argument_type(expr, name, "text", &Type::String, &argument_types[0]);
        return_type
    }

    fn check_bytes_value_builtin(
        &mut self,
        expr: &Expr,
        name: &str,
        arguments: &[Expr],
        return_type: Type,
    ) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, name, 1, argument_types.len()) {
            return Type::Error;
        }

        self.expect_builtin_argument_type(expr, name, "data", &Type::Bytes, &argument_types[0]);
        return_type
    }
}
