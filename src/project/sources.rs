use std::fs;
use std::path::Component;

use super::*;

pub(in crate::project) fn load_project_source(project: &Project) -> Result<SourceFile, String> {
    let mut segments = Vec::new();
    for path in project.source_paths() {
        let text = fs::read_to_string(path).map_err(|error| {
            format!("failed to read project source {}: {error}", path.display())
        })?;
        segments.push((path.clone(), text));
    }

    let entry_text = fs::read_to_string(project.entry_path()).map_err(|error| {
        format!(
            "failed to read project entry {}: {error}",
            project.entry_path().display()
        )
    })?;
    segments.push((project.entry_path().to_path_buf(), entry_text));

    Ok(SourceFile::from_segments(
        project.entry_path().to_path_buf(),
        segments,
    ))
}

pub(in crate::project) fn resolve_project_source_file_path(
    root_dir: &Path,
    manifest_path: &Path,
    raw_path: &str,
    label: &str,
) -> Result<PathBuf, String> {
    let path = resolve_project_relative_path(root_dir, manifest_path, raw_path, label)?;

    if path.extension().and_then(|ext| ext.to_str()) != Some("ax") {
        return Err(format!(
            "project {label} `{raw_path}` in {} must point to an `.ax` source file",
            manifest_path.display()
        ));
    }

    let metadata = fs::metadata(&path).map_err(|error| {
        format!(
            "failed to access project {label} {} declared in {}: {error}",
            path.display(),
            manifest_path.display()
        )
    })?;
    if !metadata.is_file() {
        return Err(format!(
            "project {label} {} declared in {} must be a file",
            path.display(),
            manifest_path.display()
        ));
    }

    Ok(path)
}

pub(in crate::project) struct SupportSourceSpec {
    pub(in crate::project) root_path: PathBuf,
    pub(in crate::project) root_alias: String,
    pub(in crate::project) expanded_paths: Vec<PathBuf>,
}

pub(in crate::project) fn resolve_project_support_source_spec(
    root_dir: &Path,
    manifest_path: &Path,
    raw_path: &str,
) -> Result<SupportSourceSpec, String> {
    let path = resolve_project_relative_path(root_dir, manifest_path, raw_path, "support source")?;
    let metadata = fs::metadata(&path).map_err(|error| {
        format!(
            "failed to access project support source {} declared in {}: {error}",
            path.display(),
            manifest_path.display()
        )
    })?;

    if metadata.is_file() {
        if path.extension().and_then(|ext| ext.to_str()) != Some("ax") {
            return Err(format!(
                "project support source `{raw_path}` in {} must point to an `.ax` source file or directory",
                manifest_path.display()
            ));
        }
        return Ok(SupportSourceSpec {
            root_alias: module_root_alias_for_source_root(&path).ok_or_else(|| {
                format!(
                    "support source {} must have a valid module root alias",
                    path.display()
                )
            })?,
            root_path: path.clone(),
            expanded_paths: vec![path],
        });
    }

    if metadata.is_dir() {
        let mut ax_files = collect_ax_files_recursively(&path).map_err(|error| {
            format!(
                "failed to read project support source directory {} declared in {}: {error}",
                path.display(),
                manifest_path.display()
            )
        })?;
        ax_files.sort();

        if ax_files.is_empty() {
            return Err(format!(
                "project support source directory {} declared in {} must contain at least one `.ax` source file",
                path.display(),
                manifest_path.display()
            ));
        }

        return Ok(SupportSourceSpec {
            root_alias: module_root_alias_for_source_root(&path).ok_or_else(|| {
                format!(
                    "support source {} must have a valid module root alias",
                    path.display()
                )
            })?,
            root_path: path,
            expanded_paths: ax_files,
        });
    }

    Err(format!(
        "project support source {} declared in {} must be a file or directory",
        path.display(),
        manifest_path.display()
    ))
}

pub(in crate::project) fn resolve_project_relative_path(
    root_dir: &Path,
    manifest_path: &Path,
    raw_path: &str,
    label: &str,
) -> Result<PathBuf, String> {
    let relative_path = Path::new(raw_path);
    if relative_path.is_absolute() {
        return Err(format!(
            "project {label} `{raw_path}` in {} must be relative to the project root",
            manifest_path.display()
        ));
    }

    normalize_project_path(root_dir, relative_path).ok_or_else(|| {
        format!(
            "project {label} `{raw_path}` in {} cannot escape the filesystem root",
            manifest_path.display()
        )
    })
}

pub(in crate::project) fn normalize_project_path(
    root_dir: &Path,
    relative_path: &Path,
) -> Option<PathBuf> {
    let mut resolved = root_dir.to_path_buf();

    for component in relative_path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => resolved.push(part),
            Component::ParentDir => {
                if !resolved.pop() {
                    return None;
                }
            }
            Component::Prefix(_) | Component::RootDir => return None,
        }
    }

    Some(resolved)
}

pub(in crate::project) fn collect_ax_files_recursively(
    dir: &Path,
) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut entries = fs::read_dir(dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();

    let mut files = Vec::new();
    for entry in entries {
        let metadata = fs::metadata(&entry)?;
        if metadata.is_dir() {
            files.extend(collect_ax_files_recursively(&entry)?);
        } else if metadata.is_file() && entry.extension().and_then(|ext| ext.to_str()) == Some("ax")
        {
            files.push(entry);
        }
    }

    Ok(files)
}

pub(in crate::project) fn module_root_alias_for_source_root(path: &Path) -> Option<String> {
    if path.is_dir() {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
    } else {
        path.file_stem()
            .and_then(|name| name.to_str())
            .map(str::to_string)
    }
}

pub(in crate::project) fn expected_module_path_for_support_source(
    root_path: &Path,
    root_alias: &str,
    source_path: &Path,
) -> Result<String, String> {
    let mut segments = vec![root_alias.to_string()];
    if root_path.is_dir() {
        let relative_path = source_path.strip_prefix(root_path).map_err(|_| {
            format!(
                "{} is not under {}",
                source_path.display(),
                root_path.display()
            )
        })?;
        let Some(file_stem) = relative_path.file_stem().and_then(|stem| stem.to_str()) else {
            return Err(format!(
                "support source {} does not have a valid file stem",
                source_path.display()
            ));
        };
        for component in relative_path
            .parent()
            .into_iter()
            .flat_map(Path::components)
        {
            if let Component::Normal(segment) = component {
                segments.push(segment.to_string_lossy().to_string());
            }
        }
        segments.push(file_stem.to_string());
    }
    Ok(segments.join("."))
}
