use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use poise::samples::register_in_guild;
use serenity::{Client, all::GatewayIntents};
use tracing::{error, info};

use crate::{
    cancellation::CancellationRegistry,
    command::{CommandData, cleanup},
    config::{Config, ConfigStore},
    scheduler::spawn_scheduler,
};

mod cancellation;
mod cleanup;
mod command;
mod config;
mod media;
mod scheduler;

#[tokio::main]
async fn main() -> Result<()> {
    shared::init_tracing!()?;
    let bot_config = shared::load_bot_config!()?;
    let config_store = ConfigStore::new(Config::load()?);
    let cancellation = Arc::new(Mutex::new(CancellationRegistry::new()));
    let intents = GatewayIntents::MESSAGE_CONTENT | GatewayIntents::GUILD_MESSAGES;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![cleanup()],
            ..Default::default()
        })
        .setup({
            let config_store = config_store.clone();
            let cancellation = Arc::clone(&cancellation);

            move |ctx, ready, framework| {
                let http = Arc::clone(&ctx.http);

                Box::pin(async move {
                    info!("Connected!");

                    for guild_id in &ready.guilds {
                        register_in_guild(ctx, &framework.options().commands, guild_id.id).await?;
                    }

                    // Spawn the cleanup scheduler
                    spawn_scheduler(
                        Arc::clone(&http),
                        config_store.clone(),
                        Arc::clone(&cancellation),
                    );

                    Ok(CommandData {
                        config: config_store,
                        cancellation,
                    })
                })
            }
        })
        .build();

    let mut client = Client::builder(&bot_config.discord_token, intents)
        .framework(framework)
        .await
        .context("Error creating client")?;

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }

    Ok(())
}
