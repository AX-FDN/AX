use std::fs;
use std::path::{Path, PathBuf};

use crate::ai::{AiDiagnostic, AiRepairContract, AiRuleCard, TeachingLevel, enhance_diagnostics};
use crate::build::{
    BuildOptions, build_input_from_project, build_input_from_source, build_program,
    default_output_dir,
};
use crate::context::{ContextView, render_context_json};
use crate::diagnostics::{Diagnostic, render_diagnostics};
use crate::formatter::format_source;
use crate::frontend::{analyze_with_project, check_only_with_project};
use crate::interpreter::{RunContext, run_program_with_context};
use crate::lockfile::{LockfileCheckReport, check_lockfile, render_lockfile};
use crate::package_diagnostics::{
    append_package_repair_hint, package_repair_hint, render_package_repair_hint,
};
use crate::project::{ResolvedInput, resolve_input};
use crate::source::{SourceFile, Span};

mod commands;
mod options;
mod render;

use self::commands::*;
#[cfg(test)]
use self::options::*;
use self::render::*;

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
        "lock" => run_lock(rest),
        "run" => run_run(rest),
        "fmt" => run_fmt(rest),
        "context" => run_context_command(rest),
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

#[cfg(test)]
#[path = "cli/tests.rs"]
mod tests;
