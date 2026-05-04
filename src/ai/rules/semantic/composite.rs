use super::RuleTemplate;

pub(super) const RULE_STRUCT_FIELD_MUST_EXIST: RuleTemplate = RuleTemplate {
    rule_id: "struct_field_must_exist",
    normalized_pattern: "struct_field_must_exist",
    repair_goal: "Use a field name that exists in the referenced struct declaration.",
    summary: "Struct field access and struct literal fields must match the declared field names exactly.",
    pattern: "Point { x: 1, y: 2 }",
    minimal_example: "return point.x;",
    anti_pattern: Some("Point { x: 1, z: 2 }"),
    default_fixit: "change this field name to one declared on the struct",
};

pub(super) const RULE_FIELD_ACCESS_REQUIRES_STRUCT_VALUE: RuleTemplate = RuleTemplate {
    rule_id: "field_access_requires_struct_value",
    normalized_pattern: "field_access_requires_struct_value",
    repair_goal: "Change the base expression so it evaluates to a struct value before using `.`.",
    summary: "AX field access with `.` only works on struct values.",
    pattern: "point.x",
    minimal_example: "let point: Point = Point { x: 1, y: 2 };",
    anti_pattern: Some("1.x"),
    default_fixit: "replace the base expression with a struct value or remove the field access",
};

pub(super) const RULE_STRUCT_LITERAL_REQUIRES_STRUCT_TYPE: RuleTemplate = RuleTemplate {
    rule_id: "struct_literal_requires_struct_type",
    normalized_pattern: "struct_literal_requires_struct_type",
    repair_goal: "Use a declared struct name with `Name { field: value }` syntax.",
    summary: "Struct literal syntax is only valid for declared `struct` types in AX.",
    pattern: "Point { x: 1, y: 2 }",
    minimal_example: "struct Point { x: i32, y: i32 }",
    anti_pattern: Some("bool { value: true }"),
    default_fixit: "replace this with a declared struct type or another expression form",
};

pub(super) const RULE_STRUCT_LITERAL_FIELDS_UNIQUE: RuleTemplate = RuleTemplate {
    rule_id: "struct_literal_fields_must_be_unique",
    normalized_pattern: "struct_literal_fields_must_be_unique",
    repair_goal: "Keep only one initializer for each field in this struct literal.",
    summary: "Each field may appear at most once inside an AX struct literal.",
    pattern: "Point { x: 1, y: 2 }",
    minimal_example: "Pair { left: 1, right: 2 }",
    anti_pattern: Some("Point { x: 1, x: 2 }"),
    default_fixit: "remove or rename the duplicate field initializer",
};

pub(super) const RULE_STRUCT_LITERAL_FIELDS_COMPLETE: RuleTemplate = RuleTemplate {
    rule_id: "struct_literal_fields_must_be_complete",
    normalized_pattern: "struct_literal_fields_must_be_complete",
    repair_goal: "Add the missing field initializer(s) so the struct literal is complete.",
    summary: "AX struct literals must initialize every declared field.",
    pattern: "Point { x: 1, y: 2 }",
    minimal_example: "Pair { left: 1, right: 2 }",
    anti_pattern: Some("Point { x: 1 }"),
    default_fixit: "add the missing field initializer(s)",
};

pub(super) const RULE_ENUM_VARIANT_MUST_EXIST: RuleTemplate = RuleTemplate {
    rule_id: "enum_variant_must_exist",
    normalized_pattern: "enum_variant_must_exist",
    repair_goal: "Use a variant name that is declared on the enum.",
    summary: "Enum value syntax in AX must use an existing variant from the enum declaration.",
    pattern: "Color.Red",
    minimal_example: "enum Color { Red, Blue }",
    anti_pattern: Some("Color.Green"),
    default_fixit: "replace this with an existing enum variant",
};

