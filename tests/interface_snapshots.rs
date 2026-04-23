use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

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

fn assert_clean_stderr(output: &Output) {
    let stderr = string_output(&output.stderr);
    assert!(
        stderr.trim().is_empty(),
        "expected empty stderr, got:\n{}",
        stderr
    );
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
