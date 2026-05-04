use std::path::Path;
use std::process::Command;

pub enum LinkOutcome {
    Skipped,
    ToolchainMissing {
        compiler: String,
    },
    Failed {
        compiler: String,
        exit_code: String,
        stderr: String,
    },
    Built {
        compiler: String,
    },
}

pub fn link_executable(llvm_ir_path: &Path, executable_path: &Path) -> LinkOutcome {
    link_with_clang(llvm_ir_path, executable_path)
}

pub fn link_if_enabled(llvm_ir_path: &Path, executable_path: &Path) -> LinkOutcome {
    if !linking_enabled() {
        return LinkOutcome::Skipped;
    }

    link_with_clang(llvm_ir_path, executable_path)
}

fn link_with_clang(llvm_ir_path: &Path, executable_path: &Path) -> LinkOutcome {
    let compiler = std::env::var("AX_LLVM_CLANG")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "clang".to_string());

    if let Some(parent) = executable_path.parent()
        && let Err(error) = std::fs::create_dir_all(parent)
    {
        return LinkOutcome::Failed {
            compiler,
            exit_code: "io-error".to_string(),
            stderr: format!(
                "failed to create executable output directory {}: {error}",
                parent.display()
            ),
        };
    }

    let output = match Command::new(&compiler)
        .arg(llvm_ir_path)
        .arg("-o")
        .arg(executable_path)
        .output()
    {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return LinkOutcome::ToolchainMissing { compiler };
        }
        Err(error) => {
            return LinkOutcome::Failed {
                compiler,
                exit_code: "spawn-error".to_string(),
                stderr: error.to_string(),
            };
        }
    };

    if output.status.success() {
        return LinkOutcome::Built { compiler };
    }

    LinkOutcome::Failed {
        compiler,
        exit_code: output
            .status
            .code()
            .map(|code| code.to_string())
            .unwrap_or_else(|| "terminated-by-signal".to_string()),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    }
}

fn linking_enabled() -> bool {
    std::env::var("AX_LLVM_AOT_LINK").is_ok_and(|value| {
        let normalized = value.trim().to_ascii_lowercase();
        matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
    })
}
