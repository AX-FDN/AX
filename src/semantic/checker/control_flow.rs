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
    EnumVariant { variant: String },
}

struct MatchCase<'a> {
    pattern: &'a MatchPattern,
}

struct MatchCoverage {
    scrutinee_type: Type,
    scrutinee_supported: bool,
    wildcard_seen: bool,
    concrete_pattern_seen: bool,
    seen_bools: HashSet<bool>,
    seen_ints: HashSet<i32>,
    seen_variants: HashSet<String>,
}

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_break_statement(&mut self, statement: &Stmt) {
        if self.loop_depth > 0 {
            return;
        }

        self.diagnostics.push(
            Diagnostic::new(
                "S0036",
                "`break` may only be used inside `while` or `for` loops",
                self.info.source,
                statement.span,
            )
            .with_kind(DiagnosticKind::BreakOutsideLoop)
            .with_note("AX uses `break;` to exit the nearest enclosing loop early")
            .with_suggestion(
                "move `break;` into a loop body, or use `return ...;` to exit the function",
            ),
        );
    }

    pub(super) fn check_continue_statement(&mut self, statement: &Stmt) {
        if self.loop_depth > 0 {
            return;
        }

        self.diagnostics.push(
            Diagnostic::new(
                "S0044",
                "`continue` may only be used inside `while` or `for` loops",
                self.info.source,
                statement.span,
            )
            .with_kind(DiagnosticKind::ContinueOutsideLoop)
            .with_note("AX uses `continue;` to skip to the next iteration of the nearest loop")
            .with_suggestion(
                "move `continue;` into a loop body, or rewrite the control flow with `if` / `else`",
            ),
        );
    }

    pub(super) fn check_match_statement(
        &mut self,
        statement: &Stmt,
        scrutinee: &Expr,
        arms: &[MatchArm],
    ) {
        let cases = arms
            .iter()
            .map(|arm| MatchCase {
                pattern: &arm.pattern,
            })
            .collect::<Vec<_>>();
        let coverage = self.analyze_match_cases(statement.span, scrutinee, &cases);
        for arm in arms {
            self.check_match_arm_block(&coverage.scrutinee_type, &arm.pattern, &arm.body);
        }
        self.report_match_exhaustiveness(statement.span, &coverage);
    }

    pub(super) fn check_match_expression(
        &mut self,
        expr: &Expr,
        scrutinee: &Expr,
        arms: &[MatchExprArm],
    ) -> Type {
        let cases = arms
            .iter()
            .map(|arm| MatchCase {
                pattern: &arm.pattern,
            })
            .collect::<Vec<_>>();
        let coverage = self.analyze_match_cases(expr.span, scrutinee, &cases);

        let mut result_type = None::<Type>;
        for arm in arms {
            let arm_type =
                self.check_match_expression_arm(&coverage.scrutinee_type, &arm.pattern, &arm.value);
            if arm_type.is_error() {
                continue;
            }

            if let Some(expected_type) = &result_type {
                self.expect_type_match_with_kind(
                    expected_type,
                    &arm_type,
                    arm.value.span,
                    format!(
                        "match expression arm `{}` must produce `{}`, found `{}`",
                        pattern_label(&arm.pattern),
                        expected_type.describe(),
                        arm_type.describe()
                    ),
                    DiagnosticKind::MatchExpressionArmTypeMismatch,
                );
            } else {
                result_type = Some(arm_type);
            }
        }

        self.report_match_exhaustiveness(expr.span, &coverage);

        result_type.unwrap_or(Type::Error)
    }

    pub(super) fn check_return_statement(&mut self, statement: &Stmt, value: Option<&Expr>) {
        let actual_type = match value {
            Some(expr) => self.check_expr(expr),
            None => Type::Void,
        };
        self.expect_type_match_with_kind(
            &self.return_type.clone(),
            &actual_type,
            statement.span,
            return_type_message(&self.return_type, &actual_type),
            DiagnosticKind::ReturnTypeMismatch,
        );
    }

    pub(super) fn check_if_statement(
        &mut self,
        condition: &Expr,
        then_branch: &Block,
        else_branch: Option<&Block>,
    ) {
        self.check_condition("if", condition);
        self.check_block(then_branch);
        if let Some(block) = else_branch {
            self.check_block(block);
        }
    }

    pub(super) fn check_while_statement(&mut self, condition: &Expr, body: &Block) {
        self.check_condition("while", condition);
        self.loop_depth += 1;
        self.check_block(body);
        self.loop_depth -= 1;
    }

    pub(super) fn check_for_statement(
        &mut self,
        initializer: Option<&Stmt>,
        condition: Option<&Expr>,
        step: Option<&Stmt>,
        body: &Block,
    ) {
        self.scopes.push(Default::default());

        if let Some(statement) = initializer {
            self.check_for_header_statement(statement);
        }

        if let Some(condition) = condition {
            self.check_condition("for", condition);
        }

        self.loop_depth += 1;
        self.check_block(body);
        self.loop_depth -= 1;

        if let Some(statement) = step {
            self.check_for_header_statement(statement);
        }

        self.scopes.pop();
    }

    pub(super) fn check_for_in_statement(
        &mut self,
        binding: &ForInBinding,
        iterable: &Expr,
        body: &Block,
    ) {
        let current_unit_path = self.current_unit_path().to_string();
        let binding_type =
            self.info
                .resolve_type_ref(&binding.ty, &current_unit_path, self.diagnostics);
        let iterable_type = self.check_expr(iterable);

        match &iterable_type {
            Type::Array { element, .. } | Type::Slice { element } => {
                self.expect_type_match_with_kind(
                    element,
                    &binding_type,
                    binding.span,
                    format!(
                        "`for in` loop variable `{}` must use element type `{}`, found `{}`",
                        binding.name,
                        element.describe(),
                        binding_type.describe()
                    ),
                    DiagnosticKind::ForInBindingTypeMismatch,
                );
            }
            Type::Error => {}
            _ => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0052",
                        format!(
                            "`for in` currently only supports arrays and slices, found `{}`",
                            iterable_type.describe()
                        ),
                        self.info.source,
                        iterable.span,
                    )
                    .with_kind(DiagnosticKind::ForInIterableTypeMismatch)
                    .with_note(
                        "the first `for in` prototype only iterates `[T; N]` arrays and `[T]` slices",
                    )
                    .with_suggestion(
                        "iterate over an array or slice value, or rewrite this loop as an indexed `for (...)`",
                    ),
                );
            }
        }

        self.scopes.push(Default::default());
        self.declare(
            &binding.name,
            binding_type,
            binding.mutable,
            binding.span.start,
        );
        self.loop_depth += 1;
        self.check_block(body);
        self.loop_depth -= 1;
        self.scopes.pop();
    }

    fn check_for_header_statement(&mut self, statement: &Stmt) {
        match &statement.kind {
            StmtKind::Let { .. } | StmtKind::Assign { .. } | StmtKind::Expr { .. } => {
                self.check_statement(statement);
            }
            _ => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0031",
                        "`for` headers only support `let`, assignment, or expression clauses",
                        self.info.source,
                        statement.span,
                    )
                    .with_suggestion(
                        "use a header like `for (let i: i32 = 0; i < 3; i = i + 1) { ... }`",
                    ),
                );
            }
        }
    }

    fn check_condition(&mut self, keyword: &str, condition: &Expr) {
        let condition_type = self.check_expr(condition);
        self.expect_type_match_with_kind(
            &Type::Bool,
            &condition_type,
            condition.span,
            format!(
                "`{keyword}` condition must be `bool`, found `{}`",
                condition_type.describe()
            ),
            DiagnosticKind::ConditionTypeMismatch,
        );
    }

    fn analyze_match_cases(
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
            Type::Bool | Type::I32 | Type::Enum(_) => true,
            Type::Error => false,
            _ => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0045",
                        format!(
                            "`match` currently requires `bool`, `i32`, or enum input, found `{}`",
                            scrutinee_type.describe()
                        ),
                        self.info.source,
                        scrutinee.span,
                    )
                    .with_kind(DiagnosticKind::MatchScrutineeTypeUnsupported)
                    .with_note(
                        "the current AX `match` only covers boolean values, integer literals, and enum variants",
                    )
                    .with_suggestion(
                        "rewrite this with `if / else`, or change the match input to `bool`, `i32`, or an enum value",
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
            seen_variants: HashSet::new(),
        };

        for (index, case) in cases.iter().enumerate() {
            match &case.pattern.kind {
                MatchPatternKind::Wildcard | MatchPatternKind::Binding { .. } => {
                    if coverage.wildcard_seen || index + 1 < cases.len() {
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
                    coverage.wildcard_seen = true;
                }
                MatchPatternKind::Error => {}
                _ => {
                    coverage.concrete_pattern_seen = true;
                }
            }

            if coverage.scrutinee_supported
                && let Some(resolved) =
                    self.resolve_match_pattern(case.pattern, &coverage.scrutinee_type)
            {
                match resolved {
                    ResolvedMatchPattern::Bool(value) => {
                        if !coverage.seen_bools.insert(value) {
                            self.report_duplicate_match_pattern(
                                case.pattern.span,
                                pattern_label(case.pattern),
                            );
                        }
                    }
                    ResolvedMatchPattern::Int(value) => {
                        if !coverage.seen_ints.insert(value) {
                            self.report_duplicate_match_pattern(
                                case.pattern.span,
                                pattern_label(case.pattern),
                            );
                        }
                    }
                    ResolvedMatchPattern::EnumVariant { variant } => {
                        if !coverage.seen_variants.insert(variant.clone()) {
                            self.report_duplicate_match_pattern(
                                case.pattern.span,
                                pattern_label(case.pattern),
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

    fn report_match_exhaustiveness(&mut self, match_span: Span, coverage: &MatchCoverage) {
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
            _ => {}
        }
    }

    fn resolve_match_pattern(
        &mut self,
        pattern: &MatchPattern,
        scrutinee_type: &Type,
    ) -> Option<ResolvedMatchPattern> {
        match &pattern.kind {
            MatchPatternKind::Wildcard
            | MatchPatternKind::Binding { .. }
            | MatchPatternKind::Error => None,
            MatchPatternKind::Bool { value } => {
                if !matches!(scrutinee_type, Type::Bool | Type::Error) {
                    self.report_match_pattern_type_mismatch(pattern, scrutinee_type);
                    return None;
                }
                Some(ResolvedMatchPattern::Bool(*value))
            }
            MatchPatternKind::Int { value } => {
                let Ok(value) = i32::try_from(*value) else {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0009",
                            "integer literal is out of range for `i32`",
                            self.info.source,
                            pattern.span,
                        )
                        .with_suggestion("use a value that fits in the AX `i32` range"),
                    );
                    return None;
                };
                if !matches!(scrutinee_type, Type::I32 | Type::Error) {
                    self.report_match_pattern_type_mismatch(pattern, scrutinee_type);
                    return None;
                }
                Some(ResolvedMatchPattern::Int(value))
            }
            MatchPatternKind::EnumVariant { path, payload } => {
                let Some((enum_path, variant)) = path.rsplit_once('.') else {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0046",
                            format!(
                                "match pattern `{path}` must be a literal, `_`, or `EnumName.Variant`",
                            ),
                            self.info.source,
                            pattern.span,
                        )
                        .with_kind(DiagnosticKind::MatchPatternTypeMismatch)
                        .with_suggestion(
                            "rewrite this pattern as `true`, `false`, an integer literal, `_`, or `EnumName.Variant`",
                        ),
                    );
                    return None;
                };

                let current_unit_path = self.current_unit_path().to_string();
                let Some(resolved_key) = self.info.resolve_named_type_key(
                    enum_path,
                    &current_unit_path,
                    pattern.span,
                    self.diagnostics,
                ) else {
                    return None;
                };
                let Some(Type::Enum(enum_name)) = self.info.named_types.get(&resolved_key).cloned()
                else {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0046",
                            format!("match pattern `{path}` does not name an enum variant"),
                            self.info.source,
                            pattern.span,
                        )
                        .with_kind(DiagnosticKind::MatchPatternTypeMismatch)
                        .with_suggestion("use a real enum variant like `Flag.On`"),
                    );
                    return None;
                };

                let Some(enum_info) = self.info.enums.get(&enum_name) else {
                    return None;
                };
                let Some(variant_info) = enum_info.variants.get(variant) else {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0029",
                            format!("unknown enum variant `{variant}` for enum `{enum_name}`"),
                            self.info.source,
                            pattern.span,
                        )
                        .with_suggestion("use one of the declared enum variants"),
                    );
                    return None;
                };

                if !matches!(scrutinee_type, Type::Enum(name) if name == &enum_name)
                    && !matches!(scrutinee_type, Type::Error)
                {
                    self.report_match_pattern_type_mismatch(pattern, scrutinee_type);
                    return None;
                }

                match (&variant_info.payload, payload) {
                    (Some(_), None) => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0055",
                                format!(
                                    "match pattern `{path}` must bind or ignore the payload for enum variant `{enum_name}.{variant}`"
                                ),
                                self.info.source,
                                pattern.span,
                            )
                            .with_kind(DiagnosticKind::MatchEnumVariantPayloadShapeMismatch)
                            .with_note(
                                "payload enum variants must appear in patterns as `EnumName.Variant(name)` or `EnumName.Variant(_)` in the current AX slice",
                            )
                            .with_suggestion(format!(
                                "rewrite this arm as `{path}(value) => ...` or `{path}(_) => ...`",
                            )),
                        );
                    }
                    (None, Some(_)) => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0055",
                                format!(
                                    "match pattern `{}` cannot bind a payload because enum variant `{enum_name}.{variant}` has no payload",
                                    pattern_label(pattern)
                                ),
                                self.info.source,
                                pattern.span,
                            )
                            .with_kind(DiagnosticKind::MatchEnumVariantPayloadShapeMismatch)
                            .with_suggestion(format!(
                                "rewrite this arm as `{path} => ...` without payload binding",
                            )),
                        );
                    }
                    _ => {}
                }

                Some(ResolvedMatchPattern::EnumVariant {
                    variant: variant.to_string(),
                })
            }
        }
    }

    fn report_match_pattern_type_mismatch(
        &mut self,
        pattern: &MatchPattern,
        scrutinee_type: &Type,
    ) {
        self.diagnostics.push(
            Diagnostic::new(
                "S0046",
                format!(
                    "match pattern `{}` does not match input type `{}`",
                    pattern_label(pattern),
                    scrutinee_type.describe()
                ),
                self.info.source,
                pattern.span,
            )
            .with_kind(DiagnosticKind::MatchPatternTypeMismatch)
            .with_suggestion(
                "change the pattern so it uses the same type as the match input, or change the matched value",
            ),
        );
    }

    fn report_duplicate_match_pattern(&mut self, span: Span, pattern: String) {
        self.diagnostics.push(
            Diagnostic::new(
                "S0047",
                format!("duplicate match pattern `{pattern}`"),
                self.info.source,
                span,
            )
            .with_kind(DiagnosticKind::DuplicateMatchPattern)
            .with_suggestion("remove the duplicate arm or merge its logic into the earlier arm"),
        );
    }

    fn check_match_arm_block(
        &mut self,
        scrutinee_type: &Type,
        pattern: &MatchPattern,
        body: &Block,
    ) {
        self.scopes.push(HashMap::new());
        self.declare_match_binding(scrutinee_type, pattern);
        self.check_block(body);
        self.scopes.pop();
    }

    fn check_match_expression_arm(
        &mut self,
        scrutinee_type: &Type,
        pattern: &MatchPattern,
        value: &Expr,
    ) -> Type {
        self.scopes.push(HashMap::new());
        self.declare_match_binding(scrutinee_type, pattern);
        let ty = self.check_expr(value);
        self.scopes.pop();
        ty
    }

    fn declare_match_binding(&mut self, scrutinee_type: &Type, pattern: &MatchPattern) {
        match &pattern.kind {
            MatchPatternKind::Binding { name } => {
                self.declare(name, scrutinee_type.clone(), false, pattern.span.start);
            }
            MatchPatternKind::EnumVariant {
                path,
                payload: Some(EnumVariantPayloadPattern::Binding { name }),
            } => {
                if let Some(payload_type) =
                    self.resolve_enum_pattern_payload_type(path, pattern.span)
                {
                    self.declare(name, payload_type, false, pattern.span.start);
                }
            }
            _ => {}
        }
    }

    fn resolve_enum_pattern_payload_type(&mut self, path: &str, span: Span) -> Option<Type> {
        let (enum_path, variant) = path.rsplit_once('.')?;
        let current_unit_path = self.current_unit_path().to_string();
        let resolved_key = self.info.resolve_named_type_key(
            enum_path,
            &current_unit_path,
            span,
            self.diagnostics,
        )?;
        let Type::Enum(enum_name) = self.info.named_types.get(&resolved_key).cloned()? else {
            return None;
        };
        let enum_info = self.info.enums.get(&enum_name)?;
        enum_info.variants.get(variant)?.payload.clone()
    }
}

fn pattern_label(pattern: &MatchPattern) -> String {
    match &pattern.kind {
        MatchPatternKind::Wildcard => "_".to_string(),
        MatchPatternKind::Binding { name } => name.clone(),
        MatchPatternKind::Bool { value } => value.to_string(),
        MatchPatternKind::Int { value } => value.to_string(),
        MatchPatternKind::EnumVariant { path, payload } => match payload {
            Some(EnumVariantPayloadPattern::Wildcard) => format!("{path}(_)"),
            Some(EnumVariantPayloadPattern::Binding { name }) => format!("{path}({name})"),
            None => path.clone(),
        },
        MatchPatternKind::Error => "<invalid-pattern>".to_string(),
    }
}
