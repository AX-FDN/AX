use super::RuleTemplate;

pub(super) const RULE_FOR_HEADER_CLAUSE_SUPPORTED: RuleTemplate = RuleTemplate {
    rule_id: "for_header_clause_supported",
    normalized_pattern: "for_header_clause_supported",
    repair_goal: "Rewrite the `for` header so each clause is a `let`, assignment, or expression.",
    summary: "The current AX `for` prototype only supports `let`, assignment, or expression clauses.",
    pattern: "for (let i: i32 = 0; i < 3; i = i + 1) { println(i); }",
    minimal_example: "for (let i: i32 = 0; i < 3; i = i + 1) { return i; }",
    anti_pattern: Some("for (return 0; true; step()) { }"),
    default_fixit: "rewrite the header using only `let`, assignment, or expression clauses",
};

pub(super) const RULE_FOR_IN_REQUIRES_SEQUENCE_VALUE: RuleTemplate = RuleTemplate {
    rule_id: "for_in_requires_array_or_slice",
    normalized_pattern: "for_in_requires_array_or_slice",
    repair_goal: "Iterate over an array or slice value, or rewrite the loop as an indexed `for (...)` loop.",
    summary: "The first AX `for in` prototype only iterates `[T; N]` arrays and `[T]` slices.",
    pattern: "for (let value: i32 in values) { println(value); }",
    minimal_example: "let values: [i32; 3] = [1, 2, 3];",
    anti_pattern: Some("for (let ch: string in message) { println(ch); }"),
    default_fixit: "change the iterated value to an array or slice, or fall back to an indexed `for (...)` loop",
};

pub(super) const RULE_FOR_IN_BINDING_MUST_MATCH_ELEMENT_TYPE: RuleTemplate = RuleTemplate {
    rule_id: "for_in_binding_must_match_element_type",
    normalized_pattern: "for_in_binding_must_match_element_type",
    repair_goal: "Declare the loop variable with the iterable's element type.",
    summary: "AX `for in` loop variables must use the same element type as the array or slice being iterated.",
    pattern: "for (let value: i32 in values) { println(value); }",
    minimal_example: "let names: [string; 2] = [\"a\", \"b\"];",
    anti_pattern: Some("for (let value: bool in values) { println(value); }"),
    default_fixit: "change the loop variable type so it matches the iterated element type",
};

pub(super) const RULE_BREAK_REQUIRES_LOOP_CONTEXT: RuleTemplate = RuleTemplate {
    rule_id: "break_requires_loop_context",
    normalized_pattern: "break_requires_loop_context",
    repair_goal: "Keep `break;` inside a `while` or `for` loop, or replace it with control flow that is valid at the current scope.",
    summary: "`break;` only exits the nearest enclosing `while` or `for` loop.",
    pattern: "while (ready == false) { if (stop_now) { break; } }",
    minimal_example: "for (let i: i32 = 0; i < 3; i = i + 1) { if (i == 1) { break; } }",
    anti_pattern: Some("fn main() -> i32 { break; return 0; }"),
    default_fixit: "move `break;` into a loop body or use `return ...;` if you want to exit the function",
};

pub(super) const RULE_CONTINUE_REQUIRES_LOOP_CONTEXT: RuleTemplate = RuleTemplate {
    rule_id: "continue_requires_loop_context",
    normalized_pattern: "continue_requires_loop_context",
    repair_goal: "Keep `continue;` inside a `while` or `for` loop so it skips only the next loop iteration.",
    summary: "`continue;` is only valid inside a loop body, where it jumps to the next iteration of the nearest loop.",
    pattern: "for (let i: i32 = 0; i < 3; i = i + 1) { if (i == 1) { continue; } println(i); }",
    minimal_example: "while (count < 3) { count = count + 1; if (count == 2) { continue; } println(count); }",
    anti_pattern: Some("fn main() -> i32 { continue; return 0; }"),
    default_fixit: "move `continue;` into a loop body or rewrite the surrounding control flow with `if` / `else`",
};

pub(super) const RULE_MATCH_INPUT_MUST_USE_SUPPORTED_TYPE: RuleTemplate = RuleTemplate {
    rule_id: "match_input_must_use_supported_type",
    normalized_pattern: "match_input_must_use_supported_type",
    repair_goal: "Match only on supported AX value domains: `bool`, `i32`, `string`, structs, or enum values.",
    summary: "AX `match` currently supports boolean inputs, integer inputs, string inputs, full-field struct destructuring, and enum values.",
    pattern: "match (flag) { true => { return 1; } false => { return 0; } }",
    minimal_example: "match (status) { Status.Ready => { return 1; } _ => { return 0; } }",
    anti_pattern: Some("match (items) { _ => { return 1; } }"),
    default_fixit: "rewrite this branch with `if / else`, or match on a supported scalar, struct, or enum value",
};

