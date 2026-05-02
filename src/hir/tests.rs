use super::{
    EnumVariantPayloadPattern, ExprKind, ItemKind, MatchPatternKind, PlaceKind, StmtKind, Type,
    lower_program,
};
use crate::lexer::tokenize;
use crate::parser::parse;
use crate::semantic::check_program;
use crate::source::SourceFile;
use std::path::PathBuf;

fn lower(source_text: &str) -> super::Program {
    let source = SourceFile::anonymous(source_text);
    lower_source(&source)
}

fn lower_source(source: &SourceFile) -> super::Program {
    let tokens = tokenize(&source);
    let parsed = parse(&source, tokens.tokens);
    let diagnostics = check_program(&source, &parsed.program);
    assert!(
        diagnostics.is_empty(),
        "semantic diagnostics must be empty before lowering: {:?}",
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>()
    );

    lower_program(&source, &parsed.program).expect("HIR lowering should succeed")
}

#[test]
fn lowers_module_qualified_types_and_calls() {
    let source = SourceFile::from_segments(
        "src/main.ax",
        vec![
            (
                PathBuf::from("lib/report.ax"),
                "\
module lib.report;

struct Summary {
count: i32,
}

fn build_summary() -> Summary {
return Summary { count: 7 };
}
"
                .to_string(),
            ),
            (
                PathBuf::from("src/main.ax"),
                "\
import lib.report;

fn main() -> i32 {
let summary: lib.report.Summary = lib.report.build_summary();
return summary.count;
}
"
                .to_string(),
            ),
        ],
    );

    let program = lower_source(&source);

    assert!(matches!(
        program.items[0].kind,
        ItemKind::Struct { ref name, .. } if name == "lib.report.Summary"
    ));
    assert!(matches!(
        program.items[1].kind,
        ItemKind::Function { ref name, .. } if name == "lib.report.build_summary"
    ));

    let ItemKind::Function { body, .. } = &program.items[2].kind else {
        panic!("expected main function");
    };
    let StmtKind::Let {
        ty, initializer, ..
    } = &body.statements[0].kind
    else {
        panic!("expected let statement");
    };
    assert!(matches!(
        ty,
        Type::Struct { name } if name == "lib.report.Summary"
    ));
    assert!(matches!(
        initializer.kind,
        ExprKind::Call { ref function, .. } if function == "lib.report.build_summary"
    ));
}

#[test]
fn lowers_module_qualified_enum_variants() {
    let source = SourceFile::from_segments(
        "src/main.ax",
        vec![
            (
                PathBuf::from("lib/flag.ax"),
                "\
module lib.flag;

enum Flag {
On,
Off,
}
"
                .to_string(),
            ),
            (
                PathBuf::from("src/main.ax"),
                "\
import lib.flag;

fn main() -> i32 {
let flag: lib.flag.Flag = lib.flag.Flag.On;
println(flag);
return 0;
}
"
                .to_string(),
            ),
        ],
    );

    let program = lower_source(&source);

    let ItemKind::Function { body, .. } = &program.items[1].kind else {
        panic!("expected main function");
    };
    let StmtKind::Let { initializer, .. } = &body.statements[0].kind else {
        panic!("expected let statement");
    };
    assert!(matches!(
        initializer.kind,
        ExprKind::EnumVariant {
            ref enum_name,
            ref variant,
            payload: None
        }
            if enum_name == "lib.flag.Flag" && variant == "On"
    ));
}

