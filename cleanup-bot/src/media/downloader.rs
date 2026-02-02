use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use reqwest::Client;
use serenity::all::MessageId;
use tokio::{fs, io::AsyncWriteExt};
use tracing::{debug, info};

use crate::extensions::MediaAttachment;

/// Downloads media attachments to the local filesystem.
pub struct MediaDownloader {
    client: Client,
    base_dir: PathBuf,
}

/// Result of a successful download.
#[derive(Debug, Clone)]
pub struct DownloadResult;

impl MediaDownloader {
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            client: Client::new(),
            base_dir,
        }
    }

    /// Download all media attachments for a message.
    /// Returns the local paths where files were saved.
    pub async fn download_attachments(
        &self,
        message_id: MessageId,
        timestamp: DateTime<Utc>,
        attachments: &[MediaAttachment],
    ) -> Result<Vec<DownloadResult>> {
        let dir = self.get_download_dir(timestamp);
        fs::create_dir_all(&dir)
            .await
            .context("Failed to create download directory")?;

        let mut results = Vec::with_capacity(attachments.len());

        for attachment in attachments {
            let result = self
                .download_attachment(&dir, message_id, attachment)
                .await
                .with_context(|| format!("Failed to download {}", attachment.filename))?;
            results.push(result);
        }

        Ok(results)
    }

    /// Get the download directory path for a date.
    /// Format: base_dir/YYYY-MM-DD/
    fn get_download_dir(&self, timestamp: DateTime<Utc>) -> PathBuf {
        let date_str = timestamp.format("%Y-%m-%d").to_string();
        self.base_dir.join(date_str)
    }

    /// Download an attachment.
    async fn download_attachment(
        &self,
        dir: &Path,
        message_id: MessageId,
        attachment: &MediaAttachment,
    ) -> Result<DownloadResult> {
        // Prefix filename with message ID to avoid collisions
        let filename = format!("{}_{}", message_id, attachment.filename);
        let path = dir.join(&filename);

        debug!("Downloading {} to {path:?}", attachment.url);

        let response = self
            .client
            .get(&attachment.url)
            .send()
            .await
            .context("HTTP request failed")?
            .error_for_status()
            .context("HTTP error response")?;

        let mut file = fs::File::create(&path)
            .await
            .context("Failed to create file")?;

        let mut stream = response.bytes_stream();
        let mut bytes_written: u64 = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Failed to read response chunk")?;
            file.write_all(&chunk)
                .await
                .context("Failed to write to file")?;
            bytes_written += chunk.len() as u64;
        }

        file.flush().await.context("Failed to flush file")?;

        info!(
            "Downloaded {} ({bytes_written} bytes) to {path:?}",
            attachment.filename,
        );

        Ok(DownloadResult)
    }
}
