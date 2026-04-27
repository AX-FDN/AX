use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use serde_json::Value;

static INVOCABLE_AXC_COPY_SERIAL: AtomicU64 = AtomicU64::new(0);

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn axc_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_axc"))
}

fn invocable_axc_binary() -> PathBuf {
    let source = axc_binary();
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{value}"))
        .unwrap_or_default();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    let serial = INVOCABLE_AXC_COPY_SERIAL.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "axc-interface-bin-{}-{nonce}-{serial}{extension}",
        std::process::id(),
    ));
    fs::copy(&source, &path).unwrap_or_else(|error| {
        panic!(
            "failed to create invocable AX binary copy from `{}` to `{}`: {error}",
            source.display(),
            path.display()
        )
    });
    path
}

fn remove_temp_file_best_effort(path: &Path) {
    for _ in 0..20 {
        match fs::remove_file(path) {
            Ok(()) => return,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
            Err(_) => thread::sleep(Duration::from_millis(100)),
        }
    }
}

fn write_temp_text(path: &Path, text: &str) {
    if path.extension().and_then(|value| value.to_str()) == Some("ps1") {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.as_bytes());
        fs::write(path, bytes).expect("failed to write temp file");
        return;
    }

    fs::write(path, text).expect("failed to write temp file");
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

fn normalize_build_manifest_value(manifest: &mut Value) {
    if let Some(path) = manifest
        .get("artifacts")
        .and_then(|artifacts| artifacts.get("planned_executable"))
        .and_then(Value::as_str)
    {
        let normalized = path.strip_suffix(".exe").unwrap_or(path).to_string();
        manifest["artifacts"]["planned_executable"] = Value::String(normalized);
    }
}

fn normalized_build_manifest_json(text: &str) -> String {
    let mut manifest: Value =
        serde_json::from_str(text).expect("build manifest snapshot should be valid JSON");
    normalize_build_manifest_value(&mut manifest);
    serde_json::to_string_pretty(&manifest).expect("build manifest JSON should serialize") + "\n"
}

fn run_axc<I, S>(args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args: Vec<OsString> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_os_string())
        .collect();
    let binary = invocable_axc_binary();
    let mut last_busy_error = None;
    for _ in 0..20 {
        match Command::new(&binary)
            .args(&args)
            .current_dir(repo_root())
            .output()
        {
            Ok(output) => {
                remove_temp_file_best_effort(&binary);
                return output;
            }
            Err(error) if error.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                last_busy_error = Some(error);
                thread::sleep(Duration::from_millis(50));
            }
            Err(error) => {
                remove_temp_file_best_effort(&binary);
                panic!("failed to execute axc: {error}");
            }
        }
    }

    remove_temp_file_best_effort(&binary);
    let error = last_busy_error.expect("executable file busy retries should record an error");
    panic!("failed to execute axc after retries: {error}");
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
    let binary = invocable_axc_binary();
    let mut command = Command::new(powershell_executable());
    command.arg("-NoProfile");
    if cfg!(windows) {
        command.arg("-ExecutionPolicy").arg("Bypass");
    }
    command
        .arg("-File")
        .arg(script)
        .env("AXC_BINARY", &binary)
        .args(args)
        .current_dir(repo_root())
        .output()
        .map(|output| {
            remove_temp_file_best_effort(&binary);
            output
        })
        .unwrap_or_else(|error| {
            remove_temp_file_best_effort(&binary);
            panic!("failed to execute PowerShell script: {error}")
        })
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
        write_temp_text(&path, text);
        path
    }

    fn write_nested(&self, name: &str, text: &str) -> PathBuf {
        let path = self.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("failed to create temp subdirectory");
        }
        write_temp_text(&path, text);
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
    serde_json::from_str(&text).unwrap_or_else(|error| {
        panic!(
            "failed to parse {label} `{}` as JSON: {error}",
            path.display()
        )
    })
}

fn powershell_literal_path(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "''")
}

fn json_string_array(value: &Value, label: &str) -> Vec<String> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("{label} should be a JSON array"))
        .iter()
        .map(|item| {
            item.as_str()
                .unwrap_or_else(|| panic!("{label} should only contain strings"))
                .to_string()
        })
        .collect()
}

fn assert_json_f64(value: &Value, expected: f64, label: &str) {
    let actual = value
        .as_f64()
        .unwrap_or_else(|| panic!("{label} should be a JSON number"));
    assert!(
        (actual - expected).abs() < 1e-9,
        "{label} expected {expected}, got {actual}"
    );
}

fn assert_json_path_exists(value: &Value, label: &str) -> PathBuf {
    let path = PathBuf::from(
        value
            .as_str()
            .unwrap_or_else(|| panic!("{label} should be a JSON string path")),
    );
    assert!(
        path.exists(),
        "{label} should exist at `{}`",
        path.display()
    );
    path
}

