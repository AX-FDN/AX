use std::path::PathBuf;

use serde::Serialize;

use crate::project::{PROJECT_MANIFEST_FILE, Project};

#[derive(Debug, Clone, Serialize)]
pub struct Lockfile {
    pub schema_version: u32,
    pub package: LockfilePackage,
    pub dependencies: Vec<LockfileDependency>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LockfilePackage {
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LockfileDependency {
    pub alias: String,
    pub kind: String,
    pub package: String,
    pub path: String,
    pub manifest: String,
    pub source_count: usize,
    pub modules: Vec<String>,
}

pub fn render_lockfile(project: &Project) -> Result<String, String> {
    let lockfile = lockfile_from_project(project);
    let text = serde_json::to_string_pretty(&lockfile)
        .map_err(|error| format!("failed to serialize AX.lock: {error}"))?;
    Ok(format!("{text}\n"))
}

fn lockfile_from_project(project: &Project) -> Lockfile {
    let mut dependencies = project
        .local_path_dependencies()
        .iter()
        .map(|dependency| {
            let mut modules = dependency
                .source_paths()
                .iter()
                .filter_map(|path| {
                    project
                        .expected_module_path(path)
                        .map(std::string::ToString::to_string)
                })
                .collect::<Vec<_>>();
            modules.sort();

            LockfileDependency {
                alias: dependency.alias().to_string(),
                kind: "path".to_string(),
                package: dependency.package_name().to_string(),
                path: dependency.declared_path().to_string(),
                manifest: declared_manifest_path(dependency.declared_path()),
                source_count: dependency.source_paths().len(),
                modules,
            }
        })
        .collect::<Vec<_>>();
    dependencies.sort_by(|left, right| left.alias.cmp(&right.alias));

    Lockfile {
        schema_version: 1,
        package: LockfilePackage {
            name: project.target_name().to_string(),
        },
        dependencies,
    }
}

fn declared_manifest_path(declared_path: &str) -> String {
    PathBuf::from(declared_path)
        .join(PROJECT_MANIFEST_FILE)
        .to_string_lossy()
        .replace('\\', "/")
}
