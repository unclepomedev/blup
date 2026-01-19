use crate::core::config;
use anyhow::Result;

pub fn run(version_arg: Option<String>) -> Result<()> {
    let version = config::resolve_version(version_arg)?;

    println!("{}", version);
    Ok(())
}
