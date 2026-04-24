use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::diagnostics::Diagnostic;
use crate::hir::{
    BinaryOp, Block, Expr, ExprKind, ItemKind, Param, Place, PlaceKind, Program, Stmt, StmtKind,
    UnaryOp,
};
use crate::source::{SourceFile, Span};

#[derive(Debug)]
pub struct RunOutput {
    pub exit_code: i32,
    pub stdout: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RunContext {
    pub argv: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub current_dir: PathBuf,
}

impl Default for RunContext {
    fn default() -> Self {
        Self {
            argv: Vec::new(),
            env: BTreeMap::new(),
            current_dir: PathBuf::from("."),
        }
    }
}

impl RunContext {
    pub fn from_host(argv: Vec<String>) -> std::io::Result<Self> {
        Ok(Self {
            argv,
            env: std::env::vars().collect(),
            current_dir: std::env::current_dir()?,
        })
    }
}

pub fn run_program(source: &SourceFile, program: &Program) -> Result<RunOutput, Diagnostic> {
    run_program_with_context(source, program, RunContext::default())
}

pub fn run_program_with_context(
    source: &SourceFile,
    program: &Program,
    context: RunContext,
) -> Result<RunOutput, Diagnostic> {
    Interpreter::new(source, program, context)?.run_main()
}

struct Interpreter<'a> {
    source: &'a SourceFile,
    functions: HashMap<String, FunctionDef<'a>>,
    stdout: Vec<String>,
    host: RunContext,
}

#[derive(Clone, Copy)]
struct FunctionDef<'a> {
    name: &'a str,
    params: &'a [Param],
    body: &'a Block,
    span: Span,
}

struct Frame {
    scopes: Vec<HashMap<String, Slot>>,
}

#[derive(Clone)]
struct Slot {
    mutable: bool,
    value: Value,
}

#[derive(Debug, Clone, PartialEq)]
enum Value {
    I32(i32),
    F32(f32),
    Bool(bool),
    String(String),
    Array(Vec<Value>),
    Slice(Vec<Value>),
    Enum {
        name: String,
        variant: String,
    },
    Struct {
        name: String,
        fields: BTreeMap<String, Value>,
    },
    Void,
}

enum ControlFlow {
    Continue,
    Break,
    Return(Value),
}

impl Value {
    fn display(&self) -> String {
        match self {
            Self::I32(value) => value.to_string(),
            Self::F32(value) => {
                let mut text = value.to_string();
                if !text.contains('.') && !text.contains('e') && !text.contains('E') {
                    text.push_str(".0");
                }
                text
            }
            Self::Bool(value) => value.to_string(),
            Self::String(value) => value.clone(),
            Self::Array(elements) => {
                let elements = elements
                    .iter()
                    .map(Value::display)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{elements}]")
            }
            Self::Slice(elements) => {
                let elements = elements
                    .iter()
                    .map(Value::display)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{elements}]")
            }
            Self::Enum { name, variant } => format!("{name}.{variant}"),
            Self::Struct { name, fields } => {
                let fields = fields
                    .iter()
                    .map(|(field, value)| format!("{field}: {}", value.display()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{name} {{ {fields} }}")
            }
            Self::Void => "<void>".to_string(),
        }
    }
}

impl<'a> Interpreter<'a> {
    fn new(
        source: &'a SourceFile,
        program: &'a Program,
        host: RunContext,
    ) -> Result<Self, Diagnostic> {
        let mut functions = HashMap::new();

        for item in &program.items {
            match &item.kind {
                ItemKind::Function {
                    name, params, body, ..
                } => {
                    functions.insert(
                        name.clone(),
                        FunctionDef {
                            name,
                            params,
                            body,
                            span: item.span,
                        },
                    );
                }
                ItemKind::Struct { .. } | ItemKind::Enum { .. } => {}
            }
        }

        if !functions.contains_key("main") {
            return Err(Diagnostic::new(
                "R0001",
                "program does not contain a runnable `main` function",
                source,
                Span::new(0, 0),
            ));
        }

        Ok(Self {
            source,
            functions,
            stdout: Vec::new(),
            host,
        })
    }

    fn run_main(mut self) -> Result<RunOutput, Diagnostic> {
        let result = self.call_function("main", Vec::new(), Span::new(0, 0))?;
        match result {
            Value::I32(exit_code) => Ok(RunOutput {
                exit_code,
                stdout: self.stdout,
            }),
            other => Err(self.runtime_error(
                "R0002",
                format!("`main` must return `i32`, got `{}`", other.display()),
                Span::new(0, 0),
            )),
        }
    }

