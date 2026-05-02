use super::RuleTemplate;

pub(super) const RULE_UNIQUE_DEFINITION_REQUIRED: RuleTemplate = RuleTemplate {
    rule_id: "unique_definition_required",
    normalized_pattern: "unique_definition_required",
    repair_goal: "Rename one definition or remove the duplicate so each name is declared once.",
    summary: "Each AX name may only be defined once in the same scope or top-level namespace.",
    pattern: "let total: i32 = 1;",
    minimal_example: "fn helper() -> i32 { return 0; }",
    anti_pattern: Some("let total: i32 = 1; let total: i32 = 2;"),
    default_fixit: "rename or remove the duplicate definition",
};

pub(super) const RULE_TYPE_MUST_BE_DECLARED: RuleTemplate = RuleTemplate {
    rule_id: "type_must_be_declared",
    normalized_pattern: "type_must_be_declared",
    repair_goal: "Use a builtin type or declare the referenced type before using it.",
    summary: "AX type references must resolve to a builtin type or a previously declared `struct` or `enum`.",
    pattern: "struct Point { x: i32, y: i32 }",
    minimal_example: "let point: Point = Point { x: 1, y: 2 };",
    anti_pattern: Some("let point: Missing = 1;"),
    default_fixit: "declare the missing type or replace it with an existing AX type",
};

pub(super) const RULE_FUNCTION_MUST_BE_DECLARED: RuleTemplate = RuleTemplate {
    rule_id: "function_must_be_declared",
    normalized_pattern: "function_must_be_declared",
    repair_goal: "Declare the function first or change the call to a function that exists.",
    summary: "AX function calls must target a declared function or builtin.",
    pattern: "fn helper() -> i32 { return 0; }",
    minimal_example: "fn main() -> i32 { return helper(); }",
    anti_pattern: Some("fn main() -> i32 { return missing(); }"),
    default_fixit: "declare the missing function or fix the call name",
};

pub(super) const RULE_ASSIGNMENT_TARGET_REQUIRED: RuleTemplate = RuleTemplate {
    rule_id: "writable_assignment_target_required",
    normalized_pattern: "writable_assignment_target_required",
    repair_goal: "Assign only to a mutable variable, a direct mutable struct field, or a direct mutable array element.",
    summary: "AX assignments can only write to `name = expr;`, `struct_value.field = expr;`, or `array_value[index] = expr;` targets that are writable.",
    pattern: "value = 1;",
    minimal_example: "values[0] = 1;",
    anti_pattern: Some("(left + right) = 1;"),
    default_fixit: "rewrite the assignment to target a writable variable, direct field, or direct array element",
};

pub(super) const RULE_FUNCTION_NAME_NOT_RUNTIME_VALUE: RuleTemplate = RuleTemplate {
    rule_id: "function_name_not_runtime_value",
    normalized_pattern: "function_name_not_runtime_value",
    repair_goal: "Call the function with parentheses or replace it with a real runtime value.",
    summary: "Function names are not first-class runtime values in the current AX prototype.",
    pattern: "let total: i32 = helper();",
    minimal_example: "println(helper());",
    anti_pattern: Some("let total: i32 = helper;"),
    default_fixit: "add parentheses to call the function or use a different value",
};

pub(super) const RULE_FUNCTION_ARGUMENT_COUNT_MATCH: RuleTemplate = RuleTemplate {
    rule_id: "function_argument_count_must_match",
    normalized_pattern: "function_argument_count_must_match",
    repair_goal: "Pass exactly the number of arguments declared by the function signature.",
    summary: "AX does not support optional or implicit arguments; function calls must match arity exactly.",
    pattern: "add(left, right)",
    minimal_example: "fn add(left: i32, right: i32) -> i32 { return left + right; }",
    anti_pattern: Some("add(left)"),
    default_fixit: "add or remove arguments so the call arity matches the function signature",
};

pub(super) const RULE_CALL_TARGET_MUST_BE_FUNCTION_NAME: RuleTemplate = RuleTemplate {
    rule_id: "call_target_must_be_function_name",
    normalized_pattern: "call_target_must_be_function_name",
    repair_goal: "Change this call so its target is a declared function name or builtin.",
    summary: "The current AX prototype only supports direct calls to function names and builtins.",
    pattern: "helper(value)",
    minimal_example: "println(value);",
    anti_pattern: Some("value(arg)"),
    default_fixit: "replace the call target with a declared function name",
};

pub(super) const RULE_TYPE_NAME_NOT_RUNTIME_VALUE: RuleTemplate = RuleTemplate {
    rule_id: "type_name_not_runtime_value",
    normalized_pattern: "type_name_not_runtime_value",
    repair_goal: "Replace the type name with a constructed value or enum variant.",
    summary: "Type names only belong in type positions, not as runtime expressions.",
    pattern: "let point: Point = Point { x: 1, y: 2 };",
    minimal_example: "let color: Color = Color.Red;",
    anti_pattern: Some("let value: i32 = Point;"),
    default_fixit: "replace the type name with a runtime value expression",
};

pub(super) const RULE_MAIN_REQUIRED: RuleTemplate = RuleTemplate {
    rule_id: "main_function_required",
    normalized_pattern: "main_function_required",
    repair_goal: "Add a valid `main` entrypoint so the current AX program is runnable.",
    summary: "Every runnable AX program must define `fn main() -> i32`.",
    pattern: "fn main() -> i32 { return 0; }",
    minimal_example: "fn main() -> i32 { return 0; }",
    anti_pattern: None,
    default_fixit: "add `fn main() -> i32 { return 0; }`",
};