#[test]
fn lowers_for_loop_into_scoped_block_and_while() {
    let program = lower(
        "\
fn main() -> i32 {
let mut total: i32 = 0;
for (let mut i: i32 = 0; i < 3; i = i + 1) {
    total = total + i;
}
return total;
}
",
    );

    let ItemKind::Function { body, .. } = &program.items[0].kind else {
        panic!("expected function item");
    };

    assert_eq!(body.statements.len(), 3);
    let StmtKind::Block { block } = &body.statements[1].kind else {
        panic!("expected lowered for loop block");
    };
    assert_eq!(block.statements.len(), 2);
    assert!(matches!(block.statements[0].kind, StmtKind::Let { .. }));

    let StmtKind::While {
        condition,
        body: while_body,
    } = &block.statements[1].kind
    else {
        panic!("expected lowered while statement");
    };

    assert!(matches!(condition.kind, ExprKind::Binary { .. }));
    assert_eq!(while_body.statements.len(), 2);
    assert!(matches!(
        while_body.statements[0].kind,
        StmtKind::Block { .. }
    ));
    assert!(matches!(
        while_body.statements[1].kind,
        StmtKind::Assign { .. }
    ));
}

#[test]
fn lowers_for_in_into_indexed_while_with_binding() {
    let program = lower(
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

    let ItemKind::Function { body, .. } = &program.items[0].kind else {
        panic!("expected function item");
    };

    let StmtKind::Block { block } = &body.statements[1].kind else {
        panic!("expected lowered for-in outer block");
    };
    assert_eq!(block.statements.len(), 3);
    assert!(matches!(block.statements[0].kind, StmtKind::Let { .. }));
    assert!(matches!(block.statements[1].kind, StmtKind::Let { .. }));

    let StmtKind::While {
        body: while_body, ..
    } = &block.statements[2].kind
    else {
        panic!("expected lowered while statement");
    };
    assert_eq!(while_body.statements.len(), 2);

    let StmtKind::Block { block: loop_block } = &while_body.statements[0].kind else {
        panic!("expected lowered loop body block");
    };
    let StmtKind::Let {
        name, initializer, ..
    } = &loop_block.statements[0].kind
    else {
        panic!("expected synthesized element binding");
    };
    assert_eq!(name, "value");
    assert!(matches!(initializer.kind, ExprKind::Index { .. }));
    assert!(matches!(
        while_body.statements[1].kind,
        StmtKind::Assign { .. }
    ));
}

#[test]
fn lowers_enum_variants_and_assignment_places() {
    let program = lower(
        "\
enum Flag { On, Off }
struct Point { x: i32 }

fn main() -> i32 {
let flag: Flag = Flag.On;
let mut point: Point = Point { x: 1 };
point.x = 2;
println(flag);
return 0;
}
",
    );

    let ItemKind::Function { body, .. } = &program.items[2].kind else {
        panic!("expected main function");
    };

    let StmtKind::Let { initializer, .. } = &body.statements[0].kind else {
        panic!("expected let statement");
    };
    assert!(matches!(initializer.kind, ExprKind::EnumVariant { .. }));

    let StmtKind::Assign { target, .. } = &body.statements[2].kind else {
        panic!("expected assignment");
    };
    assert!(matches!(target.kind, PlaceKind::Field { .. }));

    let StmtKind::Expr { expr } = &body.statements[3].kind else {
        panic!("expected expression statement");
    };
    assert!(matches!(
        expr.kind,
        ExprKind::Call { ref function, .. } if function == "println"
    ));
}

#[test]
fn lowers_array_types_literals_and_indexing() {
    let program = lower(
        "\
fn main() -> i32 {
let values: [i32; 3] = [1, 2, 3];
return values[1];
}
",
    );

    let ItemKind::Function {
        return_type, body, ..
    } = &program.items[0].kind
    else {
        panic!("expected function");
    };

    assert!(matches!(return_type, Type::I32));

    let StmtKind::Let {
        ty, initializer, ..
    } = &body.statements[0].kind
    else {
        panic!("expected let statement");
    };
    assert!(matches!(
        ty,
        Type::Array { element, length } if **element == Type::I32 && *length == 3
    ));
    assert!(matches!(initializer.kind, ExprKind::ArrayLiteral { .. }));

    let StmtKind::Return { value } = &body.statements[1].kind else {
        panic!("expected return statement");
    };
    assert!(matches!(value.kind, ExprKind::Index { .. }));
}

