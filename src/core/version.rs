use crate::core::os::Platform;

pub const OFFICIAL_URL: &str = "https://download.blender.org/release";

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
}
