use std::fs;

use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::source::Span;

use super::super::Interpreter;
use super::super::value::Value;

impl<'a> Interpreter<'a> {
    pub(super) fn call_fs_is_file_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0097",
                format!(
                    "function `fs_is_file` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let path = arguments
            .into_iter()
            .next()
            .expect("fs_is_file argument should exist");
        return match path {
            Value::String(path) => Ok(Value::Bool(self.resolve_host_path(&path).is_file())),
            other => Err(self
                .runtime_error(
                    "R0098",
                    format!(
                        "function `fs_is_file` requires a `string` path, got `{}`",
                        other.display()
                    ),
                    span,
                )
                .with_suggestion("call `fs_is_file` with a string value like `fs_is_file(path)`")),
        };
    }

    pub(super) fn call_fs_is_dir_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0099",
                format!(
                    "function `fs_is_dir` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let path = arguments
            .into_iter()
            .next()
            .expect("fs_is_dir argument should exist");
        return match path {
            Value::String(path) => Ok(Value::Bool(self.resolve_host_path(&path).is_dir())),
            other => Err(self
                .runtime_error(
                    "R0100",
                    format!(
                        "function `fs_is_dir` requires a `string` path, got `{}`",
                        other.display()
                    ),
                    span,
                )
                .with_suggestion("call `fs_is_dir` with a string value like `fs_is_dir(path)`")),
        };
    }

    pub(super) fn call_fs_exists_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0057",
                format!(
                    "function `fs_exists` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let path = arguments
            .into_iter()
            .next()
            .expect("fs_exists argument should exist");
        return match path {
            Value::String(path) => Ok(Value::Bool(self.resolve_host_path(&path).exists())),
            other => Err(self
                .runtime_error(
                    "R0058",
                    format!(
                        "function `fs_exists` requires a `string` path, got `{}`",
                        other.display()
                    ),
                    span,
                )
                .with_suggestion("call `fs_exists` with a string value like `fs_exists(path)`")),
        };
    }

    pub(super) fn call_fs_file_size_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0101",
                format!(
                    "function `fs_file_size` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let path = arguments
            .into_iter()
            .next()
            .expect("fs_file_size argument should exist");
        return match path {
            Value::String(path) => {
                let resolved = self.resolve_host_path(&path);
                fs::metadata(&resolved)
                        .map_err(|error| {
                            self.runtime_error_with_kind(
                                "R0103",
                                format!("failed to read metadata for `{}`: {error}", resolved.display()),
                                span,
                                DiagnosticKind::ReadableFilePathRequired,
                            )
                            .with_suggestion(
                                "pass an existing file path or guard with `fs_is_file(path)` first",
                            )
                        })
                        .and_then(|metadata| {
                            i32::try_from(metadata.len()).map(Value::I32).map_err(|_| {
                                self.runtime_error(
                                    "R0104",
                                    format!(
                                        "file `{}` is too large to report as `i32` bytes",
                                        resolved.display()
                                    ),
                                    span,
                                )
                                .with_suggestion(
                                    "handle smaller files or widen the AX file-size contract before processing multi-gigabyte assets",
                                )
                            })
                        })
            }
            other => Err(self
                .runtime_error(
                    "R0102",
                    format!(
                        "function `fs_file_size` requires a `string` path, got `{}`",
                        other.display()
                    ),
                    span,
                )
                .with_suggestion(
                    "call `fs_file_size` with a string value like `fs_file_size(path)`",
                )),
        };
    }

