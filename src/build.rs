use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Serialize;

use crate::ast::{self, Program as AstProgram};
use crate::hir::Program as HirProgram;
use crate::lockfile::check_lockfile;
use crate::mir::Program as MirProgram;
use crate::project::Project;
use crate::source::SourceFile;

const BUILD_MANIFEST_FILE: &str = "build-manifest.json";
const SOURCE_COPY_FILE: &str = "source.ax";
const PROJECT_SOURCES_DIR: &str = "project-sources";
const HIR_FILE: &str = "program.hir.json";
const MIR_FILE: &str = "program.mir.json";

#[derive(Debug, Clone)]
pub struct BuildOptions {
    pub out_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct BuildResult {
    pub manifest_path: PathBuf,
    pub manifest: BuildManifest,
}

#[derive(Debug, Clone)]
pub struct BuildInput {
    pub target_name: String,
    pub entry_file: String,
    pub project_manifest: Option<ProjectManifestArtifact>,
    pub project_sources: Option<ProjectSourcesArtifact>,
    pub local_path_packages: Vec<LocalPathPackageArtifact>,
    pub package_graph_readiness: Option<BuildPackageGraphReadiness>,
}

#[derive(Debug, Clone)]
pub struct ProjectManifestArtifact {
    pub file_name: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ProjectSourcesArtifact {
    pub dir_name: String,
    pub files: Vec<ProjectSourceArtifact>,
}

#[derive(Debug, Clone)]
pub struct ProjectSourceArtifact {
    pub relative_path: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalPathPackageArtifact {
    pub alias: String,
    pub root: String,
    pub manifest: String,
    pub source_count: usize,
    pub modules: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildPackageGraphReadiness {
    pub package_mode: String,
    pub reproducible: bool,
    pub aot_ready: bool,
    pub lock_status: String,
    pub risk_level: String,
    pub blocking_reasons: Vec<String>,
    pub recommended_commands: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct AotReadinessInput<'a> {
    pub is_project: bool,
    pub has_local_path_packages: bool,
    pub package_lock_status: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AotReadiness {
    pub schema_version: u32,
    pub stage: String,
    pub status: String,
    pub executable_emission: bool,
    pub planned_executable_artifact: bool,
    pub single_file_core_candidate: bool,
    pub required_backend_features: Vec<String>,
    pub blockers: Vec<AotReadinessBlocker>,
    pub recommended_next_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AotReadinessBlocker {
    pub code: String,
    pub category: String,
    pub message: String,
    pub required_stage: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildManifest {
    pub schema_version: u32,
    pub target_name: String,
    pub entry_file: String,
    pub output_dir: String,
    pub backend: BuildBackend,
    pub aot_readiness: AotReadiness,
    pub artifacts: BuildArtifacts,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub local_path_packages: Vec<LocalPathPackageArtifact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_graph_readiness: Option<BuildPackageGraphReadiness>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildBackend {
    pub kind: String,
    pub status: String,
    pub entrypoint: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildArtifacts {
    pub source_copy: String,
    pub hir_json: String,
    pub mir_json: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_manifest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_sources_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_sources: Option<Vec<String>>,
    pub planned_executable: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable: Option<String>,
}

pub fn default_output_dir(target_name: &str) -> Result<PathBuf, String> {
    let cwd = std::env::current_dir()
        .map_err(|error| format!("failed to resolve current directory: {error}"))?;
    Ok(cwd.join("build").join(target_name))
}

pub fn target_name_from_file(input_file: &Path) -> Result<String, String> {
    let Some(stem) = input_file.file_stem().and_then(|stem| stem.to_str()) else {
        return Err(format!(
            "could not derive a build target name from {}",
            input_file.display()
        ));
    };

    if stem.is_empty() {
        return Err(format!(
            "could not derive a build target name from {}",
            input_file.display()
        ));
    }

    Ok(stem.to_string())
}

pub fn build_input_from_source(source: &SourceFile) -> Result<BuildInput, String> {
    Ok(BuildInput {
        target_name: target_name_from_file(source.path())?,
        entry_file: source.display_path(),
        project_manifest: None,
        project_sources: None,
        local_path_packages: Vec::new(),
        package_graph_readiness: None,
    })
}

pub fn build_input_from_project(
    _source: &SourceFile,
    project: &Project,
) -> Result<BuildInput, String> {
    let file_name = project
        .manifest_path()
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("AX.toml")
        .to_string();

    let mut project_source_files = Vec::new();
    for path in project.program_source_paths() {
        let text = fs::read_to_string(path).map_err(|error| {
            format!(
                "failed to read project source {} for build packaging: {error}",
                path.display()
            )
        })?;
        let relative_path = build_project_source_artifact_path(project.root_dir(), path)?;
        project_source_files.push(ProjectSourceArtifact {
            relative_path,
            text,
        });
    }

    let mut local_path_packages = Vec::new();
    for dependency in project.local_path_dependencies() {
        let mut modules = dependency
            .source_paths()
            .iter()
            .filter_map(|path| {
                project
                    .expected_module_path(path)
                    .map(std::string::ToString::to_string)
            })
            .collect::<Vec<_>>();
        modules.sort();

        local_path_packages.push(LocalPathPackageArtifact {
            alias: dependency.alias().to_string(),
            root: build_project_source_artifact_path(project.root_dir(), dependency.root_dir())?,
            manifest: build_project_source_artifact_path(
                project.root_dir(),
                dependency.manifest_path(),
            )?,
            source_count: dependency.source_paths().len(),
            modules,
        });
    }
    let package_graph_readiness = if project.local_path_dependencies().is_empty() {
        None
    } else {
        Some(build_package_graph_readiness(project))
    };

    Ok(BuildInput {
        target_name: project.target_name().to_string(),
        entry_file: project.entry_path().display().to_string(),
        project_manifest: Some(ProjectManifestArtifact {
            file_name,
            text: project.manifest_text().to_string(),
        }),
        project_sources: Some(ProjectSourcesArtifact {
            dir_name: PROJECT_SOURCES_DIR.to_string(),
            files: project_source_files,
        }),
        local_path_packages,
        package_graph_readiness,
    })
}

fn build_package_graph_readiness(project: &Project) -> BuildPackageGraphReadiness {
    let lock_report = check_lockfile(project);
    let reproducible = lock_report.status.as_str() == "current";
    let mut blocking_reasons = Vec::new();
    if !reproducible {
        blocking_reasons.push(format!(
            "local package graph is not reproducible because AX.lock status is `{}`",
            lock_report.status.as_str()
        ));
    }
    blocking_reasons
        .push("native backend has not implemented local path package linking".to_string());

    BuildPackageGraphReadiness {
        package_mode: "local_path_v0".to_string(),
        reproducible,
        aot_ready: false,
        lock_status: lock_report.status.as_str().to_string(),
        risk_level: if reproducible { "medium" } else { "high" }.to_string(),
        blocking_reasons,
        recommended_commands: vec![
            "axc lock <project> --check".to_string(),
            "axc check <project>".to_string(),
            "axc build <project>".to_string(),
        ],
    }
}

pub fn assess_aot_readiness(program: &AstProgram, input: AotReadinessInput<'_>) -> AotReadiness {
    let mut features = BTreeSet::new();
    collect_aot_features(program, &mut features);

    if input.is_project {
        features.insert("project_sources".to_string());
    }
    if input.has_local_path_packages {
        features.insert("local_path_packages".to_string());
    }
    if input.package_lock_status.is_some() {
        features.insert("package_lock".to_string());
    }

    let mut blockers = vec![aot_blocker(
        "AOT0001",
        "backend",
        "native executable emission is not implemented yet",
        "Build-1",
    )];

    if input.is_project {
        blockers.push(aot_blocker(
            "AOT0101",
            "project",
            "project source graph packaging exists, but native project linking semantics are not implemented",
            "Build-2",
        ));
    }
    if input.has_local_path_packages {
        blockers.push(aot_blocker(
            "AOT0102",
            "package",
            "local path package sources are loadable, but native package linking semantics are not implemented",
            "Build-2/P5",
        ));
        if input.package_lock_status != Some("current") {
            blockers.push(aot_blocker(
                "AOT0103",
                "package",
                "local package graph must have a current AX.lock before it can be treated as reproducible AOT input",
                "P5",
            ));
        }
    }
    if feature_starts_with(&features, "generic_") || features.contains("generic_type_instances") {
        blockers.push(aot_blocker(
            "AOT0201",
            "language",
            "generic monomorphization and type-argument lowering are not frozen for native backend input",
            "Build-1/Build-2",
        ));
    }
    if features.contains("traits")
        || features.contains("trait_bounds")
        || features.contains("trait_impls")
    {
        blockers.push(aot_blocker(
            "AOT0202",
            "language",
            "trait/interface lowering, bound dispatch, and trait impl layout are not frozen for native backend input",
            "Build-2",
        ));
    }
    if features.contains("impl_methods") || features.contains("generic_methods") {
        blockers.push(aot_blocker(
            "AOT0203",
            "language",
            "impl method lowering and method ABI are not frozen for native backend input",
            "Build-2",
        ));
    }
    if features.contains("payload_enums")
        || features.contains("match_expressions")
        || features.contains("match_statements")
        || features.contains("struct_patterns")
        || features.contains("or_patterns")
        || features.contains("range_patterns")
        || features.contains("match_guards")
        || features.contains("enum_patterns")
        || features.contains("payload_enum_patterns")
        || features.contains("result_values")
        || features.contains("option_values")
    {
        blockers.push(aot_blocker(
            "AOT0204",
            "language",
            "enum layout, pattern tests, and match lowering need a native backend contract",
            "Build-2",
        ));
    }
    if features.contains("result_propagation") {
        blockers.push(aot_blocker(
            "AOT0205",
            "language",
            "`?` result propagation needs explicit native lowering for early-return control flow",
            "Build-2",
        ));
    }
    if feature_starts_with(&features, "host_") {
        blockers.push(aot_blocker(
            "AOT0301",
            "runtime",
            "host boundary builtins need a native runtime ABI before AOT can preserve check/run behavior",
            "Build-2/Build-3",
        ));
    }
    if features.contains("string_runtime") || features.contains("string_list_runtime") {
        blockers.push(aot_blocker(
            "AOT0302",
            "runtime",
            "string and string_list values need a native runtime representation and ABI",
            "Build-1/Build-2",
        ));
    }

    let single_file_core_candidate = !input.is_project
        && !input.has_local_path_packages
        && !features.iter().any(|feature| {
            matches!(
                feature.as_str(),
                "arrays"
                    | "slices"
                    | "structs"
                    | "enums"
                    | "payload_enums"
                    | "match_expressions"
                    | "match_statements"
                    | "result_propagation"
                    | "impl_methods"
                    | "traits"
                    | "trait_bounds"
                    | "trait_impls"
                    | "string_runtime"
                    | "string_list_runtime"
                    | "project_sources"
                    | "local_path_packages"
            ) || feature.starts_with("generic_")
                || (feature.starts_with("host_") && feature != "host_stdio")
        });

    AotReadiness {
        schema_version: 1,
        stage: "Build-0 skeleton".to_string(),
        status: "blocked".to_string(),
        executable_emission: false,
        planned_executable_artifact: true,
        single_file_core_candidate,
        required_backend_features: features.into_iter().collect(),
        blockers,
        recommended_next_steps: vec![
            "freeze the MIR subset for a single-file i32 main AOT prototype".to_string(),
            "keep axc run as the semantic reference while native output is pending".to_string(),
            "when packages are present, require axc lock <project> --check before AOT planning"
                .to_string(),
        ],
    }
}

fn aot_blocker(
    code: &str,
    category: &str,
    message: &str,
    required_stage: &str,
) -> AotReadinessBlocker {
    AotReadinessBlocker {
        code: code.to_string(),
        category: category.to_string(),
        message: message.to_string(),
        required_stage: required_stage.to_string(),
    }
}

fn feature_starts_with(features: &BTreeSet<String>, prefix: &str) -> bool {
    features.iter().any(|feature| feature.starts_with(prefix))
}

fn collect_aot_features(program: &AstProgram, features: &mut BTreeSet<String>) {
    if program.source_units.len() > 1 {
        features.insert("multi_source_program".to_string());
    }
    if program
        .source_units
        .iter()
        .any(|unit| unit.module.is_some() || !unit.imports.is_empty())
    {
        features.insert("module_imports".to_string());
    }

    for item in &program.items {
        collect_item_aot_features(&item.kind, features);
    }
}

fn collect_item_aot_features(kind: &ast::ItemKind, features: &mut BTreeSet<String>) {
    match kind {
        ast::ItemKind::Function {
            type_params,
            type_param_bounds,
            params,
            return_type,
            body,
            ..
        } => {
            features.insert("functions".to_string());
            if !type_params.is_empty() {
                features.insert("generic_functions".to_string());
            }
            if !type_param_bounds.is_empty() {
                features.insert("trait_bounds".to_string());
            }
            for bound in type_param_bounds {
                collect_type_ref_aot_features(&bound.trait_ref, features);
            }
            for param in params {
                collect_type_ref_aot_features(&param.ty, features);
            }
            collect_type_ref_aot_features(return_type, features);
            collect_block_aot_features(body, features);
        }
        ast::ItemKind::Const { ty, value, .. } => {
            features.insert("consts".to_string());
            collect_type_ref_aot_features(ty, features);
            collect_expr_aot_features(value, features);
        }
        ast::ItemKind::TypeAlias {
            type_params,
            target,
            ..
        } => {
            features.insert("type_aliases".to_string());
            if !type_params.is_empty() {
                features.insert("generic_type_aliases".to_string());
            }
            collect_type_ref_aot_features(target, features);
        }
        ast::ItemKind::Struct {
            type_params,
            fields,
            ..
        } => {
            features.insert("structs".to_string());
            if !type_params.is_empty() {
                features.insert("generic_structs".to_string());
            }
            for field in fields {
                collect_type_ref_aot_features(&field.ty, features);
            }
        }
        ast::ItemKind::Enum {
            type_params,
            variants,
            ..
        } => {
            features.insert("enums".to_string());
            if !type_params.is_empty() {
                features.insert("generic_enums".to_string());
            }
            for variant in variants {
                if let Some(payload) = &variant.payload {
                    features.insert("payload_enums".to_string());
                    collect_type_ref_aot_features(payload, features);
                }
            }
        }
        ast::ItemKind::Trait { methods, .. } => {
            features.insert("traits".to_string());
            for method in methods {
                for param in &method.params {
                    collect_type_ref_aot_features(&param.ty, features);
                }
                collect_type_ref_aot_features(&method.return_type, features);
            }
        }
        ast::ItemKind::Impl {
            type_params,
            trait_ref,
            target,
            methods,
        } => {
            features.insert("impl_methods".to_string());
            if !type_params.is_empty() {
                features.insert("generic_impls".to_string());
            }
            if let Some(trait_ref) = trait_ref {
                features.insert("trait_impls".to_string());
                collect_type_ref_aot_features(trait_ref, features);
            }
            collect_type_ref_aot_features(target, features);
            for method in methods {
                if !method.type_params.is_empty() {
                    features.insert("generic_methods".to_string());
                }
                for param in &method.params {
                    collect_type_ref_aot_features(&param.ty, features);
                }
                collect_type_ref_aot_features(&method.return_type, features);
                collect_block_aot_features(&method.body, features);
            }
        }
    }
}

fn collect_block_aot_features(block: &ast::Block, features: &mut BTreeSet<String>) {
    for statement in &block.statements {
        collect_stmt_aot_features(statement, features);
    }
}

fn collect_stmt_aot_features(statement: &ast::Stmt, features: &mut BTreeSet<String>) {
    match &statement.kind {
        ast::StmtKind::Let {
            ty, initializer, ..
        } => {
            collect_type_ref_aot_features(ty, features);
            collect_expr_aot_features(initializer, features);
        }
        ast::StmtKind::Assign { target, value } => {
            collect_expr_aot_features(target, features);
            collect_expr_aot_features(value, features);
        }
        ast::StmtKind::Expr { expr } => collect_expr_aot_features(expr, features),
        ast::StmtKind::Return { value } => {
            if let Some(value) = value {
                collect_expr_aot_features(value, features);
            }
        }
        ast::StmtKind::Break | ast::StmtKind::Continue => {
            features.insert("loop_control".to_string());
        }
        ast::StmtKind::Match { scrutinee, arms } => {
            features.insert("match_statements".to_string());
            collect_expr_aot_features(scrutinee, features);
            for arm in arms {
                collect_match_pattern_aot_features(&arm.pattern, features);
                if arm.guard.is_some() {
                    features.insert("match_guards".to_string());
                }
                if let Some(guard) = &arm.guard {
                    collect_expr_aot_features(guard, features);
                }
                collect_block_aot_features(&arm.body, features);
            }
        }
        ast::StmtKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            features.insert("control_flow".to_string());
            collect_expr_aot_features(condition, features);
            collect_block_aot_features(then_branch, features);
            if let Some(else_branch) = else_branch {
                collect_block_aot_features(else_branch, features);
            }
        }
        ast::StmtKind::While { condition, body } => {
            features.insert("loops".to_string());
            collect_expr_aot_features(condition, features);
            collect_block_aot_features(body, features);
        }
        ast::StmtKind::For {
            initializer,
            condition,
            step,
            body,
        } => {
            features.insert("loops".to_string());
            if let Some(initializer) = initializer {
                collect_stmt_aot_features(initializer, features);
            }
            if let Some(condition) = condition {
                collect_expr_aot_features(condition, features);
            }
            if let Some(step) = step {
                collect_stmt_aot_features(step, features);
            }
            collect_block_aot_features(body, features);
        }
        ast::StmtKind::ForIn {
            binding,
            iterable,
            body,
        } => {
            features.insert("for_in".to_string());
            collect_type_ref_aot_features(&binding.ty, features);
            collect_expr_aot_features(iterable, features);
            collect_block_aot_features(body, features);
        }
        ast::StmtKind::Block { block } => collect_block_aot_features(block, features),
    }
}

fn collect_expr_aot_features(expression: &ast::Expr, features: &mut BTreeSet<String>) {
    match &expression.kind {
        ast::ExprKind::Int { .. } => {
            features.insert("i32_values".to_string());
        }
        ast::ExprKind::Float { .. } => {
            features.insert("f32_values".to_string());
        }
        ast::ExprKind::Bool { .. } => {
            features.insert("bool_values".to_string());
        }
        ast::ExprKind::String { .. } => {
            features.insert("string_runtime".to_string());
        }
        ast::ExprKind::Name { .. } | ast::ExprKind::Error => {}
        ast::ExprKind::Unary { expr, .. } => collect_expr_aot_features(expr, features),
        ast::ExprKind::Try { expr } => {
            features.insert("result_propagation".to_string());
            collect_expr_aot_features(expr, features);
        }
        ast::ExprKind::Binary { left, right, .. } => {
            collect_expr_aot_features(left, features);
            collect_expr_aot_features(right, features);
        }
        ast::ExprKind::Call { callee, arguments } => {
            if let Some(name) = callee.qualified_name() {
                collect_call_aot_features(&name, features);
            }
            collect_expr_aot_features(callee, features);
            for argument in arguments {
                collect_expr_aot_features(argument, features);
            }
        }
        ast::ExprKind::StructLiteral { fields, .. } => {
            features.insert("structs".to_string());
            for field in fields {
                collect_expr_aot_features(&field.value, features);
            }
        }
        ast::ExprKind::ArrayLiteral { elements } => {
            features.insert("arrays".to_string());
            for element in elements {
                collect_expr_aot_features(element, features);
            }
        }
        ast::ExprKind::Block { statements, value } => {
            for statement in statements {
                collect_stmt_aot_features(statement, features);
            }
            collect_expr_aot_features(value, features);
        }
        ast::ExprKind::Match { scrutinee, arms } => {
            features.insert("match_expressions".to_string());
            collect_expr_aot_features(scrutinee, features);
            for arm in arms {
                collect_match_pattern_aot_features(&arm.pattern, features);
                if arm.guard.is_some() {
                    features.insert("match_guards".to_string());
                }
                if let Some(guard) = &arm.guard {
                    collect_expr_aot_features(guard, features);
                }
                collect_expr_aot_features(&arm.value, features);
            }
        }
        ast::ExprKind::Field { base, .. } => collect_expr_aot_features(base, features),
        ast::ExprKind::Index { base, index } => {
            features.insert("arrays".to_string());
            collect_expr_aot_features(base, features);
            collect_expr_aot_features(index, features);
        }
        ast::ExprKind::Slice { base, start, end } => {
            features.insert("slices".to_string());
            collect_expr_aot_features(base, features);
            collect_expr_aot_features(start, features);
            collect_expr_aot_features(end, features);
        }
    }
}

fn collect_match_pattern_aot_features(
    pattern: &ast::MatchPattern,
    features: &mut BTreeSet<String>,
) {
    match &pattern.kind {
        ast::MatchPatternKind::Wildcard
        | ast::MatchPatternKind::Bool { .. }
        | ast::MatchPatternKind::Int { .. }
        | ast::MatchPatternKind::String { .. }
        | ast::MatchPatternKind::Error => {}
        ast::MatchPatternKind::Binding { .. } => {
            features.insert("pattern_bindings".to_string());
        }
        ast::MatchPatternKind::IntRange { .. } => {
            features.insert("range_patterns".to_string());
        }
        ast::MatchPatternKind::EnumVariant { payload, .. } => {
            features.insert("enum_patterns".to_string());
            if let Some(payload) = payload {
                features.insert("payload_enum_patterns".to_string());
                if matches!(payload, ast::EnumVariantPayloadPattern::Binding { .. }) {
                    features.insert("pattern_bindings".to_string());
                }
            }
        }
        ast::MatchPatternKind::Struct { fields, .. } => {
            features.insert("struct_patterns".to_string());
            if !fields.is_empty() {
                features.insert("pattern_bindings".to_string());
            }
        }
        ast::MatchPatternKind::Or { alternatives } => {
            features.insert("or_patterns".to_string());
            for alternative in alternatives {
                collect_match_pattern_aot_features(alternative, features);
            }
        }
    }
}

fn collect_type_ref_aot_features(ty: &ast::TypeRef, features: &mut BTreeSet<String>) {
    if let Some(name) = &ty.name {
        match name.as_str() {
            "bool" => {
                features.insert("bool_values".to_string());
            }
            "i32" => {
                features.insert("i32_values".to_string());
            }
            "f32" => {
                features.insert("f32_values".to_string());
            }
            "string" => {
                features.insert("string_runtime".to_string());
            }
            "string_list" => {
                features.insert("string_list_runtime".to_string());
            }
            _ => {
                if name.ends_with("Result") || name.ends_with(".Result") {
                    features.insert("result_values".to_string());
                }
                if name.ends_with("Option") || name.ends_with(".Option") {
                    features.insert("option_values".to_string());
                }
            }
        }
    }
    if !ty.type_args.is_empty() {
        features.insert("generic_type_instances".to_string());
        for arg in &ty.type_args {
            collect_type_ref_aot_features(arg, features);
        }
    }
    if let Some(element) = &ty.element {
        if ty.length.is_some() {
            features.insert("arrays".to_string());
        } else {
            features.insert("slices".to_string());
        }
        collect_type_ref_aot_features(element, features);
    }
}

fn collect_call_aot_features(name: &str, features: &mut BTreeSet<String>) {
    if name == "println" {
        features.insert("host_stdio".to_string());
    }
    if name == "argv_len" || name == "argv_get" || name.starts_with("std.cli.") {
        features.insert("host_argv".to_string());
    }
    if name.starts_with("env_") || name.starts_with("std.env.") {
        features.insert("host_env".to_string());
    }
    if name.starts_with("fs_") || name.starts_with("std.fs.") {
        features.insert("host_fs".to_string());
    }
    if name.starts_with("process_") || name.starts_with("std.process.") {
        features.insert("host_process".to_string());
    }
    if name.starts_with("string_list_") || name.starts_with("std.collections.") {
        features.insert("string_list_runtime".to_string());
    }
    if name.starts_with("string_")
        || name == "to_string"
        || name.starts_with("std.text.")
        || name.starts_with("std.report.")
        || name.starts_with("std.path.")
    {
        features.insert("string_runtime".to_string());
    }
}

fn build_project_source_artifact_path(
    project_root: &Path,
    source_path: &Path,
) -> Result<String, String> {
    if let Ok(relative_path) = source_path.strip_prefix(project_root) {
        return Ok(relative_path.to_string_lossy().replace('\\', "/"));
    }

    let project_components = project_root.components().collect::<Vec<_>>();
    let source_components = source_path.components().collect::<Vec<_>>();
    let mut common_len = 0;
    while common_len < project_components.len()
        && common_len < source_components.len()
        && project_components[common_len] == source_components[common_len]
    {
        common_len += 1;
    }

    let mut artifact_path = PathBuf::from("external");
    for component in &source_components[common_len..] {
        match component {
            Component::Normal(part) => artifact_path.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(format!(
                    "failed to package project source {}: normalized source path still contains parent traversal",
                    source_path.display()
                ));
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err(format!(
                    "failed to package project source {}: source path does not share a copyable root with the project",
                    source_path.display()
                ));
            }
        }
    }

    if artifact_path == PathBuf::from("external") {
        return Err(format!(
            "failed to package project source {}: could not derive a relative artifact path",
            source_path.display()
        ));
    }

    Ok(artifact_path.to_string_lossy().replace('\\', "/"))
}

pub fn build_program(
    source: &SourceFile,
    program: &AstProgram,
    hir: &HirProgram,
    mir: &MirProgram,
    input: &BuildInput,
    options: &BuildOptions,
) -> Result<BuildResult, String> {
    fs::create_dir_all(&options.out_dir).map_err(|error| {
        format!(
            "failed to create build output directory {}: {error}",
            options.out_dir.display()
        )
    })?;

    let bin_dir = options.out_dir.join("bin");
    fs::create_dir_all(&bin_dir).map_err(|error| {
        format!(
            "failed to create build bin directory {}: {error}",
            bin_dir.display()
        )
    })?;

    let source_copy_path = options.out_dir.join(SOURCE_COPY_FILE);
    fs::write(&source_copy_path, source.text()).map_err(|error| {
        format!(
            "failed to write build source copy {}: {error}",
            source_copy_path.display()
        )
    })?;

    let hir_path = options.out_dir.join(HIR_FILE);
    let hir_text = serde_json::to_string_pretty(hir)
        .map_err(|error| format!("failed to serialize HIR for build output: {error}"))?;
    fs::write(&hir_path, format!("{hir_text}\n"))
        .map_err(|error| format!("failed to write build HIR {}: {error}", hir_path.display()))?;

    let mir_path = options.out_dir.join(MIR_FILE);
    let mir_text = serde_json::to_string_pretty(mir)
        .map_err(|error| format!("failed to serialize MIR for build output: {error}"))?;
    fs::write(&mir_path, format!("{mir_text}\n"))
        .map_err(|error| format!("failed to write build MIR {}: {error}", mir_path.display()))?;

    if let Some(project_manifest) = &input.project_manifest {
        let project_manifest_path = options.out_dir.join(&project_manifest.file_name);
        fs::write(&project_manifest_path, &project_manifest.text).map_err(|error| {
            format!(
                "failed to write copied project manifest {}: {error}",
                project_manifest_path.display()
            )
        })?;
    }

    if let Some(project_sources) = &input.project_sources {
        let project_sources_dir = options.out_dir.join(&project_sources.dir_name);
        fs::create_dir_all(&project_sources_dir).map_err(|error| {
            format!(
                "failed to create copied project sources directory {}: {error}",
                project_sources_dir.display()
            )
        })?;

        for project_source in &project_sources.files {
            let copied_path = project_sources_dir.join(&project_source.relative_path);
            if let Some(parent) = copied_path.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    format!(
                        "failed to create copied project source directory {}: {error}",
                        parent.display()
                    )
                })?;
            }
            fs::write(&copied_path, &project_source.text).map_err(|error| {
                format!(
                    "failed to write copied project source {}: {error}",
                    copied_path.display()
                )
            })?;
        }
    }

    let manifest = BuildManifest {
        schema_version: 5,
        target_name: input.target_name.clone(),
        entry_file: input.entry_file.clone(),
        output_dir: options.out_dir.display().to_string(),
        backend: BuildBackend {
            kind: "native".to_string(),
            status: "pending".to_string(),
            entrypoint: "main".to_string(),
        },
        aot_readiness: assess_aot_readiness(
            program,
            AotReadinessInput {
                is_project: input.project_manifest.is_some(),
                has_local_path_packages: !input.local_path_packages.is_empty(),
                package_lock_status: input
                    .package_graph_readiness
                    .as_ref()
                    .map(|readiness| readiness.lock_status.as_str()),
            },
        ),
        artifacts: BuildArtifacts {
            source_copy: SOURCE_COPY_FILE.to_string(),
            hir_json: HIR_FILE.to_string(),
            mir_json: MIR_FILE.to_string(),
            project_manifest: input
                .project_manifest
                .as_ref()
                .map(|artifact| artifact.file_name.clone()),
            project_sources_dir: input
                .project_sources
                .as_ref()
                .map(|artifact| artifact.dir_name.clone()),
            project_sources: input.project_sources.as_ref().map(|artifact| {
                artifact
                    .files
                    .iter()
                    .map(|file| file.relative_path.clone())
                    .collect()
            }),
            planned_executable: format!("bin/{}{}", input.target_name, executable_suffix()),
            executable: None,
        },
        local_path_packages: input.local_path_packages.clone(),
        package_graph_readiness: input.package_graph_readiness.clone(),
        notes: vec![
            "This build currently emits frontend and midend stable artifacts only.".to_string(),
            "Native executable emission will be added in the future backend stage.".to_string(),
        ],
    };

    let manifest_path = options.out_dir.join(BUILD_MANIFEST_FILE);
    let manifest_text = serde_json::to_string_pretty(&manifest)
        .map_err(|error| format!("failed to serialize build manifest: {error}"))?;
    fs::write(&manifest_path, format!("{manifest_text}\n")).map_err(|error| {
        format!(
            "failed to write build manifest {}: {error}",
            manifest_path.display()
        )
    })?;

    Ok(BuildResult {
        manifest_path,
        manifest,
    })
}

fn executable_suffix() -> &'static str {
    if cfg!(windows) { ".exe" } else { "" }
}

#[cfg(test)]
mod tests {
    use super::{
        AotReadiness, AotReadinessInput, assess_aot_readiness, build_input_from_project,
        default_output_dir, target_name_from_file,
    };
    use crate::frontend::analyze;
    use crate::project::resolve_input;
    use crate::source::SourceFile;
    use std::path::{Path, PathBuf};

