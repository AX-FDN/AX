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
