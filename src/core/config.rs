use crate::core::{os, version};
use anyhow::{Context, Result, bail};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

/// Represents a linked Blender executable entry.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LinkedEntry {
    pub path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detected_version: Option<String>,
}

/// Returns true if a version string matches an installed version folder or registered link.
pub fn is_version_or_link_installed(v: &str) -> Result<bool> {
    let app_root = get_app_root()?;
    let install_dir = app_root.join("versions").join(v);
    if install_dir.is_dir() {
        return Ok(true);
    }

    let settings = load().unwrap_or_default();
    if settings.links.contains_key(v) {
        return Ok(true);
    }

    Ok(false)
}

/// Resolves the executable path for a given version or link name.
/// Returns the path and whether it is a link.
pub fn get_executable_for_version(v: &str) -> Result<(PathBuf, bool)> {
    let settings = load().unwrap_or_default();
    if let Some(entry) = settings.links.get(v) {
        if !entry.path.exists() {
            bail!(
                "Linked Blender '{}' executable does not exist at: {:?}",
                v,
                entry.path
            );
        }
        return Ok((entry.path.clone(), true));
    }

    let app_root = get_app_root()?;
    let install_dir = app_root.join("versions").join(v);
    if !install_dir.is_dir() {
        bail!(
            "Blender {} is not installed or linked. Run `blup install {}` or `blup link <path> --as {}` first.",
            v,
            v,
            v
        );
    }

    let bin_path = os::get_bin_path(&install_dir)?;
    Ok((bin_path, false))
}
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
pub struct Settings {
    pub default_version: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub links: HashMap<String, LinkedEntry>,
}

/// Returns the root directory where the application stores its data.
/// The path can be overridden by the `BLUP_ROOT` environment variable.
pub fn get_app_root() -> Result<PathBuf> {
    if let Ok(root) = std::env::var("BLUP_ROOT") {
        return Ok(PathBuf::from(root));
    }

    let base_dirs = BaseDirs::new().context("Could not determine home directory")?;
    Ok(base_dirs.data_local_dir().join("blup"))
}

/// Returns the directory where the application's configuration files are stored.
/// Creates the directory if it doesn't exist.
pub fn get_config_dir() -> Result<PathBuf> {
    let root = get_app_root()?;
    let config_dir = root.join("config");

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)?;
    }
    Ok(config_dir)
}

/// Loads the application settings from the configuration file.
/// Returns default settings if the file doesn't exist.
pub fn load() -> Result<Settings> {
    let config_path = get_config_dir()?.join("settings.toml");
    if !config_path.exists() {
        return Ok(Settings::default());
    }

    let content = fs::read_to_string(config_path)?;
    let settings: Settings = toml::from_str(&content).context("Failed to parse settings.toml")?;

    Ok(settings)
}

/// Saves the application settings to the configuration file.
pub fn save(settings: &Settings) -> Result<()> {
    let config_path = get_config_dir()?.join("settings.toml");
    let content = toml::to_string_pretty(settings)?;
    fs::write(config_path, content)?;
    Ok(())
}

/// Resolves a version string from either the provided argument or a local `.blender-version` file.
pub fn resolve_from_args_or_file(arg_version: Option<String>) -> Result<Option<String>> {
    if let Some(v) = arg_version {
        if let Err(e) = version::validate_version_string(&v) {
            bail!("Provided version '{}' is invalid. Reason: {}", v, e);
        }
        return Ok(Some(v));
    }

    let local_file = Path::new(".blender-version");
    if local_file.exists() {
        let content = fs::read_to_string(local_file).context("Failed to read .blender-version")?;
        let v = content.trim().to_string();

        if v.is_empty() {
            bail!("Found .blender-version but it is empty.");
        }

        if let Err(e) = version::validate_version_string(&v) {
            bail!(
                "Found .blender-version but content '{}' is not a valid version string. Reason: {}",
                v,
                e
            );
        }

        eprintln!(
            "{} Found .blender-version: {}",
            console::style("i").blue(),
            v
        );
        return Ok(Some(v));
    }

    Ok(None)
}

