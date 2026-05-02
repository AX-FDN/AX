use super::{AiAction, DiagnosticLayer, TeachingLevel, enhance_diagnostics, match_rule};
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
            message: "match guard type placeholder",
            kind: DiagnosticKind::MatchGuardTypeMismatch,
            expected_rule_id: "match_guard_must_be_bool",
        },
        KindCase {
            code: "S0056",
            message: "match range placeholder",
            kind: DiagnosticKind::MatchRangeMustBeNonEmpty,
            expected_rule_id: "match_range_must_be_non_empty",
        },
        KindCase {
            code: "S0060",
            message: "match struct pattern placeholder",
            kind: DiagnosticKind::MatchStructPatternShapeMismatch,
            expected_rule_id: "match_struct_pattern_must_match_declaration",
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
            code: "S0059",
            message: "trait reference placeholder",
            kind: DiagnosticKind::TraitReferenceMustResolve,
            expected_rule_id: "trait_reference_must_resolve",
        },
        KindCase {
            code: "S0059",
            message: "trait bound placeholder",
            kind: DiagnosticKind::TraitBoundNotSatisfied,
            expected_rule_id: "trait_bound_must_be_satisfied",
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
            code: "R0142",
            message: "string list negative placeholder",
            kind: DiagnosticKind::StringListIndexNegative,
            expected_rule_id: "string_list_index_must_be_non_negative",
        },
        KindCase {
            code: "R0143",
            message: "string list bounds placeholder",
            kind: DiagnosticKind::StringListIndexOutOfBounds,
            expected_rule_id: "string_list_index_must_stay_in_bounds",
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
        let diagnostic =
            Diagnostic::new(case.code, case.message, &source, Span::new(0, 2)).with_kind(case.kind);
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
    let json = serde_json::to_string(&analysis.diagnostics).expect("diagnostics should serialize");
    assert!(!json.contains("\"ai\""));
}

#[test]
fn ai_repair_contract_classifies_parser_semantic_and_runtime_layers() {
    let source = SourceFile::anonymous("fn main() -> i32 { let value: i32 = 1 return value; }");
    let mut analysis = analyze(&source);
    enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
        .expect("parser diagnostic should enhance");
    let ai = analysis.diagnostics[0]
        .ai
        .as_ref()
        .expect("parser diagnostic should include ai payload");
    assert_eq!(ai.layer, DiagnosticLayer::Parser);
    assert_eq!(ai.ai_action, AiAction::EditSource);
    assert!(ai.safe_to_edit);
    assert_eq!(ai.validation, vec!["axc check <target>".to_string()]);

    let source = SourceFile::anonymous("fn main() -> i32 { if (1) { return 1; } return 0; }");
    let mut analysis = analyze(&source);
    enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
        .expect("semantic diagnostic should enhance");
    let ai = analysis.diagnostics[0]
        .ai
        .as_ref()
        .expect("semantic diagnostic should include ai payload");
    assert_eq!(ai.layer, DiagnosticLayer::Semantic);
    assert_eq!(ai.ai_action, AiAction::EditSource);
    assert!(ai.safe_to_edit);
    assert_eq!(ai.validation, vec!["axc check <target>".to_string()]);

    let source = SourceFile::anonymous(
        "fn main() -> i32 { let values: [i32; 2] = [1, 2]; return values[2]; }",
    );
    let analysis = analyze(&source);
    let hir = analysis
        .hir
        .as_ref()
        .expect("HIR should exist for runtime contract case");
    let runtime_error = run_program(&source, hir).expect_err("runtime contract case should fail");
    let mut diagnostics = vec![runtime_error];
    enhance_diagnostics(&source, &analysis.program, &mut diagnostics, None)
        .expect("runtime diagnostic should enhance");
    let ai = diagnostics[0]
        .ai
        .as_ref()
        .expect("runtime diagnostic should include ai payload");
    assert_eq!(ai.layer, DiagnosticLayer::Interpreter);
    assert_eq!(ai.ai_action, AiAction::EditSource);
    assert!(ai.safe_to_edit);
    assert_eq!(
        ai.validation,
        vec![
            "axc check <target>".to_string(),
            "axc run <target>".to_string()
        ]
    );

    let missing_path = unique_session_path("runtime-contract-missing-file").with_extension("txt");
    let _ = fs::remove_file(&missing_path);
    let missing_text = missing_path.to_string_lossy().replace('\\', "/");
    let source = SourceFile::anonymous(&format!(
        "fn main() -> i32 {{ let text: string = fs_read_to_string(\"{missing_text}\"); println(text); return 0; }}"
    ));
    let analysis = analyze(&source);
    let hir = analysis
        .hir
        .as_ref()
        .expect("HIR should exist for host runtime contract case");
    let runtime_error =
        run_program(&source, hir).expect_err("host runtime contract case should fail");
    let mut diagnostics = vec![runtime_error];
    enhance_diagnostics(&source, &analysis.program, &mut diagnostics, None)
        .expect("host runtime diagnostic should enhance");
    let ai = diagnostics[0]
        .ai
        .as_ref()
        .expect("host runtime diagnostic should include ai payload");
    assert_eq!(ai.layer, DiagnosticLayer::Interpreter);
    assert_eq!(ai.ai_action, AiAction::FixRuntimeInput);
    assert!(!ai.safe_to_edit);
    assert_eq!(
        ai.validation,
        vec![
            "axc check <target>".to_string(),
            "axc run <target>".to_string()
        ]
    );
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
    let source = SourceFile::anonymous("fn main() -> i32 { let value: Missing = 1; return 0; }");
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
            diagnostic.code == "S0022" && diagnostic.message.contains("condition must be `bool`")
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
            diagnostic.code == "S0022" && diagnostic.message.contains("array index must be `i32`")
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
fn enhances_trait_bound_mismatch_with_specific_rule_card() {
    let source = SourceFile::anonymous(
        "\
trait Label {
    fn label(self: Self) -> string;
}

fn render<T: Label>(value: T) -> string {
    return value.label();
}

fn main() -> i32 {
    return string_len(render(1));
}
",
    );
    let mut analysis = analyze(&source);
    enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
        .expect("ai enhancement should succeed");

    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.message.contains("does not satisfy trait bound"))
        .expect("trait bound diagnostic should exist");
    let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
    assert_eq!(ai.rule_id, "trait_bound_must_be_satisfied");
    assert_eq!(
        ai.repair_goal,
        "Pass a value whose type implements the required trait, or add the missing `impl Trait for Type` block."
    );
}

