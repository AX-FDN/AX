use std::collections::HashMap;

use crate::ast::{BinaryOp, ItemKind};
use crate::diagnostics::Diagnostic;
use crate::source::{SourceFile, Span};

use super::types::Type;

pub(super) fn type_name_as_value_diagnostic(
    source: &SourceFile,
    span: Span,
    name: &str,
    ty: &Type,
) -> Diagnostic {
    let diagnostic = Diagnostic::new(
        "S0028",
        format!("type name `{name}` cannot be used as a runtime value"),
        source,
        span,
    );

    match ty {
        Type::Enum(enum_name) => diagnostic.with_suggestion(format!(
            "use an enum variant like `{enum_name}.VariantName`"
        )),
        Type::Struct(struct_name) => diagnostic.with_suggestion(format!(
            "construct `{struct_name}` with `{struct_name} {{ field: ... }}`",
        )),
        _ => diagnostic.with_suggestion("use the type name only in type positions"),
    }
}

pub(super) fn builtin_types() -> HashMap<String, Type> {
    [
        ("bool", Type::Bool),
        ("i32", Type::I32),
        ("f32", Type::F32),
        ("string", Type::String),
        ("string_list", Type::StringList),
    ]
    .into_iter()
    .map(|(name, ty)| (name.to_string(), ty))
    .collect()
}

pub(super) fn return_type_message(expected: &Type, actual: &Type) -> String {
    if *actual == Type::Void {
        format!(
            "return statement must produce `{}`, but no value was returned",
            expected.describe()
        )
    } else {
        format!(
            "return statement must produce `{}`, found `{}`",
            expected.describe(),
            actual.describe()
        )
    }
}

pub(super) fn type_mismatch_suggestion(expected: &Type, actual: &Type) -> String {
    match expected {
        Type::Bool => format!(
            "make the expression produce `bool`; AX does not coerce `{}` into a condition",
            actual.describe()
        ),
        Type::I32 => format!(
            "make the expression produce `i32`, or change the declared type if `{}` is intended",
            actual.describe()
        ),
        Type::F32 => format!(
            "make the expression produce `f32`, or change the declared type if `{}` is intended",
            actual.describe()
        ),
        Type::String => format!(
            "make the expression produce `string`, or change the declared type if `{}` is intended",
            actual.describe()
        ),
        Type::StringList => format!(
            "make the expression produce `string_list`, or build one with `string_list_new()` and `string_list_push(...)`",
        ),
        Type::Slice { .. } => format!(
            "make the expression produce `{}`, or pass/slice an array so the expected view type matches",
            expected.describe()
        ),
        other => format!(
            "make the expression produce `{}`, or change the surrounding declaration so both sides agree",
            other.describe()
        ),
    }
}

pub(super) fn binary_op_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::LogicalOr => "||",
        BinaryOp::LogicalAnd => "&&",
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
        BinaryOp::Remainder => "%",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "!=",
        BinaryOp::Less => "<",
        BinaryOp::LessEqual => "<=",
        BinaryOp::Greater => ">",
        BinaryOp::GreaterEqual => ">=",
    }
}

pub(super) fn item_name(kind: &ItemKind) -> &str {
    match kind {
        ItemKind::Function { name, .. }
        | ItemKind::Struct { name, .. }
        | ItemKind::Enum { name, .. } => name.as_str(),
    }
}
