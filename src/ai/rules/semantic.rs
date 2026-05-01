use crate::diagnostics::DiagnosticKind;

use super::RuleTemplate;

pub(super) fn match_code(code: &str) -> Option<RuleTemplate> {
    match code {
        "S0001" => Some(RULE_UNIQUE_DEFINITION_REQUIRED),
        "S0002" => Some(RULE_UNDEFINED_VARIABLE),
        "S0003" => Some(RULE_IMMUTABLE_ASSIGNMENT),
        "S0004" => Some(RULE_MAIN_REQUIRED),
        "S0005" => Some(RULE_MAIN_SIGNATURE),
        "S0006" => Some(RULE_TYPE_MUST_BE_DECLARED),
        "S0007" => Some(RULE_FUNCTION_MUST_BE_DECLARED),
        "S0008" => Some(RULE_ASSIGNMENT_TARGET_REQUIRED),
        "S0011" => Some(RULE_FUNCTION_NAME_NOT_RUNTIME_VALUE),
        "S0017" => Some(RULE_FUNCTION_ARGUMENT_COUNT_MATCH),
        "S0018" | "S0019" => Some(RULE_CALL_TARGET_MUST_BE_FUNCTION_NAME),
        "S0020" | "S0027" => Some(RULE_STRUCT_FIELD_MUST_EXIST),
        "S0021" => Some(RULE_FIELD_ACCESS_REQUIRES_STRUCT_VALUE),
        "S0022" => Some(RULE_TYPE_MISMATCH),
        "S0023" => Some(RULE_MISSING_RETURN),
        "S0024" => Some(RULE_STRUCT_LITERAL_REQUIRES_STRUCT_TYPE),
        "S0025" => Some(RULE_STRUCT_LITERAL_FIELDS_UNIQUE),
        "S0026" => Some(RULE_STRUCT_LITERAL_FIELDS_COMPLETE),
        "S0028" => Some(RULE_TYPE_NAME_NOT_RUNTIME_VALUE),
        "S0029" => Some(RULE_ENUM_VARIANT_MUST_EXIST),
        "S0030" => Some(RULE_MUTABLE_STRUCT_FIELD_ASSIGNMENT_REQUIRED),
        "S0031" => Some(RULE_FOR_HEADER_CLAUSE_SUPPORTED),
        "S0052" => Some(RULE_FOR_IN_REQUIRES_SEQUENCE_VALUE),
        "S0032" => Some(RULE_NON_EMPTY_ARRAY_LITERAL_REQUIRED),
        "S0033" => Some(RULE_INDEX_BASE_MUST_BE_ARRAY),
        "S0034" => Some(RULE_SLICE_BASE_MUST_BE_ARRAY_OR_SLICE),
        "S0035" => Some(RULE_SLICE_VALUES_ARE_READ_ONLY),
        "S0057" => Some(RULE_BLOCK_MATCH_ARM_MUST_STAY_LINEAR),
        "S0060" => Some(RULE_MATCH_STRUCT_PATTERN_MUST_MATCH_DECLARATION),
        "R0040" => Some(RULE_LEN_BUILTIN_REQUIRES_COUNTABLE_VALUE),
        _ => None,
    }
}

pub(super) fn match_kind(kind: DiagnosticKind) -> Option<RuleTemplate> {
    match kind {
        DiagnosticKind::BreakOutsideLoop => Some(RULE_BREAK_REQUIRES_LOOP_CONTEXT),
        DiagnosticKind::ContinueOutsideLoop => Some(RULE_CONTINUE_REQUIRES_LOOP_CONTEXT),
        DiagnosticKind::MatchScrutineeTypeUnsupported => {
            Some(RULE_MATCH_INPUT_MUST_USE_SUPPORTED_TYPE)
        }
        DiagnosticKind::MatchPatternTypeMismatch => Some(RULE_MATCH_PATTERN_MUST_MATCH_INPUT),
        DiagnosticKind::DuplicateMatchPattern => Some(RULE_MATCH_PATTERNS_MUST_BE_UNIQUE),
        DiagnosticKind::MatchWildcardMustBeLast => Some(RULE_MATCH_WILDCARD_MUST_BE_LAST),
        DiagnosticKind::MatchNotExhaustive => Some(RULE_MATCH_MUST_BE_EXHAUSTIVE),
        DiagnosticKind::MatchRequiresConcretePattern => Some(RULE_MATCH_REQUIRES_CONCRETE_PATTERN),
        DiagnosticKind::MatchExpressionArmTypeMismatch => {
            Some(RULE_MATCH_EXPRESSION_ARMS_MUST_SHARE_TYPE)
        }
        DiagnosticKind::MatchEnumVariantPayloadShapeMismatch => {
            Some(RULE_MATCH_ENUM_VARIANT_PAYLOAD_MUST_MATCH_DECLARATION)
        }
        DiagnosticKind::MatchStructPatternShapeMismatch => {
            Some(RULE_MATCH_STRUCT_PATTERN_MUST_MATCH_DECLARATION)
        }
        DiagnosticKind::MatchGuardTypeMismatch => Some(RULE_MATCH_GUARD_MUST_BE_BOOL),
        DiagnosticKind::MatchRangeMustBeNonEmpty => Some(RULE_MATCH_RANGE_MUST_BE_NON_EMPTY),
        DiagnosticKind::FunctionArgumentTypeMismatch => {
            Some(RULE_FUNCTION_ARGUMENT_TYPE_MUST_MATCH)
        }
        DiagnosticKind::ReturnTypeMismatch => Some(RULE_RETURN_VALUE_MUST_MATCH_DECLARED_TYPE),
        DiagnosticKind::ConditionTypeMismatch => Some(RULE_CONDITION_MUST_BE_BOOL),
        DiagnosticKind::ArrayIndexTypeMismatch => Some(RULE_ARRAY_INDEX_MUST_BE_I32),
        DiagnosticKind::LenBuiltinTypeMismatch => Some(RULE_LEN_BUILTIN_REQUIRES_COUNTABLE_VALUE),
        DiagnosticKind::ForInIterableTypeMismatch => Some(RULE_FOR_IN_REQUIRES_SEQUENCE_VALUE),
        DiagnosticKind::ForInBindingTypeMismatch => {
            Some(RULE_FOR_IN_BINDING_MUST_MATCH_ELEMENT_TYPE)
        }
        DiagnosticKind::EnumVariantPayloadShapeMismatch => {
            Some(RULE_ENUM_VARIANT_PAYLOAD_MUST_MATCH_DECLARATION)
        }
        DiagnosticKind::EnumVariantPayloadTypeMismatch => {
            Some(RULE_ENUM_VARIANT_PAYLOAD_TYPE_MUST_MATCH_DECLARATION)
        }
        DiagnosticKind::TraitReferenceMustResolve => Some(RULE_TRAIT_REFERENCE_MUST_RESOLVE),
        DiagnosticKind::TraitBoundNotSatisfied => Some(RULE_TRAIT_BOUND_MUST_BE_SATISFIED),
        DiagnosticKind::ResultPropagationRequiresResult => {
            Some(RULE_RESULT_PROPAGATION_REQUIRES_RESULT)
        }
        _ => None,
    }
}

