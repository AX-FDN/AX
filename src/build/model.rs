use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone)]
pub struct BuildOptions {
    pub out_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct BuildResult {
    pub manifest_path: PathBuf,
    pub manifest: BuildManifest,
}

#[derive(Debug, Clone)]
pub struct BuildInput {
    pub target_name: String,
    pub entry_file: String,
    pub project_manifest: Option<ProjectManifestArtifact>,
    pub project_sources: Option<ProjectSourcesArtifact>,
    pub local_path_packages: Vec<LocalPathPackageArtifact>,
    pub package_graph_readiness: Option<BuildPackageGraphReadiness>,
}

#[derive(Debug, Clone)]
pub struct ProjectManifestArtifact {
    pub file_name: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ProjectSourcesArtifact {
    pub dir_name: String,
    pub files: Vec<ProjectSourceArtifact>,
}

#[derive(Debug, Clone)]
pub struct ProjectSourceArtifact {
    pub relative_path: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalPathPackageArtifact {
    pub alias: String,
    pub root: String,
    pub manifest: String,
    pub source_count: usize,
    pub modules: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildPackageGraphReadiness {
    pub package_mode: String,
    pub reproducible: bool,
    pub aot_ready: bool,
    pub lock_status: String,
    pub risk_level: String,
    pub blocking_reasons: Vec<String>,
    pub recommended_commands: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct AotReadinessInput<'a> {
    pub is_project: bool,
    pub has_local_path_packages: bool,
    pub package_lock_status: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AotReadiness {
    pub schema_version: u32,
    pub stage: String,
    pub status: String,
    pub executable_emission: bool,
    pub planned_executable_artifact: bool,
    pub single_file_core_candidate: bool,
    pub required_backend_features: Vec<String>,
    pub blockers: Vec<AotReadinessBlocker>,
    pub recommended_next_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AotReadinessBlocker {
    pub code: String,
    pub category: String,
    pub message: String,
    pub required_stage: String,
    pub resolution: AotBlockerResolution,
}

impl AotReadinessBlocker {
    pub fn new(
        code: impl Into<String>,
        category: impl Into<String>,
        message: impl Into<String>,
        required_stage: impl Into<String>,
    ) -> Self {
        let code = code.into();
        Self {
            resolution: AotBlockerResolution::for_code(&code),
            code,
            category: category.into(),
            message: message.into(),
            required_stage: required_stage.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AotBlockerResolution {
    pub agent_action: String,
    pub source_edit_safe: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_command: Option<String>,
}

impl AotBlockerResolution {
    fn for_code(code: &str) -> Self {
        let (agent_action, source_edit_safe, recommended_command) = match code {
            "AOT0103" => ("verify_lockfile", false, Some("axc lock <project> --check")),
            "AOT1000" => (
                "enable_linking",
                false,
                Some("$env:AX_LLVM_AOT_LINK = \"1\""),
            ),
            "AOT1001" => (
                "configure_toolchain",
                false,
                Some("$env:AX_LLVM_CLANG = \"<path-to-clang>\""),
            ),
            "AOT1002" => (
                "inspect_toolchain_failure",
                false,
                Some("rerun axc build with AX_LLVM_AOT_LINK=1 and inspect clang stderr"),
            ),
            _ => ("explain_unsupported", false, None),
        };

        Self {
            agent_action: agent_action.to_string(),
            source_edit_safe,
            recommended_command: recommended_command.map(str::to_string),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildManifest {
    pub schema_version: u32,
    pub target_name: String,
    pub entry_file: String,
    pub output_dir: String,
    pub backend: BuildBackend,
    pub aot_readiness: AotReadiness,
    pub artifacts: BuildArtifacts,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub local_path_packages: Vec<LocalPathPackageArtifact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_graph_readiness: Option<BuildPackageGraphReadiness>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildBackend {
    pub kind: String,
    pub status: String,
    pub entrypoint: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildArtifacts {
    pub source_copy: String,
    pub hir_json: String,
    pub mir_json: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llvm_ir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_manifest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_sources_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_sources: Option<Vec<String>>,
    pub planned_executable: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable: Option<String>,
}