    fn call_function(
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
                Value::Array(elements) | Value::Slice(elements) => {
                    Ok(Value::I32(elements.len() as i32))
                }
                other => Err(self
                    .runtime_error(
                        "R0040",
                        format!(
                            "function `len` requires a `string`, array, or slice argument, got `{}`",
                            other.display()
                        ),
                        span,
                    )
                    .with_suggestion("call `len` with a string, array, or slice value like `len(values)`")),
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
                        "call `to_string` on a string, number, bool, enum, struct, array, or slice value",
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
                    .runtime_error(
                        "R0048",
                        format!("argv index `{index}` must be non-negative"),
                        span,
                    )
                    .with_note("AX argv positions use zero-based `i32` indices")
                    .with_suggestion(
                        "check the length first with `argv_len()` before calling `argv_get(index)`",
                    )),
                Value::I32(index) => {
                    let index = index as usize;
                    self.host.argv.get(index).cloned().map(Value::String).ok_or_else(|| {
                        self.runtime_error(
                            "R0048",
                            format!("argv index `{index}` is out of bounds"),
                            span,
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
                Value::String(name) => Ok(Value::Bool(self.host.env.contains_key(&name))),
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
                Value::String(name) => self.host.env.get(&name).cloned().map(Value::String).ok_or_else(|| {
                    self.runtime_error(
                        "R0053",
                        format!("environment variable `{name}` is not available"),
                        span,
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
                        self.runtime_error(
                            "R0090",
                            format!("failed to run `{command_text}`: {error}"),
                            span,
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
                            .runtime_error(
                                "R0094",
                                format!("command `{command_text}` exited with status {exit_code}"),
                                span,
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
                        self.runtime_error(
                            "R0061",
                            format!("failed to read `{}`: {error}", resolved.display()),
                            span,
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

        let function = self.functions.get(name).copied().ok_or_else(|| {
            self.runtime_error("R0003", format!("call to unknown function `{name}`"), span)
        })?;

        if function.params.len() != arguments.len() {
            return Err(self.runtime_error(
                "R0004",
                format!(
                    "function `{name}` expected {} argument(s), got {}",
                    function.params.len(),
                    arguments.len()
                ),
                span,
            ));
        }

        let mut frame = Frame {
            scopes: vec![HashMap::new()],
        };
        for (param, argument) in function.params.iter().zip(arguments.into_iter()) {
            frame
                .scopes
                .last_mut()
                .expect("frame scope should exist")
                .insert(
                    param.name.clone(),
                    Slot {
                        mutable: false,
                        value: argument,
                    },
                );
        }

        match self.exec_block(function.body, &mut frame)? {
            ControlFlow::Return(value) => Ok(value),
            ControlFlow::Break => Err(self
                .runtime_error(
                    "R0005",
                    format!(
                        "function `{}` completed without returning a value",
                        function.name
                    ),
                    function.span,
                )
                .with_note(
                    "runtime reached the end of the function body after an unexpected `break`",
                )
                .with_suggestion(
                    "keep `break;` inside loops and ensure the function still returns a value",
                )),
            ControlFlow::Continue => Err(self
                .runtime_error(
                    "R0005",
                    format!(
                        "function `{}` completed without returning a value",
                        function.name
                    ),
                    function.span,
                )
                .with_note(
                    "runtime reached the end of the function body without executing `return`",
                )
                .with_suggestion("add an explicit `return` on every reachable path")),
        }
    }

    fn exec_block(&mut self, block: &Block, frame: &mut Frame) -> Result<ControlFlow, Diagnostic> {
        frame.scopes.push(HashMap::new());
        for statement in &block.statements {
            match self.exec_statement(statement, frame)? {
                ControlFlow::Continue => {}
                ControlFlow::Break => {
                    frame.scopes.pop();
                    return Ok(ControlFlow::Break);
                }
                ControlFlow::Return(value) => {
                    frame.scopes.pop();
                    return Ok(ControlFlow::Return(value));
                }
            }
        }
        frame.scopes.pop();
        Ok(ControlFlow::Continue)
    }

    fn exec_statement(
        &mut self,
        statement: &Stmt,
        frame: &mut Frame,
    ) -> Result<ControlFlow, Diagnostic> {
        match &statement.kind {
            StmtKind::Let {
                mutable,
                name,
                initializer,
                ..
            } => {
                let value = self.eval_expr(initializer, frame)?;
                frame.scopes.last_mut().expect("scope should exist").insert(
                    name.clone(),
                    Slot {
                        mutable: *mutable,
                        value,
                    },
                );
                Ok(ControlFlow::Continue)
            }
            StmtKind::Assign { target, value } => {
                let next_value = self.eval_expr(value, frame)?;
                self.assign_target(frame, target, next_value)?;
                Ok(ControlFlow::Continue)
            }
            StmtKind::Break => Ok(ControlFlow::Break),
            StmtKind::Expr { expr } => {
                self.eval_expr(expr, frame)?;
                Ok(ControlFlow::Continue)
            }
            StmtKind::Return { value } => {
                let value = self.eval_expr(value, frame)?;
                Ok(ControlFlow::Return(value))
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                if self.eval_condition(condition, frame)? {
                    self.exec_block(then_branch, frame)
                } else if let Some(block) = else_branch {
                    self.exec_block(block, frame)
                } else {
                    Ok(ControlFlow::Continue)
                }
            }
            StmtKind::While { condition, body } => {
                while self.eval_condition(condition, frame)? {
                    match self.exec_block(body, frame)? {
                        ControlFlow::Continue => {}
                        ControlFlow::Break => break,
                        ControlFlow::Return(value) => {
                            return Ok(ControlFlow::Return(value));
                        }
                    }
                }
                Ok(ControlFlow::Continue)
            }
            StmtKind::Block { block } => self.exec_block(block, frame),
        }
    }

    fn assign_target(
        &mut self,
        frame: &mut Frame,
        target: &Place,
        next_value: Value,
    ) -> Result<(), Diagnostic> {
        let root_name = place_root_name(target);
        let root_slot = lookup_slot(frame, root_name).ok_or_else(|| {
            self.runtime_error(
                "R0006",
                format!("assignment to unknown variable `{root_name}`"),
                target.span,
            )
        })?;

        if !root_slot.mutable {
            return match &target.kind {
                PlaceKind::Local { .. } => Err(self.runtime_error(
                    "R0007",
                    format!("cannot assign to immutable variable `{root_name}`"),
                    target.span,
                )),
                PlaceKind::Field { field, .. } => Err(self.runtime_error(
                    "R0025",
                    format!("cannot assign to field `{field}` on immutable variable `{root_name}`"),
                    target.span,
                )),
                PlaceKind::Index { .. } => Err(self.runtime_error(
                    "R0007",
                    format!("cannot assign through immutable array variable `{root_name}`"),
                    target.span,
                )),
            };
        }

        let target_value = self.resolve_place_value_mut(frame, target)?;
        *target_value = next_value;
        Ok(())
    }

    fn resolve_place_value_mut<'f>(
        &mut self,
        frame: &'f mut Frame,
        place: &Place,
    ) -> Result<&'f mut Value, Diagnostic> {
        match &place.kind {
            PlaceKind::Local { name } => {
                let slot = lookup_slot_mut(frame, name).ok_or_else(|| {
                    self.runtime_error(
                        "R0006",
                        format!("assignment to unknown variable `{name}`"),
                        place.span,
                    )
                })?;
                Ok(&mut slot.value)
            }
            PlaceKind::Field { base, field } => {
                let base_value = self.resolve_place_value_mut(frame, base)?;
                match base_value {
                    Value::Struct { fields, .. } => fields.get_mut(field).ok_or_else(|| {
                        self.runtime_error(
                            "R0026",
                            format!("struct value does not contain field `{field}`"),
                            place.span,
                        )
                    }),
                    other => Err(self.runtime_error(
                        "R0027",
                        format!(
                            "field assignment requires a struct value, got `{}`",
                            other.display()
                        ),
                        place.span,
                    )),
                }
            }
            PlaceKind::Index { base, index } => {
                let index_value = self.eval_expr(index, frame)?;
                let base_value = self.resolve_place_value_mut(frame, base)?;
                match base_value {
                    Value::Array(elements) => {
                        let resolved_index = self.resolve_array_index(
                            index_value,
                            index.span,
                            elements.len(),
                            place.span,
                        )?;
                        Ok(&mut elements[resolved_index])
                    }
                    Value::Slice(_) => Err(self
                        .runtime_error(
                            "R0036",
                            format!(
                                "cannot assign through slice variable `{}` because slices are read-only",
                                place_root_name(base)
                            ),
                            place.span,
                        )
                        .with_suggestion(
                            "assign through the original mutable array instead of a slice view",
                        )),
                    other => Err(self.runtime_error(
                        "R0028",
                        format!(
                            "array element assignment requires an array value, got `{}`",
                            other.display()
                        ),
                        place.span,
                    )),
                }
            }
        }
    }

    fn eval_condition(&mut self, expr: &Expr, frame: &mut Frame) -> Result<bool, Diagnostic> {
        match self.eval_expr(expr, frame)? {
            Value::Bool(value) => Ok(value),
            other => Err(self.runtime_error(
                "R0009",
                format!(
                    "condition must evaluate to `bool`, got `{}`",
                    other.display()
                ),
                expr.span,
            )),
        }
    }

    fn eval_expr(&mut self, expr: &Expr, frame: &mut Frame) -> Result<Value, Diagnostic> {
        match &expr.kind {
            ExprKind::Int { value } => Ok(Value::I32(*value)),
            ExprKind::Float { value } => Ok(Value::F32(*value)),
            ExprKind::Bool { value } => Ok(Value::Bool(*value)),
            ExprKind::String { value } => Ok(Value::String(value.clone())),
            ExprKind::Name { value } => lookup_slot(frame, value)
                .map(|slot| slot.value.clone())
                .ok_or_else(|| {
                    self.runtime_error(
                        "R0011",
                        format!("use of unknown variable `{value}`"),
                        expr.span,
                    )
                }),
            ExprKind::Unary { op, expr: inner } => {
                let inner = self.eval_expr(inner, frame)?;
                match (op, inner) {
                    (UnaryOp::Negate, Value::I32(value)) => {
                        value.checked_neg().map(Value::I32).ok_or_else(|| {
                            self.runtime_error("R0012", "integer negation overflowed", expr.span)
                        })
                    }
                    (UnaryOp::Negate, Value::F32(value)) => Ok(Value::F32(-value)),
                    (UnaryOp::Not, Value::Bool(value)) => Ok(Value::Bool(!value)),
                    (_, other) => Err(self.runtime_error(
                        "R0013",
                        format!("invalid unary operation on `{}`", other.display()),
                        expr.span,
                    )),
                }
            }
            ExprKind::Binary { op, left, right } => {
                let left = self.eval_expr(left, frame)?;
                let right = self.eval_expr(right, frame)?;
                self.eval_binary(*op, left, right, expr.span)
            }
            ExprKind::Call {
                function,
                arguments,
            } => {
                let argument_values = arguments
                    .iter()
                    .map(|argument| self.eval_expr(argument, frame))
                    .collect::<Result<Vec<_>, _>>()?;
                self.call_function(function, argument_values, expr.span)
            }
            ExprKind::StructLiteral { name, fields } => {
                let mut values = BTreeMap::new();
                for field in fields {
                    values.insert(field.name.clone(), self.eval_expr(&field.value, frame)?);
                }
                Ok(Value::Struct {
                    name: name.clone(),
                    fields: values,
                })
            }
            ExprKind::ArrayLiteral { elements } => Ok(Value::Array(
                elements
                    .iter()
                    .map(|element| self.eval_expr(element, frame))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            ExprKind::EnumVariant { enum_name, variant } => Ok(Value::Enum {
                name: enum_name.clone(),
                variant: variant.clone(),
            }),
            ExprKind::Field { base, field } => match self.eval_expr(base, frame)? {
                Value::Struct { fields, .. } => fields.get(field).cloned().ok_or_else(|| {
                    self.runtime_error(
                        "R0015",
                        format!("struct value does not contain field `{field}`"),
                        expr.span,
                    )
                }),
                other => Err(self.runtime_error(
                    "R0016",
                    format!(
                        "field access requires a struct value, got `{}`",
                        other.display()
                    ),
                    expr.span,
                )),
            },
            ExprKind::Index { base, index } => {
                let base_value = self.eval_expr(base, frame)?;
                let elements = self.indexable_elements(base_value, expr.span)?;

                let index_value = self.eval_expr(index, frame)?;
                let resolved =
                    self.resolve_array_index(index_value, index.span, elements.len(), expr.span)?;
                Ok(elements[resolved].clone())
            }
            ExprKind::Slice { base, start, end } => {
                let base_value = self.eval_expr(base, frame)?;
                let elements = self.indexable_elements(base_value, expr.span)?;
                let start_value = self.eval_expr(start, frame)?;
                let end_value = self.eval_expr(end, frame)?;
                let start_index =
                    self.resolve_slice_bound(start_value, start.span, elements.len(), "start")?;
                let end_index =
                    self.resolve_slice_bound(end_value, end.span, elements.len(), "end")?;

                if start_index > end_index {
                    return Err(self
                        .runtime_error(
                            "R0035",
                            format!(
                                "slice start `{start_index}` cannot be greater than slice end `{end_index}`"
                            ),
                            expr.span,
                        )
                        .with_note("AX slice ranges are half-open: `values[start:end]` includes `start` and excludes `end`")
                        .with_suggestion("ensure the start bound is less than or equal to the end bound"));
                }

                Ok(Value::Slice(elements[start_index..end_index].to_vec()))
            }
        }
    }

    fn indexable_elements(&self, value: Value, span: Span) -> Result<Vec<Value>, Diagnostic> {
        match value {
            Value::Array(elements) | Value::Slice(elements) => Ok(elements),
            other => Err(self.runtime_error(
                "R0028",
                format!(
                    "index access requires an array or slice value, got `{}`",
                    other.display()
                ),
                span,
            )),
        }
    }

    fn resolve_array_index(
        &self,
        index_value: Value,
        index_span: Span,
        array_len: usize,
        overall_span: Span,
    ) -> Result<usize, Diagnostic> {
        let Value::I32(index) = index_value else {
            return Err(self
                .runtime_error(
                    "R0029",
                    format!(
                        "array index must evaluate to `i32`, got `{}`",
                        index_value.display()
                    ),
                    index_span,
                )
                .with_note("AX array indices use `i32` values in the current prototype")
                .with_suggestion("compute or convert an `i32` index before indexing the array"));
        };

        if index < 0 {
            return Err(self
                .runtime_error(
                    "R0030",
                    format!("array index cannot be negative, got `{index}`"),
                    index_span,
                )
                .with_note("AX arrays use zero-based indexing")
                .with_suggestion("use an index in the range `0..len-1`"));
        }

        let index = usize::try_from(index).expect("non-negative i32 should fit in usize");
        if index >= array_len {
            return Err(self
                .runtime_error(
                    "R0031",
                    format!("array index `{index}` is out of bounds for length {array_len}"),
                    overall_span,
                )
                .with_note(format!(
                    "this access targets a fixed-size array with length {array_len}"
                ))
                .with_suggestion(
                    "change the index or array length so the access stays within bounds",
                ));
        }

        Ok(index)
    }

    fn resolve_slice_bound(
        &self,
        bound_value: Value,
        bound_span: Span,
        array_len: usize,
        label: &str,
    ) -> Result<usize, Diagnostic> {
        let Value::I32(bound) = bound_value else {
            return Err(self
                .runtime_error(
                    "R0032",
                    format!(
                        "slice {label} bound must evaluate to `i32`, got `{}`",
                        bound_value.display()
                    ),
                    bound_span,
                )
                .with_note("AX slice bounds currently use `i32` values")
                .with_suggestion("compute or convert an `i32` bound before slicing"));
        };

        if bound < 0 {
            return Err(self
                .runtime_error(
                    "R0033",
                    format!("slice {label} bound cannot be negative, got `{bound}`"),
                    bound_span,
                )
                .with_note("AX slice bounds use zero-based positions")
                .with_suggestion("use a bound in the range `0..len`"));
        }

        let bound = usize::try_from(bound).expect("non-negative i32 should fit in usize");
        if bound > array_len {
            return Err(self
                .runtime_error(
                    "R0034",
                    format!(
                        "slice {label} bound `{bound}` is out of bounds for length {array_len}"
                    ),
                    bound_span,
                )
                .with_note(format!(
                    "slice bounds may range from 0 up to the collection length {array_len}"
                ))
                .with_suggestion("change the slice bounds so they stay within `0..len`"));
        }

        Ok(bound)
    }

    fn eval_binary(
        &self,
        op: BinaryOp,
        left: Value,
        right: Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        match (op, left, right) {
            (BinaryOp::Add, Value::I32(left), Value::I32(right)) => left
                .checked_add(right)
                .map(Value::I32)
                .ok_or_else(|| self.runtime_error("R0018", "integer addition overflowed", span)),
            (BinaryOp::Add, Value::String(left), Value::String(right)) => {
                Ok(Value::String(format!("{left}{right}")))
            }
            (BinaryOp::Subtract, Value::I32(left), Value::I32(right)) => left
                .checked_sub(right)
                .map(Value::I32)
                .ok_or_else(|| self.runtime_error("R0019", "integer subtraction overflowed", span)),
            (BinaryOp::Multiply, Value::I32(left), Value::I32(right)) => {
                left.checked_mul(right).map(Value::I32).ok_or_else(|| {
                    self.runtime_error("R0020", "integer multiplication overflowed", span)
                })
            }
            (BinaryOp::Divide, Value::I32(_), Value::I32(0)) => Err(self
                .runtime_error("R0021", "division by zero", span)
                .with_note("AX checks integer division by zero at runtime")
                .with_suggestion(
                    "guard the divisor or rewrite the calculation so the right-hand side cannot be zero",
                )),
            (BinaryOp::Divide, Value::I32(left), Value::I32(right)) => left
                .checked_div(right)
                .map(Value::I32)
                .ok_or_else(|| self.runtime_error("R0022", "integer division overflowed", span)),
            (BinaryOp::Add, Value::F32(left), Value::F32(right)) => Ok(Value::F32(left + right)),
            (BinaryOp::Subtract, Value::F32(left), Value::F32(right)) => {
                Ok(Value::F32(left - right))
            }
            (BinaryOp::Multiply, Value::F32(left), Value::F32(right)) => {
                Ok(Value::F32(left * right))
            }
            (BinaryOp::Divide, Value::F32(_), Value::F32(0.0)) => Err(self
                .runtime_error("R0021", "division by zero", span)
                .with_note("AX checks floating-point division by zero at runtime")
                .with_suggestion(
                    "guard the divisor or rewrite the calculation so the right-hand side cannot be zero",
                )),
            (BinaryOp::Divide, Value::F32(left), Value::F32(right)) => Ok(Value::F32(left / right)),
            (BinaryOp::Equal, left, right) => Ok(Value::Bool(left == right)),
            (BinaryOp::NotEqual, left, right) => Ok(Value::Bool(left != right)),
            (BinaryOp::Less, Value::I32(left), Value::I32(right)) => Ok(Value::Bool(left < right)),
            (BinaryOp::LessEqual, Value::I32(left), Value::I32(right)) => {
                Ok(Value::Bool(left <= right))
            }
            (BinaryOp::Greater, Value::I32(left), Value::I32(right)) => {
                Ok(Value::Bool(left > right))
            }
            (BinaryOp::GreaterEqual, Value::I32(left), Value::I32(right)) => {
                Ok(Value::Bool(left >= right))
            }
            (BinaryOp::Less, Value::F32(left), Value::F32(right)) => Ok(Value::Bool(left < right)),
            (BinaryOp::LessEqual, Value::F32(left), Value::F32(right)) => {
                Ok(Value::Bool(left <= right))
            }
            (BinaryOp::Greater, Value::F32(left), Value::F32(right)) => {
                Ok(Value::Bool(left > right))
            }
            (BinaryOp::GreaterEqual, Value::F32(left), Value::F32(right)) => {
                Ok(Value::Bool(left >= right))
            }
            (_, left, right) => Err(self.runtime_error(
                "R0023",
                format!(
                    "invalid binary operation for runtime values `{}` and `{}`",
                    left.display(),
                    right.display()
                ),
                span,
            )),
        }
    }

    fn resolve_host_path(&self, path_text: &str) -> PathBuf {
        let path = Path::new(path_text);
        if path.is_relative() {
            self.host.current_dir.join(path)
        } else {
            path.to_path_buf()
        }
    }

    fn host_shell_command(&self, command_text: &str) -> Command {
        let mut command = if cfg!(windows) {
            let mut command = Command::new("cmd");
            command.arg("/C").arg(command_text);
            command
        } else {
            let mut command = Command::new("sh");
            command.arg("-lc").arg(command_text);
            command
        };
        command.current_dir(&self.host.current_dir);
        command.envs(&self.host.env);
        command
    }

    fn runtime_error(&self, code: &str, message: impl Into<String>, span: Span) -> Diagnostic {
        Diagnostic::new(code, message.into(), self.source, span)
    }
}

fn lookup_slot<'a>(frame: &'a Frame, name: &str) -> Option<&'a Slot> {
    frame.scopes.iter().rev().find_map(|scope| scope.get(name))
}

fn lookup_slot_mut<'a>(frame: &'a mut Frame, name: &str) -> Option<&'a mut Slot> {
    frame
        .scopes
        .iter_mut()
        .rev()
        .find_map(|scope| scope.get_mut(name))
}

fn place_root_name<'a>(place: &'a Place) -> &'a str {
    match &place.kind {
        PlaceKind::Local { name } => name.as_str(),
        PlaceKind::Field { base, .. } | PlaceKind::Index { base, .. } => place_root_name(base),
    }
}

#[cfg(test)]
mod tests {
    use super::run_program;
    use crate::frontend::analyze;
    use crate::source::SourceFile;

    fn analyzed_hir(source_text: &str) -> (SourceFile, crate::hir::Program) {
        let source = SourceFile::anonymous(source_text);
        let analysis = analyze(&source);
        assert!(
            analysis.diagnostics.is_empty(),
            "unexpected diagnostics: {:?}",
            analysis
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>()
        );

        (
            source,
            analysis
                .hir
                .expect("HIR should be available after successful analysis"),
        )
    }

    #[test]
    fn runs_loops_functions_and_println() {
        let (source, hir) = analyzed_hir(
            "\
fn step(value: i32) -> i32 {
    return value + 1;
}

fn main() -> i32 {
    let mut count: i32 = 0;
    while (count < 3) {
        count = step(count);
    }
    println(count);
    return count;
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 3);
        assert_eq!(output.stdout, vec!["3"]);
    }

    #[test]
    fn runs_conditionals() {
        let (source, hir) = analyzed_hir(
            "\
fn main() -> i32 {
    let flag: bool = true;
    if (flag) {
        println(\"ready\");
        return 0;
    } else {
        return 1;
    }
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, vec!["ready"]);
    }

    #[test]
    fn runs_recursive_functions() {
        let (source, hir) = analyzed_hir(
            "\
fn fact(n: i32) -> i32 {
    if (n == 0) {
        return 1;
    } else {
        return n * fact(n - 1);
    }
}

fn main() -> i32 {
    println(fact(5));
    return 0;
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, vec!["120"]);
    }

    #[test]
    fn runs_struct_literals_and_field_access() {
        let (source, hir) = analyzed_hir(
            "\
struct Point {
    x: i32,
    y: i32,
}

fn total(point: Point) -> i32 {
    return point.x + point.y;
}

fn main() -> i32 {
    let point: Point = Point { x: 2, y: 3 };
    println(point.x);
    println(total(point));
    return 0;
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, vec!["2", "5"]);
    }

    #[test]
    fn runs_enum_values_and_mutable_field_assignment() {
        let (source, hir) = analyzed_hir(
            "\
struct Point {
    x: i32,
    y: i32,
}

enum Flag {
    On,
    Off,
}

fn total(point: Point) -> i32 {
    return point.x + point.y;
}

fn main() -> i32 {
    let mut point: Point = Point { x: 2, y: 3 };
    point.x = point.x + 1;

    let flag: Flag = Flag.On;
    println(flag);
    println(point.x);
    println(total(point));
    return 0;
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, vec!["Flag.On", "3", "6"]);
    }

    #[test]
    fn runs_lowered_for_loops() {
        let (source, hir) = analyzed_hir(
            "\
fn main() -> i32 {
    let mut total: i32 = 0;
    for (let mut i: i32 = 0; i < 4; i = i + 1) {
        total = total + i;
    }
    println(total);
    return total;
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 6);
        assert_eq!(output.stdout, vec!["6"]);
    }

    #[test]
    fn runs_break_inside_loops() {
        let (source, hir) = analyzed_hir(
            "\
fn main() -> i32 {
    let mut count: i32 = 0;
    while (true) {
        count = count + 1;
        break;
    }
    println(count);
    return count;
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 1);
        assert_eq!(output.stdout, vec!["1"]);
    }

    #[test]
    fn runs_fixed_size_arrays_and_index_reads() {
        let (source, hir) = analyzed_hir(
            "\
fn main() -> i32 {
    let values: [i32; 3] = [1, 2, 3];
    println(values);
    println(values[1]);
    return values[0] + values[2];
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 4);
        assert_eq!(output.stdout, vec!["[1, 2, 3]", "2"]);
    }

    #[test]
    fn runs_mutable_array_element_assignment() {
        let (source, hir) = analyzed_hir(
            "\
fn main() -> i32 {
    let mut values: [i32; 3] = [1, 2, 3];
    values[1] = values[0] + values[2];
    println(values);
    return values[1];
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 4);
        assert_eq!(output.stdout, vec!["[1, 4, 3]"]);
    }

    #[test]
    fn runs_nested_assignment_through_array_elements_and_fields() {
        let (source, hir) = analyzed_hir(
            "\
struct Token {
    value: i32,
}

fn main() -> i32 {
    let mut tokens: [Token; 3] = [
        Token { value: 1 },
        Token { value: 2 },
        Token { value: 3 },
    ];

    let mut index: i32 = 0;
    while (index < len(tokens)) {
        tokens[index].value = tokens[index].value + 10;
        index = index + 1;
    }

    println(tokens);
    return tokens[0].value + tokens[2].value;
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 24);
        assert_eq!(
            output.stdout,
            vec!["[Token { value: 11 }, Token { value: 12 }, Token { value: 13 }]"]
        );
    }

    #[test]
    fn runs_nested_struct_field_assignment_paths() {
        let (source, hir) = analyzed_hir(
            "\
struct Inner {
    value: i32,
}

struct Outer {
    inner: Inner,
}

fn main() -> i32 {
    let mut outer: Outer = Outer { inner: Inner { value: 5 } };
    outer.inner.value = outer.inner.value + 7;
    println(outer);
    return outer.inner.value;
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 12);
        assert_eq!(output.stdout, vec!["Outer { inner: Inner { value: 12 } }"]);
    }

    #[test]
    fn runs_slice_reads_and_slice_parameters() {
        let (source, hir) = analyzed_hir(
            "\
fn second(values: [i32]) -> i32 {
    println(values);
    return values[1];
}

fn main() -> i32 {
    let values: [i32; 4] = [1, 2, 3, 4];
    let window: [i32] = values[1:3];
    println(window);
    return second(window);
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 3);
        assert_eq!(output.stdout, vec!["[2, 3]", "[2, 3]"]);
    }

    #[test]
    fn runs_integer_division() {
        let (source, hir) = analyzed_hir(
            "\
fn main() -> i32 {
    let value: i32 = 8 / 2;
    println(value);
    return value;
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 4);
        assert_eq!(output.stdout, vec!["4"]);
    }

    #[test]
    fn runs_string_concat_and_string_len() {
        let (source, hir) = analyzed_hir(
            "\
fn main() -> i32 {
    let prefix: string = \"AX\";
    let message: string = prefix + \" tools\";
    println(message);
    println(string_len(message));
    return string_len(\"hey\");
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 3);
        assert_eq!(output.stdout, vec!["AX tools", "8"]);
    }

    #[test]
    fn runs_len_for_strings_arrays_and_slices() {
        let (source, hir) = analyzed_hir(
            "\
fn sum(values: [i32]) -> i32 {
    let mut total: i32 = 0;
    for (let mut i: i32 = 0; i < len(values); i = i + 1) {
        total = total + values[i];
    }
    return total;
}

fn main() -> i32 {
    let values: [i32; 5] = [1, 2, 3, 4, 5];
    let middle: [i32] = values[1:4];
    println(len(\"AX\"));
    println(len(values));
    println(len(middle));
    println(sum(middle));
    return sum(values);
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 15);
        assert_eq!(output.stdout, vec!["2", "5", "3", "9"]);
    }

    #[test]
    fn runs_to_string_for_tool_style_reports() {
        let (source, hir) = analyzed_hir(
            "\
struct Summary {
    count: i32,
    ready: bool,
}

fn build_report(summary: Summary, values: [i32]) -> string {
    let mut report: string = \"count=\" + to_string(summary.count);
    report = report + \", ready=\" + to_string(summary.ready);
    report = report + \", values=\" + to_string(values);
    return report;
}

fn main() -> i32 {
    let summary: Summary = Summary { count: 3, ready: true };
    let values: [i32; 3] = [2, 4, 6];
    let report: string = build_report(summary, values[0:2]);
    println(report);
    return string_len(report);
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 34);
        assert_eq!(output.stdout, vec!["count=3, ready=true, values=[2, 4]"]);
    }
}
