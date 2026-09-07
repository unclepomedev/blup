use crate::core::{config, version};
use anyhow::Result;

pub fn run(target_version: Option<String>) -> Result<()> {
    let mut version = config::resolve_version(target_version)?;

    if version == "daily" {
        version = version::find_latest_daily_installed()?;
    }

    let (bin_path, _) = config::get_executable_for_version(&version)?;

    println!("{}", bin_path.display());
    Ok(())
}
