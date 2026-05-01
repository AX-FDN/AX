use std::path::{Path, PathBuf};

use crate::diagnostics::Diagnostic;
use crate::source::Span;

use super::super::Interpreter;
use super::super::value::Value;

impl<'a> Interpreter<'a> {
    pub(super) fn call_path_join_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 2 {
            return Err(self.runtime_error(
                "R0055",
                format!(
                    "function `path_join` expected 2 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let mut arguments = arguments.into_iter();
        let left = arguments
            .next()
            .expect("path_join left argument should exist");
        let right = arguments
            .next()
            .expect("path_join right argument should exist");
        return match (left, right) {
            (Value::String(left), Value::String(right)) => {
                let mut path = PathBuf::from(left);
                path.push(right);
                Ok(Value::String(path.to_string_lossy().into_owned()))
            }
            (left, right) => Err(self
                .runtime_error(
                    "R0056",
                    format!(
                        "function `path_join` requires `string` arguments, got `{}` and `{}`",
                        left.display(),
                        right.display()
                    ),
                    span,
                )
                .with_suggestion("call `path_join` like `path_join(base, child)`")),
        };
    }

    pub(super) fn call_path_resolve_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0095",
                format!(
                    "function `path_resolve` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let path = arguments
            .into_iter()
            .next()
            .expect("path_resolve argument should exist");
        return match path {
            Value::String(path) => Ok(Value::String(
                self.resolve_host_path(&path).to_string_lossy().into_owned(),
            )),
            other => Err(self
                .runtime_error(
                    "R0096",
                    format!(
                        "function `path_resolve` requires a `string` path, got `{}`",
                        other.display()
                    ),
                    span,
                )
                .with_suggestion(
                    "call `path_resolve` with a string value like `path_resolve(path)`",
                )),
        };
    }

    pub(super) fn call_path_parent_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0068",
                format!(
                    "function `path_parent` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let path = arguments
            .into_iter()
            .next()
            .expect("path_parent argument should exist");
        return match path {
            Value::String(path) => {
                let parent = Path::new(&path)
                    .parent()
                    .map(|value| value.to_string_lossy().into_owned())
                    .unwrap_or_default();
                Ok(Value::String(parent))
            }
            other => Err(self
                .runtime_error(
                    "R0069",
                    format!(
                        "function `path_parent` requires a `string` path, got `{}`",
                        other.display()
                    ),
                    span,
                )
                .with_suggestion(
                    "call `path_parent` with a string value like `path_parent(path)`",
                )),
        };
    }

    pub(super) fn call_path_file_name_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0076",
                format!(
                    "function `path_file_name` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let path = arguments
            .into_iter()
            .next()
            .expect("path_file_name argument should exist");
        return match path {
            Value::String(path) => {
                let name = Path::new(&path)
                    .file_name()
                    .map(|value| value.to_string_lossy().into_owned())
                    .unwrap_or_default();
                Ok(Value::String(name))
            }
            other => Err(self
                .runtime_error(
                    "R0077",
                    format!(
                        "function `path_file_name` requires a `string` path, got `{}`",
                        other.display()
                    ),
                    span,
                )
                .with_suggestion(
                    "call `path_file_name` with a string value like `path_file_name(path)`",
                )),
        };
    }

    pub(super) fn call_path_stem_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0078",
                format!(
                    "function `path_stem` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let path = arguments
            .into_iter()
            .next()
            .expect("path_stem argument should exist");
        return match path {
            Value::String(path) => {
                let stem = Path::new(&path)
                    .file_stem()
                    .map(|value| value.to_string_lossy().into_owned())
                    .unwrap_or_default();
                Ok(Value::String(stem))
            }
            other => Err(self
                .runtime_error(
                    "R0079",
                    format!(
                        "function `path_stem` requires a `string` path, got `{}`",
                        other.display()
                    ),
                    span,
                )
                .with_suggestion("call `path_stem` with a string value like `path_stem(path)`")),
        };
    }

    pub(super) fn call_path_extension_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0080",
                format!(
                    "function `path_extension` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let path = arguments
            .into_iter()
            .next()
            .expect("path_extension argument should exist");
        return match path {
            Value::String(path) => {
                let extension = Path::new(&path)
                    .extension()
                    .map(|value| value.to_string_lossy().into_owned())
                    .unwrap_or_default();
                Ok(Value::String(extension))
            }
            other => Err(self
                .runtime_error(
                    "R0081",
                    format!(
                        "function `path_extension` requires a `string` path, got `{}`",
                        other.display()
                    ),
                    span,
                )
                .with_suggestion(
                    "call `path_extension` with a string value like `path_extension(path)`",
                )),
        };
    }

    pub(super) fn call_path_is_absolute_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0082",
                format!(
                    "function `path_is_absolute` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let path = arguments
            .into_iter()
            .next()
            .expect("path_is_absolute argument should exist");
        return match path {
            Value::String(path) => Ok(Value::Bool(Path::new(&path).is_absolute())),
            other => Err(self
                .runtime_error(
                    "R0083",
                    format!(
                        "function `path_is_absolute` requires a `string` path, got `{}`",
                        other.display()
                    ),
                    span,
                )
                .with_suggestion(
                    "call `path_is_absolute` with a string value like `path_is_absolute(path)`",
                )),
        };
    }
}
