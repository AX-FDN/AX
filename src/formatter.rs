use std::fmt::Write;

use crate::ast::{
    BinaryOp, Block, Expr, ExprKind, ImplMethod, Item, ItemKind, MatchArm, MatchExprArm,
    MatchPattern, MatchPatternKind, Param, Program, Stmt, StmtKind, StructField,
    StructLiteralField, TraitMethod, TypeParamBound, TypeRef, UnaryOp, Visibility,
};
use crate::diagnostics::Diagnostic;
use crate::lexer::tokenize;
use crate::parser::parse;
use crate::source::SourceFile;

const PREC_LOGICAL_OR: u8 = 5;
const PREC_LOGICAL_AND: u8 = 10;
const PREC_EQUALITY: u8 = 20;
const PREC_COMPARISON: u8 = 30;
const PREC_ADDITIVE: u8 = 40;
const PREC_MULTIPLICATIVE: u8 = 50;
const PREC_UNARY: u8 = 60;
const PREC_POSTFIX: u8 = 70;
const PREC_PRIMARY: u8 = 80;

pub fn format_source(source: &SourceFile) -> Result<String, Vec<Diagnostic>> {
    let lexer_output = tokenize(source);
    let parse_output = parse(source, lexer_output.tokens);

    let mut diagnostics = lexer_output.diagnostics;
    diagnostics.extend(parse_output.diagnostics);
    diagnostics.sort_by(|left, right| {
        left.span
            .start
            .cmp(&right.span.start)
            .then(left.span.end.cmp(&right.span.end))
            .then(left.code.cmp(&right.code))
    });

    if diagnostics.is_empty() {
        Ok(format_program(&parse_output.program))
    } else {
        Err(diagnostics)
    }
}

pub fn format_program(program: &Program) -> String {
    let mut formatter = Formatter::new();
    formatter.format_program(program);
    formatter.finish()
}

struct Formatter {
    out: String,
    indent: usize,
}

impl Formatter {
    fn new() -> Self {
        Self {
            out: String::new(),
            indent: 0,
        }
    }

    fn finish(mut self) -> String {
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        self.out
    }
}

#[path = "formatter/items.rs"]
mod items;
#[path = "formatter/statements.rs"]
mod statements;

fn else_if_statement(block: &Block) -> Option<&Stmt> {
    if block.statements.len() != 1 {
        return None;
    }

    let statement = &block.statements[0];
    if matches!(statement.kind, StmtKind::If { .. }) {
        Some(statement)
    } else {
        None
    }
}

fn format_for_header_statement(statement: &Stmt) -> String {
    match &statement.kind {
        StmtKind::Let {
            mutable,
            name,
            ty,
            initializer,
        } => {
            let binding = if *mutable { "let mut" } else { "let" };
            format!(
                "{binding} {name}: {} = {}",
                format_type_ref(ty),
                format_expr(initializer)
            )
        }
        StmtKind::Assign { target, value } => {
            format!("{} = {}", format_expr(target), format_expr(value))
        }
        StmtKind::Expr { expr } => format_expr(expr),
        _ => "<unsupported-for-header>".to_string(),
    }
}

fn format_match_pattern(pattern: &MatchPattern) -> String {
    match &pattern.kind {
        MatchPatternKind::Wildcard => "_".to_string(),
        MatchPatternKind::Binding { name } => name.clone(),
        MatchPatternKind::Bool { value } => value.to_string(),
        MatchPatternKind::Int { value } => value.to_string(),
        MatchPatternKind::IntRange { start, end } => format!("{start}..={end}"),
        MatchPatternKind::String { value } => escape_string_literal(value),
        MatchPatternKind::EnumVariant { path, payload } => match payload {
            Some(crate::ast::EnumVariantPayloadPattern::Wildcard) => format!("{path}(_)"),
            Some(crate::ast::EnumVariantPayloadPattern::Binding { name }) => {
                format!("{path}({name})")
            }
            None => path.clone(),
        },
        MatchPatternKind::Struct { path, fields } => {
            let fields = fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!("{path} {{ {fields} }}")
        }
        MatchPatternKind::Or { alternatives } => alternatives
            .iter()
            .map(format_match_pattern)
            .collect::<Vec<_>>()
            .join(" | "),
        MatchPatternKind::Error => "<invalid-pattern>".to_string(),
    }
}

