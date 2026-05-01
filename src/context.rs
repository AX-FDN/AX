use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::ast::Program;
use crate::diagnostics::Diagnostic;
use crate::lockfile::check_lockfile;
use crate::package_diagnostics::package_repair_hint;
use crate::project::{Project, ResolvedInput};

mod boundaries;
mod catalog;
mod evidence;
mod flow;
mod impact;
mod overview;
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
