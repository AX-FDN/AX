use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use super::support::*;

#[test]
#[cfg_attr(
    not(windows),
    ignore = "Windows-only PowerShell benchmark orchestration"
)]
fn repair_benchmark_export_keeps_cold_base_ai_artifact_contracts() {
    let temp = TempDir::new("repair-benchmark-export");
    let output_dir = export_smoke_repair_benchmark(&temp);

    let index_path = output_dir.join("index.json");
    let index = read_json_file(&index_path, "repair benchmark index");
    assert_eq!(
        index["schema_version"],
        Value::from(1),
        "repair benchmark export should keep schema version 1"
    );

    let cases = index["cases"]
        .as_array()
        .expect("repair benchmark export should include cases");
    assert_eq!(
        cases.len(),
        13,
        "smoke export should keep the 13-case repair manifest subset"
    );

    let syntax_case = cases
        .iter()
        .find(|case| case["id"].as_str() == Some("missing_semicolon_basic"))
        .expect("smoke export should include the missing semicolon case");
    assert_eq!(
        syntax_case["diagnostic_command"],
        Value::from("check"),
        "syntax smoke export should default to the `check` diagnostic command"
    );

    let syntax_artifacts = syntax_case["artifacts"]
        .as_object()
        .expect("syntax case should include an artifacts object");
    for key in [
        "source",
        "cold_bundle",
        "cold_prompt",
        "base_diagnostics",
        "ai_diagnostics",
        "base_bundle",
        "ai_bundle",
        "base_prompt",
        "ai_prompt",
    ] {
        let relative_path = syntax_artifacts[key]
            .as_str()
            .unwrap_or_else(|| panic!("syntax artifact `{key}` should be a string path"));
        assert!(
            output_dir.join(relative_path).exists(),
            "syntax artifact `{key}` should exist at `{relative_path}`"
        );
    }

    let syntax_case_summary = read_json_file(
        &output_dir.join("missing_semicolon_basic").join("case.json"),
        "syntax case summary",
    );
    assert_eq!(
        syntax_case_summary["artifacts"]["cold_bundle"],
        Value::from("missing_semicolon_basic/bundle.cold.json"),
        "case summary should expose the cold bundle artifact path"
    );
    assert_eq!(
        syntax_case_summary["artifacts"]["cold_prompt"],
        Value::from("missing_semicolon_basic/prompt.cold.md"),
        "case summary should expose the cold prompt artifact path"
    );

    let project_case = cases
        .iter()
        .find(|case| case["id"].as_str() == Some("project_helper_missing_semicolon"))
        .expect("smoke export should include the project-backed helper case");
    assert_eq!(
        project_case["project_target_relative_path"],
        Value::from("lib/helper.ax")
    );
    assert_eq!(
        project_case["artifacts"]["project_root"],
        Value::from("project_helper_missing_semicolon/project")
    );
    assert!(
        output_dir
            .join("project_helper_missing_semicolon")
            .join("project")
            .join("AX.toml")
            .exists(),
        "project-backed export should include AX.toml in the read-only snapshot"
    );
    assert!(
        output_dir
            .join("project_helper_missing_semicolon")
            .join("project")
            .join("src")
            .join("main.ax")
            .exists(),
        "project-backed export should include the supporting read-only AX sources"
    );

    let cold_bundle = read_json_file(
        &output_dir
            .join("missing_semicolon_basic")
            .join("bundle.cold.json"),
        "cold bundle",
    );
    assert_eq!(cold_bundle["schema_version"], Value::from(1));
    assert_eq!(cold_bundle["feedback_mode"], Value::from("cold_prompt"));
    assert_eq!(cold_bundle["diagnostic_command"], Value::from("check"));
    assert_eq!(
        cold_bundle["source_file"],
        Value::from("missing_semicolon_basic/source.ax")
    );
    assert!(
        cold_bundle["diagnostics"]
            .as_array()
            .expect("cold bundle diagnostics should be an array")
            .is_empty(),
        "cold bundle should keep diagnostics empty so adapters can measure prompt-only repair"
    );

    let base_bundle = read_json_file(
        &output_dir
            .join("missing_semicolon_basic")
            .join("bundle.base.json"),
        "base bundle",
    );
    assert_eq!(base_bundle["feedback_mode"], Value::from("base_json"));
    assert_eq!(base_bundle["diagnostic_command"], Value::from("check"));
    assert_eq!(
        diagnostic_codes(&base_bundle["diagnostics"]),
        vec!["P0001".to_string()],
        "base bundle should preserve the expected base diagnostic code sequence"
    );

    let ai_bundle = read_json_file(
        &output_dir
            .join("missing_semicolon_basic")
            .join("bundle.ai.json"),
        "ai bundle",
    );
    assert_eq!(ai_bundle["feedback_mode"], Value::from("ai_json"));
    assert_eq!(ai_bundle["diagnostic_command"], Value::from("check"));
    assert_eq!(
        diagnostic_codes(&ai_bundle["diagnostics"]),
        vec!["P0001".to_string()],
        "ai bundle should preserve the expected base diagnostic code sequence"
    );
    let ai_rule_ids: Vec<String> = ai_bundle["diagnostics"]
        .as_array()
        .expect("ai bundle diagnostics should be an array")
        .iter()
        .filter_map(|diagnostic| {
            diagnostic
                .get("ai")
                .and_then(|ai| ai.get("rule_id"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .collect();
    assert_eq!(
        ai_rule_ids,
        vec!["statement_terminator_required".to_string()],
        "ai bundle should preserve the expected ai rule ids"
    );

    let cold_prompt = normalize_text(
        &fs::read_to_string(
            output_dir
                .join("missing_semicolon_basic")
                .join("prompt.cold.md"),
        )
        .expect("cold prompt should be readable"),
    );
    assert!(
        cold_prompt.contains("Feedback mode: cold_prompt"),
        "cold prompt should expose the prompt-only feedback mode"
    );
    assert!(
        !cold_prompt.contains("Compiler diagnostics:"),
        "cold prompt should omit compiler diagnostics entirely"
    );

    let base_prompt = normalize_text(
        &fs::read_to_string(
            output_dir
                .join("missing_semicolon_basic")
                .join("prompt.base.md"),
        )
        .expect("base prompt should be readable"),
    );
    assert!(
        base_prompt.contains("Feedback mode: base_json"),
        "base prompt should expose the base-json feedback mode"
    );
    assert!(
        base_prompt.contains("Compiler diagnostics:"),
        "base prompt should embed compiler diagnostics"
    );
    assert!(
        base_prompt.contains("\"code\":") && base_prompt.contains("P0001"),
        "base prompt should include the exported diagnostic code"
    );

    let ai_prompt = normalize_text(
        &fs::read_to_string(
            output_dir
                .join("missing_semicolon_basic")
                .join("prompt.ai.md"),
        )
        .expect("ai prompt should be readable"),
    );
    assert!(
        ai_prompt.contains("Feedback mode: ai_json"),
        "ai prompt should expose the ai-json feedback mode"
    );
    assert!(
        ai_prompt.contains("statement_terminator_required"),
        "ai prompt should include the AI rule-bearing diagnostic payload"
    );

    let runtime_case = cases
        .iter()
        .find(|case| case["id"].as_str() == Some("index_out_of_bounds_runtime"))
        .expect("smoke export should include the runtime bounds case");
    assert_eq!(
        runtime_case["diagnostic_command"],
        Value::from("run"),
        "runtime smoke export should preserve `run` as the diagnostic command"
    );

    let runtime_cold_bundle = read_json_file(
        &output_dir
            .join("index_out_of_bounds_runtime")
            .join("bundle.cold.json"),
        "runtime cold bundle",
    );
    assert_eq!(
        runtime_cold_bundle["feedback_mode"],
        Value::from("cold_prompt")
    );
    assert_eq!(
        runtime_cold_bundle["diagnostic_command"],
        Value::from("run")
    );
    assert!(
        runtime_cold_bundle["diagnostics"]
            .as_array()
            .expect("runtime cold bundle diagnostics should be an array")
            .is_empty(),
        "runtime cold bundle should stay prompt-only"
    );

    let runtime_base_bundle = read_json_file(
        &output_dir
            .join("index_out_of_bounds_runtime")
            .join("bundle.base.json"),
        "runtime base bundle",
    );
    assert_eq!(
        diagnostic_codes(&runtime_base_bundle["diagnostics"]),
        vec!["R0031".to_string()],
        "runtime base bundle should preserve runtime diagnostic codes"
    );

    let runtime_cold_prompt = normalize_text(
        &fs::read_to_string(
            output_dir
                .join("index_out_of_bounds_runtime")
                .join("prompt.cold.md"),
        )
        .expect("runtime cold prompt should be readable"),
    );
    assert!(
        runtime_cold_prompt.contains("Diagnostic command: axc run --json"),
        "runtime cold prompt should name the `run --json` origin explicitly"
    );
    assert!(
        runtime_cold_prompt.contains(
            "The program already gets far enough to execute, so repair the runtime failure without introducing new check-time diagnostics."
        ),
        "runtime cold prompt should preserve the runtime-specific repair guidance"
    );
}

#[test]
#[cfg_attr(
    not(windows),
    ignore = "Windows-only PowerShell benchmark orchestration"
)]
fn repair_benchmark_export_supports_project_context_cases() {
    let temp = TempDir::new("repair-benchmark-project-export");
    let project_dir = temp.join("project");
    fs::create_dir_all(project_dir.join("lib")).expect("project lib directory should exist");
    fs::create_dir_all(project_dir.join("src")).expect("project src directory should exist");

    fs::write(
        project_dir.join("AX.toml"),
        "\
manifest_version = 1

[package]
name = \"repair_project_case\"
entry = \"src/main.ax\"
sources = [\"lib\"]
",
    )
    .expect("project manifest should exist");
    fs::write(
        project_dir.join("lib").join("helper.ax"),
        "\
fn helper() -> i32 {
    let value: i32 = 1
    return value;
}
",
    )
    .expect("broken helper should exist");
    fs::write(
        project_dir.join("src").join("main.ax"),
        "\
fn main() -> i32 {
    return helper();
}
",
    )
    .expect("entry source should exist");

    let manifest_path = write_single_project_case_manifest(
        &temp,
        "single-project-case-manifest.json",
        &project_dir,
        &project_dir.join("lib").join("helper.ax"),
    );
    let output_dir = export_repair_benchmark(&temp, &manifest_path);

    let index = read_json_file(
        &output_dir.join("index.json"),
        "project repair benchmark index",
    );
    let cases = index["cases"]
        .as_array()
        .expect("project repair benchmark export should include cases");
    assert_eq!(cases.len(), 1);

    let case_summary = &cases[0];
    assert_eq!(
        case_summary["project_target_relative_path"],
        Value::from("lib/helper.ax")
    );
    assert_eq!(
        json_string_array(
            &case_summary["project_source_relative_paths"],
            "project source relative paths",
        ),
        vec!["lib/helper.ax".to_string(), "src/main.ax".to_string()]
    );
    assert_eq!(
        case_summary["artifacts"]["project_root"],
        Value::from("project_missing_semicolon/project")
    );
    assert!(
        output_dir
            .join("project_missing_semicolon")
            .join("project")
            .join("AX.toml")
            .exists(),
        "project export should include AX.toml"
    );
    assert!(
        output_dir
            .join("project_missing_semicolon")
            .join("project")
            .join("lib")
            .join("helper.ax")
            .exists(),
        "project export should include the target helper source"
    );
    assert!(
        output_dir
            .join("project_missing_semicolon")
            .join("project")
            .join("src")
            .join("main.ax")
            .exists(),
        "project export should include the read-only project context files"
    );

    let cold_bundle = read_json_file(
        &output_dir
            .join("project_missing_semicolon")
            .join("bundle.cold.json"),
        "project cold bundle",
    );
    assert_eq!(
        cold_bundle["project_root"],
        Value::from("project_missing_semicolon/project")
    );
    assert_eq!(
        cold_bundle["project_manifest_relative_path"],
        Value::from("AX.toml")
    );
    assert_eq!(
        cold_bundle["project_target_relative_path"],
        Value::from("lib/helper.ax")
    );
    assert_eq!(
        json_string_array(
            &cold_bundle["project_source_relative_paths"],
            "project bundle source relative paths",
        ),
        vec!["lib/helper.ax".to_string(), "src/main.ax".to_string()]
    );

    let prompt = normalize_text(
        &fs::read_to_string(
            output_dir
                .join("project_missing_semicolon")
                .join("prompt.base.md"),
        )
        .expect("project prompt should be readable"),
    );
    assert!(
        prompt.contains("Broken AX source (target file: lib/helper.ax):"),
        "project prompt should identify the target file explicitly"
    );
    assert!(
        prompt.contains("Project manifest:"),
        "project prompt should include the manifest as read-only context"
    );
    assert!(
        prompt.contains("Read-only project file: src/main.ax"),
        "project prompt should include supporting project source context"
    );
}

#[test]
#[cfg_attr(
    not(windows),
    ignore = "Windows-only PowerShell benchmark orchestration"
)]
fn repair_benchmark_export_can_include_context_bundle() {
    let temp = TempDir::new("repair-benchmark-context-export");
    let project_dir = temp.join("project");
    fs::create_dir_all(project_dir.join("lib")).expect("project lib directory should exist");
    fs::create_dir_all(project_dir.join("src")).expect("project src directory should exist");

    fs::write(
        project_dir.join("AX.toml"),
        "\
manifest_version = 1

[package]
name = \"repair_project_case\"
entry = \"src/main.ax\"
sources = [\"lib\"]
",
    )
    .expect("project manifest should exist");
    fs::write(
        project_dir.join("lib").join("helper.ax"),
        "\
fn helper() -> i32 {
    let value: i32 = 1
    return value;
}
",
    )
    .expect("broken helper should exist");
    fs::write(
        project_dir.join("src").join("main.ax"),
        "\
fn main() -> i32 {
    return helper();
}
",
    )
    .expect("entry source should exist");

    let manifest_path = write_single_project_case_manifest(
        &temp,
        "single-project-context-case-manifest.json",
        &project_dir,
        &project_dir.join("lib").join("helper.ax"),
    );
    let output_dir = export_repair_benchmark_with_context(&temp, &manifest_path);

    let bundle = read_json_file(
        &output_dir
            .join("project_missing_semicolon")
            .join("bundle.ai.json"),
        "context-enabled ai bundle",
    );
    assert_eq!(
        bundle["context_bundle"]["schema_version"],
        Value::from(1),
        "context bundle should keep its own schema version"
    );
    assert_eq!(
        bundle["context_bundle"]["symbol"],
        Value::from("helper"),
        "context bundle should use the manifest-provided context symbol"
    );
    assert_eq!(
        bundle["context_bundle"]["views"]["overview"]["view"],
        Value::from("overview"),
        "context bundle should include the overview view"
    );
    assert_eq!(
        bundle["context_bundle"]["views"]["boundaries"]["view"],
        Value::from("boundaries"),
        "context bundle should include the boundaries view"
    );
    assert_eq!(
        bundle["context_bundle"]["views"]["evidence"]["view"],
        Value::from("evidence"),
        "context bundle should include the evidence view"
    );

    let index = read_json_file(
        &output_dir.join("index.json"),
        "context-enabled repair benchmark index",
    );
    let case_summary = &index["cases"][0];
    assert_eq!(
        case_summary["context_symbol"],
        Value::from("helper"),
        "index should expose the context symbol used for export"
    );
    assert_eq!(
        json_string_array(&case_summary["context_views"], "context views"),
        vec![
            "overview".to_string(),
            "boundaries".to_string(),
            "evidence".to_string()
        ],
        "index should expose the context views included in the bundle"
    );

    let prompt = normalize_text(
        &fs::read_to_string(
            output_dir
                .join("project_missing_semicolon")
                .join("prompt.ai.md"),
        )
        .expect("context prompt should be readable"),
    );
    assert!(
        prompt.contains("AX context bundle:"),
        "context-enabled prompt should include a dedicated context section"
    );
    assert!(
        prompt.contains("\"evidence\""),
        "context-enabled prompt should include evidence JSON"
    );
}