pub(super) fn is_main_required_rule(rule: &RuleTemplate) -> bool {
    rule.rule_id == RULE_MAIN_REQUIRED.rule_id
}

const RULE_UNIQUE_DEFINITION_REQUIRED: RuleTemplate = RuleTemplate {
    rule_id: "unique_definition_required",
    normalized_pattern: "unique_definition_required",
    repair_goal: "Rename one definition or remove the duplicate so each name is declared once.",
    summary: "Each AX name may only be defined once in the same scope or top-level namespace.",
    pattern: "let total: i32 = 1;",
    minimal_example: "fn helper() -> i32 { return 0; }",
    anti_pattern: Some("let total: i32 = 1; let total: i32 = 2;"),
    default_fixit: "rename or remove the duplicate definition",
};

const RULE_TYPE_MUST_BE_DECLARED: RuleTemplate = RuleTemplate {
    rule_id: "type_must_be_declared",
    normalized_pattern: "type_must_be_declared",
    repair_goal: "Use a builtin type or declare the referenced type before using it.",
    summary: "AX type references must resolve to a builtin type or a previously declared `struct` or `enum`.",
    pattern: "struct Point { x: i32, y: i32 }",
    minimal_example: "let point: Point = Point { x: 1, y: 2 };",
    anti_pattern: Some("let point: Missing = 1;"),
    default_fixit: "declare the missing type or replace it with an existing AX type",
};

const RULE_FUNCTION_MUST_BE_DECLARED: RuleTemplate = RuleTemplate {
    rule_id: "function_must_be_declared",
    normalized_pattern: "function_must_be_declared",
    repair_goal: "Declare the function first or change the call to a function that exists.",
    summary: "AX function calls must target a declared function or builtin.",
    pattern: "fn helper() -> i32 { return 0; }",
    minimal_example: "fn main() -> i32 { return helper(); }",
    anti_pattern: Some("fn main() -> i32 { return missing(); }"),
    default_fixit: "declare the missing function or fix the call name",
};

const RULE_ASSIGNMENT_TARGET_REQUIRED: RuleTemplate = RuleTemplate {
    rule_id: "writable_assignment_target_required",
    normalized_pattern: "writable_assignment_target_required",
    repair_goal: "Assign only to a mutable variable, a direct mutable struct field, or a direct mutable array element.",
    summary: "AX assignments can only write to `name = expr;`, `struct_value.field = expr;`, or `array_value[index] = expr;` targets that are writable.",
    pattern: "value = 1;",
    minimal_example: "values[0] = 1;",
    anti_pattern: Some("(left + right) = 1;"),
    default_fixit: "rewrite the assignment to target a writable variable, direct field, or direct array element",
};

const RULE_FUNCTION_NAME_NOT_RUNTIME_VALUE: RuleTemplate = RuleTemplate {
    rule_id: "function_name_not_runtime_value",
    normalized_pattern: "function_name_not_runtime_value",
    repair_goal: "Call the function with parentheses or replace it with a real runtime value.",
    summary: "Function names are not first-class runtime values in the current AX prototype.",
    pattern: "let total: i32 = helper();",
    minimal_example: "println(helper());",
    anti_pattern: Some("let total: i32 = helper;"),
    default_fixit: "add parentheses to call the function or use a different value",
};

const RULE_FUNCTION_ARGUMENT_COUNT_MATCH: RuleTemplate = RuleTemplate {
    rule_id: "function_argument_count_must_match",
    normalized_pattern: "function_argument_count_must_match",
    repair_goal: "Pass exactly the number of arguments declared by the function signature.",
    summary: "AX does not support optional or implicit arguments; function calls must match arity exactly.",
    pattern: "add(left, right)",
    minimal_example: "fn add(left: i32, right: i32) -> i32 { return left + right; }",
    anti_pattern: Some("add(left)"),
    default_fixit: "add or remove arguments so the call arity matches the function signature",
};

