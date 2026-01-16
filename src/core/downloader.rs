use anyhow::{Context, Result};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
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
