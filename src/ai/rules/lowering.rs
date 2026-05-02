use super::RuleTemplate;

pub(super) fn match_code(code: &str) -> Option<RuleTemplate> {
    if is_lowering_code(code, 'H') {
        return Some(RULE_HIR_LOWERING_REQUIRES_COMPILER_ATTENTION);
    }
    if is_lowering_code(code, 'M') {
        return Some(RULE_MIR_LOWERING_REQUIRES_COMPILER_ATTENTION);
    }
    None
}

fn is_lowering_code(code: &str, prefix: char) -> bool {
    let mut chars = code.chars();
    matches!(chars.next(), Some(first) if first == prefix)
        && chars.clone().count() == 4
        && chars.all(|character| character.is_ascii_digit())
}

const RULE_HIR_LOWERING_REQUIRES_COMPILER_ATTENTION: RuleTemplate = RuleTemplate {
    rule_id: "hir_lowering_requires_compiler_attention",
    normalized_pattern: "hir_lowering_requires_compiler_attention",
    repair_goal: "Treat this as a compiler lowering boundary: preserve user logic and report the HIR lowering failure with a minimal reproducer.",
    summary: "AX reached HIR lowering after frontend checks; this failure points at a compiler boundary, not an ordinary source edit.",
    pattern: "fn main() -> i32 { return 0; }",
    minimal_example: "fn main() -> i32 { return 0; }",
    anti_pattern: Some("rewriting valid user logic only to hide a compiler lowering failure"),
    default_fixit: "preserve the source semantics and report the HIR lowering failure with the smallest reproducer",
};

const RULE_MIR_LOWERING_REQUIRES_COMPILER_ATTENTION: RuleTemplate = RuleTemplate {
    rule_id: "mir_lowering_requires_compiler_attention",
    normalized_pattern: "mir_lowering_requires_compiler_attention",
    repair_goal: "Treat this as a compiler lowering boundary: preserve user logic and report the MIR lowering failure with a minimal reproducer.",
    summary: "AX reached MIR lowering after HIR construction; this failure points at a compiler boundary, not an ordinary source edit.",
    pattern: "fn main() -> i32 { return 0; }",
    minimal_example: "fn main() -> i32 { return 0; }",
    anti_pattern: Some("rewriting valid user logic only to hide a compiler lowering failure"),
    default_fixit: "preserve the source semantics and report the MIR lowering failure with the smallest reproducer",
};
