use crate::ast::{Stmt, StmtKind};

use super::TypeChecker;

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_statement(&mut self, statement: &Stmt) {
        match &statement.kind {
            StmtKind::Let {
                mutable,
                name,
                ty,
                initializer,
            } => {
                let current_unit_path = self.current_unit_path().to_string();
                let declared_type =
                    self.info
                        .resolve_type_ref(ty, &current_unit_path, self.diagnostics);
                let initializer_type = self.check_expr(initializer);
                self.expect_type_match(
                    &declared_type,
                    &initializer_type,
                    initializer.span,
                    format!(
                        "cannot initialize `{name}` of type `{}` with `{}`",
                        declared_type.describe(),
                        initializer_type.describe()
                    ),
                );
                self.declare(name, declared_type, *mutable, statement.span.start);
            }
            StmtKind::Assign { target, value } => {
                let value_type = self.check_expr(value);
                self.check_assignment_target(target, &value_type, value.span);
            }
            StmtKind::Break => self.check_break_statement(statement),
            StmtKind::Continue => self.check_continue_statement(statement),
            StmtKind::Match { scrutinee, arms } => {
                self.check_match_statement(statement, scrutinee, arms)
            }
            StmtKind::Expr { expr } => {
                self.check_expr(expr);
            }
            StmtKind::Return { value } => self.check_return_statement(statement, value.as_ref()),
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => self.check_if_statement(condition, then_branch, else_branch.as_ref()),
            StmtKind::While { condition, body } => self.check_while_statement(condition, body),
            StmtKind::For {
                initializer,
                condition,
                step,
                body,
            } => {
                self.check_for_statement(
                    initializer.as_deref(),
                    condition.as_ref(),
                    step.as_deref(),
                    body,
                );
            }
            StmtKind::Block { block } => self.check_block(block),
        }
    }
}
