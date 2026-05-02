use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{
    Block, Expr, ExprKind, Item, ItemKind, MatchPattern, MatchPatternKind, Program, Stmt, StmtKind,
    TypeRef, Visibility,
};
use crate::diagnostics::Diagnostic;
use crate::source::{SourceFile, Span};

use super::rules::{RuleTemplate, is_main_required_rule};
use super::{
    AiContextSnippet, AiDiagnostic, AiFocusItem, AiRelatedSymbol, AiRepairContract, AiRuleCard,
    TeachingLevel,
};

pub(super) struct DiagnosticContext {
    focus_item: Option<AiFocusItem>,
    relevant_spans: Vec<Span>,
    related_symbols: Vec<AiRelatedSymbol>,
    context_snippets: Vec<AiContextSnippet>,
}

impl DiagnosticContext {
    pub(super) fn new(
        source: &SourceFile,
        program: &Program,
        diagnostic: &Diagnostic,
        rule: &RuleTemplate,
    ) -> Self {
        let mut relevant_spans = vec![diagnostic.span];

        if is_main_required_rule(rule) {
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

    pub(super) fn build(
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

        let contract = AiRepairContract::for_diagnostic(diagnostic);

        AiDiagnostic {
            rule_id: rule.rule_id.to_string(),
            layer: contract.layer,
            ai_action: contract.ai_action,
            safe_to_edit: contract.safe_to_edit,
            validation: contract.validation,
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
    let prefix = visibility_prefix(item.visibility);
    match &item.kind {
        ItemKind::Function {
            name,
            params,
            return_type,
            ..
        } => AiFocusItem {
            kind: "function".to_string(),
            name: name.clone(),
            visibility: item.visibility,
            signature: Some(format!(
                "{prefix}fn {name}({}) -> {}",
                params
                    .iter()
                    .map(|param| format!("{}: {}", param.name, param.ty.describe()))
                    .collect::<Vec<_>>()
                    .join(", "),
                return_type.describe()
            )),
            span: item.span,
        },
        ItemKind::Const { name, ty, .. } => AiFocusItem {
            kind: "const".to_string(),
            name: name.clone(),
            visibility: item.visibility,
            signature: Some(format!("{prefix}const {name}: {}", ty.describe())),
            span: item.span,
        },
        ItemKind::TypeAlias {
            name,
            type_params,
            target,
        } => AiFocusItem {
            kind: "type_alias".to_string(),
            name: name.clone(),
            visibility: item.visibility,
            signature: Some(format!(
                "{prefix}type {name}{} = {}",
                format_type_params(type_params),
                target.describe()
            )),
            span: item.span,
        },
        ItemKind::Struct { name, fields, .. } => AiFocusItem {
            kind: "struct".to_string(),
            name: name.clone(),
            visibility: item.visibility,
            signature: Some(format!(
                "{prefix}struct {name} {{ {} }}",
                fields
                    .iter()
                    .map(|field| format!("{}: {}", field.name, field.ty.describe()))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            span: item.span,
        },
        ItemKind::Enum {
            name,
            type_params,
            variants,
        } => AiFocusItem {
            kind: "enum".to_string(),
            name: name.clone(),
            visibility: item.visibility,
            signature: Some(format!(
                "{prefix}enum {name}{} {{ {} }}",
                format_type_params(type_params),
                variants
                    .iter()
                    .map(|variant| match &variant.payload {
                        Some(payload) => format!("{}({})", variant.name, payload.describe()),
                        None => variant.name.clone(),
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            span: item.span,
        },
        ItemKind::Trait { name, methods } => AiFocusItem {
            kind: "trait".to_string(),
            name: name.clone(),
            visibility: item.visibility,
            signature: Some(format!(
                "{prefix}trait {name} {{ {} }}",
                methods
                    .iter()
                    .map(|method| format!("fn {}(...)", method.name))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            span: item.span,
        },
        ItemKind::Impl {
            type_params,
            trait_ref,
            target,
            ..
        } => AiFocusItem {
            kind: "impl".to_string(),
            name: target.describe(),
            visibility: item.visibility,
            signature: Some(match trait_ref {
                Some(trait_ref) => {
                    format!(
                        "{prefix}impl{} {} for {}",
                        format_type_params(type_params),
                        trait_ref.describe(),
                        target.describe()
                    )
                }
                None => format!(
                    "{prefix}impl{} {}",
                    format_type_params(type_params),
                    target.describe()
                ),
            }),
            span: item.span,
        },
    }
}

fn visibility_prefix(visibility: Visibility) -> &'static str {
    match visibility {
        Visibility::Private => "",
        Visibility::Public => "pub ",
    }
}

fn related_symbols_for_item(program: &Program, focus_item: &Item) -> Vec<AiRelatedSymbol> {
    let mut top_level = BTreeMap::new();
    for item in &program.items {
        let name = match &item.kind {
            ItemKind::Function { name, .. }
            | ItemKind::Const { name, .. }
            | ItemKind::TypeAlias { name, .. }
            | ItemKind::Struct { name, .. }
            | ItemKind::Enum { name, .. }
            | ItemKind::Trait { name, .. } => name.clone(),
            ItemKind::Impl { target, .. } => format!("impl {}", target.describe()),
        };
        top_level.insert(name, item);
    }

    let focus_name = match &focus_item.kind {
        ItemKind::Function { name, .. }
        | ItemKind::Const { name, .. }
        | ItemKind::TypeAlias { name, .. }
        | ItemKind::Struct { name, .. }
        | ItemKind::Enum { name, .. }
        | ItemKind::Trait { name, .. } => name.clone(),
        ItemKind::Impl { target, .. } => format!("impl {}", target.describe()),
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
                collect_type_ref_names(&param.ty, &mut referenced);
            }
            collect_type_ref_names(return_type, &mut referenced);
            collect_block_names(body, &mut referenced);
        }
        ItemKind::Struct { fields, .. } => {
            for field in fields {
                collect_type_ref_names(&field.ty, &mut referenced);
            }
        }
        ItemKind::Const { ty, value, .. } => {
            collect_type_ref_names(ty, &mut referenced);
            collect_expr_names(value, &mut referenced);
        }
        ItemKind::TypeAlias { target, .. } => {
            collect_type_ref_names(target, &mut referenced);
        }
        ItemKind::Enum { .. } => {}
        ItemKind::Trait { methods, .. } => {
            for method in methods {
                for param in &method.params {
                    collect_type_ref_names(&param.ty, &mut referenced);
                }
                collect_type_ref_names(&method.return_type, &mut referenced);
            }
        }
        ItemKind::Impl {
            type_params,
            trait_ref,
            target,
            methods,
        } => {
            if let Some(trait_ref) = trait_ref {
                collect_type_ref_names(trait_ref, &mut referenced);
            }
            collect_type_ref_names(target, &mut referenced);
            for type_param in type_params {
                referenced.remove(type_param);
            }
            for method in methods {
                for param in &method.params {
                    collect_type_ref_names(&param.ty, &mut referenced);
                }
                collect_type_ref_names(&method.return_type, &mut referenced);
                collect_block_names(&method.body, &mut referenced);
            }
        }
    }

    referenced
        .into_iter()
        .filter(|name| name != &focus_name)
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
            collect_type_ref_names(ty, names);
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
        StmtKind::Break => {}
        StmtKind::Continue => {}
        StmtKind::Match { scrutinee, arms } => {
            collect_expr_names(scrutinee, names);
            for arm in arms {
                collect_match_pattern_names(&arm.pattern, names);
                collect_block_names(&arm.body, names);
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
        StmtKind::ForIn {
            binding,
            iterable,
            body,
        } => {
            collect_type_ref_names(&binding.ty, names);
            collect_expr_names(iterable, names);
            collect_block_names(body, names);
        }
        StmtKind::Block { block } => collect_block_names(block, names),
    }
}

fn collect_match_pattern_names(pattern: &MatchPattern, names: &mut BTreeSet<String>) {
    if let MatchPatternKind::EnumVariant { path, .. } = &pattern.kind
        && let Some((enum_path, _)) = path.rsplit_once('.')
    {
        names.insert(enum_path.to_string());
    }
    if let MatchPatternKind::Struct { path, .. } = &pattern.kind {
        names.insert(path.to_string());
    }
}

fn collect_expr_names(expr: &Expr, names: &mut BTreeSet<String>) {
    match &expr.kind {
        ExprKind::Name { value } => {
            names.insert(value.clone());
        }
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => collect_expr_names(expr, names),
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
        ExprKind::ArrayLiteral { elements } => {
            for element in elements {
                collect_expr_names(element, names);
            }
        }
        ExprKind::Block { statements, value } => {
            for statement in statements {
                collect_statement_names(statement, names);
            }
            collect_expr_names(value, names);
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_expr_names(scrutinee, names);
            for arm in arms {
                collect_match_pattern_names(&arm.pattern, names);
                collect_expr_names(&arm.value, names);
            }
        }
        ExprKind::Field { base, .. } => collect_expr_names(base, names),
        ExprKind::Index { base, index } => {
            collect_expr_names(base, names);
            collect_expr_names(index, names);
        }
        ExprKind::Slice { base, start, end } => {
            collect_expr_names(base, names);
            collect_expr_names(start, names);
            collect_expr_names(end, names);
        }
        ExprKind::Int { .. }
        | ExprKind::Float { .. }
        | ExprKind::Bool { .. }
        | ExprKind::String { .. }
        | ExprKind::Error => {}
    }
}

fn collect_type_ref_names(ty: &TypeRef, names: &mut BTreeSet<String>) {
    match (&ty.name, &ty.element, ty.length) {
        (Some(name), None, None) => {
            names.insert(name.clone());
        }
        (None, Some(element), None) | (None, Some(element), Some(_)) => {
            collect_type_ref_names(element, names)
        }
        _ => {}
    }
}

fn format_type_params(type_params: &[String]) -> String {
    if type_params.is_empty() {
        String::new()
    } else {
        format!("<{}>", type_params.join(", "))
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
            StmtKind::Match { arms, .. } => {
                for arm in arms {
                    if let Some(inner) = find_smallest_statement_span(&arm.body, target) {
                        found = Some(inner);
                    }
                }
            }
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
            StmtKind::ForIn { body, .. } => {
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
    let segment_end = source.segment_end(span.start);
    let mut end_offset = span.end.min(segment_end);
    if end_offset == span.start {
        end_offset = end_offset.saturating_add(1).min(segment_end);
    }
    let safe_end_offset = end_offset
        .saturating_sub(1)
        .max(span.start)
        .min(segment_end.saturating_sub(1));
    let (end_line, _) = source.line_col(safe_end_offset);
    let stop = end_line.min(start_line + max_lines.saturating_sub(1));
    let mut lines = Vec::new();
    for line in start_line..=stop {
        lines.push(source.line_text_for_offset(span.start, line).to_string());
    }
    if end_line > stop {
        lines.push("...".to_string());
    }
    lines.join("\n").trim().to_string()
}
