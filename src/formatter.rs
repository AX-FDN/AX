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

    fn format_program(&mut self, program: &Program) {
        for (index, item) in program.items.iter().enumerate() {
            self.format_item(item);
            if index + 1 < program.items.len() {
                self.out.push_str("\n\n");
            }
        }
    }

    fn format_item(&mut self, item: &Item) {
        if item.visibility == Visibility::Public {
            self.out.push_str("pub ");
        }
        match &item.kind {
            ItemKind::Function {
                name,
                type_params,
                type_param_bounds,
                params,
                return_type,
                body,
            } => self.format_function_item(
                name,
                type_params,
                type_param_bounds,
                params,
                return_type,
                body,
            ),
            ItemKind::Const { name, ty, value } => self.format_const_item(name, ty, value),
            ItemKind::TypeAlias {
                name,
                type_params,
                target,
            } => self.format_type_alias_item(name, type_params, target),
            ItemKind::Struct {
                name,
                type_params,
                fields,
            } => self.format_struct_item(name, type_params, fields),
            ItemKind::Enum {
                name,
                type_params,
                variants,
            } => self.format_enum_item(name, type_params, variants),
            ItemKind::Trait { name, methods } => self.format_trait_item(name, methods),
            ItemKind::Impl {
                type_params,
                trait_ref,
                target,
                methods,
            } => self.format_impl_item(type_params, trait_ref.as_ref(), target, methods),
        }
    }

    fn format_const_item(&mut self, name: &str, ty: &TypeRef, value: &Expr) {
        let _ = write!(
            self.out,
            "const {name}: {} = {};",
            format_type_ref(ty),
            format_expr(value)
        );
    }

    fn format_type_alias_item(&mut self, name: &str, type_params: &[String], target: &TypeRef) {
        let _ = write!(
            self.out,
            "type {name}{} = {};",
            format_type_params(type_params),
            format_type_ref(target)
        );
    }

    fn format_function_item(
        &mut self,
        name: &str,
        type_params: &[String],
        type_param_bounds: &[TypeParamBound],
        params: &[Param],
        return_type: &TypeRef,
        body: &Block,
    ) {
        let _ = write!(
            self.out,
            "fn {name}{}(",
            format_function_type_params(type_params, type_param_bounds)
        );
        for (index, param) in params.iter().enumerate() {
            if index > 0 {
                self.out.push_str(", ");
            }
            let _ = write!(self.out, "{}: {}", param.name, format_type_ref(&param.ty));
        }
        let _ = write!(self.out, ") -> {} ", format_type_ref(return_type));
        self.format_block(body);
    }

    fn format_struct_item(&mut self, name: &str, type_params: &[String], fields: &[StructField]) {
        let params = format_type_params(type_params);
        if fields.is_empty() {
            let _ = write!(self.out, "struct {name}{params} {{}}");
            return;
        }

        let _ = writeln!(self.out, "struct {name}{params} {{");
        self.indent += 1;
        for field in fields {
            self.write_indent();
            let _ = writeln!(self.out, "{}: {},", field.name, format_type_ref(&field.ty));
        }
        self.indent -= 1;
        self.write_indent();
        self.out.push('}');
    }

    fn format_impl_item(
        &mut self,
        type_params: &[String],
        trait_ref: Option<&TypeRef>,
        target: &TypeRef,
        methods: &[ImplMethod],
    ) {
        let params = format_type_params(type_params);
        if let Some(trait_ref) = trait_ref {
            let _ = writeln!(
                self.out,
                "impl{} {} for {} {{",
                params,
                format_type_ref(trait_ref),
                format_type_ref(target)
            );
        } else {
            let _ = writeln!(self.out, "impl{} {} {{", params, format_type_ref(target));
        }
        self.indent += 1;
        for (index, method) in methods.iter().enumerate() {
            self.write_indent();
            self.format_method_item(method);
            if index + 1 < methods.len() {
                self.out.push('\n');
                self.out.push('\n');
            } else {
                self.out.push('\n');
            }
        }
        self.indent -= 1;
        self.write_indent();
        self.out.push('}');
    }

    fn format_trait_item(&mut self, name: &str, methods: &[TraitMethod]) {
        if methods.is_empty() {
            let _ = write!(self.out, "trait {name} {{}}");
            return;
        }

        let _ = writeln!(self.out, "trait {name} {{");
        self.indent += 1;
        for method in methods {
            self.write_indent();
            let _ = write!(self.out, "fn {}(", method.name);
            for (index, param) in method.params.iter().enumerate() {
                if index > 0 {
                    self.out.push_str(", ");
                }
                let _ = write!(self.out, "{}: {}", param.name, format_type_ref(&param.ty));
            }
            let _ = writeln!(self.out, ") -> {};", format_type_ref(&method.return_type));
        }
        self.indent -= 1;
        self.write_indent();
        self.out.push('}');
    }

    fn format_method_item(&mut self, method: &ImplMethod) {
        let _ = write!(
            self.out,
            "fn {}{}(",
            method.name,
            format_type_params(&method.type_params)
        );
        for (index, param) in method.params.iter().enumerate() {
            if index > 0 {
                self.out.push_str(", ");
            }
            let _ = write!(self.out, "{}: {}", param.name, format_type_ref(&param.ty));
        }
        let _ = write!(self.out, ") -> {} ", format_type_ref(&method.return_type));
        self.format_block(&method.body);
    }

    fn format_enum_item(
        &mut self,
        name: &str,
        type_params: &[String],
        variants: &[crate::ast::EnumVariant],
    ) {
        let params = format_type_params(type_params);
        if variants.is_empty() {
            let _ = write!(self.out, "enum {name}{params} {{}}");
            return;
        }

        let _ = writeln!(self.out, "enum {name}{params} {{");
        self.indent += 1;
        for variant in variants {
            self.write_indent();
            if let Some(payload) = &variant.payload {
                let _ = writeln!(self.out, "{}({}),", variant.name, format_type_ref(payload));
            } else {
                let _ = writeln!(self.out, "{},", variant.name);
            }
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
                    format_type_ref(ty),
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
            StmtKind::Break => self.out.push_str("break;"),
            StmtKind::Continue => self.out.push_str("continue;"),
            StmtKind::Match { scrutinee, arms } => self.format_match_statement(scrutinee, arms),
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
            StmtKind::ForIn {
                binding,
                iterable,
                body,
            } => {
                let binding_prefix = if binding.mutable { "let mut" } else { "let" };
                let _ = write!(
                    self.out,
                    "for ({} {}: {} in {}) ",
                    binding_prefix,
                    binding.name,
                    format_type_ref(&binding.ty),
                    format_expr(iterable)
                );
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

    fn format_match_statement(&mut self, scrutinee: &Expr, arms: &[MatchArm]) {
        let _ = write!(self.out, "match ({}) ", format_expr(scrutinee));
        if arms.is_empty() {
            self.out.push_str("{}");
            return;
        }

        self.out.push_str("{\n");
        self.indent += 1;
        for arm in arms {
            self.write_indent();
            let _ = write!(
                self.out,
                "{}{} => ",
                format_match_pattern(&arm.pattern),
                format_match_guard(arm.guard.as_ref())
            );
            self.format_block(&arm.body);
            self.out.push('\n');
        }
        self.indent -= 1;
        self.write_indent();
        self.out.push('}');
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
        | ExprKind::Field { .. }
        | ExprKind::Index { .. }
        | ExprKind::Slice { .. } => PREC_POSTFIX,
        ExprKind::Int { .. }
        | ExprKind::Float { .. }
        | ExprKind::Bool { .. }
        | ExprKind::String { .. }
        | ExprKind::Name { .. }
        | ExprKind::Match { .. }
        | ExprKind::StructLiteral { .. }
        | ExprKind::ArrayLiteral { .. }
        | ExprKind::Error => PREC_PRIMARY,
    }
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
mod tests {
    use super::format_source;
    use crate::source::SourceFile;

    #[test]
    fn formats_current_prototype_syntax() {
        let source = SourceFile::anonymous(
            "struct Point{x:i32,y:i32} enum Flag{On,Off} fn main()->i32{let mut point:Point=Point{x:1,y:2};let values:[i32;3]=[1,2,3];if(point.x==1){println(\"ready\");}else if(point.x==2){println(values[1]);}else{println(\"other\");}for(let mut i:i32=0;i<2;i=i+1){if(i==1){continue;}point.x=point.x+i;if(i>2){break;}}return values[0];}",
        );

        let formatted = format_source(&source).expect("source should format");
        assert_eq!(
            formatted,
            concat!(
                "struct Point {\n",
                "    x: i32,\n",
                "    y: i32,\n",
                "}\n",
                "\n",
                "enum Flag {\n",
                "    On,\n",
                "    Off,\n",
                "}\n",
                "\n",
                "fn main() -> i32 {\n",
                "    let mut point: Point = Point { x: 1, y: 2 };\n",
                "    let values: [i32; 3] = [1, 2, 3];\n",
                "    if (point.x == 1) {\n",
                "        println(\"ready\");\n",
                "    } else if (point.x == 2) {\n",
                "        println(values[1]);\n",
                "    } else {\n",
                "        println(\"other\");\n",
                "    }\n",
                "    for (let mut i: i32 = 0; i < 2; i = i + 1) {\n",
                "        if (i == 1) {\n",
                "            continue;\n",
                "        }\n",
                "        point.x = point.x + i;\n",
                "        if (i > 2) {\n",
                "            break;\n",
                "        }\n",
                "    }\n",
                "    return values[0];\n",
                "}\n"
            )
        );
    }

    #[test]
    fn formats_const_items() {
        let source = SourceFile::anonymous("const EXIT_OK:i32=7;fn main()->i32{return EXIT_OK;}");
        let formatted = format_source(&source).expect("source should format");
        assert_eq!(
            formatted,
            "const EXIT_OK: i32 = 7;\n\nfn main() -> i32 {\n    return EXIT_OK;\n}\n"
        );
    }

    #[test]
    fn formats_public_items() {
        let source = SourceFile::anonymous("pub fn helper()->i32{return 1;}");
        let formatted = format_source(&source).expect("source should format");
        assert_eq!(formatted, "pub fn helper() -> i32 {\n    return 1;\n}\n");
    }

    #[test]
    fn formats_multiple_trait_bounds() {
        let source = SourceFile::anonymous(
            "fn render<T:Label+Code>(value:T)->string{return value.label();}",
        );
        let formatted = format_source(&source).expect("source should format");
        assert_eq!(
            formatted,
            "fn render<T: Label + Code>(value: T) -> string {\n    return value.label();\n}\n"
        );
    }

    #[test]
    fn formats_where_trait_bounds_into_canonical_generic_params() {
        let source = SourceFile::anonymous(
            "fn render<T>(value:T)->string where T:Label+Code{return value.label();}",
        );
        let formatted = format_source(&source).expect("source should format");
        assert_eq!(
            formatted,
            "fn render<T: Label + Code>(value: T) -> string {\n    return value.label();\n}\n"
        );
    }

    #[test]
    fn formats_type_alias_items() {
        let source = SourceFile::anonymous("type UserId=i32;fn main()->i32{return 0;}");
        let formatted = format_source(&source).expect("source should format");
        assert_eq!(
            formatted,
            "type UserId = i32;\n\nfn main() -> i32 {\n    return 0;\n}\n"
        );
    }

    #[test]
    fn formats_generic_impl_blocks() {
        let source = SourceFile::anonymous(
            "struct Box<T>{value:T}impl<T> Box<T>{fn get(self:Box<T>)->T{return self.value;}}",
        );
        let formatted = format_source(&source).expect("source should format");
        assert_eq!(
            formatted,
            concat!(
                "struct Box<T> {\n",
                "    value: T,\n",
                "}\n",
                "\n",
                "impl<T> Box<T> {\n",
                "    fn get(self: Box<T>) -> T {\n",
                "        return self.value;\n",
                "    }\n",
                "}\n"
            )
        );
    }

    #[test]
    fn formats_generic_impl_methods() {
        let source = SourceFile::anonymous(
            "struct Pair<T,U>{left:T,right:U}impl<T> Pair<T,i32>{fn replace_right<U>(self:Pair<T,i32>,right:U)->Pair<T,U>{return Pair{left:self.left,right:right};}}",
        );
        let formatted = format_source(&source).expect("source should format");
        assert_eq!(
            formatted,
            "struct Pair<T, U> {\n    left: T,\n    right: U,\n}\n\nimpl<T> Pair<T, i32> {\n    fn replace_right<U>(self: Pair<T, i32>, right: U) -> Pair<T, U> {\n        return Pair { left: self.left, right: right };\n    }\n}\n"
        );
    }

    #[test]
    fn formats_generic_trait_impl_blocks() {
        let source = SourceFile::anonymous(
            "trait Label{fn label(self:Self)->string;}struct Box<T>{value:T}impl<T> Label for Box<T>{fn label(self:Box<T>)->string{return to_string(self.value);}}",
        );
        let formatted = format_source(&source).expect("source should format");
        assert_eq!(
            formatted,
            concat!(
                "trait Label {\n",
                "    fn label(self: Self) -> string;\n",
                "}\n",
                "\n",
                "struct Box<T> {\n",
                "    value: T,\n",
                "}\n",
                "\n",
                "impl<T> Label for Box<T> {\n",
                "    fn label(self: Box<T>) -> string {\n",
                "        return to_string(self.value);\n",
                "    }\n",
                "}\n"
            )
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
    fn formats_slice_types_and_expressions() {
        let source = SourceFile::anonymous(
            "fn take(window:[i32])->i32{let head:[i32]=window[0:2];return head[1];}",
        );

        let formatted = format_source(&source).expect("source should format");
        assert_eq!(
            formatted,
            concat!(
                "fn take(window: [i32]) -> i32 {\n",
                "    let head: [i32] = window[0:2];\n",
                "    return head[1];\n",
                "}\n"
            )
        );
    }

    #[test]
    fn formats_match_statements() {
        let source = SourceFile::anonymous(
            "enum Flag{On,Off} fn choose(flag:Flag)->i32{match(flag){Flag.On=>{return 1;} Flag.Off=>{return 0;}}}",
        );

        let formatted = format_source(&source).expect("source should format");
        assert_eq!(
            formatted,
            concat!(
                "enum Flag {\n",
                "    On,\n",
                "    Off,\n",
                "}\n",
                "\n",
                "fn choose(flag: Flag) -> i32 {\n",
                "    match (flag) {\n",
                "        Flag.On => {\n",
                "            return 1;\n",
                "        }\n",
                "        Flag.Off => {\n",
                "            return 0;\n",
                "        }\n",
                "    }\n",
                "}\n"
            )
        );
    }

    #[test]
    fn formats_match_expressions() {
        let source = SourceFile::anonymous(
            "fn main()->i32{let flag:bool=true;let value:i32=match(flag){true=>1,false=>0};return value;}",
        );

        let formatted = format_source(&source).expect("source should format");
        assert_eq!(
            formatted,
            concat!(
                "fn main() -> i32 {\n",
                "    let flag: bool = true;\n",
                "    let value: i32 = match (flag) { true => 1, false => 0 };\n",
                "    return value;\n",
                "}\n"
            )
        );
    }

    #[test]
    fn formats_match_binding_patterns() {
        let source = SourceFile::anonymous(
            "fn main()->i32{let value:i32=match(4){0=>1,other=>other};return value;}",
        );

        let formatted = format_source(&source).expect("source should format");
        assert_eq!(
            formatted,
            concat!(
                "fn main() -> i32 {\n",
                "    let value: i32 = match (4) { 0 => 1, other => other };\n",
                "    return value;\n",
                "}\n"
            )
        );
    }

    #[test]
    fn formats_match_or_patterns() {
        let source = SourceFile::anonymous(
            "fn main()->i32{let value:i32=match(1){0|1=>10,_=>0};return value;}",
        );

        let formatted = format_source(&source).expect("source should format");
        assert_eq!(
            formatted,
            concat!(
                "fn main() -> i32 {\n",
                "    let value: i32 = match (1) { 0 | 1 => 10, _ => 0 };\n",
                "    return value;\n",
                "}\n"
            )
        );
    }

    #[test]
    fn formats_match_guards() {
        let source = SourceFile::anonymous(
            "fn main()->i32{let value:i32=match(2){2 if true=>10,_=>0};return value;}",
        );

        let formatted = format_source(&source).expect("source should format");
        assert_eq!(
            formatted,
            concat!(
                "fn main() -> i32 {\n",
                "    let value: i32 = match (2) { 2 if true => 10, _ => 0 };\n",
                "    return value;\n",
                "}\n"
            )
        );
    }

    #[test]
    fn formats_match_range_patterns() {
        let source = SourceFile::anonymous(
            "fn main()->i32{let value:i32=match(404){400..=499=>4,_=>0};return value;}",
        );

        let formatted = format_source(&source).expect("source should format");
        assert_eq!(
            formatted,
            concat!(
                "fn main() -> i32 {\n",
                "    let value: i32 = match (404) { 400..=499 => 4, _ => 0 };\n",
                "    return value;\n",
                "}\n"
            )
        );
    }

    #[test]
    fn formats_payload_enum_variants_and_patterns() {
        let source = SourceFile::anonymous(
            "enum Result{Ok(i32),Err(string),Empty} fn main()->i32{let result:Result=Result.Ok(7);let value:i32=match(result){Result.Ok(found)=>found,Result.Err(_)=>0,Result.Empty=>-1};return value;}",
        );

        let formatted = format_source(&source).expect("source should format");
        assert_eq!(
            formatted,
            concat!(
                "enum Result {\n",
                "    Ok(i32),\n",
                "    Err(string),\n",
                "    Empty,\n",
                "}\n",
                "\n",
                "fn main() -> i32 {\n",
                "    let result: Result = Result.Ok(7);\n",
                "    let value: i32 = match (result) { Result.Ok(found) => found, Result.Err(_) => 0, Result.Empty => -1 };\n",
                "    return value;\n",
                "}\n"
            )
        );
    }

    #[test]
    fn formats_impl_methods() {
        let source = SourceFile::anonymous(
            "struct Point{x:i32,y:i32} impl Point{fn sum(self:Point)->i32{return self.x+self.y;}}",
        );

        let formatted = format_source(&source).expect("source should format");
        assert_eq!(
            formatted,
            concat!(
                "struct Point {\n",
                "    x: i32,\n",
                "    y: i32,\n",
                "}\n",
                "\n",
                "impl Point {\n",
                "    fn sum(self: Point) -> i32 {\n",
                "        return self.x + self.y;\n",
                "    }\n",
                "}\n",
            )
        );
    }

    #[test]
    fn formats_logical_operator_precedence() {
        let source =
            SourceFile::anonymous("fn main()->i32{if(!(true||false)&&true){return 1;}return 0;}");

        let formatted = format_source(&source).expect("source should format");
        assert_eq!(
            formatted,
            concat!(
                "fn main() -> i32 {\n",
                "    if (!(true || false) && true) {\n",
                "        return 1;\n",
                "    }\n",
                "    return 0;\n",
                "}\n"
            )
        );
    }

    #[test]
    fn formats_modulo_operator_precedence() {
        let source = SourceFile::anonymous("fn main()->i32{return 8%3*2+1;}");

        let formatted = format_source(&source).expect("source should format");
        assert_eq!(
            formatted,
            concat!("fn main() -> i32 {\n", "    return 8 % 3 * 2 + 1;\n", "}\n")
        );
    }

    #[test]
    fn formats_for_in_statements() {
        let source = SourceFile::anonymous(
            "fn main()->i32{let values:[i32;3]=[1,2,3];for(let value:i32 in values){println(value);}return 0;}",
        );

        let formatted = format_source(&source).expect("source should format");
        assert_eq!(
            formatted,
            concat!(
                "fn main() -> i32 {\n",
                "    let values: [i32; 3] = [1, 2, 3];\n",
                "    for (let value: i32 in values) {\n",
                "        println(value);\n",
                "    }\n",
                "    return 0;\n",
                "}\n"
            )
        );
    }

    #[test]
    fn formatter_reports_parse_errors() {
        let source = SourceFile::anonymous("fn main() -> i32 { let value: i32 = 1 return value; }");
        assert!(format_source(&source).is_err());
    }
}
