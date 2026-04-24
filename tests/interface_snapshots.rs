use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use serde_json::Value;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn axc_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_axc"))
}

fn snapshots_dir() -> PathBuf {
    repo_root().join("tests").join("snapshots")
}

fn normalize_text(text: &str) -> String {
    text.replace("\r\n", "\n")
}

fn snapshot(name: &str) -> String {
    normalize_text(
        &fs::read_to_string(snapshots_dir().join(name))
            .unwrap_or_else(|error| panic!("failed to read snapshot `{name}`: {error}")),
    )
}

fn run_axc<I, S>(args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new(axc_binary())
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("failed to execute axc")
}

fn powershell_executable() -> &'static str {
    if cfg!(windows) {
        "powershell.exe"
    } else {
        "pwsh"
    }
}

fn run_powershell_script<I, S>(script: &Path, args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new(powershell_executable());
    command.arg("-NoProfile");
    if cfg!(windows) {
        command.arg("-ExecutionPolicy").arg("Bypass");
    }
    command
        .arg("-File")
        .arg(script)
        .env("AXC_BINARY", axc_binary())
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("failed to execute PowerShell script")
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "axc-interface-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("failed to create temp directory");
        Self { path }
    }

    fn join(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.join(name);
        fs::write(&path, text).expect("failed to write temp file");
        path
    }

    fn write_nested(&self, name: &str, text: &str) -> PathBuf {
        let path = self.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("failed to create temp subdirectory");
        }
        fs::write(&path, text).expect("failed to write temp file");
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn string_output(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("process output should be valid UTF-8")
}

fn read_json_file(path: &Path, label: &str) -> Value {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {label} `{}`: {error}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("failed to parse {label} `{}` as JSON: {error}", path.display()))
}

fn diagnostic_codes(value: &Value) -> Vec<String> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("expected diagnostics array, got {value:?}"))
        .iter()
        .map(|diagnostic| {
            diagnostic["code"]
                .as_str()
                .unwrap_or_else(|| panic!("diagnostic should include code: {diagnostic:?}"))
                .to_string()
        })
        .collect()
}

fn assert_clean_stderr(output: &Output) {
    let stderr = string_output(&output.stderr);
    assert!(
        stderr.trim().is_empty(),
        "expected empty stderr, got:\n{}",
        stderr
    );
}

#[derive(Deserialize)]
struct RepairCaseManifest {
    cases: Vec<RepairCaseEntry>,
}

#[derive(Deserialize)]
struct RepairCaseEntry {
    id: String,
    file: String,
    diagnostic_command: Option<String>,
    expected_codes: Vec<String>,
    expected_ai_rule_ids: Vec<String>,
}

impl RepairCaseEntry {
    fn diagnostic_command(&self) -> &str {
        self.diagnostic_command.as_deref().unwrap_or("check")
    }
}

fn load_repair_manifest(path: &str) -> RepairCaseManifest {
    let manifest_text = fs::read_to_string(repo_root().join(path))
        .unwrap_or_else(|error| panic!("repair benchmark manifest `{path}` should be readable: {error}"));
    serde_json::from_str(&manifest_text)
        .unwrap_or_else(|error| panic!("repair benchmark manifest `{path}` should be valid: {error}"))
}

fn assert_manifest_cases_keep_stable_diagnostics(manifest_path: &str) {
    let manifest = load_repair_manifest(manifest_path);

    for case in manifest.cases {
        let output = run_axc([
            OsStr::new(case.diagnostic_command()),
            OsStr::new(case.file.as_str()),
            OsStr::new("--json"),
            OsStr::new("--ai"),
        ]);
        assert_eq!(
            output.status.code(),
            Some(1),
            "case `{}` from `{}` should emit diagnostics via `{}`",
            case.id,
            manifest_path,
            case.diagnostic_command()
        );
        assert_clean_stderr(&output);

        let diagnostics: Value =
            serde_json::from_slice(&output.stdout).expect("diagnostics output should be JSON");
        let diagnostics = diagnostics.as_array().unwrap_or_else(|| {
            panic!(
                "case `{}` from `{}` should return a diagnostic array",
                case.id, manifest_path
            )
        });

        let observed_codes: Vec<String> = diagnostics
            .iter()
            .map(|diagnostic| {
                diagnostic["code"]
                    .as_str()
                    .unwrap_or_else(|| panic!("case `{}` diagnostic should include code", case.id))
                    .to_string()
            })
            .collect();
        assert_eq!(
            observed_codes, case.expected_codes,
            "case `{}` from `{}` should keep stable diagnostic codes",
            case.id, manifest_path
        );

        let observed_rule_ids: Vec<String> = diagnostics
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
            observed_rule_ids, case.expected_ai_rule_ids,
            "case `{}` from `{}` should keep stable ai rule ids",
            case.id, manifest_path
        );

        for diagnostic in diagnostics {
            if let Some(ai) = diagnostic.get("ai") {
                assert_eq!(
                    ai["teaching_level"].as_str(),
                    Some("L1"),
                    "case `{}` from `{}` should default to L1 without session reuse",
                    case.id,
                    manifest_path
                );
            }
        }
    }
}

