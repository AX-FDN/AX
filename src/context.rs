use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::ast::{Block, Expr, ExprKind, ItemKind, Program, Stmt, StmtKind, Visibility};
use crate::diagnostics::Diagnostic;
use crate::project::{Project, ResolvedInput};
use crate::source::SourceFile;

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
            facts: build_topology_facts(&units, &unit_stats, &symbol_catalog),
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
            let evidence_facts =
                build_evidence_facts(requested_symbol, request_path, input, &symbol_catalog)?;
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

#[derive(Serialize)]
struct ContextDocument<Facts, Hints> {
    schema_version: u32,
    view: &'static str,
    subject: ContextSubject,
    facts: Facts,
    hints: Hints,
    validation: ContextValidation,
}

#[derive(Serialize)]
struct ContextSubject {
    kind: &'static str,
    path: String,
    entry: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    symbol: Option<String>,
}

#[derive(Serialize)]
struct ContextValidation {
    diagnostic_count: usize,
    partial: bool,
    recommended_commands: Vec<String>,
    notes: Vec<String>,
}

#[derive(Serialize)]
struct OverviewFacts {
    #[serde(skip_serializing_if = "Option::is_none")]
    project_name: Option<String>,
    entry: String,
    module_mode: bool,
    source_roots: Vec<String>,
    summary: OverviewSummary,
    source_units: Vec<OverviewUnit>,
}

#[derive(Serialize)]
struct OverviewSummary {
    source_unit_count: usize,
    support_unit_count: usize,
    module_count: usize,
    import_count: usize,
    function_count: usize,
    struct_count: usize,
    enum_count: usize,
    type_count: usize,
    diagnostic_count: usize,
}

#[derive(Serialize)]
struct OverviewUnit {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    module_path: Option<String>,
    is_entry: bool,
    imports: Vec<String>,
    function_count: usize,
    type_count: usize,
}

#[derive(Serialize)]
struct OverviewHints {
    entrypoints: Vec<String>,
    support_modules: Vec<String>,
    core_symbols: Vec<String>,
}

#[derive(Serialize)]
struct BoundariesFacts {
    host_boundary_classes: Vec<String>,
    unit_boundary_usage: Vec<UnitBoundaryUsage>,
}

#[derive(Serialize)]
struct UnitBoundaryUsage {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    module_path: Option<String>,
    is_entry: bool,
    function_count: usize,
    type_count: usize,
    host_classes: Vec<String>,
    host_builtins: Vec<String>,
    host_call_count: usize,
    filesystem_write_builtins: Vec<String>,
}

#[derive(Serialize)]
struct BoundariesHints {
    host_heavy_units: Vec<HostHeavyUnitHint>,
    safe_logic_units: Vec<SafeLogicUnitHint>,
    constraint_candidates: Vec<ConstraintCandidate>,
}

#[derive(Serialize)]
struct TopologyFacts {
    module_mode: bool,
    summary: TopologySummary,
    source_units: Vec<TopologyUnit>,
    module_edges: Vec<ModuleEdge>,
    symbol_edges: Vec<SymbolEdge>,
}

#[derive(Serialize)]
struct TopologySummary {
    source_unit_count: usize,
    module_edge_count: usize,
    symbol_count: usize,
    symbol_edge_count: usize,
}

#[derive(Serialize)]
struct TopologyUnit {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    module_path: Option<String>,
    is_entry: bool,
    imports: Vec<String>,
    imported_by_count: usize,
    defined_symbols: Vec<String>,
    host_classes: Vec<String>,
    role_hints: Vec<String>,
    role_evidence: Vec<String>,
}

#[derive(Serialize)]
struct ModuleEdge {
    from_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    from_module: Option<String>,
    to_module: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    to_path: Option<String>,
    kind: &'static str,
    resolved: bool,
}

#[derive(Serialize)]
struct SymbolEdge {
    from: String,
    to: String,
    kind: &'static str,
    cross_unit: bool,
}

#[derive(Serialize)]
struct TopologyHints {
    entry_orchestrators: Vec<String>,
    shared_foundations: Vec<String>,
    central_symbols: Vec<String>,
}

#[derive(Serialize)]
struct FlowFacts {
    #[serde(skip_serializing_if = "Option::is_none")]
    entry_symbol: Option<String>,
    summary: FlowSummary,
    top_level_calls: Vec<String>,
    reachable_symbols: Vec<FlowReachableSymbol>,
    flow_edges: Vec<FlowEdge>,
    branch_points: Vec<FlowBranchPoint>,
    recursive_symbols: Vec<String>,
}

#[derive(Serialize)]
struct FlowSummary {
    reachable_symbol_count: usize,
    flow_edge_count: usize,
    branch_point_count: usize,
    recursive_symbol_count: usize,
    max_depth: usize,
}

#[derive(Serialize)]
struct FlowReachableSymbol {
    symbol: String,
    depth: usize,
    source_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    module_path: Option<String>,
    host_boundary_classes: Vec<String>,
    branch_count: usize,
}

#[derive(Serialize)]
struct FlowEdge {
    from: String,
    to: String,
    target_depth: usize,
    cross_unit: bool,
}

#[derive(Serialize)]
struct FlowBranchPoint {
    symbol: String,
    branch_kinds: Vec<String>,
    branch_count: usize,
    note: String,
}

#[derive(Serialize)]
struct FlowHints {
    orchestration_chain: Vec<String>,
    host_boundary_symbols: Vec<String>,
    leaf_symbols: Vec<String>,
}

#[derive(Serialize)]
struct ImpactFacts {
    requested_symbol: String,
    resolved_symbol: String,
    direct_callers: Vec<String>,
    direct_callees: Vec<String>,
    upstream_callers: Vec<String>,
    downstream_callees: Vec<String>,
    affected_units: Vec<ImpactUnit>,
    recursive: bool,
    change_risk: ImpactRisk,
}

#[derive(Serialize)]
struct ImpactUnit {
    path: String,
    symbol_count: usize,
    includes_target: bool,
    host_boundary_classes: Vec<String>,
}

#[derive(Serialize)]
struct ImpactRisk {
    level: &'static str,
    reasons: Vec<String>,
}

#[derive(Serialize)]
struct ImpactHints {
    smallest_safe_edit_scope: Vec<String>,
    likely_breakages: Vec<String>,
    regression_targets: Vec<String>,
}

