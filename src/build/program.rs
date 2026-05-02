use std::fs;

use crate::ast::Program as AstProgram;
use crate::backend::llvm::{self, LlvmAotOptions, LlvmAotResult, LlvmAotStatus};
use crate::hir::Program as HirProgram;
use crate::mir::Program as MirProgram;
use crate::source::SourceFile;

use super::readiness::assess_aot_readiness;
use super::*;

pub fn build_program(
    source: &SourceFile,
    program: &AstProgram,
    hir: &HirProgram,
    mir: &MirProgram,
    input: &BuildInput,
    options: &BuildOptions,
) -> Result<BuildResult, String> {
    fs::create_dir_all(&options.out_dir).map_err(|error| {
        format!(
            "failed to create build output directory {}: {error}",
            options.out_dir.display()
        )
    })?;

    let bin_dir = options.out_dir.join("bin");
    fs::create_dir_all(&bin_dir).map_err(|error| {
        format!(
            "failed to create build bin directory {}: {error}",
            bin_dir.display()
        )
    })?;

    let source_copy_path = options.out_dir.join(SOURCE_COPY_FILE);
    fs::write(&source_copy_path, source.text()).map_err(|error| {
        format!(
            "failed to write build source copy {}: {error}",
            source_copy_path.display()
        )
    })?;

    let hir_path = options.out_dir.join(HIR_FILE);
    let hir_text = serde_json::to_string_pretty(hir)
        .map_err(|error| format!("failed to serialize HIR for build output: {error}"))?;
    fs::write(&hir_path, format!("{hir_text}\n"))
        .map_err(|error| format!("failed to write build HIR {}: {error}", hir_path.display()))?;

    let mir_path = options.out_dir.join(MIR_FILE);
    let mir_text = serde_json::to_string_pretty(mir)
        .map_err(|error| format!("failed to serialize MIR for build output: {error}"))?;
    fs::write(&mir_path, format!("{mir_text}\n"))
        .map_err(|error| format!("failed to write build MIR {}: {error}", mir_path.display()))?;

    if let Some(project_manifest) = &input.project_manifest {
        let project_manifest_path = options.out_dir.join(&project_manifest.file_name);
        fs::write(&project_manifest_path, &project_manifest.text).map_err(|error| {
            format!(
                "failed to write copied project manifest {}: {error}",
                project_manifest_path.display()
            )
        })?;
    }

    if let Some(project_sources) = &input.project_sources {
        let project_sources_dir = options.out_dir.join(&project_sources.dir_name);
        fs::create_dir_all(&project_sources_dir).map_err(|error| {
            format!(
                "failed to create copied project sources directory {}: {error}",
                project_sources_dir.display()
            )
        })?;

        for project_source in &project_sources.files {
            let copied_path = project_sources_dir.join(&project_source.relative_path);
            if let Some(parent) = copied_path.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    format!(
                        "failed to create copied project source directory {}: {error}",
                        parent.display()
                    )
                })?;
            }
            fs::write(&copied_path, &project_source.text).map_err(|error| {
                format!(
                    "failed to write copied project source {}: {error}",
                    copied_path.display()
                )
            })?;
        }
    }

    let mut backend = BuildBackend {
        kind: "native".to_string(),
        status: "pending".to_string(),
        entrypoint: "main".to_string(),
    };
    let mut aot_readiness = assess_aot_readiness(
        program,
        AotReadinessInput {
            is_project: input.project_manifest.is_some(),
            has_local_path_packages: !input.local_path_packages.is_empty(),
            package_lock_status: input
                .package_graph_readiness
                .as_ref()
                .map(|readiness| readiness.lock_status.as_str()),
        },
    );
    let mut artifacts = BuildArtifacts {
        source_copy: SOURCE_COPY_FILE.to_string(),
        hir_json: HIR_FILE.to_string(),
        mir_json: MIR_FILE.to_string(),
        llvm_ir: None,
        project_manifest: input
            .project_manifest
            .as_ref()
            .map(|artifact| artifact.file_name.clone()),
        project_sources_dir: input
            .project_sources
            .as_ref()
            .map(|artifact| artifact.dir_name.clone()),
        project_sources: input.project_sources.as_ref().map(|artifact| {
            artifact
                .files
                .iter()
                .map(|file| file.relative_path.clone())
                .collect()
        }),
        planned_executable: format!("bin/{}{}", input.target_name, executable_suffix()),
        executable: None,
    };
    let mut notes = vec![
        "This build currently emits frontend and midend stable artifacts.".to_string(),
        "LLVM AOT v0 may emit a textual LLVM IR artifact for the current single-file MIR subset."
            .to_string(),
    ];

    let llvm_result = if input.project_manifest.is_none() && program.source_units.len() == 1 {
        llvm::build(
            mir,
            LlvmAotOptions {
                out_dir: &options.out_dir,
                target_name: &input.target_name,
                executable_suffix: executable_suffix(),
            },
        )?
    } else {
        LlvmAotResult {
            status: LlvmAotStatus::Unsupported,
            llvm_ir_artifact: None,
            executable_artifact: None,
            notes: Vec::new(),
        }
    };
    apply_llvm_aot_result(
        &llvm_result,
        &mut backend,
        &mut aot_readiness,
        &mut artifacts,
        &mut notes,
    );

    let manifest = BuildManifest {
        schema_version: 7,
        target_name: input.target_name.clone(),
        entry_file: input.entry_file.clone(),
        output_dir: options.out_dir.display().to_string(),
        backend,
        aot_readiness,
        artifacts,
        local_path_packages: input.local_path_packages.clone(),
        package_graph_readiness: input.package_graph_readiness.clone(),
        notes,
    };

    let manifest_path = options.out_dir.join(BUILD_MANIFEST_FILE);
    let manifest_text = serde_json::to_string_pretty(&manifest)
        .map_err(|error| format!("failed to serialize build manifest: {error}"))?;
    fs::write(&manifest_path, format!("{manifest_text}\n")).map_err(|error| {
        format!(
            "failed to write build manifest {}: {error}",
            manifest_path.display()
        )
    })?;

    Ok(BuildResult {
        manifest_path,
        manifest,
    })
}

