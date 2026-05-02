use super::*;

impl<'a> Interpreter<'a> {
    pub(in crate::interpreter) fn new(
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

    pub(in crate::interpreter) fn run_main(mut self) -> Result<RunOutput, Diagnostic> {
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

    pub(in crate::interpreter) fn call_declared_function(
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
}
