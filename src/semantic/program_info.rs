use std::collections::{HashMap, HashSet};

use crate::source::SourceFile;

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

#[path = "program_info/collect.rs"]
mod collect;
#[path = "program_info/helpers.rs"]
mod helpers;
#[path = "program_info/resolve.rs"]
mod resolve;
