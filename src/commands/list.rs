use anyhow::{Context, Result};
use console::style;
use directories::BaseDirs;
use std::fs;

pub fn run() -> Result<()> {
    let base_dirs = BaseDirs::new().context("Could not determine home directory")?;
    let data_dir = base_dirs.data_local_dir().join("blup").join("versions");

    if !data_dir.exists() {
        println!("No Blender versions installed yet.");
        return Ok(());
    }

    println!("{}", style("Installed Blender Versions:").bold());

    let mut entries: Vec<_> = fs::read_dir(&data_dir)?
        .filter_map(|res| res.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();

    entries.sort();

    for path in entries {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            println!("  {} {}", style("•").green(), name);
        }
    }

    Ok(())
}