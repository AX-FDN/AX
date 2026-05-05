use super::options::*;
use super::render::*;
use super::*;

pub(in crate::cli) fn run_check(args: Vec<String>) -> i32 {
    let options = match parse_check_args(args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}\nusage: axc check <path> [--json] [--ai] [--ai-session <path>]");
            return 2;
        }
    };

    let input = match load_input(&options.file) {
        Ok(input) => input,
        Err(error) => {
            return render_load_input_error(&options.file, &error, options.json, options.ai);
        }
    };
    let source = &input.source;

    let mut output = check_only_with_project(source, input.project.as_ref());
    if output.diagnostics.is_empty() {
        println!(
            "{}",
            render_check_success(options.json, &source.display_path())
        );
        return 0;
    }

    if options.ai {
        if let Err(error) = enhance_diagnostics(
            source,
            &output.program,
            &mut output.diagnostics,
            options.ai_session.as_deref(),
        ) {
            eprintln!("{error}");
            return 1;
        }
    }

    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&output.diagnostics)
                .expect("diagnostics json should serialize")
        );
    } else {
        eprintln!("{}", render_diagnostics(source, &output.diagnostics));
    }

    1
}

pub(in crate::cli) fn run_ast(args: Vec<String>) -> i32 {
    if args.len() != 1 {
        eprintln!("usage: axc ast <path>");
        return 2;
    }

    let path = PathBuf::from(&args[0]);
    let input = match load_input(&path) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    let source = &input.source;

    let output = check_only_with_project(source, input.project.as_ref());
    if !output.diagnostics.is_empty() {
        eprintln!("{}", render_diagnostics(source, &output.diagnostics));
        return 1;
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&output.program).expect("ast json should serialize")
    );
    0
}

pub(in crate::cli) fn run_hir(args: Vec<String>) -> i32 {
    if args.len() != 1 {
        eprintln!("usage: axc hir <path>");
        return 2;
    }

    let path = PathBuf::from(&args[0]);
    let input = match load_input(&path) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    let source = &input.source;

    let output = analyze_with_project(source, input.project.as_ref());
    if !output.diagnostics.is_empty() {
        eprintln!("{}", render_diagnostics(source, &output.diagnostics));
        return 1;
    }

    let Some(hir) = output.hir else {
        eprintln!("internal error: HIR should be available after a successful analysis");
        return 1;
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&hir).expect("hir json should serialize")
    );
    0
}

pub(in crate::cli) fn run_build(args: Vec<String>) -> i32 {
    let options = match parse_build_args(args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!(
                "{error}\nusage: axc build <path> [--emit <ir|exe|all>] [--no-link] [--out-dir <path>] [--json]"
            );
            return 2;
        }
    };

    let input = match load_input(&options.file) {
        Ok(input) => input,
        Err(error) => {
            return render_load_input_error(&options.file, &error, options.json, false);
        }
    };
    let source = &input.source;

    let output = analyze_with_project(source, input.project.as_ref());
    if !output.diagnostics.is_empty() {
        if options.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&output.diagnostics)
                    .expect("build diagnostics json should serialize")
            );
        } else {
            eprintln!("{}", render_diagnostics(source, &output.diagnostics));
        }
        return 1;
    }

    let Some(hir) = output.hir.as_ref() else {
        eprintln!("internal error: HIR should be available after a successful analysis");
        return 1;
    };
    let Some(mir) = output.mir.as_ref() else {
        eprintln!("internal error: MIR should be available after a successful analysis");
        return 1;
    };

    let build_input = match input.project.as_ref() {
        Some(project) => match build_input_from_project(source, project) {
            Ok(input) => input,
            Err(error) => {
                eprintln!("{error}");
                return 1;
            }
        },
        None => match build_input_from_source(source) {
            Ok(input) => input,
            Err(error) => {
                eprintln!("{error}");
                return 1;
            }
        },
    };
    let out_dir = match options.out_dir {
        Some(out_dir) => out_dir,
        None => match default_output_dir(&build_input.target_name) {
            Ok(out_dir) => out_dir,
            Err(error) => {
                eprintln!("{error}");
                return 1;
            }
        },
    };

    let result = match build_program(
        source,
        &output.program,
        hir,
        mir,
        &build_input,
        &BuildOptions {
            out_dir,
            emit: options.emit,
        },
    ) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };

    let missing_requested_executable =
        options.emit.requires_executable() && result.manifest.artifacts.executable.is_none();

    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result.manifest)
                .expect("build manifest json should serialize")
        );
    } else if missing_requested_executable {
        eprintln!(
            "build emitted artifacts but did not produce requested --emit {}; inspect {} for AOT/toolchain blockers",
            options.emit.as_str(),
            result.manifest_path.display()
        );
    } else {
        println!("build succeeded: {}", result.manifest_path.display());
        if let Some(executable) = &result.manifest.artifacts.executable {
            println!("executable: {executable}");
        }
    }
    if missing_requested_executable {
        return 1;
    }
    0
}

