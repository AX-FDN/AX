use super::*;

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_net_tcp_exchange_builtin(
        &mut self,
        expr: &Expr,
        arguments: &[Expr],
    ) -> Type {
        let argument_types = self.check_builtin_argument_types(arguments);

        if !self.check_builtin_arity(expr, "net_tcp_exchange", 3, argument_types.len()) {
            return Type::Error;
        }

        self.expect_builtin_argument_type(
            expr,
            "net_tcp_exchange",
            "host",
            &Type::String,
            &argument_types[0],
        );
        self.expect_builtin_argument_type(
            expr,
            "net_tcp_exchange",
            "port",
            &Type::I32,
            &argument_types[1],
        );
        self.expect_builtin_argument_type(
            expr,
            "net_tcp_exchange",
            "request",
            &Type::String,
            &argument_types[2],
        );
        Type::Struct("std.net.TcpResponse".to_string())
    }
}
