use super::*;

pub(in crate::cli) fn usage() -> &'static str {
    "\
axc <command> [options]

Commands:
  check <path> [--json] [--ai] [--ai-session <path>]   Run lexer, parser, and base semantic checks
  ast <path>               Print stable AST JSON
  hir <path>               Print stable HIR JSON
  mir <path>               Print stable MIR JSON
  build <path> [--emit <ir|exe|all>] [--no-link] [--out-dir <path>] [--json]   Emit build artifacts, LLVM IR, or a native executable
  lock <project> [--check] Generate or validate AX.lock for local path packages
  pkg search [query] [--registry <path>]   Search the curated registry preview
  pkg info <package> [--registry <path>]   Show registry package metadata
  pkg check [--registry <path>]            Validate registry metadata
  pkg tree <project>                       Show the current project dependency tree
  pkg add <package> --dry-run [--registry <path>]   Preview the AX.toml registry dependency entry
  pkg install <project> --dry-run [--registry <path>]   Preview registry package resolution
  pkg hash <package-dir>                   Print the stable sha256 package checksum
  run <path> [--json] [--ai] [--ai-session <path>] [-- <args...>]   Execute the minimal interpreter
  fmt <path>               Rewrite the file or project sources to the canonical AX format
  context <overview|boundaries|topology|flow> <path> [--json]
  context <symbol|impact|evidence> <path> <symbol> [--json]   Print stable project/source context JSON
"
}

pub(in crate::cli) fn render_check_success(json: bool, display_path: &str) -> String {
    if json {
        "[]".to_string()
    } else {
        format!("check succeeded: {display_path}")
    }
}

pub(in crate::cli) fn render_load_input_error(
    path: &Path,
    error: &str,
    json: bool,
    ai: bool,
) -> i32 {
    if json {
        if let Some((_source, diagnostic)) = package_load_error_diagnostic(path, error, ai) {
            println!(
                "{}",
                serde_json::to_string_pretty(&vec![diagnostic])
                    .expect("package load diagnostic json should serialize")
            );
            return 1;
        }

        let diagnostic = source_input_error_diagnostic(path, error, ai);
        println!(
            "{}",
            serde_json::to_string_pretty(&vec![diagnostic])
                .expect("source input diagnostic json should serialize")
        );
        return 1;
    }

    eprintln!("{error}");
    1
}

pub(in crate::cli) fn source_input_error_diagnostic(
    path: &Path,
    error: &str,
    ai: bool,
) -> Diagnostic {
    let source = source_for_load_error(path);
    let span = first_source_span(&source);
    let mut diagnostic = Diagnostic::new("I0001", error.trim(), &source, span)
        .with_expected("readable AX source file or project manifest")
        .with_suggestion("pass an existing `.ax` file, project directory, or `AX.toml` path");

    if ai {
        let contract = AiRepairContract::source_input();
        diagnostic = diagnostic.with_ai(AiDiagnostic {
            rule_id: "input_target_must_be_readable".to_string(),
            layer: contract.layer,
            ai_action: contract.ai_action,
            safe_to_edit: contract.safe_to_edit,
            validation: contract.validation,
            teaching_level: TeachingLevel::L1,
            repeat_count: 1,
            repair_goal:
                "Point the command at a readable AX source file, project directory, or AX.toml manifest."
                    .to_string(),
            focus_item: None,
            relevant_spans: vec![span],
            related_symbols: Vec::new(),
            rule_card: AiRuleCard {
                summary: "AX commands need a readable input target before lexer, parser, semantic, run, or build stages can start.".to_string(),
                pattern: Some("axc check examples/hello.ax".to_string()),
                minimal_example: Some("axc check path/to/project".to_string()),
                anti_pattern: Some("axc check missing/file.ax".to_string()),
            },
            fixits: vec![
                "pass an existing `.ax` file, project directory, or `AX.toml` path".to_string(),
            ],
            context_snippets: Vec::new(),
        });
    }

    diagnostic
}

pub(in crate::cli) fn package_load_error_diagnostic(
    path: &Path,
    error: &str,
    ai: bool,
) -> Option<(SourceFile, Diagnostic)> {
    let base_message = error.split("\nrepair_rule:").next().unwrap_or(error).trim();
    let code = package_error_code(base_message)?;
    let hint = package_repair_hint(code)?;
    let source = source_for_load_error(path);
    let span = first_source_span(&source);
    let mut diagnostic = Diagnostic::new(code, base_message, &source, span)
        .with_expected("valid local path package graph or installed registry package graph")
        .with_suggestion(hint.fixit);

    if ai {
        let contract = AiRepairContract::source_input();
        diagnostic = diagnostic.with_ai(AiDiagnostic {
            rule_id: hint.rule_id.to_string(),
            layer: contract.layer,
            ai_action: contract.ai_action,
            safe_to_edit: contract.safe_to_edit,
            validation: contract.validation,
            teaching_level: TeachingLevel::L1,
            repeat_count: 1,
            repair_goal: hint.repair_goal.to_string(),
            focus_item: None,
            relevant_spans: vec![span],
            related_symbols: Vec::new(),
            rule_card: AiRuleCard {
                summary: hint.repair_goal.to_string(),
                pattern: Some("AX package manifests must describe dependencies the current package loader can materialize into source modules.".to_string()),
                minimal_example: Some("[dependencies]\nconfig_rules = { path = \"packages/config_rules\" }\ntext_tools = { registry = \"ax\", version = \"0.1.0\" }".to_string()),
                anti_pattern: Some("Pointing a dependency alias at a missing directory, invalid manifest, duplicate module root, unresolved registry package, or transitive package graph.".to_string()),
            },
            fixits: vec![hint.fixit.to_string()],
            context_snippets: Vec::new(),
        });
    }

    Some((source, diagnostic))
}

pub(in crate::cli) fn package_error_code(message: &str) -> Option<&str> {
    let code = message.get(0..6)?;
    if code.len() == 6 && code.starts_with("PX") && code[2..].chars().all(|ch| ch.is_ascii_digit())
    {
        return Some(code);
    }
    None
}

pub(in crate::cli) fn source_for_load_error(path: &Path) -> SourceFile {
    let source_path = if path.is_dir() {
        path.join("AX.toml")
    } else {
        path.to_path_buf()
    };
    let text = fs::read_to_string(&source_path).unwrap_or_default();
    SourceFile::new(source_path, text)
}

pub(in crate::cli) fn first_source_span(source: &SourceFile) -> Span {
    let start = source
        .text()
        .char_indices()
        .find(|(_, ch)| !ch.is_whitespace())
        .map(|(index, _)| index)
        .unwrap_or(0);
    let end = source
        .text()
        .get(start..)
        .and_then(|rest| rest.chars().next())
        .map(|ch| start + ch.len_utf8())
        .unwrap_or(start);
    Span::new(start, end)
}
