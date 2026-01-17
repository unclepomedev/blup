use crate::core::{config, os};
use anyhow::{Result, bail};

pub fn run(target_version: Option<String>) -> Result<()> {
    let version = config::resolve_version(target_version)?;

    let app_root = config::get_app_root()?;
    let install_dir = app_root.join("versions").join(&version);

    if !install_dir.exists() {
        bail!(
            "Blender {} is not installed. Run `blup install {}` first.",
            version,
            version
        );
    }

    let bin_path = os::get_bin_path(&install_dir)?;

    println!("{}", bin_path.display());
    Ok(())
}
