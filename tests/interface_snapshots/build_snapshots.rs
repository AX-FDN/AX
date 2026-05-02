use std::ffi::{OsStr, OsString};
use std::fs;

use serde_json::Value;

use super::support::*;

#[test]
fn project_build_manifest_matches_snapshot() {
    let temp = TempDir::new("project-build");
    temp.write(
        "AX.toml",
        "\
manifest_version = 1

[package]
name = \"project_hello\"
entry = \"src/main.ax\"
",
    );
    let input = temp.write_nested(
        "src/main.ax",
        "\
fn add(left: i32, right: i32) -> i32 {
    return left + right;
}

fn main() -> i32 {
    let value: i32 = add(1, 2);
    println(value);
    return 0;
}
",
    );
    let out_dir = temp.join("build-out");

    let output = run_axc([
        OsStr::new("build"),
        temp.path.as_os_str(),
        OsStr::new("--out-dir"),
        out_dir.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);

    let manifest_path = out_dir.join("build-manifest.json");
    let project_manifest_path = out_dir.join("AX.toml");
    let source_copy_path = out_dir.join("source.ax");
    let project_sources_dir = out_dir.join("project-sources");
    let copied_entry_path = project_sources_dir.join("src").join("main.ax");

    assert!(
        manifest_path.exists(),
        "project build manifest should exist"
    );
    assert!(
        project_manifest_path.exists(),
        "project build should copy AX.toml"
    );
    assert!(
        source_copy_path.exists(),
        "project build source copy should exist"
    );
    assert!(
        project_sources_dir.exists(),
        "project build should copy the original project source tree"
    );
    assert!(
        copied_entry_path.exists(),
        "project build should copy the entry source file"
    );

    let source_copy = normalize_text(
        &fs::read_to_string(&source_copy_path).expect("project build source copy should exist"),
    );
    let original = normalize_text(
        &fs::read_to_string(&input).expect("project build input source should exist"),
    );
    assert_eq!(source_copy, original);
    assert_eq!(
        normalize_text(
            &fs::read_to_string(&copied_entry_path)
                .expect("project build copied entry source should be readable"),
        ),
        original
    );

    let manifest_copy = normalize_text(
        &fs::read_to_string(&project_manifest_path)
            .expect("project build copied AX.toml should be readable"),
    );
    assert_eq!(
        manifest_copy,
        "manifest_version = 1\n\n[package]\nname = \"project_hello\"\nentry = \"src/main.ax\"\n"
    );

    let mut manifest: Value = serde_json::from_str(
        &fs::read_to_string(&manifest_path).expect("project build manifest should be readable"),
    )
    .expect("project build manifest should be valid JSON");
    manifest["entry_file"] = Value::String("<project>/src/main.ax".to_string());
    manifest["output_dir"] = Value::String("<build-out>".to_string());

    let rendered = serde_json::to_string_pretty(&manifest)
        .expect("project build manifest JSON should serialize")
        + "\n";
    assert_eq!(
        normalized_build_manifest_json(&rendered),
        normalized_build_manifest_json(&snapshot("build_project_hello_manifest.json"))
    );
}

#[test]
fn project_build_copies_source_tree_for_support_source_directories() {
    let temp = TempDir::new("project-build-sources-dir");
    temp.write(
        "AX.toml",
        "\
manifest_version = 1

[package]
name = \"project_build_sources\"
entry = \"src/main.ax\"
sources = [\"lib\"]
",
    );
    temp.write_nested(
        "lib/math.ax",
        "\
fn add(left: i32, right: i32) -> i32 {
    return left + right;
}
",
    );
    temp.write_nested(
        "lib/nested/report.ax",
        "\
fn render_total(total: i32) -> string {
    return \"total=\" + to_string(total);
}
",
    );
    temp.write_nested(
        "src/main.ax",
        "\
fn main() -> i32 {
    let total: i32 = add(2, 5);
    println(render_total(total));
    return total;
}
",
    );
    let out_dir = temp.join("build-out");

    let output = run_axc([
        OsStr::new("build"),
        temp.path.as_os_str(),
        OsStr::new("--out-dir"),
        out_dir.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);

    let project_manifest_copy = normalize_text(
        &fs::read_to_string(out_dir.join("AX.toml"))
            .expect("project build copied manifest should be readable"),
    );
    assert_eq!(
        project_manifest_copy,
        "\
manifest_version = 1

[package]
name = \"project_build_sources\"
entry = \"src/main.ax\"
sources = [\"lib\"]
"
    );

    let copied_math = normalize_text(
        &fs::read_to_string(out_dir.join("project-sources").join("lib").join("math.ax"))
            .expect("project build copied math source should be readable"),
    );
    assert_eq!(
        copied_math,
        "\
fn add(left: i32, right: i32) -> i32 {
    return left + right;
}
"
    );

    let copied_report = normalize_text(
        &fs::read_to_string(
            out_dir
                .join("project-sources")
                .join("lib")
                .join("nested")
                .join("report.ax"),
        )
        .expect("project build copied nested report source should be readable"),
    );
    assert_eq!(
        copied_report,
        "\
fn render_total(total: i32) -> string {
    return \"total=\" + to_string(total);
}
"
    );

    let copied_entry = normalize_text(
        &fs::read_to_string(out_dir.join("project-sources").join("src").join("main.ax"))
            .expect("project build copied entry source should be readable"),
    );
    assert_eq!(
        copied_entry,
        "\
fn main() -> i32 {
    let total: i32 = add(2, 5);
    println(render_total(total));
    return total;
}
"
    );

    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(out_dir.join("build-manifest.json"))
            .expect("project build manifest should be readable"),
    )
    .expect("project build manifest should be valid JSON");
    assert_eq!(
        json_string_array(&manifest["artifacts"]["project_sources"], "project sources"),
        vec![
            "lib/math.ax".to_string(),
            "lib/nested/report.ax".to_string(),
            "src/main.ax".to_string(),
        ]
    );
}

