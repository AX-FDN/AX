use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::ast::{Block, Expr, ExprKind, ItemKind, Program, Stmt, StmtKind};
use crate::diagnostics::Diagnostic;
use crate::lockfile::check_lockfile;
use crate::package_diagnostics::package_repair_hint;
use crate::project::{Project, ResolvedInput};
use crate::source::SourceFile;

mod boundaries;
mod catalog;
mod evidence;
mod flow;
mod impact;
mod overview;
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
        ExprKind::Block { statements, value } => {
            for statement in statements {
                visit_stmt(statement, stats);
            }
            visit_expr(value, stats);
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