    pub(super) fn call_fs_copy_file_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 2 {
            return Err(self.runtime_error(
                "R0084",
                format!(
                    "function `fs_copy_file` expected 2 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let mut arguments = arguments.into_iter();
        let source_path = arguments
            .next()
            .expect("fs_copy_file source argument should exist");
        let destination_path = arguments
            .next()
            .expect("fs_copy_file destination argument should exist");
        return match (source_path, destination_path) {
            (Value::String(source_path), Value::String(destination_path)) => {
                let resolved_source = self.resolve_host_path(&source_path);
                let resolved_destination = self.resolve_host_path(&destination_path);
                fs::copy(&resolved_source, &resolved_destination)
                        .map_err(|error| {
                            self.runtime_error(
                                "R0087",
                                format!(
                                    "failed to copy `{}` to `{}`: {error}",
                                    resolved_source.display(),
                                    resolved_destination.display()
                                ),
                                span,
                            )
                            .with_suggestion(
                                "ensure the source file exists and the destination directory is writable",
                            )
                        })
                        .and_then(|bytes| {
                            i32::try_from(bytes).map(Value::I32).map_err(|_| {
                                self.runtime_error(
                                    "R0086",
                                    format!(
                                        "copied file `{}` is too large to report as `i32` bytes",
                                        resolved_destination.display()
                                    ),
                                    span,
                                )
                                .with_suggestion(
                                    "copy smaller files or change the runtime byte-count contract before handling multi-gigabyte assets",
                                )
                            })
                        })
            }
            (source_path, destination_path) => Err(self
                .runtime_error(
                    "R0085",
                    format!(
                        "function `fs_copy_file` requires `string` arguments, got `{}` and `{}`",
                        source_path.display(),
                        destination_path.display()
                    ),
                    span,
                )
                .with_suggestion(
                    "call `fs_copy_file` like `fs_copy_file(source_path, destination_path)`",
                )),
        };
    }

    pub(super) fn call_fs_rename_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 2 {
            return Err(self.runtime_error(
                "R0105",
                format!(
                    "function `fs_rename` expected 2 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let mut arguments = arguments.into_iter();
        let source_path = arguments
            .next()
            .expect("fs_rename source argument should exist");
        let destination_path = arguments
            .next()
            .expect("fs_rename destination argument should exist");
        return match (source_path, destination_path) {
            (Value::String(source_path), Value::String(destination_path)) => {
                let resolved_source = self.resolve_host_path(&source_path);
                let resolved_destination = self.resolve_host_path(&destination_path);
                fs::rename(&resolved_source, &resolved_destination)
                        .map(|_| Value::Void)
                        .map_err(|error| {
                            self.runtime_error(
                                "R0107",
                                format!(
                                    "failed to rename `{}` to `{}`: {error}",
                                    resolved_source.display(),
                                    resolved_destination.display()
                                ),
                                span,
                            )
                            .with_suggestion(
                                "ensure the source exists and the destination path is writable before calling `fs_rename`",
                            )
                        })
            }
            (source_path, destination_path) => Err(self
                .runtime_error(
                    "R0106",
                    format!(
                        "function `fs_rename` requires `string` arguments, got `{}` and `{}`",
                        source_path.display(),
                        destination_path.display()
                    ),
                    span,
                )
                .with_suggestion(
                    "call `fs_rename` like `fs_rename(source_path, destination_path)`",
                )),
        };
    }

    pub(super) fn call_fs_create_dir_all_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0070",
                format!(
                    "function `fs_create_dir_all` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let path = arguments
            .into_iter()
            .next()
            .expect("fs_create_dir_all argument should exist");
        return match path {
            Value::String(path) => {
                if path.is_empty() {
                    return Ok(Value::Void);
                }

                let resolved = self.resolve_host_path(&path);
                fs::create_dir_all(&resolved).map(|_| Value::Void).map_err(|error| {
                        self.runtime_error(
                            "R0072",
                            format!("failed to create directory `{}`: {error}", resolved.display()),
                            span,
                        )
                        .with_suggestion(
                            "pass a writable directory path or check the parent path before creating it",
                        )
                    })
            }
            other => Err(self
                .runtime_error(
                    "R0071",
                    format!(
                        "function `fs_create_dir_all` requires a `string` path, got `{}`",
                        other.display()
                    ),
                    span,
                )
                .with_suggestion(
                    "call `fs_create_dir_all` with a string value like `fs_create_dir_all(path)`",
                )),
        };
    }

    pub(super) fn call_fs_remove_file_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0108",
                format!(
                    "function `fs_remove_file` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let path = arguments
            .into_iter()
            .next()
            .expect("fs_remove_file argument should exist");
        return match path {
            Value::String(path) => {
                let resolved = self.resolve_host_path(&path);
                fs::remove_file(&resolved).map(|_| Value::Void).map_err(|error| {
                        self.runtime_error(
                            "R0110",
                            format!("failed to remove `{}`: {error}", resolved.display()),
                            span,
                        )
                        .with_suggestion(
                            "guard with `fs_exists(path)` before removing or pass an existing writable file path",
                        )
                    })
            }
            other => Err(self
                .runtime_error(
                    "R0109",
                    format!(
                        "function `fs_remove_file` requires a `string` path, got `{}`",
                        other.display()
                    ),
                    span,
                )
                .with_suggestion(
                    "call `fs_remove_file` with a string value like `fs_remove_file(path)`",
                )),
        };
    }

    pub(super) fn call_fs_remove_dir_all_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0111",
                format!(
                    "function `fs_remove_dir_all` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let path = arguments
            .into_iter()
            .next()
            .expect("fs_remove_dir_all argument should exist");
        return match path {
            Value::String(path) => {
                let resolved = self.resolve_host_path(&path);
                fs::remove_dir_all(&resolved).map(|_| Value::Void).map_err(|error| {
                        self.runtime_error(
                            "R0113",
                            format!(
                                "failed to remove directory tree `{}`: {error}",
                                resolved.display()
                            ),
                            span,
                        )
                        .with_suggestion(
                            "guard with `fs_is_dir(path)` before removing or pass an existing writable directory path",
                        )
                    })
            }
            other => Err(self
                .runtime_error(
                    "R0112",
                    format!(
                        "function `fs_remove_dir_all` requires a `string` path, got `{}`",
                        other.display()
                    ),
                    span,
                )
                .with_suggestion(
                    "call `fs_remove_dir_all` with a string value like `fs_remove_dir_all(path)`",
                )),
        };
    }

    pub(super) fn call_fs_read_dir_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0121",
                format!(
                    "function `fs_read_dir` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let path = arguments
            .into_iter()
            .next()
            .expect("fs_read_dir argument should exist");
        return match path {
            Value::String(path) => {
                let resolved = self.resolve_host_path(&path);
                let mut entries = fs::read_dir(&resolved)
                        .map_err(|error| {
                            self.runtime_error_with_kind(
                                "R0123",
                                format!("failed to read directory `{}`: {error}", resolved.display()),
                                span,
                                DiagnosticKind::ReadableDirectoryPathRequired,
                            )
                            .with_suggestion(
                                "pass an existing readable directory path or guard with `fs_is_dir(path)` first",
                            )
                        })?
                        .map(|entry_result| {
                            entry_result.map(|entry| {
                                Value::String(entry.path().to_string_lossy().into_owned())
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|error| {
                            self.runtime_error_with_kind(
                                "R0123",
                                format!(
                                    "failed while enumerating directory `{}`: {error}",
                                    resolved.display()
                                ),
                                span,
                                DiagnosticKind::ReadableDirectoryPathRequired,
                            )
                            .with_suggestion(
                                "pass an existing readable directory path or guard with `fs_is_dir(path)` first",
                            )
                        })?;

                entries.sort_by(|left, right| match (left, right) {
                    (Value::String(left), Value::String(right)) => left.cmp(right),
                    _ => std::cmp::Ordering::Equal,
                });
                Ok(Value::Slice(entries))
            }
            other => Err(self
                .runtime_error(
                    "R0122",
                    format!(
                        "function `fs_read_dir` requires a `string` path, got `{}`",
                        other.display()
                    ),
                    span,
                )
                .with_suggestion(
                    "call `fs_read_dir` with a string value like `fs_read_dir(path)`",
                )),
        };
    }

    pub(super) fn call_fs_read_to_string_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 1 {
            return Err(self.runtime_error(
                "R0059",
                format!(
                    "function `fs_read_to_string` expected 1 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let path = arguments
            .into_iter()
            .next()
            .expect("fs_read_to_string argument should exist");
        return match path {
            Value::String(path) => {
                let resolved = self.resolve_host_path(&path);
                fs::read_to_string(&resolved).map(Value::String).map_err(|error| {
                        self.runtime_error_with_kind(
                            "R0061",
                            format!("failed to read `{}`: {error}", resolved.display()),
                            span,
                            DiagnosticKind::ReadableFilePathRequired,
                        )
                        .with_suggestion(
                            "pass an existing readable text file path or guard with `fs_exists(path)` first",
                        )
                    })
            }
            other => Err(self
                .runtime_error(
                    "R0060",
                    format!(
                        "function `fs_read_to_string` requires a `string` path, got `{}`",
                        other.display()
                    ),
                    span,
                )
                .with_suggestion(
                    "call `fs_read_to_string` with a string value like `fs_read_to_string(path)`",
                )),
        };
    }

    pub(super) fn call_fs_write_string_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != 2 {
            return Err(self.runtime_error(
                "R0073",
                format!(
                    "function `fs_write_string` expected 2 argument(s), got {}",
                    arguments.len()
                ),
                span,
            ));
        }

        let mut arguments = arguments.into_iter();
        let path = arguments
            .next()
            .expect("fs_write_string path argument should exist");
        let text = arguments
            .next()
            .expect("fs_write_string text argument should exist");
        return match (path, text) {
            (Value::String(path), Value::String(text)) => {
                let resolved = self.resolve_host_path(&path);
                fs::write(&resolved, text).map(|_| Value::Void).map_err(|error| {
                        self.runtime_error(
                            "R0075",
                            format!("failed to write `{}`: {error}", resolved.display()),
                            span,
                        )
                        .with_suggestion(
                            "create the parent directory first with `fs_create_dir_all(path_parent(path))` or choose a writable path",
                        )
                    })
            }
            (path, text) => Err(self
                .runtime_error(
                    "R0074",
                    format!(
                        "function `fs_write_string` requires `string` arguments, got `{}` and `{}`",
                        path.display(),
                        text.display()
                    ),
                    span,
                )
                .with_suggestion("call `fs_write_string` like `fs_write_string(path, text)`")),
        };
    }
}