pub(super) const RULE_MUTABLE_STRUCT_FIELD_ASSIGNMENT_REQUIRED: RuleTemplate = RuleTemplate {
    rule_id: "mutable_struct_field_assignment_required",
    normalized_pattern: "mutable_struct_field_assignment_required",
    repair_goal: "Assign only through a mutable struct variable and only to declared fields.",
    summary: "Field assignment requires a mutable struct variable, a real field name, and a compatible value type.",
    pattern: "let mut point: Point = Point { x: 1, y: 2 }; point.x = 3;",
    minimal_example: "let mut pair: Pair = Pair { left: 1, right: 2 }; pair.left = 3;",
    anti_pattern: Some("let point: Point = Point { x: 1, y: 2 }; point.x = 3;"),
    default_fixit: "use `let mut` on the struct variable and assign only to declared fields",
};

pub(super) const RULE_ENUM_VARIANT_PAYLOAD_MUST_MATCH_DECLARATION: RuleTemplate = RuleTemplate {
    rule_id: "enum_variant_payload_must_match_declaration",
    normalized_pattern: "enum_variant_payload_must_match_declaration",
    repair_goal: "Construct the enum variant using the payload shape declared on that variant.",
    summary: "Unit enum variants are bare values like `Flag.On`, while payload enum variants are constructed as `EnumName.Variant(value)`.",
    pattern: "let result: Result = Result.Ok(7);",
    minimal_example: "enum Result { Ok(i32), Err(string) }",
    anti_pattern: Some("let result: Result = Result.Ok;"),
    default_fixit: "either add the required payload argument or remove `(...)` when the variant is unit-like",
};

pub(super) const RULE_ENUM_VARIANT_PAYLOAD_TYPE_MUST_MATCH_DECLARATION: RuleTemplate =
    RuleTemplate {
        rule_id: "enum_variant_payload_type_must_match_declaration",
        normalized_pattern: "enum_variant_payload_type_must_match_declaration",
        repair_goal: "Pass a payload value whose type matches the enum variant declaration.",
        summary: "The payload argument for `EnumName.Variant(value)` must use the type declared on that enum variant.",
        pattern: "enum Result { Ok(i32) } fn main() -> i32 { let result: Result = Result.Ok(7); return 0; }",
        minimal_example: "let result: Result = Result.Ok(1);",
        anti_pattern: Some("let result: Result = Result.Ok(true);"),
        default_fixit: "rewrite the payload expression so it produces the variant's declared payload type",
    };

pub(super) const RULE_TRAIT_REFERENCE_MUST_RESOLVE: RuleTemplate = RuleTemplate {
    rule_id: "trait_reference_must_resolve",
    normalized_pattern: "trait_reference_must_resolve",
    repair_goal: "Reference a trait that is declared and visible before using it in an impl or generic bound.",
    summary: "AX trait references must point at a declared trait; generic bounds like `T: Label` and `impl Label for Type` cannot invent traits implicitly.",
    pattern: "trait Label { fn label(self: Self) -> string; }\nfn render<T: Label>(value: T) -> string { return value.label(); }",
    minimal_example: "trait Named { fn name(self: Self) -> string; }",
    anti_pattern: Some("fn render<T: MissingTrait>(value: T) -> T { return value; }"),
    default_fixit: "declare the missing trait, import the module that owns it, or change the reference to an existing trait",
};

pub(super) const RULE_TRAIT_BOUND_MUST_BE_SATISFIED: RuleTemplate = RuleTemplate {
    rule_id: "trait_bound_must_be_satisfied",
    normalized_pattern: "trait_bound_must_be_satisfied",
    repair_goal: "Pass a value whose type implements the required trait, or add the missing `impl Trait for Type` block.",
    summary: "AX generic function bounds are checked at the call site, so `fn render<T: Label>(value: T)` only accepts types with `impl Label for ThatType`.",
    pattern: "impl Label for Command { fn label(self: Command) -> string { return self.name; } }",
    minimal_example: "fn render<T: Label>(value: T) -> string { return value.label(); }",
    anti_pattern: Some("render(1)"),
    default_fixit: "add the required trait impl for the concrete type or call the generic function with an implementing value",
};

