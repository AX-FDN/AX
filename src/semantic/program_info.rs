use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::ast::{ItemKind, Program, TypeRef};
use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::project::Project;
use crate::source::SourceFile;

use super::helpers::{builtin_types, item_name};
use super::types::{
    ConstInfo, EnumInfo, EnumVariantInfo, FunctionSignature, MethodSignature, ParamInfo,
    StructFieldInfo, StructInfo, TraitInfo, Type, TypeAliasInfo, TypeParamBoundInfo,
};

#[derive(Debug, Clone, Default)]
pub(super) struct UnitContext {
    pub(super) module_path: Option<String>,
    pub(super) imports: HashSet<String>,
    pub(super) is_entry: bool,
}

pub(super) struct ProgramInfo<'a> {
    pub(super) source: &'a SourceFile,
    pub(super) named_types: HashMap<String, Type>,
    pub(super) functions: HashMap<String, FunctionSignature>,
    pub(super) constants: HashMap<String, ConstInfo>,
    pub(super) type_aliases: HashMap<String, TypeAliasInfo>,
    pub(super) methods: HashMap<String, MethodSignature>,
    pub(super) traits: HashMap<String, TraitInfo>,
    trait_impls: Vec<(Type, String)>,
    pub(super) structs: HashMap<String, StructInfo>,
    pub(super) enums: HashMap<String, EnumInfo>,
    pub(super) has_main: bool,
    pub(super) module_mode: bool,
    modules: HashSet<String>,
    units: HashMap<String, UnitContext>,
}

impl<'a> ProgramInfo<'a> {
    pub(super) fn collect(
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

    pub(super) fn resolve_type_ref(
        &self,
        ty: &TypeRef,
        current_unit_path: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Type {
        self.resolve_type_ref_with_params(ty, current_unit_path, &[], diagnostics)
    }

    pub(super) fn resolve_type_ref_with_params(
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

    pub(super) fn resolve_named_type_key(
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

    pub(super) fn named_type_candidate_exists(&self, name: &str, current_unit_path: &str) -> bool {
        self.named_key_candidate_exists(name, current_unit_path, &self.named_types)
            || self.named_key_candidate_exists(name, current_unit_path, &self.type_aliases)
    }

    pub(super) fn resolve_function_key(
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

    pub(super) fn function_candidate_exists(&self, name: &str, current_unit_path: &str) -> bool {
        self.named_key_candidate_exists(name, current_unit_path, &self.functions)
    }

    pub(super) fn resolve_constant_key(
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

    pub(super) fn constant_candidate_exists(&self, name: &str, current_unit_path: &str) -> bool {
        self.named_key_candidate_exists(name, current_unit_path, &self.constants)
    }

    pub(super) fn function_signature_for_definition(
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

    fn resolve_trait_ref(
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

    fn resolve_type_param_bounds(
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

    pub(super) fn method_signature(
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

    pub(super) fn type_satisfies_trait_bound(&self, ty: &Type, trait_name: &str) -> bool {
        self.trait_impls
            .iter()
            .any(|(impl_type, implemented_trait)| {
                implemented_trait == trait_name && type_matches_impl_target(ty, impl_type)
            })
    }

    pub(super) fn trait_bound_method_signature(
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

    pub(super) fn unit_context(&self, path: &str) -> Option<&UnitContext> {
        self.units.get(path)
    }
}

fn collect_unit_contexts(
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

fn method_lookup_type_name(ty: &Type) -> String {
    match ty {
        Type::StructInstance { name, .. } | Type::EnumInstance { name, .. } => name.clone(),
        other => other.describe(),
    }
}

fn type_matches_impl_target(actual: &Type, impl_target: &Type) -> bool {
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

fn canonical_item_name(module_mode: bool, unit: &UnitContext, item_name: &str) -> String {
    if module_mode && let Some(module_path) = &unit.module_path {
        return format!("{module_path}.{item_name}");
    }
    item_name.to_string()
}

fn check_generic_type_params(
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

fn check_trait_impl(
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
