use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::source::Span;

use super::super::Interpreter;
use super::super::value::Value;

impl<'a> Interpreter<'a> {
    pub(super) fn call_process_cwd_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
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

    pub(super) fn call_process_run_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
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

    pub(super) fn call_process_capture_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
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

    pub(super) fn call_process_run_in_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
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

    pub(super) fn call_process_capture_in_builtin(
        &mut self,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
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
}
