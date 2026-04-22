use std::fmt::Write;

use crate::ast::{
    BinaryOp, Block, Expr, ExprKind, Item, ItemKind, Param, Program, Stmt, StmtKind, StructField,
    StructLiteralField, TypeRef, UnaryOp,
};
use crate::diagnostics::Diagnostic;
use crate::lexer::tokenize;
use crate::parser::parse;
use crate::source::SourceFile;

const PREC_EQUALITY: u8 = 10;
const PREC_COMPARISON: u8 = 20;
const PREC_ADDITIVE: u8 = 30;
const PREC_MULTIPLICATIVE: u8 = 40;
const PREC_UNARY: u8 = 50;
const PREC_POSTFIX: u8 = 60;
const PREC_PRIMARY: u8 = 70;

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

    fn format_program(&mut self, program: &Program) {
        for (index, item) in program.items.iter().enumerate() {
            self.format_item(item);
            if index + 1 < program.items.len() {
                self.out.push_str("\n\n");
            }
        }
    }

    fn format_item(&mut self, item: &Item) {
        match &item.kind {
            ItemKind::Function {
                name,
                params,
                return_type,
                body,
            } => self.format_function_item(name, params, return_type, body),
            ItemKind::Struct { name, fields } => self.format_struct_item(name, fields),
            ItemKind::Enum { name, variants } => self.format_enum_item(name, variants),
        }
    }

    fn format_function_item(
        &mut self,
        name: &str,
        params: &[Param],
        return_type: &TypeRef,
        body: &Block,
    ) {
        let _ = write!(self.out, "fn {name}(");
        for (index, param) in params.iter().enumerate() {
            if index > 0 {
                self.out.push_str(", ");
            }
            let _ = write!(self.out, "{}: {}", param.name, param.ty.name);
        }
        let _ = write!(self.out, ") -> {} ", return_type.name);
        self.format_block(body);
    }

    fn format_struct_item(&mut self, name: &str, fields: &[StructField]) {
        if fields.is_empty() {
            let _ = write!(self.out, "struct {name} {{}}");
            return;
        }

        let _ = writeln!(self.out, "struct {name} {{");
        self.indent += 1;
        for field in fields {
            self.write_indent();
            let _ = writeln!(self.out, "{}: {},", field.name, field.ty.name);
        }
        self.indent -= 1;
        self.write_indent();
        self.out.push('}');
    }

    fn format_enum_item(&mut self, name: &str, variants: &[crate::ast::EnumVariant]) {
        if variants.is_empty() {
            let _ = write!(self.out, "enum {name} {{}}");
            return;
        }

        let _ = writeln!(self.out, "enum {name} {{");
        self.indent += 1;
        for variant in variants {
            self.write_indent();
            let _ = writeln!(self.out, "{},", variant.name);
        }
        self.indent -= 1;
        self.write_indent();
        self.out.push('}');
    }

    fn format_block(&mut self, block: &Block) {
        if block.statements.is_empty() {
            self.out.push_str("{}");
            return;
        }

        self.out.push_str("{\n");
        self.indent += 1;
        for statement in &block.statements {
            self.write_indent();
            self.format_statement(statement);
            self.out.push('\n');
        }
        self.indent -= 1;
        self.write_indent();
        self.out.push('}');
    }

    fn format_statement(&mut self, statement: &Stmt) {
        match &statement.kind {
            StmtKind::Let {
                mutable,
                name,
                ty,
                initializer,
            } => {
                let binding = if *mutable { "let mut" } else { "let" };
                let _ = write!(
                    self.out,
                    "{binding} {name}: {} = {};",
                    ty.name,
                    format_expr(initializer)
                );
            }
            StmtKind::Assign { target, value } => {
                let _ = write!(
                    self.out,
                    "{} = {};",
                    format_expr(target),
                    format_expr(value)
                );
            }
            StmtKind::Expr { expr } => {
                let _ = write!(self.out, "{};", format_expr(expr));
            }
            StmtKind::Return { value } => {
                if let Some(expr) = value {
                    let _ = write!(self.out, "return {};", format_expr(expr));
                } else {
                    self.out.push_str("return;");
                }
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => self.format_if_statement(condition, then_branch, else_branch.as_ref()),
            StmtKind::While { condition, body } => {
                let _ = write!(self.out, "while ({}) ", format_expr(condition));
                self.format_block(body);
            }
            StmtKind::For {
                initializer,
                condition,
                step,
                body,
            } => {
                self.out.push_str("for (");
                if let Some(statement) = initializer {
                    self.out
                        .push_str(&format_for_header_statement(statement.as_ref()));
                }
                self.out.push(';');
                if let Some(expr) = condition {
                    self.out.push(' ');
                    self.out.push_str(&format_expr(expr));
                }
                self.out.push(';');
                if let Some(statement) = step {
                    self.out.push(' ');
                    self.out
                        .push_str(&format_for_header_statement(statement.as_ref()));
                }
                self.out.push_str(") ");
                self.format_block(body);
            }
            StmtKind::Block { block } => self.format_block(block),
        }
    }

    fn format_if_statement(
        &mut self,
        condition: &Expr,
        then_branch: &Block,
        else_branch: Option<&Block>,
    ) {
        let _ = write!(self.out, "if ({}) ", format_expr(condition));
        self.format_block(then_branch);
        if let Some(block) = else_branch {
            if let Some(else_if) = else_if_statement(block) {
                self.out.push_str(" else ");
                if let StmtKind::If {
                    condition,
                    then_branch,
                    else_branch,
                } = &else_if.kind
                {
                    self.format_if_statement(condition, then_branch, else_branch.as_ref());
                }
            } else {
                self.out.push_str(" else ");
                self.format_block(block);
            }
        }
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.out.push_str("    ");
        }
    }
}

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
                ty.name,
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
        ExprKind::Field { base, field } => {
            let base_text = format_expr_with_min_precedence(base, PREC_POSTFIX);
            format!("{base_text}.{field}")
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

fn expr_precedence(expr: &Expr) -> u8 {
    match &expr.kind {
        ExprKind::Binary { op, .. } => binary_precedence(*op),
        ExprKind::Unary { .. } => PREC_UNARY,
        ExprKind::Call { .. } | ExprKind::Field { .. } => PREC_POSTFIX,
        ExprKind::Int { .. }
        | ExprKind::Float { .. }
        | ExprKind::Bool { .. }
        | ExprKind::String { .. }
        | ExprKind::Name { .. }
        | ExprKind::StructLiteral { .. }
        | ExprKind::Error => PREC_PRIMARY,
    }
}

fn binary_precedence(op: BinaryOp) -> u8 {
    match op {
        BinaryOp::Equal | BinaryOp::NotEqual => PREC_EQUALITY,
        BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
            PREC_COMPARISON
        }
        BinaryOp::Add | BinaryOp::Subtract => PREC_ADDITIVE,
        BinaryOp::Multiply | BinaryOp::Divide => PREC_MULTIPLICATIVE,
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
mod tests {
    use super::format_source;
    use crate::source::SourceFile;

    #[test]
    fn formats_current_prototype_syntax() {
        let source = SourceFile::anonymous(
            "struct Point{x:i32,y:i32} enum Flag{On,Off} fn main()->i32{let mut point:Point=Point{x:1,y:2};if(point.x==1){println(\"ready\");}else if(point.x==2){println(\"fallback\");}else{println(\"other\");}for(let mut i:i32=0;i<2;i=i+1){point.x=point.x+i;}return 0;}",
        );

        let formatted = format_source(&source).expect("source should format");
        assert_eq!(
            formatted,
            "\
struct Point {\n\
    x: i32,\n\
    y: i32,\n\
}\n\
\n\
enum Flag {\n\
    On,\n\
    Off,\n\
}\n\
\n\
fn main() -> i32 {\n\
    let mut point: Point = Point { x: 1, y: 2 };\n\
    if (point.x == 1) {\n\
        println(\"ready\");\n\
    } else if (point.x == 2) {\n\
        println(\"fallback\");\n\
    } else {\n\
        println(\"other\");\n\
    }\n\
    for (let mut i: i32 = 0; i < 2; i = i + 1) {\n\
        point.x = point.x + i;\n\
    }\n\
    return 0;\n\
}\n"
        );
    }

    #[test]
    fn formatting_is_idempotent_for_formatted_input() {
        let source = SourceFile::anonymous(
            "fn main() -> i32 {\n    let value: f32 = 3.0;\n    println(\"line\\nvalue\");\n    return 0;\n}\n",
        );

        let first = format_source(&source).expect("source should format");
        let second =
            format_source(&SourceFile::anonymous(first.clone())).expect("source should reformat");
        assert_eq!(first, second);
    }

    #[test]
    fn formatter_reports_parse_errors() {
        let source = SourceFile::anonymous("fn main() -> i32 { let value: i32 = 1 return value; }");
        assert!(format_source(&source).is_err());
    }
}
