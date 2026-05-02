use std::fs;

use super::manifest::ProjectManifest;
use super::*;

pub(in crate::project) fn is_valid_package_name(name: &str) -> bool {
    name.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

pub(in crate::project) fn validate_dependency_alias(
    alias: &str,
    manifest_path: &Path,
) -> Result<(), String> {
    if alias.is_empty() {
        return Err(project_package_error(
            "PX0001",
            format!(
                "dependency alias in {} must not be empty",
                manifest_path.display()
            ),
        ));
    }

    let mut chars = alias.chars();
    let Some(first) = chars.next() else {
        return Err(project_package_error(
            "PX0001",
            format!(
                "dependency alias in {} must not be empty",
                manifest_path.display()
            ),
        ));
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(project_package_error(
            "PX0001",
            format!(
                "dependency alias `{alias}` in {} must start with an ASCII letter or `_`",
                manifest_path.display()
            ),
        ));
    }
    if !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
        return Err(project_package_error(
            "PX0001",
            format!(
                "dependency alias `{alias}` in {} may only contain ASCII letters, digits, and `_` because it becomes an AX module root",
                manifest_path.display()
            ),
        ));
    }

    Ok(())
}

pub(in crate::project) fn load_path_dependency_manifest(
    alias: &str,
    dependency_manifest_path: &Path,
    parent_manifest_path: &Path,
) -> Result<ProjectManifest, String> {
    let manifest_text = fs::read_to_string(dependency_manifest_path).map_err(|error| {
        project_package_error(
            "PX0003",
            format!(
                "failed to read dependency `{alias}` manifest {} declared in {}: {error}",
                dependency_manifest_path.display(),
                parent_manifest_path.display()
            ),
        )
    })?;
    let manifest: ProjectManifest = toml::from_str(&manifest_text).map_err(|error| {
        project_package_error(
            "PX0003",
            format!(
                "failed to parse dependency `{alias}` manifest {} declared in {}: {error}",
                dependency_manifest_path.display(),
                parent_manifest_path.display()
            ),
        )
    })?;

    if manifest.manifest_version != SUPPORTED_MANIFEST_VERSION {
        return Err(project_package_error(
            "PX0003",
            format!(
                "unsupported AX dependency `{alias}` manifest version `{}` in {}; expected `{}`",
                manifest.manifest_version,
                dependency_manifest_path.display(),
                SUPPORTED_MANIFEST_VERSION
            ),
        ));
    }
    if manifest.package.name.trim().is_empty() {
        return Err(project_package_error(
            "PX0003",
            format!(
                "dependency `{alias}` manifest {} must declare a non-empty `[package].name`",
                dependency_manifest_path.display()
            ),
        ));
    }
    if !is_valid_package_name(&manifest.package.name) {
        return Err(project_package_error(
            "PX0003",
            format!(
                "dependency `{alias}` package name `{}` in {} may only contain ASCII letters, digits, `-`, and `_`",
                manifest.package.name,
                dependency_manifest_path.display()
            ),
        ));
    }
    if !manifest.dependencies.is_empty() {
        return Err(project_package_error(
            "PX0006",
            format!(
                "dependency `{alias}` in {} declares nested `[dependencies]`; transitive path packages are not supported in v0",
                dependency_manifest_path.display()
            ),
        ));
    }

    Ok(manifest)
}

pub(in crate::project) fn project_package_error(code: &str, message: String) -> String {
    format!("{code}: {message}")
}