const RULE_CALL_TARGET_MUST_BE_FUNCTION_NAME: RuleTemplate = RuleTemplate {
    rule_id: "call_target_must_be_function_name",
    normalized_pattern: "call_target_must_be_function_name",
    repair_goal: "Change this call so its target is a declared function name or builtin.",
    summary: "The current AX prototype only supports direct calls to function names and builtins.",
    pattern: "helper(value)",
    minimal_example: "println(value);",
    anti_pattern: Some("value(arg)"),
    default_fixit: "replace the call target with a declared function name",
};

const RULE_STRUCT_FIELD_MUST_EXIST: RuleTemplate = RuleTemplate {
    rule_id: "struct_field_must_exist",
    normalized_pattern: "struct_field_must_exist",
    repair_goal: "Use a field name that exists in the referenced struct declaration.",
    summary: "Struct field access and struct literal fields must match the declared field names exactly.",
    pattern: "Point { x: 1, y: 2 }",
    minimal_example: "return point.x;",
    anti_pattern: Some("Point { x: 1, z: 2 }"),
    default_fixit: "change this field name to one declared on the struct",
};

const RULE_FIELD_ACCESS_REQUIRES_STRUCT_VALUE: RuleTemplate = RuleTemplate {
    rule_id: "field_access_requires_struct_value",
    normalized_pattern: "field_access_requires_struct_value",
    repair_goal: "Change the base expression so it evaluates to a struct value before using `.`.",
    summary: "AX field access with `.` only works on struct values.",
    pattern: "point.x",
    minimal_example: "let point: Point = Point { x: 1, y: 2 };",
    anti_pattern: Some("1.x"),
    default_fixit: "replace the base expression with a struct value or remove the field access",
};

const RULE_STRUCT_LITERAL_REQUIRES_STRUCT_TYPE: RuleTemplate = RuleTemplate {
    rule_id: "struct_literal_requires_struct_type",
    normalized_pattern: "struct_literal_requires_struct_type",
    repair_goal: "Use a declared struct name with `Name { field: value }` syntax.",
    summary: "Struct literal syntax is only valid for declared `struct` types in AX.",
    pattern: "Point { x: 1, y: 2 }",
    minimal_example: "struct Point { x: i32, y: i32 }",
    anti_pattern: Some("bool { value: true }"),
    default_fixit: "replace this with a declared struct type or another expression form",
};

const RULE_STRUCT_LITERAL_FIELDS_UNIQUE: RuleTemplate = RuleTemplate {
    rule_id: "struct_literal_fields_must_be_unique",
    normalized_pattern: "struct_literal_fields_must_be_unique",
    repair_goal: "Keep only one initializer for each field in this struct literal.",
    summary: "Each field may appear at most once inside an AX struct literal.",
    pattern: "Point { x: 1, y: 2 }",
    minimal_example: "Pair { left: 1, right: 2 }",
    anti_pattern: Some("Point { x: 1, x: 2 }"),
    default_fixit: "remove or rename the duplicate field initializer",
};

const RULE_STRUCT_LITERAL_FIELDS_COMPLETE: RuleTemplate = RuleTemplate {
    rule_id: "struct_literal_fields_must_be_complete",
    normalized_pattern: "struct_literal_fields_must_be_complete",
    repair_goal: "Add the missing field initializer(s) so the struct literal is complete.",
    summary: "AX struct literals must initialize every declared field.",
    pattern: "Point { x: 1, y: 2 }",
    minimal_example: "Pair { left: 1, right: 2 }",
    anti_pattern: Some("Point { x: 1 }"),
    default_fixit: "add the missing field initializer(s)",
};

const RULE_TYPE_NAME_NOT_RUNTIME_VALUE: RuleTemplate = RuleTemplate {
    rule_id: "type_name_not_runtime_value",
    normalized_pattern: "type_name_not_runtime_value",
    repair_goal: "Replace the type name with a constructed value or enum variant.",
    summary: "Type names only belong in type positions, not as runtime expressions.",
    pattern: "let point: Point = Point { x: 1, y: 2 };",
    minimal_example: "let color: Color = Color.Red;",
    anti_pattern: Some("let value: i32 = Point;"),
    default_fixit: "replace the type name with a runtime value expression",
};

const RULE_ENUM_VARIANT_MUST_EXIST: RuleTemplate = RuleTemplate {
    rule_id: "enum_variant_must_exist",
    normalized_pattern: "enum_variant_must_exist",
    repair_goal: "Use a variant name that is declared on the enum.",
    summary: "Enum value syntax in AX must use an existing variant from the enum declaration.",
    pattern: "Color.Red",
    minimal_example: "enum Color { Red, Blue }",
    anti_pattern: Some("Color.Green"),
    default_fixit: "replace this with an existing enum variant",
};

const RULE_MUTABLE_STRUCT_FIELD_ASSIGNMENT_REQUIRED: RuleTemplate = RuleTemplate {
    rule_id: "mutable_struct_field_assignment_required",
    normalized_pattern: "mutable_struct_field_assignment_required",
    repair_goal: "Assign only through a mutable struct variable and only to declared fields.",
    summary: "Field assignment requires a mutable struct variable, a real field name, and a compatible value type.",
    pattern: "let mut point: Point = Point { x: 1, y: 2 }; point.x = 3;",
    minimal_example: "let mut pair: Pair = Pair { left: 1, right: 2 }; pair.left = 3;",
    anti_pattern: Some("let point: Point = Point { x: 1, y: 2 }; point.x = 3;"),
    default_fixit: "use `let mut` on the struct variable and assign only to declared fields",
};