#[test]
#[cfg_attr(
    not(windows),
    ignore = "Windows-only PowerShell benchmark orchestration"
)]
fn repair_benchmark_run_accepts_large_stdout_only_adapter_output_without_timeout() {
    let temp = TempDir::new("repair-benchmark-stdout-only");
    let manifest_path = write_single_case_manifest(&temp, "single-case-manifest.json");
    let benchmark_dir = export_repair_benchmark(&temp, &manifest_path);
    let runner_script = write_stdout_only_runner(&temp, "stdout-only-runner.ps1");
    let output_dir = temp.join("run");
    let script_path = repo_root().join("scripts").join("run-repair-benchmark.ps1");

    let output = run_powershell_script(
        &script_path,
        [
            OsStr::new("-BenchmarkDir"),
            benchmark_dir.as_os_str(),
            OsStr::new("-RunnerScript"),
            runner_script.as_os_str(),
            OsStr::new("-FeedbackMode"),
            OsStr::new("ai"),
            OsStr::new("-OutputDir"),
            output_dir.as_os_str(),
            OsStr::new("-SkipBuild"),
            OsStr::new("-SkipScore"),
            OsStr::new("-TimeoutSeconds"),
            OsStr::new("5"),
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout-only runner contract should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let run_summary = read_json_file(
        &output_dir.join("run-summary.json"),
        "stdout-only runner summary",
    );
    assert_eq!(run_summary["schema_version"], Value::from(1));
    assert_eq!(run_summary["feedback_mode"], Value::from("ai"));
    assert_eq!(
        assert_json_path_exists(
            &run_summary["benchmark_index"],
            "stdout-only benchmark_index"
        ),
        benchmark_dir.join("index.json")
    );
    assert_eq!(
        assert_json_path_exists(&run_summary["benchmark_root"], "stdout-only benchmark_root"),
        benchmark_dir
    );
    assert_eq!(
        PathBuf::from(
            run_summary["runner_script"]
                .as_str()
                .expect("stdout-only runner summary should include runner_script"),
        ),
        runner_script
    );
    assert_eq!(
        json_string_array(
            &run_summary["runner_extra_args"],
            "stdout-only runner summary runner_extra_args",
        ),
        Vec::<String>::new()
    );
    assert_eq!(
        assert_json_path_exists(&run_summary["candidates_dir"], "stdout-only candidates_dir"),
        output_dir.join("candidates")
    );
    assert_eq!(
        assert_json_path_exists(&run_summary["output_dir"], "stdout-only output_dir"),
        output_dir
    );
    assert_eq!(run_summary["totals"]["total"], Value::from(1));
    assert_eq!(run_summary["totals"]["ok"], Value::from(1));
    assert_eq!(run_summary["totals"]["failed"], Value::from(0));
    assert_eq!(run_summary["totals"]["timed_out"], Value::from(0));
    assert_eq!(run_summary["score"]["skipped"], Value::from(true));
    assert!(run_summary["score"]["summary_path"].is_null());
    assert!(run_summary["score"]["exit_code"].is_null());

    let cases = run_summary["cases"]
        .as_array()
        .expect("stdout-only runner summary should include cases");
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0]["id"], Value::from("missing_semicolon_basic"));
    assert_eq!(cases[0]["feedback_mode"], Value::from("ai"));
    assert_eq!(cases[0]["status"], Value::from("ok"));
    assert_eq!(cases[0]["timed_out"], Value::from(false));
    assert_eq!(cases[0]["exit_code"], Value::from(0));
    assert_json_path_exists(&cases[0]["prompt_path"], "stdout-only prompt_path");
    assert_json_path_exists(&cases[0]["bundle_path"], "stdout-only bundle_path");
    assert_json_path_exists(&cases[0]["stdout_log"], "stdout-only stdout_log");
    assert_json_path_exists(&cases[0]["stderr_log"], "stdout-only stderr_log");

    let candidate_path = PathBuf::from(
        cases[0]["output_path"]
            .as_str()
            .expect("stdout-only runner should report output path"),
    );
    let candidate_text = fs::read_to_string(&candidate_path)
        .expect("stdout-only runner should materialize captured stdout as candidate source");
    assert!(
        candidate_text.starts_with("fn main() -> i32 { let payload: string = \"AAAA"),
        "captured stdout candidate should preserve the emitted AX source"
    );
    assert!(
        candidate_text.len() > 100_000,
        "captured stdout candidate should keep the large emitted payload"
    );
}

