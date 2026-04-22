use std::path::PathBuf;

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
    let mut json = false;
    let mut file = None;

    for arg in args {
        if arg == "--json" {
            json = true;
        } else if file.is_none() {
            file = Some(PathBuf::from(arg));
        } else {
            eprintln!("unexpected argument `{arg}`");
            return 2;
        }
    }

    let Some(path) = file else {
        eprintln!("usage: axc check <file> [--json]");
        return 2;
    };

    let source = match SourceFile::from_path(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("failed to read {}: {error}", path.display());
            return 1;
        }
    };

    let output = analyze(&source);
    if output.diagnostics.is_empty() {
        println!("check succeeded: {}", source.display_path());
        return 0;
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&output.diagnostics).expect("diagnostics json should serialize")
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
  check <file> [--json]   Run lexer, parser, and base semantic checks
  ast <file>              Print stable AST JSON
  run <file>              Execute the minimal interpreter
  fmt <file>              Reserved for the upcoming formatter
"
}
