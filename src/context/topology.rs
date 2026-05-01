use super::*;

pub(super) fn build_topology_facts(
    project: Option<&Project>,
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
        local_path_packages: build_local_path_package_facts(project, units),
        local_package_lock: build_local_package_lock_fact(project),
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

pub(super) fn build_topology_hints(
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
