use crate::core::config::Settings;
use crate::core::{config, version};
use anyhow::{Result, bail};
use console::style;

pub fn run(version: Option<String>) -> Result<()> {
    let mut settings = config::load()?;

    match version {
        Some(v) => set_default_version(&mut settings, &v)?,
        None => show_default_version(&settings),
    }

    Ok(())
}

fn set_default_version(settings: &mut Settings, version: &str) -> Result<()> {
    version::validate_link_name(version)?;

    if !config::is_version_or_link_installed(version)? {
        bail!(
            "Version or link '{}' is not installed. Please install or link it first.",
            version
        );
    }

    settings.default_version = Some(version.to_string());
    config::save(settings)?;
    println!(
        "{} Default Blender version set to {}",
        style("✓").green(),
        style(version).bold()
    );
    Ok(())
}

fn show_default_version(settings: &Settings) {
    match settings.default_version {
        Some(ref v) => println!("Current default: {}", style(v).bold()),
        None => println!("No default version set."),
    }
}
