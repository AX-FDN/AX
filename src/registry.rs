use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const REGISTRY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct Registry {
    root: PathBuf,
    packages: Vec<RegistryPackage>,
}

impl Registry {
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn packages(&self) -> &[RegistryPackage] {
        &self.packages
    }

    pub fn find_package(&self, name: &str) -> Option<&RegistryPackage> {
        self.packages
            .iter()
            .find(|package| package.name.eq_ignore_ascii_case(name))
    }

    pub fn search(&self, query: &str) -> Vec<&RegistryPackage> {
        let query = query.to_ascii_lowercase();
        self.packages
            .iter()
            .filter(|package| package.matches_query(&query))
            .collect()
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct RegistryIndex {
    pub schema_version: u32,
    pub packages: Vec<RegistryIndexPackage>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct RegistryIndexPackage {
    pub name: String,
    pub metadata: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct RegistryPackage {
    pub schema_version: u32,
    pub name: String,
    pub description: String,
    pub owner: String,
    pub license: String,
    pub versions: Vec<RegistryPackageVersion>,
}

impl RegistryPackage {
    pub fn latest_version(&self) -> Option<&RegistryPackageVersion> {
        self.versions.last()
    }

    pub fn find_version(&self, version: &str) -> Option<&RegistryPackageVersion> {
        self.versions
            .iter()
            .find(|candidate| candidate.version == version)
    }

    fn matches_query(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        self.name.to_ascii_lowercase().contains(query)
            || self.description.to_ascii_lowercase().contains(query)
            || self
                .versions
                .iter()
                .any(|version| version.modules.iter().any(|module| module.contains(query)))
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct RegistryPackageVersion {
    pub version: String,
    pub source: RegistryPackageSource,
    pub checksum: String,
    pub modules: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct RegistryPackageSource {
    pub kind: String,
    pub url: String,
    pub rev: String,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryIssue {
    pub code: &'static str,
    pub path: PathBuf,
    pub message: String,
}

pub fn default_registry_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("registry")
}

pub fn load_registry(path: impl AsRef<Path>) -> Result<Registry, String> {
    let root = path.as_ref().to_path_buf();
    let index_path = root.join("index.json");
    let index_text = fs::read_to_string(&index_path).map_err(|error| {
        format!(
            "failed to read registry index {}: {error}",
            index_path.display()
        )
    })?;
    let index: RegistryIndex = serde_json::from_str(&index_text).map_err(|error| {
        format!(
            "failed to parse registry index {}: {error}",
            index_path.display()
        )
    })?;

    if index.schema_version != REGISTRY_SCHEMA_VERSION {
        return Err(format!(
            "unsupported registry index schema_version `{}` in {}; expected `{}`",
            index.schema_version,
            index_path.display(),
            REGISTRY_SCHEMA_VERSION
        ));
    }

    let mut packages = Vec::new();
    for entry in index.packages {
        let metadata_path = resolve_metadata_path(&root, &entry.metadata)?;
        let metadata_text = fs::read_to_string(&metadata_path).map_err(|error| {
            format!(
                "failed to read registry package metadata {}: {error}",
                metadata_path.display()
            )
        })?;
        let package: RegistryPackage = serde_json::from_str(&metadata_text).map_err(|error| {
            format!(
                "failed to parse registry package metadata {}: {error}",
                metadata_path.display()
            )
        })?;
        if package.name != entry.name {
            return Err(format!(
                "registry index entry `{}` points to package metadata `{}` in {}",
                entry.name,
                package.name,
                metadata_path.display()
            ));
        }
        packages.push(package);
    }

    packages.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(Registry { root, packages })
}

pub fn validate_registry(path: impl AsRef<Path>) -> Vec<RegistryIssue> {
    let root = path.as_ref().to_path_buf();
    let mut issues = Vec::new();
    let index_path = root.join("index.json");
    let index_text = match fs::read_to_string(&index_path) {
        Ok(text) => text,
        Err(error) => {
            issues.push(RegistryIssue {
                code: "RG0001",
                path: index_path,
                message: format!("failed to read registry index: {error}"),
            });
            return issues;
        }
    };
    let index: RegistryIndex = match serde_json::from_str(&index_text) {
        Ok(index) => index,
        Err(error) => {
            issues.push(RegistryIssue {
                code: "RG0002",
                path: index_path,
                message: format!("failed to parse registry index JSON: {error}"),
            });
            return issues;
        }
    };

    if index.schema_version != REGISTRY_SCHEMA_VERSION {
        issues.push(RegistryIssue {
            code: "RG0003",
            path: index_path.clone(),
            message: format!(
                "registry index schema_version is `{}`, expected `{}`",
                index.schema_version, REGISTRY_SCHEMA_VERSION
            ),
        });
    }

    let mut seen_names = Vec::<String>::new();
    for entry in index.packages {
        if entry.name.trim().is_empty() {
            issues.push(RegistryIssue {
                code: "RG0004",
                path: index_path.clone(),
                message: "registry package name must not be empty".to_string(),
            });
        }
        if seen_names.iter().any(|name| name == &entry.name) {
            issues.push(RegistryIssue {
                code: "RG0005",
                path: index_path.clone(),
                message: format!("registry package `{}` is listed more than once", entry.name),
            });
        }
        seen_names.push(entry.name.clone());

        let metadata_path = match resolve_metadata_path(&root, &entry.metadata) {
            Ok(path) => path,
            Err(error) => {
                issues.push(RegistryIssue {
                    code: "RG0006",
                    path: index_path.clone(),
                    message: error,
                });
                continue;
            }
        };
        validate_package_metadata(&mut issues, &metadata_path, &entry.name);
    }

    issues
}

pub fn render_search_results(packages: &[&RegistryPackage]) -> String {
    if packages.is_empty() {
        return "no registry packages matched".to_string();
    }

    packages
        .iter()
        .map(|package| {
            let version = package
                .latest_version()
                .map(|version| version.version.as_str())
                .unwrap_or("no-version");
            format!("{} {} - {}", package.name, version, package.description)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn render_package_info(package: &RegistryPackage) -> String {
    let mut lines = vec![
        format!("package: {}", package.name),
        format!("owner: {}", package.owner),
        format!("license: {}", package.license),
        format!("description: {}", package.description),
    ];
    for version in &package.versions {
        lines.push(format!("version: {}", version.version));
        lines.push(format!(
            "source: {} {}",
            version.source.kind, version.source.url
        ));
        lines.push(format!("rev: {}", version.source.rev));
        if let Some(path) = &version.source.path {
            lines.push(format!("path: {path}"));
        }
        lines.push(format!("checksum: {}", version.checksum));
        lines.push(format!("modules: {}", version.modules.join(", ")));
    }
    lines.join("\n")
}

fn validate_package_metadata(issues: &mut Vec<RegistryIssue>, path: &Path, expected_name: &str) {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => {
            issues.push(RegistryIssue {
                code: "RG0007",
                path: path.to_path_buf(),
                message: format!("failed to read package metadata: {error}"),
            });
            return;
        }
    };
    let package: RegistryPackage = match serde_json::from_str(&text) {
        Ok(package) => package,
        Err(error) => {
            issues.push(RegistryIssue {
                code: "RG0008",
                path: path.to_path_buf(),
                message: format!("failed to parse package metadata JSON: {error}"),
            });
            return;
        }
    };

    if package.schema_version != REGISTRY_SCHEMA_VERSION {
        issues.push(RegistryIssue {
            code: "RG0009",
            path: path.to_path_buf(),
            message: format!(
                "package `{}` schema_version is `{}`, expected `{}`",
                package.name, package.schema_version, REGISTRY_SCHEMA_VERSION
            ),
        });
    }
    if package.name != expected_name {
        issues.push(RegistryIssue {
            code: "RG0010",
            path: path.to_path_buf(),
            message: format!(
                "metadata package name `{}` does not match index name `{expected_name}`",
                package.name
            ),
        });
    }
    if package.versions.is_empty() {
        issues.push(RegistryIssue {
            code: "RG0011",
            path: path.to_path_buf(),
            message: format!(
                "package `{}` must declare at least one version",
                package.name
            ),
        });
    }
    for version in &package.versions {
        if version.version.trim().is_empty() {
            issues.push(RegistryIssue {
                code: "RG0012",
                path: path.to_path_buf(),
                message: format!("package `{}` has an empty version", package.name),
            });
        }
        if version.source.kind != "git" {
            issues.push(RegistryIssue {
                code: "RG0013",
                path: path.to_path_buf(),
                message: format!(
                    "package `{}` version `{}` source kind is `{}`, expected `git`",
                    package.name, version.version, version.source.kind
                ),
            });
        }
        if version.source.url.trim().is_empty() || version.source.rev.trim().is_empty() {
            issues.push(RegistryIssue {
                code: "RG0014",
                path: path.to_path_buf(),
                message: format!(
                    "package `{}` version `{}` must pin source url and rev",
                    package.name, version.version
                ),
            });
        }
        if version
            .source
            .path
            .as_deref()
            .is_some_and(|source_path| !is_relative_package_path(source_path))
        {
            issues.push(RegistryIssue {
                code: "RG0017",
                path: path.to_path_buf(),
                message: format!(
                    "package `{}` version `{}` source path must be relative and stay inside the package repository",
                    package.name, version.version
                ),
            });
        }
        if !version.checksum.starts_with("sha256:") {
            issues.push(RegistryIssue {
                code: "RG0015",
                path: path.to_path_buf(),
                message: format!(
                    "package `{}` version `{}` checksum must start with `sha256:`",
                    package.name, version.version
                ),
            });
        }
        if version.modules.is_empty() {
            issues.push(RegistryIssue {
                code: "RG0016",
                path: path.to_path_buf(),
                message: format!(
                    "package `{}` version `{}` must list exposed modules",
                    package.name, version.version
                ),
            });
        }
    }
}

fn resolve_metadata_path(root: &Path, metadata: &str) -> Result<PathBuf, String> {
    let metadata_path = PathBuf::from(metadata);
    if metadata_path.is_absolute()
        || metadata_path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(format!(
            "registry metadata path `{metadata}` must be relative and stay inside registry root"
        ));
    }
    Ok(root.join(metadata_path))
}

fn is_relative_package_path(path: &str) -> bool {
    let path = PathBuf::from(path);
    !path.is_absolute()
        && path
            .components()
            .all(|part| !matches!(part, std::path::Component::ParentDir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_registry_loads_and_searches_packages() {
        let registry = load_registry(default_registry_dir()).expect("registry should load");
        let matches = registry.search("text");

        assert!(matches.iter().any(|package| package.name == "text_tools"));
        assert!(registry.find_package("config_rules").is_some());
    }

    #[test]
    fn built_in_registry_validates_cleanly() {
        let issues = validate_registry(default_registry_dir());
        assert_eq!(issues, Vec::new());
    }

    #[test]
    fn built_in_registry_resolves_exact_versions() {
        let registry = load_registry(default_registry_dir()).expect("registry should load");
        let package = registry
            .find_package("text_tools")
            .expect("text_tools should exist");
        let version = package
            .find_version("0.1.0")
            .expect("text_tools 0.1.0 should exist");

        assert_eq!(
            version.checksum,
            "sha256:bfcc4fbb9e8765ac9bd85b037d06360a5ae900425c11425c6fc7ff775331daf3"
        );
        assert!(package.find_version("9.9.9").is_none());
    }
}
