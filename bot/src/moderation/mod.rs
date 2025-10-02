mod burst_guard;
mod debug_perms;
pub(crate) mod purge;
mod trap;

pub use burst_guard::*;
use poise::serenity_prelude::Error::Url;
use poise::serenity_prelude::{Builder, CacheHttp, ChannelId, Context, CreateMessage, EditMember, GuildId, Message, MessageId, MessageReference, MessageReferenceKind, Timestamp, UserId};
use poise::CreateReply;
pub use trap::*;

const MOD_INFO_CHANNEL_ID: u64 = 211092999835222017;

pub async fn forward_to_mod_info(
    http: impl CacheHttp,
    offending_message: &Message,
    content: impl Into<String>,
) -> poise::serenity_prelude::Result<Message> {
    let mod_channel_id = ChannelId::new(MOD_INFO_CHANNEL_ID);
    let forward = MessageReference::new(MessageReferenceKind::Forward, offending_message.channel_id).message_id(offending_message.id);

    mod_channel_id.send_message(&http, CreateMessage::new().reference_message(forward)).await;
    mod_channel_id.send_message(&http, CreateMessage::new().content(content)).await
}

type Error = Box<dyn std::error::Error + Send + Sync>;

async fn timeout_member(
    ctx: &Context,
    guild_id: GuildId,
    user_id: UserId,
    minutes: u64,
    reason: &str,
) -> Result<(), Error> {
    let until = chrono::Utc::now() + chrono::Duration::minutes(minutes as i64);
    let until_ts =
        Timestamp::from_unix_timestamp(until.timestamp()).unwrap_or_else(|_| Timestamp::now());

    guild_id
        .edit_member(
            &ctx.http,
            user_id,
            EditMember::default()
                .disable_communication_until_datetime(until_ts)
                .audit_log_reason(reason),
        )
        .await?;
    Ok(())
}
