use anyhow::{Result, bail};
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub struct Platform {
    pub os: String,
    pub arch: String,
    pub ext: String, // .zip, .tar.xz, .dmg
}

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
}