pub(super) const RULE_MATCH_PATTERN_MUST_MATCH_INPUT: RuleTemplate = RuleTemplate {
    rule_id: "match_pattern_must_match_input",
    normalized_pattern: "match_pattern_must_match_input",
    repair_goal: "Keep every `match` arm pattern in the same value domain as the matched input.",
    summary: "AX `match` patterns must align with the scrutinee type: `bool` uses `true`/`false`, `i32` uses integer literals, `string` uses string literals, structs use full-field shorthand destructuring, and enums use `EnumName.Variant`.",
    pattern: "match (flag) { true => { return 1; } false => { return 0; } }",
    minimal_example: "match (value) { 0 => { return 1; } _ => { return 2; } }",
    anti_pattern: Some("match (flag) { 0 => { return 1; } }"),
    default_fixit: "rewrite this arm pattern so it matches the same type as the input",
};

pub(super) const RULE_MATCH_PATTERNS_MUST_BE_UNIQUE: RuleTemplate = RuleTemplate {
    rule_id: "match_patterns_must_be_unique",
    normalized_pattern: "match_patterns_must_be_unique",
    repair_goal: "Keep only one arm for each concrete `match` pattern.",
    summary: "Duplicate `match` patterns make later arms unreachable and should be merged or removed.",
    pattern: "match (value) { 0 => { return 1; } 1 => { return 2; } _ => { return 3; } }",
    minimal_example: "match (flag) { true => { return 1; } false => { return 0; } }",
    anti_pattern: Some("match (value) { 0 => { return 1; } 0 => { return 2; } }"),
    default_fixit: "remove the duplicate arm or merge its logic into the earlier arm",
};

pub(super) const RULE_MATCH_WILDCARD_MUST_BE_LAST: RuleTemplate = RuleTemplate {
    rule_id: "match_wildcard_must_be_last",
    normalized_pattern: "match_wildcard_must_be_last",
    repair_goal: "Place at most one `_` arm at the end of the `match`.",
    summary: "The catch-all `_` arm in AX `match` is a final fallback and cannot appear before later arms.",
    pattern: "match (value) { 0 => { return 1; } _ => { return 2; } }",
    minimal_example: "match (flag) { true => { return 1; } _ => { return 0; } }",
    anti_pattern: Some("match (value) { _ => { return 1; } 0 => { return 2; } }"),
    default_fixit: "move the `_` arm to the end or remove the extra wildcard arm",
};

pub(super) const RULE_MATCH_MUST_BE_EXHAUSTIVE: RuleTemplate = RuleTemplate {
    rule_id: "match_must_be_exhaustive",
    normalized_pattern: "match_must_be_exhaustive",
    repair_goal: "Cover every remaining input case before the `match` can compile.",
    summary: "AX `match` must be exhaustive: `bool` needs both values, enums need every variant, and `i32` currently needs a final `_` arm.",
    pattern: "match (flag) { true => { return 1; } false => { return 0; } }",
    minimal_example: "match (state) { State.Ready => { return 1; } State.Done => { return 2; } }",
    anti_pattern: Some("match (flag) { true => { return 1; } }"),
    default_fixit: "add the missing arm(s) or finish the `match` with `_ => { ... }`",
};

pub(super) const RULE_MATCH_REQUIRES_CONCRETE_PATTERN: RuleTemplate = RuleTemplate {
    rule_id: "match_requires_concrete_pattern",
    normalized_pattern: "match_requires_concrete_pattern",
    repair_goal: "Start each `match` with at least one concrete literal or enum-variant arm.",
    summary: "AX uses the concrete arms to establish the typed branch set, so a wildcard-only `match` is rejected.",
    pattern: "match (value) { 0 => { return 1; } _ => { return 2; } }",
    minimal_example: "match (flag) { true => { return 1; } false => { return 0; } }",
    anti_pattern: Some("match (value) { _ => { return 1; } }"),
    default_fixit: "add a concrete pattern before `_`, or replace the `match` with a normal block",
};

pub(super) const RULE_MATCH_EXPRESSION_ARMS_MUST_SHARE_TYPE: RuleTemplate = RuleTemplate {
    rule_id: "match_expression_arms_must_share_type",
    normalized_pattern: "match_expression_arms_must_share_type",
    repair_goal: "Rewrite every `match` expression arm so they all produce the same type.",
    summary: "AX `match` expressions are typed expressions, so every arm must evaluate to one shared result type.",
    pattern: "let label: string = match (flag) { true => \"on\", false => \"off\" };",
    minimal_example: "let code: i32 = match (ready) { true => 1, false => 0 };",
    anti_pattern: Some("let value: i32 = match (flag) { true => 1, false => \"off\" };"),
    default_fixit: "change the mismatching arm so it returns the same type as the other match-expression arms",
};

