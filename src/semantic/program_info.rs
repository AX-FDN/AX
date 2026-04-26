use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::ast::{ItemKind, Program, TypeRef};
use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::project::Project;
use crate::source::SourceFile;

use super::helpers::{builtin_types, item_name};
use super::types::{
    EnumInfo, EnumVariantInfo, FunctionSignature, ParamInfo, StructFieldInfo, StructInfo, Type,
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

            if let Some(previous_start) =
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
                ItemKind::Function {
                    name,
                    params,
                    return_type,
                    ..
                } if name == "main" && unit.is_entry => {
                    has_main = true;
                    if !params.is_empty() || return_type.direct_name() != Some("i32") {
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
                ItemKind::Struct { fields, .. } => {
                    let mut field_map = HashMap::new();
                    for field in fields {
                        let resolved_type =
                            info.resolve_type_ref(&field.ty, &unit_path, diagnostics);
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
                    info.structs
                        .insert(canonical_name, StructInfo { fields: field_map });
                }
                ItemKind::Enum { variants, .. } => {
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
                        let payload = variant
                            .payload
                            .as_ref()
                            .map(|payload| info.resolve_type_ref(payload, &unit_path, diagnostics));
                        variant_names.insert(variant.name.clone(), EnumVariantInfo { payload });
                    }
                    info.enums.insert(
                        canonical_name,
                        EnumInfo {
                            variants: variant_names,
                        },
                    );
                }
                ItemKind::Function {
                    params,
                    return_type,
                    ..
                } => {
                    let resolved_params = params
                        .iter()
                        .map(|param| ParamInfo {
                            name: param.name.clone(),
                            ty: info.resolve_type_ref(&param.ty, &unit_path, diagnostics),
                        })
                        .collect::<Vec<_>>();
                    let resolved_return_type =
                        info.resolve_type_ref(return_type, &unit_path, diagnostics);
                    info.functions.insert(
                        canonical_name,
                        FunctionSignature {
                            params: resolved_params,
                            return_type: resolved_return_type,
                        },
                    );
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
        match (&ty.name, &ty.element, ty.length) {
            (Some(name), None, None) => {
                match self.resolve_named_type_key(name, current_unit_path, ty.span, diagnostics) {
                    Some(found) => self
                        .named_types
                        .get(&found)
                        .cloned()
                        .expect("resolved type should exist"),
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
            (None, Some(element), None) => Type::Slice {
                element: Box::new(self.resolve_type_ref(element, current_unit_path, diagnostics)),
            },
            (None, Some(element), Some(length)) => Type::Array {
                element: Box::new(self.resolve_type_ref(element, current_unit_path, diagnostics)),
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

fn canonical_item_name(module_mode: bool, unit: &UnitContext, item_name: &str) -> String {
    if module_mode && let Some(module_path) = &unit.module_path {
        return format!("{module_path}.{item_name}");
    }
    item_name.to_string()
}