pub(in crate::cli) fn run_lock(args: Vec<String>) -> i32 {
    let options = match parse_lock_args(args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}\nusage: axc lock <project> [--check]");
            return 2;
        }
    };

    let input = match load_input(&options.file) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    let Some(project) = input.project.as_ref() else {
        eprintln!("axc lock requires a project directory or AX.toml path");
        return 2;
    };

    let lockfile_text = match render_lockfile(project) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    let lockfile_path = project.root_dir().join("AX.lock");

    if options.check {
        let report = check_lockfile(project);
        if !report.issues.is_empty() {
            eprintln!("{}", render_lock_check_failure(&report));
            return 1;
        }
        println!("AX.lock is up to date: {}", lockfile_path.display());
        return 0;
    }

    if let Err(error) = fs::write(&lockfile_path, lockfile_text) {
        eprintln!(
            "failed to write AX.lock {}: {error}",
            lockfile_path.display()
        );
        return 1;
    }
    println!("wrote AX.lock: {}", lockfile_path.display());
    0
}

pub(in crate::cli) fn run_pkg(args: Vec<String>) -> i32 {
    let options = match parse_pkg_args(args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!(
                "{error}\nusage: axc pkg <search|info|check|tree|add|install|hash> [args] [--registry <path>]"
            );
            return 2;
        }
    };

    match options {
        PkgCliOptions::Search { query, registry } => {
            let registry = match load_registry(&registry) {
                Ok(registry) => registry,
                Err(error) => {
                    eprintln!("{error}");
                    return 1;
                }
            };
            let matches = registry.search(&query);
            println!("{}", render_search_results(&matches));
            0
        }
        PkgCliOptions::Info { package, registry } => {
            let registry = match load_registry(&registry) {
                Ok(registry) => registry,
                Err(error) => {
                    eprintln!("{error}");
                    return 1;
                }
            };
            let Some(package) = registry.find_package(&package) else {
                eprintln!("registry package `{package}` was not found");
                return 1;
            };
            println!("{}", render_package_info(package));
            0
        }
        PkgCliOptions::Check { registry } => {
            let issues = validate_registry(&registry);
            if issues.is_empty() {
                println!("registry check succeeded: {}", registry.display());
                return 0;
            }
            eprintln!("{}", render_registry_check_failure(&issues));
            1
        }
        PkgCliOptions::Tree { project } => run_pkg_tree(&project),
        PkgCliOptions::Add {
            package,
            project,
            registry,
            dry_run,
        } => run_pkg_add(&package, &project, &registry, dry_run),
        PkgCliOptions::Install {
            project,
            registry,
            dry_run,
        } => run_pkg_install(&project, &registry, dry_run),
        PkgCliOptions::Hash { path } => match hash_package_dir(&path) {
            Ok(checksum) => {
                println!("{checksum}");
                0
            }
            Err(error) => {
                eprintln!("{error}");
                1
            }
        },
    }
}