fn export_repair_benchmark(temp: &TempDir, manifest_path: &Path) -> PathBuf {
    let output_dir = temp.join("benchmark");
    let script_path = repo_root()
        .join("scripts")
        .join("export-repair-benchmark.ps1");

    let output = run_powershell_script(
        &script_path,
        [
            OsStr::new("-ManifestPath"),
            manifest_path.as_os_str(),
            OsStr::new("-OutputDir"),
            output_dir.as_os_str(),
            OsStr::new("-SkipBuild"),
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "smoke repair benchmark export should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    output_dir
}

fn export_repair_benchmark_with_context(temp: &TempDir, manifest_path: &Path) -> PathBuf {
    let output_dir = temp.join("benchmark-with-context");
    let script_path = repo_root()
        .join("scripts")
        .join("export-repair-benchmark.ps1");

    let output = run_powershell_script(
        &script_path,
        [
            OsStr::new("-ManifestPath"),
            manifest_path.as_os_str(),
            OsStr::new("-OutputDir"),
            output_dir.as_os_str(),
            OsStr::new("-IncludeContext"),
            OsStr::new("-SkipBuild"),
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "context repair benchmark export should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    output_dir
}

fn export_smoke_repair_benchmark(temp: &TempDir) -> PathBuf {
    export_repair_benchmark(
        temp,
        &repo_root()
            .join("benchmarks")
            .join("repair-cases-smoke.json"),
    )
}

fn write_replay_wrapper(
    temp: &TempDir,
    name: &str,
    shared_source_dir: &Path,
    cold_source_dir: Option<&Path>,
    base_source_dir: Option<&Path>,
    ai_source_dir: Option<&Path>,
) -> PathBuf {
    let replay_adapter = repo_root()
        .join("scripts")
        .join("replay-repair-adapter.ps1");
    let mut args = vec![
        "-PromptPath $PromptPath".to_string(),
        "-BundlePath $BundlePath".to_string(),
        "-OutputPath $OutputPath".to_string(),
        "-CaseId $CaseId".to_string(),
        "-FeedbackMode $FeedbackMode".to_string(),
        format!(
            "-SourceDir '{}'",
            powershell_literal_path(shared_source_dir)
        ),
    ];

    if let Some(path) = cold_source_dir {
        args.push(format!(
            "-SourceDirCold '{}'",
            powershell_literal_path(path)
        ));
    }

    if let Some(path) = base_source_dir {
        args.push(format!(
            "-SourceDirBase '{}'",
            powershell_literal_path(path)
        ));
    }

    if let Some(path) = ai_source_dir {
        args.push(format!("-SourceDirAi '{}'", powershell_literal_path(path)));
    }

    let script_text = format!(
        "\
param(
    [Parameter(Mandatory = $true)]
    [string] $PromptPath,
    [Parameter(Mandatory = $true)]
    [string] $BundlePath,
    [Parameter(Mandatory = $true)]
    [string] $OutputPath,
    [Parameter(Mandatory = $true)]
    [string] $CaseId,
    [Parameter(Mandatory = $true)]
    [string] $FeedbackMode
)

$ErrorActionPreference = \"Stop\"

& '{}' `
    {}
",
        powershell_literal_path(&replay_adapter),
        args.join(" `\n    ")
    );

    temp.write(name, &script_text)
}

fn write_stdout_only_runner(temp: &TempDir, name: &str) -> PathBuf {
    temp.write(
        name,
        "\
param(
    [Parameter(Mandatory = $true)]
    [string] $PromptPath,
    [Parameter(Mandatory = $true)]
    [string] $BundlePath,
    [Parameter(Mandatory = $true)]
    [string] $OutputPath,
    [Parameter(Mandatory = $true)]
    [string] $CaseId,
    [Parameter(Mandatory = $true)]
    [string] $FeedbackMode
)

$ErrorActionPreference = \"Stop\"
$payload = \"A\" * 131072
$source = \"fn main() -> i32 { let payload: string = `\"$payload`\"; return 0; }`n\"
[Console]::Out.Write($source)
",
    )
}

fn write_silent_success_runner(temp: &TempDir, name: &str) -> PathBuf {
    temp.write(
        name,
        "\
param(
    [Parameter(Mandatory = $true)]
    [string] $PromptPath,
    [Parameter(Mandatory = $true)]
    [string] $BundlePath,
    [Parameter(Mandatory = $true)]
    [string] $OutputPath,
    [Parameter(Mandatory = $true)]
    [string] $CaseId,
    [Parameter(Mandatory = $true)]
    [string] $FeedbackMode
)

$ErrorActionPreference = \"Stop\"
exit 0
",
    )
}

fn write_single_case_manifest(temp: &TempDir, name: &str) -> PathBuf {
    temp.write(
        name,
        "\
{
  \"version\": 1,
  \"description\": \"Single-case runner contract benchmark.\",
  \"cases\": [
    {
      \"id\": \"missing_semicolon_basic\",
      \"file\": \"examples/missing_semicolon.ax\",
      \"category\": \"syntax\",
      \"diagnostic_command\": \"check\",
      \"expected_codes\": [\"P0001\"],
      \"expected_ai_rule_ids\": [\"statement_terminator_required\"],
      \"repair_goal\": \"Insert the missing semicolon after the let binding.\",
      \"notes\": \"Contract-only benchmark case.\"
    }
  ]
}
",
    )
}

