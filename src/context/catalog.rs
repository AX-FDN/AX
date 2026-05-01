use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{Block, Expr, ExprKind, ItemKind, Program, Stmt, StmtKind, Visibility};
use crate::source::SourceFile;

use super::{ResolvedUnit, host_boundary_class, normalize_path_text};

#[derive(Debug, Clone)]
pub(super) struct SymbolCatalog {
    pub(super) definitions: BTreeMap<String, DefinedSymbol>,
    pub(super) simple_names: BTreeMap<String, Vec<String>>,
    pub(super) callers_by_symbol: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Debug, Clone)]
pub(super) struct DefinedSymbol {
    pub(super) qualified_name: String,
    pub(super) kind: DefinedSymbolKind,
    pub(super) visibility: Visibility,
    pub(super) source_path: String,
    pub(super) module_path: Option<String>,
    pub(super) is_entry: bool,
    pub(super) imports: Vec<String>,
    pub(super) params: Vec<SymbolParamData>,
    pub(super) return_type: Option<String>,
    pub(super) related_types: BTreeSet<String>,
    pub(super) raw_call_order: Vec<String>,
    pub(super) resolved_callees: BTreeSet<String>,
    pub(super) resolved_callee_order: Vec<String>,
    pub(super) host_classes: BTreeSet<String>,
    pub(super) branch_kinds: BTreeSet<String>,
    pub(super) branch_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DefinedSymbolKind {
    Function,
    Const,
    TypeAlias,
    Struct,
    Enum,
    Trait,
}

impl DefinedSymbolKind {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Const => "const",
            Self::TypeAlias => "type_alias",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Trait => "trait",
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct SymbolParamData {
    pub(super) name: String,
    pub(super) ty: String,
}

#[derive(Debug, Clone, Default)]
struct SymbolWalk {
    raw_calls: BTreeSet<String>,
    raw_call_order: Vec<String>,
    related_types: BTreeSet<String>,
    branch_kinds: BTreeSet<String>,
    branch_count: usize,
}

pub(super) fn symbol_reaches_target(
    symbol_catalog: &SymbolCatalog,
    current: &str,
    target: &str,
    reachable_set: &BTreeSet<String>,
    visited: &mut BTreeSet<String>,
) -> bool {
    let Some(symbol) = symbol_catalog.definitions.get(current) else {
        return false;
    };

    for callee in &symbol.resolved_callee_order {
        if !reachable_set.contains(callee) {
            continue;
        }
        if callee == target {
            return true;
        }
        if visited.insert(callee.clone())
            && symbol_reaches_target(symbol_catalog, callee, target, reachable_set, visited)
        {
            return true;
        }
    }

    false
}

pub(super) fn build_symbol_catalog(
    source: &SourceFile,
    program: &Program,
    units: &[ResolvedUnit],
) -> SymbolCatalog {
    let units_by_path = units
        .iter()
        .map(|unit| (unit.path.clone(), unit.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut definitions = BTreeMap::<String, DefinedSymbol>::new();
    let mut simple_names = BTreeMap::<String, Vec<String>>::new();

    for item in &program.items {
        let source_path = normalize_path_text(source.display_path_for_offset(item.span.start));
        let Some(unit) = units_by_path.get(&source_path) else {
            continue;
        };

        match &item.kind {
            ItemKind::Function {
                name,
                params,
                return_type,
                body,
                ..
            } => {
                let qualified_name = qualify_symbol_name(unit.module_path.as_deref(), name);
                let mut walk = SymbolWalk::default();
                collect_type_ref_names(return_type, &mut walk.related_types);
                for param in params {
                    collect_type_ref_names(&param.ty, &mut walk.related_types);
                }
                collect_symbol_walk_for_block(body, &mut walk);
                let host_classes = walk
                    .raw_calls
                    .iter()
                    .filter_map(|call| host_boundary_class(call))
                    .map(str::to_string)
                    .collect::<BTreeSet<_>>();

                let definition = DefinedSymbol {
                    qualified_name: qualified_name.clone(),
                    kind: DefinedSymbolKind::Function,
                    visibility: item.visibility,
                    source_path: source_path.clone(),
                    module_path: unit.module_path.clone(),
                    is_entry: unit.is_entry,
                    imports: unit.imports.clone(),
                    params: params
                        .iter()
                        .map(|param| SymbolParamData {
                            name: param.name.clone(),
                            ty: param.ty.describe(),
                        })
                        .collect(),
                    return_type: Some(return_type.describe()),
                    related_types: walk.related_types,
                    raw_call_order: walk.raw_call_order,
                    resolved_callees: BTreeSet::new(),
                    resolved_callee_order: Vec::new(),
                    host_classes,
                    branch_kinds: walk.branch_kinds,
                    branch_count: walk.branch_count,
                };
                definitions.insert(qualified_name.clone(), definition);
                simple_names
                    .entry(name.clone())
                    .or_default()
                    .push(qualified_name);
            }
            ItemKind::Const { name, ty, value } => {
                let qualified_name = qualify_symbol_name(unit.module_path.as_deref(), name);
                let mut walk = SymbolWalk::default();
                collect_type_ref_names(ty, &mut walk.related_types);
                collect_symbol_walk_for_expr(value, &mut walk);
                definitions.insert(
                    qualified_name.clone(),
                    DefinedSymbol {
                        qualified_name: qualified_name.clone(),
                        kind: DefinedSymbolKind::Const,
                        visibility: item.visibility,
                        source_path: source_path.clone(),
                        module_path: unit.module_path.clone(),
                        is_entry: unit.is_entry,
                        imports: unit.imports.clone(),
                        params: Vec::new(),
                        return_type: Some(ty.describe()),
                        related_types: walk.related_types,
                        raw_call_order: walk.raw_call_order,
                        resolved_callees: BTreeSet::new(),
                        resolved_callee_order: Vec::new(),
                        host_classes: BTreeSet::new(),
                        branch_kinds: walk.branch_kinds,
                        branch_count: walk.branch_count,
                    },
                );
                simple_names
                    .entry(name.clone())
                    .or_default()
                    .push(qualified_name);
            }
            ItemKind::TypeAlias { name, target, .. } => {
                let qualified_name = qualify_symbol_name(unit.module_path.as_deref(), name);
                let mut related_types = BTreeSet::new();
                collect_type_ref_names(target, &mut related_types);
                definitions.insert(
                    qualified_name.clone(),
                    DefinedSymbol {
                        qualified_name: qualified_name.clone(),
                        kind: DefinedSymbolKind::TypeAlias,
                        visibility: item.visibility,
                        source_path: source_path.clone(),
                        module_path: unit.module_path.clone(),
                        is_entry: unit.is_entry,
                        imports: unit.imports.clone(),
                        params: Vec::new(),
                        return_type: Some(target.describe()),
                        related_types,
                        raw_call_order: Vec::new(),
                        resolved_callees: BTreeSet::new(),
                        resolved_callee_order: Vec::new(),
                        host_classes: BTreeSet::new(),
                        branch_kinds: BTreeSet::new(),
                        branch_count: 0,
                    },
                );
                simple_names
                    .entry(name.clone())
                    .or_default()
                    .push(qualified_name);
            }
            ItemKind::Struct { name, fields, .. } => {
                let qualified_name = qualify_symbol_name(unit.module_path.as_deref(), name);
                let mut related_types = BTreeSet::new();
                for field in fields {
                    collect_type_ref_names(&field.ty, &mut related_types);
                }
                definitions.insert(
                    qualified_name.clone(),
                    DefinedSymbol {
                        qualified_name: qualified_name.clone(),
                        kind: DefinedSymbolKind::Struct,
                        visibility: item.visibility,
                        source_path: source_path.clone(),
                        module_path: unit.module_path.clone(),
                        is_entry: unit.is_entry,
                        imports: unit.imports.clone(),
                        params: Vec::new(),
                        return_type: None,
                        related_types,
                        raw_call_order: Vec::new(),
                        resolved_callees: BTreeSet::new(),
                        resolved_callee_order: Vec::new(),
                        host_classes: BTreeSet::new(),
                        branch_kinds: BTreeSet::new(),
                        branch_count: 0,
                    },
                );
                simple_names
                    .entry(name.clone())
                    .or_default()
                    .push(qualified_name);
            }
            ItemKind::Enum { name, variants, .. } => {
                let qualified_name = qualify_symbol_name(unit.module_path.as_deref(), name);
                let mut related_types = BTreeSet::new();
                for variant in variants {
                    if let Some(payload) = variant.payload.as_ref() {
                        collect_type_ref_names(payload, &mut related_types);
                    }
                }
                definitions.insert(
                    qualified_name.clone(),
                    DefinedSymbol {
                        qualified_name: qualified_name.clone(),
                        kind: DefinedSymbolKind::Enum,
                        visibility: item.visibility,
                        source_path: source_path.clone(),
                        module_path: unit.module_path.clone(),
                        is_entry: unit.is_entry,
                        imports: unit.imports.clone(),
                        params: Vec::new(),
                        return_type: None,
                        related_types,
                        raw_call_order: Vec::new(),
                        resolved_callees: BTreeSet::new(),
                        resolved_callee_order: Vec::new(),
                        host_classes: BTreeSet::new(),
                        branch_kinds: BTreeSet::new(),
                        branch_count: 0,
                    },
                );
                simple_names
                    .entry(name.clone())
                    .or_default()
                    .push(qualified_name);
            }
            ItemKind::Trait { name, methods } => {
                let qualified_name = qualify_symbol_name(unit.module_path.as_deref(), name);
                let mut related_types = BTreeSet::new();
                for method in methods {
                    collect_type_ref_names(&method.return_type, &mut related_types);
                    for param in &method.params {
                        collect_type_ref_names(&param.ty, &mut related_types);
                    }
                }
                definitions.insert(
                    qualified_name.clone(),
                    DefinedSymbol {
                        qualified_name: qualified_name.clone(),
                        kind: DefinedSymbolKind::Trait,
                        visibility: item.visibility,
                        source_path: source_path.clone(),
                        module_path: unit.module_path.clone(),
                        is_entry: unit.is_entry,
                        imports: unit.imports.clone(),
                        params: Vec::new(),
                        return_type: None,
                        related_types,
                        raw_call_order: Vec::new(),
                        resolved_callees: BTreeSet::new(),
                        resolved_callee_order: Vec::new(),
                        host_classes: BTreeSet::new(),
                        branch_kinds: BTreeSet::new(),
                        branch_count: 0,
                    },
                );
                simple_names
                    .entry(name.clone())
                    .or_default()
                    .push(qualified_name);
            }
            ItemKind::Impl {
                trait_ref,
                target,
                methods,
                ..
            } => {
                let target_name = target.describe();
                for method in methods {
                    let qualified_name = qualify_symbol_name(
                        unit.module_path.as_deref(),
                        &format!("{target_name}.{}", method.name),
                    );
                    let mut walk = SymbolWalk::default();
                    if let Some(trait_ref) = trait_ref {
                        collect_type_ref_names(trait_ref, &mut walk.related_types);
                    }
                    collect_type_ref_names(target, &mut walk.related_types);
                    collect_type_ref_names(&method.return_type, &mut walk.related_types);
                    for param in &method.params {
                        collect_type_ref_names(&param.ty, &mut walk.related_types);
                    }
                    collect_symbol_walk_for_block(&method.body, &mut walk);
                    let host_classes = walk
                        .raw_calls
                        .iter()
                        .filter_map(|call| host_boundary_class(call))
                        .map(str::to_string)
                        .collect::<BTreeSet<_>>();

                    definitions.insert(
                        qualified_name.clone(),
                        DefinedSymbol {
                            qualified_name: qualified_name.clone(),
                            kind: DefinedSymbolKind::Function,
                            visibility: item.visibility,
                            source_path: source_path.clone(),
                            module_path: unit.module_path.clone(),
                            is_entry: unit.is_entry,
                            imports: unit.imports.clone(),
                            params: method
                                .params
                                .iter()
                                .map(|param| SymbolParamData {
                                    name: param.name.clone(),
                                    ty: param.ty.describe(),
                                })
                                .collect(),
                            return_type: Some(method.return_type.describe()),
                            related_types: walk.related_types,
                            raw_call_order: walk.raw_call_order,
                            resolved_callees: BTreeSet::new(),
                            resolved_callee_order: Vec::new(),
                            host_classes,
                            branch_kinds: walk.branch_kinds,
                            branch_count: walk.branch_count,
                        },
                    );
                    simple_names
                        .entry(method.name.clone())
                        .or_default()
                        .push(qualified_name);
                }
            }
        }
    }

    let mut callers_by_symbol = BTreeMap::<String, BTreeSet<String>>::new();
    let symbol_names = definitions.keys().cloned().collect::<Vec<_>>();
    for symbol_name in symbol_names {
        let Some(definition) = definitions.get(&symbol_name).cloned() else {
            continue;
        };
        if definition.kind != DefinedSymbolKind::Function {
            continue;
        }

        let mut resolved_callees = BTreeSet::new();
        let mut resolved_callee_order = Vec::new();
        for call in &definition.raw_call_order {
            let Some(resolved) =
                resolve_symbol_reference(&definition, call, &definitions, &simple_names)
            else {
                continue;
            };
            if resolved_callees.insert(resolved.clone()) {
                resolved_callee_order.push(resolved);
            }
        }

        if let Some(symbol) = definitions.get_mut(&symbol_name) {
            symbol.resolved_callees = resolved_callees.clone();
            symbol.resolved_callee_order = resolved_callee_order.clone();
        }
        for callee in resolved_callees {
            callers_by_symbol
                .entry(callee)
                .or_default()
                .insert(symbol_name.clone());
        }
    }

    SymbolCatalog {
        definitions,
        simple_names,
        callers_by_symbol,
    }
}

fn qualify_symbol_name(module_path: Option<&str>, name: &str) -> String {
    match module_path {
        Some(module_path) => format!("{module_path}.{name}"),
        None => name.to_string(),
    }
}

fn collect_symbol_walk_for_block(block: &Block, walk: &mut SymbolWalk) {
    for statement in &block.statements {
        collect_symbol_walk_for_stmt(statement, walk);
    }
}

fn collect_symbol_walk_for_stmt(statement: &Stmt, walk: &mut SymbolWalk) {
    match &statement.kind {
        StmtKind::Let {
            ty, initializer, ..
        } => {
            collect_type_ref_names(ty, &mut walk.related_types);
            collect_symbol_walk_for_expr(initializer, walk);
        }
        StmtKind::Assign { target, value } => {
            collect_symbol_walk_for_expr(target, walk);
            collect_symbol_walk_for_expr(value, walk);
        }
        StmtKind::Expr { expr } => collect_symbol_walk_for_expr(expr, walk),
        StmtKind::Return { value } => {
            if let Some(value) = value.as_ref() {
                collect_symbol_walk_for_expr(value, walk);
            }
        }
        StmtKind::Break | StmtKind::Continue => {}
        StmtKind::Match { scrutinee, arms } => {
            record_branch_kind(walk, "match");
            collect_symbol_walk_for_expr(scrutinee, walk);
            for arm in arms {
                collect_symbol_walk_for_block(&arm.body, walk);
            }
        }
        StmtKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            record_branch_kind(walk, "if");
            collect_symbol_walk_for_expr(condition, walk);
            collect_symbol_walk_for_block(then_branch, walk);
            if let Some(else_branch) = else_branch.as_ref() {
                collect_symbol_walk_for_block(else_branch, walk);
            }
        }
        StmtKind::While { condition, body } => {
            record_branch_kind(walk, "while");
            collect_symbol_walk_for_expr(condition, walk);
            collect_symbol_walk_for_block(body, walk);
        }
        StmtKind::For {
            initializer,
            condition,
            step,
            body,
        } => {
            record_branch_kind(walk, "for");
            if let Some(initializer) = initializer.as_ref() {
                collect_symbol_walk_for_stmt(initializer, walk);
            }
            if let Some(condition) = condition.as_ref() {
                collect_symbol_walk_for_expr(condition, walk);
            }
            if let Some(step) = step.as_ref() {
                collect_symbol_walk_for_stmt(step, walk);
            }
            collect_symbol_walk_for_block(body, walk);
        }
        StmtKind::ForIn {
            binding,
            iterable,
            body,
        } => {
            record_branch_kind(walk, "for_in");
            collect_type_ref_names(&binding.ty, &mut walk.related_types);
            collect_symbol_walk_for_expr(iterable, walk);
            collect_symbol_walk_for_block(body, walk);
        }
        StmtKind::Block { block } => collect_symbol_walk_for_block(block, walk),
    }
}

fn collect_symbol_walk_for_expr(expression: &Expr, walk: &mut SymbolWalk) {
    match &expression.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => {
            collect_symbol_walk_for_expr(expr, walk)
        }
        ExprKind::Binary { left, right, .. } => {
            collect_symbol_walk_for_expr(left, walk);
            collect_symbol_walk_for_expr(right, walk);
        }
        ExprKind::Call { callee, arguments } => {
            if let Some(name) = callee.qualified_name() {
                if walk.raw_calls.insert(name.clone()) {
                    walk.raw_call_order.push(name);
                }
            }
            collect_symbol_walk_for_expr(callee, walk);
            for argument in arguments {
                collect_symbol_walk_for_expr(argument, walk);
            }
        }
        ExprKind::StructLiteral { name, fields } => {
            walk.related_types.insert(name.clone());
            for field in fields {
                collect_symbol_walk_for_expr(&field.value, walk);
            }
        }
        ExprKind::ArrayLiteral { elements } => {
            for element in elements {
                collect_symbol_walk_for_expr(element, walk);
            }
        }
        ExprKind::Block { statements, value } => {
            for statement in statements {
                collect_symbol_walk_for_stmt(statement, walk);
            }
            collect_symbol_walk_for_expr(value, walk);
        }
        ExprKind::Match { scrutinee, arms } => {
            record_branch_kind(walk, "match");
            collect_symbol_walk_for_expr(scrutinee, walk);
            for arm in arms {
                collect_symbol_walk_for_expr(&arm.value, walk);
            }
        }
        ExprKind::Field { base, .. } => collect_symbol_walk_for_expr(base, walk),
        ExprKind::Index { base, index } => {
            collect_symbol_walk_for_expr(base, walk);
            collect_symbol_walk_for_expr(index, walk);
        }
        ExprKind::Slice { base, start, end } => {
            collect_symbol_walk_for_expr(base, walk);
            collect_symbol_walk_for_expr(start, walk);
            collect_symbol_walk_for_expr(end, walk);
        }
        ExprKind::Int { .. }
        | ExprKind::Float { .. }
        | ExprKind::Bool { .. }
        | ExprKind::String { .. }
        | ExprKind::Name { .. }
        | ExprKind::Error => {}
    }
}

fn record_branch_kind(walk: &mut SymbolWalk, kind: &str) {
    walk.branch_kinds.insert(kind.to_string());
    walk.branch_count += 1;
}

fn collect_type_ref_names(ty: &crate::ast::TypeRef, related_types: &mut BTreeSet<String>) {
    if let Some(name) = ty.name.as_ref() {
        related_types.insert(name.clone());
    }
    if let Some(element) = ty.element.as_ref() {
        collect_type_ref_names(element, related_types);
    }
}

fn resolve_symbol_reference(
    definition: &DefinedSymbol,
    raw_call: &str,
    definitions: &BTreeMap<String, DefinedSymbol>,
    simple_names: &BTreeMap<String, Vec<String>>,
) -> Option<String> {
    if host_boundary_class(raw_call).is_some() {
        return None;
    }
    if definitions.contains_key(raw_call) {
        return Some(raw_call.to_string());
    }
    if let Some(module_path) = definition.module_path.as_deref() {
        let candidate = format!("{module_path}.{raw_call}");
        if definitions.contains_key(&candidate) {
            return Some(candidate);
        }
    }
    for import in &definition.imports {
        let candidate = format!("{import}.{raw_call}");
        if definitions.contains_key(&candidate) {
            return Some(candidate);
        }
    }
    if let Some(matches) = simple_names.get(raw_call) {
        if matches.len() == 1 {
            return matches.first().cloned();
        }
    }
    None
}

pub(super) fn resolve_symbol_query(
    symbol_catalog: &SymbolCatalog,
    query: &str,
) -> Result<String, String> {
    if symbol_catalog.definitions.contains_key(query) {
        return Ok(query.to_string());
    }

    let Some(matches) = symbol_catalog.simple_names.get(query) else {
        return Err(format!("unknown symbol `{query}`"));
    };

    if matches.len() == 1 {
        return Ok(matches[0].clone());
    }

    Err(format!(
        "symbol `{query}` is ambiguous; candidates: {}",
        matches.join(", ")
    ))
}
