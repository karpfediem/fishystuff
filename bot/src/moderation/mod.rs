pub(crate) mod purge;
mod burst_guard;
mod trap;
mod debug_perms;

use poise::serenity_prelude::{ChannelId, CacheHttp, EditMember, Message, Timestamp, Context, GuildId, UserId};
pub use burst_guard::*;
pub use trap::*;


const MOD_INFO_CHANNEL_ID: u64 = 211092999835222017;


pub async fn notify_moderators(http: impl CacheHttp, content: impl Into<String>) -> poise::serenity_prelude::Result<Message> {
    ChannelId::new(MOD_INFO_CHANNEL_ID).say(http, content).await
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
    let until_ts = Timestamp::from_unix_timestamp(until.timestamp())
        .unwrap_or_else(|_| Timestamp::now());

    guild_id.edit_member(&ctx.http, user_id, EditMember::default().disable_communication_until_datetime(until_ts).audit_log_reason(reason)).await?;
    Ok(())
}