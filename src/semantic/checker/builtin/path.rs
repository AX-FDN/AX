use super::*;

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_path_join_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, "path_join", 2, argument_types.len()) {
            return Type::Error;
        }

        self.expect_builtin_argument_type(
            expr,
            "path_join",
            "left",
            &Type::String,
            &argument_types[0],
        );
        self.expect_builtin_argument_type(
            expr,
            "path_join",
            "right",
            &Type::String,
            &argument_types[1],
        );
        Type::String
    }

    pub(super) fn check_path_parent_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        self.check_path_string_builtin(expr, "path_parent", arguments, Type::String)
    }

    pub(super) fn check_path_resolve_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        self.check_path_string_builtin(expr, "path_resolve", arguments, Type::String)
    }

    pub(super) fn check_path_file_name_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        self.check_path_string_builtin(expr, "path_file_name", arguments, Type::String)
    }

    pub(super) fn check_path_stem_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        self.check_path_string_builtin(expr, "path_stem", arguments, Type::String)
    }

    pub(super) fn check_path_extension_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        self.check_path_string_builtin(expr, "path_extension", arguments, Type::String)
    }

    pub(super) fn check_path_is_absolute_builtin(
        &mut self,
        expr: &Expr,
        arguments: &[Expr],
    ) -> Type {
        self.check_path_string_builtin(expr, "path_is_absolute", arguments, Type::Bool)
    }

    fn check_path_string_builtin(
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

        self.expect_builtin_argument_type(expr, name, "path", &Type::String, &argument_types[0]);
        return_type
    }
}
