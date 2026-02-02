use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::Days;
use serenity::all::{ChannelId, GetMessages, Http, Timestamp};
use tokio::sync::watch;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::cancellation_registry::CancellationRegistry;
use crate::cleanup::queue::{BackupJob, DeleteJob, classify_messages, filter_expired_messages};
use crate::config::Config;
use crate::media::MediaDownloader;

// Note: Discord requires messages to be < 14 days old for bulk delete
// see (https://discord.com/developers/docs/resources/message#bulk-delete-messages).
const BULK_DELETE_THRESHOLD: Days = Days::new(14);
const BULK_DELETE_MIN: usize = 2;
const BULK_DELETE_MAX: usize = 100;
const SINGLE_DELETE_DELAY: Duration = Duration::from_millis(200);
const BULK_DELETE_DELAY: Duration = Duration::from_secs(1);
const MAX_MESSAGES_PER_FETCH: u8 = 100;
const TARGET_EXPIRED_MESSAGES: usize = 100;
const MAX_PAGINATION_ROUNDS: usize = 10;

/// Run cleanup for a single channel.
pub async fn cleanup_channel(
    http: Arc<Http>,
    config: Arc<Mutex<Config>>,
    cancellation: Arc<Mutex<CancellationRegistry>>,
    channel_id: ChannelId,
    retention_days: u32,
    cancel_rx: watch::Receiver<bool>,
) {
    let result = run_cleanup(http, config, channel_id, retention_days, cancel_rx).await;

    // Deregister cancellation token
    cancellation.lock().unwrap().deregister(channel_id);

    if let Err(e) = result {
        error!("Cleanup failed for channel {channel_id}: {e:?}");
    }
}

async fn run_cleanup(
    http: Arc<Http>,
    config: Arc<Mutex<Config>>,
    channel_id: ChannelId,
    retention_days: u32,
    mut cancel_rx: watch::Receiver<bool>,
) -> Result<()> {
    use serenity::all::{Message, MessageId};

    info!("Starting cleanup for channel {channel_id} (retention: {retention_days} days)");

    // Load pagination cursor from config
    let mut cursor: Option<MessageId> = config
        .lock()
        .unwrap()
        .get_pagination_cursor(channel_id)
        .map(MessageId::new);

    let mut expired_messages: Vec<Message> = Vec::new();
    let mut reached_end = false;

    // Pagination loop
    for round in 0..MAX_PAGINATION_ROUNDS {
        // Check cancellation
        if *cancel_rx.borrow() {
            info!("Cleanup cancelled for channel {channel_id}");
            return Ok(());
        }

        // Build request with pagination
        let request = match cursor {
            Some(before_id) => GetMessages::new()
                .limit(MAX_MESSAGES_PER_FETCH)
                .before(before_id),
            None => GetMessages::new().limit(MAX_MESSAGES_PER_FETCH),
        };

        debug!(
            "Pagination round {}: fetching messages before {}",
            round + 1,
            cursor.unwrap_or_default()
        );

        // Fetch messages
        let messages = channel_id
            .messages(&http, request)
            .await
            .context("Failed to fetch messages")?;

        if messages.is_empty() {
            debug!("No more messages in channel {channel_id}");
            reached_end = true;
            break;
        }

        debug!(
            "Fetched {} messages from channel {channel_id}",
            messages.len()
        );

        // Update cursor to oldest message in batch (last element, since messages are newest-first)
        if let Some(oldest) = messages.last() {
            cursor = Some(oldest.id);
        }

        // Check if we got a partial batch (indicates end of channel history)
        let batch_size = messages.len();
        if batch_size < MAX_MESSAGES_PER_FETCH as usize {
            reached_end = true;
        }

        // Filter expired messages and add to collection
        let batch_expired = filter_expired_messages(messages, retention_days);
        debug!("Found {} expired messages in batch", batch_expired.len());
        expired_messages.extend(batch_expired);

        // Check if we've collected enough
        if expired_messages.len() >= TARGET_EXPIRED_MESSAGES {
            expired_messages.truncate(TARGET_EXPIRED_MESSAGES);
            debug!(
                "Reached target of {} expired messages",
                TARGET_EXPIRED_MESSAGES
            );

            // Update cursor to oldest message in truncated batch
            if let Some(oldest) = expired_messages.last() {
                cursor = Some(oldest.id);
            }

            break;
        }

        if reached_end {
            break;
        }
    }

    // Save pagination state
    if reached_end {
        debug!("Reached end of channel history, clearing pagination cursor");
        config
            .lock()
            .unwrap()
            .set_pagination_cursor(channel_id, None)?;
    } else {
        debug!("Saving pagination cursor: {:?}", cursor);
        config
            .lock()
            .unwrap()
            .set_pagination_cursor(channel_id, cursor.map(|c| c.get()))?;
    }

    if expired_messages.is_empty() {
        info!("No expired messages in channel {channel_id}");
        return Ok(());
    }

    info!(
        "Found {} expired messages in channel {channel_id}",
        expired_messages.len()
    );

    // Classify into delete vs backup jobs
    let classified = classify_messages(expired_messages);
    info!(
        "Classified: {} delete jobs, {} backup jobs",
        classified.delete_jobs.len(),
        classified.backup_jobs.len()
    );

    // Check cancellation
    if *cancel_rx.borrow() {
        info!("Cleanup cancelled for channel {channel_id}");
        return Ok(());
    }

    // Process delete jobs (non-media messages)
    if !classified.delete_jobs.is_empty() {
        delete_messages(&http, channel_id, &classified.delete_jobs, &mut cancel_rx).await?;
    }

    // Check cancellation again
    if *cancel_rx.borrow() {
        info!("Cleanup cancelled for channel {channel_id}");
        return Ok(());
    }

    // Process backup jobs (media messages)
    if !classified.backup_jobs.is_empty() {
        let download_dir = config.lock().unwrap().media_backup.download_dir.clone();

        process_backup_jobs(&http, download_dir, &classified.backup_jobs, &mut cancel_rx).await?;
    }

    info!("Cleanup completed for channel {channel_id}");

    Ok(())
}

