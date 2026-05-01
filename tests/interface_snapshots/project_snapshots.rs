use std::ffi::OsStr;
use std::fs;

use serde_json::Value;

use super::support::*;

#[test]
fn project_directory_run_executes_manifest_entry() {
    let temp = TempDir::new("project-run");
    temp.write(
        "AX.toml",
        "\
manifest_version = 1

[package]
name = \"project_hello\"
entry = \"src/main.ax\"
",
    );
    temp.write_nested(
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

    let output = run_axc([OsStr::new("run"), temp.path.as_os_str()]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "3\n");
}

#[test]
fn project_directory_run_executes_manifest_entry_with_support_sources() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/project_split")]);
    assert_eq!(output.status.code(), Some(7));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "total=7\n");
}

#[test]
fn project_module_smoke_run_executes_manifest_entry_with_module_support_sources() {
    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/project_module_smoke"),
    ]);
    assert_eq!(output.status.code(), Some(7));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "");
}

#[test]
fn representative_project_examples_check_cleanly() {
    for example_path in [
        "examples/project_directory_index",
        "examples/project_text_normalize",
        "examples/project_release_promote",
        "examples/project_command_capture",
        "examples/project_command_batch",
        "examples/project_option_result",
        "examples/project_env_result",
        "examples/project_file_result",
        "examples/project_process_result",
        "examples/project_result_pipeline",
        "examples/project_config_validate",
        "examples/project_collections_report",
        "examples/project_package_config",
        "examples/project_job_runner",
    ] {
        assert_project_example_checks(example_path);
    }
}

#[test]
fn project_check_reports_support_source_file_in_json_diagnostics() {
    let temp = TempDir::new("project-support-diagnostic");
    temp.write(
        "AX.toml",
        "\
manifest_version = 1

[package]
name = \"project_support_diagnostic\"
entry = \"src/main.ax\"
sources = [\"src/lib.ax\"]
",
    );
    temp.write_nested(
        "src/lib.ax",
        "\
fn helper() -> i32 {
    let value: i32 = 1
    return value;
}
",
    );
    temp.write_nested(
        "src/main.ax",
        "\
fn main() -> i32 {
    return helper();
}
",
    );

    let output = run_axc([
        OsStr::new("check"),
        temp.path.as_os_str(),
        OsStr::new("--json"),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_clean_stderr(&output);

    let diagnostics: Value =
        serde_json::from_slice(&output.stdout).expect("project diagnostics should be JSON");
    let diagnostics = diagnostics
        .as_array()
        .expect("project diagnostics should be an array");
    assert!(!diagnostics.is_empty(), "expected project diagnostics");
    let diagnostic_file = diagnostics[0]["file"]
        .as_str()
        .map(|value| value.replace('\\', "/"));
    let expected_file = temp.join("src/lib.ax").to_string_lossy().replace('\\', "/");
    assert_eq!(
        diagnostic_file.as_deref(),
        Some(expected_file.as_str()),
        "diagnostics should point at the support source file"
    );
    assert_eq!(
        diagnostics[0]["code"].as_str(),
        Some("P0001"),
        "support source parse failure should preserve the parse diagnostic code"
    );
}

#[test]
fn check_accepts_minimal_module_mode_project() {
    let output = run_axc([
        OsStr::new("check"),
        OsStr::new("examples/project_module_smoke"),
        OsStr::new("--json"),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "[]\n");
}

#[test]
fn fmt_formats_multifile_project_inputs() {
    let temp = TempDir::new("fmt-project");
    temp.write(
        "AX.toml",
        "\
manifest_version = 1

[package]
name = \"fmt_project\"
entry = \"src/main.ax\"
sources = [\"src/math.ax\", \"src/report.ax\"]
",
    );
    let math = temp.write_nested(
        "src/math.ax",
        "\
fn add(left:i32,right:i32)->i32 {
return left + right;
}
",
    );
    let report = temp.write_nested(
        "src/report.ax",
        "\
fn render_total(total:i32)->string {
return \"total=\" + to_string(total);
}
",
    );
    let main = temp.write_nested(
        "src/main.ax",
        "\
fn main() -> i32 {
let total:i32=add(2,5);
println(render_total(total));
return total;
}
",
    );

    let first = run_axc([OsStr::new("fmt"), temp.path.as_os_str()]);
    assert_eq!(first.status.code(), Some(0));
    assert_clean_stderr(&first);
    assert_eq!(
        normalize_temp_output(&string_output(&first.stdout), &temp),
        "\
formatted: <root>/src/math.ax
formatted: <root>/src/report.ax
formatted: <root>/src/main.ax
"
    );

    assert_eq!(
        normalize_text(&fs::read_to_string(&math).expect("math source should be formatted")),
        "\
fn add(left: i32, right: i32) -> i32 {
    return left + right;
}
"
    );
    assert_eq!(
        normalize_text(&fs::read_to_string(&report).expect("report source should be formatted")),
        "\
fn render_total(total: i32) -> string {
    return \"total=\" + to_string(total);
}
"
    );
    assert_eq!(
        normalize_text(&fs::read_to_string(&main).expect("main source should be formatted")),
        "\
fn main() -> i32 {
    let total: i32 = add(2, 5);
    println(render_total(total));
    return total;
}
"
    );

    let second = run_axc([OsStr::new("fmt"), temp.path.as_os_str()]);
    assert_eq!(second.status.code(), Some(0));
    assert_clean_stderr(&second);
    assert_eq!(
        normalize_temp_output(&string_output(&second.stdout), &temp),
        "\
already formatted: <root>/src/math.ax
already formatted: <root>/src/report.ax
already formatted: <root>/src/main.ax
"
    );
}

