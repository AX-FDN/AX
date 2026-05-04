use std::fs;
use std::path::PathBuf;

use crate::build::{AotReadinessInput, assess_aot_readiness};

use super::impact::build_impact_facts;
use super::*;

pub(super) fn build_evidence_facts(
    requested_symbol: &str,
    request_path: &Path,
    input: &ResolvedInput,
    program: &Program,
    symbol_catalog: &SymbolCatalog,
) -> Result<EvidenceFacts, String> {
    let impact_facts = build_impact_facts(requested_symbol, symbol_catalog)?;
    let expected_artifacts =
        build_context_expected_artifacts(request_path, input, requested_symbol);
    let local_package_lock = build_local_package_lock_fact(input.project.as_ref());
    let package_graph_readiness =
        build_package_graph_readiness_fact(input.project.as_ref(), local_package_lock.as_ref());
    Ok(EvidenceFacts {
        requested_symbol: requested_symbol.to_string(),
        resolved_symbol: impact_facts.resolved_symbol.clone(),
        affected_units: impact_facts
            .affected_units
            .iter()
            .map(|unit| unit.path.clone())
            .collect(),
        related_examples: build_related_examples(request_path, input, &impact_facts),
        related_tests: build_related_tests(input, &impact_facts),
        related_docs: build_related_docs(input, &impact_facts),
        related_benchmarks: build_related_benchmarks(input, &impact_facts),
        expected_artifacts,
        build_readiness: build_build_readiness_fact(input, program, local_package_lock.as_ref()),
        local_package_lock,
        package_graph_readiness,
    })
}

fn build_package_graph_readiness_fact(
    project: Option<&Project>,
    local_package_lock: Option<&ContextPackageLock>,
) -> Option<PackageGraphReadiness> {
    let project = project?;
    if project.local_path_dependencies().is_empty() {
        return None;
    }

    let lock_status = local_package_lock
        .map(|lock| lock.status)
        .unwrap_or("unavailable");
    let reproducible = lock_status == "current";
    let mut blocking_reasons = Vec::new();
    if !reproducible {
        blocking_reasons.push(format!(
            "local package graph is not reproducible because AX.lock status is `{lock_status}`"
        ));
    }

    let risk_level = if reproducible { "low" } else { "high" };
    Some(PackageGraphReadiness {
        package_mode: "local_path_v0",
        reproducible,
        aot_ready: reproducible,
        lock_status,
        risk_level,
        blocking_reasons,
        recommended_commands: vec![
            "axc lock <project> --check".to_string(),
            "axc check <project>".to_string(),
            "axc build <project>".to_string(),
        ],
    })
}

fn build_build_readiness_fact(
    input: &ResolvedInput,
    program: &Program,
    local_package_lock: Option<&ContextPackageLock>,
) -> BuildReadiness {
    let mut blocking_features =
        vec!["LLVM AOT v0 only covers a narrow single-file i32/bool MIR subset".to_string()];
    let mut notes = vec![
        "axc build emits source, HIR, MIR, build-manifest, and may emit LLVM IR for the current single-file MIR subset"
            .to_string(),
        "use axc run as the semantic reference while LLVM AOT v0 is validated".to_string(),
    ];

    if let Some(project) = input.project.as_ref() {
        blocking_features
            .push("project source graph still needs native AOT packaging and linking".to_string());
        notes.push(format!(
            "project `{}` is build-ready for artifacts, but project AOT linking is not implemented",
            project.target_name()
        ));

        if !project.local_path_dependencies().is_empty() {
            blocking_features.push(
                "local path package graph still needs future AOT package linking".to_string(),
            );
            notes.push(
                "run axc lock <project> --check before treating package input as reproducible"
                    .to_string(),
            );
        }
    }

    let has_local_path_packages = input
        .project
        .as_ref()
        .is_some_and(|project| !project.local_path_dependencies().is_empty());
    let aot_readiness = assess_aot_readiness(
        program,
        AotReadinessInput {
            is_project: input.project.is_some(),
            has_local_path_packages,
            package_lock_status: local_package_lock.map(|lock| lock.status),
        },
    );

    BuildReadiness {
        build_mode: "stable_artifacts_with_llvm_ir_v0",
        aot_status: "llvm_ir_prototype",
        executable_emission: false,
        planned_executable_artifact: true,
        blocking_features,
        notes,
        aot_readiness,
    }
}

