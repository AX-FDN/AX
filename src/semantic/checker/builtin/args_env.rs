use super::*;

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_argv_len_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        if !self.check_builtin_no_arguments(expr, "argv_len", arguments) {
            return Type::Error;
        }

        Type::I32
    }

    pub(super) fn check_argv_get_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, "argv_get", 1, argument_types.len()) {
            return Type::Error;
        }

        self.expect_builtin_argument_type(
            expr,
            "argv_get",
            "index",
            &Type::I32,
            &argument_types[0],
        );
        Type::String
    }

    pub(super) fn check_env_has_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, "env_has", 1, argument_types.len()) {
            return Type::Error;
        }

        self.expect_builtin_argument_type(
            expr,
            "env_has",
            "name",
            &Type::String,
            &argument_types[0],
        );
        Type::Bool
    }

    pub(super) fn check_env_get_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, "env_get", 1, argument_types.len()) {
            return Type::Error;
        }

        self.expect_builtin_argument_type(
            expr,
            "env_get",
            "name",
            &Type::String,
            &argument_types[0],
        );
        Type::String
    }
}
