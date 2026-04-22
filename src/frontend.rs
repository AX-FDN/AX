use crate::ast::Program;
use crate::diagnostics::Diagnostic;
use crate::hir::{Program as HirProgram, lower_program};
use crate::lexer::tokenize;
use crate::parser::parse;
use crate::semantic::check_program;
use crate::source::SourceFile;

pub struct FrontendOutput {
    pub program: Program,
    pub hir: Option<HirProgram>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn analyze(source: &SourceFile) -> FrontendOutput {
    let lexer_output = tokenize(source);
    let parse_output = parse(source, lexer_output.tokens);

    let mut diagnostics = lexer_output.diagnostics;
    diagnostics.extend(parse_output.diagnostics);

    let mut hir = None;
    if diagnostics.is_empty() {
        diagnostics.extend(check_program(source, &parse_output.program));
    }

    if diagnostics.is_empty() {
        match lower_program(source, &parse_output.program) {
            Ok(lowered) => {
                hir = Some(lowered);
            }
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
        diagnostics,
    }
}
