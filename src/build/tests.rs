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