#[test]
fn diagnostics_json_matches_snapshot() {
    let temp = TempDir::new("diagnostics");
    let input = temp.write(
        "missing_semicolon.ax",
        "fn main() -> i32 {\n    let value: i32 = 1\n    return value;\n}\n",
    );

    let output = run_axc([OsStr::new("check"), input.as_os_str(), OsStr::new("--json")]);
    assert_eq!(output.status.code(), Some(1));
    assert_clean_stderr(&output);

    let mut diagnostics: Value =
        serde_json::from_slice(&output.stdout).expect("diagnostics output should be JSON");
    let placeholder = "<input>/missing_semicolon.ax".to_string();
    for diagnostic in diagnostics
        .as_array_mut()
        .expect("diagnostics output should be an array")
    {
        diagnostic["file"] = Value::String(placeholder.clone());
    }

    let rendered = serde_json::to_string_pretty(&diagnostics)
        .expect("diagnostics JSON should serialize")
        + "\n";
    assert_eq!(
        normalize_text(&rendered),
        snapshot("diagnostics_missing_semicolon.json")
    );
}

#[test]
fn diagnostics_success_json_matches_snapshot() {
    let output = run_axc([
        OsStr::new("check"),
        OsStr::new("examples/hello.ax"),
        OsStr::new("--json"),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);

    let stdout = normalize_text(&string_output(&output.stdout));
    assert_eq!(stdout, snapshot("diagnostics_success.json"));
}

#[test]
fn diagnostics_success_json_with_ai_matches_snapshot() {
    let output = run_axc([
        OsStr::new("check"),
        OsStr::new("examples/hello.ax"),
        OsStr::new("--json"),
        OsStr::new("--ai"),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);

    let stdout = normalize_text(&string_output(&output.stdout));
    assert_eq!(stdout, snapshot("diagnostics_success.json"));
}

#[test]
fn diagnostics_json_with_ai_matches_snapshot() {
    let temp = TempDir::new("diagnostics-ai");
    let input = temp.write(
        "missing_semicolon.ax",
        "fn main() -> i32 {\n    let value: i32 = 1\n    return value;\n}\n",
    );

    let output = run_axc([
        OsStr::new("check"),
        input.as_os_str(),
        OsStr::new("--json"),
        OsStr::new("--ai"),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_clean_stderr(&output);

    let mut diagnostics: Value =
        serde_json::from_slice(&output.stdout).expect("diagnostics output should be JSON");
    let placeholder = "<input>/missing_semicolon.ax".to_string();
    for diagnostic in diagnostics
        .as_array_mut()
        .expect("diagnostics output should be an array")
    {
        diagnostic["file"] = Value::String(placeholder.clone());
    }

    let rendered = serde_json::to_string_pretty(&diagnostics)
        .expect("diagnostics JSON should serialize")
        + "\n";
    assert_eq!(
        normalize_text(&rendered),
        snapshot("diagnostics_missing_semicolon_ai.json")
    );
}

#[test]
fn diagnostics_ai_session_escalation_matches_snapshots() {
    let temp = TempDir::new("diagnostics-ai-session");
    let input = temp.write(
        "missing_semicolon.ax",
        "fn main() -> i32 {\n    let value: i32 = 1\n    return value;\n}\n",
    );
    let session = temp.join("session.json");

    let first = run_axc([
        OsStr::new("check"),
        input.as_os_str(),
        OsStr::new("--json"),
        OsStr::new("--ai"),
        OsStr::new("--ai-session"),
        session.as_os_str(),
    ]);
    assert_eq!(first.status.code(), Some(1));
    assert_clean_stderr(&first);

    let second = run_axc([
        OsStr::new("check"),
        input.as_os_str(),
        OsStr::new("--json"),
        OsStr::new("--ai"),
        OsStr::new("--ai-session"),
        session.as_os_str(),
    ]);
    assert_eq!(second.status.code(), Some(1));
    assert_clean_stderr(&second);

    let placeholder = "<input>/missing_semicolon.ax".to_string();

    let mut first_diagnostics: Value =
        serde_json::from_slice(&first.stdout).expect("first diagnostics output should be JSON");
    for diagnostic in first_diagnostics
        .as_array_mut()
        .expect("first diagnostics output should be an array")
    {
        diagnostic["file"] = Value::String(placeholder.clone());
    }

    let mut second_diagnostics: Value =
        serde_json::from_slice(&second.stdout).expect("second diagnostics output should be JSON");
    for diagnostic in second_diagnostics
        .as_array_mut()
        .expect("second diagnostics output should be an array")
    {
        diagnostic["file"] = Value::String(placeholder.clone());
    }

    let first_rendered = serde_json::to_string_pretty(&first_diagnostics)
        .expect("first diagnostics JSON should serialize")
        + "\n";
    assert_eq!(
        normalize_text(&first_rendered),
        snapshot("diagnostics_missing_semicolon_ai_session_l1.json")
    );

    let second_rendered = serde_json::to_string_pretty(&second_diagnostics)
        .expect("second diagnostics JSON should serialize")
        + "\n";
    assert_eq!(
        normalize_text(&second_rendered),
        snapshot("diagnostics_missing_semicolon_ai_session_l2.json")
    );
}

#[test]
fn diagnostics_non_bool_condition_json_with_ai_matches_snapshot() {
    let temp = TempDir::new("diagnostics-non-bool-condition-ai");
    let input = temp.write(
        "non_bool_condition.ax",
        "\
fn main() -> i32 {
    if (1) {
        return 1;
    }
    return 0;
}
",
    );

    let output = run_axc([
        OsStr::new("check"),
        input.as_os_str(),
        OsStr::new("--json"),
        OsStr::new("--ai"),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_clean_stderr(&output);

    let mut diagnostics: Value =
        serde_json::from_slice(&output.stdout).expect("diagnostics output should be JSON");
    let placeholder = "<input>/non_bool_condition.ax".to_string();
    for diagnostic in diagnostics
        .as_array_mut()
        .expect("diagnostics output should be an array")
    {
        diagnostic["file"] = Value::String(placeholder.clone());
    }

    let rendered = serde_json::to_string_pretty(&diagnostics)
        .expect("diagnostics JSON should serialize")
        + "\n";
    assert_eq!(
        normalize_text(&rendered),
        snapshot("diagnostics_non_bool_condition_ai.json")
    );
}

#[test]
fn diagnostics_function_argument_type_json_with_ai_matches_snapshot() {
    let temp = TempDir::new("diagnostics-function-argument-type-ai");
    let input = temp.write(
        "function_argument_type_mismatch.ax",
        "\
fn add(value: i32) -> i32 {
    return value;
}

fn main() -> i32 {
    return add(true);
}
",
    );

    let output = run_axc([
        OsStr::new("check"),
        input.as_os_str(),
        OsStr::new("--json"),
        OsStr::new("--ai"),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_clean_stderr(&output);

    let mut diagnostics: Value =
        serde_json::from_slice(&output.stdout).expect("diagnostics output should be JSON");
    let placeholder = "<input>/function_argument_type_mismatch.ax".to_string();
    for diagnostic in diagnostics
        .as_array_mut()
        .expect("diagnostics output should be an array")
    {
        diagnostic["file"] = Value::String(placeholder.clone());
    }

    let rendered = serde_json::to_string_pretty(&diagnostics)
        .expect("diagnostics JSON should serialize")
        + "\n";
    assert_eq!(
        normalize_text(&rendered),
        snapshot("diagnostics_function_argument_type_ai.json")
    );
}

#[test]
fn check_rejects_unsupported_ai_session_version() {
    let temp = TempDir::new("unsupported-ai-session");
    let input = temp.write(
        "missing_semicolon.ax",
        "fn main() -> i32 {\n    let value: i32 = 1\n    return value;\n}\n",
    );
    let session = temp.write(
        "session.json",
        "{\n  \"version\": 99,\n  \"entries\": {}\n}\n",
    );

    let output = run_axc([
        OsStr::new("check"),
        input.as_os_str(),
        OsStr::new("--json"),
        OsStr::new("--ai"),
        OsStr::new("--ai-session"),
        session.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(string_output(&output.stdout), "");

    let stderr = normalize_text(&string_output(&output.stderr));
    assert!(
        stderr.contains("unsupported AI session version `99`"),
        "expected unsupported session version error, got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("expected `1`"),
        "expected supported version hint, got:\n{}",
        stderr
    );
}

#[test]
fn diagnostics_ai_rule_ids_match_repair_manifest_cases() {
    assert_manifest_cases_keep_stable_diagnostics("benchmarks/repair-cases.json");
}

#[test]
fn diagnostics_ai_rule_ids_match_smoke_repair_manifest_cases() {
    assert_manifest_cases_keep_stable_diagnostics("benchmarks/repair-cases-smoke.json");
}

#[test]
fn repair_benchmark_export_keeps_cold_base_ai_artifact_contracts() {
    let temp = TempDir::new("repair-benchmark-export");
    let output_dir = temp.join("export");
    let script_path = repo_root().join("scripts").join("export-repair-benchmark.ps1");

    let output = run_powershell_script(
        &script_path,
        [
            OsStr::new("-ManifestPath"),
            OsStr::new("benchmarks/repair-cases-smoke.json"),
            OsStr::new("-OutputDir"),
            output_dir.as_os_str(),
            OsStr::new("-SkipBuild"),
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "repair benchmark export should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

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
        10,
        "smoke export should keep the 10-case repair manifest subset"
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

    let cold_bundle = read_json_file(
        &output_dir.join("missing_semicolon_basic").join("bundle.cold.json"),
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
        &output_dir.join("missing_semicolon_basic").join("bundle.base.json"),
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
        &output_dir.join("missing_semicolon_basic").join("bundle.ai.json"),
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
        &fs::read_to_string(output_dir.join("missing_semicolon_basic").join("prompt.cold.md"))
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
        &fs::read_to_string(output_dir.join("missing_semicolon_basic").join("prompt.base.md"))
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
        base_prompt.contains("\"code\": \"P0001\""),
        "base prompt should include the exported diagnostic payload"
    );

    let ai_prompt = normalize_text(
        &fs::read_to_string(output_dir.join("missing_semicolon_basic").join("prompt.ai.md"))
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
        &output_dir.join("index_out_of_bounds_runtime").join("bundle.cold.json"),
        "runtime cold bundle",
    );
    assert_eq!(runtime_cold_bundle["feedback_mode"], Value::from("cold_prompt"));
    assert_eq!(runtime_cold_bundle["diagnostic_command"], Value::from("run"));
    assert!(
        runtime_cold_bundle["diagnostics"]
            .as_array()
            .expect("runtime cold bundle diagnostics should be an array")
            .is_empty(),
        "runtime cold bundle should stay prompt-only"
    );

    let runtime_base_bundle = read_json_file(
        &output_dir.join("index_out_of_bounds_runtime").join("bundle.base.json"),
        "runtime base bundle",
    );
    assert_eq!(
        diagnostic_codes(&runtime_base_bundle["diagnostics"]),
        vec!["R0031".to_string()],
        "runtime base bundle should preserve runtime diagnostic codes"
    );

    let runtime_cold_prompt = normalize_text(
        &fs::read_to_string(output_dir.join("index_out_of_bounds_runtime").join("prompt.cold.md"))
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
fn smoke_repair_manifest_stays_aligned_with_full_manifest() {
    let full_manifest = load_repair_manifest("benchmarks/repair-cases.json");
    let smoke_manifest = load_repair_manifest("benchmarks/repair-cases-smoke.json");

    let full_cases_by_id: BTreeMap<&str, &RepairCaseEntry> = full_manifest
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect();

    assert_eq!(
        smoke_manifest.cases.len(),
        10,
        "smoke manifest should currently pin the 10-case CI subset"
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
        check_case_count, 8,
        "smoke manifest should keep the diagnostics benchmark subset at 8 check-based cases"
    );

    for smoke_case in &smoke_manifest.cases {
        let full_case = *full_cases_by_id.get(smoke_case.id.as_str()).unwrap_or_else(|| {
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
fn diagnostics_ai_session_file_written_by_cli_keeps_versioned_state() {
    let temp = TempDir::new("diagnostics-ai-session-state");
    let input = temp.write(
        "missing_semicolon.ax",
        "fn main() -> i32 {\n    let value: i32 = 1\n    return value;\n}\n",
    );
    let session = temp.join("session.json");

    let output = run_axc([
        OsStr::new("check"),
        input.as_os_str(),
        OsStr::new("--json"),
        OsStr::new("--ai"),
        OsStr::new("--ai-session"),
        session.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_clean_stderr(&output);

    let session_text =
        fs::read_to_string(&session).expect("cli ai session file should be written to disk");
    let session_json: Value =
        serde_json::from_str(&session_text).expect("cli ai session file should be valid JSON");

    assert_eq!(
        session_json["version"],
        Value::from(1),
        "cli ai session file should keep schema version 1"
    );

    let entries = session_json["entries"]
        .as_object()
        .expect("cli ai session file should store an entries object");
    assert_eq!(entries.len(), 1, "expected one tracked session entry");

    let entry = entries
        .values()
        .next()
        .expect("expected one stored ai session entry");
    assert_eq!(
        entry["diagnostic_code"].as_str(),
        Some("P0001"),
        "stored session entry should record the diagnostic code"
    );
    assert_eq!(
        entry["rule_id"].as_str(),
        Some("statement_terminator_required"),
        "stored session entry should record the ai rule id"
    );
    assert_eq!(
        entry["repeat_count"].as_u64(),
        Some(1),
        "stored session entry should start at repeat_count 1"
    );
    assert_eq!(
        entry["last_teaching_level"].as_str(),
        Some("L1"),
        "stored session entry should start at teaching level L1"
    );
}

#[test]
fn ast_dump_matches_snapshot() {
    let temp = TempDir::new("ast");
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

    let output = run_axc([OsStr::new("ast"), input.as_os_str()]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);

    let stdout = normalize_text(&string_output(&output.stdout));
    assert_eq!(stdout, snapshot("ast_hello.json"));
}

#[test]
fn hir_dump_matches_snapshot() {
    let temp = TempDir::new("hir");
    let input = temp.write(
        "for_loop.ax",
        "\
fn main() -> i32 {
    let mut total: i32 = 0;

    for (let mut i: i32 = 0; i < 5; i = i + 1) {
        total = total + i;
    }

    println(total);
    return total;
}
",
    );

    let output = run_axc([OsStr::new("hir"), input.as_os_str()]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);

    let stdout = normalize_text(&string_output(&output.stdout));
    assert_eq!(stdout, snapshot("hir_for_loop.json"));
}

#[test]
fn mir_dump_matches_snapshot() {
    let temp = TempDir::new("mir");
    let input = temp.write(
        "for_loop.ax",
        "\
fn main() -> i32 {
    let mut total: i32 = 0;

    for (let mut i: i32 = 0; i < 5; i = i + 1) {
        total = total + i;
    }

    println(total);
    return total;
}
",
    );

    let output = run_axc([OsStr::new("mir"), input.as_os_str()]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);

    let stdout = normalize_text(&string_output(&output.stdout));
    assert_eq!(stdout, snapshot("mir_for_loop.json"));
}

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
fn bootstrap_state_machine_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/bootstrap_state_machine.ax")]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "1\n0\n");
}

#[test]
fn bootstrap_block_summary_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/bootstrap_block_summary.ax")]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_text(&string_output(&output.stdout)),
        "4\n2\n1\n1\n1\n0\n"
    );
}

#[test]
fn slices_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/slices.ax")]);
    assert_eq!(output.status.code(), Some(3));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "[2, 3]\n7\n");
}

#[test]
fn string_tools_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/string_tools.ax")]);
    assert_eq!(output.status.code(), Some(2));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "AX report ready\n15\n");
}

#[test]
fn traversal_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/traversal.ax")]);
    assert_eq!(output.status.code(), Some(15));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "2\n5\n3\n9\n");
}

#[test]
fn format_report_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/format_report.ax")]);
    assert_eq!(output.status.code(), Some(34));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_text(&string_output(&output.stdout)),
        "count=3, ready=true, values=[2, 4]\n"
    );
}

#[test]
fn empty_array_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/empty_array.ax")]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "[]\n");
}

#[test]
fn run_runtime_error_json_matches_snapshot() {
    let temp = TempDir::new("run-runtime-error");
    let input = temp.write(
        "index_out_of_bounds.ax",
        "\
fn main() -> i32 {
    let values: [i32; 2] = [1, 2];
    return values[2];
}
",
    );

    let output = run_axc([OsStr::new("run"), input.as_os_str(), OsStr::new("--json")]);
    assert_eq!(output.status.code(), Some(1));
    assert_clean_stderr(&output);

    let mut diagnostics: Value =
        serde_json::from_slice(&output.stdout).expect("run diagnostics output should be JSON");
    let placeholder = "<input>/index_out_of_bounds.ax".to_string();
    for diagnostic in diagnostics
        .as_array_mut()
        .expect("run diagnostics output should be an array")
    {
        diagnostic["file"] = Value::String(placeholder.clone());
    }

    let rendered = serde_json::to_string_pretty(&diagnostics)
        .expect("run diagnostics JSON should serialize")
        + "\n";
    assert_eq!(
        normalize_text(&rendered),
        snapshot("run_index_out_of_bounds.json")
    );
}

#[test]
fn run_runtime_error_json_with_ai_matches_snapshot() {
    let temp = TempDir::new("run-runtime-error-ai");
    let input = temp.write(
        "index_out_of_bounds.ax",
        "\
fn main() -> i32 {
    let values: [i32; 2] = [1, 2];
    return values[2];
}
",
    );

    let output = run_axc([
        OsStr::new("run"),
        input.as_os_str(),
        OsStr::new("--json"),
        OsStr::new("--ai"),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_clean_stderr(&output);

    let mut diagnostics: Value =
        serde_json::from_slice(&output.stdout).expect("run diagnostics output should be JSON");
    let placeholder = "<input>/index_out_of_bounds.ax".to_string();
    for diagnostic in diagnostics
        .as_array_mut()
        .expect("run diagnostics output should be an array")
    {
        diagnostic["file"] = Value::String(placeholder.clone());
    }

    let rendered = serde_json::to_string_pretty(&diagnostics)
        .expect("run diagnostics JSON should serialize")
        + "\n";
    assert_eq!(
        normalize_text(&rendered),
        snapshot("run_index_out_of_bounds_ai.json")
    );
}

#[test]
fn run_runtime_division_by_zero_json_matches_snapshot() {
    let temp = TempDir::new("run-runtime-division-by-zero");
    let input = temp.write(
        "division_by_zero.ax",
        "\
fn main() -> i32 {
    let total: i32 = 8;
    let count: i32 = 0;
    return total / count;
}
",
    );

    let output = run_axc([OsStr::new("run"), input.as_os_str(), OsStr::new("--json")]);
    assert_eq!(output.status.code(), Some(1));
    assert_clean_stderr(&output);

    let mut diagnostics: Value =
        serde_json::from_slice(&output.stdout).expect("run diagnostics output should be JSON");
    let placeholder = "<input>/division_by_zero.ax".to_string();
    for diagnostic in diagnostics
        .as_array_mut()
        .expect("run diagnostics output should be an array")
    {
        diagnostic["file"] = Value::String(placeholder.clone());
    }

    let rendered = serde_json::to_string_pretty(&diagnostics)
        .expect("run diagnostics JSON should serialize")
        + "\n";
    assert_eq!(
        normalize_text(&rendered),
        snapshot("run_division_by_zero.json")
    );
}

#[test]
fn run_runtime_division_by_zero_json_with_ai_matches_snapshot() {
    let temp = TempDir::new("run-runtime-division-by-zero-ai");
    let input = temp.write(
        "division_by_zero.ax",
        "\
fn main() -> i32 {
    let total: i32 = 8;
    let count: i32 = 0;
    return total / count;
}
",
    );

    let output = run_axc([
        OsStr::new("run"),
        input.as_os_str(),
        OsStr::new("--json"),
        OsStr::new("--ai"),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_clean_stderr(&output);

    let mut diagnostics: Value =
        serde_json::from_slice(&output.stdout).expect("run diagnostics output should be JSON");
    let placeholder = "<input>/division_by_zero.ax".to_string();
    for diagnostic in diagnostics
        .as_array_mut()
        .expect("run diagnostics output should be an array")
    {
        diagnostic["file"] = Value::String(placeholder.clone());
    }

    let rendered = serde_json::to_string_pretty(&diagnostics)
        .expect("run diagnostics JSON should serialize")
        + "\n";
    assert_eq!(
        normalize_text(&rendered),
        snapshot("run_division_by_zero_ai.json")
    );
}

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

    let source_copy = normalize_text(
        &fs::read_to_string(&source_copy_path).expect("project build source copy should exist"),
    );
    let original = normalize_text(
        &fs::read_to_string(&input).expect("project build input source should exist"),
    );
    assert_eq!(source_copy, original);

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
        normalize_text(&rendered),
        snapshot("build_project_hello_manifest.json")
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
        normalize_text(&rendered),
        snapshot("build_hello_manifest.json")
    );

    let stdout = normalize_text(&string_output(&output.stdout));
    assert!(
        stdout.starts_with("build succeeded: "),
        "expected build success message, got:\n{}",
        stdout
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
