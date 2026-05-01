use super::diagnostics::enrich_parse_error;
use super::parse;
use crate::ast::{
    EnumVariantPayloadPattern, ExprKind, ItemKind, MatchPatternKind, StmtKind, Visibility,
};
use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::lexer::tokenize;
use crate::source::{SourceFile, Span};
use crate::token::{Token, TokenKind};
use std::path::PathBuf;

#[test]
fn parses_minimal_main() {
    let source = SourceFile::anonymous("fn main() -> i32 { return 0; }");
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(output.diagnostics.is_empty());
    assert_eq!(output.program.items.len(), 1);
    assert_eq!(output.program.source_units.len(), 1);
    match &output.program.items[0].kind {
        ItemKind::Function { name, body, .. } => {
            assert_eq!(name, "main");
            assert_eq!(body.statements.len(), 1);
        }
        _ => panic!("expected function"),
    }
}

#[test]
fn parses_top_level_const_items() {
    let source =
        SourceFile::anonymous("const EXIT_OK: i32 = 7; fn main() -> i32 { return EXIT_OK; }");
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

    let ItemKind::Const { name, ty, value } = &output.program.items[0].kind else {
        panic!("expected const item");
    };
    assert_eq!(name, "EXIT_OK");
    assert_eq!(ty.describe(), "i32");
    assert!(matches!(value.kind, ExprKind::Int { value: 7 }));
}

