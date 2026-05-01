use std::ffi::OsStr;

use super::support::*;

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
