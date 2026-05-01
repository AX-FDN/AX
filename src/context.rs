use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::ast::{Block, Expr, ExprKind, ItemKind, Program, Stmt, StmtKind, Visibility};
use crate::diagnostics::Diagnostic;
use crate::lockfile::check_lockfile;
use crate::package_diagnostics::package_repair_hint;
use crate::project::{Project, ResolvedInput};
use crate::source::SourceFile;

mod boundaries;
mod evidence;
mod flow;
mod impact;
mod overview;
mod symbol;
mod topology;
mod types;

use self::boundaries::{build_boundaries_facts, build_boundaries_hints};
use self::evidence::{build_evidence_facts, build_evidence_hints};
use self::flow::{build_flow_facts, build_flow_hints};
use self::impact::{build_impact_facts, build_impact_hints};
use self::overview::{build_overview_facts, build_overview_hints};
use self::symbol::{build_symbol_facts, build_symbol_hints};
use self::topology::{build_topology_facts, build_topology_hints};
use self::types::*;

const CONTEXT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextView {
    Overview,
    Boundaries,
    Topology,
    Flow,
    Symbol,
    Impact,
    Evidence,
}

impl ContextView {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Boundaries => "boundaries",
            Self::Topology => "topology",
            Self::Flow => "flow",
            Self::Symbol => "symbol",
            Self::Impact => "impact",
            Self::Evidence => "evidence",
        }
    }
}

pub fn render_context_json(
    view: ContextView,
    request_path: &Path,
    input: &ResolvedInput,
    program: &Program,
    diagnostics: &[Diagnostic],
    requested_symbol: Option<&str>,
) -> Result<String, String> {
    let units = collect_source_units(input, program);
    let unit_stats = collect_unit_stats(&input.source, program);
    let symbol_catalog = build_symbol_catalog(&input.source, program, &units);
    let command_target = match input.project.as_ref() {
        Some(project) => normalize_path(project.root_dir()),
        None => normalize_path(request_path),
    };

    let rendered = match view {
        ContextView::Overview => serde_json::to_string_pretty(&ContextDocument {
            schema_version: CONTEXT_SCHEMA_VERSION,
            view: view.as_str(),
            subject: build_subject(request_path, input, None),
            facts: build_overview_facts(input.project.as_ref(), &units, &unit_stats, diagnostics),
            hints: build_overview_hints(&units, &unit_stats),
            validation: build_validation(
                diagnostics,
                &command_target,
                ContextView::Overview,
                vec![
                    "overview facts come from parsed source units and top-level items".to_string(),
                ],
            ),
        }),
        ContextView::Boundaries => serde_json::to_string_pretty(&ContextDocument {
            schema_version: CONTEXT_SCHEMA_VERSION,
            view: view.as_str(),
            subject: build_subject(request_path, input, None),
            facts: build_boundaries_facts(&units, &unit_stats),
            hints: build_boundaries_hints(&units, &unit_stats),
            validation: build_validation(
                diagnostics,
                &command_target,
                ContextView::Boundaries,
                vec![
                    "P0 host-boundary classification tracks builtin calls only".to_string(),
                    "path and string helpers stay outside host-boundary classes in this view"
                        .to_string(),
                ],
            ),
        }),
        ContextView::Topology => serde_json::to_string_pretty(&ContextDocument {
            schema_version: CONTEXT_SCHEMA_VERSION,
            view: view.as_str(),
            subject: build_subject(request_path, input, None),
            facts: build_topology_facts(
                input.project.as_ref(),
                &units,
                &unit_stats,
                &symbol_catalog,
            ),
            hints: build_topology_hints(&units, &unit_stats, &symbol_catalog),
            validation: build_validation(
                diagnostics,
                &command_target,
                ContextView::Topology,
                vec![
                    "P1 topology tracks source units, imports, and resolved top-level call edges"
                        .to_string(),
                ],
            ),
        }),
        ContextView::Flow => serde_json::to_string_pretty(&ContextDocument {
            schema_version: CONTEXT_SCHEMA_VERSION,
            view: view.as_str(),
            subject: build_subject(request_path, input, None),
            facts: build_flow_facts(&symbol_catalog),
            hints: build_flow_hints(&symbol_catalog),
            validation: build_validation(
                diagnostics,
                &command_target,
                ContextView::Flow,
                vec![
                    "P2 flow tracks entry reachability, direct call order, branch points, and recursion"
                        .to_string(),
                ],
            ),
        }),
        ContextView::Symbol => {
            let requested_symbol = requested_symbol
                .ok_or_else(|| "symbol view requires a symbol query".to_string())?;
            let symbol_facts = build_symbol_facts(requested_symbol, &symbol_catalog)?;
            serde_json::to_string_pretty(&ContextDocument {
                schema_version: CONTEXT_SCHEMA_VERSION,
                view: view.as_str(),
                subject: build_subject(
                    request_path,
                    input,
                    Some(symbol_facts.resolved_symbol.clone()),
                ),
                hints: build_symbol_hints(&symbol_facts, &symbol_catalog),
                facts: symbol_facts,
                validation: build_validation(
                    diagnostics,
                    &command_target,
                    ContextView::Symbol,
                    vec![
                        "P1 symbol view resolves one top-level symbol plus direct call neighbors"
                            .to_string(),
                    ],
                ),
            })
        }
        ContextView::Impact => {
            let requested_symbol = requested_symbol
                .ok_or_else(|| "impact view requires a symbol query".to_string())?;
            let impact_facts = build_impact_facts(requested_symbol, &symbol_catalog)?;
            serde_json::to_string_pretty(&ContextDocument {
                schema_version: CONTEXT_SCHEMA_VERSION,
                view: view.as_str(),
                subject: build_subject(
                    request_path,
                    input,
                    Some(impact_facts.resolved_symbol.clone()),
                ),
                hints: build_impact_hints(&impact_facts),
                facts: impact_facts,
                validation: build_validation(
                    diagnostics,
                    &command_target,
                    ContextView::Impact,
                    vec![
                        "P2 impact maps upstream callers, downstream callees, affected units, and change risk"
                            .to_string(),
                    ],
                ),
            })
        }
        ContextView::Evidence => {
            let requested_symbol = requested_symbol
                .ok_or_else(|| "evidence view requires a symbol query".to_string())?;
            let evidence_facts = build_evidence_facts(
                requested_symbol,
                request_path,
                input,
                program,
                &symbol_catalog,
            )?;
            serde_json::to_string_pretty(&ContextDocument {
                schema_version: CONTEXT_SCHEMA_VERSION,
                view: view.as_str(),
                subject: build_subject(
                    request_path,
                    input,
                    Some(evidence_facts.resolved_symbol.clone()),
                ),
                hints: build_evidence_hints(
                    &command_target,
                    input,
                    &evidence_facts.related_tests,
                    &evidence_facts.resolved_symbol,
                    &evidence_facts.expected_artifacts,
                ),
                facts: evidence_facts,
                validation: build_validation(
                    diagnostics,
                    &command_target,
                    ContextView::Evidence,
                    vec![
                        "P2 evidence maps the nearest examples, tests, docs, benchmarks, and expected artifacts".to_string(),
                    ],
                ),
            })
        }
    }
    .map(|rendered| rendered + "\n")
    .expect("context json should serialize");

    Ok(rendered)
}