#[test]
#[cfg_attr(
    not(windows),
    ignore = "Windows-only PowerShell benchmark orchestration"
)]
fn repair_benchmark_run_rejects_zero_exit_without_file_or_stdout() {
    let temp = TempDir::new("repair-benchmark-silent-runner");
    let manifest_path = write_single_case_manifest(&temp, "single-case-manifest.json");
    let benchmark_dir = export_repair_benchmark(&temp, &manifest_path);
    let runner_script = write_silent_success_runner(&temp, "silent-success-runner.ps1");
    let output_dir = temp.join("run");
    let script_path = repo_root().join("scripts").join("run-repair-benchmark.ps1");

    let output = run_powershell_script(
        &script_path,
        [
            OsStr::new("-BenchmarkDir"),
            benchmark_dir.as_os_str(),
            OsStr::new("-RunnerScript"),
            runner_script.as_os_str(),
            OsStr::new("-FeedbackMode"),
            OsStr::new("ai"),
            OsStr::new("-OutputDir"),
            output_dir.as_os_str(),
            OsStr::new("-SkipBuild"),
            OsStr::new("-SkipScore"),
            OsStr::new("-TimeoutSeconds"),
            OsStr::new("5"),
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "silent success runner should be treated as failed\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let run_summary = read_json_file(
        &output_dir.join("run-summary.json"),
        "silent runner summary",
    );
    assert_eq!(run_summary["schema_version"], Value::from(1));
    assert_eq!(
        assert_json_path_exists(&run_summary["benchmark_index"], "silent benchmark_index"),
        benchmark_dir.join("index.json")
    );
    assert_eq!(
        assert_json_path_exists(&run_summary["benchmark_root"], "silent benchmark_root"),
        benchmark_dir
    );
    assert_eq!(
        PathBuf::from(
            run_summary["runner_script"]
                .as_str()
                .expect("silent runner summary should include runner_script"),
        ),
        runner_script
    );
    assert_eq!(
        json_string_array(
            &run_summary["runner_extra_args"],
            "silent runner summary runner_extra_args",
        ),
        Vec::<String>::new()
    );
    assert_eq!(
        assert_json_path_exists(&run_summary["candidates_dir"], "silent candidates_dir"),
        output_dir.join("candidates")
    );
    assert_eq!(
        assert_json_path_exists(&run_summary["output_dir"], "silent output_dir"),
        output_dir
    );
    assert_eq!(run_summary["totals"]["total"], Value::from(1));
    assert_eq!(run_summary["totals"]["ok"], Value::from(0));
    assert_eq!(run_summary["totals"]["failed"], Value::from(1));
    assert_eq!(run_summary["totals"]["timed_out"], Value::from(0));
    assert_eq!(run_summary["score"]["skipped"], Value::from(true));

    let cases = run_summary["cases"]
        .as_array()
        .expect("silent runner summary should include cases");
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0]["id"], Value::from("missing_semicolon_basic"));
    assert_eq!(cases[0]["feedback_mode"], Value::from("ai"));
    assert_eq!(cases[0]["status"], Value::from("failed"));
    assert_eq!(cases[0]["timed_out"], Value::from(false));
    assert_eq!(cases[0]["exit_code"], Value::from(0));
    assert_json_path_exists(&cases[0]["prompt_path"], "silent prompt_path");
    assert_json_path_exists(&cases[0]["bundle_path"], "silent bundle_path");
    assert_json_path_exists(&cases[0]["stdout_log"], "silent stdout_log");
    assert_json_path_exists(&cases[0]["stderr_log"], "silent stderr_log");

    let candidate_path = PathBuf::from(
        cases[0]["output_path"]
            .as_str()
            .expect("silent runner should still report the preferred output path"),
    );
    assert!(
        !candidate_path.exists(),
        "silent runner should not produce a candidate file when it emits nothing"
    );
}

