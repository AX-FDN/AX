use std::collections::{HashMap, HashSet};

use crate::ast::{
    Block, EnumVariantPayloadPattern, Expr, ForInBinding, MatchArm, MatchExprArm, MatchPattern,
    MatchPatternKind, Stmt, StmtKind,
};
use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::source::Span;

use super::{Type, TypeChecker, return_type_message};

enum ResolvedMatchPattern {
    Bool(bool),
    Int(i32),
    String(String),
    EnumVariant { variant: String },
    Struct { name: String },
}

struct MatchCase<'a> {
    pattern: &'a MatchPattern,
    guarded: bool,
}

struct MatchCoverage {
    scrutinee_type: Type,
    scrutinee_supported: bool,
    wildcard_seen: bool,
    concrete_pattern_seen: bool,
    seen_bools: HashSet<bool>,
    seen_ints: HashSet<i32>,
    seen_strings: HashSet<String>,
    seen_variants: HashSet<String>,
    seen_structs: HashSet<String>,
}
#[path = "control_flow/arms.rs"]
mod arms;
#[path = "control_flow/coverage.rs"]
mod coverage;
#[path = "control_flow/flow.rs"]
mod flow;
#[path = "control_flow/helpers.rs"]
mod helpers;
#[path = "control_flow/match_entry.rs"]
mod match_entry;
#[path = "control_flow/patterns.rs"]
mod patterns;
