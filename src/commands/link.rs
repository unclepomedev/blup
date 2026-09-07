use crate::core::config::LinkedEntry;
use crate::core::{config, os, version};
use anyhow::{Result, bail};
use console::style;
use std::path::{Path, PathBuf};

/// Links an existing Blender executable into blup's managed store.
pub fn run(path_str: &str, as_name: &str, force: bool) -> Result<()> {
    version::validate_link_name(as_name)?;

    let exec_path = resolve_and_verify_executable(path_str)?;

    ensure_name_available(as_name, force)?;

    let detected_version = os::detect_blender_version(&exec_path);

    register_link(as_name, &exec_path, detected_version.clone())?;

    print_link_success(as_name, &exec_path, detected_version.as_deref());

    Ok(())
}

fn resolve_and_verify_executable(path_str: &str) -> Result<PathBuf> {
    let exec_path = os::normalize_executable_path(path_str)?;

    if !exec_path.exists() {
        bail!("Executable does not exist at: {:?}", exec_path);
    }
    if !exec_path.is_file() {
        bail!("Path is not a regular file: {:?}", exec_path);
    }

    verify_executable_permission(&exec_path)?;

    Ok(exec_path)
}

fn verify_executable_permission(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(path)?;
        if metadata.permissions().mode() & 0o111 == 0 {
            bail!("File is not executable: {:?}", path);
        }
    }

    #[cfg(not(unix))]
    let _ = path;

    Ok(())
}

fn ensure_name_available(as_name: &str, force: bool) -> Result<()> {
    let app_root = config::get_app_root()?;
    let install_dir = app_root.join("versions").join(as_name);
    if install_dir.is_dir() {
        bail!(
            "An installed version named '{}' already exists. Cannot link with this name.",
            as_name
        );
    }

    let settings = config::load()?;
    if settings.links.contains_key(as_name) && !force {
        bail!(
            "A link named '{}' already exists. Use --force to overwrite.",
            as_name
        );
    }

    Ok(())
}

fn register_link(as_name: &str, exec_path: &Path, detected_version: Option<String>) -> Result<()> {
    let mut settings = config::load()?;
    let entry = LinkedEntry {
        path: exec_path.to_path_buf(),
        detected_version,
    };
    settings.links.insert(as_name.to_string(), entry);
    config::save(&settings)
}

fn print_link_success(as_name: &str, exec_path: &Path, detected_version: Option<&str>) {
    let version_suffix = match detected_version {
        Some(ver) => format!(" (detected version: {})", ver),
        None => String::new(),
    };

    println!(
        "{} Linked '{}' -> {:?}{}",
        style("✓").green(),
        as_name,
        exec_path,
        version_suffix
    );
}
