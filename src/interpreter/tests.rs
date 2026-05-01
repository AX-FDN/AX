use std::collections::BTreeMap;

use super::{RunContext, run_program, run_program_with_context};
use crate::frontend::analyze;
use crate::source::SourceFile;

fn analyzed_hir(source_text: &str) -> (SourceFile, crate::hir::Program) {
    let source = SourceFile::anonymous(source_text);
    let analysis = analyze(&source);
    assert!(
        analysis.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        analysis
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>()
    );

    (
        source,
        analysis
            .hir
            .expect("HIR should be available after successful analysis"),
    )
}

#[test]
fn runs_loops_functions_and_println() {
    let (source, hir) = analyzed_hir(
        "\
fn step(value: i32) -> i32 {
    return value + 1;
}

fn main() -> i32 {
    let mut count: i32 = 0;
    while (count < 3) {
        count = step(count);
    }
    println(count);
    return count;
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 3);
    assert_eq!(output.stdout, vec!["3"]);
}

#[test]
fn runs_conditionals() {
    let (source, hir) = analyzed_hir(
        "\
fn main() -> i32 {
    let flag: bool = true;
    if (flag) {
        println(\"ready\");
        return 0;
    } else {
        return 1;
    }
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 0);
    assert_eq!(output.stdout, vec!["ready"]);
}

#[test]
fn runs_recursive_functions() {
    let (source, hir) = analyzed_hir(
        "\
fn fact(n: i32) -> i32 {
    if (n == 0) {
        return 1;
    } else {
        return n * fact(n - 1);
    }
}

fn main() -> i32 {
    println(fact(5));
    return 0;
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 0);
    assert_eq!(output.stdout, vec!["120"]);
}

#[test]
fn runs_struct_literals_and_field_access() {
    let (source, hir) = analyzed_hir(
        "\
struct Point {
    x: i32,
    y: i32,
}

fn total(point: Point) -> i32 {
    return point.x + point.y;
}

fn main() -> i32 {
    let point: Point = Point { x: 2, y: 3 };
    println(point.x);
    println(total(point));
    return 0;
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 0);
    assert_eq!(output.stdout, vec!["2", "5"]);
}

#[test]
fn runs_generic_impl_methods() {
    let (source, hir) = analyzed_hir(
        "\
struct Box<T> {
    value: T,
}

impl<T> Box<T> {
    fn get(self: Box<T>) -> T {
        return self.value;
    }
}

fn main() -> i32 {
    let number: Box<i32> = Box { value: 9 };
    println(number.get());
    return number.get();
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 9);
    assert_eq!(output.stdout, vec!["9"]);
}

#[test]
fn runs_generic_trait_impl_methods() {
    let (source, hir) = analyzed_hir(
        "\
trait Label {
    fn label(self: Self) -> string;
}

struct Box<T> {
    value: T,
}

impl<T> Label for Box<T> {
    fn label(self: Box<T>) -> string {
        return to_string(self.value);
    }
}

fn render<T: Label>(value: T) -> string {
    return value.label();
}

fn main() -> i32 {
    let number: Box<i32> = Box { value: 42 };
    println(render(number));
    return 0;
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 0);
    assert_eq!(output.stdout, vec!["42"]);
}

#[test]
fn runs_enum_values_and_mutable_field_assignment() {
    let (source, hir) = analyzed_hir(
        "\
struct Point {
    x: i32,
    y: i32,
}

enum Flag {
    On,
    Off,
}

fn total(point: Point) -> i32 {
    return point.x + point.y;
}

fn main() -> i32 {
    let mut point: Point = Point { x: 2, y: 3 };
    point.x = point.x + 1;

    let flag: Flag = Flag.On;
    println(flag);
    println(point.x);
    println(total(point));
    return 0;
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 0);
    assert_eq!(output.stdout, vec!["Flag.On", "3", "6"]);
}

#[test]
fn runs_lowered_for_loops() {
    let (source, hir) = analyzed_hir(
        "\
fn main() -> i32 {
    let mut total: i32 = 0;
    for (let mut i: i32 = 0; i < 4; i = i + 1) {
        total = total + i;
    }
    println(total);
    return total;
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 6);
    assert_eq!(output.stdout, vec!["6"]);
}

#[test]
fn runs_match_statements() {
    let (source, hir) = analyzed_hir(
        "\
enum Flag {
    On,
    Off,
}

fn choose(flag: Flag) -> i32 {
    match (flag) {
        Flag.On => {
            return 1;
        }
        Flag.Off => {
            return 2;
        }
    }
}

fn classify(value: i32) -> i32 {
    match (value) {
        0 => {
            return 7;
        }
        _ => {
            return value;
        }
    }
}

fn main() -> i32 {
    let truthy: bool = true;
    let mut total: i32 = 0;
    match (truthy) {
        true => {
            total = total + 10;
        }
        false => {
            total = total + 1;
        }
    }
    total = total + choose(Flag.On);
    total = total + choose(Flag.Off);
    total = total + classify(0);
    total = total + classify(5);
    println(total);
    return total;
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 25);
    assert_eq!(output.stdout, vec!["25"]);
}

#[test]
fn runs_match_expressions() {
    let (source, hir) = analyzed_hir(
        "\
fn classify(flag: bool) -> i32 {
    return match (flag) { true => 3, false => 1 };
}

fn main() -> i32 {
    let left: i32 = match (false) { true => 8, false => 2 };
    let right: i32 = classify(true);
    println(left);
    println(right);
    return left + right;
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 5);
    assert_eq!(output.stdout, vec!["2", "3"]);
}

#[test]
fn runs_block_valued_match_expression_arms() {
    let (source, hir) = analyzed_hir(
        "\
fn classify(flag: bool) -> i32 {
    return match (flag) {
        true => { let base: i32 = 40; base + 2 },
        false => { let fallback: i32 = 5; fallback },
    };
}

fn main() -> i32 {
    let value: i32 = classify(true);
    println(value);
    return value;
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 42);
    assert_eq!(output.stdout, vec!["42"]);
}

#[test]
fn runs_match_binding_patterns() {
    let (source, hir) = analyzed_hir(
        "\
fn classify(value: i32) -> i32 {
    return match (value) { 0 => 10, other => other + 2 };
}

fn main() -> i32 {
    let flag: bool = false;
    match (flag) {
        true => {
            println(\"true\");
        }
        current => {
            if (current) {
                println(\"unexpected\");
            } else {
                println(\"false\");
            }
        }
    }
    let code: i32 = classify(4);
    println(code);
    return code;
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 6);
    assert_eq!(output.stdout, vec!["false", "6"]);
}

#[test]
fn runs_match_struct_destructuring_patterns() {
    let (source, hir) = analyzed_hir(
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
    let point: Point = Point { x: 20, y: 22 };
    let value: i32 = score(point);
    println(value);
    return value;
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 42);
    assert_eq!(output.stdout, vec!["42"]);
}

#[test]
fn runs_payload_enum_constructors_and_matches() {
    let (source, hir) = analyzed_hir(
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
    let empty: Result = Result.Empty;
    println(score(ok));
    println(score(err));
    println(score(empty));
    return score(ok);
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 7);
    assert_eq!(output.stdout, vec!["7", "0", "-1"]);
}

#[test]
fn runs_logical_short_circuit_operators() {
    let (source, hir) = analyzed_hir(
        "\
fn main() -> i32 {
    if (false && 8 / 0 == 0) {
        return 1;
    }
    if (true || 8 / 0 == 0) {
        println(\"short-circuit\");
        return 7;
    }
    return 0;
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 7);
    assert_eq!(output.stdout, vec!["short-circuit"]);
}

#[test]
fn runs_modulo_operator() {
    let (source, hir) = analyzed_hir(
        "\
fn main() -> i32 {
    let bucket: i32 = 10 % 3;
    println(bucket);
    return bucket;
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 1);
    assert_eq!(output.stdout, vec!["1"]);
}

#[test]
fn runs_break_inside_loops() {
    let (source, hir) = analyzed_hir(
        "\
fn main() -> i32 {
    let mut count: i32 = 0;
    while (true) {
        count = count + 1;
        break;
    }
    println(count);
    return count;
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 1);
    assert_eq!(output.stdout, vec!["1"]);
}

#[test]
fn env_lookup_matches_windows_case_insensitive_behavior() {
    let (source, hir) = analyzed_hir(
        "\
fn main() -> i32 {
    let present: bool = env_has(\"PATH\");
    println(present);
    if (present) {
        let value: string = env_get(\"PATH\");
        println(value);
        return len(value);
    }
    return 0;
}
",
    );

    let mut env = BTreeMap::new();
    env.insert("Path".to_string(), "ready".to_string());
    let context = RunContext {
        argv: Vec::new(),
        env,
        current_dir: ".".into(),
    };

    let output = run_program_with_context(&source, &hir, context).expect("program should run");

    if cfg!(windows) {
        assert_eq!(output.exit_code, 5);
        assert_eq!(output.stdout, vec!["true", "ready"]);
    } else {
        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, vec!["false"]);
    }
}

#[test]
fn runs_fixed_size_arrays_and_index_reads() {
    let (source, hir) = analyzed_hir(
        "\
fn main() -> i32 {
    let values: [i32; 3] = [1, 2, 3];
    println(values);
    println(values[1]);
    return values[0] + values[2];
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 4);
    assert_eq!(output.stdout, vec!["[1, 2, 3]", "2"]);
}

#[test]
fn runs_mutable_array_element_assignment() {
    let (source, hir) = analyzed_hir(
        "\
fn main() -> i32 {
    let mut values: [i32; 3] = [1, 2, 3];
    values[1] = values[0] + values[2];
    println(values);
    return values[1];
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 4);
    assert_eq!(output.stdout, vec!["[1, 4, 3]"]);
}

#[test]
fn runs_nested_assignment_through_array_elements_and_fields() {
    let (source, hir) = analyzed_hir(
        "\
struct Token {
    value: i32,
}

fn main() -> i32 {
    let mut tokens: [Token; 3] = [
        Token { value: 1 },
        Token { value: 2 },
        Token { value: 3 },
    ];

    let mut index: i32 = 0;
    while (index < len(tokens)) {
        tokens[index].value = tokens[index].value + 10;
        index = index + 1;
    }

    println(tokens);
    return tokens[0].value + tokens[2].value;
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 24);
    assert_eq!(
        output.stdout,
        vec!["[Token { value: 11 }, Token { value: 12 }, Token { value: 13 }]"]
    );
}

#[test]
fn runs_nested_struct_field_assignment_paths() {
    let (source, hir) = analyzed_hir(
        "\
struct Inner {
    value: i32,
}

struct Outer {
    inner: Inner,
}

fn main() -> i32 {
    let mut outer: Outer = Outer { inner: Inner { value: 5 } };
    outer.inner.value = outer.inner.value + 7;
    println(outer);
    return outer.inner.value;
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 12);
    assert_eq!(output.stdout, vec!["Outer { inner: Inner { value: 12 } }"]);
}

#[test]
fn runs_slice_reads_and_slice_parameters() {
    let (source, hir) = analyzed_hir(
        "\
fn second(values: [i32]) -> i32 {
    println(values);
    return values[1];
}

fn main() -> i32 {
    let values: [i32; 4] = [1, 2, 3, 4];
    let window: [i32] = values[1:3];
    println(window);
    return second(window);
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 3);
    assert_eq!(output.stdout, vec!["[2, 3]", "[2, 3]"]);
}

#[test]
fn runs_integer_division() {
    let (source, hir) = analyzed_hir(
        "\
fn main() -> i32 {
    let value: i32 = 8 / 2;
    println(value);
    return value;
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 4);
    assert_eq!(output.stdout, vec!["4"]);
}

#[test]
fn runs_string_concat_and_string_len() {
    let (source, hir) = analyzed_hir(
        "\
fn main() -> i32 {
    let prefix: string = \"AX\";
    let message: string = prefix + \" tools\";
    println(message);
    println(string_len(message));
    return string_len(\"hey\");
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 3);
    assert_eq!(output.stdout, vec!["AX tools", "8"]);
}

#[test]
fn runs_long_left_associative_string_concat_chain() {
    let (source, hir) = analyzed_hir(
            "\
fn main() -> i32 {
    let message: string = \"a\" + \"b\" + \"c\" + \"d\" + \"e\" + \"f\" + \"g\" + \"h\" + \"i\" + \"j\" + \"k\" + \"l\" + \"m\" + \"n\" + \"o\" + \"p\" + \"q\" + \"r\" + \"s\" + \"t\" + \"u\" + \"v\" + \"w\" + \"x\" + \"y\" + \"z\";
    println(message);
    return string_len(message);
}
",
        );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 26);
    assert_eq!(output.stdout, vec!["abcdefghijklmnopqrstuvwxyz"]);
}

#[test]
fn runs_len_for_strings_arrays_and_slices() {
    let (source, hir) = analyzed_hir(
        "\
fn sum(values: [i32]) -> i32 {
    let mut total: i32 = 0;
    for (let mut i: i32 = 0; i < len(values); i = i + 1) {
        total = total + values[i];
    }
    return total;
}

fn main() -> i32 {
    let values: [i32; 5] = [1, 2, 3, 4, 5];
    let middle: [i32] = values[1:4];
    println(len(\"AX\"));
    println(len(values));
    println(len(middle));
    println(sum(middle));
    return sum(values);
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 15);
    assert_eq!(output.stdout, vec!["2", "5", "3", "9"]);
}

#[test]
fn runs_string_list_builtins() {
    let (source, hir) = analyzed_hir(
        "\
fn main() -> i32 {
    let mut lines: string_list = string_list_new();
    lines = string_list_push(lines, \"alpha\");
    lines = string_list_push(lines, \"beta\");
    println(len(lines));
    println(string_list_join(lines, \", \"));
    println(string_list_get(lines, 1));
    return len(lines);
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 2);
    assert_eq!(output.stdout, vec!["2", "alpha, beta", "beta"]);
}

#[test]
fn runs_to_string_for_tool_style_reports() {
    let (source, hir) = analyzed_hir(
        "\
struct Summary {
    count: i32,
    ready: bool,
}

fn build_report(summary: Summary, values: [i32]) -> string {
    let mut report: string = \"count=\" + to_string(summary.count);
    report = report + \", ready=\" + to_string(summary.ready);
    report = report + \", values=\" + to_string(values);
    return report;
}

fn main() -> i32 {
    let summary: Summary = Summary { count: 3, ready: true };
    let values: [i32; 3] = [2, 4, 6];
    let report: string = build_report(summary, values[0:2]);
    println(report);
    return string_len(report);
}
",
    );

    let output = run_program(&source, &hir).expect("program should run");
    assert_eq!(output.exit_code, 34);
    assert_eq!(output.stdout, vec!["count=3, ready=true, values=[2, 4]"]);
}