#[test]
fn project_workspace_search_report_build_copies_real_example_source_tree() {
    let temp = TempDir::new("project-workspace-search-build");
    let out_dir = temp.join("build-out");

    let output = run_axc([
        OsStr::new("build"),
        OsStr::new("examples/project_workspace_search_report"),
        OsStr::new("--out-dir"),
        out_dir.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);

    assert!(
        out_dir
            .join("project-sources")
            .join("src")
            .join("main.ax")
            .exists(),
        "build should copy the example entry source"
    );
    assert!(
        out_dir
            .join("project-sources")
            .join("lib")
            .join("file_search.ax")
            .exists(),
        "build should copy the example helper sources"
    );
    assert!(
        out_dir
            .join("project-sources")
            .join("external")
            .join("foundation")
            .join("cli.ax")
            .exists(),
        "build should copy shared sibling foundation sources"
    );
    assert!(
        out_dir
            .join("project-sources")
            .join("external")
            .join("foundation")
            .join("search.ax")
            .exists(),
        "build should copy shared foundation search helpers"
    );

    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(out_dir.join("build-manifest.json"))
            .expect("project build manifest should be readable"),
    )
    .expect("project build manifest should be valid JSON");
    assert_eq!(
        json_string_array(&manifest["artifacts"]["project_sources"], "project sources"),
        project_sources_with_shared_foundation(&[
            "lib/file_search.ax",
            "lib/report.ax",
            "lib/search_totals.ax",
            "src/main.ax",
        ])
    );
}

#[test]
fn project_command_capture_build_copies_real_example_source_tree() {
    assert_project_example_build_sources(
        "project-command-capture-build",
        "examples/project_command_capture",
        &project_sources_with_shared_std(&["src/main.ax"]),
    );
}

#[test]
fn project_release_promote_build_copies_real_example_source_tree() {
    assert_project_example_build_sources(
        "project-release-promote-build",
        "examples/project_release_promote",
        &project_sources_with_shared_std(&["lib/receipt.ax", "src/main.ax"]),
    );
}

#[test]
fn project_directory_index_build_copies_real_example_source_tree() {
    assert_project_example_build_sources(
        "project-directory-index-build",
        "examples/project_directory_index",
        &project_sources_with_shared_std(&[
            "lib/index_totals.ax",
            "lib/report.ax",
            "lib/scan.ax",
            "src/main.ax",
        ]),
    );
}

#[test]
fn project_command_batch_build_copies_real_example_source_tree() {
    assert_project_example_build_sources(
        "project-command-batch-build",
        "examples/project_command_batch",
        &project_sources_with_shared_std(&["lib/report.ax", "src/main.ax"]),
    );
}

#[test]
fn project_option_result_build_copies_real_example_source_tree() {
    assert_project_example_build_sources(
        "project-option-result-build",
        "examples/project_option_result",
        &project_sources_with_shared_std(&["src/main.ax"]),
    );
}

#[test]
fn project_env_result_build_copies_real_example_source_tree() {
    assert_project_example_build_sources(
        "project-env-result-build",
        "examples/project_env_result",
        &project_sources_with_shared_std(&["src/main.ax"]),
    );
}

#[test]
fn project_file_result_build_copies_real_example_source_tree() {
    assert_project_example_build_sources(
        "project-file-result-build",
        "examples/project_file_result",
        &project_sources_with_shared_std(&["src/main.ax"]),
    );
}

#[test]
fn project_process_result_build_copies_real_example_source_tree() {
    assert_project_example_build_sources(
        "project-process-result-build",
        "examples/project_process_result",
        &project_sources_with_shared_std(&["src/main.ax"]),
    );
}

#[test]
fn project_result_pipeline_build_copies_real_example_source_tree() {
    assert_project_example_build_sources(
        "project-result-pipeline-build",
        "examples/project_result_pipeline",
        &project_sources_with_shared_std(&["src/main.ax"]),
    );
}

#[test]
fn project_payload_event_report_build_copies_real_example_source_tree() {
    assert_project_example_build_sources(
        "project-payload-event-report-build",
        "examples/project_payload_event_report",
        &project_sources_with_shared_std(&["src/events.ax", "src/report.ax", "src/main.ax"]),
    );
}

#[test]
fn project_config_validate_build_copies_real_example_source_tree() {
    assert_project_example_build_sources(
        "project-config-validate-build",
        "examples/project_config_validate",
        &project_sources_with_shared_std(&["src/main.ax"]),
    );
}

#[test]
fn project_package_config_build_copies_real_example_source_tree() {
    let mut expected_sources = vec!["packages/config_rules/src/validate.ax".to_string()];
    expected_sources.extend(project_sources_with_shared_std(&["src/main.ax"]));
    assert_project_example_build_sources(
        "project-package-config-build",
        "examples/project_package_config",
        &expected_sources,
    );
}

#[test]
fn project_job_runner_build_copies_real_example_source_tree() {
    let mut expected_sources = vec![
        "packages/job_rules/src/jobs.ax".to_string(),
        "packages/job_rules/src/report.ax".to_string(),
    ];
    expected_sources.extend(project_sources_with_shared_std(&["src/main.ax"]));
    assert_project_example_build_sources(
        "project-job-runner-build",
        "examples/project_job_runner",
        &expected_sources,
    );
}

#[test]
fn project_package_config_build_manifest_exposes_local_path_package() {
    let temp = TempDir::new("project-package-config-build-manifest");
    let out_dir = temp.join("build-out");

    let output = run_axc([
        OsStr::new("build"),
        OsStr::new("examples/project_package_config"),
        OsStr::new("--out-dir"),
        out_dir.as_os_str(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "build should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(out_dir.join("build-manifest.json"))
            .expect("build manifest should be readable"),
    )
    .expect("build manifest should be valid JSON");
    let packages = manifest["local_path_packages"]
        .as_array()
        .expect("build manifest should expose local_path_packages");
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0]["alias"], "config_rules");
    assert_eq!(packages[0]["root"], "packages/config_rules");
    assert_eq!(packages[0]["manifest"], "packages/config_rules/AX.toml");
    assert_eq!(packages[0]["source_count"], 1);
    assert_eq!(
        json_string_array(&packages[0]["modules"], "local package modules"),
        vec!["config_rules.validate".to_string()]
    );
    let package_graph = &manifest["package_graph_readiness"];
    assert_eq!(package_graph["package_mode"], "local_path_v0");
    assert_eq!(package_graph["reproducible"], Value::Bool(true));
    assert_eq!(package_graph["aot_ready"], Value::Bool(false));
    assert_eq!(package_graph["lock_status"], "current");
    assert_eq!(package_graph["risk_level"], "medium");
    assert!(
        json_string_array(
            &package_graph["blocking_reasons"],
            "build package graph blocking reasons"
        )
        .iter()
        .any(|reason| reason.contains("local path package linking")),
        "build manifest should keep package graph out of AOT-ready state"
    );
}

