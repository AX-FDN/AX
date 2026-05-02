use super::helpers::{
    is_catch_all_pattern, match_pattern_alternatives, pattern_contains_binding, pattern_label,
};
use super::*;

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn analyze_match_cases(
        &mut self,
        match_span: Span,
        scrutinee: &Expr,
        cases: &[MatchCase<'_>],
    ) -> MatchCoverage {
        let scrutinee_type = self.check_expr(scrutinee);

        if cases.is_empty() {
            self.diagnostics.push(
                Diagnostic::new(
                    "S0050",
                    "`match` requires at least one arm",
                    self.info.source,
                    match_span,
                )
                .with_kind(DiagnosticKind::MatchRequiresConcretePattern)
                .with_suggestion("add at least one arm like `value => ...` inside the match"),
            );
        }

        let scrutinee_supported = match &scrutinee_type {
            Type::Bool
            | Type::I32
            | Type::String
            | Type::Struct(_)
            | Type::StructInstance { .. }
            | Type::Enum(_)
            | Type::EnumInstance { .. } => true,
            Type::Error => false,
            _ => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0045",
                        format!(
                            "`match` currently requires `bool`, `i32`, `string`, struct, or enum input, found `{}`",
                            scrutinee_type.describe()
                        ),
                        self.info.source,
                        scrutinee.span,
                    )
                    .with_kind(DiagnosticKind::MatchScrutineeTypeUnsupported)
                    .with_note(
                        "the current AX `match` covers boolean values, integer literals, string literals, struct destructuring, and enum variants",
                    )
                    .with_suggestion(
                        "rewrite this with `if / else`, or change the match input to `bool`, `i32`, `string`, a struct value, or an enum value",
                    ),
                );
                false
            }
        };

        let mut coverage = MatchCoverage {
            scrutinee_type,
            scrutinee_supported,
            wildcard_seen: false,
            concrete_pattern_seen: false,
            seen_bools: HashSet::new(),
            seen_ints: HashSet::new(),
            seen_strings: HashSet::new(),
            seen_variants: HashSet::new(),
            seen_structs: HashSet::new(),
        };

        for (index, case) in cases.iter().enumerate() {
            match &case.pattern.kind {
                MatchPatternKind::Wildcard | MatchPatternKind::Binding { .. } => {
                    if !case.guarded && (coverage.wildcard_seen || index + 1 < cases.len()) {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0048",
                                "the catch-all match arm must appear at most once and only as the final arm",
                                self.info.source,
                                case.pattern.span,
                            )
                            .with_kind(DiagnosticKind::MatchWildcardMustBeLast)
                            .with_suggestion(
                                "keep a single final catch-all arm like `_ => ...` or `value => ...`, or remove it",
                            ),
                        );
                    }
                    if !case.guarded {
                        coverage.wildcard_seen = true;
                    }
                }
                MatchPatternKind::Or { alternatives } => {
                    if alternatives.iter().any(is_catch_all_pattern) {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0048",
                                "catch-all patterns cannot be mixed into a `|` match arm",
                                self.info.source,
                                case.pattern.span,
                            )
                            .with_kind(DiagnosticKind::MatchWildcardMustBeLast)
                            .with_suggestion(
                                "move `_` or a bare binding into its own final match arm",
                            ),
                        );
                    }
                    if alternatives.iter().any(pattern_contains_binding) {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0046",
                                "`|` match arms cannot introduce bindings in the current AX slice",
                                self.info.source,
                                case.pattern.span,
                            )
                            .with_kind(DiagnosticKind::MatchPatternTypeMismatch)
                            .with_suggestion(
                                "use only literal or unit enum variant alternatives, or split binding patterns into separate arms",
                            ),
                        );
                    }
                    coverage.concrete_pattern_seen |= alternatives
                        .iter()
                        .any(|pattern| !matches!(pattern.kind, MatchPatternKind::Error));
                }
                MatchPatternKind::Error => {}
                _ => {
                    coverage.concrete_pattern_seen = true;
                }
            }

            for pattern in match_pattern_alternatives(case.pattern) {
                if !coverage.scrutinee_supported {
                    continue;
                }
                let Some(resolved) = self.resolve_match_pattern(pattern, &coverage.scrutinee_type)
                else {
                    continue;
                };
                if case.guarded {
                    continue;
                }
                match resolved {
                    ResolvedMatchPattern::Bool(value) => {
                        if !coverage.seen_bools.insert(value) {
                            self.report_duplicate_match_pattern(
                                pattern.span,
                                pattern_label(pattern),
                            );
                        }
                    }
                    ResolvedMatchPattern::Int(value) => {
                        if !coverage.seen_ints.insert(value) {
                            self.report_duplicate_match_pattern(
                                pattern.span,
                                pattern_label(pattern),
                            );
                        }
                    }
                    ResolvedMatchPattern::String(value) => {
                        if !coverage.seen_strings.insert(value.clone()) {
                            self.report_duplicate_match_pattern(
                                pattern.span,
                                pattern_label(pattern),
                            );
                        }
                    }
                    ResolvedMatchPattern::EnumVariant { variant } => {
                        if !coverage.seen_variants.insert(variant.clone()) {
                            self.report_duplicate_match_pattern(
                                pattern.span,
                                pattern_label(pattern),
                            );
                        }
                    }
                    ResolvedMatchPattern::Struct { name } => {
                        if !coverage.seen_structs.insert(name.clone()) {
                            self.report_duplicate_match_pattern(
                                pattern.span,
                                pattern_label(pattern),
                            );
                        }
                    }
                }
            }
        }

        if !coverage.concrete_pattern_seen {
            self.diagnostics.push(
                Diagnostic::new(
                    "S0050",
                    "`match` requires at least one concrete pattern before an optional `_` arm",
                    self.info.source,
                    match_span,
                )
                .with_kind(DiagnosticKind::MatchRequiresConcretePattern)
                .with_note(
                    "a catch-all-only match does not establish a stable, typed branch set",
                )
                .with_suggestion(
                    "add at least one literal or enum-variant pattern before the catch-all arm, or replace the whole construct with a normal block",
                ),
            );
        }

        coverage
    }

    pub(super) fn report_match_exhaustiveness(
        &mut self,
        match_span: Span,
        coverage: &MatchCoverage,
    ) {
        if !coverage.scrutinee_supported
            || coverage.wildcard_seen
            || !coverage.concrete_pattern_seen
        {
            return;
        }

        match &coverage.scrutinee_type {
            Type::Bool => {
                if coverage.seen_bools.len() != 2 {
                    let missing = [true, false]
                        .into_iter()
                        .filter(|value| !coverage.seen_bools.contains(value))
                        .map(|value| value.to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0049",
                            format!("non-exhaustive `match`: missing arm(s) for `{missing}`"),
                            self.info.source,
                            match_span,
                        )
                        .with_kind(DiagnosticKind::MatchNotExhaustive)
                        .with_suggestion(
                            "cover the remaining boolean case or add a final `_ => ...` arm",
                        ),
                    );
                }
            }
            Type::I32 => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0049",
                        "non-exhaustive `match`: `i32` matches require a final `_` arm",
                        self.info.source,
                        match_span,
                    )
                    .with_kind(DiagnosticKind::MatchNotExhaustive)
                    .with_note("AX does not allow integer matches to fall through silently")
                    .with_suggestion("add a final `_ => ...` arm to cover the remaining values"),
                );
            }
            Type::String => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0049",
                        "non-exhaustive `match`: `string` matches require a final `_` arm",
                        self.info.source,
                        match_span,
                    )
                    .with_kind(DiagnosticKind::MatchNotExhaustive)
                    .with_note("AX does not allow string matches to fall through silently")
                    .with_suggestion("add a final `_ => ...` arm to cover the remaining values"),
                );
            }
            Type::Enum(enum_name) => {
                let Some(enum_info) = self.info.enums.get(enum_name) else {
                    return;
                };
                let missing = enum_info
                    .variants
                    .keys()
                    .filter(|variant| !coverage.seen_variants.contains(*variant))
                    .cloned()
                    .collect::<Vec<_>>();
                if !missing.is_empty() {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0049",
                            format!(
                                "non-exhaustive `match`: missing enum arm(s) for {}",
                                missing.join(", ")
                            ),
                            self.info.source,
                            match_span,
                        )
                        .with_kind(DiagnosticKind::MatchNotExhaustive)
                        .with_suggestion(
                            "cover the remaining enum variants or add a final `_ => ...` arm",
                        ),
                    );
                }
            }
            Type::EnumInstance { name, .. } => {
                let Some(enum_info) = self.info.enums.get(name) else {
                    return;
                };
                let missing = enum_info
                    .variants
                    .keys()
                    .filter(|variant| !coverage.seen_variants.contains(*variant))
                    .cloned()
                    .collect::<Vec<_>>();
                if !missing.is_empty() {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0049",
                            format!(
                                "non-exhaustive `match`: missing enum arm(s) for {}",
                                missing.join(", ")
                            ),
                            self.info.source,
                            match_span,
                        )
                        .with_kind(DiagnosticKind::MatchNotExhaustive)
                        .with_suggestion(
                            "cover the remaining enum variants or add a final `_ => ...` arm",
                        ),
                    );
                }
            }
            Type::Struct(_) | Type::StructInstance { .. } => {}
            _ => {}
        }
    }
}
