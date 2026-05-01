use crate::ast::{
    BinaryOp, Block, EnumVariant, EnumVariantPayloadPattern, Expr, ExprKind, ForInBinding,
    ImplMethod, ImportDecl, Item, ItemKind, MatchArm, MatchExprArm, MatchPattern, MatchPatternKind,
    ModuleDecl, Param, Program, SourceUnit, Stmt, StmtKind, StructField, StructLiteralField,
    StructPatternField, TraitMethod, TypeParamBound, TypeRef, UnaryOp, Visibility,
};
use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::source::{SourceFile, Span};
use crate::token::{Token, TokenKind};

mod cursor;
mod diagnostics;
mod expressions;
mod items;
mod literals;
mod patterns;
mod statements;

#[cfg(test)]
mod tests;

pub struct ParseOutput {
    pub program: Program,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn parse(source: &SourceFile, tokens: Vec<Token>) -> ParseOutput {
    Parser::new(source, tokens).parse_program()
}

struct Parser<'a> {
    source: &'a SourceFile,
    tokens: Vec<Token>,
    current: usize,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn new(source: &'a SourceFile, tokens: Vec<Token>) -> Self {
        Self {
            source,
            tokens,
            current: 0,
            diagnostics: Vec::new(),
        }
    }

    fn parse_program(mut self) -> ParseOutput {
        let mut items = Vec::new();
        let mut source_units = Vec::new();

        for segment in self.source.segments() {
            source_units.push(self.parse_source_unit(&segment.path, segment.span, &mut items));
        }

        ParseOutput {
            program: Program {
                items,
                source_units,
            },
            diagnostics: self.diagnostics,
        }
    }
}
