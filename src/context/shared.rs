use std::collections::BTreeSet;
use std::path::Path;

use crate::diagnostics::Diagnostic;

use super::ContextView;
use super::types::ContextValidation;

pub(super) fn build_validation(
    diagnostics: &[Diagnostic],
    command_target: &str,
    view: ContextView,
    mut notes: Vec<String>,
) -> ContextValidation {
    if diagnostics.is_empty() {
        notes.push("context built from a clean diagnostic pass".to_string());
    } else {
        notes.push(format!(
            "context built from a partial program with {} diagnostic(s)",
            diagnostics.len()
        ));
    }

    let recommended_commands = match view {
        ContextView::Overview => vec![
            format!("axc check {command_target}"),
            format!("axc context overview {command_target} --json"),
            format!("axc context boundaries {command_target} --json"),
        ],
        ContextView::Boundaries => vec![
            format!("axc check {command_target}"),
            format!("axc context boundaries {command_target} --json"),
            format!("axc run {command_target}"),
        ],
        ContextView::Topology => vec![
            format!("axc check {command_target}"),
            format!("axc context topology {command_target} --json"),
            format!("axc context symbol {command_target} <symbol> --json"),
        ],
        ContextView::Flow => vec![
            format!("axc check {command_target}"),
            format!("axc context flow {command_target} --json"),
            format!("axc context symbol {command_target} <symbol> --json"),
        ],
        ContextView::Symbol => vec![
            format!("axc check {command_target}"),
            format!("axc context topology {command_target} --json"),
            format!("axc context symbol {command_target} <symbol> --json"),
        ],
        ContextView::Impact => vec![
            format!("axc check {command_target}"),
            format!("axc context flow {command_target} --json"),
            format!("axc context impact {command_target} <symbol> --json"),
        ],
        ContextView::Evidence => vec![
            format!("axc check {command_target}"),
            format!("axc context impact {command_target} <symbol> --json"),
            format!("axc context evidence {command_target} <symbol> --json"),
        ],
    };

    ContextValidation {
        diagnostic_count: diagnostics.len(),
        partial: !diagnostics.is_empty(),
        recommended_commands,
        notes,
    }
}

pub(super) fn push_unique(
    output: &mut Vec<String>,
    seen: &mut BTreeSet<String>,
    values: &[String],
    limit: usize,
) {
    for value in values {
        if output.len() >= limit {
            break;
        }
        if seen.insert(value.clone()) {
            output.push(value.clone());
        }
    }
}

pub(super) fn normalize_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

pub(super) fn normalize_path_text(path: &str) -> String {
    path.replace('\\', "/")
}
