use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::source::Span;

use super::Interpreter;
use super::value::Value;

impl<'a> Interpreter<'a> {
    pub(super) fn call_function(
        &mut self,
        name: &str,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if name == "println" {
            let rendered = arguments
                .into_iter()
                .map(|value| value.display())
                .collect::<Vec<_>>()
                .join(" ");
            self.stdout.push(rendered);
            return Ok(Value::Void);
        }

        if name == "string_len" {
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

        if name == "string_contains" {
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
                (Value::String(text), Value::String(needle)) => {
                    Ok(Value::Bool(text.contains(&needle)))
                }
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
                    .with_suggestion(
                        "call `string_contains` like `string_contains(text, needle)`",
                    )),
            };
        }

        if name == "string_starts_with" {
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

        if name == "string_ends_with" {
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

        if name == "string_replace" {
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

        if name == "string_trim" {
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

        if name == "string_split_lines" {
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

        if name == "string_list_new" {
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

        if name == "string_list_push" {
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

        if name == "string_list_join" {
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

        if name == "string_list_get" {
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

        if name == "len" {
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
                Value::StringList(values) => Ok(Value::I32(values.len() as i32)),
                Value::Array(elements) | Value::Slice(elements) => {
                    Ok(Value::I32(elements.len() as i32))
                }
                other => Err(self
                    .runtime_error(
                        "R0040",
                        format!(
                            "function `len` requires a `string`, `string_list`, array, or slice argument, got `{}`",
                            other.display()
                        ),
                        span,
                    )
                    .with_suggestion(
                        "call `len` with a string, string list, array, or slice value like `len(values)`",
                    )),
            };
        }

        if name == "to_string" {
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

        if name == "argv_len" {
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

        if name == "argv_get" {
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

        if name == "env_has" {
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
                    .with_suggestion(
                        "call `env_has` with a string value like `env_has(\"HOME\")`",
                    )),
            };
        }

        if name == "env_get" {
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

        if name == "process_cwd" {
            if !arguments.is_empty() {
                return Err(self.runtime_error(
                    "R0054",
                    format!(
                        "function `process_cwd` expected 0 argument(s), got {}",
                        arguments.len()
                    ),
                    span,
                ));
            }

            return Ok(Value::String(
                self.host.current_dir.to_string_lossy().into_owned(),
            ));
        }

        if name == "process_run" {
            if arguments.len() != 1 {
                return Err(self.runtime_error(
                    "R0088",
                    format!(
                        "function `process_run` expected 1 argument(s), got {}",
                        arguments.len()
                    ),
                    span,
                ));
            }

            let command_text = arguments
                .into_iter()
                .next()
                .expect("process_run argument should exist");
            return match command_text {
                Value::String(command_text) => self
                    .host_shell_command(&command_text)
                    .status()
                    .map(|status| Value::I32(status.code().unwrap_or(-1)))
                    .map_err(|error| {
                        self.runtime_error_with_kind(
                            "R0090",
                            format!("failed to run `{command_text}`: {error}"),
                            span,
                            DiagnosticKind::ProcessCommandNotLaunchable,
                        )
                        .with_suggestion(
                            "pass a valid shell command string like `process_run(\"echo ready\")`",
                        )
                    }),
                other => Err(self
                    .runtime_error(
                        "R0089",
                        format!(
                            "function `process_run` requires a `string` command, got `{}`",
                            other.display()
                        ),
                        span,
                    )
                    .with_suggestion(
                        "call `process_run` with a string value like `process_run(command)`",
                    )),
            };
        }

        if name == "process_capture" {
            if arguments.len() != 1 {
                return Err(self.runtime_error(
                    "R0091",
                    format!(
                        "function `process_capture` expected 1 argument(s), got {}",
                        arguments.len()
                    ),
                    span,
                ));
            }

            let command_text = arguments
                .into_iter()
                .next()
                .expect("process_capture argument should exist");
            return match command_text {
                Value::String(command_text) => {
                    let output = self
                        .host_shell_command(&command_text)
                        .output()
                        .map_err(|error| {
                            self.runtime_error(
                                "R0093",
                                format!("failed to capture `{command_text}`: {error}"),
                                span,
                            )
                            .with_suggestion(
                                "pass a valid shell command string like `process_capture(\"echo ready\")`",
                            )
                        })?;

                    if !output.status.success() {
                        let exit_code = output.status.code().unwrap_or(-1);
                        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                        let mut diagnostic = self
                            .runtime_error_with_kind(
                                "R0094",
                                format!("command `{command_text}` exited with status {exit_code}"),
                                span,
                                DiagnosticKind::ProcessCaptureNonZeroExit,
                            )
                            .with_suggestion(
                                "use `process_run(command)` when you need to inspect a failing exit code without raising a runtime error",
                            );
                        if !stderr.is_empty() {
                            diagnostic = diagnostic.with_note(format!("stderr: {stderr}"));
                        }
                        return Err(diagnostic);
                    }

                    Ok(Value::String(
                        String::from_utf8_lossy(&output.stdout).into_owned(),
                    ))
                }
                other => Err(self
                    .runtime_error(
                        "R0092",
                        format!(
                            "function `process_capture` requires a `string` command, got `{}`",
                            other.display()
                        ),
                        span,
                    )
                    .with_suggestion(
                        "call `process_capture` with a string value like `process_capture(command)`",
                    )),
            };
        }

        if name == "process_run_in" {
            if arguments.len() != 2 {
                return Err(self.runtime_error(
                    "R0114",
                    format!(
                        "function `process_run_in` expected 2 argument(s), got {}",
                        arguments.len()
                    ),
                    span,
                ));
            }

            let mut arguments = arguments.into_iter();
            let working_dir = arguments
                .next()
                .expect("process_run_in working_dir argument should exist");
            let command_text = arguments
                .next()
                .expect("process_run_in command argument should exist");
            return match (working_dir, command_text) {
                (Value::String(working_dir), Value::String(command_text)) => self
                    .host_shell_command_at(&self.resolve_host_path(&working_dir), &command_text)
                    .status()
                    .map(|status| Value::I32(status.code().unwrap_or(-1)))
                    .map_err(|error| {
                        self.runtime_error_with_kind(
                            "R0116",
                            format!(
                                "failed to run `{command_text}` in `{}`: {error}",
                                self.resolve_host_path(&working_dir).display()
                            ),
                            span,
                            DiagnosticKind::ProcessCommandNotLaunchable,
                        )
                        .with_suggestion(
                            "pass an existing working directory and a valid shell command string like `process_run_in(dir, command)`",
                        )
                    }),
                (working_dir, command_text) => Err(self
                    .runtime_error(
                        "R0115",
                        format!(
                            "function `process_run_in` requires `string` arguments, got `{}` and `{}`",
                            working_dir.display(),
                            command_text.display()
                        ),
                        span,
                    )
                    .with_suggestion(
                        "call `process_run_in` like `process_run_in(working_dir, command)`",
                    )),
            };
        }

        if name == "process_capture_in" {
            if arguments.len() != 2 {
                return Err(self.runtime_error(
                    "R0117",
                    format!(
                        "function `process_capture_in` expected 2 argument(s), got {}",
                        arguments.len()
                    ),
                    span,
                ));
            }

            let mut arguments = arguments.into_iter();
            let working_dir = arguments
                .next()
                .expect("process_capture_in working_dir argument should exist");
            let command_text = arguments
                .next()
                .expect("process_capture_in command argument should exist");
            return match (working_dir, command_text) {
                (Value::String(working_dir), Value::String(command_text)) => {
                    let resolved_dir = self.resolve_host_path(&working_dir);
                    let output = self
                        .host_shell_command_at(&resolved_dir, &command_text)
                        .output()
                        .map_err(|error| {
                            self.runtime_error_with_kind(
                                "R0119",
                                format!(
                                    "failed to capture `{command_text}` in `{}`: {error}",
                                    resolved_dir.display()
                                ),
                                span,
                                DiagnosticKind::ProcessCommandNotLaunchable,
                            )
                            .with_suggestion(
                                "pass an existing working directory and a valid shell command string like `process_capture_in(dir, command)`",
                            )
                        })?;

                    if !output.status.success() {
                        let exit_code = output.status.code().unwrap_or(-1);
                        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                        let mut diagnostic = self
                            .runtime_error_with_kind(
                                "R0120",
                                format!(
                                    "command `{command_text}` in `{}` exited with status {exit_code}",
                                    resolved_dir.display()
                                ),
                                span,
                                DiagnosticKind::ProcessCaptureNonZeroExit,
                            )
                            .with_suggestion(
                                "use `process_run_in(working_dir, command)` when you need the exit code without raising a runtime error",
                            );
                        if !stderr.is_empty() {
                            diagnostic = diagnostic.with_note(format!("stderr: {stderr}"));
                        }
                        return Err(diagnostic);
                    }

                    Ok(Value::String(
                        String::from_utf8_lossy(&output.stdout).into_owned(),
                    ))
                }
                (working_dir, command_text) => Err(self
                    .runtime_error(
                        "R0118",
                        format!(
                            "function `process_capture_in` requires `string` arguments, got `{}` and `{}`",
                            working_dir.display(),
                            command_text.display()
                        ),
                        span,
                    )
                    .with_suggestion(
                        "call `process_capture_in` like `process_capture_in(working_dir, command)`",
                    )),
            };
        }

        if name == "path_join" {
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

        if name == "path_resolve" {
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

        if name == "path_parent" {
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

        if name == "path_file_name" {
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

        if name == "path_stem" {
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
                    .with_suggestion(
                        "call `path_stem` with a string value like `path_stem(path)`",
                    )),
            };
        }

        if name == "path_extension" {
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

        if name == "path_is_absolute" {
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

        if name == "fs_is_file" {
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
                    .with_suggestion(
                        "call `fs_is_file` with a string value like `fs_is_file(path)`",
                    )),
            };
        }

        if name == "fs_is_dir" {
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
                    .with_suggestion(
                        "call `fs_is_dir` with a string value like `fs_is_dir(path)`",
                    )),
            };
        }

        if name == "fs_exists" {
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
                    .with_suggestion(
                        "call `fs_exists` with a string value like `fs_exists(path)`",
                    )),
            };
        }

        if name == "fs_file_size" {
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

        if name == "fs_copy_file" {
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

        if name == "fs_rename" {
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

        if name == "fs_create_dir_all" {
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

        if name == "fs_remove_file" {
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

        if name == "fs_remove_dir_all" {
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

        if name == "fs_read_dir" {
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

        if name == "fs_read_to_string" {
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

        if name == "fs_write_string" {
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
                    .with_suggestion(
                        "call `fs_write_string` like `fs_write_string(path, text)`",
                    )),
            };
        }

        self.call_declared_function(name, arguments, span)
    }
}
