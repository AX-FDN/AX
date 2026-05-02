use super::*;

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_println_builtin(&mut self, arguments: &[Expr]) -> Type {
        for argument in arguments {
            self.check_expr(argument);
        }

        Type::Void
    }
}
