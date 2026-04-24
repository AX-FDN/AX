use std::fs;
use std::path::{Path, PathBuf};

use crate::ai::enhance_diagnostics;
use crate::build::{
    BuildOptions, build_input_from_project, build_input_from_source, build_program,
    default_output_dir,
};
use crate::diagnostics::render_diagnostics;
use crate::formatter::format_source;
use crate::frontend::analyze;
use crate::interpreter::{RunContext, run_program_with_context};
use crate::project::{ResolvedInput, resolve_input};

pub fn run_cli(args: Vec<String>) -> i32 {
    let mut args = args.into_iter();
    let _binary = args.next();
    let Some(command) = args.next() else {
        eprintln!("{}", usage());
        return 2;
    };

    let rest = args.collect::<Vec<_>>();
    match command.as_str() {
        "check" => run_check(rest),
        "ast" => run_ast(rest),
        "hir" => run_hir(rest),
        "mir" => run_mir(rest),
        "build" => run_build(rest),
        "run" => run_run(rest),
        "fmt" => run_fmt(rest),
        "--help" | "-h" | "help" => {
            println!("{}", usage());
            0
        }
        _ => {
            eprintln!("unknown command `{command}`\n\n{}", usage());
            2
        }
    }
}

fn run_check(args: Vec<String>) -> i32 {
    let options = match parse_check_args(args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}\nusage: axc check <path> [--json] [--ai] [--ai-session <path>]");
            return 2;
        }
    };

    let input = match load_input(&options.file) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    let source = &input.source;

    let mut output = analyze(source);
    if output.diagnostics.is_empty() {
        println!(
            "{}",
            render_check_success(options.json, &source.display_path())
        );
        return 0;
    }

    if options.ai {
        if let Err(error) = enhance_diagnostics(
            source,
            &output.program,
            &mut output.diagnostics,
            options.ai_session.as_deref(),
        ) {
            eprintln!("{error}");
            return 1;
        }
    }

    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&output.diagnostics)
                .expect("diagnostics json should serialize")
        );
    } else {
        eprintln!("{}", render_diagnostics(source, &output.diagnostics));
    }

    1
}

fn run_ast(args: Vec<String>) -> i32 {
    if args.len() != 1 {
        eprintln!("usage: axc ast <path>");
        return 2;
    }

    let path = PathBuf::from(&args[0]);
    let input = match load_input(&path) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    let source = &input.source;

    let output = analyze(source);
    if !output.diagnostics.is_empty() {
        eprintln!("{}", render_diagnostics(source, &output.diagnostics));
        return 1;
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&output.program).expect("ast json should serialize")
    );
    0
}

fn run_hir(args: Vec<String>) -> i32 {
    if args.len() != 1 {
        eprintln!("usage: axc hir <path>");
        return 2;
    }

    let path = PathBuf::from(&args[0]);
    let input = match load_input(&path) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    let source = &input.source;

    let output = analyze(source);
    if !output.diagnostics.is_empty() {
        eprintln!("{}", render_diagnostics(source, &output.diagnostics));
        return 1;
    }

    let Some(hir) = output.hir else {
        eprintln!("internal error: HIR should be available after a successful analysis");
        return 1;
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&hir).expect("hir json should serialize")
    );
    0
}

fn run_build(args: Vec<String>) -> i32 {
    let options = match parse_build_args(args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}\nusage: axc build <path> [--out-dir <path>]");
            return 2;
        }
    };

    let input = match load_input(&options.file) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    let source = &input.source;

    let output = analyze(source);
    if !output.diagnostics.is_empty() {
        eprintln!("{}", render_diagnostics(source, &output.diagnostics));
        return 1;
    }

    let Some(hir) = output.hir.as_ref() else {
        eprintln!("internal error: HIR should be available after a successful analysis");
        return 1;
    };
    let Some(mir) = output.mir.as_ref() else {
        eprintln!("internal error: MIR should be available after a successful analysis");
        return 1;
    };

    let build_input = match input.project.as_ref() {
        Some(project) => match build_input_from_project(source, project) {
            Ok(input) => input,
            Err(error) => {
                eprintln!("{error}");
                return 1;
            }
        },
        None => match build_input_from_source(source) {
            Ok(input) => input,
            Err(error) => {
                eprintln!("{error}");
                return 1;
            }
        },
    };
    let out_dir = match options.out_dir {
        Some(out_dir) => out_dir,
        None => match default_output_dir(&build_input.target_name) {
            Ok(out_dir) => out_dir,
            Err(error) => {
                eprintln!("{error}");
                return 1;
            }
        },
    };

    let result = match build_program(source, hir, mir, &build_input, &BuildOptions { out_dir }) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };

    println!("build succeeded: {}", result.manifest_path.display());
    0
}