#[test]
fn project_foundation_report_runs_with_ax_side_library_helpers() {
    let temp = TempDir::new("project-foundation-report");
    let input_text = "\
# Release
TODO polish

## Follow Up
FIXME rename
";
    let input_path = temp.write("notes.md", input_text);

    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/project_foundation_report"),
        OsStr::new("--"),
        input_path.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert_clean_stderr(&output);

    let expected = format!(
        "\
file=notes.md
kind=markdown
chars={}
lines={}
nonempty={}
headings={}
action_items={}
",
        input_text.chars().count(),
        line_count(input_text),
        nonempty_line_count(input_text),
        heading_count(input_text),
        action_item_count(input_text),
    );
    assert_eq!(normalize_text(&string_output(&output.stdout)), expected);
}

#[test]
fn project_command_capture_runs_on_controlled_fixture() {
    let temp = TempDir::new("project-command-capture");
    let workspace_dir = temp.join("workspace");
    fs::create_dir_all(&workspace_dir).expect("workspace directory should exist");

    let output_path = temp.join("command-report.txt");
    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/project_command_capture"),
        OsStr::new("--"),
        workspace_dir.as_os_str(),
        OsStr::new("echo repair-ready"),
        output_path.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);

    let stdout = normalize_temp_output(&string_output(&output.stdout), &temp);
    assert_eq!(stdout, "captured=<root>/command-report.txt\n");

    let report = normalize_temp_output(
        &fs::read_to_string(&output_path).expect("command capture report should exist"),
        &temp,
    );
    assert!(
        report.contains("working_dir=<root>/workspace\n"),
        "expected working directory in report, got:\n{}",
        report
    );
    assert!(
        report.contains("command=echo repair-ready\n"),
        "expected command in report, got:\n{}",
        report
    );
    assert!(
        report.contains("lines=1\n"),
        "expected single-line command output stats, got:\n{}",
        report
    );
    assert!(
        report.contains("nonempty=1\n"),
        "expected nonempty output stats, got:\n{}",
        report
    );
    assert!(
        report.contains("path_env_present="),
        "expected environment presence stat, got:\n{}",
        report
    );
    assert!(
        report.contains("\n\noutput:\nrepair-ready\n"),
        "expected captured command output section, got:\n{}",
        report
    );
}

