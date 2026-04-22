use crate::ast::Program;
use crate::diagnostics::Diagnostic;
use crate::hir::{Program as HirProgram, lower_program};
use crate::lexer::tokenize;
use crate::mir::{Program as MirProgram, lower_program as lower_mir_program};
use crate::parser::parse;
use crate::semantic::check_program;
use crate::source::{SourceFile, Span};

pub struct FrontendOutput {
    pub program: Program,
    pub hir: Option<HirProgram>,
    pub mir: Option<MirProgram>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn analyze(source: &SourceFile) -> FrontendOutput {
    let lexer_output = tokenize(source);
    let parse_output = parse(source, lexer_output.tokens);

    let mut diagnostics = lexer_output.diagnostics;
    diagnostics.extend(parse_output.diagnostics);

    let mut hir = None;
    let mut mir = None;
    if diagnostics.is_empty() {
        diagnostics.extend(check_program(source, &parse_output.program));
    }

    if diagnostics.is_empty() {
        match lower_program(source, &parse_output.program) {
            Ok(lowered) => match lower_mir_program(&lowered) {
                Ok(lowered_mir) => {
                    mir = Some(lowered_mir);
                    hir = Some(lowered);
                }
                Err(error) => {
                    diagnostics.push(Diagnostic::new("M0001", error, source, Span::new(0, 0)))
                }
            },
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
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
        hir,
        mir,
        diagnostics,
    }
}
