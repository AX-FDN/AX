use crate::ast::{BinaryOp, Expr, ExprKind, UnaryOp};
use crate::diagnostics::Diagnostic;

use super::{binary_op_name, type_name_as_value_diagnostic, Type, TypeChecker};

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_expr(&mut self, expr: &Expr) -> Type {
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
            ExprKind::Call { callee, arguments } => {
                self.check_call_expr(expr, callee, arguments)
            }
            ExprKind::StructLiteral { name, fields } => {
                self.check_struct_literal_expr(expr, name, fields)
            }
            ExprKind::ArrayLiteral { elements } => self.check_array_literal_expr(expr, elements),
            ExprKind::Field { base, field } => self.check_field_expr(expr, base, field),
            ExprKind::Index { base, index } => self.check_index_expr(expr, base, index),
            ExprKind::Slice { base, start, end } => self.check_slice_expr(expr, base, start, end),
            ExprKind::Error => Type::Error,
        }
    }
}