#[test]
fn project_release_promote_runs_on_controlled_fixture() {
    let temp = TempDir::new("project-release-promote");
    let incoming_dir = temp.join("incoming");
    let release_dir = temp.join("release");
    fs::create_dir_all(&incoming_dir).expect("incoming directory should exist");
    fs::create_dir_all(&release_dir).expect("release directory should exist");

    let input_text = "release build v2\n";
    let input_path = incoming_dir.join("release-notes.txt");
    fs::write(&input_path, input_text).expect("input file should exist");
    fs::write(release_dir.join("release-notes.txt"), "old build\n")
        .expect("existing promoted file should exist");

    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/project_release_promote"),
        OsStr::new("--"),
        input_path.as_os_str(),
        release_dir.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);

    let stdout = normalize_temp_output(&string_output(&output.stdout), &temp);
    assert_eq!(stdout, "promoted=<root>/release/release-notes.txt\n");

    assert!(
        !input_path.exists(),
        "source file should be moved into the release directory"
    );

    let promoted_path = release_dir.join("release-notes.txt");
    assert!(
        promoted_path.exists(),
        "promoted file should exist in the release directory"
    );
    assert_eq!(
        fs::read_to_string(&promoted_path).expect("promoted file should be readable"),
        input_text
    );

    let receipt = normalize_temp_output(
        &fs::read_to_string(release_dir.join("release-notes.receipt.txt"))
            .expect("promotion receipt should exist"),
        &temp,
    );
    let expected_receipt = format!(
        "\
source=<root>/incoming/release-notes.txt
promoted=<root>/release/release-notes.txt
release_dir=<root>/release
name=release-notes.txt
stem=release-notes
extension=txt
replaced_existing=true
source_exists_after=false
release_is_dir=true
promoted_is_file=true
size={}
",
        input_text.len()
    );
    assert_eq!(receipt, expected_receipt);
}

#[test]
fn project_directory_index_runs_on_controlled_fixture() {
    let temp = TempDir::new("project-directory-index");
    let workspace_dir = temp.join("workspace");
    let docs_dir = workspace_dir.join("docs");
    let api_dir = docs_dir.join("api");
    fs::create_dir_all(&docs_dir).expect("docs directory should exist");
    fs::create_dir_all(&api_dir).expect("api directory should exist");

    let app_text = "fn main() {}\n";
    let notes_text = "# Notes\nTODO refine\n";
    let guide_text = "## Guide\n";
    let blob_bytes = b"\x01\x02\x03\x04";

    fs::write(workspace_dir.join("app.ax"), app_text).expect("app.ax should exist");
    fs::write(workspace_dir.join("notes.md"), notes_text).expect("notes.md should exist");
    fs::write(workspace_dir.join("blob.bin"), blob_bytes).expect("blob.bin should exist");
    fs::write(api_dir.join("guide.md"), guide_text).expect("guide.md should exist");

    let output_path = temp.join("index.txt");
    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/project_directory_index"),
        OsStr::new("--"),
        workspace_dir.as_os_str(),
        output_path.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);

    let stdout = normalize_temp_output(&string_output(&output.stdout), &temp);
    assert_eq!(stdout, "indexed=<root>/index.txt\n");

    let report = normalize_temp_output(
        &fs::read_to_string(&output_path).expect("directory index report should exist"),
        &temp,
    );
    let expected = format!(
        "\
root=<root>/workspace
entries=6
directories=2
files=4
text_files=3
bytes={}

entries:
app.ax | file | kind=ax | text=true | bytes={}
blob.bin | file | kind=plain | text=false | bytes={}
docs | dir | children=1
  docs/api | dir | children=1
    api/guide.md | file | kind=markdown | text=true | bytes={}
notes.md | file | kind=markdown | text=true | bytes={}
",
        app_text.len() + blob_bytes.len() + notes_text.len() + guide_text.len(),
        app_text.len(),
        blob_bytes.len(),
        guide_text.len(),
        notes_text.len(),
    );
    assert_eq!(report, expected);
}

#[test]
fn project_command_batch_runs_on_controlled_fixture() {
    let temp = TempDir::new("project-command-batch");
    let workspace_dir = temp.join("workspace");
    let output_dir = temp.join("batch-out");
    fs::create_dir_all(&workspace_dir).expect("workspace directory should exist");

    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/project_command_batch"),
        OsStr::new("--"),
        workspace_dir.as_os_str(),
        output_dir.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);

    let stdout = normalize_temp_output(&string_output(&output.stdout), &temp);
    assert_eq!(
        stdout,
        "global-ready\nbatched=<root>/batch-out/BATCH-REPORT.txt\n"
    );

    let global_marker = output_dir.join("global-ready.txt");
    let local_marker = workspace_dir.join("local-ready.txt");
    assert_eq!(
        normalize_text(&fs::read_to_string(&global_marker).expect("global marker should exist")),
        "global-ready\n"
    );
    assert_eq!(
        normalize_text(&fs::read_to_string(&local_marker).expect("local marker should exist"))
            .trim(),
        "local-ready"
    );

    let path_value = std::env::var("PATH").ok();
    let expected_path_present = path_value.is_some();
    let expected_path_length = path_value
        .as_deref()
        .map(|value| value.chars().count())
        .unwrap_or(0);

    let report = normalize_temp_output(
        &fs::read_to_string(output_dir.join("BATCH-REPORT.txt"))
            .expect("batch report should exist"),
        &temp,
    );
    let expected = format!(
        "\
working_dir=<root>/workspace
output_dir=<root>/batch-out
global_marker=<root>/batch-out/global-ready.txt
local_marker=<root>/workspace/local-ready.txt
global_exit=0
local_exit=0
path_env_present={}
path_length={}
global_marker_exists=true
local_marker_exists=true

markers:
global-ready.txt | global-ready
local-ready.txt | local-ready
",
        expected_path_present, expected_path_length
    );
    assert_eq!(report, expected);
}

