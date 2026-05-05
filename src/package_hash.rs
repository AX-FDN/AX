use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

pub fn hash_package_dir(path: impl AsRef<Path>) -> Result<String, String> {
    let root = path.as_ref();
    let metadata = fs::metadata(root)
        .map_err(|error| format!("failed to access package path {}: {error}", root.display()))?;
    if !metadata.is_dir() {
        return Err(format!(
            "package hash input {} must be a directory",
            root.display()
        ));
    }

    let mut files = Vec::new();
    collect_package_files(root, root, &mut files)?;
    files.sort();

    let mut hasher = Sha256::new();
    for relative_path in files {
        let display_path = relative_path.to_string_lossy().replace('\\', "/");
        let absolute_path = root.join(&relative_path);
        let bytes = fs::read(&absolute_path)
            .map_err(|error| format!("failed to read {}: {error}", absolute_path.display()))?;
        hasher.update(display_path.as_bytes());
        hasher.update(b"\0");
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(b"\0");
        hasher.update(&bytes);
        hasher.update(b"\0");
    }

    Ok(format!("sha256:{}", hex_lower(&hasher.finalize())))
}

fn collect_package_files(
    root: &Path,
    current: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let mut entries = fs::read_dir(current)
        .map_err(|error| format!("failed to read directory {}: {error}", current.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            format!(
                "failed to read directory entry in {}: {error}",
                current.display()
            )
        })?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name == ".git" || file_name == "target" {
            continue;
        }
        let metadata = entry
            .metadata()
            .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
        if metadata.is_dir() {
            collect_package_files(root, &path, files)?;
        } else if metadata.is_file() {
            let relative_path = path.strip_prefix(root).map_err(|error| {
                format!(
                    "failed to derive relative package path for {}: {error}",
                    path.display()
                )
            })?;
            files.push(relative_path.to_path_buf());
        }
    }

    Ok(())
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(hex_digit(byte >> 4));
        out.push(hex_digit(byte & 0x0f));
    }
    out
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + value - 10) as char,
        _ => unreachable!("hex digit nibble must be <= 15"),
    }
}

#[cfg(test)]
mod tests {
    use super::hash_package_dir;
    use std::fs;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn hashes_package_directories_deterministically() {
        let root = repo_root().join("target").join("package-hash-test");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).expect("package src should exist");
        fs::write(root.join("AX.toml"), "manifest_version = 1\n").expect("manifest should exist");
        fs::write(
            root.join("src").join("lib.ax"),
            "fn value() -> i32 { return 1; }\n",
        )
        .expect("source should exist");

        let first = hash_package_dir(&root).expect("hash should render");
        let second = hash_package_dir(&root).expect("hash should render");

        assert!(first.starts_with("sha256:"));
        assert_eq!(first, second);

        let _ = fs::remove_dir_all(&root);
    }
}