#[derive(Serialize)]
struct EvidenceFacts {
    requested_symbol: String,
    resolved_symbol: String,
    affected_units: Vec<String>,
    related_examples: Vec<String>,
    related_tests: Vec<String>,
    related_docs: Vec<String>,
    related_benchmarks: Vec<String>,
    expected_artifacts: Vec<String>,
}

#[derive(Serialize)]
struct EvidenceHints {
    recommended_commands: Vec<String>,
    expected_artifacts: Vec<String>,
}

#[derive(Serialize)]
struct SymbolFacts {
    requested_symbol: String,
    resolved_symbol: String,
    kind: &'static str,
    #[serde(default, skip_serializing_if = "Visibility::is_private")]
    visibility: Visibility,
    source_unit: SymbolSourceUnit,
    signature: SymbolSignature,
    callers: Vec<String>,
    callees: Vec<String>,
    related_types: Vec<String>,
    host_boundary_classes: Vec<String>,
}

#[derive(Serialize)]
struct SymbolSourceUnit {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    module_path: Option<String>,
    is_entry: bool,
    imports: Vec<String>,
}

#[derive(Serialize)]
struct SymbolSignature {
    params: Vec<SymbolParamView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    return_type: Option<String>,
}

#[derive(Serialize)]
struct SymbolParamView {
    name: String,
    ty: String,
}

#[derive(Serialize)]
struct SymbolHints {
    role_hints: Vec<String>,
    role_evidence: Vec<String>,
    adjacent_symbols: Vec<String>,
}

#[derive(Serialize)]
struct HostHeavyUnitHint {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    module_path: Option<String>,
    host_classes: Vec<String>,
    host_builtins: Vec<String>,
    reason: String,
}

#[derive(Serialize)]
struct SafeLogicUnitHint {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    module_path: Option<String>,
    function_count: usize,
    type_count: usize,
    reason: String,
}

