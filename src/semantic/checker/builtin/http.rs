use super::*;

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_http_get_builtin(&mut self, expr: &Expr, arguments: &[Expr]) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, "http_get", 1, argument_types.len()) {
            return Type::Error;
        }

        self.expect_builtin_argument_type(
            expr,
            "http_get",
            "url",
            &Type::String,
            &argument_types[0],
        );
        Type::Struct("std.http.HttpResponse".to_string())
    }
}
