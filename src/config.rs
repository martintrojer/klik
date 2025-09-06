use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub number_of_words: usize,
    pub number_of_secs: Option<usize>,
    pub supported_language: String,
    pub random_words: bool,
    pub capitalize: bool,
    pub strict: bool,
    pub symbols: bool,
    pub substitute: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            number_of_words: 15,
            number_of_secs: None,
            supported_language: "english".to_string(),
            random_words: false,
            capitalize: false,
            strict: false,
            symbols: false,
            substitute: false,
        }
    }
}

impl From<&crate::RuntimeSettings> for Config {
    fn from(rs: &crate::RuntimeSettings) -> Self {
        Self {
            number_of_words: rs.number_of_words,
            number_of_secs: rs.number_of_secs,
            supported_language: rs.supported_language.to_string().to_lowercase(),
            random_words: rs.random_words,
            capitalize: rs.capitalize,
            strict: rs.strict,
            symbols: rs.symbols,
            substitute: rs.substitute,
        }
    }
}

pub trait ConfigStore {
    fn load(&self) -> Config;
    fn save(&self, cfg: &Config) -> std::io::Result<()>;
}

#[derive(Debug, Clone)]
pub struct FileConfigStore {
    path: PathBuf,
}

impl FileConfigStore {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let path = if let Some(pd) = ProjectDirs::from("", "", "klik") {
            pd.config_dir().join("config.json")
        } else {
            PathBuf::from("klik_config.json")
        };
        Self { path }
    }

    pub fn with_path<P: AsRef<Path>>(p: P) -> Self {
        Self {
            path: p.as_ref().to_path_buf(),
        }
    }
}

impl Default for FileConfigStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigStore for FileConfigStore {
    fn load(&self) -> Config {
        if let Ok(bytes) = fs::read(&self.path) {
            if let Ok(cfg) = serde_json::from_slice::<Config>(&bytes) {
                return cfg;
            }
        }
        Config::default()
    }

    fn save(&self, cfg: &Config) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_vec_pretty(cfg).unwrap_or_default();
        fs::write(&self.path, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn roundtrip_default_config() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        let store = FileConfigStore::with_path(&path);
        let cfg = Config::default();
        store.save(&cfg).unwrap();
        let loaded = store.load();
        assert_eq!(cfg, loaded);
    }

    #[test]
    fn save_and_load_custom_config() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.json");
        let store = FileConfigStore::with_path(&path);
        let cfg = Config {
            number_of_words: 50,
            number_of_secs: Some(60),
            supported_language: "english10k".into(),
            random_words: true,
            capitalize: true,
            strict: true,
            symbols: true,
            substitute: true,
        };
        store.save(&cfg).unwrap();
        let loaded = store.load();
        assert_eq!(cfg, loaded);
    }
}