#[test]
fn enhances_unknown_trait_bound_with_specific_rule_card() {
    let source = SourceFile::anonymous(
        "\
fn render<T: MissingTrait>(value: T) -> T {
    return value;
}

fn main() -> i32 {
    return render(1);
}
",
    );
    let mut analysis = analyze(&source);
    enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
        .expect("ai enhancement should succeed");

    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.message.contains("unknown trait"))
        .expect("unknown trait diagnostic should exist");
    let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
    assert_eq!(ai.rule_id, "trait_reference_must_resolve");
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
fn enhances_match_struct_pattern_shape_with_specific_rule_card() {
    let source = SourceFile::anonymous(
        "\
struct Point { x: i32, y: i32 }

fn main() -> i32 {
    let point: Point = Point { x: 1, y: 2 };
    return match (point) {
        Point { x } => x,
    };
}
",
    );
    let mut analysis = analyze(&source);
    enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
        .expect("ai enhancement should succeed");

    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "S0060")
        .expect("match struct pattern diagnostic should exist");
    let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
    assert_eq!(ai.rule_id, "match_struct_pattern_must_match_declaration");
    assert_eq!(ai.teaching_level, TeachingLevel::L1);
}

#[test]
fn enhances_struct_pattern_aliases_with_specific_rule_card() {
    let source = SourceFile::anonymous(
        "\
struct Point { x: i32, y: i32 }

fn main() -> i32 {
    let point: Point = Point { x: 1, y: 2 };
    return match (point) {
        Point { x: left, y } => left + y,
    };
}
",
    );
    let mut analysis = analyze(&source);
    enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
        .expect("ai enhancement should succeed");

    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "P0003")
        .expect("struct pattern alias diagnostic should exist");
    let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
    assert_eq!(ai.rule_id, "match_struct_pattern_must_match_declaration");
    assert_eq!(ai.teaching_level, TeachingLevel::L1);
}

#[test]
fn enhances_block_match_arm_linearity_with_specific_rule_card() {
    let source = SourceFile::anonymous(
        "\
fn main() -> i32 {
    let flag: bool = true;
    let value: i32 = match (flag) {
        true => { if (flag) { println(1); } 1 },
        false => 0,
    };
    return value;
}
",
    );
    let mut analysis = analyze(&source);
    enhance_diagnostics(&source, &analysis.program, &mut analysis.diagnostics, None)
        .expect("ai enhancement should succeed");

    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "S0057")
        .expect("block match arm linearity diagnostic should exist");
    let ai = diagnostic.ai.as_ref().expect("ai payload should exist");
    assert_eq!(ai.rule_id, "block_match_arm_must_stay_linear");
    assert_eq!(ai.teaching_level, TeachingLevel::L1);
}

#[test]
fn adds_empty_array_guidance_for_unimplemented_literals() {
    let source = SourceFile::anonymous("fn main() -> i32 { let values: [i32; 1] = []; return 0; }");
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
fn enhances_runtime_string_list_bounds_failure_with_specific_rule_card() {
    let source = SourceFile::anonymous(
        "\
fn main() -> i32 {
    let mut items: string_list = string_list_new();
    items = string_list_push(items, \"alpha\");
    println(string_list_get(items, 2));
    return 0;
}
",
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
    assert_eq!(diagnostics[0].code, "R0143");
    assert_eq!(ai.rule_id, "string_list_index_must_stay_in_bounds");
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
