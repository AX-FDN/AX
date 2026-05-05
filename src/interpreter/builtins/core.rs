use crate::diagnostics::Diagnostic;
use crate::source::Span;

use super::super::Interpreter;
use super::super::value::Value;

impl<'a> Interpreter<'a> {
    pub(super) fn call_println_builtin(
        &mut self,
        arguments: Vec<Value>,
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        let rendered = arguments
            .into_iter()
            .map(|value| value.display())
            .collect::<Vec<_>>()
            .join(" ");
        self.stdout.push(rendered);
        return Ok(Value::Void);
    }

    pub(super) fn call_len_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0039",
                format!(
                    "function `len` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let value = arguments
            .into_iter()
            .next()
            .expect("len argument should exist");
        return match value {
                Value::String(text) => Ok(Value::I32(text.chars().count() as i32)),
                Value::Bytes(values) => Ok(Value::I32(values.len() as i32)),
                Value::StringList(values) => Ok(Value::I32(values.len() as i32)),
                Value::Array(elements) | Value::Slice(elements) => {
                    Ok(Value::I32(elements.len() as i32))
                }
                other => Err(self
                    .runtime_error(
                        "R0040",
                        format!(
                            "function `len` requires a `string`, `bytes`, `string_list`, array, or slice argument, got `{}`",
                            other.display()
                        ),
                        span,
                    )
                    .with_suggestion(
                        "call `len` with a string, bytes, string list, array, or slice value like `len(values)`",
                    )),
            };
    }

    pub(super) fn call_to_string_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0041",
                format!(
                    "function `to_string` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let value = arguments
            .into_iter()
            .next()
            .expect("to_string argument should exist");
        return match value {
                Value::Void => Err(self
                    .runtime_error(
                        "R0042",
                        "function `to_string` requires a concrete runtime value, got `<void>`",
                        span,
                    )
                    .with_suggestion(
                        "call `to_string` on a string, number, bool, string list, enum, struct, array, or slice value",
                    )),
                other => Ok(Value::String(other.display())),
            };
    }
}