fn write_single_runtime_case_manifest(temp: &TempDir, name: &str) -> PathBuf {
    temp.write(
        name,
        "\
{
  \"version\": 1,
  \"description\": \"Single-case runtime runner contract benchmark.\",
  \"cases\": [
    {
      \"id\": \"index_out_of_bounds_runtime\",
      \"file\": \"examples/index_out_of_bounds.ax\",
      \"category\": \"runtime\",
      \"diagnostic_command\": \"run\",
      \"expected_codes\": [\"R0031\"],
      \"expected_ai_rule_ids\": [\"array_index_must_stay_in_bounds\"],
      \"repair_goal\": \"Keep the array index within the declared fixed-size array bounds.\",
      \"notes\": \"Runtime contract-only benchmark case.\"
    }
  ]
}
",
    )
}

fn json_path_literal(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "\\\\")
}

fn write_single_project_case_manifest(
    temp: &TempDir,
    name: &str,
    project_path: &Path,
    target_file_path: &Path,
) -> PathBuf {
    let manifest = serde_json::json!({
        "version": 1,
        "description": "Single-case project-context benchmark.",
        "cases": [
            {
                "id": "project_missing_semicolon",
                "file": json_path_literal(target_file_path),
                "project": json_path_literal(project_path),
                "category": "syntax",
                "diagnostic_command": "check",
                "expected_codes": ["P0001"],
                "expected_ai_rule_ids": ["statement_terminator_required"],
                "repair_goal": "Insert the missing semicolon in the project helper file.",
                "notes": "Project-context contract-only benchmark case.",
                "context_symbol": "helper"
            }
        ]
    });
    let text =
        serde_json::to_string_pretty(&manifest).expect("project manifest should serialize") + "\n";
    temp.write(name, &text)
}

fn write_utf8_bom_candidate(path: &Path, text: &str) {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    fs::write(path, bytes).expect("failed to write UTF-8 BOM candidate");
}

