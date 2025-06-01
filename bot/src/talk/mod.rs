use crate::{Context, Error};
use poise::serenity_prelude::CreateMessage;
use poise::CreateReply;

/// Crio shares his wisdom
#[poise::command(slash_command, prefix_command)]
pub async fn talk(
    ctx: Context<'_>,
    #[description = "What's on Crio's mind?"] wisdom: String,
) -> Result<(), Error> {
    let create_message = CreateMessage::new().content(wisdom);
    ctx.channel_id().send_message(ctx, create_message).await?;
    ctx.send(CreateReply::default().ephemeral(true).content("Qweek."))
        .await?;
    Ok(())
}
