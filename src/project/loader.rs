use std::fs;

use crate::lockfile::{Lockfile, LockfileDependency};
use crate::package_cache::cached_registry_package_dir;
use crate::registry::{default_registry_dir, load_registry};

use super::dependencies::{
    is_valid_package_name, load_path_dependency_manifest, project_package_error,
    validate_dependency_alias,
};
use super::manifest::{DependencyManifest, ProjectManifest};
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
    let mut registry_dependencies = Vec::new();

    for (alias, dependency) in &manifest.dependencies {
        validate_dependency_alias(alias, manifest_path)?;
        let DependencyManifest::Path(dependency) = dependency else {
            let DependencyManifest::Registry(dependency) = dependency else {
                unreachable!("all dependency manifest variants should be covered")
            };
            load_registry_dependency_sources(
                alias,
                dependency.registry.trim(),
                dependency.version.trim(),
                root_dir,
                manifest_path,
                &entry_path,
                &mut source_paths,
                &mut source_module_paths,
                &mut source_root_aliases,
                &mut dependency_module_paths,
                &mut registry_dependencies,
            )?;
            continue;
        };
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
        registry_dependencies,
        entry_path,
    })
}

#[allow(clippy::too_many_arguments)]
fn load_registry_dependency_sources(
    alias: &str,
    registry: &str,
    version: &str,
    root_dir: &Path,
    manifest_path: &Path,
    entry_path: &Path,
    source_paths: &mut Vec<PathBuf>,
    source_module_paths: &mut Vec<(PathBuf, String)>,
    source_root_aliases: &mut Vec<(String, String)>,
    dependency_module_paths: &mut Vec<(String, String)>,
    registry_dependencies: &mut Vec<ResolvedRegistryDependency>,
) -> Result<(), String> {
    if registry.is_empty() {
        return Err(project_package_error(
            "PX0101",
            format!(
                "registry dependency `{alias}` in {} must declare a non-empty `registry` name",
                manifest_path.display()
            ),
        ));
    }
    if version.is_empty() {
        return Err(project_package_error(
            "PX0101",
            format!(
                "registry dependency `{alias}` in {} must declare a non-empty `version`",
                manifest_path.display()
            ),
        ));
    }
    if registry != "ax" {
        return Err(project_package_error(
            "PX0102",
            format!(
                "registry dependency `{alias}` in {} uses registry `{registry}`, but only built-in registry `ax` is supported in this preview",
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
    source_root_aliases.push((alias.to_string(), format!("registry dependency `{alias}`")));

    let lockfile = load_registry_lockfile(alias, root_dir, manifest_path)?;
    let lock_entry = find_registry_lock_entry(alias, registry, version, &lockfile, manifest_path)?;
    let package_version = lock_entry.version.as_deref().ok_or_else(|| {
        project_package_error(
            "PX0115",
            format!(
                "AX.lock dependency `{alias}` in {} is missing registry package version",
                root_dir.join("AX.lock").display()
            ),
        )
    })?;
    let registry_index = load_registry(default_registry_dir()).map_err(|error| {
        project_package_error(
            "PX0117",
            format!("failed to load built-in registry metadata for dependency `{alias}`: {error}"),
        )
    })?;
    let registry_package = registry_index.find_package(&lock_entry.package).ok_or_else(|| {
        project_package_error(
            "PX0117",
            format!(
                "AX.lock dependency `{alias}` package `{}` is not present in built-in registry metadata",
                lock_entry.package
            ),
        )
    })?;
    if registry_package.find_version(package_version).is_none() {
        return Err(project_package_error(
            "PX0117",
            format!(
                "AX.lock dependency `{alias}` package `{}` version `{package_version}` is not present in built-in registry metadata",
                lock_entry.package
            ),
        ));
    }
    let package_dir = cached_registry_package_dir(registry, &lock_entry.package, package_version);
    let dependency_manifest_path = package_dir.join(PROJECT_MANIFEST_FILE);
    let dependency_manifest =
        load_path_dependency_manifest(alias, &dependency_manifest_path, manifest_path)?;
    if dependency_manifest.package.sources.is_empty() {
        return Err(project_package_error(
            "PX0004",
            format!(
                "registry dependency `{alias}` in {} must declare at least one `[package].sources` entry after install",
                dependency_manifest_path.display()
            ),
        ));
    }

    for source in &dependency_manifest.package.sources {
        let source = source.trim();
        if source.is_empty() {
            return Err(project_package_error(
                "PX0004",
                format!(
                    "registry dependency `{alias}` in {} must not include an empty `[package].sources` entry",
                    dependency_manifest_path.display()
                ),
            ));
        }

        let support_spec =
            resolve_project_support_source_spec(&package_dir, &dependency_manifest_path, source)?;
        let expanded_paths = support_spec.expanded_paths;
        for source_path in expanded_paths {
            if source_path == entry_path {
                return Err(project_package_error(
                    "PX0007",
                    format!(
                        "registry dependency `{alias}` source `{source}` in {} duplicates the configured entry file",
                        manifest_path.display()
                    ),
                ));
            }
            if source_paths.iter().any(|existing| existing == &source_path) {
                return Err(project_package_error(
                    "PX0007",
                    format!(
                        "registry dependency `{alias}` source `{source}` in {} expands to duplicate file {}",
                        manifest_path.display(),
                        source_path.display()
                    ),
                ));
            }
            let module_path =
                expected_module_path_for_support_source(&support_spec.root_path, alias, &source_path)
                    .map_err(|error| {
                        format!(
                            "failed to derive module path for registry dependency `{alias}` source {} declared in {}: {error}",
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
                        "registry dependency `{alias}` source `{source}` in {} derives duplicate module path `{previous_module_path}` already claimed by `{previous_source}`",
                        manifest_path.display()
                    ),
                ));
            }
            dependency_module_paths.push((module_path.clone(), source_path.display().to_string()));
            source_paths.push(source_path);
            source_module_paths.push((
                source_paths.last().expect("source path must exist").clone(),
                module_path,
            ));
        }
    }

    let mut modules = lock_entry.modules.clone();
    modules.sort();
    registry_dependencies.push(ResolvedRegistryDependency {
        alias: alias.to_string(),
        registry: registry.to_string(),
        package_name: lock_entry.package.clone(),
        version: package_version.to_string(),
        maturity: registry_package.maturity.clone(),
        modules,
    });

    Ok(())
}

fn load_registry_lockfile(
    alias: &str,
    root_dir: &Path,
    manifest_path: &Path,
) -> Result<Lockfile, String> {
    let lockfile_path = root_dir.join("AX.lock");
    let text = fs::read_to_string(&lockfile_path).map_err(|error| {
        project_package_error(
            "PX0112",
            format!(
                "registry dependency `{alias}` in {} requires AX.lock schema v2, but {} could not be read: {error}; run `axc pkg install {}`",
                manifest_path.display(),
                lockfile_path.display(),
                root_dir.display()
            ),
        )
    })?;
    let lockfile: Lockfile = serde_json::from_str(&text).map_err(|error| {
        project_package_error(
            "PX0113",
            format!(
                "failed to parse registry AX.lock {} for dependency `{alias}`: {error}; rerun `axc pkg install {}`",
                lockfile_path.display(),
                root_dir.display()
            ),
        )
    })?;
    if lockfile.schema_version < 2 {
        return Err(project_package_error(
            "PX0114",
            format!(
                "registry dependency `{alias}` in {} requires AX.lock schema v2, found schema `{}` in {}; rerun `axc pkg install {}`",
                manifest_path.display(),
                lockfile.schema_version,
                lockfile_path.display(),
                root_dir.display()
            ),
        ));
    }
    Ok(lockfile)
}

fn find_registry_lock_entry<'a>(
    alias: &str,
    registry: &str,
    version: &str,
    lockfile: &'a Lockfile,
    manifest_path: &Path,
) -> Result<&'a LockfileDependency, String> {
    let Some(entry) = lockfile
        .dependencies
        .iter()
        .find(|dependency| dependency.alias == alias)
    else {
        return Err(project_package_error(
            "PX0115",
            format!(
                "registry dependency `{alias}` in {} is missing from AX.lock; run `axc pkg install`",
                manifest_path.display()
            ),
        ));
    };
    if entry.kind != "registry" {
        return Err(project_package_error(
            "PX0115",
            format!(
                "AX.lock dependency `{alias}` is kind `{}`, expected `registry`",
                entry.kind
            ),
        ));
    }
    if entry.version.as_deref() != Some(version) {
        return Err(project_package_error(
            "PX0115",
            format!(
                "AX.lock dependency `{alias}` version is `{:?}`, expected `{version}`; rerun `axc pkg install`",
                entry.version
            ),
        ));
    }
    let Some(source) = entry.source.as_ref() else {
        return Err(project_package_error(
            "PX0115",
            format!("AX.lock dependency `{alias}` is missing registry source metadata"),
        ));
    };
    if source.registry != registry {
        return Err(project_package_error(
            "PX0115",
            format!(
                "AX.lock dependency `{alias}` registry is `{}`, expected `{registry}`; rerun `axc pkg install`",
                source.registry
            ),
        ));
    }
    let package_version = entry.version.as_deref().unwrap_or(version);
    let package_dir = cached_registry_package_dir(registry, &entry.package, package_version);
    if !package_dir.join(PROJECT_MANIFEST_FILE).is_file() {
        return Err(project_package_error(
            "PX0116",
            format!(
                "registry dependency `{alias}` is locked but not cached at {}; publish real rev/checksum metadata and rerun `axc pkg install`",
                package_dir.display()
            ),
        ));
    }
    Ok(entry)
}
