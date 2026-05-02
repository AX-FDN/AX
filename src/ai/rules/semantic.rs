use crate::diagnostics::DiagnosticKind;

use super::RuleTemplate;

pub(super) fn match_code(code: &str) -> Option<RuleTemplate> {
    match code {
        "S0001" => Some(RULE_UNIQUE_DEFINITION_REQUIRED),
        "S0002" => Some(RULE_UNDEFINED_VARIABLE),
        "S0003" => Some(RULE_IMMUTABLE_ASSIGNMENT),
        "S0004" => Some(RULE_MAIN_REQUIRED),
        "S0005" => Some(RULE_MAIN_SIGNATURE),
        "S0006" => Some(RULE_TYPE_MUST_BE_DECLARED),
        "S0007" => Some(RULE_FUNCTION_MUST_BE_DECLARED),
        "S0008" => Some(RULE_ASSIGNMENT_TARGET_REQUIRED),
        "S0011" => Some(RULE_FUNCTION_NAME_NOT_RUNTIME_VALUE),
        "S0017" => Some(RULE_FUNCTION_ARGUMENT_COUNT_MATCH),
        "S0018" | "S0019" => Some(RULE_CALL_TARGET_MUST_BE_FUNCTION_NAME),
        "S0020" | "S0027" => Some(RULE_STRUCT_FIELD_MUST_EXIST),
        "S0021" => Some(RULE_FIELD_ACCESS_REQUIRES_STRUCT_VALUE),
        "S0022" => Some(RULE_TYPE_MISMATCH),
        "S0023" => Some(RULE_MISSING_RETURN),
        "S0024" => Some(RULE_STRUCT_LITERAL_REQUIRES_STRUCT_TYPE),
        "S0025" => Some(RULE_STRUCT_LITERAL_FIELDS_UNIQUE),
        "S0026" => Some(RULE_STRUCT_LITERAL_FIELDS_COMPLETE),
        "S0028" => Some(RULE_TYPE_NAME_NOT_RUNTIME_VALUE),
        "S0029" => Some(RULE_ENUM_VARIANT_MUST_EXIST),
        "S0030" => Some(RULE_MUTABLE_STRUCT_FIELD_ASSIGNMENT_REQUIRED),
        "S0031" => Some(RULE_FOR_HEADER_CLAUSE_SUPPORTED),
        "S0052" => Some(RULE_FOR_IN_REQUIRES_SEQUENCE_VALUE),
        "S0032" => Some(RULE_NON_EMPTY_ARRAY_LITERAL_REQUIRED),
        "S0033" => Some(RULE_INDEX_BASE_MUST_BE_ARRAY),
        "S0034" => Some(RULE_SLICE_BASE_MUST_BE_ARRAY_OR_SLICE),
        "S0035" => Some(RULE_SLICE_VALUES_ARE_READ_ONLY),
        "S0057" => Some(RULE_BLOCK_MATCH_ARM_MUST_STAY_LINEAR),
        "S0060" => Some(RULE_MATCH_STRUCT_PATTERN_MUST_MATCH_DECLARATION),
        "R0040" => Some(RULE_LEN_BUILTIN_REQUIRES_COUNTABLE_VALUE),
        _ => None,
    }
}

pub(super) fn match_kind(kind: DiagnosticKind) -> Option<RuleTemplate> {
    match kind {
        DiagnosticKind::BreakOutsideLoop => Some(RULE_BREAK_REQUIRES_LOOP_CONTEXT),
        DiagnosticKind::ContinueOutsideLoop => Some(RULE_CONTINUE_REQUIRES_LOOP_CONTEXT),
        DiagnosticKind::MatchScrutineeTypeUnsupported => {
            Some(RULE_MATCH_INPUT_MUST_USE_SUPPORTED_TYPE)
        }
        DiagnosticKind::MatchPatternTypeMismatch => Some(RULE_MATCH_PATTERN_MUST_MATCH_INPUT),
        DiagnosticKind::DuplicateMatchPattern => Some(RULE_MATCH_PATTERNS_MUST_BE_UNIQUE),
        DiagnosticKind::MatchWildcardMustBeLast => Some(RULE_MATCH_WILDCARD_MUST_BE_LAST),
        DiagnosticKind::MatchNotExhaustive => Some(RULE_MATCH_MUST_BE_EXHAUSTIVE),
        DiagnosticKind::MatchRequiresConcretePattern => Some(RULE_MATCH_REQUIRES_CONCRETE_PATTERN),
        DiagnosticKind::MatchExpressionArmTypeMismatch => {
            Some(RULE_MATCH_EXPRESSION_ARMS_MUST_SHARE_TYPE)
        }
        DiagnosticKind::MatchEnumVariantPayloadShapeMismatch => {
            Some(RULE_MATCH_ENUM_VARIANT_PAYLOAD_MUST_MATCH_DECLARATION)
        }
        DiagnosticKind::MatchStructPatternShapeMismatch => {
            Some(RULE_MATCH_STRUCT_PATTERN_MUST_MATCH_DECLARATION)
        }
        DiagnosticKind::MatchGuardTypeMismatch => Some(RULE_MATCH_GUARD_MUST_BE_BOOL),
        DiagnosticKind::MatchRangeMustBeNonEmpty => Some(RULE_MATCH_RANGE_MUST_BE_NON_EMPTY),
        DiagnosticKind::FunctionArgumentTypeMismatch => {
            Some(RULE_FUNCTION_ARGUMENT_TYPE_MUST_MATCH)
        }
        DiagnosticKind::ReturnTypeMismatch => Some(RULE_RETURN_VALUE_MUST_MATCH_DECLARED_TYPE),
        DiagnosticKind::ConditionTypeMismatch => Some(RULE_CONDITION_MUST_BE_BOOL),
        DiagnosticKind::ArrayIndexTypeMismatch => Some(RULE_ARRAY_INDEX_MUST_BE_I32),
        DiagnosticKind::LenBuiltinTypeMismatch => Some(RULE_LEN_BUILTIN_REQUIRES_COUNTABLE_VALUE),
        DiagnosticKind::ForInIterableTypeMismatch => Some(RULE_FOR_IN_REQUIRES_SEQUENCE_VALUE),
        DiagnosticKind::ForInBindingTypeMismatch => {
            Some(RULE_FOR_IN_BINDING_MUST_MATCH_ELEMENT_TYPE)
        }
        DiagnosticKind::EnumVariantPayloadShapeMismatch => {
            Some(RULE_ENUM_VARIANT_PAYLOAD_MUST_MATCH_DECLARATION)
        }
        DiagnosticKind::EnumVariantPayloadTypeMismatch => {
            Some(RULE_ENUM_VARIANT_PAYLOAD_TYPE_MUST_MATCH_DECLARATION)
        }
        DiagnosticKind::TraitReferenceMustResolve => Some(RULE_TRAIT_REFERENCE_MUST_RESOLVE),
        DiagnosticKind::TraitBoundNotSatisfied => Some(RULE_TRAIT_BOUND_MUST_BE_SATISFIED),
        DiagnosticKind::ResultPropagationRequiresResult => {
            Some(RULE_RESULT_PROPAGATION_REQUIRES_RESULT)
        }
        _ => None,
    }
}

pub(super) fn is_main_required_rule(rule: &RuleTemplate) -> bool {
    rule.rule_id == RULE_MAIN_REQUIRED.rule_id
}
#[path = "semantic/composite.rs"]
mod composite;
#[path = "semantic/control_flow.rs"]
mod control_flow;
#[path = "semantic/core.rs"]
mod core;

use self::composite::*;
use self::control_flow::*;
use self::core::*;
