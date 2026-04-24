use crate::ast::{Block, Expr, Stmt, StmtKind};
use crate::diagnostics::Diagnostic;

use super::{Type, TypeChecker, return_type_message};

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_break_statement(&mut self, statement: &Stmt) {
        if self.loop_depth > 0 {
            return;
        }

        self.diagnostics.push(
            Diagnostic::new(
                "S0036",
                "`break` may only be used inside `while` or `for` loops",
                self.info.source,
                statement.span,
            )
            .with_note("AX uses `break;` to exit the nearest enclosing loop early")
            .with_suggestion(
                "move `break;` into a loop body, or use `return ...;` to exit the function",
            ),
        );
    }

    pub(super) fn check_return_statement(&mut self, statement: &Stmt, value: Option<&Expr>) {
        let actual_type = match value {
            Some(expr) => self.check_expr(expr),
            None => Type::Void,
        };
        self.expect_type_match(
            &self.return_type.clone(),
            &actual_type,
            statement.span,
            return_type_message(&self.return_type, &actual_type),
        );
    }

    pub(super) fn check_if_statement(
        &mut self,
        condition: &Expr,
        then_branch: &Block,
        else_branch: Option<&Block>,
    ) {
        self.check_condition("if", condition);
        self.check_block(then_branch);
        if let Some(block) = else_branch {
            self.check_block(block);
        }
    }

    pub(super) fn check_while_statement(&mut self, condition: &Expr, body: &Block) {
        self.check_condition("while", condition);
        self.loop_depth += 1;
        self.check_block(body);
        self.loop_depth -= 1;
    }

    pub(super) fn check_for_statement(
        &mut self,
        initializer: Option<&Stmt>,
        condition: Option<&Expr>,
        step: Option<&Stmt>,
        body: &Block,
    ) {
        self.scopes.push(Default::default());

        if let Some(statement) = initializer {
            self.check_for_header_statement(statement);
        }

        if let Some(condition) = condition {
            self.check_condition("for", condition);
        }

        self.loop_depth += 1;
        self.check_block(body);
        self.loop_depth -= 1;

        if let Some(statement) = step {
            self.check_for_header_statement(statement);
        }

        self.scopes.pop();
    }

    fn check_for_header_statement(&mut self, statement: &Stmt) {
        match &statement.kind {
            StmtKind::Let { .. } | StmtKind::Assign { .. } | StmtKind::Expr { .. } => {
                self.check_statement(statement);
            }
            _ => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0031",
                        "`for` headers only support `let`, assignment, or expression clauses",
                        self.info.source,
                        statement.span,
                    )
                    .with_suggestion(
                        "use a header like `for (let i: i32 = 0; i < 3; i = i + 1) { ... }`",
                    ),
                );
            }
        }
    }

    fn check_condition(&mut self, keyword: &str, condition: &Expr) {
        let condition_type = self.check_expr(condition);
        self.expect_type_match(
            &Type::Bool,
            &condition_type,
            condition.span,
            format!(
                "`{keyword}` condition must be `bool`, found `{}`",
                condition_type.describe()
            ),
        );
    }
}