#[test]
#[cfg_attr(
    not(windows),
    ignore = "Windows-only PowerShell benchmark orchestration"
)]
fn repair_benchmark_score_accepts_clean_runtime_nonzero_exit_without_diagnostics() {
    let temp = TempDir::new("repair-benchmark-runtime-exit-contract");
    let manifest_path = write_single_runtime_case_manifest(&temp, "single-runtime-manifest.json");
    let benchmark_dir = export_repair_benchmark(&temp, &manifest_path);
    let candidates_dir = temp.join("candidates");
    fs::create_dir_all(&candidates_dir).expect("failed to create runtime candidates directory");
    fs::write(
        candidates_dir.join("index_out_of_bounds_runtime.ax"),
        "\
fn main() -> i32 {
    let values: [i32; 2] = [1, 2];
    return values[1];
}
",
    )
    .expect("failed to write runtime candidate");

    let output_dir = temp.join("score");
    let script_path = repo_root()
        .join("scripts")
        .join("score-repair-benchmark.ps1");

    let output = run_powershell_script(
        &script_path,
        [
            OsStr::new("-BenchmarkDir"),
            benchmark_dir.as_os_str(),
            OsStr::new("-CandidatesDir"),
            candidates_dir.as_os_str(),
            OsStr::new("-OutputDir"),
            output_dir.as_os_str(),
            OsStr::new("-SkipBuild"),
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "clean runtime candidate should pass even when main returns a non-zero value\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let summary = read_json_file(
        &output_dir.join("summary.json"),
        "runtime exit score summary",
    );
    assert_eq!(summary["totals"]["total"], Value::from(1));
    assert_eq!(summary["totals"]["passed"], Value::from(1));
    assert_eq!(summary["totals"]["failed"], Value::from(0));
    assert_eq!(summary["totals"]["missing"], Value::from(0));

    let cases = summary["cases"]
        .as_array()
        .expect("runtime exit score summary should include cases");
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0]["id"], Value::from("index_out_of_bounds_runtime"));
    assert_eq!(cases[0]["diagnostic_command"], Value::from("run"));
    assert_eq!(cases[0]["status"], Value::from("passed"));
    assert_eq!(cases[0]["success"], Value::from(true));
    assert_eq!(cases[0]["check_exit_code"], Value::from(0));
    assert_eq!(
        json_string_array(&cases[0]["remaining_codes"], "runtime exit remaining_codes"),
        Vec::<String>::new()
    );
    assert_eq!(cases[0]["run"]["command"], Value::from("run --json"));
    assert_eq!(cases[0]["run"]["command_exit_code"], Value::from(2));
    assert_eq!(cases[0]["run"]["parsed_diagnostics"], Value::from(false));
    assert_eq!(
        json_string_array(
            &cases[0]["run"]["remaining_codes"],
            "runtime exit run.remaining_codes",
        ),
        Vec::<String>::new()
    );
}

#[test]
#[cfg_attr(
    not(windows),
    ignore = "Windows-only PowerShell benchmark orchestration"
)]
fn repair_benchmark_score_normalizes_utf8_bom_candidates() {
    let temp = TempDir::new("repair-benchmark-bom-candidate");
    let manifest_path = write_single_case_manifest(&temp, "single-case-manifest.json");
    let benchmark_dir = export_repair_benchmark(&temp, &manifest_path);
    let candidates_dir = temp.join("candidates");
    fs::create_dir_all(&candidates_dir).expect("failed to create BOM candidates directory");

    let candidate_path = candidates_dir.join("missing_semicolon_basic.ax");
    write_utf8_bom_candidate(
        &candidate_path,
        "\
fn main() -> i32 {
    let value: i32 = 1;
    return value;
}
",
    );

    let output_dir = temp.join("score");
    let script_path = repo_root()
        .join("scripts")
        .join("score-repair-benchmark.ps1");
    let output = run_powershell_script(
        &script_path,
        [
            OsStr::new("-BenchmarkDir"),
            benchmark_dir.as_os_str(),
            OsStr::new("-CandidatesDir"),
            candidates_dir.as_os_str(),
            OsStr::new("-OutputDir"),
            output_dir.as_os_str(),
            OsStr::new("-SkipBuild"),
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "UTF-8 BOM candidate should be normalized before scoring\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let summary = read_json_file(
        &output_dir.join("summary.json"),
        "BOM candidate score summary",
    );
    assert_eq!(summary["totals"]["total"], Value::from(1));
    assert_eq!(summary["totals"]["passed"], Value::from(1));
    assert_eq!(summary["totals"]["failed"], Value::from(0));
    assert_eq!(summary["totals"]["missing"], Value::from(0));

    let cases = summary["cases"]
        .as_array()
        .expect("BOM candidate score summary should include cases");
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0]["id"], Value::from("missing_semicolon_basic"));
    assert_eq!(cases[0]["status"], Value::from("passed"));
    assert_eq!(cases[0]["success"], Value::from(true));
    assert_eq!(
        PathBuf::from(
            cases[0]["candidate_path"]
                .as_str()
                .expect("BOM candidate summary should preserve the original path"),
        ),
        candidate_path
    );
    assert_eq!(cases[0]["check_exit_code"], Value::from(0));
    assert_eq!(
        json_string_array(
            &cases[0]["remaining_codes"],
            "BOM candidate remaining_codes"
        ),
        Vec::<String>::new()
    );

    let normalized_copy = fs::read(
        output_dir
            .join("missing_semicolon_basic")
            .join("candidate.ax"),
    )
    .expect("scorer should emit a normalized candidate copy");
    assert!(
        !normalized_copy.starts_with(&[0xEF, 0xBB, 0xBF]),
        "normalized candidate copy should strip the UTF-8 BOM"
    );
}

#[test]
#[cfg_attr(
    not(windows),
    ignore = "Windows-only PowerShell benchmark orchestration"
)]
fn repair_benchmark_score_normalizes_utf16le_bom_candidates() {
    let temp = TempDir::new("repair-benchmark-utf16le-bom-candidate");
    let manifest_path = write_single_case_manifest(&temp, "single-case-manifest.json");
    let benchmark_dir = export_repair_benchmark(&temp, &manifest_path);
    let candidates_dir = temp.join("candidates");
    fs::create_dir_all(&candidates_dir).expect("failed to create UTF-16 BOM candidates directory");

    let candidate_path = candidates_dir.join("missing_semicolon_basic.ax");
    write_utf16le_bom_candidate(
        &candidate_path,
        "\
fn main() -> i32 {
    let value: i32 = 1;
    return value;
}
",
    );

    let output_dir = temp.join("score");
    let script_path = repo_root()
        .join("scripts")
        .join("score-repair-benchmark.ps1");
    let output = run_powershell_script(
        &script_path,
        [
            OsStr::new("-BenchmarkDir"),
            benchmark_dir.as_os_str(),
            OsStr::new("-CandidatesDir"),
            candidates_dir.as_os_str(),
            OsStr::new("-OutputDir"),
            output_dir.as_os_str(),
            OsStr::new("-SkipBuild"),
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "UTF-16 LE BOM candidate should be normalized before scoring\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let summary = read_json_file(
        &output_dir.join("summary.json"),
        "UTF-16 BOM candidate score summary",
    );
    assert_eq!(summary["totals"]["total"], Value::from(1));
    assert_eq!(summary["totals"]["passed"], Value::from(1));
    assert_eq!(summary["totals"]["failed"], Value::from(0));
    assert_eq!(summary["totals"]["missing"], Value::from(0));

    let cases = summary["cases"]
        .as_array()
        .expect("UTF-16 BOM candidate score summary should include cases");
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0]["id"], Value::from("missing_semicolon_basic"));
    assert_eq!(cases[0]["status"], Value::from("passed"));
    assert_eq!(cases[0]["success"], Value::from(true));
    assert_eq!(
        PathBuf::from(
            cases[0]["candidate_path"]
                .as_str()
                .expect("UTF-16 BOM candidate summary should preserve the original path"),
        ),
        candidate_path
    );
    assert_eq!(cases[0]["check_exit_code"], Value::from(0));
    assert_eq!(
        json_string_array(
            &cases[0]["remaining_codes"],
            "UTF-16 BOM candidate remaining_codes",
        ),
        Vec::<String>::new()
    );

    let normalized_copy = fs::read(
        output_dir
            .join("missing_semicolon_basic")
            .join("candidate.ax"),
    )
    .expect("scorer should emit a normalized UTF-16 candidate copy");
    assert!(
        !normalized_copy.starts_with(&[0xFF, 0xFE]),
        "normalized candidate copy should strip the UTF-16 LE BOM"
    );
    assert!(
        !normalized_copy.starts_with(&[0xEF, 0xBB, 0xBF]),
        "normalized candidate copy should stay BOM-free after UTF-16 normalization"
    );
}

