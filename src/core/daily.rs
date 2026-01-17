use crate::core::os::Platform;
use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde::Deserialize;

const DAILY_JSON_URL: &str = "https://builder.blender.org/download/daily/?format=json&v=2";

#[derive(Debug, Deserialize, Clone)]
pub struct DailyBuild {
    pub url: String,
    pub version: String, // "4.2.17"
    pub risk_id: String, // "candidate", "alpha", "stable"
    pub branch: String,  // "v42", "main"
    pub hash: String,
    pub platform: String,     // "windows", "darwin", "linux"
    pub architecture: String, // "amd64", "arm64", "x86_64"
    pub file_name: String,
    pub file_mtime: u64,
    pub file_extension: String,
}

pub async fn fetch_daily_list(client: &Client) -> Result<Vec<DailyBuild>> {
    let response = client
        .get(DAILY_JSON_URL)
        .send()
        .await?
        .error_for_status()
        .context("Failed to fetch daily builds JSON")?;

    let builds: Vec<DailyBuild> = response
        .json()
        .await
        .context("Failed to parse daily builds JSON")?;

    Ok(builds)
}

pub fn find_match(
    builds: &[DailyBuild],
    version_query: &str,
    platform: &Platform,
) -> Result<DailyBuild> {
    let target_platform = match platform.os.as_str() {
        "macos" => "darwin",
        other => other,
    };

    let target_arch = match (platform.os.as_str(), platform.arch.as_str()) {
        ("windows", "x64") => "amd64",
        (_, "x64") => "x86_64",
        (_, arch) => arch, // "arm64" matches
    };

    let preferred_ext = if platform.os == "windows" {
        "zip"
    } else {
        &platform.ext
    };

    let mut candidates: Vec<&DailyBuild> = builds
        .iter()
        .filter(|b| {
            if b.platform != target_platform || b.architecture != target_arch {
                return false;
            }

            let version_match = b.version.starts_with(version_query)
                || (version_query == "daily" && b.branch == "main");

            if !version_match {
                return false;
            }
            b.file_extension == preferred_ext
        })
        .collect();

    if candidates.is_empty() {
        bail!(
            "No daily build found for version query '{}' on {} ({})",
            version_query,
            platform.os,
            platform.arch
        );
    }

    candidates.sort_by(|a, b| b.file_mtime.cmp(&a.file_mtime));
    Ok(candidates[0].clone())
}