fn run_pkg_add(package: &str, project_path: &Path, registry_path: &Path, dry_run: bool) -> i32 {
    let registry = match load_registry(registry_path) {
        Ok(registry) => registry,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    let Some(metadata) = registry.find_package(package) else {
        eprintln!("registry package `{package}` was not found");
        return 1;
    };
    let Some(version) = metadata.latest_version() else {
        eprintln!("registry package `{package}` has no versions");
        return 1;
    };

    let dependency_entry = format!(
        "{} = {{ registry = \"ax\", version = \"{}\" }}",
        metadata.name, version.version
    );
    if dry_run {
        println!("preview dependency entry:");
        println!("{dependency_entry}");
        println!("note: dry-run only; AX.toml was not changed");
        return 0;
    }

    let (manifest_path, value) = match load_project_manifest_value(project_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    if value
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .is_some_and(|dependencies| dependencies.contains_key(&metadata.name))
    {
        eprintln!(
            "PX0117: dependency `{}` is already declared in {}",
            metadata.name,
            manifest_path.display()
        );
        return 1;
    }

    let manifest_text = match fs::read_to_string(&manifest_path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!(
                "failed to read AX project manifest {}: {error}",
                manifest_path.display()
            );
            return 1;
        }
    };
    let updated =
        add_registry_dependency_to_manifest_text(&manifest_text, &metadata.name, &version.version);
    if let Err(error) = fs::write(&manifest_path, updated) {
        eprintln!(
            "failed to write AX project manifest {}: {error}",
            manifest_path.display()
        );
        return 1;
    }
    println!("added dependency:");
    println!("{dependency_entry}");
    println!("updated manifest: {}", manifest_path.display());
    0
}

fn add_registry_dependency_to_manifest_text(
    manifest_text: &str,
    package: &str,
    version: &str,
) -> String {
    let line_to_add = format!("{package} = {{ registry = \"ax\", version = \"{version}\" }}");
    let normalized = manifest_text.replace("\r\n", "\n");
    let mut lines = normalized.split('\n').collect::<Vec<_>>();
    let had_trailing_newline = normalized.ends_with('\n');
    if had_trailing_newline {
        lines.pop();
    }

    let mut dependency_header = None;
    for (index, line) in lines.iter().enumerate() {
        if line.trim() == "[dependencies]" {
            dependency_header = Some(index);
            break;
        }
    }

    let mut output = Vec::new();
    match dependency_header {
        Some(header_index) => {
            let mut insert_index = lines.len();
            for (index, line) in lines.iter().enumerate().skip(header_index + 1) {
                let trimmed = line.trim();
                if trimmed.starts_with('[') && trimmed.ends_with(']') {
                    insert_index = index;
                    break;
                }
            }
            output.extend(
                lines
                    .iter()
                    .take(insert_index)
                    .map(|line| (*line).to_string()),
            );
            while output.last().is_some_and(|line| line.trim().is_empty()) {
                output.pop();
            }
            output.push(line_to_add);
            if insert_index < lines.len() {
                output.push(String::new());
            }
            output.extend(
                lines
                    .iter()
                    .skip(insert_index)
                    .map(|line| (*line).to_string()),
            );
        }
        None => {
            output.extend(lines.iter().map(|line| (*line).to_string()));
            if output.last().is_some_and(|line| !line.trim().is_empty()) {
                output.push(String::new());
            }
            output.push("[dependencies]".to_string());
            output.push(line_to_add);
        }
    }

    let mut rendered = output.join("\n");
    rendered.push('\n');
    rendered
}

#[cfg(test)]
mod tests {
    use super::add_registry_dependency_to_manifest_text;

    #[test]
    fn adds_registry_dependency_to_existing_dependencies_table() {
        let updated = add_registry_dependency_to_manifest_text(
            "manifest_version = 1\n\n[package]\nname = \"demo\"\nentry = \"src/main.ax\"\n\n[dependencies]\ntext_tools = { registry = \"ax\", version = \"0.1.0\" }\n\n[profile]\nname = \"dev\"\n",
            "math_rules",
            "0.1.0",
        );

        assert!(updated.contains(
            "[dependencies]\ntext_tools = { registry = \"ax\", version = \"0.1.0\" }\nmath_rules = { registry = \"ax\", version = \"0.1.0\" }\n\n[profile]"
        ));
    }

    #[test]
    fn adds_dependencies_table_when_missing() {
        let updated = add_registry_dependency_to_manifest_text(
            "manifest_version = 1\n\n[package]\nname = \"demo\"\nentry = \"src/main.ax\"\n",
            "text_tools",
            "0.1.0",
        );

        assert!(updated.ends_with(
            "\n[dependencies]\ntext_tools = { registry = \"ax\", version = \"0.1.0\" }\n"
        ));
    }
}

fn run_pkg_tree(project_path: &Path) -> i32 {
    let input = match load_input(project_path) {
        Ok(input) => input,
        Err(error) => {
            if error.starts_with("PX0101:") {
                return run_pkg_tree_from_manifest(project_path);
            }
            eprintln!("{error}");
            return 1;
        }
    };
    let Some(project) = input.project.as_ref() else {
        eprintln!("axc pkg tree requires a project directory or AX.toml path");
        return 2;
    };

    println!("{} dependencies", project.target_name());
    if project.local_path_dependencies().is_empty() {
        println!("(no dependencies)");
        return 0;
    }
    for dependency in project.local_path_dependencies() {
        println!(
            "{} path {} package {} sources {}",
            dependency.alias(),
            dependency.declared_path(),
            dependency.package_name(),
            dependency.source_paths().len()
        );
    }
    0
}

fn run_pkg_tree_from_manifest(project_path: &Path) -> i32 {
    let (_manifest_path, value) = match load_project_manifest_value(project_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    let package_name = value
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(toml::Value::as_str)
        .unwrap_or("unknown");
    println!("{package_name} dependencies");

    let Some(dependencies) = value.get("dependencies").and_then(toml::Value::as_table) else {
        println!("(no dependencies)");
        return 0;
    };
    if dependencies.is_empty() {
        println!("(no dependencies)");
        return 0;
    }

    for (alias, dependency) in dependencies {
        let Some(table) = dependency.as_table() else {
            println!("{alias} invalid dependency declaration");
            continue;
        };
        if let Some(path) = table.get("path").and_then(toml::Value::as_str) {
            println!("{alias} path {path}");
        } else if let Some(version) = table.get("version").and_then(toml::Value::as_str) {
            let registry = table
                .get("registry")
                .and_then(toml::Value::as_str)
                .unwrap_or("ax");
            println!("{alias} registry {registry} version {version} status preview-uninstalled");
        } else {
            println!("{alias} invalid dependency declaration");
        }
    }
    0
}

fn run_pkg_install(project_path: &Path, registry_path: &Path, dry_run: bool) -> i32 {
    let (manifest_path, value) = match load_project_manifest_value(project_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    let registry = match load_registry(registry_path) {
        Ok(registry) => registry,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    let package_name = value
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(toml::Value::as_str)
        .unwrap_or("unknown");
    println!("install plan: {package_name}");
    println!("manifest: {}", manifest_path.display());

    let Some(dependencies) = value.get("dependencies").and_then(toml::Value::as_table) else {
        println!("(no registry dependencies)");
        return 0;
    };

    let mut planned = 0;
    let mut path_dependency_count = 0;
    let mut lock_entries = Vec::new();
    let mut install_plans = Vec::new();
    for (alias, dependency) in dependencies {
        let Some(table) = dependency.as_table() else {
            continue;
        };
        if table.get("path").and_then(toml::Value::as_str).is_some() {
            path_dependency_count += 1;
            continue;
        }
        let Some(version) = table.get("version").and_then(toml::Value::as_str) else {
            continue;
        };
        let registry_name = table
            .get("registry")
            .and_then(toml::Value::as_str)
            .unwrap_or("ax");
        if registry_name != "ax" {
            eprintln!(
                "PX0102: registry dependency `{alias}` uses registry `{registry_name}`, but only the built-in `ax` registry is supported in this preview"
            );
            return 1;
        }
        let Some(package) = registry.find_package(alias) else {
            eprintln!("PX0103: registry package `{alias}` was not found");
            return 1;
        };
        let Some(package_version) = package.find_version(version) else {
            eprintln!("PX0104: registry package `{alias}` has no version `{version}`");
            return 1;
        };
        planned += 1;
        lock_entries.push(registry_lock_dependency_preview(
            alias,
            &package.name,
            package_version,
        ));
        install_plans.push((package.name.clone(), package_version.clone()));
        let source_path = package_version.source.path.as_deref().unwrap_or(".");
        println!(
            "{} registry ax version {} source {} {} path {} rev {} checksum {} modules {}",
            alias,
            package_version.version,
            package_version.source.kind,
            package_version.source.url,
            source_path,
            package_version.source.rev,
            package_version.checksum,
            package_version.modules.join(", ")
        );
    }
    if planned == 0 {
        println!("(no registry dependencies)");
    } else {
        if !dry_run && path_dependency_count > 0 {
            eprintln!(
                "PX0106: `axc pkg install` cannot write mixed local path + registry AX.lock yet; run with `--dry-run` to inspect registry entries"
            );
            return 1;
        }
        let lockfile_text = match render_registry_lockfile_preview(package_name, lock_entries) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("{error}");
                return 1;
            }
        };
        if dry_run {
            println!("AX.lock preview:");
            print!("{lockfile_text}");
            println!("note: dry-run only; no cache, AX.toml, or AX.lock files were changed");
            return 0;
        }
        for (package_name, package_version) in &install_plans {
            match install_registry_package_to_cache("ax", package_name, package_version) {
                Ok(PackageCacheInstall::Installed(package)) => {
                    println!(
                        "cached package {package_name}: {}",
                        package.package_dir.display()
                    );
                    println!("verified checksum: {}", package.checksum);
                }
                Ok(PackageCacheInstall::Skipped { reason }) => {
                    println!("cache skipped: {reason}");
                }
                Err(error) => {
                    eprintln!("{error}");
                    return 1;
                }
            }
        }
        let Some(project_root) = manifest_path.parent() else {
            eprintln!(
                "failed to resolve project root for manifest {}",
                manifest_path.display()
            );
            return 1;
        };
        let lockfile_path = project_root.join("AX.lock");
        if let Err(error) = fs::write(&lockfile_path, lockfile_text) {
            eprintln!(
                "failed to write AX.lock {}: {error}",
                lockfile_path.display()
            );
            return 1;
        }
        println!(
            "wrote AX.lock registry preview: {}",
            lockfile_path.display()
        );
        println!("note: packages with placeholder rev/checksum metadata are not cached yet");
    }
    0
}

fn load_project_manifest_value(project_path: &Path) -> Result<(PathBuf, toml::Value), String> {
    let manifest_path = if project_path.is_dir() {
        project_path.join(crate::project::PROJECT_MANIFEST_FILE)
    } else {
        project_path.to_path_buf()
    };
    let text = fs::read_to_string(&manifest_path).map_err(|error| {
        format!(
            "failed to read AX project manifest {}: {error}",
            manifest_path.display()
        )
    })?;
    let value = toml::from_str::<toml::Value>(&text).map_err(|error| {
        format!(
            "failed to parse AX project manifest {}: {error}",
            manifest_path.display()
        )
    })?;
    Ok((manifest_path, value))
}

fn render_registry_check_failure(issues: &[crate::registry::RegistryIssue]) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "{}: registry check failed",
        issues.first().map(|issue| issue.code).unwrap_or("RG0000")
    ));
    for issue in issues {
        lines.push(format!(
            "- {} {}: {}",
            issue.code,
            issue.path.display(),
            issue.message
        ));
    }
    lines.join("\n")
}

