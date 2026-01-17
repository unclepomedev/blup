use crate::core::{config, daily, downloader, extractor, os, version};
use anyhow::Result;
use chrono::DateTime;
use console::style;
use reqwest::Client;
use std::fs;
use std::path::Path;

pub async fn run(target_version: String, is_daily: bool) -> Result<()> {
    let app_root = config::get_app_root()?;
    let client = Client::new();
    let platform = os::detect_platform()?;

    let (download_url, version_name) = if is_daily {
        resolve_daily_version(&client, &target_version, &platform).await?
    } else {
        resolve_stable_version(&target_version, &platform)
    };

    let install_dir = app_root.join("versions").join(&version_name);

    if install_dir.exists() {
        println!(
            "{} Version {} is already installed at {:?}",
            style("i").blue(),
            version_name,
            install_dir
        );
        return Ok(());
    }

    println!(
        "{} Installing Blender {}...",
        style("==>").green(),
        version_name
    );

    download_and_extract(&client, &download_url, &install_dir).await?;

    println!(
        "\n{} Blender {} installed successfully! 🎉",
        style("SUCCESS").green().bold(),
        version_name
    );
    println!("    Location: {:?}", install_dir);

    if is_daily {
        println!(
            "    To run this version: {}",
            style(format!("blup run {}", version_name)).yellow()
        );
    }

    Ok(())
}

fn resolve_stable_version(target_version: &str, platform: &os::Platform) -> (String, String) {
    let url = version::build_url(version::OFFICIAL_URL, target_version, platform);
    (url, target_version.to_string())
}

async fn resolve_daily_version(
    client: &Client,
    target_version: &str,
    platform: &os::Platform,
) -> Result<(String, String)> {
    println!(
        "{} Fetching daily build list for '{}'...",
        style("==>").green(),
        target_version
    );

    let builds = daily::fetch_daily_list(client).await?;
    let best_match = daily::find_match(&builds, target_version, platform)?;

    let folder_name = format!(
        "{}-{}-{}",
        best_match.version, best_match.risk_id, best_match.hash
    );

    println!(
        "{} Found: {} (Branch: {}, Date: {})",
        style("i").blue(),
        best_match.file_name,
        best_match.branch,
        DateTime::from_timestamp(best_match.file_mtime as i64, 0)
            .unwrap_or_default()
            .format("%Y-%m-%d")
    );

    Ok((best_match.url, folder_name))
}

async fn download_and_extract(
    client: &Client,
    download_url: &str,
    install_dir: &Path,
) -> Result<()> {
    println!("  {} Fetching from: {}", style("->").dim(), download_url);

    let temp_dir = tempfile::tempdir()?;
    let archive_name = download_url
        .split('/')
        .next_back()
        .filter(|s| !s.is_empty() && s.contains('.'))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Could not determine archive filename from URL: {}",
                download_url
            )
        })?;
    let archive_path = temp_dir.path().join(archive_name);

    downloader::download_file(client, download_url, &archive_path).await?;

    println!("  {} Extracting...", style("->").dim());
    fs::create_dir_all(install_dir)?;

    extractor::extract(&archive_path, install_dir)?;
    Ok(())
}
