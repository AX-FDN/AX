use super::*;

#[test]
fn accepts_string_match_patterns() {
    let codes = check(
        "\
fn classify(command: string) -> i32 {
    return match (command) {
        \"check\" => 1,
        \"run\" => 2,
        _ => 0,
    };
}

fn main() -> i32 {
    return classify(\"check\");
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn reports_string_match_without_catch_all() {
    let codes = check(
        "\
fn main() -> i32 {
    return match (\"check\") {
        \"check\" => 1,
    };
}
",
    );
    assert!(codes.iter().any(|code| code == "S0049"));
}
