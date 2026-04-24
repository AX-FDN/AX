use crate::ast::{Expr, ExprKind};
use crate::diagnostics::Diagnostic;
use crate::source::Span;

use super::{Type, TypeChecker};

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_assignment_target(
        &mut self,
        target: &Expr,
        value_type: &Type,
        value_span: Span,
    ) {
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
                    Type::Slice { .. } => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0035",
                                format!(
                                    "cannot assign through slice variable `{base_name}` because slices are read-only",
                                ),
                                self.info.source,
                                target_span,
                            )
                            .with_suggestion(
                                "assign through the original mutable array instead of a slice view",
                            ),
                        );
                    }
                    Type::Error => {}
                    other => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0033",
                                format!(
                                    "indexed assignment requires an array value, found `{}`",
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
}