fn write_utf16le_bom_candidate(path: &Path, text: &str) {
    let mut bytes = vec![0xFF, 0xFE];
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    fs::write(path, bytes).expect("failed to write UTF-16 LE BOM candidate");
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

const SHARED_FOUNDATION_PROJECT_SOURCES: &[&str] = &[
    "external/foundation/cli.ax",
    "external/foundation/file_kind.ax",
    "external/foundation/report.ax",
    "external/foundation/search.ax",
    "external/foundation/text.ax",
    "external/foundation/workspace.ax",
];

const SHARED_STD_PROJECT_SOURCES: &[&str] = &[
    "external/std/cli.ax",
    "external/std/fs.ax",
    "external/std/path.ax",
    "external/std/report.ax",
    "external/std/text.ax",
    "external/std/workspace.ax",
];

fn project_sources_with_shared_foundation(extra: &[&str]) -> Vec<String> {
    SHARED_FOUNDATION_PROJECT_SOURCES
        .iter()
        .copied()
        .chain(extra.iter().copied())
        .map(str::to_string)
        .collect()
}

fn project_sources_with_shared_std(extra: &[&str]) -> Vec<String> {
    SHARED_STD_PROJECT_SOURCES
        .iter()
        .copied()
        .chain(extra.iter().copied())
        .map(str::to_string)
        .collect()
}

fn project_sources_path(root: &Path, relative: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    for segment in relative.split('/') {
        path.push(segment);
    }
    path
}

fn assert_project_example_build_sources(
    label: &str,
    example_path: &str,
    expected_sources: &[String],
) {
    let temp = TempDir::new(label);
    let out_dir = temp.join("build-out");

    let output = run_axc([
        OsStr::new("build"),
        OsStr::new(example_path),
        OsStr::new("--out-dir"),
        out_dir.as_os_str(),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "build for `{example_path}` should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    assert!(
        out_dir.join("build-manifest.json").exists(),
        "build should emit build-manifest.json for `{example_path}`"
    );
    assert!(
        out_dir.join("AX.toml").exists(),
        "build should copy AX.toml for `{example_path}`"
    );
    assert!(
        out_dir.join("source.ax").exists(),
        "build should copy source.ax for `{example_path}`"
    );
    assert!(
        out_dir.join("project-sources").exists(),
        "build should copy project-sources for `{example_path}`"
    );

    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(out_dir.join("build-manifest.json"))
            .expect("project build manifest should be readable"),
    )
    .expect("project build manifest should be valid JSON");
    assert_eq!(
        json_string_array(&manifest["artifacts"]["project_sources"], "project sources"),
        expected_sources,
        "build should keep stable packaged source order for `{example_path}`"
    );

    let project_sources_root = out_dir.join("project-sources");
    for source in expected_sources {
        let copied_path = project_sources_path(&project_sources_root, source);
        assert!(
            copied_path.exists(),
            "build should copy `{}` for `{example_path}`",
            source
        );
    }
}

fn assert_project_example_checks(example_path: &str) {
    let output = run_axc([OsStr::new("check"), OsStr::new(example_path)]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "check for `{example_path}` should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);
}

fn normalize_temp_output(text: &str, temp: &TempDir) -> String {
    let normalized = normalize_text(text).replace('\\', "/");
    let root = temp.path.display().to_string().replace('\\', "/");
    normalized.replace(&root, "<root>")
}

fn line_count(text: &str) -> i32 {
    text.lines().count() as i32
}

fn nonempty_line_count(text: &str) -> i32 {
    text.lines().filter(|line| !line.trim().is_empty()).count() as i32
}

fn heading_count(text: &str) -> i32 {
    text.lines()
        .filter(|line| line.trim().starts_with('#'))
        .count() as i32
}

fn action_item_count(text: &str) -> i32 {
    text.lines()
        .filter(|line| line.contains("TODO") || line.contains("FIXME"))
        .count() as i32
}

#[derive(Deserialize)]
struct RepairCaseManifest {
    cases: Vec<RepairCaseEntry>,
}

#[derive(Deserialize)]
struct RepairCaseEntry {
    id: String,
    file: String,
    project: Option<String>,
    diagnostic_command: Option<String>,
    expected_codes: Vec<String>,
    expected_ai_rule_ids: Vec<String>,
}

impl RepairCaseEntry {
    fn diagnostic_command(&self) -> &str {
        self.diagnostic_command.as_deref().unwrap_or("check")
    }

    fn diagnostic_target(&self) -> &str {
        self.project.as_deref().unwrap_or(self.file.as_str())
    }
}

fn load_repair_manifest(path: &str) -> RepairCaseManifest {
    let manifest_text = fs::read_to_string(repo_root().join(path)).unwrap_or_else(|error| {
        panic!("repair benchmark manifest `{path}` should be readable: {error}")
    });
    serde_json::from_str(&manifest_text).unwrap_or_else(|error| {
        panic!("repair benchmark manifest `{path}` should be valid: {error}")
    })
}

fn find_replay_candidate(root: &Path, case_id: &str) -> Option<PathBuf> {
    let flat = root.join(format!("{case_id}.ax"));
    if flat.exists() {
        return Some(flat);
    }

    let nested = root.join(case_id).join("repaired.ax");
    if nested.exists() {
        return Some(nested);
    }

    None
}

fn assert_manifest_cases_keep_stable_diagnostics(manifest_path: &str) {
    let manifest = load_repair_manifest(manifest_path);

    for case in manifest.cases {
        let output = run_axc([
            OsStr::new(case.diagnostic_command()),
            OsStr::new(case.diagnostic_target()),
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
        11,
        "smoke export should keep the 11-case repair manifest subset"
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
    assert_eq!(run_summary["totals"]["total"], Value::from(11));
    assert_eq!(run_summary["totals"]["ok"], Value::from(11));
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
    assert_eq!(run_cases.len(), 11);
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
    assert_eq!(score_summary["totals"]["total"], Value::from(11));
    assert_eq!(score_summary["totals"]["passed"], Value::from(11));
    assert_eq!(score_summary["totals"]["failed"], Value::from(0));
    assert_eq!(score_summary["totals"]["missing"], Value::from(0));

    let score_cases = score_summary["cases"]
        .as_array()
        .expect("score summary should include cases");
    assert_eq!(score_cases.len(), 11);
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
    assert_eq!(check_cases.len(), 9);
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
    assert_eq!(comparison["comparison"]["total_cases"], Value::from(11));
    assert_eq!(comparison["comparison"]["base_passed"], Value::from(6));
    assert_eq!(comparison["comparison"]["ai_passed"], Value::from(11));
    assert_eq!(
        comparison["comparison"]["absolute_lift_cases"],
        Value::from(5)
    );
    assert_json_f64(
        &comparison["comparison"]["absolute_lift_pp"],
        45.45,
        "comparison absolute_lift_pp",
    );
    assert_json_f64(
        &comparison["comparison"]["relative_lift_pct"],
        83.33,
        "comparison relative_lift_pct",
    );
    assert_eq!(
        comparison["modes"]["base"]["invocation_totals"]["ok"],
        Value::from(11)
    );
    assert_eq!(
        comparison["modes"]["ai"]["invocation_totals"]["ok"],
        Value::from(11)
    );
    assert_eq!(
        comparison["modes"]["base"]["score_totals"]["failed"],
        Value::from(5)
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
            "slice_assignment_read_only".to_string(),
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
        6
    );

    let case_deltas = comparison["cases"]
        .as_array()
        .expect("comparison should include per-case deltas");
    assert_eq!(case_deltas.len(), 11);
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
    assert_eq!(semantic["total"], Value::from(6));
    assert_eq!(semantic["base_passed"], Value::from(3));
    assert_eq!(semantic["ai_passed"], Value::from(6));
    assert_eq!(semantic["improved"], Value::from(3));
    assert_eq!(semantic["regressed"], Value::from(0));
    assert_eq!(
        json_string_array(&semantic["improved_case_ids"], "semantic improved_case_ids",),
        vec![
            "type_mismatch_bool_from_int".to_string(),
            "missing_struct_literal_field".to_string(),
            "slice_assignment_read_only".to_string(),
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
    assert_eq!(comparison["summary"]["total_cases"], Value::from(11));
    assert_eq!(comparison["summary"]["cold_passed"], Value::from(4));
    assert_eq!(comparison["summary"]["base_passed"], Value::from(6));
    assert_eq!(comparison["summary"]["ai_passed"], Value::from(11));
    assert_eq!(comparison["modes"]["cold"]["exit_code"], Value::from(1));
    assert_eq!(comparison["modes"]["base"]["exit_code"], Value::from(1));
    assert_eq!(comparison["modes"]["ai"]["exit_code"], Value::from(0));
    assert_eq!(
        comparison["modes"]["cold"]["score_totals"]["failed"],
        Value::from(7)
    );
    assert_eq!(
        comparison["modes"]["base"]["score_totals"]["failed"],
        Value::from(5)
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
        Value::from(5)
    );
    assert_eq!(
        comparison["summary"]["pairwise_comparisons"]["cold_to_ai"]["absolute_lift_cases"],
        Value::from(7)
    );
    assert_eq!(
        comparison["summary"]["pairwise_comparisons"]["cold_to_base"]["absolute_lift_pp"]
            .as_f64()
            .expect("cold_to_base.absolute_lift_pp should be numeric"),
        18.19
    );
    assert_eq!(
        comparison["summary"]["pairwise_comparisons"]["base_to_ai"]["absolute_lift_pp"]
            .as_f64()
            .expect("base_to_ai.absolute_lift_pp should be numeric"),
        45.45
    );
    assert_eq!(
        comparison["summary"]["pairwise_comparisons"]["cold_to_ai"]["absolute_lift_pp"]
            .as_f64()
            .expect("cold_to_ai.absolute_lift_pp should be numeric"),
        63.64
    );
    assert_json_f64(
        &comparison["summary"]["pairwise_comparisons"]["cold_to_base"]["relative_lift_pct"],
        50.0,
        "cold_to_base relative_lift_pct",
    );
    assert_json_f64(
        &comparison["summary"]["pairwise_comparisons"]["base_to_ai"]["relative_lift_pct"],
        83.33,
        "base_to_ai relative_lift_pct",
    );
    assert_json_f64(
        &comparison["summary"]["pairwise_comparisons"]["cold_to_ai"]["relative_lift_pct"],
        175.0,
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
            "slice_assignment_read_only".to_string(),
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
            "len_builtin_non_countable_value".to_string(),
            "slice_assignment_read_only".to_string(),
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
        9
    );
    assert_eq!(
        comparison["summary"]["pairwise_comparisons"]["base_to_ai"]["unchanged_cases"]
            .as_array()
            .expect("base_to_ai unchanged_cases should be an array")
            .len(),
        6
    );
    assert_eq!(
        comparison["summary"]["pairwise_comparisons"]["cold_to_ai"]["unchanged_cases"]
            .as_array()
            .expect("cold_to_ai unchanged_cases should be an array")
            .len(),
        4
    );

    let case_deltas = comparison["cases"]
        .as_array()
        .expect("mode comparison should include per-case deltas");
    assert_eq!(case_deltas.len(), 11);
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
    assert_eq!(semantic["total"], Value::from(6));
    assert_eq!(semantic["cold_passed"], Value::from(1));
    assert_eq!(semantic["base_passed"], Value::from(3));
    assert_eq!(semantic["ai_passed"], Value::from(6));

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
        30,
        "full manifest should currently pin the 30-case repair benchmark baseline"
    );

    assert_eq!(
        smoke_manifest.cases.len(),
        11,
        "smoke manifest should currently pin the 11-case CI subset"
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
        check_case_count, 9,
        "smoke manifest should keep the diagnostics benchmark subset at 9 check-based cases"
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
    assert_eq!(summary["totals"]["total"], Value::from(30));
    assert_eq!(summary["totals"]["passed"], Value::from(30));
    assert_eq!(summary["totals"]["failed"], Value::from(0));
    assert_eq!(summary["totals"]["missing"], Value::from(0));
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

#[test]
fn bootstrap_state_machine_example_runs() {
    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/bootstrap_state_machine.ax"),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "1\n0\n");
}

#[test]
fn bootstrap_block_summary_example_runs() {
    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/bootstrap_block_summary.ax"),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_text(&string_output(&output.stdout)),
        "4\n2\n1\n1\n1\n0\n"
    );
}

#[test]
fn bootstrap_token_scan_example_runs() {
    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/bootstrap_token_scan.ax"),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "3\n21\n");
}

#[test]
fn slices_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/slices.ax")]);
    assert_eq!(output.status.code(), Some(3));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_text(&string_output(&output.stdout)),
        "[2, 3]\n7\n"
    );
}

#[test]
fn string_tools_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/string_tools.ax")]);
    assert_eq!(output.status.code(), Some(2));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_text(&string_output(&output.stdout)),
        "AX report ready\n15\n"
    );
}

#[test]
fn string_list_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/string_list.ax")]);
    assert_eq!(output.status.code(), Some(2));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_text(&string_output(&output.stdout)),
        "2\nalpha, beta\n"
    );
}

#[test]
fn traversal_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/traversal.ax")]);
    assert_eq!(output.status.code(), Some(15));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_text(&string_output(&output.stdout)),
        "2\n5\n3\n9\n"
    );
}

