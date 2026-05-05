use std::collections::BTreeMap;

use serde::Deserialize;

use super::DEFAULT_ENTRY_FILE;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::project) struct ProjectManifest {
    pub(in crate::project) manifest_version: u32,
    pub(in crate::project) package: PackageManifest,
    #[serde(default)]
    pub(in crate::project) dependencies: BTreeMap<String, DependencyManifest>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::project) struct PackageManifest {
    pub(in crate::project) name: String,
    #[serde(default = "default_entry")]
    pub(in crate::project) entry: String,
    #[serde(default)]
    pub(in crate::project) sources: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(in crate::project) enum DependencyManifest {
    Path(PathDependencyManifest),
    Registry(RegistryDependencyManifest),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::project) struct PathDependencyManifest {
    pub(in crate::project) path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::project) struct RegistryDependencyManifest {
    #[serde(default = "default_registry")]
    pub(in crate::project) registry: String,
    pub(in crate::project) version: String,
}

fn default_entry() -> String {
    DEFAULT_ENTRY_FILE.to_string()
}

fn default_registry() -> String {
    "ax".to_string()
}
