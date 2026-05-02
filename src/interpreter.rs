use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::hir::{
    BinaryOp, Block, EnumVariantPayloadPattern as MatchPatternPayload, Expr, ExprKind, ItemKind,
    MatchExprArm, MatchPattern, MatchPatternKind, Param, Place, PlaceKind, Program, Stmt, StmtKind,
    UnaryOp,
};
use crate::source::{SourceFile, Span};

mod builtins;
mod flow;
mod frame;
mod host;
mod value;

use self::flow::{ConditionFlow, ControlFlow, EvalFlow};
use self::frame::{Frame, Slot, lookup_slot, lookup_slot_mut};
pub use self::host::RunContext;
use self::value::Value;

#[derive(Debug)]
pub struct RunOutput {
    pub exit_code: i32,
    pub stdout: Vec<String>,
}

pub fn run_program(source: &SourceFile, program: &Program) -> Result<RunOutput, Diagnostic> {
    run_program_with_context(source, program, RunContext::default())
}

pub fn run_program_with_context(
    source: &SourceFile,
    program: &Program,
    context: RunContext,
) -> Result<RunOutput, Diagnostic> {
    Interpreter::new(source, program, context)?.run_main()
}

struct Interpreter<'a> {
    source: &'a SourceFile,
    functions: HashMap<String, FunctionDef<'a>>,
    constants: HashMap<String, Value>,
    stdout: Vec<String>,
    host: RunContext,
}

#[derive(Clone, Copy)]
struct FunctionDef<'a> {
    name: &'a str,
    params: &'a [Param],
    body: &'a Block,
    span: Span,
}

fn collect_left_associative_binary_operands<'a>(
    expr: &'a Expr,
    op: BinaryOp,
    operands: &mut Vec<&'a Expr>,
) {
    match &expr.kind {
        ExprKind::Binary {
            op: current_op,
            left,
            right,
        } if *current_op == op => {
            collect_left_associative_binary_operands(left, op, operands);
            operands.push(right);
        }
        _ => operands.push(expr),
    }
}
mod assignment;
mod binary;
mod collections;
mod expressions;
mod init;
mod matches;
mod runtime;
mod statements;

fn place_root_name<'a>(place: &'a Place) -> &'a str {
    match &place.kind {
        PlaceKind::Local { name } => name.as_str(),
        PlaceKind::Field { base, .. } | PlaceKind::Index { base, .. } => place_root_name(base),
    }
}

#[cfg(test)]
mod tests;
