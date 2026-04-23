use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::ast::{Block, Expr, ExprKind, Item, ItemKind, Program, Stmt, StmtKind, TypeRef};
use crate::diagnostics::Diagnostic;
use crate::source::{SourceFile, Span};

#[derive(Debug, Clone, Serialize)]
pub struct AiDiagnostic {
    pub rule_id: String,
    pub teaching_level: TeachingLevel,
    pub repeat_count: u32,
    pub repair_goal: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus_item: Option<AiFocusItem>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub relevant_spans: Vec<Span>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub related_symbols: Vec<AiRelatedSymbol>,
    pub rule_card: AiRuleCard,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fixits: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub context_snippets: Vec<AiContextSnippet>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AiFocusItem {
    pub kind: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct AiRelatedSymbol {
    pub kind: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct AiRuleCard {
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimal_example: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anti_pattern: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AiContextSnippet {
    pub label: String,
    pub text: String,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeachingLevel {
    #[serde(rename = "L1")]
    L1,
    #[serde(rename = "L2")]
    L2,
    #[serde(rename = "L3")]
    L3,
}

impl TeachingLevel {
    fn from_repeat_count(repeat_count: u32) -> Self {
        match repeat_count {
            0 | 1 => Self::L1,
            2 | 3 => Self::L2,
            _ => Self::L3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AiSessionEntry {
    diagnostic_code: String,
    rule_id: String,
    normalized_pattern: String,
    repeat_count: u32,
    last_teaching_level: TeachingLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AiSessionFile {
    version: u32,
    entries: BTreeMap<String, AiSessionEntry>,
}

const AI_SESSION_VERSION: u32 = 1;

impl Default for AiSessionFile {
    fn default() -> Self {
        Self {
            version: AI_SESSION_VERSION,
            entries: BTreeMap::new(),
        }
    }
}

pub fn enhance_diagnostics(
    source: &SourceFile,
    program: &Program,
    diagnostics: &mut [Diagnostic],
    session_path: Option<&Path>,
) -> Result<(), String> {
    let mut session = match session_path {
        Some(path) => Some(load_session(path)?),
        None => None,
    };

    for diagnostic in diagnostics.iter_mut() {
        let Some(rule) = match_rule(source, diagnostic) else {
            continue;
        };

        let repeat_count = session
            .as_mut()
            .map(|state| {
                state.bump(
                    diagnostic.code.as_str(),
                    rule.rule_id,
                    rule.normalized_pattern,
                )
            })
            .unwrap_or(1);
        let teaching_level = TeachingLevel::from_repeat_count(repeat_count);
        let context = DiagnosticContext::new(source, program, diagnostic, &rule);
        diagnostic.ai = Some(context.build(rule, diagnostic, repeat_count, teaching_level));
    }

    if let (Some(path), Some(session)) = (session_path, session.as_ref()) {
        save_session(path, session)?;
    }

    Ok(())
}

#[derive(Clone, Copy)]
struct RuleTemplate {
    rule_id: &'static str,
    normalized_pattern: &'static str,
    repair_goal: &'static str,
    summary: &'static str,
    pattern: &'static str,
    minimal_example: &'static str,
    anti_pattern: Option<&'static str>,
    default_fixit: &'static str,
}

fn match_rule(source: &SourceFile, diagnostic: &Diagnostic) -> Option<RuleTemplate> {
    let expects = diagnostic.expected.join(" ");
    match diagnostic.code.as_str() {
        "L0001" => Some(RULE_UNEXPECTED_CHARACTER),
        "L0002" => Some(RULE_UNTERMINATED_STRING_LITERAL),
        "L0003" => Some(RULE_INTEGER_LITERAL_SYNTAX),
        "L0004" => Some(RULE_FLOAT_LITERAL_SYNTAX),
        "L0005" => Some(RULE_SUPPORTED_STRING_ESCAPE_REQUIRED),
        "P0001"
            if diagnostic.message == "expected a top-level declaration"
                && note_contains(diagnostic, "identifier `import`") =>
        {
            Some(RULE_IMPORT_NOT_SUPPORTED)
        }
        "P0001"
            if diagnostic.message == "expected a top-level declaration"
                && note_contains(diagnostic, "identifier `module`") =>
        {
            Some(RULE_MODULE_NOT_SUPPORTED)
        }
        "P0001" if looks_like_match_attempt(source, diagnostic) => Some(RULE_MATCH_NOT_SUPPORTED),
        "P0001" if diagnostic.message.contains("expected `;`") || expects.contains("`;`") => {
            Some(RULE_MISSING_SEMICOLON)
        }
        "P0001" if diagnostic.message.contains("expected `)`") || expects.contains("`)`") => {
            Some(RULE_MISSING_RPAREN)
        }
        "P0001" if diagnostic.message.contains("expected `}`") || expects.contains("`}`") => {
            Some(RULE_MISSING_RBRACE)
        }
        "P0001" if diagnostic.message == "expected a top-level declaration" => {
            Some(RULE_TOP_LEVEL_DECLARATION_REQUIRED)
        }
        "P0002" => Some(RULE_TYPE_NAME_REQUIRED),
        "P0003" => Some(RULE_EXPRESSION_REQUIRED),
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
        "S0022" if looks_like_condition_type_mismatch(diagnostic) => {
            Some(RULE_CONDITION_MUST_BE_BOOL)
        }
        "S0022" if looks_like_array_index_type_mismatch(diagnostic) => {
            Some(RULE_ARRAY_INDEX_MUST_BE_I32)
        }
        "S0022" => Some(RULE_TYPE_MISMATCH),
        "S0023" => Some(RULE_MISSING_RETURN),
        "S0024" => Some(RULE_STRUCT_LITERAL_REQUIRES_STRUCT_TYPE),
        "S0025" => Some(RULE_STRUCT_LITERAL_FIELDS_UNIQUE),
        "S0026" => Some(RULE_STRUCT_LITERAL_FIELDS_COMPLETE),
        "S0028" => Some(RULE_TYPE_NAME_NOT_RUNTIME_VALUE),
        "S0029" => Some(RULE_ENUM_VARIANT_MUST_EXIST),
        "S0030" => Some(RULE_MUTABLE_STRUCT_FIELD_ASSIGNMENT_REQUIRED),
        "S0031" => Some(RULE_FOR_HEADER_CLAUSE_SUPPORTED),
        "S0032" => Some(RULE_NON_EMPTY_ARRAY_LITERAL_REQUIRED),
        "S0033" => Some(RULE_INDEX_BASE_MUST_BE_ARRAY),
        "R0012" | "R0018" | "R0019" | "R0020" | "R0022" => Some(RULE_INTEGER_ARITHMETIC_IN_RANGE),
        "R0021" => Some(RULE_DIVISION_BY_ZERO),
        "R0030" => Some(RULE_ARRAY_INDEX_NON_NEGATIVE),
        "R0031" => Some(RULE_ARRAY_INDEX_IN_BOUNDS),
        _ => None,
    }
}

fn note_contains(diagnostic: &Diagnostic, needle: &str) -> bool {
    diagnostic.notes.iter().any(|note| note.contains(needle))
}

fn looks_like_condition_type_mismatch(diagnostic: &Diagnostic) -> bool {
    diagnostic.message.contains("condition must be `bool`")
}

fn looks_like_array_index_type_mismatch(diagnostic: &Diagnostic) -> bool {
    diagnostic.message.contains("array index must be `i32`")
}

fn looks_like_match_attempt(source: &SourceFile, diagnostic: &Diagnostic) -> bool {
    if !diagnostic
        .message
        .contains("expected `;` after expression statement")
    {
        return false;
    }

    let window = diagnostic_window(source, diagnostic.span, 48);
    has_keyword_followed_by(window, "match", '(') && window.contains('{')
}

fn diagnostic_window(source: &SourceFile, span: Span, radius: usize) -> &str {
    let start = span.start.saturating_sub(radius);
    let end = span.end.saturating_add(radius).min(source.text().len());
    source.slice(Span::new(start, end))
}

fn has_keyword_followed_by(text: &str, keyword: &str, next: char) -> bool {
    text.match_indices(keyword).any(|(index, _)| {
        if text[..index]
            .chars()
            .next_back()
            .is_some_and(is_identifier_char)
        {
            return false;
        }

        let tail = &text[index + keyword.len()..];
        let mut tail_chars = tail.chars();
        while let Some(ch) = tail_chars.next() {
            if ch.is_whitespace() {
                continue;
            }
            return ch == next;
        }

        false
    })
}

fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
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

const RULE_IMPORT_NOT_SUPPORTED: RuleTemplate = RuleTemplate {
    rule_id: "import_declarations_not_supported",
    normalized_pattern: "import_declarations_not_supported",
    repair_goal: "Keep the needed declarations in the current file because `import` is not implemented yet.",
    summary: "The current AX prototype does not support `import` declarations or multi-file symbol loading.",
    pattern: "fn helper() -> i32 { return 1; }\nfn main() -> i32 { return helper(); }",
    minimal_example: "fn helper() -> i32 { return 1; }\nfn main() -> i32 { return helper(); }",
    anti_pattern: Some("import math"),
    default_fixit: "move the needed declarations into the same file for now",
};

const RULE_MODULE_NOT_SUPPORTED: RuleTemplate = RuleTemplate {
    rule_id: "module_declarations_not_supported",
    normalized_pattern: "module_declarations_not_supported",
    repair_goal: "Keep the program in a single file because module declarations are not implemented yet.",
    summary: "The current AX prototype does not support `module` declarations or namespace files yet.",
    pattern: "struct Point { x: i32, y: i32 }\nfn main() -> i32 { return 0; }",
    minimal_example: "fn helper() -> i32 { return 1; }\nfn main() -> i32 { return helper(); }",
    anti_pattern: Some("module math"),
    default_fixit: "remove the module declaration and keep the code in one file",
};

const RULE_MATCH_NOT_SUPPORTED: RuleTemplate = RuleTemplate {
    rule_id: "match_expressions_not_supported",
    normalized_pattern: "match_expressions_not_supported",
    repair_goal: "Rewrite this control flow with `if / else` because `match` is not in the current prototype.",
    summary: "The current AX prototype supports `if / else`, but it does not support `match` yet.",
    pattern: "if (flag) { return 1; } else { return 0; }",
    minimal_example: "if (value == 0) { return 1; } else { return 2; }",
    anti_pattern: Some("match (value) { ... }"),
    default_fixit: "rewrite this branch with `if / else`",
};

const RULE_TOP_LEVEL_DECLARATION_REQUIRED: RuleTemplate = RuleTemplate {
    rule_id: "top_level_item_required",
    normalized_pattern: "top_level_item_required",
    repair_goal: "Rewrite this top-level code as a `fn`, `struct`, or `enum` declaration.",
    summary: "Top-level AX source currently only allows `fn`, `struct`, and `enum` declarations.",
    pattern: "fn helper() -> i32 { return 0; }",
    minimal_example: "struct Point { x: i32, y: i32 }",
    anti_pattern: Some("let value: i32 = 1;"),
    default_fixit: "start this top-level item with `fn`, `struct`, or `enum`",
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

const RULE_NON_EMPTY_ARRAY_LITERAL_REQUIRED: RuleTemplate = RuleTemplate {
    rule_id: "non_empty_array_literal_required",
    normalized_pattern: "non_empty_array_literal_required",
    repair_goal: "Provide at least one element so AX can infer the fixed array length and element type.",
    summary: "The current AX prototype supports fixed-size arrays, but empty array literals are not implemented yet.",
    pattern: "let values: [i32; 3] = [1, 2, 3];",
    minimal_example: "let flags: [bool; 1] = [true];",
    anti_pattern: Some("let values: [i32; 0] = [];"),
    default_fixit: "add at least one element to the array literal",
};

const RULE_INDEX_BASE_MUST_BE_ARRAY: RuleTemplate = RuleTemplate {
    rule_id: "index_base_must_be_array",
    normalized_pattern: "index_base_must_be_array",
    repair_goal: "Use `expr[index]` only when the base expression evaluates to a fixed-size array.",
    summary: "AX indexing with `[]` only works on fixed-size arrays, both for reads and element writes.",
    pattern: "let value: i32 = values[0];",
    minimal_example: "let mut values: [i32; 2] = [1, 2]; values[1] = values[0];",
    anti_pattern: Some("let value: i32 = number[0];"),
    default_fixit: "index into an array value like `values[0]`",
};

const RULE_ARRAY_INDEX_MUST_BE_I32: RuleTemplate = RuleTemplate {
    rule_id: "array_index_must_be_i32",
    normalized_pattern: "array_index_must_be_i32",
    repair_goal: "Rewrite the index expression so it produces an `i32` value.",
    summary: "AX array indexing accepts only `i32` index expressions before runtime bounds checks run.",
    pattern: "let value: i32 = values[index];",
    minimal_example: "let index: i32 = 1; return values[index];",
    anti_pattern: Some("return values[true];"),
    default_fixit: "change the index expression to an `i32` value",
};

const RULE_ARRAY_INDEX_IN_BOUNDS: RuleTemplate = RuleTemplate {
    rule_id: "array_index_must_stay_in_bounds",
    normalized_pattern: "array_index_must_stay_in_bounds",
    repair_goal: "Keep the index within `0..len-1` for the current fixed-size array.",
    summary: "AX array indexing is bounds-checked at runtime, so the accessed index must stay within the declared array length.",
    pattern: "let values: [i32; 2] = [1, 2]; return values[1];",
    minimal_example: "let values: [i32; 3] = [1, 2, 3]; println(values[2]);",
    anti_pattern: Some("let values: [i32; 2] = [1, 2]; return values[2];"),
    default_fixit: "change the index or array length so the access stays within bounds",
};

const RULE_ARRAY_INDEX_NON_NEGATIVE: RuleTemplate = RuleTemplate {
    rule_id: "array_index_must_be_non_negative",
    normalized_pattern: "array_index_must_be_non_negative",
    repair_goal: "Use an index expression that never evaluates to a negative `i32` value.",
    summary: "AX array indexing accepts `i32`, but runtime indexing still requires the resolved value to be zero or greater.",
    pattern: "let values: [i32; 2] = [1, 2]; return values[0];",
    minimal_example: "let values: [i32; 3] = [1, 2, 3]; println(values[index]);",
    anti_pattern: Some("let values: [i32; 2] = [1, 2]; return values[-1];"),
    default_fixit: "change the index expression so it stays at 0 or above",
};

const RULE_DIVISION_BY_ZERO: RuleTemplate = RuleTemplate {
    rule_id: "division_by_zero_must_be_avoided",
    normalized_pattern: "division_by_zero_must_be_avoided",
    repair_goal: "Prove that the divisor is never zero before dividing.",
    summary: "AX rejects division by zero at runtime for both `i32` and `f32` division.",
    pattern: "if (divisor == 0) { return 0; } return value / divisor;",
    minimal_example: "let safe: i32 = total / count;",
    anti_pattern: Some("return value / 0;"),
    default_fixit: "guard the divisor or rewrite the calculation so the right-hand side cannot be zero",
};

const RULE_INTEGER_ARITHMETIC_IN_RANGE: RuleTemplate = RuleTemplate {
    rule_id: "integer_arithmetic_must_stay_in_range",
    normalized_pattern: "integer_arithmetic_must_stay_in_range",
    repair_goal: "Rewrite the arithmetic so every intermediate `i32` result stays within the valid range.",
    summary: "AX checks `i32` arithmetic at runtime, so negation, addition, subtraction, multiplication, and division must stay within range.",
    pattern: "let value: i32 = left + right;",
    minimal_example: "let value: i32 = count - 1;",
    anti_pattern: Some("let value: i32 = 2147483647 + 1;"),
    default_fixit: "use smaller operands or rewrite the arithmetic so the `i32` result stays in range",
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

struct AiSession {
    entries: BTreeMap<String, AiSessionEntry>,
}

impl Default for AiSession {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }
}

impl AiSession {
    fn bump(&mut self, diagnostic_code: &str, rule_id: &str, normalized_pattern: &str) -> u32 {
        let key = format!("{diagnostic_code}::{normalized_pattern}");
        let entry = self.entries.entry(key).or_insert_with(|| AiSessionEntry {
            diagnostic_code: diagnostic_code.to_string(),
            rule_id: rule_id.to_string(),
            normalized_pattern: normalized_pattern.to_string(),
            repeat_count: 0,
            last_teaching_level: TeachingLevel::L1,
        });
        entry.repeat_count += 1;
        entry.last_teaching_level = TeachingLevel::from_repeat_count(entry.repeat_count);
        entry.repeat_count
    }
}

fn load_session(path: &Path) -> Result<AiSession, String> {
    match fs::read_to_string(path) {
        Ok(text) => {
            let file: AiSessionFile = serde_json::from_str(&text).map_err(|error| {
                format!("failed to parse AI session {}: {error}", path.display())
            })?;
            if file.version != AI_SESSION_VERSION {
                return Err(format!(
                    "unsupported AI session version `{}` in {}; expected `{}`",
                    file.version,
                    path.display(),
                    AI_SESSION_VERSION
                ));
            }
            Ok(AiSession {
                entries: file.entries,
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(AiSession::default()),
        Err(error) => Err(format!(
            "failed to read AI session {}: {error}",
            path.display()
        )),
    }
}

fn save_session(path: &Path, session: &AiSession) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
    }

    let file = AiSessionFile {
        version: AI_SESSION_VERSION,
        entries: session.entries.clone(),
    };
    let text = serde_json::to_string_pretty(&file)
        .map_err(|error| format!("failed to serialize AI session {}: {error}", path.display()))?;
    fs::write(path, text)
        .map_err(|error| format!("failed to write AI session {}: {error}", path.display()))
}

struct DiagnosticContext {
    focus_item: Option<AiFocusItem>,
    relevant_spans: Vec<Span>,
    related_symbols: Vec<AiRelatedSymbol>,
    context_snippets: Vec<AiContextSnippet>,
}

impl DiagnosticContext {
    fn new(
        source: &SourceFile,
        program: &Program,
        diagnostic: &Diagnostic,
        rule: &RuleTemplate,
    ) -> Self {
        let mut relevant_spans = vec![diagnostic.span];

        if rule.rule_id == RULE_MAIN_REQUIRED.rule_id {
            return Self {
                focus_item: None,
                relevant_spans,
                related_symbols: Vec::new(),
                context_snippets: vec![AiContextSnippet {
                    label: "diagnostic_site".to_string(),
                    text: snippet_text(source, diagnostic.span, 3),
                    span: diagnostic.span,
                }],
            };
        }

        let Some(item) = find_focus_item(program, diagnostic.span) else {
            return Self {
                focus_item: None,
                relevant_spans,
                related_symbols: Vec::new(),
                context_snippets: vec![AiContextSnippet {
                    label: "diagnostic_site".to_string(),
                    text: snippet_text(source, diagnostic.span, 3),
                    span: diagnostic.span,
                }],
            };
        };

        push_unique_span(&mut relevant_spans, item.span);
        let mut snippet_spans = vec![("diagnostic_site".to_string(), diagnostic.span)];

        let focus_item = Some(item_descriptor(item));
        let related_symbols = related_symbols_for_item(program, item);

        if let ItemKind::Function { body, .. } = &item.kind {
            push_unique_span(&mut relevant_spans, body.span);
            if let Some(statement_span) = find_smallest_statement_span(body, diagnostic.span) {
                push_unique_span(&mut relevant_spans, statement_span);
                snippet_spans.push(("enclosing_statement".to_string(), statement_span));
            }
            snippet_spans.push(("function_context".to_string(), body.span));
        } else {
            snippet_spans.push(("focus_item".to_string(), item.span));
        }

        let context_snippets = snippet_spans
            .into_iter()
            .filter_map(|(label, span)| {
                let text = snippet_text(source, span, 4);
                if text.is_empty() {
                    None
                } else {
                    Some(AiContextSnippet { label, text, span })
                }
            })
            .collect::<Vec<_>>();

        Self {
            focus_item,
            relevant_spans,
            related_symbols,
            context_snippets,
        }
    }

    fn build(
        &self,
        rule: RuleTemplate,
        diagnostic: &Diagnostic,
        repeat_count: u32,
        teaching_level: TeachingLevel,
    ) -> AiDiagnostic {
        let mut fixits = Vec::new();
        if let Some(suggestion) = &diagnostic.suggestion {
            fixits.push(suggestion.clone());
        }
        if fixits.is_empty() {
            fixits.push(rule.default_fixit.to_string());
        }

        let rule_card = match teaching_level {
            TeachingLevel::L1 => AiRuleCard {
                summary: rule.summary.to_string(),
                pattern: None,
                minimal_example: None,
                anti_pattern: None,
            },
            TeachingLevel::L2 => AiRuleCard {
                summary: rule.summary.to_string(),
                pattern: Some(rule.pattern.to_string()),
                minimal_example: None,
                anti_pattern: None,
            },
            TeachingLevel::L3 => AiRuleCard {
                summary: rule.summary.to_string(),
                pattern: Some(rule.pattern.to_string()),
                minimal_example: Some(rule.minimal_example.to_string()),
                anti_pattern: rule.anti_pattern.map(str::to_string),
            },
        };

        AiDiagnostic {
            rule_id: rule.rule_id.to_string(),
            teaching_level,
            repeat_count,
            repair_goal: rule.repair_goal.to_string(),
            focus_item: self.focus_item.clone(),
            relevant_spans: self.relevant_spans.clone(),
            related_symbols: match teaching_level {
                TeachingLevel::L3 => self.related_symbols.clone(),
                _ => Vec::new(),
            },
            rule_card,
            fixits,
            context_snippets: match teaching_level {
                TeachingLevel::L3 => self.context_snippets.clone(),
                _ => Vec::new(),
            },
        }
    }
}

fn find_focus_item(program: &Program, span: Span) -> Option<&Item> {
    program
        .items
        .iter()
        .find(|item| item.span.start <= span.start && item.span.end >= span.end)
}

fn item_descriptor(item: &Item) -> AiFocusItem {
    match &item.kind {
        ItemKind::Function {
            name,
            params,
            return_type,
            ..
        } => AiFocusItem {
            kind: "function".to_string(),
            name: name.clone(),
            signature: Some(format!(
                "fn {name}({}) -> {}",
                params
                    .iter()
                    .map(|param| format!("{}: {}", param.name, param.ty.describe()))
                    .collect::<Vec<_>>()
                    .join(", "),
                return_type.describe()
            )),
            span: item.span,
        },
        ItemKind::Struct { name, fields } => AiFocusItem {
            kind: "struct".to_string(),
            name: name.clone(),
            signature: Some(format!(
                "struct {name} {{ {} }}",
                fields
                    .iter()
                    .map(|field| format!("{}: {}", field.name, field.ty.describe()))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            span: item.span,
        },
        ItemKind::Enum { name, variants } => AiFocusItem {
            kind: "enum".to_string(),
            name: name.clone(),
            signature: Some(format!(
                "enum {name} {{ {} }}",
                variants
                    .iter()
                    .map(|variant| variant.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            span: item.span,
        },
    }
}

fn related_symbols_for_item(program: &Program, focus_item: &Item) -> Vec<AiRelatedSymbol> {
    let mut top_level = BTreeMap::new();
    for item in &program.items {
        let name = match &item.kind {
            ItemKind::Function { name, .. }
            | ItemKind::Struct { name, .. }
            | ItemKind::Enum { name, .. } => name.clone(),
        };
        top_level.insert(name, item);
    }

    let focus_name = match &focus_item.kind {
        ItemKind::Function { name, .. }
        | ItemKind::Struct { name, .. }
        | ItemKind::Enum { name, .. } => name,
    };

    let mut referenced = BTreeSet::new();
    match &focus_item.kind {
        ItemKind::Function {
            params,
            return_type,
            body,
            ..
        } => {
            for param in params {
                collect_type_ref_names(&param.ty, &mut referenced);
            }
            collect_type_ref_names(return_type, &mut referenced);
            collect_block_names(body, &mut referenced);
        }
        ItemKind::Struct { fields, .. } => {
            for field in fields {
                collect_type_ref_names(&field.ty, &mut referenced);
            }
        }
        ItemKind::Enum { .. } => {}
    }

    referenced
        .into_iter()
        .filter(|name| name != focus_name)
        .filter_map(|name| top_level.get(&name).copied())
        .map(item_descriptor)
        .map(|item| AiRelatedSymbol {
            kind: item.kind,
            name: item.name,
            signature: item.signature,
            span: item.span,
        })
        .collect()
}

fn collect_block_names(block: &Block, names: &mut BTreeSet<String>) {
    for statement in &block.statements {
        collect_statement_names(statement, names);
    }
}

fn collect_statement_names(statement: &Stmt, names: &mut BTreeSet<String>) {
    match &statement.kind {
        StmtKind::Let {
            ty, initializer, ..
        } => {
            collect_type_ref_names(ty, names);
            collect_expr_names(initializer, names);
        }
        StmtKind::Assign { target, value } => {
            collect_expr_names(target, names);
            collect_expr_names(value, names);
        }
        StmtKind::Expr { expr } => collect_expr_names(expr, names),
        StmtKind::Return { value } => {
            if let Some(expr) = value {
                collect_expr_names(expr, names);
            }
        }
        StmtKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_expr_names(condition, names);
            collect_block_names(then_branch, names);
            if let Some(block) = else_branch {
                collect_block_names(block, names);
            }
        }
        StmtKind::While { condition, body } => {
            collect_expr_names(condition, names);
            collect_block_names(body, names);
        }
        StmtKind::For {
            initializer,
            condition,
            step,
            body,
        } => {
            if let Some(statement) = initializer {
                collect_statement_names(statement, names);
            }
            if let Some(expr) = condition {
                collect_expr_names(expr, names);
            }
            if let Some(statement) = step {
                collect_statement_names(statement, names);
            }
            collect_block_names(body, names);
        }
        StmtKind::Block { block } => collect_block_names(block, names),
    }
}

fn collect_expr_names(expr: &Expr, names: &mut BTreeSet<String>) {
    match &expr.kind {
        ExprKind::Name { value } => {
            names.insert(value.clone());
        }
        ExprKind::Unary { expr, .. } => collect_expr_names(expr, names),
        ExprKind::Binary { left, right, .. } => {
            collect_expr_names(left, names);
            collect_expr_names(right, names);
        }
        ExprKind::Call { callee, arguments } => {
            collect_expr_names(callee, names);
            for argument in arguments {
                collect_expr_names(argument, names);
            }
        }
        ExprKind::StructLiteral { name, fields } => {
            names.insert(name.clone());
            for field in fields {
                collect_expr_names(&field.value, names);
            }
        }
        ExprKind::ArrayLiteral { elements } => {
            for element in elements {
                collect_expr_names(element, names);
            }
        }
        ExprKind::Field { base, .. } => collect_expr_names(base, names),
        ExprKind::Index { base, index } => {
            collect_expr_names(base, names);
            collect_expr_names(index, names);
        }
        ExprKind::Int { .. }
        | ExprKind::Float { .. }
        | ExprKind::Bool { .. }
        | ExprKind::String { .. }
        | ExprKind::Error => {}
    }
}

fn collect_type_ref_names(ty: &TypeRef, names: &mut BTreeSet<String>) {
    match (&ty.name, &ty.element, ty.length) {
        (Some(name), None, None) => {
            names.insert(name.clone());
        }
        (None, Some(element), Some(_)) => collect_type_ref_names(element, names),
        _ => {}
    }
}

fn find_smallest_statement_span(block: &Block, target: Span) -> Option<Span> {
    let mut found = None;
    for statement in &block.statements {
        if !span_contains(statement.span, target) {
            continue;
        }

        found = Some(statement.span);
        match &statement.kind {
            StmtKind::If {
                then_branch,
                else_branch,
                ..
            } => {
                if let Some(inner) = find_smallest_statement_span(then_branch, target) {
                    found = Some(inner);
                }
                if let Some(block) = else_branch {
                    if let Some(inner) = find_smallest_statement_span(block, target) {
                        found = Some(inner);
                    }
                }
            }
            StmtKind::While { body, .. } => {
                if let Some(inner) = find_smallest_statement_span(body, target) {
                    found = Some(inner);
                }
            }
            StmtKind::For { body, .. } | StmtKind::Block { block: body } => {
                if let Some(inner) = find_smallest_statement_span(body, target) {
                    found = Some(inner);
                }
            }
            _ => {}
        }
    }
    found
}

fn span_contains(container: Span, inner: Span) -> bool {
    container.start <= inner.start && container.end >= inner.end
}

fn push_unique_span(spans: &mut Vec<Span>, span: Span) {
    if !spans.contains(&span) {
        spans.push(span);
    }
}

fn snippet_text(source: &SourceFile, span: Span, max_lines: usize) -> String {
    let (start_line, _) = source.line_col(span.start);
    let mut end_offset = span.end;
    if end_offset == span.start {
        end_offset = end_offset.saturating_add(1);
    }
    let (end_line, _) = source.line_col(end_offset);
    let stop = end_line.min(start_line + max_lines.saturating_sub(1));
    let mut lines = Vec::new();
    for line in start_line..=stop {
        lines.push(source.line_text(line).to_string());
    }
    if end_line > stop {
        lines.push("...".to_string());
    }
    lines.join("\n").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::{TeachingLevel, enhance_diagnostics};
    use std::fs;
    use std::path::PathBuf;

    use crate::frontend::analyze;
    use crate::interpreter::run_program;
    use crate::source::SourceFile;

    fn unique_session_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ax-ai-session-{label}-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time should be monotonic")
                .as_nanos()
        ))
    }

    #[test]
    fn base_diagnostics_omit_ai_when_not_enhanced() {
        let source = SourceFile::anonymous("fn main() -> i32 { let value: i32 = 1 return value; }");
        let analysis = analyze(&source);
        let json =
            serde_json::to_string(&analysis.diagnostics).expect("diagnostics should serialize");
        assert!(!json.contains("\"ai\""));
    }

    #[test]
    fn enhances_missing_return_with_rule_card_and_context() {
        let source = SourceFile::anonymous(
            "fn helper(flag: bool) -> i32 { if (flag) { return 1; } }\nfn main() -> i32 { return helper(true); }",
        );
        let mut analysis = analyze(&source);
        enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
            .expect("ai enhancement should succeed");

        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "S0023")
            .expect("missing return diagnostic should exist");
        let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
        assert_eq!(ai.rule_id, "all_paths_must_return");
        assert_eq!(ai.teaching_level, TeachingLevel::L1);
        assert_eq!(ai.repeat_count, 1);
        assert_eq!(
            ai.focus_item.as_ref().map(|item| item.name.as_str()),
            Some("helper")
        );
        assert!(
            ai.relevant_spans
                .iter()
                .any(|span| span.start == diagnostic.span.start)
        );
    }

    #[test]
    fn enhances_unknown_type_with_specific_rule_card() {
        let source =
            SourceFile::anonymous("fn main() -> i32 { let value: Missing = 1; return 0; }");
        let mut analysis = analyze(&source);
        enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
            .expect("ai enhancement should succeed");

        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "S0006")
            .expect("unknown type diagnostic should exist");
        let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
        assert_eq!(ai.rule_id, "type_must_be_declared");
        assert_eq!(
            ai.repair_goal,
            "Use a builtin type or declare the referenced type before using it."
        );
    }

    #[test]
    fn enhances_non_bool_condition_with_specific_rule_card() {
        let source = SourceFile::anonymous("fn main() -> i32 { if (1) { return 1; } return 0; }");
        let mut analysis = analyze(&source);
        enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
            .expect("ai enhancement should succeed");

        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.code == "S0022" && diagnostic.message.contains("condition must be `bool`")
            })
            .expect("condition type diagnostic should exist");
        let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
        assert_eq!(ai.rule_id, "condition_expression_must_be_bool");
        assert_eq!(ai.teaching_level, TeachingLevel::L1);
    }

    #[test]
    fn enhances_array_index_type_mismatch_with_specific_rule_card() {
        let source =
            SourceFile::anonymous("fn main() -> i32 { let values: [i32; 2] = [1, 2]; return values[true]; }");
        let mut analysis = analyze(&source);
        enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
            .expect("ai enhancement should succeed");

        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.code == "S0022" && diagnostic.message.contains("array index must be `i32`")
            })
            .expect("array index type diagnostic should exist");
        let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
        assert_eq!(ai.rule_id, "array_index_must_be_i32");
        assert_eq!(ai.teaching_level, TeachingLevel::L1);
    }

    #[test]
    fn adds_import_guidance_for_unsupported_feature_attempts() {
        let source = SourceFile::anonymous("import math\nfn main() -> i32 { return 0; }");
        let mut analysis = analyze(&source);
        enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
            .expect("ai enhancement should succeed");

        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.code == "P0001"
                    && diagnostic.message == "expected a top-level declaration"
            })
            .expect("import parse diagnostic should exist");
        let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
        assert_eq!(ai.rule_id, "import_declarations_not_supported");
        assert_eq!(ai.teaching_level, TeachingLevel::L1);
    }

    #[test]
    fn adds_match_guidance_for_unsupported_feature_attempts() {
        let source = SourceFile::anonymous("fn main() -> i32 { match (true) { } return 0; }");
        let mut analysis = analyze(&source);
        enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
            .expect("ai enhancement should succeed");

        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.code == "P0001"
                    && diagnostic
                        .message
                        .contains("expected `;` after expression statement")
            })
            .expect("match parse diagnostic should exist");
        let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
        assert_eq!(ai.rule_id, "match_expressions_not_supported");
        assert_eq!(
            ai.fixits,
            vec!["insert `;` before the next statement or closing `}`".to_string()]
        );
    }

    #[test]
    fn adds_empty_array_guidance_for_unimplemented_literals() {
        let source =
            SourceFile::anonymous("fn main() -> i32 { let values: [i32; 0] = []; return 0; }");
        let mut analysis = analyze(&source);
        enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
            .expect("ai enhancement should succeed");

        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "S0032")
            .expect("empty array diagnostic should exist");
        let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
        assert_eq!(ai.rule_id, "non_empty_array_literal_required");
        assert_eq!(ai.teaching_level, TeachingLevel::L1);
    }

    #[test]
    fn enhances_runtime_array_bounds_error_with_specific_rule_card() {
        let source = SourceFile::anonymous(
            "fn main() -> i32 { let values: [i32; 2] = [1, 2]; return values[2]; }",
        );
        let analysis = analyze(&source);
        assert!(
            analysis.diagnostics.is_empty(),
            "analysis should succeed before runtime failure"
        );

        let hir = analysis
            .hir
            .as_ref()
            .expect("HIR should be available after successful analysis");
        let runtime_error = run_program(&source, hir).expect_err("program should fail at runtime");
        let mut diagnostics = vec![runtime_error];

        enhance_diagnostics(&source, &analysis.program, &mut diagnostics, None)
            .expect("ai enhancement should succeed for runtime diagnostics");

        let ai = diagnostics[0]
            .ai
            .as_ref()
            .expect("runtime diagnostic should have ai payload");
        assert_eq!(diagnostics[0].code, "R0031");
        assert_eq!(ai.rule_id, "array_index_must_stay_in_bounds");
        assert_eq!(
            ai.repair_goal,
            "Keep the index within `0..len-1` for the current fixed-size array."
        );
    }

    #[test]
    fn enhances_runtime_negative_index_error_with_specific_rule_card() {
        let source = SourceFile::anonymous(
            "fn main() -> i32 { let values: [i32; 2] = [1, 2]; return values[-1]; }",
        );
        let analysis = analyze(&source);
        assert!(
            analysis.diagnostics.is_empty(),
            "analysis should succeed before runtime failure"
        );

        let hir = analysis
            .hir
            .as_ref()
            .expect("HIR should be available after successful analysis");
        let runtime_error = run_program(&source, hir).expect_err("program should fail at runtime");
        let mut diagnostics = vec![runtime_error];

        enhance_diagnostics(&source, &analysis.program, &mut diagnostics, None)
            .expect("ai enhancement should succeed for runtime diagnostics");

        let ai = diagnostics[0]
            .ai
            .as_ref()
            .expect("runtime diagnostic should have ai payload");
        assert_eq!(diagnostics[0].code, "R0030");
        assert_eq!(ai.rule_id, "array_index_must_be_non_negative");
    }

    #[test]
    fn enhances_runtime_integer_overflow_with_specific_rule_card() {
        let source =
            SourceFile::anonymous("fn main() -> i32 { return 2147483647 + 1; }");
        let analysis = analyze(&source);
        assert!(
            analysis.diagnostics.is_empty(),
            "analysis should succeed before runtime failure"
        );

        let hir = analysis
            .hir
            .as_ref()
            .expect("HIR should be available after successful analysis");
        let runtime_error = run_program(&source, hir).expect_err("program should fail at runtime");
        let mut diagnostics = vec![runtime_error];

        enhance_diagnostics(&source, &analysis.program, &mut diagnostics, None)
            .expect("ai enhancement should succeed for runtime diagnostics");

        let ai = diagnostics[0]
            .ai
            .as_ref()
            .expect("runtime diagnostic should have ai payload");
        assert_eq!(diagnostics[0].code, "R0018");
        assert_eq!(ai.rule_id, "integer_arithmetic_must_stay_in_range");
    }

    #[test]
    fn enhances_runtime_division_by_zero_with_specific_rule_card() {
        let source = SourceFile::anonymous("fn main() -> i32 { return 8 / 0; }");
        let analysis = analyze(&source);
        assert!(
            analysis.diagnostics.is_empty(),
            "analysis should succeed before runtime failure"
        );

        let hir = analysis
            .hir
            .as_ref()
            .expect("HIR should be available after successful analysis");
        let runtime_error = run_program(&source, hir).expect_err("program should fail at runtime");
        let mut diagnostics = vec![runtime_error];

        enhance_diagnostics(&source, &analysis.program, &mut diagnostics, None)
            .expect("ai enhancement should succeed for runtime diagnostics");

        let ai = diagnostics[0]
            .ai
            .as_ref()
            .expect("runtime diagnostic should have ai payload");
        assert_eq!(diagnostics[0].code, "R0021");
        assert_eq!(ai.rule_id, "division_by_zero_must_be_avoided");
        assert_eq!(
            ai.repair_goal,
            "Prove that the divisor is never zero before dividing."
        );
    }

    #[test]
    fn high_value_diagnostics_keep_stable_rule_ids() {
        struct RuleCase<'a> {
            name: &'a str,
            source: &'a str,
            diagnostic_code: &'a str,
            message_fragment: &'a str,
            expected_rule_id: &'a str,
        }

        let cases = [
            RuleCase {
                name: "missing_semicolon",
                source: "fn main() -> i32 { let value: i32 = 1 return value; }",
                diagnostic_code: "P0001",
                message_fragment: "expected `;`",
                expected_rule_id: "statement_terminator_required",
            },
            RuleCase {
                name: "missing_right_paren",
                source: "fn main() -> i32 { if (true { return 1; } return 0; }",
                diagnostic_code: "P0001",
                message_fragment: "expected `)`",
                expected_rule_id: "close_parenthesized_construct",
            },
            RuleCase {
                name: "undefined_variable",
                source: "fn main() -> i32 { return missing; }",
                diagnostic_code: "S0002",
                message_fragment: "undefined variable",
                expected_rule_id: "variable_must_be_declared_in_scope",
            },
            RuleCase {
                name: "immutable_assignment",
                source: "fn main() -> i32 { let value: i32 = 1; value = 2; return value; }",
                diagnostic_code: "S0003",
                message_fragment: "cannot assign to immutable variable",
                expected_rule_id: "mutable_binding_required",
            },
            RuleCase {
                name: "missing_main",
                source: "fn helper() -> i32 { return 0; }",
                diagnostic_code: "S0004",
                message_fragment: "program is missing",
                expected_rule_id: "main_function_required",
            },
            RuleCase {
                name: "unknown_type",
                source: "fn main() -> i32 { let value: Missing = 1; return 0; }",
                diagnostic_code: "S0006",
                message_fragment: "unknown type",
                expected_rule_id: "type_must_be_declared",
            },
            RuleCase {
                name: "type_mismatch",
                source: "fn main() -> i32 { let value: bool = 1; return 0; }",
                diagnostic_code: "S0022",
                message_fragment: "cannot initialize",
                expected_rule_id: "type_match_required",
            },
            RuleCase {
                name: "non_bool_condition",
                source: "fn main() -> i32 { if (1) { return 1; } return 0; }",
                diagnostic_code: "S0022",
                message_fragment: "condition must be `bool`",
                expected_rule_id: "condition_expression_must_be_bool",
            },
            RuleCase {
                name: "array_index_type",
                source: "fn main() -> i32 { let values: [i32; 2] = [1, 2]; return values[true]; }",
                diagnostic_code: "S0022",
                message_fragment: "array index must be `i32`",
                expected_rule_id: "array_index_must_be_i32",
            },
            RuleCase {
                name: "missing_return",
                source: "fn helper(flag: bool) -> i32 { if (flag) { return 1; } }\nfn main() -> i32 { return helper(true); }",
                diagnostic_code: "S0023",
                message_fragment: "may complete without returning",
                expected_rule_id: "all_paths_must_return",
            },
        ];

        for case in cases {
            let source = SourceFile::anonymous(case.source);
            let mut analysis = analyze(&source);
            enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
                .expect("ai enhancement should succeed");

            let diagnostic = analysis
                .diagnostics
                .iter()
                .find(|diagnostic| {
                    diagnostic.code == case.diagnostic_code
                        && diagnostic.message.contains(case.message_fragment)
                })
                .unwrap_or_else(|| {
                    panic!(
                        "case `{}` should produce diagnostic `{}` containing `{}`; got {:?}",
                        case.name,
                        case.diagnostic_code,
                        case.message_fragment,
                        analysis
                            .diagnostics
                            .iter()
                            .map(|diagnostic| (&diagnostic.code, &diagnostic.message))
                            .collect::<Vec<_>>()
                    )
                });

            let ai = diagnostic
                .ai
                .as_ref()
                .unwrap_or_else(|| panic!("case `{}` should include ai payload", case.name));
            assert_eq!(
                ai.rule_id, case.expected_rule_id,
                "case `{}` should keep its stable rule_id",
                case.name
            );
        }

        let source =
            SourceFile::anonymous("fn main() -> i32 { let values: [i32; 2] = [1, 2]; return values[2]; }");
        let analysis = analyze(&source);
        let hir = analysis
            .hir
            .as_ref()
            .expect("HIR should exist for runtime rule case");
        let runtime_error = run_program(&source, hir).expect_err("runtime rule case should fail");
        let mut diagnostics = vec![runtime_error];
        enhance_diagnostics(&source, &analysis.program, &mut diagnostics, None)
            .expect("runtime diagnostics should enhance");
        let ai = diagnostics[0]
            .ai
            .as_ref()
            .expect("runtime diagnostic should include ai payload");
        assert_eq!(ai.rule_id, "array_index_must_stay_in_bounds");

        let source = SourceFile::anonymous(
            "fn main() -> i32 { let values: [i32; 2] = [1, 2]; return values[-1]; }",
        );
        let analysis = analyze(&source);
        let hir = analysis
            .hir
            .as_ref()
            .expect("HIR should exist for runtime rule case");
        let runtime_error = run_program(&source, hir).expect_err("runtime rule case should fail");
        let mut diagnostics = vec![runtime_error];
        enhance_diagnostics(&source, &analysis.program, &mut diagnostics, None)
            .expect("runtime diagnostics should enhance");
        let ai = diagnostics[0]
            .ai
            .as_ref()
            .expect("runtime diagnostic should include ai payload");
        assert_eq!(ai.rule_id, "array_index_must_be_non_negative");

        let source =
            SourceFile::anonymous("fn main() -> i32 { return 2147483647 + 1; }");
        let analysis = analyze(&source);
        let hir = analysis
            .hir
            .as_ref()
            .expect("HIR should exist for runtime rule case");
        let runtime_error = run_program(&source, hir).expect_err("runtime rule case should fail");
        let mut diagnostics = vec![runtime_error];
        enhance_diagnostics(&source, &analysis.program, &mut diagnostics, None)
            .expect("runtime diagnostics should enhance");
        let ai = diagnostics[0]
            .ai
            .as_ref()
            .expect("runtime diagnostic should include ai payload");
        assert_eq!(ai.rule_id, "integer_arithmetic_must_stay_in_range");

        let source = SourceFile::anonymous("fn main() -> i32 { return 8 / 0; }");
        let analysis = analyze(&source);
        let hir = analysis
            .hir
            .as_ref()
            .expect("HIR should exist for runtime rule case");
        let runtime_error = run_program(&source, hir).expect_err("runtime rule case should fail");
        let mut diagnostics = vec![runtime_error];
        enhance_diagnostics(&source, &analysis.program, &mut diagnostics, None)
            .expect("runtime diagnostics should enhance");
        let ai = diagnostics[0]
            .ai
            .as_ref()
            .expect("runtime diagnostic should include ai payload");
        assert_eq!(ai.rule_id, "division_by_zero_must_be_avoided");
    }

