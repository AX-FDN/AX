use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::ast::{Program, Visibility};
use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::source::{SourceFile, Span};

mod context_snippets;
mod rules;
mod session;

use self::context_snippets::DiagnosticContext;
use self::rules::match_rule;
use self::session::{load_session, save_session};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticLayer {
    SourceInput,
    Lexer,
    Parser,
    Semantic,
    HirLowering,
    MirLowering,
    Interpreter,
    BuildArtifact,
    AotReadiness,
    LlvmLowering,
    ToolchainLink,
    InternalCompiler,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AiAction {
    EditSource,
    FixInputOrConfig,
    FixRuntimeInput,
    FixEnvironment,
    ExplainUnsupported,
    ConfigureToolchain,
    InspectToolchainFailure,
    ReportCompilerBug,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiRepairContract {
    pub layer: DiagnosticLayer,
    pub ai_action: AiAction,
    pub safe_to_edit: bool,
    pub validation: Vec<String>,
}

impl AiRepairContract {
    pub fn for_diagnostic(diagnostic: &Diagnostic) -> Self {
        let layer = layer_for_diagnostic(diagnostic);
        let ai_action = action_for_diagnostic(layer, diagnostic);
        Self {
            layer,
            ai_action,
            safe_to_edit: source_edit_is_safe(ai_action),
            validation: validation_for_layer(layer),
        }
    }

    pub fn source_input() -> Self {
        let layer = DiagnosticLayer::SourceInput;
        let ai_action = AiAction::FixInputOrConfig;
        Self {
            layer,
            ai_action,
            safe_to_edit: source_edit_is_safe(ai_action),
            validation: validation_for_layer(layer),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AiDiagnostic {
    pub rule_id: String,
    pub layer: DiagnosticLayer,
    pub ai_action: AiAction,
    pub safe_to_edit: bool,
    pub validation: Vec<String>,
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

fn layer_for_diagnostic(diagnostic: &Diagnostic) -> DiagnosticLayer {
    if let Some(kind) = diagnostic.kind() {
        if is_parser_kind(kind) {
            return DiagnosticLayer::Parser;
        }
        if is_runtime_kind(kind) {
            return DiagnosticLayer::Interpreter;
        }
        return DiagnosticLayer::Semantic;
    }

    if diagnostic.code.starts_with("PX") || diagnostic.code.starts_with("LX") {
        return DiagnosticLayer::SourceInput;
    }

    match diagnostic.code.chars().next() {
        Some('I') => DiagnosticLayer::SourceInput,
        Some('L') => DiagnosticLayer::Lexer,
        Some('P') => DiagnosticLayer::Parser,
        Some('S') => DiagnosticLayer::Semantic,
        Some('H') => DiagnosticLayer::HirLowering,
        Some('M') => DiagnosticLayer::MirLowering,
        Some('R') => DiagnosticLayer::Interpreter,
        _ => DiagnosticLayer::InternalCompiler,
    }
}

fn action_for_diagnostic(layer: DiagnosticLayer, diagnostic: &Diagnostic) -> AiAction {
    match layer {
        DiagnosticLayer::SourceInput => AiAction::FixInputOrConfig,
        DiagnosticLayer::Lexer | DiagnosticLayer::Parser | DiagnosticLayer::Semantic => {
            AiAction::EditSource
        }
        DiagnosticLayer::Interpreter => action_for_runtime_diagnostic(diagnostic),
        DiagnosticLayer::BuildArtifact => AiAction::FixEnvironment,
        DiagnosticLayer::AotReadiness | DiagnosticLayer::LlvmLowering => {
            AiAction::ExplainUnsupported
        }
        DiagnosticLayer::ToolchainLink => AiAction::ConfigureToolchain,
        DiagnosticLayer::HirLowering
        | DiagnosticLayer::MirLowering
        | DiagnosticLayer::InternalCompiler => AiAction::ReportCompilerBug,
    }
}

fn action_for_runtime_diagnostic(diagnostic: &Diagnostic) -> AiAction {
    match diagnostic.kind() {
        Some(
            DiagnosticKind::ArgvIndexNegative
            | DiagnosticKind::ArgvIndexOutOfBounds
            | DiagnosticKind::EnvironmentVariableUnavailable
            | DiagnosticKind::ReadableFilePathRequired
            | DiagnosticKind::ReadableDirectoryPathRequired
            | DiagnosticKind::ProcessCommandNotLaunchable
            | DiagnosticKind::ProcessCaptureNonZeroExit,
        ) => AiAction::FixRuntimeInput,
        _ => AiAction::EditSource,
    }
}

fn source_edit_is_safe(action: AiAction) -> bool {
    matches!(action, AiAction::EditSource | AiAction::FixInputOrConfig)
}

fn validation_for_layer(layer: DiagnosticLayer) -> Vec<String> {
    match layer {
        DiagnosticLayer::Interpreter => vec![
            "axc check <target>".to_string(),
            "axc run <target>".to_string(),
        ],
        DiagnosticLayer::AotReadiness
        | DiagnosticLayer::LlvmLowering
        | DiagnosticLayer::ToolchainLink => vec![
            "axc run <target>".to_string(),
            "axc build <target>".to_string(),
        ],
        DiagnosticLayer::BuildArtifact => vec!["axc build <target>".to_string()],
        DiagnosticLayer::HirLowering | DiagnosticLayer::MirLowering => {
            vec!["axc check <target>".to_string()]
        }
        DiagnosticLayer::SourceInput
        | DiagnosticLayer::Lexer
        | DiagnosticLayer::Parser
        | DiagnosticLayer::Semantic
        | DiagnosticLayer::InternalCompiler => vec!["axc check <target>".to_string()],
    }
}

fn is_parser_kind(kind: DiagnosticKind) -> bool {
    matches!(
        kind,
        DiagnosticKind::MissingSemicolon
            | DiagnosticKind::MissingRightParen
            | DiagnosticKind::MissingRightBracket
            | DiagnosticKind::MissingRightBrace
            | DiagnosticKind::TopLevelDeclarationRequired
            | DiagnosticKind::TypeNameRequired
            | DiagnosticKind::ExpressionRequired
    )
}

fn is_runtime_kind(kind: DiagnosticKind) -> bool {
    matches!(
        kind,
        DiagnosticKind::ArgvIndexNegative
            | DiagnosticKind::ArgvIndexOutOfBounds
            | DiagnosticKind::StringListIndexNegative
            | DiagnosticKind::StringListIndexOutOfBounds
            | DiagnosticKind::EnvironmentVariableUnavailable
            | DiagnosticKind::ReadableFilePathRequired
            | DiagnosticKind::ReadableDirectoryPathRequired
            | DiagnosticKind::ProcessCommandNotLaunchable
            | DiagnosticKind::ProcessCaptureNonZeroExit
    )
}

#[cfg(test)]
mod tests;
