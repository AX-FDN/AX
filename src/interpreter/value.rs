use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Value {
    I32(i32),
    F32(f32),
    Bool(bool),
    String(String),
    StringList(Vec<String>),
    Array(Vec<Value>),
    Slice(Vec<Value>),
    Enum {
        name: String,
        variant: String,
        payload: Option<Box<Value>>,
    },
    Struct {
        name: String,
        fields: BTreeMap<String, Value>,
    },
    Void,
}

impl Value {
    pub(super) fn display(&self) -> String {
        match self {
            Self::I32(value) => value.to_string(),
            Self::F32(value) => {
                let mut text = value.to_string();
                if !text.contains('.') && !text.contains('e') && !text.contains('E') {
                    text.push_str(".0");
                }
                text
            }
            Self::Bool(value) => value.to_string(),
            Self::String(value) => value.clone(),
            Self::StringList(values) => {
                let values = values.join(", ");
                format!("[{values}]")
            }
            Self::Array(elements) => {
                let elements = elements
                    .iter()
                    .map(Value::display)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{elements}]")
            }
            Self::Slice(elements) => {
                let elements = elements
                    .iter()
                    .map(Value::display)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{elements}]")
            }
            Self::Enum {
                name,
                variant,
                payload,
            } => match payload {
                Some(payload) => format!("{name}.{variant}({})", payload.display()),
                None => format!("{name}.{variant}"),
            },
            Self::Struct { name, fields } => {
                let fields = fields
                    .iter()
                    .map(|(field, value)| format!("{field}: {}", value.display()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{name} {{ {fields} }}")
            }
            Self::Void => "<void>".to_string(),
        }
    }
}
