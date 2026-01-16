use anyhow::{bail, Result};
use std::env;

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
