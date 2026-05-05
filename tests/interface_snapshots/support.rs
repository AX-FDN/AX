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

pub(crate) fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub(crate) fn axc_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_axc"))
}

pub(crate) fn invocable_axc_binary() -> PathBuf {
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

pub(crate) fn remove_temp_file_best_effort(path: &Path) {
    for _ in 0..20 {
        match fs::remove_file(path) {
            Ok(()) => return,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
            Err(_) => thread::sleep(Duration::from_millis(100)),
        }
    }
}

pub(crate) fn write_temp_text(path: &Path, text: &str) {
    if path.extension().and_then(|value| value.to_str()) == Some("ps1") {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(text.as_bytes());
        fs::write(path, bytes).expect("failed to write temp file");
        return;
    }

    fs::write(path, text).expect("failed to write temp file");
}

pub(crate) fn snapshots_dir() -> PathBuf {
    repo_root().join("tests").join("snapshots")
}

pub(crate) fn normalize_text(text: &str) -> String {
    text.replace("\r\n", "\n")
}

pub(crate) fn snapshot(name: &str) -> String {
    normalize_text(
        &fs::read_to_string(snapshots_dir().join(name))
            .unwrap_or_else(|error| panic!("failed to read snapshot `{name}`: {error}")),
    )
}

pub(crate) fn normalize_build_manifest_value(manifest: &mut Value) {
    if let Some(path) = manifest
        .get("artifacts")
        .and_then(|artifacts| artifacts.get("planned_executable"))
        .and_then(Value::as_str)
    {
        let normalized = path.strip_suffix(".exe").unwrap_or(path).to_string();
        manifest["artifacts"]["planned_executable"] = Value::String(normalized);
    }
}

pub(crate) fn normalized_build_manifest_json(text: &str) -> String {
    let mut manifest: Value =
        serde_json::from_str(text).expect("build manifest snapshot should be valid JSON");
    normalize_build_manifest_value(&mut manifest);
    serde_json::to_string_pretty(&manifest).expect("build manifest JSON should serialize") + "\n"
}

pub(crate) fn run_axc<I, S>(args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    run_axc_with_removed_env(args, std::iter::empty::<&str>())
}

pub(crate) fn run_axc_with_removed_env<I, S, E>(args: I, env_names: E) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
    E: IntoIterator,
    E::Item: AsRef<OsStr>,
{
    let args: Vec<OsString> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_os_string())
        .collect();
    let env_names: Vec<OsString> = env_names
        .into_iter()
        .map(|name| name.as_ref().to_os_string())
        .collect();
    let binary = invocable_axc_binary();
    let mut last_busy_error = None;
    for _ in 0..20 {
        let mut command = Command::new(&binary);
        command.args(&args).current_dir(repo_root());
        for name in &env_names {
            command.env_remove(name);
        }
        match command.output() {
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

pub(crate) fn run_axc_with_env<I, S, E, K, V>(args: I, env_pairs: E) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
    E: IntoIterator<Item = (K, V)>,
    K: AsRef<OsStr>,
    V: AsRef<OsStr>,
{
    let args: Vec<OsString> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_os_string())
        .collect();
    let env_pairs: Vec<(OsString, OsString)> = env_pairs
        .into_iter()
        .map(|(name, value)| (name.as_ref().to_os_string(), value.as_ref().to_os_string()))
        .collect();
    let binary = invocable_axc_binary();
    let mut last_busy_error = None;
    for _ in 0..20 {
        let mut command = Command::new(&binary);
        command.args(&args).current_dir(repo_root());
        for (name, value) in &env_pairs {
            command.env(name, value);
        }
        match command.output() {
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

pub(crate) fn powershell_executable() -> &'static str {
    if cfg!(windows) {
        "powershell.exe"
    } else {
        "pwsh"
    }
}

pub(crate) fn run_powershell_script<I, S>(script: &Path, args: I) -> Output
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

pub(crate) struct TempDir {
    pub(crate) path: PathBuf,
}

impl TempDir {
    pub(crate) fn new(label: &str) -> Self {
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

    pub(crate) fn join(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }

    pub(crate) fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.join(name);
        write_temp_text(&path, text);
        path
    }

    pub(crate) fn write_nested(&self, name: &str, text: &str) -> PathBuf {
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

pub(crate) fn string_output(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("process output should be valid UTF-8")
}

pub(crate) fn read_json_file(path: &Path, label: &str) -> Value {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {label} `{}`: {error}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|error| {
        panic!(
            "failed to parse {label} `{}` as JSON: {error}",
            path.display()
        )
    })
}

pub(crate) fn powershell_literal_path(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "''")
}

pub(crate) fn json_string_array(value: &Value, label: &str) -> Vec<String> {
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

pub(crate) fn assert_json_f64(value: &Value, expected: f64, label: &str) {
    let actual = value
        .as_f64()
        .unwrap_or_else(|| panic!("{label} should be a JSON number"));
    assert!(
        (actual - expected).abs() < 1e-9,
        "{label} expected {expected}, got {actual}"
    );
}

pub(crate) fn assert_json_path_exists(value: &Value, label: &str) -> PathBuf {
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

pub(crate) fn export_repair_benchmark(temp: &TempDir, manifest_path: &Path) -> PathBuf {
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

pub(crate) fn export_repair_benchmark_with_context(
    temp: &TempDir,
    manifest_path: &Path,
) -> PathBuf {
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

pub(crate) fn export_smoke_repair_benchmark(temp: &TempDir) -> PathBuf {
    export_repair_benchmark(
        temp,
        &repo_root()
            .join("benchmarks")
            .join("repair-cases-smoke.json"),
    )
}

pub(crate) fn write_replay_wrapper(
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

pub(crate) fn write_stdout_only_runner(temp: &TempDir, name: &str) -> PathBuf {
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

pub(crate) fn write_silent_success_runner(temp: &TempDir, name: &str) -> PathBuf {
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

pub(crate) fn write_single_case_manifest(temp: &TempDir, name: &str) -> PathBuf {
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

pub(crate) fn write_single_runtime_case_manifest(temp: &TempDir, name: &str) -> PathBuf {
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

pub(crate) fn json_path_literal(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "\\\\")
}

pub(crate) fn write_single_project_case_manifest(
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

pub(crate) fn write_utf8_bom_candidate(path: &Path, text: &str) {
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(text.as_bytes());
    fs::write(path, bytes).expect("failed to write UTF-8 BOM candidate");
}

pub(crate) fn write_utf16le_bom_candidate(path: &Path, text: &str) {
    let mut bytes = vec![0xFF, 0xFE];
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    fs::write(path, bytes).expect("failed to write UTF-16 LE BOM candidate");
}

pub(crate) fn diagnostic_codes(value: &Value) -> Vec<String> {
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

pub(crate) fn assert_clean_stderr(output: &Output) {
    let stderr = string_output(&output.stderr);
    assert!(
        stderr.trim().is_empty(),
        "expected empty stderr, got:\n{}",
        stderr
    );
}

pub(crate) const SHARED_FOUNDATION_PROJECT_SOURCES: &[&str] = &[
    "external/foundation/cli.ax",
    "external/foundation/file_kind.ax",
    "external/foundation/report.ax",
    "external/foundation/search.ax",
    "external/foundation/text.ax",
    "external/foundation/workspace.ax",
];

pub(crate) const SHARED_STD_PROJECT_SOURCES: &[&str] = &[
    "external/std/bytes.ax",
    "external/std/cli.ax",
    "external/std/collections.ax",
    "external/std/encoding.ax",
    "external/std/env.ax",
    "external/std/fs.ax",
    "external/std/hash.ax",
    "external/std/http.ax",
    "external/std/json.ax",
    "external/std/net.ax",
    "external/std/option.ax",
    "external/std/path.ax",
    "external/std/process.ax",
    "external/std/report.ax",
    "external/std/result.ax",
    "external/std/text.ax",
    "external/std/workspace.ax",
];

pub(crate) fn project_sources_with_shared_foundation(extra: &[&str]) -> Vec<String> {
    SHARED_FOUNDATION_PROJECT_SOURCES
        .iter()
        .copied()
        .chain(extra.iter().copied())
        .map(str::to_string)
        .collect()
}

pub(crate) fn project_sources_with_shared_std(extra: &[&str]) -> Vec<String> {
    SHARED_STD_PROJECT_SOURCES
        .iter()
        .copied()
        .chain(extra.iter().copied())
        .map(str::to_string)
        .collect()
}

pub(crate) fn project_sources_path(root: &Path, relative: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    for segment in relative.split('/') {
        path.push(segment);
    }
    path
}

pub(crate) fn assert_project_example_build_sources(
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

pub(crate) fn assert_project_example_checks(example_path: &str) {
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

pub(crate) fn normalize_temp_output(text: &str, temp: &TempDir) -> String {
    let normalized = normalize_text(text).replace('\\', "/");
    let root = temp.path.display().to_string().replace('\\', "/");
    normalized.replace(&root, "<root>")
}

pub(crate) fn line_count(text: &str) -> i32 {
    text.lines().count() as i32
}

pub(crate) fn nonempty_line_count(text: &str) -> i32 {
    text.lines().filter(|line| !line.trim().is_empty()).count() as i32
}

pub(crate) fn heading_count(text: &str) -> i32 {
    text.lines()
        .filter(|line| line.trim().starts_with('#'))
        .count() as i32
}

pub(crate) fn action_item_count(text: &str) -> i32 {
    text.lines()
        .filter(|line| line.contains("TODO") || line.contains("FIXME"))
        .count() as i32
}

#[derive(Deserialize)]
pub(crate) struct RepairCaseManifest {
    pub(crate) cases: Vec<RepairCaseEntry>,
}

#[derive(Deserialize)]
pub(crate) struct RepairCaseEntry {
    pub(crate) id: String,
    pub(crate) file: String,
    pub(crate) project: Option<String>,
    pub(crate) diagnostic_command: Option<String>,
    pub(crate) expected_codes: Vec<String>,
    pub(crate) expected_ai_rule_ids: Vec<String>,
}

impl RepairCaseEntry {
    pub(crate) fn diagnostic_command(&self) -> &str {
        self.diagnostic_command.as_deref().unwrap_or("check")
    }

    pub(crate) fn diagnostic_target(&self) -> &str {
        self.project.as_deref().unwrap_or(self.file.as_str())
    }
}

pub(crate) fn load_repair_manifest(path: &str) -> RepairCaseManifest {
    let manifest_text = fs::read_to_string(repo_root().join(path)).unwrap_or_else(|error| {
        panic!("repair benchmark manifest `{path}` should be readable: {error}")
    });
    serde_json::from_str(&manifest_text).unwrap_or_else(|error| {
        panic!("repair benchmark manifest `{path}` should be valid: {error}")
    })
}

pub(crate) fn find_replay_candidate(root: &Path, case_id: &str) -> Option<PathBuf> {
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

pub(crate) fn assert_manifest_cases_keep_stable_diagnostics(manifest_path: &str) {
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
