use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::source::Span;

use super::super::Interpreter;
use super::super::value::Value;

impl<'a> Interpreter<'a> {
    pub(super) fn call_string_list_new_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if !arguments.is_empty() {
            return Err(self.runtime_error(
                "R0128",
                format!(
                    "function `string_list_new` expected 0 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        return Ok(Value::StringList(Vec::new()));
    }

    pub(super) fn call_string_list_push_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 2 {
            return Err(self.runtime_error(
                "R0129",
                format!(
                    "function `string_list_push` expected 2 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let mut arguments = arguments.into_iter();
        let list = arguments
            .next()
            .expect("string_list_push list argument should exist");
        let value = arguments
            .next()
            .expect("string_list_push value argument should exist");
        return match (list, value) {
                (Value::StringList(mut values), Value::String(value)) => {
                    values.push(value);
                    Ok(Value::StringList(values))
                }
                (list, value) => Err(self
                    .runtime_error(
                        "R0130",
                        format!(
                            "function `string_list_push` requires `string_list` and `string` arguments, got `{}` and `{}`",
                            list.display(),
                            value.display()
                        ),
                        span,
                    )
                    .with_suggestion(
                        "call `string_list_push` like `items = string_list_push(items, \"value\")`",
                    )),
            };
    }

    pub(super) fn call_string_list_join_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 2 {
            return Err(self.runtime_error(
                "R0131",
                format!(
                    "function `string_list_join` expected 2 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let mut arguments = arguments.into_iter();
        let list = arguments
            .next()
            .expect("string_list_join list argument should exist");
        let separator = arguments
            .next()
            .expect("string_list_join separator argument should exist");
        return match (list, separator) {
                (Value::StringList(values), Value::String(separator)) => {
                    Ok(Value::String(values.join(&separator)))
                }
                (list, separator) => Err(self
                    .runtime_error(
                        "R0132",
                        format!(
                            "function `string_list_join` requires `string_list` and `string` arguments, got `{}` and `{}`",
                            list.display(),
                            separator.display()
                        ),
                        span,
                    )
                    .with_suggestion(
                        "call `string_list_join` like `string_list_join(items, \"\\n\")`",
                    )),
            };
    }

    pub(super) fn call_string_list_get_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 2 {
            return Err(self.runtime_error(
                "R0141",
                format!(
                    "function `string_list_get` expected 2 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let mut arguments = arguments.into_iter();
        let list = arguments
            .next()
            .expect("string_list_get list argument should exist");
        let index = arguments
            .next()
            .expect("string_list_get index argument should exist");
        return match (list, index) {
                (Value::StringList(values), Value::I32(index)) => {
                    if index < 0 {
                        return Err(self
                            .runtime_error_with_kind(
                                "R0142",
                                format!("string_list index `{index}` must not be negative"),
                                span,
                                DiagnosticKind::StringListIndexNegative,
                            )
                            .with_suggestion(
                                "check the index before calling `string_list_get(list, index)`",
                            ));
                    }
                    let Some(value) = values.get(index as usize) else {
                        return Err(self
                            .runtime_error_with_kind(
                                "R0143",
                                format!(
                                    "string_list index `{index}` is out of bounds for length {}",
                                    values.len()
                                ),
                                span,
                                DiagnosticKind::StringListIndexOutOfBounds,
                            )
                            .with_suggestion(
                                "check `index < len(list)` before calling `string_list_get(list, index)`",
                            ));
                    };
                    Ok(Value::String(value.clone()))
                }
                (list, index) => Err(self
                    .runtime_error(
                        "R0144",
                        format!(
                            "function `string_list_get` requires `string_list` and `i32` arguments, got `{}` and `{}`",
                            list.display(),
                            index.display()
                        ),
                        span,
                    )
                    .with_suggestion(
                        "call `string_list_get` like `string_list_get(items, 0)`",
                    )),
            };
    }
}
