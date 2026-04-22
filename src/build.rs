use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::hir::Program as HirProgram;
use crate::mir::Program as MirProgram;
use crate::source::SourceFile;

const BUILD_MANIFEST_FILE: &str = "build-manifest.json";
const SOURCE_COPY_FILE: &str = "source.ax";
const HIR_FILE: &str = "program.hir.json";
const MIR_FILE: &str = "program.mir.json";

#[derive(Debug, Clone)]
pub struct BuildOptions {
    pub out_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct BuildResult {
    pub manifest_path: PathBuf,
    pub manifest: BuildManifest,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildManifest {
    pub schema_version: u32,
    pub target_name: String,
    pub entry_file: String,
    pub output_dir: String,
    pub backend: BuildBackend,
    pub artifacts: BuildArtifacts,
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
    pub planned_executable: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable: Option<String>,
}

pub fn default_output_dir(input_file: &Path) -> Result<PathBuf, String> {
    let cwd = std::env::current_dir()
        .map_err(|error| format!("failed to resolve current directory: {error}"))?;
    Ok(cwd.join("build").join(target_name(input_file)?))
}

pub fn target_name(input_file: &Path) -> Result<String, String> {
    let Some(stem) = input_file.file_stem().and_then(|stem| stem.to_str()) else {
        return Err(format!(
            "could not derive a build target name from {}",
            input_file.display()
        ));
    };

    if stem.is_empty() {
        return Err(format!(
            "could not derive a build target name from {}",
            input_file.display()
        ));
    }

    Ok(stem.to_string())
}

pub fn build_program(
    source: &SourceFile,
    hir: &HirProgram,
    mir: &MirProgram,
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

    let target_name = target_name(source.path())?;
    let manifest = BuildManifest {
        schema_version: 2,
        target_name: target_name.clone(),
        entry_file: source.display_path(),
        output_dir: options.out_dir.display().to_string(),
        backend: BuildBackend {
            kind: "native".to_string(),
            status: "pending".to_string(),
            entrypoint: "main".to_string(),
        },
        artifacts: BuildArtifacts {
            source_copy: SOURCE_COPY_FILE.to_string(),
            hir_json: HIR_FILE.to_string(),
            mir_json: MIR_FILE.to_string(),
            planned_executable: format!("bin/{}{}", target_name, executable_suffix()),
            executable: None,
        },
        notes: vec![
            "This build currently emits frontend and midend stable artifacts only.".to_string(),
            "Native executable emission will be added in the future backend stage.".to_string(),
        ],
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

fn executable_suffix() -> &'static str {
    if cfg!(windows) { ".exe" } else { "" }
}

#[cfg(test)]
mod tests {
    use super::{default_output_dir, target_name};
    use std::path::Path;

    #[test]
    fn derives_target_name_from_input_path() {
        assert_eq!(
            target_name(Path::new("examples/hello.ax")).expect("target name should exist"),
            "hello"
        );
    }

    #[test]
    fn default_output_dir_uses_build_root_and_target_name() {
        let output_dir = default_output_dir(Path::new("examples/hello.ax"))
            .expect("default output dir should resolve");
        let rendered = output_dir.display().to_string().replace('\\', "/");
        assert!(rendered.ends_with("/build/hello"));
    }
}
