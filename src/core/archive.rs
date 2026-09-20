use crate::core::daily::human_sort_version;
use crate::core::os::Platform;
use anyhow::{Context, Result};
use futures_util::stream::{self, StreamExt};
use reqwest::Client;

/// Release directories older than this are ignored (legacy naming, no modern platform builds).
const MIN_ARCHIVE_SERIES: &str = "2.93";

/// Number of directory index pages fetched in parallel.
const FETCH_CONCURRENCY: usize = 8;

/// Extracts the `href` values from a simple Nginx style directory index page.
fn parse_hrefs(html: &str) -> Vec<&str> {
    let mut hrefs = Vec::new();
    for part in html.split("href=\"").skip(1) {
        if let Some(end) = part.find('"') {
            hrefs.push(&part[..end]);
        }
    }
    hrefs
}

/// Parses the root release index and returns the release series (e.g. "5.2") worth fetching.
pub fn parse_release_series(html: &str) -> Vec<String> {
    let mut series: Vec<String> = parse_hrefs(html)
        .into_iter()
        .filter_map(|href| href.strip_suffix('/'))
        .filter_map(|dir| dir.strip_prefix("Blender"))
        .filter(|name| name.contains('.') && name.chars().all(|c| c.is_ascii_digit() || c == '.'))
        .filter(|name| human_sort_version(name, MIN_ARCHIVE_SERIES).is_ge())
        .map(|name| name.to_string())
        .collect();

    series.sort_by(|a, b| human_sort_version(b, a));
    series.dedup();
    series
}

/// Parses a release series index page and returns the versions available for the given platform.
pub fn parse_series_versions(html: &str, platform: &Platform) -> Vec<String> {
    let suffix = format!("-{}-{}.{}", platform.os, platform.arch, platform.ext);

    let mut versions: Vec<String> = parse_hrefs(html)
        .into_iter()
        .filter_map(|href| href.strip_prefix("blender-"))
        .filter_map(|rest| rest.strip_suffix(&suffix))
        .filter(|version| !version.is_empty())
        .map(|version| version.to_string())
        .collect();

    versions.sort_by(|a, b| human_sort_version(b, a));
    versions.dedup();
    versions
}

async fn fetch_text(client: &Client, url: &str) -> Result<String> {
    let response = client
        .get(url)
        .send()
        .await?
        .error_for_status()
        .with_context(|| format!("Failed to fetch {}", url))?;

    let body = response
        .text()
        .await
        .with_context(|| format!("Failed to read response from {}", url))?;

    Ok(body)
}

/// Fetches every official release available for the given platform from the download archive.
/// Returns versions sorted from newest to oldest.
pub async fn fetch_all_versions(
    client: &Client,
    base: &str,
    platform: &Platform,
) -> Result<Vec<String>> {
    let base_url = base.trim_end_matches('/');
    let index = fetch_text(client, &format!("{}/", base_url)).await?;
    let series = parse_release_series(&index);

    let pages = stream::iter(series.into_iter().map(|name| {
        let url = format!("{}/Blender{}/", base_url, name);
        async move { fetch_text(client, &url).await }
    }))
    .buffer_unordered(FETCH_CONCURRENCY)
    .collect::<Vec<_>>()
    .await;

    let mut versions = Vec::new();
    // A single unreachable series must not break the whole listing.
    for html in pages.into_iter().flatten() {
        versions.extend(parse_series_versions(&html, platform));
    }

    versions.sort_by(|a, b| human_sort_version(b, a));
    versions.dedup();
    Ok(versions)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT_INDEX: &str = r#"
        <a href="../">../</a>
        <a href="Blender2.93/">Blender2.93/</a>
        <a href="Blender5.2/">Blender5.2/</a>
        <a href="Blender5.10/">Blender5.10/</a>
        <a href="Blender2.28a/">Blender2.28a/</a>
        <a href="add-ons-legacy-bundle.zip">add-ons-legacy-bundle.zip</a>
    "#;

    const SERIES_INDEX: &str = r#"
        <a href="../">../</a>
        <a href="blender-5.2.0-macos-arm64.dmg">blender-5.2.0-macos-arm64.dmg</a>
        <a href="blender-5.2.0-macos-x64.dmg">blender-5.2.0-macos-x64.dmg</a>
        <a href="blender-5.2.10-macos-arm64.dmg">blender-5.2.10-macos-arm64.dmg</a>
        <a href="blender-5.2.0-linux-x64.tar.xz">blender-5.2.0-linux-x64.tar.xz</a>
        <a href="blender-5.2.0.sha256">blender-5.2.0.sha256</a>
    "#;

    fn macos_platform() -> Platform {
        Platform {
            os: "macos".to_string(),
            arch: "arm64".to_string(),
            ext: "dmg".to_string(),
        }
    }

    #[test]
    fn test_parse_release_series() {
        let series = parse_release_series(ROOT_INDEX);

        assert_eq!(series, vec!["5.10", "5.2", "2.93"]);
    }

    #[test]
    fn test_parse_series_versions() {
        let versions = parse_series_versions(SERIES_INDEX, &macos_platform());

        assert_eq!(versions, vec!["5.2.10", "5.2.0"]);
    }

    #[test]
    fn test_parse_series_versions_other_platform() {
        let platform = Platform {
            os: "linux".to_string(),
            arch: "x64".to_string(),
            ext: "tar.xz".to_string(),
        };

        let versions = parse_series_versions(SERIES_INDEX, &platform);

        assert_eq!(versions, vec!["5.2.0"]);
    }
}
