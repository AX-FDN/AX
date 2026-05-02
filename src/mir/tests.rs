use super::{
    ExprKind, ItemKind, LocalKind, MatchPatternKind, PlaceKind, StatementKind, TerminatorKind,
    Type, lower_program,
};
use crate::hir::lower_program as lower_hir_program;
use crate::lexer::tokenize;
use crate::parser::parse;
use crate::semantic::check_program;
use crate::source::SourceFile;

fn lower(source_text: &str) -> super::Program {
    let source = SourceFile::anonymous(source_text);
    let tokens = tokenize(&source);
    let parsed = parse(&source, tokens.tokens);
    let diagnostics = check_program(&source, &parsed.program);
    assert!(
        diagnostics.is_empty(),
        "semantic diagnostics must be empty before MIR lowering: {:?}",
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>()
    );

    let hir = lower_hir_program(&source, &parsed.program).expect("HIR lowering should succeed");
    lower_program(&hir).expect("MIR lowering should succeed")
}

#[test]
fn lowers_for_loop_into_basic_block_cfg() {
    let program = lower(
        "\
fn main() -> i32 {
let mut total: i32 = 0;
for (let mut i: i32 = 0; i < 3; i = i + 1) {
    total = total + i;
}
println(total);
return total;
}
",
    );

    let ItemKind::Function {
        entry_block,
        locals,
        blocks,
        ..
    } = &program.items[0].kind
    else {
        panic!("expected function item");
    };

    assert_eq!(*entry_block, 0);
    assert_eq!(locals.len(), 2);
    assert!(
        locals
            .iter()
            .any(|local| local.name == "total" && local.kind == LocalKind::Local)
    );
    assert!(
        locals
            .iter()
            .any(|local| local.name == "i" && local.kind == LocalKind::Local)
    );

    assert!(matches!(
        blocks[0].statements[0].kind,
        StatementKind::Let { .. }
    ));
    assert!(matches!(
        blocks[0].statements[1].kind,
        StatementKind::Let { .. }
    ));
    assert!(matches!(
        blocks[0].terminator.kind,
        TerminatorKind::Goto { target: 1 }
    ));
    assert!(matches!(
        blocks[1].terminator.kind,
        TerminatorKind::Branch {
            then_block: 2,
            else_block: 3,
            ..
        }
    ));
    assert!(matches!(
        blocks[2].terminator.kind,
        TerminatorKind::Goto { target: 1 }
    ));
    assert!(
        blocks
            .iter()
            .any(|block| matches!(block.terminator.kind, TerminatorKind::Return { .. }))
    );
}

#[test]
fn resolves_shadowed_bindings_to_distinct_locals() {
    let program = lower(
        "\
fn main() -> i32 {
let value: i32 = 1;
{
    let value: i32 = 2;
    println(value);
}
println(value);
return value;
}
",
    );

    let ItemKind::Function { blocks, .. } = &program.items[0].kind else {
        panic!("expected function item");
    };

    let mut printed_locals = Vec::new();
    for block in blocks {
        for statement in &block.statements {
            let StatementKind::Eval { expr } = &statement.kind else {
                continue;
            };
            let ExprKind::Call {
                function,
                arguments,
            } = &expr.kind
            else {
                continue;
            };
            if function != "println" {
                continue;
            }
            let ExprKind::Local { local, .. } = arguments[0].kind else {
                panic!("println should lower to a local argument");
            };
            printed_locals.push(local);
        }
    }

    assert_eq!(printed_locals.len(), 2);
    assert_ne!(printed_locals[0], printed_locals[1]);

    let returned_local = blocks
        .iter()
        .find_map(|block| match &block.terminator.kind {
            TerminatorKind::Return { value } => match value.kind {
                ExprKind::Local { local, .. } => Some(local),
                _ => None,
            },
            _ => None,
        })
        .expect("return terminator should exist");

    assert_eq!(printed_locals[1], returned_local);
}

