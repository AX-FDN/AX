use super::*;

#[test]
fn infers_static_method_generic_params_from_expected_return_type() {
    let codes = check(
        "\
enum Result<T, E> {
    Ok(T),
    Err(E),
}

impl<T, E> Result<T, E> {
    fn err(error: E) -> Result<T, E> {
        return Result.Err(error);
    }
}

fn parse() -> Result<i32, string> {
    return Result.err(\"bad\");
}

fn main() -> i32 {
    let result: Result<i32, string> = Result.err(\"missing\");
    return match (result) { Result.Ok(value) => value, Result.Err(_) => 0 };
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn accepts_result_error_propagation_operator() {
    let codes = check(
        "\
enum Result<T, E> {
    Ok(T),
    Err(E),
}

impl<T, E> Result<T, E> {
    fn ok(value: T) -> Result<T, E> {
        return Result.Ok(value);
    }

    fn err(error: E) -> Result<T, E> {
        return Result.Err(error);
    }
}

fn parse(text: string) -> Result<i32, string> {
    if (text == \"ok\") {
        return Result.ok(7);
    }
    return Result.err(\"bad\");
}

fn add_one(text: string) -> Result<i32, string> {
    let value: i32 = parse(text)?;
    return Result.ok(value + 1);
}

fn main() -> i32 {
    return match (add_one(\"ok\")) { Result.Ok(value) => value, Result.Err(_) => 0 };
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn reports_result_error_propagation_outside_result_return() {
    let diagnostics = diagnostics(
        "\
enum Result<T, E> {
    Ok(T),
    Err(E),
}

fn parse() -> Result<i32, string> {
    return Result.Ok(1);
}

fn main() -> i32 {
    let value: i32 = parse()?;
    return value;
}
",
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "S0054"
            && diagnostic.kind()
                == Some(crate::diagnostics::DiagnosticKind::ResultPropagationRequiresResult)
    }));
}
