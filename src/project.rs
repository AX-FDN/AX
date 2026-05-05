use std::path::{Path, PathBuf};

use crate::source::SourceFile;

pub const PROJECT_MANIFEST_FILE: &str = "AX.toml";
const DEFAULT_ENTRY_FILE: &str = "src/main.ax";
const SUPPORTED_MANIFEST_VERSION: u32 = 1;

mod dependencies;
mod loader;
mod manifest;
mod sources;

use self::loader::load_project;
use self::manifest::ProjectManifest;
use self::sources::load_project_source;

#[derive(Debug, Clone)]
pub struct Project {
    root_dir: PathBuf,
    manifest_path: PathBuf,
    manifest_text: String,
    manifest: ProjectManifest,
    source_paths: Vec<PathBuf>,
    source_module_paths: Vec<(PathBuf, String)>,
    local_path_dependencies: Vec<ResolvedPathDependency>,
    registry_dependencies: Vec<ResolvedRegistryDependency>,
    entry_path: PathBuf,
}

impl Project {
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    pub fn manifest_text(&self) -> &str {
        &self.manifest_text
    }

    pub fn target_name(&self) -> &str {
        &self.manifest.package.name
    }

    pub fn source_paths(&self) -> &[PathBuf] {
        &self.source_paths
    }

    pub fn expected_module_path(&self, path: &Path) -> Option<&str> {
        self.source_module_paths
            .iter()
            .find(|(source_path, _)| source_path == path)
            .map(|(_, module_path)| module_path.as_str())
    }

    pub fn has_additional_sources(&self) -> bool {
        !self.source_paths.is_empty()
    }

    pub fn entry_path(&self) -> &Path {
        &self.entry_path
    }

    pub fn program_source_paths(&self) -> Vec<&Path> {
        let mut paths = self
            .source_paths
            .iter()
            .map(PathBuf::as_path)
            .collect::<Vec<_>>();
        paths.push(self.entry_path.as_path());
        paths
    }

    pub fn local_path_dependencies(&self) -> &[ResolvedPathDependency] {
        &self.local_path_dependencies
    }

    pub fn registry_dependencies(&self) -> &[ResolvedRegistryDependency] {
        &self.registry_dependencies
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedPathDependency {
    alias: String,
    declared_path: String,
    package_name: String,
    root_dir: PathBuf,
    manifest_path: PathBuf,
    source_paths: Vec<PathBuf>,
}

impl ResolvedPathDependency {
    pub fn alias(&self) -> &str {
        &self.alias
    }

    pub fn declared_path(&self) -> &str {
        &self.declared_path
    }

    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    pub fn source_paths(&self) -> &[PathBuf] {
        &self.source_paths
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedRegistryDependency {
    alias: String,
    registry: String,
    package_name: String,
    version: String,
    maturity: String,
    modules: Vec<String>,
}

impl ResolvedRegistryDependency {
    pub fn alias(&self) -> &str {
        &self.alias
    }

    pub fn registry(&self) -> &str {
        &self.registry
    }

    pub fn package_name(&self) -> &str {
        &self.package_name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn maturity(&self) -> &str {
        &self.maturity
    }

    pub fn modules(&self) -> &[String] {
        &self.modules
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedInput {
    pub source: SourceFile,
    pub project: Option<Project>,
}

pub fn resolve_input(path: impl AsRef<Path>) -> Result<ResolvedInput, String> {
    let path = path.as_ref();

    if path.is_dir() {
        return resolve_project_from_manifest(&path.join(PROJECT_MANIFEST_FILE));
    }

    if path
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .is_some_and(|file_name| file_name.eq_ignore_ascii_case(PROJECT_MANIFEST_FILE))
    {
        return resolve_project_from_manifest(path);
    }

    let source = SourceFile::from_path(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    Ok(ResolvedInput {
        source,
        project: None,
    })
}

pub fn resolve_project_from_manifest(manifest_path: &Path) -> Result<ResolvedInput, String> {
    let project = load_project(manifest_path)?;
    let source = load_project_source(&project)?;
    Ok(ResolvedInput {
        source,
        project: Some(project),
    })
}

#[cfg(test)]
#[path = "project/tests.rs"]
mod tests;
