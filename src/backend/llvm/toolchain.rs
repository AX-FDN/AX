use std::path::{Path, PathBuf};
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

    let primary_exit_code = command_exit_code(&output);
    let primary_stderr = command_stderr(&output);
    if let Some(fallback) = link_with_windows_gnu_fallback(
        &compiler,
        llvm_ir_path,
        executable_path,
        &primary_exit_code,
        &primary_stderr,
    ) {
        return fallback;
    }

    LinkOutcome::Failed {
        compiler,
        exit_code: primary_exit_code,
        stderr: primary_stderr,
    }
}

fn link_with_windows_gnu_fallback(
    compiler: &str,
    llvm_ir_path: &Path,
    executable_path: &Path,
    primary_exit_code: &str,
    primary_stderr: &str,
) -> Option<LinkOutcome> {
    if !cfg!(windows) {
        return None;
    }

    let sysroot = windows_gnu_self_contained_dir()?;
    let crt2 = sysroot.join("crt2.o");
    if !crt2.is_file() {
        return None;
    }

    let output = match Command::new(compiler)
        .arg("-target")
        .arg("x86_64-w64-windows-gnu")
        .arg("-fuse-ld=lld")
        .arg("-nostdlib")
        .arg(&crt2)
        .arg(llvm_ir_path)
        .arg("-o")
        .arg(executable_path)
        .arg("-L")
        .arg(&sysroot)
        .arg("-lmingw32")
        .arg("-lgcc")
        .arg("-lgcc_eh")
        .arg("-lmoldname")
        .arg("-lmingwex")
        .arg("-lmsvcrt")
        .arg("-lkernel32")
        .output()
    {
        Ok(output) => output,
        Err(_) => return None,
    };

    if output.status.success() {
        return Some(LinkOutcome::Built {
            compiler: format!("{compiler} (windows-gnu self-contained fallback)"),
        });
    }

    let fallback_exit_code = command_exit_code(&output);
    let fallback_stderr = command_stderr(&output);
    Some(LinkOutcome::Failed {
        compiler: compiler.to_string(),
        exit_code: fallback_exit_code,
        stderr: format!(
            "primary clang link failed with exit code {primary_exit_code}: {primary_stderr}\nwindows-gnu self-contained fallback failed with exit code {}: {}",
            command_exit_code(&output),
            fallback_stderr
        ),
    })
}

fn windows_gnu_self_contained_dir() -> Option<PathBuf> {
    if let Some(path) = std::env::var("AX_LLVM_MINGW_SYSROOT")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
    {
        return Some(path);
    }

    let rustup_home = std::env::var("RUSTUP_HOME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("USERPROFILE")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .map(|home| PathBuf::from(home).join(".rustup"))
        })?;

    let toolchains = rustup_home.join("toolchains");
    let preferred = toolchains
        .join("stable-x86_64-pc-windows-gnu")
        .join("lib")
        .join("rustlib")
        .join("x86_64-pc-windows-gnu")
        .join("lib")
        .join("self-contained");
    if preferred.is_dir() {
        return Some(preferred);
    }

    let entries = std::fs::read_dir(toolchains).ok()?;
    for entry in entries.flatten() {
        let candidate = entry
            .path()
            .join("lib")
            .join("rustlib")
            .join("x86_64-pc-windows-gnu")
            .join("lib")
            .join("self-contained");
        if candidate.is_dir() {
            return Some(candidate);
        }
    }

    None
}

fn command_exit_code(output: &std::process::Output) -> String {
    output
        .status
        .code()
        .map(|code| code.to_string())
        .unwrap_or_else(|| "terminated-by-signal".to_string())
}

fn command_stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).trim().to_string()
}

fn linking_enabled() -> bool {
    std::env::var("AX_LLVM_AOT_LINK").is_ok_and(|value| {
        let normalized = value.trim().to_ascii_lowercase();
        matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
    })
}
