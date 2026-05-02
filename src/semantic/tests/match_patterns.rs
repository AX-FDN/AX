use super::*;

#[test]
fn accepts_match_expressions() {
    let codes = check(
        "\
fn main() -> i32 {
    let flag: bool = true;
    let value: i32 = match (flag) { true => 1, false => 0 };
    return value;
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn accepts_block_valued_match_expression_arms() {
    let codes = check(
        "\
fn main() -> i32 {
    let flag: bool = true;
    let value: i32 = match (flag) {
        true => { let base: i32 = 40; base + 2 },
        false => { let fallback: i32 = 0; fallback },
    };
    return value;
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn reports_block_valued_match_arm_type_mismatch() {
    let codes = check(
        "\
fn main() -> i32 {
    let flag: bool = true;
    let value: i32 = match (flag) {
        true => { let base: i32 = 40; base + 2 },
        false => { let fallback: string = \"off\"; fallback },
    };
    return value;
}
",
    );
    assert!(codes.iter().any(|code| code == "S0022"), "{codes:?}");
}

#[test]
fn reports_control_flow_inside_block_valued_match_arm() {
    let codes = check(
        "\
fn main() -> i32 {
    let flag: bool = true;
    let value: i32 = match (flag) {
        true => {
            if (flag) {
                println(1);
            }
            1
        },
        false => 0,
    };
    return value;
}
",
    );
    assert!(codes.iter().any(|code| code == "S0057"), "{codes:?}");
}

#[test]
fn reports_match_expression_arm_type_mismatch() {
    let codes = check(
        "\
fn main() -> i32 {
    let flag: bool = true;
    let value: i32 = match (flag) { true => 1, false => \"off\" };
    return value;
}
",
    );
    assert!(codes.iter().any(|code| code == "S0022"));
}

#[test]
fn accepts_match_binding_patterns() {
    let codes = check(
        "\
fn main() -> i32 {
    let source: i32 = 4;
    let value: i32 = match (source) { 0 => 1, other => other };
    return value;
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn accepts_match_or_patterns() {
    let codes = check(
        "\
enum Mode { Check, Run, Build, Other }

fn score(mode: Mode) -> i32 {
    return match (mode) {
        Mode.Check | Mode.Run => 1,
        Mode.Build => 2,
        Mode.Other => 0,
    };
}

fn main() -> i32 {
    return score(Mode.Run);
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn accepts_match_guards() {
    let codes = check(
        "\
enum Token { Number(i32), End }

fn main() -> i32 {
    let code: i32 = match (Token.Number(12)) {
        Token.Number(value) if value > 9 => value,
        Token.Number(_) => 1,
        Token.End => 0,
    };
    return code;
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn rejects_non_bool_match_guards() {
    let codes = check(
        "\
fn main() -> i32 {
    let code: i32 = match (2) {
        2 if 1 => 10,
        _ => 0,
    };
    return code;
}
",
    );
    assert!(codes.contains(&"S0022".to_string()), "{codes:?}");
}

#[test]
fn accepts_match_range_patterns() {
    let codes = check(
        "\
fn main() -> i32 {
    let code: i32 = match (404) {
        100..=199 => 1,
        400..=499 => 4,
        _ => 0,
    };
    return code;
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn accepts_match_struct_destructuring_patterns() {
    let codes = check(
        "\
struct Point {
    x: i32,
    y: i32,
}

fn score(point: Point) -> i32 {
    return match (point) {
        Point { x, y } => x + y,
    };
}

fn main() -> i32 {
    let point: Point = Point { x: 2, y: 3 };
    return score(point);
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn accepts_generic_match_struct_destructuring_patterns() {
    let codes = check(
        "\
struct Pair<T> {
    left: T,
    right: T,
}

fn sum(pair: Pair<i32>) -> i32 {
    return match (pair) {
        Pair { left, right } => left + right,
    };
}

fn main() -> i32 {
    let pair: Pair<i32> = Pair { left: 20, right: 22 };
    return sum(pair);
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn reports_incomplete_match_struct_destructuring_patterns() {
    let codes = check(
        "\
struct Point {
    x: i32,
    y: i32,
}

fn score(point: Point) -> i32 {
    return match (point) {
        Point { x } => x,
    };
}

fn main() -> i32 {
    let point: Point = Point { x: 2, y: 3 };
    return score(point);
}
",
    );
    assert!(codes.iter().any(|code| code == "S0060"), "{codes:?}");
}

#[test]
fn reports_duplicate_match_struct_destructuring_fields() {
    let codes = check(
        "\
struct Point {
    x: i32,
    y: i32,
}

fn score(point: Point) -> i32 {
    return match (point) {
        Point { x, x, y } => x + y,
    };
}

fn main() -> i32 {
    let point: Point = Point { x: 2, y: 3 };
    return score(point);
}
",
    );
    assert!(codes.iter().any(|code| code == "S0060"), "{codes:?}");
}

#[test]
fn reports_unknown_match_struct_destructuring_fields() {
    let codes = check(
        "\
struct Point {
    x: i32,
    y: i32,
}

fn score(point: Point) -> i32 {
    return match (point) {
        Point { x, y, z } => x + y,
    };
}

fn main() -> i32 {
    let point: Point = Point { x: 2, y: 3 };
    return score(point);
}
",
    );
    assert!(codes.iter().any(|code| code == "S0060"), "{codes:?}");
}

#[test]
fn accepts_project_match_struct_destructuring_patterns() {
    let project_root = repo_root()
        .join("target")
        .join("semantic-project-match-struct-pattern-test");
    let _ = fs::remove_dir_all(&project_root);
    fs::create_dir_all(project_root.join("lib")).expect("lib directory should exist");
    fs::create_dir_all(project_root.join("src")).expect("src directory should exist");
    fs::write(
        project_root.join("AX.toml"),
        "\
manifest_version = 1

[package]
name = \"semantic_project_match_struct_pattern\"
entry = \"src/main.ax\"
sources = [\"lib\"]
",
    )
    .expect("manifest should exist");
    fs::write(
        project_root.join("lib").join("point.ax"),
        "\
module lib.point;

struct Point {
    x: i32,
    y: i32,
}

fn make_point() -> Point {
    return Point { x: 20, y: 22 };
}
",
    )
    .expect("support file should exist");
    fs::write(
        project_root.join("src").join("main.ax"),
        "\
import lib.point;

fn main() -> i32 {
    let point: lib.point.Point = lib.point.make_point();
    return match (point) {
        lib.point.Point { x, y } => x + y,
    };
}
",
    )
    .expect("entry file should exist");

    let diagnostics = project_diagnostics(&project_root);
    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>()
    );

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn reports_empty_match_range_patterns() {
    let codes = check(
        "\
fn main() -> i32 {
    let code: i32 = match (404) {
        499..=400 => 4,
        _ => 0,
    };
    return code;
}
",
    );
    assert!(codes.contains(&"S0056".to_string()), "{codes:?}");
}

#[test]
fn accepts_payload_enum_construction_and_match_patterns() {
    let codes = check(
        "\
enum Result {
    Ok(i32),
    Err(string),
    Empty,
}

fn score(result: Result) -> i32 {
    return match (result) {
        Result.Ok(value) => value,
        Result.Err(_) => 0,
        Result.Empty => -1,
    };
}

fn main() -> i32 {
    let ok: Result = Result.Ok(7);
    let err: Result = Result.Err(\"bad\");
    println(score(ok));
    println(score(err));
    return score(ok);
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}
