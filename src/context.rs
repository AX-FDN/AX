use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::ast::Program;
use crate::diagnostics::Diagnostic;
use crate::project::{Project, ResolvedInput};

mod boundaries;
mod catalog;
mod evidence;
mod flow;
mod impact;
mod overview;
mod package;
mod shared;
mod stats;
mod symbol;
mod topology;
mod types;

use self::boundaries::{build_boundaries_facts, build_boundaries_hints};
use self::catalog::{
    DefinedSymbol, DefinedSymbolKind, SymbolCatalog, build_symbol_catalog, resolve_symbol_query,
    symbol_reaches_target,
};
use self::evidence::{build_evidence_facts, build_evidence_hints};
use self::flow::{build_flow_facts, build_flow_hints};
use self::impact::{build_impact_facts, build_impact_hints};
use self::overview::{build_overview_facts, build_overview_hints};
use self::package::{build_local_package_lock_fact, build_local_path_package_facts};
use self::shared::{build_validation, normalize_path, normalize_path_text, push_unique};
use self::stats::{
    ResolvedUnit, UnitStats, collect_source_units, collect_unit_stats, host_heavy_reason,
    is_host_heavy,
};
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
