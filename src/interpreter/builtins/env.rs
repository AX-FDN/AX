use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::source::Span;

use super::super::Interpreter;
use super::super::value::Value;

impl<'a> Interpreter<'a> {
    pub(super) fn call_env_has_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0049",
                format!(
                    "function `env_has` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let name = arguments
            .into_iter()
            .next()
            .expect("env_has argument should exist");
        return match name {
            Value::String(name) => Ok(Value::Bool(self.host.env_contains(&name))),
            other => Err(self
                .runtime_error(
                    "R0050",
                    format!(
                        "function `env_has` requires a `string` name, got `{}`",
                        other.display()
                    ),
                    span,
                )
                .with_suggestion("call `env_has` with a string value like `env_has(\"HOME\")`")),
        };
    }

    pub(super) fn call_env_get_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0051",
                format!(
                    "function `env_get` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let name = arguments
            .into_iter()
            .next()
            .expect("env_get argument should exist");
        return match name {
                Value::String(name) => self
                    .host
                    .env_value(&name)
                    .map(|value| Value::String(value.to_string()))
                    .ok_or_else(|| {
                        self.runtime_error_with_kind(
                            "R0053",
                            format!("environment variable `{name}` is not available"),
                            span,
                            DiagnosticKind::EnvironmentVariableUnavailable,
                        )
                        .with_suggestion(
                            "check the variable first with `env_has(name)` or set it in the host environment",
                        )
                    }),
                other => Err(self
                    .runtime_error(
                        "R0052",
                        format!("function `env_get` requires a `string` name, got `{}`", other.display()),
                        span,
                    )
                    .with_suggestion("call `env_get` with a string value like `env_get(\"HOME\")`")),
            };
    }
}
