use super::*;

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_fs_is_file_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        self.check_fs_path_builtin(expr, "fs_is_file", arguments, Type::Bool)
    }

    pub(super) fn check_fs_is_dir_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        self.check_fs_path_builtin(expr, "fs_is_dir", arguments, Type::Bool)
    }

    pub(super) fn check_fs_exists_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        self.check_fs_path_builtin(expr, "fs_exists", arguments, Type::Bool)
    }

    pub(super) fn check_fs_file_size_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        self.check_fs_path_builtin(expr, "fs_file_size", arguments, Type::I32)
    }

    pub(super) fn check_fs_copy_file_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        self.check_fs_two_path_builtin(
            expr,
            "fs_copy_file",
            arguments,
            "source_path",
            "destination_path",
            Type::I32,
        )
    }

    pub(super) fn check_fs_rename_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        self.check_fs_two_path_builtin(
            expr,
            "fs_rename",
            arguments,
            "source_path",
            "destination_path",
            Type::Void,
        )
    }

    pub(super) fn check_fs_create_dir_all_builtin(
        &mut self,
        expr: &Expr,
        arguments: &[Expr],
    ) -> Type {
        self.check_fs_path_builtin(expr, "fs_create_dir_all", arguments, Type::Void)
    }

    pub(super) fn check_fs_remove_file_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        self.check_fs_path_builtin(expr, "fs_remove_file", arguments, Type::Void)
    }

    pub(super) fn check_fs_remove_dir_all_builtin(
        &mut self,
        expr: &Expr,
        arguments: &[Expr],
    ) -> Type {
        self.check_fs_path_builtin(expr, "fs_remove_dir_all", arguments, Type::Void)
    }

    pub(super) fn check_fs_read_dir_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        self.check_fs_path_builtin(
            expr,
            "fs_read_dir",
            arguments,
            Type::Slice {
                element: Box::new(Type::String),
            },
        )
    }

    pub(super) fn check_fs_read_to_string_builtin(
        &mut self,
        expr: &Expr,
        arguments: &[Expr],
    ) -> Type {
        self.check_fs_path_builtin(expr, "fs_read_to_string", arguments, Type::String)
    }

    pub(super) fn check_fs_write_string_builtin(
        &mut self,
        expr: &Expr,
        arguments: &[Expr],
    ) -> Type {
        self.check_fs_two_path_builtin(
            expr,
            "fs_write_string",
            arguments,
            "path",
            "text",
            Type::Void,
        )
    }

    fn check_fs_path_builtin(
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

    fn check_fs_two_path_builtin(
        &mut self,
        expr: &Expr,
        name: &str,
        arguments: &[Expr],
        first_argument: &str,
        second_argument: &str,
        return_type: Type,
    ) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, name, 2, argument_types.len()) {
            return Type::Error;
        }

        self.expect_builtin_argument_type(
            expr,
            name,
            first_argument,
            &Type::String,
            &argument_types[0],
        );
        self.expect_builtin_argument_type(
            expr,
            name,
            second_argument,
            &Type::String,
            &argument_types[1],
        );
        return_type
    }
}
