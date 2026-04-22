use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::Deserialize;

use crate::source::SourceFile;

pub const PROJECT_MANIFEST_FILE: &str = "AX.toml";
const DEFAULT_ENTRY_FILE: &str = "src/main.ax";
const SUPPORTED_MANIFEST_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct Project {
    root_dir: PathBuf,
    manifest_path: PathBuf,
    manifest_text: String,
    manifest: ProjectManifest,
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

    pub fn entry_path(&self) -> &Path {
        &self.entry_path
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
    let source = SourceFile::from_path(project.entry_path()).map_err(|error| {
        format!(
            "failed to read project entry {}: {error}",
            project.entry_path().display()
        )
    })?;
    Ok(ResolvedInput {
        source,
        project: Some(project),
    })
}

fn load_project(manifest_path: &Path) -> Result<Project, String> {
    let manifest_text = fs::read_to_string(manifest_path).map_err(|error| {
        format!(
            "failed to read AX project manifest {}: {error}",
            manifest_path.display()
        )
    })?;

    let manifest: ProjectManifest = toml::from_str(&manifest_text).map_err(|error| {
        format!(
            "failed to parse AX project manifest {}: {error}",
            manifest_path.display()
        )
    })?;

    if manifest.manifest_version != SUPPORTED_MANIFEST_VERSION {
        return Err(format!(
            "unsupported AX project manifest version `{}` in {}; expected `{}`",
            manifest.manifest_version,
            manifest_path.display(),
            SUPPORTED_MANIFEST_VERSION
        ));
    }

    if manifest.package.name.trim().is_empty() {
        return Err(format!(
            "project manifest {} must declare a non-empty `[package].name`",
            manifest_path.display()
        ));
    }

    if !is_valid_package_name(&manifest.package.name) {
        return Err(format!(
            "project name `{}` in {} may only contain ASCII letters, digits, `-`, and `_`",
            manifest.package.name,
            manifest_path.display()
        ));
    }

    let root_dir = manifest_path.parent().ok_or_else(|| {
        format!(
            "failed to resolve the parent directory for {}",
            manifest_path.display()
        )
    })?;

    let entry = manifest.package.entry.trim();
    if entry.is_empty() {
        return Err(format!(
            "project manifest {} must declare a non-empty `[package].entry`",
            manifest_path.display()
        ));
    }

    let entry_relative_path = Path::new(entry);
    if entry_relative_path.is_absolute() {
        return Err(format!(
            "project entry `{entry}` in {} must be relative to the project root",
            manifest_path.display()
        ));
    }

    if entry_relative_path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(format!(
            "project entry `{entry}` in {} cannot escape the project root",
            manifest_path.display()
        ));
    }

    if entry_relative_path.extension().and_then(|ext| ext.to_str()) != Some("ax") {
        return Err(format!(
            "project entry `{entry}` in {} must point to an `.ax` source file",
            manifest_path.display()
        ));
    }

    let entry_path = root_dir.join(entry_relative_path);
    let metadata = fs::metadata(&entry_path).map_err(|error| {
        format!(
            "failed to access project entry {} declared in {}: {error}",
            entry_path.display(),
            manifest_path.display()
        )
    })?;
    if !metadata.is_file() {
        return Err(format!(
            "project entry {} declared in {} must be a file",
            entry_path.display(),
            manifest_path.display()
        ));
    }

    Ok(Project {
        root_dir: root_dir.to_path_buf(),
        manifest_path: manifest_path.to_path_buf(),
        manifest_text,
        manifest,
        entry_path,
    })
}

fn is_valid_package_name(name: &str) -> bool {
    name.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectManifest {
    manifest_version: u32,
    package: PackageManifest,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PackageManifest {
    name: String,
    #[serde(default = "default_entry")]
    entry: String,
}

fn default_entry() -> String {
    DEFAULT_ENTRY_FILE.to_string()
}

#[cfg(test)]
mod tests {
    use super::{PROJECT_MANIFEST_FILE, resolve_input};
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn resolves_project_directory_to_main_entry() {
        let project_dir = repo_root().join("examples").join("project_hello");
        let resolved = resolve_input(&project_dir).expect("project directory should resolve");

        let project = resolved
            .project
            .as_ref()
            .expect("project metadata should be available");
        assert_eq!(project.target_name(), "project_hello");
        assert!(
            resolved
                .source
                .display_path()
                .replace('\\', "/")
                .ends_with("examples/project_hello/src/main.ax")
        );
    }

    #[test]
    fn resolves_manifest_path_to_same_entry() {
        let manifest_path = repo_root()
            .join("examples")
            .join("project_hello")
            .join(PROJECT_MANIFEST_FILE);
        let resolved = resolve_input(&manifest_path).expect("manifest path should resolve");

        let project = resolved.project.expect("project metadata should exist");
        assert_eq!(project.target_name(), "project_hello");
        assert!(
            project
                .entry_path()
                .display()
                .to_string()
                .replace('\\', "/")
                .ends_with("examples/project_hello/src/main.ax")
        );
    }
}