const RULE_FOR_HEADER_CLAUSE_SUPPORTED: RuleTemplate = RuleTemplate {
    rule_id: "for_header_clause_supported",
    normalized_pattern: "for_header_clause_supported",
    repair_goal: "Rewrite the `for` header so each clause is a `let`, assignment, or expression.",
    summary: "The current AX `for` prototype only supports `let`, assignment, or expression clauses.",
    pattern: "for (let i: i32 = 0; i < 3; i = i + 1) { println(i); }",
    minimal_example: "for (let i: i32 = 0; i < 3; i = i + 1) { return i; }",
    anti_pattern: Some("for (return 0; true; step()) { }"),
    default_fixit: "rewrite the header using only `let`, assignment, or expression clauses",
};

const RULE_FOR_IN_REQUIRES_SEQUENCE_VALUE: RuleTemplate = RuleTemplate {
    rule_id: "for_in_requires_array_or_slice",
    normalized_pattern: "for_in_requires_array_or_slice",
    repair_goal: "Iterate over an array or slice value, or rewrite the loop as an indexed `for (...)` loop.",
    summary: "The first AX `for in` prototype only iterates `[T; N]` arrays and `[T]` slices.",
    pattern: "for (let value: i32 in values) { println(value); }",
    minimal_example: "let values: [i32; 3] = [1, 2, 3];",
    anti_pattern: Some("for (let ch: string in message) { println(ch); }"),
    default_fixit: "change the iterated value to an array or slice, or fall back to an indexed `for (...)` loop",
};

const RULE_FOR_IN_BINDING_MUST_MATCH_ELEMENT_TYPE: RuleTemplate = RuleTemplate {
    rule_id: "for_in_binding_must_match_element_type",
    normalized_pattern: "for_in_binding_must_match_element_type",
    repair_goal: "Declare the loop variable with the iterable's element type.",
    summary: "AX `for in` loop variables must use the same element type as the array or slice being iterated.",
    pattern: "for (let value: i32 in values) { println(value); }",
    minimal_example: "let names: [string; 2] = [\"a\", \"b\"];",
    anti_pattern: Some("for (let value: bool in values) { println(value); }"),
    default_fixit: "change the loop variable type so it matches the iterated element type",
};

const RULE_BREAK_REQUIRES_LOOP_CONTEXT: RuleTemplate = RuleTemplate {
    rule_id: "break_requires_loop_context",
    normalized_pattern: "break_requires_loop_context",
    repair_goal: "Keep `break;` inside a `while` or `for` loop, or replace it with control flow that is valid at the current scope.",
    summary: "`break;` only exits the nearest enclosing `while` or `for` loop.",
    pattern: "while (ready == false) { if (stop_now) { break; } }",
    minimal_example: "for (let i: i32 = 0; i < 3; i = i + 1) { if (i == 1) { break; } }",
    anti_pattern: Some("fn main() -> i32 { break; return 0; }"),
    default_fixit: "move `break;` into a loop body or use `return ...;` if you want to exit the function",
};

const RULE_CONTINUE_REQUIRES_LOOP_CONTEXT: RuleTemplate = RuleTemplate {
    rule_id: "continue_requires_loop_context",
    normalized_pattern: "continue_requires_loop_context",
    repair_goal: "Keep `continue;` inside a `while` or `for` loop so it skips only the next loop iteration.",
    summary: "`continue;` is only valid inside a loop body, where it jumps to the next iteration of the nearest loop.",
    pattern: "for (let i: i32 = 0; i < 3; i = i + 1) { if (i == 1) { continue; } println(i); }",
    minimal_example: "while (count < 3) { count = count + 1; if (count == 2) { continue; } println(count); }",
    anti_pattern: Some("fn main() -> i32 { continue; return 0; }"),
    default_fixit: "move `continue;` into a loop body or rewrite the surrounding control flow with `if` / `else`",
};

const RULE_MATCH_INPUT_MUST_USE_SUPPORTED_TYPE: RuleTemplate = RuleTemplate {
    rule_id: "match_input_must_use_supported_type",
    normalized_pattern: "match_input_must_use_supported_type",
    repair_goal: "Match only on supported AX value domains: `bool`, `i32`, `string`, structs, or enum values.",
    summary: "AX `match` currently supports boolean inputs, integer inputs, string inputs, full-field struct destructuring, and enum values.",
    pattern: "match (flag) { true => { return 1; } false => { return 0; } }",
    minimal_example: "match (status) { Status.Ready => { return 1; } _ => { return 0; } }",
    anti_pattern: Some("match (items) { _ => { return 1; } }"),
    default_fixit: "rewrite this branch with `if / else`, or match on a supported scalar, struct, or enum value",
};

const RULE_MATCH_PATTERN_MUST_MATCH_INPUT: RuleTemplate = RuleTemplate {
    rule_id: "match_pattern_must_match_input",
    normalized_pattern: "match_pattern_must_match_input",
    repair_goal: "Keep every `match` arm pattern in the same value domain as the matched input.",
    summary: "AX `match` patterns must align with the scrutinee type: `bool` uses `true`/`false`, `i32` uses integer literals, `string` uses string literals, structs use full-field shorthand destructuring, and enums use `EnumName.Variant`.",
    pattern: "match (flag) { true => { return 1; } false => { return 0; } }",
    minimal_example: "match (value) { 0 => { return 1; } _ => { return 2; } }",
    anti_pattern: Some("match (flag) { 0 => { return 1; } }"),
    default_fixit: "rewrite this arm pattern so it matches the same type as the input",
};