#[test]
fn project_option_result_runs() {
    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/project_option_result"),
    ]);
    assert_eq!(output.status.code(), Some(7));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_text(&string_output(&output.stdout)),
        "score=5\nscore=0\ntrue\ntrue\ninvalid:bad\n"
    );
}

#[test]
fn project_payload_event_report_runs_on_controlled_fixture() {
    let temp = TempDir::new("project-payload-event-report");
    let output_dir = temp.join("out");

    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/project_payload_event_report"),
        OsStr::new("--"),
        output_dir.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_temp_output(&string_output(&output.stdout), &temp),
        "payload_event_report=<root>/out/PAYLOAD-EVENT-REPORT.txt\n"
    );

    let report = fs::read_to_string(output_dir.join("PAYLOAD-EVENT-REPORT.txt"))
        .expect("payload event report should be written");
    assert_eq!(
        normalize_text(&report),
        "payload_enum_events=5\nevent_0=syntax\nscore_0=101\nfailure_0=true\nevent_1=semantic\nscore_1=222\nfailure_1=true\nevent_2=runtime\nscore_2=900\nfailure_2=true\nevent_3=note:context-ready\nscore_3=10\nfailure_3=false\nevent_4=clean\nscore_4=0\nfailure_4=false\ntotal_score=1233\nfailure_count=3\n"
    );
}

#[test]
fn project_env_result_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/project_env_result")]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_text(&string_output(&output.stdout)),
        "path_ok=true\nmissing environment variable: AX_THIS_VARIABLE_SHOULD_NOT_EXIST_7A9F3D0C\n"
    );
}

#[test]
fn project_file_result_runs_on_controlled_fixture() {
    let temp = TempDir::new("project-file-result");
    let input_path = temp.write("config.txt", "alpha\nbeta\n");

    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/project_file_result"),
        OsStr::new("--"),
        input_path.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_temp_output(&string_output(&output.stdout), &temp),
        "read_ok=true\nsize=11\nparent_entries=1\nreadable file does not exist: <root>/missing-file-result-input.txt\n"
    );
}

#[test]
fn project_process_result_runs_on_controlled_fixture() {
    let temp = TempDir::new("project-process-result");

    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/project_process_result"),
        OsStr::new("--"),
        temp.path.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_temp_output(&string_output(&output.stdout), &temp),
        "run_status=0\nstatus_success=true\nworking directory does not exist: <root>/missing-process-result-dir\ncommand must not be empty\n"
    );
}

#[test]
fn project_result_pipeline_runs_on_controlled_fixture() {
    let temp = TempDir::new("project-result-pipeline");
    let input_path = temp.write("input.txt", "alpha\nbeta\n");
    let output_dir = temp.join("out");

    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/project_result_pipeline"),
        OsStr::new("--"),
        input_path.as_os_str(),
        output_dir.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_temp_output(&string_output(&output.stdout), &temp),
        "pipeline_report=<root>/out/RESULT-PIPELINE.txt\n"
    );

    let report = fs::read_to_string(output_dir.join("RESULT-PIPELINE.txt"))
        .expect("pipeline report should be written");
    assert_eq!(
        normalize_temp_output(&report, &temp),
        "input=<root>/input.txt\ninput_len=11\nenv_path_ok=true\nprocess_success=true\nprocess_code=0\nmissing_error=readable file does not exist: <root>/missing-result-pipeline.txt\n"
    );
}

