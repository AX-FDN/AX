#[derive(Debug, Clone, Copy)]
pub struct PackageRepairHint {
    pub code: &'static str,
    pub rule_id: &'static str,
    pub repair_goal: &'static str,
    pub fixit: &'static str,
}

pub fn append_package_repair_hint(message: &str) -> String {
    let Some(code) = extract_package_code(message) else {
        return message.to_string();
    };
    let Some(hint) = package_repair_hint(code) else {
        return message.to_string();
    };

    format!("{message}\n{}", render_package_repair_hint(hint))
}

pub fn package_repair_hint(code: &str) -> Option<PackageRepairHint> {
    match code {
        "PX0001" => Some(PackageRepairHint {
            code: "PX0001",
            rule_id: "package_dependency_alias_must_be_module_root",
            repair_goal: "Rename the dependency alias so it is a valid AX module root.",
            fixit: "use an alias that starts with an ASCII letter or `_` and contains only ASCII letters, digits, and `_`",
        }),
        "PX0002" => Some(PackageRepairHint {
            code: "PX0002",
            rule_id: "package_dependency_path_must_exist",
            repair_goal: "Point the dependency to an existing local AX package directory.",
            fixit: "create the dependency directory or change `[dependencies].<alias>.path` to the correct relative directory",
        }),
        "PX0003" => Some(PackageRepairHint {
            code: "PX0003",
            rule_id: "package_dependency_manifest_must_be_valid",
            repair_goal: "Make the dependency directory contain a valid AX.toml package manifest.",
            fixit: "add or fix the dependency `AX.toml` with `manifest_version = 1` and a non-empty `[package].name`",
        }),
        "PX0004" => Some(PackageRepairHint {
            code: "PX0004",
            rule_id: "package_dependency_sources_must_be_declared",
            repair_goal: "Declare at least one valid AX source file or source directory in the dependency package.",
            fixit: "add non-empty `[package].sources` entries that point to `.ax` files or directories containing `.ax` files",
        }),
        "PX0005" => Some(PackageRepairHint {
            code: "PX0005",
            rule_id: "package_module_roots_must_be_unique",
            repair_goal: "Keep every package module root and derived module path owned by exactly one loaded source.",
            fixit: "rename the dependency alias, move one source root, or change duplicate module paths so they no longer collide",
        }),
        "PX0006" => Some(PackageRepairHint {
            code: "PX0006",
            rule_id: "package_dependency_graph_must_stay_one_level",
            repair_goal: "Keep local path package v0 dependency graphs one level deep.",
            fixit: "move nested dependency declarations to the root project or inline that package until transitive dependencies are supported",
        }),
        "PX0007" => Some(PackageRepairHint {
            code: "PX0007",
            rule_id: "package_sources_must_not_duplicate_loaded_inputs",
            repair_goal: "Ensure dependency sources do not duplicate the project entry or another loaded source.",
            fixit: "remove the duplicate source entry or move the dependency source so each file is loaded once",
        }),
        "PX0101" => Some(PackageRepairHint {
            code: "PX0101",
            rule_id: "registry_dependency_requires_install_preview",
            repair_goal: "Treat the AX.toml registry dependency as valid package intent, but do not load it as source until registry install and lockfile support are available.",
            fixit: "inspect the package with `axc pkg info <package>` or use `{ path = \"...\" }` until `axc pkg install` lands",
        }),
        "PX0102" => Some(PackageRepairHint {
            code: "PX0102",
            rule_id: "registry_dependency_must_use_supported_registry",
            repair_goal: "Use the built-in `ax` registry until custom registries are supported.",
            fixit: "change the dependency to `{ registry = \"ax\", version = \"...\" }`",
        }),
        "PX0106" => Some(PackageRepairHint {
            code: "PX0106",
            rule_id: "registry_lockfile_mixed_graph_not_enabled",
            repair_goal: "Avoid writing mixed local path and registry lockfiles until schema v2 handles both dependency kinds together.",
            fixit: "run `axc pkg install <project> --dry-run` to inspect registry entries, or keep registry and path dependencies in separate preview projects",
        }),
        "PX0107" => Some(PackageRepairHint {
            code: "PX0107",
            rule_id: "registry_package_rev_must_be_pinned",
            repair_goal: "Publish the package source and pin a real git commit before cache installation.",
            fixit: "replace the all-zero registry `rev` with the AX-PKG commit hash for this package version",
        }),
        "PX0108" => Some(PackageRepairHint {
            code: "PX0108",
            rule_id: "registry_package_checksum_must_be_real",
            repair_goal: "Replace preview checksums with stable package directory hashes.",
            fixit: "run `axc pkg hash <package-dir>` and update registry metadata with the resulting `sha256:<hex>`",
        }),
        "PX0109" => Some(PackageRepairHint {
            code: "PX0109",
            rule_id: "registry_package_source_path_must_exist",
            repair_goal: "Make the locked package source path exist inside the checked-out package repository.",
            fixit: "fix `source.path` in registry metadata or add the package directory to AX-PKG",
        }),
        "PX0110" => Some(PackageRepairHint {
            code: "PX0110",
            rule_id: "registry_package_checksum_must_match",
            repair_goal: "Ensure the cached package source exactly matches the checksum pinned in registry metadata.",
            fixit: "review package source changes, then update registry metadata checksum only if the change is intended",
        }),
        "PX0111" => Some(PackageRepairHint {
            code: "PX0111",
            rule_id: "registry_package_git_command_must_succeed",
            repair_goal: "Make git available and ensure the package source repository/rev can be fetched.",
            fixit: "install git, check network access, verify the package URL, and pin an existing commit rev",
        }),
        "PX0112" => Some(PackageRepairHint {
            code: "PX0112",
            rule_id: "registry_dependency_requires_lockfile",
            repair_goal: "Create AX.lock schema v2 before loading registry packages into the project source graph.",
            fixit: "run `axc pkg install <project>`",
        }),
        "PX0113" => Some(PackageRepairHint {
            code: "PX0113",
            rule_id: "registry_lockfile_must_parse",
            repair_goal: "Keep AX.lock valid JSON so registry package resolution can read it.",
            fixit: "rerun `axc pkg install <project>` to regenerate AX.lock",
        }),
        "PX0114" => Some(PackageRepairHint {
            code: "PX0114",
            rule_id: "registry_lockfile_requires_schema_v2",
            repair_goal: "Upgrade AX.lock to schema v2 before loading registry dependencies.",
            fixit: "rerun `axc pkg install <project>`",
        }),
        "PX0115" => Some(PackageRepairHint {
            code: "PX0115",
            rule_id: "registry_lockfile_must_match_manifest",
            repair_goal: "Keep AX.lock registry entries aligned with AX.toml dependency aliases, registries, and versions.",
            fixit: "rerun `axc pkg install <project>` after editing AX.toml dependencies",
        }),
        "PX0116" => Some(PackageRepairHint {
            code: "PX0116",
            rule_id: "registry_package_must_be_cached",
            repair_goal: "Install the locked registry package into the local AX cache before check/run/build can load it.",
            fixit: "publish real rev/checksum metadata, then rerun `axc pkg install <project>`",
        }),
        "LX0001" => Some(PackageRepairHint {
            code: "LX0001",
            rule_id: "package_lockfile_must_exist",
            repair_goal: "Create AX.lock before treating the local package graph as reproducible.",
            fixit: "run `axc lock <project>`",
        }),
        "LX0002" => Some(PackageRepairHint {
            code: "LX0002",
            rule_id: "package_lockfile_must_match_graph",
            repair_goal: "Regenerate AX.lock so it matches the current local path package graph.",
            fixit: "run `axc lock <project>` after reviewing the package graph changes",
        }),
        "LX0003" => Some(PackageRepairHint {
            code: "LX0003",
            rule_id: "package_lockfile_must_be_readable",
            repair_goal: "Make AX.lock readable before lockfile verification can run.",
            fixit: "fix file permissions, remove the unreadable file, or regenerate it with `axc lock <project>`",
        }),
        "LX0004" => Some(PackageRepairHint {
            code: "LX0004",
            rule_id: "package_lockfile_expected_graph_must_render",
            repair_goal: "Fix the package graph before AX can compute the expected lockfile.",
            fixit: "resolve package manifest and source graph errors, then rerun `axc lock <project> --check`",
        }),
        _ => None,
    }
}

