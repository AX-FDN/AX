use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::package_hash::hash_package_dir;
use crate::registry::RegistryPackageVersion;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageCacheInstall {
    Installed(CachedPackage),
    Skipped { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedPackage {
    pub source_dir: PathBuf,
    pub package_dir: PathBuf,
    pub checksum: String,
}

pub fn default_ax_home() -> PathBuf {
    if let Some(path) = env::var_os("AX_HOME") {
        return PathBuf::from(path);
    }
    if let Some(path) = env::var_os("USERPROFILE") {
        return PathBuf::from(path).join(".ax");
    }
    if let Some(path) = env::var_os("HOME") {
        return PathBuf::from(path).join(".ax");
    }
    env::temp_dir().join(".ax")
}

pub fn cached_registry_package_dir(
    registry_name: &str,
    package_name: &str,
    version: &str,
) -> PathBuf {
    default_ax_home()
        .join("packages")
        .join(registry_name)
        .join(package_name)
        .join(version)
}

pub fn install_registry_package_to_cache(
    registry_name: &str,
    package_name: &str,
    version: &RegistryPackageVersion,
) -> Result<PackageCacheInstall, String> {
    if is_placeholder_rev(&version.source.rev) {
        return Ok(PackageCacheInstall::Skipped {
            reason: format!(
                "PX0107: registry package `{package_name}` version `{}` has placeholder rev metadata; publish AX-PKG and pin a real commit before cache install",
                version.version
            ),
        });
    }
    if is_placeholder_checksum(&version.checksum) {
        return Ok(PackageCacheInstall::Skipped {
            reason: format!(
                "PX0108: registry package `{package_name}` version `{}` has placeholder checksum metadata; run `axc pkg hash` on the package source and update registry metadata",
                version.version
            ),
        });
    }

    let ax_home = default_ax_home();
    let git_dir = ax_home
        .join("git")
        .join(sanitize_cache_component(&version.source.url));
    let package_dir = cached_registry_package_dir(registry_name, package_name, &version.version);

    ensure_git_checkout(&version.source.url, &version.source.rev, &git_dir)?;

    let source_path = version.source.path.as_deref().unwrap_or(".");
    let source_dir = git_dir.join(source_path);
    let metadata = fs::metadata(&source_dir).map_err(|error| {
        format!(
            "PX0109: package `{package_name}` source path {} is not available after checkout: {error}",
            source_dir.display()
        )
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "PX0109: package `{package_name}` source path {} must be a directory",
            source_dir.display()
        ));
    }

    let actual_checksum = hash_package_dir(&source_dir)?;
    if actual_checksum != version.checksum {
        return Err(format!(
            "PX0110: registry package `{package_name}` checksum mismatch: expected `{}`, found `{actual_checksum}`",
            version.checksum
        ));
    }

    if package_dir.exists() {
        fs::remove_dir_all(&package_dir).map_err(|error| {
            format!(
                "failed to replace cached package directory {}: {error}",
                package_dir.display()
            )
        })?;
    }
    copy_dir_recursively(&source_dir, &package_dir)?;

    Ok(PackageCacheInstall::Installed(CachedPackage {
        source_dir,
        package_dir,
        checksum: actual_checksum,
    }))
}

fn ensure_git_checkout(url: &str, rev: &str, git_dir: &Path) -> Result<(), String> {
    if git_dir.join(".git").is_dir() {
        run_git(&[
            "-C".to_string(),
            path_arg(git_dir),
            "fetch".to_string(),
            "--all".to_string(),
            "--tags".to_string(),
        ])?;
    } else {
        let parent = git_dir.parent().ok_or_else(|| {
            format!(
                "failed to resolve parent directory for git cache {}",
                git_dir.display()
            )
        })?;
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create git cache directory {}: {error}",
                parent.display()
            )
        })?;
        let clone_args = vec![
            "clone".to_string(),
            "--no-checkout".to_string(),
            url.to_string(),
            path_arg(git_dir),
        ];
        if let Err(first_error) = run_git(&clone_args) {
            let Some(fallback_url) = github_https_to_ssh(url) else {
                return Err(first_error);
            };
            if git_dir.exists() {
                fs::remove_dir_all(git_dir).map_err(|error| {
                    format!(
                        "failed to remove partial git cache {} after clone failure: {error}",
                        git_dir.display()
                    )
                })?;
            }
            let fallback_args = vec![
                "clone".to_string(),
                "--no-checkout".to_string(),
                fallback_url,
                path_arg(git_dir),
            ];
            run_git(&fallback_args).map_err(|fallback_error| {
                format!("{first_error}\nfallback clone also failed: {fallback_error}")
            })?;
        }
    }
    run_git(&[
        "-C".to_string(),
        path_arg(git_dir),
        "checkout".to_string(),
        "--detach".to_string(),
        rev.to_string(),
    ])?;
    Ok(())
}

