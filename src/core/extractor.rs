use anyhow::{Context, Result};
use std::fs;
use std::io;
use std::path::Path;

pub fn extract(archive_path: &Path, dest_dir: &Path) -> Result<()> {
    let path_str = archive_path.to_string_lossy();

    if path_str.ends_with(".zip") {
        extract_zip(archive_path, dest_dir)
    } else if path_str.ends_with(".tar.xz") {
        extract_tar_xz(archive_path, dest_dir)
    } else {
        anyhow::bail!("Unsupported archive format: {:?}", archive_path);
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
            io::copy(&mut file, &mut outfile)?;
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

    archive
        .unpack(dest_dir)
        .context("Failed to unpack tar.xz archive")?;
    Ok(())
}