fn run_mir(args: Vec<String>) -> i32 {
    if args.len() != 1 {
        eprintln!("usage: axc mir <path>");
        return 2;
    }

    let path = PathBuf::from(&args[0]);
    let input = match load_input(&path) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    let source = &input.source;

    let output = analyze(source);
    if !output.diagnostics.is_empty() {
        eprintln!("{}", render_diagnostics(source, &output.diagnostics));
        return 1;
    }

    let Some(mir) = output.mir else {
        eprintln!("internal error: MIR should be available after a successful analysis");
        return 1;
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&mir).expect("mir json should serialize")
    );
    0
}

fn run_run(args: Vec<String>) -> i32 {
    let options = match parse_run_args(args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!(
                "{error}\nusage: axc run <path> [--json] [--ai] [--ai-session <path>] [-- <args...>]"
            );
            return 2;
        }
    };

    let input = match load_input(&options.file) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    let source = &input.source;

    let mut output = analyze(source);
    if !output.diagnostics.is_empty() {
        if options.ai {
            if let Err(error) = enhance_diagnostics(
                source,
                &output.program,
                &mut output.diagnostics,
                options.ai_session.as_deref(),
            ) {
                eprintln!("{error}");
                return 1;
            }
        }

        if options.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&output.diagnostics)
                    .expect("run diagnostics json should serialize")
            );
        } else {
            eprintln!("{}", render_diagnostics(source, &output.diagnostics));
        }
        return 1;
    }

    let Some(hir) = output.hir.as_ref() else {
        eprintln!("internal error: HIR should be available after a successful analysis");
        return 1;
    };

    let run_context = match RunContext::from_host(options.argv.clone()) {
        Ok(context) => context,
        Err(error) => {
            eprintln!("failed to capture host context: {error}");
            return 1;
        }
    };

    match run_program_with_context(source, hir, run_context) {
        Ok(result) => {
            for line in result.stdout {
                println!("{line}");
            }
            result.exit_code
        }
        Err(error) => {
            let mut diagnostics = vec![error];
            if options.ai {
                if let Err(error) = enhance_diagnostics(
                    source,
                    &output.program,
                    &mut diagnostics,
                    options.ai_session.as_deref(),
                ) {
                    eprintln!("{error}");
                    return 1;
                }
            }

            if options.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&diagnostics)
                        .expect("run diagnostics json should serialize")
                );
            } else {
                eprintln!("{}", render_diagnostics(source, &diagnostics));
            }
            1
        }
    }
}

fn run_fmt(args: Vec<String>) -> i32 {
    if args.len() != 1 {
        eprintln!("usage: axc fmt <path>");
        return 2;
    }

    let path = PathBuf::from(&args[0]);
    let input = match load_input(&path) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };

    if let Some(project) = input.project.as_ref() {
        return format_project_sources(project.program_source_paths());
    }

    format_single_source(&input.source)
}

fn format_project_sources(paths: Vec<&Path>) -> i32 {
    for path in paths {
        let source = match crate::source::SourceFile::from_path(path) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("failed to read {}: {error}", path.display());
                return 1;
            }
        };

        let status = format_single_source(&source);
        if status != 0 {
            return status;
        }
    }

    0
}

fn format_single_source(source: &crate::source::SourceFile) -> i32 {
    let formatted = match format_source(source) {
        Ok(formatted) => formatted,
        Err(diagnostics) => {
            eprintln!("{}", render_diagnostics(source, &diagnostics));
            return 1;
        }
    };

    if source.text() == formatted {
        println!("already formatted: {}", source.path().display());
        return 0;
    }

    if let Err(error) = fs::write(source.path(), formatted) {
        eprintln!("failed to write {}: {error}", source.path().display());
        return 1;
    }

    println!("formatted: {}", source.path().display());
    0
}

fn usage() -> &'static str {
    "\
axc <command> [options]

Commands:
  check <path> [--json] [--ai] [--ai-session <path>]   Run lexer, parser, and base semantic checks
  ast <path>               Print stable AST JSON
  hir <path>               Print stable HIR JSON
  mir <path>               Print stable MIR JSON
  build <path> [--out-dir <path>]   Emit the build skeleton artifacts for the native backend stage
  run <path> [--json] [--ai] [--ai-session <path>] [-- <args...>]   Execute the minimal interpreter
  fmt <path>               Rewrite the file or project sources to the canonical AX format
"
}

fn render_check_success(json: bool, display_path: &str) -> String {
    if json {
        "[]".to_string()
    } else {
        format!("check succeeded: {display_path}")
    }
}

