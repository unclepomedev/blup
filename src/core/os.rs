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
