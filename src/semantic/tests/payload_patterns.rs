use super::*;

#[test]
fn reports_unknown_payload_enum_constructor_variant_without_function_fallback() {
    let diagnostics = diagnostics(
        "\
enum Result { Ok(i32) }

fn main() -> i32 {
    let result: Result = Result.Unknown(1);
    return 0;
}
",
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "S0029")
    );
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "S0007")
    );
}

#[test]
fn reports_payload_enum_variant_used_without_payload() {
    let diagnostics = diagnostics(
        "\
enum Result { Ok(i32), Err(string) }

fn main() -> i32 {
    let result: Result = Result.Ok;
    return 0;
}
",
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "S0053"
            && diagnostic.kind() == Some(DiagnosticKind::EnumVariantPayloadShapeMismatch)
    }));
}

#[test]
fn reports_payload_enum_constructor_type_mismatch() {
    let diagnostics = diagnostics(
        "\
enum Result { Ok(i32) }

fn main() -> i32 {
    let result: Result = Result.Ok(true);
    return 0;
}
",
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "S0022"
            && diagnostic.kind() == Some(DiagnosticKind::EnumVariantPayloadTypeMismatch)
    }));
}

#[test]
fn reports_payload_enum_pattern_without_payload_binding() {
    let diagnostics = diagnostics(
        "\
enum Result { Ok(i32), Err(string) }

fn main() -> i32 {
    let result: Result = Result.Ok(7);
    return match (result) {
        Result.Ok => 1,
        Result.Err(_) => 0,
    };
}
",
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "S0055"
            && diagnostic.kind() == Some(DiagnosticKind::MatchEnumVariantPayloadShapeMismatch)
    }));
}

#[test]
fn payload_enum_pattern_shape_error_does_not_cascade_into_exhaustiveness() {
    let diagnostics = diagnostics(
        "\
enum Result { Ok(i32), Err(string), Empty }

fn score(result: Result) -> i32 {
    return match (result) {
        Result.Ok => 1,
        Result.Err(_) => 0,
        Result.Empty => -1,
    };
}

fn main() -> i32 {
    return score(Result.Ok(7));
}
",
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "S0055"
            && diagnostic.kind() == Some(DiagnosticKind::MatchEnumVariantPayloadShapeMismatch)
    }));
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "S0049"),
        "payload shape mistakes should keep the repair target focused instead of also reporting non-exhaustiveness: {diagnostics:?}"
    );
}

#[test]
fn unit_enum_pattern_payload_error_does_not_cascade_into_exhaustiveness() {
    let diagnostics = diagnostics(
        "\
enum Result { Ok(i32), Empty }

fn score(result: Result) -> i32 {
    return match (result) {
        Result.Ok(value) => value,
        Result.Empty(_) => 0,
    };
}

fn main() -> i32 {
    return score(Result.Empty);
}
",
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "S0055"
            && diagnostic.kind() == Some(DiagnosticKind::MatchEnumVariantPayloadShapeMismatch)
    }));
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "S0049"),
        "unit variant payload mistakes should not produce a second non-exhaustive match diagnostic: {diagnostics:?}"
    );
}

#[test]
fn reports_match_binding_before_final_arm() {
    let codes = check(
        "\
fn main() -> i32 {
    let flag: bool = true;
    match (flag) {
        current => {
            return 1;
        }
        true => {
            return 0;
        }
    }
}
",
    );
    assert!(codes.iter().any(|code| code == "S0048"));
}

#[test]
fn accepts_for_in_over_arrays() {
    let codes = check(
        "\
fn main() -> i32 {
    let values: [i32; 3] = [1, 2, 3];
    let mut total: i32 = 0;
    for (let value: i32 in values) {
        total = total + value;
    }
    return total;
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn reports_non_sequence_for_in_iterable() {
    let codes = check(
        "\
fn main() -> i32 {
    let message: string = \"AX\";
    for (let value: string in message) {
        println(value);
    }
    return 0;
}
",
    );
    assert!(codes.iter().any(|code| code == "S0052"));
}

#[test]
fn reports_for_in_binding_type_mismatch() {
    let codes = check(
        "\
fn main() -> i32 {
    let values: [i32; 2] = [1, 2];
    for (let value: bool in values) {
        println(value);
    }
    return 0;
}
",
    );
    assert!(codes.iter().any(|code| code == "S0022"));
}

#[test]
fn reports_non_exhaustive_bool_match() {
    let codes = check(
        "\
fn main() -> i32 {
    let flag: bool = true;
    match (flag) {
        true => {
            return 1;
        }
    }
}
",
    );
    assert!(codes.iter().any(|code| code == "S0049"));
}

#[test]
fn reports_duplicate_match_pattern() {
    let codes = check(
        "\
fn main() -> i32 {
    let value: i32 = 1;
    match (value) {
        0 => {
            return 1;
        }
        0 => {
            return 2;
        }
        _ => {
            return 3;
        }
    }
}
",
    );
    assert!(codes.iter().any(|code| code == "S0047"));
}

#[test]
fn reports_match_wildcard_before_final_arm() {
    let codes = check(
        "\
fn main() -> i32 {
    let value: i32 = 1;
    match (value) {
        _ => {
            return 1;
        }
        0 => {
            return 2;
        }
    }
}
",
    );
    assert!(codes.iter().any(|code| code == "S0048"));
}

#[test]
fn reports_match_pattern_type_mismatch() {
    let codes = check(
        "\
fn main() -> i32 {
    let flag: bool = true;
    match (flag) {
        0 => {
            return 1;
        }
        _ => {
            return 0;
        }
    }
}
",
    );
    assert!(codes.iter().any(|code| code == "S0046"));
}

#[test]
fn reports_match_without_concrete_pattern() {
    let codes = check(
        "\
fn main() -> i32 {
    let value: i32 = 1;
    match (value) {
        _ => {
            return value;
        }
    }
}
",
    );
    assert!(codes.iter().any(|code| code == "S0050"));
}