    fn readiness_for(source_text: &str, input: AotReadinessInput<'_>) -> AotReadiness {
        let source = SourceFile::anonymous(source_text);
        let output = analyze(&source);
        assert!(
            output.diagnostics.is_empty(),
            "test source should analyze cleanly: {:?}",
            output.diagnostics
        );
        assess_aot_readiness(&output.program, input)
    }

    fn blocker_codes(readiness: &AotReadiness) -> Vec<&str> {
        readiness
            .blockers
            .iter()
            .map(|blocker| blocker.code.as_str())
            .collect()
    }

    #[test]
    fn derives_target_name_from_input_path() {
        assert_eq!(
            target_name_from_file(Path::new("examples/hello.ax"))
                .expect("target name should exist"),
            "hello"
        );
    }

    #[test]
    fn default_output_dir_uses_build_root_and_target_name() {
        let output_dir = default_output_dir("hello").expect("default output dir should resolve");
        let rendered = output_dir.display().to_string().replace('\\', "/");
        assert!(rendered.ends_with("/build/hello"));
    }

    #[test]
    fn packages_shared_sibling_support_sources_under_external_prefix() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let resolved = resolve_input(
            repo_root
                .join("examples")
                .join("project_workspace_search_report"),
        )
        .expect("project input should resolve");
        let project = resolved
            .project
            .as_ref()
            .expect("project metadata should be available");

