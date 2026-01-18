use crate::core::os::Platform;
use anyhow::{Result, anyhow, bail};
use std::env;
use std::path::{Component, Path};

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
        _ => bail!("Invalid version string: {}", v),
    }

    if components.next().is_some() {
        bail!("Invalid version string: {}", v);
    }

    Ok(())
}

pub fn build_url(base: &str, version: &str, platform: &Platform) -> String {
    let base_url = env::var("BLUP_MIRROR_URL")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| base.to_string());

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
    let base_url = env::var("BLUP_MIRROR_URL")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| base.to_string());

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
}
