use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::ast::{
    Block, Expr, ExprKind, Item, ItemKind, MatchPattern, MatchPatternKind, Program, Stmt, StmtKind,
    TypeRef,
};
use crate::diagnostics::{Diagnostic, DiagnosticKind};
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

fn match_rule(_source: &SourceFile, diagnostic: &Diagnostic) -> Option<RuleTemplate> {
    if let Some(kind) = diagnostic.kind()
        && let Some(rule) = match_rule_by_kind(kind)
    {
        return Some(rule);
    }

    match diagnostic.code.as_str() {
        "L0001" => Some(RULE_UNEXPECTED_CHARACTER),
        "L0002" => Some(RULE_UNTERMINATED_STRING_LITERAL),
        "L0003" => Some(RULE_INTEGER_LITERAL_SYNTAX),
        "L0004" => Some(RULE_FLOAT_LITERAL_SYNTAX),
        "L0005" => Some(RULE_SUPPORTED_STRING_ESCAPE_REQUIRED),
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
        "S0037" => Some(RULE_ENTRY_FILE_MUST_NOT_DECLARE_MODULE),
        "S0039" => Some(RULE_MODULE_PATH_MUST_MATCH_SOURCE_PATH),
        "S0040" => Some(RULE_MODULE_PATH_MUST_BE_UNIQUE),
        "S0041" => Some(RULE_MODULE_IMPORT_MUST_BE_UNIQUE),
        "S0042" => Some(RULE_IMPORTED_MODULE_MUST_EXIST),
        "S0043" => Some(RULE_CROSS_MODULE_REFERENCE_REQUIRES_IMPORT),
        "R0012" | "R0018" | "R0019" | "R0020" | "R0022" | "R0024" => {
            Some(RULE_INTEGER_ARITHMETIC_IN_RANGE)
        }
        "R0021" => Some(RULE_DIVISION_BY_ZERO),
        "R0030" => Some(RULE_ARRAY_INDEX_NON_NEGATIVE),
        "R0031" => Some(RULE_ARRAY_INDEX_IN_BOUNDS),
        "R0040" => Some(RULE_LEN_BUILTIN_REQUIRES_COUNTABLE_VALUE),
        _ => None,
    }
}