        let build_input = build_input_from_project(&resolved.source, project)
            .expect("build input should package project sources");
        let project_sources = build_input
            .project_sources
            .expect("project sources artifact should exist");
        let relative_paths = project_sources
            .files
            .into_iter()
            .map(|file| file.relative_path)
            .collect::<Vec<_>>();

        assert!(relative_paths.contains(&"external/foundation/cli.ax".to_string()));
        assert!(relative_paths.contains(&"external/foundation/file_kind.ax".to_string()));
        assert!(relative_paths.contains(&"external/foundation/report.ax".to_string()));
        assert!(relative_paths.contains(&"external/foundation/search.ax".to_string()));
        assert!(relative_paths.contains(&"external/foundation/text.ax".to_string()));
        assert!(relative_paths.contains(&"external/foundation/workspace.ax".to_string()));
        assert!(relative_paths.contains(&"lib/file_search.ax".to_string()));
        assert!(relative_paths.contains(&"src/main.ax".to_string()));
    }

    #[test]
    fn aot_readiness_marks_single_file_stdio_as_core_candidate() {
        let readiness = readiness_for(
            "\
fn main() -> i32 {
    println(1);
    return 0;
}
",
            AotReadinessInput {
                is_project: false,
                has_local_path_packages: false,
                package_lock_status: None,
            },
        );

        assert!(readiness.single_file_core_candidate);
        assert_eq!(
            readiness.required_backend_features,
            vec![
                "functions".to_string(),
                "host_stdio".to_string(),
                "i32_values".to_string()
            ]
        );
        assert_eq!(blocker_codes(&readiness), vec!["AOT0001", "AOT0301"]);
    }

    #[test]
    fn aot_readiness_reports_project_package_and_generic_blockers() {
        let readiness = readiness_for(
            "\
fn id<T>(value: T) -> T {
    return value;
}

fn main() -> i32 {
    return id(1);
}
",
            AotReadinessInput {
                is_project: true,
                has_local_path_packages: true,
                package_lock_status: Some("missing"),
            },
        );

        assert!(!readiness.single_file_core_candidate);
        assert!(
            readiness
                .required_backend_features
                .contains(&"generic_functions".to_string())
        );
        assert_eq!(
            blocker_codes(&readiness),
            vec!["AOT0001", "AOT0101", "AOT0102", "AOT0103", "AOT0201"]
        );
    }
}