const RULE_MATCH_PATTERNS_MUST_BE_UNIQUE: RuleTemplate = RuleTemplate {
    rule_id: "match_patterns_must_be_unique",
    normalized_pattern: "match_patterns_must_be_unique",
    repair_goal: "Keep only one arm for each concrete `match` pattern.",
    summary: "Duplicate `match` patterns make later arms unreachable and should be merged or removed.",
    pattern: "match (value) { 0 => { return 1; } 1 => { return 2; } _ => { return 3; } }",
    minimal_example: "match (flag) { true => { return 1; } false => { return 0; } }",
    anti_pattern: Some("match (value) { 0 => { return 1; } 0 => { return 2; } }"),
    default_fixit: "remove the duplicate arm or merge its logic into the earlier arm",
};

const RULE_MATCH_WILDCARD_MUST_BE_LAST: RuleTemplate = RuleTemplate {
    rule_id: "match_wildcard_must_be_last",
    normalized_pattern: "match_wildcard_must_be_last",
    repair_goal: "Place at most one `_` arm at the end of the `match`.",
    summary: "The catch-all `_` arm in AX `match` is a final fallback and cannot appear before later arms.",
    pattern: "match (value) { 0 => { return 1; } _ => { return 2; } }",
    minimal_example: "match (flag) { true => { return 1; } _ => { return 0; } }",
    anti_pattern: Some("match (value) { _ => { return 1; } 0 => { return 2; } }"),
    default_fixit: "move the `_` arm to the end or remove the extra wildcard arm",
};

const RULE_MATCH_MUST_BE_EXHAUSTIVE: RuleTemplate = RuleTemplate {
    rule_id: "match_must_be_exhaustive",
    normalized_pattern: "match_must_be_exhaustive",
    repair_goal: "Cover every remaining input case before the `match` can compile.",
    summary: "AX `match` must be exhaustive: `bool` needs both values, enums need every variant, and `i32` currently needs a final `_` arm.",
    pattern: "match (flag) { true => { return 1; } false => { return 0; } }",
    minimal_example: "match (state) { State.Ready => { return 1; } State.Done => { return 2; } }",
    anti_pattern: Some("match (flag) { true => { return 1; } }"),
    default_fixit: "add the missing arm(s) or finish the `match` with `_ => { ... }`",
};

const RULE_MATCH_REQUIRES_CONCRETE_PATTERN: RuleTemplate = RuleTemplate {
    rule_id: "match_requires_concrete_pattern",
    normalized_pattern: "match_requires_concrete_pattern",
    repair_goal: "Start each `match` with at least one concrete literal or enum-variant arm.",
    summary: "AX uses the concrete arms to establish the typed branch set, so a wildcard-only `match` is rejected.",
    pattern: "match (value) { 0 => { return 1; } _ => { return 2; } }",
    minimal_example: "match (flag) { true => { return 1; } false => { return 0; } }",
    anti_pattern: Some("match (value) { _ => { return 1; } }"),
    default_fixit: "add a concrete pattern before `_`, or replace the `match` with a normal block",
};

const RULE_MATCH_EXPRESSION_ARMS_MUST_SHARE_TYPE: RuleTemplate = RuleTemplate {
    rule_id: "match_expression_arms_must_share_type",
    normalized_pattern: "match_expression_arms_must_share_type",
    repair_goal: "Rewrite every `match` expression arm so they all produce the same type.",
    summary: "AX `match` expressions are typed expressions, so every arm must evaluate to one shared result type.",
    pattern: "let label: string = match (flag) { true => \"on\", false => \"off\" };",
    minimal_example: "let code: i32 = match (ready) { true => 1, false => 0 };",
    anti_pattern: Some("let value: i32 = match (flag) { true => 1, false => \"off\" };"),
    default_fixit: "change the mismatching arm so it returns the same type as the other match-expression arms",
};

const RULE_MATCH_ENUM_VARIANT_PAYLOAD_MUST_MATCH_DECLARATION: RuleTemplate = RuleTemplate {
    rule_id: "match_enum_variant_payload_must_match_declaration",
    normalized_pattern: "match_enum_variant_payload_must_match_declaration",
    repair_goal: "Match payload enum variants using the payload shape declared on the enum variant.",
    summary: "Payload enum variants must be matched as `EnumName.Variant(name)` or `EnumName.Variant(_)`, while unit variants stay as bare `EnumName.Variant`.",
    pattern: "match (result) { Result.Ok(value) => value, Result.Err(_) => 0 }",
    minimal_example: "enum Result { Ok(i32), Err(string) }",
    anti_pattern: Some("match (result) { Result.Ok => 1, Result.Err(message) => 0 }"),
    default_fixit: "rewrite the match arm so its payload binding or `_` exactly matches the enum variant declaration",
};

const RULE_MATCH_STRUCT_PATTERN_MUST_MATCH_DECLARATION: RuleTemplate = RuleTemplate {
    rule_id: "match_struct_pattern_must_match_declaration",
    normalized_pattern: "match_struct_pattern_must_match_declaration",
    repair_goal: "Keep struct destructuring patterns aligned with the matched struct declaration.",
    summary: "AX struct match patterns use full-field shorthand destructuring in this slice: every declared field appears once, and each field name becomes an arm-local binding.",
    pattern: "match (point) { Point { x, y } => { return x + y; } }",
    minimal_example: "let score: i32 = match (point) { Point { x, y } => x + y, };",
    anti_pattern: Some("match (point) { Point { x } => x, }"),
    default_fixit: "rewrite the struct pattern so it lists each declared field exactly once using shorthand field names",
};

