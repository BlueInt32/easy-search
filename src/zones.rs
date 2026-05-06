use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub zones: Vec<Zone>,
}

pub fn config_path() -> PathBuf {
    dirs_next::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("ratafzf.yaml")
}

pub fn load_config() -> Config {
    let path = config_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(cfg) = serde_yaml::from_str(&content) {
            return cfg;
        }
    }
    let default = Config {
        zones: vec![
            Zone { name: "home".into(),    path: "/home/user".into() },
            Zone { name: "musique".into(), path: "/mnt/WIN_E/Musique/".into() },
        ],
    };
    let _ = save_config(&default);
    default
}

pub fn save_config(cfg: &Config) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_yaml::to_string(cfg)?)?;
    Ok(())
}
