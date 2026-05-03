use super::{
    AotReadiness, AotReadinessInput, assess_aot_readiness, build_input_from_project,
    default_output_dir, target_name_from_file,
};
use crate::frontend::analyze;
use crate::project::resolve_input;
use crate::source::SourceFile;
use std::path::{Path, PathBuf};

fn readiness_for(source_text: &str, input: AotReadinessInput<'_>) -> AotReadiness {
    let source = SourceFile::anonymous(source_text);
    let output = analyze(&source);
    assert!(
        output.diagnostics.is_empty(),
        "test source should analyze cleanly: {:?}",
        output.diagnostics
    );
    assess_aot_readiness(&output.program, input)
}

fn blocker_codes(readiness: &AotReadiness) -> Vec<&str> {
    readiness
        .blockers
        .iter()
        .map(|blocker| blocker.code.as_str())
        .collect()
}

#[test]
fn derives_target_name_from_input_path() {
    assert_eq!(
        target_name_from_file(Path::new("examples/hello.ax")).expect("target name should exist"),
        "hello"
    );
}

#[test]
fn default_output_dir_uses_build_root_and_target_name() {
    let output_dir = default_output_dir("hello").expect("default output dir should resolve");
    let rendered = output_dir.display().to_string().replace('\\', "/");
    assert!(rendered.ends_with("/build/hello"));
}

#[test]
fn packages_shared_sibling_support_sources_under_external_prefix() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let resolved = resolve_input(
        repo_root
            .join("examples")
            .join("project_workspace_search_report"),
    )
    .expect("project input should resolve");
    let project = resolved
        .project
        .as_ref()
        .expect("project metadata should be available");

    let build_input = build_input_from_project(&resolved.source, project)
        .expect("build input should package project sources");
    let project_sources = build_input
        .project_sources
        .expect("project sources artifact should exist");
    let relative_paths = project_sources
        .files
        .into_iter()
        .map(|file| file.relative_path)
        .collect::<Vec<_>>();

    assert!(relative_paths.contains(&"external/foundation/cli.ax".to_string()));
    assert!(relative_paths.contains(&"external/foundation/file_kind.ax".to_string()));
    assert!(relative_paths.contains(&"external/foundation/report.ax".to_string()));
    assert!(relative_paths.contains(&"external/foundation/search.ax".to_string()));
    assert!(relative_paths.contains(&"external/foundation/text.ax".to_string()));
    assert!(relative_paths.contains(&"external/foundation/workspace.ax".to_string()));
    assert!(relative_paths.contains(&"lib/file_search.ax".to_string()));
    assert!(relative_paths.contains(&"src/main.ax".to_string()));
}