const RULE_MATCH_GUARD_MUST_BE_BOOL: RuleTemplate = RuleTemplate {
    rule_id: "match_guard_must_be_bool",
    normalized_pattern: "match_guard_must_be_bool",
    repair_goal: "Rewrite the `if` guard on the match arm so it evaluates to `bool`.",
    summary: "AX match guards are boolean filters: `pattern if condition => ...` only accepts a `bool` condition.",
    pattern: "match (token) { Token.Number(value) if value > 9 => 10, _ => 0 }",
    minimal_example: "match (value) { 400..=499 if value != 418 => 4, _ => 0 }",
    anti_pattern: Some("match (value) { 1 if 1 => 10, _ => 0 }"),
    default_fixit: "replace the guard expression with a comparison or boolean expression",
};

const RULE_MATCH_RANGE_MUST_BE_NON_EMPTY: RuleTemplate = RuleTemplate {
    rule_id: "match_range_must_be_non_empty",
    normalized_pattern: "match_range_must_be_non_empty",
    repair_goal: "Rewrite the inclusive `i32` range pattern so the start bound is less than or equal to the end bound.",
    summary: "AX range patterns use inclusive `start..=end` syntax and cannot represent an empty interval.",
    pattern: "match (status) { 400..=499 => 4, _ => 0 }",
    minimal_example: "match (exit_code) { 0..=0 => 0, 1..=255 => 1, _ => 2 }",
    anti_pattern: Some("match (status) { 499..=400 => 4, _ => 0 }"),
    default_fixit: "swap the range bounds or change them to a non-empty inclusive interval",
};

const RULE_BLOCK_MATCH_ARM_MUST_STAY_LINEAR: RuleTemplate = RuleTemplate {
    rule_id: "block_match_arm_must_stay_linear",
    normalized_pattern: "block_match_arm_must_stay_linear",
    repair_goal: "Keep block-valued match expression arms as local linear preparation followed by one final value expression.",
    summary: "AX block-valued match expression arms currently allow `let`, assignment, expression statements, and nested linear blocks before the final expression; control flow belongs outside this arm form.",
    pattern: "match (flag) { true => { let base: i32 = 1; base + 1 }, false => 0 }",
    minimal_example: "pattern => { let base: i32 = 1; base + 1 }",
    anti_pattern: Some("pattern => { if (flag) { println(1); } 1 }"),
    default_fixit: "move `if` / loops / return / break / continue outside the block-valued arm, or rewrite the arm as a simple expression",
};

const RULE_ENUM_VARIANT_PAYLOAD_MUST_MATCH_DECLARATION: RuleTemplate = RuleTemplate {
    rule_id: "enum_variant_payload_must_match_declaration",
    normalized_pattern: "enum_variant_payload_must_match_declaration",
    repair_goal: "Construct the enum variant using the payload shape declared on that variant.",
    summary: "Unit enum variants are bare values like `Flag.On`, while payload enum variants are constructed as `EnumName.Variant(value)`.",
    pattern: "let result: Result = Result.Ok(7);",
    minimal_example: "enum Result { Ok(i32), Err(string) }",
    anti_pattern: Some("let result: Result = Result.Ok;"),
    default_fixit: "either add the required payload argument or remove `(...)` when the variant is unit-like",
};

const RULE_ENUM_VARIANT_PAYLOAD_TYPE_MUST_MATCH_DECLARATION: RuleTemplate = RuleTemplate {
    rule_id: "enum_variant_payload_type_must_match_declaration",
    normalized_pattern: "enum_variant_payload_type_must_match_declaration",
    repair_goal: "Pass a payload value whose type matches the enum variant declaration.",
    summary: "The payload argument for `EnumName.Variant(value)` must use the type declared on that enum variant.",
    pattern: "enum Result { Ok(i32) } fn main() -> i32 { let result: Result = Result.Ok(7); return 0; }",
    minimal_example: "let result: Result = Result.Ok(1);",
    anti_pattern: Some("let result: Result = Result.Ok(true);"),
    default_fixit: "rewrite the payload expression so it produces the variant's declared payload type",
};

const RULE_TRAIT_REFERENCE_MUST_RESOLVE: RuleTemplate = RuleTemplate {
    rule_id: "trait_reference_must_resolve",
    normalized_pattern: "trait_reference_must_resolve",
    repair_goal: "Reference a trait that is declared and visible before using it in an impl or generic bound.",
    summary: "AX trait references must point at a declared trait; generic bounds like `T: Label` and `impl Label for Type` cannot invent traits implicitly.",
    pattern: "trait Label { fn label(self: Self) -> string; }\nfn render<T: Label>(value: T) -> string { return value.label(); }",
    minimal_example: "trait Named { fn name(self: Self) -> string; }",
    anti_pattern: Some("fn render<T: MissingTrait>(value: T) -> T { return value; }"),
    default_fixit: "declare the missing trait, import the module that owns it, or change the reference to an existing trait",
};

const RULE_TRAIT_BOUND_MUST_BE_SATISFIED: RuleTemplate = RuleTemplate {
    rule_id: "trait_bound_must_be_satisfied",
    normalized_pattern: "trait_bound_must_be_satisfied",
    repair_goal: "Pass a value whose type implements the required trait, or add the missing `impl Trait for Type` block.",
    summary: "AX generic function bounds are checked at the call site, so `fn render<T: Label>(value: T)` only accepts types with `impl Label for ThatType`.",
    pattern: "impl Label for Command { fn label(self: Command) -> string { return self.name; } }",
    minimal_example: "fn render<T: Label>(value: T) -> string { return value.label(); }",
    anti_pattern: Some("render(1)"),
    default_fixit: "add the required trait impl for the concrete type or call the generic function with an implementing value",
};

