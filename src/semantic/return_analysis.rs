use std::collections::HashSet;

use crate::ast::{Block, MatchArm, MatchPatternKind, Stmt, StmtKind};
use crate::diagnostics::Diagnostic;
use crate::source::SourceFile;

use super::program_info::ProgramInfo;
use super::types::Type;

pub(super) fn missing_return_diagnostic(
    source: &SourceFile,
    function_name: &str,
    return_type: &Type,
    body: &Block,
    info: &ProgramInfo<'_>,
    current_unit_path: &str,
) -> Option<Diagnostic> {
    if block_guarantees_return(body, info, current_unit_path) {
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
        .with_note(missing_return_note(body, info, current_unit_path))
        .with_suggestion(missing_return_suggestion(return_type)),
    )
}

fn block_guarantees_return(block: &Block, info: &ProgramInfo<'_>, current_unit_path: &str) -> bool {
    block
        .statements
        .iter()
        .any(|statement| statement_guarantees_return(statement, info, current_unit_path))
}

fn statement_guarantees_return(
    statement: &Stmt,
    info: &ProgramInfo<'_>,
    current_unit_path: &str,
) -> bool {
    match &statement.kind {
        StmtKind::Return { .. } => true,
        StmtKind::Block { block } => block_guarantees_return(block, info, current_unit_path),
        StmtKind::If {
            then_branch,
            else_branch: Some(else_branch),
            ..
        } => {
            block_guarantees_return(then_branch, info, current_unit_path)
                && block_guarantees_return(else_branch, info, current_unit_path)
        }
        StmtKind::Match { arms, .. } => {
            match_is_exhaustive(arms, info, current_unit_path)
                && arms
                    .iter()
                    .all(|arm| block_guarantees_return(&arm.body, info, current_unit_path))
        }
        _ => false,
    }
}

fn match_is_exhaustive(arms: &[MatchArm], info: &ProgramInfo<'_>, current_unit_path: &str) -> bool {
    if arms.is_empty() {
        return false;
    }

    if arms
        .iter()
        .any(|arm| matches!(arm.pattern.kind, MatchPatternKind::Wildcard))
    {
        return arms.iter().any(|arm| {
            !matches!(
                arm.pattern.kind,
                MatchPatternKind::Wildcard | MatchPatternKind::Error
            )
        });
    }

    let mut bools = HashSet::new();
    let mut enum_name = None::<String>;
    let mut enum_variants = HashSet::new();
    let mut only_bools = true;
    let mut only_enums = true;

    for arm in arms {
        match &arm.pattern.kind {
            MatchPatternKind::Bool { value } => {
                bools.insert(*value);
                only_enums = false;
            }
            MatchPatternKind::EnumVariant { path } => {
                only_bools = false;
                let Some((enum_path, variant)) = path.rsplit_once('.') else {
                    return false;
                };
                let mut diagnostics = Vec::new();
                let Some(resolved_key) = info.resolve_named_type_key(
                    enum_path,
                    current_unit_path,
                    arm.pattern.span,
                    &mut diagnostics,
                ) else {
                    return false;
                };
                let Some(Type::Enum(resolved_enum_name)) = info.named_types.get(&resolved_key)
                else {
                    return false;
                };
                if let Some(existing) = &enum_name {
                    if existing != resolved_enum_name {
                        return false;
                    }
                } else {
                    enum_name = Some(resolved_enum_name.clone());
                }
                enum_variants.insert(variant.to_string());
            }
            MatchPatternKind::Int { .. } | MatchPatternKind::Wildcard | MatchPatternKind::Error => {
                return false;
            }
        }
    }

    if only_bools {
        return bools.len() == 2;
    }

    if only_enums {
        let Some(enum_name) = enum_name else {
            return false;
        };
        let Some(enum_info) = info.enums.get(&enum_name) else {
            return false;
        };
        return enum_info
            .variants
            .iter()
            .all(|variant| enum_variants.contains(variant));
    }

    false
}

fn missing_return_note(body: &Block, info: &ProgramInfo<'_>, current_unit_path: &str) -> String {
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
        Some(StmtKind::Match { arms, .. }) => {
            if match_is_exhaustive(arms, info, current_unit_path) {
                "every `match` arm must still return; add a `return` to each arm or a fallback after the match"
                    .to_string()
            } else {
                "the final `match` is not exhaustive, so the function can still fall through after unmatched cases"
                    .to_string()
            }
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
