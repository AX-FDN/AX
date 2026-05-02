use std::cell::Cell;
use std::collections::{HashMap, HashSet};

use crate::ast::{self};
use crate::diagnostics::Diagnostic;
use crate::source::{SourceFile, Span};

mod model;
pub use crate::ast::{BinaryOp, UnaryOp};
pub use model::*;

pub fn lower_program(source: &SourceFile, program: &ast::Program) -> Result<Program, Diagnostic> {
    LoweringContext::new(source, program).lower_program(program)
}

struct LoweringContext<'a> {
    source: &'a SourceFile,
    unit_modules: HashMap<String, String>,
    function_names: HashSet<String>,
    struct_names: HashSet<String>,
    enum_names: HashSet<String>,
    trait_names: HashSet<String>,
    type_aliases: HashMap<String, (Vec<String>, ast::TypeRef)>,
    struct_fields: HashMap<String, (Vec<String>, HashMap<String, Type>)>,
    enum_variant_payloads: HashMap<String, HashMap<String, Type>>,
    next_match_temp: Cell<u32>,
    next_for_in_temp: Cell<u32>,
}

#[path = "hir/expressions.rs"]
mod expressions;
#[path = "hir/init.rs"]
mod init;
#[path = "hir/items.rs"]
mod items;
#[path = "hir/loops.rs"]
mod loops;
#[path = "hir/matches.rs"]
mod matches;
#[path = "hir/names.rs"]
mod names;
#[path = "hir/statements.rs"]
mod statements;
#[path = "hir/types.rs"]
mod types;

fn canonical_item_name(
    source: &SourceFile,
    unit_modules: &HashMap<String, String>,
    item: &ast::Item,
) -> String {
    let unit_path = source.display_path_for_offset(item.span.start);
    match &item.kind {
        ast::ItemKind::Function { name, .. }
        | ast::ItemKind::Const { name, .. }
        | ast::ItemKind::TypeAlias { name, .. }
        | ast::ItemKind::Struct { name, .. }
        | ast::ItemKind::Enum { name, .. }
        | ast::ItemKind::Trait { name, .. } => unit_modules
            .get(unit_path)
            .map(|module_path| format!("{module_path}.{name}"))
            .unwrap_or_else(|| name.clone()),
        ast::ItemKind::Impl { target, .. } => target
            .direct_name()
            .map(|name| {
                unit_modules
                    .get(unit_path)
                    .map(|module_path| format!("{module_path}.{name}"))
                    .unwrap_or_else(|| name.to_string())
            })
            .unwrap_or_else(|| "<impl>".to_string()),
    }
}

fn looks_like_type_param(name: &str) -> bool {
    name.chars()
        .next()
        .map(|ch| ch.is_ascii_uppercase())
        .unwrap_or(false)
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn substitute_struct_field_type(
    field_type: Type,
    type_params: &[String],
    scrutinee_type: Option<&Type>,
) -> Type {
    let Some(Type::StructInstance { args, .. }) = scrutinee_type else {
        return field_type;
    };
    if type_params.len() != args.len() {
        return field_type;
    }
    let substitutions = type_params
        .iter()
        .cloned()
        .zip(args.iter().cloned())
        .collect::<HashMap<_, _>>();
    substitute_type_params(&field_type, &substitutions)
}

fn substitute_type_params(ty: &Type, substitutions: &HashMap<String, Type>) -> Type {
    match ty {
        Type::TypeParam { name } => substitutions
            .get(name)
            .cloned()
            .unwrap_or_else(|| ty.clone()),
        Type::Slice { element } => Type::Slice {
            element: Box::new(substitute_type_params(element, substitutions)),
        },
        Type::Array { element, length } => Type::Array {
            element: Box::new(substitute_type_params(element, substitutions)),
            length: *length,
        },
        Type::StructInstance { name, args } => Type::StructInstance {
            name: name.clone(),
            args: args
                .iter()
                .map(|arg| substitute_type_params(arg, substitutions))
                .collect(),
        },
        Type::EnumInstance { name, args } => Type::EnumInstance {
            name: name.clone(),
            args: args
                .iter()
                .map(|arg| substitute_type_params(arg, substitutions))
                .collect(),
        },
        _ => ty.clone(),
    }
}

#[cfg(test)]
#[path = "hir/tests.rs"]
mod tests;
