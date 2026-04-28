use std::collections::HashMap;

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
        let Some(place) = self.resolve_assignment_place(target) else {
            return;
        };

        if !place.root_mutable {
            self.report_immutable_assignment_target(target, &place.root_name, place.root_start);
            return;
        }

        if place.through_slice {
            self.diagnostics.push(
                Diagnostic::new(
                    "S0035",
                    format!(
                        "cannot assign through slice variable `{}` because slices are read-only",
                        place.root_name
                    ),
                    self.info.source,
                    target.span,
                )
                .with_suggestion(
                    "assign through the original mutable array instead of a slice view",
                ),
            );
            return;
        }

        self.expect_type_match(
            &place.ty,
            value_type,
            value_span,
            format!(
                "cannot assign `{}` to target of type `{}`",
                value_type.describe(),
                place.ty.describe()
            ),
        );
    }

    fn resolve_assignment_place(&mut self, target: &Expr) -> Option<ResolvedAssignmentPlace> {
        match &target.kind {
            ExprKind::Name { value } => {
                let binding = match self.lookup(value) {
                    Some(binding) => binding,
                    None => {
                        self.diagnostics.push(self.undefined_variable_diagnostic(
                            value,
                            target.span,
                            format!("declare `{value}` before assigning to it"),
                        ));
                        return None;
                    }
                };

                Some(ResolvedAssignmentPlace {
                    ty: binding.ty.clone(),
                    root_name: value.clone(),
                    root_mutable: binding.mutable,
                    root_start: binding.start,
                    through_slice: false,
                })
            }
            ExprKind::Field { base, field } => {
                let base_place = self.resolve_assignment_place(base)?;
                match &base_place.ty {
                    Type::Struct(struct_name) => {
                        let struct_info = self.info.structs.get(struct_name).cloned();
                        match struct_info.and_then(|info| info.fields.get(field).cloned()) {
                            Some(field_info) => Some(ResolvedAssignmentPlace {
                                ty: field_info.ty,
                                ..base_place
                            }),
                            None => {
                                self.diagnostics.push(
                                    Diagnostic::new(
                                        "S0020",
                                        format!(
                                            "struct `{struct_name}` does not have a field `{field}`"
                                        ),
                                        self.info.source,
                                        target.span,
                                    )
                                    .with_suggestion(
                                        "use an existing field name from the struct declaration",
                                    ),
                                );
                                None
                            }
                        }
                    }
                    Type::StructInstance { name, args } => {
                        let struct_info = self.info.structs.get(name).cloned();
                        let substitutions = struct_info
                            .as_ref()
                            .map(|info| {
                                info.type_params
                                    .iter()
                                    .cloned()
                                    .zip(args.iter().cloned())
                                    .collect::<HashMap<_, _>>()
                            })
                            .unwrap_or_default();
                        match struct_info.and_then(|info| info.fields.get(field).cloned()) {
                            Some(field_info) => Some(ResolvedAssignmentPlace {
                                ty: substitute_type_params(&field_info.ty, &substitutions),
                                ..base_place
                            }),
                            None => {
                                self.diagnostics.push(
                                    Diagnostic::new(
                                        "S0020",
                                        format!("struct `{name}` does not have a field `{field}`"),
                                        self.info.source,
                                        target.span,
                                    )
                                    .with_suggestion(
                                        "use an existing field name from the struct declaration",
                                    ),
                                );
                                None
                            }
                        }
                    }
                    Type::Error => None,
                    other => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0021",
                                format!(
                                    "field access expects a struct value, found `{}`",
                                    other.describe()
                                ),
                                self.info.source,
                                target.span,
                            )
                            .with_suggestion(
                                "use `.field` only after a struct value or a struct element selected from an array",
                            ),
                        );
                        None
                    }
                }
            }
            ExprKind::Index { base, index } => {
                let base_place = self.resolve_assignment_place(base)?;
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

                match &base_place.ty {
                    Type::Array { element, .. } => Some(ResolvedAssignmentPlace {
                        ty: element.as_ref().clone(),
                        ..base_place
                    }),
                    Type::Slice { element } => Some(ResolvedAssignmentPlace {
                        ty: element.as_ref().clone(),
                        through_slice: true,
                        ..base_place
                    }),
                    Type::Error => None,
                    other => {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0033",
                                format!(
                                    "indexed assignment requires an array or slice value, found `{}`",
                                    other.describe()
                                ),
                                self.info.source,
                                target.span,
                            )
                            .with_suggestion(
                                "assign through an array element like `values[index] = expr;` or a field selected from one",
                            ),
                        );
                        None
                    }
                }
            }
            _ => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0008",
                        "assignment target must be a mutable variable, field path, or array element path",
                        self.info.source,
                        target.span,
                    )
                    .with_suggestion(
                        "assign to `value = expr;`, `point.x = expr;`, `values[index] = expr;`, or `values[index].field = expr;`",
                    ),
                );
                self.check_expr(target);
                None
            }
        }
    }

    fn report_immutable_assignment_target(
        &mut self,
        target: &Expr,
        root_name: &str,
        root_start: usize,
    ) {
        let (line, column) = self.info.source.line_col(root_start);
        match &target.kind {
            ExprKind::Name { .. } => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0003",
                        format!("cannot assign to immutable variable `{root_name}`"),
                        self.info.source,
                        target.span,
                    )
                    .with_note(format!(
                        "`{root_name}` was declared immutable at {line}:{column}"
                    ))
                    .with_note("AX fixes local mutability at the declaration site; later assignments require `let mut`")
                    .with_suggestion(format!("declare `{root_name}` with `let mut`")),
                );
            }
            ExprKind::Index { .. } => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0003",
                        format!("cannot assign through immutable variable `{root_name}`"),
                        self.info.source,
                        target.span,
                    )
                    .with_note(format!(
                        "`{root_name}` was declared immutable at {line}:{column}"
                    ))
                    .with_note("AX fixes local mutability at the declaration site; later assignments require `let mut`")
                    .with_suggestion(format!("declare `{root_name}` with `let mut`")),
                );
            }
            ExprKind::Field { field, .. } => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0030",
                        format!(
                            "cannot assign to field `{field}` on immutable variable `{root_name}`"
                        ),
                        self.info.source,
                        target.span,
                    )
                    .with_note(format!(
                        "`{root_name}` was declared immutable at {line}:{column}"
                    ))
                    .with_suggestion(format!(
                        "declare `{root_name}` with `let mut` before assigning through this field path"
                    )),
                );
            }
            _ => {}
        }
    }
}

fn substitute_type_params(ty: &Type, substitutions: &HashMap<String, Type>) -> Type {
    match ty {
        Type::TypeParam(name) => substitutions
            .get(name)
            .cloned()
            .unwrap_or_else(|| ty.clone()),
        Type::Slice { element } => Type::Slice {
            element: Box::new(substitute_type_params(element, substitutions)),
        },
        Type::Array { element, length } => Type::Array {
            element: Box::new(substitute_type_params(element, substitutions)),
            length: *length,
        },
        Type::StructInstance { name, args } => Type::StructInstance {
            name: name.clone(),
            args: args
                .iter()
                .map(|arg| substitute_type_params(arg, substitutions))
                .collect(),
        },
        _ => ty.clone(),
    }
}

#[derive(Clone)]
struct ResolvedAssignmentPlace {
    ty: Type,
    root_name: String,
    root_mutable: bool,
    root_start: usize,
    through_slice: bool,
}
