use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct RunContext {
    pub argv: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub current_dir: PathBuf,
}

impl Default for RunContext {
    fn default() -> Self {
        Self {
            argv: Vec::new(),
            env: BTreeMap::new(),
            current_dir: PathBuf::from("."),
        }
    }
}

impl RunContext {
    pub fn from_host(argv: Vec<String>) -> std::io::Result<Self> {
        Ok(Self {
            argv,
            env: std::env::vars().collect(),
            current_dir: std::env::current_dir()?,
        })
    }

    pub(super) fn env_contains(&self, name: &str) -> bool {
        self.env_value(name).is_some()
    }

    pub(super) fn env_value(&self, name: &str) -> Option<&str> {
        if let Some(value) = self.env.get(name) {
            return Some(value.as_str());
        }

        #[cfg(windows)]
        {
            self.env
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(name))
                .map(|(_, value)| value.as_str())
        }

        #[cfg(not(windows))]
        {
            None
        }
    }
}
