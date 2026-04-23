use std::collections::HashMap;

use crate::ast::{BinaryOp, Block, ItemKind, Stmt, StmtKind};
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
        Type::Enum(enum_name) => {
            diagnostic.with_suggestion(format!("use an enum variant like `{enum_name}.VariantName`"))
        }
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

pub(super) fn missing_return_note(body: &Block) -> String {
    match body.statements.last().map(|statement| &statement.kind) {
        None => "the function body is empty, so no control-flow path returns a value".to_string(),
        Some(StmtKind::If { else_branch: None, .. }) => {
            "the final `if` has no `else`, so the function can still fall through when the condition is false"
                .to_string()
        }
        Some(StmtKind::While { .. }) => {
            "a `while` loop may not run, so the function still needs a fallback `return` after the loop"
                .to_string()
        }
        Some(StmtKind::For { .. }) => {
            "a `for` loop may not run, so the function still needs a fallback `return` after the loop"
                .to_string()
        }
        _ => "add a final `return ...;` before the function body closes, or make every branch return".to_string(),
    }
}

pub(super) fn missing_return_suggestion(return_type: &Type) -> String {
    match return_type {
        Type::Bool => {
            "add a fallback like `return false;`, or make every branch return `bool`".to_string()
        }
        Type::I32 => {
            "add a fallback like `return 0;`, or make every branch return `i32`".to_string()
        }
        Type::F32 => {
            "add a fallback like `return 0.0;`, or make every branch return `f32`".to_string()
        }
        Type::String => {
            "add a fallback like `return \"\";`, or make every branch return `string`".to_string()
        }
        _ => "ensure every control-flow path ends with `return ...;`".to_string(),
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
        other => format!(
            "make the expression produce `{}`, or change the surrounding declaration so both sides agree",
            other.describe()
        ),
    }
}

pub(super) fn binary_op_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
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

pub(super) fn block_guarantees_return(block: &Block) -> bool {
    block.statements.iter().any(statement_guarantees_return)
}

fn statement_guarantees_return(statement: &Stmt) -> bool {
    match &statement.kind {
        StmtKind::Return { .. } => true,
        StmtKind::Block { block } => block_guarantees_return(block),
        StmtKind::If {
            then_branch,
            else_branch: Some(else_branch),
            ..
        } => block_guarantees_return(then_branch) && block_guarantees_return(else_branch),
        _ => false,
    }
}
