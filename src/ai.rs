use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::ast::{Program, Visibility};
use crate::diagnostics::Diagnostic;
use crate::source::{SourceFile, Span};

mod context_snippets;
mod rules;
mod session;

use self::context_snippets::DiagnosticContext;
use self::rules::match_rule;
use self::session::{load_session, save_session};

#[derive(Debug, Clone, Serialize)]
pub struct AiDiagnostic {
    pub rule_id: String,
    pub teaching_level: TeachingLevel,
    pub repeat_count: u32,
    pub repair_goal: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus_item: Option<AiFocusItem>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub relevant_spans: Vec<Span>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub related_symbols: Vec<AiRelatedSymbol>,
    pub rule_card: AiRuleCard,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fixits: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub context_snippets: Vec<AiContextSnippet>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AiFocusItem {
    pub kind: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Visibility::is_private")]
    pub visibility: Visibility,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct AiRelatedSymbol {
    pub kind: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct AiRuleCard {
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimal_example: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anti_pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AiContextSnippet {
    pub label: String,
    pub text: String,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeachingLevel {
    #[serde(rename = "L1")]
    L1,
    #[serde(rename = "L2")]
    L2,
    #[serde(rename = "L3")]
    L3,
}

impl TeachingLevel {
    fn from_repeat_count(repeat_count: u32) -> Self {
        match repeat_count {
            0 | 1 => Self::L1,
            2 | 3 => Self::L2,
            _ => Self::L3,
        }
    }
}

pub fn enhance_diagnostics(
    source: &SourceFile,
    program: &Program,
    diagnostics: &mut [Diagnostic],
    session_path: Option<&Path>,
) -> Result<(), String> {
    let mut session = match session_path {
        Some(path) => Some(load_session(path)?),
        None => None,
    };

    for diagnostic in diagnostics.iter_mut() {
        let Some(rule) = match_rule(source, diagnostic) else {
            continue;
        };

        let repeat_count = session
            .as_mut()
            .map(|state| {
                state.bump(
                    diagnostic.code.as_str(),
                    rule.rule_id,
                    rule.normalized_pattern,
                )
            })
            .unwrap_or(1);
        let teaching_level = TeachingLevel::from_repeat_count(repeat_count);
        let context = DiagnosticContext::new(source, program, diagnostic, &rule);
        diagnostic.ai = Some(context.build(rule, diagnostic, repeat_count, teaching_level));
    }

    if let (Some(path), Some(session)) = (session_path, session.as_ref()) {
        save_session(path, session)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests;
