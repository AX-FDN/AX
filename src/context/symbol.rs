use super::*;

pub(super) fn build_symbol_facts(
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

pub(super) fn build_symbol_hints(
    symbol_facts: &SymbolFacts,
    symbol_catalog: &SymbolCatalog,
) -> SymbolHints {
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
