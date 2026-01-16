use anyhow::{Context, Result, bail};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub fn extract(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let path_str = archive_path.to_string_lossy();

    if path_str.ends_with(".zip") {
        extract_zip(archive_path, dest_dir)
    } else if path_str.ends_with(".tar.xz") {
        extract_tar_xz(archive_path, dest_dir)
    } else if path_str.ends_with(".dmg") {
        extract_dmg(archive_path, dest_dir)
    } else {
        bail!("Unsupported archive format: {:?}", archive_path);
    }
}

fn extract_zip(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let file = fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => dest_dir.join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }

        #[cfg(unix)]
        {
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
            }
        }
    }
    Ok(())
}

fn extract_tar_xz(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let file = fs::File::open(archive_path)?;
    let tar = xz2::read::XzDecoder::new(file);
    let mut archive = tar::Archive::new(tar);

    archive
        .unpack(dest_dir)
        .context("Failed to unpack tar.xz archive")?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn extract_dmg(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    use std::process::Command;
    use tempfile::TempDir;

    let mount_dir = TempDir::new().context("Failed to create temp dir for mounting")?;
    let mount_point = mount_dir.path();

    let status = Command::new("hdiutil")
        .args(&["attach", "-nobrowse", "-readonly", "-mountpoint"])
        .arg(mount_point)
        .arg(archive_path)
        .status()
        .context("Failed to execute hdiutil attach")?;

    if !status.success() {
        bail!("hdiutil failed to mount the DMG");
    }

    let app_source = mount_point.join("Blender.app");

    if app_source.exists() {
        let copy_status = Command::new("cp")
            .arg("-R")
            .arg(&app_source)
            .arg(dest_dir)
            .status()
            .context("Failed to execute cp command")?;

        if !copy_status.success() {
            let _ = Command::new("hdiutil")
                .args(&["detach"])
                .arg(mount_point)
                .status();
            bail!("Failed to copy Blender.app from DMG");
        }
    } else {
        let _ = Command::new("hdiutil")
            .args(&["detach"])
            .arg(mount_point)
            .status();
        bail!("Blender.app not found in DMG");
    }

    let detach_status = Command::new("hdiutil")
        .args(&["detach", "-force"])
        .arg(mount_point)
        .status()
        .context("Failed to execute hdiutil detach")?;

    if !detach_status.success() {
        eprintln!("Warning: Failed to detach DMG at {:?}", mount_point);
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn extract_dmg(_: &Path, _: &Path) -> Result<()> {
    bail!("DMG extraction is only supported on macOS");
}