const RULE_RESULT_PROPAGATION_REQUIRES_RESULT: RuleTemplate = RuleTemplate {
    rule_id: "result_propagation_requires_result",
    normalized_pattern: "result_propagation_requires_result",
    repair_goal: "Use `?` only when the expression and current function both use compatible `Result<T, E>` types.",
    summary: "`expr?` unwraps `Result.Ok(value)` and returns `Result.Err(error)` from the current function, so both sides need compatible `Result` error types.",
    pattern: "fn load() -> std.result.Result<string, string> { let text: string = std.fs.try_read_to_string(\"input.txt\")?; return std.result.Result.ok(text); }",
    minimal_example: "let text: string = read_text()?;",
    anti_pattern: Some("fn main() -> i32 { let text: string = read_text()?; return 0; }"),
    default_fixit: "return a compatible `Result<_, E>` from the function or replace `?` with an explicit `match`",
};

const RULE_NON_EMPTY_ARRAY_LITERAL_REQUIRED: RuleTemplate = RuleTemplate {
    rule_id: "non_empty_array_literal_required",
    normalized_pattern: "non_empty_array_literal_required",
    repair_goal: "Either give `[]` a zero-length array context like `[i32; 0]`, or add elements so the array has a concrete non-zero length.",
    summary: "AX accepts `[]` only when the surrounding context fixes it to a length-0 array type such as `[i32; 0]`.",
    pattern: "let values: [i32; 3] = [1, 2, 3];",
    minimal_example: "let values: [i32; 0] = [];",
    anti_pattern: Some("let values: [i32; 1] = [];"),
    default_fixit: "change the surrounding type to `[Type; 0]` or add elements to the array literal",
};

const RULE_INDEX_BASE_MUST_BE_ARRAY: RuleTemplate = RuleTemplate {
    rule_id: "index_base_must_be_array",
    normalized_pattern: "index_base_must_be_array",
    repair_goal: "Use `expr[index]` only when the base expression evaluates to an array or slice for reads, or to a mutable array for writes.",
    summary: "AX indexing with `[]` reads from arrays and slice views, but only mutable arrays can be write targets.",
    pattern: "let value: i32 = values[0];",
    minimal_example: "let mut values: [i32; 2] = [1, 2]; values[1] = values[0];",
    anti_pattern: Some("let value: i32 = number[0];"),
    default_fixit: "index into an array value like `values[0]`",
};

const RULE_ARRAY_INDEX_MUST_BE_I32: RuleTemplate = RuleTemplate {
    rule_id: "array_index_must_be_i32",
    normalized_pattern: "array_index_must_be_i32",
    repair_goal: "Rewrite the index expression so it produces an `i32` value.",
    summary: "AX array and slice indexing accepts only `i32` index expressions before runtime bounds checks run.",
    pattern: "let value: i32 = values[index];",
    minimal_example: "let index: i32 = 1; return values[index];",
    anti_pattern: Some("return values[true];"),
    default_fixit: "change the index expression to an `i32` value",
};

const RULE_SLICE_BASE_MUST_BE_ARRAY_OR_SLICE: RuleTemplate = RuleTemplate {
    rule_id: "slice_base_must_be_array_or_slice",
    normalized_pattern: "slice_base_must_be_array_or_slice",
    repair_goal: "Use `base[start:end]` only when `base` is already an array or slice value.",
    summary: "AX slice expressions create read-only views from arrays or existing slices; scalars and structs cannot be sliced.",
    pattern: "let window: [i32] = values[1:3];",
    minimal_example: "let values: [i32; 4] = [1, 2, 3, 4]; let head: [i32] = values[0:2];",
    anti_pattern: Some("let window: [i32] = count[0:1];"),
    default_fixit: "slice an array or slice value instead of a scalar or struct",
};

const RULE_SLICE_VALUES_ARE_READ_ONLY: RuleTemplate = RuleTemplate {
    rule_id: "slice_values_are_read_only",
    normalized_pattern: "slice_values_are_read_only",
    repair_goal: "Write through the original mutable array instead of trying to assign through a slice view.",
    summary: "Current AX slices are read-only views, so `slice[index] = expr;` is not allowed even if the slice binding itself is `mut`.",
    pattern: "let window: [i32] = values[0:2]; println(window[0]);",
    minimal_example: "let mut values: [i32; 3] = [1, 2, 3]; values[0] = 9;",
    anti_pattern: Some("let mut window: [i32] = values[0:2]; window[0] = 9;"),
    default_fixit: "rewrite the assignment to target the original mutable array",
};

const RULE_LEN_BUILTIN_REQUIRES_COUNTABLE_VALUE: RuleTemplate = RuleTemplate {
    rule_id: "len_builtin_requires_countable_value",
    normalized_pattern: "len_builtin_requires_countable_value",
    repair_goal: "Call `len(value)` only with a `string`, `string_list`, fixed-size array, or slice value.",
    summary: "AX uses `len(value)` as the unified length helper for strings, `string_list`, and sequence-like values that already have a stable length in the prototype.",
    pattern: "let size: i32 = len(values);",
    minimal_example: "let values: [i32; 3] = [1, 2, 3]; return len(values);",
    anti_pattern: Some("return len(true);"),
    default_fixit: "pass a string, string_list, array, or slice to `len(...)`",
};

