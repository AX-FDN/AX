use std::collections::{HashMap, HashSet};

use crate::ast::{ItemKind, Program, TypeRef};
use crate::diagnostics::Diagnostic;
use crate::source::SourceFile;

use super::helpers::{builtin_types, item_name};
use super::types::{EnumInfo, FunctionSignature, ParamInfo, StructFieldInfo, StructInfo, Type};

pub(super) struct ProgramInfo<'a> {
    pub(super) source: &'a SourceFile,
    pub(super) named_types: HashMap<String, Type>,
    pub(super) functions: HashMap<String, FunctionSignature>,
    pub(super) structs: HashMap<String, StructInfo>,
    pub(super) enums: HashMap<String, EnumInfo>,
    pub(super) has_main: bool,
}

impl<'a> ProgramInfo<'a> {
    pub(super) fn collect(
        source: &'a SourceFile,
        program: &Program,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Self {
        let mut named_types = builtin_types();
        let mut all_item_names: HashMap<String, usize> = HashMap::new();
        let mut has_main = false;

        for item in &program.items {
            let name = item_name(&item.kind);
            if let Some(previous_start) = all_item_names.insert(name.to_string(), item.span.start) {
                let (line, column) = source.line_col(previous_start);
                diagnostics.push(
                    Diagnostic::new(
                        "S0001",
                        format!("duplicate definition of `{name}`"),
                        source,
                        item.span,
                    )
                    .with_note(format!("previous definition was at {line}:{column}")),
                );
            }

            match &item.kind {
                ItemKind::Struct { name, .. } => {
                    named_types.insert(name.clone(), Type::Struct(name.clone()));
                }
                ItemKind::Enum { name, .. } => {
                    named_types.insert(name.clone(), Type::Enum(name.clone()));
                }
                ItemKind::Function {
                    name,
                    params,
                    return_type,
                    ..
                } if name == "main" => {
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
        }

        let mut info = Self {
            source,
            named_types,
            functions: HashMap::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            has_main,
        };

        for item in &program.items {
            match &item.kind {
                ItemKind::Struct { name, fields } => {
                    let mut field_map = HashMap::new();
                    for field in fields {
                        let resolved_type = info.resolve_type_ref(&field.ty, diagnostics);
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
                                    format!("duplicate field `{}` in struct `{name}`", field.name),
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
                        .insert(name.clone(), StructInfo { fields: field_map });
                }
                ItemKind::Enum { name, variants } => {
                    let mut variant_names = HashSet::new();
                    for variant in variants {
                        if !variant_names.insert(variant.name.clone()) {
                            diagnostics.push(
                                Diagnostic::new(
                                    "S0001",
                                    format!(
                                        "duplicate variant `{}` in enum `{name}`",
                                        variant.name
                                    ),
                                    source,
                                    variant.span,
                                )
                                .with_suggestion("remove or rename the duplicate variant"),
                            );
                        }
                    }
                    info.enums.insert(
                        name.clone(),
                        EnumInfo {
                            variants: variant_names,
                        },
                    );
                }
                ItemKind::Function {
                    name,
                    params,
                    return_type,
                    ..
                } => {
                    let resolved_params = params
                        .iter()
                        .map(|param| ParamInfo {
                            name: param.name.clone(),
                            ty: info.resolve_type_ref(&param.ty, diagnostics),
                        })
                        .collect::<Vec<_>>();
                    let resolved_return_type = info.resolve_type_ref(return_type, diagnostics);
                    info.functions.insert(
                        name.clone(),
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

    pub(super) fn resolve_type_ref(&self, ty: &TypeRef, diagnostics: &mut Vec<Diagnostic>) -> Type {
        match (&ty.name, &ty.element, ty.length) {
            (Some(name), None, None) => match self.named_types.get(name) {
                Some(found) => found.clone(),
                None => {
                    diagnostics.push(
                        Diagnostic::new(
                            "S0006",
                            format!("unknown type `{}`", name),
                            self.source,
                            ty.span,
                        )
                        .with_suggestion(
                            "use a builtin type, `[Type]`, `[Type; N]`, or declare the type before referencing it",
                        ),
                    );
                    Type::Error
                }
            },
            (None, Some(element), None) => Type::Slice {
                element: Box::new(self.resolve_type_ref(element, diagnostics)),
            },
            (None, Some(element), Some(length)) => Type::Array {
                element: Box::new(self.resolve_type_ref(element, diagnostics)),
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
}
