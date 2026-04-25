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
    source_paths: Vec<PathBuf>,
    source_module_paths: Vec<(PathBuf, String)>,
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

    let entry_path = resolve_project_source_file_path(root_dir, manifest_path, entry, "entry")?;
    let mut source_paths = Vec::new();
    let mut source_module_paths = Vec::new();
    let mut source_root_aliases = Vec::<(String, String)>::new();
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
            return Err(format!(
                "project support source `{source}` in {} reuses module root alias `{}` already claimed by `{previous_source}`",
                manifest_path.display(),
                support_spec.root_alias,
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
        entry_path,
    })
}

fn load_project_source(project: &Project) -> Result<SourceFile, String> {
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

fn resolve_project_source_file_path(
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

struct SupportSourceSpec {
    root_path: PathBuf,
    root_alias: String,
    expanded_paths: Vec<PathBuf>,
}

fn resolve_project_support_source_spec(
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

fn resolve_project_relative_path(
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

fn normalize_project_path(root_dir: &Path, relative_path: &Path) -> Option<PathBuf> {
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

fn collect_ax_files_recursively(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
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

fn module_root_alias_for_source_root(path: &Path) -> Option<String> {
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

fn expected_module_path_for_support_source(
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
    #[serde(default)]
    sources: Vec<String>,
}

fn default_entry() -> String {
    DEFAULT_ENTRY_FILE.to_string()
}

#[cfg(test)]
mod tests {
    use super::{PROJECT_MANIFEST_FILE, resolve_input};
    use std::fs;
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

    #[test]
    fn resolves_support_sources_before_entry_file() {
        let project_root = repo_root().join("target").join("project-resolve-test");
        let _ = fs::remove_dir_all(&project_root);
        fs::create_dir_all(project_root.join("src")).expect("project src directory should exist");
        fs::write(
            project_root.join(PROJECT_MANIFEST_FILE),
            "\
manifest_version = 1

[package]
name = \"project_supports\"
entry = \"src/main.ax\"
sources = [\"src/lib.ax\"]
",
        )
        .expect("manifest should exist");
        fs::write(
            project_root.join("src").join("lib.ax"),
            "fn helper() -> i32 { return 1; }\n",
        )
        .expect("lib.ax should exist");
        fs::write(
            project_root.join("src").join("main.ax"),
            "fn main() -> i32 { return helper(); }\n",
        )
        .expect("main.ax should exist");

        let resolved =
            resolve_input(&project_root).expect("project with support sources should resolve");
        let project = resolved.project.expect("project metadata should exist");
        assert_eq!(project.source_paths().len(), 1);
        assert!(
            project.source_paths()[0]
                .display()
                .to_string()
                .replace('\\', "/")
                .ends_with("target/project-resolve-test/src/lib.ax")
        );
        assert!(resolved.source.text().contains("fn helper() -> i32"));
        assert!(resolved.source.text().contains("fn main() -> i32"));

        let _ = fs::remove_dir_all(&project_root);
    }

    #[test]
    fn resolves_support_source_directories_in_sorted_recursive_path_order() {
        let project_root = repo_root().join("target").join("project-resolve-dir-test");
        let _ = fs::remove_dir_all(&project_root);
        fs::create_dir_all(project_root.join("lib").join("nested"))
            .expect("project lib directory should exist");
        fs::create_dir_all(project_root.join("src")).expect("project src directory should exist");
        fs::write(
            project_root.join(PROJECT_MANIFEST_FILE),
            "\
manifest_version = 1

[package]
name = \"project_support_dir\"
entry = \"src/main.ax\"
sources = [\"lib\"]
",
        )
        .expect("manifest should exist");
        fs::write(
            project_root.join("lib").join("z.ax"),
            "fn helper_z() -> i32 { return 30; }\n",
        )
        .expect("z.ax should exist");
        fs::write(
            project_root.join("lib").join("a.ax"),
            "fn helper_a() -> i32 { return 10; }\n",
        )
        .expect("a.ax should exist");
        fs::write(
            project_root.join("lib").join("nested").join("c.ax"),
            "fn helper_c() -> i32 { return 20; }\n",
        )
        .expect("c.ax should exist");
        fs::write(project_root.join("lib").join("notes.txt"), "ignore me\n")
            .expect("notes.txt should exist");
        fs::write(
            project_root.join("src").join("main.ax"),
            "fn main() -> i32 { return helper_a() + helper_c() + helper_z(); }\n",
        )
        .expect("main.ax should exist");

        let resolved =
            resolve_input(&project_root).expect("project with support directory should resolve");
        let project = resolved.project.expect("project metadata should exist");
        let source_paths = project
            .source_paths()
            .iter()
            .map(|path| path.display().to_string().replace('\\', "/"))
            .collect::<Vec<_>>();
        assert_eq!(source_paths.len(), 3);
        assert!(source_paths[0].ends_with("target/project-resolve-dir-test/lib/a.ax"));
        assert!(source_paths[1].ends_with("target/project-resolve-dir-test/lib/nested/c.ax"));
        assert!(source_paths[2].ends_with("target/project-resolve-dir-test/lib/z.ax"));
        assert!(resolved.source.text().contains("fn helper_a() -> i32"));
        assert!(resolved.source.text().contains("fn helper_c() -> i32"));
        assert!(resolved.source.text().contains("fn helper_z() -> i32"));
        assert!(resolved.source.text().contains("fn main() -> i32"));

        let _ = fs::remove_dir_all(&project_root);
    }

    #[test]
    fn resolves_support_sources_from_shared_sibling_directory() {
        let test_root = repo_root()
            .join("target")
            .join("project-shared-support-test");
        let project_root = test_root.join("project");
        let shared_root = test_root.join("shared");
        let _ = fs::remove_dir_all(&test_root);
        fs::create_dir_all(project_root.join("src")).expect("project src directory should exist");
        fs::create_dir_all(&shared_root).expect("shared foundation directory should exist");
        fs::write(
            project_root.join(PROJECT_MANIFEST_FILE),
            "\
manifest_version = 1

[package]
name = \"project_shared_supports\"
entry = \"src/main.ax\"
sources = [\"../shared\"]
",
        )
        .expect("manifest should exist");
        fs::write(
            shared_root.join("foundation.ax"),
            "fn shared_helper() -> i32 { return 7; }\n",
        )
        .expect("shared foundation file should exist");
        fs::write(
            project_root.join("src").join("main.ax"),
            "fn main() -> i32 { return shared_helper(); }\n",
        )
        .expect("main.ax should exist");

        let resolved = resolve_input(&project_root)
            .expect("project with shared sibling support sources should resolve");
        let project = resolved.project.expect("project metadata should exist");
        assert_eq!(project.source_paths().len(), 1);
        assert!(
            project.source_paths()[0]
                .display()
                .to_string()
                .replace('\\', "/")
                .ends_with("target/project-shared-support-test/shared/foundation.ax")
        );
        assert!(resolved.source.text().contains("fn shared_helper() -> i32"));
        assert!(resolved.source.text().contains("fn main() -> i32"));

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn derives_expected_module_paths_for_support_sources() {
        let test_root = repo_root().join("target").join("project-module-path-test");
        let project_root = test_root.join("project");
        let shared_root = test_root.join("foundation");
        let _ = fs::remove_dir_all(&test_root);
        fs::create_dir_all(project_root.join("lib").join("audit"))
            .expect("project lib directory should exist");
        fs::create_dir_all(project_root.join("src")).expect("project src directory should exist");
        fs::create_dir_all(&shared_root).expect("shared root should exist");
        fs::write(
            project_root.join(PROJECT_MANIFEST_FILE),
            "\
manifest_version = 1

[package]
name = \"project_module_paths\"
entry = \"src/main.ax\"
sources = [\"../foundation\", \"lib/report.ax\", \"lib/audit\"]
",
        )
        .expect("manifest should exist");
        fs::write(
            shared_root.join("search.ax"),
            "fn helper() -> i32 { return 1; }\n",
        )
        .expect("shared support file should exist");
        fs::write(
            project_root.join("lib").join("report.ax"),
            "fn build_report() -> i32 { return 2; }\n",
        )
        .expect("project report file should exist");
        fs::write(
            project_root.join("lib").join("audit").join("summary.ax"),
            "fn summarize() -> i32 { return 3; }\n",
        )
        .expect("audit summary file should exist");
        fs::write(
            project_root.join("src").join("main.ax"),
            "fn main() -> i32 { return 0; }\n",
        )
        .expect("main.ax should exist");

        let resolved = resolve_input(&project_root).expect("project should resolve");
        let project = resolved.project.expect("project metadata should exist");

        assert_eq!(
            project.expected_module_path(&shared_root.join("search.ax")),
            Some("foundation.search")
        );
        assert_eq!(
            project.expected_module_path(&project_root.join("lib").join("report.ax")),
            Some("report")
        );
        assert_eq!(
            project
                .expected_module_path(&project_root.join("lib").join("audit").join("summary.ax")),
            Some("audit.summary")
        );

        let _ = fs::remove_dir_all(&test_root);
    }

    #[test]
    fn rejects_duplicate_module_root_aliases() {
        let project_root = repo_root()
            .join("target")
            .join("project-duplicate-module-root-test");
        let _ = fs::remove_dir_all(&project_root);
        fs::create_dir_all(project_root.join("src")).expect("project src directory should exist");
        fs::create_dir_all(project_root.join("lib")).expect("project lib directory should exist");
        fs::write(
            project_root.join(PROJECT_MANIFEST_FILE),
            "\
manifest_version = 1

[package]
name = \"project_duplicate_module_root\"
entry = \"src/main.ax\"
sources = [\"lib\", \"lib.ax\"]
",
        )
        .expect("manifest should exist");
        fs::write(
            project_root.join("lib").join("report.ax"),
            "fn report() -> i32 { return 1; }\n",
        )
        .expect("lib/report.ax should exist");
        fs::write(
            project_root.join("lib.ax"),
            "fn helper() -> i32 { return 2; }\n",
        )
        .expect("lib.ax should exist");
        fs::write(
            project_root.join("src").join("main.ax"),
            "fn main() -> i32 { return 0; }\n",
        )
        .expect("main.ax should exist");

        let error = resolve_input(&project_root).expect_err("duplicate module roots should fail");
        assert!(error.contains("reuses module root alias `lib`"));

        let _ = fs::remove_dir_all(&project_root);
    }
}