/// Delete non-media messages with rate limiting.
async fn delete_messages(
    http: &Http,
    channel_id: ChannelId,
    jobs: &[DeleteJob],
    cancel_rx: &mut watch::Receiver<bool>,
) -> Result<()> {
    let bulk_delete_cutoff: Timestamp = Timestamp::now()
        .checked_sub_days(BULK_DELETE_THRESHOLD)
        .context("can't compute bulk delete cutoff")?
        .into();
    let (mut bulk_jobs, mut individual_jobs): (Vec<_>, Vec<_>) = jobs
        .into_iter()
        .partition(|j| j.message_id.created_at() <= bulk_delete_cutoff);

    if bulk_jobs.len() < BULK_DELETE_MIN {
        individual_jobs.extend(bulk_jobs.drain(..));
    }

    if !bulk_jobs.is_empty() {
        let chunks: Vec<_> = bulk_jobs.chunks(BULK_DELETE_MAX).collect();

        for chunk in chunks {
            if *cancel_rx.borrow() {
                return Ok(());
            }

            if let Err(e) = channel_id
                .delete_messages(http, chunk.iter().map(|f| f.message_id))
                .await
            {
                warn!("Bulk delete failed: {e:?}",);
            } else {
                info!(
                    "Bulk deleted {} messages from channel {channel_id}",
                    chunk.len(),
                );
            }

            sleep(BULK_DELETE_DELAY).await;
        }
    }

    if !individual_jobs.is_empty() {
        for job in jobs {
            if *cancel_rx.borrow() {
                return Ok(());
            }

            if let Err(e) = channel_id.delete_message(http, job.message_id).await {
                error!("Failed to delete message {}: {e:?}", job.message_id);
            } else {
                debug!("Deleted message {}", job.message_id);
            }

            sleep(SINGLE_DELETE_DELAY).await;
        }
    }

    Ok(())
}

/// Process backup jobs: download media locally, mark as pending backup.
/// Messages with media are NOT deleted until OneDrive upload is implemented.
async fn process_backup_jobs(
    _http: &Http,
    download_dir: std::path::PathBuf,
    jobs: &[BackupJob],
    cancel_rx: &mut watch::Receiver<bool>,
) -> Result<()> {
    let downloader = MediaDownloader::new(download_dir);

    for job in jobs {
        if *cancel_rx.borrow() {
            return Ok(());
        }

        info!(
            "Processing media backup for message {} ({} attachments)",
            job.message_id,
            job.attachments.len()
        );

        match downloader
            .download_attachments(job.message_id, job.timestamp, &job.attachments)
            .await
        {
            Ok(results) => {
                info!(
                    "Downloaded {} files for message {}",
                    results.len(),
                    job.message_id
                );

                // TODO: Track in pending_backups.toml for persistence
                // TODO: When OneDrive upload is implemented:
                //   1. Upload files to OneDrive
                //   2. On success, delete the Discord message
                //   3. Optionally delete local files after confirmed upload

                // For now, we do NOT delete the message - it stays in Discord
                // until OneDrive upload is working
                warn!(
                    "Message {} has media pending cloud backup - NOT deleting",
                    job.message_id
                );
            }
            Err(e) => {
                error!(
                    "Failed to download media for message {}: {e:?}",
                    job.message_id
                );
            }
        }
    }

    Ok(())
}
