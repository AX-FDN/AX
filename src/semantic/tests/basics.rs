use super::*;

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
    let codes =
        check("fn add(value: i32) -> i32 { return value; } fn main() -> i32 { return add(true); }");
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
    let codes = check("fn main() -> i32 { let values: [i32; 3] = [1, 2, 3]; return values[1]; }");
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
    let codes =
        check("fn main() -> i32 { let value: i32 = 1; let part: [i32] = value[0:1]; return 0; }");
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
fn accepts_mutable_slice_element_assignment() {
    let codes = check(
        "\
fn main() -> i32 {
    let values: [i32; 3] = [1, 2, 3];
    let mut head: [i32] = values[0:2];
    head[0] = 9;
    return head[0];
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn accepts_nested_assignment_through_slice_elements() {
    let codes = check(
        "\
struct Token { value: i32 }

fn main() -> i32 {
    let mut tokens: [Token; 2] = [Token { value: 1 }, Token { value: 2 }];
    let mut view: [Token] = tokens[0:2];
    view[0].value = 9;
    return view[0].value;
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
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
fn accepts_string_list_get_builtin() {
    let codes = check(
        "\
fn main() -> i32 {
    let mut items: string_list = string_list_new();
    items = string_list_push(items, \"alpha\");
    let first: string = string_list_get(items, 0);
    return string_len(first);
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
    let codes = check("fn main() -> i32 { let values: [i32; 1] = [1]; values[0] = 2; return 0; }");
    assert!(codes.iter().any(|code| code == "S0003"));
}