#[test]
fn continue_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/continue.ax")]);
    assert_eq!(output.status.code(), Some(16));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "8\n8\n");
}

#[test]
fn match_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/match.ax")]);
    assert_eq!(output.status.code(), Some(25));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "25\n");
}

#[test]
fn match_expr_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/match_expr.ax")]);
    assert_eq!(output.status.code(), Some(6));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "2\n6\n");
}

#[test]
fn match_binding_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/match_binding.ax")]);
    assert_eq!(output.status.code(), Some(6));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "false\n6\n");
}

#[test]
fn payload_enum_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/payload_enum.ax")]);
    assert_eq!(output.status.code(), Some(7));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "7\n0\n-1\n");
}

#[test]
fn logical_ops_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/logical_ops.ax")]);
    assert_eq!(output.status.code(), Some(7));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "logical\n");
}

#[test]
fn modulo_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/modulo.ax")]);
    assert_eq!(output.status.code(), Some(3));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "1\n2\n");
}

#[test]
fn for_in_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/for_in.ax")]);
    assert_eq!(output.status.code(), Some(9));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "1\n3\n5\n");
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
fn workspace_audit_example_runs_on_controlled_fixture() {
    let temp = TempDir::new("workspace-audit-example");
    let workspace_dir = temp.join("workspace");
    fs::create_dir_all(workspace_dir.join("docs")).expect("docs directory should exist");
    fs::create_dir_all(workspace_dir.join("tmp").join("inner"))
        .expect("nested directory should exist");

    let app_text = "\
fn main() -> i32 {
    return 0;
}
";
    let guide_text = "\
# Guide
TODO: refine
Details
";
    let blob_bytes = b"AX\x00\x01";

    fs::write(workspace_dir.join("app.ax"), app_text).expect("app.ax should exist");
    fs::write(workspace_dir.join("docs").join("guide.md"), guide_text)
        .expect("guide.md should exist");
    fs::write(workspace_dir.join("docs").join("blob.bin"), blob_bytes)
        .expect("blob.bin should exist");

    let output_path = temp.join("audit.txt");
    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/workspace_audit.ax"),
        OsStr::new("--"),
        workspace_dir.as_os_str(),
        output_path.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);

    let stdout = normalize_temp_output(&string_output(&output.stdout), &temp);
    assert_eq!(stdout, "audited=<root>/audit.txt\n");

    let rendered = normalize_temp_output(
        &fs::read_to_string(&output_path).expect("audit report should exist"),
        &temp,
    );
    let expected = format!(
        "\
root=<root>/workspace
scope=top-level + one nested level
top_level_entries=3
directories=3
files=3
text_files=2
bytes={}
lines={}
nonempty={}
headings={}
action_items={}

entries:
app.ax | file | bytes={} | lines={} | nonempty={} | headings=0 | action_items=0
docs | dir | children=2
  docs/blob.bin | file | bytes={}
  docs/guide.md | file | bytes={} | lines={} | nonempty={} | headings={} | action_items={}
tmp | dir | children=1
  tmp/inner | dir | children=0
",
        app_text.len() + guide_text.len() + blob_bytes.len(),
        line_count(app_text) + line_count(guide_text),
        nonempty_line_count(app_text) + nonempty_line_count(guide_text),
        heading_count(guide_text),
        action_item_count(guide_text),
        app_text.len(),
        line_count(app_text),
        nonempty_line_count(app_text),
        blob_bytes.len(),
        guide_text.len(),
        line_count(guide_text),
        nonempty_line_count(guide_text),
        heading_count(guide_text),
        action_item_count(guide_text),
    );
    assert_eq!(rendered, expected);
}

