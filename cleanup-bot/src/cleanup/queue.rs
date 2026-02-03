use std::num::NonZeroU32;

use serenity::all::{Message, MessageId};

use crate::media::{AttachmentsExt, MediaAttachment};

/// A message that should be deleted immediately (no media backup needed).
#[derive(Debug)]
pub struct DeleteJob {
    pub message_id: MessageId,
}

/// A message that needs media backup before deletion.
#[derive(Debug)]
pub struct BackupJob {
    pub message_id: MessageId,
    pub attachments: Vec<MediaAttachment>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Result of classifying messages for cleanup.
#[derive(Debug)]
pub struct ClassifiedMessages {
    /// Messages that can be deleted immediately (no media).
    pub delete_jobs: Vec<DeleteJob>,
    /// Messages that need media backup before deletion.
    pub backup_jobs: Vec<BackupJob>,
}

impl ClassifiedMessages {
    pub fn new() -> Self {
        Self {
            delete_jobs: Vec::new(),
            backup_jobs: Vec::new(),
        }
    }
}

/// Classify messages into delete jobs (no media) and backup jobs (has media).
pub fn classify_messages(messages: Vec<Message>) -> ClassifiedMessages {
    let mut result = ClassifiedMessages::new();

    for message in messages {
        let media_attachments = message.attachments.extract_media();

        if media_attachments.is_empty() {
            result.delete_jobs.push(DeleteJob {
                message_id: message.id,
            });
        } else {
            result.backup_jobs.push(BackupJob {
                message_id: message.id,
                attachments: media_attachments,
                timestamp: *message.timestamp,
            });
        }
    }

    result
}

/// Filter messages to only those older than the retention cutoff.
pub fn filter_expired_messages(messages: Vec<Message>, retention_days: NonZeroU32) -> Vec<Message> {
    let cutoff = chrono::Utc::now() - chrono::Duration::days(retention_days.get() as i64);

    messages
        .into_iter()
        .filter(|m| *m.timestamp < cutoff)
        .collect()
}
