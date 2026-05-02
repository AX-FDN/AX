use super::{check_program, check_program_with_project};
use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::lexer::tokenize;
use crate::parser::parse;
use crate::project::resolve_input;
use crate::source::SourceFile;
use std::fs;
use std::path::PathBuf;

fn check(source_text: &str) -> Vec<String> {
    diagnostics(source_text)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

fn diagnostics(source_text: &str) -> Vec<Diagnostic> {
    let source = SourceFile::anonymous(source_text);
    let tokens = tokenize(&source).tokens;
    let parsed = parse(&source, tokens);
    check_program(&source, &parsed.program)
}

fn project_diagnostics(project_root: &PathBuf) -> Vec<Diagnostic> {
    let resolved = resolve_input(project_root).expect("project should resolve");
    let tokens = tokenize(&resolved.source).tokens;
    let parsed = parse(&resolved.source, tokens);
    check_program_with_project(&resolved.source, &parsed.program, resolved.project.as_ref())
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[path = "tests/basics.rs"]
mod basics;
#[path = "tests/control_flow.rs"]
mod control_flow;
#[path = "tests/diagnostics.rs"]
mod diagnostics;
#[path = "tests/generics_traits.rs"]
mod generics_traits;
#[path = "tests/match_patterns.rs"]
mod match_patterns;
#[path = "tests/module_projects.rs"]
mod module_projects;
#[path = "tests/payload_patterns.rs"]
mod payload_patterns;
