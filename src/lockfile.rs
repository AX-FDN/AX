use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::project::{PROJECT_MANIFEST_FILE, Project};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct Lockfile {
    pub schema_version: u32,
    pub package: LockfilePackage,
    pub dependencies: Vec<LockfileDependency>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct LockfilePackage {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct LockfileDependency {
    pub alias: String,
    pub kind: String,
    pub package: String,
    pub path: String,
    pub manifest: String,
    pub source_count: usize,
    pub modules: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct LockfileCheckReport {
    pub path: PathBuf,
    pub status: LockfileStatus,
    pub dependency_count: usize,
    pub note: String,
    pub issues: Vec<LockfileIssue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockfileStatus {
    Current,
    Missing,
    Stale,
    Unreadable,
    Unavailable,
}

impl LockfileStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Missing => "missing",
            Self::Stale => "stale",
            Self::Unreadable => "unreadable",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LockfileIssue {
    pub code: &'static str,
    pub kind: &'static str,
    pub message: String,
    pub fixit: String,
}

pub fn render_lockfile(project: &Project) -> Result<String, String> {
    let lockfile = lockfile_from_project(project);
    let text = serde_json::to_string_pretty(&lockfile)
        .map_err(|error| format!("failed to serialize AX.lock: {error}"))?;
    Ok(format!("{text}\n"))
}

pub fn check_lockfile(project: &Project) -> LockfileCheckReport {
    let lockfile_path = project.root_dir().join("AX.lock");
    let dependency_count = project.local_path_dependencies().len();
    let expected_lockfile = lockfile_from_project(project);
    let expected_text = match serde_json::to_string_pretty(&expected_lockfile) {
        Ok(text) => format!("{text}\n"),
        Err(error) => {
            return LockfileCheckReport {
                path: lockfile_path,
                status: LockfileStatus::Unavailable,
                dependency_count,
                note: format!("failed to render expected AX.lock: {error}"),
                issues: vec![LockfileIssue {
                    code: "LX0004",
                    kind: "expected_lockfile_unavailable",
                    message: format!("failed to render expected AX.lock: {error}"),
                    fixit: "fix the project package graph before checking AX.lock".to_string(),
                }],
            };
        }
    };

    let current_text = match std::fs::read_to_string(&lockfile_path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return LockfileCheckReport {
                path: lockfile_path,
                status: LockfileStatus::Missing,
                dependency_count,
                note: "AX.lock is missing; run `axc lock <project>` to freeze local path packages"
                    .to_string(),
                issues: vec![LockfileIssue {
                    code: "LX0001",
                    kind: "lockfile_missing",
                    message: "AX.lock is missing for a project with local path packages"
                        .to_string(),
                    fixit: "run `axc lock <project>`".to_string(),
                }],
            };
        }
        Err(error) => {
            return LockfileCheckReport {
                path: lockfile_path,
                status: LockfileStatus::Unreadable,
                dependency_count,
                note: format!("failed to read AX.lock: {error}"),
                issues: vec![LockfileIssue {
                    code: "LX0003",
                    kind: "lockfile_unreadable",
                    message: format!("failed to read AX.lock: {error}"),
                    fixit: "fix file permissions or regenerate AX.lock with `axc lock <project>`"
                        .to_string(),
                }],
            };
        }
    };

    if current_text == expected_text {
        return LockfileCheckReport {
            path: lockfile_path,
            status: LockfileStatus::Current,
            dependency_count,
            note: "AX.lock matches the current local path package graph".to_string(),
            issues: Vec::new(),
        };
    }

    let mut issues = Vec::new();
    match serde_json::from_str::<Lockfile>(&current_text) {
        Ok(current_lockfile) => {
            issues.extend(compare_lockfiles(&expected_lockfile, &current_lockfile));
        }
        Err(error) => issues.push(LockfileIssue {
            code: "LX0002",
            kind: "lockfile_invalid_json",
            message: format!("AX.lock is not valid lockfile JSON: {error}"),
            fixit: "regenerate AX.lock with `axc lock <project>`".to_string(),
        }),
    }

    if issues.is_empty() {
        issues.push(LockfileIssue {
            code: "LX0002",
            kind: "lockfile_text_drift",
            message: "AX.lock JSON differs from the canonical rendering".to_string(),
            fixit: "regenerate AX.lock with `axc lock <project>`".to_string(),
        });
    }

    LockfileCheckReport {
        path: lockfile_path,
        status: LockfileStatus::Stale,
        dependency_count,
        note: "AX.lock differs from the current local path package graph; run `axc lock <project>`"
            .to_string(),
        issues,
    }
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

fn compare_lockfiles(expected: &Lockfile, current: &Lockfile) -> Vec<LockfileIssue> {
    let mut issues = Vec::new();

    if current.schema_version != expected.schema_version {
        issues.push(LockfileIssue {
            code: "LX0002",
            kind: "schema_version_changed",
            message: format!(
                "AX.lock schema_version is `{}`, expected `{}`",
                current.schema_version, expected.schema_version
            ),
            fixit: "regenerate AX.lock with `axc lock <project>`".to_string(),
        });
    }

    if current.package.name != expected.package.name {
        issues.push(LockfileIssue {
            code: "LX0002",
            kind: "root_package_changed",
            message: format!(
                "AX.lock root package is `{}`, expected `{}`",
                current.package.name, expected.package.name
            ),
            fixit: "regenerate AX.lock with `axc lock <project>`".to_string(),
        });
    }

    if current.dependencies.len() != expected.dependencies.len() {
        issues.push(LockfileIssue {
            code: "LX0002",
            kind: "dependency_count_changed",
            message: format!(
                "AX.lock records {} local path dependencies, expected {}",
                current.dependencies.len(),
                expected.dependencies.len()
            ),
            fixit: "regenerate AX.lock with `axc lock <project>`".to_string(),
        });
    }

    for expected_dependency in &expected.dependencies {
        let Some(current_dependency) = current
            .dependencies
            .iter()
            .find(|dependency| dependency.alias == expected_dependency.alias)
        else {
            issues.push(LockfileIssue {
                code: "LX0002",
                kind: "dependency_missing",
                message: format!(
                    "AX.lock is missing dependency `{}`",
                    expected_dependency.alias
                ),
                fixit: "regenerate AX.lock with `axc lock <project>`".to_string(),
            });
            continue;
        };

        compare_dependency_field(
            &mut issues,
            &expected_dependency.alias,
            "kind",
            &expected_dependency.kind,
            &current_dependency.kind,
        );
        compare_dependency_field(
            &mut issues,
            &expected_dependency.alias,
            "package",
            &expected_dependency.package,
            &current_dependency.package,
        );
        compare_dependency_field(
            &mut issues,
            &expected_dependency.alias,
            "path",
            &expected_dependency.path,
            &current_dependency.path,
        );
        compare_dependency_field(
            &mut issues,
            &expected_dependency.alias,
            "manifest",
            &expected_dependency.manifest,
            &current_dependency.manifest,
        );

        if current_dependency.source_count != expected_dependency.source_count {
            issues.push(LockfileIssue {
                code: "LX0002",
                kind: "dependency_source_count_changed",
                message: format!(
                    "AX.lock dependency `{}` records source_count `{}`, expected `{}`",
                    expected_dependency.alias,
                    current_dependency.source_count,
                    expected_dependency.source_count
                ),
                fixit: "regenerate AX.lock with `axc lock <project>`".to_string(),
            });
        }

        if current_dependency.modules != expected_dependency.modules {
            issues.push(LockfileIssue {
                code: "LX0002",
                kind: "dependency_modules_changed",
                message: format!(
                    "AX.lock dependency `{}` modules changed; expected [{}], found [{}]",
                    expected_dependency.alias,
                    expected_dependency.modules.join(", "),
                    current_dependency.modules.join(", ")
                ),
                fixit: "regenerate AX.lock with `axc lock <project>`".to_string(),
            });
        }
    }

    for current_dependency in &current.dependencies {
        if expected
            .dependencies
            .iter()
            .all(|dependency| dependency.alias != current_dependency.alias)
        {
            issues.push(LockfileIssue {
                code: "LX0002",
                kind: "dependency_removed",
                message: format!(
                    "AX.lock contains dependency `{}` that is not in the current local path package graph",
                    current_dependency.alias
                ),
                fixit: "regenerate AX.lock with `axc lock <project>`".to_string(),
            });
        }
    }

    issues
}

fn compare_dependency_field(
    issues: &mut Vec<LockfileIssue>,
    alias: &str,
    field: &'static str,
    expected: &str,
    current: &str,
) {
    if current == expected {
        return;
    }

    issues.push(LockfileIssue {
        code: "LX0002",
        kind: "dependency_metadata_changed",
        message: format!(
            "AX.lock dependency `{alias}` field `{field}` is `{current}`, expected `{expected}`"
        ),
        fixit: "regenerate AX.lock with `axc lock <project>`".to_string(),
    });
}

fn declared_manifest_path(declared_path: &str) -> String {
    PathBuf::from(declared_path)
        .join(PROJECT_MANIFEST_FILE)
        .to_string_lossy()
        .replace('\\', "/")
}
