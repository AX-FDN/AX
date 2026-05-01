use super::RuleTemplate;

pub(super) fn match_code(code: &str) -> Option<RuleTemplate> {
    match code {
        "L0001" => Some(RULE_UNEXPECTED_CHARACTER),
        "L0002" => Some(RULE_UNTERMINATED_STRING_LITERAL),
        "L0003" => Some(RULE_INTEGER_LITERAL_SYNTAX),
        "L0004" => Some(RULE_FLOAT_LITERAL_SYNTAX),
        "L0005" => Some(RULE_SUPPORTED_STRING_ESCAPE_REQUIRED),
        _ => None,
    }
}

const RULE_UNEXPECTED_CHARACTER: RuleTemplate = RuleTemplate {
    rule_id: "unexpected_character_in_source",
    normalized_pattern: "unexpected_character_in_source",
    repair_goal: "Remove or replace the unexpected character with valid AX syntax.",
    summary: "The current AX prototype only accepts its defined punctuation, operators, keywords, and literals.",
    pattern: "fn main() -> i32 { return 0; }",
    minimal_example: "let value: i32 = 1;",
    anti_pattern: Some("fn main() -> i32 [ return 0; }"),
    default_fixit: "delete the unsupported character or rewrite the surrounding syntax with supported AX tokens",
};

const RULE_UNTERMINATED_STRING_LITERAL: RuleTemplate = RuleTemplate {
    rule_id: "string_literal_must_terminate",
    normalized_pattern: "string_literal_must_terminate",
    repair_goal: "Close the current string literal with a matching `\"`.",
    summary: "AX string literals must start and end with `\"` on the same literal.",
    pattern: "let message: string = \"hello\";",
    minimal_example: "println(\"hello\");",
    anti_pattern: Some("println(\"hello);"),
    default_fixit: "insert the missing closing `\"` for this string literal",
};

const RULE_INTEGER_LITERAL_SYNTAX: RuleTemplate = RuleTemplate {
    rule_id: "integer_literal_must_be_valid",
    normalized_pattern: "integer_literal_must_be_valid",
    repair_goal: "Rewrite the integer literal using a valid AX integer form.",
    summary: "AX integer literals must use valid decimal digits before semantic range checks run.",
    pattern: "let value: i32 = 42;",
    minimal_example: "return 123;",
    anti_pattern: Some("let value: i32 = 12abc;"),
    default_fixit: "rewrite the literal as a valid AX integer",
};

const RULE_FLOAT_LITERAL_SYNTAX: RuleTemplate = RuleTemplate {
    rule_id: "float_literal_must_be_valid",
    normalized_pattern: "float_literal_must_be_valid",
    repair_goal: "Rewrite the float literal using a valid AX floating-point form.",
    summary: "AX float literals must use supported decimal syntax before semantic range checks run.",
    pattern: "let ratio: f32 = 1.5;",
    minimal_example: "return 3.25;",
    anti_pattern: Some("let ratio: f32 = 1.2.3;"),
    default_fixit: "rewrite the literal as a valid AX float",
};

const RULE_SUPPORTED_STRING_ESCAPE_REQUIRED: RuleTemplate = RuleTemplate {
    rule_id: "string_escape_must_be_supported",
    normalized_pattern: "string_escape_must_be_supported",
    repair_goal: "Replace the unsupported escape sequence with one the AX lexer accepts.",
    summary: "AX currently supports `\\\\`, `\\\"`, `\\n`, and `\\t` inside string literals.",
    pattern: "println(\"line 1\\nline 2\");",
    minimal_example: "let path: string = \"C:\\\\temp\";",
    anti_pattern: Some("println(\"\\r\");"),
    default_fixit: "replace this escape with `\\\\`, `\\\"`, `\\n`, or `\\t`",
};