fn apply_llvm_aot_result(
    result: &LlvmAotResult,
    backend: &mut BuildBackend,
    readiness: &mut AotReadiness,
    artifacts: &mut BuildArtifacts,
    notes: &mut Vec<String>,
) {
    if result.status == LlvmAotStatus::Unsupported {
        return;
    }

    backend.kind = "llvm-aot".to_string();
    backend.status = result.status.as_manifest_status().to_string();

    artifacts.llvm_ir = result.llvm_ir_artifact.clone();
    artifacts.executable = result.executable_artifact.clone();

    readiness.stage = "Build-1 LLVM IR prototype".to_string();
    readiness.status = if result.status == LlvmAotStatus::Built {
        "built".to_string()
    } else {
        "ir_generated".to_string()
    };
    readiness.executable_emission = result.executable_artifact.is_some();
    readiness
        .blockers
        .retain(|blocker| blocker.code != "AOT0001");
    if let (Some(code), Some(message)) = (
        result.status.blocker_code(),
        result.status.blocker_message(),
    ) {
        readiness.blockers.push(AotReadinessBlocker::new(
            code,
            "toolchain",
            message,
            "Build-1",
        ));
    }
    readiness.recommended_next_steps = vec![
        "compare axc run with the generated LLVM AOT artifact for the same minimal MIR subset"
            .to_string(),
        "keep unsupported syntax in aot_readiness blockers until its MIR-to-LLVM lowering is explicit"
            .to_string(),
        "set AX_LLVM_AOT_LINK=1 and AX_LLVM_CLANG=<path> when validating executable linking"
            .to_string(),
    ];

    notes.extend(result.notes.iter().cloned());
}

fn executable_suffix() -> &'static str {
    if cfg!(windows) { ".exe" } else { "" }
}