pub(super) const RULE_RESULT_PROPAGATION_REQUIRES_RESULT: RuleTemplate = RuleTemplate {
    rule_id: "result_propagation_requires_result",
    normalized_pattern: "result_propagation_requires_result",
    repair_goal: "Use `?` only when the expression and current function both use compatible `Result<T, E>` types.",
    summary: "`expr?` unwraps `Result.Ok(value)` and returns `Result.Err(error)` from the current function, so both sides need compatible `Result` error types.",
    pattern: "fn load() -> std.result.Result<string, string> { let text: string = std.fs.try_read_to_string(\"input.txt\")?; return std.result.Result.ok(text); }",
    minimal_example: "let text: string = read_text()?;",
    anti_pattern: Some("fn main() -> i32 { let text: string = read_text()?; return 0; }"),
    default_fixit: "return a compatible `Result<_, E>` from the function or replace `?` with an explicit `match`",
};

pub(super) const RULE_NON_EMPTY_ARRAY_LITERAL_REQUIRED: RuleTemplate = RuleTemplate {
    rule_id: "non_empty_array_literal_required",
    normalized_pattern: "non_empty_array_literal_required",
    repair_goal: "Either give `[]` a zero-length array context like `[i32; 0]`, or add elements so the array has a concrete non-zero length.",
    summary: "AX accepts `[]` only when the surrounding context fixes it to a length-0 array type such as `[i32; 0]`.",
    pattern: "let values: [i32; 3] = [1, 2, 3];",
    minimal_example: "let values: [i32; 0] = [];",
    anti_pattern: Some("let values: [i32; 1] = [];"),
    default_fixit: "change the surrounding type to `[Type; 0]` or add elements to the array literal",
};

pub(super) const RULE_INDEX_BASE_MUST_BE_ARRAY: RuleTemplate = RuleTemplate {
    rule_id: "index_base_must_be_array",
    normalized_pattern: "index_base_must_be_array",
    repair_goal: "Use `expr[index]` only when the base expression evaluates to an array or slice; writes require a mutable binding.",
    summary: "AX indexing with `[]` reads from arrays and slices, and assignment through an index requires the indexed root binding to be `mut`.",
    pattern: "let value: i32 = values[0];",
    minimal_example: "let mut values: [i32; 2] = [1, 2]; values[1] = values[0];",
    anti_pattern: Some("let value: i32 = number[0];"),
    default_fixit: "index into an array value like `values[0]`",
};

pub(super) const RULE_ARRAY_INDEX_MUST_BE_I32: RuleTemplate = RuleTemplate {
    rule_id: "array_index_must_be_i32",
    normalized_pattern: "array_index_must_be_i32",
    repair_goal: "Rewrite the index expression so it produces an `i32` value.",
    summary: "AX array and slice indexing accepts only `i32` index expressions before runtime bounds checks run.",
    pattern: "let value: i32 = values[index];",
    minimal_example: "let index: i32 = 1; return values[index];",
    anti_pattern: Some("return values[true];"),
    default_fixit: "change the index expression to an `i32` value",
};

pub(super) const RULE_SLICE_BASE_MUST_BE_ARRAY_OR_SLICE: RuleTemplate = RuleTemplate {
    rule_id: "slice_base_must_be_array_or_slice",
    normalized_pattern: "slice_base_must_be_array_or_slice",
    repair_goal: "Use `base[start:end]` only when `base` is already an array or slice value.",
    summary: "AX slice expressions create slice values from arrays or existing slices; scalars and structs cannot be sliced.",
    pattern: "let window: [i32] = values[1:3];",
    minimal_example: "let values: [i32; 4] = [1, 2, 3, 4]; let head: [i32] = values[0:2];",
    anti_pattern: Some("let window: [i32] = count[0:1];"),
    default_fixit: "slice an array or slice value instead of a scalar or struct",
};

pub(super) const RULE_LEN_BUILTIN_REQUIRES_COUNTABLE_VALUE: RuleTemplate = RuleTemplate {
    rule_id: "len_builtin_requires_countable_value",
    normalized_pattern: "len_builtin_requires_countable_value",
    repair_goal: "Call `len(value)` only with a `string`, `string_list`, fixed-size array, or slice value.",
    summary: "AX uses `len(value)` as the unified length helper for strings, `string_list`, and sequence-like values that already have a stable length in the prototype.",
    pattern: "let size: i32 = len(values);",
    minimal_example: "let values: [i32; 3] = [1, 2, 3]; return len(values);",
    anti_pattern: Some("return len(true);"),
    default_fixit: "pass a string, string_list, array, or slice to `len(...)`",
};
