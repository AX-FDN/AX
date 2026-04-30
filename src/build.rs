use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Serialize;

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

#[derive(Debug, Clone, Serialize)]
pub struct BuildManifest {
    pub schema_version: u32,
    pub target_name: String,
    pub entry_file: String,
    pub output_dir: String,
    pub backend: BuildBackend,
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
        schema_version: 4,
        target_name: input.target_name.clone(),
        entry_file: input.entry_file.clone(),
        output_dir: options.out_dir.display().to_string(),
        backend: BuildBackend {
            kind: "native".to_string(),
            status: "pending".to_string(),
            entrypoint: "main".to_string(),
        },
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
    use super::{build_input_from_project, default_output_dir, target_name_from_file};
    use crate::project::resolve_input;
    use std::path::{Path, PathBuf};

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
}
