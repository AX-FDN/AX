use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::ast::{Block, Expr, ExprKind, ItemKind, Program, Stmt, StmtKind};
use crate::project::{Project, ResolvedInput};
use crate::source::SourceFile;

use super::{normalize_path, normalize_path_text};

#[derive(Debug, Clone)]
pub(super) struct ResolvedUnit {
    pub(super) path: String,
    pub(super) module_path: Option<String>,
    pub(super) is_entry: bool,
    pub(super) imports: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct UnitStats {
    pub(super) function_count: usize,
    pub(super) struct_count: usize,
    pub(super) enum_count: usize,
    pub(super) function_names: Vec<String>,
    pub(super) symbols: Vec<String>,
    pub(super) host_classes: BTreeSet<String>,
    pub(super) host_builtins: BTreeSet<String>,
    pub(super) host_call_count: usize,
    pub(super) filesystem_write_builtins: BTreeSet<String>,
}

impl UnitStats {
    pub(super) fn type_count(&self) -> usize {
        self.struct_count + self.enum_count
    }
}

pub(super) fn collect_source_units(input: &ResolvedInput, program: &Program) -> Vec<ResolvedUnit> {
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

pub(super) fn collect_unit_stats(
    source: &SourceFile,
    program: &Program,
) -> BTreeMap<String, UnitStats> {
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

pub(super) fn host_boundary_class(name: &str) -> Option<&'static str> {
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

pub(super) fn is_host_heavy(stats: &UnitStats) -> bool {
    stats.host_classes.len() >= 2
        || !stats.filesystem_write_builtins.is_empty()
        || stats.host_call_count >= 3
        || stats.host_classes.contains("process")
}

pub(super) fn host_heavy_reason(stats: &UnitStats) -> String {
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