#[test]
#[cfg_attr(
    not(windows),
    ignore = "Windows-only PowerShell benchmark orchestration"
)]
fn repair_benchmark_score_supports_project_context_cases() {
    let temp = TempDir::new("repair-benchmark-project-score");
    let project_dir = temp.join("project");
    fs::create_dir_all(project_dir.join("lib")).expect("project lib directory should exist");
    fs::create_dir_all(project_dir.join("src")).expect("project src directory should exist");

    fs::write(
        project_dir.join("AX.toml"),
        "\
manifest_version = 1

[package]
name = \"repair_project_case\"
entry = \"src/main.ax\"
sources = [\"lib\"]
",
    )
    .expect("project manifest should exist");
    fs::write(
        project_dir.join("lib").join("helper.ax"),
        "\
fn helper() -> i32 {
    let value: i32 = 1
    return value;
}
",
    )
    .expect("broken helper should exist");
    fs::write(
        project_dir.join("src").join("main.ax"),
        "\
fn main() -> i32 {
    return helper();
}
",
    )
    .expect("entry source should exist");

    let manifest_path = write_single_project_case_manifest(
        &temp,
        "single-project-case-manifest.json",
        &project_dir,
        &project_dir.join("lib").join("helper.ax"),
    );
    let benchmark_dir = export_repair_benchmark(&temp, &manifest_path);

    let candidates_dir = temp.join("candidates");
    fs::create_dir_all(&candidates_dir).expect("project candidates directory should exist");
    fs::write(
        candidates_dir.join("project_missing_semicolon.ax"),
        "\
fn helper() -> i32 {
    let value: i32 = 1;
    return value;
}
",
    )
    .expect("project candidate should exist");

    let output_dir = temp.join("score");
    let script_path = repo_root()
        .join("scripts")
        .join("score-repair-benchmark.ps1");
    let output = run_powershell_script(
        &script_path,
        [
            OsStr::new("-BenchmarkDir"),
            benchmark_dir.as_os_str(),
            OsStr::new("-CandidatesDir"),
            candidates_dir.as_os_str(),
            OsStr::new("-OutputDir"),
            output_dir.as_os_str(),
            OsStr::new("-SkipBuild"),
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "project-context score should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let summary = read_json_file(&output_dir.join("summary.json"), "project score summary");
    let cases = summary["cases"]
        .as_array()
        .expect("project score summary should include cases");
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0]["status"], Value::from("passed"));
    assert_eq!(cases[0]["check_exit_code"], Value::from(0));
    assert_eq!(
        json_string_array(&cases[0]["remaining_codes"], "project remaining codes"),
        Vec::<String>::new()
    );

    let working_helper = normalize_text(
        &fs::read_to_string(
            output_dir
                .join("project_missing_semicolon")
                .join("project")
                .join("lib")
                .join("helper.ax"),
        )
        .expect("score should reconstruct the project helper file"),
    );
    assert_eq!(
        working_helper,
        "\
fn helper() -> i32 {
    let value: i32 = 1;
    return value;
}
"
    );
    assert!(
        output_dir
            .join("project_missing_semicolon")
            .join("project")
            .join("src")
            .join("main.ax")
            .exists(),
        "score should keep the read-only project context files in the working tree"
    );
}

