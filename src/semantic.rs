use crate::ast::{ItemKind, Program};
use crate::diagnostics::Diagnostic;
use crate::project::Project;
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
    check_program_with_project(source, program, None)
}

pub fn check_program_with_project(
    source: &SourceFile,
    program: &Program,
    project: Option<&Project>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let program_info = ProgramInfo::collect(source, program, project, &mut diagnostics);

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
            let current_unit_path = source.display_path_for_offset(item.span.start).to_string();
            let resolved_return_type =
                program_info.resolve_type_ref(return_type, &current_unit_path, &mut diagnostics);
            let mut checker = TypeChecker::new(
                &program_info,
                resolved_return_type,
                current_unit_path.clone(),
                &mut diagnostics,
            );

            for param in params {
                let resolved_param_type = program_info.resolve_type_ref(
                    &param.ty,
                    &current_unit_path,
                    checker.diagnostics_mut(),
                );
                checker.declare(&param.name, resolved_param_type, false, param.span.start);
            }

            checker.check_block(body);
            let missing_return = missing_return_diagnostic(
                source,
                name,
                checker.return_type(),
                body,
                &program_info,
                &current_unit_path,
            );
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
    use super::{check_program, check_program_with_project};
    use crate::diagnostics::{Diagnostic, DiagnosticKind};
    use crate::lexer::tokenize;
    use crate::parser::parse;
    use crate::project::resolve_input;
    use crate::source::SourceFile;
    use std::fs;
    use std::path::PathBuf;

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

    fn project_diagnostics(project_root: &PathBuf) -> Vec<Diagnostic> {
        let resolved = resolve_input(project_root).expect("project should resolve");
        let tokens = tokenize(&resolved.source).tokens;
        let parsed = parse(&resolved.source, tokens);
        check_program_with_project(&resolved.source, &parsed.program, resolved.project.as_ref())
    }

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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
        let codes =
            check("fn main() -> i32 { let values: [i32; 0] = []; println(values); return 0; }");
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
        let codes = check(
            "fn main() -> i32 { let value: i32 = 1; let part: [i32] = value[0:1]; return 0; }",
        );
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
    fn accepts_nested_assignment_through_array_elements_and_fields() {
        let codes = check(
            "\
struct Token { value: i32 }

fn main() -> i32 {
    let mut tokens: [Token; 2] = [Token { value: 1 }, Token { value: 2 }];
    tokens[1].value = tokens[0].value + 4;
    return tokens[1].value;
}
",
        );
        assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
    }

    #[test]
    fn accepts_nested_struct_field_assignment_paths() {
        let codes = check(
            "\
struct Inner { value: i32 }
struct Outer { inner: Inner }

fn main() -> i32 {
    let mut outer: Outer = Outer { inner: Inner { value: 1 } };
    outer.inner.value = outer.inner.value + 2;
    return outer.inner.value;
}
",
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
    fn reports_nested_assignment_through_slice_as_read_only() {
        let codes = check(
            "\
struct Token { value: i32 }

fn main() -> i32 {
    let mut tokens: [Token; 2] = [Token { value: 1 }, Token { value: 2 }];
    let mut view: [Token] = tokens[0:2];
    view[0].value = 9;
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
    fn accepts_to_string_for_concrete_runtime_values() {
        let codes = check(
            "\
struct Summary { count: i32, ready: bool }

fn main() -> i32 {
    let summary: Summary = Summary { count: 3, ready: true };
    let values: [i32; 3] = [1, 2, 3];
    let text: string = to_string(summary) + to_string(values[0:2]);
    println(text);
    return string_len(text);
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
    fn reports_invalid_argument_to_to_string() {
        let codes = check("fn main() -> i32 { return string_len(to_string(println(1))); }");
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

    #[test]
    fn reports_missing_module_declaration_in_module_mode_project() {
        let project_root = repo_root()
            .join("target")
            .join("semantic-module-missing-decl-test");
        let _ = fs::remove_dir_all(&project_root);
        fs::create_dir_all(project_root.join("lib")).expect("lib directory should exist");
        fs::create_dir_all(project_root.join("src")).expect("src directory should exist");
        fs::write(
            project_root.join("AX.toml"),
            "\
manifest_version = 1

[package]
name = \"semantic_module_missing_decl\"
entry = \"src/main.ax\"
sources = [\"lib\"]
",
        )
        .expect("manifest should exist");
        fs::write(
            project_root.join("lib").join("report.ax"),
            "fn helper() -> i32 { return 1; }\n",
        )
        .expect("support file should exist");
        fs::write(
            project_root.join("src").join("main.ax"),
            "import lib.report;\nfn main() -> i32 { return 0; }\n",
        )
        .expect("entry file should exist");

        let diagnostics = project_diagnostics(&project_root);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "S0038")
        );

        let _ = fs::remove_dir_all(&project_root);
    }

    #[test]
    fn reports_missing_import_for_cross_module_reference() {
        let project_root = repo_root()
            .join("target")
            .join("semantic-module-missing-import-test");
        let _ = fs::remove_dir_all(&project_root);
        fs::create_dir_all(project_root.join("lib")).expect("lib directory should exist");
        fs::create_dir_all(project_root.join("src")).expect("src directory should exist");
        fs::write(
            project_root.join("AX.toml"),
            "\
manifest_version = 1

[package]
name = \"semantic_module_missing_import\"
entry = \"src/main.ax\"
sources = [\"lib\"]
",
        )
        .expect("manifest should exist");
        fs::write(
            project_root.join("lib").join("report.ax"),
            "module lib.report;\nfn helper() -> i32 { return 1; }\n",
        )
        .expect("support file should exist");
        fs::write(
            project_root.join("src").join("main.ax"),
            "fn main() -> i32 { return lib.report.helper(); }\n",
        )
        .expect("entry file should exist");

        let diagnostics = project_diagnostics(&project_root);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "S0043")
        );

        let _ = fs::remove_dir_all(&project_root);
    }
}
