use super::*;

pub(super) fn build_impact_facts(
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

pub(super) fn build_impact_hints(impact_facts: &ImpactFacts) -> ImpactHints {
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
