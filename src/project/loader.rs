use std::fs;

use super::dependencies::{
    is_valid_package_name, load_path_dependency_manifest, project_package_error,
    validate_dependency_alias,
};
use super::manifest::ProjectManifest;
use super::sources::{
    expected_module_path_for_support_source, resolve_project_relative_path,
    resolve_project_source_file_path, resolve_project_support_source_spec,
};
use super::*;

pub(in crate::project) fn load_project(manifest_path: &Path) -> Result<Project, String> {
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

    let entry_path = resolve_project_source_file_path(root_dir, manifest_path, entry, "entry")?;
    let mut source_paths = Vec::new();
    let mut source_module_paths = Vec::new();
    let mut source_root_aliases = Vec::<(String, String)>::new();
    let mut dependency_module_paths = Vec::<(String, String)>::new();
    let mut local_path_dependencies = Vec::new();

    for (alias, dependency) in &manifest.dependencies {
        validate_dependency_alias(alias, manifest_path)?;
        if dependency.path.trim().is_empty() {
            return Err(project_package_error(
                "PX0002",
                format!(
                    "dependency `{alias}` in {} must declare a non-empty `path`",
                    manifest_path.display()
                ),
            ));
        }
        let dependency_path = resolve_project_relative_path(
            root_dir,
            manifest_path,
            dependency.path.trim(),
            &format!("dependency `{alias}` path"),
        )?;
        let metadata = fs::metadata(&dependency_path).map_err(|error| {
            project_package_error(
                "PX0002",
                format!(
                    "failed to access dependency `{alias}` path {} declared in {}: {error}",
                    dependency_path.display(),
                    manifest_path.display()
                ),
            )
        })?;
        if !metadata.is_dir() {
            return Err(project_package_error(
                "PX0002",
                format!(
                    "dependency `{alias}` path {} declared in {} must be a directory",
                    dependency_path.display(),
                    manifest_path.display()
                ),
            ));
        }

        if let Some((_, previous_source)) = source_root_aliases
            .iter()
            .find(|(claimed_alias, _)| claimed_alias == alias)
        {
            return Err(project_package_error(
                "PX0005",
                format!(
                    "dependency `{alias}` in {} reuses module root alias `{alias}` already claimed by `{previous_source}`",
                    manifest_path.display()
                ),
            ));
        }
        source_root_aliases.push((alias.clone(), format!("dependency `{alias}`")));

        let dependency_manifest_path = dependency_path.join(PROJECT_MANIFEST_FILE);
        let dependency_manifest =
            load_path_dependency_manifest(alias, &dependency_manifest_path, manifest_path)?;
        if dependency_manifest.package.sources.is_empty() {
            return Err(project_package_error(
                "PX0004",
                format!(
                    "dependency `{alias}` in {} must declare at least one `[package].sources` entry",
                    manifest_path.display()
                ),
            ));
        }

        let mut dependency_source_paths = Vec::new();
        for source in &dependency_manifest.package.sources {
            let source = source.trim();
            if source.is_empty() {
                return Err(project_package_error(
                    "PX0004",
                    format!(
                        "dependency `{alias}` in {} must not include an empty `[package].sources` entry",
                        manifest_path.display()
                    ),
                ));
            }

            let support_spec = resolve_project_support_source_spec(
                &dependency_path,
                &dependency_manifest_path,
                source,
            )?;
            let expanded_paths = support_spec.expanded_paths;
            for source_path in expanded_paths {
                if source_path == entry_path {
                    return Err(project_package_error(
                        "PX0007",
                        format!(
                            "dependency `{alias}` source `{source}` in {} duplicates the configured entry file",
                            manifest_path.display()
                        ),
                    ));
                }
                if source_paths.iter().any(|existing| existing == &source_path) {
                    return Err(project_package_error(
                        "PX0007",
                        format!(
                            "dependency `{alias}` source `{source}` in {} expands to duplicate file {}",
                            manifest_path.display(),
                            source_path.display()
                        ),
                    ));
                }
                let module_path = expected_module_path_for_support_source(
                    &support_spec.root_path,
                    alias,
                    &source_path,
                )
                .map_err(|error| {
                    format!(
                        "failed to derive module path for dependency `{alias}` source {} declared in {}: {error}",
                        source_path.display(),
                        manifest_path.display()
                    )
                })?;
                if let Some((previous_module_path, previous_source)) = dependency_module_paths
                    .iter()
                    .find(|(existing_module_path, _)| existing_module_path == &module_path)
                {
                    return Err(project_package_error(
                        "PX0005",
                        format!(
                            "dependency `{alias}` source `{source}` in {} derives duplicate module path `{previous_module_path}` already claimed by `{previous_source}`",
                            manifest_path.display()
                        ),
                    ));
                }
                dependency_module_paths
                    .push((module_path.clone(), source_path.display().to_string()));
                dependency_source_paths.push(source_path.clone());
                source_paths.push(source_path);
                source_module_paths.push((
                    source_paths.last().expect("source path must exist").clone(),
                    module_path,
                ));
            }
        }
        local_path_dependencies.push(ResolvedPathDependency {
            alias: alias.clone(),
            declared_path: dependency.path.trim().replace('\\', "/"),
            package_name: dependency_manifest.package.name.clone(),
            root_dir: dependency_path,
            manifest_path: dependency_manifest_path,
            source_paths: dependency_source_paths,
        });
    }

    for source in &manifest.package.sources {
        let source = source.trim();
        if source.is_empty() {
            return Err(format!(
                "project manifest {} must not include an empty `[package].sources` entry",
                manifest_path.display()
            ));
        }

        let support_spec = resolve_project_support_source_spec(root_dir, manifest_path, source)?;
        if let Some((_, previous_source)) = source_root_aliases
            .iter()
            .find(|(alias, _)| alias == &support_spec.root_alias)
        {
            return Err(project_package_error(
                "PX0005",
                format!(
                    "project support source `{source}` in {} reuses module root alias `{}` already claimed by `{previous_source}`",
                    manifest_path.display(),
                    support_spec.root_alias,
                ),
            ));
        }
        source_root_aliases.push((support_spec.root_alias.clone(), source.to_string()));

        let expanded_paths = support_spec.expanded_paths;
        for source_path in expanded_paths {
            if source_path == entry_path {
                return Err(format!(
                    "project support source `{source}` in {} duplicates the configured entry file",
                    manifest_path.display()
                ));
            }
            if source_paths.iter().any(|existing| existing == &source_path) {
                return Err(format!(
                    "project support source `{source}` in {} expands to duplicate file {}",
                    manifest_path.display(),
                    source_path.display()
                ));
            }
            let module_path = expected_module_path_for_support_source(
                &support_spec.root_path,
                &support_spec.root_alias,
                &source_path,
            )
            .map_err(|error| {
                format!(
                    "failed to derive module path for support source {} declared in {}: {error}",
                    source_path.display(),
                    manifest_path.display()
                )
            })?;
            source_paths.push(source_path);
            source_module_paths.push((
                source_paths.last().expect("source path must exist").clone(),
                module_path,
            ));
        }
    }

    Ok(Project {
        root_dir: root_dir.to_path_buf(),
        manifest_path: manifest_path.to_path_buf(),
        manifest_text,
        manifest,
        source_paths,
        source_module_paths,
        local_path_dependencies,
        entry_path,
    })
}