#[test]
fn project_workspace_audit_runs_on_controlled_fixture() {
    let temp = TempDir::new("project-workspace-audit");
    let workspace_dir = temp.join("workspace");
    fs::create_dir_all(workspace_dir.join("docs")).expect("docs directory should exist");
    fs::create_dir_all(workspace_dir.join("tmp").join("inner"))
        .expect("nested directory should exist");

    let app_text = "\
fn main() -> i32 {
    return 0;
}
";
    let guide_text = "\
# Guide
TODO: refine
Details
";
    let blob_bytes = b"AX\x00\x01";

    fs::write(workspace_dir.join("app.ax"), app_text).expect("app.ax should exist");
    fs::write(workspace_dir.join("docs").join("guide.md"), guide_text)
        .expect("guide.md should exist");
    fs::write(workspace_dir.join("docs").join("blob.bin"), blob_bytes)
        .expect("blob.bin should exist");

    let output_path = temp.join("audit.txt");
    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/project_workspace_audit"),
        OsStr::new("--"),
        workspace_dir.as_os_str(),
        output_path.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);

    let stdout = normalize_temp_output(&string_output(&output.stdout), &temp);
    assert_eq!(stdout, "audited=<root>/audit.txt\n");

    let rendered = normalize_temp_output(
        &fs::read_to_string(&output_path).expect("audit report should exist"),
        &temp,
    );
    let expected = format!(
        "\
root=<root>/workspace
scope=top-level + one nested level
top_level_entries=3
directories=3
files=3
text_files=2
bytes={}
lines={}
nonempty={}
headings={}
action_items={}

entries:
app.ax | file | bytes={} | lines={} | nonempty={} | headings=0 | action_items=0
docs | dir | children=2
  docs/blob.bin | file | bytes={}
  docs/guide.md | file | bytes={} | lines={} | nonempty={} | headings={} | action_items={}
tmp | dir | children=1
  tmp/inner | dir | children=0
",
        app_text.len() + guide_text.len() + blob_bytes.len(),
        line_count(app_text) + line_count(guide_text),
        nonempty_line_count(app_text) + nonempty_line_count(guide_text),
        heading_count(guide_text),
        action_item_count(guide_text),
        app_text.len(),
        line_count(app_text),
        nonempty_line_count(app_text),
        blob_bytes.len(),
        guide_text.len(),
        line_count(guide_text),
        nonempty_line_count(guide_text),
        heading_count(guide_text),
        action_item_count(guide_text),
    );
    assert_eq!(rendered, expected);
}

