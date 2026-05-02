use super::*;

#[test]
fn enriches_undefined_variable_diagnostic_with_scope_notes() {
    let diagnostics =
        diagnostics("fn main() -> i32 { let count: i32 = 1; return missing + count; }");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "S0002")
        .expect("undefined variable diagnostic should exist");

    assert!(
        diagnostic
            .notes
            .iter()
            .any(|note| note.contains("block-scoped"))
    );
    assert!(
        diagnostic
            .notes
            .iter()
            .any(|note| note.contains("visible variables here: count"))
    );
}

#[test]
fn enriches_type_mismatch_diagnostic_with_conversion_note() {
    let diagnostics = diagnostics("fn main() -> i32 { let value: bool = 1; return 0; }");
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "S0022")
        .expect("type mismatch diagnostic should exist");

    assert!(
        diagnostic
            .notes
            .iter()
            .any(|note| note.contains("does not implicitly convert"))
    );
    assert!(
        diagnostic
            .suggestion
            .as_deref()
            .is_some_and(|suggestion| suggestion.contains("produce `bool`"))
    );
}

#[test]
fn enriches_missing_return_diagnostic_with_fallback_hint() {
    let diagnostics = diagnostics(
        "fn helper(flag: bool) -> i32 { if (flag) { return 1; } } fn main() -> i32 { return helper(true); }",
    );
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "S0023")
        .expect("missing return diagnostic should exist");

    assert!(
        diagnostic
            .notes
            .iter()
            .any(|note| note.contains("no `else`"))
    );
    assert_eq!(
        diagnostic.suggestion.as_deref(),
        Some("add a fallback like `return 0;`, or make every branch return `i32`")
    );
}