#[test]
fn lowers_slice_types_and_expressions() {
    let program = lower(
        "\
fn window(values: [i32]) -> i32 {
let head: [i32] = values[0:2];
return head[1];
}

fn main() -> i32 {
let values: [i32; 3] = [1, 2, 3];
return window(values);
}
",
    );

    let ItemKind::Function {
        params,
        return_type,
        body,
        ..
    } = &program.items[0].kind
    else {
        panic!("expected function");
    };

    assert!(matches!(
        params[0].ty,
        Type::Slice { ref element } if **element == Type::I32
    ));
    assert!(matches!(return_type, Type::I32));

    let StmtKind::Let {
        ty, initializer, ..
    } = &body.statements[0].kind
    else {
        panic!("expected let");
    };
    assert!(matches!(
        ty,
        Type::Slice { element } if **element == Type::I32
    ));
    assert!(matches!(initializer.kind, ExprKind::Slice { .. }));
}

#[test]
fn lowers_array_element_assignment_places() {
    let program = lower(
        "\
fn main() -> i32 {
let mut values: [i32; 2] = [1, 2];
values[0] = 3;
return values[0];
}
",
    );

    let ItemKind::Function { body, .. } = &program.items[0].kind else {
        panic!("expected function");
    };

    let StmtKind::Assign { target, .. } = &body.statements[1].kind else {
        panic!("expected assignment statement");
    };
    assert!(matches!(target.kind, PlaceKind::Index { .. }));
}

#[test]
fn lowers_nested_assignment_places() {
    let program = lower(
        "\
struct Point { x: i32 }

fn main() -> i32 {
let mut points: [Point; 2] = [Point { x: 1 }, Point { x: 2 }];
points[0].x = 3;
return points[0].x;
}
",
    );

    let ItemKind::Function { body, .. } = &program.items[1].kind else {
        panic!("expected main function");
    };

    let StmtKind::Assign { target, .. } = &body.statements[1].kind else {
        panic!("expected assignment statement");
    };

    match &target.kind {
        PlaceKind::Field { base, field } => {
            assert_eq!(field, "x");
            assert!(matches!(base.kind, PlaceKind::Index { .. }));
        }
        _ => panic!("expected nested field assignment place"),
    }
}

#[test]
fn lowers_break_statements() {
    let program = lower(
        "\
fn main() -> i32 {
while (true) {
    break;
}
return 0;
}
",
    );

    let ItemKind::Function { body, .. } = &program.items[0].kind else {
        panic!("expected function item");
    };

    let StmtKind::While { body, .. } = &body.statements[0].kind else {
        panic!("expected while statement");
    };

    assert!(matches!(body.statements[0].kind, StmtKind::Break));
}

#[test]
fn lowers_continue_statements() {
    let program = lower(
        "\
fn main() -> i32 {
while (true) {
    continue;
}
return 0;
}
",
    );

    let ItemKind::Function { body, .. } = &program.items[0].kind else {
        panic!("expected function item");
    };

    let StmtKind::While { body, .. } = &body.statements[0].kind else {
        panic!("expected while statement");
    };

    assert!(matches!(body.statements[0].kind, StmtKind::Continue));
}

#[test]
fn lowers_match_statements_into_temp_and_if_chain() {
    let program = lower(
        "\
fn main() -> i32 {
let flag: bool = true;
match (flag) {
    true => {
        println(1);
    }
    _ => {
        println(0);
    }
}
return 0;
}
",
    );

    let ItemKind::Function { body, .. } = &program.items[0].kind else {
        panic!("expected function item");
    };

    let StmtKind::Block { block } = &body.statements[1].kind else {
        panic!("expected lowered match outer block");
    };

    let StmtKind::Let {
        name,
        ty,
        initializer,
        ..
    } = &block.statements[0].kind
    else {
        panic!("expected synthesized match scrutinee binding");
    };
    assert_eq!(name, "__match_scrutinee_0");
    assert!(matches!(ty, Type::Bool));
    assert!(matches!(initializer.kind, ExprKind::Name { ref value } if value == "flag"));

    let StmtKind::If {
        condition,
        then_branch,
        else_branch,
    } = &block.statements[1].kind
    else {
        panic!("expected lowered match if chain");
    };

    assert!(matches!(condition.kind, ExprKind::MatchTest { .. }));
    assert!(matches!(
        then_branch.statements[0].kind,
        StmtKind::Expr { .. }
    ));

    let else_branch = else_branch
        .as_ref()
        .expect("match should lower wildcard arm into else branch");
    assert!(matches!(
        else_branch.statements[0].kind,
        StmtKind::Block { .. }
    ));
}