#[test]
fn docs_release_snapshot_example_runs_on_controlled_fixture() {
    let temp = TempDir::new("docs-release-snapshot-example");
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
        OsStr::new("examples/docs_release_snapshot.ax"),
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

    let copied_alpha =
        fs::read_to_string(snapshot_dir.join("alpha.md")).expect("copied alpha.md should exist");
    let copied_beta =
        fs::read_to_string(snapshot_dir.join("beta.md")).expect("copied beta.md should exist");
    assert_eq!(copied_alpha, alpha_text);
    assert_eq!(copied_beta, beta_text);

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

#[test]
fn workspace_search_report_example_runs_on_controlled_fixture() {
    let temp = TempDir::new("workspace-search-report-example");
    let workspace_dir = temp.join("workspace");
    fs::create_dir_all(workspace_dir.join("docs")).expect("docs directory should exist");
    fs::create_dir_all(workspace_dir.join("tmp").join("inner"))
        .expect("nested directory should exist");

    let app_text = "\
repair plan
stable repair output
done
";
    let guide_text = "\
repair evidence
more detail
";
    let notes_text = "\
plain note
still stable
";

    fs::write(workspace_dir.join("app.ax"), app_text).expect("app.ax should exist");
    fs::write(workspace_dir.join("docs").join("guide.md"), guide_text)
        .expect("guide.md should exist");
    fs::write(workspace_dir.join("notes.md"), notes_text).expect("notes.md should exist");
    fs::write(workspace_dir.join("docs").join("blob.bin"), b"\x01\x02\x03")
        .expect("blob.bin should exist");

    let output_path = temp.join("search.txt");
    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/workspace_search_report.ax"),
        OsStr::new("--"),
        workspace_dir.as_os_str(),
        OsStr::new("repair"),
        output_path.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);

    let stdout = normalize_temp_output(&string_output(&output.stdout), &temp);
    assert_eq!(stdout, "searched=<root>/search.txt\n");

    let rendered = normalize_temp_output(
        &fs::read_to_string(&output_path).expect("search report should exist"),
        &temp,
    );
    let expected = format!(
        "\
root=<root>/workspace
needle=repair
scope=top-level + one nested level
searchable_files=3
matched_files=2
bytes={}
lines={}
matched_lines=3

matches:
app.ax | bytes={} | lines={} | matched_lines=2
  docs/guide.md | bytes={} | lines={} | matched_lines=1
",
        app_text.len() + guide_text.len() + notes_text.len(),
        line_count(app_text) + line_count(guide_text) + line_count(notes_text),
        app_text.len(),
        line_count(app_text),
        guide_text.len(),
        line_count(guide_text),
    );
    assert_eq!(rendered, expected);
}

