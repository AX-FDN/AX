use std::collections::HashMap;

use crate::ast::{Block, Stmt, StmtKind};
use crate::diagnostics::Diagnostic;
use crate::source::Span;

#[path = "checker/assignment.rs"]
mod assignment;
#[path = "checker/control_flow.rs"]
mod control_flow;
#[path = "checker/expr.rs"]
mod expr;
#[path = "checker/names.rs"]
mod names;

pub(super) use super::helpers::{binary_op_name, type_name_as_value_diagnostic};
use super::helpers::{return_type_message, type_mismatch_suggestion};
use super::program_info::ProgramInfo;
use super::types::Type;
use names::Binding;

pub(super) struct TypeChecker<'a, 'b> {
    info: &'a ProgramInfo<'a>,
    return_type: Type,
    scopes: Vec<HashMap<String, Binding>>,
    diagnostics: &'b mut Vec<Diagnostic>,
}

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn new(
        info: &'a ProgramInfo<'a>,
        return_type: Type,
        diagnostics: &'b mut Vec<Diagnostic>,
    ) -> Self {
        Self {
            info,
            return_type,
            scopes: vec![HashMap::new()],
            diagnostics,
        }
    }

    pub(super) fn diagnostics_mut(&mut self) -> &mut Vec<Diagnostic> {
        self.diagnostics
    }

    pub(super) fn return_type(&self) -> &Type {
        &self.return_type
    }

    pub(super) fn check_block(&mut self, block: &Block) {
        self.scopes.push(HashMap::new());
        for statement in &block.statements {
            self.check_statement(statement);
        }
        self.scopes.pop();
    }

    fn check_statement(&mut self, statement: &Stmt) {
        match &statement.kind {
            StmtKind::Let {
                mutable,
                name,
                ty,
                initializer,
            } => {
                let declared_type = self.info.resolve_type_ref(ty, self.diagnostics);
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

    fn expect_type_match(&mut self, expected: &Type, actual: &Type, span: Span, message: String) {
        if expected.is_error() || actual.is_error() || expected == actual {
            return;
        }

        self.diagnostics.push(
            Diagnostic::new("S0022", message, self.info.source, span)
                .with_note(format!(
                    "AX does not implicitly convert `{}` to `{}`",
                    actual.describe(),
                    expected.describe()
                ))
                .with_suggestion(type_mismatch_suggestion(expected, actual)),
        );
    }
}
