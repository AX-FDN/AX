use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::TeachingLevel;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AiSessionEntry {
    diagnostic_code: String,
    rule_id: String,
    normalized_pattern: String,
    repeat_count: u32,
    last_teaching_level: TeachingLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AiSessionFile {
    version: u32,
    entries: BTreeMap<String, AiSessionEntry>,
}

const AI_SESSION_VERSION: u32 = 1;

impl Default for AiSessionFile {
    fn default() -> Self {
        Self {
            version: AI_SESSION_VERSION,
            entries: BTreeMap::new(),
        }
    }
}

pub(super) struct AiSession {
    entries: BTreeMap<String, AiSessionEntry>,
}

impl Default for AiSession {
    fn default() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }
}

impl AiSession {
    pub(super) fn bump(
        &mut self,
        diagnostic_code: &str,
        rule_id: &str,
        normalized_pattern: &str,
    ) -> u32 {
        let key = format!("{diagnostic_code}::{normalized_pattern}");
        let entry = self.entries.entry(key).or_insert_with(|| AiSessionEntry {
            diagnostic_code: diagnostic_code.to_string(),
            rule_id: rule_id.to_string(),
            normalized_pattern: normalized_pattern.to_string(),
            repeat_count: 0,
            last_teaching_level: TeachingLevel::L1,
        });
        entry.repeat_count += 1;
        entry.last_teaching_level = TeachingLevel::from_repeat_count(entry.repeat_count);
        entry.repeat_count
    }
}

pub(super) fn load_session(path: &Path) -> Result<AiSession, String> {
    match fs::read_to_string(path) {
        Ok(text) => {
            let file: AiSessionFile = serde_json::from_str(&text).map_err(|error| {
                format!("failed to parse AI session {}: {error}", path.display())
            })?;
            if file.version != AI_SESSION_VERSION {
                return Err(format!(
                    "unsupported AI session version `{}` in {}; expected `{}`",
                    file.version,
                    path.display(),
                    AI_SESSION_VERSION
                ));
            }
            Ok(AiSession {
                entries: file.entries,
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(AiSession::default()),
        Err(error) => Err(format!(
            "failed to read AI session {}: {error}",
            path.display()
        )),
    }
}

pub(super) fn save_session(path: &Path, session: &AiSession) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
    }

    let file = AiSessionFile {
        version: AI_SESSION_VERSION,
        entries: session.entries.clone(),
    };
    let text = serde_json::to_string_pretty(&file)
        .map_err(|error| format!("failed to serialize AI session {}: {error}", path.display()))?;
    fs::write(path, text)
        .map_err(|error| format!("failed to write AI session {}: {error}", path.display()))
}