pub(in crate::cli) fn render_lock_check_failure(report: &LockfileCheckReport) -> String {
    let mut lines = vec![format!(
        "{}: AX.lock {}: {}",
        report
            .issues
            .first()
            .map(|issue| issue.code)
            .unwrap_or("LX0000"),
        report.status.as_str(),
        report.path.display()
    )];
    lines.push(format!("note: {}", report.note));
    lines.push(format!("dependency_count: {}", report.dependency_count));
    for issue in &report.issues {
        lines.push(format!(
            "- {} [{}]: {}",
            issue.code, issue.kind, issue.message
        ));
        lines.push(format!("  fixit: {}", issue.fixit));
        if let Some(hint) = package_repair_hint(issue.code) {
            for hint_line in render_package_repair_hint(hint).lines() {
                lines.push(format!("  {hint_line}"));
            }
        }
    }
    lines.join("\n")
}

pub(in crate::cli) fn run_mir(args: Vec<String>) -> i32 {
    if args.len() != 1 {
        eprintln!("usage: axc mir <path>");
        return 2;
    }

    let path = PathBuf::from(&args[0]);
    let input = match load_input(&path) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    let source = &input.source;

    let output = analyze_with_project(source, input.project.as_ref());
    if !output.diagnostics.is_empty() {
        eprintln!("{}", render_diagnostics(source, &output.diagnostics));
        return 1;
    }

    let Some(mir) = output.mir else {
        eprintln!("internal error: MIR should be available after a successful analysis");
        return 1;
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&mir).expect("mir json should serialize")
    );
    0
}

