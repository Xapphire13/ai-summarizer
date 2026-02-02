use std::{collections::HashMap, fs, path::PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serenity::all::ChannelId;

const CONFIG_PATH: &str = "./config.toml";

#[derive(Serialize, Deserialize, Debug)]
pub struct ChannelConfig {
    pub name: String,
    /// Override for the global retention policy
    pub policy_days: Option<u32>,
    /// Pagination cursor: oldest message ID seen, next run fetches BEFORE this
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pagination_cursor: Option<u64>,
}

impl ChannelConfig {
    pub fn resolve_policy_days(&self, config: &Config) -> u32 {
        self.policy_days
            .unwrap_or(config.retention.default_policy_days)
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct RetentionConfig {
    pub default_policy_days: u32,
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

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    pub schedule_interal_seconds: u32,
    pub retention: RetentionConfig,
    #[serde(default)]
    pub media_backup: MediaBackupConfig,
    #[serde(default)]
    channels: HashMap<ChannelId, ChannelConfig>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config = if let Ok(bytes) = fs::read(CONFIG_PATH) {
            toml::from_slice(bytes.as_slice())?
        } else {
            Self::default()
        };

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        fs::write(CONFIG_PATH, toml::to_string_pretty(&self)?)?;

        Ok(())
    }

    pub fn add_channel_config(
        &mut self,
        channel_id: ChannelId,
        config: ChannelConfig,
    ) -> Result<()> {
        // Check if policy is becoming stricter (fewer days) - if so, clear pagination cursor
        if let Some(existing) = self.channels.get(&channel_id) {
            let old_days = existing.resolve_policy_days(self);
            let new_days = config
                .policy_days
                .unwrap_or(self.retention.default_policy_days);
            if new_days < old_days {
                // Policy is stricter, start fresh from newest messages
                let mut config = config;
                config.pagination_cursor = None;
                self.channels.insert(channel_id, config);
                return self.save();
            }
        }
        self.channels.insert(channel_id, config);
        self.save()
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
    pub fn enabled_channels(&self) -> Vec<(ChannelId, u32)> {
        self.channels
            .iter()
            .map(|(id, config)| (*id, config.resolve_policy_days(self)))
            .collect()
    }
}