#[test]
fn aot_readiness_marks_single_file_stdio_as_core_candidate() {
    let readiness = readiness_for(
        "\
fn main() -> i32 {
println(1);
return 0;
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert_eq!(
        readiness.required_backend_features,
        vec![
            "functions".to_string(),
            "host_stdio".to_string(),
            "i32_values".to_string()
        ]
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
    assert_eq!(
        readiness.blockers[0].resolution.agent_action,
        "explain_unsupported"
    );
    assert!(!readiness.blockers[0].resolution.source_edit_safe);
    assert_eq!(
        readiness.blockers[0].ai.rule_id,
        "aot_native_emission_pending"
    );
    assert_eq!(readiness.blockers[0].ai.layer, "aot_readiness");
}

#[test]
fn aot_readiness_allows_string_literals_without_full_string_runtime() {
    let readiness = readiness_for(
        "\
fn main() -> i32 {
println(\"hello\");
return 0;
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert_eq!(
        readiness.required_backend_features,
        vec![
            "functions".to_string(),
            "host_stdio".to_string(),
            "i32_values".to_string(),
            "string_literals".to_string()
        ]
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_basic_string_values_without_full_string_runtime() {
    let readiness = readiness_for(
        "\
fn identity(value: string) -> string {
return value;
}

fn main() -> i32 {
let text: string = identity(\"hello\");
println(text);
return 0;
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert_eq!(
        readiness.required_backend_features,
        vec![
            "functions".to_string(),
            "host_stdio".to_string(),
            "i32_values".to_string(),
            "string_literals".to_string(),
            "string_values".to_string()
        ]
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_string_len_without_full_string_runtime() {
    let readiness = readiness_for(
        "\
fn main() -> i32 {
let text: string = \"hello\";
if (text == \"hello\") {
return string_len(text);
}
return len(\"fallback\");
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert_eq!(
        readiness.required_backend_features,
        vec![
            "control_flow".to_string(),
            "functions".to_string(),
            "i32_values".to_string(),
            "string_len".to_string(),
            "string_literals".to_string(),
            "string_values".to_string()
        ]
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_string_runtime_v0_without_full_string_runtime() {
    let readiness = readiness_for(
        "\
fn main() -> i32 {
let message: string = \"count=\" + to_string(7) + \", ok=\" + to_string(true);
println(message);
return string_len(message);
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert_eq!(
        readiness.required_backend_features,
        vec![
            "bool_values".to_string(),
            "functions".to_string(),
            "host_stdio".to_string(),
            "i32_values".to_string(),
            "string_concat".to_string(),
            "string_len".to_string(),
            "string_literals".to_string(),
            "string_values".to_string(),
            "to_string_values".to_string()
        ]
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_string_predicates_v0_without_full_string_runtime() {
    let readiness = readiness_for(
        "\
fn main() -> i32 {
let text: string = \"AX compiler\";
if (string_contains(text, \"comp\") && string_starts_with(text, \"AX\") && string_ends_with(text, \"iler\")) {
return 17;
}
return 1;
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert_eq!(
        readiness.required_backend_features,
        vec![
            "control_flow".to_string(),
            "functions".to_string(),
            "i32_values".to_string(),
            "string_literals".to_string(),
            "string_predicates".to_string(),
            "string_values".to_string()
        ]
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_string_trim_v0_without_full_string_runtime() {
    let readiness = readiness_for(
        "\
fn main() -> i32 {
let text: string = string_trim(\"  AX compiler\\n\");
println(text);
return string_len(text);
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert_eq!(
        readiness.required_backend_features,
        vec![
            "functions".to_string(),
            "host_stdio".to_string(),
            "i32_values".to_string(),
            "string_len".to_string(),
            "string_literals".to_string(),
            "string_trim".to_string(),
            "string_values".to_string()
        ]
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_string_replace_v0_without_full_string_runtime() {
    let readiness = readiness_for(
        "\
fn main() -> i32 {
let text: string = string_replace(\"AX compiler AX\", \"AX\", \"A\");
println(text);
return string_len(text);
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert_eq!(
        readiness.required_backend_features,
        vec![
            "functions".to_string(),
            "host_stdio".to_string(),
            "i32_values".to_string(),
            "string_len".to_string(),
            "string_literals".to_string(),
            "string_replace".to_string(),
            "string_values".to_string()
        ]
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_string_split_lines_v0_without_full_string_runtime() {
    let readiness = readiness_for(
        "\
fn main() -> i32 {
let lines: [string] = string_split_lines(\"alpha\\nbeta\\ngamma\\n\");
println(lines[1]);
return len(lines);
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert_eq!(
        readiness.required_backend_features,
        vec![
            "arrays".to_string(),
            "functions".to_string(),
            "host_stdio".to_string(),
            "i32_values".to_string(),
            "slices".to_string(),
            "string_literals".to_string(),
            "string_split_lines".to_string(),
            "string_values".to_string()
        ]
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_for_in_over_string_split_lines_v0() {
    let readiness = readiness_for(
        "\
fn main() -> i32 {
let lines: [string] = string_split_lines(\"alpha\\nbeta\\ngamma\\n\");
let mut total: i32 = 0;
for (let line: string in lines) {
println(line);
total = total + string_len(line);
}
return total;
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert_eq!(
        readiness.required_backend_features,
        vec![
            "for_in".to_string(),
            "functions".to_string(),
            "host_stdio".to_string(),
            "i32_values".to_string(),
            "slices".to_string(),
            "string_len".to_string(),
            "string_literals".to_string(),
            "string_split_lines".to_string(),
            "string_values".to_string()
        ]
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_fixed_array_read_v0() {
    let readiness = readiness_for(
        "\
fn pick(values: [i32; 4], index: i32) -> i32 {
return values[index];
}

fn main() -> i32 {
let values: [i32; 4] = [3, 5, 8, 13];
return values[0] + pick(values, len(values) - 1);
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert_eq!(
        readiness.required_backend_features,
        vec![
            "arrays".to_string(),
            "functions".to_string(),
            "i32_values".to_string()
        ]
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_fixed_array_write_v0() {
    let readiness = readiness_for(
        "\
fn main() -> i32 {
let mut values: [i32; 2] = [1, 2];
values[0] = 3;
return values[0];
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert_eq!(
        readiness.required_backend_features,
        vec![
            "array_writes".to_string(),
            "arrays".to_string(),
            "functions".to_string(),
            "i32_values".to_string()
        ]
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_fixed_array_formatter_v0() {
    let readiness = readiness_for(
        "\
fn main() -> i32 {
let values: [i32; 3] = [1, 2, 3];
println(values);
return string_len(to_string(values));
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert!(
        readiness
            .required_backend_features
            .contains(&"arrays".to_string())
    );
    assert!(
        readiness
            .required_backend_features
            .contains(&"host_stdio".to_string())
    );
    assert!(
        readiness
            .required_backend_features
            .contains(&"to_string_values".to_string())
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_fixed_array_for_in_v0() {
    let readiness = readiness_for(
        "\
fn main() -> i32 {
let values: [i32; 3] = [2, 4, 6];
let mut total: i32 = 0;
for (let value: i32 in values) {
total = total + value;
}
return total;
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert!(
        readiness
            .required_backend_features
            .contains(&"for_in".to_string())
    );
    assert!(
        readiness
            .required_backend_features
            .contains(&"arrays".to_string())
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_slice_range_for_in_v0() {
    let readiness = readiness_for(
        "\
fn main() -> i32 {
let values: [i32; 5] = [1, 2, 3, 4, 5];
let mut total: i32 = 0;
for (let value: i32 in values[1:4]) {
total = total + value;
}
return total;
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert!(
        readiness
            .required_backend_features
            .contains(&"for_in".to_string())
    );
    assert!(
        readiness
            .required_backend_features
            .contains(&"slices".to_string())
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_slice_range_read_v0() {
    let readiness = readiness_for(
        "\
fn sum_pair(values: [i32]) -> i32 {
return values[0] + values[1];
}

fn main() -> i32 {
let values: [i32; 5] = [1, 2, 3, 4, 5];
let middle: [i32] = values[1:4];
return len(middle) + middle[0] + middle[2] + sum_pair(values[2:4]) + sum_pair(values);
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert!(
        readiness
            .required_backend_features
            .contains(&"slices".to_string())
    );
    assert!(
        readiness
            .required_backend_features
            .contains(&"arrays".to_string())
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_struct_read_v0() {
    let readiness = readiness_for(
        "\
struct Point {
x: i32,
y: i32,
}

fn main() -> i32 {
let point: Point = Point { x: 2, y: 5 };
return point.x + point.y;
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert_eq!(
        readiness.required_backend_features,
        vec![
            "functions".to_string(),
            "i32_values".to_string(),
            "structs".to_string()
        ]
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_struct_formatter_v0() {
    let readiness = readiness_for(
        "\
struct Summary {
count: i32,
ready: bool,
label: string,
}

fn main() -> i32 {
let summary: Summary = Summary { ready: true, label: \"ok\", count: 3 };
println(summary);
return string_len(to_string(summary));
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert!(
        readiness
            .required_backend_features
            .contains(&"structs".to_string())
    );
    assert!(
        readiness
            .required_backend_features
            .contains(&"host_stdio".to_string())
    );
    assert!(
        readiness
            .required_backend_features
            .contains(&"to_string_values".to_string())
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_unit_enum_v0() {
    let readiness = readiness_for(
        "\
enum Flag {
Off,
On,
}

fn choose(flag: Flag) -> i32 {
if (flag == Flag.On) {
return 9;
}
return 2;
}

fn main() -> i32 {
let flag: Flag = Flag.On;
return choose(flag);
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert_eq!(
        readiness.required_backend_features,
        vec![
            "control_flow".to_string(),
            "enums".to_string(),
            "functions".to_string(),
            "i32_values".to_string()
        ]
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_unit_enum_match_v0() {
    let readiness = readiness_for(
        "\
enum Flag {
Off,
On,
}

fn score(flag: Flag) -> i32 {
match (flag) {
Flag.On => {
return 9;
}
Flag.Off => {
return 2;
}
}
}

fn main() -> i32 {
return score(Flag.On);
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert_eq!(
        readiness.required_backend_features,
        vec![
            "enum_patterns".to_string(),
            "enums".to_string(),
            "functions".to_string(),
            "i32_values".to_string(),
            "match_statements".to_string()
        ]
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_payload_enum_match_v0() {
    let readiness = readiness_for(
        "\
enum Maybe {
None,
Some(i32),
}

fn score(value: Maybe) -> i32 {
match (value) {
Maybe.Some(number) => {
return number;
}
Maybe.None => {
return 0;
}
}
}

fn main() -> i32 {
let value: Maybe = Maybe.Some(7);
return score(value);
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert_eq!(
        readiness.required_backend_features,
        vec![
            "enum_patterns".to_string(),
            "enums".to_string(),
            "functions".to_string(),
            "i32_values".to_string(),
            "match_statements".to_string(),
            "pattern_bindings".to_string(),
            "payload_enum_patterns".to_string(),
            "payload_enums".to_string()
        ]
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_enum_to_string_v0() {
    let readiness = readiness_for(
        "\
enum Status {
Code(i32),
Flag(bool),
Label(string),
Done,
}

fn main() -> i32 {
let code: Status = Status.Code(7);
let flag: Status = Status.Flag(true);
let label: Status = Status.Label(\"ok\");
let done: Status = Status.Done;
return string_len(to_string(code)) + string_len(to_string(flag)) + string_len(to_string(label)) + string_len(to_string(done));
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert_eq!(
        readiness.required_backend_features,
        vec![
            "bool_values".to_string(),
            "enums".to_string(),
            "functions".to_string(),
            "i32_values".to_string(),
            "payload_enums".to_string(),
            "string_len".to_string(),
            "string_literals".to_string(),
            "string_values".to_string(),
            "to_string_values".to_string()
        ]
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_direct_enum_print_v0() {
    let readiness = readiness_for(
        "\
enum Status {
Code(i32),
Flag(bool),
Label(string),
Done,
}

fn main() -> i32 {
let code: Status = Status.Code(7);
let flag: Status = Status.Flag(true);
let label: Status = Status.Label(\"ok\");
let done: Status = Status.Done;
println(code);
println(flag);
println(label);
println(done);
return 58;
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert_eq!(
        readiness.required_backend_features,
        vec![
            "bool_values".to_string(),
            "enums".to_string(),
            "functions".to_string(),
            "host_stdio".to_string(),
            "i32_values".to_string(),
            "payload_enums".to_string(),
            "string_literals".to_string(),
            "string_values".to_string()
        ]
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_match_expression_v0() {
    let readiness = readiness_for(
        "\
enum Maybe {
None,
Some(i32),
}

fn score(value: Maybe) -> i32 {
return match (value) {
Maybe.Some(number) => {
let bonus: i32 = 1;
number + bonus
},
Maybe.None => 0,
};
}

fn main() -> i32 {
let value: Maybe = Maybe.Some(7);
return score(value);
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert!(
        readiness
            .required_backend_features
            .contains(&"match_expressions".to_string())
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_result_option_instance_v0() {
    let readiness = readiness_for(
        "\
enum Option<T> {
None,
Some(T),
}

enum Result<T, E> {
Ok(T),
Err(E),
}

fn option_or(value: Option<i32>, fallback: i32) -> i32 {
return match (value) { Option.Some(found) => found, Option.None => fallback };
}

fn value_or_zero(result: Result<i32, string>) -> i32 {
return match (result) { Result.Ok(value) => value, Result.Err(_) => 0 };
}

fn main() -> i32 {
let present: Option<i32> = Option.Some(5);
let ok: Result<i32, string> = Result.Ok(7);
return option_or(present, 0) + value_or_zero(ok);
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert!(
        readiness
            .required_backend_features
            .contains(&"result_values".to_string())
    );
    assert!(
        readiness
            .required_backend_features
            .contains(&"option_values".to_string())
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_generic_enum_formatter_and_print_v0() {
    let readiness = readiness_for(
        "\
enum Option<T> {
None,
Some(T),
}

enum Result<T, E> {
Ok(T),
Err(E),
}

fn main() -> i32 {
let present: Option<i32> = Option.Some(5);
let missing: Option<i32> = Option.None;
let ok: Result<i32, string> = Result.Ok(7);
let err: Result<i32, string> = Result.Err(\"bad\");
println(to_string(present));
println(missing);
println(to_string(ok));
println(err);
return string_len(to_string(present)) + string_len(to_string(missing)) + string_len(to_string(ok)) + string_len(to_string(err));
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert!(
        readiness
            .required_backend_features
            .contains(&"generic_enums".to_string())
    );
    assert!(
        readiness
            .required_backend_features
            .contains(&"generic_type_instances".to_string())
    );
    assert!(
        readiness
            .required_backend_features
            .contains(&"option_values".to_string())
    );
    assert!(
        readiness
            .required_backend_features
            .contains(&"result_values".to_string())
    );
    assert!(
        readiness
            .required_backend_features
            .contains(&"to_string_values".to_string())
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_result_try_v0() {
    let readiness = readiness_for(
        "\
enum Result<T, E> {
Ok(T),
Err(E),
}

fn parse(text: string) -> Result<i32, string> {
if (text == \"ok\") {
return Result.Ok(7);
}
return Result.Err(\"bad\");
}

fn render_score(text: string) -> Result<string, string> {
let score: i32 = parse(text)?;
return Result.Ok(\"score=\" + to_string(score));
}

fn main() -> i32 {
return match (render_score(\"ok\")) { Result.Ok(text) => string_len(text), Result.Err(message) => string_len(message) };
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert!(
        readiness
            .required_backend_features
            .contains(&"result_propagation".to_string())
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_range_patterns_v0() {
    let readiness = readiness_for(
        "\
fn classify(status: i32) -> i32 {
return match (status) {
200..=299 => 2,
400..=499 => 4,
_ => 0,
};
}

fn main() -> i32 {
return classify(204);
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert!(
        readiness
            .required_backend_features
            .contains(&"range_patterns".to_string())
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_or_patterns_v0() {
    let readiness = readiness_for(
        "\
enum Mode {
Check,
Run,
Build,
}

fn score(mode: Mode) -> i32 {
return match (mode) {
Mode.Check | Mode.Run => 1,
Mode.Build => 2,
};
}

fn main() -> i32 {
return score(Mode.Check);
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert!(
        readiness
            .required_backend_features
            .contains(&"or_patterns".to_string())
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_match_guards_v0() {
    let readiness = readiness_for(
        "\
enum Token {
Number(i32),
End,
}

fn score(token: Token) -> i32 {
return match (token) {
Token.Number(value) if value > 9 => value,
Token.Number(_) => 1,
Token.End => 0,
};
}

fn main() -> i32 {
return score(Token.Number(12));
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert!(
        readiness
            .required_backend_features
            .contains(&"match_guards".to_string())
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_struct_patterns_v0() {
    let readiness = readiness_for(
        "\
struct Point {
x: i32,
y: i32,
}

fn score(point: Point) -> i32 {
return match (point) {
Point { x, y } => x + y,
};
}

fn main() -> i32 {
return score(Point { x: 20, y: 22 });
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert!(
        readiness
            .required_backend_features
            .contains(&"struct_patterns".to_string())
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_allows_struct_field_write_v0() {
    let readiness = readiness_for(
        "\
struct Point {
x: i32,
}

fn main() -> i32 {
let mut point: Point = Point { x: 1 };
point.x = 3;
return point.x;
}
",
        AotReadinessInput {
            is_project: false,
            has_local_path_packages: false,
            package_lock_status: None,
        },
    );

    assert!(readiness.single_file_core_candidate);
    assert_eq!(
        readiness.required_backend_features,
        vec![
            "functions".to_string(),
            "i32_values".to_string(),
            "struct_writes".to_string(),
            "structs".to_string()
        ]
    );
    assert_eq!(blocker_codes(&readiness), vec!["AOT0001"]);
}

#[test]
fn aot_readiness_reports_project_package_and_generic_blockers() {
    let readiness = readiness_for(
        "\
fn id<T>(value: T) -> T {
return value;
}

fn main() -> i32 {
return id(1);
}
",
        AotReadinessInput {
            is_project: true,
            has_local_path_packages: true,
            package_lock_status: Some("missing"),
        },
    );

    assert!(!readiness.single_file_core_candidate);
    assert!(
        readiness
            .required_backend_features
            .contains(&"generic_functions".to_string())
    );
    assert_eq!(
        blocker_codes(&readiness),
        vec!["AOT0001", "AOT0101", "AOT0102", "AOT0103", "AOT0201"]
    );
    let lock_blocker = readiness
        .blockers
        .iter()
        .find(|blocker| blocker.code == "AOT0103")
        .expect("package lock blocker should be present");
    assert_eq!(lock_blocker.resolution.agent_action, "verify_lockfile");
    assert_eq!(
        lock_blocker.resolution.recommended_command.as_deref(),
        Some("axc lock <project> --check")
    );
    assert_eq!(lock_blocker.ai.rule_id, "aot_package_lock_must_be_current");
    assert_eq!(lock_blocker.ai.ai_action, "verify_lockfile");
    assert_eq!(
        lock_blocker.ai.validation,
        vec![
            "axc lock <project> --check".to_string(),
            "axc build <project> --json".to_string()
        ]
    );
}