#[test]
fn project_config_validate_runs_on_controlled_fixture() {
    let temp = TempDir::new("project-config-validate");
    let config_path = temp.write("app.conf", "host=localhost\nport=8080\n");
    let output_dir = temp.join("out");

    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/project_config_validate"),
        OsStr::new("--"),
        config_path.as_os_str(),
        output_dir.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_temp_output(&string_output(&output.stdout), &temp),
        "config_report=<root>/out/CONFIG-VALIDATION.txt\n"
    );

    let report = fs::read_to_string(output_dir.join("CONFIG-VALIDATION.txt"))
        .expect("config validation report should be written");
    assert_eq!(
        normalize_temp_output(&report, &temp),
        "config=<root>/app.conf\nhost_present=true\nport_present=true\nbytes=25\noptional_tagged=optional config: readable file does not exist: <root>/out/optional.conf\noptional_replaced=optional config missing\n"
    );

    let bad_config_path = temp.write("missing-port.conf", "host=localhost\n");
    let bad_output_dir = temp.join("bad-out");
    let bad_output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/project_config_validate"),
        OsStr::new("--"),
        bad_config_path.as_os_str(),
        bad_output_dir.as_os_str(),
    ]);
    assert_eq!(bad_output.status.code(), Some(1));
    assert_clean_stderr(&bad_output);
    assert_eq!(
        normalize_temp_output(&string_output(&bad_output.stdout), &temp),
        "config_error=missing field: port\n"
    );
}

#[test]
fn project_package_config_runs_on_controlled_fixture() {
    let temp = TempDir::new("project-package-config");
    let config_path = temp.write("service.conf", "host=localhost\nport=8080\n");
    let output_dir = temp.join("out");

    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/project_package_config"),
        OsStr::new("--"),
        config_path.as_os_str(),
        output_dir.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_temp_output(&string_output(&output.stdout), &temp),
        "package_config_report=<root>/out/PACKAGE-CONFIG.txt\n"
    );

    let report = fs::read_to_string(output_dir.join("PACKAGE-CONFIG.txt"))
        .expect("package config report should be written");
    assert_eq!(
        normalize_temp_output(&report, &temp),
        "config=<root>/service.conf\npath_package=true\nbytes=25\n"
    );

    let bad_config_path = temp.write("zero-port.conf", "host=localhost\nport=0\n");
    let bad_output_dir = temp.join("bad-out");
    let bad_output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/project_package_config"),
        OsStr::new("--"),
        bad_config_path.as_os_str(),
        bad_output_dir.as_os_str(),
    ]);
    assert_eq!(bad_output.status.code(), Some(1));
    assert_clean_stderr(&bad_output);
    assert_eq!(
        normalize_temp_output(&string_output(&bad_output.stdout), &temp),
        "package_config_error=invalid field: port must not be zero\n"
    );
}

#[test]
fn project_package_config_context_exposes_local_path_package() {
    let output = run_axc([
        OsStr::new("context"),
        OsStr::new("overview"),
        OsStr::new("examples/project_package_config"),
        OsStr::new("--json"),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);

    let context: Value =
        serde_json::from_slice(&output.stdout).expect("context output should be JSON");
    let packages = context["facts"]["local_path_packages"]
        .as_array()
        .expect("overview should expose local_path_packages");
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0]["alias"], "config_rules");
    assert_eq!(packages[0]["source_count"], 1);
    assert_eq!(
        json_string_array(&packages[0]["modules"], "package modules"),
        vec!["config_rules.validate".to_string()]
    );
    assert_eq!(context["facts"]["local_package_lock"]["status"], "current");
    assert_eq!(
        context["facts"]["local_package_lock"]["dependency_count"],
        Value::from(1)
    );
}

