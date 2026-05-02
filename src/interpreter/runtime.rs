use super::*;

impl<'a> Interpreter<'a> {
    pub(in crate::interpreter) fn resolve_host_path(&self, path_text: &str) -> PathBuf {
        let path = Path::new(path_text);
        if path.is_relative() {
            self.host.current_dir.join(path)
        } else {
            path.to_path_buf()
        }
    }

    pub(in crate::interpreter) fn host_shell_command(&self, command_text: &str) -> Command {
        self.host_shell_command_at(&self.host.current_dir, command_text)
    }

    pub(in crate::interpreter) fn host_shell_command_at(
        &self,
        working_dir: &Path,
        command_text: &str,
    ) -> Command {
        let mut command = if cfg!(windows) {
            let mut command = Command::new("cmd");
            command.arg("/C").arg(command_text);
            command
        } else {
            let mut command = Command::new("sh");
            command.arg("-lc").arg(command_text);
            command
        };
        command.current_dir(working_dir);
        command.envs(&self.host.env);
        command
    }

    pub(in crate::interpreter) fn runtime_error(
        &self,
        code: &str,
        message: impl Into<String>,
        span: Span,
    ) -> Diagnostic {
        Diagnostic::new(code, message.into(), self.source, span)
    }

    pub(in crate::interpreter) fn runtime_error_with_kind(
        &self,
        code: &str,
        message: impl Into<String>,
        span: Span,
        kind: DiagnosticKind,
    ) -> Diagnostic {
        self.runtime_error(code, message, span).with_kind(kind)
    }
}
