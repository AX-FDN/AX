use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Type {
    Bool,
    I32,
    F32,
    String,
    StringList,
    EmptyArrayLiteral,
    Slice { element: Box<Type> },
    Array { element: Box<Type>, length: usize },
    Struct(String),
    StructInstance { name: String, args: Vec<Type> },
    Enum(String),
    EnumInstance { name: String, args: Vec<Type> },
    TypeParam(String),
    Void,
    Error,
}

impl Type {
    pub(super) fn describe(&self) -> String {
        match self {
            Self::Bool => "bool".to_string(),
            Self::I32 => "i32".to_string(),
            Self::F32 => "f32".to_string(),
            Self::String => "string".to_string(),
            Self::StringList => "string_list".to_string(),
            Self::EmptyArrayLiteral => "[]".to_string(),
            Self::Slice { element } => format!("[{}]", element.describe()),
            Self::Array { element, length } => format!("[{}; {}]", element.describe(), length),
            Self::Struct(name) | Self::Enum(name) => name.clone(),
            Self::StructInstance { name, args } | Self::EnumInstance { name, args } => {
                let args = args
                    .iter()
                    .map(Type::describe)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{name}<{args}>")
            }
            Self::TypeParam(name) => name.clone(),
            Self::Void => "<void>".to_string(),
            Self::Error => "<error>".to_string(),
        }
    }

    pub(super) fn is_numeric(&self) -> bool {
        matches!(self, Self::I32 | Self::F32)
    }

    pub(super) fn is_error(&self) -> bool {
        matches!(self, Self::Error)
    }

    pub(super) fn is_assignable_to(&self, expected: &Type) -> bool {
        self == expected
            || matches!(
                (expected, self),
                (Self::Array { length: 0, .. }, Self::EmptyArrayLiteral)
            )
            || matches!(
                (expected, self),
                (Self::Slice { element: expected_element }, Self::Array { element: actual_element, .. })
                    if expected_element.as_ref() == actual_element.as_ref()
            )
            || generic_instance_assignable(self, expected)
    }
}

fn generic_instance_assignable(actual: &Type, expected: &Type) -> bool {
    match (actual, expected) {
        (
            Type::StructInstance {
                name: actual_name,
                args: actual_args,
            },
            Type::StructInstance {
                name: expected_name,
                args: expected_args,
            },
        )
        | (
            Type::EnumInstance {
                name: actual_name,
                args: actual_args,
            },
            Type::EnumInstance {
                name: expected_name,
                args: expected_args,
            },
        ) if actual_name == expected_name && actual_args.len() == expected_args.len() => {
            actual_args
                .iter()
                .zip(expected_args)
                .all(|(actual, expected)| {
                    matches!(actual, Type::TypeParam(_)) || actual.is_assignable_to(expected)
                })
        }
        _ => false,
    }
}

#[derive(Debug, Clone)]
pub(super) struct FunctionSignature {
    pub(super) type_params: Vec<String>,
    pub(super) type_param_bounds: Vec<TypeParamBoundInfo>,
    pub(super) params: Vec<ParamInfo>,
    pub(super) return_type: Type,
}

#[derive(Debug, Clone)]
pub(super) struct TypeParamBoundInfo {
    pub(super) type_param: String,
    pub(super) trait_name: String,
}

#[derive(Debug, Clone)]
pub(super) struct MethodSignature {
    pub(super) function: FunctionSignature,
}

#[derive(Debug, Clone)]
pub(super) struct TraitInfo {
    pub(super) methods: HashMap<String, FunctionSignature>,
}

#[derive(Debug, Clone)]
pub(super) struct ParamInfo {
    pub(super) name: String,
    pub(super) ty: Type,
}

#[derive(Debug, Clone)]
pub(super) struct ConstInfo {
    pub(super) ty: Type,
    pub(super) start: usize,
}

#[derive(Debug, Clone)]
pub(super) struct StructInfo {
    pub(super) type_params: Vec<String>,
    pub(super) fields: HashMap<String, StructFieldInfo>,
}

#[derive(Debug, Clone)]
pub(super) struct EnumInfo {
    pub(super) type_params: Vec<String>,
    pub(super) variants: HashMap<String, EnumVariantInfo>,
}

#[derive(Debug, Clone)]
pub(super) struct EnumVariantInfo {
    pub(super) payload: Option<Type>,
}

#[derive(Debug, Clone)]
pub(super) struct StructFieldInfo {
    pub(super) ty: Type,
    pub(super) start: usize,
}