#[test]
fn lowers_match_expressions() {
    let program = lower(
        "\
fn main() -> i32 {
let flag: bool = true;
let value: i32 = match (flag) { true => 1, false => 0 };
return value;
}
",
    );

    let ItemKind::Function { body, .. } = &program.items[0].kind else {
        panic!("expected function item");
    };

    let StmtKind::Let { initializer, .. } = &body.statements[1].kind else {
        panic!("expected match-expression let");
    };
    let ExprKind::Match { scrutinee, arms } = &initializer.kind else {
        panic!("expected lowered match expression");
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
fn lowers_block_valued_match_expression_arms() {
    let program = lower(
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

    let ItemKind::Function { body, .. } = &program.items[0].kind else {
        panic!("expected function item");
    };

    let StmtKind::Let { initializer, .. } = &body.statements[0].kind else {
        panic!("expected match-expression let");
    };
    let ExprKind::Match { arms, .. } = &initializer.kind else {
        panic!("expected lowered match expression");
    };
    let ExprKind::Block { statements, value } = &arms[0].value.kind else {
        panic!("expected lowered block expression");
    };
    assert!(matches!(statements[0].kind, StmtKind::Let { .. }));
    assert!(matches!(value.kind, ExprKind::Binary { .. }));
}

#[test]
fn lowers_match_expression_binding_patterns() {
    let program = lower(
        "\
fn main() -> i32 {
let value: i32 = match (4) { 0 => 1, other => other };
return value;
}
",
    );

    let ItemKind::Function { body, .. } = &program.items[0].kind else {
        panic!("expected function item");
    };

    let StmtKind::Let { initializer, .. } = &body.statements[0].kind else {
        panic!("expected match-expression let");
    };
    let ExprKind::Match { arms, .. } = &initializer.kind else {
        panic!("expected lowered match expression");
    };
    assert!(matches!(
        arms[1].pattern.kind,
        MatchPatternKind::Binding { ref name } if name == "other"
    ));
    assert!(matches!(arms[1].value.kind, ExprKind::Name { ref value } if value == "other"));
}

#[test]
fn lowers_match_struct_patterns() {
    let program = lower(
        "\
struct Point { x: i32, y: i32 }

fn main() -> i32 {
let point: Point = Point { x: 1, y: 2 };
let value: i32 = match (point) { Point { x, y } => x + y };
return value;
}
",
    );

    let ItemKind::Function { body, .. } = &program.items[1].kind else {
        panic!("expected function item");
    };
    let StmtKind::Let { initializer, .. } = &body.statements[1].kind else {
        panic!("expected match-expression let");
    };
    let ExprKind::Match { arms, .. } = &initializer.kind else {
        panic!("expected lowered match expression");
    };
    assert!(matches!(
        arms[0].pattern.kind,
        MatchPatternKind::Struct { ref struct_name, ref fields }
            if struct_name == "Point"
                && fields.len() == 2
                && fields[0].binding == "x"
                && fields[0].ty == Type::I32
                && fields[1].binding == "y"
                && fields[1].ty == Type::I32
    ));
}

#[test]
fn lowers_payload_enum_constructors_and_patterns() {
    let program = lower(
        "\
enum Result { Ok(i32), Err(string), Empty }

fn score(result: Result) -> i32 {
return match (result) {
    Result.Ok(value) => value,
    Result.Err(_) => 0,
    Result.Empty => -1,
};
}

fn main() -> i32 {
let ok: Result = Result.Ok(7);
return score(ok);
}
",
    );

    let ItemKind::Function { body, .. } = &program.items[1].kind else {
        panic!("expected score function");
    };
    let StmtKind::Return { value: expr } = &body.statements[0].kind else {
        panic!("expected return statement");
    };
    let ExprKind::Match { arms, .. } = &expr.kind else {
        panic!("expected lowered match expression");
    };
    assert!(matches!(
        arms[0].pattern.kind,
        MatchPatternKind::EnumVariant {
            ref enum_name,
            ref variant,
            payload: Some(EnumVariantPayloadPattern::Binding { ref name }),
            payload_type: Some(Type::I32),
        } if enum_name == "Result" && variant == "Ok" && name == "value"
    ));
    assert!(matches!(
        arms[1].pattern.kind,
        MatchPatternKind::EnumVariant {
            ref enum_name,
            ref variant,
            payload: Some(EnumVariantPayloadPattern::Wildcard),
            payload_type: Some(Type::String),
        } if enum_name == "Result" && variant == "Err"
    ));
    assert!(matches!(
        arms[2].pattern.kind,
        MatchPatternKind::EnumVariant {
            ref enum_name,
            ref variant,
            payload: None,
            payload_type: None,
        } if enum_name == "Result" && variant == "Empty"
    ));

    let ItemKind::Function { body, .. } = &program.items[2].kind else {
        panic!("expected main function");
    };
    let StmtKind::Let { initializer, .. } = &body.statements[0].kind else {
        panic!("expected let statement");
    };
    assert!(matches!(
        initializer.kind,
        ExprKind::EnumVariant {
            ref enum_name,
            ref variant,
            payload: Some(_),
        } if enum_name == "Result" && variant == "Ok"
    ));
}

#[test]
fn keeps_invalid_multi_argument_enum_constructor_calls_as_calls_in_hir() {
    let source = SourceFile::anonymous(
        "\
enum Result { Ok(i32) }

fn main() -> i32 {
Result.Ok(1, 2);
return 0;
}
",
    );
    let tokens = tokenize(&source);
    let parsed = parse(&source, tokens.tokens);
    let program =
        lower_program(&source, &parsed.program).expect("HIR lowering should stay lossless");

    let ItemKind::Function { body, .. } = &program.items[1].kind else {
        panic!("expected main function");
    };
    let StmtKind::Expr { expr } = &body.statements[0].kind else {
        panic!("expected expression statement");
    };
    assert!(matches!(
        expr.kind,
        ExprKind::Call {
            ref function,
            ref arguments,
        } if function == "Result.Ok" && arguments.len() == 2
    ));
}

#[test]
fn rewrites_for_continue_to_run_step_before_loop_continue() {
    let program = lower(
        "\
fn main() -> i32 {
let mut total: i32 = 0;
for (let mut i: i32 = 0; i < 4; i = i + 1) {
    if (i == 1) {
        continue;
    }
    total = total + i;
}
return total;
}
",
    );

    let ItemKind::Function { body, .. } = &program.items[0].kind else {
        panic!("expected function item");
    };

    let StmtKind::Block { block } = &body.statements[1].kind else {
        panic!("expected lowered for loop outer block");
    };

    let StmtKind::While { body, .. } = &block.statements[1].kind else {
        panic!("expected lowered while statement");
    };

    let StmtKind::Block {
        block: lowered_body,
    } = &body.statements[0].kind
    else {
        panic!("expected original for body wrapper block");
    };

    let StmtKind::If {
        then_branch: continue_branch,
        ..
    } = &lowered_body.statements[0].kind
    else {
        panic!("expected if statement guarding continue");
    };

    assert!(
        matches!(continue_branch.statements[0].kind, StmtKind::Assign { .. }),
        "for-loop continue branch should run the step before continuing"
    );
    assert!(
        matches!(continue_branch.statements[1].kind, StmtKind::Continue),
        "for-loop continue branch should still end with continue"
    );
    assert!(
        matches!(body.statements[1].kind, StmtKind::Assign { .. }),
        "lowered for loop should keep the normal step at the end of the while body"
    );
}
