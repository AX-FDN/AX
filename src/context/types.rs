use serde::Serialize;

use crate::ast::Visibility;
use crate::build::AotReadiness;

#[derive(Serialize)]
pub(super) struct ContextDocument<Facts, Hints> {
    pub(super) schema_version: u32,
    pub(super) view: &'static str,
    pub(super) subject: ContextSubject,
    pub(super) facts: Facts,
    pub(super) hints: Hints,
    pub(super) validation: ContextValidation,
}

#[derive(Serialize)]
pub(super) struct ContextSubject {
    pub(super) kind: &'static str,
    pub(super) path: String,
    pub(super) entry: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) project_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) symbol: Option<String>,
}

#[derive(Serialize)]
pub(super) struct ContextValidation {
    pub(super) diagnostic_count: usize,
    pub(super) partial: bool,
    pub(super) recommended_commands: Vec<String>,
    pub(super) notes: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct OverviewFacts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) project_name: Option<String>,
    pub(super) entry: String,
    pub(super) module_mode: bool,
    pub(super) source_roots: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) local_path_packages: Vec<ContextPathPackage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) local_package_lock: Option<ContextPackageLock>,
    pub(super) summary: OverviewSummary,
    pub(super) source_units: Vec<OverviewUnit>,
}

#[derive(Serialize)]
pub(super) struct OverviewSummary {
    pub(super) source_unit_count: usize,
    pub(super) support_unit_count: usize,
    pub(super) module_count: usize,
    pub(super) import_count: usize,
    pub(super) function_count: usize,
    pub(super) struct_count: usize,
    pub(super) enum_count: usize,
    pub(super) type_count: usize,
    pub(super) diagnostic_count: usize,
}

#[derive(Serialize)]
pub(super) struct OverviewUnit {
    pub(super) path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) module_path: Option<String>,
    pub(super) is_entry: bool,
    pub(super) imports: Vec<String>,
    pub(super) function_count: usize,
    pub(super) type_count: usize,
}

#[derive(Serialize)]
pub(super) struct OverviewHints {
    pub(super) entrypoints: Vec<String>,
    pub(super) support_modules: Vec<String>,
    pub(super) core_symbols: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct BoundariesFacts {
    pub(super) host_boundary_classes: Vec<String>,
    pub(super) unit_boundary_usage: Vec<UnitBoundaryUsage>,
}

#[derive(Serialize)]
pub(super) struct UnitBoundaryUsage {
    pub(super) path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) module_path: Option<String>,
    pub(super) is_entry: bool,
    pub(super) function_count: usize,
    pub(super) type_count: usize,
    pub(super) host_classes: Vec<String>,
    pub(super) host_builtins: Vec<String>,
    pub(super) host_call_count: usize,
    pub(super) filesystem_write_builtins: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct BoundariesHints {
    pub(super) host_heavy_units: Vec<HostHeavyUnitHint>,
    pub(super) safe_logic_units: Vec<SafeLogicUnitHint>,
    pub(super) constraint_candidates: Vec<ConstraintCandidate>,
}

#[derive(Serialize)]
pub(super) struct TopologyFacts {
    pub(super) module_mode: bool,
    pub(super) summary: TopologySummary,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) local_path_packages: Vec<ContextPathPackage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) local_package_lock: Option<ContextPackageLock>,
    pub(super) source_units: Vec<TopologyUnit>,
    pub(super) module_edges: Vec<ModuleEdge>,
    pub(super) symbol_edges: Vec<SymbolEdge>,
}

#[derive(Serialize)]
pub(super) struct ContextPathPackage {
    pub(super) alias: String,
    pub(super) root: String,
    pub(super) manifest: String,
    pub(super) source_count: usize,
    pub(super) modules: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct ContextPackageLock {
    pub(super) path: String,
    pub(super) schema_version: u32,
    pub(super) status: &'static str,
    pub(super) dependency_count: usize,
    pub(super) note: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) issues: Vec<ContextPackageLockIssue>,
}

#[derive(Serialize)]
pub(super) struct ContextPackageLockIssue {
    pub(super) code: &'static str,
    pub(super) kind: &'static str,
    pub(super) message: String,
    pub(super) fixit: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) repair_rule: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) repair_goal: Option<&'static str>,
}

#[derive(Serialize)]
pub(super) struct BuildReadiness {
    pub(super) build_mode: &'static str,
    pub(super) aot_status: &'static str,
    pub(super) executable_emission: bool,
    pub(super) planned_executable_artifact: bool,
    pub(super) blocking_features: Vec<String>,
    pub(super) notes: Vec<String>,
    pub(super) aot_readiness: AotReadiness,
}

#[derive(Serialize)]
pub(super) struct PackageGraphReadiness {
    pub(super) package_mode: &'static str,
    pub(super) reproducible: bool,
    pub(super) aot_ready: bool,
    pub(super) lock_status: &'static str,
    pub(super) risk_level: &'static str,
    pub(super) blocking_reasons: Vec<String>,
    pub(super) recommended_commands: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct TopologySummary {
    pub(super) source_unit_count: usize,
    pub(super) module_edge_count: usize,
    pub(super) symbol_count: usize,
    pub(super) symbol_edge_count: usize,
}

#[derive(Serialize)]
pub(super) struct TopologyUnit {
    pub(super) path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) module_path: Option<String>,
    pub(super) is_entry: bool,
    pub(super) imports: Vec<String>,
    pub(super) imported_by_count: usize,
    pub(super) defined_symbols: Vec<String>,
    pub(super) host_classes: Vec<String>,
    pub(super) role_hints: Vec<String>,
    pub(super) role_evidence: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct ModuleEdge {
    pub(super) from_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) from_module: Option<String>,
    pub(super) to_module: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) to_path: Option<String>,
    pub(super) kind: &'static str,
    pub(super) resolved: bool,
}