#[test]
fn project_job_runner_runs_on_controlled_fixture() {
    let temp = TempDir::new("project-job-runner");
    let output_dir = temp.join("out");

    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/project_job_runner"),
        OsStr::new("--"),
        repo_root().as_os_str(),
        output_dir.as_os_str(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "job runner should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_temp_output(&string_output(&output.stdout), &temp),
        "job_runner_report=<root>/out/JOB-RUNNER.txt\n"
    );

    let report =
        fs::read_to_string(output_dir.join("JOB-RUNNER.txt")).expect("job report should exist");
    assert_eq!(
        normalize_text(&report),
        "job_runner=local_path_package\njob_0_name=check\njob_0_kind=check\njob_0_exit=0\njob_0_env_ready=true\njob_1_name=package\njob_1_kind=package\njob_1_exit=0\njob_1_env_ready=true\njob_2_name=publish\njob_2_kind=publish\njob_2_exit=0\njob_2_env_ready=true\nfailure_count=0\npackage_backed=true\n"
    );
}

#[test]
fn project_job_runner_lock_and_context_expose_package_graph() {
    let check_output = run_axc([
        OsStr::new("lock"),
        OsStr::new("examples/project_job_runner"),
        OsStr::new("--check"),
    ]);
    assert_eq!(
        check_output.status.code(),
        Some(0),
        "lock --check should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&check_output.stdout),
        string_output(&check_output.stderr)
    );
    assert_clean_stderr(&check_output);

    let evidence_output = run_axc([
        OsStr::new("context"),
        OsStr::new("evidence"),
        OsStr::new("examples/project_job_runner"),
        OsStr::new("main"),
        OsStr::new("--json"),
    ]);
    assert_eq!(
        evidence_output.status.code(),
        Some(0),
        "context evidence should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&evidence_output.stdout),
        string_output(&evidence_output.stderr)
    );
    assert_clean_stderr(&evidence_output);
    let evidence: Value =
        serde_json::from_slice(&evidence_output.stdout).expect("evidence should be JSON");
    assert_eq!(evidence["facts"]["local_package_lock"]["status"], "current");
    assert_eq!(
        evidence["facts"]["package_graph_readiness"]["package_mode"],
        "local_path_v0"
    );
    assert_eq!(
        evidence["facts"]["package_graph_readiness"]["reproducible"],
        Value::Bool(true)
    );
    assert_eq!(
        evidence["facts"]["package_graph_readiness"]["aot_ready"],
        Value::Bool(false)
    );
    assert!(
        json_string_array(
            &evidence["facts"]["package_graph_readiness"]["blocking_reasons"],
            "package graph readiness blocking reasons"
        )
        .iter()
        .any(|reason| reason.contains("AOT")),
        "context evidence should keep local packages out of AOT-ready state"
    );
}

