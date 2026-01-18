use anyhow::{Context, Result, bail};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub async fn download_file(client: &Client, url: &str, dest_path: &Path) -> Result<()> {
    let res = client
        .get(url)
        .send()
        .await?
        .error_for_status()
        .context(format!("Failed to connect to {}", url))?;

    let total_size = res.content_length();

    let pb = create_progress_bar(total_size)?;

    let mut file = File::create(dest_path)
        .await
        .context(format!("Failed to create file at {:?}", dest_path))?;

    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.context("Error while downloading file")?;
        file.write_all(&chunk)
            .await
            .context("Error while writing to file")?;

        pb.inc(chunk.len() as u64);
    }

    file.flush().await?;
    pb.finish_with_message("Download complete 🚀");
    Ok(())
}

pub async fn verify_checksum(file_path: &Path, expected_checksum: &str) -> Result<()> {
    let file_path = file_path.to_path_buf();
    let expected_checksum = expected_checksum.to_string();

    tokio::task::spawn_blocking(move || verify_checksum_sync(&file_path, &expected_checksum))
        .await?
}

fn verify_checksum_sync(file_path: &Path, expected_checksum: &str) -> Result<()> {
    let mut file = std::fs::File::open(file_path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];

    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    let result = hasher.finalize();
    let calculated_checksum = hex::encode(result);

    if calculated_checksum.eq_ignore_ascii_case(expected_checksum) {
        Ok(())
    } else {
        bail!(
            "Checksum mismatch!\nExpected: {}\nActual:   {}",
            expected_checksum,
            calculated_checksum
        )
    }
}

pub fn find_checksum_in_list(list_content: &str, target_filename: &str) -> Option<String> {
    for line in list_content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let hash = parts[0];
            let filename = parts[1];

            if filename == target_filename && hash.len() == 64 {
                return Some(hash.to_string());
            }
        }
    }
    None
}

fn create_progress_bar(len: Option<u64>) -> Result<ProgressBar> {
    let pb = match len {
        Some(len) => {
            let pb = ProgressBar::new(len);
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
                .progress_chars("#>-"));
            pb
        }
        None => {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} [{elapsed_precise}] {bytes} downloaded")?,
            );
            pb
        }
    };
    Ok(pb)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_verify_checksum_match() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        write!(temp_file, "hello world")?;

        let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";

        verify_checksum(temp_file.path(), expected).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_verify_checksum_mismatch() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        write!(temp_file, "malicious content")?;

        let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";

        let result = verify_checksum(temp_file.path(), expected).await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Checksum mismatch"));

        Ok(())
    }

    #[test]
    fn test_find_checksum_in_list() {
        let content = r#"
102a81ddee5346c96339c6a529069a2d52df05f330eb9bfd431c8dd79fb4afb6  blender-5.0.1-macos-arm64.dmg
8019580ee1b7262e505f4196a00237ccf743c88d205b38d34201510676e60b09  blender-5.0.1-linux-x64.tar.xz
651c29a8b99be806768d6fb949505d280543bd6904b5c4aa1367306b9d9702bc  blender-5.0.1-windows-x64.msi
"#;

        let hash = find_checksum_in_list(content, "blender-5.0.1-linux-x64.tar.xz");
        assert_eq!(
            hash,
            Some("8019580ee1b7262e505f4196a00237ccf743c88d205b38d34201510676e60b09".to_string())
        );

        let hash_none = find_checksum_in_list(content, "blender-9.9.9-fake.zip");
        assert_eq!(hash_none, None);

        let hash_partial = find_checksum_in_list(content, "blender-5.0.1-linux-x64");
        assert_eq!(hash_partial, None);
    }
}
