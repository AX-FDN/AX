use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::ast::{ItemKind, Program};
use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::project::Project;
use crate::source::SourceFile;

use super::super::helpers::{builtin_types, item_name};
use super::helpers::{
    canonical_item_name, check_generic_type_params, check_trait_impl, collect_unit_contexts,
    method_lookup_type_name,
};
use super::*;
impl<'a> ProgramInfo<'a> {
    pub(in crate::semantic) fn collect(
        source: &'a SourceFile,
        program: &Program,
        project: Option<&Project>,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Self {
        let mut named_types = builtin_types();
        let mut unit_contexts = collect_unit_contexts(source, program, project, diagnostics);
        let module_mode = program
            .source_units
            .iter()
            .any(|unit| unit.module.is_some() || !unit.imports.is_empty());
        let mut modules = HashSet::new();

        if module_mode {
            for unit in &program.source_units {
                let mut duplicate_imports = HashSet::new();
                for import in &unit.imports {
                    if !duplicate_imports.insert(import.path.clone()) {
                        diagnostics.push(
                            Diagnostic::new(
                                "S0041",
                                format!("duplicate import of module `{}`", import.path),
                                source,
                                import.span,
                            )
                            .with_kind(DiagnosticKind::DuplicateModuleImport)
                            .with_suggestion("keep only one import for each module path"),
                        );
                    }
                }

                if unit.is_entry {
                    if let Some(module) = &unit.module {
                        diagnostics.push(
                            Diagnostic::new(
                                "S0037",
                                "entry file cannot declare `module` in the minimal module mode",
                                source,
                                module.span,
                            )
                            .with_kind(DiagnosticKind::EntryFileDeclaresModule)
                            .with_note(
                                "the project entry remains the manifest-owned root unit for `fn main() -> i32`",
                            )
                            .with_suggestion("remove the `module ...;` line from the entry file"),
                        );
                    }
                    continue;
                }

                let Some(module) = &unit.module else {
                    diagnostics.push(
                        Diagnostic::new(
                            "S0038",
                            "support source is missing a `module` declaration in module mode",
                            source,
                            unit.span,
                        )
                        .with_kind(DiagnosticKind::SupportSourceMissingModuleDeclaration)
                        .with_note(format!(
                            "support source `{}` must declare its module path before top-level items",
                            unit.path
                        ))
                        .with_suggestion("add a top-of-file declaration like `module foundation.search;`"),
                    );
                    continue;
                };

                if let Some(project) = project
                    && let Some(expected) = project.expected_module_path(Path::new(&unit.path))
                    && module.path != expected
                {
                    diagnostics.push(
                        Diagnostic::new(
                            "S0039",
                            format!(
                                "module path `{}` does not match the expected path `{expected}`",
                                module.path
                            ),
                            source,
                            module.span,
                        )
                        .with_kind(DiagnosticKind::ModulePathMismatch)
                        .with_note(format!(
                            "AX derives the minimal module path from the support-source root and the file path of `{}`",
                            unit.path
                        ))
                        .with_suggestion(format!("change the declaration to `module {expected};`")),
                    );
                }

                if !modules.insert(module.path.clone()) {
                    diagnostics.push(
                        Diagnostic::new(
                            "S0040",
                            format!("duplicate module path `{}`", module.path),
                            source,
                            module.span,
                        )
                        .with_kind(DiagnosticKind::DuplicateModulePath)
                        .with_suggestion(
                            "rename or move one of the support files so each module path is unique",
                        ),
                    );
                }
            }

            for unit in &program.source_units {
                for import in &unit.imports {
                    if !modules.contains(&import.path) {
                        diagnostics.push(
                            Diagnostic::new(
                                "S0042",
                                format!("imported module `{}` does not exist", import.path),
                                source,
                                import.span,
                            )
                            .with_kind(DiagnosticKind::ImportedModuleMissing)
                            .with_suggestion(
                                "import an existing support module declared in this project",
                            ),
                        );
                    }
                }
            }
        }

        let mut all_item_names: HashMap<String, usize> = HashMap::new();
        let mut has_main = false;

        for item in &program.items {
            let unit_path = source.display_path_for_offset(item.span.start).to_string();
            let unit = unit_contexts.get(&unit_path).cloned().unwrap_or_default();
            let canonical_name = canonical_item_name(module_mode, &unit, item_name(&item.kind));

            if !matches!(item.kind, ItemKind::Impl { .. })
                && let Some(previous_start) =
                    all_item_names.insert(canonical_name.clone(), item.span.start)
            {
                let (line, column) = source.line_col(previous_start);
                diagnostics.push(
                    Diagnostic::new(
                        "S0001",
                        format!("duplicate definition of `{canonical_name}`"),
                        source,
                        item.span,
                    )
                    .with_note(format!("previous definition was at {line}:{column}")),
                );
            }

            match &item.kind {
                ItemKind::Struct { .. } => {
                    named_types.insert(canonical_name.clone(), Type::Struct(canonical_name));
                }
                ItemKind::Enum { .. } => {
                    named_types.insert(canonical_name.clone(), Type::Enum(canonical_name));
                }
                ItemKind::TypeAlias { .. } => {}
                ItemKind::Trait { .. } => {}
                ItemKind::Impl { .. } => {}
                ItemKind::Function {
                    name,
                    type_params,
                    type_param_bounds,
                    params,
                    return_type,
                    ..
                } if name == "main" && unit.is_entry => {
                    has_main = true;
                    if !type_params.is_empty()
                        || !type_param_bounds.is_empty()
                        || !params.is_empty()
                        || return_type.direct_name() != Some("i32")
                    {
                        diagnostics.push(
                            Diagnostic::new(
                                "S0005",
                                "`main` must have the signature `fn main() -> i32`",
                                source,
                                return_type.span,
                            )
                            .with_note(
                                "the current AX prototype does not allow parameters on `main`",
                            )
                            .with_suggestion("change main to `fn main() -> i32 { ... }`"),
                        );
                    }
                }
                _ => {}
            }

            unit_contexts.insert(unit_path, unit);
        }

        let mut info = Self {
            source,
            named_types,
            functions: HashMap::new(),
            constants: HashMap::new(),
            type_aliases: HashMap::new(),
            methods: HashMap::new(),
            traits: HashMap::new(),
            trait_impls: Vec::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            has_main,
            module_mode,
            modules,
            units: unit_contexts,
        };

        for item in &program.items {
            let unit_path = source.display_path_for_offset(item.span.start).to_string();
            let unit = info.unit_context(&unit_path).cloned().unwrap_or_default();
            let canonical_name =
                canonical_item_name(info.module_mode, &unit, item_name(&item.kind));

            match &item.kind {
                ItemKind::Struct { type_params, .. } => {
                    info.structs
                        .entry(canonical_name)
                        .or_insert_with(|| StructInfo {
                            type_params: type_params.clone(),
                            fields: HashMap::new(),
                        });
                }
                ItemKind::Enum { type_params, .. } => {
                    info.enums
                        .entry(canonical_name)
                        .or_insert_with(|| EnumInfo {
                            type_params: type_params.clone(),
                            variants: HashMap::new(),
                        });
                }
                _ => {}
            }
        }

        for item in &program.items {
            let unit_path = source.display_path_for_offset(item.span.start).to_string();
            let unit = info.unit_context(&unit_path).cloned().unwrap_or_default();
            let canonical_name =
                canonical_item_name(info.module_mode, &unit, item_name(&item.kind));

            let ItemKind::TypeAlias {
                type_params,
                target,
                ..
            } = &item.kind
            else {
                continue;
            };

            check_generic_type_params(source, &canonical_name, type_params, item.span, diagnostics);
            let resolved_target =
                info.resolve_type_ref_with_params(target, &unit_path, type_params, diagnostics);
            if !resolved_target.is_error() {
                info.type_aliases.insert(
                    canonical_name.clone(),
                    TypeAliasInfo {
                        type_params: type_params.clone(),
                        target: resolved_target.clone(),
                    },
                );
                if type_params.is_empty() {
                    info.named_types.insert(canonical_name, resolved_target);
                }
            }
        }

        for item in &program.items {
            let unit_path = source.display_path_for_offset(item.span.start).to_string();
            let unit = info.unit_context(&unit_path).cloned().unwrap_or_default();
            let canonical_name =
                canonical_item_name(info.module_mode, &unit, item_name(&item.kind));

            match &item.kind {
                ItemKind::Struct {
                    type_params,
                    fields,
                    ..
                } => {
                    check_generic_type_params(
                        source,
                        &canonical_name,
                        type_params,
                        item.span,
                        diagnostics,
                    );

                    let mut field_map = HashMap::new();
                    for field in fields {
                        let resolved_type = info.resolve_type_ref_with_params(
                            &field.ty,
                            &unit_path,
                            type_params,
                            diagnostics,
                        );
                        if let Some(previous_field) = field_map.insert(
                            field.name.clone(),
                            StructFieldInfo {
                                ty: resolved_type,
                                start: field.span.start,
                            },
                        ) {
                            let (line, column) = source.line_col(previous_field.start);
                            diagnostics.push(
                                Diagnostic::new(
                                    "S0001",
                                    format!(
                                        "duplicate field `{}` in struct `{canonical_name}`",
                                        field.name
                                    ),
                                    source,
                                    field.span,
                                )
                                .with_note(format!(
                                    "previous field was declared at {line}:{column}"
                                )),
                            );
                        }
                    }
                    info.structs.insert(
                        canonical_name,
                        StructInfo {
                            type_params: type_params.clone(),
                            fields: field_map,
                        },
                    );
                }
                ItemKind::Enum {
                    type_params,
                    variants,
                    ..
                } => {
                    check_generic_type_params(
                        source,
                        &canonical_name,
                        type_params,
                        item.span,
                        diagnostics,
                    );
                    let mut variant_names = HashMap::new();
                    for variant in variants {
                        if variant_names.contains_key(&variant.name) {
                            diagnostics.push(
                                Diagnostic::new(
                                    "S0001",
                                    format!(
                                        "duplicate variant `{}` in enum `{canonical_name}`",
                                        variant.name
                                    ),
                                    source,
                                    variant.span,
                                )
                                .with_suggestion("remove or rename the duplicate variant"),
                            );
                            continue;
                        }
                        let payload = variant.payload.as_ref().map(|payload| {
                            info.resolve_type_ref_with_params(
                                payload,
                                &unit_path,
                                type_params,
                                diagnostics,
                            )
                        });
                        variant_names.insert(variant.name.clone(), EnumVariantInfo { payload });
                    }
                    info.enums.insert(
                        canonical_name,
                        EnumInfo {
                            type_params: type_params.clone(),
                            variants: variant_names,
                        },
                    );
                }
                ItemKind::Trait { methods, .. } => {
                    let mut method_map = HashMap::new();
                    let self_params = vec!["Self".to_string()];
                    for method in methods {
                        let resolved_params = method
                            .params
                            .iter()
                            .map(|param| ParamInfo {
                                name: param.name.clone(),
                                ty: info.resolve_type_ref_with_params(
                                    &param.ty,
                                    &unit_path,
                                    &self_params,
                                    diagnostics,
                                ),
                            })
                            .collect::<Vec<_>>();
                        let resolved_return_type = info.resolve_type_ref_with_params(
                            &method.return_type,
                            &unit_path,
                            &self_params,
                            diagnostics,
                        );
                        if method_map
                            .insert(
                                method.name.clone(),
                                FunctionSignature {
                                    type_params: Vec::new(),
                                    type_param_bounds: Vec::new(),
                                    params: resolved_params,
                                    return_type: resolved_return_type,
                                },
                            )
                            .is_some()
                        {
                            diagnostics.push(
                                Diagnostic::new(
                                    "S0001",
                                    format!(
                                        "duplicate trait method `{}` in trait `{canonical_name}`",
                                        method.name
                                    ),
                                    source,
                                    method.span,
                                )
                                .with_suggestion("keep each trait method name unique"),
                            );
                        }
                    }
                    info.traits.insert(
                        canonical_name,
                        TraitInfo {
                            methods: method_map,
                        },
                    );
                }
                ItemKind::Const { ty, .. } => {
                    let resolved_type = info.resolve_type_ref(ty, &unit_path, diagnostics);
                    info.constants.insert(
                        canonical_name,
                        ConstInfo {
                            ty: resolved_type,
                            start: item.span.start,
                        },
                    );
                }
                ItemKind::TypeAlias { .. } => {}
                ItemKind::Function {
                    type_params,
                    type_param_bounds,
                    params,
                    return_type,
                    ..
                } => {
                    check_generic_type_params(
                        source,
                        &canonical_name,
                        type_params,
                        item.span,
                        diagnostics,
                    );
                    let resolved_bounds = info.resolve_type_param_bounds(
                        type_params,
                        type_param_bounds,
                        &unit_path,
                        diagnostics,
                    );
                    let resolved_params = params
                        .iter()
                        .map(|param| ParamInfo {
                            name: param.name.clone(),
                            ty: info.resolve_type_ref_with_params(
                                &param.ty,
                                &unit_path,
                                type_params,
                                diagnostics,
                            ),
                        })
                        .collect::<Vec<_>>();
                    let resolved_return_type = info.resolve_type_ref_with_params(
                        return_type,
                        &unit_path,
                        type_params,
                        diagnostics,
                    );
                    info.functions.insert(
                        canonical_name,
                        FunctionSignature {
                            type_params: type_params.clone(),
                            type_param_bounds: resolved_bounds,
                            params: resolved_params,
                            return_type: resolved_return_type,
                        },
                    );
                }
                ItemKind::Impl {
                    type_params,
                    trait_ref,
                    target,
                    methods,
                } => {
                    check_generic_type_params(
                        source,
                        &format!("impl {}", target.describe()),
                        type_params,
                        item.span,
                        diagnostics,
                    );
                    let self_type = info.resolve_type_ref_with_params(
                        target,
                        &unit_path,
                        type_params,
                        diagnostics,
                    );
                    if matches!(self_type, Type::Error) {
                        continue;
                    }
                    let trait_info = trait_ref.as_ref().and_then(|trait_ref| {
                        info.resolve_trait_ref(trait_ref, &unit_path, diagnostics)
                            .and_then(|name| {
                                info.traits
                                    .get(&name)
                                    .cloned()
                                    .map(|trait_info| (name, trait_info))
                            })
                    });

                    let mut method_names: HashMap<String, usize> = HashMap::new();
                    let mut impl_signatures = HashMap::new();
                    for method in methods {
                        check_generic_type_params(
                            source,
                            &format!("method {}", method.name),
                            &method.type_params,
                            method.span,
                            diagnostics,
                        );
                        for method_type_param in &method.type_params {
                            if type_params.iter().any(|param| param == method_type_param) {
                                diagnostics.push(
                                    Diagnostic::new(
                                        "S0058",
                                        format!(
                                            "method `{}` redeclares generic type parameter `{method_type_param}` from the enclosing impl",
                                            method.name
                                        ),
                                        source,
                                        method.span,
                                    )
                                    .with_suggestion(
                                        "use a distinct method type parameter name",
                                    ),
                                );
                            }
                        }
                        let all_type_params = type_params
                            .iter()
                            .cloned()
                            .chain(method.type_params.iter().cloned())
                            .collect::<Vec<_>>();

                        if let Some(previous_start) =
                            method_names.insert(method.name.clone(), method.span.start)
                        {
                            let (line, column) = source.line_col(previous_start);
                            diagnostics.push(
                                Diagnostic::new(
                                    "S0001",
                                    format!(
                                        "duplicate method `{}` for `{}`",
                                        method.name,
                                        self_type.describe()
                                    ),
                                    source,
                                    method.span,
                                )
                                .with_note(format!(
                                    "previous method was declared at {line}:{column}"
                                )),
                            );
                            continue;
                        }

                        let resolved_params = method
                            .params
                            .iter()
                            .map(|param| ParamInfo {
                                name: param.name.clone(),
                                ty: info.resolve_type_ref_with_params(
                                    &param.ty,
                                    &unit_path,
                                    &all_type_params,
                                    diagnostics,
                                ),
                            })
                            .collect::<Vec<_>>();
                        let resolved_return_type = info.resolve_type_ref_with_params(
                            &method.return_type,
                            &unit_path,
                            &all_type_params,
                            diagnostics,
                        );
                        let method_signature = FunctionSignature {
                            type_params: all_type_params.clone(),
                            type_param_bounds: Vec::new(),
                            params: resolved_params.clone(),
                            return_type: resolved_return_type.clone(),
                        };
                        let method_function_name =
                            format!("{}.{}", method_lookup_type_name(&self_type), method.name);
                        let has_self_param =
                            resolved_params.first().map(|param| param.name.as_str())
                                == Some("self");

                        if !has_self_param && trait_ref.is_some() {
                            diagnostics.push(
                                Diagnostic::new(
                                    "S0056",
                                    format!(
                                        "method `{}` must declare an explicit `self` parameter first",
                                        method.name
                                    ),
                                    source,
                                    method.span,
                                )
                                .with_suggestion(format!(
                                    "write the method as `fn {}(self: {}) -> ...`",
                                    method.name,
                                    self_type.describe()
                                )),
                            );
                        } else if has_self_param
                            && let Some(self_param) = resolved_params.first()
                            && !self_param.ty.is_assignable_to(&self_type)
                        {
                            diagnostics.push(
                                Diagnostic::new(
                                    "S0056",
                                    format!(
                                        "method `{}` self parameter must be `{}`, found `{}`",
                                        method.name,
                                        self_type.describe(),
                                        self_param.ty.describe()
                                    ),
                                    source,
                                    method.params[0].span,
                                )
                                .with_suggestion(format!(
                                    "change the first parameter to `self: {}`",
                                    self_type.describe()
                                )),
                            );
                        }

                        if has_self_param {
                            info.methods.insert(
                                method_function_name,
                                MethodSignature {
                                    function: method_signature.clone(),
                                },
                            );
                        } else if trait_ref.is_none() {
                            info.functions
                                .insert(method_function_name, method_signature);
                        }
                        impl_signatures.insert(
                            method.name.clone(),
                            FunctionSignature {
                                type_params: Vec::new(),
                                type_param_bounds: Vec::new(),
                                params: method
                                    .params
                                    .iter()
                                    .map(|param| ParamInfo {
                                        name: param.name.clone(),
                                        ty: info.resolve_type_ref_with_params(
                                            &param.ty,
                                            &unit_path,
                                            &all_type_params,
                                            diagnostics,
                                        ),
                                    })
                                    .collect(),
                                return_type: info.resolve_type_ref_with_params(
                                    &method.return_type,
                                    &unit_path,
                                    &all_type_params,
                                    diagnostics,
                                ),
                            },
                        );
                    }

                    if let Some((trait_name, trait_info)) = trait_info {
                        check_trait_impl(
                            source,
                            &trait_name,
                            &self_type,
                            &trait_info,
                            &impl_signatures,
                            item.span,
                            diagnostics,
                        );
                        info.trait_impls.push((self_type, trait_name));
                    }
                }
            }
        }

        info
    }
}