#[test]
fn parses_type_alias_items() {
    let source = SourceFile::anonymous(
        "type UserId = i32; fn main() -> i32 { let id: UserId = 7; return id; }",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

    let ItemKind::TypeAlias {
        name,
        type_params,
        target,
    } = &output.program.items[0].kind
    else {
        panic!("expected type alias item");
    };
    assert_eq!(name, "UserId");
    assert!(type_params.is_empty());
    assert_eq!(target.describe(), "i32");
}

#[test]
fn parses_public_top_level_items() {
    let source = SourceFile::anonymous("pub fn helper() -> i32 { return 1; }");
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(output.program.items[0].visibility, Visibility::Public);
    assert!(matches!(
        output.program.items[0].kind,
        ItemKind::Function { .. }
    ));
}

#[test]
fn parses_multiple_trait_bounds_on_generic_function() {
    let source = SourceFile::anonymous(
        "fn render<T: Label + Code>(value: T) -> string { return value.label(); }",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

    let ItemKind::Function {
        type_param_bounds, ..
    } = &output.program.items[0].kind
    else {
        panic!("expected function");
    };
    assert_eq!(type_param_bounds.len(), 2);
    assert_eq!(type_param_bounds[0].type_param, "T");
    assert_eq!(type_param_bounds[0].trait_ref.describe(), "Label");
    assert_eq!(type_param_bounds[1].type_param, "T");
    assert_eq!(type_param_bounds[1].trait_ref.describe(), "Code");
}

#[test]
fn parses_where_trait_bounds_on_generic_function() {
    let source = SourceFile::anonymous(
        "fn render<T>(value: T) -> string where T: Label + Code { return value.label(); }",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

    let ItemKind::Function {
        type_param_bounds, ..
    } = &output.program.items[0].kind
    else {
        panic!("expected function");
    };
    assert_eq!(type_param_bounds.len(), 2);
    assert_eq!(type_param_bounds[0].type_param, "T");
    assert_eq!(type_param_bounds[0].trait_ref.describe(), "Label");
    assert_eq!(type_param_bounds[1].type_param, "T");
    assert_eq!(type_param_bounds[1].trait_ref.describe(), "Code");
}

#[test]
fn parses_generic_impl_blocks() {
    let source = SourceFile::anonymous(
        "struct Box<T> { value: T } impl<T> Box<T> { fn get(self: Box<T>) -> T { return self.value; } }",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

    let ItemKind::Impl {
        type_params,
        target,
        methods,
        ..
    } = &output.program.items[1].kind
    else {
        panic!("expected impl");
    };
    assert_eq!(type_params, &vec!["T".to_string()]);
    assert_eq!(target.describe(), "Box<T>");
    assert_eq!(methods[0].return_type.describe(), "T");
}

#[test]
fn parses_generic_impl_methods() {
    let source = SourceFile::anonymous(
        "struct Pair<T, U> { left: T, right: U } impl<T> Pair<T, i32> { fn replace_right<U>(self: Pair<T, i32>, right: U) -> Pair<T, U> { return Pair { left: self.left, right: right }; } }",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

    let ItemKind::Impl { methods, .. } = &output.program.items[1].kind else {
        panic!("expected impl");
    };
    assert_eq!(methods[0].type_params, vec!["U".to_string()]);
    assert_eq!(methods[0].return_type.describe(), "Pair<T, U>");
}

#[test]
fn parses_module_headers_per_source_segment() {
    let source = SourceFile::from_segments(
        "src/main.ax",
        vec![
            (
                PathBuf::from("foundation/search.ax"),
                "module foundation.search;\nfn helper() -> i32 { return 1; }\n".to_string(),
            ),
            (
                PathBuf::from("src/main.ax"),
                "import foundation.search;\nfn main() -> i32 { return foundation.search.helper(); }\n"
                    .to_string(),
            ),
        ],
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
    assert_eq!(output.program.source_units.len(), 2);

    let support = &output.program.source_units[0];
    assert_eq!(support.path, "foundation/search.ax");
    assert_eq!(
        support.module.as_ref().map(|module| module.path.as_str()),
        Some("foundation.search")
    );
    assert!(support.imports.is_empty());
    assert!(!support.is_entry);

    let entry = &output.program.source_units[1];
    assert_eq!(entry.path, "src/main.ax");
    assert_eq!(entry.imports.len(), 1);
    assert_eq!(entry.imports[0].path, "foundation.search");
    assert!(entry.is_entry);
}

#[test]
fn parses_qualified_type_path() {
    let source = SourceFile::anonymous(
        "fn main() -> i32 { let value: foundation.search.SearchStats = foundation.search.SearchStats { match_count: 0 }; return 0; }",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

    let ItemKind::Function { body, .. } = &output.program.items[0].kind else {
        panic!("expected function");
    };
    let StmtKind::Let {
        ty, initializer, ..
    } = &body.statements[0].kind
    else {
        panic!("expected let statement");
    };

    assert_eq!(ty.describe(), "foundation.search.SearchStats");
    assert!(matches!(initializer.kind, ExprKind::StructLiteral { .. }));
}

#[test]
fn respects_operator_precedence() {
    let source = SourceFile::anonymous("fn main() -> i32 { return 1 + 2 * 3; }");
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    let function = &output.program.items[0];
    match &function.kind {
        ItemKind::Function { body, .. } => match &body.statements[0].kind {
            StmtKind::Return { value: Some(expr) } => match &expr.kind {
                ExprKind::Binary { op, right, .. } => {
                    assert!(matches!(op, crate::ast::BinaryOp::Add));
                    assert!(matches!(right.kind, ExprKind::Binary { .. }));
                }
                _ => panic!("expected binary expr"),
            },
            _ => panic!("expected return"),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn respects_logical_operator_precedence() {
    let source = SourceFile::anonymous("fn main() -> i32 { return true || false && false; }");
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    let function = &output.program.items[0];
    match &function.kind {
        ItemKind::Function { body, .. } => match &body.statements[0].kind {
            StmtKind::Return { value: Some(expr) } => match &expr.kind {
                ExprKind::Binary { op, right, .. } => {
                    assert!(matches!(op, crate::ast::BinaryOp::LogicalOr));
                    assert!(matches!(
                        right.kind,
                        ExprKind::Binary {
                            op: crate::ast::BinaryOp::LogicalAnd,
                            ..
                        }
                    ));
                }
                _ => panic!("expected logical binary expr"),
            },
            _ => panic!("expected return"),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn respects_modulo_precedence() {
    let source = SourceFile::anonymous("fn main() -> i32 { return 8 % 3 * 2; }");
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    let function = &output.program.items[0];
    match &function.kind {
        ItemKind::Function { body, .. } => match &body.statements[0].kind {
            StmtKind::Return { value: Some(expr) } => match &expr.kind {
                ExprKind::Binary { op, left, .. } => {
                    assert!(matches!(op, crate::ast::BinaryOp::Multiply));
                    assert!(matches!(
                        left.kind,
                        ExprKind::Binary {
                            op: crate::ast::BinaryOp::Remainder,
                            ..
                        }
                    ));
                }
                _ => panic!("expected multiplicative binary expr"),
            },
            _ => panic!("expected return"),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn parses_struct_literal_and_field_access() {
    let source = SourceFile::anonymous(
        "fn main() -> i32 { let point: Point = Point { x: 1, y: 2 }; return point.x; }",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(output.diagnostics.is_empty());
    match &output.program.items[0].kind {
        ItemKind::Function { body, .. } => match &body.statements[0].kind {
            StmtKind::Let { initializer, .. } => {
                assert!(matches!(initializer.kind, ExprKind::StructLiteral { .. }));
            }
            _ => panic!("expected let statement"),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn parses_for_statement() {
    let source = SourceFile::anonymous(
        "\
fn main() -> i32 {
for (let mut i: i32 = 0; i < 3; i = i + 1) {
    println(i);
}
return 0;
}
",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(
        output.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        output.diagnostics
    );
    match &output.program.items[0].kind {
        ItemKind::Function { body, .. } => match &body.statements[0].kind {
            StmtKind::For {
                initializer,
                condition,
                step,
                ..
            } => {
                assert!(initializer.is_some());
                assert!(condition.is_some());
                assert!(step.is_some());
            }
            _ => panic!("expected for statement"),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn parses_for_in_statement() {
    let source = SourceFile::anonymous(
        "\
fn main() -> i32 {
let values: [i32; 3] = [1, 2, 3];
for (let value: i32 in values) {
    println(value);
}
return 0;
}
",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(
        output.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        output.diagnostics
    );
    match &output.program.items[0].kind {
        ItemKind::Function { body, .. } => match &body.statements[1].kind {
            StmtKind::ForIn {
                binding, iterable, ..
            } => {
                assert_eq!(binding.name, "value");
                assert_eq!(binding.ty.describe(), "i32");
                assert!(matches!(iterable.kind, ExprKind::Name { ref value } if value == "values"));
            }
            _ => panic!("expected for-in statement"),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn parses_break_statement() {
    let source = SourceFile::anonymous(
        "\
fn main() -> i32 {
while (true) {
    break;
}
return 0;
}
",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(
        output.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        output.diagnostics
    );
    match &output.program.items[0].kind {
        ItemKind::Function { body, .. } => match &body.statements[0].kind {
            StmtKind::While { body, .. } => {
                assert!(matches!(body.statements[0].kind, StmtKind::Break));
            }
            _ => panic!("expected while statement"),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn parses_continue_statement() {
    let source = SourceFile::anonymous(
        "\
fn main() -> i32 {
while (true) {
    continue;
}
return 0;
}
",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(
        output.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        output.diagnostics
    );
    match &output.program.items[0].kind {
        ItemKind::Function { body, .. } => match &body.statements[0].kind {
            StmtKind::While { body, .. } => {
                assert!(matches!(body.statements[0].kind, StmtKind::Continue));
            }
            _ => panic!("expected while statement"),
        },
        _ => panic!("expected function"),
    }
}

#[test]
fn parses_match_statement() {
    let source = SourceFile::anonymous(
        "\
enum Flag { On, Off }
fn main() -> i32 {
match (Flag.On) {
    Flag.On => {
        return 1;
    }
    Flag.Off => {
        return 0;
    }
}
}
",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(
        output.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        output.diagnostics
    );

    let ItemKind::Function { body, .. } = &output.program.items[1].kind else {
        panic!("expected main function");
    };
    let StmtKind::Match { scrutinee, arms } = &body.statements[0].kind else {
        panic!("expected match statement");
    };
    assert!(matches!(scrutinee.kind, ExprKind::Field { .. }));
    assert_eq!(arms.len(), 2);
    assert!(matches!(
        arms[0].pattern.kind,
        MatchPatternKind::EnumVariant { ref path, .. } if path == "Flag.On"
    ));
    assert!(matches!(
        arms[1].pattern.kind,
        MatchPatternKind::EnumVariant { ref path, .. } if path == "Flag.Off"
    ));
}

#[test]
fn parses_match_expression() {
    let source = SourceFile::anonymous(
        "\
fn main() -> i32 {
let flag: bool = true;
let value: i32 = match (flag) { true => 1, false => 0 };
return value;
}
",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

    let ItemKind::Function { body, .. } = &output.program.items[0].kind else {
        panic!("expected function");
    };
    let StmtKind::Let { initializer, .. } = &body.statements[1].kind else {
        panic!("expected let statement");
    };
    let ExprKind::Match { scrutinee, arms } = &initializer.kind else {
        panic!("expected match expression");
    };
    assert!(matches!(scrutinee.kind, ExprKind::Name { ref value } if value == "flag"));
    assert_eq!(arms.len(), 2);
    assert!(matches!(
        arms[0].pattern.kind,
        MatchPatternKind::Bool { value: true }
    ));
    assert!(matches!(arms[0].value.kind, ExprKind::Int { value: 1 }));
}

#[test]
fn parses_block_valued_match_expression_arms() {
    let source = SourceFile::anonymous(
        "\
fn main() -> i32 {
let value: i32 = match (true) {
    true => { let base: i32 = 40; base + 2 },
    false => 0,
};
return value;
}
",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

    let ItemKind::Function { body, .. } = &output.program.items[0].kind else {
        panic!("expected function");
    };
    let StmtKind::Let { initializer, .. } = &body.statements[0].kind else {
        panic!("expected let statement");
    };
    let ExprKind::Match { arms, .. } = &initializer.kind else {
        panic!("expected match expression");
    };
    let ExprKind::Block { statements, value } = &arms[0].value.kind else {
        panic!("expected block-valued match arm");
    };
    assert_eq!(statements.len(), 1);
    assert!(matches!(statements[0].kind, StmtKind::Let { .. }));
    assert!(matches!(value.kind, ExprKind::Binary { .. }));
}

#[test]
fn parses_match_binding_pattern() {
    let source = SourceFile::anonymous(
        "\
fn main() -> i32 {
let value: i32 = match (4) { 0 => 1, other => other };
return value;
}
",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

    let ItemKind::Function { body, .. } = &output.program.items[0].kind else {
        panic!("expected function");
    };
    let StmtKind::Let { initializer, .. } = &body.statements[0].kind else {
        panic!("expected let statement");
    };
    let ExprKind::Match { arms, .. } = &initializer.kind else {
        panic!("expected match expression");
    };
    assert!(matches!(
        arms[1].pattern.kind,
        MatchPatternKind::Binding { ref name } if name == "other"
    ));
}

#[test]
fn parses_match_or_patterns() {
    let source = SourceFile::anonymous(
        "\
fn main() -> i32 {
let value: i32 = match (1) { 0 | 1 => 10, _ => 0 };
return value;
}
",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

    let ItemKind::Function { body, .. } = &output.program.items[0].kind else {
        panic!("expected function");
    };
    let StmtKind::Let { initializer, .. } = &body.statements[0].kind else {
        panic!("expected let statement");
    };
    let ExprKind::Match { arms, .. } = &initializer.kind else {
        panic!("expected match expression");
    };
    assert!(matches!(
        arms[0].pattern.kind,
        MatchPatternKind::Or { ref alternatives } if alternatives.len() == 2
    ));
}

#[test]
fn parses_match_guards() {
    let source = SourceFile::anonymous(
        "\
fn main() -> i32 {
let value: i32 = match (2) { 2 if true => 10, _ => 0 };
return value;
}
",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

    let ItemKind::Function { body, .. } = &output.program.items[0].kind else {
        panic!("expected function");
    };
    let StmtKind::Let { initializer, .. } = &body.statements[0].kind else {
        panic!("expected let statement");
    };
    let ExprKind::Match { arms, .. } = &initializer.kind else {
        panic!("expected match expression");
    };
    assert!(arms[0].guard.is_some());
}

#[test]
fn parses_match_range_patterns() {
    let source = SourceFile::anonymous(
        "\
fn main() -> i32 {
let value: i32 = match (404) { 400..=499 => 4, _ => 0 };
return value;
}
",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

    let ItemKind::Function { body, .. } = &output.program.items[0].kind else {
        panic!("expected function");
    };
    let StmtKind::Let { initializer, .. } = &body.statements[0].kind else {
        panic!("expected let statement");
    };
    let ExprKind::Match { arms, .. } = &initializer.kind else {
        panic!("expected match expression");
    };
    assert!(matches!(
        arms[0].pattern.kind,
        MatchPatternKind::IntRange {
            start: 400,
            end: 499
        }
    ));
}

#[test]
fn parses_match_struct_patterns() {
    let source = SourceFile::anonymous(
        "\
struct Point { x: i32, y: i32 }

fn main() -> i32 {
let point: Point = Point { x: 1, y: 2 };
let value: i32 = match (point) { Point { x, y } => x + y };
return value;
}
",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

    let ItemKind::Function { body, .. } = &output.program.items[1].kind else {
        panic!("expected function");
    };
    let StmtKind::Let { initializer, .. } = &body.statements[1].kind else {
        panic!("expected let statement");
    };
    let ExprKind::Match { arms, .. } = &initializer.kind else {
        panic!("expected match expression");
    };
    assert!(matches!(
        arms[0].pattern.kind,
        MatchPatternKind::Struct { ref path, ref fields }
            if path == "Point"
                && fields.len() == 2
                && fields[0].name == "x"
                && fields[1].name == "y"
    ));
}

#[test]
fn reports_struct_pattern_aliases_as_non_canonical() {
    let source = SourceFile::anonymous(
        "\
struct Point { x: i32, y: i32 }

fn main() -> i32 {
let point: Point = Point { x: 1, y: 2 };
let value: i32 = match (point) { Point { x: left, y } => left + y };
return value;
}
",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(
        output.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "P0003"
                && diagnostic.kind() == Some(DiagnosticKind::MatchStructPatternShapeMismatch)
        }),
        "expected canonical struct-pattern diagnostic, got {:?}",
        output.diagnostics
    );
}

#[test]
fn parses_payload_enum_variants_and_patterns() {
    let source = SourceFile::anonymous(
        "\
enum Result {
Ok(i32),
Err(string),
Empty,
}

fn main() -> i32 {
let result: Result = Result.Ok(7);
let value: i32 = match (result) { Result.Ok(found) => found, Result.Err(_) => 0, Result.Empty => -1 };
return value;
}
",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

    let ItemKind::Enum { variants, .. } = &output.program.items[0].kind else {
        panic!("expected enum item");
    };
    assert_eq!(variants.len(), 3);
    assert_eq!(
        variants[0]
            .payload
            .as_ref()
            .expect("payload variant should record its payload type")
            .describe(),
        "i32"
    );
    assert_eq!(
        variants[1]
            .payload
            .as_ref()
            .expect("payload variant should record its payload type")
            .describe(),
        "string"
    );
    assert!(variants[2].payload.is_none());

    let ItemKind::Function { body, .. } = &output.program.items[1].kind else {
        panic!("expected function");
    };
    let StmtKind::Let { initializer, .. } = &body.statements[1].kind else {
        panic!("expected let statement");
    };
    let ExprKind::Match { arms, .. } = &initializer.kind else {
        panic!("expected match expression");
    };
    assert!(matches!(
        arms[0].pattern.kind,
        MatchPatternKind::EnumVariant {
            ref path,
            payload: Some(EnumVariantPayloadPattern::Binding { ref name }),
        } if path == "Result.Ok" && name == "found"
    ));
    assert!(matches!(
        arms[1].pattern.kind,
        MatchPatternKind::EnumVariant {
            ref path,
            payload: Some(EnumVariantPayloadPattern::Wildcard),
        } if path == "Result.Err"
    ));
    assert!(matches!(
        arms[2].pattern.kind,
        MatchPatternKind::EnumVariant {
            ref path,
            payload: None,
        } if path == "Result.Empty"
    ));
}

#[test]
fn parses_array_types_literals_and_indexing() {
    let source = SourceFile::anonymous(
        "fn main() -> i32 { let values: [i32; 3] = [1, 2, 3]; return values[1]; }",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

    let ItemKind::Function { body, .. } = &output.program.items[0].kind else {
        panic!("expected function");
    };

    let StmtKind::Let {
        ty, initializer, ..
    } = &body.statements[0].kind
    else {
        panic!("expected let statement");
    };
    assert_eq!(ty.describe(), "[i32; 3]");
    assert!(matches!(initializer.kind, ExprKind::ArrayLiteral { .. }));

    let StmtKind::Return { value: Some(expr) } = &body.statements[1].kind else {
        panic!("expected return statement");
    };
    assert!(matches!(expr.kind, ExprKind::Index { .. }));
}

#[test]
fn parses_slice_types_and_expressions() {
    let source = SourceFile::anonymous(
        "fn read(values: [i32]) -> i32 { let head: [i32] = values[0:2]; return head[1]; }",
    );
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);

    let ItemKind::Function {
        params,
        return_type,
        body,
        ..
    } = &output.program.items[0].kind
    else {
        panic!("expected function");
    };

    assert_eq!(params[0].ty.describe(), "[i32]");
    assert_eq!(return_type.describe(), "i32");

    let StmtKind::Let {
        ty, initializer, ..
    } = &body.statements[0].kind
    else {
        panic!("expected let statement");
    };
    assert_eq!(ty.describe(), "[i32]");
    assert!(matches!(initializer.kind, ExprKind::Slice { .. }));

    let StmtKind::Return { value: Some(expr) } = &body.statements[1].kind else {
        panic!("expected return");
    };
    assert!(matches!(expr.kind, ExprKind::Index { .. }));
}

#[test]
fn enriches_missing_semicolon_diagnostic() {
    let source = SourceFile::anonymous("fn main() -> i32 { let value: i32 = 1 return value; }");
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic
                .message
                .contains("expected `;` after variable declaration")
        })
        .expect("missing semicolon diagnostic should exist");

    assert!(
        diagnostic
            .notes
            .iter()
            .any(|note| note.contains("explicit semicolons"))
    );
    assert_eq!(diagnostic.kind(), Some(DiagnosticKind::MissingSemicolon));
    assert_eq!(
        diagnostic.suggestion.as_deref(),
        Some("insert `;` before the next statement or closing `}`")
    );
}

#[test]
fn enriches_missing_right_paren_diagnostic() {
    let source = SourceFile::anonymous("fn main() -> i32 { if (true { return 1; } return 0; }");
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic
                .message
                .contains("expected `)` after if condition")
        })
        .expect("missing right paren diagnostic should exist");

    assert!(
        diagnostic
            .notes
            .iter()
            .any(|note| note.contains("left open"))
    );
    assert_eq!(diagnostic.kind(), Some(DiagnosticKind::MissingRightParen));
    assert_eq!(
        diagnostic.suggestion.as_deref(),
        Some("insert `)` to close the current parenthesized construct")
    );
}

#[test]
fn enriches_missing_right_brace_diagnostic_with_stable_kind() {
    let source = SourceFile::anonymous("fn main() -> i32 { if (true) { return 1; }");
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic
                .message
                .contains("expected `}` to close the block")
        })
        .expect("missing right brace diagnostic should exist");

    assert_eq!(diagnostic.kind(), Some(DiagnosticKind::MissingRightBrace));
    assert_eq!(
        diagnostic.suggestion.as_deref(),
        Some("insert `}` to close the current block or literal")
    );
}

#[test]
fn enriches_missing_right_bracket_diagnostic() {
    let source =
        SourceFile::anonymous("fn main() -> i32 { let values: [i32; 2 = [1, 2]; return 0; }");
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.message.contains("expected `]` after array type"))
        .expect("missing right bracket diagnostic should exist");

    assert!(
        diagnostic
            .notes
            .iter()
            .any(|note| note.contains("slice types"))
    );
    assert_eq!(diagnostic.kind(), Some(DiagnosticKind::MissingRightBracket));
    assert_eq!(
        diagnostic.suggestion.as_deref(),
        Some("insert `]` to close the current bracketed construct")
    );
}

#[test]
fn classifies_top_level_declaration_error_with_stable_kind() {
    let source = SourceFile::anonymous("let value: i32 = 1;");
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.message == "expected a top-level declaration")
        .expect("top-level declaration diagnostic should exist");

    assert_eq!(
        diagnostic.kind(),
        Some(DiagnosticKind::TopLevelDeclarationRequired)
    );
    assert_eq!(
        diagnostic.suggestion.as_deref(),
        Some(
            "start a top-level item with `pub`, `fn`, `const`, `struct`, `enum`, `trait`, or `impl`"
        )
    );
}

#[test]
fn stable_kind_keeps_parse_help_even_if_message_text_changes() {
    let source = SourceFile::anonymous("fn main() -> i32 { return 0 }");
    let token = Token {
        kind: TokenKind::RBrace,
        lexeme: "}".to_string(),
        span: Span::new(27, 28),
    };
    let diagnostic = Diagnostic::new("P0001", "placeholder parser wording", &source, token.span)
        .with_kind(DiagnosticKind::MissingSemicolon);

    let enriched = enrich_parse_error(diagnostic, &token, "placeholder parser wording");

    assert!(
        enriched
            .notes
            .iter()
            .any(|note| note.contains("explicit semicolons"))
    );
    assert_eq!(
        enriched.suggestion.as_deref(),
        Some("insert `;` before the next statement or closing `}`")
    );
}

#[test]
fn classifies_type_name_error_with_stable_kind() {
    let source = SourceFile::anonymous("fn main() -> i32 { let value: = 1; return 0; }");
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.message == "expected a type name")
        .expect("type name diagnostic should exist");

    assert_eq!(diagnostic.kind(), Some(DiagnosticKind::TypeNameRequired));
    assert_eq!(
        diagnostic.suggestion.as_deref(),
        Some(
            "use `bool`, `i32`, `f32`, `string`, `[Type]`, `[Type; N]`, or a previously declared type name"
        )
    );
}

#[test]
fn classifies_expression_error_with_stable_kind() {
    let source = SourceFile::anonymous("fn main() -> i32 { let value: i32 = ; return 0; }");
    let tokens = tokenize(&source).tokens;
    let output = parse(&source, tokens);
    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.message == "expected an expression")
        .expect("expression diagnostic should exist");

    assert_eq!(diagnostic.kind(), Some(DiagnosticKind::ExpressionRequired));
    assert_eq!(
        diagnostic.suggestion.as_deref(),
        Some(
            "insert a runtime expression such as a literal, array literal, name, call, or parenthesized expression"
        )
    );
}
