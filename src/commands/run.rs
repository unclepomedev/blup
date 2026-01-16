use crate::core::{config, os};
use anyhow::{Context, Result, bail};
use console::style;
use directories::BaseDirs;
use std::fs;
use std::process::Command;

pub fn run(version_arg: Option<String>, scripts: Option<String>, args: Vec<String>) -> Result<()> {
    let version = config::resolve_version(version_arg)?;

    let base_dirs = BaseDirs::new().context("Could not determine home directory")?;
    let install_dir = base_dirs
        .data_local_dir()
        .join("blup")
        .join("versions")
        .join(&version);

    if !install_dir.exists() {
        bail!(
            "Blender {} is not installed. Run `blup install {}` first.",
            version,
            version
        );
    }

    let bin_path = os::get_bin_path(&install_dir)?;

    println!("{} Starting Blender {}...", style("==>").green(), version);

    let mut command = Command::new(bin_path);

    if let Some(scripts_path) = scripts {
        let abs_path = fs::canonicalize(&scripts_path)
            .context(format!("Failed to resolve scripts path: {}", scripts_path))?;

        println!("{} Scripts path: {:?}", style("->").dim(), abs_path);
        command.env("BLENDER_USER_SCRIPTS", abs_path);
    }

    let status = command
        .args(&args)
        .status()
        .context("Failed to start Blender")?;

    if !status.success() {
        bail!("Blender exited with non-zero status code");
    }

    Ok(())
}
