use std::collections::{HashMap, HashSet};

use crate::ast::{BinaryOp, Block, Expr, ExprKind, Stmt, StmtKind, UnaryOp};
use crate::diagnostics::Diagnostic;
use crate::source::Span;

use super::helpers::{
    binary_op_name, return_type_message, type_mismatch_suggestion, type_name_as_value_diagnostic,
};
use super::program_info::ProgramInfo;
use super::types::Type;

pub(super) struct TypeChecker<'a, 'b> {
    info: &'a ProgramInfo<'a>,
    return_type: Type,
    scopes: Vec<HashMap<String, Binding>>,
    diagnostics: &'b mut Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
struct Binding {
    mutable: bool,
    ty: Type,
    start: usize,
}

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn new(
        info: &'a ProgramInfo<'a>,
        return_type: Type,
        diagnostics: &'b mut Vec<Diagnostic>,
    ) -> Self {
        Self {
            info,
            return_type,
            scopes: vec![HashMap::new()],
            diagnostics,
        }
    }

    pub(super) fn diagnostics_mut(&mut self) -> &mut Vec<Diagnostic> {
        self.diagnostics
    }

    pub(super) fn return_type(&self) -> &Type {
        &self.return_type
    }

    pub(super) fn check_block(&mut self, block: &Block) {
        self.scopes.push(HashMap::new());
        for statement in &block.statements {
            self.check_statement(statement);
        }
        self.scopes.pop();
    }

    pub(super) fn declare(&mut self, name: &str, ty: Type, mutable: bool, start: usize) {
        let current_scope = self.scopes.last_mut().expect("scope must exist");
        if let Some(previous) =
            current_scope.insert(name.to_string(), Binding { mutable, ty, start })
        {
            let (line, column) = self.info.source.line_col(previous.start);
            self.diagnostics.push(
                Diagnostic::new(
                    "S0001",
                    format!("duplicate definition of `{name}`"),
                    self.info.source,
                    Span::new(start, start + name.len()),
                )
                .with_note(format!("previous definition was at {line}:{column}")),
            );
        }
    }