#[test]
fn project_build_manifest_exposes_external_local_path_package() {
    let temp = TempDir::new("project-external-package-build-manifest");
    let project_dir = temp.join("app");
    let shared_pkg_dir = temp.join("shared_rules");
    fs::create_dir_all(project_dir.join("src")).expect("project src directory should exist");
    fs::create_dir_all(shared_pkg_dir.join("src"))
        .expect("shared package src directory should exist");
    fs::write(
        project_dir.join("AX.toml"),
        "\
manifest_version = 1

[package]
name = \"external_package_app\"
entry = \"src/main.ax\"

[dependencies]
shared_rules = { path = \"../shared_rules\" }
",
    )
    .expect("project manifest should exist");
    fs::write(
        shared_pkg_dir.join("AX.toml"),
        "\
manifest_version = 1

[package]
name = \"shared_rules\"
sources = [\"src\"]
",
    )
    .expect("shared package manifest should exist");
    fs::write(
        shared_pkg_dir.join("src").join("validate.ax"),
        "module shared_rules.validate;\nfn require_port(port: i32) -> i32 { return port; }\n",
    )
    .expect("shared package source should exist");
    fs::write(
        project_dir.join("src").join("main.ax"),
        "import shared_rules.validate;\nfn main() -> i32 { return shared_rules.validate.require_port(8080); }\n",
    )
    .expect("project entry should exist");

    let out_dir = temp.join("build-out");
    let output = run_axc([
        OsStr::new("build"),
        project_dir.as_os_str(),
        OsStr::new("--out-dir"),
        out_dir.as_os_str(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "build should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(out_dir.join("build-manifest.json"))
            .expect("build manifest should be readable"),
    )
    .expect("build manifest should be valid JSON");
    let packages = manifest["local_path_packages"]
        .as_array()
        .expect("build manifest should expose local_path_packages");
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0]["alias"], "shared_rules");
    assert_eq!(packages[0]["root"], "external/shared_rules");
    assert_eq!(packages[0]["manifest"], "external/shared_rules/AX.toml");
    assert_eq!(packages[0]["source_count"], 1);
    assert_eq!(
        json_string_array(&packages[0]["modules"], "external local package modules"),
        vec!["shared_rules.validate".to_string()]
    );
    assert!(
        out_dir
            .join("project-sources")
            .join("external")
            .join("shared_rules")
            .join("src")
            .join("validate.ax")
            .exists(),
        "build should copy sibling local path package sources under external/"
    );
}

#[test]
fn project_build_manifest_exposes_stale_package_graph_readiness() {
    let temp = TempDir::new("project-build-stale-package-graph");
    let project_dir = temp.join("app");
    let package_dir = temp.join("packages").join("rules");
    fs::create_dir_all(project_dir.join("src")).expect("project src directory should exist");
    fs::create_dir_all(package_dir.join("src")).expect("package src directory should exist");
    fs::write(
        project_dir.join("AX.toml"),
        "\
manifest_version = 1

[package]
name = \"build_stale_package_app\"
entry = \"src/main.ax\"

[dependencies]
rules = { path = \"../packages/rules\" }
",
    )
    .expect("project manifest should exist");
    fs::write(
        package_dir.join("AX.toml"),
        "\
manifest_version = 1

[package]
name = \"rules_pkg\"
sources = [\"src\"]
",
    )
    .expect("package manifest should exist");
    fs::write(
        package_dir.join("src").join("validate.ax"),
        "module rules.validate;\nfn ok() -> i32 { return 1; }\n",
    )
    .expect("package source should exist");
    fs::write(
        project_dir.join("src").join("main.ax"),
        "import rules.validate;\nfn main() -> i32 { return rules.validate.ok(); }\n",
    )
    .expect("project entry should exist");

    let lock_output = run_axc([OsStr::new("lock"), project_dir.as_os_str()]);
    assert_eq!(lock_output.status.code(), Some(0));
    assert_clean_stderr(&lock_output);
    fs::write(
        package_dir.join("src").join("extra.ax"),
        "module rules.extra;\nfn value() -> i32 { return 2; }\n",
    )
    .expect("extra package source should exist");

    let out_dir = temp.join("build-out");
    let output = run_axc([
        OsStr::new("build"),
        project_dir.as_os_str(),
        OsStr::new("--out-dir"),
        out_dir.as_os_str(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "build should succeed while reporting package graph readiness\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(out_dir.join("build-manifest.json"))
            .expect("build manifest should be readable"),
    )
    .expect("build manifest should be valid JSON");
    let package_graph = &manifest["package_graph_readiness"];
    assert_eq!(package_graph["lock_status"], "stale");
    assert_eq!(package_graph["risk_level"], "high");
    assert_eq!(package_graph["reproducible"], Value::Bool(false));
    assert_eq!(package_graph["aot_ready"], Value::Bool(false));
    assert!(
        json_string_array(
            &package_graph["blocking_reasons"],
            "build package graph blocking reasons"
        )
        .iter()
        .any(|reason| reason.contains("AX.lock status is `stale`")),
        "build manifest should expose stale lock risk"
    );
}

#[test]
fn project_lock_generates_and_checks_local_path_packages() {
    let temp = TempDir::new("project-lock-local-path-package");
    let project_dir = temp.join("app");
    let package_dir = temp.join("packages").join("rules");
    fs::create_dir_all(project_dir.join("src")).expect("project src directory should exist");
    fs::create_dir_all(package_dir.join("src")).expect("package src directory should exist");
    fs::write(
        project_dir.join("AX.toml"),
        "\
manifest_version = 1

[package]
name = \"lock_app\"
entry = \"src/main.ax\"

[dependencies]
rules = { path = \"../packages/rules\" }
",
    )
    .expect("project manifest should exist");
    fs::write(
        package_dir.join("AX.toml"),
        "\
manifest_version = 1

[package]
name = \"rules_pkg\"
sources = [\"src\"]
",
    )
    .expect("package manifest should exist");
    fs::write(
        package_dir.join("src").join("validate.ax"),
        "module rules.validate;\nfn ok() -> i32 { return 1; }\n",
    )
    .expect("package source should exist");
    fs::write(
        project_dir.join("src").join("main.ax"),
        "import rules.validate;\nfn main() -> i32 { return rules.validate.ok(); }\n",
    )
    .expect("project entry should exist");

    let output = run_axc([OsStr::new("lock"), project_dir.as_os_str()]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "lock should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let lock_path = project_dir.join("AX.lock");
    let lock: Value =
        serde_json::from_str(&fs::read_to_string(&lock_path).expect("AX.lock should be readable"))
            .expect("AX.lock should be valid JSON");
    assert_eq!(lock["schema_version"], 1);
    assert_eq!(lock["package"]["name"], "lock_app");
    let dependencies = lock["dependencies"]
        .as_array()
        .expect("AX.lock dependencies should be an array");
    assert_eq!(dependencies.len(), 1);
    assert_eq!(dependencies[0]["alias"], "rules");
    assert_eq!(dependencies[0]["kind"], "path");
    assert_eq!(dependencies[0]["package"], "rules_pkg");
    assert_eq!(dependencies[0]["path"], "../packages/rules");
    assert_eq!(dependencies[0]["manifest"], "../packages/rules/AX.toml");
    assert_eq!(dependencies[0]["source_count"], 1);
    assert_eq!(
        json_string_array(&dependencies[0]["modules"], "locked package modules"),
        vec!["rules.validate".to_string()]
    );

    let check_output = run_axc([
        OsStr::new("lock"),
        project_dir.as_os_str(),
        OsStr::new("--check"),
    ]);
    assert_eq!(
        check_output.status.code(),
        Some(0),
        "lock --check should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&check_output.stdout),
        string_output(&check_output.stderr)
    );
    assert_clean_stderr(&check_output);

    let overview_output = run_axc([
        OsStr::new("context"),
        OsStr::new("overview"),
        project_dir.as_os_str(),
        OsStr::new("--json"),
    ]);
    assert_eq!(
        overview_output.status.code(),
        Some(0),
        "context overview should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&overview_output.stdout),
        string_output(&overview_output.stderr)
    );
    assert_clean_stderr(&overview_output);
    let overview: Value =
        serde_json::from_slice(&overview_output.stdout).expect("overview should be JSON");
    assert_eq!(overview["facts"]["local_package_lock"]["status"], "current");
    assert_eq!(
        overview["facts"]["local_package_lock"]["dependency_count"],
        Value::from(1)
    );

    let topology_output = run_axc([
        OsStr::new("context"),
        OsStr::new("topology"),
        project_dir.as_os_str(),
        OsStr::new("--json"),
    ]);
    assert_eq!(
        topology_output.status.code(),
        Some(0),
        "context topology should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&topology_output.stdout),
        string_output(&topology_output.stderr)
    );
    assert_clean_stderr(&topology_output);
    let topology: Value =
        serde_json::from_slice(&topology_output.stdout).expect("topology should be JSON");
    assert_eq!(topology["facts"]["local_package_lock"]["status"], "current");

    let evidence_output = run_axc([
        OsStr::new("context"),
        OsStr::new("evidence"),
        project_dir.as_os_str(),
        OsStr::new("main"),
        OsStr::new("--json"),
    ]);
    assert_eq!(
        evidence_output.status.code(),
        Some(0),
        "context evidence should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&evidence_output.stdout),
        string_output(&evidence_output.stderr)
    );
    assert_clean_stderr(&evidence_output);
    let evidence: Value =
        serde_json::from_slice(&evidence_output.stdout).expect("evidence should be JSON");
    assert_eq!(evidence["facts"]["local_package_lock"]["status"], "current");
    assert_eq!(
        evidence["facts"]["package_graph_readiness"]["package_mode"],
        "local_path_v0"
    );
    assert_eq!(
        evidence["facts"]["package_graph_readiness"]["reproducible"],
        Value::Bool(true)
    );
    assert_eq!(
        evidence["facts"]["package_graph_readiness"]["aot_ready"],
        Value::Bool(false)
    );
    assert_eq!(
        evidence["facts"]["package_graph_readiness"]["risk_level"],
        "medium"
    );
    assert!(
        json_string_array(
            &evidence["hints"]["recommended_commands"],
            "evidence recommended commands"
        )
        .iter()
        .any(|command| command.contains("axc lock") && command.contains("--check")),
        "evidence should recommend lockfile verification for local path package projects"
    );
}

#[test]
fn project_lock_check_reports_stale_package_graph_details() {
    let temp = TempDir::new("project-lock-stale-package");
    let project_dir = temp.join("app");
    let package_dir = temp.join("packages").join("rules");
    fs::create_dir_all(project_dir.join("src")).expect("project src directory should exist");
    fs::create_dir_all(package_dir.join("src")).expect("package src directory should exist");
    fs::write(
        project_dir.join("AX.toml"),
        "\
manifest_version = 1

[package]
name = \"lock_stale_app\"
entry = \"src/main.ax\"

[dependencies]
rules = { path = \"../packages/rules\" }
",
    )
    .expect("project manifest should exist");
    fs::write(
        package_dir.join("AX.toml"),
        "\
manifest_version = 1

[package]
name = \"rules_pkg\"
sources = [\"src\"]
",
    )
    .expect("package manifest should exist");
    fs::write(
        package_dir.join("src").join("validate.ax"),
        "module rules.validate;\nfn ok() -> i32 { return 1; }\n",
    )
    .expect("package source should exist");
    fs::write(
        project_dir.join("src").join("main.ax"),
        "import rules.validate;\nfn main() -> i32 { return rules.validate.ok(); }\n",
    )
    .expect("project entry should exist");

    let lock_output = run_axc([OsStr::new("lock"), project_dir.as_os_str()]);
    assert_eq!(lock_output.status.code(), Some(0));
    assert_clean_stderr(&lock_output);

    fs::write(
        package_dir.join("src").join("extra.ax"),
        "module rules.extra;\nfn value() -> i32 { return 2; }\n",
    )
    .expect("extra package source should exist");

    let check_output = run_axc([
        OsStr::new("lock"),
        project_dir.as_os_str(),
        OsStr::new("--check"),
    ]);
    assert_eq!(check_output.status.code(), Some(1));
    let stderr = normalize_temp_output(&string_output(&check_output.stderr), &temp);
    assert!(
        stderr.contains("LX0002: AX.lock stale"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("dependency_source_count_changed"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("dependency_modules_changed"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("fixit: regenerate AX.lock with `axc lock <project>`"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("repair_rule: package_lockfile_must_match_graph"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains(
            "repair_goal: Regenerate AX.lock so it matches the current local path package graph."
        ),
        "stderr:\n{stderr}"
    );

    let overview_output = run_axc([
        OsStr::new("context"),
        OsStr::new("overview"),
        project_dir.as_os_str(),
        OsStr::new("--json"),
    ]);
    assert_eq!(
        overview_output.status.code(),
        Some(0),
        "context overview should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&overview_output.stdout),
        string_output(&overview_output.stderr)
    );
    assert_clean_stderr(&overview_output);

    let overview: Value =
        serde_json::from_slice(&overview_output.stdout).expect("overview should be JSON");
    let lock = &overview["facts"]["local_package_lock"];
    assert_eq!(lock["status"], "stale");
    let issues = lock["issues"]
        .as_array()
        .expect("local_package_lock issues should be an array");
    assert!(
        issues
            .iter()
            .any(|issue| issue["kind"] == "dependency_modules_changed"),
        "context should expose module drift issue: {issues:?}"
    );
    assert!(
        issues.iter().any(|issue| {
            issue["repair_rule"] == "package_lockfile_must_match_graph"
                && issue["repair_goal"]
                    == "Regenerate AX.lock so it matches the current local path package graph."
        }),
        "context should expose AI-facing lock repair hints: {issues:?}"
    );

    let evidence_output = run_axc([
        OsStr::new("context"),
        OsStr::new("evidence"),
        project_dir.as_os_str(),
        OsStr::new("main"),
        OsStr::new("--json"),
    ]);
    assert_eq!(
        evidence_output.status.code(),
        Some(0),
        "context evidence should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&evidence_output.stdout),
        string_output(&evidence_output.stderr)
    );
    assert_clean_stderr(&evidence_output);
    let evidence: Value =
        serde_json::from_slice(&evidence_output.stdout).expect("evidence should be JSON");
    assert_eq!(
        evidence["facts"]["package_graph_readiness"]["lock_status"],
        "stale"
    );
    assert_eq!(
        evidence["facts"]["package_graph_readiness"]["risk_level"],
        "high"
    );
    assert_eq!(
        evidence["facts"]["package_graph_readiness"]["reproducible"],
        Value::Bool(false)
    );
    assert!(
        json_string_array(
            &evidence["facts"]["package_graph_readiness"]["blocking_reasons"],
            "package graph readiness blocking reasons"
        )
        .iter()
        .any(|reason| reason.contains("AX.lock status is `stale`")),
        "evidence should explain stale lock risk"
    );
}

#[test]
fn project_collections_report_build_copies_real_example_source_tree() {
    assert_project_example_build_sources(
        "project-collections-report-build",
        "examples/project_collections_report",
        &project_sources_with_shared_std(&["src/main.ax"]),
    );
}

#[test]
fn project_text_normalize_build_copies_real_example_source_tree() {
    assert_project_example_build_sources(
        "project-text-normalize-build",
        "examples/project_text_normalize",
        &project_sources_with_shared_std(&["lib/normalize.ax", "lib/report.ax", "src/main.ax"]),
    );
}

#[test]
fn build_manifest_matches_snapshot() {
    let temp = TempDir::new("build");
    let input = temp.write(
        "hello.ax",
        "\
fn main() -> i32 {
    let mut value: i32 = 1;
    value = value + 2;
    println(value);
    return 0;
}
",
    );
    let out_dir = temp.join("build-out");

    let output = run_axc([
        OsStr::new("build"),
        input.as_os_str(),
        OsStr::new("--out-dir"),
        out_dir.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);

    let manifest_path = out_dir.join("build-manifest.json");
    let hir_path = out_dir.join("program.hir.json");
    let mir_path = out_dir.join("program.mir.json");
    let source_copy_path = out_dir.join("source.ax");
    let planned_binary_dir = out_dir.join("bin");

    assert!(manifest_path.exists(), "build manifest should exist");
    assert!(hir_path.exists(), "build HIR should exist");
    assert!(mir_path.exists(), "build MIR should exist");
    assert!(source_copy_path.exists(), "build source copy should exist");
    assert!(
        planned_binary_dir.exists(),
        "build bin directory should exist even before native backend emission"
    );

    let source_copy = normalize_text(
        &fs::read_to_string(&source_copy_path).expect("build source copy should be readable"),
    );
    let original = normalize_text(
        &fs::read_to_string(&input).expect("original build input should be readable"),
    );
    assert_eq!(source_copy, original);

    let mut manifest: Value = serde_json::from_str(
        &fs::read_to_string(&manifest_path).expect("build manifest should be readable"),
    )
    .expect("build manifest should be valid JSON");
    manifest["entry_file"] = Value::String("<input>/hello.ax".to_string());
    manifest["output_dir"] = Value::String("<build-out>".to_string());

    let rendered = serde_json::to_string_pretty(&manifest)
        .expect("build manifest JSON should serialize")
        + "\n";
    assert_eq!(
        normalized_build_manifest_json(&rendered),
        normalized_build_manifest_json(&snapshot("build_hello_manifest.json"))
    );

    let stdout = normalize_text(&string_output(&output.stdout));
    assert!(
        stdout.starts_with("build succeeded: "),
        "expected build success message, got:\n{}",
        stdout
    );
}

#[test]
fn build_json_prints_build_manifest_object() {
    let temp = TempDir::new("build-json");
    let input = temp.write(
        "hello.ax",
        "\
fn main() -> i32 {
    return 0;
}
",
    );
    let out_dir = temp.join("build-out");

    let output = run_axc([
        OsStr::new("build"),
        input.as_os_str(),
        OsStr::new("--out-dir"),
        out_dir.as_os_str(),
        OsStr::new("--json"),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "build --json should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let printed: Value = serde_json::from_str(&string_output(&output.stdout))
        .expect("build --json stdout should be a manifest JSON object");
    let manifest = read_json_file(
        &out_dir.join("build-manifest.json"),
        "build --json manifest file",
    );

    assert_eq!(printed, manifest);
    assert_eq!(printed["schema_version"], Value::from(9));
    assert_eq!(printed["user_code_valid"], Value::Bool(true));
    assert_eq!(printed["interpreter_supported"], Value::Bool(true));
    assert!(
        !string_output(&output.stdout).contains("build succeeded"),
        "build --json should not mix text success output into stdout"
    );
}

#[test]
fn llvm_aot_return_build_emits_ir_artifact_without_linking_by_default() {
    let temp = TempDir::new("llvm-aot-return-build");
    let out_dir = temp.join("build-out");

    let output = run_axc_with_removed_env(
        [
            OsStr::new("build"),
            OsStr::new("examples/aot_return.ax"),
            OsStr::new("--out-dir"),
            out_dir.as_os_str(),
        ],
        ["AX_LLVM_AOT_LINK", "AX_LLVM_CLANG"],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "LLVM AOT IR build should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let manifest_path = out_dir.join("build-manifest.json");
    let llvm_ir_path = out_dir.join("generated").join("main.ll");
    let planned_executable_path = out_dir.join("bin").join(format!(
        "aot_return{}",
        if cfg!(windows) { ".exe" } else { "" }
    ));

    assert!(
        manifest_path.exists(),
        "LLVM AOT build should keep writing build-manifest.json"
    );
    assert!(llvm_ir_path.exists(), "LLVM AOT build should emit main.ll");
    assert!(
        !planned_executable_path.exists(),
        "LLVM AOT build should not link an executable unless AX_LLVM_AOT_LINK=1"
    );

    let manifest = read_json_file(&manifest_path, "LLVM AOT build manifest");
    assert_eq!(manifest["schema_version"], Value::from(9));
    assert_eq!(manifest["user_code_valid"], Value::Bool(true));
    assert_eq!(manifest["interpreter_supported"], Value::Bool(true));
    assert_eq!(manifest["aot_supported"], Value::Bool(false));
    assert_eq!(manifest["backend"]["kind"], Value::from("llvm-aot"));
    assert_eq!(manifest["backend"]["status"], Value::from("ir_generated"));
    assert_eq!(
        manifest["artifacts"]["llvm_ir"],
        Value::from("generated/main.ll")
    );
    assert!(
        manifest["artifacts"]["executable"].is_null(),
        "LLVM AOT build should omit executable artifact when linking is skipped"
    );
    assert_eq!(
        manifest["aot_readiness"]["stage"],
        Value::from("Build-1 LLVM IR prototype")
    );
    assert_eq!(
        manifest["aot_readiness"]["status"],
        Value::from("ir_generated")
    );
    let blockers = manifest["aot_readiness"]["blockers"]
        .as_array()
        .expect("AOT readiness blockers should be an array");
    assert_eq!(blockers.len(), 1);
    assert_eq!(blockers[0]["code"], Value::from("AOT1000"));
    assert_eq!(
        blockers[0]["ai"]["rule_id"],
        Value::from("aot_linking_must_be_enabled")
    );
    assert_eq!(blockers[0]["ai"]["layer"], Value::from("toolchain_link"));
    assert_eq!(
        blockers[0]["ai"]["ai_action"],
        Value::from("enable_linking")
    );
    assert_eq!(blockers[0]["ai"]["safe_to_edit"], Value::Bool(false));

    let llvm_ir = normalize_text(
        &fs::read_to_string(&llvm_ir_path).expect("LLVM IR artifact should be readable"),
    );
    assert!(llvm_ir.contains("define i32 @ax_add(i32 %arg0, i32 %arg1)"));
    assert!(llvm_ir.contains("define i32 @main()"));
    assert!(llvm_ir.contains("call i32 @ax_add(i32 40, i32 2)"));
}

#[test]
fn llvm_aot_lowering_unsupported_is_readiness_blocker() {
    let temp = TempDir::new("llvm-aot-f32-unsupported");
    let input = temp.write(
        "aot_f32.ax",
        "\
fn main() -> i32 {
    let value: f32 = 1.5;
    return 0;
}
",
    );
    let out_dir = temp.join("build-out");

    let output = run_axc_with_removed_env(
        [
            OsStr::new("build"),
            input.as_os_str(),
            OsStr::new("--out-dir"),
            out_dir.as_os_str(),
        ],
        ["AX_LLVM_AOT_LINK", "AX_LLVM_CLANG"],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "LLVM AOT unsupported lowering should still emit a manifest\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let manifest = read_json_file(
        &out_dir.join("build-manifest.json"),
        "LLVM AOT unsupported lowering manifest",
    );
    assert_eq!(manifest["schema_version"], Value::from(9));
    assert_eq!(manifest["user_code_valid"], Value::Bool(true));
    assert_eq!(manifest["interpreter_supported"], Value::Bool(true));
    assert_eq!(manifest["aot_supported"], Value::Bool(false));
    assert_eq!(manifest["backend"]["kind"], Value::from("llvm-aot"));
    assert_eq!(manifest["backend"]["status"], Value::from("unsupported"));
    assert!(
        manifest["artifacts"]["llvm_ir"].is_null(),
        "unsupported LLVM lowering should not claim an IR artifact"
    );
    assert_eq!(
        manifest["aot_readiness"]["stage"],
        Value::from("Build-1 LLVM IR prototype")
    );
    assert_eq!(manifest["aot_readiness"]["status"], Value::from("blocked"));

    let blockers = manifest["aot_readiness"]["blockers"]
        .as_array()
        .expect("AOT readiness blockers should be an array");
    assert!(blockers.iter().any(|blocker| {
        blocker["code"] == Value::from("AOT2001")
            && blocker["category"] == Value::from("llvm_lowering")
            && blocker["resolution"]["agent_action"] == Value::from("explain_unsupported")
            && blocker["resolution"]["source_edit_safe"] == Value::Bool(false)
            && blocker["ai"]["rule_id"] == Value::from("aot_llvm_lowering_unsupported")
            && blocker["ai"]["layer"] == Value::from("llvm_lowering")
            && blocker["ai"]["safe_to_edit"] == Value::Bool(false)
    }));

    let notes = manifest["notes"]
        .as_array()
        .expect("manifest notes should be an array")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        notes.contains("unsupported type f32"),
        "unsupported LLVM lowering notes should retain the concrete reason: {notes}"
    );
}

#[test]
fn llvm_aot_core_examples_check_run_and_emit_ir_without_linking_by_default() {
    for (label, example, target_name, expected_exit_code) in [
        (
            "llvm-aot-math-build",
            "examples/aot_math.ax",
            "aot_math",
            16,
        ),
        (
            "llvm-aot-control-flow-build",
            "examples/aot_control_flow.ax",
            "aot_control_flow",
            3,
        ),
        (
            "llvm-aot-loop-build",
            "examples/aot_loop.ax",
            "aot_loop",
            10,
        ),
        (
            "llvm-aot-bool-logic-build",
            "examples/aot_bool_logic.ax",
            "aot_bool_logic",
            7,
        ),
        (
            "llvm-aot-comparisons-build",
            "examples/aot_comparisons.ax",
            "aot_comparisons",
            11,
        ),
        (
            "llvm-aot-nested-calls-build",
            "examples/aot_nested_calls.ax",
            "aot_nested_calls",
            17,
        ),
        (
            "llvm-aot-print-build",
            "examples/aot_print.ax",
            "aot_print",
            5,
        ),
        (
            "llvm-aot-print-string-build",
            "examples/aot_print_string.ax",
            "aot_print_string",
            6,
        ),
        (
            "llvm-aot-string-values-build",
            "examples/aot_string_values.ax",
            "aot_string_values",
            8,
        ),
        (
            "llvm-aot-string-len-compare-build",
            "examples/aot_string_len_compare.ax",
            "aot_string_len_compare",
            7,
        ),
        (
            "llvm-aot-string-runtime-build",
            "examples/aot_string_runtime.ax",
            "aot_string_runtime",
            25,
        ),
        (
            "llvm-aot-array-read-build",
            "examples/aot_array_read.ax",
            "aot_array_read",
            20,
        ),
    ] {
        let check_output = run_axc([OsStr::new("check"), OsStr::new(example)]);
        assert_eq!(
            check_output.status.code(),
            Some(0),
            "AOT core example `{example}` should check\nstdout:\n{}\nstderr:\n{}",
            string_output(&check_output.stdout),
            string_output(&check_output.stderr)
        );
        assert_clean_stderr(&check_output);

        let run_output = run_axc([OsStr::new("run"), OsStr::new(example)]);
        assert_eq!(
            run_output.status.code(),
            Some(expected_exit_code),
            "AOT core example `{example}` should keep interpreter semantics\nstdout:\n{}\nstderr:\n{}",
            string_output(&run_output.stdout),
            string_output(&run_output.stderr)
        );
        assert_clean_stderr(&run_output);

        let temp = TempDir::new(label);
        let out_dir = temp.join("build-out");
        let build_output = run_axc_with_removed_env(
            [
                OsStr::new("build"),
                OsStr::new(example),
                OsStr::new("--out-dir"),
                out_dir.as_os_str(),
            ],
            ["AX_LLVM_AOT_LINK", "AX_LLVM_CLANG"],
        );
        assert_eq!(
            build_output.status.code(),
            Some(0),
            "AOT core example `{example}` should build to LLVM IR\nstdout:\n{}\nstderr:\n{}",
            string_output(&build_output.stdout),
            string_output(&build_output.stderr)
        );
        assert_clean_stderr(&build_output);

        let manifest = read_json_file(
            &out_dir.join("build-manifest.json"),
            "AOT core example build manifest",
        );
        assert_eq!(manifest["schema_version"], Value::from(9));
        assert_eq!(manifest["aot_readiness"]["schema_version"], Value::from(3));
        assert_eq!(manifest["user_code_valid"], Value::Bool(true));
        assert_eq!(manifest["interpreter_supported"], Value::Bool(true));
        assert_eq!(manifest["aot_supported"], Value::Bool(false));
        assert_eq!(manifest["target_name"], Value::from(target_name));
        assert_eq!(manifest["backend"]["kind"], Value::from("llvm-aot"));
        assert_eq!(manifest["backend"]["status"], Value::from("ir_generated"));
        assert_eq!(
            manifest["artifacts"]["llvm_ir"],
            Value::from("generated/main.ll")
        );
        assert!(
            out_dir.join("generated").join("main.ll").exists(),
            "AOT core example `{example}` should emit LLVM IR"
        );
        let blockers = manifest["aot_readiness"]["blockers"]
            .as_array()
            .expect("AOT readiness blockers should be an array");
        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0]["code"], Value::from("AOT1000"));
        assert_eq!(
            blockers[0]["resolution"]["agent_action"],
            Value::from("enable_linking")
        );
        assert_eq!(
            blockers[0]["resolution"]["source_edit_safe"],
            Value::Bool(false)
        );
    }
}

#[test]
fn llvm_aot_link_reports_missing_clang_as_readiness_blocker() {
    let temp = TempDir::new("llvm-aot-missing-clang");
    let out_dir = temp.join("build-out");
    let missing_clang = temp.join("missing-clang").join("clang.exe");
    let env_pairs = vec![
        (OsString::from("AX_LLVM_AOT_LINK"), OsString::from("1")),
        (
            OsString::from("AX_LLVM_CLANG"),
            missing_clang.as_os_str().to_os_string(),
        ),
    ];

    let output = run_axc_with_env(
        [
            OsStr::new("build"),
            OsStr::new("examples/aot_return.ax"),
            OsStr::new("--out-dir"),
            out_dir.as_os_str(),
        ],
        env_pairs,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "LLVM AOT build should report missing clang without failing build\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let manifest = read_json_file(
        &out_dir.join("build-manifest.json"),
        "missing clang AOT build manifest",
    );
    assert_eq!(manifest["schema_version"], Value::from(9));
    assert_eq!(manifest["user_code_valid"], Value::Bool(true));
    assert_eq!(manifest["interpreter_supported"], Value::Bool(true));
    assert_eq!(manifest["aot_supported"], Value::Bool(false));
    assert_eq!(manifest["backend"]["kind"], Value::from("llvm-aot"));
    assert_eq!(manifest["backend"]["status"], Value::from("ir_generated"));
    assert!(
        manifest["artifacts"]["executable"].is_null(),
        "missing clang should not claim an executable artifact"
    );
    let blockers = manifest["aot_readiness"]["blockers"]
        .as_array()
        .expect("AOT readiness blockers should be an array");
    assert_eq!(blockers.len(), 1);
    assert_eq!(blockers[0]["code"], Value::from("AOT1001"));
    assert_eq!(
        blockers[0]["ai"]["rule_id"],
        Value::from("aot_clang_toolchain_required")
    );
    assert_eq!(blockers[0]["ai"]["layer"], Value::from("toolchain_link"));
    assert_eq!(
        blockers[0]["ai"]["ai_action"],
        Value::from("configure_toolchain")
    );
    assert_eq!(blockers[0]["ai"]["safe_to_edit"], Value::Bool(false));
    assert_eq!(blockers[0]["category"], Value::from("toolchain"));
    assert_eq!(
        blockers[0]["resolution"]["agent_action"],
        Value::from("configure_toolchain")
    );
    assert_eq!(
        blockers[0]["resolution"]["recommended_command"],
        Value::from("$env:AX_LLVM_CLANG = \"<path-to-clang>\"")
    );
}

#[test]
fn fmt_is_idempotent_and_matches_snapshot() {
    let temp = TempDir::new("fmt");
    let input = temp.write(
        "format_me.ax",
        "\
fn main() -> i32 {
let mut value: i32 = 1;
println(value);
return 0;
}
",
    );

    let first = run_axc([OsStr::new("fmt"), input.as_os_str()]);
    assert_eq!(first.status.code(), Some(0));
    assert_clean_stderr(&first);

    let formatted = normalize_text(
        &fs::read_to_string(&input).expect("formatted file should be readable after first fmt"),
    );
    assert_eq!(formatted, snapshot("fmt_canonical.ax"));

    let second = run_axc([OsStr::new("fmt"), input.as_os_str()]);
    assert_eq!(second.status.code(), Some(0));
    assert_clean_stderr(&second);

    let second_stdout = string_output(&second.stdout);
    assert!(
        second_stdout.starts_with("already formatted: "),
        "expected idempotent fmt message, got:\n{}",
        second_stdout
    );

    let formatted_again = normalize_text(
        &fs::read_to_string(&input).expect("formatted file should be readable after second fmt"),
    );
    assert_eq!(formatted_again, snapshot("fmt_canonical.ax"));
}
