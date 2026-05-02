use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::ast::Program;
use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::project::Project;
use crate::source::SourceFile;

use super::*;
pub(super) fn collect_unit_contexts(
    source: &SourceFile,
    program: &Program,
    project: Option<&Project>,
    diagnostics: &mut Vec<Diagnostic>,
) -> HashMap<String, UnitContext> {
    let mut units = HashMap::new();

    for unit in &program.source_units {
        let imports = unit
            .imports
            .iter()
            .map(|import| import.path.clone())
            .collect::<HashSet<_>>();

        units.insert(
            unit.path.clone(),
            UnitContext {
                module_path: unit.module.as_ref().map(|module| module.path.clone()),
                imports,
                is_entry: unit.is_entry,
            },
        );

        if let Some(project) = project
            && !unit.is_entry
            && project
                .expected_module_path(Path::new(&unit.path))
                .is_none()
        {
            diagnostics.push(
                Diagnostic::new(
                    "S0038",
                    format!(
                        "support source `{}` is not declared in `[package].sources` for this project",
                        unit.path
                    ),
                    source,
                    unit.span,
                )
                .with_kind(DiagnosticKind::SupportSourceMissingManifestListing)
                .with_suggestion("add the file or its parent directory to `[package].sources`"),
            );
        }
    }

    units
}

pub(super) fn method_lookup_type_name(ty: &Type) -> String {
    match ty {
        Type::StructInstance { name, .. } | Type::EnumInstance { name, .. } => name.clone(),
        other => other.describe(),
    }
}

pub(super) fn type_matches_impl_target(actual: &Type, impl_target: &Type) -> bool {
    match impl_target {
        Type::TypeParam(_) => true,
        Type::StructInstance {
            name: impl_name,
            args: impl_args,
        } => match actual {
            Type::StructInstance {
                name: actual_name,
                args: actual_args,
            } if actual_name == impl_name && actual_args.len() == impl_args.len() => actual_args
                .iter()
                .zip(impl_args)
                .all(|(actual_arg, impl_arg)| type_matches_impl_target(actual_arg, impl_arg)),
            _ => false,
        },
        Type::EnumInstance {
            name: impl_name,
            args: impl_args,
        } => match actual {
            Type::EnumInstance {
                name: actual_name,
                args: actual_args,
            } if actual_name == impl_name && actual_args.len() == impl_args.len() => actual_args
                .iter()
                .zip(impl_args)
                .all(|(actual_arg, impl_arg)| type_matches_impl_target(actual_arg, impl_arg)),
            _ => false,
        },
        _ => actual == impl_target,
    }
}

pub(super) fn canonical_item_name(
    module_mode: bool,
    unit: &UnitContext,
    item_name: &str,
) -> String {
    if module_mode && let Some(module_path) = &unit.module_path {
        return format!("{module_path}.{item_name}");
    }
    item_name.to_string()
}

pub(super) fn check_generic_type_params(
    source: &SourceFile,
    owner: &str,
    type_params: &[String],
    span: crate::source::Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut names = HashSet::new();
    for type_param in type_params {
        if !names.insert(type_param.clone()) {
            diagnostics.push(
                Diagnostic::new(
                    "S0058",
                    format!("duplicate generic type parameter `{type_param}` in `{owner}`"),
                    source,
                    span,
                )
                .with_suggestion("keep each generic type parameter name unique"),
            );
        }
    }
}

pub(super) fn check_trait_impl(
    source: &SourceFile,
    trait_name: &str,
    self_type: &Type,
    trait_info: &TraitInfo,
    impl_signatures: &HashMap<String, FunctionSignature>,
    span: crate::source::Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (method_name, trait_signature) in &trait_info.methods {
        let Some(impl_signature) = impl_signatures.get(method_name) else {
            diagnostics.push(
                Diagnostic::new(
                    "S0059",
                    format!(
                        "impl for trait `{trait_name}` is missing method `{method_name}`"
                    ),
                    source,
                    span,
                )
                .with_suggestion(format!(
                    "add `fn {method_name}(...) -> ...` with the signature required by `{trait_name}`"
                )),
            );
            continue;
        };

        let expected_params = trait_signature
            .params
            .iter()
            .map(|param| substitute_self_type(&param.ty, self_type))
            .collect::<Vec<_>>();
        let expected_return = substitute_self_type(&trait_signature.return_type, self_type);

        if expected_params.len() != impl_signature.params.len() {
            diagnostics.push(
                Diagnostic::new(
                    "S0059",
                    format!(
                        "trait method `{method_name}` expects {} parameter(s), impl provides {}",
                        expected_params.len(),
                        impl_signature.params.len()
                    ),
                    source,
                    span,
                )
                .with_suggestion("make the impl method parameter list match the trait method"),
            );
            continue;
        }

        for (expected, actual) in expected_params.iter().zip(&impl_signature.params) {
            if expected != &actual.ty {
                diagnostics.push(
                    Diagnostic::new(
                        "S0059",
                        format!(
                            "trait method `{method_name}` parameter `{}` must be `{}`, found `{}`",
                            actual.name,
                            expected.describe(),
                            actual.ty.describe()
                        ),
                        source,
                        span,
                    )
                    .with_suggestion("make the impl method signature match the trait method"),
                );
            }
        }

        if expected_return != impl_signature.return_type {
            diagnostics.push(
                Diagnostic::new(
                    "S0059",
                    format!(
                        "trait method `{method_name}` must return `{}`, found `{}`",
                        expected_return.describe(),
                        impl_signature.return_type.describe()
                    ),
                    source,
                    span,
                )
                .with_suggestion("make the impl method return type match the trait method"),
            );
        }
    }
}

pub(super) fn substitute_self_type(ty: &Type, self_type: &Type) -> Type {
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

pub(super) fn substitute_type_params(ty: &Type, substitutions: &HashMap<String, Type>) -> Type {
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