#[test]
#[cfg_attr(
    not(windows),
    ignore = "Windows-only PowerShell benchmark orchestration"
)]
fn repair_benchmark_run_keeps_smoke_run_and_score_contracts_without_rebuild() {
    let temp = TempDir::new("repair-benchmark-run");
    let benchmark_dir = export_smoke_repair_benchmark(&temp);
    let runner_script = write_replay_wrapper(
        &temp,
        "replay-smoke.ps1",
        &repo_root()
            .join("benchmarks")
            .join("repair-candidates")
            .join("smoke"),
        None,
        None,
        None,
    );
    let output_dir = temp.join("run");
    let script_path = repo_root().join("scripts").join("run-repair-benchmark.ps1");

    let output = run_powershell_script(
        &script_path,
        [
            OsStr::new("-BenchmarkDir"),
            benchmark_dir.as_os_str(),
            OsStr::new("-RunnerScript"),
            runner_script.as_os_str(),
            OsStr::new("-FeedbackMode"),
            OsStr::new("ai"),
            OsStr::new("-OutputDir"),
            output_dir.as_os_str(),
            OsStr::new("-SkipBuild"),
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "repair benchmark run should succeed without rebuilding\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let run_summary_path = output_dir.join("run-summary.json");
    let score_summary_path = output_dir.join("score").join("summary.json");
    let run_summary = read_json_file(&run_summary_path, "repair benchmark run summary");
    let score_summary = read_json_file(&score_summary_path, "repair benchmark score summary");

    assert_eq!(run_summary["schema_version"], Value::from(1));
    assert_eq!(run_summary["feedback_mode"], Value::from("ai"));
    assert_eq!(
        assert_json_path_exists(
            &run_summary["benchmark_index"],
            "run summary benchmark_index"
        ),
        benchmark_dir.join("index.json")
    );
    assert_eq!(
        assert_json_path_exists(&run_summary["benchmark_root"], "run summary benchmark_root"),
        benchmark_dir.clone()
    );
    assert_eq!(
        PathBuf::from(
            run_summary["runner_script"]
                .as_str()
                .expect("run summary should include runner_script"),
        ),
        runner_script
    );
    assert_eq!(
        json_string_array(
            &run_summary["runner_extra_args"],
            "run summary runner_extra_args"
        ),
        Vec::<String>::new()
    );
    assert_eq!(
        assert_json_path_exists(&run_summary["candidates_dir"], "run summary candidates_dir"),
        output_dir.join("candidates")
    );
    assert_eq!(
        assert_json_path_exists(&run_summary["output_dir"], "run summary output_dir"),
        output_dir
    );
    assert_eq!(run_summary["totals"]["total"], Value::from(13));
    assert_eq!(run_summary["totals"]["ok"], Value::from(13));
    assert_eq!(run_summary["totals"]["failed"], Value::from(0));
    assert_eq!(run_summary["totals"]["timed_out"], Value::from(0));
    assert_eq!(run_summary["score"]["skipped"], Value::from(false));
    assert_eq!(run_summary["score"]["exit_code"], Value::from(0));

    let reported_score_summary_path = PathBuf::from(
        run_summary["score"]["summary_path"]
            .as_str()
            .expect("run summary should include score summary path"),
    );
    assert!(
        reported_score_summary_path.exists(),
        "run summary should point at an existing score summary"
    );
    assert_eq!(reported_score_summary_path, score_summary_path);

    let run_cases = run_summary["cases"]
        .as_array()
        .expect("run summary should include cases");
    assert_eq!(run_cases.len(), 13);
    for case in run_cases {
        assert_eq!(case["feedback_mode"], Value::from("ai"));
        assert_eq!(case["status"], Value::from("ok"));
        assert_eq!(case["timed_out"], Value::from(false));
        assert_eq!(case["exit_code"], Value::from(0));
        assert_json_path_exists(&case["prompt_path"], "run case prompt_path");
        assert_json_path_exists(&case["bundle_path"], "run case bundle_path");
        assert_json_path_exists(&case["output_path"], "run case output_path");
        assert_json_path_exists(&case["stdout_log"], "run case stdout_log");
        assert_json_path_exists(&case["stderr_log"], "run case stderr_log");
    }

    assert_eq!(score_summary["schema_version"], Value::from(1));
    assert_eq!(
        assert_json_path_exists(
            &score_summary["benchmark_dir"],
            "score summary benchmark_dir"
        ),
        benchmark_dir.clone()
    );
    assert_eq!(
        assert_json_path_exists(
            &score_summary["benchmark_index"],
            "score summary benchmark_index",
        ),
        benchmark_dir.join("index.json")
    );
    assert_eq!(
        assert_json_path_exists(
            &score_summary["candidates_dir"],
            "score summary candidates_dir",
        ),
        output_dir.join("candidates")
    );
    assert_eq!(
        assert_json_path_exists(&score_summary["output_dir"], "score summary output_dir"),
        output_dir.join("score")
    );
    assert_eq!(score_summary["totals"]["total"], Value::from(13));
    assert_eq!(score_summary["totals"]["passed"], Value::from(13));
    assert_eq!(score_summary["totals"]["failed"], Value::from(0));
    assert_eq!(score_summary["totals"]["missing"], Value::from(0));

    let score_cases = score_summary["cases"]
        .as_array()
        .expect("score summary should include cases");
    assert_eq!(score_cases.len(), 13);
    for case in score_cases {
        assert_eq!(case["status"], Value::from("passed"));
        assert_eq!(case["success"], Value::from(true));
        assert_eq!(case["check_exit_code"], Value::from(0));
        assert_json_path_exists(&case["candidate_path"], "score case candidate_path");
    }

    let runtime_cases: Vec<&Value> = score_cases
        .iter()
        .filter(|case| case["diagnostic_command"].as_str() == Some("run"))
        .collect();
    let check_cases: Vec<&Value> = score_cases
        .iter()
        .filter(|case| case["diagnostic_command"].as_str() == Some("check"))
        .collect();
    assert_eq!(runtime_cases.len(), 2);
    assert_eq!(check_cases.len(), 11);
    assert_eq!(
        runtime_cases
            .iter()
            .map(|case| case["id"].as_str().expect("runtime case should have id"))
            .collect::<Vec<_>>(),
        vec!["index_out_of_bounds_runtime", "division_by_zero_runtime"]
    );

    for runtime_case in runtime_cases {
        let case_id = runtime_case["id"]
            .as_str()
            .expect("runtime score case should have id");
        assert_eq!(
            runtime_case["status"],
            Value::from("passed"),
            "runtime smoke case `{case_id}` should pass"
        );
        assert_eq!(
            runtime_case["run"]["command"],
            Value::from("run --json"),
            "runtime smoke case `{case_id}` should record run --json validation"
        );
        assert_eq!(
            runtime_case["run"]["command_exit_code"],
            Value::from(0),
            "runtime smoke case `{case_id}` should keep a zero runtime exit code"
        );
        assert_eq!(
            runtime_case["run"]["parsed_diagnostics"],
            Value::from(false),
            "runtime smoke case `{case_id}` should record the no-diagnostics runtime path"
        );
        assert!(
            runtime_case["run"]["remaining_codes"]
                .as_array()
                .expect("runtime smoke case should expose remaining runtime codes")
                .is_empty(),
            "runtime smoke case `{case_id}` should clear remaining runtime diagnostics"
        );
    }
    for check_case in check_cases {
        assert!(
            check_case.get("run").is_none(),
            "check-only smoke case should not include runtime validation details"
        );
    }

    let project_case = score_cases
        .iter()
        .find(|case| case["id"].as_str() == Some("project_helper_missing_semicolon"))
        .expect("score summary should include the project-backed helper case");
    assert_eq!(project_case["status"], Value::from("passed"));
    assert_eq!(
        project_case["benchmark_case"]["project"],
        Value::from("benchmarks/repair-projects/helper_missing_semicolon")
    );
    assert_eq!(
        project_case["benchmark_case"]["project_target_relative_path"],
        Value::from("lib/helper.ax")
    );
}

#[test]
#[cfg_attr(
    not(windows),
    ignore = "Windows-only PowerShell benchmark orchestration"
)]
fn repair_feedback_comparison_keeps_smoke_contract_without_rebuild() {
    let temp = TempDir::new("repair-feedback-comparison");
    let benchmark_dir = export_smoke_repair_benchmark(&temp);
    let runner_script = write_replay_wrapper(
        &temp,
        "replay-compare-feedback.ps1",
        &repo_root()
            .join("benchmarks")
            .join("repair-candidates")
            .join("smoke"),
        None,
        Some(
            &repo_root()
                .join("benchmarks")
                .join("repair-candidates")
                .join("compare")
                .join("base"),
        ),
        None,
    );
    let output_dir = temp.join("comparison");
    let script_path = repo_root()
        .join("scripts")
        .join("compare-repair-feedback.ps1");

    let output = run_powershell_script(
        &script_path,
        [
            OsStr::new("-BenchmarkDir"),
            benchmark_dir.as_os_str(),
            OsStr::new("-RunnerScript"),
            runner_script.as_os_str(),
            OsStr::new("-OutputDir"),
            output_dir.as_os_str(),
            OsStr::new("-SkipBuild"),
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "repair feedback comparison should succeed without rebuilding\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let comparison = read_json_file(
        &output_dir.join("comparison.json"),
        "repair feedback comparison summary",
    );
    assert_eq!(comparison["schema_version"], Value::from(1));
    assert_eq!(
        assert_json_path_exists(
            &comparison["benchmark_index"],
            "feedback comparison benchmark_index",
        ),
        benchmark_dir.join("index.json")
    );
    assert_eq!(
        PathBuf::from(
            comparison["runner_script"]
                .as_str()
                .expect("feedback comparison should include runner_script"),
        ),
        runner_script
    );
    assert_eq!(
        json_string_array(
            &comparison["runner_extra_args"],
            "feedback comparison runner_extra_args",
        ),
        Vec::<String>::new()
    );
    assert_eq!(
        assert_json_path_exists(&comparison["output_dir"], "feedback comparison output_dir"),
        output_dir
    );
    assert_eq!(comparison["comparison"]["total_cases"], Value::from(13));
    assert_eq!(comparison["comparison"]["base_passed"], Value::from(7));
    assert_eq!(comparison["comparison"]["ai_passed"], Value::from(13));
    assert_eq!(
        comparison["comparison"]["absolute_lift_cases"],
        Value::from(6)
    );
    assert_json_f64(
        &comparison["comparison"]["absolute_lift_pp"],
        46.15,
        "comparison absolute_lift_pp",
    );
    assert_json_f64(
        &comparison["comparison"]["relative_lift_pct"],
        85.71,
        "comparison relative_lift_pct",
    );
    assert_eq!(
        comparison["modes"]["base"]["invocation_totals"]["ok"],
        Value::from(13)
    );
    assert_eq!(
        comparison["modes"]["ai"]["invocation_totals"]["ok"],
        Value::from(13)
    );
    assert_eq!(
        comparison["modes"]["base"]["score_totals"]["failed"],
        Value::from(6)
    );
    assert_eq!(
        comparison["modes"]["ai"]["score_totals"]["failed"],
        Value::from(0)
    );
    assert_eq!(comparison["modes"]["base"]["exit_code"], Value::from(1));
    assert_eq!(comparison["modes"]["ai"]["exit_code"], Value::from(0));
    assert_eq!(comparison["modes"]["base"]["timed_out"], Value::from(false));
    assert_eq!(comparison["modes"]["ai"]["timed_out"], Value::from(false));
    assert_json_path_exists(
        &comparison["modes"]["base"]["stdout_log"],
        "base stdout_log",
    );
    assert_json_path_exists(
        &comparison["modes"]["base"]["stderr_log"],
        "base stderr_log",
    );
    assert_json_path_exists(
        &comparison["modes"]["base"]["run_summary_path"],
        "base run_summary_path",
    );
    assert_json_path_exists(
        &comparison["modes"]["base"]["score_summary_path"],
        "base score_summary_path",
    );
    assert_json_path_exists(&comparison["modes"]["ai"]["stdout_log"], "ai stdout_log");
    assert_json_path_exists(&comparison["modes"]["ai"]["stderr_log"], "ai stderr_log");
    assert_json_path_exists(
        &comparison["modes"]["ai"]["run_summary_path"],
        "ai run_summary_path",
    );
    assert_json_path_exists(
        &comparison["modes"]["ai"]["score_summary_path"],
        "ai score_summary_path",
    );
    assert_eq!(
        json_string_array(
            &comparison["comparison"]["improved_cases"],
            "comparison improved_cases",
        ),
        vec![
            "type_mismatch_bool_from_int".to_string(),
            "missing_struct_literal_field".to_string(),
            "match_struct_pattern_missing_field".to_string(),
            "slice_assignment_requires_mutable_binding".to_string(),
            "index_out_of_bounds_runtime".to_string(),
            "division_by_zero_runtime".to_string(),
        ]
    );
    assert_eq!(
        json_string_array(
            &comparison["comparison"]["regressed_cases"],
            "comparison regressed_cases",
        ),
        Vec::<String>::new()
    );
    assert_eq!(
        comparison["comparison"]["unchanged_cases"]
            .as_array()
            .expect("comparison unchanged_cases should be an array")
            .len(),
        7
    );

    let case_deltas = comparison["cases"]
        .as_array()
        .expect("comparison should include per-case deltas");
    assert_eq!(case_deltas.len(), 13);
    let runtime_case = case_deltas
        .iter()
        .find(|case| case["id"].as_str() == Some("index_out_of_bounds_runtime"))
        .expect("comparison should include runtime case delta");
    assert_eq!(runtime_case["base_status"], Value::from("failed"));
    assert_eq!(runtime_case["ai_status"], Value::from("passed"));
    assert_eq!(runtime_case["base_success"], Value::from(false));
    assert_eq!(runtime_case["ai_success"], Value::from(true));
    assert_eq!(runtime_case["delta"], Value::from("improved"));
    assert_eq!(
        json_string_array(
            &runtime_case["base_remaining_codes"],
            "runtime_case base_remaining_codes",
        ),
        Vec::<String>::new()
    );
    assert_eq!(
        json_string_array(
            &runtime_case["ai_remaining_codes"],
            "runtime_case ai_remaining_codes",
        ),
        Vec::<String>::new()
    );

    let categories = comparison["categories"]
        .as_array()
        .expect("comparison should include category summaries");
    let semantic = categories
        .iter()
        .find(|category| category["category"].as_str() == Some("semantic"))
        .expect("comparison should include the semantic category");
    assert_eq!(semantic["total"], Value::from(7));
    assert_eq!(semantic["base_passed"], Value::from(3));
    assert_eq!(semantic["ai_passed"], Value::from(7));
    assert_eq!(semantic["improved"], Value::from(4));
    assert_eq!(semantic["regressed"], Value::from(0));
    assert_eq!(
        json_string_array(&semantic["improved_case_ids"], "semantic improved_case_ids",),
        vec![
            "type_mismatch_bool_from_int".to_string(),
            "missing_struct_literal_field".to_string(),
            "match_struct_pattern_missing_field".to_string(),
            "slice_assignment_requires_mutable_binding".to_string(),
        ]
    );
    assert_eq!(
        json_string_array(
            &semantic["regressed_case_ids"],
            "semantic regressed_case_ids",
        ),
        Vec::<String>::new()
    );

    let runtime = categories
        .iter()
        .find(|category| category["category"].as_str() == Some("runtime"))
        .expect("comparison should include the runtime category");
    assert_eq!(runtime["total"], Value::from(2));
    assert_eq!(runtime["base_passed"], Value::from(0));
    assert_eq!(runtime["ai_passed"], Value::from(2));
    assert_eq!(runtime["improved"], Value::from(2));
    assert_eq!(runtime["regressed"], Value::from(0));
    assert_eq!(
        json_string_array(&runtime["improved_case_ids"], "runtime improved_case_ids",),
        vec![
            "index_out_of_bounds_runtime".to_string(),
            "division_by_zero_runtime".to_string(),
        ]
    );
    assert_eq!(
        json_string_array(&runtime["regressed_case_ids"], "runtime regressed_case_ids",),
        Vec::<String>::new()
    );
}

#[test]
#[cfg_attr(
    not(windows),
    ignore = "Windows-only PowerShell benchmark orchestration"
)]
fn repair_mode_comparison_keeps_smoke_contract_without_rebuild() {
    let temp = TempDir::new("repair-mode-comparison");
    let benchmark_dir = export_smoke_repair_benchmark(&temp);
    let runner_script = write_replay_wrapper(
        &temp,
        "replay-compare-modes.ps1",
        &repo_root()
            .join("benchmarks")
            .join("repair-candidates")
            .join("smoke"),
        Some(
            &repo_root()
                .join("benchmarks")
                .join("repair-candidates")
                .join("compare")
                .join("cold"),
        ),
        Some(
            &repo_root()
                .join("benchmarks")
                .join("repair-candidates")
                .join("compare")
                .join("base"),
        ),
        None,
    );
    let output_dir = temp.join("comparison");
    let script_path = repo_root().join("scripts").join("compare-repair-modes.ps1");

    let output = run_powershell_script(
        &script_path,
        [
            OsStr::new("-BenchmarkDir"),
            benchmark_dir.as_os_str(),
            OsStr::new("-RunnerScript"),
            runner_script.as_os_str(),
            OsStr::new("-OutputDir"),
            output_dir.as_os_str(),
            OsStr::new("-SkipBuild"),
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "repair mode comparison should succeed without rebuilding\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let comparison = read_json_file(
        &output_dir.join("comparison.json"),
        "repair mode comparison summary",
    );
    assert_eq!(comparison["schema_version"], Value::from(1));
    assert_eq!(
        assert_json_path_exists(
            &comparison["benchmark_index"],
            "mode comparison benchmark_index",
        ),
        benchmark_dir.join("index.json")
    );
    assert_eq!(
        PathBuf::from(
            comparison["runner_script"]
                .as_str()
                .expect("mode comparison should include runner_script"),
        ),
        runner_script
    );
    assert_eq!(
        json_string_array(
            &comparison["runner_extra_args"],
            "mode comparison runner_extra_args",
        ),
        Vec::<String>::new()
    );
    assert_eq!(
        json_string_array(&comparison["mode_order"], "mode comparison mode_order"),
        vec!["cold".to_string(), "base".to_string(), "ai".to_string()]
    );
    assert_eq!(
        assert_json_path_exists(&comparison["output_dir"], "mode comparison output_dir"),
        output_dir
    );
    assert_eq!(comparison["summary"]["total_cases"], Value::from(13));
    assert_eq!(comparison["summary"]["cold_passed"], Value::from(5));
    assert_eq!(comparison["summary"]["base_passed"], Value::from(7));
    assert_eq!(comparison["summary"]["ai_passed"], Value::from(13));
    assert_eq!(comparison["modes"]["cold"]["exit_code"], Value::from(1));
    assert_eq!(comparison["modes"]["base"]["exit_code"], Value::from(1));
    assert_eq!(comparison["modes"]["ai"]["exit_code"], Value::from(0));
    assert_eq!(
        comparison["modes"]["cold"]["score_totals"]["failed"],
        Value::from(8)
    );
    assert_eq!(
        comparison["modes"]["base"]["score_totals"]["failed"],
        Value::from(6)
    );
    assert_eq!(
        comparison["modes"]["ai"]["score_totals"]["failed"],
        Value::from(0)
    );
    assert_json_path_exists(
        &comparison["modes"]["cold"]["stdout_log"],
        "cold stdout_log",
    );
    assert_json_path_exists(
        &comparison["modes"]["cold"]["stderr_log"],
        "cold stderr_log",
    );
    assert_json_path_exists(
        &comparison["modes"]["cold"]["run_summary_path"],
        "cold run_summary_path",
    );
    assert_json_path_exists(
        &comparison["modes"]["cold"]["score_summary_path"],
        "cold score_summary_path",
    );
    assert_json_path_exists(
        &comparison["modes"]["base"]["stdout_log"],
        "base stdout_log",
    );
    assert_json_path_exists(
        &comparison["modes"]["base"]["stderr_log"],
        "base stderr_log",
    );
    assert_json_path_exists(
        &comparison["modes"]["base"]["run_summary_path"],
        "base run_summary_path",
    );
    assert_json_path_exists(
        &comparison["modes"]["base"]["score_summary_path"],
        "base score_summary_path",
    );
    assert_json_path_exists(&comparison["modes"]["ai"]["stdout_log"], "ai stdout_log");
    assert_json_path_exists(&comparison["modes"]["ai"]["stderr_log"], "ai stderr_log");
    assert_json_path_exists(
        &comparison["modes"]["ai"]["run_summary_path"],
        "ai run_summary_path",
    );
    assert_json_path_exists(
        &comparison["modes"]["ai"]["score_summary_path"],
        "ai score_summary_path",
    );
    assert_eq!(
        comparison["summary"]["pairwise_comparisons"]["cold_to_base"]["absolute_lift_cases"],
        Value::from(2)
    );
    assert_eq!(
        comparison["summary"]["pairwise_comparisons"]["base_to_ai"]["absolute_lift_cases"],
        Value::from(6)
    );
    assert_eq!(
        comparison["summary"]["pairwise_comparisons"]["cold_to_ai"]["absolute_lift_cases"],
        Value::from(8)
    );
    assert_eq!(
        comparison["summary"]["pairwise_comparisons"]["cold_to_base"]["absolute_lift_pp"]
            .as_f64()
            .expect("cold_to_base.absolute_lift_pp should be numeric"),
        15.39
    );
    assert_eq!(
        comparison["summary"]["pairwise_comparisons"]["base_to_ai"]["absolute_lift_pp"]
            .as_f64()
            .expect("base_to_ai.absolute_lift_pp should be numeric"),
        46.15
    );
    assert_eq!(
        comparison["summary"]["pairwise_comparisons"]["cold_to_ai"]["absolute_lift_pp"]
            .as_f64()
            .expect("cold_to_ai.absolute_lift_pp should be numeric"),
        61.54
    );
    assert_json_f64(
        &comparison["summary"]["pairwise_comparisons"]["cold_to_base"]["relative_lift_pct"],
        40.0,
        "cold_to_base relative_lift_pct",
    );
    assert_json_f64(
        &comparison["summary"]["pairwise_comparisons"]["base_to_ai"]["relative_lift_pct"],
        85.71,
        "base_to_ai relative_lift_pct",
    );
    assert_json_f64(
        &comparison["summary"]["pairwise_comparisons"]["cold_to_ai"]["relative_lift_pct"],
        160.0,
        "cold_to_ai relative_lift_pct",
    );
    assert_eq!(
        json_string_array(
            &comparison["summary"]["pairwise_comparisons"]["cold_to_base"]["improved_cases"],
            "mode comparison cold_to_base improved_cases",
        ),
        vec![
            "unknown_type_missing".to_string(),
            "len_builtin_non_countable_value".to_string(),
        ]
    );
    assert_eq!(
        json_string_array(
            &comparison["summary"]["pairwise_comparisons"]["base_to_ai"]["improved_cases"],
            "mode comparison base_to_ai improved_cases",
        ),
        vec![
            "type_mismatch_bool_from_int".to_string(),
            "missing_struct_literal_field".to_string(),
            "match_struct_pattern_missing_field".to_string(),
            "slice_assignment_requires_mutable_binding".to_string(),
            "index_out_of_bounds_runtime".to_string(),
            "division_by_zero_runtime".to_string(),
        ]
    );
    assert_eq!(
        json_string_array(
            &comparison["summary"]["pairwise_comparisons"]["cold_to_ai"]["improved_cases"],
            "mode comparison cold_to_ai improved_cases",
        ),
        vec![
            "type_mismatch_bool_from_int".to_string(),
            "unknown_type_missing".to_string(),
            "missing_struct_literal_field".to_string(),
            "match_struct_pattern_missing_field".to_string(),
            "len_builtin_non_countable_value".to_string(),
            "slice_assignment_requires_mutable_binding".to_string(),
            "index_out_of_bounds_runtime".to_string(),
            "division_by_zero_runtime".to_string(),
        ]
    );
    assert_eq!(
        json_string_array(
            &comparison["summary"]["pairwise_comparisons"]["cold_to_ai"]["regressed_cases"],
            "mode comparison cold_to_ai regressed_cases",
        ),
        Vec::<String>::new()
    );
    assert_eq!(
        comparison["summary"]["pairwise_comparisons"]["cold_to_base"]["unchanged_cases"]
            .as_array()
            .expect("cold_to_base unchanged_cases should be an array")
            .len(),
        11
    );
    assert_eq!(
        comparison["summary"]["pairwise_comparisons"]["base_to_ai"]["unchanged_cases"]
            .as_array()
            .expect("base_to_ai unchanged_cases should be an array")
            .len(),
        7
    );
    assert_eq!(
        comparison["summary"]["pairwise_comparisons"]["cold_to_ai"]["unchanged_cases"]
            .as_array()
            .expect("cold_to_ai unchanged_cases should be an array")
            .len(),
        5
    );

    let case_deltas = comparison["cases"]
        .as_array()
        .expect("mode comparison should include per-case deltas");
    assert_eq!(case_deltas.len(), 13);
    let cold_to_base_case = case_deltas
        .iter()
        .find(|case| case["id"].as_str() == Some("unknown_type_missing"))
        .expect("mode comparison should include unknown_type_missing");
    assert_eq!(
        cold_to_base_case["cold_to_base_delta"],
        Value::from("improved")
    );
    assert_eq!(
        cold_to_base_case["base_to_ai_delta"],
        Value::from("both_pass")
    );
    assert_eq!(
        cold_to_base_case["cold_to_ai_delta"],
        Value::from("improved")
    );

    let categories = comparison["categories"]
        .as_array()
        .expect("mode comparison should include category summaries");
    let semantic = categories
        .iter()
        .find(|category| category["category"].as_str() == Some("semantic"))
        .expect("mode comparison should include the semantic category");
    assert_eq!(semantic["total"], Value::from(7));
    assert_eq!(semantic["cold_passed"], Value::from(1));
    assert_eq!(semantic["base_passed"], Value::from(3));
    assert_eq!(semantic["ai_passed"], Value::from(7));

    let runtime = categories
        .iter()
        .find(|category| category["category"].as_str() == Some("runtime"))
        .expect("mode comparison should include the runtime category");
    assert_eq!(runtime["total"], Value::from(2));
    assert_eq!(runtime["cold_passed"], Value::from(0));
    assert_eq!(runtime["base_passed"], Value::from(0));
    assert_eq!(runtime["ai_passed"], Value::from(2));
}

#[test]
fn smoke_repair_manifest_stays_aligned_with_full_manifest() {
    let full_manifest = load_repair_manifest("benchmarks/repair-cases.json");
    let smoke_manifest = load_repair_manifest("benchmarks/repair-cases-smoke.json");

    let full_cases_by_id: BTreeMap<&str, &RepairCaseEntry> = full_manifest
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect();

    assert_eq!(
        full_manifest.cases.len(),
        43,
        "full manifest should currently pin the 43-case repair benchmark baseline"
    );

    assert_eq!(
        smoke_manifest.cases.len(),
        13,
        "smoke manifest should currently pin the 13-case CI subset"
    );

    let runtime_case_ids: Vec<&str> = smoke_manifest
        .cases
        .iter()
        .filter(|case| case.diagnostic_command() == "run")
        .map(|case| case.id.as_str())
        .collect();
    assert_eq!(
        runtime_case_ids,
        vec!["index_out_of_bounds_runtime", "division_by_zero_runtime"],
        "smoke manifest should keep the two runtime replay cases in a stable order"
    );

    let check_case_count = smoke_manifest
        .cases
        .iter()
        .filter(|case| case.diagnostic_command() == "check")
        .count();
    assert_eq!(
        check_case_count, 11,
        "smoke manifest should keep the diagnostics benchmark subset at 11 check-based cases"
    );

    for smoke_case in &smoke_manifest.cases {
        let full_case = *full_cases_by_id
            .get(smoke_case.id.as_str())
            .unwrap_or_else(|| {
                panic!(
                    "smoke case `{}` should also exist in the full repair manifest",
                    smoke_case.id
                )
            });

        assert_eq!(
            smoke_case.file.as_str(),
            full_case.file.as_str(),
            "smoke case `{}` should point at the same source file as the full manifest",
            smoke_case.id
        );
        assert_eq!(
            smoke_case.project.as_deref(),
            full_case.project.as_deref(),
            "smoke case `{}` should keep the same project context as the full manifest",
            smoke_case.id
        );
        assert_eq!(
            smoke_case.diagnostic_command(),
            full_case.diagnostic_command(),
            "smoke case `{}` should keep the same diagnostic command as the full manifest",
            smoke_case.id
        );
        assert_eq!(
            smoke_case.expected_codes.as_slice(),
            full_case.expected_codes.as_slice(),
            "smoke case `{}` should keep the same expected diagnostic codes as the full manifest",
            smoke_case.id
        );
        assert_eq!(
            smoke_case.expected_ai_rule_ids.as_slice(),
            full_case.expected_ai_rule_ids.as_slice(),
            "smoke case `{}` should keep the same expected ai rule ids as the full manifest",
            smoke_case.id
        );
    }
}

#[test]
#[cfg_attr(
    not(windows),
    ignore = "Windows-only PowerShell benchmark orchestration"
)]
fn full_compare_shared_replay_covers_full_manifest() {
    let full_manifest = load_repair_manifest("benchmarks/repair-cases.json");
    let shared_root = repo_root()
        .join("benchmarks")
        .join("repair-candidates")
        .join("compare")
        .join("shared");

    assert!(
        shared_root.exists(),
        "full compare shared replay root should exist at `{}`",
        shared_root.display()
    );

    let missing_case_ids: Vec<String> = full_manifest
        .cases
        .iter()
        .filter(|case| find_replay_candidate(&shared_root, &case.id).is_none())
        .map(|case| case.id.clone())
        .collect();

    assert!(
        missing_case_ids.is_empty(),
        "full compare shared replay root should cover every full manifest case, missing: {}",
        missing_case_ids.join(", ")
    );
}

