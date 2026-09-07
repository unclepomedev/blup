use anyhow::{Result, bail};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Represents the target platform (OS, architecture, and file extension).
#[derive(Debug, PartialEq)]
pub struct Platform {
    pub os: String,
    pub arch: String,
    pub ext: String, // .zip, .tar.xz, .dmg
}

/// Detects the current platform's OS and architecture.
pub fn detect_platform() -> Result<Platform> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    let (os_str, ext) = match os {
        "windows" => ("windows", "zip"),
        "linux" => ("linux", "tar.xz"),
        "macos" => ("macos", "dmg"),
        _ => bail!("Unsupported OS: {}", os),
    };

    let arch_str = match arch {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        _ => bail!("Unsupported Architecture: {}", arch),
    };

    Ok(Platform {
        os: os_str.to_string(),
        arch: arch_str.to_string(),
        ext: ext.to_string(),
    })
}

/// Resolves and normalizes an executable path provided by user.
/// Supports standard absolute paths, mixed slashes, and MSYS/WSL style paths (e.g. `/c/...` -> `C:\...` on Windows).
pub fn normalize_executable_path(raw_path: &str) -> Result<PathBuf> {
    let raw = raw_path.trim();
    if raw.is_empty() {
        bail!("Executable path cannot be empty.");
    }

    #[allow(unused_mut)]
    let mut path_str = raw.to_string();

    #[cfg(windows)]
    {
        // Handle MSYS/WSL style paths: /c/Program Files/... or /c/... -> C:\Program Files\...
        if (path_str.starts_with('/') || path_str.starts_with('\\')) && path_str.len() >= 3 {
            let bytes = path_str.as_bytes();
            if (bytes[0] == b'/' || bytes[0] == b'\\')
                && bytes[1].is_ascii_alphabetic()
                && (bytes[2] == b'/' || bytes[2] == b'\\')
            {
                let drive = (bytes[1] as char).to_ascii_uppercase();
                let rest = &path_str[2..];
                path_str = format!("{}:{}", drive, rest);
            }
        }
        // Normalize forward slashes to backslashes on Windows
        path_str = path_str.replace('/', "\\");
    }

    let path = PathBuf::from(&path_str);

    // Require an absolute path
    if !path.is_absolute() {
        bail!("Path must be an absolute path: '{}'", raw_path);
    }

    Ok(path)
}

/// Detects the Blender version from an executable by executing `blender --version`.
pub fn detect_blender_version(executable: &Path) -> Option<String> {
    let output = Command::new(executable).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Typical output: Blender 4.2.0 (hash ...) / Blender 5.0.0 Alpha
    for line in stdout.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("Blender ") {
            let ver = rest.split_whitespace().next()?;
            if !ver.is_empty() {
                return Some(ver.to_string());
            }
        }
    }
    None
}

pub fn get_bin_path(install_dir: &Path) -> Result<PathBuf> {
    let os = env::consts::OS;

    let bin_path = match os {
        "windows" => install_dir.join("blender.exe"),
        "linux" => install_dir.join("blender"),
        "macos" => install_dir
            .join("Blender.app")
            .join("Contents")
            .join("MacOS")
            .join("Blender"),
        _ => bail!("Unsupported OS for running: {}", os),
    };

    if !bin_path.exists() {
        bail!("Blender executable not found at: {:?}", bin_path);
    }

    Ok(bin_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_detect_platform_sanity_check() {
        let platform = detect_platform();
        assert!(platform.is_ok());

        let p = platform.unwrap();
        if cfg!(windows) {
            assert_eq!(p.os, "windows");
            assert_eq!(p.ext, "zip");
        } else if cfg!(target_os = "macos") {
            assert_eq!(p.os, "macos");
            assert_eq!(p.ext, "dmg");
        } else if cfg!(target_os = "linux") {
            assert_eq!(p.os, "linux");
            assert_eq!(p.ext, "tar.xz");
        }
    }

    #[test]
    fn test_get_bin_path_success() -> Result<()> {
        let temp = tempdir()?;
        let root = temp.path();

        let expected_bin = if cfg!(target_os = "macos") {
            let path = root.join("Blender.app/Contents/MacOS/Blender");
            fs::create_dir_all(path.parent().unwrap())?;
            fs::File::create(&path)?;
            path
        } else if cfg!(windows) {
            let path = root.join("blender.exe");
            fs::File::create(&path)?;
            path
        } else {
            // Linux and others
            let path = root.join("blender");
            fs::File::create(&path)?;
            path
        };

        let result = get_bin_path(root)?;

        assert_eq!(result, expected_bin);

        Ok(())
    }

    #[test]
    fn test_get_bin_path_not_found() -> Result<()> {
        let temp = tempdir()?;
        let root = temp.path();

        let result = get_bin_path(root);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Blender executable not found"));

        Ok(())
    }

    #[test]
    fn test_normalize_executable_path() {
        assert!(normalize_executable_path("   ").is_err());
        assert!(normalize_executable_path("relative/path").is_err());

        #[cfg(windows)]
        {
            assert_eq!(
                normalize_executable_path("/c/Program Files/Blender/blender.exe").unwrap(),
                PathBuf::from("C:\\Program Files\\Blender\\blender.exe")
            );
            assert_eq!(
                normalize_executable_path("\\c\\Blender\\blender.exe").unwrap(),
                PathBuf::from("C:\\Blender\\blender.exe")
            );
            assert_eq!(
                normalize_executable_path("C:/Blender/blender.exe").unwrap(),
                PathBuf::from("C:\\Blender\\blender.exe")
            );
        }

        #[cfg(not(windows))]
        {
            assert_eq!(
                normalize_executable_path("/usr/local/bin/blender").unwrap(),
                PathBuf::from("/usr/local/bin/blender")
            );
        }
    }
}
