use std::collections::HashMap;

use crate::hir::{self};
pub use crate::hir::{
    BinaryOp, EnumVariant, EnumVariantPayloadPattern, StructField, Type, TypeParamBound, UnaryOp,
};
use crate::source::Span;

mod model;
pub use model::*;

pub fn lower_program(program: &hir::Program) -> Result<Program, String> {
    Ok(Program {
        items: program
            .items
            .iter()
            .map(lower_item)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn lower_item(item: &hir::Item) -> Result<Item, String> {
    let kind = match &item.kind {
        hir::ItemKind::Function {
            name,
            type_params,
            type_param_bounds,
            params,
            return_type,
            body,
        } => {
            let mut lowerer = FunctionLowerer::new();
            let params = lowerer.lower_params(params);
            let entry_block = lowerer.new_block(body.span);
            let exit_block = lowerer.lower_block(body, entry_block)?;
            if !lowerer.block_is_terminated(exit_block) {
                lowerer.set_terminator(
                    exit_block,
                    Terminator {
                        kind: TerminatorKind::Unreachable,
                        span: body.span,
                    },
                );
            }
            let (locals, blocks) = lowerer.finish();

            ItemKind::Function {
                name: name.clone(),
                type_params: type_params.clone(),
                type_param_bounds: type_param_bounds.clone(),
                params,
                return_type: return_type.clone(),
                locals,
                entry_block,
                blocks,
            }
        }
        hir::ItemKind::Const { name, ty, value } => {
            let mut lowerer = FunctionLowerer::new();
            ItemKind::Const {
                name: name.clone(),
                ty: ty.clone(),
                value: lowerer.lower_expr(value)?,
            }
        }
        hir::ItemKind::Struct {
            name,
            type_params,
            fields,
        } => ItemKind::Struct {
            name: name.clone(),
            type_params: type_params.clone(),
            fields: fields.clone(),
        },
        hir::ItemKind::Enum {
            name,
            type_params,
            variants,
        } => ItemKind::Enum {
            name: name.clone(),
            type_params: type_params.clone(),
            variants: variants.clone(),
        },
    };

    Ok(Item {
        kind,
        visibility: item.visibility,
        span: item.span,
    })
}

struct FunctionLowerer {
    locals: Vec<Local>,
    scopes: Vec<HashMap<String, u32>>,
    blocks: Vec<BasicBlockBuilder>,
    loop_stack: Vec<LoopTargets>,
}

struct BasicBlockBuilder {
    id: u32,
    span: Span,
    statements: Vec<Statement>,
    terminator: Option<Terminator>,
}

#[derive(Clone, Copy)]
struct LoopTargets {
    break_target: u32,
    continue_target: u32,
}

#[path = "mir/builder.rs"]
mod builder;
#[path = "mir/expressions.rs"]
mod expressions;
#[path = "mir/matches.rs"]
mod matches;
#[path = "mir/statements.rs"]
mod statements;

fn goto(target: u32, span: Span) -> Terminator {
    Terminator {
        kind: TerminatorKind::Goto { target },
        span,
    }
}

#[cfg(test)]
#[path = "mir/tests.rs"]
mod tests;
