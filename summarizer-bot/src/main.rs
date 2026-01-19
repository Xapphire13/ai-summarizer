use ::tracing::error;
use anyhow::{Context, Result};
use serenity::prelude::*;

use crate::config::Config;
use crate::handler::Handler;
use crate::llm::SummaryGenerator;

mod config;
mod handler;
mod llm;

#[tokio::main]
async fn main() -> Result<()> {
    shared::init_tracing!()?;
    let config = Config::from_env()?;

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::DIRECT_MESSAGES;

    let summary_generator = SummaryGenerator::new(&config);
    let handler = Handler::new(summary_generator, &config);

    let mut client = Client::builder(&config.bot.discord_token, intents)
        .event_handler(handler)
        .await
        .context("Error creating client")?;

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }

    Ok(())
}
