use std::ffi::OsStr;

use serde_json::Value;

use super::support::*;

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
    assert_eq!(diagnostics[0]["ai"]["layer"], Value::from("interpreter"));
    assert_eq!(
        diagnostics[0]["ai"]["ai_action"],
        Value::from("fix_runtime_input")
    );
    assert_eq!(diagnostics[0]["ai"]["safe_to_edit"], Value::from(false));
    assert_eq!(
        json_string_array(&diagnostics[0]["ai"]["validation"], "runtime validation"),
        vec![
            "axc check <target>".to_string(),
            "axc run <target>".to_string()
        ]
    );
    assert_eq!(
        diagnostics[0]["suggestion"],
        Value::from(
            "pass an existing readable text file path or guard with `fs_exists(path)` first"
        )
    );
}
