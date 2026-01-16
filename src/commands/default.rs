use crate::core::{config, version};
use anyhow::{Result, bail};
use console::style;

pub fn run(version: Option<String>) -> Result<()> {
    let mut settings = config::load()?;

    match version {
        Some(v) => {
            version::validate_version_string(&v)?;

            let app_root = config::get_app_root()?;
            let install_dir = app_root.join("versions").join(&v);

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