#[test]
#[cfg_attr(
    not(windows),
    ignore = "Windows-only PowerShell benchmark orchestration"
)]
fn full_compare_shared_replay_scores_cleanly() {
    let temp = TempDir::new("full-compare-shared-score");
    let benchmark_dir = export_repair_benchmark(
        &temp,
        &repo_root().join("benchmarks").join("repair-cases.json"),
    );
    let candidates_dir = repo_root()
        .join("benchmarks")
        .join("repair-candidates")
        .join("compare")
        .join("shared");
    let output_dir = temp.join("score");
    let script_path = repo_root()
        .join("scripts")
        .join("score-repair-benchmark.ps1");

    let output = run_powershell_script(
        &script_path,
        [
            OsStr::new("-BenchmarkDir"),
            benchmark_dir.as_os_str(),
            OsStr::new("-CandidatesDir"),
            candidates_dir.as_os_str(),
            OsStr::new("-OutputDir"),
            output_dir.as_os_str(),
            OsStr::new("-SkipBuild"),
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "full compare shared replay candidates should score cleanly\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let summary = read_json_file(
        &output_dir.join("summary.json"),
        "full compare shared score summary",
    );
    assert_eq!(summary["totals"]["total"], Value::from(43));
    assert_eq!(summary["totals"]["passed"], Value::from(43));
    assert_eq!(summary["totals"]["failed"], Value::from(0));
    assert_eq!(summary["totals"]["missing"], Value::from(0));
}
