use super::*;

pub(super) fn build_boundaries_facts(
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

pub(super) fn build_boundaries_hints(
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
