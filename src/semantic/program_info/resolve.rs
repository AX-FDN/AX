use std::collections::HashMap;

use crate::ast::TypeRef;
use crate::diagnostics::{Diagnostic, DiagnosticKind};

use super::helpers::{method_lookup_type_name, substitute_type_params, type_matches_impl_target};
use super::*;
impl<'a> ProgramInfo<'a> {
    pub(in crate::semantic) fn resolve_type_ref(
        &self,
        ty: &TypeRef,
        current_unit_path: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Type {
        self.resolve_type_ref_with_params(ty, current_unit_path, &[], diagnostics)
    }

    pub(in crate::semantic) fn resolve_type_ref_with_params(
        &self,
        ty: &TypeRef,
        current_unit_path: &str,
        generic_params: &[String],
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Type {
        match (&ty.name, &ty.type_args[..], &ty.element, ty.length) {
            (Some(name), [], None, None) if generic_params.iter().any(|param| param == name) => {
                Type::TypeParam(name.clone())
            }
            (Some(name), [], None, None) => {
                if let Some(alias) =
                    self.resolve_type_alias(name, current_unit_path, ty.span, diagnostics)
                {
                    if !alias.type_params.is_empty() {
                        diagnostics.push(
                            Diagnostic::new(
                                "S0058",
                                format!("generic type alias `{name}` requires type arguments"),
                                self.source,
                                ty.span,
                            )
                            .with_suggestion(format!(
                                "write `{}` with type arguments like `{}<i32>`",
                                name, name
                            )),
                        );
                        return Type::Error;
                    }
                    return alias.target;
                }

                match self.resolve_named_type_key(name, current_unit_path, ty.span, diagnostics) {
                    Some(found) => {
                        let resolved = self
                            .named_types
                            .get(&found)
                            .cloned()
                            .expect("resolved type should exist");
                        if let Some((kind, generic_name, count)) =
                            self.generic_type_arity(&resolved)
                            && count > 0
                        {
                            diagnostics.push(
                                Diagnostic::new(
                                    "S0058",
                                    format!(
                                        "generic {kind} `{generic_name}` requires type arguments"
                                    ),
                                    self.source,
                                    ty.span,
                                )
                                .with_suggestion(format!(
                                    "write `{}` with type arguments like `{}<i32>`",
                                    generic_name, generic_name
                                )),
                            );
                            Type::Error
                        } else {
                            resolved
                        }
                    }
                    None if self.named_type_candidate_exists(name, current_unit_path) => {
                        Type::Error
                    }
                    None => {
                        diagnostics.push(
                            Diagnostic::new(
                                "S0006",
                                format!("unknown type `{}`", name),
                                self.source,
                                ty.span,
                            )
                            .with_suggestion(
                                "use a builtin type, `[Type]`, `[Type; N]`, a same-module type, or an imported fully qualified type name",
                            ),
                        );
                        Type::Error
                    }
                }
            }
            (Some(name), args, None, None) => {
                if let Some(alias) =
                    self.resolve_type_alias(name, current_unit_path, ty.span, diagnostics)
                {
                    let resolved_args = args
                        .iter()
                        .map(|arg| {
                            self.resolve_type_ref_with_params(
                                arg,
                                current_unit_path,
                                generic_params,
                                diagnostics,
                            )
                        })
                        .collect::<Vec<_>>();

                    if alias.type_params.len() != resolved_args.len() {
                        diagnostics.push(
                            Diagnostic::new(
                                "S0058",
                                format!(
                                    "generic type alias `{name}` expects {} type argument(s), found {}",
                                    alias.type_params.len(),
                                    resolved_args.len()
                                ),
                                self.source,
                                ty.span,
                            )
                            .with_suggestion(format!(
                                "write `{}` with exactly {} type argument(s)",
                                name,
                                alias.type_params.len()
                            )),
                        );
                        return Type::Error;
                    }

                    let substitutions = alias
                        .type_params
                        .iter()
                        .cloned()
                        .zip(resolved_args)
                        .collect::<HashMap<_, _>>();
                    return substitute_type_params(&alias.target, &substitutions);
                }

                let Some(found) =
                    self.resolve_named_type_key(name, current_unit_path, ty.span, diagnostics)
                else {
                    if !self.named_type_candidate_exists(name, current_unit_path) {
                        diagnostics.push(
                            Diagnostic::new(
                                "S0006",
                                format!("unknown type `{name}`"),
                                self.source,
                                ty.span,
                            )
                            .with_suggestion(
                                "use a declared generic type like `Box<i32>` or `Result<i32, string>`",
                            ),
                        );
                    }
                    return Type::Error;
                };

                let Some(named_type) = self.named_types.get(&found).cloned() else {
                    return Type::Error;
                };
                let resolved_args = args
                    .iter()
                    .map(|arg| {
                        self.resolve_type_ref_with_params(
                            arg,
                            current_unit_path,
                            generic_params,
                            diagnostics,
                        )
                    })
                    .collect::<Vec<_>>();

                match named_type {
                    Type::Struct(struct_name) => {
                        let Some(struct_info) = self.structs.get(&struct_name) else {
                            return Type::Error;
                        };
                        if struct_info.type_params.len() != resolved_args.len() {
                            diagnostics.push(
                                Diagnostic::new(
                                    "S0058",
                                    format!(
                                        "generic struct `{struct_name}` expects {} type argument(s), found {}",
                                        struct_info.type_params.len(),
                                        resolved_args.len()
                                    ),
                                    self.source,
                                    ty.span,
                                )
                                .with_suggestion(format!(
                                    "write `{}` with exactly {} type argument(s)",
                                    struct_name,
                                    struct_info.type_params.len()
                                )),
                            );
                            Type::Error
                        } else {
                            Type::StructInstance {
                                name: struct_name,
                                args: resolved_args,
                            }
                        }
                    }
                    Type::Enum(enum_name) => {
                        let Some(enum_info) = self.enums.get(&enum_name) else {
                            return Type::Error;
                        };
                        if enum_info.type_params.len() != resolved_args.len() {
                            diagnostics.push(
                                Diagnostic::new(
                                    "S0058",
                                    format!(
                                        "generic enum `{enum_name}` expects {} type argument(s), found {}",
                                        enum_info.type_params.len(),
                                        resolved_args.len()
                                    ),
                                    self.source,
                                    ty.span,
                                )
                                .with_suggestion(format!(
                                    "write `{}` with exactly {} type argument(s)",
                                    enum_name,
                                    enum_info.type_params.len()
                                )),
                            );
                            Type::Error
                        } else {
                            Type::EnumInstance {
                                name: enum_name,
                                args: resolved_args,
                            }
                        }
                    }
                    other => {
                        diagnostics.push(
                            Diagnostic::new(
                                "S0058",
                                format!(
                                    "`{found}` cannot take generic type arguments because it is `{}`",
                                    other.describe()
                                ),
                                self.source,
                                ty.span,
                            )
                            .with_suggestion("only generic struct or enum types accept `<...>`"),
                        );
                        Type::Error
                    }
                }
            }
            (None, [], Some(element), None) => Type::Slice {
                element: Box::new(self.resolve_type_ref_with_params(
                    element,
                    current_unit_path,
                    generic_params,
                    diagnostics,
                )),
            },
            (None, [], Some(element), Some(length)) => Type::Array {
                element: Box::new(self.resolve_type_ref_with_params(
                    element,
                    current_unit_path,
                    generic_params,
                    diagnostics,
                )),
                length,
            },
            _ => {
                diagnostics.push(
                    Diagnostic::new("S0006", "invalid type syntax", self.source, ty.span)
                        .with_suggestion(
                            "use a named type, a slice type like `[i32]`, or an array type like `[i32; 3]`",
                        ),
                );
                Type::Error
            }
        }
    }

    pub(in crate::semantic) fn resolve_named_type_key(
        &self,
        name: &str,
        current_unit_path: &str,
        span: crate::source::Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<String> {
        self.resolve_named_key(
            name,
            current_unit_path,
            span,
            diagnostics,
            &self.named_types,
            "type",
        )
    }

    fn resolve_type_alias(
        &self,
        name: &str,
        current_unit_path: &str,
        span: crate::source::Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<TypeAliasInfo> {
        self.resolve_named_key(
            name,
            current_unit_path,
            span,
            diagnostics,
            &self.type_aliases,
            "type alias",
        )
        .and_then(|key| self.type_aliases.get(&key).cloned())
    }

    pub(in crate::semantic) fn named_type_candidate_exists(
        &self,
        name: &str,
        current_unit_path: &str,
    ) -> bool {
        self.named_key_candidate_exists(name, current_unit_path, &self.named_types)
            || self.named_key_candidate_exists(name, current_unit_path, &self.type_aliases)
    }

    pub(in crate::semantic) fn resolve_function_key(
        &self,
        name: &str,
        current_unit_path: &str,
        span: crate::source::Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<String> {
        self.resolve_named_key(
            name,
            current_unit_path,
            span,
            diagnostics,
            &self.functions,
            "function",
        )
    }

    pub(in crate::semantic) fn function_candidate_exists(
        &self,
        name: &str,
        current_unit_path: &str,
    ) -> bool {
        self.named_key_candidate_exists(name, current_unit_path, &self.functions)
    }

    pub(in crate::semantic) fn resolve_constant_key(
        &self,
        name: &str,
        current_unit_path: &str,
        span: crate::source::Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<String> {
        self.resolve_named_key(
            name,
            current_unit_path,
            span,
            diagnostics,
            &self.constants,
            "constant",
        )
    }

    pub(in crate::semantic) fn constant_candidate_exists(
        &self,
        name: &str,
        current_unit_path: &str,
    ) -> bool {
        self.named_key_candidate_exists(name, current_unit_path, &self.constants)
    }

    pub(in crate::semantic) fn function_signature_for_definition(
        &self,
        name: &str,
        current_unit_path: &str,
    ) -> Option<&FunctionSignature> {
        if let Some(signature) = self.functions.get(name) {
            return Some(signature);
        }

        if self.module_mode
            && !name.contains('.')
            && let Some(unit) = self.unit_context(current_unit_path)
            && let Some(module_path) = &unit.module_path
        {
            return self.functions.get(&format!("{module_path}.{name}"));
        }

        None
    }

    fn generic_type_arity(&self, ty: &Type) -> Option<(&'static str, String, usize)> {
        match ty {
            Type::Struct(name) => self
                .structs
                .get(name)
                .map(|info| ("struct", name.clone(), info.type_params.len())),
            Type::Enum(name) => self
                .enums
                .get(name)
                .map(|info| ("enum", name.clone(), info.type_params.len())),
            _ => None,
        }
    }

    pub(super) fn resolve_trait_ref(
        &self,
        trait_ref: &TypeRef,
        current_unit_path: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<String> {
        let Some(name) = trait_ref.direct_name() else {
            diagnostics.push(
                Diagnostic::new(
                    "S0059",
                    "trait impl must name a trait before `for`",
                    self.source,
                    trait_ref.span,
                )
                .with_kind(DiagnosticKind::TraitReferenceMustResolve)
                .with_suggestion("write `impl TraitName for TypeName { ... }`"),
            );
            return None;
        };
        self.resolve_named_key(
            name,
            current_unit_path,
            trait_ref.span,
            diagnostics,
            &self.traits,
            "trait",
        )
        .or_else(|| {
            if !self.named_key_candidate_exists(name, current_unit_path, &self.traits) {
                diagnostics.push(
                    Diagnostic::new(
                        "S0059",
                        format!("unknown trait `{name}`"),
                        self.source,
                        trait_ref.span,
                    )
                    .with_kind(DiagnosticKind::TraitReferenceMustResolve)
                    .with_suggestion("declare the trait before implementing it"),
                );
            }
            None
        })
    }

    pub(super) fn resolve_type_param_bounds(
        &self,
        type_params: &[String],
        bounds: &[crate::ast::TypeParamBound],
        current_unit_path: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Vec<TypeParamBoundInfo> {
        let mut resolved = Vec::new();
        for bound in bounds {
            if !type_params.iter().any(|param| param == &bound.type_param) {
                diagnostics.push(
                    Diagnostic::new(
                        "S0058",
                        format!(
                            "trait bound references unknown generic type parameter `{}`",
                            bound.type_param
                        ),
                        self.source,
                        bound.span,
                    )
                    .with_suggestion("bind only type parameters declared in this function"),
                );
                continue;
            }

            let Some(trait_name) =
                self.resolve_trait_ref(&bound.trait_ref, current_unit_path, diagnostics)
            else {
                continue;
            };
            resolved.push(TypeParamBoundInfo {
                type_param: bound.type_param.clone(),
                trait_name,
            });
        }
        resolved
    }

    pub(in crate::semantic) fn method_signature(
        &self,
        receiver_type: &Type,
        method: &str,
    ) -> Option<&MethodSignature> {
        self.methods
            .get(&format!("{}.{}", receiver_type.describe(), method))
            .or_else(|| {
                self.methods.get(&format!(
                    "{}.{}",
                    method_lookup_type_name(receiver_type),
                    method
                ))
            })
    }

    pub(in crate::semantic) fn type_satisfies_trait_bound(
        &self,
        ty: &Type,
        trait_name: &str,
    ) -> bool {
        self.trait_impls
            .iter()
            .any(|(impl_type, implemented_trait)| {
                implemented_trait == trait_name && type_matches_impl_target(ty, impl_type)
            })
    }

    pub(in crate::semantic) fn trait_bound_method_signature(
        &self,
        type_param: &str,
        method: &str,
        active_bounds: &[TypeParamBoundInfo],
    ) -> Option<FunctionSignature> {
        active_bounds
            .iter()
            .filter(|bound| bound.type_param == type_param)
            .find_map(|bound| {
                self.traits
                    .get(&bound.trait_name)
                    .and_then(|trait_info| trait_info.methods.get(method))
                    .cloned()
            })
    }

    fn resolve_named_key<T>(
        &self,
        name: &str,
        current_unit_path: &str,
        span: crate::source::Span,
        diagnostics: &mut Vec<Diagnostic>,
        table: &HashMap<String, T>,
        kind: &str,
    ) -> Option<String> {
        if table.contains_key(name) {
            if self.module_access_allowed(name, current_unit_path, span, diagnostics, kind) {
                return Some(name.to_string());
            }
            return None;
        }

        if self.module_mode && !name.contains('.') {
            if let Some(unit) = self.unit_context(current_unit_path)
                && let Some(module_path) = &unit.module_path
            {
                let local_name = format!("{module_path}.{name}");
                if table.contains_key(&local_name) {
                    return Some(local_name);
                }
            }
        }

        None
    }

    fn named_key_candidate_exists<T>(
        &self,
        name: &str,
        current_unit_path: &str,
        table: &HashMap<String, T>,
    ) -> bool {
        if table.contains_key(name) {
            return true;
        }

        if self.module_mode
            && !name.contains('.')
            && let Some(unit) = self.unit_context(current_unit_path)
            && let Some(module_path) = &unit.module_path
        {
            let local_name = format!("{module_path}.{name}");
            return table.contains_key(&local_name);
        }

        false
    }

    fn module_access_allowed(
        &self,
        qualified_name: &str,
        current_unit_path: &str,
        span: crate::source::Span,
        diagnostics: &mut Vec<Diagnostic>,
        kind: &str,
    ) -> bool {
        if !self.module_mode {
            return true;
        }

        let Some((module_path, _)) = qualified_name.rsplit_once('.') else {
            return true;
        };

        let Some(unit) = self.unit_context(current_unit_path) else {
            return true;
        };

        if unit.module_path.as_deref() == Some(module_path) || unit.imports.contains(module_path) {
            return true;
        }

        if self.modules.contains(module_path) {
            diagnostics.push(
                Diagnostic::new(
                    "S0043",
                    format!(
                        "{} `{qualified_name}` requires `import {module_path};`",
                        kind
                    ),
                    self.source,
                    span,
                )
                .with_kind(DiagnosticKind::CrossModuleReferenceMissingImport)
                .with_note(
                    "AX minimal module mode requires explicit imports for cross-module references",
                )
                .with_suggestion(format!(
                    "add `import {module_path};` near the top of `{current_unit_path}`"
                )),
            );
            return false;
        }

        true
    }

    pub(in crate::semantic) fn unit_context(&self, path: &str) -> Option<&UnitContext> {
        self.units.get(path)
    }
}
