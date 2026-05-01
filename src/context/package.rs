use crate::lockfile::check_lockfile;
use crate::package_diagnostics::package_repair_hint;
use crate::project::Project;

use super::shared::normalize_path;
use super::stats::ResolvedUnit;
use super::types::{ContextPackageLock, ContextPackageLockIssue, ContextPathPackage};

pub(super) fn build_local_path_package_facts(
    project: Option<&Project>,
    units: &[ResolvedUnit],
) -> Vec<ContextPathPackage> {
    let Some(project) = project else {
        return Vec::new();
    };

    project
        .local_path_dependencies()
        .iter()
        .map(|dependency| {
            let prefix = format!("{}.", dependency.alias());
            let modules = units
                .iter()
                .filter_map(|unit| unit.module_path.as_ref())
                .filter(|module_path| {
                    module_path.as_str() == dependency.alias()
                        || module_path.starts_with(prefix.as_str())
                })
                .cloned()
                .collect::<Vec<_>>();
            ContextPathPackage {
                alias: dependency.alias().to_string(),
                root: normalize_path(dependency.root_dir()),
                manifest: normalize_path(dependency.manifest_path()),
                source_count: dependency.source_paths().len(),
                modules,
            }
        })
        .collect()
}

pub(super) fn build_local_package_lock_fact(
    project: Option<&Project>,
) -> Option<ContextPackageLock> {
    let project = project?;
    let dependency_count = project.local_path_dependencies().len();
    if dependency_count == 0 {
        return None;
    }

    let report = check_lockfile(project);
    Some(ContextPackageLock {
        path: normalize_path(&report.path),
        schema_version: 1,
        status: report.status.as_str(),
        dependency_count,
        note: report.note,
        issues: report
            .issues
            .into_iter()
            .map(|issue| ContextPackageLockIssue {
                repair_rule: package_repair_hint(issue.code).map(|hint| hint.rule_id),
                repair_goal: package_repair_hint(issue.code).map(|hint| hint.repair_goal),
                code: issue.code,
                kind: issue.kind,
                message: issue.message,
                fixit: issue.fixit,
            })
            .collect(),
    })
}
