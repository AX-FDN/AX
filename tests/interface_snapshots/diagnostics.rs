use std::ffi::OsStr;
use std::fs;

use serde_json::Value;

use super::support::*;

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
fn diagnostics_missing_input_json_with_ai_exposes_source_input_contract() {
    let temp = TempDir::new("diagnostics-missing-input-ai");
    let input = temp.join("missing.ax");

    let base = run_axc([OsStr::new("check"), input.as_os_str(), OsStr::new("--json")]);
    assert_eq!(base.status.code(), Some(1));
    assert_clean_stderr(&base);
    let base_diagnostics: Value =
        serde_json::from_slice(&base.stdout).expect("base input error should be JSON");
    let base_diagnostics = base_diagnostics
        .as_array()
        .expect("base input error should be an array");
    assert_eq!(base_diagnostics.len(), 1);
    assert_eq!(base_diagnostics[0]["code"], "I0001");
    assert!(base_diagnostics[0].get("ai").is_none());

    let output = run_axc([
        OsStr::new("check"),
        input.as_os_str(),
        OsStr::new("--json"),
        OsStr::new("--ai"),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_clean_stderr(&output);

    let diagnostics: Value =
        serde_json::from_slice(&output.stdout).expect("AI input error should be JSON");
    let diagnostics = diagnostics
        .as_array()
        .expect("AI input error should be an array");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["code"], "I0001");
    assert_eq!(
        diagnostics[0]["ai"]["rule_id"],
        "input_target_must_be_readable"
    );
    assert_eq!(diagnostics[0]["ai"]["layer"], "source_input");
    assert_eq!(diagnostics[0]["ai"]["ai_action"], "fix_input_or_config");
    assert_eq!(diagnostics[0]["ai"]["safe_to_edit"], true);
    assert_eq!(
        json_string_array(&diagnostics[0]["ai"]["validation"], "input validation"),
        vec!["axc check <target>".to_string()]
    );
}

#[test]
fn diagnostics_unexpected_character_json_with_ai_matches_snapshot() {
    let temp = TempDir::new("diagnostics-lexer-ai");
    let input = temp.write(
        "unexpected_character.ax",
        "fn main() -> i32 {\n    @\n    return 0;\n}\n",
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
    let placeholder = "<input>/unexpected_character.ax".to_string();
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
        snapshot("diagnostics_unexpected_character_ai.json")
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
fn diagnostics_result_propagation_json_with_ai_matches_snapshot() {
    let temp = TempDir::new("diagnostics-result-propagation-ai");
    let input = temp.write(
        "result_propagation_outside_result.ax",
        "\
enum Result<T, E> {
    Ok(T),
    Err(E),
}

impl<T, E> Result<T, E> {
    fn ok(value: T) -> Result<T, E> {
        return Result.Ok(value);
    }

    fn err(error: E) -> Result<T, E> {
        return Result.Err(error);
    }
}

fn read_score() -> Result<i32, string> {
    return Result.ok(7);
}

fn score_or_default() -> i32 {
    let score: i32 = read_score()?;
    return score;
}

fn main() -> i32 {
    return score_or_default();
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
    let placeholder = "<input>/result_propagation_outside_result.ax".to_string();
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
        snapshot("diagnostics_result_propagation_ai.json")
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
