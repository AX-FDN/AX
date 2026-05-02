use super::*;

pub(super) fn substitute_enum_payload_type(
    payload_type: Type,
    type_params: &[String],
    scrutinee_type: &Type,
) -> Type {
    let Type::EnumInstance { args, .. } = scrutinee_type else {
        return payload_type;
    };
    if type_params.len() != args.len() {
        return payload_type;
    }
    let substitutions = type_params
        .iter()
        .cloned()
        .zip(args.iter().cloned())
        .collect::<HashMap<_, _>>();
    substitute_type_params(&payload_type, &substitutions)
}

pub(super) fn substitute_struct_field_type(
    field_type: Type,
    type_params: &[String],
    scrutinee_type: &Type,
) -> Type {
    let Type::StructInstance { args, .. } = scrutinee_type else {
        return field_type;
    };
    if type_params.len() != args.len() {
        return field_type;
    }
    let substitutions = type_params
        .iter()
        .cloned()
        .zip(args.iter().cloned())
        .collect::<HashMap<_, _>>();
    substitute_type_params(&field_type, &substitutions)
}

pub(super) fn substitute_type_params(ty: &Type, substitutions: &HashMap<String, Type>) -> Type {
    match ty {
        Type::TypeParam(name) => substitutions
            .get(name)
            .cloned()
            .unwrap_or_else(|| ty.clone()),
        Type::Slice { element } => Type::Slice {
            element: Box::new(substitute_type_params(element, substitutions)),
        },
        Type::Array { element, length } => Type::Array {
            element: Box::new(substitute_type_params(element, substitutions)),
            length: *length,
        },
        Type::StructInstance { name, args } => Type::StructInstance {
            name: name.clone(),
            args: args
                .iter()
                .map(|arg| substitute_type_params(arg, substitutions))
                .collect(),
        },
        Type::EnumInstance { name, args } => Type::EnumInstance {
            name: name.clone(),
            args: args
                .iter()
                .map(|arg| substitute_type_params(arg, substitutions))
                .collect(),
        },
        _ => ty.clone(),
    }
}

pub(super) fn pattern_label(pattern: &MatchPattern) -> String {
    match &pattern.kind {
        MatchPatternKind::Wildcard => "_".to_string(),
        MatchPatternKind::Binding { name } => name.clone(),
        MatchPatternKind::Bool { value } => value.to_string(),
        MatchPatternKind::Int { value } => value.to_string(),
        MatchPatternKind::IntRange { start, end } => format!("{start}..={end}"),
        MatchPatternKind::String { value } => format!("{value:?}"),
        MatchPatternKind::EnumVariant { path, payload } => match payload {
            Some(EnumVariantPayloadPattern::Wildcard) => format!("{path}(_)"),
            Some(EnumVariantPayloadPattern::Binding { name }) => format!("{path}({name})"),
            None => path.clone(),
        },
        MatchPatternKind::Struct { path, fields } => {
            let fields = fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!("{path} {{ {fields} }}")
        }
        MatchPatternKind::Or { alternatives } => alternatives
            .iter()
            .map(pattern_label)
            .collect::<Vec<_>>()
            .join(" | "),
        MatchPatternKind::Error => "<invalid-pattern>".to_string(),
    }
}

pub(super) fn match_pattern_alternatives(pattern: &MatchPattern) -> Vec<&MatchPattern> {
    match &pattern.kind {
        MatchPatternKind::Or { alternatives } => alternatives.iter().collect(),
        _ => vec![pattern],
    }
}

pub(super) fn is_catch_all_pattern(pattern: &MatchPattern) -> bool {
    matches!(
        pattern.kind,
        MatchPatternKind::Wildcard | MatchPatternKind::Binding { .. }
    )
}

pub(super) fn pattern_contains_binding(pattern: &MatchPattern) -> bool {
    match &pattern.kind {
        MatchPatternKind::Binding { .. } => true,
        MatchPatternKind::EnumVariant {
            payload: Some(EnumVariantPayloadPattern::Binding { .. }),
            ..
        } => true,
        MatchPatternKind::Struct { .. } => true,
        MatchPatternKind::Or { alternatives } => alternatives.iter().any(pattern_contains_binding),
        _ => false,
    }
}
