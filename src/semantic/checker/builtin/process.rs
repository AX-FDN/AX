use super::*;

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_process_cwd_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        if !self.check_builtin_no_arguments(expr, "process_cwd", arguments) {
            return Type::Error;
        }

        Type::String
    }

    pub(super) fn check_process_run_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        self.check_process_command_builtin(expr, "process_run", arguments, Type::I32)
    }

    pub(super) fn check_process_capture_builtin(
        &mut self,
        expr: &Expr,
        arguments: &[Expr],
    ) -> Type {
        self.check_process_command_builtin(expr, "process_capture", arguments, Type::String)
    }

    pub(super) fn check_process_run_in_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        self.check_process_in_builtin(expr, "process_run_in", arguments, Type::I32)
    }

    pub(super) fn check_process_capture_in_builtin(
        &mut self,
        expr: &Expr,
        arguments: &[Expr],
    ) -> Type {
        self.check_process_in_builtin(expr, "process_capture_in", arguments, Type::String)
    }

    fn check_process_command_builtin(
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

        self.expect_builtin_argument_type(expr, name, "command", &Type::String, &argument_types[0]);
        return_type
    }

    fn check_process_in_builtin(
        &mut self,
        expr: &Expr,
        name: &str,
        arguments: &[Expr],
        return_type: Type,
    ) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, name, 2, argument_types.len()) {
            return Type::Error;
        }

        self.expect_builtin_argument_type(
            expr,
            name,
            "working_dir",
            &Type::String,
            &argument_types[0],
        );
        self.expect_builtin_argument_type(expr, name, "command", &Type::String, &argument_types[1]);
        return_type
    }
}
