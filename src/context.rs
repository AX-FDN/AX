use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Serialize;

use crate::ast::{Block, Expr, ExprKind, ItemKind, Program, Stmt, StmtKind};
use crate::diagnostics::Diagnostic;
use crate::project::{Project, ResolvedInput};
use crate::source::SourceFile;

const CONTEXT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextView {
    Overview,
    Boundaries,
}

impl ContextView {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Boundaries => "boundaries",
        }
    }
}

pub fn render_context_json(
    view: ContextView,
    request_path: &Path,
    input: &ResolvedInput,
    program: &Program,
    diagnostics: &[Diagnostic],
) -> String {
    let units = collect_source_units(input, program);
    let unit_stats = collect_unit_stats(&input.source, program);
    let subject = build_subject(request_path, input);
    let command_target = subject.path.clone();

    let rendered = match view {
        ContextView::Overview => serde_json::to_string_pretty(&ContextDocument {
            schema_version: CONTEXT_SCHEMA_VERSION,
            view: view.as_str(),
            subject,
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
            subject,
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
    }
    .expect("context json should serialize");

    rendered + "\n"
}

#[derive(Serialize)]
struct ContextDocument<Facts, Hints> {
    schema_version: u32,
    view: &'static str,
    subject: ContextSubject,
    facts: Facts,
    hints: Hints,
    validation: ContextValidation,
}

#[derive(Serialize)]
struct ContextSubject {
    kind: &'static str,
    path: String,
    entry: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_name: Option<String>,
}

#[derive(Serialize)]
struct ContextValidation {
    diagnostic_count: usize,
    partial: bool,
    recommended_commands: Vec<String>,
    notes: Vec<String>,
}

#[derive(Serialize)]
struct OverviewFacts {
    #[serde(skip_serializing_if = "Option::is_none")]
    project_name: Option<String>,
    entry: String,
    module_mode: bool,
    source_roots: Vec<String>,
    summary: OverviewSummary,
    source_units: Vec<OverviewUnit>,
}

#[derive(Serialize)]
struct OverviewSummary {
    source_unit_count: usize,
    support_unit_count: usize,
    module_count: usize,
    import_count: usize,
    function_count: usize,
    struct_count: usize,
    enum_count: usize,
    type_count: usize,
    diagnostic_count: usize,
}

#[derive(Serialize)]
struct OverviewUnit {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    module_path: Option<String>,
    is_entry: bool,
    imports: Vec<String>,
    function_count: usize,
    type_count: usize,
}

#[derive(Serialize)]
struct OverviewHints {
    entrypoints: Vec<String>,
    support_modules: Vec<String>,
    core_symbols: Vec<String>,
}

#[derive(Serialize)]
struct BoundariesFacts {
    host_boundary_classes: Vec<String>,
    unit_boundary_usage: Vec<UnitBoundaryUsage>,
}

#[derive(Serialize)]
struct UnitBoundaryUsage {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    module_path: Option<String>,
    is_entry: bool,
    function_count: usize,
    type_count: usize,
    host_classes: Vec<String>,
    host_builtins: Vec<String>,
    host_call_count: usize,
    filesystem_write_builtins: Vec<String>,
}

#[derive(Serialize)]
struct BoundariesHints {
    host_heavy_units: Vec<HostHeavyUnitHint>,
    safe_logic_units: Vec<SafeLogicUnitHint>,
    constraint_candidates: Vec<ConstraintCandidate>,
}

#[derive(Serialize)]
struct HostHeavyUnitHint {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    module_path: Option<String>,
    host_classes: Vec<String>,
    host_builtins: Vec<String>,
    reason: String,
}

#[derive(Serialize)]
struct SafeLogicUnitHint {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    module_path: Option<String>,
    function_count: usize,
    type_count: usize,
    reason: String,
}

#[derive(Serialize)]
struct ConstraintCandidate {
    kind: &'static str,
    targets: Vec<String>,
    reason: String,
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

fn build_subject(request_path: &Path, input: &ResolvedInput) -> ContextSubject {
    match input.project.as_ref() {
        Some(project) => ContextSubject {
            kind: "project",
            path: normalize_path(project.root_dir()),
            entry: normalize_path(project.entry_path()),
            project_name: Some(project.target_name().to_string()),
        },
        None => ContextSubject {
            kind: "source",
            path: normalize_path(request_path),
            entry: normalize_path(input.source.path()),
            project_name: None,
        },
    }
}

fn build_overview_facts(
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

fn build_overview_hints(
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

fn build_boundaries_facts(
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

fn build_boundaries_hints(
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
            ItemKind::Struct { name, .. } => {
                stats.struct_count += 1;
                stats.symbols.push(name.clone());
            }
            ItemKind::Enum { name, .. } => {
                stats.enum_count += 1;
                stats.symbols.push(name.clone());
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
        ExprKind::Unary { expr, .. } => visit_expr(expr, stats),
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
