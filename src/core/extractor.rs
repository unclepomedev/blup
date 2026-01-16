use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Component, Path, PathBuf};

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

fn sanitize_and_strip_path(path: &Path) -> Result<Option<PathBuf>> {
    let mut components = path.components();

    if components.next().is_none() {
        return Ok(None);
    }

    let mut sanitized = PathBuf::new();

    for component in components {
        match component {
            Component::Normal(c) => sanitized.push(c),
            Component::CurDir => continue,
            Component::ParentDir => {
                bail!("Security violation: Path traversal attempted: {:?}", path);
            }
            Component::RootDir | Component::Prefix(_) => {
                bail!(
                    "Security violation: Absolute path or prefix detected: {:?}",
                    path
                );
            }
        }
    }

    if sanitized.as_os_str().is_empty() {
        Ok(None)
    } else {
        Ok(Some(sanitized))
    }
}

fn extract_zip(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let file = fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        let raw_path = match file.enclosed_name() {
            Some(p) => p.to_owned(),
            None => file.mangled_name(),
        };

        let stripped_path = match sanitize_and_strip_path(&raw_path)? {
            Some(p) => p,
            None => continue,
        };

        let outpath = dest_dir.join(stripped_path);

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent()
                && !p.exists()
            {
                fs::create_dir_all(p)?;
            }
            let mut outfile = fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
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

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_path_buf();

        let stripped_path = match sanitize_and_strip_path(&path)? {
            Some(p) => p,
            None => continue,
        };

        let outpath = dest_dir.join(stripped_path);

        if let Some(p) = outpath.parent()
            && !p.exists()
        {
            fs::create_dir_all(p)?;
        }

        entry
            .unpack(&outpath)
            .context("Failed to unpack file from tar")?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn extract_dmg(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    use std::process::Command;
    use tempfile::TempDir;

    if !dest_dir.exists() {
        fs::create_dir_all(dest_dir)
            .context("Failed to create destination directory for DMG extraction")?;
    }

    let mount_dir = TempDir::new().context("Failed to create temp dir for mounting")?;
    let mount_point = mount_dir.path().to_path_buf();

    let status = Command::new("hdiutil")
        .args(["attach", "-nobrowse", "-readonly", "-mountpoint"])
        .arg(&mount_point)
        .arg(archive_path)
        .status()
        .context("Failed to execute hdiutil attach")?;

    if !status.success() {
        bail!("hdiutil failed to mount the DMG");
    }

    let _guard = DmgGuard(&mount_point);

    let app_source = mount_point.join("Blender.app");
    if !app_source.exists() {
        bail!("Blender.app not found in DMG");
    }

    let copy_status = Command::new("cp")
        .arg("-R")
        .arg(&app_source)
        .arg(dest_dir)
        .status()
        .context("Failed to execute cp command")?;

    if !copy_status.success() {
        bail!("Failed to copy Blender.app from DMG");
    }

    Ok(())
}

#[cfg(target_os = "macos")]
struct DmgGuard<'a>(&'a Path);

#[cfg(target_os = "macos")]
impl<'a> Drop for DmgGuard<'a> {
    fn drop(&mut self) {
        use std::process::Command;
        let _ = Command::new("hdiutil")
            .args(["detach", "-force"])
            .arg(self.0)
            .status();
    }
}

#[cfg(not(target_os = "macos"))]
fn extract_dmg(_: &Path, _: &Path) -> Result<()> {
    bail!("DMG extraction is only supported on macOS");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;
    use zip::write::FileOptions;

    #[test]
    fn test_extract_zip_strips_root() -> Result<()> {
        let temp_dir = tempdir()?;
        let archive_path = temp_dir.path().join("test.zip");
        let dest_dir = temp_dir.path().join("out");

        {
            let file = fs::File::create(&archive_path)?;
            let mut zip = zip::ZipWriter::new(file);
            let options = FileOptions::<()>::default();

            zip.start_file("RootFolder/file.txt", options)?;
            zip.write_all(b"content")?;

            zip.start_file("RootFolder/subdir/nested.txt", options)?;
            zip.write_all(b"nested")?;

            zip.finish()?;
        }

        extract(&archive_path, &dest_dir)?;

        let extracted_file = dest_dir.join("file.txt");
        assert!(extracted_file.exists());
        assert_eq!(fs::read_to_string(extracted_file)?, "content");

        let nested_file = dest_dir.join("subdir/nested.txt");
        assert!(nested_file.exists());

        Ok(())
    }
}