#[derive(Serialize)]
pub(super) struct SymbolEdge {
    pub(super) from: String,
    pub(super) to: String,
    pub(super) kind: &'static str,
    pub(super) cross_unit: bool,
}

#[derive(Serialize)]
pub(super) struct TopologyHints {
    pub(super) entry_orchestrators: Vec<String>,
    pub(super) shared_foundations: Vec<String>,
    pub(super) central_symbols: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct FlowFacts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) entry_symbol: Option<String>,
    pub(super) summary: FlowSummary,
    pub(super) top_level_calls: Vec<String>,
    pub(super) reachable_symbols: Vec<FlowReachableSymbol>,
    pub(super) flow_edges: Vec<FlowEdge>,
    pub(super) branch_points: Vec<FlowBranchPoint>,
    pub(super) recursive_symbols: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct FlowSummary {
    pub(super) reachable_symbol_count: usize,
    pub(super) flow_edge_count: usize,
    pub(super) branch_point_count: usize,
    pub(super) recursive_symbol_count: usize,
    pub(super) max_depth: usize,
}

#[derive(Serialize)]
pub(super) struct FlowReachableSymbol {
    pub(super) symbol: String,
    pub(super) depth: usize,
    pub(super) source_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) module_path: Option<String>,
    pub(super) host_boundary_classes: Vec<String>,
    pub(super) branch_count: usize,
}

#[derive(Serialize)]
pub(super) struct FlowEdge {
    pub(super) from: String,
    pub(super) to: String,
    pub(super) target_depth: usize,
    pub(super) cross_unit: bool,
}

#[derive(Serialize)]
pub(super) struct FlowBranchPoint {
    pub(super) symbol: String,
    pub(super) branch_kinds: Vec<String>,
    pub(super) branch_count: usize,
    pub(super) note: String,
}

#[derive(Serialize)]
pub(super) struct FlowHints {
    pub(super) orchestration_chain: Vec<String>,
    pub(super) host_boundary_symbols: Vec<String>,
    pub(super) leaf_symbols: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct ImpactFacts {
    pub(super) requested_symbol: String,
    pub(super) resolved_symbol: String,
    pub(super) direct_callers: Vec<String>,
    pub(super) direct_callees: Vec<String>,
    pub(super) upstream_callers: Vec<String>,
    pub(super) downstream_callees: Vec<String>,
    pub(super) affected_units: Vec<ImpactUnit>,
    pub(super) recursive: bool,
    pub(super) change_risk: ImpactRisk,
}

#[derive(Serialize)]
pub(super) struct ImpactUnit {
    pub(super) path: String,
    pub(super) symbol_count: usize,
    pub(super) includes_target: bool,
    pub(super) host_boundary_classes: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct ImpactRisk {
    pub(super) level: &'static str,
    pub(super) reasons: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct ImpactHints {
    pub(super) smallest_safe_edit_scope: Vec<String>,
    pub(super) likely_breakages: Vec<String>,
    pub(super) regression_targets: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct EvidenceFacts {
    pub(super) requested_symbol: String,
    pub(super) resolved_symbol: String,
    pub(super) affected_units: Vec<String>,
    pub(super) related_examples: Vec<String>,
    pub(super) related_tests: Vec<String>,
    pub(super) related_docs: Vec<String>,
    pub(super) related_benchmarks: Vec<String>,
    pub(super) expected_artifacts: Vec<String>,
    pub(super) build_readiness: BuildReadiness,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) local_package_lock: Option<ContextPackageLock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) package_graph_readiness: Option<PackageGraphReadiness>,
}

#[derive(Serialize)]
pub(super) struct EvidenceHints {
    pub(super) recommended_commands: Vec<String>,
    pub(super) expected_artifacts: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct SymbolFacts {
    pub(super) requested_symbol: String,
    pub(super) resolved_symbol: String,
    pub(super) kind: &'static str,
    #[serde(default, skip_serializing_if = "Visibility::is_private")]
    pub(super) visibility: Visibility,
    pub(super) source_unit: SymbolSourceUnit,
    pub(super) signature: SymbolSignature,
    pub(super) callers: Vec<String>,
    pub(super) callees: Vec<String>,
    pub(super) related_types: Vec<String>,
    pub(super) host_boundary_classes: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct SymbolSourceUnit {
    pub(super) path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) module_path: Option<String>,
    pub(super) is_entry: bool,
    pub(super) imports: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct SymbolSignature {
    pub(super) params: Vec<SymbolParamView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) return_type: Option<String>,
}

#[derive(Serialize)]
pub(super) struct SymbolParamView {
    pub(super) name: String,
    pub(super) ty: String,
}

#[derive(Serialize)]
pub(super) struct SymbolHints {
    pub(super) role_hints: Vec<String>,
    pub(super) role_evidence: Vec<String>,
    pub(super) adjacent_symbols: Vec<String>,
}

#[derive(Serialize)]
pub(super) struct HostHeavyUnitHint {
    pub(super) path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) module_path: Option<String>,
    pub(super) host_classes: Vec<String>,
    pub(super) host_builtins: Vec<String>,
    pub(super) reason: String,
}

#[derive(Serialize)]
pub(super) struct SafeLogicUnitHint {
    pub(super) path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) module_path: Option<String>,
    pub(super) function_count: usize,
    pub(super) type_count: usize,
    pub(super) reason: String,
}

#[derive(Serialize)]
pub(super) struct ConstraintCandidate {
    pub(super) kind: &'static str,
    pub(super) targets: Vec<String>,
    pub(super) reason: String,
}
