use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::hir::{
    BinaryOp, Block, EnumVariantPayloadPattern as MatchPatternPayload, Expr, ExprKind, ItemKind,
    MatchExprArm, MatchPattern, MatchPatternKind, Param, Place, PlaceKind, Program, Stmt, StmtKind,
    UnaryOp,
};
use crate::source::{SourceFile, Span};

mod builtins;
mod flow;
mod frame;
mod host;
mod value;

use self::flow::{ConditionFlow, ControlFlow, EvalFlow};
use self::frame::{Frame, Slot, lookup_slot, lookup_slot_mut};
pub use self::host::RunContext;
use self::value::Value;

#[derive(Debug)]
pub struct RunOutput {
    pub exit_code: i32,
    pub stdout: Vec<String>,
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
            ExprKind::Block { statements, value } => {
                self.eval_block_expr(statements, value, frame, expr.span)
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

    fn eval_block_expr(
        &mut self,
        statements: &[Stmt],
        value: &Expr,
        frame: &mut Frame,
        span: Span,
    ) -> Result<EvalFlow, Diagnostic> {
        frame.scopes.push(HashMap::new());
        for statement in statements {
            match self.exec_statement(statement, frame)? {
                ControlFlow::Continue => {}
                ControlFlow::Return(value) => {
                    frame.scopes.pop();
                    return Ok(EvalFlow::Return(value));
                }
                ControlFlow::Break => {
                    frame.scopes.pop();
                    return Err(self.runtime_error(
                        "R0137",
                        "`break` cannot leave a block-valued expression",
                        span,
                    ));
                }
                ControlFlow::LoopContinue => {
                    frame.scopes.pop();
                    return Err(self.runtime_error(
                        "R0138",
                        "`continue` cannot leave a block-valued expression",
                        span,
                    ));
                }
            }
        }
        let result = self.eval_expr(value, frame);
        frame.scopes.pop();
        result
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
            MatchPatternKind::Struct {
                struct_name,
                fields,
            } => match scrutinee {
                Value::Struct {
                    name,
                    fields: values,
                } => {
                    if name != struct_name {
                        return Ok(false);
                    }
                    Ok(fields.iter().all(|field| values.contains_key(&field.name)))
                }
                other => Err(self.runtime_error(
                    "R0037",
                    format!(
                        "match struct pattern cannot be applied to runtime value `{}`",
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
            MatchPatternKind::Struct { fields, .. } => {
                let Value::Struct { fields: values, .. } = scrutinee else {
                    return Err(self.runtime_error(
                        "R0043",
                        "struct pattern binding requires a struct value",
                        pattern.span,
                    ));
                };
                for field in fields {
                    let Some(value) = values.get(&field.name) else {
                        return Err(self.runtime_error(
                            "R0043",
                            format!(
                                "struct pattern binding `{}` requires field `{}`",
                                field.binding, field.name
                            ),
                            field.span,
                        ));
                    };
                    frame.scopes.last_mut().expect("scope should exist").insert(
                        field.binding.clone(),
                        Slot {
                            mutable: false,
                            value: value.clone(),
                        },
                    );
                }
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
            MatchPatternKind::Struct {
                struct_name,
                fields,
            } => {
                let fields = fields
                    .iter()
                    .map(|field| field.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{struct_name} {{ {fields} }}")
            }
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
fn place_root_name<'a>(place: &'a Place) -> &'a str {
    match &place.kind {
        PlaceKind::Local { name } => name.as_str(),
        PlaceKind::Field { base, .. } | PlaceKind::Index { base, .. } => place_root_name(base),
    }
}

#[cfg(test)]
mod tests;
