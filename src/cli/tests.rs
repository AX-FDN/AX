use super::{
    BuildCliOptions, CheckOptions, ContextOptions, LockCliOptions, PkgCliOptions, RunOptions,
    parse_build_args, parse_check_args, parse_context_args, parse_lock_args, parse_pkg_args,
    parse_run_args, render_check_success,
};
use crate::build::BuildEmit;
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
        "--json".to_string(),
    ])
    .expect("build arguments should parse");

    assert_eq!(
        options,
        BuildCliOptions {
            file: PathBuf::from("examples/hello.ax"),
            out_dir: Some(PathBuf::from("artifacts/hello")),
            emit: BuildEmit::Default,
            json: true,
        }
    );
}

#[test]
fn parses_build_emit_exe_options() {
    let options = parse_build_args(vec![
        "examples/hello.ax".to_string(),
        "--emit".to_string(),
        "exe".to_string(),
    ])
    .expect("build arguments should parse");

    assert_eq!(
        options,
        BuildCliOptions {
            file: PathBuf::from("examples/hello.ax"),
            out_dir: None,
            emit: BuildEmit::Exe,
            json: false,
        }
    );
}

#[test]
fn parses_build_no_link_as_ir_emit() {
    let options = parse_build_args(vec![
        "examples/hello.ax".to_string(),
        "--no-link".to_string(),
    ])
    .expect("build arguments should parse");

    assert_eq!(
        options,
        BuildCliOptions {
            file: PathBuf::from("examples/hello.ax"),
            out_dir: None,
            emit: BuildEmit::Ir,
            json: false,
        }
    );
}

#[test]
fn rejects_conflicting_build_emit_and_no_link() {
    let error = parse_build_args(vec![
        "examples/hello.ax".to_string(),
        "--emit".to_string(),
        "exe".to_string(),
        "--no-link".to_string(),
    ])
    .expect_err("build arguments should be rejected");
    assert!(error.contains("`--no-link` cannot be combined"));
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
fn parses_pkg_search_with_default_registry() {
    let options = parse_pkg_args(vec!["search".to_string(), "text".to_string()])
        .expect("pkg search arguments should parse");

    match options {
        PkgCliOptions::Search { query, registry } => {
            assert_eq!(query, "text");
            assert!(registry.ends_with("registry"));
        }
        other => panic!("expected search options, got {other:?}"),
    }
}

#[test]
fn parses_pkg_info_with_explicit_registry() {
    let options = parse_pkg_args(vec![
        "info".to_string(),
        "text_tools".to_string(),
        "--registry".to_string(),
        "registry".to_string(),
    ])
    .expect("pkg info arguments should parse");

    assert_eq!(
        options,
        PkgCliOptions::Info {
            package: "text_tools".to_string(),
            registry: PathBuf::from("registry"),
        }
    );
}

#[test]
fn parses_pkg_tree_project() {
    let options = parse_pkg_args(vec![
        "tree".to_string(),
        "examples/project_package_config".to_string(),
    ])
    .expect("pkg tree arguments should parse");

    assert_eq!(
        options,
        PkgCliOptions::Tree {
            project: PathBuf::from("examples/project_package_config"),
        }
    );
}

#[test]
fn parses_pkg_add_dry_run() {
    let options = parse_pkg_args(vec![
        "add".to_string(),
        "text_tools".to_string(),
        "--dry-run".to_string(),
    ])
    .expect("pkg add dry-run arguments should parse");

    match options {
        PkgCliOptions::Add {
            package,
            project,
            registry,
            dry_run,
        } => {
            assert_eq!(package, "text_tools");
            assert_eq!(project, PathBuf::from("."));
            assert!(registry.ends_with("registry"));
            assert!(dry_run);
        }
        other => panic!("expected add options, got {other:?}"),
    }
}

#[test]
fn parses_pkg_add_with_project() {
    let options = parse_pkg_args(vec![
        "add".to_string(),
        "text_tools".to_string(),
        "examples/project_package_config".to_string(),
    ])
    .expect("pkg add arguments should parse");

    match options {
        PkgCliOptions::Add {
            package,
            project,
            registry,
            dry_run,
        } => {
            assert_eq!(package, "text_tools");
            assert_eq!(project, PathBuf::from("examples/project_package_config"));
            assert!(registry.ends_with("registry"));
            assert!(!dry_run);
        }
        other => panic!("expected add options, got {other:?}"),
    }
}

#[test]
fn parses_pkg_install_dry_run() {
    let options = parse_pkg_args(vec![
        "install".to_string(),
        "examples/project_package_config".to_string(),
        "--dry-run".to_string(),
    ])
    .expect("pkg install dry-run arguments should parse");

    match options {
        PkgCliOptions::Install {
            project,
            registry,
            dry_run,
        } => {
            assert_eq!(project, PathBuf::from("examples/project_package_config"));
            assert!(registry.ends_with("registry"));
            assert!(dry_run);
        }
        other => panic!("expected install options, got {other:?}"),
    }
}

#[test]
fn parses_pkg_install_without_dry_run() {
    let options = parse_pkg_args(vec![
        "install".to_string(),
        "examples/project_package_config".to_string(),
    ])
    .expect("pkg install arguments should parse");

    match options {
        PkgCliOptions::Install {
            project,
            registry,
            dry_run,
        } => {
            assert_eq!(project, PathBuf::from("examples/project_package_config"));
            assert!(registry.ends_with("registry"));
            assert!(!dry_run);
        }
        other => panic!("expected install options, got {other:?}"),
    }
}

#[test]
fn parses_pkg_hash_path() {
    let options = parse_pkg_args(vec!["hash".to_string(), "packages/text_tools".to_string()])
        .expect("pkg hash arguments should parse");

    assert_eq!(
        options,
        PkgCliOptions::Hash {
            path: PathBuf::from("packages/text_tools"),
        }
    );
}

#[test]
fn rejects_unknown_pkg_command() {
    let error =
        parse_pkg_args(vec!["publish".to_string()]).expect_err("pkg arguments should be rejected");
    assert!(error.contains("unknown pkg command"));
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
