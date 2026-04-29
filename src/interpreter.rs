use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::hir::{
    BinaryOp, Block, EnumVariantPayloadPattern as MatchPatternPayload, Expr, ExprKind, ItemKind,
    MatchExprArm, MatchPattern, MatchPatternKind, Param, Place, PlaceKind, Program, Stmt, StmtKind,
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

    fn env_contains(&self, name: &str) -> bool {
        self.env_value(name).is_some()
    }

    fn env_value(&self, name: &str) -> Option<&str> {
        if let Some(value) = self.env.get(name) {
            return Some(value.as_str());
        }

        #[cfg(windows)]
        {
            self.env
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(name))
                .map(|(_, value)| value.as_str())
        }

        #[cfg(not(windows))]
        {
            None
        }
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
    constants: HashMap<String, Value>,
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
    StringList(Vec<String>),
    Array(Vec<Value>),
    Slice(Vec<Value>),
    Enum {
        name: String,
        variant: String,
        payload: Option<Box<Value>>,
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
    LoopContinue,
    Return(Value),
}

enum EvalFlow {
    Value(Value),
    Return(Value),
}

enum ConditionFlow {
    Value(bool),
    Return(Value),
}

fn collect_left_associative_binary_operands<'a>(
    expr: &'a Expr,
    op: BinaryOp,
    operands: &mut Vec<&'a Expr>,
) {
    match &expr.kind {
        ExprKind::Binary {
            op: current_op,
            left,
            right,
        } if *current_op == op => {
            collect_left_associative_binary_operands(left, op, operands);
            operands.push(right);
        }
        _ => operands.push(expr),
    }
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
            Self::StringList(values) => {
                let values = values.join(", ");
                format!("[{values}]")
            }
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
            Self::Enum {
                name,
                variant,
                payload,
            } => match payload {
                Some(payload) => format!("{name}.{variant}({})", payload.display()),
                None => format!("{name}.{variant}"),
            },
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
        let mut const_items = Vec::new();

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
                ItemKind::Const { name, value, .. } => {
                    const_items.push((name.clone(), value, item.span));
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

        let mut interpreter = Self {
            source,
            functions,
            constants: HashMap::new(),
            stdout: Vec::new(),
            host,
        };

        for (name, value, span) in const_items {
            let mut frame = Frame {
                scopes: vec![HashMap::new()],
            };
            let value = interpreter
                .eval_expr(value, &mut frame)
                .map_err(|diagnostic| {
                    Diagnostic::new(
                        diagnostic.code,
                        format!("failed to evaluate constant `{name}`"),
                        source,
                        span,
                    )
                    .with_note(diagnostic.message)
                    .with_suggestion("keep top-level constants as deterministic values")
                })?;
            let value = match value {
                EvalFlow::Value(value) => value,
                EvalFlow::Return(_) => {
                    return Err(Diagnostic::new(
                        "R0134",
                        format!("constant `{name}` cannot use error propagation"),
                        source,
                        span,
                    )
                    .with_suggestion("remove `?` from the constant initializer"));
                }
            };
            interpreter.constants.insert(name, value);
        }

        Ok(interpreter)
    }

    fn run_main(mut self) -> Result<RunOutput, Diagnostic> {
        let result = self.call_declared_function("main", Vec::new(), Span::new(0, 0))?;
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

    fn call_declared_function(
        &mut self,
        name: &str,
        arguments: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
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
            ControlFlow::LoopContinue => Err(self
                .runtime_error(
                    "R0005",
                    format!(
                        "function `{}` completed without returning a value",
                        function.name
                    ),
                    function.span,
                )
                .with_note(
                    "runtime reached the end of the function body after an unexpected `continue`",
                )
                .with_suggestion(
                    "keep `continue;` inside loops and ensure the function still returns a value",
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
                ControlFlow::LoopContinue => {
                    frame.scopes.pop();
                    return Ok(ControlFlow::LoopContinue);
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
                let value = match self.eval_expr(initializer, frame)? {
                    EvalFlow::Value(value) => value,
                    EvalFlow::Return(value) => return Ok(ControlFlow::Return(value)),
                };
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
                let next_value = match self.eval_expr(value, frame)? {
                    EvalFlow::Value(value) => value,
                    EvalFlow::Return(value) => return Ok(ControlFlow::Return(value)),
                };
                self.assign_target(frame, target, next_value)?;
                Ok(ControlFlow::Continue)
            }
            StmtKind::Break => Ok(ControlFlow::Break),
            StmtKind::Continue => Ok(ControlFlow::LoopContinue),
            StmtKind::Expr { expr } => match self.eval_expr(expr, frame)? {
                EvalFlow::Value(_) => Ok(ControlFlow::Continue),
                EvalFlow::Return(value) => Ok(ControlFlow::Return(value)),
            },
            StmtKind::Return { value } => {
                let value = match self.eval_expr(value, frame)? {
                    EvalFlow::Value(value) => value,
                    EvalFlow::Return(value) => value,
                };
                Ok(ControlFlow::Return(value))
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => match self.eval_condition(condition, frame)? {
                ConditionFlow::Return(value) => Ok(ControlFlow::Return(value)),
                ConditionFlow::Value(true) => self.exec_block(then_branch, frame),
                ConditionFlow::Value(false) => {
                    if let Some(block) = else_branch {
                        self.exec_block(block, frame)
                    } else {
                        Ok(ControlFlow::Continue)
                    }
                }
            },
            StmtKind::While { condition, body } => {
                loop {
                    match self.eval_condition(condition, frame)? {
                        ConditionFlow::Return(value) => return Ok(ControlFlow::Return(value)),
                        ConditionFlow::Value(true) => {}
                        ConditionFlow::Value(false) => break,
                    }
                    match self.exec_block(body, frame)? {
                        ControlFlow::Continue => {}
                        ControlFlow::Break => break,
                        ControlFlow::LoopContinue => continue,
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
                let index_value = match self.eval_expr(index, frame)? {
                    EvalFlow::Value(value) => value,
                    EvalFlow::Return(_) => {
                        return Err(self.runtime_error(
                            "R0135",
                            "`?` cannot propagate while resolving an assignment target",
                            index.span,
                        ));
                    }
                };
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

    fn eval_condition(
        &mut self,
        expr: &Expr,
        frame: &mut Frame,
    ) -> Result<ConditionFlow, Diagnostic> {
        match self.eval_expr(expr, frame)? {
            EvalFlow::Return(value) => Ok(ConditionFlow::Return(value)),
            EvalFlow::Value(Value::Bool(value)) => Ok(ConditionFlow::Value(value)),
            EvalFlow::Value(other) => Err(self.runtime_error(
                "R0009",
                format!(
                    "condition must evaluate to `bool`, got `{}`",
                    other.display()
                ),
                expr.span,
            )),
        }
    }

    fn eval_expr(&mut self, expr: &Expr, frame: &mut Frame) -> Result<EvalFlow, Diagnostic> {
        macro_rules! eval_value {
            ($inner:expr) => {
                match self.eval_expr($inner, frame)? {
                    EvalFlow::Value(value) => value,
                    early @ EvalFlow::Return(_) => return Ok(early),
                }
            };
        }

        match &expr.kind {
            ExprKind::Int { value } => Ok(EvalFlow::Value(Value::I32(*value))),
            ExprKind::Float { value } => Ok(EvalFlow::Value(Value::F32(*value))),
            ExprKind::Bool { value } => Ok(EvalFlow::Value(Value::Bool(*value))),
            ExprKind::String { value } => Ok(EvalFlow::Value(Value::String(value.clone()))),
            ExprKind::Name { value } => Ok(EvalFlow::Value(
                lookup_slot(frame, value)
                    .map(|slot| slot.value.clone())
                    .or_else(|| self.constants.get(value).cloned())
                    .ok_or_else(|| {
                        self.runtime_error(
                            "R0011",
                            format!("use of unknown variable `{value}`"),
                            expr.span,
                        )
                    })?,
            )),
            ExprKind::Unary { op, expr: inner } => {
                let inner = eval_value!(inner);
                let value = match (op, inner) {
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
                }?;
                Ok(EvalFlow::Value(value))
            }
            ExprKind::Try { expr: inner } => {
                let value = eval_value!(inner);
                match value {
                    Value::Enum {
                        variant,
                        payload: Some(payload),
                        ..
                    } if variant == "Ok" => Ok(EvalFlow::Value(*payload)),
                    Value::Enum {
                        name,
                        variant,
                        payload,
                    } if variant == "Err" => Ok(EvalFlow::Return(Value::Enum {
                        name,
                        variant,
                        payload,
                    })),
                    Value::Enum { variant, .. } => Err(self.runtime_error(
                        "R0136",
                        format!(
                            "`?` expected `Result.Ok` or `Result.Err`, got variant `{variant}`"
                        ),
                        expr.span,
                    )),
                    other => Err(self.runtime_error(
                        "R0136",
                        format!("`?` expected a `Result` value, got `{}`", other.display()),
                        expr.span,
                    )),
                }
            }
            ExprKind::Binary { op, left, right } => {
                if matches!(*op, BinaryOp::LogicalAnd | BinaryOp::LogicalOr) {
                    let left_value = eval_value!(left);
                    let value = match (*op, left_value) {
                        (BinaryOp::LogicalAnd, Value::Bool(false)) => Ok(Value::Bool(false)),
                        (BinaryOp::LogicalAnd, Value::Bool(true)) => {
                            let right_value = eval_value!(right);
                            self.eval_binary(*op, Value::Bool(true), right_value, expr.span)
                        }
                        (BinaryOp::LogicalOr, Value::Bool(true)) => Ok(Value::Bool(true)),
                        (BinaryOp::LogicalOr, Value::Bool(false)) => {
                            let right_value = eval_value!(right);
                            self.eval_binary(*op, Value::Bool(false), right_value, expr.span)
                        }
                        (_, other) => Err(self.runtime_error(
                            "R0023",
                            format!(
                                "invalid binary operation for runtime values `{}` and `<unevaluated>`",
                                other.display()
                            ),
                            expr.span,
                        )),
                    }?;
                    return Ok(EvalFlow::Value(value));
                }

                let mut operands = Vec::new();
                collect_left_associative_binary_operands(expr, *op, &mut operands);
                if operands.len() > 2 {
                    let mut operands = operands.into_iter();
                    let first = operands
                        .next()
                        .expect("binary chain should contain at least one operand");
                    let mut value = eval_value!(first);
                    for operand in operands {
                        let right = eval_value!(operand);
                        value = self.eval_binary(*op, value, right, expr.span)?;
                    }
                    return Ok(EvalFlow::Value(value));
                }

                let left = eval_value!(left);
                let right = eval_value!(right);
                Ok(EvalFlow::Value(
                    self.eval_binary(*op, left, right, expr.span)?,
                ))
            }
            ExprKind::Call {
                function,
                arguments,
            } => {
                let mut argument_values = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    argument_values.push(eval_value!(argument));
                }
                let value = if self.functions.contains_key(function) {
                    self.call_declared_function(function, argument_values, expr.span)
                } else {
                    self.call_function(function, argument_values, expr.span)
                }?;
                Ok(EvalFlow::Value(value))
            }
            ExprKind::MethodCall {
                receiver,
                method,
                arguments,
            } => {
                let receiver_value = eval_value!(receiver);
                let method_function = match &receiver_value {
                    Value::Struct { name, .. } | Value::Enum { name, .. } => {
                        format!("{name}.{method}")
                    }
                    other => {
                        return Err(self.runtime_error(
                            "R0133",
                            format!(
                                "method call `{method}` requires a struct or enum receiver, got `{}`",
                                other.display()
                            ),
                            expr.span,
                        ));
                    }
                };
                let mut argument_values = Vec::with_capacity(arguments.len() + 1);
                argument_values.push(receiver_value);
                for argument in arguments {
                    argument_values.push(eval_value!(argument));
                }
                Ok(EvalFlow::Value(self.call_declared_function(
                    &method_function,
                    argument_values,
                    expr.span,
                )?))
            }
            ExprKind::StructLiteral { name, fields } => {
                let mut values = BTreeMap::new();
                for field in fields {
                    values.insert(field.name.clone(), eval_value!(&field.value));
                }
                Ok(EvalFlow::Value(Value::Struct {
                    name: name.clone(),
                    fields: values,
                }))
            }
            ExprKind::ArrayLiteral { elements } => {
                let mut values = Vec::with_capacity(elements.len());
                for element in elements {
                    values.push(eval_value!(element));
                }
                Ok(EvalFlow::Value(Value::Array(values)))
            }
            ExprKind::Match { scrutinee, arms } => {
                let scrutinee_value = eval_value!(scrutinee);
                self.eval_match_expression(scrutinee_value, arms, expr.span, frame)
            }
            ExprKind::EnumVariant {
                enum_name,
                variant,
                payload,
            } => Ok(EvalFlow::Value(Value::Enum {
                name: enum_name.clone(),
                variant: variant.clone(),
                payload: match payload {
                    Some(payload) => Some(Box::new(eval_value!(payload))),
                    None => None,
                },
            })),
            ExprKind::MatchTest { scrutinee, pattern } => {
                let scrutinee_value = eval_value!(scrutinee);
                Ok(EvalFlow::Value(Value::Bool(
                    self.match_pattern_matches_value(pattern, &scrutinee_value, expr.span)?,
                )))
            }
            ExprKind::EnumPayload { value } => match eval_value!(value) {
                Value::Enum {
                    payload: Some(payload),
                    ..
                } => Ok(EvalFlow::Value(*payload)),
                other => Err(self.runtime_error(
                    "R0042",
                    format!(
                        "payload extraction requires a payload enum value, got `{}`",
                        other.display()
                    ),
                    expr.span,
                )),
            },
            ExprKind::Field { base, field } => match eval_value!(base) {
                Value::Struct { fields, .. } => Ok(EvalFlow::Value(
                    fields.get(field).cloned().ok_or_else(|| {
                        self.runtime_error(
                            "R0015",
                            format!("struct value does not contain field `{field}`"),
                            expr.span,
                        )
                    })?,
                )),
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
                let base_value = eval_value!(base);
                let elements = self.indexable_elements(base_value, expr.span)?;

                let index_value = eval_value!(index);
                let resolved =
                    self.resolve_array_index(index_value, index.span, elements.len(), expr.span)?;
                Ok(EvalFlow::Value(elements[resolved].clone()))
            }
            ExprKind::Slice { base, start, end } => {
                let base_value = eval_value!(base);
                let elements = self.indexable_elements(base_value, expr.span)?;
                let start_value = eval_value!(start);
                let end_value = eval_value!(end);
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

                Ok(EvalFlow::Value(Value::Slice(
                    elements[start_index..end_index].to_vec(),
                )))
            }
        }
    }

    fn eval_match_expression(
        &mut self,
        scrutinee: Value,
        arms: &[MatchExprArm],
        span: Span,
        frame: &mut Frame,
    ) -> Result<EvalFlow, Diagnostic> {
        for arm in arms {
            if self.match_pattern_matches_value(&arm.pattern, &scrutinee, span)? {
                if let Some(value) = self.eval_match_expression_arm_value(
                    &arm.pattern,
                    arm.guard.as_ref(),
                    &scrutinee,
                    &arm.value,
                    frame,
                )? {
                    return Ok(value);
                }
            }
        }

        Err(self.runtime_error(
            "R0036",
            "non-exhaustive match expression reached runtime without a matching arm",
            span,
        ))
    }

    fn match_pattern_matches_value(
        &self,
        pattern: &MatchPattern,
        scrutinee: &Value,
        span: Span,
    ) -> Result<bool, Diagnostic> {
        match &pattern.kind {
            MatchPatternKind::Wildcard => Ok(true),
            MatchPatternKind::Binding { .. } => Ok(true),
            MatchPatternKind::Bool { value } => match scrutinee {
                Value::Bool(actual) => Ok(actual == value),
                other => Err(self.runtime_error(
                    "R0037",
                    format!(
                        "match pattern `bool` cannot be applied to runtime value `{}`",
                        other.display()
                    ),
                    span,
                )),
            },
            MatchPatternKind::Int { value } => match scrutinee {
                Value::I32(actual) => Ok(actual == value),
                other => Err(self.runtime_error(
                    "R0037",
                    format!(
                        "match pattern `i32` cannot be applied to runtime value `{}`",
                        other.display()
                    ),
                    span,
                )),
            },
            MatchPatternKind::IntRange { start, end } => match scrutinee {
                Value::I32(actual) => Ok(actual >= start && actual <= end),
                other => Err(self.runtime_error(
                    "R0037",
                    format!(
                        "match pattern `i32` range cannot be applied to runtime value `{}`",
                        other.display()
                    ),
                    span,
                )),
            },
            MatchPatternKind::String { value } => match scrutinee {
                Value::String(actual) => Ok(actual == value),
                other => Err(self.runtime_error(
                    "R0037",
                    format!(
                        "match pattern `string` cannot be applied to runtime value `{}`",
                        other.display()
                    ),
                    span,
                )),
            },
            MatchPatternKind::EnumVariant {
                enum_name,
                variant,
                payload,
                ..
            } => match scrutinee {
                Value::Enum {
                    name,
                    variant: actual_variant,
                    payload: actual_payload,
                } => {
                    if name != enum_name || actual_variant != variant {
                        return Ok(false);
                    }

                    match (payload, actual_payload.as_ref()) {
                        (None, _) => Ok(true),
                        (Some(MatchPatternPayload::Wildcard), Some(_)) => Ok(true),
                        (Some(MatchPatternPayload::Binding { .. }), Some(_)) => Ok(true),
                        (Some(_), None) => Err(self.runtime_error(
                            "R0037",
                            format!(
                                "match enum pattern `{}` expects a payload value",
                                Self::match_pattern_label(pattern)
                            ),
                            span,
                        )),
                    }
                }
                other => Err(self.runtime_error(
                    "R0037",
                    format!(
                        "match enum pattern cannot be applied to runtime value `{}`",
                        other.display()
                    ),
                    span,
                )),
            },
            MatchPatternKind::Or { alternatives } => {
                for alternative in alternatives {
                    if self.match_pattern_matches_value(alternative, scrutinee, span)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            MatchPatternKind::Error => {
                Err(self.runtime_error("R0038", "invalid match pattern reached the runtime", span))
            }
        }
    }

    fn eval_match_expression_arm_value(
        &mut self,
        pattern: &MatchPattern,
        guard: Option<&Expr>,
        scrutinee: &Value,
        value: &Expr,
        frame: &mut Frame,
    ) -> Result<Option<EvalFlow>, Diagnostic> {
        frame.scopes.push(HashMap::new());
        if let Err(error) = self.bind_match_pattern_locals(pattern, scrutinee, frame) {
            frame.scopes.pop();
            return Err(error);
        }
        if let Some(guard) = guard {
            let guard_matches = self.eval_condition(guard, frame);
            match guard_matches {
                Ok(ConditionFlow::Value(true)) => {}
                Ok(ConditionFlow::Value(false)) => {
                    frame.scopes.pop();
                    return Ok(None);
                }
                Ok(ConditionFlow::Return(value)) => {
                    frame.scopes.pop();
                    return Ok(Some(EvalFlow::Return(value)));
                }
                Err(error) => {
                    frame.scopes.pop();
                    return Err(error);
                }
            }
        }
        let result = self.eval_expr(value, frame).map(Some);
        frame.scopes.pop();
        result
    }

    fn bind_match_pattern_locals(
        &self,
        pattern: &MatchPattern,
        scrutinee: &Value,
        frame: &mut Frame,
    ) -> Result<(), Diagnostic> {
        match &pattern.kind {
            MatchPatternKind::Binding { name } => {
                frame.scopes.last_mut().expect("scope should exist").insert(
                    name.clone(),
                    Slot {
                        mutable: false,
                        value: scrutinee.clone(),
                    },
                );
            }
            MatchPatternKind::EnumVariant {
                payload: Some(MatchPatternPayload::Binding { name }),
                ..
            } => {
                let Value::Enum {
                    payload: Some(payload),
                    ..
                } = scrutinee
                else {
                    return Err(self.runtime_error(
                        "R0042",
                        format!("payload binding `{}` requires a payload enum value", name),
                        pattern.span,
                    ));
                };
                frame.scopes.last_mut().expect("scope should exist").insert(
                    name.clone(),
                    Slot {
                        mutable: false,
                        value: (**payload).clone(),
                    },
                );
            }
            _ => {}
        }
        Ok(())
    }

    fn match_pattern_label(pattern: &MatchPattern) -> String {
        match &pattern.kind {
            MatchPatternKind::Wildcard => "_".to_string(),
            MatchPatternKind::Binding { name } => name.clone(),
            MatchPatternKind::Bool { value } => value.to_string(),
            MatchPatternKind::Int { value } => value.to_string(),
            MatchPatternKind::IntRange { start, end } => format!("{start}..={end}"),
            MatchPatternKind::String { value } => format!("{value:?}"),
            MatchPatternKind::EnumVariant {
                enum_name,
                variant,
                payload: Some(MatchPatternPayload::Wildcard),
                ..
            } => format!("{enum_name}.{variant}(_)"),
            MatchPatternKind::EnumVariant {
                enum_name,
                variant,
                payload: Some(MatchPatternPayload::Binding { name }),
                ..
            } => format!("{enum_name}.{variant}({name})"),
            MatchPatternKind::EnumVariant {
                enum_name, variant, ..
            } => format!("{enum_name}.{variant}"),
            MatchPatternKind::Or { alternatives } => alternatives
                .iter()
                .map(Self::match_pattern_label)
                .collect::<Vec<_>>()
                .join(" | "),
            MatchPatternKind::Error => "<invalid-pattern>".to_string(),
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
            (BinaryOp::LogicalAnd, Value::Bool(left), Value::Bool(right)) => {
                Ok(Value::Bool(left && right))
            }
            (BinaryOp::LogicalOr, Value::Bool(left), Value::Bool(right)) => {
                Ok(Value::Bool(left || right))
            }
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
            (BinaryOp::Remainder, Value::I32(_), Value::I32(0)) => Err(self
                .runtime_error("R0021", "modulo by zero", span)
                .with_note("AX checks integer remainder by zero at runtime")
                .with_suggestion(
                    "guard the divisor or rewrite the calculation so the right-hand side cannot be zero",
                )),
            (BinaryOp::Remainder, Value::I32(left), Value::I32(right)) => left
                .checked_rem(right)
                .map(Value::I32)
                .ok_or_else(|| self.runtime_error("R0024", "integer remainder overflowed", span)),
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
        self.host_shell_command_at(&self.host.current_dir, command_text)
    }

    fn host_shell_command_at(&self, working_dir: &Path, command_text: &str) -> Command {
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

    fn runtime_error(&self, code: &str, message: impl Into<String>, span: Span) -> Diagnostic {
        Diagnostic::new(code, message.into(), self.source, span)
    }

    fn runtime_error_with_kind(
        &self,
        code: &str,
        message: impl Into<String>,
        span: Span,
        kind: DiagnosticKind,
    ) -> Diagnostic {
        self.runtime_error(code, message, span).with_kind(kind)
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
    use std::collections::BTreeMap;

    use super::{RunContext, run_program, run_program_with_context};
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
    fn runs_generic_impl_methods() {
        let (source, hir) = analyzed_hir(
            "\
struct Box<T> {
    value: T,
}

impl<T> Box<T> {
    fn get(self: Box<T>) -> T {
        return self.value;
    }
}

fn main() -> i32 {
    let number: Box<i32> = Box { value: 9 };
    println(number.get());
    return number.get();
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 9);
        assert_eq!(output.stdout, vec!["9"]);
    }

    #[test]
    fn runs_generic_trait_impl_methods() {
        let (source, hir) = analyzed_hir(
            "\
trait Label {
    fn label(self: Self) -> string;
}

struct Box<T> {
    value: T,
}

impl<T> Label for Box<T> {
    fn label(self: Box<T>) -> string {
        return to_string(self.value);
    }
}

fn render<T: Label>(value: T) -> string {
    return value.label();
}

fn main() -> i32 {
    let number: Box<i32> = Box { value: 42 };
    println(render(number));
    return 0;
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, vec!["42"]);
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
    fn runs_match_statements() {
        let (source, hir) = analyzed_hir(
            "\
enum Flag {
    On,
    Off,
}

fn choose(flag: Flag) -> i32 {
    match (flag) {
        Flag.On => {
            return 1;
        }
        Flag.Off => {
            return 2;
        }
    }
}

fn classify(value: i32) -> i32 {
    match (value) {
        0 => {
            return 7;
        }
        _ => {
            return value;
        }
    }
}

fn main() -> i32 {
    let truthy: bool = true;
    let mut total: i32 = 0;
    match (truthy) {
        true => {
            total = total + 10;
        }
        false => {
            total = total + 1;
        }
    }
    total = total + choose(Flag.On);
    total = total + choose(Flag.Off);
    total = total + classify(0);
    total = total + classify(5);
    println(total);
    return total;
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 25);
        assert_eq!(output.stdout, vec!["25"]);
    }

    #[test]
    fn runs_match_expressions() {
        let (source, hir) = analyzed_hir(
            "\
fn classify(flag: bool) -> i32 {
    return match (flag) { true => 3, false => 1 };
}

fn main() -> i32 {
    let left: i32 = match (false) { true => 8, false => 2 };
    let right: i32 = classify(true);
    println(left);
    println(right);
    return left + right;
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 5);
        assert_eq!(output.stdout, vec!["2", "3"]);
    }

    #[test]
    fn runs_match_binding_patterns() {
        let (source, hir) = analyzed_hir(
            "\
fn classify(value: i32) -> i32 {
    return match (value) { 0 => 10, other => other + 2 };
}

fn main() -> i32 {
    let flag: bool = false;
    match (flag) {
        true => {
            println(\"true\");
        }
        current => {
            if (current) {
                println(\"unexpected\");
            } else {
                println(\"false\");
            }
        }
    }
    let code: i32 = classify(4);
    println(code);
    return code;
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 6);
        assert_eq!(output.stdout, vec!["false", "6"]);
    }

    #[test]
    fn runs_payload_enum_constructors_and_matches() {
        let (source, hir) = analyzed_hir(
            "\
enum Result {
    Ok(i32),
    Err(string),
    Empty,
}

fn score(result: Result) -> i32 {
    return match (result) {
        Result.Ok(value) => value,
        Result.Err(_) => 0,
        Result.Empty => -1,
    };
}

fn main() -> i32 {
    let ok: Result = Result.Ok(7);
    let err: Result = Result.Err(\"bad\");
    let empty: Result = Result.Empty;
    println(score(ok));
    println(score(err));
    println(score(empty));
    return score(ok);
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 7);
        assert_eq!(output.stdout, vec!["7", "0", "-1"]);
    }

    #[test]
    fn runs_logical_short_circuit_operators() {
        let (source, hir) = analyzed_hir(
            "\
fn main() -> i32 {
    if (false && 8 / 0 == 0) {
        return 1;
    }
    if (true || 8 / 0 == 0) {
        println(\"short-circuit\");
        return 7;
    }
    return 0;
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 7);
        assert_eq!(output.stdout, vec!["short-circuit"]);
    }

    #[test]
    fn runs_modulo_operator() {
        let (source, hir) = analyzed_hir(
            "\
fn main() -> i32 {
    let bucket: i32 = 10 % 3;
    println(bucket);
    return bucket;
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 1);
        assert_eq!(output.stdout, vec!["1"]);
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
    fn env_lookup_matches_windows_case_insensitive_behavior() {
        let (source, hir) = analyzed_hir(
            "\
fn main() -> i32 {
    let present: bool = env_has(\"PATH\");
    println(present);
    if (present) {
        let value: string = env_get(\"PATH\");
        println(value);
        return len(value);
    }
    return 0;
}
",
        );

        let mut env = BTreeMap::new();
        env.insert("Path".to_string(), "ready".to_string());
        let context = RunContext {
            argv: Vec::new(),
            env,
            current_dir: ".".into(),
        };

        let output = run_program_with_context(&source, &hir, context).expect("program should run");

        if cfg!(windows) {
            assert_eq!(output.exit_code, 5);
            assert_eq!(output.stdout, vec!["true", "ready"]);
        } else {
            assert_eq!(output.exit_code, 0);
            assert_eq!(output.stdout, vec!["false"]);
        }
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
    fn runs_long_left_associative_string_concat_chain() {
        let (source, hir) = analyzed_hir(
            "\
fn main() -> i32 {
    let message: string = \"a\" + \"b\" + \"c\" + \"d\" + \"e\" + \"f\" + \"g\" + \"h\" + \"i\" + \"j\" + \"k\" + \"l\" + \"m\" + \"n\" + \"o\" + \"p\" + \"q\" + \"r\" + \"s\" + \"t\" + \"u\" + \"v\" + \"w\" + \"x\" + \"y\" + \"z\";
    println(message);
    return string_len(message);
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 26);
        assert_eq!(output.stdout, vec!["abcdefghijklmnopqrstuvwxyz"]);
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
    fn runs_string_list_builtins() {
        let (source, hir) = analyzed_hir(
            "\
fn main() -> i32 {
    let mut lines: string_list = string_list_new();
    lines = string_list_push(lines, \"alpha\");
    lines = string_list_push(lines, \"beta\");
    println(len(lines));
    println(string_list_join(lines, \", \"));
    return len(lines);
}
",
        );

        let output = run_program(&source, &hir).expect("program should run");
        assert_eq!(output.exit_code, 2);
        assert_eq!(output.stdout, vec!["2", "alpha, beta"]);
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
