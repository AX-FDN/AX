use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::lockfile::check_lockfile;
use crate::project::Project;
use crate::source::SourceFile;

use super::*;

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
    let aot_ready = reproducible;
    let mut blocking_reasons = Vec::new();
    if !reproducible {
        blocking_reasons.push(format!(
            "local package graph is not reproducible because AX.lock status is `{}`",
            lock_report.status.as_str()
        ));
    }

    BuildPackageGraphReadiness {
        package_mode: "local_path_v0".to_string(),
        reproducible,
        aot_ready,
        lock_status: lock_report.status.as_str().to_string(),
        risk_level: if reproducible { "low" } else { "high" }.to_string(),
        blocking_reasons,
        recommended_commands: vec![
            "axc lock <project> --check".to_string(),
            "axc check <project>".to_string(),
            "axc build <project>".to_string(),
        ],
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
