use super::helpers::pattern_label;
use super::*;

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(in crate::semantic::checker) fn check_match_statement(
        &mut self,
        statement: &Stmt,
        scrutinee: &Expr,
        arms: &[MatchArm],
    ) {
        let cases = arms
            .iter()
            .map(|arm| MatchCase {
                pattern: &arm.pattern,
                guarded: arm.guard.is_some(),
            })
            .collect::<Vec<_>>();
        let coverage = self.analyze_match_cases(statement.span, scrutinee, &cases);
        for arm in arms {
            self.check_match_arm_guard(&coverage.scrutinee_type, &arm.pattern, arm.guard.as_ref());
            self.check_match_arm_block(&coverage.scrutinee_type, &arm.pattern, &arm.body);
        }
        self.report_match_exhaustiveness(statement.span, &coverage);
    }

    pub(in crate::semantic::checker) fn check_match_expression(
        &mut self,
        expr: &Expr,
        scrutinee: &Expr,
        arms: &[MatchExprArm],
    ) -> Type {
        let expected_result_type = self.take_expected_type();
        let cases = arms
            .iter()
            .map(|arm| MatchCase {
                pattern: &arm.pattern,
                guarded: arm.guard.is_some(),
            })
            .collect::<Vec<_>>();
        let coverage = self.analyze_match_cases(expr.span, scrutinee, &cases);

        let mut result_type = None::<Type>;
        for arm in arms {
            self.check_match_arm_guard(&coverage.scrutinee_type, &arm.pattern, arm.guard.as_ref());
            let arm_expected_type = expected_result_type.as_ref().or(result_type.as_ref());
            let arm_type = self.check_match_expression_arm(
                &coverage.scrutinee_type,
                &arm.pattern,
                &arm.value,
                arm_expected_type,
            );
            if arm_type.is_error() {
                continue;
            }

            if let Some(expected_type) = expected_result_type.as_ref().or(result_type.as_ref()) {
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

        expected_result_type.or(result_type).unwrap_or(Type::Error)
    }
}