pub(super) fn build_evidence_hints(
    command_target: &str,
    input: &ResolvedInput,
    related_tests: &[String],
    resolved_symbol: &str,
    expected_artifacts: &[String],
) -> EvidenceHints {
    let mut recommended_commands = vec![
        format!("axc check {command_target}"),
        format!("axc build {command_target}"),
        format!("axc context impact {command_target} {resolved_symbol} --json"),
        format!("axc context evidence {command_target} {resolved_symbol} --json"),
    ];

    if input.project.is_some() || command_target.starts_with("examples/") {
        recommended_commands.push(format!("axc run {command_target} -- <args...>"));
    }
    if input
        .project
        .as_ref()
        .is_some_and(|project| !project.local_path_dependencies().is_empty())
    {
        recommended_commands.push(format!("axc lock {command_target} --check"));
    }
    if related_tests
        .iter()
        .any(|path| path == "tests/interface_snapshots.rs")
    {
        recommended_commands.push("cargo test --test interface_snapshots context_".to_string());
    }

    EvidenceHints {
        recommended_commands,
        expected_artifacts: expected_artifacts.to_vec(),
    }
}

fn build_related_examples(
    request_path: &Path,
    input: &ResolvedInput,
    _impact_facts: &ImpactFacts,
) -> Vec<String> {
    let subject_path = evidence_subject_path(request_path, input);
    let mut related = BTreeSet::new();
    if subject_path.starts_with("examples/") {
        related.insert(subject_path.clone());
    }

    if let Some(project) = input.project.as_ref() {
        if let Some(stem) = project.target_name().strip_prefix("project_") {
            let file_candidate = format!("examples/{stem}.ax");
            let directory_candidate = format!("examples/{stem}");
            insert_repo_path_if_exists(&mut related, &file_candidate);
            insert_repo_path_if_exists(&mut related, &directory_candidate);
        }
    } else if let Some(stem) = request_path.file_stem().and_then(|value| value.to_str()) {
        let directory_candidate = format!("examples/project_{stem}");
        insert_repo_path_if_exists(&mut related, &directory_candidate);
    }

    related.into_iter().take(6).collect()
}

fn build_related_tests(input: &ResolvedInput, impact_facts: &ImpactFacts) -> Vec<String> {
    let mut related = BTreeSet::new();
    let tokens = build_evidence_search_tokens(input, impact_facts);
    if interface_snapshot_tests_match(&tokens) {
        related.insert("tests/interface_snapshots.rs".to_string());
    }

    related.into_iter().collect()
}

fn interface_snapshot_tests_match(tokens: &[String]) -> bool {
    let root = repo_root();
    let test_file = root.join("tests").join("interface_snapshots.rs");
    if file_matches_tokens(&test_file, tokens) {
        return true;
    }

    let modules_dir = root.join("tests").join("interface_snapshots");
    rust_test_module_matches_tokens(&modules_dir, tokens)
}

fn rust_test_module_matches_tokens(root: &Path, tokens: &[String]) -> bool {
    let Ok(entries) = fs::read_dir(root) else {
        return false;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if rust_test_module_matches_tokens(&path, tokens) {
                return true;
            }
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        if file_matches_tokens(&path, tokens) {
            return true;
        }
    }

    false
}

fn build_related_docs(input: &ResolvedInput, impact_facts: &ImpactFacts) -> Vec<String> {
    let mut related = BTreeSet::new();
    insert_repo_path_if_exists(&mut related, "架构上下文文档.md");
    insert_repo_path_if_exists(&mut related, "docs/README.md");
    insert_repo_path_if_exists(&mut related, "docs/feature-matrix.md");

    if input.project.is_some() {
        insert_repo_path_if_exists(&mut related, "docs/import-module-minimal-design.md");
    }
    if impact_facts
        .affected_units
        .iter()
        .any(|unit| !unit.host_boundary_classes.is_empty())
    {
        insert_repo_path_if_exists(&mut related, "docs/host-runtime-boundary.md");
    }

    related.into_iter().collect()
}

