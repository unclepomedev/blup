use crate::core::os::Platform;
use crate::core::{config, daily};
use anyhow::{Context, Result, anyhow, bail};
use std::cmp::Ordering;
use std::env;
use std::fs;
use std::path::{Component, Path};
use std::time::SystemTime;

pub const OFFICIAL_URL: &str = "https://download.blender.org/release";

pub fn extract_filename_from_url(url: &str) -> Result<String> {
    url.split('/')
        .next_back()
        .filter(|s| !s.is_empty() && s.contains('.'))
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("Could not determine archive filename from URL: {}", url))
}

pub fn validate_version_string(v: &str) -> Result<()> {
    let path = Path::new(v);
    let mut components = path.components();

    match components.next() {
        Some(Component::Normal(_)) => {}
        _ => bail!("'{}' is not a valid version string", v),
    }

    if components.next().is_some() {
        bail!("'{}' is not a valid version string", v);
    }

    Ok(())
}

pub fn build_url(base: &str, version: &str, platform: &Platform) -> String {
    let base_url = get_base_url(base);

    // version: "5.0.0" -> major_minor: "5.0"
    let parts: Vec<&str> = version.split('.').collect();
    let major_minor = if parts.len() >= 2 {
        format!("{}.{}", parts[0], parts[1])
    } else {
        version.to_string()
    };

    // e.g. https://download.blender.org/release/Blender5.0/blender-5.0.0-windows-x64.zip
    format!(
        "{}/Blender{}/blender-{}-{}-{}.{}",
        base_url, major_minor, version, platform.os, platform.arch, platform.ext
    )
}

pub fn build_checksum_list_url(base: &str, version: &str) -> String {
    let base_url = get_base_url(base);

    let parts: Vec<&str> = version.split('.').collect();
    let major_minor = if parts.len() >= 2 {
        format!("{}.{}", parts[0], parts[1])
    } else {
        version.to_string()
    };

    format!(
        "{}/Blender{}/blender-{}.sha256",
        base_url, major_minor, version
    )
}

pub fn find_latest_daily_installed() -> Result<String> {
    let app_root = config::get_app_root()?;
    let versions_dir = app_root.join("versions");

    if !versions_dir.exists() {
        bail!("No versions installed (versions directory not found).");
    }

    let entries = fs::read_dir(&versions_dir)
        .with_context(|| format!("Failed to read directory: {:?}", versions_dir))?;

    let mut candidates: Vec<(String, SystemTime)> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }

        if is_daily_build_name(&name) {
            let mtime = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);

            candidates.push((name, mtime));
        }
    }

    if candidates.is_empty() {
        bail!("No daily builds found locally. Run `blup install daily --daily` first.");
    }

    candidates.sort_by(|(name_a, time_a), (name_b, time_b)| {
        let version_order = daily::human_sort_version(name_b, name_a);

        match version_order {
            Ordering::Equal => time_b.cmp(time_a),
            other => other,
        }
    });

    Ok(candidates[0].0.clone())
}

fn is_daily_build_name(name: &str) -> bool {
    let keywords = ["alpha", "beta", "candidate", "rc"];
    keywords.iter().any(|&k| name.contains(k))
}
fn get_base_url(base: &str) -> String {
    env::var("BLUP_MIRROR_URL")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| base.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_url_generation_windows() {
        let platform = Platform {
            os: "windows".to_string(),
            arch: "x64".to_string(),
            ext: "zip".to_string(),
        };
        let url = build_url(OFFICIAL_URL, "5.0.0", &platform);
        assert_eq!(
            url,
            "https://download.blender.org/release/Blender5.0/blender-5.0.0-windows-x64.zip"
        );
    }

    #[test]
    fn test_url_generation_linux() {
        let platform = Platform {
            os: "linux".to_string(),
            arch: "x64".to_string(),
            ext: "tar.xz".to_string(),
        };
        let url = build_url(OFFICIAL_URL, "5.0.0", &platform);
        assert_eq!(
            url,
            "https://download.blender.org/release/Blender5.0/blender-5.0.0-linux-x64.tar.xz"
        );
    }

    #[test]
    fn test_url_generation_macos() {
        let platform = Platform {
            os: "macos".to_string(),
            arch: "arm64".to_string(),
            ext: "dmg".to_string(),
        };
        let url = build_url(OFFICIAL_URL, "5.0.0", &platform);
        assert_eq!(
            url,
            "https://download.blender.org/release/Blender5.0/blender-5.0.0-macos-arm64.dmg"
        );
    }

    #[test]
    fn test_validate_version_string() {
        // ✅
        assert!(validate_version_string("5.0.0").is_ok());
        assert!(validate_version_string("4.2").is_ok());
        assert!(validate_version_string("custom-build-v1").is_ok());

        // ❌
        assert!(validate_version_string("..").is_err());
        assert!(validate_version_string("../5.0.0").is_err());
        assert!(validate_version_string("5.0.0/..").is_err());
        assert!(validate_version_string("/usr/bin").is_err());
        assert!(validate_version_string("/5.0.0").is_err());
        assert!(validate_version_string("subdir/5.0.0").is_err());

        if std::path::MAIN_SEPARATOR == '\\' {
            assert!(validate_version_string("..\\5.0.0").is_err());
            assert!(validate_version_string("C:\\Windows").is_err());
        }
    }

    #[test]
    fn test_url_generation_with_mirror() {
        let _lock = ENV_LOCK.lock().unwrap();

        unsafe {
            env::set_var("BLUP_MIRROR_URL", "https://mirror.example.com");
        }

        let platform = Platform {
            os: "windows".to_string(),
            arch: "x64".to_string(),
            ext: "zip".to_string(),
        };

        let url = build_url(OFFICIAL_URL, "4.5.0", &platform);
        unsafe {
            env::remove_var("BLUP_MIRROR_URL");
        }
        assert_eq!(
            url,
            "https://mirror.example.com/Blender4.5/blender-4.5.0-windows-x64.zip"
        );
    }

    #[test]
    fn test_build_checksum_list_url() {
        let url = build_checksum_list_url(OFFICIAL_URL, "5.0.1");
        assert_eq!(
            url,
            "https://download.blender.org/release/Blender5.0/blender-5.0.1.sha256"
        );

        let url_old = build_checksum_list_url(OFFICIAL_URL, "3.6.23");
        assert_eq!(
            url_old,
            "https://download.blender.org/release/Blender3.6/blender-3.6.23.sha256"
        );
    }

    #[test]
    fn test_extract_filename_from_url() {
        // ✅
        let url = "https://example.com/download/blender-4.2.0.zip";
        assert_eq!(extract_filename_from_url(url).unwrap(), "blender-4.2.0.zip");

        // ❌
        let url_no_ext = "https://example.com/download/blender";
        assert!(extract_filename_from_url(url_no_ext).is_err());

        let url_slash = "https://example.com/download/";
        assert!(extract_filename_from_url(url_slash).is_err());

        assert!(extract_filename_from_url("").is_err());
    }

    #[test]
    fn test_is_daily_build_name() {
        // ✅
        assert!(is_daily_build_name("4.2.0-alpha-abcdef"));
        assert!(is_daily_build_name("5.0.0-beta-123456"));
        assert!(is_daily_build_name("3.6.0-candidate+main"));
        assert!(is_daily_build_name("4.1.0-rc"));
        // ❌
        assert!(!is_daily_build_name("3.6.0"));
        assert!(!is_daily_build_name("4.2.0"));
        assert!(!is_daily_build_name("custom-build"));
    }
}