#[test]
fn lowers_match_struct_pattern_bindings() {
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

    let ItemKind::Function { locals, blocks, .. } = &program.items[1].kind else {
        panic!("expected function item");
    };
    assert!(
        locals
            .iter()
            .any(|local| local.name == "x" && local.ty == Type::I32)
    );
    assert!(
        locals
            .iter()
            .any(|local| local.name == "y" && local.ty == Type::I32)
    );
    let match_expr = blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .find_map(|statement| match &statement.kind {
            StatementKind::Let { initializer, .. } => match &initializer.kind {
                ExprKind::Match { arms, .. } => Some(&arms[0].pattern.kind),
                _ => None,
            },
            _ => None,
        })
        .expect("match expression should lower into MIR");
    assert!(matches!(
        match_expr,
        MatchPatternKind::Struct { struct_name, fields }
            if struct_name == "Point" && fields.len() == 2
    ));
}

#[test]
fn lowers_array_literals_and_index_reads() {
    let program = lower(
        "\
fn main() -> i32 {
let values: [i32; 3] = [1, 2, 3];
return values[2];
}
",
    );

    let ItemKind::Function { blocks, .. } = &program.items[0].kind else {
        panic!("expected function item");
    };

    let StatementKind::Let { initializer, .. } = &blocks[0].statements[0].kind else {
        panic!("expected let statement");
    };
    assert!(matches!(initializer.kind, ExprKind::ArrayLiteral { .. }));

    let returned = blocks
        .iter()
        .find_map(|block| match &block.terminator.kind {
            TerminatorKind::Return { value } => Some(value),
            _ => None,
        })
        .expect("return terminator should exist");
    assert!(matches!(returned.kind, ExprKind::Index { .. }));
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

    let ItemKind::Function { blocks, .. } = &program.items[0].kind else {
        panic!("expected function item");
    };

    let StatementKind::Assign { target, .. } = &blocks[0].statements[1].kind else {
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

    let ItemKind::Function { blocks, .. } = &program.items[1].kind else {
        panic!("expected main function");
    };

    let StatementKind::Assign { target, .. } = &blocks[0].statements[1].kind else {
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

    let ItemKind::Function { blocks, .. } = &program.items[0].kind else {
        panic!("expected function item");
    };
    let StatementKind::Let { initializer, .. } = &blocks[0].statements[0].kind else {
        panic!("expected value let statement");
    };
    let ExprKind::Match { arms, .. } = &initializer.kind else {
        panic!("expected match initializer");
    };
    let ExprKind::Block { statements, value } = &arms[0].value.kind else {
        panic!("expected block-valued match arm");
    };
    assert!(matches!(statements[0].kind, StatementKind::Let { .. }));
    assert!(matches!(value.kind, ExprKind::Binary { .. }));
}

#[test]
fn lowers_break_to_loop_exit_block() {
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

    let ItemKind::Function { blocks, .. } = &program.items[0].kind else {
        panic!("expected function item");
    };

    assert!(matches!(
        blocks[0].terminator.kind,
        TerminatorKind::Goto { target: 1 }
    ));
    assert!(matches!(
        blocks[1].terminator.kind,
        TerminatorKind::Branch {
            then_block: 2,
            else_block: 3,
            ..
        }
    ));
    assert!(matches!(
        blocks[2].terminator.kind,
        TerminatorKind::Goto { target: 3 }
    ));
}

#[test]
fn lowers_continue_to_loop_condition_block() {
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

    let ItemKind::Function { blocks, .. } = &program.items[0].kind else {
        panic!("expected function item");
    };

    assert!(matches!(
        blocks[0].terminator.kind,
        TerminatorKind::Goto { target: 1 }
    ));
    assert!(matches!(
        blocks[1].terminator.kind,
        TerminatorKind::Branch {
            then_block: 2,
            else_block: 3,
            ..
        }
    ));
    assert!(matches!(
        blocks[2].terminator.kind,
        TerminatorKind::Goto { target: 1 }
    ));
}