pub(super) const RULE_MAIN_SIGNATURE: RuleTemplate = RuleTemplate {
    rule_id: "main_signature_fixed",
    normalized_pattern: "main_signature_fixed",
    repair_goal: "Change `main` so it takes no parameters and returns `i32`.",
    summary: "The current AX prototype requires `main` to use the fixed signature `fn main() -> i32`.",
    pattern: "fn main() -> i32 { return 0; }",
    minimal_example: "fn main() -> i32 { return 0; }",
    anti_pattern: Some("fn main(value: i32) -> bool { return false; }"),
    default_fixit: "rewrite `main` to `fn main() -> i32 { ... }`",
};

pub(super) const RULE_FUNCTION_ARGUMENT_TYPE_MUST_MATCH: RuleTemplate = RuleTemplate {
    rule_id: "function_argument_type_must_match",
    normalized_pattern: "function_argument_type_must_match",
    repair_goal: "Make each call argument produce the exact type declared by the target parameter.",
    summary: "AX checks every call argument against the function signature and does not coerce argument types.",
    pattern: "fn add(value: i32) -> i32 { return value; }",
    minimal_example: "fn main() -> i32 { return add(1); }",
    anti_pattern: Some("fn main() -> i32 { return add(true); }"),
    default_fixit: "change the argument expression or parameter type so the call matches the function signature",
};

pub(super) const RULE_RETURN_VALUE_MUST_MATCH_DECLARED_TYPE: RuleTemplate = RuleTemplate {
    rule_id: "return_value_must_match_declared_type",
    normalized_pattern: "return_value_must_match_declared_type",
    repair_goal: "Return a value whose type matches the function's declared return type.",
    summary: "AX checks every `return` statement against the declared function return type and does not coerce values.",
    pattern: "fn main() -> i32 { return 0; }",
    minimal_example: "fn ready() -> bool { return true; }",
    anti_pattern: Some("fn main() -> i32 { return false; }"),
    default_fixit: "change the returned expression or the function return type so they match exactly",
};

pub(super) const RULE_CONDITION_MUST_BE_BOOL: RuleTemplate = RuleTemplate {
    rule_id: "condition_expression_must_be_bool",
    normalized_pattern: "condition_expression_must_be_bool",
    repair_goal: "Make the condition expression evaluate to `bool`.",
    summary: "AX does not coerce integers, strings, or other values into `if`, `while`, or `for` conditions.",
    pattern: "if (count < limit) { return 1; }",
    minimal_example: "while (index < len) { index = index + 1; }",
    anti_pattern: Some("if (1) { return 0; }"),
    default_fixit: "rewrite the condition as a boolean comparison or boolean variable",
};

pub(super) const RULE_TYPE_MISMATCH: RuleTemplate = RuleTemplate {
    rule_id: "type_match_required",
    normalized_pattern: "type_match_required",
    repair_goal: "Change the expression or the declared type so both sides use the same AX type.",
    summary: "AX requires assignments, arguments, returns, and conditions to use the declared type exactly.",
    pattern: "let value: i32 = 1;",
    minimal_example: "fn add(value: i32) -> i32 { return value; }",
    anti_pattern: Some("let value: bool = 1;"),
    default_fixit: "make the expression and the expected AX type agree",
};

pub(super) const RULE_MISSING_RETURN: RuleTemplate = RuleTemplate {
    rule_id: "all_paths_must_return",
    normalized_pattern: "all_paths_must_return",
    repair_goal: "Make every control-flow path return a value of the declared function type.",
    summary: "Functions with a non-void return type must return a value on every control-flow path.",
    pattern: "fn helper(flag: bool) -> i32 { if (flag) { return 1; } return 0; }",
    minimal_example: "fn helper(flag: bool) -> i32 { if (flag) { return 1; } return 0; }",
    anti_pattern: Some("fn helper(flag: bool) -> i32 { if (flag) { return 1; } }"),
    default_fixit: "add a `return ...;` on the missing control-flow path",
};

pub(super) const RULE_IMMUTABLE_ASSIGNMENT: RuleTemplate = RuleTemplate {
    rule_id: "mutable_binding_required",
    normalized_pattern: "mutable_binding_required",
    repair_goal: "Either declare the binding with `let mut` or stop assigning to it.",
    summary: "AX bindings are immutable unless they are declared with `let mut`.",
    pattern: "let mut value: i32 = 0; value = value + 1;",
    minimal_example: "let mut value: i32 = 0; value = value + 1;",
    anti_pattern: Some("let value: i32 = 0; value = 1;"),
    default_fixit: "change the declaration to `let mut ...` or remove the assignment",
};

pub(super) const RULE_UNDEFINED_VARIABLE: RuleTemplate = RuleTemplate {
    rule_id: "variable_must_be_declared_in_scope",
    normalized_pattern: "variable_must_be_declared_in_scope",
    repair_goal: "Introduce a declaration in scope before using the variable.",
    summary: "AX requires variables to be declared before use within the current scope.",
    pattern: "let value: i32 = 1; println(value);",
    minimal_example: "let total: i32 = 1; println(total);",
    anti_pattern: Some("println(total);"),
    default_fixit: "declare the variable before this use",
};
