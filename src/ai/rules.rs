mod lexer;
mod parser;
mod project;
mod rule_registry;
mod runtime;
mod semantic;

pub(super) use rule_registry::{is_main_required_rule, match_rule};

#[derive(Clone, Copy)]
pub(super) struct RuleTemplate {
    pub(super) rule_id: &'static str,
    pub(super) normalized_pattern: &'static str,
    pub(super) repair_goal: &'static str,
    pub(super) summary: &'static str,
    pub(super) pattern: &'static str,
    pub(super) minimal_example: &'static str,
    pub(super) anti_pattern: Option<&'static str>,
    pub(super) default_fixit: &'static str,
}
