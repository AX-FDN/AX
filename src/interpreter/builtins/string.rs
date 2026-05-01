use crate::diagnostics::Diagnostic;
use crate::source::Span;

use super::super::Interpreter;
use super::super::value::Value;

impl<'a> Interpreter<'a> {
    pub(super) fn call_string_len_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0037",
                format!(
                    "function `string_len` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let text = arguments
            .into_iter()
            .next()
            .expect("string_len argument should exist");
        return match text {
            Value::String(text) => Ok(Value::I32(text.chars().count() as i32)),
            other => Err(self
                .runtime_error(
                    "R0038",
                    format!(
                        "function `string_len` requires a `string` argument, got `{}`",
                        other.display()
                    ),
                    span,
                )
                .with_suggestion(
                    "call `string_len` with a string value like `string_len(message)`",
                )),
        };
    }

    pub(super) fn call_string_contains_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 2 {
            return Err(self.runtime_error(
                "R0043",
                format!(
                    "function `string_contains` expected 2 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let mut arguments = arguments.into_iter();
        let text = arguments
            .next()
            .expect("string_contains text argument should exist");
        let needle = arguments
            .next()
            .expect("string_contains needle argument should exist");
        return match (text, needle) {
            (Value::String(text), Value::String(needle)) => Ok(Value::Bool(text.contains(&needle))),
            (text, needle) => Err(self
                .runtime_error(
                    "R0044",
                    format!(
                        "function `string_contains` requires `string` arguments, got `{}` and `{}`",
                        text.display(),
                        needle.display()
                    ),
                    span,
                )
                .with_suggestion("call `string_contains` like `string_contains(text, needle)`")),
        };
    }

    pub(super) fn call_string_starts_with_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 2 {
            return Err(self.runtime_error(
                "R0062",
                format!(
                    "function `string_starts_with` expected 2 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let mut arguments = arguments.into_iter();
        let text = arguments
            .next()
            .expect("string_starts_with text argument should exist");
        let prefix = arguments
            .next()
            .expect("string_starts_with prefix argument should exist");
        return match (text, prefix) {
                (Value::String(text), Value::String(prefix)) => {
                    Ok(Value::Bool(text.starts_with(&prefix)))
                }
                (text, prefix) => Err(self
                    .runtime_error(
                        "R0063",
                        format!(
                            "function `string_starts_with` requires `string` arguments, got `{}` and `{}`",
                            text.display(),
                            prefix.display()
                        ),
                        span,
                    )
                    .with_suggestion(
                        "call `string_starts_with` like `string_starts_with(text, prefix)`",
                    )),
            };
    }

    pub(super) fn call_string_ends_with_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 2 {
            return Err(self.runtime_error(
                "R0064",
                format!(
                    "function `string_ends_with` expected 2 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let mut arguments = arguments.into_iter();
        let text = arguments
            .next()
            .expect("string_ends_with text argument should exist");
        let suffix = arguments
            .next()
            .expect("string_ends_with suffix argument should exist");
        return match (text, suffix) {
                (Value::String(text), Value::String(suffix)) => {
                    Ok(Value::Bool(text.ends_with(&suffix)))
                }
                (text, suffix) => Err(self
                    .runtime_error(
                        "R0065",
                        format!(
                            "function `string_ends_with` requires `string` arguments, got `{}` and `{}`",
                            text.display(),
                            suffix.display()
                        ),
                        span,
                    )
                    .with_suggestion(
                        "call `string_ends_with` like `string_ends_with(text, suffix)`",
                    )),
            };
    }

    pub(super) fn call_string_replace_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 3 {
            return Err(self.runtime_error(
                "R0066",
                format!(
                    "function `string_replace` expected 3 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let mut arguments = arguments.into_iter();
        let text = arguments
            .next()
            .expect("string_replace text argument should exist");
        let from = arguments
            .next()
            .expect("string_replace from argument should exist");
        let to = arguments
            .next()
            .expect("string_replace to argument should exist");
        return match (text, from, to) {
                (Value::String(text), Value::String(from), Value::String(to)) => {
                    Ok(Value::String(text.replace(&from, &to)))
                }
                (text, from, to) => Err(self
                    .runtime_error(
                        "R0067",
                        format!(
                            "function `string_replace` requires `string` arguments, got `{}`, `{}`, and `{}`",
                            text.display(),
                            from.display(),
                            to.display()
                        ),
                        span,
                    )
                    .with_suggestion(
                        "call `string_replace` like `string_replace(text, from, to)`",
                    )),
            };
    }

    pub(super) fn call_string_trim_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0124",
                format!(
                    "function `string_trim` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let text = arguments
            .into_iter()
            .next()
            .expect("string_trim argument should exist");
        return match text {
            Value::String(text) => Ok(Value::String(text.trim().to_string())),
            other => Err(self
                .runtime_error(
                    "R0125",
                    format!(
                        "function `string_trim` requires a `string` argument, got `{}`",
                        other.display()
                    ),
                    span,
                )
                .with_suggestion(
                    "call `string_trim` with a string value like `string_trim(text)`",
                )),
        };
    }

    pub(super) fn call_string_split_lines_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0126",
                format!(
                    "function `string_split_lines` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let text = arguments
            .into_iter()
            .next()
            .expect("string_split_lines argument should exist");
        return match text {
            Value::String(text) => Ok(Value::Slice(
                text.lines()
                    .map(|line| Value::String(line.to_string()))
                    .collect(),
            )),
            other => Err(self
                .runtime_error(
                    "R0127",
                    format!(
                        "function `string_split_lines` requires a `string` argument, got `{}`",
                        other.display()
                    ),
                    span,
                )
                .with_suggestion(
                    "call `string_split_lines` with a string value like `string_split_lines(text)`",
                )),
        };
    }
}
