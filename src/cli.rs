use std::path::PathBuf;

use crate::ai::enhance_diagnostics;
use crate::diagnostics::render_diagnostics;
use crate::frontend::analyze;
use crate::interpreter::run_program;
use crate::source::SourceFile;

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
        "run" => run_run(rest),
        "fmt" => run_placeholder("fmt", rest),
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
            eprintln!("{error}\nusage: axc check <file> [--json] [--ai] [--ai-session <path>]");
            return 2;
        }
    };

    let source = match SourceFile::from_path(&options.file) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("failed to read {}: {error}", options.file.display());
            return 1;
        }
    };

    let mut output = analyze(&source);
    if output.diagnostics.is_empty() {
        println!("check succeeded: {}", source.display_path());
        return 0;
    }

    if options.ai {
        if let Err(error) = enhance_diagnostics(
            &source,
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
        eprintln!("{}", render_diagnostics(&source, &output.diagnostics));
    }

    1
}

fn run_ast(args: Vec<String>) -> i32 {
    if args.len() != 1 {
        eprintln!("usage: axc ast <file>");
        return 2;
    }

    let path = PathBuf::from(&args[0]);
    let source = match SourceFile::from_path(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("failed to read {}: {error}", path.display());
            return 1;
        }
    };

    let output = analyze(&source);
    if !output.diagnostics.is_empty() {
        eprintln!("{}", render_diagnostics(&source, &output.diagnostics));
        return 1;
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&output.program).expect("ast json should serialize")
    );
    0
}

fn run_run(args: Vec<String>) -> i32 {
    if args.len() != 1 {
        eprintln!("usage: axc run <file>");
        return 2;
    }

    let path = PathBuf::from(&args[0]);
    let source = match SourceFile::from_path(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("failed to read {}: {error}", path.display());
            return 1;
        }
    };

    let output = analyze(&source);
    if !output.diagnostics.is_empty() {
        eprintln!("{}", render_diagnostics(&source, &output.diagnostics));
        return 1;
    }

    match run_program(&source, &output.program) {
        Ok(result) => {
            for line in result.stdout {
                println!("{line}");
            }
            result.exit_code
        }
        Err(error) => {
            eprintln!("{}", render_diagnostics(&source, &[error]));
            1
        }
    }
}

fn run_placeholder(command: &str, args: Vec<String>) -> i32 {
    if args.len() != 1 {
        eprintln!("usage: axc {command} <file>");
        return 2;
    }

    eprintln!(
        "`axc {command}` is reserved but not implemented yet; the current prototype only guarantees `check` and `ast`."
    );
    2
}

fn usage() -> &'static str {
    "\
axc <command> [options]

Commands:
  check <file> [--json] [--ai] [--ai-session <path>]   Run lexer, parser, and base semantic checks
  ast <file>              Print stable AST JSON
  run <file>              Execute the minimal interpreter
  fmt <file>              Reserved for the upcoming formatter
"
}

#[derive(Debug, PartialEq, Eq)]
struct CheckOptions {
    file: PathBuf,
    json: bool,
    ai: bool,
    ai_session: Option<PathBuf>,
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

#[cfg(test)]
mod tests {
    use super::{CheckOptions, parse_check_args};
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
}
