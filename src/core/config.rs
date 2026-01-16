use anyhow::{Context, Result};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct Settings {
    pub default_version: Option<String>,
}

pub fn get_config_dir() -> Result<PathBuf> {
    let base_dirs = BaseDirs::new().context("Could not determine home directory")?;
    let config_dir = base_dirs.config_dir().join("blup");
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }
    Ok(config_dir)
}

pub fn load() -> Result<Settings> {
    let config_path = get_config_dir()?.join("settings.toml");
    if !config_path.exists() {
        return Ok(Settings::default());
    }

    let content = fs::read_to_string(config_path)?;
    let settings: Settings = toml::from_str(&content)
        .context("Failed to parse settings.toml")?;

    Ok(settings)
}

pub fn save(settings: &Settings) -> Result<()> {
    let config_path = get_config_dir()?.join("settings.toml");
    let content = toml::to_string_pretty(settings)?;
    fs::write(config_path, content)?;
    Ok(())
}