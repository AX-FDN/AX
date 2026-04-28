use std::collections::{HashMap, HashSet};

use crate::ast::{Expr, StructLiteralField};
use crate::diagnostics::{Diagnostic, DiagnosticKind};

use super::{Type, TypeChecker};

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_struct_literal_expr(
        &mut self,
        expr: &Expr,
        name: &str,
        fields: &[StructLiteralField],
    ) -> Type {
        let current_unit_path = self.current_unit_path().to_string();
        let resolved_type_name =
            self.info
                .resolve_named_type_key(name, &current_unit_path, expr.span, self.diagnostics);
        let struct_info = match resolved_type_name
            .as_ref()
            .and_then(|name| self.info.named_types.get(name))
            .cloned()
        {
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
            None if self
                .info
                .named_type_candidate_exists(name, &current_unit_path) =>
            {
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
        let mut generic_args = HashMap::new();
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
                    if !unify_generic_field_type(&expected_field.ty, &value_type, &mut generic_args)
                    {
                        let expected = substitute_type_params(&expected_field.ty, &generic_args);
                        self.expect_type_match(
                            &expected,
                            &value_type,
                            field.value.span,
                            format!(
                                "field `{}` of `{struct_name}` expects `{}`, found `{}`",
                                field.name,
                                expected.describe(),
                                value_type.describe()
                            ),
                        );
                    }
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
                        .with_suggestion("use an existing field name from the struct declaration"),
                    );
                }
            }
        }

        for field_name in struct_info.fields.keys() {
            if !seen_fields.contains(field_name) {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0026",
                        format!("struct literal `{struct_name}` is missing field `{field_name}`"),
                        self.info.source,
                        expr.span,
                    )
                    .with_suggestion(format!("provide `{field_name}: ...` in the struct literal",)),
                );
            }
        }

        if struct_info.type_params.is_empty() {
            Type::Struct(struct_name)
        } else {
            let mut args = Vec::new();
            for param in &struct_info.type_params {
                let Some(arg) = generic_args.get(param).cloned() else {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0058",
                            format!(
                                "could not infer generic type parameter `{param}` for `{struct_name}`"
                            ),
                            self.info.source,
                            expr.span,
                        )
                        .with_suggestion(format!(
                            "initialize a field that fixes `{param}` or use a non-generic struct"
                        )),
                    );
                    return Type::Error;
                };
                args.push(arg);
            }
            Type::StructInstance {
                name: struct_name,
                args,
            }
        }
    }

    pub(super) fn check_array_literal_expr(&mut self, _expr: &Expr, elements: &[Expr]) -> Type {
        let Some((first, rest)) = elements.split_first() else {
            return Type::EmptyArrayLiteral;
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

    pub(super) fn check_field_expr(&mut self, expr: &Expr, base: &Expr, field: &str) -> Type {
        if let Some(enum_name) = base.qualified_name() {
            let current_unit_path = self.current_unit_path().to_string();
            if let Some(resolved_enum_name) = self.info.resolve_named_type_key(
                &enum_name,
                &current_unit_path,
                expr.span,
                self.diagnostics,
            ) && let Some(enum_info) = self.info.enums.get(&resolved_enum_name)
            {
                if let Some(variant_info) = enum_info.variants.get(field) {
                    if variant_info.payload.is_none() {
                        return Type::Enum(resolved_enum_name);
                    }

                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0053",
                            format!(
                                "enum variant `{}.{field}` requires a payload value",
                                resolved_enum_name
                            ),
                            self.info.source,
                            expr.span,
                        )
                        .with_kind(DiagnosticKind::EnumVariantPayloadShapeMismatch)
                        .with_note(
                            "payload enum variants must be constructed with `EnumName.Variant(value)` in the current AX slice",
                        )
                        .with_suggestion(format!(
                            "call this variant like `{}.{field}(...)` with a value of the declared payload type",
                            resolved_enum_name
                        )),
                    );
                    return Type::Error;
                }

                self.diagnostics.push(
                    Diagnostic::new(
                        "S0029",
                        format!(
                            "enum `{}` does not have a variant `{field}`",
                            resolved_enum_name
                        ),
                        self.info.source,
                        expr.span,
                    )
                    .with_suggestion("use an existing variant name from the enum declaration"),
                );
                return Type::Error;
            }

            if self
                .info
                .named_type_candidate_exists(&enum_name, &current_unit_path)
            {
                return Type::Error;
            }
        }

        let base_type = self.check_expr(base);
        match base_type {
            Type::Struct(struct_name) => {
                let struct_info = self.info.structs.get(&struct_name).cloned();
                self.field_type_for_struct(expr, &struct_name, struct_info, field, &HashMap::new())
            }
            Type::StructInstance { name, args } => {
                let struct_info = self.info.structs.get(&name).cloned();
                let substitutions = struct_info
                    .as_ref()
                    .map(|info| {
                        info.type_params
                            .iter()
                            .cloned()
                            .zip(args)
                            .collect::<HashMap<_, _>>()
                    })
                    .unwrap_or_default();
                self.field_type_for_struct(expr, &name, struct_info, field, &substitutions)
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

    fn field_type_for_struct(
        &mut self,
        expr: &Expr,
        struct_name: &str,
        struct_info: Option<super::super::types::StructInfo>,
        field: &str,
        substitutions: &HashMap<String, Type>,
    ) -> Type {
        match struct_info {
            Some(struct_info) => match struct_info.fields.get(field) {
                Some(field_info) => substitute_type_params(&field_info.ty, substitutions),
                None => {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0020",
                            format!("struct `{struct_name}` does not have a field `{field}`"),
                            self.info.source,
                            expr.span,
                        )
                        .with_suggestion("use an existing field name from the struct declaration"),
                    );
                    Type::Error
                }
            },
            None => Type::Error,
        }
    }

    pub(super) fn check_index_expr(&mut self, expr: &Expr, base: &Expr, index: &Expr) -> Type {
        let base_type = self.check_expr(base);
        let index_type = self.check_expr(index);
        self.expect_type_match_with_kind(
            &Type::I32,
            &index_type,
            index.span,
            format!(
                "array index must be `i32`, found `{}`",
                index_type.describe()
            ),
            DiagnosticKind::ArrayIndexTypeMismatch,
        );

        match base_type {
            Type::Array { element, .. } | Type::Slice { element } => *element,
            Type::Error => Type::Error,
            other => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0033",
                        format!(
                            "index access expects an array or slice value, found `{}`",
                            other.describe()
                        ),
                        self.info.source,
                        expr.span,
                    )
                    .with_suggestion("index into an array or slice value like `values[0]`"),
                );
                Type::Error
            }
        }
    }

    pub(super) fn check_slice_expr(
        &mut self,
        expr: &Expr,
        base: &Expr,
        start: &Expr,
        end: &Expr,
    ) -> Type {
        let base_type = self.check_expr(base);
        let start_type = self.check_expr(start);
        let end_type = self.check_expr(end);

        self.expect_type_match(
            &Type::I32,
            &start_type,
            start.span,
            format!(
                "slice start bound must be `i32`, found `{}`",
                start_type.describe()
            ),
        );
        self.expect_type_match(
            &Type::I32,
            &end_type,
            end.span,
            format!(
                "slice end bound must be `i32`, found `{}`",
                end_type.describe()
            ),
        );

        match base_type {
            Type::Array { element, .. } | Type::Slice { element } => Type::Slice { element },
            Type::Error => Type::Error,
            other => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0034",
                        format!(
                            "slice expression expects an array or slice value, found `{}`",
                            other.describe()
                        ),
                        self.info.source,
                        expr.span,
                    )
                    .with_suggestion("slice an array or slice value like `values[start:end]`"),
                );
                Type::Error
            }
        }
    }
}