pub(in crate::cli) fn run_run(args: Vec<String>) -> i32 {
    let options = match parse_run_args(args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!(
                "{error}\nusage: axc run <path> [--json] [--ai] [--ai-session <path>] [-- <args...>]"
            );
            return 2;
        }
    };

    let input = match load_input(&options.file) {
        Ok(input) => input,
        Err(error) => {
            return render_load_input_error(&options.file, &error, options.json, options.ai);
        }
    };
    let source = &input.source;

    let mut output = analyze_with_project(source, input.project.as_ref());
    if !output.diagnostics.is_empty() {
        if options.ai {
            if let Err(error) = enhance_diagnostics(
                source,
                &output.program,
                &mut output.diagnostics,
                options.ai_session.as_deref(),
            ) {
                eprintln!("{error}");
                return 1;
            }
        }

        if options.json {
            println!(
                "{}",
                serde_json::to_string_pretty(&output.diagnostics)
                    .expect("run diagnostics json should serialize")
            );
        } else {
            eprintln!("{}", render_diagnostics(source, &output.diagnostics));
        }
        return 1;
    }

    let Some(hir) = output.hir.as_ref() else {
        eprintln!("internal error: HIR should be available after a successful analysis");
        return 1;
    };

    let run_context = match RunContext::from_host(options.argv.clone()) {
        Ok(context) => context,
        Err(error) => {
            eprintln!("failed to capture host context: {error}");
            return 1;
        }
    };

    match run_program_with_context(source, hir, run_context) {
        Ok(result) => {
            for line in result.stdout {
                println!("{line}");
            }
            result.exit_code
        }
        Err(error) => {
            let mut diagnostics = vec![error];
            if options.ai {
                if let Err(error) = enhance_diagnostics(
                    source,
                    &output.program,
                    &mut diagnostics,
                    options.ai_session.as_deref(),
                ) {
                    eprintln!("{error}");
                    return 1;
                }
            }

            if options.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&diagnostics)
                        .expect("run diagnostics json should serialize")
                );
            } else {
                eprintln!("{}", render_diagnostics(source, &diagnostics));
            }
            1
        }
    }
}