#[test]
fn project_workspace_search_report_runs_on_controlled_fixture() {
    let temp = TempDir::new("project-workspace-search-report");
    let workspace_dir = temp.join("workspace");
    let docs_dir = workspace_dir.join("docs");
    let api_dir = docs_dir.join("api");
    fs::create_dir_all(&api_dir).expect("docs/api directory should exist");
    fs::create_dir_all(workspace_dir.join("tmp").join("inner"))
        .expect("nested directory should exist");

    let app_text = "\
repair plan
stable repair output
done
";
    let guide_text = "\
repair evidence
more detail
";
    let notes_text = "\
plain note
still stable
";

    fs::write(workspace_dir.join("app.ax"), app_text).expect("app.ax should exist");
    fs::write(api_dir.join("guide.md"), guide_text).expect("guide.md should exist");
    fs::write(workspace_dir.join("notes.md"), notes_text).expect("notes.md should exist");
    fs::write(docs_dir.join("blob.bin"), b"\x01\x02\x03").expect("blob.bin should exist");

    let output_path = temp.join("search.txt");
    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/project_workspace_search_report"),
        OsStr::new("--"),
        workspace_dir.as_os_str(),
        OsStr::new("repair"),
        output_path.as_os_str(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);

    let stdout = normalize_temp_output(&string_output(&output.stdout), &temp);
    assert_eq!(stdout, "searched=<root>/search.txt\n");

    let rendered = normalize_temp_output(
        &fs::read_to_string(&output_path).expect("search report should exist"),
        &temp,
    );
    let expected = format!(
        "\
root=<root>/workspace
needle=repair
scope=recursive
searchable_files=3
matched_files=2
bytes={}
lines={}
matched_lines=3

matches:
app.ax | bytes={} | lines={} | matched_lines=2
    api/guide.md | bytes={} | lines={} | matched_lines=1
",
        app_text.len() + guide_text.len() + notes_text.len(),
        line_count(app_text) + line_count(guide_text) + line_count(notes_text),
        app_text.len(),
        line_count(app_text),
        guide_text.len(),
        line_count(guide_text),
    );
    assert_eq!(rendered, expected);
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
fn run_runtime_missing_file_read_json_with_ai_exposes_host_rule_id() {
    let temp = TempDir::new("run-runtime-missing-file-read-ai");
    let missing_path = temp.join("missing-input.txt");
    let missing_literal = missing_path.to_string_lossy().replace('\\', "/");
    let input = temp.write(
        "missing_file_read.ax",
        &format!(
            "\
fn main() -> i32 {{
    let text: string = fs_read_to_string(\"{missing_literal}\");
    println(text);
    return 0;
}}
"
        ),
    );

    let output = run_axc([
        OsStr::new("run"),
        input.as_os_str(),
        OsStr::new("--json"),
        OsStr::new("--ai"),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_clean_stderr(&output);

    let diagnostics: Value =
        serde_json::from_slice(&output.stdout).expect("run diagnostics output should be JSON");
    let diagnostics = diagnostics
        .as_array()
        .expect("run diagnostics output should be an array");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["code"], Value::from("R0061"));
    assert_eq!(
        diagnostics[0]["ai"]["rule_id"],
        Value::from("readable_file_path_required")
    );
    assert_eq!(
        diagnostics[0]["ai"]["repair_goal"],
        Value::from(
            "Pass an existing readable file path before reading file contents or file metadata."
        )
    );
    assert_eq!(
        diagnostics[0]["suggestion"],
        Value::from(
            "pass an existing readable text file path or guard with `fs_exists(path)` first"
        )
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
        &project_sources_with_shared_foundation(&["src/main.ax"]),
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
        &project_sources_with_shared_foundation(&["lib/report.ax", "src/main.ax"]),
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

#[test]
fn context_overview_matches_snapshot() {
    let output = run_axc([
        OsStr::new("context"),
        OsStr::new("overview"),
        OsStr::new("examples/project_module_smoke"),
        OsStr::new("--json"),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "context overview should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let stdout = normalize_text(&string_output(&output.stdout));
    assert_eq!(
        stdout,
        snapshot("context_overview_project_module_smoke.json")
    );
}

#[test]
fn context_boundaries_matches_snapshot() {
    let output = run_axc([
        OsStr::new("context"),
        OsStr::new("boundaries"),
        OsStr::new("examples/project_workspace_search_report"),
        OsStr::new("--json"),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "context boundaries should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let stdout = normalize_text(&string_output(&output.stdout));
    assert_eq!(
        stdout,
        snapshot("context_boundaries_project_workspace_search_report.json")
    );
}

#[test]
fn context_topology_matches_snapshot() {
    let output = run_axc([
        OsStr::new("context"),
        OsStr::new("topology"),
        OsStr::new("examples/project_module_smoke"),
        OsStr::new("--json"),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "context topology should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let stdout = normalize_text(&string_output(&output.stdout));
    assert_eq!(
        stdout,
        snapshot("context_topology_project_module_smoke.json")
    );
}

#[test]
fn context_flow_matches_snapshot() {
    let output = run_axc([
        OsStr::new("context"),
        OsStr::new("flow"),
        OsStr::new("examples/project_workspace_search_report"),
        OsStr::new("--json"),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "context flow should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let stdout = normalize_text(&string_output(&output.stdout));
    assert_eq!(
        stdout,
        snapshot("context_flow_project_workspace_search_report.json")
    );
}

#[test]
fn context_symbol_matches_snapshot() {
    let output = run_axc([
        OsStr::new("context"),
        OsStr::new("symbol"),
        OsStr::new("examples/project_workspace_search_report"),
        OsStr::new("lib.file_search.search_path"),
        OsStr::new("--json"),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "context symbol should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let stdout = normalize_text(&string_output(&output.stdout));
    assert_eq!(
        stdout,
        snapshot("context_symbol_project_workspace_search_report_search_path.json")
    );
}

#[test]
fn context_impact_matches_snapshot() {
    let output = run_axc([
        OsStr::new("context"),
        OsStr::new("impact"),
        OsStr::new("examples/project_workspace_search_report"),
        OsStr::new("lib.file_search.search_path"),
        OsStr::new("--json"),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "context impact should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let stdout = normalize_text(&string_output(&output.stdout));
    assert_eq!(
        stdout,
        snapshot("context_impact_project_workspace_search_report_search_path.json")
    );
}

#[test]
fn context_evidence_matches_snapshot() {
    let output = run_axc([
        OsStr::new("context"),
        OsStr::new("evidence"),
        OsStr::new("examples/project_workspace_search_report"),
        OsStr::new("lib.file_search.search_path"),
        OsStr::new("--json"),
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "context evidence should succeed\nstdout:\n{}\nstderr:\n{}",
        string_output(&output.stdout),
        string_output(&output.stderr)
    );
    assert_clean_stderr(&output);

    let stdout = normalize_text(&string_output(&output.stdout));
    assert_eq!(
        stdout,
        snapshot("context_evidence_project_workspace_search_report_search_path.json")
    );
}
