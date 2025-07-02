use crate::{Context, Error};
use poise::serenity_prelude::CreateMessage;
use poise::CreateReply;

/// Crio shares his wisdom
#[poise::command(slash_command, prefix_command)]
pub async fn talk(
    ctx: Context<'_>,
    #[description = "What's on Crio's mind?"] wisdom: String,
) -> Result<(), Error> {
    let mut message_content = String::from("Qweek!");
    if let Some(guild_id) = ctx.guild_id() {
        if guild_id == 161861855332139008 {
            message_content = wisdom;
        }
    }

    let create_message = CreateMessage::new().content(message_content);
    ctx.channel_id().send_message(ctx, create_message).await?;
    ctx.send(CreateReply::default().ephemeral(true).content("Qweek."))
        .await?;
    Ok(())
}
