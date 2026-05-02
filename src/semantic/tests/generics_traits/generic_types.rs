use super::*;

#[test]
fn accepts_generic_struct_instances() {
    let codes = check(
        "\
struct Box<T> { value: T }
struct Pair<T> { left: T, right: T }

fn main() -> i32 {
    let mut number_box: Box<i32> = Box { value: 7 };
    number_box.value = number_box.value + 1;
    let text_box: Box<string> = Box { value: \"ax\" };
    let pair: Pair<i32> = Pair { left: number_box.value, right: 5 };
    println(text_box.value);
    return pair.left + pair.right;
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn accepts_top_level_consts() {
    let codes = check(
        "\
const EXIT_OK: i32 = 7;
const LABEL: string = \"const-ready\";

fn main() -> i32 {
    println(LABEL);
    return EXIT_OK;
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn reports_const_initializer_type_mismatch() {
    let codes = check(
        "\
const EXIT_OK: i32 = \"bad\";

fn main() -> i32 {
    return 0;
}
",
    );
    assert!(codes.iter().any(|code| code == "S0022"));
}

#[test]
fn reports_assignment_to_const_as_immutable() {
    let codes = check(
        "\
const EXIT_OK: i32 = 7;

fn main() -> i32 {
    EXIT_OK = 8;
    return EXIT_OK;
}
",
    );
    assert!(codes.iter().any(|code| code == "S0003"));
}

#[test]
fn reports_generic_struct_type_argument_count_mismatch() {
    let codes = check(
        "\
struct Box<T> { value: T }

fn main() -> i32 {
    let number_box: Box = Box { value: 7 };
    return number_box.value;
}
",
    );
    assert!(codes.iter().any(|code| code == "S0058"));
}

#[test]
fn reports_generic_struct_field_type_mismatch() {
    let codes = check(
        "\
struct Pair<T> { left: T, right: T }

fn main() -> i32 {
    let pair: Pair<i32> = Pair { left: 1, right: \"bad\" };
    return pair.left;
}
",
    );
    assert!(codes.iter().any(|code| code == "S0022"));
}

#[test]
fn accepts_generic_functions() {
    let codes = check(
        "\
struct Box<T> { value: T }

fn identity<T>(value: T) -> T {
    return value;
}

fn unwrap_box<T>(box: Box<T>) -> T {
    return box.value;
}

fn main() -> i32 {
    let number_box: Box<i32> = Box { value: identity(9) };
    let text_box: Box<string> = Box { value: identity(\"ax\") };
    println(unwrap_box(text_box));
    return unwrap_box(number_box);
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn reports_generic_function_argument_type_mismatch() {
    let codes = check(
        "\
fn choose<T>(left: T, right: T) -> T {
    return left;
}

fn main() -> i32 {
    return choose(1, \"bad\");
}
",
    );
    assert!(codes.iter().any(|code| code == "S0022"));
}

#[test]
fn accepts_generic_enum_construction_and_match_payloads() {
    let codes = check(
        "\
enum Result<T, E> {
    Ok(T),
    Err(E),
}

fn value_or_zero(result: Result<i32, string>) -> i32 {
    return match (result) {
        Result.Ok(value) => value,
        Result.Err(_) => 0,
    };
}

fn main() -> i32 {
    let ok: Result<i32, string> = Result.Ok(7);
    let err: Result<i32, string> = Result.Err(\"bad\");
    return value_or_zero(ok) + value_or_zero(err);
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn infers_match_expression_enum_constructors_from_return_type() {
    let codes = check(
        "\
enum Result<T, E> {
    Ok(T),
    Err(E),
}

enum ConfigError {
    Io(string),
}

fn from_io<T>(value: Result<T, string>) -> Result<T, ConfigError> {
    return match (value) {
        Result.Ok(found) => Result.Ok(found),
        Result.Err(error) => Result.Err(ConfigError.Io(error)),
    };
}

fn main() -> i32 {
    return 0;
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn accepts_generic_enum_unit_variant_with_expected_instance_type() {
    let codes = check(
        "\
enum Option<T> {
    Some(T),
    None,
}
fn main() -> i32 {
    let missing: Option<i32> = Option.None;
    let value: i32 = match (missing) { Option.Some(found) => found, Option.None => 0 };
    return value;
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn reports_generic_enum_type_argument_count_mismatch() {
    let codes = check(
        "\
enum Result<T, E> {
    Ok(T),
    Err(E),
}

fn main() -> i32 {
    let ok: Result<i32> = Result.Ok(7);
    return 0;
}
",
    );
    assert!(codes.iter().any(|code| code == "S0058"));
}

#[test]
fn reports_generic_enum_payload_assignment_mismatch() {
    let codes = check(
        "\
enum Result<T, E> {
    Ok(T),
    Err(E),
}

fn main() -> i32 {
    let ok: Result<i32, string> = Result.Ok(\"bad\");
    return 0;
}
",
    );
    assert!(codes.iter().any(|code| code == "S0022"));
}
