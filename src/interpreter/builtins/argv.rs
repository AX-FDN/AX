use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::source::Span;

use super::super::Interpreter;
use super::super::value::Value;

impl<'a> Interpreter<'a> {
    pub(super) fn call_argv_len_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if !arguments.is_empty() {
            return Err(self.runtime_error(
                "R0045",
                format!(
                    "function `argv_len` expected 0 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        return Ok(Value::I32(self.host.argv.len() as i32));
    }

    pub(super) fn call_argv_get_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0046",
                format!(
                    "function `argv_get` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let index = arguments
            .into_iter()
            .next()
            .expect("argv_get argument should exist");
        return match index {
            Value::I32(index) if index < 0 => Err(self
                .runtime_error_with_kind(
                    "R0048",
                    format!("argv index `{index}` must be non-negative"),
                    span,
                    DiagnosticKind::ArgvIndexNegative,
                )
                .with_note("AX argv positions use zero-based `i32` indices")
                .with_suggestion(
                    "check the length first with `argv_len()` before calling `argv_get(index)`",
                )),
            Value::I32(index) => {
                let index = index as usize;
                self.host.argv.get(index).cloned().map(Value::String).ok_or_else(|| {
                        self.runtime_error_with_kind(
                            "R0048",
                            format!("argv index `{index}` is out of bounds"),
                            span,
                            DiagnosticKind::ArgvIndexOutOfBounds,
                        )
                        .with_note(format!(
                            "the current AX runtime was started with {} argument(s)",
                            self.host.argv.len()
                        ))
                        .with_suggestion(
                            "check the length first with `argv_len()` before calling `argv_get(index)`",
                        )
                    })
            }
            other => Err(self
                .runtime_error(
                    "R0047",
                    format!(
                        "function `argv_get` requires an `i32` index, got `{}`",
                        other.display()
                    ),
                    span,
                )
                .with_suggestion("call `argv_get` with an `i32` value like `argv_get(0)`")),
        };
    }
}
