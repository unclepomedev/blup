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

#[derive(Debug, Deserialize, Clone)]
pub struct RemoteSection {
    pub stable: Vec<DailyBuild>,
    pub daily: Vec<DailyBuild>,
}

const LTS_VERSIONS: &[&str] = &["3.3", "3.6", "4.2", "4.5"]; // ignore 2.93, 2.83

pub fn is_lts(version: &str) -> bool {
    LTS_VERSIONS.iter().any(|&lts| {
        version == lts || version.starts_with(&format!("{}.", lts))
    })
}

pub fn categorize_builds(builds: Vec<DailyBuild>, platform: &Platform) -> RemoteSection {
    let target_platform = match platform.os.as_str() {
        "macos" => "darwin",
        other => other,
    };

    let target_arch = match (platform.os.as_str(), platform.arch.as_str()) {
        ("windows", "x64") => "amd64",
        (_, "x64") => "x86_64",
        (_, arch) => arch,
    };

    let preferred_ext = if platform.os == "windows" {
        "zip"
    } else {
        &platform.ext
    };

    let mut stable = Vec::new();
    let mut daily = Vec::new();

    for build in builds {
        if build.platform != target_platform || build.architecture != target_arch {
            continue;
        }
        if build.file_extension != preferred_ext {
            continue;
        }

        if build.risk_id == "stable" {
            stable.push(build);
        } else {
            daily.push(build);
        }
    }

    stable.sort_by(|a, b| human_sort_version(&b.version, &a.version));
    daily.sort_by(|a, b| b.file_mtime.cmp(&a.file_mtime));

    RemoteSection { stable, daily }
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

fn human_sort_version(v1: &str, v2: &str) -> std::cmp::Ordering {
    let v1_parts: Vec<&str> = v1.split('.').collect();
    let v2_parts: Vec<&str> = v2.split('.').collect();

    let len = std::cmp::max(v1_parts.len(), v2_parts.len());

    for i in 0..len {
        let p1 = v1_parts.get(i).unwrap_or(&"0");
        let p2 = v2_parts.get(i).unwrap_or(&"0");

        let n1 = p1.parse::<u32>();
        let n2 = p2.parse::<u32>();

        match (n1, n2) {
            (Ok(num1), Ok(num2)) => match num1.cmp(&num2) {
                std::cmp::Ordering::Equal => continue,
                ord => return ord,
            },
            _ => match p1.cmp(p2) {
                std::cmp::Ordering::Equal => continue,
                ord => return ord,
            },
        }
    }
    std::cmp::Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_human_sort_version() {
        use std::cmp::Ordering;

        // Basic
        assert_eq!(human_sort_version("3.6.0", "3.6.0"), Ordering::Equal);
        assert_eq!(human_sort_version("3.6.0", "4.0.0"), Ordering::Less);
        assert_eq!(human_sort_version("4.0.0", "3.6.0"), Ordering::Greater);

        // Numeric order (Crucial: 4.10 > 4.2)
        assert_eq!(human_sort_version("4.2.0", "4.10.0"), Ordering::Less);
        assert_eq!(human_sort_version("4.10.0", "4.2.0"), Ordering::Greater);

        // Length mismatch
        assert_eq!(human_sort_version("3.6", "3.6.1"), Ordering::Less);
        assert_eq!(human_sort_version("3.6.1", "3.6"), Ordering::Greater);

        // Non-numeric
        assert_eq!(human_sort_version("3.6.a", "3.6.b"), Ordering::Less);
    }

    #[test]
    fn test_is_lts() {
        assert!(is_lts("3.3.21"));
        assert!(is_lts("3.6.0"));
        assert!(is_lts("4.2.10"));
        assert!(!is_lts("2.83.0"));
        assert!(!is_lts("5.0.0"));
    }
}
