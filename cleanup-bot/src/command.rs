use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};

use anyhow::{Error, Result};
use indoc::formatdoc;
use serenity::all::Mentionable;

use crate::cancellation::CancellationRegistry;
use crate::config::{ChannelConfig, ConfigStore};

pub struct CommandData {
    pub config: ConfigStore,
    pub cancellation: Arc<Mutex<CancellationRegistry>>,
}

type Context<'a> = poise::Context<'a, CommandData, Error>;

#[poise::command(slash_command, subcommands("enable", "disable"))]
pub async fn cleanup(_ctx: Context<'_>) -> Result<()> {
    Ok(())
}

#[poise::command(slash_command)]
pub async fn enable(
    ctx: Context<'_>,
    #[description = "How many days should messages be retained"]
    #[min = 1]
    policy_days: Option<NonZeroU32>,
) -> Result<()> {
    let channel_config = ChannelConfig {
        name: ctx.channel_id().name(&ctx.http()).await?,
        policy_days,
        pagination_cursor: None,
    };

    let policy_days = ctx
        .data()
        .config
        .add_channel(ctx.channel_id(), channel_config)?;

    ctx.say(formatdoc! {"
        Enabled cleanup for {channel}
        Retention policy: **{policy_days} {day_suffix}**
        ",
        channel = ctx.channel_id().mention(),
        day_suffix = if policy_days.get() == 1 {"day"}  else {"days"}
    })
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn disable(ctx: Context<'_>) -> Result<()> {
    ctx.data().config.remove_channel(ctx.channel_id())?;

    // Cancel any running cleanup task for the channel
    let was_running = ctx
        .data()
        .cancellation
        .lock()
        .unwrap()
        .cancel(ctx.channel_id());

    let mut message = format!(
        "Disabled cleanup for {channel}",
        channel = ctx.channel_id().mention()
    );

    if was_running {
        message.push_str("\n_Cancelled running cleanup task._");
    }

    ctx.say(message).await?;
    Ok(())
}