fn build_related_benchmarks(input: &ResolvedInput, impact_facts: &ImpactFacts) -> Vec<String> {
    let mut related = BTreeSet::new();
    let tokens = build_evidence_search_tokens(input, impact_facts);
    let benchmarks_dir = repo_root().join("benchmarks");
    collect_matching_repo_files(&benchmarks_dir, &tokens, &mut related);
    related.into_iter().take(8).collect()
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn evidence_subject_path(request_path: &Path, input: &ResolvedInput) -> String {
    input
        .project
        .as_ref()
        .map(|project| normalize_path(project.root_dir()))
        .unwrap_or_else(|| normalize_path(request_path))
}

fn build_context_expected_artifacts(
    request_path: &Path,
    input: &ResolvedInput,
    requested_symbol: &str,
) -> Vec<String> {
    let subject_key = context_subject_snapshot_key(request_path, input);
    let symbol_key = snapshot_symbol_key(requested_symbol);
    vec![
        format!("tests/snapshots/context_flow_{subject_key}.json"),
        format!("tests/snapshots/context_symbol_{subject_key}_{symbol_key}.json"),
        format!("tests/snapshots/context_impact_{subject_key}_{symbol_key}.json"),
        format!("tests/snapshots/context_evidence_{subject_key}_{symbol_key}.json"),
    ]
}

fn context_subject_snapshot_key(request_path: &Path, input: &ResolvedInput) -> String {
    let subject_path = input
        .project
        .as_ref()
        .map(|project| project.root_dir())
        .unwrap_or(request_path);
    subject_path
        .file_stem()
        .or_else(|| subject_path.file_name())
        .and_then(|value| value.to_str())
        .map(snapshot_key_fragment)
        .unwrap_or_else(|| "context".to_string())
}

fn snapshot_symbol_key(symbol: &str) -> String {
    snapshot_key_fragment(symbol.rsplit('.').next().unwrap_or(symbol))
}

fn snapshot_key_fragment(text: &str) -> String {
    let mut key = String::new();
    for character in text.chars() {
        if character.is_ascii_alphanumeric() {
            key.push(character.to_ascii_lowercase());
        } else {
            key.push('_');
        }
    }

    while key.contains("__") {
        key = key.replace("__", "_");
    }

    key.trim_matches('_').to_string()
}

fn build_evidence_search_tokens(input: &ResolvedInput, impact_facts: &ImpactFacts) -> Vec<String> {
    let mut tokens = BTreeSet::new();
    if let Some(project) = input.project.as_ref() {
        collect_search_token(&mut tokens, project.target_name());
    }
    collect_search_token(&mut tokens, &impact_facts.requested_symbol);
    collect_search_token(&mut tokens, &impact_facts.resolved_symbol);
    collect_search_token(
        &mut tokens,
        impact_facts
            .resolved_symbol
            .rsplit('.')
            .next()
            .unwrap_or(&impact_facts.resolved_symbol),
    );
    for unit in &impact_facts.affected_units {
        if let Some(stem) = Path::new(&unit.path)
            .file_stem()
            .and_then(|value| value.to_str())
        {
            collect_search_token(&mut tokens, stem);
        }
    }

    tokens.into_iter().collect()
}

fn collect_search_token(tokens: &mut BTreeSet<String>, raw: &str) {
    let lowered = raw.replace('\\', "/").to_ascii_lowercase();
    for fragment in [
        lowered.as_str(),
        lowered.strip_prefix("project_").unwrap_or(&lowered),
    ] {
        let candidate = fragment.trim_matches('/');
        if is_high_signal_evidence_token(candidate) {
            tokens.insert(candidate.to_string());
        }
    }
}

fn is_high_signal_evidence_token(token: &str) -> bool {
    token.len() >= 4
        && !token.starts_with("examples/")
        && (token.contains('_') || token.contains('.') || token.len() >= 12)
}

fn insert_repo_path_if_exists(paths: &mut BTreeSet<String>, relative_path: &str) {
    let absolute = repo_root().join(relative_path);
    if absolute.exists() {
        paths.insert(relative_path.replace('\\', "/"));
    }
}

fn collect_matching_repo_files(root: &Path, tokens: &[String], matches: &mut BTreeSet<String>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_matching_repo_files(&path, tokens, matches);
            continue;
        }
        let Some(relative_path) = repo_relative_path(&path) else {
            continue;
        };
        if file_matches_path_or_contents(&path, &relative_path, tokens) {
            matches.insert(relative_path);
        }
    }
}

fn file_matches_path_or_contents(path: &Path, relative_path: &str, tokens: &[String]) -> bool {
    path_string_matches_tokens(relative_path, tokens) || file_matches_tokens(path, tokens)
}

fn file_matches_tokens(path: &Path, tokens: &[String]) -> bool {
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    path_string_matches_tokens(&text, tokens)
}

fn path_string_matches_tokens(text: &str, tokens: &[String]) -> bool {
    let lowered = text.replace('\\', "/").to_ascii_lowercase();
    tokens.iter().any(|token| lowered.contains(token))
}

fn repo_relative_path(path: &Path) -> Option<String> {
    path.strip_prefix(repo_root()).ok().map(normalize_path)
}