    #[test]
    fn teaching_level_escalates_with_session_reuse() {
        let temp_path = unique_session_path("teaching-level");

        let source = SourceFile::anonymous("fn main() -> i32 { let value: i32 = 1 return value; }");

        let mut first = analyze(&source);
        enhance_diagnostics(
            &source,
            &first.program,
            &mut first.diagnostics,
            Some(temp_path.as_path()),
        )
        .expect("first enhancement should succeed");

        let mut second = analyze(&source);
        enhance_diagnostics(
            &source,
            &second.program,
            &mut second.diagnostics,
            Some(temp_path.as_path()),
        )
        .expect("second enhancement should succeed");

        let first_ai = first.diagnostics[0]
            .ai
            .as_ref()
            .expect("first diagnostic should have ai");
        let second_ai = second.diagnostics[0]
            .ai
            .as_ref()
            .expect("second diagnostic should have ai");

        assert_eq!(first_ai.teaching_level, TeachingLevel::L1);
        assert_eq!(second_ai.teaching_level, TeachingLevel::L2);
        assert_eq!(second_ai.repeat_count, 2);
        assert!(second_ai.rule_card.pattern.is_some());

        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn rejects_unsupported_session_versions() {
        let temp_path = unique_session_path("unsupported-version");
        fs::write(&temp_path, "{\n  \"version\": 99,\n  \"entries\": {}\n}")
            .expect("test session file should be written");

        let source = SourceFile::anonymous("fn main() -> i32 { let value: i32 = 1 return value; }");
        let mut analysis = analyze(&source);
        let error = enhance_diagnostics(
            &source,
            &analysis.program,
            &mut analysis.diagnostics,
            Some(temp_path.as_path()),
        )
        .expect_err("unsupported version should be rejected");

        assert!(error.contains("unsupported AI session version `99`"));
        assert!(error.contains("expected `1`"));

        let _ = fs::remove_file(temp_path);
    }

    #[test]
    fn persists_session_schema_version_when_writing_state() {
        let temp_path = unique_session_path("persisted-version");
        let source = SourceFile::anonymous("fn main() -> i32 { let value: i32 = 1 return value; }");
        let mut analysis = analyze(&source);

        enhance_diagnostics(
            &source,
            &analysis.program,
            &mut analysis.diagnostics,
            Some(temp_path.as_path()),
        )
        .expect("enhancement should write a session file");

        let saved = fs::read_to_string(&temp_path).expect("session file should be readable");
        let json: serde_json::Value =
            serde_json::from_str(&saved).expect("session file should contain valid json");
        assert_eq!(json["version"], serde_json::Value::from(1));
        assert!(json["entries"].is_object());

        let _ = fs::remove_file(temp_path);
    }
}
