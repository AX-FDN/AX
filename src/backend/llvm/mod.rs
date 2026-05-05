use std::fs;
use std::path::Path;

use crate::mir::Program as MirProgram;

mod abi;
mod diagnostic;
mod ir;
mod linking;
mod monomorph;
mod runtime;
mod symbols;
mod toolchain;

const GENERATED_DIR: &str = "generated";
const LLVM_IR_FILE: &str = "main.ll";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlvmAotStatus {
    Unsupported,
    LoweringUnsupported,
    IrGenerated,
    Built,
    LinkSkipped,
    ToolchainMissing,
    ToolchainFailed,
}

impl LlvmAotStatus {
    pub fn as_manifest_status(self) -> &'static str {
        match self {
            LlvmAotStatus::Unsupported => "unsupported",
            LlvmAotStatus::LoweringUnsupported => "unsupported",
            LlvmAotStatus::IrGenerated
            | LlvmAotStatus::LinkSkipped
            | LlvmAotStatus::ToolchainMissing
            | LlvmAotStatus::ToolchainFailed => "ir_generated",
            LlvmAotStatus::Built => "built",
        }
    }

    pub fn blocker_code(self) -> Option<&'static str> {
        match self {
            LlvmAotStatus::LoweringUnsupported => Some("AOT2001"),
            LlvmAotStatus::LinkSkipped => Some("AOT1000"),
            LlvmAotStatus::ToolchainMissing => Some("AOT1001"),
            LlvmAotStatus::ToolchainFailed => Some("AOT1002"),
            LlvmAotStatus::Unsupported | LlvmAotStatus::IrGenerated | LlvmAotStatus::Built => None,
        }
    }

    pub fn blocker_category(self) -> Option<&'static str> {
        match self {
            LlvmAotStatus::LoweringUnsupported => Some("llvm_lowering"),
            LlvmAotStatus::LinkSkipped
            | LlvmAotStatus::ToolchainMissing
            | LlvmAotStatus::ToolchainFailed => Some("toolchain"),
            LlvmAotStatus::Unsupported | LlvmAotStatus::IrGenerated | LlvmAotStatus::Built => None,
        }
    }

    pub fn blocker_message(self) -> Option<&'static str> {
        match self {
            LlvmAotStatus::LoweringUnsupported => Some(
                "LLVM IR generation was skipped because the current MIR uses features outside the LLVM AOT v0 subset",
            ),
            LlvmAotStatus::LinkSkipped => Some(
                "LLVM IR was generated, but executable linking is disabled; use --emit exe or set AX_LLVM_AOT_LINK=1 to let axc build try clang",
            ),
            LlvmAotStatus::ToolchainMissing => {
                Some("LLVM IR was generated, but clang was not found for executable linking")
            }
            LlvmAotStatus::ToolchainFailed => Some(
                "LLVM IR was generated, but clang failed while linking the executable artifact",
            ),
            LlvmAotStatus::Unsupported | LlvmAotStatus::IrGenerated | LlvmAotStatus::Built => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlvmAotLinkMode {
    Environment,
    Force,
    Skip,
}

#[derive(Debug, Clone)]
pub struct LlvmAotOptions<'a> {
    pub out_dir: &'a Path,
    pub target_name: &'a str,
    pub executable_suffix: &'a str,
    pub link_mode: LlvmAotLinkMode,
}

#[derive(Debug, Clone)]
pub struct LlvmAotResult {
    pub status: LlvmAotStatus,
    pub llvm_ir_artifact: Option<String>,
    pub executable_artifact: Option<String>,
    pub notes: Vec<String>,
}

pub fn build(mir: &MirProgram, options: LlvmAotOptions<'_>) -> Result<LlvmAotResult, String> {
    let module = match ir::render_program(mir) {
        Ok(module) => module,
        Err(reasons) => {
            return Ok(LlvmAotResult {
                status: LlvmAotStatus::LoweringUnsupported,
                llvm_ir_artifact: None,
                executable_artifact: None,
                notes: reasons
                    .into_iter()
                    .map(|reason| format!("LLVM AOT v0 skipped: {reason}"))
                    .collect(),
            });
        }
    };

    let generated_dir = options.out_dir.join(GENERATED_DIR);
    fs::create_dir_all(&generated_dir).map_err(|error| {
        format!(
            "failed to create LLVM generated artifact directory {}: {error}",
            generated_dir.display()
        )
    })?;

    let llvm_ir_path = generated_dir.join(LLVM_IR_FILE);
    fs::write(&llvm_ir_path, module).map_err(|error| {
        format!(
            "failed to write LLVM IR artifact {}: {error}",
            llvm_ir_path.display()
        )
    })?;

    let llvm_ir_artifact = format!("{GENERATED_DIR}/{LLVM_IR_FILE}");
    let executable_artifact = format!("bin/{}{}", options.target_name, options.executable_suffix);
    let executable_path = options.out_dir.join(&executable_artifact);
    let mut notes = vec![
        "LLVM AOT v0 generated textual LLVM IR for the current single-file MIR subset.".to_string(),
    ];

    if options.link_mode == LlvmAotLinkMode::Skip {
        notes.push(
            "LLVM executable linking was not requested; --emit ir/--no-link keeps IR as the build artifact."
                .to_string(),
        );
        return Ok(LlvmAotResult {
            status: LlvmAotStatus::IrGenerated,
            llvm_ir_artifact: Some(llvm_ir_artifact),
            executable_artifact: None,
            notes,
        });
    }

    let link_plan = linking::NativeLinkPlan::single_ir_executable(
        options.target_name,
        &llvm_ir_path,
        &executable_path,
        options.link_mode,
    );
    let link_outcome = linking::execute(&link_plan);

    match link_outcome {
        toolchain::LinkOutcome::Skipped => {
            notes.push(
                "LLVM executable linking was skipped; use --emit exe or set AX_LLVM_AOT_LINK=1 to try clang."
                    .to_string(),
            );
            Ok(LlvmAotResult {
                status: LlvmAotStatus::LinkSkipped,
                llvm_ir_artifact: Some(llvm_ir_artifact),
                executable_artifact: None,
                notes,
            })
        }
        toolchain::LinkOutcome::ToolchainMissing { compiler } => {
            notes.push(format!(
                "LLVM executable linking requested, but `{compiler}` was not found."
            ));
            Ok(LlvmAotResult {
                status: LlvmAotStatus::ToolchainMissing,
                llvm_ir_artifact: Some(llvm_ir_artifact),
                executable_artifact: None,
                notes,
            })
        }
        toolchain::LinkOutcome::Failed {
            compiler,
            exit_code,
            stderr,
        } => {
            notes.push(format!(
                "LLVM executable linking with `{compiler}` failed with exit code {exit_code}: {stderr}"
            ));
            Ok(LlvmAotResult {
                status: LlvmAotStatus::ToolchainFailed,
                llvm_ir_artifact: Some(llvm_ir_artifact),
                executable_artifact: None,
                notes,
            })
        }
        toolchain::LinkOutcome::Built { compiler } => {
            notes.push(format!(
                "LLVM executable artifact was linked with `{compiler}`."
            ));
            Ok(LlvmAotResult {
                status: LlvmAotStatus::Built,
                llvm_ir_artifact: Some(llvm_ir_artifact),
                executable_artifact: Some(executable_artifact.replace('\\', "/")),
                notes,
            })
        }
    }
}
