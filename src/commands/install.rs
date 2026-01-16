use crate::core::{downloader, extractor, os, version};
use anyhow::{Context, Result};
use console::style;
use directories::BaseDirs;
use reqwest::Client;
use std::fs;

pub async fn run(target_version: String) -> Result<()> {
    let base_dirs = BaseDirs::new().context("Could not determine home directory")?;
    let data_dir = base_dirs.data_local_dir().join("blup").join("versions");
    let install_dir = data_dir.join(&target_version);

    if install_dir.exists() {
        println!("{} Version {} is already installed at {:?}",
                 style("i").blue(), target_version, install_dir);
        return Ok(());
    }

    println!("{} Installing Blender {}...", style("==>").green(), target_version);

    let platform = os::detect_platform()?;
    let url = version::build_url(version::OFFICIAL_URL, &target_version, &platform);

    println!("  {} Fetching from: {}", style("->").dim(), url);

    let temp_dir = tempfile::tempdir()?;
    let archive_name = format!("blender.{}.{}", target_version, platform.ext);
    let archive_path = temp_dir.path().join(&archive_name);

    let client = Client::new();
    downloader::download_file(&client, &url, &archive_path).await?;

    println!("  {} Extracting...", style("->").dim());
    fs::create_dir_all(&install_dir)?;

    extractor::extract(&archive_path, &install_dir)?;

    println!("\n{} Blender {} installed successfully! 🎉", style("SUCCESS").green().bold(), target_version);
    println!("    Location: {:?}", install_dir);

    Ok(())
}