#[derive(Debug, PartialEq, Eq)]
struct CheckOptions {
    file: PathBuf,
    json: bool,
    ai: bool,
    ai_session: Option<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
struct BuildCliOptions {
    file: PathBuf,
    out_dir: Option<PathBuf>,
}

#[derive(Debug, PartialEq, Eq)]
struct RunOptions {
    file: PathBuf,
    json: bool,
    ai: bool,
    ai_session: Option<PathBuf>,
    argv: Vec<String>,
}

fn parse_check_args(args: Vec<String>) -> Result<CheckOptions, String> {
    let mut json = false;
    let mut ai = false;
    let mut ai_session = None;
    let mut file = None;
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "--json" => {
                json = true;
            }
            "--ai" => {
                ai = true;
            }
            "--ai-session" => {
                let Some(path) = args.get(index + 1) else {
                    return Err("missing path after `--ai-session`".to_string());
                };
                ai_session = Some(PathBuf::from(path));
                index += 1;
            }
            _ if arg.starts_with("--ai-session=") => {
                let path = arg
                    .split_once('=')
                    .map(|(_, value)| value)
                    .unwrap_or_default();
                if path.is_empty() {
                    return Err("missing path after `--ai-session=`".to_string());
                }
                ai_session = Some(PathBuf::from(path));
            }
            _ if file.is_none() => {
                file = Some(PathBuf::from(arg));
            }
            _ => {
                return Err(format!("unexpected argument `{arg}`"));
            }
        }
        index += 1;
    }

    let Some(file) = file else {
        return Err("missing input file for `axc check`".to_string());
    };

    if ai && !json {
        return Err("`--ai` requires `--json`".to_string());
    }

    if ai_session.is_some() && !ai {
        return Err("`--ai-session` requires `--ai`".to_string());
    }

    Ok(CheckOptions {
        file,
        json,
        ai,
        ai_session,
    })
}

fn parse_run_args(args: Vec<String>) -> Result<RunOptions, String> {
    let mut json = false;
    let mut ai = false;
    let mut ai_session = None;
    let mut argv = Vec::new();
    let mut file = None;
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "--" => {
                argv.extend(args[index + 1..].iter().cloned());
                break;
            }
            "--json" => {
                json = true;
            }
            "--ai" => {
                ai = true;
            }
            "--ai-session" => {
                let Some(path) = args.get(index + 1) else {
                    return Err("missing path after `--ai-session`".to_string());
                };
                ai_session = Some(PathBuf::from(path));
                index += 1;
            }
            _ if arg.starts_with("--ai-session=") => {
                let path = arg
                    .split_once('=')
                    .map(|(_, value)| value)
                    .unwrap_or_default();
                if path.is_empty() {
                    return Err("missing path after `--ai-session=`".to_string());
                }
                ai_session = Some(PathBuf::from(path));
            }
            _ if file.is_none() => {
                file = Some(PathBuf::from(arg));
            }
            _ => {
                return Err(format!("unexpected argument `{arg}`"));
            }
        }
        index += 1;
    }

    let Some(file) = file else {
        return Err("missing input file for `axc run`".to_string());
    };

    if ai && !json {
        return Err("`--ai` requires `--json`".to_string());
    }

    if ai_session.is_some() && !ai {
        return Err("`--ai-session` requires `--ai`".to_string());
    }

    Ok(RunOptions {
        file,
        json,
        ai,
        ai_session,
        argv,
    })
}

fn parse_build_args(args: Vec<String>) -> Result<BuildCliOptions, String> {
    let mut out_dir = None;
    let mut file = None;
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "--out-dir" => {
                let Some(path) = args.get(index + 1) else {
                    return Err("missing path after `--out-dir`".to_string());
                };
                out_dir = Some(PathBuf::from(path));
                index += 1;
            }
            _ if arg.starts_with("--out-dir=") => {
                let path = arg
                    .split_once('=')
                    .map(|(_, value)| value)
                    .unwrap_or_default();
                if path.is_empty() {
                    return Err("missing path after `--out-dir=`".to_string());
                }
                out_dir = Some(PathBuf::from(path));
            }
            _ if file.is_none() => {
                file = Some(PathBuf::from(arg));
            }
            _ => {
                return Err(format!("unexpected argument `{arg}`"));
            }
        }
        index += 1;
    }

    let Some(file) = file else {
        return Err("missing input file for `axc build`".to_string());
    };

    Ok(BuildCliOptions { file, out_dir })
}

fn load_input(path: &Path) -> Result<ResolvedInput, String> {
    resolve_input(path)
}

#[cfg(test)]
mod tests {
    use super::{
        BuildCliOptions, CheckOptions, RunOptions, parse_build_args, parse_check_args,
        parse_run_args, render_check_success,
    };
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
}
