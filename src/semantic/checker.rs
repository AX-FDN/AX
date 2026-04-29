use std::collections::HashMap;

use crate::ast::Block;
use crate::diagnostics::Diagnostic;

#[path = "checker/assignment.rs"]
mod assignment;
#[path = "checker/builtin.rs"]
mod builtin;
#[path = "checker/calls.rs"]
mod calls;
#[path = "checker/composite.rs"]
mod composite;
#[path = "checker/control_flow.rs"]
mod control_flow;
#[path = "checker/expr.rs"]
mod expr;
#[path = "checker/names.rs"]
mod names;
#[path = "checker/statements.rs"]
mod statements;
#[path = "checker/type_rules.rs"]
mod type_rules;

use super::helpers::return_type_message;
pub(super) use super::helpers::{
    binary_op_name, type_mismatch_suggestion, type_name_as_value_diagnostic,
};
use super::program_info::ProgramInfo;
use super::types::{Type, TypeParamBoundInfo};
use names::Binding;

pub(super) struct TypeChecker<'a, 'b> {
    info: &'a ProgramInfo<'a>,
    return_type: Type,
    current_unit_path: String,
    active_type_param_bounds: Vec<TypeParamBoundInfo>,
    expected_type: Option<Type>,
    scopes: Vec<HashMap<String, Binding>>,
    loop_depth: usize,
    diagnostics: &'b mut Vec<Diagnostic>,
}

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn new(
        info: &'a ProgramInfo<'a>,
        return_type: Type,
        current_unit_path: String,
        active_type_param_bounds: Vec<TypeParamBoundInfo>,
        diagnostics: &'b mut Vec<Diagnostic>,
    ) -> Self {
        Self {
            info,
            return_type,
            current_unit_path,
            active_type_param_bounds,
            expected_type: None,
            scopes: vec![HashMap::new()],
            loop_depth: 0,
            diagnostics,
        }
    }

    pub(super) fn diagnostics_mut(&mut self) -> &mut Vec<Diagnostic> {
        self.diagnostics
    }

    pub(super) fn return_type(&self) -> &Type {
        &self.return_type
    }

    pub(super) fn current_unit_path(&self) -> &str {
        &self.current_unit_path
    }

    pub(super) fn check_expr_with_expected(
        &mut self,
        expr: &crate::ast::Expr,
        expected: &Type,
    ) -> Type {
        let previous_expected = self.expected_type.replace(expected.clone());
        let ty = self.check_expr(expr);
        self.expected_type = previous_expected;
        ty
    }

    pub(super) fn take_expected_type(&mut self) -> Option<Type> {
        self.expected_type.take()
    }

    fn lookup_constant(&mut self, name: &str, span: crate::source::Span) -> Option<Binding> {
        let resolved = self.info.resolve_constant_key(
            name,
            &self.current_unit_path,
            span,
            self.diagnostics,
        )?;
        self.info.constants.get(&resolved).map(|constant| Binding {
            mutable: false,
            ty: constant.ty.clone(),
            start: constant.start,
        })
    }

    pub(super) fn check_block(&mut self, block: &Block) {
        self.scopes.push(HashMap::new());
        for statement in &block.statements {
            self.check_statement(statement);
        }
        self.scopes.pop();
    }
}
