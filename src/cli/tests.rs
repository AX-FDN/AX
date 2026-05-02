use super::{
    BuildCliOptions, CheckOptions, ContextOptions, LockCliOptions, RunOptions, parse_build_args,
    parse_check_args, parse_context_args, parse_lock_args, parse_run_args, render_check_success,
};
use crate::context::ContextView;
use std::path::PathBuf;

#[test]
fn parses_ai_check_options() {
    let options = parse_check_args(vec![
        "examples/hello.ax".to_string(),
        "--json".to_string(),
        "--ai".to_string(),
        "--ai-session".to_string(),
        ".ax-ai-session.json".to_string(),
    ])
    .expect("arguments should parse");

    assert_eq!(
        options,
        CheckOptions {
            file: PathBuf::from("examples/hello.ax"),
            json: true,
            ai: true,
            ai_session: Some(PathBuf::from(".ax-ai-session.json")),
        }
    );
}

#[test]
fn rejects_ai_without_json() {
    let error = parse_check_args(vec!["examples/hello.ax".to_string(), "--ai".to_string()])
        .expect_err("arguments should be rejected");
    assert!(error.contains("`--ai` requires `--json`"));
}

#[test]
fn renders_json_success_as_empty_array() {
    assert_eq!(render_check_success(true, "examples/hello.ax"), "[]");
}

#[test]
fn renders_text_success_with_path() {
    assert_eq!(
        render_check_success(false, "examples/hello.ax"),
        "check succeeded: examples/hello.ax"
    );
}

#[test]
fn parses_build_options_with_explicit_out_dir() {
    let options = parse_build_args(vec![
        "examples/hello.ax".to_string(),
        "--out-dir".to_string(),
        "artifacts/hello".to_string(),
    ])
    .expect("build arguments should parse");

    assert_eq!(
        options,
        BuildCliOptions {
            file: PathBuf::from("examples/hello.ax"),
            out_dir: Some(PathBuf::from("artifacts/hello")),
        }
    );
}

#[test]
fn rejects_build_without_input_file() {
    let error = parse_build_args(vec!["--out-dir".to_string(), "build/hello".to_string()])
        .expect_err("build arguments should be rejected");
    assert!(error.contains("missing input file for `axc build`"));
}

#[test]
fn parses_lock_check_options() {
    let options = parse_lock_args(vec![
        "examples/project_package_config".to_string(),
        "--check".to_string(),
    ])
    .expect("lock arguments should parse");

    assert_eq!(
        options,
        LockCliOptions {
            file: PathBuf::from("examples/project_package_config"),
            check: true,
        }
    );
}

#[test]
fn rejects_lock_without_project() {
    let error = parse_lock_args(Vec::new()).expect_err("lock arguments should be rejected");
    assert!(error.contains("missing input project for `axc lock`"));
}

#[test]
fn parses_run_options_with_ai_session() {
    let options = parse_run_args(vec![
        "examples/hello.ax".to_string(),
        "--json".to_string(),
        "--ai".to_string(),
        "--ai-session".to_string(),
        ".ax-ai-session.json".to_string(),
    ])
    .expect("run arguments should parse");

    assert_eq!(
        options,
        RunOptions {
            file: PathBuf::from("examples/hello.ax"),
            json: true,
            ai: true,
            ai_session: Some(PathBuf::from(".ax-ai-session.json")),
            argv: Vec::new(),
        }
    );
}

#[test]
fn parses_run_options_with_program_args() {
    let options = parse_run_args(vec![
        "examples/hello.ax".to_string(),
        "--".to_string(),
        "alpha".to_string(),
        "beta".to_string(),
    ])
    .expect("run arguments should parse");

    assert_eq!(
        options,
        RunOptions {
            file: PathBuf::from("examples/hello.ax"),
            json: false,
            ai: false,
            ai_session: None,
            argv: vec!["alpha".to_string(), "beta".to_string()],
        }
    );
}

#[test]
fn rejects_run_ai_without_json() {
    let error = parse_run_args(vec!["examples/hello.ax".to_string(), "--ai".to_string()])
        .expect_err("run arguments should be rejected");
    assert!(error.contains("`--ai` requires `--json`"));
}

#[test]
fn parses_context_overview_options() {
    let options = parse_context_args(vec![
        "overview".to_string(),
        "examples/project_module_smoke".to_string(),
        "--json".to_string(),
    ])
    .expect("context arguments should parse");

    assert_eq!(
        options,
        ContextOptions {
            view: ContextView::Overview,
            file: PathBuf::from("examples/project_module_smoke"),
            symbol: None,
        }
    );
}

#[test]
fn rejects_unknown_context_view() {
    let error = parse_context_args(vec!["unknown".to_string(), "examples/hello.ax".to_string()])
        .expect_err("unknown context view should be rejected");
    assert!(error.contains("unknown context view"));
}

#[test]
fn parses_context_symbol_options() {
    let options = parse_context_args(vec![
        "symbol".to_string(),
        "examples/project_workspace_search_report".to_string(),
        "lib.file_search.search_path".to_string(),
        "--json".to_string(),
    ])
    .expect("symbol context arguments should parse");

    assert_eq!(
        options,
        ContextOptions {
            view: ContextView::Symbol,
            file: PathBuf::from("examples/project_workspace_search_report"),
            symbol: Some("lib.file_search.search_path".to_string()),
        }
    );
}

#[test]
fn parses_context_flow_options() {
    let options = parse_context_args(vec![
        "flow".to_string(),
        "examples/project_workspace_search_report".to_string(),
        "--json".to_string(),
    ])
    .expect("flow context arguments should parse");

    assert_eq!(
        options,
        ContextOptions {
            view: ContextView::Flow,
            file: PathBuf::from("examples/project_workspace_search_report"),
            symbol: None,
        }
    );
}

#[test]
fn parses_context_impact_options() {
    let options = parse_context_args(vec![
        "impact".to_string(),
        "examples/project_workspace_search_report".to_string(),
        "lib.file_search.search_path".to_string(),
        "--json".to_string(),
    ])
    .expect("impact context arguments should parse");

    assert_eq!(
        options,
        ContextOptions {
            view: ContextView::Impact,
            file: PathBuf::from("examples/project_workspace_search_report"),
            symbol: Some("lib.file_search.search_path".to_string()),
        }
    );
}

#[test]
fn parses_context_evidence_options() {
    let options = parse_context_args(vec![
        "evidence".to_string(),
        "examples/project_workspace_search_report".to_string(),
        "lib.file_search.search_path".to_string(),
        "--json".to_string(),
    ])
    .expect("evidence context arguments should parse");

    assert_eq!(
        options,
        ContextOptions {
            view: ContextView::Evidence,
            file: PathBuf::from("examples/project_workspace_search_report"),
            symbol: Some("lib.file_search.search_path".to_string()),
        }
    );
}
