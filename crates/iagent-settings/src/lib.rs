use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IagentConfig {
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub api_base: Option<String>,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub start_minimized: bool,
    #[serde(default)]
    pub always_on_top: bool,
}

impl IagentConfig {
    pub fn load() -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path)?;
        toml::from_str(&content).map_err(|e| e.into())
    }

    pub fn save(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    fn config_path() -> std::result::Result<PathBuf, Box<dyn std::error::Error>> {
        let config_dir = dirs::config_dir()
            .ok_or("cannot find config dir")?
            .join("iAgent");
        Ok(config_dir.join("settings.toml"))
    }
}