pub(super) const RULE_MATCH_ENUM_VARIANT_PAYLOAD_MUST_MATCH_DECLARATION: RuleTemplate =
    RuleTemplate {
        rule_id: "match_enum_variant_payload_must_match_declaration",
        normalized_pattern: "match_enum_variant_payload_must_match_declaration",
        repair_goal: "Match payload enum variants using the payload shape declared on the enum variant.",
        summary: "Payload enum variants must be matched as `EnumName.Variant(name)` or `EnumName.Variant(_)`, while unit variants stay as bare `EnumName.Variant`.",
        pattern: "match (result) { Result.Ok(value) => value, Result.Err(_) => 0 }",
        minimal_example: "enum Result { Ok(i32), Err(string) }",
        anti_pattern: Some("match (result) { Result.Ok => 1, Result.Err(message) => 0 }"),
        default_fixit: "rewrite the match arm so its payload binding or `_` exactly matches the enum variant declaration",
    };

pub(super) const RULE_MATCH_STRUCT_PATTERN_MUST_MATCH_DECLARATION: RuleTemplate = RuleTemplate {
    rule_id: "match_struct_pattern_must_match_declaration",
    normalized_pattern: "match_struct_pattern_must_match_declaration",
    repair_goal: "Keep struct destructuring patterns aligned with the matched struct declaration.",
    summary: "AX struct match patterns use full-field shorthand destructuring in this slice: every declared field appears once, and each field name becomes an arm-local binding.",
    pattern: "match (point) { Point { x, y } => { return x + y; } }",
    minimal_example: "let score: i32 = match (point) { Point { x, y } => x + y, };",
    anti_pattern: Some("match (point) { Point { x } => x, }"),
    default_fixit: "rewrite the struct pattern so it lists each declared field exactly once using shorthand field names",
};

pub(super) const RULE_MATCH_GUARD_MUST_BE_BOOL: RuleTemplate = RuleTemplate {
    rule_id: "match_guard_must_be_bool",
    normalized_pattern: "match_guard_must_be_bool",
    repair_goal: "Rewrite the `if` guard on the match arm so it evaluates to `bool`.",
    summary: "AX match guards are boolean filters: `pattern if condition => ...` only accepts a `bool` condition.",
    pattern: "match (token) { Token.Number(value) if value > 9 => 10, _ => 0 }",
    minimal_example: "match (value) { 400..=499 if value != 418 => 4, _ => 0 }",
    anti_pattern: Some("match (value) { 1 if 1 => 10, _ => 0 }"),
    default_fixit: "replace the guard expression with a comparison or boolean expression",
};

pub(super) const RULE_MATCH_RANGE_MUST_BE_NON_EMPTY: RuleTemplate = RuleTemplate {
    rule_id: "match_range_must_be_non_empty",
    normalized_pattern: "match_range_must_be_non_empty",
    repair_goal: "Rewrite the inclusive `i32` range pattern so the start bound is less than or equal to the end bound.",
    summary: "AX range patterns use inclusive `start..=end` syntax and cannot represent an empty interval.",
    pattern: "match (status) { 400..=499 => 4, _ => 0 }",
    minimal_example: "match (exit_code) { 0..=0 => 0, 1..=255 => 1, _ => 2 }",
    anti_pattern: Some("match (status) { 499..=400 => 4, _ => 0 }"),
    default_fixit: "swap the range bounds or change them to a non-empty inclusive interval",
};

pub(super) const RULE_BLOCK_MATCH_ARM_MUST_STAY_LINEAR: RuleTemplate = RuleTemplate {
    rule_id: "block_match_arm_must_stay_linear",
    normalized_pattern: "block_match_arm_must_stay_linear",
    repair_goal: "Keep block-valued match expression arms as local linear preparation followed by one final value expression.",
    summary: "AX block-valued match expression arms currently allow `let`, assignment, expression statements, and nested linear blocks before the final expression; control flow belongs outside this arm form.",
    pattern: "match (flag) { true => { let base: i32 = 1; base + 1 }, false => 0 }",
    minimal_example: "pattern => { let base: i32 = 1; base + 1 }",
    anti_pattern: Some("pattern => { if (flag) { println(1); } 1 }"),
    default_fixit: "move `if` / loops / return / break / continue outside the block-valued arm, or rewrite the arm as a simple expression",
};