fn format_match_expression_arms(arms: &[MatchExprArm]) -> String {
    arms.iter()
        .map(|arm| {
            format!(
                "{}{} => {}",
                format_match_pattern(&arm.pattern),
                format_match_guard(arm.guard.as_ref()),
                format_expr(&arm.value)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_match_guard(guard: Option<&Expr>) -> String {
    guard
        .map(|guard| format!(" if {}", format_expr(guard)))
        .unwrap_or_default()
}

fn format_expr(expr: &Expr) -> String {
    format_expr_with_min_precedence(expr, 0)
}

fn format_expr_with_min_precedence(expr: &Expr, min_precedence: u8) -> String {
    let precedence = expr_precedence(expr);
    let text = match &expr.kind {
        ExprKind::Int { value } => value.to_string(),
        ExprKind::Float { value } => format_float_literal(*value),
        ExprKind::Bool { value } => value.to_string(),
        ExprKind::String { value } => escape_string_literal(value),
        ExprKind::Name { value } => value.clone(),
        ExprKind::Unary { op, expr } => {
            let operand = format_expr_with_min_precedence(expr, PREC_UNARY + 1);
            format!("{}{}", unary_op_text(*op), operand)
        }
        ExprKind::Try { expr } => {
            let operand = format_expr_with_min_precedence(expr, PREC_POSTFIX);
            format!("{operand}?")
        }
        ExprKind::Binary { op, left, right } => {
            let precedence = binary_precedence(*op);
            let left_text = format_expr_with_min_precedence(left, precedence);
            let right_text = format_expr_with_min_precedence(right, precedence + 1);
            format!("{left_text} {} {right_text}", binary_op_text(*op))
        }
        ExprKind::Call { callee, arguments } => {
            let callee_text = format_expr_with_min_precedence(callee, PREC_POSTFIX);
            let arguments_text = arguments
                .iter()
                .map(format_expr)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{callee_text}({arguments_text})")
        }
        ExprKind::StructLiteral { name, fields } => {
            format!("{name} {}", format_struct_literal_fields(fields))
        }
        ExprKind::ArrayLiteral { elements } => {
            let body = elements
                .iter()
                .map(format_expr)
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{body}]")
        }
        ExprKind::Block { statements, value } => format_block_expr(statements, value),
        ExprKind::Match { scrutinee, arms } => format!(
            "match ({}) {{ {} }}",
            format_expr(scrutinee),
            format_match_expression_arms(arms)
        ),
        ExprKind::Field { base, field } => {
            let base_text = format_expr_with_min_precedence(base, PREC_POSTFIX);
            format!("{base_text}.{field}")
        }
        ExprKind::Index { base, index } => {
            let base_text = format_expr_with_min_precedence(base, PREC_POSTFIX);
            let index_text = format_expr(index);
            format!("{base_text}[{index_text}]")
        }
        ExprKind::Slice { base, start, end } => {
            let base_text = format_expr_with_min_precedence(base, PREC_POSTFIX);
            let start_text = format_expr(start);
            let end_text = format_expr(end);
            format!("{base_text}[{start_text}:{end_text}]")
        }
        ExprKind::Error => "<error>".to_string(),
    };

    if precedence < min_precedence {
        format!("({text})")
    } else {
        text
    }
}

fn format_struct_literal_fields(fields: &[StructLiteralField]) -> String {
    if fields.is_empty() {
        return "{}".to_string();
    }

    let body = fields
        .iter()
        .map(|field| format!("{}: {}", field.name, format_expr(&field.value)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{ {body} }}")
}

fn format_type_ref(ty: &TypeRef) -> String {
    match (&ty.name, &ty.type_args[..], &ty.element, ty.length) {
        (Some(name), [], None, None) => name.clone(),
        (Some(name), args, None, None) => {
            let args = args
                .iter()
                .map(format_type_ref)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{name}<{args}>")
        }
        (None, [], Some(element), None) => format!("[{}]", format_type_ref(element)),
        (None, [], Some(element), Some(length)) => {
            format!("[{}; {}]", format_type_ref(element), length)
        }
        _ => "<invalid-type>".to_string(),
    }
}

fn format_type_params(type_params: &[String]) -> String {
    if type_params.is_empty() {
        String::new()
    } else {
        format!("<{}>", type_params.join(", "))
    }
}

fn format_function_type_params(
    type_params: &[String],
    type_param_bounds: &[TypeParamBound],
) -> String {
    if type_params.is_empty() {
        return String::new();
    }

    let params = type_params
        .iter()
        .map(|param| {
            let bounds = type_param_bounds
                .iter()
                .filter(|bound| bound.type_param == *param)
                .map(|bound| format_type_ref(&bound.trait_ref))
                .collect::<Vec<_>>();
            if bounds.is_empty() {
                param.clone()
            } else {
                format!("{param}: {}", bounds.join(" + "))
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("<{params}>")
}

fn expr_precedence(expr: &Expr) -> u8 {
    match &expr.kind {
        ExprKind::Binary { op, .. } => binary_precedence(*op),
        ExprKind::Unary { .. } => PREC_UNARY,
        ExprKind::Call { .. }
        | ExprKind::Try { .. }
        | ExprKind::Field { .. }
        | ExprKind::Index { .. }
        | ExprKind::Slice { .. } => PREC_POSTFIX,
        ExprKind::Int { .. }
        | ExprKind::Float { .. }
        | ExprKind::Bool { .. }
        | ExprKind::String { .. }
        | ExprKind::Name { .. }
        | ExprKind::Block { .. }
        | ExprKind::Match { .. }
        | ExprKind::StructLiteral { .. }
        | ExprKind::ArrayLiteral { .. }
        | ExprKind::Error => PREC_PRIMARY,
    }
}

fn format_block_expr(statements: &[Stmt], value: &Expr) -> String {
    let mut parts = statements
        .iter()
        .map(|statement| {
            let mut formatter = Formatter {
                out: String::new(),
                indent: 0,
            };
            formatter.format_statement(statement);
            formatter.out
        })
        .collect::<Vec<_>>();
    parts.push(format_expr(value));
    format!("{{ {} }}", parts.join(" "))
}

fn binary_precedence(op: BinaryOp) -> u8 {
    match op {
        BinaryOp::LogicalOr => PREC_LOGICAL_OR,
        BinaryOp::LogicalAnd => PREC_LOGICAL_AND,
        BinaryOp::Equal | BinaryOp::NotEqual => PREC_EQUALITY,
        BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
            PREC_COMPARISON
        }
        BinaryOp::Add | BinaryOp::Subtract => PREC_ADDITIVE,
        BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Remainder => PREC_MULTIPLICATIVE,
    }
}

fn unary_op_text(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Negate => "-",
        UnaryOp::Not => "!",
    }
}

fn binary_op_text(op: BinaryOp) -> &'static str {
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

fn format_float_literal(value: f64) -> String {
    let text = value.to_string();
    if text.contains('.') || text.contains('e') || text.contains('E') {
        text
    } else {
        format!("{text}.0")
    }
}

fn escape_string_literal(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
#[path = "formatter/tests.rs"]
mod tests;
