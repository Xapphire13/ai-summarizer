use std::{
    collections::HashMap,
    fs,
    num::NonZeroU32,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serenity::all::ChannelId;

const CONFIG_PATH: &str = "./config.toml";
const CONFIG_TEMP_PATH: &str = "./config.toml.tmp";

#[derive(Serialize, Deserialize, Debug)]
pub struct ChannelConfig {
    pub name: String,
    /// Override for the global retention policy
    pub policy_days: Option<NonZeroU32>,
    /// Pagination cursor: oldest message ID seen, next run fetches BEFORE this
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pagination_cursor: Option<u64>,
}

impl ChannelConfig {
    pub fn resolve_policy_days(&self, config: &Config) -> NonZeroU32 {
        self.policy_days
            .unwrap_or(config.retention.default_policy_days)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RetentionConfig {
    pub default_policy_days: NonZeroU32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MediaBackupConfig {
    pub download_dir: PathBuf,
}

impl Default for MediaBackupConfig {
    fn default() -> Self {
        Self {
            download_dir: PathBuf::from("./media_backups"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub schedule_interval_seconds: NonZeroU32,
    pub retention: RetentionConfig,
    pub media_backup: MediaBackupConfig,
    #[serde(default)]
    channels: HashMap<ChannelId, ChannelConfig>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let bytes = fs::read(CONFIG_PATH)?;
        let config = toml::from_slice(bytes.as_slice())?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let content = toml::to_string_pretty(&self)?;
        fs::write(CONFIG_TEMP_PATH, &content)?;
        fs::rename(CONFIG_TEMP_PATH, CONFIG_PATH)?;
        Ok(())
    }

    pub fn add_channel_config(
        &mut self,
        channel_id: ChannelId,
        config: ChannelConfig,
    ) -> Result<NonZeroU32> {
        let new_days = config
            .policy_days
            .unwrap_or(self.retention.default_policy_days);

        // Check if policy is becoming stricter (fewer days) - if so, clear pagination cursor
        if let Some(existing) = self.channels.get(&channel_id) {
            let old_days = existing.resolve_policy_days(self);
            if new_days < old_days {
                // Policy is stricter, start fresh from newest messages
                let mut config = config;
                config.pagination_cursor = None;
                self.channels.insert(channel_id, config);
                self.save()?;
                return Ok(new_days);
            }
        }

        self.channels.insert(channel_id, config);
        self.save()?;
        Ok(new_days)
    }

    pub fn get_pagination_cursor(&self, channel_id: ChannelId) -> Option<u64> {
        self.channels
            .get(&channel_id)
            .and_then(|c| c.pagination_cursor)
    }

    pub fn set_pagination_cursor(
        &mut self,
        channel_id: ChannelId,
        cursor: Option<u64>,
    ) -> Result<()> {
        if let Some(config) = self.channels.get_mut(&channel_id) {
            config.pagination_cursor = cursor;
            self.save()?;
        }
        Ok(())
    }

    pub fn remove_channel(&mut self, channel_id: ChannelId) -> Result<()> {
        self.channels.remove(&channel_id);
        self.save()
    }

    /// Returns a list of all enabled channels with their resolved retention policies.
    pub fn enabled_channels(&self) -> Vec<(ChannelId, NonZeroU32)> {
        self.channels
            .iter()
            .map(|(id, config)| (*id, config.resolve_policy_days(self)))
            .collect()
    }
}

/// Thread-safe wrapper around Config for clean state management.
#[derive(Clone)]
pub struct ConfigStore {
    inner: Arc<Mutex<Config>>,
}

impl ConfigStore {
    pub fn new(config: Config) -> Self {
        Self {
            inner: Arc::new(Mutex::new(config)),
        }
    }

    /// Returns the schedule interval in seconds.
    pub fn schedule_interval_seconds(&self) -> NonZeroU32 {
        self.inner.lock().unwrap().schedule_interval_seconds
    }

    /// Returns a list of all enabled channels with their resolved retention policies.
    pub fn enabled_channels(&self) -> Vec<(ChannelId, NonZeroU32)> {
        self.inner.lock().unwrap().enabled_channels()
    }

    /// Returns the media backup configuration.
    pub fn media_backup_config(&self) -> MediaBackupConfig {
        self.inner.lock().unwrap().media_backup.clone()
    }

    /// Adds or updates a channel configuration.
    /// Returns the resolved policy days for the channel.
    pub fn add_channel(&self, channel_id: ChannelId, config: ChannelConfig) -> Result<NonZeroU32> {
        self.inner
            .lock()
            .unwrap()
            .add_channel_config(channel_id, config)
    }

    /// Removes a channel from the configuration.
    pub fn remove_channel(&self, channel_id: ChannelId) -> Result<()> {
        self.inner.lock().unwrap().remove_channel(channel_id)
    }

    /// Gets the pagination cursor for a channel.
    pub fn get_pagination_cursor(&self, channel_id: ChannelId) -> Option<u64> {
        self.inner.lock().unwrap().get_pagination_cursor(channel_id)
    }

    /// Sets the pagination cursor for a channel.
    pub fn set_pagination_cursor(&self, channel_id: ChannelId, cursor: Option<u64>) -> Result<()> {
        self.inner
            .lock()
            .unwrap()
            .set_pagination_cursor(channel_id, cursor)
    }
}
