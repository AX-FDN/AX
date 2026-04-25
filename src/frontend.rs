use crate::ast::Program;
use crate::diagnostics::Diagnostic;
use crate::hir::{Program as HirProgram, lower_program};
use crate::lexer::tokenize;
use crate::mir::{Program as MirProgram, lower_program as lower_mir_program};
use crate::parser::parse;
use crate::project::Project;
use crate::semantic::check_program_with_project;
use crate::source::{SourceFile, Span};

pub struct FrontendOutput {
    pub program: Program,
    pub hir: Option<HirProgram>,
    pub mir: Option<MirProgram>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn analyze(source: &SourceFile) -> FrontendOutput {
    analyze_with_project(source, None)
}

pub fn analyze_with_project(source: &SourceFile, project: Option<&Project>) -> FrontendOutput {
    let mut output = check_only_with_project(source, project);

    if output.diagnostics.is_empty() {
        match lower_program(source, &output.program) {
            Ok(lowered) => match lower_mir_program(&lowered) {
                Ok(lowered_mir) => {
                    output.mir = Some(lowered_mir);
                    output.hir = Some(lowered);
                }
                Err(error) => output.diagnostics.push(Diagnostic::new(
                    "M0001",
                    error,
                    source,
                    Span::new(0, 0),
                )),
            },
            Err(diagnostic) => output.diagnostics.push(diagnostic),
        }
    }

    output
}

pub fn check_only_with_project(source: &SourceFile, project: Option<&Project>) -> FrontendOutput {
    let lexer_output = tokenize(source);
    let parse_output = parse(source, lexer_output.tokens);

    let mut diagnostics = lexer_output.diagnostics;
    diagnostics.extend(parse_output.diagnostics);

    if diagnostics.is_empty() {
        diagnostics.extend(check_program_with_project(
            source,
            &parse_output.program,
            project,
        ));
    }

    diagnostics.sort_by(|left, right| {
        left.span
            .start
            .cmp(&right.span.start)
            .then(left.span.end.cmp(&right.span.end))
            .then(left.code.cmp(&right.code))
    });

    FrontendOutput {
        program: parse_output.program,
        hir: None,
        mir: None,
        diagnostics,
    }
}
