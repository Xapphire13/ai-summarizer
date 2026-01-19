use anyhow::{Error, Result};
use serenity::all::Mentionable;

type Context<'a> = poise::Context<'a, (), Error>;

#[poise::command(slash_command, subcommands("enable", "disable"))]
pub async fn cleanup(_ctx: Context<'_>) -> Result<()> {
    Ok(())
}

#[poise::command(slash_command)]
pub async fn enable(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say(format!("Enable command for {}", ctx.channel_id().mention()))
        .await?;
    Ok(())
}

#[poise::command(slash_command)]
pub async fn disable(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say(format!(
        "Disable command for {}",
        ctx.channel_id().mention()
    ))
    .await?;
    Ok(())
}