#[test]
fn project_path_package_manifest_errors_are_stable() {
    let temp = TempDir::new("project-package-errors");
    temp.write(
        "AX.toml",
        "\
manifest_version = 1

[package]
name = \"project_package_errors\"
entry = \"src/main.ax\"

[dependencies]
config_rules = { path = \"packages/config_rules\" }
",
    );
    temp.write_nested("src/main.ax", "fn main() -> i32 { return 0; }\n");

    let missing_path = run_axc([OsStr::new("check"), temp.path.as_os_str()]);
    assert_eq!(missing_path.status.code(), Some(1));
    let stderr = string_output(&missing_path.stderr);
    assert!(stderr.contains("PX0002"), "stderr:\n{stderr}");
    assert!(
        stderr.contains("repair_rule: package_dependency_path_must_exist"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains(
            "repair_goal: Point the dependency to an existing local AX package directory."
        ),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("failed to access dependency `config_rules` path"),
        "stderr:\n{stderr}"
    );

    fs::create_dir_all(temp.join("packages").join("config_rules"))
        .expect("dependency directory should exist");
    let missing_manifest = run_axc([OsStr::new("check"), temp.path.as_os_str()]);
    assert_eq!(missing_manifest.status.code(), Some(1));
    let stderr = string_output(&missing_manifest.stderr);
    assert!(stderr.contains("PX0003"), "stderr:\n{stderr}");
    assert!(
        stderr.contains("repair_rule: package_dependency_manifest_must_be_valid"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("failed to read dependency `config_rules` manifest"),
        "stderr:\n{stderr}"
    );
}

#[test]
fn project_path_package_manifest_errors_have_json_ai_diagnostics() {
    let temp = TempDir::new("project-package-json-errors");
    temp.write(
        "AX.toml",
        "\
manifest_version = 1

[package]
name = \"project_package_json_errors\"
entry = \"src/main.ax\"

[dependencies]
config_rules = { path = \"packages/config_rules\" }
",
    );
    temp.write_nested("src/main.ax", "fn main() -> i32 { return 0; }\n");

    let output = run_axc([
        OsStr::new("check"),
        temp.path.as_os_str(),
        OsStr::new("--json"),
        OsStr::new("--ai"),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_clean_stderr(&output);

    let diagnostics: Value =
        serde_json::from_slice(&output.stdout).expect("diagnostics output should be JSON");
    let diagnostics = diagnostics
        .as_array()
        .expect("diagnostics output should be an array");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["code"], "PX0002");
    assert_eq!(
        diagnostics[0]["ai"]["rule_id"],
        "package_dependency_path_must_exist"
    );
    assert_eq!(
        diagnostics[0]["ai"]["repair_goal"],
        "Point the dependency to an existing local AX package directory."
    );
    assert_eq!(
        json_string_array(&diagnostics[0]["expected"], "package expected"),
        vec!["valid local path package graph".to_string()]
    );
}

#[test]
fn project_path_package_module_mismatch_keeps_ai_repair_rule() {
    let temp = TempDir::new("project-package-module-mismatch");
    temp.write(
        "AX.toml",
        "\
manifest_version = 1

[package]
name = \"project_package_module_mismatch\"
entry = \"src/main.ax\"

[dependencies]
config_rules = { path = \"packages/config_rules\" }
",
    );
    temp.write_nested(
        "packages/config_rules/AX.toml",
        "\
manifest_version = 1

[package]
name = \"config_rules\"
sources = [\"src\"]
",
    );
    temp.write_nested(
        "packages/config_rules/src/validate.ax",
        "\
module wrong.validate;

fn value() -> i32 {
    return 1;
}
",
    );
    temp.write_nested(
        "src/main.ax",
        "\
import config_rules.validate;

fn main() -> i32 {
    return config_rules.validate.value();
}
",
    );

    let output = run_axc([
        OsStr::new("check"),
        temp.path.as_os_str(),
        OsStr::new("--json"),
        OsStr::new("--ai"),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_clean_stderr(&output);

    let diagnostics: Value =
        serde_json::from_slice(&output.stdout).expect("diagnostics output should be JSON");
    let diagnostics = diagnostics
        .as_array()
        .expect("diagnostics output should be an array");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic["code"] == "S0039"
                && diagnostic["ai"]["rule_id"] == "module_path_must_match_source_path"),
        "expected S0039 module_path_must_match_source_path diagnostic, got:\n{}",
        serde_json::to_string_pretty(&diagnostics).expect("diagnostics should serialize")
    );
}

#[test]
fn project_collections_report_runs_on_controlled_fixture() {
    let temp = TempDir::new("project-collections-report");
    let output_dir = temp.join("out");

    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/project_collections_report"),
        OsStr::new("--"),
        output_dir.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_temp_output(&string_output(&output.stdout), &temp),
        "collections_report=<root>/out/COLLECTIONS-REPORT.txt\n"
    );

    let report = fs::read_to_string(output_dir.join("COLLECTIONS-REPORT.txt"))
        .expect("collections report should be written");
    assert_eq!(
        normalize_temp_output(&report, &temp),
        "label_count=3\nlabels=api,worker,scheduler\nfirst_label=api\nhas_worker=true\nscheduler_index=2\nmissing_index=-1\n"
    );
}

#[test]
fn project_text_normalize_runs_on_controlled_fixture() {
    let temp = TempDir::new("project-text-normalize");
    let input_text = "\t# Title  \n\n\n  TODO fix  \n\tBody line\t\n";
    let input_path = temp.write("notes.md", input_text);
    let output_dir = temp.join("normalized");

    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/project_text_normalize"),
        OsStr::new("--"),
        input_path.as_os_str(),
        output_dir.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);

    let stdout = normalize_temp_output(&string_output(&output.stdout), &temp);
    assert_eq!(
        stdout,
        "normalized=<root>/normalized/notes.normalized.txt\n"
    );

    let normalized_output = normalize_text(
        &fs::read_to_string(output_dir.join("notes.normalized.txt"))
            .expect("normalized output should exist"),
    );
    let expected_normalized = "# Title\n\nTODO fix\nBody line\n";
    assert_eq!(normalized_output, expected_normalized);

    let report = normalize_temp_output(
        &fs::read_to_string(output_dir.join("NORMALIZE-REPORT.txt"))
            .expect("normalize report should exist"),
        &temp,
    );
    let expected_report = format!(
        "\
input=<root>/notes.md
output=<root>/normalized/notes.normalized.txt
changed=true
before_lines={}
before_nonempty={}
before_headings={}
before_action_items={}
after_lines={}
after_nonempty={}
after_headings={}
after_action_items={}
output_bytes={}

preview:
# Title
TODO fix
Body line",
        line_count(input_text),
        nonempty_line_count(input_text),
        heading_count(input_text),
        action_item_count(input_text),
        line_count(expected_normalized),
        nonempty_line_count(expected_normalized),
        heading_count(expected_normalized),
        action_item_count(expected_normalized),
        expected_normalized.len()
    );
    assert_eq!(report, expected_report);
}

#[test]
fn project_docs_release_runs_on_controlled_fixture() {
    let temp = TempDir::new("project-docs-release");
    let docs_dir = temp.join("docs");
    fs::create_dir_all(docs_dir.join("nested")).expect("nested docs directory should exist");

    let alpha_text = "\
# Alpha
TODO polish
";
    let beta_text = "\
## Beta
Stable
";

    fs::write(docs_dir.join("alpha.md"), alpha_text).expect("alpha.md should exist");
    fs::write(docs_dir.join("beta.md"), beta_text).expect("beta.md should exist");
    fs::write(docs_dir.join("notes.txt"), "ignore me\n").expect("notes.txt should exist");

    let out_dir = temp.join("release");
    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/project_docs_release"),
        OsStr::new("--"),
        docs_dir.as_os_str(),
        out_dir.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);

    let stdout = normalize_temp_output(&string_output(&output.stdout), &temp);
    assert_eq!(stdout, "snapshotted=<root>/release/docs-snapshot\n");

    let snapshot_dir = out_dir.join("docs-snapshot");
    let summary_path = snapshot_dir.join("SUMMARY.txt");
    let summary = normalize_temp_output(
        &fs::read_to_string(&summary_path).expect("summary should exist"),
        &temp,
    );
    let expected_summary = format!(
        "\
source=<root>/docs
snapshot=<root>/release/docs-snapshot
entries_seen=4
copied_files=2
skipped_entries=2
copied_bytes={}
lines={}
headings={}
action_items={}

files:
alpha.md | bytes={} | lines={} | headings={} | action_items={}
beta.md | bytes={} | lines={} | headings={} | action_items={}
",
        alpha_text.len() + beta_text.len(),
        line_count(alpha_text) + line_count(beta_text),
        heading_count(alpha_text) + heading_count(beta_text),
        action_item_count(alpha_text) + action_item_count(beta_text),
        alpha_text.len(),
        line_count(alpha_text),
        heading_count(alpha_text),
        action_item_count(alpha_text),
        beta_text.len(),
        line_count(beta_text),
        heading_count(beta_text),
        action_item_count(beta_text),
    );
    assert_eq!(summary, expected_summary);

    let receipt = normalize_temp_output(
        &fs::read_to_string(snapshot_dir.join("receipts").join("alpha.receipt.txt"))
            .expect("alpha receipt should exist"),
        &temp,
    );
    let expected_receipt = format!(
        "\
source=<root>/docs/alpha.md
destination=<root>/release/docs-snapshot/alpha.md
bytes={}
lines={}
headings={}
action_items={}
",
        alpha_text.len(),
        line_count(alpha_text),
        heading_count(alpha_text),
        action_item_count(alpha_text),
    );
    assert_eq!(receipt, expected_receipt);
}
