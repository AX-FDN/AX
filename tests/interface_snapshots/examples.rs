use std::ffi::OsStr;
use std::fs;

use super::support::*;

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
        "2\nalpha, beta\nbeta\n"
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
fn match_or_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/match_or.ax")]);
    assert_eq!(output.status.code(), Some(2));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "1\n1\n");
}

#[test]
fn match_guard_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/match_guard.ax")]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_text(&string_output(&output.stdout)),
        "12\n10\n1\n2\n"
    );
}

#[test]
fn match_range_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/match_range.ax")]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "2\n4\n5\n");
}

#[test]
fn payload_enum_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/payload_enum.ax")]);
    assert_eq!(output.status.code(), Some(7));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "7\n0\n-1\n");
}

#[test]
fn match_repair_triage_example_runs() {
    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/match_repair_triage.ax"),
    ]);
    assert_eq!(output.status.code(), Some(7));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_text(&string_output(&output.stdout)),
        "parser\n107\nsemantic\n222\nruntime\n900\nclean\n0\n"
    );
}

#[test]
fn methods_impl_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/methods_impl.ax")]);
    assert_eq!(output.status.code(), Some(12));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "9\n12\n");
}

#[test]
fn static_methods_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/static_methods.ax")]);
    assert_eq!(output.status.code(), Some(12));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "0\n12\n");
}

#[test]
fn generic_box_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/generic_box.ax")]);
    assert_eq!(output.status.code(), Some(13));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "ax\n13\n");
}

#[test]
fn generic_method_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/generic_method.ax")]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "ax:ok\n");
}

#[test]
fn generic_type_alias_example_runs() {
    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/generic_type_alias.ax"),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "ax\n");
}

#[test]
fn generic_functions_example_runs() {
    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/generic_functions.ax"),
    ]);
    assert_eq!(output.status.code(), Some(9));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "ax\n9\n");
}

#[test]
fn generic_result_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/generic_result.ax")]);
    assert_eq!(output.status.code(), Some(7));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_text(&string_output(&output.stdout)),
        "7\n0\nbad\n"
    );
}

#[test]
fn result_static_constructors_example_runs() {
    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/result_static_constructors.ax"),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_text(&string_output(&output.stdout)),
        "5\nmissing\nbad:no\n"
    );
}

#[test]
fn result_propagation_example_runs() {
    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/result_propagation.ax"),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_text(&string_output(&output.stdout)),
        "8\nbad:no\n"
    );
}

#[test]
fn string_match_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/string_match.ax")]);
    assert_eq!(output.status.code(), Some(3));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "1\n3\n");
}

#[test]
fn trait_impl_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/trait_impl.ax")]);
    assert_eq!(output.status.code(), Some(5));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "build\n");
}

#[test]
fn trait_bounds_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/trait_bounds.ax")]);
    assert_eq!(output.status.code(), Some(5));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "build\n");
}

#[test]
fn trait_multi_bounds_example_runs() {
    let output = run_axc([
        OsStr::new("run"),
        OsStr::new("examples/trait_multi_bounds.ax"),
    ]);
    assert_eq!(output.status.code(), Some(5));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "build:5\n");
}

#[test]
fn consts_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/consts.ax")]);
    assert_eq!(output.status.code(), Some(7));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_text(&string_output(&output.stdout)),
        "const-ready\n"
    );
}

#[test]
fn public_api_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/public_api.ax")]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(normalize_text(&string_output(&output.stdout)), "status=0\n");
}

#[test]
fn type_alias_example_runs() {
    let output = run_axc([OsStr::new("run"), OsStr::new("examples/type_alias.ax")]);
    assert_eq!(output.status.code(), Some(0));
    assert_clean_stderr(&output);
    assert_eq!(
        normalize_text(&string_output(&output.stdout)),
        "score total: 6\n"
    );
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