    fn check_statement(&mut self, statement: &Stmt) {
        match &statement.kind {
            StmtKind::Let {
                mutable,
                name,
                ty,
                initializer,
            } => {
                let declared_type = self.info.resolve_type_ref(ty, self.diagnostics);
                let initializer_type = self.check_expr(initializer);
                self.expect_type_match(
                    &declared_type,
                    &initializer_type,
                    initializer.span,
                    format!(
                        "cannot initialize `{name}` of type `{}` with `{}`",
                        declared_type.describe(),
                        initializer_type.describe()
                    ),
                );
                self.declare(name, declared_type, *mutable, statement.span.start);
            }
            StmtKind::Assign { target, value } => {
                let value_type = self.check_expr(value);
                self.check_assignment_target(target, &value_type, value.span);
            }
            StmtKind::Expr { expr } => {
                self.check_expr(expr);
            }
            StmtKind::Return { value } => {
                let actual_type = match value {
                    Some(expr) => self.check_expr(expr),
                    None => Type::Void,
                };
                self.expect_type_match(
                    &self.return_type.clone(),
                    &actual_type,
                    statement.span,
                    return_type_message(&self.return_type, &actual_type),
                );
            }
            StmtKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition_type = self.check_expr(condition);
                self.expect_type_match(
                    &Type::Bool,
                    &condition_type,
                    condition.span,
                    format!(
                        "`if` condition must be `bool`, found `{}`",
                        condition_type.describe()
                    ),
                );
                self.check_block(then_branch);
                if let Some(block) = else_branch {
                    self.check_block(block);
                }
            }
            StmtKind::While { condition, body } => {
                let condition_type = self.check_expr(condition);
                self.expect_type_match(
                    &Type::Bool,
                    &condition_type,
                    condition.span,
                    format!(
                        "`while` condition must be `bool`, found `{}`",
                        condition_type.describe()
                    ),
                );
                self.check_block(body);
            }
            StmtKind::For {
                initializer,
                condition,
                step,
                body,
            } => {
                self.check_for_statement(
                    initializer.as_deref(),
                    condition.as_ref(),
                    step.as_deref(),
                    body,
                );
            }
            StmtKind::Block { block } => self.check_block(block),
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> Type {
        match &expr.kind {
            ExprKind::Int { value } => {
                if i32::try_from(*value).is_err() {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0009",
                            "integer literal is out of range for `i32`",
                            self.info.source,
                            expr.span,
                        )
                        .with_suggestion("use a value that fits in the AX `i32` range"),
                    );
                    Type::Error
                } else {
                    Type::I32
                }
            }
            ExprKind::Float { value } => {
                let narrowed = *value as f32;
                if value.is_finite() && !narrowed.is_finite() {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0010",
                            "float literal is out of range for `f32`",
                            self.info.source,
                            expr.span,
                        )
                        .with_suggestion("use a smaller floating-point value that fits in `f32`"),
                    );
                    Type::Error
                } else {
                    Type::F32
                }
            }
            ExprKind::Bool { .. } => Type::Bool,
            ExprKind::String { .. } => Type::String,
            ExprKind::Name { value } => match self.lookup(value) {
                Some(binding) => binding.ty,
                None if self.info.functions.contains_key(value) => {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0011",
                            format!("function `{value}` cannot be used as a value"),
                            self.info.source,
                            expr.span,
                        )
                        .with_suggestion(format!(
                            "call `{value}` with parentheses, for example `{value}(...)`",
                        )),
                    );
                    Type::Error
                }
                None if self.info.named_types.contains_key(value) => {
                    self.diagnostics.push(type_name_as_value_diagnostic(
                        self.info.source,
                        expr.span,
                        value,
                        self.info.named_types.get(value).expect("type must exist"),
                    ));
                    Type::Error
                }
                None => {
                    self.diagnostics.push(self.undefined_variable_diagnostic(
                        value,
                        expr.span,
                        format!("declare `{value}` before using it"),
                    ));
                    Type::Error
                }
            },
            ExprKind::Unary { op, expr: inner } => {
                let inner_type = self.check_expr(inner);
                if inner_type.is_error() {
                    return Type::Error;
                }

                match op {
                    UnaryOp::Negate if inner_type.is_numeric() => inner_type,
                    UnaryOp::Negate => {
                        self.diagnostics.push(Diagnostic::new(
                            "S0012",
                            format!(
                                "unary `-` expects `i32` or `f32`, found `{}`",
                                inner_type.describe()
                            ),
                            self.info.source,
                            expr.span,
                        ));
                        Type::Error
                    }
                    UnaryOp::Not if inner_type == Type::Bool => Type::Bool,
                    UnaryOp::Not => {
                        self.diagnostics.push(Diagnostic::new(
                            "S0013",
                            format!(
                                "unary `!` expects `bool`, found `{}`",
                                inner_type.describe()
                            ),
                            self.info.source,
                            expr.span,
                        ));
                        Type::Error
                    }
                }
            }
            ExprKind::Binary { op, left, right } => {
                let left_type = self.check_expr(left);
                let right_type = self.check_expr(right);
                if left_type.is_error() || right_type.is_error() {
                    return Type::Error;
                }

                match op {
                    BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => {
                        if left_type.is_numeric() && left_type == right_type {
                            left_type
                        } else {
                            self.diagnostics.push(
                                Diagnostic::new(
                                    "S0014",
                                    format!(
                                        "operator `{}` expects matching numeric operands, found `{}` and `{}`",
                                        binary_op_name(*op),
                                        left_type.describe(),
                                        right_type.describe()
                                    ),
                                    self.info.source,
                                    expr.span,
                                ),
                            );
                            Type::Error
                        }
                    }
                    BinaryOp::Equal | BinaryOp::NotEqual => {
                        if left_type == right_type && left_type.is_equality_comparable() {
                            Type::Bool
                        } else {
                            self.diagnostics.push(
                                Diagnostic::new(
                                    "S0015",
                                    format!(
                                        "operator `{}` expects matching comparable operands, found `{}` and `{}`",
                                        binary_op_name(*op),
                                        left_type.describe(),
                                        right_type.describe()
                                    ),
                                    self.info.source,
                                    expr.span,
                                ),
                            );
                            Type::Error
                        }
                    }
                    BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => {
                        if left_type.is_numeric() && left_type == right_type {
                            Type::Bool
                        } else {
                            self.diagnostics.push(
                                Diagnostic::new(
                                    "S0016",
                                    format!(
                                        "operator `{}` expects matching numeric operands, found `{}` and `{}`",
                                        binary_op_name(*op),
                                        left_type.describe(),
                                        right_type.describe()
                                    ),
                                    self.info.source,
                                    expr.span,
                                ),
                            );
                            Type::Error
                        }
                    }
                }
            }
            ExprKind::Call { callee, arguments } => match &callee.kind {
                ExprKind::Name { value } if value == "println" => {
                    for argument in arguments {
                        self.check_expr(argument);
                    }
                    Type::Void
                }
                ExprKind::Name { value } => {
                    let signature = self.info.functions.get(value).cloned();
                    let argument_types = arguments
                        .iter()
                        .map(|argument| self.check_expr(argument))
                        .collect::<Vec<_>>();

                    match signature {
                        Some(signature) => {
                            if signature.params.len() != argument_types.len() {
                                self.diagnostics.push(Diagnostic::new(
                                    "S0017",
                                    format!(
                                        "function `{value}` expects {} argument(s), found {}",
                                        signature.params.len(),
                                        argument_types.len()
                                    ),
                                    self.info.source,
                                    expr.span,
                                ));
                            }

                            for (argument, parameter) in
                                argument_types.iter().zip(signature.params.iter())
                            {
                                self.expect_type_match(
                                    &parameter.ty,
                                    argument,
                                    expr.span,
                                    format!(
                                        "function `{value}` expects argument `{}` to be `{}`, found `{}`",
                                        parameter.name,
                                        parameter.ty.describe(),
                                        argument.describe()
                                    ),
                                );
                            }

                            signature.return_type
                        }
                        None if self.lookup(value).is_some() => {
                            self.diagnostics.push(
                                Diagnostic::new(
                                    "S0018",
                                    format!("variable `{value}` is not callable"),
                                    self.info.source,
                                    callee.span,
                                )
                                .with_suggestion(
                                    "only function names and builtin functions can be called",
                                ),
                            );
                            Type::Error
                        }
                        None => {
                            self.diagnostics.push(
                                Diagnostic::new(
                                    "S0007",
                                    format!("call to undefined function `{value}`"),
                                    self.info.source,
                                    callee.span,
                                )
                                .with_suggestion(format!(
                                    "declare `{value}` or fix the call target"
                                )),
                            );
                            Type::Error
                        }
                    }
                }
                _ => {
                    self.check_expr(callee);
                    for argument in arguments {
                        self.check_expr(argument);
                    }
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0019",
                            "call target must be a function name",
                            self.info.source,
                            callee.span,
                        )
                        .with_suggestion("use a direct function call like `name(arg1, arg2)`"),
                    );
                    Type::Error
                }
            },
            ExprKind::StructLiteral { name, fields } => {
                let struct_info = match self.info.named_types.get(name).cloned() {
                    Some(Type::Struct(struct_name)) => self
                        .info
                        .structs
                        .get(&struct_name)
                        .cloned()
                        .map(|info| (struct_name, info)),
                    Some(other) => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0024",
                                format!(
                                    "`{name}` cannot be used as a struct literal because it is `{}`",
                                    other.describe()
                                ),
                                self.info.source,
                                expr.span,
                            )
                            .with_suggestion(
                                "use the name of a declared `struct` for struct literal construction",
                            ),
                        );
                        None
                    }
                    None => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0006",
                                format!("unknown type `{name}`"),
                                self.info.source,
                                expr.span,
                            )
                            .with_suggestion("declare the struct before constructing it"),
                        );
                        None
                    }
                };

                let Some((struct_name, struct_info)) = struct_info else {
                    for field in fields {
                        self.check_expr(&field.value);
                    }
                    return Type::Error;
                };

                let mut seen_fields = HashSet::new();
                for field in fields {
                    let value_type = self.check_expr(&field.value);
                    if !seen_fields.insert(field.name.clone()) {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0025",
                                format!(
                                    "duplicate field `{}` in struct literal `{struct_name}`",
                                    field.name
                                ),
                                self.info.source,
                                field.span,
                            )
                            .with_suggestion("keep only one initializer for each field"),
                        );
                        continue;
                    }

                    match struct_info.fields.get(&field.name) {
                        Some(expected_field) => {
                            self.expect_type_match(
                                &expected_field.ty,
                                &value_type,
                                field.value.span,
                                format!(
                                    "field `{}` of `{struct_name}` expects `{}`, found `{}`",
                                    field.name,
                                    expected_field.ty.describe(),
                                    value_type.describe()
                                ),
                            );
                        }
                        None => {
                            self.diagnostics.push(
                                Diagnostic::new(
                                    "S0027",
                                    format!(
                                        "struct `{struct_name}` does not have a field `{}`",
                                        field.name
                                    ),
                                    self.info.source,
                                    field.span,
                                )
                                .with_suggestion(
                                    "use an existing field name from the struct declaration",
                                ),
                            );
                        }
                    }
                }

                for field_name in struct_info.fields.keys() {
                    if !seen_fields.contains(field_name) {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0026",
                                format!(
                                    "struct literal `{struct_name}` is missing field `{field_name}`",
                                ),
                                self.info.source,
                                expr.span,
                            )
                            .with_suggestion(format!(
                                "provide `{field_name}: ...` in the struct literal",
                            )),
                        );
                    }
                }

                Type::Struct(struct_name)
            }
            ExprKind::ArrayLiteral { elements } => {
                let Some((first, rest)) = elements.split_first() else {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0032",
                            "empty array literals are not supported yet",
                            self.info.source,
                            expr.span,
                        )
                        .with_suggestion("add at least one element to the array literal"),
                    );
                    return Type::Error;
                };

                let element_type = self.check_expr(first);
                for element in rest {
                    let current_type = self.check_expr(element);
                    self.expect_type_match(
                        &element_type,
                        &current_type,
                        element.span,
                        format!(
                            "array literal element expects `{}`, found `{}`",
                            element_type.describe(),
                            current_type.describe()
                        ),
                    );
                }

                if element_type.is_error() {
                    Type::Error
                } else {
                    Type::Array {
                        element: Box::new(element_type),
                        length: elements.len(),
                    }
                }
            }
            ExprKind::Field { base, field } => {
                if let ExprKind::Name { value: enum_name } = &base.kind {
                    if let Some(enum_info) = self.info.enums.get(enum_name) {
                        if enum_info.variants.contains(field) {
                            return Type::Enum(enum_name.clone());
                        }

                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0029",
                                format!("enum `{enum_name}` does not have a variant `{field}`"),
                                self.info.source,
                                expr.span,
                            )
                            .with_suggestion(
                                "use an existing variant name from the enum declaration",
                            ),
                        );
                        return Type::Error;
                    }
                }

                let base_type = self.check_expr(base);
                match base_type {
                    Type::Struct(struct_name) => {
                        let struct_info = self.info.structs.get(&struct_name).cloned();
                        match struct_info {
                            Some(struct_info) => match struct_info.fields.get(field) {
                                Some(field_info) => field_info.ty.clone(),
                                None => {
                                    self.diagnostics.push(
                                        Diagnostic::new(
                                            "S0020",
                                            format!(
                                                "struct `{struct_name}` does not have a field `{field}`",
                                            ),
                                            self.info.source,
                                            expr.span,
                                        )
                                        .with_suggestion(
                                            "use an existing field name from the struct declaration",
                                        ),
                                    );
                                    Type::Error
                                }
                            },
                            None => Type::Error,
                        }
                    }
                    Type::Error => Type::Error,
                    other => {
                        self.diagnostics.push(Diagnostic::new(
                            "S0021",
                            format!(
                                "field access expects a struct value, found `{}`",
                                other.describe()
                            ),
                            self.info.source,
                            expr.span,
                        ));
                        Type::Error
                    }
                }
            }
            ExprKind::Index { base, index } => {
                let base_type = self.check_expr(base);
                let index_type = self.check_expr(index);
                self.expect_type_match(
                    &Type::I32,
                    &index_type,
                    index.span,
                    format!(
                        "array index must be `i32`, found `{}`",
                        index_type.describe()
                    ),
                );

                match base_type {
                    Type::Array { element, .. } => *element,
                    Type::Error => Type::Error,
                    other => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0033",
                                format!(
                                    "index access expects an array value, found `{}`",
                                    other.describe()
                                ),
                                self.info.source,
                                expr.span,
                            )
                            .with_suggestion("index into an array value like `values[0]`"),
                        );
                        Type::Error
                    }
                }
            }
            ExprKind::Error => Type::Error,
        }
    }

    fn check_assignment_target(&mut self, target: &Expr, value_type: &Type, value_span: Span) {
        match &target.kind {
            ExprKind::Name { value: name } => {
                self.check_variable_assignment(name, target.span, value_span, value_type);
            }
            ExprKind::Field { base, field } => {
                self.check_field_assignment(base, field, target.span, value_span, value_type);
            }
            ExprKind::Index { base, index } => {
                self.check_array_assignment(base, index, target.span, value_span, value_type);
            }
            _ => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0008",
                        "assignment target must be a mutable variable, direct mutable struct field, or direct mutable array element",
                        self.info.source,
                        target.span,
                    )
                    .with_suggestion(
                        "assign to `value = expr;`, `point.x = expr;`, or `values[index] = expr;`",
                    ),
                );
                self.check_expr(target);
            }
        }
    }

    fn check_for_statement(
        &mut self,
        initializer: Option<&Stmt>,
        condition: Option<&Expr>,
        step: Option<&Stmt>,
        body: &Block,
    ) {
        self.scopes.push(HashMap::new());

        if let Some(statement) = initializer {
            self.check_for_header_statement(statement);
        }

        if let Some(condition) = condition {
            let condition_type = self.check_expr(condition);
            self.expect_type_match(
                &Type::Bool,
                &condition_type,
                condition.span,
                format!(
                    "`for` condition must be `bool`, found `{}`",
                    condition_type.describe()
                ),
            );
        }

        self.check_block(body);

        if let Some(statement) = step {
            self.check_for_header_statement(statement);
        }

        self.scopes.pop();
    }

    fn check_for_header_statement(&mut self, statement: &Stmt) {
        match &statement.kind {
            StmtKind::Let { .. } | StmtKind::Assign { .. } | StmtKind::Expr { .. } => {
                self.check_statement(statement);
            }
            _ => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0031",
                        "`for` headers only support `let`, assignment, or expression clauses",
                        self.info.source,
                        statement.span,
                    )
                    .with_suggestion(
                        "use a header like `for (let i: i32 = 0; i < 3; i = i + 1) { ... }`",
                    ),
                );
            }
        }
    }

    fn check_variable_assignment(
        &mut self,
        name: &str,
        target_span: Span,
        value_span: Span,
        value_type: &Type,
    ) {
        match self.lookup(name) {
            Some(binding) if binding.mutable => {
                self.expect_type_match(
                    &binding.ty,
                    value_type,
                    value_span,
                    format!(
                        "cannot assign `{}` to `{name}` of type `{}`",
                        value_type.describe(),
                        binding.ty.describe()
                    ),
                );
            }
            Some(binding) => {
                let (line, column) = self.info.source.line_col(binding.start);
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0003",
                        format!("cannot assign to immutable variable `{name}`"),
                        self.info.source,
                        target_span,
                    )
                    .with_note(format!(
                        "`{name}` was declared immutable at {line}:{column}"
                    ))
                    .with_note("AX fixes local mutability at the declaration site; later assignments require `let mut`")
                    .with_suggestion(format!("declare `{name}` with `let mut`")),
                );
            }
            None => {
                self.diagnostics.push(self.undefined_variable_diagnostic(
                    name,
                    target_span,
                    format!("declare `{name}` before assigning to it"),
                ));
            }
        }
    }

    fn check_field_assignment(
        &mut self,
        base: &Expr,
        field: &str,
        target_span: Span,
        value_span: Span,
        value_type: &Type,
    ) {
        let ExprKind::Name { value: base_name } = &base.kind else {
            self.diagnostics.push(
                Diagnostic::new(
                    "S0008",
                    "assignment target must be a mutable variable or direct mutable struct field",
                    self.info.source,
                    target_span,
                )
                .with_suggestion("use a direct field write like `point.x = expr;`"),
            );
            self.check_expr(base);
            return;
        };

        match self.lookup(base_name) {
            Some(binding) if !binding.mutable => {
                let (line, column) = self.info.source.line_col(binding.start);
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0030",
                        format!(
                            "cannot assign to field `{field}` on immutable variable `{base_name}`"
                        ),
                        self.info.source,
                        target_span,
                    )
                    .with_note(format!(
                        "`{base_name}` was declared immutable at {line}:{column}"
                    ))
                    .with_suggestion(format!(
                        "declare `{base_name}` with `let mut` before assigning to `{base_name}.{field}`"
                    )),
                );
            }
            Some(binding) => match binding.ty {
                Type::Struct(struct_name) => {
                    let struct_info = self.info.structs.get(&struct_name).cloned();
                    match struct_info {
                        Some(struct_info) => match struct_info.fields.get(field) {
                            Some(field_info) => {
                                self.expect_type_match(
                                    &field_info.ty,
                                    value_type,
                                    value_span,
                                    format!(
                                        "cannot assign `{}` to field `{field}` of `{struct_name}` because the field has type `{}`",
                                        value_type.describe(),
                                        field_info.ty.describe()
                                    ),
                                );
                            }
                            None => {
                                self.diagnostics.push(
                                    Diagnostic::new(
                                        "S0020",
                                        format!(
                                            "struct `{struct_name}` does not have a field `{field}`",
                                        ),
                                        self.info.source,
                                        target_span,
                                    )
                                    .with_suggestion(
                                        "use an existing field name from the struct declaration",
                                    ),
                                );
                            }
                        },
                        None => {}
                    }
                }
                Type::Error => {}
                other => {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0030",
                            format!(
                                "field assignment requires a mutable struct variable, found `{}`",
                                other.describe()
                            ),
                            self.info.source,
                            target_span,
                        )
                        .with_suggestion(
                            "assign to a field on a mutable struct variable like `point.x = expr;`",
                        ),
                    );
                }
            },
            None => {
                self.diagnostics.push(self.undefined_variable_diagnostic(
                    base_name,
                    base.span,
                    format!("declare `{base_name}` before assigning to its field"),
                ));
            }
        }
    }

    fn check_array_assignment(
        &mut self,
        base: &Expr,
        index: &Expr,
        target_span: Span,
        value_span: Span,
        value_type: &Type,
    ) {
        let ExprKind::Name { value: base_name } = &base.kind else {
            self.diagnostics.push(
                Diagnostic::new(
                    "S0008",
                    "assignment target must be a mutable variable, direct mutable struct field, or direct mutable array element",
                    self.info.source,
                    target_span,
                )
                .with_suggestion("use a direct array write like `values[index] = expr;`"),
            );
            self.check_expr(base);
            self.check_expr(index);
            return;
        };

        match self.lookup(base_name) {
            Some(binding) if !binding.mutable => {
                let (line, column) = self.info.source.line_col(binding.start);
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0003",
                        format!("cannot assign through immutable array variable `{base_name}`"),
                        self.info.source,
                        target_span,
                    )
                    .with_note(format!(
                        "`{base_name}` was declared immutable at {line}:{column}"
                    ))
                    .with_note("AX fixes local mutability at the declaration site; array element writes require `let mut`")
                    .with_suggestion(format!("declare `{base_name}` with `let mut`")),
                );
            }
            Some(binding) => {
                let index_type = self.check_expr(index);
                self.expect_type_match(
                    &Type::I32,
                    &index_type,
                    index.span,
                    format!(
                        "array index must be `i32`, found `{}`",
                        index_type.describe()
                    ),
                );

                match binding.ty {
                    Type::Array { element, .. } => {
                        self.expect_type_match(
                            element.as_ref(),
                            value_type,
                            value_span,
                            format!(
                                "cannot assign `{}` to an array element of type `{}`",
                                value_type.describe(),
                                element.describe()
                            ),
                        );
                    }
                    Type::Error => {}
                    other => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0033",
                                format!(
                                    "array element assignment requires an array value, found `{}`",
                                    other.describe()
                                ),
                                self.info.source,
                                target_span,
                            )
                            .with_suggestion(
                                "assign through a mutable array variable like `values[index] = expr;`",
                            ),
                        );
                    }
                }
            }
            None => {
                self.diagnostics.push(self.undefined_variable_diagnostic(
                    base_name,
                    base.span,
                    format!("declare `{base_name}` before assigning to its elements"),
                ));
            }
        }
    }

    fn lookup(&self, name: &str) -> Option<Binding> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    fn visible_binding_names(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut names = Vec::new();

        for scope in self.scopes.iter().rev() {
            let mut scope_names = scope.keys().cloned().collect::<Vec<_>>();
            scope_names.sort();
            for name in scope_names {
                if seen.insert(name.clone()) {
                    names.push(name);
                }
            }
        }

        names
    }

    fn undefined_variable_diagnostic(
        &self,
        name: &str,
        span: Span,
        suggestion: String,
    ) -> Diagnostic {
        let mut diagnostic = Diagnostic::new(
            "S0002",
            format!("use of undefined variable `{name}`"),
            self.info.source,
            span,
        )
        .with_note("AX variables are block-scoped and must be declared before use")
        .with_suggestion(suggestion);

        let visible = self.visible_binding_names();
        if !visible.is_empty() {
            diagnostic =
                diagnostic.with_note(format!("visible variables here: {}", visible.join(", ")));
        }

        diagnostic
    }

    fn expect_type_match(&mut self, expected: &Type, actual: &Type, span: Span, message: String) {
        if expected.is_error() || actual.is_error() || expected == actual {
            return;
        }

        self.diagnostics.push(
            Diagnostic::new("S0022", message, self.info.source, span)
                .with_note(format!(
                    "AX does not implicitly convert `{}` to `{}`",
                    actual.describe(),
                    expected.describe()
                ))
                .with_suggestion(type_mismatch_suggestion(expected, actual)),
        );
    }
}
