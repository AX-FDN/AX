use super::*;

pub(super) fn build_flow_facts(symbol_catalog: &SymbolCatalog) -> FlowFacts {
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

pub(super) fn build_flow_hints(symbol_catalog: &SymbolCatalog) -> FlowHints {
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
