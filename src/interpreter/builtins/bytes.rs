use crate::diagnostics::Diagnostic;
use crate::source::Span;

use super::super::Interpreter;
use super::super::value::Value;

impl<'a> Interpreter<'a> {
    pub(super) fn call_bytes_empty_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if !arguments.is_empty() {
            return Err(self.runtime_error(
                "R0145",
                format!(
                    "function `bytes_empty` expected 0 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        Ok(Value::Bytes(Vec::new()))
    }

    pub(super) fn call_bytes_from_string_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0146",
                format!(
                    "function `bytes_from_string` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let text = arguments
            .into_iter()
            .next()
            .expect("bytes_from_string argument should exist");
        match text {
            Value::String(text) => Ok(Value::Bytes(text.into_bytes())),
            other => Err(self
                .runtime_error(
                    "R0147",
                    format!(
                        "function `bytes_from_string` requires a `string` argument, got `{}`",
                        other.display()
                    ),
                    span,
                )
                .with_suggestion(
                    "call `bytes_from_string` with a string value like `bytes_from_string(text)`",
                )),
        }
    }

    pub(super) fn call_bytes_to_string_lossy_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        self.call_bytes_to_string_builtin(arguments, span, "bytes_to_string_lossy", |bytes| {
            String::from_utf8_lossy(bytes).into_owned()
        })
    }

    pub(super) fn call_bytes_to_hex_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        self.call_bytes_to_string_builtin(arguments, span, "bytes_to_hex", bytes_to_hex)
    }

    pub(super) fn call_bytes_push_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 2 {
            return Err(self.runtime_error(
                "R0150",
                format!(
                    "function `bytes_push` expected 2 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let mut arguments = arguments.into_iter();
        let data = arguments
            .next()
            .expect("bytes_push data argument should exist");
        let value = arguments
            .next()
            .expect("bytes_push value argument should exist");
        match (data, value) {
            (Value::Bytes(mut bytes), Value::I32(value)) => {
                if !(0..=255).contains(&value) {
                    return Err(self
                        .runtime_error(
                            "R0151",
                            format!("byte value `{value}` must be between 0 and 255"),
                            span,
                        )
                        .with_suggestion(
                            "clamp or validate the value before calling `bytes_push(data, value)`",
                        ));
                }
                bytes.push(value as u8);
                Ok(Value::Bytes(bytes))
            }
            (data, value) => Err(self
                .runtime_error(
                    "R0152",
                    format!(
                        "function `bytes_push` requires `bytes` and `i32` arguments, got `{}` and `{}`",
                        data.display(),
                        value.display()
                    ),
                    span,
                )
                .with_suggestion("call `bytes_push` like `data = bytes_push(data, 65)`")),
        }
    }

    pub(super) fn call_bytes_get_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 2 {
            return Err(self.runtime_error(
                "R0153",
                format!(
                    "function `bytes_get` expected 2 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let mut arguments = arguments.into_iter();
        let data = arguments
            .next()
            .expect("bytes_get data argument should exist");
        let index = arguments
            .next()
            .expect("bytes_get index argument should exist");
        match (data, index) {
            (Value::Bytes(bytes), Value::I32(index)) => {
                if index < 0 {
                    return Err(self
                        .runtime_error(
                            "R0154",
                            format!("bytes index `{index}` must not be negative"),
                            span,
                        )
                        .with_suggestion(
                            "check the index before calling `bytes_get(data, index)`",
                        ));
                }
                let Some(value) = bytes.get(index as usize) else {
                    return Err(self
                        .runtime_error(
                            "R0155",
                            format!(
                                "bytes index `{index}` is out of bounds for length {}",
                                bytes.len()
                            ),
                            span,
                        )
                        .with_suggestion(
                            "check `index < len(data)` before calling `bytes_get(data, index)`",
                        ));
                };
                Ok(Value::I32(i32::from(*value)))
            }
            (data, index) => Err(self
                .runtime_error(
                    "R0156",
                    format!(
                        "function `bytes_get` requires `bytes` and `i32` arguments, got `{}` and `{}`",
                        data.display(),
                        index.display()
                    ),
                    span,
                )
                .with_suggestion("call `bytes_get` like `bytes_get(data, 0)`")),
        }
    }

    fn call_bytes_to_string_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
        name: &str,
        render: fn(&[u8]) -> String,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0148",
                format!(
                    "function `{name}` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let data = arguments
            .into_iter()
            .next()
            .expect("bytes string conversion argument should exist");
        match data {
            Value::Bytes(bytes) => Ok(Value::String(render(&bytes))),
            other => Err(self
                .runtime_error(
                    "R0149",
                    format!(
                        "function `{name}` requires a `bytes` argument, got `{}`",
                        other.display()
                    ),
                    span,
                )
                .with_suggestion(format!("call `{name}` with a bytes value"))),
        }
    }
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}
