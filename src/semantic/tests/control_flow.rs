use super::*;

#[test]
fn accepts_for_loop_with_local_initializer() {
    let codes = check(
        "\
fn main() -> i32 {
    let mut total: i32 = 0;
    for (let mut i: i32 = 0; i < 4; i = i + 1) {
        total = total + i;
    }
    return total;
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn accepts_break_inside_loops() {
    let codes = check(
        "\
fn main() -> i32 {
    let mut total: i32 = 0;
    while (true) {
        total = total + 1;
        break;
    }
    return total;
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn accepts_continue_inside_loops() {
    let codes = check(
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
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn accepts_logical_and_or_expressions() {
    let codes = check(
        "\
fn main() -> i32 {
    let ready: bool = true;
    let has_input: bool = false;
    let should_run: bool = ready && !has_input || false;
    if (should_run) {
        return 1;
    }
    return 0;
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn accepts_modulo_expressions() {
    let codes = check(
        "\
fn main() -> i32 {
    let bucket: i32 = 10 % 3;
    return bucket;
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn accepts_exhaustive_bool_match() {
    let codes = check(
        "\
fn main() -> i32 {
    let flag: bool = true;
    match (flag) {
        true => {
            return 1;
        }
        false => {
            return 0;
        }
    }
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn accepts_exhaustive_enum_match_that_returns_on_all_arms() {
    let codes = check(
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

fn main() -> i32 {
    return choose(Flag.On);
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn reports_non_exhaustive_enum_match() {
    let codes = check(
        "\
enum Flag {
    On,
    Off,
}

fn choose(flag: Flag) -> i32 {
    return match (flag) {
        Flag.On => 1,
    };
}

fn main() -> i32 {
    return choose(Flag.On);
}
",
    );
    assert!(codes.iter().any(|code| code == "S0049"));
}

#[test]
fn reports_for_initializer_variable_used_outside_loop() {
    let codes = check(
        "\
fn main() -> i32 {
    for (let mut i: i32 = 0; i < 1; i = i + 1) {
        println(i);
    }
    return i;
}
",
    );
    assert!(codes.iter().any(|code| code == "S0002"));
}

#[test]
fn reports_break_outside_loop() {
    let codes = check("fn main() -> i32 { break; return 0; }");
    assert!(codes.iter().any(|code| code == "S0036"));
}

#[test]
fn reports_continue_outside_loop() {
    let codes = check("fn main() -> i32 { continue; return 0; }");
    assert!(codes.iter().any(|code| code == "S0044"));
}

#[test]
fn reports_non_bool_logical_operands() {
    let codes = check("fn main() -> i32 { let value: bool = 1 && true; return 0; }");
    assert!(codes.iter().any(|code| code == "S0051"));
}

#[test]
fn reports_non_i32_modulo_operands() {
    let codes = check("fn main() -> i32 { let value: i32 = 1.0 % 2.0; return value; }");
    assert!(codes.iter().any(|code| code == "S0014"));
}
