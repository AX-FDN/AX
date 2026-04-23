use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Type {
    Bool,
    I32,
    F32,
    String,
    Array { element: Box<Type>, length: usize },
    Struct(String),
    Enum(String),
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
            Self::Array { element, length } => format!("[{}; {}]", element.describe(), length),
            Self::Struct(name) | Self::Enum(name) => name.clone(),
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

    fn is_comparable_primitive(&self) -> bool {
        matches!(self, Self::Bool | Self::I32 | Self::F32 | Self::String)
    }

    pub(super) fn is_equality_comparable(&self) -> bool {
        self.is_comparable_primitive()
            || matches!(self, Self::Enum(_))
            || matches!(
                self,
                Self::Array { element, .. } if element.is_equality_comparable()
            )
    }
}

#[derive(Debug, Clone)]
pub(super) struct FunctionSignature {
    pub(super) params: Vec<ParamInfo>,
    pub(super) return_type: Type,
}

#[derive(Debug, Clone)]
pub(super) struct ParamInfo {
    pub(super) name: String,
    pub(super) ty: Type,
}

#[derive(Debug, Clone)]
pub(super) struct StructInfo {
    pub(super) fields: HashMap<String, StructFieldInfo>,
}

#[derive(Debug, Clone)]
pub(super) struct EnumInfo {
    pub(super) variants: HashSet<String>,
}

#[derive(Debug, Clone)]
pub(super) struct StructFieldInfo {
    pub(super) ty: Type,
    pub(super) start: usize,
}
