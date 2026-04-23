use crate::ast::{Block, Stmt, StmtKind};
use crate::diagnostics::Diagnostic;
use crate::source::SourceFile;

use super::types::Type;

pub(super) fn missing_return_diagnostic(
    source: &SourceFile,
    function_name: &str,
    return_type: &Type,
    body: &Block,
) -> Option<Diagnostic> {
    if block_guarantees_return(body) {
        return None;
    }

    let return_type_name = return_type.describe();
    Some(
        Diagnostic::new(
            "S0023",
            format!(
                "function `{function_name}` may complete without returning `{}`",
                return_type_name
            ),
            source,
            body.span,
        )
        .with_note(format!(
            "`{function_name}` is declared to return `{return_type_name}` on every control-flow path"
        ))
        .with_note(missing_return_note(body))
        .with_suggestion(missing_return_suggestion(return_type)),
    )
}

fn block_guarantees_return(block: &Block) -> bool {
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

fn missing_return_note(body: &Block) -> String {
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

fn missing_return_suggestion(return_type: &Type) -> String {
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
