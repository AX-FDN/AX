use crate::ast::{ItemKind, Program};
use crate::diagnostics::Diagnostic;
use crate::source::{SourceFile, Span};

#[path = "semantic/checker.rs"]
mod checker;
#[path = "semantic/helpers.rs"]
mod helpers;
#[path = "semantic/program_info.rs"]
mod program_info;
#[path = "semantic/return_analysis.rs"]
mod return_analysis;
#[path = "semantic/types.rs"]
mod types;

use checker::TypeChecker;
use program_info::ProgramInfo;
use return_analysis::missing_return_diagnostic;

pub fn check_program(source: &SourceFile, program: &Program) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let program_info = ProgramInfo::collect(source, program, &mut diagnostics);

    if !program_info.has_main {
        diagnostics.push(
            Diagnostic::new(
                "S0004",
                "program is missing `fn main() -> i32`",
                source,
                Span::new(0, 0),
            )
            .with_note("runnable AX programs currently require a zero-argument `main` entrypoint")
            .with_suggestion("add `fn main() -> i32 { return 0; }`"),
        );
    }

    for item in &program.items {
        if let ItemKind::Function {
            name,
            params,
            return_type,
            body,
            ..
        } = &item.kind
        {
            let resolved_return_type = program_info.resolve_type_ref(return_type, &mut diagnostics);
            let mut checker = TypeChecker::new(&program_info, resolved_return_type, &mut diagnostics);

            for param in params {
                let resolved_param_type =
                    program_info.resolve_type_ref(&param.ty, checker.diagnostics_mut());
                checker.declare(&param.name, resolved_param_type, false, param.span.start);
            }

            checker.check_block(body);
            let missing_return =
                missing_return_diagnostic(source, name, checker.return_type(), body);
            drop(checker);

            if let Some(diagnostic) = missing_return {
                diagnostics.push(diagnostic);
            }
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::check_program;
    use crate::diagnostics::Diagnostic;
    use crate::lexer::tokenize;
    use crate::parser::parse;
    use crate::source::SourceFile;

    fn check(source_text: &str) -> Vec<String> {
        diagnostics(source_text)
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect()
    }

    fn diagnostics(source_text: &str) -> Vec<Diagnostic> {
        let source = SourceFile::anonymous(source_text);
        let tokens = tokenize(&source).tokens;
        let parsed = parse(&source, tokens);
        check_program(&source, &parsed.program)
    }

    #[test]
    fn reports_immutable_assignment() {
        let codes = check("fn main() -> i32 { let value: i32 = 1; value = 2; return value; }");
        assert!(codes.iter().any(|code| code == "S0003"));
    }

    #[test]
    fn reports_missing_main() {
        let codes = check("fn helper() -> i32 { return 0; }");
        assert!(codes.iter().any(|code| code == "S0004"));
    }

    #[test]
    fn reports_duplicate_function_definitions() {
        let codes = check(
            "fn helper() -> i32 { return 0; } fn helper() -> i32 { return 1; } fn main() -> i32 { return helper(); }",
        );
        assert!(codes.iter().any(|code| code == "S0001"));
    }

    #[test]
    fn reports_type_mismatch_in_variable_declaration() {
        let codes = check("fn main() -> i32 { let value: bool = 1; return 0; }");
        assert!(codes.iter().any(|code| code == "S0022"));
    }

    #[test]
    fn reports_bad_function_argument_type() {
        let codes = check(
            "fn add(value: i32) -> i32 { return value; } fn main() -> i32 { return add(true); }",
        );
        assert!(codes.iter().any(|code| code == "S0022"));
    }

    #[test]
    fn reports_function_that_can_fall_through() {
        let codes = check(
            "fn helper(flag: bool) -> i32 { if (flag) { return 1; } } fn main() -> i32 { return helper(true); }",
        );
        assert!(codes.iter().any(|code| code == "S0023"));
    }

    #[test]
    fn checks_struct_literal_fields() {
        let codes = check(
            "struct Point { x: i32, y: i32 } fn main() -> i32 { let point: Point = Point { x: 1, y: 2 }; return point.x; }",
        );
        assert!(!codes.iter().any(|code| code == "S0022"));
        assert!(!codes.iter().any(|code| code == "S0026"));
    }

    #[test]
    fn reports_missing_struct_literal_field() {
        let codes = check(
            "struct Point { x: i32, y: i32 } fn main() -> i32 { let point: Point = Point { x: 1 }; return 0; }",
        );
        assert!(codes.iter().any(|code| code == "S0026"));
    }

    #[test]
    fn accepts_enum_variants_and_mutable_struct_field_assignment() {
        let codes = check(
            "\
enum Flag { On, Off }
struct Point { x: i32, y: i32 }
fn main() -> i32 {
    let flag: Flag = Flag.On;
    let mut point: Point = Point { x: 1, y: 2 };
    point.x = point.x + 1;
    if (flag == Flag.On) {
        return point.x;
    }
    return 0;
}
",
        );
        assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
    }

    #[test]
    fn reports_type_name_used_as_value() {
        let codes =
            check("enum Flag { On, Off } fn main() -> i32 { let flag: Flag = Flag; return 0; }");
        assert!(codes.iter().any(|code| code == "S0028"));
    }

    #[test]
    fn reports_unknown_enum_variant() {
        let codes = check(
            "enum Flag { On, Off } fn main() -> i32 { let flag: Flag = Flag.Unknown; return 0; }",
        );
        assert!(codes.iter().any(|code| code == "S0029"));
    }

    #[test]
    fn reports_immutable_struct_field_assignment() {
        let codes = check(
            "struct Point { x: i32 } fn main() -> i32 { let point: Point = Point { x: 1 }; point.x = 2; return 0; }",
        );
        assert!(codes.iter().any(|code| code == "S0030"));
    }

    #[test]
    fn accepts_fixed_size_arrays_and_index_reads() {
        let codes =
            check("fn main() -> i32 { let values: [i32; 3] = [1, 2, 3]; return values[1]; }");
        assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
    }

    #[test]
    fn accepts_empty_array_literal_in_zero_length_array_context() {
        let codes = check("fn main() -> i32 { let values: [i32; 0] = []; println(values); return 0; }");
        assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
    }

    #[test]
    fn accepts_slice_params_and_slice_expressions() {
        let codes = check(
            "\
fn second(values: [i32]) -> i32 {
    return values[1];
}

fn main() -> i32 {
    let values: [i32; 3] = [1, 2, 3];
    let head: [i32] = values[0:2];
    return second(head);
}
",
        );
        assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
    }

    #[test]
    fn reports_non_array_index_base() {
        let codes = check("fn main() -> i32 { let value: i32 = 1; return value[0]; }");
        assert!(codes.iter().any(|code| code == "S0033"));
    }

    #[test]
    fn reports_empty_array_literal_without_zero_length_context() {
        let codes = check("fn main() -> i32 { let values: [i32; 1] = []; return 0; }");
        assert!(codes.iter().any(|code| code == "S0032"));
    }

    #[test]
    fn reports_non_slice_base() {
        let codes = check("fn main() -> i32 { let value: i32 = 1; let part: [i32] = value[0:1]; return 0; }");
        assert!(codes.iter().any(|code| code == "S0034"));
    }

    #[test]
    fn accepts_mutable_array_element_assignment() {
        let codes = check(
            "fn main() -> i32 { let mut values: [i32; 2] = [1, 2]; values[0] = 3; return values[0]; }",
        );
        assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
    }

    #[test]
    fn reports_slice_assignment_as_read_only() {
        let codes = check(
            "\
fn main() -> i32 {
    let values: [i32; 3] = [1, 2, 3];
    let mut head: [i32] = values[0:2];
    head[0] = 9;
    return 0;
}
",
        );
        assert!(codes.iter().any(|code| code == "S0035"));
    }

    #[test]
    fn accepts_string_concat_and_string_len() {
        let codes = check(
            "\
fn main() -> i32 {
    let prefix: string = \"AX\";
    let message: string = prefix + \" tools\";
    println(message);
    return string_len(message);
}
",
        );
        assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
    }

    #[test]
    fn accepts_len_for_strings_arrays_and_slices() {
        let codes = check(
            "\
fn main() -> i32 {
    let values: [i32; 4] = [1, 2, 3, 4];
    let view: [i32] = values[1:3];
    let chars: i32 = len(\"AX\");
    let total: i32 = len(values) + len(view);
    return chars + total;
}
",
        );
        assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
    }

    #[test]
    fn reports_non_string_argument_to_string_len() {
        let codes = check("fn main() -> i32 { return string_len(1); }");
        assert!(codes.iter().any(|code| code == "S0022"));
    }

    #[test]
    fn reports_invalid_argument_to_len() {
        let codes = check("fn main() -> i32 { return len(true); }");
        assert!(codes.iter().any(|code| code == "S0022"));
    }

    #[test]
    fn reports_immutable_array_element_assignment() {
        let codes =
            check("fn main() -> i32 { let values: [i32; 1] = [1]; values[0] = 2; return 0; }");
        assert!(codes.iter().any(|code| code == "S0003"));
    }

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
}