pub fn render_package_repair_hint(hint: PackageRepairHint) -> String {
    format!(
        "repair_rule: {}\nrepair_goal: {}\nfixit: {}",
        hint.rule_id, hint.repair_goal, hint.fixit
    )
}

fn extract_package_code(message: &str) -> Option<&str> {
    let prefix = message.get(0..6)?;
    if prefix.len() == 6
        && (prefix.starts_with("PX") || prefix.starts_with("LX"))
        && prefix[2..].chars().all(|ch| ch.is_ascii_digit())
    {
        return Some(prefix);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{append_package_repair_hint, package_repair_hint};

    #[test]
    fn maps_package_resolver_codes_to_repair_hints() {
        let hint = package_repair_hint("PX0002").expect("PX0002 should have a repair hint");
        assert_eq!(hint.rule_id, "package_dependency_path_must_exist");
        assert!(
            append_package_repair_hint("PX0002: missing dependency")
                .contains("repair_rule: package_dependency_path_must_exist")
        );
    }

    #[test]
    fn maps_lockfile_codes_to_repair_hints() {
        let hint = package_repair_hint("LX0002").expect("LX0002 should have a repair hint");
        assert_eq!(hint.rule_id, "package_lockfile_must_match_graph");
    }

    #[test]
    fn maps_registry_preview_code_to_repair_hint() {
        let hint = package_repair_hint("PX0101").expect("PX0101 should have a repair hint");
        assert_eq!(hint.rule_id, "registry_dependency_requires_install_preview");
    }

    #[test]
    fn maps_registry_cache_codes_to_repair_hints() {
        let hint = package_repair_hint("PX0116").expect("PX0116 should have a repair hint");
        assert_eq!(hint.rule_id, "registry_package_must_be_cached");
    }

    #[test]
    fn leaves_non_package_errors_unchanged() {
        assert_eq!(
            append_package_repair_hint("failed to read input"),
            "failed to read input"
        );
    }
}