#[derive(Debug, Clone)]
struct ResolvedUnit {
    path: String,
    module_path: Option<String>,
    is_entry: bool,
    imports: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct UnitStats {
    function_count: usize,
    struct_count: usize,
    enum_count: usize,
    function_names: Vec<String>,
    symbols: Vec<String>,
    host_classes: BTreeSet<String>,
    host_builtins: BTreeSet<String>,
    host_call_count: usize,
    filesystem_write_builtins: BTreeSet<String>,
}

impl UnitStats {
    fn type_count(&self) -> usize {
        self.struct_count + self.enum_count
    }
}

#[derive(Debug, Clone)]
struct SymbolCatalog {
    definitions: BTreeMap<String, DefinedSymbol>,
    simple_names: BTreeMap<String, Vec<String>>,
    callers_by_symbol: BTreeMap<String, BTreeSet<String>>,
}

#[derive(Debug, Clone)]
struct DefinedSymbol {
    qualified_name: String,
    kind: DefinedSymbolKind,
    visibility: Visibility,
    source_path: String,
    module_path: Option<String>,
    is_entry: bool,
    imports: Vec<String>,
    params: Vec<SymbolParamData>,
    return_type: Option<String>,
    related_types: BTreeSet<String>,
    raw_call_order: Vec<String>,
    resolved_callees: BTreeSet<String>,
    resolved_callee_order: Vec<String>,
    host_classes: BTreeSet<String>,
    branch_kinds: BTreeSet<String>,
    branch_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DefinedSymbolKind {
    Function,
    Const,
    TypeAlias,
    Struct,
    Enum,
    Trait,
}

impl DefinedSymbolKind {
    fn as_str(self) -> &'static str {
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
struct SymbolParamData {
    name: String,
    ty: String,
}

#[derive(Debug, Clone, Default)]
struct SymbolWalk {
    raw_calls: BTreeSet<String>,
    raw_call_order: Vec<String>,
    related_types: BTreeSet<String>,
    branch_kinds: BTreeSet<String>,
    branch_count: usize,
}

fn build_subject(
    request_path: &Path,
    input: &ResolvedInput,
    symbol: Option<String>,
) -> ContextSubject {
    match input.project.as_ref() {
        Some(project) => ContextSubject {
            kind: "project",
            path: normalize_path(project.root_dir()),
            entry: normalize_path(project.entry_path()),
            project_name: Some(project.target_name().to_string()),
            symbol,
        },
        None => ContextSubject {
            kind: "source",
            path: normalize_path(request_path),
            entry: normalize_path(input.source.path()),
            project_name: None,
            symbol,
        },
    }
}

fn build_local_path_package_facts(
    project: Option<&Project>,
    units: &[ResolvedUnit],
) -> Vec<ContextPathPackage> {
    let Some(project) = project else {
        return Vec::new();
    };

    project
        .local_path_dependencies()
        .iter()
        .map(|dependency| {
            let prefix = format!("{}.", dependency.alias());
            let modules = units
                .iter()
                .filter_map(|unit| unit.module_path.as_ref())
                .filter(|module_path| {
                    module_path.as_str() == dependency.alias()
                        || module_path.starts_with(prefix.as_str())
                })
                .cloned()
                .collect::<Vec<_>>();
            ContextPathPackage {
                alias: dependency.alias().to_string(),
                root: normalize_path(dependency.root_dir()),
                manifest: normalize_path(dependency.manifest_path()),
                source_count: dependency.source_paths().len(),
                modules,
            }
        })
        .collect()
}

fn build_local_package_lock_fact(project: Option<&Project>) -> Option<ContextPackageLock> {
    let project = project?;
    let dependency_count = project.local_path_dependencies().len();
    if dependency_count == 0 {
        return None;
    }

    let report = check_lockfile(project);
    Some(ContextPackageLock {
        path: normalize_path(&report.path),
        schema_version: 1,
        status: report.status.as_str(),
        dependency_count,
        note: report.note,
        issues: report
            .issues
            .into_iter()
            .map(|issue| ContextPackageLockIssue {
                repair_rule: package_repair_hint(issue.code).map(|hint| hint.rule_id),
                repair_goal: package_repair_hint(issue.code).map(|hint| hint.repair_goal),
                code: issue.code,
                kind: issue.kind,
                message: issue.message,
                fixit: issue.fixit,
            })
            .collect(),
    })
}

fn symbol_reaches_target(
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

fn build_symbol_catalog(
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

fn resolve_symbol_query(symbol_catalog: &SymbolCatalog, query: &str) -> Result<String, String> {
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

fn build_validation(
    diagnostics: &[Diagnostic],
    command_target: &str,
    view: ContextView,
    mut notes: Vec<String>,
) -> ContextValidation {
    if diagnostics.is_empty() {
        notes.push("context built from a clean diagnostic pass".to_string());
    } else {
        notes.push(format!(
            "context built from a partial program with {} diagnostic(s)",
            diagnostics.len()
        ));
    }

    let recommended_commands = match view {
        ContextView::Overview => vec![
            format!("axc check {command_target}"),
            format!("axc context overview {command_target} --json"),
            format!("axc context boundaries {command_target} --json"),
        ],
        ContextView::Boundaries => vec![
            format!("axc check {command_target}"),
            format!("axc context boundaries {command_target} --json"),
            format!("axc run {command_target}"),
        ],
        ContextView::Topology => vec![
            format!("axc check {command_target}"),
            format!("axc context topology {command_target} --json"),
            format!("axc context symbol {command_target} <symbol> --json"),
        ],
        ContextView::Flow => vec![
            format!("axc check {command_target}"),
            format!("axc context flow {command_target} --json"),
            format!("axc context symbol {command_target} <symbol> --json"),
        ],
        ContextView::Symbol => vec![
            format!("axc check {command_target}"),
            format!("axc context topology {command_target} --json"),
            format!("axc context symbol {command_target} <symbol> --json"),
        ],
        ContextView::Impact => vec![
            format!("axc check {command_target}"),
            format!("axc context flow {command_target} --json"),
            format!("axc context impact {command_target} <symbol> --json"),
        ],
        ContextView::Evidence => vec![
            format!("axc check {command_target}"),
            format!("axc context impact {command_target} <symbol> --json"),
            format!("axc context evidence {command_target} <symbol> --json"),
        ],
    };

    ContextValidation {
        diagnostic_count: diagnostics.len(),
        partial: !diagnostics.is_empty(),
        recommended_commands,
        notes,
    }
}

fn collect_source_units(input: &ResolvedInput, program: &Program) -> Vec<ResolvedUnit> {
    let entry_path = input
        .project
        .as_ref()
        .map(|project| normalize_path(project.entry_path()))
        .unwrap_or_else(|| normalize_path(input.source.path()));

    if program.source_units.is_empty() {
        let segments = input.source.segments();
        return segments
            .into_iter()
            .map(|segment| ResolvedUnit {
                path: normalize_path_text(&segment.path),
                module_path: None,
                is_entry: normalize_path_text(&segment.path) == entry_path,
                imports: Vec::new(),
            })
            .collect();
    }

    program
        .source_units
        .iter()
        .map(|unit| ResolvedUnit {
            path: normalize_path_text(&unit.path),
            module_path: resolve_module_path(input.project.as_ref(), unit),
            is_entry: unit.is_entry || normalize_path_text(&unit.path) == entry_path,
            imports: unit
                .imports
                .iter()
                .map(|import| import.path.clone())
                .collect(),
        })
        .collect()
}

fn resolve_module_path(project: Option<&Project>, unit: &crate::ast::SourceUnit) -> Option<String> {
    unit.module
        .as_ref()
        .map(|module| module.path.clone())
        .or_else(|| {
            project.and_then(|project| {
                project
                    .expected_module_path(Path::new(&unit.path))
                    .map(str::to_string)
            })
        })
}

fn collect_unit_stats(source: &SourceFile, program: &Program) -> BTreeMap<String, UnitStats> {
    let mut unit_stats = BTreeMap::<String, UnitStats>::new();

    for item in &program.items {
        let path = normalize_path_text(source.display_path_for_offset(item.span.start));
        let stats = unit_stats.entry(path).or_default();

        match &item.kind {
            ItemKind::Function { name, body, .. } => {
                stats.function_count += 1;
                stats.function_names.push(name.clone());
                stats.symbols.push(name.clone());
                visit_block(body, stats);
            }
            ItemKind::Const { name, .. } => {
                stats.symbols.push(name.clone());
            }
            ItemKind::TypeAlias { name, .. } => {
                stats.symbols.push(name.clone());
            }
            ItemKind::Struct { name, .. } => {
                stats.struct_count += 1;
                stats.symbols.push(name.clone());
            }
            ItemKind::Enum { name, .. } => {
                stats.enum_count += 1;
                stats.symbols.push(name.clone());
            }
            ItemKind::Trait { name, .. } => {
                stats.symbols.push(name.clone());
            }
            ItemKind::Impl { methods, .. } => {
                for method in methods {
                    stats.function_count += 1;
                    stats.function_names.push(method.name.clone());
                    stats.symbols.push(method.name.clone());
                    visit_block(&method.body, stats);
                }
            }
        }
    }

    unit_stats
}

fn visit_block(block: &Block, stats: &mut UnitStats) {
    for statement in &block.statements {
        visit_stmt(statement, stats);
    }
}

fn visit_stmt(statement: &Stmt, stats: &mut UnitStats) {
    match &statement.kind {
        StmtKind::Let { initializer, .. } => visit_expr(initializer, stats),
        StmtKind::Assign { target, value } => {
            visit_expr(target, stats);
            visit_expr(value, stats);
        }
        StmtKind::Expr { expr } => visit_expr(expr, stats),
        StmtKind::Return { value } => {
            if let Some(value) = value.as_ref() {
                visit_expr(value, stats);
            }
        }
        StmtKind::Break | StmtKind::Continue => {}
        StmtKind::Match { scrutinee, arms } => {
            visit_expr(scrutinee, stats);
            for arm in arms {
                visit_block(&arm.body, stats);
            }
        }
        StmtKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            visit_expr(condition, stats);
            visit_block(then_branch, stats);
            if let Some(else_branch) = else_branch.as_ref() {
                visit_block(else_branch, stats);
            }
        }
        StmtKind::While { condition, body } => {
            visit_expr(condition, stats);
            visit_block(body, stats);
        }
        StmtKind::For {
            initializer,
            condition,
            step,
            body,
        } => {
            if let Some(initializer) = initializer.as_ref() {
                visit_stmt(initializer, stats);
            }
            if let Some(condition) = condition.as_ref() {
                visit_expr(condition, stats);
            }
            if let Some(step) = step.as_ref() {
                visit_stmt(step, stats);
            }
            visit_block(body, stats);
        }
        StmtKind::ForIn { iterable, body, .. } => {
            visit_expr(iterable, stats);
            visit_block(body, stats);
        }
        StmtKind::Block { block } => visit_block(block, stats),
    }
}

fn visit_expr(expression: &Expr, stats: &mut UnitStats) {
    match &expression.kind {
        ExprKind::Unary { expr, .. } | ExprKind::Try { expr } => visit_expr(expr, stats),
        ExprKind::Binary { left, right, .. } => {
            visit_expr(left, stats);
            visit_expr(right, stats);
        }
        ExprKind::Call { callee, arguments } => {
            if let Some(name) = callee.qualified_name() {
                register_host_builtin(stats, &name);
            }
            visit_expr(callee, stats);
            for argument in arguments {
                visit_expr(argument, stats);
            }
        }
        ExprKind::StructLiteral { fields, .. } => {
            for field in fields {
                visit_expr(&field.value, stats);
            }
        }
        ExprKind::ArrayLiteral { elements } => {
            for element in elements {
                visit_expr(element, stats);
            }
        }
        ExprKind::Block { statements, value } => {
            for statement in statements {
                visit_stmt(statement, stats);
            }
            visit_expr(value, stats);
        }
        ExprKind::Match { scrutinee, arms } => {
            visit_expr(scrutinee, stats);
            for arm in arms {
                visit_expr(&arm.value, stats);
            }
        }
        ExprKind::Field { base, .. } => visit_expr(base, stats),
        ExprKind::Index { base, index } => {
            visit_expr(base, stats);
            visit_expr(index, stats);
        }
        ExprKind::Slice { base, start, end } => {
            visit_expr(base, stats);
            visit_expr(start, stats);
            visit_expr(end, stats);
        }
        ExprKind::Int { .. }
        | ExprKind::Float { .. }
        | ExprKind::Bool { .. }
        | ExprKind::String { .. }
        | ExprKind::Name { .. }
        | ExprKind::Error => {}
    }
}

fn register_host_builtin(stats: &mut UnitStats, name: &str) {
    let Some(class) = host_boundary_class(name) else {
        return;
    };

    stats.host_classes.insert(class.to_string());
    stats.host_builtins.insert(name.to_string());
    stats.host_call_count += 1;

    if is_filesystem_write_builtin(name) {
        stats.filesystem_write_builtins.insert(name.to_string());
    }
}

fn host_boundary_class(name: &str) -> Option<&'static str> {
    match name {
        "argv_len" | "argv_get" => Some("argv"),
        "env_has" | "env_get" => Some("env"),
        "process_cwd" | "process_run" | "process_capture" | "process_run_in"
        | "process_capture_in" => Some("process"),
        "fs_is_file" | "fs_is_dir" | "fs_exists" | "fs_file_size" | "fs_copy_file"
        | "fs_rename" | "fs_create_dir_all" | "fs_remove_file" | "fs_remove_dir_all"
        | "fs_read_dir" | "fs_read_to_string" | "fs_write_string" => Some("filesystem"),
        "println" => Some("stdout"),
        _ => None,
    }
}

fn is_filesystem_write_builtin(name: &str) -> bool {
    matches!(
        name,
        "fs_copy_file"
            | "fs_rename"
            | "fs_create_dir_all"
            | "fs_remove_file"
            | "fs_remove_dir_all"
            | "fs_write_string"
    )
}

fn is_host_heavy(stats: &UnitStats) -> bool {
    stats.host_classes.len() >= 2
        || !stats.filesystem_write_builtins.is_empty()
        || stats.host_call_count >= 3
        || stats.host_classes.contains("process")
}

fn host_heavy_reason(stats: &UnitStats) -> String {
    if !stats.filesystem_write_builtins.is_empty() {
        return "touches filesystem mutation builtins".to_string();
    }
    if stats.host_classes.contains("process") {
        return "crosses the process execution boundary".to_string();
    }
    if stats.host_classes.len() >= 2 {
        return "mixes multiple host capability classes".to_string();
    }
    if stats.host_classes.contains("filesystem") && stats.host_call_count >= 2 {
        return "concentrates repeated filesystem access".to_string();
    }
    "crosses the host boundary".to_string()
}

fn push_unique(
    output: &mut Vec<String>,
    seen: &mut BTreeSet<String>,
    values: &[String],
    limit: usize,
) {
    for value in values {
        if output.len() >= limit {
            break;
        }
        if seen.insert(value.clone()) {
            output.push(value.clone());
        }
    }
}

fn normalize_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

fn normalize_path_text(path: &str) -> String {
    path.replace('\\', "/")
}