pub(in crate::cli) fn run_fmt(args: Vec<String>) -> i32 {
    if args.len() != 1 {
        eprintln!("usage: axc fmt <path>");
        return 2;
    }

    let path = PathBuf::from(&args[0]);
    let input = match load_input(&path) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };

    if let Some(project) = input.project.as_ref() {
        return format_project_sources(project.program_source_paths());
    }

    format_single_source(&input.source)
}

pub(in crate::cli) fn run_context_command(args: Vec<String>) -> i32 {
    let options = match parse_context_args(args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!(
                "{error}\nusage: axc context <overview|boundaries|topology|flow> <path> [--json]\n       axc context <symbol|impact|evidence> <path> <symbol> [--json]"
            );
            return 2;
        }
    };

    let input = match load_input(&options.file) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };

    let output = check_only_with_project(&input.source, input.project.as_ref());
    let rendered = match render_context_json(
        options.view,
        &options.file,
        &input,
        &output.program,
        &output.diagnostics,
        options.symbol.as_deref(),
    ) {
        Ok(rendered) => rendered,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    print!("{rendered}");
    0
}

pub(in crate::cli) fn format_project_sources(paths: Vec<&Path>) -> i32 {
    for path in paths {
        let source = match crate::source::SourceFile::from_path(path) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("failed to read {}: {error}", path.display());
                return 1;
            }
        };

        let status = format_single_source(&source);
        if status != 0 {
            return status;
        }
    }

    0
}

pub(in crate::cli) fn format_single_source(source: &crate::source::SourceFile) -> i32 {
    let formatted = match format_source(source) {
        Ok(formatted) => formatted,
        Err(diagnostics) => {
            eprintln!("{}", render_diagnostics(source, &diagnostics));
            return 1;
        }
    };

    if source.text() == formatted {
        println!("already formatted: {}", source.path().display());
        return 0;
    }

    if let Err(error) = fs::write(source.path(), formatted) {
        eprintln!("failed to write {}: {error}", source.path().display());
        return 1;
    }

    println!("formatted: {}", source.path().display());
    0
}
