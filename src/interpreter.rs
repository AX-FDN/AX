use std::collections::HashMap;

use crate::ast::{BinaryOp, Block, Expr, ExprKind, ItemKind, Param, Program, Stmt, StmtKind, UnaryOp};
use crate::diagnostics::Diagnostic;
use crate::source::{SourceFile, Span};

pub struct RunOutput {
    pub exit_code: i32,
    pub stdout: Vec<String>,
}

pub fn run_program(
    source: &SourceFile,
    program: &Program,
) -> Result<RunOutput, Diagnostic> {
    Interpreter::new(source, program)?.run_main()
}

struct Interpreter<'a> {
    source: &'a SourceFile,
    functions: HashMap<String, FunctionDef<'a>>,
    stdout: Vec<String>,
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
    Void,
}

enum ControlFlow {
    Continue,
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
            Self::Void => "<void>".to_string(),
        }
    }
}

impl<'a> Interpreter<'a> {
    fn new(source: &'a SourceFile, program: &'a Program) -> Result<Self, Diagnostic> {
        let mut functions = HashMap::new();
        for item in &program.items {
            if let ItemKind::Function {
                name,
                params,
                body,
                ..
            } = &item.kind
            {
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

        let function = self.functions.get(name).copied().ok_or_else(|| {
            self.runtime_error(
                "R0003",
                format!("call to unknown function `{name}`"),
                span,
            )
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
            ControlFlow::Continue => Err(self.runtime_error(
                "R0005",
                format!("function `{}` completed without returning a value", function.name),
                function.span,
            )),
        }
    }

    fn exec_block(
        &mut self,
        block: &Block,
        frame: &mut Frame,
    ) -> Result<ControlFlow, Diagnostic> {
        frame.scopes.push(HashMap::new());
        for statement in &block.statements {
            match self.exec_statement(statement, frame)? {
                ControlFlow::Continue => {}
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
                frame
                    .scopes
                    .last_mut()
                    .expect("scope should exist")
                    .insert(
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
                match &target.kind {
                    ExprKind::Name { value: name } => {
                        let slot = lookup_slot_mut(frame, name).ok_or_else(|| {
                            self.runtime_error(
                                "R0006",
                                format!("assignment to unknown variable `{name}`"),
                                target.span,
                            )
                        })?;
                        if !slot.mutable {
                            return Err(self.runtime_error(
                                "R0007",
                                format!("cannot assign to immutable variable `{name}`"),
                                target.span,
                            ));
                        }
                        slot.value = next_value;
                        Ok(ControlFlow::Continue)
                    }
                    _ => Err(self.runtime_error(
                        "R0008",
                        "assignment target must be a variable name",
                        target.span,
                    )),
                }
            }
            StmtKind::Expr { expr } => {
                self.eval_expr(expr, frame)?;
                Ok(ControlFlow::Continue)
            }
            StmtKind::Return { value } => {
                let value = match value {
                    Some(expr) => self.eval_expr(expr, frame)?,
                    None => Value::Void,
                };
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

    fn eval_condition(
        &mut self,
        expr: &Expr,
        frame: &mut Frame,
    ) -> Result<bool, Diagnostic> {
        match self.eval_expr(expr, frame)? {
            Value::Bool(value) => Ok(value),
            other => Err(self.runtime_error(
                "R0009",
                format!("condition must evaluate to `bool`, got `{}`", other.display()),
                expr.span,
            )),
        }
    }

    fn eval_expr(
        &mut self,
        expr: &Expr,
        frame: &mut Frame,
    ) -> Result<Value, Diagnostic> {
        match &expr.kind {
            ExprKind::Int { value } => i32::try_from(*value)
                .map(Value::I32)
                .map_err(|_| {
                    self.runtime_error(
                        "R0010",
                        "integer literal is out of range for runtime `i32`",
                        expr.span,
                    )
                }),
            ExprKind::Float { value } => Ok(Value::F32(*value as f32)),
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
                    (UnaryOp::Negate, Value::I32(value)) => value
                        .checked_neg()
                        .map(Value::I32)
                        .ok_or_else(|| {
                            self.runtime_error(
                                "R0012",
                                "integer negation overflowed",
                                expr.span,
                            )
                        }),
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
            ExprKind::Call { callee, arguments } => match &callee.kind {
                ExprKind::Name { value } => {
                    let argument_values = arguments
                        .iter()
                        .map(|argument| self.eval_expr(argument, frame))
                        .collect::<Result<Vec<_>, _>>()?;
                    self.call_function(value, argument_values, expr.span)
                }
                _ => Err(self.runtime_error(
                    "R0014",
                    "call target must be a function name",
                    callee.span,
                )),
            },
            ExprKind::Field { .. } => Err(self.runtime_error(
                "R0015",
                "field access is not executable in the minimal interpreter yet",
                expr.span,
            )),
            ExprKind::Error => Err(self.runtime_error(
                "R0016",
                "cannot execute an invalid expression",
                expr.span,
            )),
        }
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
                .ok_or_else(|| self.runtime_error("R0017", "integer addition overflowed", span)),
            (BinaryOp::Subtract, Value::I32(left), Value::I32(right)) => left
                .checked_sub(right)
                .map(Value::I32)
                .ok_or_else(|| {
                    self.runtime_error("R0018", "integer subtraction overflowed", span)
                }),
            (BinaryOp::Multiply, Value::I32(left), Value::I32(right)) => left
                .checked_mul(right)
                .map(Value::I32)
                .ok_or_else(|| {
                    self.runtime_error("R0019", "integer multiplication overflowed", span)
                }),
            (BinaryOp::Divide, Value::I32(_), Value::I32(0)) => {
                Err(self.runtime_error("R0020", "division by zero", span))
            }
            (BinaryOp::Divide, Value::I32(left), Value::I32(right)) => left
                .checked_div(right)
                .map(Value::I32)
                .ok_or_else(|| self.runtime_error("R0021", "integer division overflowed", span)),
            (BinaryOp::Add, Value::F32(left), Value::F32(right)) => Ok(Value::F32(left + right)),
            (BinaryOp::Subtract, Value::F32(left), Value::F32(right)) => {
                Ok(Value::F32(left - right))
            }
            (BinaryOp::Multiply, Value::F32(left), Value::F32(right)) => {
                Ok(Value::F32(left * right))
            }
            (BinaryOp::Divide, Value::F32(_), Value::F32(0.0)) => {
                Err(self.runtime_error("R0020", "division by zero", span))
            }
            (BinaryOp::Divide, Value::F32(left), Value::F32(right)) => {
                Ok(Value::F32(left / right))
            }
            (BinaryOp::Equal, left, right) => Ok(Value::Bool(left == right)),
            (BinaryOp::NotEqual, left, right) => Ok(Value::Bool(left != right)),
            (BinaryOp::Less, Value::I32(left), Value::I32(right)) => {
                Ok(Value::Bool(left < right))
            }
            (BinaryOp::LessEqual, Value::I32(left), Value::I32(right)) => {
                Ok(Value::Bool(left <= right))
            }
            (BinaryOp::Greater, Value::I32(left), Value::I32(right)) => {
                Ok(Value::Bool(left > right))
            }
            (BinaryOp::GreaterEqual, Value::I32(left), Value::I32(right)) => {
                Ok(Value::Bool(left >= right))
            }
            (BinaryOp::Less, Value::F32(left), Value::F32(right)) => {
                Ok(Value::Bool(left < right))
            }
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
                "R0022",
                format!(
                    "invalid binary operation for runtime values `{}` and `{}`",
                    left.display(),
                    right.display()
                ),
                span,
            )),
        }
    }

    fn runtime_error(
        &self,
        code: &str,
        message: impl Into<String>,
        span: Span,
    ) -> Diagnostic {
        Diagnostic::new(code, message.into(), self.source, span)
    }
}

fn lookup_slot<'a>(frame: &'a Frame, name: &str) -> Option<&'a Slot> {
    frame
        .scopes
        .iter()
        .rev()
        .find_map(|scope| scope.get(name))
}

fn lookup_slot_mut<'a>(frame: &'a mut Frame, name: &str) -> Option<&'a mut Slot> {
    frame
        .scopes
        .iter_mut()
        .rev()
        .find_map(|scope| scope.get_mut(name))
}

#[cfg(test)]
mod tests {
    use super::run_program;
    use crate::frontend::analyze;
    use crate::source::SourceFile;

    #[test]
    fn runs_loops_functions_and_println() {
        let source = SourceFile::anonymous(
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

        let output = run_program(&source, &analysis.program).expect("program should run");
        assert_eq!(output.exit_code, 3);
        assert_eq!(output.stdout, vec!["3"]);
    }

    #[test]
    fn runs_conditionals() {
        let source = SourceFile::anonymous(
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

        let analysis = analyze(&source);
        assert!(analysis.diagnostics.is_empty());
        let output = run_program(&source, &analysis.program).expect("program should run");
        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, vec!["ready"]);
    }

    #[test]
    fn runs_recursive_functions() {
        let source = SourceFile::anonymous(
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

        let analysis = analyze(&source);
        assert!(analysis.diagnostics.is_empty());
        let output = run_program(&source, &analysis.program).expect("program should run");
        assert_eq!(output.exit_code, 0);
        assert_eq!(output.stdout, vec!["120"]);
    }
}
