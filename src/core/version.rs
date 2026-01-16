use crate::core::os::Platform;
use anyhow::Result;
use anyhow::bail;
use std::path::{Component, Path};

pub const OFFICIAL_URL: &str = "https://download.blender.org/release";

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
        base, major_minor, version, platform.os, platform.arch, platform.ext
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
