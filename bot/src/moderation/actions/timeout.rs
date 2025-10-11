use crate::Error;
use poise::serenity_prelude::{Context, EditMember, GuildId, Timestamp, UserId};

pub(crate) async fn timeout_member(
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