/// Resolves the version to use, prioritizing the provided argument, then the local `.blender-version` file,
/// and finally the default version in the application settings.
pub fn resolve_version(arg_version: Option<String>) -> Result<String> {
    if let Some(v) = resolve_from_args_or_file(arg_version)? {
        return Ok(v);
    }

    let settings = load()?;
    if let Some(v) = settings.default_version {
        if let Err(e) = version::validate_version_string(&v) {
            bail!(
                "Default version '{}' in settings is invalid. Reason: {}",
                v,
                e
            );
        }
        return Ok(v);
    }

    bail!(
        "No version specified. Use `blup run <version>` or set a default with `blup default <version>`."
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::io::Write;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct CwdGuard(PathBuf);
    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.0);
        }
    }

    struct ScopedEnv {
        key: String,
        original: Option<String>,
    }

    impl ScopedEnv {
        fn new(key: &str, value: &str) -> Self {
            let original = env::var(key).ok();
            unsafe {
                env::set_var(key, value);
            }
            Self {
                key: key.to_string(),
                original,
            }
        }
    }

    impl Drop for ScopedEnv {
        fn drop(&mut self) {
            unsafe {
                match &self.original {
                    Some(v) => env::set_var(&self.key, v),
                    None => env::remove_var(&self.key),
                }
            }
        }
    }

    #[test]
    fn test_save_and_load_settings() -> Result<()> {
        let _lock = ENV_LOCK.lock();

        let temp_dir = tempdir()?;
        let _env_guard = ScopedEnv::new("BLUP_ROOT", temp_dir.path().to_str().unwrap());

        let settings = Settings {
            default_version: Some("4.2.0".to_string()),
            ..Default::default()
        };
        save(&settings)?;

        let config_path = temp_dir.path().join("config").join("settings.toml");
        assert!(config_path.exists());

        let loaded = load()?;
        assert_eq!(loaded.default_version, Some("4.2.0".to_string()));

        Ok(())
    }

    #[test]
    fn test_load_empty_returns_default() -> Result<()> {
        let _lock = ENV_LOCK.lock();

        let temp_dir = tempdir()?;
        let _env_guard = ScopedEnv::new("BLUP_ROOT", temp_dir.path().to_str().unwrap());

        let loaded = load()?;
        assert_eq!(loaded.default_version, None);

        Ok(())
    }

    #[test]
    fn test_resolve_version_priority() -> Result<()> {
        let _lock = ENV_LOCK.lock();

        let temp_root = tempdir()?;
        let project_dir = tempdir()?;

        let _env_guard = ScopedEnv::new("BLUP_ROOT", temp_root.path().to_str().unwrap());

        let original_cwd = env::current_dir()?;
        env::set_current_dir(&project_dir)?;

        let _cwd_guard = CwdGuard(original_cwd);

        save(&Settings {
            default_version: Some("GlobalDefault".into()),
            ..Default::default()
        })?;

        {
            let mut file = fs::File::create(".blender-version")?;
            write!(file, "LocalFile")?;
        }

        let result = resolve_version(Some("ArgVersion".into()))?;
        assert_eq!(result, "ArgVersion", "arg should be used if specified");

        let result = resolve_version(None)?;
        assert_eq!(
            result, "LocalFile",
            "The local file should be used if no arg is specified"
        );

        fs::remove_file(".blender-version")?;

        let result = resolve_version(None)?;
        assert_eq!(
            result, "GlobalDefault",
            "Global setting should be used if no local exists"
        );

        save(&Settings {
            default_version: None,
            ..Default::default()
        })?;

        let result = resolve_version(None);
        assert!(result.is_err(), "Should be an error if nothing specified");

        Ok(())
    }

    #[test]
    fn test_resolve_version_invalid_default() -> Result<()> {
        let _lock = ENV_LOCK.lock();

        let temp_root = tempdir()?;
        let project_dir = tempdir()?;

        let _env_guard = ScopedEnv::new("BLUP_ROOT", temp_root.path().to_str().unwrap());

        let original_cwd = env::current_dir()?;
        env::set_current_dir(&project_dir)?;

        let _cwd_guard = CwdGuard(original_cwd);

        save(&Settings {
            default_version: Some("../invalid_version".into()),
            ..Default::default()
        })?;

        let result = resolve_version(None);
        assert!(
            result.is_err(),
            "Should be an error if default version is invalid"
        );

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Default version"));
        assert!(err_msg.contains("is invalid"));

        Ok(())
    }
}
