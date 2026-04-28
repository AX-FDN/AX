use std::collections::HashMap;

use crate::ast::{Expr, ExprKind};
use crate::diagnostics::{Diagnostic, DiagnosticKind};

use super::{Type, TypeChecker};

impl<'a, 'b> TypeChecker<'a, 'b> {
    pub(super) fn check_call_expr(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        arguments: &[Expr],
    ) -> Type {
        let Some(callee_name) = callee.qualified_name() else {
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
            return Type::Error;
        };

        if !callee_name.contains('.')
            && let Some(result) = self.check_builtin_call(expr, &callee_name, arguments)
        {
            return result;
        }

        if let Some(result) =
            self.check_enum_variant_constructor_call(expr, callee.span, &callee_name, arguments)
        {
            return result;
        }

        let current_unit_path = self.current_unit_path().to_string();
        let resolved_name = self.info.resolve_function_key(
            &callee_name,
            &current_unit_path,
            callee.span,
            self.diagnostics,
        );
        let signature = resolved_name
            .as_ref()
            .and_then(|name| self.info.functions.get(name))
            .cloned();

        match signature {
            Some(signature) => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.check_expr(argument))
                    .collect::<Vec<_>>();
                if signature.params.len() != argument_types.len() {
                    self.diagnostics.push(Diagnostic::new(
                        "S0017",
                        format!(
                            "function `{callee_name}` expects {} argument(s), found {}",
                            signature.params.len(),
                            argument_types.len()
                        ),
                        self.info.source,
                        expr.span,
                    ));
                }

                let mut generic_args = HashMap::new();
                for (argument, parameter) in argument_types.iter().zip(signature.params.iter()) {
                    if !unify_generic_call_type(&parameter.ty, argument, &mut generic_args) {
                        let expected = substitute_type_params(&parameter.ty, &generic_args);
                        self.expect_type_match_with_kind(
                            &expected,
                            argument,
                            expr.span,
                            format!(
                                "function `{callee_name}` expects argument `{}` to be `{}`, found `{}`",
                                parameter.name,
                                expected.describe(),
                                argument.describe()
                            ),
                            DiagnosticKind::FunctionArgumentTypeMismatch,
                        );
                    }
                }

                for type_param in &signature.type_params {
                    if !generic_args.contains_key(type_param) {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0058",
                                format!(
                                    "could not infer generic type parameter `{type_param}` for function `{callee_name}`"
                                ),
                                self.info.source,
                                expr.span,
                            )
                            .with_suggestion(
                                "pass an argument whose type fixes the generic parameter",
                            ),
                        );
                        return Type::Error;
                    }
                }

                for bound in &signature.type_param_bounds {
                    let Some(actual_type) = generic_args.get(&bound.type_param) else {
                        continue;
                    };
                    if actual_type.is_error() {
                        continue;
                    }
                    if !self.type_satisfies_required_trait(actual_type, &bound.trait_name) {
                        self.diagnostics.push(
                            Diagnostic::new(
                                "S0059",
                                format!(
                                    "type `{}` does not satisfy trait bound `{}: {}` for function `{callee_name}`",
                                    actual_type.describe(),
                                    bound.type_param,
                                    bound.trait_name
                                ),
                                self.info.source,
                                expr.span,
                            )
                            .with_kind(DiagnosticKind::TraitBoundNotSatisfied)
                            .with_suggestion(format!(
                                "add `impl {} for {} {{ ... }}` or pass a value that implements the trait",
                                bound.trait_name,
                                actual_type.describe()
                            )),
                        );
                        return Type::Error;
                    }
                }

                substitute_type_params(&signature.return_type, &generic_args)
            }
            None if self
                .info
                .function_candidate_exists(&callee_name, &current_unit_path) =>
            {
                Type::Error
            }
            None => {
                if let ExprKind::Field { base, field } = &callee.kind
                    && let Some(result) = self.check_method_call(expr, base, field, arguments)
                {
                    return result;
                }

                if !callee_name.contains('.') && self.lookup(&callee_name).is_some() {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0018",
                            format!("variable `{callee_name}` is not callable"),
                            self.info.source,
                            callee.span,
                        )
                        .with_suggestion("only function names and builtin functions can be called"),
                    );
                    Type::Error
                } else {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0007",
                            format!("call to undefined function `{callee_name}`"),
                            self.info.source,
                            callee.span,
                        )
                        .with_suggestion(format!("declare `{callee_name}` or fix the call target")),
                    );
                    Type::Error
                }
            }
        }
    }

    fn check_method_call(
        &mut self,
        expr: &Expr,
        receiver: &Expr,
        method: &str,
        arguments: &[Expr],
    ) -> Option<Type> {
        let receiver_type = self.check_expr(receiver);
        let argument_types = arguments
            .iter()
            .map(|argument| self.check_expr(argument))
            .collect::<Vec<_>>();

        let signature = self
            .info
            .method_signature(&receiver_type, method)
            .map(|signature| signature.function.clone())
            .or_else(|| {
                if let Type::TypeParam(type_param) = &receiver_type {
                    self.info.trait_bound_method_signature(
                        type_param,
                        method,
                        &self.active_type_param_bounds,
                    )
                } else {
                    None
                }
            });

        let Some(signature) = signature else {
            self.diagnostics.push(
                Diagnostic::new(
                    "S0057",
                    format!(
                        "type `{}` does not have a method `{method}`",
                        receiver_type.describe()
                    ),
                    self.info.source,
                    expr.span,
                )
                .with_suggestion(format!(
                    "add `impl {} {{ fn {method}(self: {}) -> ... {{ ... }} }}` or call an existing function",
                    receiver_type.describe(),
                    receiver_type.describe()
                )),
            );
            return Some(Type::Error);
        };

        let mut method_type_args = HashMap::new();
        if let Some(self_param) = signature.params.first() {
            let expected_self = substitute_self_type(&self_param.ty, &receiver_type);
            let _ = unify_generic_call_type(&expected_self, &receiver_type, &mut method_type_args);
        }

        let expected_extra_args = signature.params.len().saturating_sub(1);
        if expected_extra_args != argument_types.len() {
            self.diagnostics.push(Diagnostic::new(
                "S0017",
                format!(
                    "method `{}` expects {} argument(s) after `self`, found {}",
                    method,
                    expected_extra_args,
                    argument_types.len()
                ),
                self.info.source,
                expr.span,
            ));
        }

        for (argument, parameter) in argument_types.iter().zip(signature.params.iter().skip(1)) {
            let expected = substitute_type_params(
                &substitute_self_type(&parameter.ty, &receiver_type),
                &method_type_args,
            );
            self.expect_type_match_with_kind(
                &expected,
                argument,
                expr.span,
                format!(
                    "method `{method}` expects argument `{}` to be `{}`, found `{}`",
                    parameter.name,
                    expected.describe(),
                    argument.describe()
                ),
                DiagnosticKind::FunctionArgumentTypeMismatch,
            );
        }

        Some(substitute_type_params(
            &substitute_self_type(&signature.return_type, &receiver_type),
            &method_type_args,
        ))
    }

    fn type_satisfies_required_trait(&self, ty: &Type, trait_name: &str) -> bool {
        match ty {
            Type::TypeParam(type_param) => self
                .active_type_param_bounds
                .iter()
                .any(|bound| bound.type_param == *type_param && bound.trait_name == trait_name),
            _ => self.info.type_satisfies_trait_bound(ty, trait_name),
        }
    }

    fn check_enum_variant_constructor_call(
        &mut self,
        expr: &Expr,
        callee_span: crate::source::Span,
        callee_name: &str,
        arguments: &[Expr],
    ) -> Option<Type> {
        let (enum_path, variant) = callee_name.rsplit_once('.')?;
        let current_unit_path = self.current_unit_path().to_string();
        let local_enum_candidate = self
            .info
            .unit_context(&current_unit_path)
            .and_then(|unit| unit.module_path.as_ref())
            .map(|module_path| format!("{module_path}.{enum_path}"));
        let has_local_enum_candidate = local_enum_candidate
            .as_ref()
            .map(|candidate| self.info.named_types.contains_key(candidate))
            .unwrap_or(false);
        if !self.info.named_types.contains_key(enum_path) && !has_local_enum_candidate {
            return None;
        }
        let Some(resolved_enum_name) = self.info.resolve_named_type_key(
            enum_path,
            &current_unit_path,
            expr.span,
            self.diagnostics,
        ) else {
            return Some(Type::Error);
        };
        let Some(enum_info) = self.info.enums.get(&resolved_enum_name).cloned() else {
            return None;
        };
        let Some(variant_info) = enum_info.variants.get(variant).cloned() else {
            self.diagnostics.push(
                Diagnostic::new(
                    "S0029",
                    format!("unknown enum variant `{variant}` for enum `{resolved_enum_name}`"),
                    self.info.source,
                    callee_span,
                )
                .with_suggestion("use one of the declared enum variants"),
            );
            return Some(Type::Error);
        };

        let argument_types = arguments
            .iter()
            .map(|argument| self.check_expr(argument))
            .collect::<Vec<_>>();

        let enum_result_type = generic_enum_result_type(
            &resolved_enum_name,
            self.info
                .enums
                .get(&resolved_enum_name)
                .map(|info| info.type_params.as_slice())
                .unwrap_or(&[]),
            variant_info.payload.as_ref(),
            argument_types.first(),
        );

        match variant_info.payload {
            Some(payload_type) => {
                if argument_types.len() != 1 {
                    self.diagnostics.push(
                        Diagnostic::new(
                            "S0053",
                            format!(
                                "enum variant `{callee_name}` expects exactly 1 payload argument, found {}",
                                argument_types.len()
                            ),
                            self.info.source,
                            expr.span,
                        )
                        .with_kind(DiagnosticKind::EnumVariantPayloadShapeMismatch)
                        .with_note(
                            "payload enum variants are constructed as `EnumName.Variant(value)` in the current AX slice",
                        )
                        .with_suggestion("pass exactly one payload value that matches the declared payload type"),
                    );
                    return Some(Type::Error);
                }

                let mut payload_substitutions = HashMap::new();
                if !unify_generic_call_type(
                    &payload_type,
                    &argument_types[0],
                    &mut payload_substitutions,
                ) {
                    let expected_payload =
                        substitute_type_params(&payload_type, &payload_substitutions);
                    self.expect_type_match_with_kind(
                        &expected_payload,
                        &argument_types[0],
                        arguments[0].span,
                        format!(
                            "enum variant `{callee_name}` expects payload type `{}`, found `{}`",
                            expected_payload.describe(),
                            argument_types[0].describe()
                        ),
                        DiagnosticKind::EnumVariantPayloadTypeMismatch,
                    );
                }

                Some(enum_result_type)
            }
            None => {
                self.diagnostics.push(
                    Diagnostic::new(
                        "S0053",
                        format!("enum variant `{callee_name}` does not accept payload arguments"),
                        self.info.source,
                        expr.span,
                    )
                    .with_kind(DiagnosticKind::EnumVariantPayloadShapeMismatch)
                    .with_note(
                        "unit enum variants are already complete values, so they are written without `(...)`",
                    )
                    .with_suggestion(format!("drop the call syntax and use `{callee_name}` directly")),
                );
                Some(Type::Error)
            }
        }
    }
}

