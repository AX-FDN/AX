use super::*;

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_string_list_new_builtin(
        &mut self,
        expr: &Expr,
        arguments: &[Expr],
    ) -> Type {
        if !self.check_builtin_no_arguments(expr, "string_list_new", arguments) {
            return Type::Error;
        }

        Type::StringList
    }

    pub(super) fn check_string_list_push_builtin(
        &mut self,
        expr: &Expr,
        arguments: &[Expr],
    ) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, "string_list_push", 2, argument_types.len()) {
            return Type::Error;
        }

        self.expect_builtin_argument_type(
            expr,
            "string_list_push",
            "list",
            &Type::StringList,
            &argument_types[0],
        );
        self.expect_builtin_argument_type(
            expr,
            "string_list_push",
            "value",
            &Type::String,
            &argument_types[1],
        );
        Type::StringList
    }

    pub(super) fn check_string_list_join_builtin(
        &mut self,
        expr: &Expr,
        arguments: &[Expr],
    ) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, "string_list_join", 2, argument_types.len()) {
            return Type::Error;
        }

        self.expect_builtin_argument_type(
            expr,
            "string_list_join",
            "list",
            &Type::StringList,
            &argument_types[0],
        );
        self.expect_builtin_argument_type(
            expr,
            "string_list_join",
            "separator",
            &Type::String,
            &argument_types[1],
        );
        Type::String
    }

    pub(super) fn check_string_list_get_builtin(
        &mut self,
        expr: &Expr,
        arguments: &[Expr],
    ) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, "string_list_get", 2, argument_types.len()) {
            return Type::Error;
        }

        self.expect_builtin_argument_type(
            expr,
            "string_list_get",
            "list",
            &Type::StringList,
            &argument_types[0],
        );
        self.expect_builtin_argument_type(
            expr,
            "string_list_get",
            "index",
            &Type::I32,
            &argument_types[1],
        );
        Type::String
    }
}
