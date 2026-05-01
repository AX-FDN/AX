use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::source::SourceFile;

use super::{RuleTemplate, lexer, parser, project, runtime, semantic};

pub(in crate::ai) fn match_rule(
    _source: &SourceFile,
    diagnostic: &Diagnostic,
) -> Option<RuleTemplate> {
    if let Some(kind) = diagnostic.kind()
        && let Some(rule) = match_rule_by_kind(kind)
    {
        return Some(rule);
    }

    match_rule_by_code(diagnostic.code.as_str())
}

pub(in crate::ai) fn is_main_required_rule(rule: &RuleTemplate) -> bool {
    semantic::is_main_required_rule(rule)
}

fn match_rule_by_code(code: &str) -> Option<RuleTemplate> {
    lexer::match_code(code)
        .or_else(|| parser::match_code(code))
        .or_else(|| project::match_code(code))
        .or_else(|| semantic::match_code(code))
        .or_else(|| runtime::match_code(code))
}

fn match_rule_by_kind(kind: DiagnosticKind) -> Option<RuleTemplate> {
    parser::match_kind(kind)
        .or_else(|| project::match_kind(kind))
        .or_else(|| semantic::match_kind(kind))
        .or_else(|| runtime::match_kind(kind))
}