fn generic_enum_result_type(
    enum_name: &str,
    type_params: &[String],
    payload_type: Option<&Type>,
    argument_type: Option<&Type>,
) -> Type {
    if type_params.is_empty() {
        return Type::Enum(enum_name.to_string());
    }

    let mut substitutions = HashMap::new();
    if let (Some(payload_type), Some(argument_type)) = (payload_type, argument_type) {
        let _ = unify_generic_call_type(payload_type, argument_type, &mut substitutions);
    }

    Type::EnumInstance {
        name: enum_name.to_string(),
        args: type_params
            .iter()
            .map(|param| {
                substitutions
                    .get(param)
                    .cloned()
                    .unwrap_or_else(|| Type::TypeParam(param.clone()))
            })
            .collect(),
    }
}

fn unify_generic_call_type(
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
            } => unify_generic_call_type(expected_element, actual_element, substitutions),
            Type::Array {
                element: actual_element,
                ..
            } => unify_generic_call_type(expected_element, actual_element, substitutions),
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
                unify_generic_call_type(expected_element, actual_element, substitutions)
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
                        unify_generic_call_type(expected, actual, substitutions)
                    })
            }
            _ => expected == actual,
        },
        Type::EnumInstance {
            name: expected_name,
            args: expected_args,
        } => match actual {
            Type::EnumInstance {
                name: actual_name,
                args: actual_args,
            } if expected_name == actual_name && expected_args.len() == actual_args.len() => {
                expected_args
                    .iter()
                    .zip(actual_args)
                    .all(|(expected, actual)| {
                        unify_generic_call_type(expected, actual, substitutions)
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
        Type::EnumInstance { name, args } => Type::EnumInstance {
            name: name.clone(),
            args: args
                .iter()
                .map(|arg| substitute_type_params(arg, substitutions))
                .collect(),
        },
        _ => ty.clone(),
    }
}

fn substitute_self_type(ty: &Type, self_type: &Type) -> Type {
    match ty {
        Type::TypeParam(name) if name == "Self" => self_type.clone(),
        Type::Slice { element } => Type::Slice {
            element: Box::new(substitute_self_type(element, self_type)),
        },
        Type::Array { element, length } => Type::Array {
            element: Box::new(substitute_self_type(element, self_type)),
            length: *length,
        },
        Type::StructInstance { name, args } => Type::StructInstance {
            name: name.clone(),
            args: args
                .iter()
                .map(|arg| substitute_self_type(arg, self_type))
                .collect(),
        },
        Type::EnumInstance { name, args } => Type::EnumInstance {
            name: name.clone(),
            args: args
                .iter()
                .map(|arg| substitute_self_type(arg, self_type))
                .collect(),
        },
        _ => ty.clone(),
    }
}
