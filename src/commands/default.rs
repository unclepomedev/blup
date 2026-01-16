use crate::core::{config, version};
use anyhow::{Context, Result, bail};
use console::style;
use directories::BaseDirs;

pub fn run(version: Option<String>) -> Result<()> {
    let mut settings = config::load()?;

    match version {
        Some(v) => {
            version::validate_version_string(&v)?;
            let base_dirs = BaseDirs::new().context("Could not determine home directory")?;
            let install_dir = base_dirs
                .data_local_dir()
                .join("blup")
                .join("versions")
                .join(&v);

            if !install_dir.is_dir() {
                bail!("Version {} is not installed. Please install it first.", v);
            }

            settings.default_version = Some(v.clone());
            config::save(&settings)?;
            println!(
                "{} Default Blender version set to {}",
                style("✓").green(),
                style(v).bold()
            );
        }
        None => match settings.default_version {
            Some(v) => println!("Current default: {}", style(v).bold()),
            None => println!("No default version set."),
        },
    }

    Ok(())
}
