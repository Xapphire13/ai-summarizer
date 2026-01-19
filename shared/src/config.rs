use std::{env, path::Path};

use anyhow::{Context, Result};

pub struct BotConfig {
    /// Token allowing bot to connect bot to Discord
    pub discord_token: String,
}

impl BotConfig {
    pub fn load(manifest_dir: &Path) -> Result<Self> {
        #[cfg(debug_assertions)]
        dotenvy::from_path(manifest_dir.join(".env")).context("Can't find .env file")?;

        Ok(Self {
            discord_token: env::var("DISCORD_TOKEN")
                .context("Expected DISCORD_TOKEN in environment")?,
        })
    }
}

/// Load bot config using the calling crate's manifest directory.
#[macro_export]
macro_rules! load_bot_config {
    () => {
        $crate::config::BotConfig::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")))
    };
}
