use super::*;

pub(super) fn build_overview_facts(
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
        local_path_packages: build_local_path_package_facts(project, units),
        local_package_lock: build_local_package_lock_fact(project),
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

pub(super) fn build_overview_hints(
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
