use crate::diagnostics::DiagnosticKind;

use super::RuleTemplate;

pub(super) fn match_code(code: &str) -> Option<RuleTemplate> {
    match code {
        "P0002" => Some(RULE_TYPE_NAME_REQUIRED),
        "P0003" => Some(RULE_EXPRESSION_REQUIRED),
        _ => None,
    }
}

pub(super) fn match_kind(kind: DiagnosticKind) -> Option<RuleTemplate> {
    match kind {
        DiagnosticKind::MissingSemicolon => Some(RULE_MISSING_SEMICOLON),
        DiagnosticKind::MissingRightParen => Some(RULE_MISSING_RPAREN),
        DiagnosticKind::MissingRightBracket => Some(RULE_MISSING_RBRACKET),
        DiagnosticKind::MissingRightBrace => Some(RULE_MISSING_RBRACE),
        DiagnosticKind::TopLevelDeclarationRequired => Some(RULE_TOP_LEVEL_DECLARATION_REQUIRED),
        DiagnosticKind::TypeNameRequired => Some(RULE_TYPE_NAME_REQUIRED),
        DiagnosticKind::ExpressionRequired => Some(RULE_EXPRESSION_REQUIRED),
        _ => None,
    }
}

const RULE_TOP_LEVEL_DECLARATION_REQUIRED: RuleTemplate = RuleTemplate {
    rule_id: "top_level_item_required",
    normalized_pattern: "top_level_item_required",
    repair_goal: "Rewrite this top-level code as a `module`, `import`, `fn`, `struct`, or `enum` item.",
    summary: "Top-level AX source currently only allows `module`, `import`, `fn`, `struct`, and `enum` items.",
    pattern: "import lib.report;\nfn helper() -> i32 { return 0; }",
    minimal_example: "struct Point { x: i32, y: i32 }",
    anti_pattern: Some("let value: i32 = 1;"),
    default_fixit: "start this top-level item with `module`, `import`, `fn`, `struct`, or `enum`",
};

const RULE_TYPE_NAME_REQUIRED: RuleTemplate = RuleTemplate {
    rule_id: "type_name_required",
    normalized_pattern: "type_name_required",
    repair_goal: "Insert a valid AX type name in the current type position.",
    summary: "AX type positions require `bool`, `i32`, `f32`, `string`, `[Type; N]`, or a previously declared type.",
    pattern: "let value: [i32; 3] = [1, 2, 3];",
    minimal_example: "fn helper(value: i32) -> bool { return true; }",
    anti_pattern: Some("let value: = 1;"),
    default_fixit: "insert a builtin type or a previously declared type name",
};

const RULE_EXPRESSION_REQUIRED: RuleTemplate = RuleTemplate {
    rule_id: "expression_required",
    normalized_pattern: "expression_required",
    repair_goal: "Insert a runtime expression that produces the needed value.",
    summary: "AX expression positions require a literal, array literal, name, call, field access, index expression, unary expression, binary expression, or grouped expression.",
    pattern: "return values[index];",
    minimal_example: "let total: i32 = left + right;",
    anti_pattern: Some("return ;"),
    default_fixit: "insert a valid AX expression",
};

const RULE_MISSING_SEMICOLON: RuleTemplate = RuleTemplate {
    rule_id: "statement_terminator_required",
    normalized_pattern: "statement_terminator_required",
    repair_goal: "Insert the missing semicolon so the statement terminates correctly.",
    summary: "AX requires `let`, assignment, expression, and `return` statements to end with `;`.",
    pattern: "let name: Type = expr;",
    minimal_example: "let value: i32 = 1;",
    anti_pattern: Some("let value: i32 = 1"),
    default_fixit: "insert `;` at the end of the current statement",
};

const RULE_MISSING_RPAREN: RuleTemplate = RuleTemplate {
    rule_id: "close_parenthesized_construct",
    normalized_pattern: "close_parenthesized_construct",
    repair_goal: "Close the current parenthesized construct with `)` and keep the surrounding syntax balanced.",
    summary: "AX requires balanced parentheses in conditions, grouped expressions, calls, and `for` headers.",
    pattern: "if (cond) { ... }",
    minimal_example: "if (flag == true) { return 1; }",
    anti_pattern: Some("if (flag == true { return 1; }"),
    default_fixit: "add the missing `)` at the current construct boundary",
};

const RULE_MISSING_RBRACKET: RuleTemplate = RuleTemplate {
    rule_id: "close_bracketed_construct",
    normalized_pattern: "close_bracketed_construct",
    repair_goal: "Close the current bracketed construct with `]` and keep the surrounding syntax balanced.",
    summary: "AX requires balanced brackets in array literals, slice types, array types, index expressions, and slice expressions.",
    pattern: "let values: [i32; 2] = [1, 2];",
    minimal_example: "return values[index];",
    anti_pattern: Some("let values: [i32; 2 = [1, 2];"),
    default_fixit: "add the missing `]` at the current construct boundary",
};

const RULE_MISSING_RBRACE: RuleTemplate = RuleTemplate {
    rule_id: "close_block_or_literal",
    normalized_pattern: "close_block_or_literal",
    repair_goal: "Close the current block or literal with `}`.",
    summary: "AX requires balanced braces for blocks, function bodies, and struct literals.",
    pattern: "fn main() -> i32 { return 0; }",
    minimal_example: "struct Point { x: i32, y: i32 }",
    anti_pattern: Some("fn main() -> i32 { return 0;"),
    default_fixit: "add the missing `}` to close the current block or literal",
};
