use crate::core::config;
use anyhow::{Context, Result};
use console::style;
use directories::BaseDirs;
use std::fs;
use std::io::{self, Write};

pub fn run(version: String, yes: bool) -> Result<()> {
    if version.contains('/') || version.contains('\\') || version == ".." {
        anyhow::bail!("Invalid version format");
    }
    let base_dirs = BaseDirs::new().context("Could not determine home directory")?;
    let install_dir = base_dirs
        .data_local_dir()
        .join("blup")
        .join("versions")
        .join(&version);

    if !install_dir.exists() {
        println!(
            "{} Version {} is not installed.",
            style("i").blue(),
            version
        );
        return Ok(());
    }

    if !yes {
        print!(
            "{} Are you sure you want to uninstall Blender {}? [y/N] ",
            style("?").yellow(),
            version
        );
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim().to_lowercase();
        if input != "y" && input != "yes" {
            println!("Cancelled.");
            return Ok(());
        }
    }

    println!("{} Removing Blender {}...", style("==>").red(), version);
    fs::remove_dir_all(&install_dir).context("Failed to remove directory")?;

    if let Ok(mut settings) = config::load() {
        if settings.default_version.as_deref() == Some(&version) {
            settings.default_version = None;
            config::save(&settings)?;
            println!("{} Cleared default version.", style("i").blue());
        }
    }

    println!(
        "{} Blender {} uninstalled successfully.",
        style("✓").green(),
        version
    );

    Ok(())
}
