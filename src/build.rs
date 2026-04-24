use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::hir::Program as HirProgram;
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
pub struct BuildManifest {
    pub schema_version: u32,
    pub target_name: String,
    pub entry_file: String,
    pub output_dir: String,
    pub backend: BuildBackend,
    pub artifacts: BuildArtifacts,
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
        let relative_path = path
            .strip_prefix(project.root_dir())
            .map_err(|error| {
                format!(
                    "failed to compute project-relative source path for {}: {error}",
                    path.display()
                )
            })?
            .to_string_lossy()
            .replace('\\', "/");
        project_source_files.push(ProjectSourceArtifact {
            relative_path,
            text,
        });
    }

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
    })
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
        schema_version: 3,
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
    use super::{default_output_dir, target_name_from_file};
    use std::path::Path;

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
}
