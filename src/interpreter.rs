use std::collections::{BTreeMap, HashMap};

use crate::diagnostics::Diagnostic;
use crate::hir::{
    BinaryOp, Block, Expr, ExprKind, ItemKind, Param, Place, PlaceKind, Program, Stmt, StmtKind,
    UnaryOp,
};
use crate::source::{SourceFile, Span};

pub struct RunOutput {
    pub exit_code: i32,
    pub stdout: Vec<String>,
}

pub fn run_program(source: &SourceFile, program: &Program) -> Result<RunOutput, Diagnostic> {
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
    Array(Vec<Value>),
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
    fn new(source: &'a SourceFile, program: &'a Program) -> Result<Self, Diagnostic> {
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
            ControlFlow::Continue => Err(self.runtime_error(
                "R0005",
                format!(
                    "function `{}` completed without returning a value",
                    function.name
                ),
                function.span,
            )),
        }
    }

    fn exec_block(&mut self, block: &Block, frame: &mut Frame) -> Result<ControlFlow, Diagnostic> {
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
        &self,
        frame: &mut Frame,
        target: &Place,
        next_value: Value,
    ) -> Result<(), Diagnostic> {
        match &target.kind {
            PlaceKind::Local { name } => {
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
                Ok(())
            }
            PlaceKind::Field { base, field } => {
                let slot = lookup_slot_mut(frame, base).ok_or_else(|| {
                    self.runtime_error(
                        "R0006",
                        format!("assignment to unknown variable `{base}`"),
                        target.span,
                    )
                })?;
                if !slot.mutable {
                    return Err(self.runtime_error(
                        "R0025",
                        format!("cannot assign to field `{field}` on immutable variable `{base}`"),
                        target.span,
                    ));
                }

                match &mut slot.value {
                    Value::Struct { fields, .. } => {
                        let existing = fields.get_mut(field).ok_or_else(|| {
                            self.runtime_error(
                                "R0026",
                                format!("struct value does not contain field `{field}`"),
                                target.span,
                            )
                        })?;
                        *existing = next_value;
                        Ok(())
                    }
                    other => Err(self.runtime_error(
                        "R0027",
                        format!(
                            "field assignment requires a struct value, got `{}`",
                            other.display()
                        ),
                        target.span,
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
                let index_expr = index;
                let index_value = self.eval_expr(index_expr, frame)?;

                let Value::Array(elements) = base_value else {
                    return Err(self.runtime_error(
                        "R0028",
                        format!(
                            "index access requires an array value, got `{}`",
                            base_value.display()
                        ),
                        expr.span,
                    ));
                };

                let Value::I32(index) = index_value else {
                    return Err(self.runtime_error(
                        "R0029",
                        format!(
                            "array index must evaluate to `i32`, got `{}`",
                            index_value.display()
                        ),
                        index_expr.span,
                    ));
                };

                if index < 0 {
                    return Err(self.runtime_error(
                        "R0030",
                        format!("array index cannot be negative, got `{index}`"),
                        index_expr.span,
                    ));
                }

                let index = usize::try_from(index).expect("non-negative i32 should fit in usize");
                elements.get(index).cloned().ok_or_else(|| {
                    self.runtime_error(
                        "R0031",
                        format!(
                            "array index `{index}` is out of bounds for length {}",
                            elements.len()
                        ),
                        expr.span,
                    )
                })
            }
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
                .ok_or_else(|| self.runtime_error("R0018", "integer addition overflowed", span)),
            (BinaryOp::Subtract, Value::I32(left), Value::I32(right)) => left
                .checked_sub(right)
                .map(Value::I32)
                .ok_or_else(|| self.runtime_error("R0019", "integer subtraction overflowed", span)),
            (BinaryOp::Multiply, Value::I32(left), Value::I32(right)) => {
                left.checked_mul(right).map(Value::I32).ok_or_else(|| {
                    self.runtime_error("R0020", "integer multiplication overflowed", span)
                })
            }
            (BinaryOp::Divide, Value::I32(_), Value::I32(0)) => {
                Err(self.runtime_error("R0021", "division by zero", span))
            }
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
            (BinaryOp::Divide, Value::F32(_), Value::F32(0.0)) => {
                Err(self.runtime_error("R0021", "division by zero", span))
            }
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
}
