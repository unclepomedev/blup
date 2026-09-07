use crate::core::config::Settings;
use crate::core::{config, version};
use anyhow::{Context, Result};
use console::style;
use std::fs;
use std::io::{Write, stdin, stdout};
use std::path::Path;

pub fn run(version: String, yes: bool) -> Result<()> {
    version::validate_link_name(&version)?;

    let mut settings = config::load().unwrap_or_default();
    let is_linked = settings.links.contains_key(&version);

    let app_root = config::get_app_root()?;
    let install_dir = app_root.join("versions").join(&version);
    let is_installed = install_dir.is_dir();

    if !is_installed && !is_linked {
        println!(
            "{} Version or link '{}' is not installed.",
            style("i").blue(),
            version
        );
        return Ok(());
    }

    if !yes && !confirm_removal(&version, is_linked)? {
        println!("Cancelled.");
        return Ok(());
    }

    if is_linked {
        remove_link_entry(&version, &mut settings)?;
    } else {
        remove_installed_version(&version, &install_dir, &mut settings)?;
    }

    Ok(())
}

fn confirm_removal(version: &str, is_linked: bool) -> Result<bool> {
    let target_type = if is_linked { "link" } else { "Blender" };
    print!(
        "{} Are you sure you want to remove {} '{}'? [y/N] ",
        style("?").yellow(),
        target_type,
        version
    );
    stdout().flush()?;

    let mut input = String::new();
    stdin().read_line(&mut input)?;

    let input = input.trim().to_lowercase();
    Ok(input == "y" || input == "yes")
}

fn clear_default_if_matches(version: &str, settings: &mut Settings) -> bool {
    if settings.default_version.as_deref() == Some(version) {
        settings.default_version = None;
        println!("{} Cleared default version.", style("i").blue());
        true
    } else {
        false
    }
}

fn remove_link_entry(version: &str, settings: &mut Settings) -> Result<()> {
    println!("{} Unlinking '{}'...", style("==>").red(), version);
    settings.links.remove(version);
    clear_default_if_matches(version, settings);
    config::save(settings)?;
    println!(
        "{} Link '{}' removed successfully (executable was not deleted).",
        style("✓").green(),
        version
    );
    Ok(())
}

fn remove_installed_version(
    version: &str,
    install_dir: &Path,
    settings: &mut Settings,
) -> Result<()> {
    println!("{} Removing Blender {}...", style("==>").red(), version);
    fs::remove_dir_all(install_dir).context("Failed to remove directory")?;

    if clear_default_if_matches(version, settings) {
        config::save(settings)?;
    }

    println!(
        "{} Blender {} uninstalled successfully.",
        style("✓").green(),
        version
    );
    Ok(())
}