fn run_git(args: &[String]) -> Result<(), String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|error| format!("PX0111: failed to execute git: {error}"))?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if stderr.is_empty() { stdout } else { stderr };
    Err(format!("PX0111: git command failed: {detail}"))
}

fn copy_dir_recursively(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|error| {
        format!(
            "failed to create package cache directory {}: {error}",
            destination.display()
        )
    })?;

    for entry in fs::read_dir(source).map_err(|error| {
        format!(
            "failed to read package source {}: {error}",
            source.display()
        )
    })? {
        let entry = entry.map_err(|error| {
            format!(
                "failed to read package source entry in {}: {error}",
                source.display()
            )
        })?;
        let file_name = entry.file_name();
        let file_name_text = file_name.to_string_lossy();
        if file_name_text == ".git" || file_name_text == "target" {
            continue;
        }
        let source_path = entry.path();
        let destination_path = destination.join(&file_name);
        let metadata = entry
            .metadata()
            .map_err(|error| format!("failed to inspect {}: {error}", source_path.display()))?;
        if metadata.is_dir() {
            copy_dir_recursively(&source_path, &destination_path)?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &destination_path).map_err(|error| {
                format!(
                    "failed to copy package file {} to {}: {error}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn is_placeholder_rev(rev: &str) -> bool {
    rev.chars().all(|ch| ch == '0')
}

fn is_placeholder_checksum(checksum: &str) -> bool {
    checksum.starts_with("sha256:preview-")
}

fn sanitize_cache_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn github_https_to_ssh(url: &str) -> Option<String> {
    let rest = url.strip_prefix("https://github.com/")?;
    Some(format!("git@github.com:{rest}"))
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::github_https_to_ssh;
    use super::{PackageCacheInstall, install_registry_package_to_cache};
    use crate::registry::{RegistryPackageSource, RegistryPackageVersion};

    #[test]
    fn skips_placeholder_registry_metadata_before_git() {
        let version = RegistryPackageVersion {
            version: "0.1.0".to_string(),
            source: RegistryPackageSource {
                kind: "git".to_string(),
                url: "https://github.com/AX-FDN/AX-PKG.git".to_string(),
                rev: "0000000000000000000000000000000000000000".to_string(),
                path: Some("packages/text_tools".to_string()),
            },
            checksum: "sha256:preview-text-tools".to_string(),
            modules: vec!["text_tools.normalize".to_string()],
        };

        let result = install_registry_package_to_cache("ax", "text_tools", &version)
            .expect("placeholder metadata should produce a structured skip");

        assert!(matches!(result, PackageCacheInstall::Skipped { .. }));
    }

    #[test]
    fn converts_github_https_urls_to_ssh_fallbacks() {
        assert_eq!(
            github_https_to_ssh("https://github.com/AX-FDN/AX-PKG.git"),
            Some("git@github.com:AX-FDN/AX-PKG.git".to_string())
        );
    }
}
