use crate::core::{config, daily, downloader, extractor, os, version};
use anyhow::Result;
use chrono::DateTime;
use console::style;
use reqwest::Client;
use std::fs;
use std::path::Path;

pub async fn run(target_version: String, is_daily: bool, skip_checksum: bool) -> Result<()> {
    let app_root = config::get_app_root()?;
    let client = Client::new();
    let platform = os::detect_platform()?;

    let (download_url, version_name, expected_checksum) = if is_daily {
        resolve_daily_version(&client, &target_version, &platform).await?
    } else {
        resolve_stable_version(&client, &target_version, &platform).await?
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

    download_verify_extract(
        &client,
        &download_url,
        &install_dir,
        skip_checksum,
        expected_checksum,
    )
    .await?;

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

async fn resolve_stable_version(
    client: &Client,
    target_version: &str,
    platform: &os::Platform,
) -> Result<(String, String, Option<String>)> {
    let download_url = version::build_url(version::OFFICIAL_URL, target_version, platform);
    let target_filename = version::extract_filename_from_url(&download_url).unwrap_or_default();

    let checksum_url = version::build_checksum_list_url(version::OFFICIAL_URL, target_version);

    let checksum = match client.get(&checksum_url).send().await {
        Ok(res) if res.status().is_success() => {
            let content = res.text().await.unwrap_or_default();
            if !target_filename.is_empty() {
                downloader::find_checksum_in_list(&content, &target_filename)
            } else {
                None
            }
        }
        _ => None,
    };

    Ok((download_url, target_version.to_string(), checksum))
}

async fn resolve_daily_version(
    client: &Client,
    target_version: &str,
    platform: &os::Platform,
) -> Result<(String, String, Option<String>)> {
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

    Ok((best_match.url, folder_name, best_match.checksum))
}

async fn download_verify_extract(
    client: &Client,
    download_url: &str,
    install_dir: &Path,
    skip_checksum: bool,
    expected_checksum: Option<String>,
) -> Result<()> {
    println!("  {} Fetching from: {}", style("->").dim(), download_url);

    let temp_dir = tempfile::tempdir()?;
    let archive_name = version::extract_filename_from_url(download_url)?;
    let archive_path = temp_dir.path().join(archive_name);

    downloader::download_file(client, download_url, &archive_path).await?;

    if !skip_checksum {
        if let Some(checksum) = expected_checksum {
            println!("  {} Verifying checksum...", style("->").dim());
            downloader::verify_checksum(&archive_path, &checksum)?;
            println!("  {} Checksum OK", style("->").green());
        } else {
            println!(
                "  {} No checksum found (skipping verification)",
                style("!").yellow()
            );
        }
    } else {
        println!("  {} Checksum verification skipped", style("!").yellow());
    }

    println!("  {} Extracting...", style("->").dim());
    fs::create_dir_all(install_dir)?;

    extractor::extract(&archive_path, install_dir)?;
    Ok(())
}
