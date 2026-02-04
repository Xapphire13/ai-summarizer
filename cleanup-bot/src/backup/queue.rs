use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

const PENDING_BACKUPS_PATH: &str = "./pending_backups.toml";
const PENDING_BACKUPS_TEMP_PATH: &str = "./pending_backups.toml.tmp";

/// Status of a pending backup.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum BackupStatus {
    Pending,
    InProgress,
    Failed { error: String },
}

/// A backup that is pending cloud upload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingBackup {
    pub message_id: u64,
    pub channel_id: u64,
    pub local_path: PathBuf,
    pub original_filename: String,
    pub timestamp: DateTime<Utc>,
    pub retry_count: u32,
    pub status: BackupStatus,
}

/// Persistent queue for tracking pending backups.
#[derive(Debug, Serialize, Deserialize)]
pub struct BackupQueue {
    entries: HashMap<String, PendingBackup>,
}

impl BackupQueue {
    /// Load the backup queue from disk, or create a new empty queue.
    pub fn load() -> Result<Self> {
        if let Ok(content) = fs::read_to_string(PENDING_BACKUPS_PATH) {
            let mut queue: BackupQueue = toml::from_str(&content)
                .context(format!("Failed to parse {}", PENDING_BACKUPS_PATH))?;

            queue.entries.iter_mut().for_each(|(_, entry)| {
                if entry.status == BackupStatus::InProgress {
                    // If we're loading the list and it has InProgress items, that means the process
                    // shut down during upload, reset status to pending
                    entry.status = BackupStatus::Pending;
                }
            });

            Ok(queue)
        } else {
            Ok(Self {
                entries: HashMap::new(),
            })
        }
    }

    /// Add a backup to the queue.
    pub fn add(&mut self, backup: PendingBackup) -> Result<()> {
        let key = backup.local_path.to_string_lossy().to_string();
        self.entries.insert(key, backup);
        self.save()
    }

    /// Remove a backup from the queue by its local path.
    pub fn remove(&mut self, local_path: &Path) -> Result<()> {
        let key = local_path.to_string_lossy().to_string();
        self.entries.remove(&key);
        self.save()
    }

    /// Get all pending backups (status == Pending).
    pub fn get_pending(&self) -> Vec<&PendingBackup> {
        self.entries
            .values()
            .filter(|b| matches!(b.status, BackupStatus::Pending))
            .collect()
    }

    /// Get all failed backups that haven't exceeded max retries.
    pub fn get_failed(&self, max_retries: u32) -> Vec<&PendingBackup> {
        self.entries
            .values()
            .filter(|b| {
                matches!(b.status, BackupStatus::Failed { .. }) && b.retry_count < max_retries
            })
            .collect()
    }

    /// Mark a backup as in progress.
    pub fn mark_in_progress(&mut self, local_path: &Path) -> Result<()> {
        let key = local_path.to_string_lossy().to_string();
        if let Some(backup) = self.entries.get_mut(&key) {
            backup.status = BackupStatus::InProgress;
            self.save()?;
        }
        Ok(())
    }

    /// Mark a backup as failed with an error message.
    pub fn mark_failed(&mut self, local_path: &Path, error: String) -> Result<()> {
        let key = local_path.to_string_lossy().to_string();
        if let Some(backup) = self.entries.get_mut(&key) {
            backup.status = BackupStatus::Failed { error };
            backup.retry_count += 1;
            self.save()?;
        }
        Ok(())
    }

    /// Reset a failed backup to pending for retry.
    pub fn reset_to_pending(&mut self, local_path: &Path) -> Result<()> {
        let key = local_path.to_string_lossy().to_string();
        if let Some(backup) = self.entries.get_mut(&key) {
            backup.status = BackupStatus::Pending;
            self.save()?;
        }
        Ok(())
    }

    /// Get a backup by its local path.
    pub fn get(&self, local_path: &Path) -> Option<&PendingBackup> {
        let key = local_path.to_string_lossy().to_string();
        self.entries.get(&key)
    }

    /// Save the queue to disk atomically (write to temp file, then rename).
    fn save(&self) -> Result<()> {
        let content = toml::to_string_pretty(&self)?;
        let temp_path = PathBuf::from(PENDING_BACKUPS_TEMP_PATH);
        fs::write(&temp_path, &content).context("Failed to write temp backup queue file")?;
        fs::rename(&temp_path, PENDING_BACKUPS_PATH)
            .context("Failed to rename backup queue file")?;
        Ok(())
    }
}