fn unify_generic_field_type(
    expected: &Type,
    actual: &Type,
    substitutions: &mut HashMap<String, Type>,
) -> bool {
    match expected {
        Type::TypeParam(name) => match substitutions.get(name) {
            Some(existing) => actual.is_assignable_to(existing),
            None => {
                substitutions.insert(name.clone(), actual.clone());
                true
            }
        },
        Type::Slice {
            element: expected_element,
        } => match actual {
            Type::Slice {
                element: actual_element,
            } => unify_generic_field_type(expected_element, actual_element, substitutions),
            Type::Array {
                element: actual_element,
                ..
            } => unify_generic_field_type(expected_element, actual_element, substitutions),
            _ => expected == actual,
        },
        Type::Array {
            element: expected_element,
            length: expected_length,
        } => match actual {
            Type::Array {
                element: actual_element,
                length: actual_length,
            } if expected_length == actual_length => {
                unify_generic_field_type(expected_element, actual_element, substitutions)
            }
            _ => expected == actual,
        },
        Type::StructInstance {
            name: expected_name,
            args: expected_args,
        } => match actual {
            Type::StructInstance {
                name: actual_name,
                args: actual_args,
            } if expected_name == actual_name && expected_args.len() == actual_args.len() => {
                expected_args
                    .iter()
                    .zip(actual_args)
                    .all(|(expected, actual)| {
                        unify_generic_field_type(expected, actual, substitutions)
                    })
            }
            _ => expected == actual,
        },
        _ => expected == actual,
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