fn match_rule_by_kind(kind: DiagnosticKind) -> Option<RuleTemplate> {
    match kind {
        DiagnosticKind::MissingSemicolon => Some(RULE_MISSING_SEMICOLON),
        DiagnosticKind::MissingRightParen => Some(RULE_MISSING_RPAREN),
        DiagnosticKind::MissingRightBracket => Some(RULE_MISSING_RBRACKET),
        DiagnosticKind::MissingRightBrace => Some(RULE_MISSING_RBRACE),
        DiagnosticKind::TopLevelDeclarationRequired => Some(RULE_TOP_LEVEL_DECLARATION_REQUIRED),
        DiagnosticKind::TypeNameRequired => Some(RULE_TYPE_NAME_REQUIRED),
        DiagnosticKind::ExpressionRequired => Some(RULE_EXPRESSION_REQUIRED),
        DiagnosticKind::EntryFileDeclaresModule => Some(RULE_ENTRY_FILE_MUST_NOT_DECLARE_MODULE),
        DiagnosticKind::SupportSourceMissingModuleDeclaration => {
            Some(RULE_SUPPORT_SOURCE_MUST_DECLARE_MODULE)
        }
        DiagnosticKind::SupportSourceMissingManifestListing => {
            Some(RULE_SUPPORT_SOURCE_MUST_BE_LISTED_IN_MANIFEST)
        }
        DiagnosticKind::ModulePathMismatch => Some(RULE_MODULE_PATH_MUST_MATCH_SOURCE_PATH),
        DiagnosticKind::DuplicateModulePath => Some(RULE_MODULE_PATH_MUST_BE_UNIQUE),
        DiagnosticKind::DuplicateModuleImport => Some(RULE_MODULE_IMPORT_MUST_BE_UNIQUE),
        DiagnosticKind::ImportedModuleMissing => Some(RULE_IMPORTED_MODULE_MUST_EXIST),
        DiagnosticKind::CrossModuleReferenceMissingImport => {
            Some(RULE_CROSS_MODULE_REFERENCE_REQUIRES_IMPORT)
        }
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
        DiagnosticKind::ArgvIndexNegative => Some(RULE_ARGV_INDEX_NON_NEGATIVE),
        DiagnosticKind::ArgvIndexOutOfBounds => Some(RULE_ARGV_INDEX_IN_BOUNDS),
        DiagnosticKind::EnvironmentVariableUnavailable => {
            Some(RULE_ENVIRONMENT_VARIABLE_MUST_BE_AVAILABLE)
        }
        DiagnosticKind::ReadableFilePathRequired => Some(RULE_READABLE_FILE_PATH_REQUIRED),
        DiagnosticKind::ReadableDirectoryPathRequired => {
            Some(RULE_READABLE_DIRECTORY_PATH_REQUIRED)
        }
        DiagnosticKind::ProcessCommandNotLaunchable => {
            Some(RULE_PROCESS_COMMAND_MUST_BE_LAUNCHABLE)
        }
        DiagnosticKind::ProcessCaptureNonZeroExit => {
            Some(RULE_PROCESS_CAPTURE_REQUIRES_SUCCESSFUL_EXIT)
        }
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

const RULE_ENTRY_FILE_MUST_NOT_DECLARE_MODULE: RuleTemplate = RuleTemplate {
    rule_id: "entry_file_must_not_declare_module",
    normalized_pattern: "entry_file_must_not_declare_module",
    repair_goal: "Keep the manifest entry file as the root unit and remove its `module` declaration.",
    summary: "In AX minimal module mode, only support sources declare `module`; the entry file stays manifest-owned and provides `fn main() -> i32`.",
    pattern: "src/main.ax: import lib.report;\nfn main() -> i32 { return lib.report.helper(); }",
    minimal_example: "lib/report.ax: module lib.report;\nfn helper() -> i32 { return 1; }",
    anti_pattern: Some("src/main.ax: module app.main;"),
    default_fixit: "remove the `module ...;` line from the entry file",
};

const RULE_SUPPORT_SOURCE_MUST_DECLARE_MODULE: RuleTemplate = RuleTemplate {
    rule_id: "support_source_must_declare_module",
    normalized_pattern: "support_source_must_declare_module",
    repair_goal: "Add a top-of-file `module ...;` declaration to each support source in module mode.",
    summary: "Every support source discovered from `[package].sources` must declare its module path before top-level items.",
    pattern: "lib/report.ax: module lib.report;\nfn helper() -> i32 { return 1; }",
    minimal_example: "src/main.ax: import lib.report;\nfn main() -> i32 { return lib.report.helper(); }",
    anti_pattern: Some("lib/report.ax: fn helper() -> i32 { return 1; }"),
    default_fixit: "add a top-of-file `module ...;` declaration",
};

const RULE_SUPPORT_SOURCE_MUST_BE_LISTED_IN_MANIFEST: RuleTemplate = RuleTemplate {
    rule_id: "support_source_must_be_listed_in_manifest",
    normalized_pattern: "support_source_must_be_listed_in_manifest",
    repair_goal: "List each support source file or directory under `[package].sources` so module discovery can see it.",
    summary: "AX only loads support modules from paths declared in `[package].sources` inside `AX.toml`.",
    pattern: "sources = [\"foundation\", \"lib\"]",
    minimal_example: "sources = [\"lib/report.ax\"]",
    anti_pattern: Some("sources = []"),
    default_fixit: "add this file or its parent directory to `[package].sources`",
};

const RULE_MODULE_PATH_MUST_MATCH_SOURCE_PATH: RuleTemplate = RuleTemplate {
    rule_id: "module_path_must_match_source_path",
    normalized_pattern: "module_path_must_match_source_path",
    repair_goal: "Change the `module` declaration so it matches the support-source root alias and relative file path.",
    summary: "AX derives the minimal module path from `[package].sources` and the support file path, so the declaration must match that derived path exactly.",
    pattern: "lib/report.ax: module lib.report;",
    minimal_example: "lib/search/index.ax: module lib.search.index;",
    anti_pattern: Some("lib/report.ax: module report;"),
    default_fixit: "change the declaration to the expected module path",
};

const RULE_MODULE_PATH_MUST_BE_UNIQUE: RuleTemplate = RuleTemplate {
    rule_id: "module_path_must_be_unique",
    normalized_pattern: "module_path_must_be_unique",
    repair_goal: "Rename, move, or merge support files so each module path is owned by exactly one file.",
    summary: "Minimal module mode requires a one-to-one mapping between support files and module paths.",
    pattern: "lib/report.ax: module lib.report;\nlib/summary.ax: module lib.summary;",
    minimal_example: "foundation/text.ax: module foundation.text;",
    anti_pattern: Some("lib/report.ax: module lib.report;\nlib/report_copy.ax: module lib.report;"),
    default_fixit: "rename or move one support file so the module paths are unique",
};

const RULE_MODULE_IMPORT_MUST_BE_UNIQUE: RuleTemplate = RuleTemplate {
    rule_id: "module_import_must_be_unique",
    normalized_pattern: "module_import_must_be_unique",
    repair_goal: "Keep only one `import` line for each module path in the current file.",
    summary: "Repeated `import` lines for the same module do not add behavior and should be collapsed to a single import.",
    pattern: "import lib.report;\nfn main() -> i32 { return lib.report.helper(); }",
    minimal_example: "import foundation.text;\nimport lib.report;",
    anti_pattern: Some("import lib.report;\nimport lib.report;"),
    default_fixit: "remove the duplicate import",
};

const RULE_IMPORTED_MODULE_MUST_EXIST: RuleTemplate = RuleTemplate {
    rule_id: "imported_module_must_exist",
    normalized_pattern: "imported_module_must_exist",
    repair_goal: "Import a support module that actually exists in the current project.",
    summary: "An `import` path must match a support source module declared somewhere under the current project's `[package].sources` roots.",
    pattern: "import lib.report;",
    minimal_example: "lib/report.ax: module lib.report;",
    anti_pattern: Some("import lib.missing;"),
    default_fixit: "import an existing support module declared in this project",
};

const RULE_CROSS_MODULE_REFERENCE_REQUIRES_IMPORT: RuleTemplate = RuleTemplate {
    rule_id: "cross_module_reference_requires_import",
    normalized_pattern: "cross_module_reference_requires_import",
    repair_goal: "Add the missing `import module.path;` line before using a cross-module qualified name.",
    summary: "AX minimal module mode requires explicit imports for cross-module references, even when the code already uses a fully qualified name.",
    pattern: "import lib.report;\nfn main() -> i32 { return lib.report.helper(); }",
    minimal_example: "import lib.flag;\nlet value: lib.flag.Flag = lib.flag.Flag.On;",
    anti_pattern: Some("fn main() -> i32 { return lib.report.helper(); }"),
    default_fixit: "add the required `import module.path;` line near the top of this file",
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
    repair_goal: "Match only on `bool`, `i32`, or enum values in the current AX prototype.",
    summary: "The first AX `match` rollout only supports boolean inputs, integer inputs, and enum values.",
    pattern: "match (flag) { true => { return 1; } false => { return 0; } }",
    minimal_example: "match (status) { Status.Ready => { return 1; } _ => { return 0; } }",
    anti_pattern: Some("match (message) { \"ok\" => { return 1; } _ => { return 0; } }"),
    default_fixit: "rewrite this branch with `if / else`, or match on a `bool`, `i32`, or enum value",
};

const RULE_MATCH_PATTERN_MUST_MATCH_INPUT: RuleTemplate = RuleTemplate {
    rule_id: "match_pattern_must_match_input",
    normalized_pattern: "match_pattern_must_match_input",
    repair_goal: "Keep every `match` arm pattern in the same value domain as the matched input.",
    summary: "AX `match` patterns must align with the scrutinee type: `bool` uses `true`/`false`, `i32` uses integer literals, and enums use `EnumName.Variant`.",
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

const RULE_ARGV_INDEX_NON_NEGATIVE: RuleTemplate = RuleTemplate {
    rule_id: "argv_index_must_be_non_negative",
    normalized_pattern: "argv_index_must_be_non_negative",
    repair_goal: "Call `argv_get(index)` only with a zero-based index that stays at `0` or above.",
    summary: "AX command-line arguments use zero-based `i32` indexing, so negative values are always invalid at runtime.",
    pattern: "if (argv_len() > 0) { let first: string = argv_get(0); }",
    minimal_example: "let output: string = argv_get(1);",
    anti_pattern: Some("let flag: string = argv_get(-1);"),
    default_fixit: "change the index so it is zero or greater before calling `argv_get(...)`",
};

const RULE_ARGV_INDEX_IN_BOUNDS: RuleTemplate = RuleTemplate {
    rule_id: "argv_index_must_stay_in_bounds",
    normalized_pattern: "argv_index_must_stay_in_bounds",
    repair_goal: "Check `argv_len()` first and only read positions that exist in the current runtime invocation.",
    summary: "`argv_get(index)` fails at runtime when the selected index is outside the argument list provided by the host.",
    pattern: "if (argv_len() >= 2) { let output: string = argv_get(1); }",
    minimal_example: "let target: string = argv_get(0);",
    anti_pattern: Some("let missing: string = argv_get(3);"),
    default_fixit: "guard with `argv_len()` or reduce the requested argument index",
};

const RULE_ENVIRONMENT_VARIABLE_MUST_BE_AVAILABLE: RuleTemplate = RuleTemplate {
    rule_id: "environment_variable_must_be_available",
    normalized_pattern: "environment_variable_must_be_available",
    repair_goal: "Only call `env_get(name)` when that variable is present in the host environment, or guard first with `env_has(name)`.",
    summary: "AX exposes host environment variables directly, so missing keys still fail at runtime even though the program type-checks.",
    pattern: "if (env_has(\"PATH\")) { let path: string = env_get(\"PATH\"); }",
    minimal_example: "let home: string = env_get(\"HOME\");",
    anti_pattern: Some("let token: string = env_get(\"MISSING_KEY\");"),
    default_fixit: "guard with `env_has(name)` or ensure the host sets the variable before running the program",
};

const RULE_READABLE_FILE_PATH_REQUIRED: RuleTemplate = RuleTemplate {
    rule_id: "readable_file_path_required",
    normalized_pattern: "readable_file_path_required",
    repair_goal: "Pass an existing readable file path before reading file contents or file metadata.",
    summary: "Host file-reading builtins such as `fs_read_to_string` and `fs_file_size` fail at runtime when the target path is missing, unreadable, or not a regular readable file.",
    pattern: "if (fs_is_file(path)) { let text: string = fs_read_to_string(path); }",
    minimal_example: "let size: i32 = fs_file_size(path);",
    anti_pattern: Some("let text: string = fs_read_to_string(\"missing.txt\");"),
    default_fixit: "guard with `fs_is_file(path)` or pass an existing readable file path",
};

const RULE_READABLE_DIRECTORY_PATH_REQUIRED: RuleTemplate = RuleTemplate {
    rule_id: "readable_directory_path_required",
    normalized_pattern: "readable_directory_path_required",
    repair_goal: "Pass an existing readable directory path before calling `fs_read_dir(path)`.",
    summary: "`fs_read_dir` only succeeds on readable directory paths; missing paths, files, and unreadable directories fail at runtime.",
    pattern: "if (fs_is_dir(path)) { let children: [string] = fs_read_dir(path); }",
    minimal_example: "let children: [string] = fs_read_dir(root_dir);",
    anti_pattern: Some("let children: [string] = fs_read_dir(\"missing-dir\");"),
    default_fixit: "guard with `fs_is_dir(path)` or pass an existing readable directory path",
};

const RULE_PROCESS_COMMAND_MUST_BE_LAUNCHABLE: RuleTemplate = RuleTemplate {
    rule_id: "process_command_must_be_launchable",
    normalized_pattern: "process_command_must_be_launchable",
    repair_goal: "Use a shell command the host can start, and when using `*_in`, pass an existing working directory.",
    summary: "AX process builtins delegate to the host shell, so runtime launch fails if the command cannot start or the selected working directory does not exist.",
    pattern: "let status: i32 = process_run(command);",
    minimal_example: "let output: string = process_capture_in(work_dir, command);",
    anti_pattern: Some("let status: i32 = process_run_in(\"missing-dir\", \"echo ready\");"),
    default_fixit: "fix the command text or working directory so the host shell can start the process",
};

const RULE_PROCESS_CAPTURE_REQUIRES_SUCCESSFUL_EXIT: RuleTemplate = RuleTemplate {
    rule_id: "process_capture_requires_successful_exit",
    normalized_pattern: "process_capture_requires_successful_exit",
    repair_goal: "Use `process_capture` only when the command is expected to exit with status 0, or switch to `process_run` when non-zero exit codes are part of the workflow.",
    summary: "`process_capture` treats non-zero exit status as a runtime error instead of returning stdout, so failing commands must be rewritten or run with the status-oriented builtin.",
    pattern: "let output: string = process_capture(command);",
    minimal_example: "let output: string = process_capture(\"echo ready\");",
    anti_pattern: Some("let output: string = process_capture(\"exit 7\");"),
    default_fixit: "fix the command so it exits 0 or switch to `process_run(...)` / `process_run_in(...)`",
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
        ItemKind::Struct { name, fields, .. } => AiFocusItem {
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
        ItemKind::Enum {
            name,
            type_params,
            variants,
        } => AiFocusItem {
            kind: "enum".to_string(),
            name: name.clone(),
            signature: Some(format!(
                "enum {name}{} {{ {} }}",
                format_type_params(type_params),
                variants
                    .iter()
                    .map(|variant| match &variant.payload {
                        Some(payload) => format!("{}({})", variant.name, payload.describe()),
                        None => variant.name.clone(),
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            span: item.span,
        },
        ItemKind::Trait { name, methods } => AiFocusItem {
            kind: "trait".to_string(),
            name: name.clone(),
            signature: Some(format!(
                "trait {name} {{ {} }}",
                methods
                    .iter()
                    .map(|method| format!("fn {}(...)", method.name))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            span: item.span,
        },
        ItemKind::Impl {
            trait_ref, target, ..
        } => AiFocusItem {
            kind: "impl".to_string(),
            name: target.describe(),
            signature: Some(match trait_ref {
                Some(trait_ref) => {
                    format!("impl {} for {}", trait_ref.describe(), target.describe())
                }
                None => format!("impl {}", target.describe()),
            }),
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
            | ItemKind::Enum { name, .. }
            | ItemKind::Trait { name, .. } => name.clone(),
            ItemKind::Impl { target, .. } => format!("impl {}", target.describe()),
        };
        top_level.insert(name, item);
    }

    let focus_name = match &focus_item.kind {
        ItemKind::Function { name, .. }
        | ItemKind::Struct { name, .. }
        | ItemKind::Enum { name, .. }
        | ItemKind::Trait { name, .. } => name.clone(),
        ItemKind::Impl { target, .. } => format!("impl {}", target.describe()),
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
        ItemKind::Trait { methods, .. } => {
            for method in methods {
                for param in &method.params {
                    collect_type_ref_names(&param.ty, &mut referenced);
                }
                collect_type_ref_names(&method.return_type, &mut referenced);
            }
        }
        ItemKind::Impl {
            trait_ref,
            target,
            methods,
        } => {
            if let Some(trait_ref) = trait_ref {
                collect_type_ref_names(trait_ref, &mut referenced);
            }
            collect_type_ref_names(target, &mut referenced);
            for method in methods {
                for param in &method.params {
                    collect_type_ref_names(&param.ty, &mut referenced);
                }
                collect_type_ref_names(&method.return_type, &mut referenced);
                collect_block_names(&method.body, &mut referenced);
            }
        }
    }

    referenced
        .into_iter()
        .filter(|name| name != &focus_name)
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
        StmtKind::Break => {}
        StmtKind::Continue => {}
        StmtKind::Match { scrutinee, arms } => {
            collect_expr_names(scrutinee, names);
            for arm in arms {
                collect_match_pattern_names(&arm.pattern, names);
                collect_block_names(&arm.body, names);
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
        StmtKind::ForIn {
            binding,
            iterable,
            body,
        } => {
            collect_type_ref_names(&binding.ty, names);
            collect_expr_names(iterable, names);
            collect_block_names(body, names);
        }
        StmtKind::Block { block } => collect_block_names(block, names),
    }
}

fn collect_match_pattern_names(pattern: &MatchPattern, names: &mut BTreeSet<String>) {
    if let MatchPatternKind::EnumVariant { path, .. } = &pattern.kind
        && let Some((enum_path, _)) = path.rsplit_once('.')
    {
        names.insert(enum_path.to_string());
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
        ExprKind::Match { scrutinee, arms } => {
            collect_expr_names(scrutinee, names);
            for arm in arms {
                collect_match_pattern_names(&arm.pattern, names);
                collect_expr_names(&arm.value, names);
            }
        }
        ExprKind::Field { base, .. } => collect_expr_names(base, names),
        ExprKind::Index { base, index } => {
            collect_expr_names(base, names);
            collect_expr_names(index, names);
        }
        ExprKind::Slice { base, start, end } => {
            collect_expr_names(base, names);
            collect_expr_names(start, names);
            collect_expr_names(end, names);
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
        (None, Some(element), None) | (None, Some(element), Some(_)) => {
            collect_type_ref_names(element, names)
        }
        _ => {}
    }
}

fn format_type_params(type_params: &[String]) -> String {
    if type_params.is_empty() {
        String::new()
    } else {
        format!("<{}>", type_params.join(", "))
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
            StmtKind::Match { arms, .. } => {
                for arm in arms {
                    if let Some(inner) = find_smallest_statement_span(&arm.body, target) {
                        found = Some(inner);
                    }
                }
            }
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
            StmtKind::ForIn { body, .. } => {
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
    let segment_end = source.segment_end(span.start);
    let mut end_offset = span.end.min(segment_end);
    if end_offset == span.start {
        end_offset = end_offset.saturating_add(1).min(segment_end);
    }
    let safe_end_offset = end_offset
        .saturating_sub(1)
        .max(span.start)
        .min(segment_end.saturating_sub(1));
    let (end_line, _) = source.line_col(safe_end_offset);
    let stop = end_line.min(start_line + max_lines.saturating_sub(1));
    let mut lines = Vec::new();
    for line in start_line..=stop {
        lines.push(source.line_text_for_offset(span.start, line).to_string());
    }
    if end_line > stop {
        lines.push("...".to_string());
    }
    lines.join("\n").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::{TeachingLevel, enhance_diagnostics, match_rule};
    use std::fs;
    use std::path::{Path, PathBuf};

    use crate::diagnostics::{Diagnostic, DiagnosticKind};
    use crate::frontend::{analyze, analyze_with_project};
    use crate::interpreter::run_program;
    use crate::project::resolve_input;
    use crate::source::{SourceFile, Span};

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

    fn unique_project_root(label: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!(
                "ax-ai-project-{label}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("time should be monotonic")
                    .as_nanos()
            ))
    }

    fn write_project_file(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent directory should exist");
        }
        fs::write(path, text).expect("project file should be written");
    }

    #[test]
    fn stable_diagnostic_kinds_drive_rule_matching_without_old_message_text() {
        struct KindCase {
            code: &'static str,
            message: &'static str,
            kind: DiagnosticKind,
            expected_rule_id: &'static str,
        }

        let source = SourceFile::anonymous("fn main() -> i32 { return 0; }");
        let cases = [
            KindCase {
                code: "P0001",
                message: "parser semicolon placeholder",
                kind: DiagnosticKind::MissingSemicolon,
                expected_rule_id: "statement_terminator_required",
            },
            KindCase {
                code: "P0001",
                message: "parser right paren placeholder",
                kind: DiagnosticKind::MissingRightParen,
                expected_rule_id: "close_parenthesized_construct",
            },
            KindCase {
                code: "P0001",
                message: "parser right bracket placeholder",
                kind: DiagnosticKind::MissingRightBracket,
                expected_rule_id: "close_bracketed_construct",
            },
            KindCase {
                code: "P0001",
                message: "parser right brace placeholder",
                kind: DiagnosticKind::MissingRightBrace,
                expected_rule_id: "close_block_or_literal",
            },
            KindCase {
                code: "P0001",
                message: "parser top-level placeholder",
                kind: DiagnosticKind::TopLevelDeclarationRequired,
                expected_rule_id: "top_level_item_required",
            },
            KindCase {
                code: "P0002",
                message: "parser type name placeholder",
                kind: DiagnosticKind::TypeNameRequired,
                expected_rule_id: "type_name_required",
            },
            KindCase {
                code: "P0003",
                message: "parser expression placeholder",
                kind: DiagnosticKind::ExpressionRequired,
                expected_rule_id: "expression_required",
            },
            KindCase {
                code: "S0038",
                message: "support source manifest drift placeholder",
                kind: DiagnosticKind::SupportSourceMissingManifestListing,
                expected_rule_id: "support_source_must_be_listed_in_manifest",
            },
            KindCase {
                code: "S0038",
                message: "support source module declaration placeholder",
                kind: DiagnosticKind::SupportSourceMissingModuleDeclaration,
                expected_rule_id: "support_source_must_declare_module",
            },
            KindCase {
                code: "S0036",
                message: "break loop context placeholder",
                kind: DiagnosticKind::BreakOutsideLoop,
                expected_rule_id: "break_requires_loop_context",
            },
            KindCase {
                code: "S0044",
                message: "continue loop context placeholder",
                kind: DiagnosticKind::ContinueOutsideLoop,
                expected_rule_id: "continue_requires_loop_context",
            },
            KindCase {
                code: "S0045",
                message: "match input placeholder",
                kind: DiagnosticKind::MatchScrutineeTypeUnsupported,
                expected_rule_id: "match_input_must_use_supported_type",
            },
            KindCase {
                code: "S0046",
                message: "match pattern placeholder",
                kind: DiagnosticKind::MatchPatternTypeMismatch,
                expected_rule_id: "match_pattern_must_match_input",
            },
            KindCase {
                code: "S0047",
                message: "match duplicate placeholder",
                kind: DiagnosticKind::DuplicateMatchPattern,
                expected_rule_id: "match_patterns_must_be_unique",
            },
            KindCase {
                code: "S0048",
                message: "match wildcard placeholder",
                kind: DiagnosticKind::MatchWildcardMustBeLast,
                expected_rule_id: "match_wildcard_must_be_last",
            },
            KindCase {
                code: "S0049",
                message: "match exhaustive placeholder",
                kind: DiagnosticKind::MatchNotExhaustive,
                expected_rule_id: "match_must_be_exhaustive",
            },
            KindCase {
                code: "S0050",
                message: "match concrete placeholder",
                kind: DiagnosticKind::MatchRequiresConcretePattern,
                expected_rule_id: "match_requires_concrete_pattern",
            },
            KindCase {
                code: "S0022",
                message: "match expression arm type placeholder",
                kind: DiagnosticKind::MatchExpressionArmTypeMismatch,
                expected_rule_id: "match_expression_arms_must_share_type",
            },
            KindCase {
                code: "S0022",
                message: "return type placeholder",
                kind: DiagnosticKind::ReturnTypeMismatch,
                expected_rule_id: "return_value_must_match_declared_type",
            },
            KindCase {
                code: "S0022",
                message: "condition type placeholder",
                kind: DiagnosticKind::ConditionTypeMismatch,
                expected_rule_id: "condition_expression_must_be_bool",
            },
            KindCase {
                code: "S0022",
                message: "argument type placeholder",
                kind: DiagnosticKind::FunctionArgumentTypeMismatch,
                expected_rule_id: "function_argument_type_must_match",
            },
            KindCase {
                code: "S0022",
                message: "index type placeholder",
                kind: DiagnosticKind::ArrayIndexTypeMismatch,
                expected_rule_id: "array_index_must_be_i32",
            },
            KindCase {
                code: "S0022",
                message: "len type placeholder",
                kind: DiagnosticKind::LenBuiltinTypeMismatch,
                expected_rule_id: "len_builtin_requires_countable_value",
            },
            KindCase {
                code: "S0052",
                message: "for in iterable placeholder",
                kind: DiagnosticKind::ForInIterableTypeMismatch,
                expected_rule_id: "for_in_requires_array_or_slice",
            },
            KindCase {
                code: "S0022",
                message: "for in binding type placeholder",
                kind: DiagnosticKind::ForInBindingTypeMismatch,
                expected_rule_id: "for_in_binding_must_match_element_type",
            },
            KindCase {
                code: "R0048",
                message: "argv negative placeholder",
                kind: DiagnosticKind::ArgvIndexNegative,
                expected_rule_id: "argv_index_must_be_non_negative",
            },
            KindCase {
                code: "R0048",
                message: "argv bounds placeholder",
                kind: DiagnosticKind::ArgvIndexOutOfBounds,
                expected_rule_id: "argv_index_must_stay_in_bounds",
            },
            KindCase {
                code: "R0053",
                message: "env missing placeholder",
                kind: DiagnosticKind::EnvironmentVariableUnavailable,
                expected_rule_id: "environment_variable_must_be_available",
            },
            KindCase {
                code: "R0061",
                message: "readable file placeholder",
                kind: DiagnosticKind::ReadableFilePathRequired,
                expected_rule_id: "readable_file_path_required",
            },
            KindCase {
                code: "R0123",
                message: "readable dir placeholder",
                kind: DiagnosticKind::ReadableDirectoryPathRequired,
                expected_rule_id: "readable_directory_path_required",
            },
            KindCase {
                code: "R0090",
                message: "process launch placeholder",
                kind: DiagnosticKind::ProcessCommandNotLaunchable,
                expected_rule_id: "process_command_must_be_launchable",
            },
            KindCase {
                code: "R0094",
                message: "process capture placeholder",
                kind: DiagnosticKind::ProcessCaptureNonZeroExit,
                expected_rule_id: "process_capture_requires_successful_exit",
            },
        ];

        for case in cases {
            let diagnostic = Diagnostic::new(case.code, case.message, &source, Span::new(0, 2))
                .with_kind(case.kind);
            let rule = match_rule(&source, &diagnostic)
                .unwrap_or_else(|| panic!("kind case `{}` should match a rule", case.message));
            assert_eq!(
                rule.rule_id, case.expected_rule_id,
                "diagnostic kind should keep the rule mapping stable for `{}`",
                case.message
            );
        }
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
    fn enhances_function_argument_type_mismatch_with_specific_rule_card() {
        let source = SourceFile::anonymous(
            "fn add(value: i32) -> i32 { return value; } fn main() -> i32 { return add(true); }",
        );
        let mut analysis = analyze(&source);
        enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
            .expect("ai enhancement should succeed");

        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.code == "S0022"
                    && diagnostic
                        .message
                        .contains("expects argument `value` to be `i32`")
            })
            .expect("function argument diagnostic should exist");
        let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
        assert_eq!(ai.rule_id, "function_argument_type_must_match");
        assert_eq!(ai.teaching_level, TeachingLevel::L1);
    }

    #[test]
    fn enhances_return_type_mismatch_with_specific_rule_card() {
        let source = SourceFile::anonymous("fn main() -> i32 { return false; }");
        let mut analysis = analyze(&source);
        enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
            .expect("ai enhancement should succeed");

        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.code == "S0022"
                    && diagnostic
                        .message
                        .contains("return statement must produce `i32`")
            })
            .expect("return type diagnostic should exist");
        let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
        assert_eq!(ai.rule_id, "return_value_must_match_declared_type");
        assert_eq!(ai.teaching_level, TeachingLevel::L1);
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
                diagnostic.code == "S0022"
                    && diagnostic.message.contains("condition must be `bool`")
            })
            .expect("condition type diagnostic should exist");
        let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
        assert_eq!(ai.rule_id, "condition_expression_must_be_bool");
        assert_eq!(ai.teaching_level, TeachingLevel::L1);
    }

    #[test]
    fn enhances_array_index_type_mismatch_with_specific_rule_card() {
        let source = SourceFile::anonymous(
            "fn main() -> i32 { let values: [i32; 2] = [1, 2]; return values[true]; }",
        );
        let mut analysis = analyze(&source);
        enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
            .expect("ai enhancement should succeed");

        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.code == "S0022"
                    && diagnostic.message.contains("array index must be `i32`")
            })
            .expect("array index type diagnostic should exist");
        let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
        assert_eq!(ai.rule_id, "array_index_must_be_i32");
        assert_eq!(ai.teaching_level, TeachingLevel::L1);
    }

    #[test]
    fn enhances_len_argument_type_mismatch_with_specific_rule_card() {
        let source = SourceFile::anonymous("fn main() -> i32 { return len(true); }");
        let mut analysis = analyze(&source);
        enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
            .expect("ai enhancement should succeed");

        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.code == "S0022"
                    && diagnostic
                        .message
                        .contains("function `len` expects argument `value`")
            })
            .expect("len type diagnostic should exist");
        let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
        assert_eq!(ai.rule_id, "len_builtin_requires_countable_value");
        assert_eq!(ai.teaching_level, TeachingLevel::L1);
    }

    #[test]
    fn enhances_non_slice_base_with_specific_rule_card() {
        let source = SourceFile::anonymous(
            "fn main() -> i32 { let count: i32 = 1; let view: [i32] = count[0:1]; return 0; }",
        );
        let mut analysis = analyze(&source);
        enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
            .expect("ai enhancement should succeed");

        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "S0034")
            .expect("slice base diagnostic should exist");
        let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
        assert_eq!(ai.rule_id, "slice_base_must_be_array_or_slice");
    }

    #[test]
    fn enhances_slice_assignment_with_specific_rule_card() {
        let source = SourceFile::anonymous(
            "fn main() -> i32 { let values: [i32; 3] = [1, 2, 3]; let mut view: [i32] = values[0:2]; view[0] = 9; return 0; }",
        );
        let mut analysis = analyze(&source);
        enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
            .expect("ai enhancement should succeed");

        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "S0035")
            .expect("slice assignment diagnostic should exist");
        let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
        assert_eq!(ai.rule_id, "slice_values_are_read_only");
    }

    #[test]
    fn adds_module_declaration_guidance_for_support_sources() {
        let project_root = unique_project_root("missing-module-declaration");
        let _ = fs::remove_dir_all(&project_root);
        write_project_file(
            &project_root.join("AX.toml"),
            "\
manifest_version = 1

[package]
name = \"ai_module_missing_decl\"
entry = \"src/main.ax\"
sources = [\"lib\"]
",
        );
        write_project_file(
            &project_root.join("lib").join("report.ax"),
            "fn helper() -> i32 { return 1; }\n",
        );
        write_project_file(
            &project_root.join("src").join("main.ax"),
            "import lib.report;\nfn main() -> i32 { return lib.report.helper(); }\n",
        );

        let resolved = resolve_input(&project_root).expect("project should resolve");
        let mut analysis = analyze_with_project(&resolved.source, resolved.project.as_ref());
        enhance_diagnostics(
            &resolved.source,
            &analysis.program,
            &mut analysis.diagnostics,
            None,
        )
        .expect("ai enhancement should succeed");

        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "S0038")
            .expect("missing module declaration diagnostic should exist");
        let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
        assert_eq!(ai.rule_id, "support_source_must_declare_module");
        assert_eq!(ai.teaching_level, TeachingLevel::L1);

        let _ = fs::remove_dir_all(project_root);
    }

    #[test]
    fn adds_missing_import_guidance_for_cross_module_references() {
        let project_root = unique_project_root("missing-module-import");
        let _ = fs::remove_dir_all(&project_root);
        write_project_file(
            &project_root.join("AX.toml"),
            "\
manifest_version = 1

[package]
name = \"ai_module_missing_import\"
entry = \"src/main.ax\"
sources = [\"lib\"]
",
        );
        write_project_file(
            &project_root.join("lib").join("report.ax"),
            "module lib.report;\nfn helper() -> i32 { return 1; }\n",
        );
        write_project_file(
            &project_root.join("src").join("main.ax"),
            "fn main() -> i32 { return lib.report.helper(); }\n",
        );

        let resolved = resolve_input(&project_root).expect("project should resolve");
        let mut analysis = analyze_with_project(&resolved.source, resolved.project.as_ref());
        enhance_diagnostics(
            &resolved.source,
            &analysis.program,
            &mut analysis.diagnostics,
            None,
        )
        .expect("ai enhancement should succeed");

        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "S0043")
            .expect("missing import diagnostic should exist");
        let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
        assert_eq!(ai.rule_id, "cross_module_reference_requires_import");
        assert_eq!(ai.teaching_level, TeachingLevel::L1);
        assert!(
            ai.fixits
                .iter()
                .any(|fixit| fixit.contains("import lib.report;"))
        );

        let _ = fs::remove_dir_all(project_root);
    }

    #[test]
    fn enhances_non_exhaustive_match_with_specific_rule_card() {
        let source = SourceFile::anonymous(
            "fn main() -> i32 { let flag: bool = true; match (flag) { true => { return 1; } } }",
        );
        let mut analysis = analyze(&source);
        enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
            .expect("ai enhancement should succeed");

        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "S0049")
            .expect("match exhaustiveness diagnostic should exist");
        let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
        assert_eq!(ai.rule_id, "match_must_be_exhaustive");
        assert_eq!(ai.teaching_level, TeachingLevel::L1);
    }

    #[test]
    fn enhances_match_pattern_mismatch_with_specific_rule_card() {
        let source = SourceFile::anonymous(
            "fn main() -> i32 { let flag: bool = true; match (flag) { 0 => { return 1; } _ => { return 0; } } }",
        );
        let mut analysis = analyze(&source);
        enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
            .expect("ai enhancement should succeed");

        let diagnostic = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "S0046")
            .expect("match pattern mismatch diagnostic should exist");
        let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
        assert_eq!(ai.rule_id, "match_pattern_must_match_input");
        assert_eq!(ai.teaching_level, TeachingLevel::L1);
    }

    #[test]
    fn adds_empty_array_guidance_for_unimplemented_literals() {
        let source =
            SourceFile::anonymous("fn main() -> i32 { let values: [i32; 1] = []; return 0; }");
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
        let source = SourceFile::anonymous("fn main() -> i32 { return 2147483647 + 1; }");
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
    fn enhances_runtime_missing_file_read_with_host_rule_card() {
        let missing_path = unique_session_path("missing-file-read").with_extension("txt");
        let _ = fs::remove_file(&missing_path);
        let missing_text = missing_path.to_string_lossy().replace('\\', "/");
        let source = SourceFile::anonymous(&format!(
            "fn main() -> i32 {{ let text: string = fs_read_to_string(\"{missing_text}\"); println(text); return 0; }}"
        ));
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
        assert_eq!(diagnostics[0].code, "R0061");
        assert_eq!(ai.rule_id, "readable_file_path_required");
    }

    #[test]
    fn enhances_runtime_missing_directory_read_with_host_rule_card() {
        let missing_path = unique_session_path("missing-dir-read");
        let _ = fs::remove_dir_all(&missing_path);
        let missing_text = missing_path.to_string_lossy().replace('\\', "/");
        let source = SourceFile::anonymous(&format!(
            "fn main() -> i32 {{ let entries: [string] = fs_read_dir(\"{missing_text}\"); println(len(entries)); return 0; }}"
        ));
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
        assert_eq!(diagnostics[0].code, "R0123");
        assert_eq!(ai.rule_id, "readable_directory_path_required");
    }

    #[test]
    fn enhances_runtime_process_launch_failure_with_host_rule_card() {
        let missing_path = unique_session_path("missing-process-dir");
        let _ = fs::remove_dir_all(&missing_path);
        let missing_text = missing_path.to_string_lossy().replace('\\', "/");
        let source = SourceFile::anonymous(&format!(
            "fn main() -> i32 {{ return process_run_in(\"{missing_text}\", \"echo ready\"); }}"
        ));
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
        assert_eq!(diagnostics[0].code, "R0116");
        assert_eq!(ai.rule_id, "process_command_must_be_launchable");
    }

    #[test]
    fn enhances_runtime_process_capture_failure_with_host_rule_card() {
        let source = SourceFile::anonymous(
            "fn main() -> i32 { let output: string = process_capture(\"exit 7\"); println(output); return 0; }",
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
        assert_eq!(diagnostics[0].code, "R0094");
        assert_eq!(ai.rule_id, "process_capture_requires_successful_exit");
    }

    #[test]
    fn enhances_runtime_missing_environment_variable_with_host_rule_card() {
        let source = SourceFile::anonymous(
            "fn main() -> i32 { println(env_get(\"AX_MISSING_KEY\")); return 0; }",
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
        assert_eq!(diagnostics[0].code, "R0053");
        assert_eq!(ai.rule_id, "environment_variable_must_be_available");
    }

    #[test]
    fn enhances_runtime_argv_bounds_failure_with_host_rule_card() {
        let source = SourceFile::anonymous("fn main() -> i32 { println(argv_get(0)); return 0; }");
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
        assert_eq!(diagnostics[0].code, "R0048");
        assert_eq!(ai.rule_id, "argv_index_must_stay_in_bounds");
    }

    #[test]
    fn enhances_runtime_negative_argv_index_with_host_rule_card() {
        let source = SourceFile::anonymous("fn main() -> i32 { println(argv_get(-1)); return 0; }");
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
        assert_eq!(diagnostics[0].code, "R0048");
        assert_eq!(ai.rule_id, "argv_index_must_be_non_negative");
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
                name: "missing_right_bracket",
                source: "fn main() -> i32 { let values: [i32; 2 = [1, 2]; return 0; }",
                diagnostic_code: "P0001",
                message_fragment: "expected `]` after array type",
                expected_rule_id: "close_bracketed_construct",
            },
            RuleCase {
                name: "undefined_variable",
                source: "fn main() -> i32 { return missing; }",
                diagnostic_code: "S0002",
                message_fragment: "undefined variable",
                expected_rule_id: "variable_must_be_declared_in_scope",
            },
            RuleCase {
                name: "type_name_required",
                source: "fn main() -> i32 { let value: = 1; return 0; }",
                diagnostic_code: "P0002",
                message_fragment: "expected a type name",
                expected_rule_id: "type_name_required",
            },
            RuleCase {
                name: "expression_required",
                source: "fn main() -> i32 { let value: i32 = ; return 0; }",
                diagnostic_code: "P0003",
                message_fragment: "expected an expression",
                expected_rule_id: "expression_required",
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
                name: "function_argument_type",
                source: "fn add(value: i32) -> i32 { return value; } fn main() -> i32 { return add(true); }",
                diagnostic_code: "S0022",
                message_fragment: "expects argument `value` to be `i32`",
                expected_rule_id: "function_argument_type_must_match",
            },
            RuleCase {
                name: "return_type",
                source: "fn main() -> i32 { return false; }",
                diagnostic_code: "S0022",
                message_fragment: "return statement must produce `i32`",
                expected_rule_id: "return_value_must_match_declared_type",
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
                name: "len_argument_type",
                source: "fn main() -> i32 { return len(true); }",
                diagnostic_code: "S0022",
                message_fragment: "function `len` expects argument `value`",
                expected_rule_id: "len_builtin_requires_countable_value",
            },
            RuleCase {
                name: "missing_return",
                source: "fn helper(flag: bool) -> i32 { if (flag) { return 1; } }\nfn main() -> i32 { return helper(true); }",
                diagnostic_code: "S0023",
                message_fragment: "may complete without returning",
                expected_rule_id: "all_paths_must_return",
            },
            RuleCase {
                name: "slice_base",
                source: "fn main() -> i32 { let count: i32 = 1; let view: [i32] = count[0:1]; return 0; }",
                diagnostic_code: "S0034",
                message_fragment: "slice expression expects an array or slice value",
                expected_rule_id: "slice_base_must_be_array_or_slice",
            },
            RuleCase {
                name: "slice_assignment",
                source: "fn main() -> i32 { let values: [i32; 3] = [1, 2, 3]; let mut view: [i32] = values[0:2]; view[0] = 9; return 0; }",
                diagnostic_code: "S0035",
                message_fragment: "slices are read-only",
                expected_rule_id: "slice_values_are_read_only",
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

        let source = SourceFile::anonymous(
            "fn main() -> i32 { let values: [i32; 2] = [1, 2]; return values[2]; }",
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

        let source = SourceFile::anonymous("fn main() -> i32 { return 2147483647 + 1; }");
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

        let project_root = unique_project_root("stable-module-rule");
        let _ = fs::remove_dir_all(&project_root);
        write_project_file(
            &project_root.join("AX.toml"),
            "\
manifest_version = 1

[package]
name = \"ai_stable_module_rules\"
entry = \"src/main.ax\"
sources = [\"lib\"]
",
        );
        write_project_file(
            &project_root.join("lib").join("report.ax"),
            "module lib.report;\nfn helper() -> i32 { return 1; }\n",
        );
        write_project_file(
            &project_root.join("src").join("main.ax"),
            "import lib.missing;\nfn main() -> i32 { return lib.report.helper(); }\n",
        );

        let resolved = resolve_input(&project_root).expect("project should resolve");
        let mut analysis = analyze_with_project(&resolved.source, resolved.project.as_ref());
        enhance_diagnostics(
            &resolved.source,
            &analysis.program,
            &mut analysis.diagnostics,
            None,
        )
        .expect("project diagnostics should enhance");

        let imported_module = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "S0042")
            .expect("missing module diagnostic should exist");
        let imported_module_ai = imported_module
            .ai
            .as_ref()
            .expect("missing module diagnostic should include ai payload");
        assert_eq!(imported_module_ai.rule_id, "imported_module_must_exist");

        let missing_import = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "S0043")
            .expect("cross-module import diagnostic should exist");
        let missing_import_ai = missing_import
            .ai
            .as_ref()
            .expect("cross-module import diagnostic should include ai payload");
        assert_eq!(
            missing_import_ai.rule_id,
            "cross_module_reference_requires_import"
        );

        let _ = fs::remove_dir_all(project_root);
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