const RULE_MAIN_REQUIRED: RuleTemplate = RuleTemplate {
    rule_id: "main_function_required",
    normalized_pattern: "main_function_required",
    repair_goal: "Add a valid `main` entrypoint so the current AX program is runnable.",
    summary: "Every runnable AX program must define `fn main() -> i32`.",
    pattern: "fn main() -> i32 { return 0; }",
    minimal_example: "fn main() -> i32 { return 0; }",
    anti_pattern: None,
    default_fixit: "add `fn main() -> i32 { return 0; }`",
};

const RULE_MAIN_SIGNATURE: RuleTemplate = RuleTemplate {
    rule_id: "main_signature_fixed",
    normalized_pattern: "main_signature_fixed",
    repair_goal: "Change `main` so it takes no parameters and returns `i32`.",
    summary: "The current AX prototype requires `main` to use the fixed signature `fn main() -> i32`.",
    pattern: "fn main() -> i32 { return 0; }",
    minimal_example: "fn main() -> i32 { return 0; }",
    anti_pattern: Some("fn main(value: i32) -> bool { return false; }"),
    default_fixit: "rewrite `main` to `fn main() -> i32 { ... }`",
};

const RULE_FUNCTION_ARGUMENT_TYPE_MUST_MATCH: RuleTemplate = RuleTemplate {
    rule_id: "function_argument_type_must_match",
    normalized_pattern: "function_argument_type_must_match",
    repair_goal: "Make each call argument produce the exact type declared by the target parameter.",
    summary: "AX checks every call argument against the function signature and does not coerce argument types.",
    pattern: "fn add(value: i32) -> i32 { return value; }",
    minimal_example: "fn main() -> i32 { return add(1); }",
    anti_pattern: Some("fn main() -> i32 { return add(true); }"),
    default_fixit: "change the argument expression or parameter type so the call matches the function signature",
};

const RULE_RETURN_VALUE_MUST_MATCH_DECLARED_TYPE: RuleTemplate = RuleTemplate {
    rule_id: "return_value_must_match_declared_type",
    normalized_pattern: "return_value_must_match_declared_type",
    repair_goal: "Return a value whose type matches the function's declared return type.",
    summary: "AX checks every `return` statement against the declared function return type and does not coerce values.",
    pattern: "fn main() -> i32 { return 0; }",
    minimal_example: "fn ready() -> bool { return true; }",
    anti_pattern: Some("fn main() -> i32 { return false; }"),
    default_fixit: "change the returned expression or the function return type so they match exactly",
};

const RULE_CONDITION_MUST_BE_BOOL: RuleTemplate = RuleTemplate {
    rule_id: "condition_expression_must_be_bool",
    normalized_pattern: "condition_expression_must_be_bool",
    repair_goal: "Make the condition expression evaluate to `bool`.",
    summary: "AX does not coerce integers, strings, or other values into `if`, `while`, or `for` conditions.",
    pattern: "if (count < limit) { return 1; }",
    minimal_example: "while (index < len) { index = index + 1; }",
    anti_pattern: Some("if (1) { return 0; }"),
    default_fixit: "rewrite the condition as a boolean comparison or boolean variable",
};

const RULE_TYPE_MISMATCH: RuleTemplate = RuleTemplate {
    rule_id: "type_match_required",
    normalized_pattern: "type_match_required",
    repair_goal: "Change the expression or the declared type so both sides use the same AX type.",
    summary: "AX requires assignments, arguments, returns, and conditions to use the declared type exactly.",
    pattern: "let value: i32 = 1;",
    minimal_example: "fn add(value: i32) -> i32 { return value; }",
    anti_pattern: Some("let value: bool = 1;"),
    default_fixit: "make the expression and the expected AX type agree",
};

const RULE_MISSING_RETURN: RuleTemplate = RuleTemplate {
    rule_id: "all_paths_must_return",
    normalized_pattern: "all_paths_must_return",
    repair_goal: "Make every control-flow path return a value of the declared function type.",
    summary: "Functions with a non-void return type must return a value on every control-flow path.",
    pattern: "fn helper(flag: bool) -> i32 { if (flag) { return 1; } return 0; }",
    minimal_example: "fn helper(flag: bool) -> i32 { if (flag) { return 1; } return 0; }",
    anti_pattern: Some("fn helper(flag: bool) -> i32 { if (flag) { return 1; } }"),
    default_fixit: "add a `return ...;` on the missing control-flow path",
};

const RULE_IMMUTABLE_ASSIGNMENT: RuleTemplate = RuleTemplate {
    rule_id: "mutable_binding_required",
    normalized_pattern: "mutable_binding_required",
    repair_goal: "Either declare the binding with `let mut` or stop assigning to it.",
    summary: "AX bindings are immutable unless they are declared with `let mut`.",
    pattern: "let mut value: i32 = 0; value = value + 1;",
    minimal_example: "let mut value: i32 = 0; value = value + 1;",
    anti_pattern: Some("let value: i32 = 0; value = 1;"),
    default_fixit: "change the declaration to `let mut ...` or remove the assignment",
};

const RULE_UNDEFINED_VARIABLE: RuleTemplate = RuleTemplate {
    rule_id: "variable_must_be_declared_in_scope",
    normalized_pattern: "variable_must_be_declared_in_scope",
    repair_goal: "Introduce a declaration in scope before using the variable.",
    summary: "AX requires variables to be declared before use within the current scope.",
    pattern: "let value: i32 = 1; println(value);",
    minimal_example: "let total: i32 = 1; println(total);",
    anti_pattern: Some("println(total);"),
    default_fixit: "declare the variable before this use",
};