#[derive(Serialize)]
struct ConstraintCandidate {
    kind: &'static str,
    targets: Vec<String>,
    reason: String,
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

fn build_overview_facts(
    project: Option<&Project>,
    units: &[ResolvedUnit],
    unit_stats: &BTreeMap<String, UnitStats>,
    diagnostics: &[Diagnostic],
) -> OverviewFacts {
    let module_paths = units
        .iter()
        .filter_map(|unit| unit.module_path.clone())
        .collect::<BTreeSet<_>>();
    let source_roots = units
        .iter()
        .filter_map(|unit| unit.module_path.as_deref())
        .filter_map(|module_path| module_path.split('.').next())
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let function_count = unit_stats.values().map(|stats| stats.function_count).sum();
    let struct_count = unit_stats.values().map(|stats| stats.struct_count).sum();
    let enum_count = unit_stats.values().map(|stats| stats.enum_count).sum();

    let source_units = units
        .iter()
        .map(|unit| {
            let stats = unit_stats.get(&unit.path).cloned().unwrap_or_default();
            OverviewUnit {
                path: unit.path.clone(),
                module_path: unit.module_path.clone(),
                is_entry: unit.is_entry,
                imports: unit.imports.clone(),
                function_count: stats.function_count,
                type_count: stats.type_count(),
            }
        })
        .collect::<Vec<_>>();

    OverviewFacts {
        project_name: project.map(|project| project.target_name().to_string()),
        entry: project
            .map(|project| normalize_path(project.entry_path()))
            .unwrap_or_else(|| {
                units
                    .first()
                    .map(|unit| unit.path.clone())
                    .unwrap_or_default()
            }),
        module_mode: units.len() > 1
            || units
                .iter()
                .any(|unit| unit.module_path.is_some() || !unit.imports.is_empty()),
        source_roots,
        summary: OverviewSummary {
            source_unit_count: units.len(),
            support_unit_count: units.iter().filter(|unit| !unit.is_entry).count(),
            module_count: module_paths.len(),
            import_count: units.iter().map(|unit| unit.imports.len()).sum(),
            function_count,
            struct_count,
            enum_count,
            type_count: struct_count + enum_count,
            diagnostic_count: diagnostics.len(),
        },
        source_units,
    }
}

fn build_overview_hints(
    units: &[ResolvedUnit],
    unit_stats: &BTreeMap<String, UnitStats>,
) -> OverviewHints {
    let mut support_modules = Vec::new();
    let mut seen_support_modules = BTreeSet::new();
    let mut core_symbols = Vec::new();
    let mut seen_symbols = BTreeSet::new();
    let mut entrypoints = Vec::new();

    for unit in units.iter().filter(|unit| unit.is_entry) {
        if let Some(stats) = unit_stats.get(&unit.path) {
            entrypoints.extend(stats.function_names.iter().cloned());
            push_unique(&mut core_symbols, &mut seen_symbols, &stats.symbols, 12);
        }
    }

    for unit in units.iter().filter(|unit| !unit.is_entry) {
        if let Some(module_path) = unit.module_path.as_ref() {
            if seen_support_modules.insert(module_path.clone()) {
                support_modules.push(module_path.clone());
            }
        }
        if let Some(stats) = unit_stats.get(&unit.path) {
            push_unique(&mut core_symbols, &mut seen_symbols, &stats.symbols, 12);
        }
    }

    OverviewHints {
        entrypoints,
        support_modules,
        core_symbols,
    }
}

fn build_boundaries_facts(
    units: &[ResolvedUnit],
    unit_stats: &BTreeMap<String, UnitStats>,
) -> BoundariesFacts {
    let mut host_boundary_classes = BTreeSet::new();
    let unit_boundary_usage = units
        .iter()
        .map(|unit| {
            let stats = unit_stats.get(&unit.path).cloned().unwrap_or_default();
            host_boundary_classes.extend(stats.host_classes.iter().cloned());
            UnitBoundaryUsage {
                path: unit.path.clone(),
                module_path: unit.module_path.clone(),
                is_entry: unit.is_entry,
                function_count: stats.function_count,
                type_count: stats.type_count(),
                host_classes: stats.host_classes.into_iter().collect(),
                host_builtins: stats.host_builtins.into_iter().collect(),
                host_call_count: stats.host_call_count,
                filesystem_write_builtins: stats.filesystem_write_builtins.into_iter().collect(),
            }
        })
        .collect::<Vec<_>>();

    BoundariesFacts {
        host_boundary_classes: host_boundary_classes.into_iter().collect(),
        unit_boundary_usage,
    }
}

fn build_boundaries_hints(
    units: &[ResolvedUnit],
    unit_stats: &BTreeMap<String, UnitStats>,
) -> BoundariesHints {
    let mut host_heavy_units = Vec::new();
    let mut safe_logic_units = Vec::new();

    for unit in units {
        let stats = unit_stats.get(&unit.path).cloned().unwrap_or_default();

        if is_host_heavy(&stats) {
            host_heavy_units.push(HostHeavyUnitHint {
                path: unit.path.clone(),
                module_path: unit.module_path.clone(),
                host_classes: stats.host_classes.iter().cloned().collect(),
                host_builtins: stats.host_builtins.iter().cloned().collect(),
                reason: host_heavy_reason(&stats),
            });
        }

        if stats.host_classes.is_empty() && (stats.function_count != 0 || stats.type_count() != 0) {
            safe_logic_units.push(SafeLogicUnitHint {
                path: unit.path.clone(),
                module_path: unit.module_path.clone(),
                function_count: stats.function_count,
                type_count: stats.type_count(),
                reason: "no argv/env/process/filesystem/stdout builtins observed".to_string(),
            });
        }
    }

    BoundariesHints {
        host_heavy_units,
        safe_logic_units,
        constraint_candidates: build_constraint_candidates(units, unit_stats),
    }
}

fn build_topology_facts(
    units: &[ResolvedUnit],
    unit_stats: &BTreeMap<String, UnitStats>,
    symbol_catalog: &SymbolCatalog,
) -> TopologyFacts {
    let module_path_to_unit = units
        .iter()
        .filter_map(|unit| {
            unit.module_path
                .as_ref()
                .map(|module_path| (module_path.clone(), unit.path.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let imported_by_count = collect_imported_by_counts(units);

    let source_units = units
        .iter()
        .map(|unit| {
            let stats = unit_stats.get(&unit.path).cloned().unwrap_or_default();
            let defined_symbols = symbol_catalog
                .definitions
                .values()
                .filter(|symbol| symbol.source_path == unit.path)
                .map(|symbol| symbol.qualified_name.clone())
                .collect::<Vec<_>>();
            let role_hints = unit_role_hints(
                unit,
                &stats,
                *imported_by_count.get(&unit.path).unwrap_or(&0),
            );
            let role_evidence = unit_role_evidence(
                unit,
                &stats,
                *imported_by_count.get(&unit.path).unwrap_or(&0),
            );

            TopologyUnit {
                path: unit.path.clone(),
                module_path: unit.module_path.clone(),
                is_entry: unit.is_entry,
                imports: unit.imports.clone(),
                imported_by_count: *imported_by_count.get(&unit.path).unwrap_or(&0),
                defined_symbols,
                host_classes: stats.host_classes.iter().cloned().collect(),
                role_hints,
                role_evidence,
            }
        })
        .collect::<Vec<_>>();

    let module_edges = units
        .iter()
        .flat_map(|unit| {
            unit.imports.iter().map(|import| ModuleEdge {
                from_path: unit.path.clone(),
                from_module: unit.module_path.clone(),
                to_module: import.clone(),
                to_path: module_path_to_unit.get(import).cloned(),
                kind: "import",
                resolved: module_path_to_unit.contains_key(import),
            })
        })
        .collect::<Vec<_>>();

    let symbol_edges = symbol_catalog
        .definitions
        .values()
        .filter(|symbol| symbol.kind == DefinedSymbolKind::Function)
        .flat_map(|symbol| {
            symbol.resolved_callees.iter().filter_map(|callee| {
                let target = symbol_catalog.definitions.get(callee)?;
                Some(SymbolEdge {
                    from: symbol.qualified_name.clone(),
                    to: callee.clone(),
                    kind: "call",
                    cross_unit: symbol.source_path != target.source_path,
                })
            })
        })
        .collect::<Vec<_>>();

    TopologyFacts {
        module_mode: units.len() > 1
            || units
                .iter()
                .any(|unit| unit.module_path.is_some() || !unit.imports.is_empty()),
        summary: TopologySummary {
            source_unit_count: units.len(),
            module_edge_count: module_edges.len(),
            symbol_count: symbol_catalog.definitions.len(),
            symbol_edge_count: symbol_edges.len(),
        },
        source_units,
        module_edges,
        symbol_edges,
    }
}

fn build_topology_hints(
    units: &[ResolvedUnit],
    unit_stats: &BTreeMap<String, UnitStats>,
    symbol_catalog: &SymbolCatalog,
) -> TopologyHints {
    let imported_by_count = collect_imported_by_counts(units);
    let entry_orchestrators = units
        .iter()
        .filter(|unit| unit.is_entry)
        .map(|unit| unit.path.clone())
        .collect::<Vec<_>>();
    let shared_foundations = units
        .iter()
        .filter(|unit| is_foundation_unit(unit))
        .filter_map(|unit| unit.module_path.clone())
        .collect::<Vec<_>>();
    let mut central_symbols = symbol_catalog
        .definitions
        .values()
        .map(|symbol| {
            let out_degree = symbol.resolved_callees.len();
            let in_degree = symbol_catalog
                .callers_by_symbol
                .get(&symbol.qualified_name)
                .map(BTreeSet::len)
                .unwrap_or(0);
            let unit_bonus = imported_by_count
                .get(&symbol.source_path)
                .copied()
                .unwrap_or(0);
            (
                symbol.qualified_name.clone(),
                out_degree + in_degree + unit_bonus,
            )
        })
        .collect::<Vec<_>>();
    central_symbols.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(&right.0)));
    let central_symbols = central_symbols
        .into_iter()
        .filter(|(_, score)| *score != 0)
        .take(8)
        .map(|(name, _)| name)
        .collect::<Vec<_>>();

    let _ = unit_stats;

    TopologyHints {
        entry_orchestrators,
        shared_foundations,
        central_symbols,
    }
}

fn build_flow_facts(symbol_catalog: &SymbolCatalog) -> FlowFacts {
    let Some(entry_symbol) = select_entry_symbol(symbol_catalog) else {
        return FlowFacts {
            entry_symbol: None,
            summary: FlowSummary {
                reachable_symbol_count: 0,
                flow_edge_count: 0,
                branch_point_count: 0,
                recursive_symbol_count: 0,
                max_depth: 0,
            },
            top_level_calls: Vec::new(),
            reachable_symbols: Vec::new(),
            flow_edges: Vec::new(),
            branch_points: Vec::new(),
            recursive_symbols: Vec::new(),
        };
    };

    let (reachable_order, depth_by_symbol) =
        collect_reachable_flow_symbols(symbol_catalog, &entry_symbol);
    let recursive_symbols = collect_recursive_symbols(symbol_catalog, &reachable_order);

    let top_level_calls = symbol_catalog
        .definitions
        .get(&entry_symbol)
        .map(|symbol| filter_reachable_callee_order(symbol, &depth_by_symbol))
        .unwrap_or_default();

    let reachable_symbols = reachable_order
        .iter()
        .filter_map(|symbol_name| {
            let symbol = symbol_catalog.definitions.get(symbol_name)?;
            Some(FlowReachableSymbol {
                symbol: symbol_name.clone(),
                depth: *depth_by_symbol.get(symbol_name).unwrap_or(&0),
                source_path: symbol.source_path.clone(),
                module_path: symbol.module_path.clone(),
                host_boundary_classes: symbol.host_classes.iter().cloned().collect(),
                branch_count: symbol.branch_count,
            })
        })
        .collect::<Vec<_>>();

    let flow_edges = reachable_order
        .iter()
        .flat_map(|symbol_name| {
            let Some(symbol) = symbol_catalog.definitions.get(symbol_name) else {
                return Vec::new();
            };
            filter_reachable_callee_order(symbol, &depth_by_symbol)
                .into_iter()
                .filter_map(|callee| {
                    let target = symbol_catalog.definitions.get(&callee)?;
                    Some(FlowEdge {
                        from: symbol_name.clone(),
                        to: callee.clone(),
                        target_depth: *depth_by_symbol.get(&callee).unwrap_or(&0),
                        cross_unit: symbol.source_path != target.source_path,
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let branch_points = reachable_order
        .iter()
        .filter_map(|symbol_name| {
            let symbol = symbol_catalog.definitions.get(symbol_name)?;
            if symbol.branch_count == 0 {
                return None;
            }
            let branch_kinds = symbol.branch_kinds.iter().cloned().collect::<Vec<_>>();
            Some(FlowBranchPoint {
                symbol: symbol_name.clone(),
                branch_kinds: branch_kinds.clone(),
                branch_count: symbol.branch_count,
                note: format!(
                    "contains {} control-flow branch site(s): {}",
                    symbol.branch_count,
                    branch_kinds.join(", ")
                ),
            })
        })
        .collect::<Vec<_>>();

    let max_depth = depth_by_symbol.values().copied().max().unwrap_or(0);

    FlowFacts {
        entry_symbol: Some(entry_symbol),
        summary: FlowSummary {
            reachable_symbol_count: reachable_order.len(),
            flow_edge_count: flow_edges.len(),
            branch_point_count: branch_points.len(),
            recursive_symbol_count: recursive_symbols.len(),
            max_depth,
        },
        top_level_calls,
        reachable_symbols,
        flow_edges,
        branch_points,
        recursive_symbols,
    }
}

fn build_flow_hints(symbol_catalog: &SymbolCatalog) -> FlowHints {
    let Some(entry_symbol) = select_entry_symbol(symbol_catalog) else {
        return FlowHints {
            orchestration_chain: Vec::new(),
            host_boundary_symbols: Vec::new(),
            leaf_symbols: Vec::new(),
        };
    };

    let (reachable_order, depth_by_symbol) =
        collect_reachable_flow_symbols(symbol_catalog, &entry_symbol);
    let reachable_set = reachable_order.iter().cloned().collect::<BTreeSet<_>>();

    let orchestration_chain =
        build_longest_flow_chain(symbol_catalog, &entry_symbol, &reachable_set);
    let host_boundary_symbols = reachable_order
        .iter()
        .filter_map(|symbol_name| {
            let symbol = symbol_catalog.definitions.get(symbol_name)?;
            if symbol.host_classes.is_empty() {
                return None;
            }
            Some(symbol_name.clone())
        })
        .collect::<Vec<_>>();
    let leaf_symbols = reachable_order
        .iter()
        .filter_map(|symbol_name| {
            let symbol = symbol_catalog.definitions.get(symbol_name)?;
            let reachable_callees = filter_reachable_callee_order(symbol, &depth_by_symbol);
            if reachable_callees.is_empty() {
                return Some(symbol_name.clone());
            }
            None
        })
        .collect::<Vec<_>>();

    FlowHints {
        orchestration_chain,
        host_boundary_symbols,
        leaf_symbols,
    }
}

fn build_symbol_facts(
    requested_symbol: &str,
    symbol_catalog: &SymbolCatalog,
) -> Result<SymbolFacts, String> {
    let resolved_symbol = resolve_symbol_query(symbol_catalog, requested_symbol)?;
    let symbol = symbol_catalog
        .definitions
        .get(&resolved_symbol)
        .ok_or_else(|| {
            format!("symbol `{resolved_symbol}` disappeared during context rendering")
        })?;

    let callers = symbol_catalog
        .callers_by_symbol
        .get(&symbol.qualified_name)
        .map(|callers| callers.iter().cloned().collect())
        .unwrap_or_default();
    let callees = symbol.resolved_callee_order.clone();
    let related_types = symbol.related_types.iter().cloned().collect::<Vec<_>>();
    let host_boundary_classes = symbol.host_classes.iter().cloned().collect::<Vec<_>>();

    Ok(SymbolFacts {
        requested_symbol: requested_symbol.to_string(),
        resolved_symbol: symbol.qualified_name.clone(),
        kind: symbol.kind.as_str(),
        visibility: symbol.visibility,
        source_unit: SymbolSourceUnit {
            path: symbol.source_path.clone(),
            module_path: symbol.module_path.clone(),
            is_entry: symbol.is_entry,
            imports: symbol.imports.clone(),
        },
        signature: SymbolSignature {
            params: symbol
                .params
                .iter()
                .map(|param| SymbolParamView {
                    name: param.name.clone(),
                    ty: param.ty.clone(),
                })
                .collect(),
            return_type: symbol.return_type.clone(),
        },
        callers,
        callees,
        related_types,
        host_boundary_classes,
    })
}

fn build_symbol_hints(symbol_facts: &SymbolFacts, symbol_catalog: &SymbolCatalog) -> SymbolHints {
    let mut role_hints = Vec::new();
    let mut role_evidence = Vec::new();

    if symbol_facts.source_unit.is_entry && symbol_facts.resolved_symbol == "main" {
        role_hints.push("entrypoint".to_string());
        role_evidence.push("declared in entry source unit as `main`".to_string());
    }

    if !symbol_facts.host_boundary_classes.is_empty() {
        role_hints.push("host_boundary_symbol".to_string());
        role_evidence.push(format!(
            "touches host classes: {}",
            symbol_facts.host_boundary_classes.join(", ")
        ));
    }

    if !symbol_facts.callers.is_empty() && symbol_facts.callers.len() >= 2 {
        role_hints.push("shared_helper".to_string());
        role_evidence.push(format!("called by {} symbols", symbol_facts.callers.len()));
    }

    if !symbol_facts.callees.is_empty() && symbol_facts.callers.is_empty() {
        role_hints.push("orchestrator".to_string());
        role_evidence.push("fans out to other symbols without incoming project calls".to_string());
    }

    if symbol_facts.callees.is_empty() {
        role_hints.push("leaf_symbol".to_string());
        role_evidence.push("does not call any resolved top-level project symbol".to_string());
    }

    if role_hints.is_empty() {
        role_hints.push("local_symbol".to_string());
        role_evidence.push("symbol currently has a narrow project interaction surface".to_string());
    }

    let mut adjacent_symbols = BTreeSet::new();
    for name in &symbol_facts.callers {
        adjacent_symbols.insert(name.clone());
    }
    for name in &symbol_facts.callees {
        adjacent_symbols.insert(name.clone());
    }

    let adjacent_symbols = adjacent_symbols
        .into_iter()
        .filter(|name| symbol_catalog.definitions.contains_key(name))
        .take(12)
        .collect::<Vec<_>>();

    SymbolHints {
        role_hints,
        role_evidence,
        adjacent_symbols,
    }
}

fn build_impact_facts(
    requested_symbol: &str,
    symbol_catalog: &SymbolCatalog,
) -> Result<ImpactFacts, String> {
    let resolved_symbol = resolve_symbol_query(symbol_catalog, requested_symbol)?;
    let symbol = symbol_catalog
        .definitions
        .get(&resolved_symbol)
        .ok_or_else(|| format!("symbol `{resolved_symbol}` disappeared during impact rendering"))?;

    let direct_callers = symbol_catalog
        .callers_by_symbol
        .get(&resolved_symbol)
        .map(|callers| callers.iter().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let direct_callees = symbol.resolved_callee_order.clone();
    let upstream_callers =
        collect_upstream_symbols(symbol_catalog, &resolved_symbol, &direct_callers);
    let downstream_callees =
        collect_downstream_symbols(symbol_catalog, &resolved_symbol, &direct_callees);
    let recursive = symbol_reaches_target(
        symbol_catalog,
        &resolved_symbol,
        &resolved_symbol,
        &symbol_catalog
            .definitions
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        &mut BTreeSet::new(),
    );
    let affected_units = build_affected_units(
        symbol_catalog,
        &resolved_symbol,
        &upstream_callers,
        &downstream_callees,
    );
    let change_risk = build_impact_risk(
        symbol,
        &direct_callers,
        &direct_callees,
        &upstream_callers,
        &downstream_callees,
        &affected_units,
        recursive,
    );

    Ok(ImpactFacts {
        requested_symbol: requested_symbol.to_string(),
        resolved_symbol,
        direct_callers,
        direct_callees,
        upstream_callers,
        downstream_callees,
        affected_units,
        recursive,
        change_risk,
    })
}

fn build_impact_hints(impact_facts: &ImpactFacts) -> ImpactHints {
    let mut smallest_safe_edit_scope = impact_facts
        .affected_units
        .iter()
        .filter(|unit| unit.includes_target)
        .map(|unit| unit.path.clone())
        .collect::<Vec<_>>();

    if smallest_safe_edit_scope.is_empty() {
        smallest_safe_edit_scope = impact_facts
            .affected_units
            .iter()
            .take(1)
            .map(|unit| unit.path.clone())
            .collect();
    }

    let mut likely_breakages = Vec::new();
    if !impact_facts.direct_callers.is_empty() {
        likely_breakages.push(format!(
            "call-site expectations may shift across {} direct caller(s)",
            impact_facts.direct_callers.len()
        ));
    }
    if !impact_facts.direct_callees.is_empty() {
        likely_breakages.push(format!(
            "downstream behavior may drift across {} direct callee(s)",
            impact_facts.direct_callees.len()
        ));
    }
    if impact_facts.recursive {
        likely_breakages.push(
            "recursive behavior may affect traversal completeness or termination".to_string(),
        );
    }
    if impact_facts
        .affected_units
        .iter()
        .any(|unit| !unit.host_boundary_classes.is_empty())
    {
        likely_breakages.push("host-boundary behavior may change across touched units".to_string());
    }

    let mut regression_targets = vec![
        "axc check <path>".to_string(),
        "axc context flow <path> --json".to_string(),
        format!(
            "axc context symbol <path> {} --json",
            impact_facts.resolved_symbol
        ),
    ];
    if !impact_facts.direct_callers.is_empty() || !impact_facts.direct_callees.is_empty() {
        regression_targets.push(format!(
            "axc context impact <path> {} --json",
            impact_facts.resolved_symbol
        ));
    }

    ImpactHints {
        smallest_safe_edit_scope,
        likely_breakages,
        regression_targets,
    }
}

fn build_evidence_facts(
    requested_symbol: &str,
    request_path: &Path,
    input: &ResolvedInput,
    symbol_catalog: &SymbolCatalog,
) -> Result<EvidenceFacts, String> {
    let impact_facts = build_impact_facts(requested_symbol, symbol_catalog)?;
    let expected_artifacts =
        build_context_expected_artifacts(request_path, input, requested_symbol);
    Ok(EvidenceFacts {
        requested_symbol: requested_symbol.to_string(),
        resolved_symbol: impact_facts.resolved_symbol.clone(),
        affected_units: impact_facts
            .affected_units
            .iter()
            .map(|unit| unit.path.clone())
            .collect(),
        related_examples: build_related_examples(request_path, input, &impact_facts),
        related_tests: build_related_tests(input, &impact_facts),
        related_docs: build_related_docs(input, &impact_facts),
        related_benchmarks: build_related_benchmarks(input, &impact_facts),
        expected_artifacts,
    })
}

fn build_evidence_hints(
    command_target: &str,
    input: &ResolvedInput,
    related_tests: &[String],
    resolved_symbol: &str,
    expected_artifacts: &[String],
) -> EvidenceHints {
    let mut recommended_commands = vec![
        format!("axc check {command_target}"),
        format!("axc context impact {command_target} {resolved_symbol} --json"),
        format!("axc context evidence {command_target} {resolved_symbol} --json"),
    ];

    if input.project.is_some() || command_target.starts_with("examples/") {
        recommended_commands.push(format!("axc run {command_target} -- <args...>"));
    }
    if related_tests
        .iter()
        .any(|path| path == "tests/interface_snapshots.rs")
    {
        recommended_commands.push("cargo test --test interface_snapshots context_".to_string());
    }

    EvidenceHints {
        recommended_commands,
        expected_artifacts: expected_artifacts.to_vec(),
    }
}

fn build_related_examples(
    request_path: &Path,
    input: &ResolvedInput,
    _impact_facts: &ImpactFacts,
) -> Vec<String> {
    let subject_path = evidence_subject_path(request_path, input);
    let mut related = BTreeSet::new();
    if subject_path.starts_with("examples/") {
        related.insert(subject_path.clone());
    }

    if let Some(project) = input.project.as_ref() {
        if let Some(stem) = project.target_name().strip_prefix("project_") {
            let file_candidate = format!("examples/{stem}.ax");
            let directory_candidate = format!("examples/{stem}");
            insert_repo_path_if_exists(&mut related, &file_candidate);
            insert_repo_path_if_exists(&mut related, &directory_candidate);
        }
    } else if let Some(stem) = request_path.file_stem().and_then(|value| value.to_str()) {
        let directory_candidate = format!("examples/project_{stem}");
        insert_repo_path_if_exists(&mut related, &directory_candidate);
    }

    related.into_iter().take(6).collect()
}

fn build_related_tests(input: &ResolvedInput, impact_facts: &ImpactFacts) -> Vec<String> {
    let mut related = BTreeSet::new();
    let tokens = build_evidence_search_tokens(input, impact_facts);
    let test_file = repo_root().join("tests").join("interface_snapshots.rs");
    if file_matches_tokens(&test_file, &tokens) {
        related.insert("tests/interface_snapshots.rs".to_string());
    }

    related.into_iter().collect()
}

fn build_related_docs(input: &ResolvedInput, impact_facts: &ImpactFacts) -> Vec<String> {
    let mut related = BTreeSet::new();
    insert_repo_path_if_exists(&mut related, "架构上下文文档.md");
    insert_repo_path_if_exists(&mut related, "docs/README.md");
    insert_repo_path_if_exists(&mut related, "docs/feature-matrix.md");

    if input.project.is_some() {
        insert_repo_path_if_exists(&mut related, "docs/import-module-minimal-design.md");
    }
    if impact_facts
        .affected_units
        .iter()
        .any(|unit| !unit.host_boundary_classes.is_empty())
    {
        insert_repo_path_if_exists(&mut related, "docs/host-runtime-boundary.md");
    }

    related.into_iter().collect()
}

fn build_related_benchmarks(input: &ResolvedInput, impact_facts: &ImpactFacts) -> Vec<String> {
    let mut related = BTreeSet::new();
    let tokens = build_evidence_search_tokens(input, impact_facts);
    let benchmarks_dir = repo_root().join("benchmarks");
    collect_matching_repo_files(&benchmarks_dir, &tokens, &mut related);
    related.into_iter().take(8).collect()
}

fn collect_upstream_symbols(
    symbol_catalog: &SymbolCatalog,
    resolved_symbol: &str,
    direct_callers: &[String],
) -> Vec<String> {
    let mut upstream = Vec::new();
    let mut seen = direct_callers.iter().cloned().collect::<BTreeSet<_>>();
    let mut queue = std::collections::VecDeque::from(direct_callers.to_vec());

    while let Some(current) = queue.pop_front() {
        upstream.push(current.clone());
        let callers = symbol_catalog
            .callers_by_symbol
            .get(&current)
            .cloned()
            .unwrap_or_default();
        for caller in callers {
            if caller != resolved_symbol && seen.insert(caller.clone()) {
                queue.push_back(caller);
            }
        }
    }

    upstream
}

fn collect_downstream_symbols(
    symbol_catalog: &SymbolCatalog,
    resolved_symbol: &str,
    direct_callees: &[String],
) -> Vec<String> {
    let mut downstream = Vec::new();
    let mut seen = direct_callees.iter().cloned().collect::<BTreeSet<_>>();
    let mut queue = std::collections::VecDeque::from(direct_callees.to_vec());

    while let Some(current) = queue.pop_front() {
        downstream.push(current.clone());
        let Some(symbol) = symbol_catalog.definitions.get(&current) else {
            continue;
        };
        for callee in &symbol.resolved_callee_order {
            if callee != resolved_symbol && seen.insert(callee.clone()) {
                queue.push_back(callee.clone());
            }
        }
    }

    downstream
}

fn build_affected_units(
    symbol_catalog: &SymbolCatalog,
    resolved_symbol: &str,
    upstream_callers: &[String],
    downstream_callees: &[String],
) -> Vec<ImpactUnit> {
    let mut affected_symbols = BTreeSet::new();
    affected_symbols.insert(resolved_symbol.to_string());
    affected_symbols.extend(upstream_callers.iter().cloned());
    affected_symbols.extend(downstream_callees.iter().cloned());

    let mut by_path = BTreeMap::<String, ImpactUnit>::new();
    for symbol_name in affected_symbols {
        let Some(symbol) = symbol_catalog.definitions.get(&symbol_name) else {
            continue;
        };
        let entry = by_path
            .entry(symbol.source_path.clone())
            .or_insert_with(|| ImpactUnit {
                path: symbol.source_path.clone(),
                symbol_count: 0,
                includes_target: false,
                host_boundary_classes: Vec::new(),
            });
        entry.symbol_count += 1;
        entry.includes_target |= symbol_name == resolved_symbol;
        let mut host_classes = entry
            .host_boundary_classes
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        host_classes.extend(symbol.host_classes.iter().cloned());
        entry.host_boundary_classes = host_classes.into_iter().collect();
    }

    by_path.into_values().collect()
}

fn build_impact_risk(
    symbol: &DefinedSymbol,
    direct_callers: &[String],
    direct_callees: &[String],
    upstream_callers: &[String],
    downstream_callees: &[String],
    affected_units: &[ImpactUnit],
    recursive: bool,
) -> ImpactRisk {
    let mut reasons = Vec::new();

    if symbol.is_entry {
        reasons.push("entry symbol changes can shift the whole project command path".to_string());
    }
    if !symbol.host_classes.is_empty() {
        reasons.push(format!(
            "touches host boundary classes: {}",
            symbol
                .host_classes
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if recursive {
        reasons.push("participates in a recursive call cycle".to_string());
    }
    if direct_callers.len() >= 2 || upstream_callers.len() >= 3 {
        reasons.push("has multiple upstream dependents".to_string());
    }
    if direct_callees.len() >= 3 || downstream_callees.len() >= 5 {
        reasons.push("fans out into a wide downstream call surface".to_string());
    }
    if symbol.branch_count >= 4 {
        reasons.push(format!(
            "contains dense control flow with {} branch site(s)",
            symbol.branch_count
        ));
    }
    if affected_units.len() >= 4 {
        reasons.push(format!(
            "spans {} affected source units",
            affected_units.len()
        ));
    }

    let level = if symbol.is_entry
        || recursive
        || (!symbol.host_classes.is_empty()
            && (direct_callers.len() >= 2 || affected_units.len() >= 3))
        || affected_units.len() >= 6
    {
        "high"
    } else if !symbol.host_classes.is_empty()
        || !direct_callers.is_empty()
        || !direct_callees.is_empty()
        || symbol.branch_count >= 2
        || affected_units.len() >= 2
    {
        "medium"
    } else {
        "low"
    };

    ImpactRisk { level, reasons }
}

fn select_entry_symbol(symbol_catalog: &SymbolCatalog) -> Option<String> {
    let mut candidates = symbol_catalog
        .definitions
        .values()
        .filter(|symbol| symbol.kind == DefinedSymbolKind::Function && symbol.is_entry)
        .map(|symbol| symbol.qualified_name.clone())
        .collect::<Vec<_>>();
    candidates.sort();

    candidates
        .iter()
        .find(|symbol| symbol.as_str() == "main" || symbol.ends_with(".main"))
        .cloned()
        .or_else(|| candidates.into_iter().next())
}

fn collect_reachable_flow_symbols(
    symbol_catalog: &SymbolCatalog,
    entry_symbol: &str,
) -> (Vec<String>, BTreeMap<String, usize>) {
    let mut reachable_order = Vec::new();
    let mut depth_by_symbol = BTreeMap::<String, usize>::new();
    let mut queue = std::collections::VecDeque::<String>::new();

    depth_by_symbol.insert(entry_symbol.to_string(), 0);
    queue.push_back(entry_symbol.to_string());

    while let Some(symbol_name) = queue.pop_front() {
        let Some(depth) = depth_by_symbol.get(&symbol_name).copied() else {
            continue;
        };
        reachable_order.push(symbol_name.clone());

        let Some(symbol) = symbol_catalog.definitions.get(&symbol_name) else {
            continue;
        };
        for callee in &symbol.resolved_callee_order {
            if !symbol_catalog.definitions.contains_key(callee)
                || depth_by_symbol.contains_key(callee)
            {
                continue;
            }
            depth_by_symbol.insert(callee.clone(), depth + 1);
            queue.push_back(callee.clone());
        }
    }

    (reachable_order, depth_by_symbol)
}

fn collect_recursive_symbols(
    symbol_catalog: &SymbolCatalog,
    reachable_order: &[String],
) -> Vec<String> {
    let reachable_set = reachable_order.iter().cloned().collect::<BTreeSet<_>>();
    let mut recursive_symbols = Vec::new();

    for symbol_name in reachable_order {
        let mut visited = BTreeSet::new();
        if symbol_reaches_target(
            symbol_catalog,
            symbol_name,
            symbol_name,
            &reachable_set,
            &mut visited,
        ) {
            recursive_symbols.push(symbol_name.clone());
        }
    }

    recursive_symbols
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

fn filter_reachable_callee_order(
    symbol: &DefinedSymbol,
    depth_by_symbol: &BTreeMap<String, usize>,
) -> Vec<String> {
    symbol
        .resolved_callee_order
        .iter()
        .filter(|callee| depth_by_symbol.contains_key(*callee))
        .cloned()
        .collect()
}

fn build_longest_flow_chain(
    symbol_catalog: &SymbolCatalog,
    entry_symbol: &str,
    reachable_set: &BTreeSet<String>,
) -> Vec<String> {
    longest_flow_chain_from(
        symbol_catalog,
        entry_symbol,
        reachable_set,
        &mut BTreeSet::new(),
    )
}

fn longest_flow_chain_from(
    symbol_catalog: &SymbolCatalog,
    current: &str,
    reachable_set: &BTreeSet<String>,
    visiting: &mut BTreeSet<String>,
) -> Vec<String> {
    if !visiting.insert(current.to_string()) {
        return vec![current.to_string()];
    }

    let mut best_suffix = Vec::new();
    if let Some(symbol) = symbol_catalog.definitions.get(current) {
        for callee in &symbol.resolved_callee_order {
            if !reachable_set.contains(callee) {
                continue;
            }
            let candidate =
                longest_flow_chain_from(symbol_catalog, callee, reachable_set, visiting);
            if candidate.len() > best_suffix.len()
                || (candidate.len() == best_suffix.len() && candidate < best_suffix)
            {
                best_suffix = candidate;
            }
        }
    }

    visiting.remove(current);
    let mut chain = vec![current.to_string()];
    if !best_suffix.is_empty() {
        chain.extend(best_suffix);
    }
    chain
}

fn build_constraint_candidates(
    units: &[ResolvedUnit],
    unit_stats: &BTreeMap<String, UnitStats>,
) -> Vec<ConstraintCandidate> {
    let mut candidates = Vec::new();

    for unit in units.iter().filter(|unit| !unit.is_entry) {
        let stats = unit_stats.get(&unit.path).cloned().unwrap_or_default();
        if stats.host_classes.is_empty() && stats.function_count != 0 {
            candidates.push(ConstraintCandidate {
                kind: "keep_host_free",
                targets: vec![unit.path.clone()],
                reason: "support unit currently stays pure enough to keep free of host-boundary builtins"
                    .to_string(),
            });
        }
    }

    let write_units = units
        .iter()
        .filter(|unit| {
            let stats = unit_stats.get(&unit.path).cloned().unwrap_or_default();
            !stats.filesystem_write_builtins.is_empty()
        })
        .collect::<Vec<_>>();

    if !write_units.is_empty() && write_units.iter().all(|unit| unit.is_entry) {
        candidates.push(ConstraintCandidate {
            kind: "entry_only_filesystem_write",
            targets: write_units.iter().map(|unit| unit.path.clone()).collect(),
            reason: "filesystem mutation is currently concentrated in entry unit code".to_string(),
        });
    }

    candidates
}

fn collect_imported_by_counts(units: &[ResolvedUnit]) -> BTreeMap<String, usize> {
    let module_to_path = units
        .iter()
        .filter_map(|unit| {
            unit.module_path
                .as_ref()
                .map(|module_path| (module_path.clone(), unit.path.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let mut imported_by_count = BTreeMap::<String, usize>::new();

    for unit in units {
        for import in &unit.imports {
            if let Some(path) = module_to_path.get(import) {
                *imported_by_count.entry(path.clone()).or_insert(0) += 1;
            }
        }
    }

    imported_by_count
}

fn unit_role_hints(
    unit: &ResolvedUnit,
    stats: &UnitStats,
    imported_by_count: usize,
) -> Vec<String> {
    let mut hints = Vec::new();

    if unit.is_entry {
        hints.push("entry_orchestrator".to_string());
    }
    if is_foundation_unit(unit) {
        hints.push("shared_foundation".to_string());
    }
    if imported_by_count >= 2 {
        hints.push("shared_library".to_string());
    }
    if is_host_heavy(stats) {
        hints.push("host_bridge_heavy".to_string());
    }

    hints
}

fn unit_role_evidence(
    unit: &ResolvedUnit,
    stats: &UnitStats,
    imported_by_count: usize,
) -> Vec<String> {
    let mut evidence = Vec::new();

    if unit.is_entry {
        evidence.push("selected as the project entry unit".to_string());
    }
    if !unit.imports.is_empty() {
        evidence.push(format!("imports {} module(s)", unit.imports.len()));
    }
    if imported_by_count != 0 {
        evidence.push(format!("imported by {} other unit(s)", imported_by_count));
    }
    if is_foundation_unit(unit) {
        evidence.push("lives under the shared foundation surface".to_string());
    }
    if !stats.host_classes.is_empty() {
        evidence.push(format!(
            "touches host classes: {}",
            stats
                .host_classes
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    evidence
}

fn is_foundation_unit(unit: &ResolvedUnit) -> bool {
    unit.module_path
        .as_deref()
        .is_some_and(|module_path| module_path.starts_with("foundation."))
        || unit.path.starts_with("foundation/")
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

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn evidence_subject_path(request_path: &Path, input: &ResolvedInput) -> String {
    input
        .project
        .as_ref()
        .map(|project| normalize_path(project.root_dir()))
        .unwrap_or_else(|| normalize_path(request_path))
}

fn build_context_expected_artifacts(
    request_path: &Path,
    input: &ResolvedInput,
    requested_symbol: &str,
) -> Vec<String> {
    let subject_key = context_subject_snapshot_key(request_path, input);
    let symbol_key = snapshot_symbol_key(requested_symbol);
    vec![
        format!("tests/snapshots/context_flow_{subject_key}.json"),
        format!("tests/snapshots/context_symbol_{subject_key}_{symbol_key}.json"),
        format!("tests/snapshots/context_impact_{subject_key}_{symbol_key}.json"),
        format!("tests/snapshots/context_evidence_{subject_key}_{symbol_key}.json"),
    ]
}

fn context_subject_snapshot_key(request_path: &Path, input: &ResolvedInput) -> String {
    let subject_path = input
        .project
        .as_ref()
        .map(|project| project.root_dir())
        .unwrap_or(request_path);
    subject_path
        .file_stem()
        .or_else(|| subject_path.file_name())
        .and_then(|value| value.to_str())
        .map(snapshot_key_fragment)
        .unwrap_or_else(|| "context".to_string())
}

fn snapshot_symbol_key(symbol: &str) -> String {
    snapshot_key_fragment(symbol.rsplit('.').next().unwrap_or(symbol))
}

fn snapshot_key_fragment(text: &str) -> String {
    let mut key = String::new();
    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            key.push(character.to_ascii_lowercase());
        } else {
            key.push('_');
        }
    }

    while key.contains("__") {
        key = key.replace("__", "_");
    }

    key.trim_matches('_').to_string()
}

fn build_evidence_search_tokens(input: &ResolvedInput, impact_facts: &ImpactFacts) -> Vec<String> {
    let mut tokens = BTreeSet::new();
    if let Some(project) = input.project.as_ref() {
        collect_search_token(&mut tokens, project.target_name());
    }
    collect_search_token(&mut tokens, &impact_facts.requested_symbol);
    collect_search_token(&mut tokens, &impact_facts.resolved_symbol);
    collect_search_token(
        &mut tokens,
        impact_facts
            .resolved_symbol
            .rsplit('.')
            .next()
            .unwrap_or(&impact_facts.resolved_symbol),
    );
    for unit in &impact_facts.affected_units {
        if let Some(stem) = Path::new(&unit.path)
            .file_stem()
            .and_then(|value| value.to_str())
        {
            collect_search_token(&mut tokens, stem);
        }
    }

    tokens.into_iter().collect()
}

fn collect_search_token(tokens: &mut BTreeSet<String>, raw: &str) {
    let lowered = raw.replace('\\', "/").to_ascii_lowercase();
    for fragment in [
        lowered.as_str(),
        lowered.strip_prefix("project_").unwrap_or(&lowered),
    ] {
        let candidate = fragment.trim_matches('/');
        if is_high_signal_evidence_token(candidate) {
            tokens.insert(candidate.to_string());
        }
    }
}

fn is_high_signal_evidence_token(token: &str) -> bool {
    token.len() >= 4
        && !token.starts_with("examples/")
        && (token.contains('_') || token.contains('.') || token.len() >= 12)
}

fn insert_repo_path_if_exists(paths: &mut BTreeSet<String>, relative_path: &str) {
    let absolute = repo_root().join(relative_path);
    if absolute.exists() {
        paths.insert(relative_path.replace('\\', "/"));
    }
}

fn collect_matching_repo_files(root: &Path, tokens: &[String], matches: &mut BTreeSet<String>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_matching_repo_files(&path, tokens, matches);
            continue;
        }
        let Some(relative_path) = repo_relative_path(&path) else {
            continue;
        };
        if file_matches_path_or_contents(&path, &relative_path, tokens) {
            matches.insert(relative_path);
        }
    }
}

fn file_matches_path_or_contents(path: &Path, relative_path: &str, tokens: &[String]) -> bool {
    path_string_matches_tokens(relative_path, tokens) || file_matches_tokens(path, tokens)
}

fn file_matches_tokens(path: &Path, tokens: &[String]) -> bool {
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    path_string_matches_tokens(&text, tokens)
}

fn path_string_matches_tokens(text: &str, tokens: &[String]) -> bool {
    let lowered = text.replace('\\', "/").to_ascii_lowercase();
    tokens.iter().any(|token| lowered.contains(token))
}

fn repo_relative_path(path: &Path) -> Option<String> {
    path.strip_prefix(repo_root()).ok().map(normalize_path)
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
