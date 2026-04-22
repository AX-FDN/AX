use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::ast::{Block, Expr, ExprKind, Item, ItemKind, Program, Stmt, StmtKind};
use crate::diagnostics::Diagnostic;
use crate::source::{SourceFile, Span};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AiSessionEntry {
    diagnostic_code: String,
    rule_id: String,
    normalized_pattern: String,
    repeat_count: u32,
    last_teaching_level: TeachingLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AiSessionFile {
    version: u32,
    entries: BTreeMap<String, AiSessionEntry>,
}

impl Default for AiSessionFile {
    fn default() -> Self {
        Self {
            version: 1,
            entries: BTreeMap::new(),
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
        let Some(rule) = match_rule(diagnostic) else {
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

#[derive(Clone, Copy)]
struct RuleTemplate {
    rule_id: &'static str,
    normalized_pattern: &'static str,
    repair_goal: &'static str,
    summary: &'static str,
    pattern: &'static str,
    minimal_example: &'static str,
    anti_pattern: Option<&'static str>,
    default_fixit: &'static str,
}

fn match_rule(diagnostic: &Diagnostic) -> Option<RuleTemplate> {
    let expects = diagnostic.expected.join(" ");
    match diagnostic.code.as_str() {
        "P0001" if diagnostic.message.contains("expected `;`") || expects.contains("`;`") => {
            Some(RULE_MISSING_SEMICOLON)
        }
        "P0001" if diagnostic.message.contains("expected `)`") || expects.contains("`)`") => {
            Some(RULE_MISSING_RPAREN)
        }
        "P0001" if diagnostic.message.contains("expected `}`") || expects.contains("`}`") => {
            Some(RULE_MISSING_RBRACE)
        }
        "S0002" => Some(RULE_UNDEFINED_VARIABLE),
        "S0003" => Some(RULE_IMMUTABLE_ASSIGNMENT),
        "S0004" => Some(RULE_MAIN_REQUIRED),
        "S0005" => Some(RULE_MAIN_SIGNATURE),
        "S0022" => Some(RULE_TYPE_MISMATCH),
        "S0023" => Some(RULE_MISSING_RETURN),
        _ => None,
    }
}

const RULE_MISSING_SEMICOLON: RuleTemplate = RuleTemplate {
    rule_id: "statement_terminator_required",
    normalized_pattern: "statement_terminator_required",
    repair_goal: "Insert the missing semicolon so the statement terminates correctly.",
    summary: "AX requires `let`, assignment, expression, and `return` statements to end with `;`.",
    pattern: "let name: Type = expr;",
    minimal_example: "let value: i32 = 1;",
    anti_pattern: Some("let value: i32 = 1"),
    default_fixit: "insert `;` at the end of the current statement",
};

const RULE_MISSING_RPAREN: RuleTemplate = RuleTemplate {
    rule_id: "close_parenthesized_construct",
    normalized_pattern: "close_parenthesized_construct",
    repair_goal: "Close the current parenthesized construct with `)` and keep the surrounding syntax balanced.",
    summary: "AX requires balanced parentheses in conditions, grouped expressions, calls, and `for` headers.",
    pattern: "if (cond) { ... }",
    minimal_example: "if (flag == true) { return 1; }",
    anti_pattern: Some("if (flag == true { return 1; }"),
    default_fixit: "add the missing `)` at the current construct boundary",
};

const RULE_MISSING_RBRACE: RuleTemplate = RuleTemplate {
    rule_id: "close_block_or_literal",
    normalized_pattern: "close_block_or_literal",
    repair_goal: "Close the current block or literal with `}`.",
    summary: "AX requires balanced braces for blocks, function bodies, and struct literals.",
    pattern: "fn main() -> i32 { return 0; }",
    minimal_example: "struct Point { x: i32, y: i32 }",
    anti_pattern: Some("fn main() -> i32 { return 0;"),
    default_fixit: "add the missing `}` to close the current block or literal",
};

const RULE_MAIN_REQUIRED: RuleTemplate = RuleTemplate {
    rule_id: "main_function_required",
    normalized_pattern: "main_function_required",
    repair_goal: "Add a valid `main` entrypoint so the current AX program is runnable.",
    summary: "Every runnable AX program must define `fn main() -> i32`.",
    pattern: "fn main() -> i32 { return 0; }",
    minimal_example: "fn main() -> i32 { return 0; }",
    anti_pattern: None,
    default_fixit: "add `fn main() -> i32 { return 0; }`",
};

const RULE_MAIN_SIGNATURE: RuleTemplate = RuleTemplate {
    rule_id: "main_signature_fixed",
    normalized_pattern: "main_signature_fixed",
    repair_goal: "Change `main` so it takes no parameters and returns `i32`.",
    summary: "The current AX prototype requires `main` to use the fixed signature `fn main() -> i32`.",
    pattern: "fn main() -> i32 { return 0; }",
    minimal_example: "fn main() -> i32 { return 0; }",
    anti_pattern: Some("fn main(value: i32) -> bool { return false; }"),
    default_fixit: "rewrite `main` to `fn main() -> i32 { ... }`",
};

const RULE_TYPE_MISMATCH: RuleTemplate = RuleTemplate {
    rule_id: "type_match_required",
    normalized_pattern: "type_match_required",
    repair_goal: "Change the expression or the declared type so both sides use the same AX type.",
    summary: "AX requires assignments, arguments, returns, and conditions to use the declared type exactly.",
    pattern: "let value: i32 = 1;",
    minimal_example: "fn add(value: i32) -> i32 { return value; }",
    anti_pattern: Some("let value: bool = 1;"),
    default_fixit: "make the expression and the expected AX type agree",
};

const RULE_MISSING_RETURN: RuleTemplate = RuleTemplate {
    rule_id: "all_paths_must_return",
    normalized_pattern: "all_paths_must_return",
    repair_goal: "Make every control-flow path return a value of the declared function type.",
    summary: "Functions with a non-void return type must return a value on every control-flow path.",
    pattern: "fn helper(flag: bool) -> i32 { if (flag) { return 1; } return 0; }",
    minimal_example: "fn helper(flag: bool) -> i32 { if (flag) { return 1; } return 0; }",
    anti_pattern: Some("fn helper(flag: bool) -> i32 { if (flag) { return 1; } }"),
    default_fixit: "add a `return ...;` on the missing control-flow path",
};

const RULE_IMMUTABLE_ASSIGNMENT: RuleTemplate = RuleTemplate {
    rule_id: "mutable_binding_required",
    normalized_pattern: "mutable_binding_required",
    repair_goal: "Either declare the binding with `let mut` or stop assigning to it.",
    summary: "AX bindings are immutable unless they are declared with `let mut`.",
    pattern: "let mut value: i32 = 0; value = value + 1;",
    minimal_example: "let mut value: i32 = 0; value = value + 1;",
    anti_pattern: Some("let value: i32 = 0; value = 1;"),
    default_fixit: "change the declaration to `let mut ...` or remove the assignment",
};

const RULE_UNDEFINED_VARIABLE: RuleTemplate = RuleTemplate {
    rule_id: "variable_must_be_declared_in_scope",
    normalized_pattern: "variable_must_be_declared_in_scope",
    repair_goal: "Introduce a declaration in scope before using the variable.",
    summary: "AX requires variables to be declared before use within the current scope.",
    pattern: "let value: i32 = 1; println(value);",
    minimal_example: "let total: i32 = 1; println(total);",
    anti_pattern: Some("println(total);"),
    default_fixit: "declare the variable before this use",
};

struct AiSession {
    entries: BTreeMap<String, AiSessionEntry>,
}

impl Default for AiSession {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }
}

impl AiSession {
    fn bump(&mut self, diagnostic_code: &str, rule_id: &str, normalized_pattern: &str) -> u32 {
        let key = format!("{diagnostic_code}::{normalized_pattern}");
        let entry = self.entries.entry(key).or_insert_with(|| AiSessionEntry {
            diagnostic_code: diagnostic_code.to_string(),
            rule_id: rule_id.to_string(),
            normalized_pattern: normalized_pattern.to_string(),
            repeat_count: 0,
            last_teaching_level: TeachingLevel::L1,
        });
        entry.repeat_count += 1;
        entry.last_teaching_level = TeachingLevel::from_repeat_count(entry.repeat_count);
        entry.repeat_count
    }
}

fn load_session(path: &Path) -> Result<AiSession, String> {
    match fs::read_to_string(path) {
        Ok(text) => {
            let file: AiSessionFile = serde_json::from_str(&text).map_err(|error| {
                format!("failed to parse AI session {}: {error}", path.display())
            })?;
            Ok(AiSession {
                entries: file.entries,
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(AiSession::default()),
        Err(error) => Err(format!(
            "failed to read AI session {}: {error}",
            path.display()
        )),
    }
}

fn save_session(path: &Path, session: &AiSession) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
    }

    let file = AiSessionFile {
        version: 1,
        entries: session.entries.clone(),
    };
    let text = serde_json::to_string_pretty(&file)
        .map_err(|error| format!("failed to serialize AI session {}: {error}", path.display()))?;
    fs::write(path, text)
        .map_err(|error| format!("failed to write AI session {}: {error}", path.display()))
}

struct DiagnosticContext {
    focus_item: Option<AiFocusItem>,
    relevant_spans: Vec<Span>,
    related_symbols: Vec<AiRelatedSymbol>,
    context_snippets: Vec<AiContextSnippet>,
}

impl DiagnosticContext {
    fn new(
        source: &SourceFile,
        program: &Program,
        diagnostic: &Diagnostic,
        rule: &RuleTemplate,
    ) -> Self {
        let mut relevant_spans = vec![diagnostic.span];

        if rule.rule_id == RULE_MAIN_REQUIRED.rule_id {
            return Self {
                focus_item: None,
                relevant_spans,
                related_symbols: Vec::new(),
                context_snippets: vec![AiContextSnippet {
                    label: "diagnostic_site".to_string(),
                    text: snippet_text(source, diagnostic.span, 3),
                    span: diagnostic.span,
                }],
            };
        }

        let Some(item) = find_focus_item(program, diagnostic.span) else {
            return Self {
                focus_item: None,
                relevant_spans,
                related_symbols: Vec::new(),
                context_snippets: vec![AiContextSnippet {
                    label: "diagnostic_site".to_string(),
                    text: snippet_text(source, diagnostic.span, 3),
                    span: diagnostic.span,
                }],
            };
        };

        push_unique_span(&mut relevant_spans, item.span);
        let mut snippet_spans = vec![("diagnostic_site".to_string(), diagnostic.span)];

        let focus_item = Some(item_descriptor(item));
        let related_symbols = related_symbols_for_item(program, item);

        if let ItemKind::Function { body, .. } = &item.kind {
            push_unique_span(&mut relevant_spans, body.span);
            if let Some(statement_span) = find_smallest_statement_span(body, diagnostic.span) {
                push_unique_span(&mut relevant_spans, statement_span);
                snippet_spans.push(("enclosing_statement".to_string(), statement_span));
            }
            snippet_spans.push(("function_context".to_string(), body.span));
        } else {
            snippet_spans.push(("focus_item".to_string(), item.span));
        }

        let context_snippets = snippet_spans
            .into_iter()
            .filter_map(|(label, span)| {
                let text = snippet_text(source, span, 4);
                if text.is_empty() {
                    None
                } else {
                    Some(AiContextSnippet { label, text, span })
                }
            })
            .collect::<Vec<_>>();

        Self {
            focus_item,
            relevant_spans,
            related_symbols,
            context_snippets,
        }
    }

    fn build(
        &self,
        rule: RuleTemplate,
        diagnostic: &Diagnostic,
        repeat_count: u32,
        teaching_level: TeachingLevel,
    ) -> AiDiagnostic {
        let mut fixits = Vec::new();
        if let Some(suggestion) = &diagnostic.suggestion {
            fixits.push(suggestion.clone());
        }
        if fixits.is_empty() {
            fixits.push(rule.default_fixit.to_string());
        }

        let rule_card = match teaching_level {
            TeachingLevel::L1 => AiRuleCard {
                summary: rule.summary.to_string(),
                pattern: None,
                minimal_example: None,
                anti_pattern: None,
            },
            TeachingLevel::L2 => AiRuleCard {
                summary: rule.summary.to_string(),
                pattern: Some(rule.pattern.to_string()),
                minimal_example: None,
                anti_pattern: None,
            },
            TeachingLevel::L3 => AiRuleCard {
                summary: rule.summary.to_string(),
                pattern: Some(rule.pattern.to_string()),
                minimal_example: Some(rule.minimal_example.to_string()),
                anti_pattern: rule.anti_pattern.map(str::to_string),
            },
        };

        AiDiagnostic {
            rule_id: rule.rule_id.to_string(),
            teaching_level,
            repeat_count,
            repair_goal: rule.repair_goal.to_string(),
            focus_item: self.focus_item.clone(),
            relevant_spans: self.relevant_spans.clone(),
            related_symbols: match teaching_level {
                TeachingLevel::L3 => self.related_symbols.clone(),
                _ => Vec::new(),
            },
            rule_card,
            fixits,
            context_snippets: match teaching_level {
                TeachingLevel::L3 => self.context_snippets.clone(),
                _ => Vec::new(),
            },
        }
    }
}

fn find_focus_item(program: &Program, span: Span) -> Option<&Item> {
    program
        .items
        .iter()
        .find(|item| item.span.start <= span.start && item.span.end >= span.end)
}

fn item_descriptor(item: &Item) -> AiFocusItem {
    match &item.kind {
        ItemKind::Function {
            name,
            params,
            return_type,
            ..
        } => AiFocusItem {
            kind: "function".to_string(),
            name: name.clone(),
            signature: Some(format!(
                "fn {name}({}) -> {}",
                params
                    .iter()
                    .map(|param| format!("{}: {}", param.name, param.ty.name))
                    .collect::<Vec<_>>()
                    .join(", "),
                return_type.name
            )),
            span: item.span,
        },
        ItemKind::Struct { name, fields } => AiFocusItem {
            kind: "struct".to_string(),
            name: name.clone(),
            signature: Some(format!(
                "struct {name} {{ {} }}",
                fields
                    .iter()
                    .map(|field| format!("{}: {}", field.name, field.ty.name))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            span: item.span,
        },
        ItemKind::Enum { name, variants } => AiFocusItem {
            kind: "enum".to_string(),
            name: name.clone(),
            signature: Some(format!(
                "enum {name} {{ {} }}",
                variants
                    .iter()
                    .map(|variant| variant.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            span: item.span,
        },
    }
}

fn related_symbols_for_item(program: &Program, focus_item: &Item) -> Vec<AiRelatedSymbol> {
    let mut top_level = BTreeMap::new();
    for item in &program.items {
        let name = match &item.kind {
            ItemKind::Function { name, .. }
            | ItemKind::Struct { name, .. }
            | ItemKind::Enum { name, .. } => name.clone(),
        };
        top_level.insert(name, item);
    }

    let focus_name = match &focus_item.kind {
        ItemKind::Function { name, .. }
        | ItemKind::Struct { name, .. }
        | ItemKind::Enum { name, .. } => name,
    };

    let mut referenced = BTreeSet::new();
    match &focus_item.kind {
        ItemKind::Function {
            params,
            return_type,
            body,
            ..
        } => {
            for param in params {
                referenced.insert(param.ty.name.clone());
            }
            referenced.insert(return_type.name.clone());
            collect_block_names(body, &mut referenced);
        }
        ItemKind::Struct { fields, .. } => {
            for field in fields {
                referenced.insert(field.ty.name.clone());
            }
        }
        ItemKind::Enum { .. } => {}
    }

    referenced
        .into_iter()
        .filter(|name| name != focus_name)
        .filter_map(|name| top_level.get(&name).copied())
        .map(item_descriptor)
        .map(|item| AiRelatedSymbol {
            kind: item.kind,
            name: item.name,
            signature: item.signature,
            span: item.span,
        })
        .collect()
}

fn collect_block_names(block: &Block, names: &mut BTreeSet<String>) {
    for statement in &block.statements {
        collect_statement_names(statement, names);
    }
}

fn collect_statement_names(statement: &Stmt, names: &mut BTreeSet<String>) {
    match &statement.kind {
        StmtKind::Let {
            ty, initializer, ..
        } => {
            names.insert(ty.name.clone());
            collect_expr_names(initializer, names);
        }
        StmtKind::Assign { target, value } => {
            collect_expr_names(target, names);
            collect_expr_names(value, names);
        }
        StmtKind::Expr { expr } => collect_expr_names(expr, names),
        StmtKind::Return { value } => {
            if let Some(expr) = value {
                collect_expr_names(expr, names);
            }
        }
        StmtKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_expr_names(condition, names);
            collect_block_names(then_branch, names);
            if let Some(block) = else_branch {
                collect_block_names(block, names);
            }
        }
        StmtKind::While { condition, body } => {
            collect_expr_names(condition, names);
            collect_block_names(body, names);
        }
        StmtKind::For {
            initializer,
            condition,
            step,
            body,
        } => {
            if let Some(statement) = initializer {
                collect_statement_names(statement, names);
            }
            if let Some(expr) = condition {
                collect_expr_names(expr, names);
            }
            if let Some(statement) = step {
                collect_statement_names(statement, names);
            }
            collect_block_names(body, names);
        }
        StmtKind::Block { block } => collect_block_names(block, names),
    }
}

fn collect_expr_names(expr: &Expr, names: &mut BTreeSet<String>) {
    match &expr.kind {
        ExprKind::Name { value } => {
            names.insert(value.clone());
        }
        ExprKind::Unary { expr, .. } => collect_expr_names(expr, names),
        ExprKind::Binary { left, right, .. } => {
            collect_expr_names(left, names);
            collect_expr_names(right, names);
        }
        ExprKind::Call { callee, arguments } => {
            collect_expr_names(callee, names);
            for argument in arguments {
                collect_expr_names(argument, names);
            }
        }
        ExprKind::StructLiteral { name, fields } => {
            names.insert(name.clone());
            for field in fields {
                collect_expr_names(&field.value, names);
            }
        }
        ExprKind::Field { base, .. } => collect_expr_names(base, names),
        ExprKind::Int { .. }
        | ExprKind::Float { .. }
        | ExprKind::Bool { .. }
        | ExprKind::String { .. }
        | ExprKind::Error => {}
    }
}

fn find_smallest_statement_span(block: &Block, target: Span) -> Option<Span> {
    let mut found = None;
    for statement in &block.statements {
        if !span_contains(statement.span, target) {
            continue;
        }

        found = Some(statement.span);
        match &statement.kind {
            StmtKind::If {
                then_branch,
                else_branch,
                ..
            } => {
                if let Some(inner) = find_smallest_statement_span(then_branch, target) {
                    found = Some(inner);
                }
                if let Some(block) = else_branch {
                    if let Some(inner) = find_smallest_statement_span(block, target) {
                        found = Some(inner);
                    }
                }
            }
            StmtKind::While { body, .. } => {
                if let Some(inner) = find_smallest_statement_span(body, target) {
                    found = Some(inner);
                }
            }
            StmtKind::For { body, .. } | StmtKind::Block { block: body } => {
                if let Some(inner) = find_smallest_statement_span(body, target) {
                    found = Some(inner);
                }
            }
            _ => {}
        }
    }
    found
}

fn span_contains(container: Span, inner: Span) -> bool {
    container.start <= inner.start && container.end >= inner.end
}

fn push_unique_span(spans: &mut Vec<Span>, span: Span) {
    if !spans.contains(&span) {
        spans.push(span);
    }
}

fn snippet_text(source: &SourceFile, span: Span, max_lines: usize) -> String {
    let (start_line, _) = source.line_col(span.start);
    let mut end_offset = span.end;
    if end_offset == span.start {
        end_offset = end_offset.saturating_add(1);
    }
    let (end_line, _) = source.line_col(end_offset);
    let stop = end_line.min(start_line + max_lines.saturating_sub(1));
    let mut lines = Vec::new();
    for line in start_line..=stop {
        lines.push(source.line_text(line).to_string());
    }
    if end_line > stop {
        lines.push("...".to_string());
    }
    lines.join("\n").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::{TeachingLevel, enhance_diagnostics};
    use std::fs;

    use crate::frontend::analyze;
    use crate::source::SourceFile;

    #[test]
    fn base_diagnostics_omit_ai_when_not_enhanced() {
        let source = SourceFile::anonymous("fn main() -> i32 { let value: i32 = 1 return value; }");
        let analysis = analyze(&source);
        let json =
            serde_json::to_string(&analysis.diagnostics).expect("diagnostics should serialize");
        assert!(!json.contains("\"ai\""));
    }

    #[test]
    fn enhances_missing_return_with_rule_card_and_context() {
        let source = SourceFile::anonymous(
            "fn helper(flag: bool) -> i32 { if (flag) { return 1; } }\nfn main() -> i32 { return helper(true); }",
        );
        let mut analysis = analyze(&source);
        enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
            .expect("ai enhancement should succeed");

        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "S0023")
            .expect("missing return diagnostic should exist");
        let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
        assert_eq!(ai.rule_id, "all_paths_must_return");
        assert_eq!(ai.teaching_level, TeachingLevel::L1);
        assert_eq!(ai.repeat_count, 1);
        assert_eq!(
            ai.focus_item.as_ref().map(|item| item.name.as_str()),
            Some("helper")
        );
        assert!(
            ai.relevant_spans
                .iter()
                .any(|span| span.start == diagnostic.span.start)
        );
    }

    #[test]
    fn teaching_level_escalates_with_session_reuse() {
        let temp_path = std::env::temp_dir().join(format!(
            "ax-ai-session-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should be monotonic")
                .as_nanos()
        ));

        let source = SourceFile::anonymous("fn main() -> i32 { let value: i32 = 1 return value; }");

        let mut first = analyze(&source);
        enhance_diagnostics(
            &source,
            &first.program,
            &mut first.diagnostics,
            Some(temp_path.as_path()),
        )
        .expect("first enhancement should succeed");

        let mut second = analyze(&source);
        enhance_diagnostics(
            &source,
            &second.program,
            &mut second.diagnostics,
            Some(temp_path.as_path()),
        )
        .expect("second enhancement should succeed");

        let first_ai = first.diagnostics[0]
            .ai
            .as_ref()
            .expect("first diagnostic should have ai");
        let second_ai = second.diagnostics[0]
            .ai
            .as_ref()
            .expect("second diagnostic should have ai");

        assert_eq!(first_ai.teaching_level, TeachingLevel::L1);
        assert_eq!(second_ai.teaching_level, TeachingLevel::L2);
        assert_eq!(second_ai.repeat_count, 2);
        assert!(second_ai.rule_card.pattern.is_some());

        let _ = fs::remove_file(temp_path);
    }
}
