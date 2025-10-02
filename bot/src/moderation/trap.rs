//! Trap channel enforcement:
//! - Kick the member (fallback to timeout if hierarchy/perms block)
//! - Reply to the offending message
//! - Purge their recent messages via UserRecentIndex (default 60s)
//! - Notify #mod-info
//!
//! Env:
//! - TRAP_CHANNEL_ID        : u64 (required)
//! 
//! Permissions needed: Manage Messages, Kick Members, Moderate Members, Send Messages (#mod-info)

use std::env;
use std::str::FromStr;

use crate::moderation::notify_moderators;
use crate::moderation::purge::UserRecentIndex;
use crate::poke::pick_phrase;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::{Builder, CreateMessage};
use serenity::FullEvent;

const PHRASES: &[&str] = &[
    "Crio pokes you",
    "don’t touch what you can’t afford",
    "that’s Crio harassment, that is!",
    "poke me one more time and I’ll make you chum!",
    "don’t make me slap you with a mackerel! - Qweek!",
    "fishin’ for trouble, eh?",
    "do you *know* who I am?! - Qweek!",
    "I’ll report you to the Otter Council!",
    "I smell... treachery. Or herring. - Qweek!",
    "you should be ashamed.",
    "i'll tell MaoMao to ban you - Qweek!",
    "Crio will remember this. - Qweek!",
    "FISH RO DAH!",
    "Mao?  Mao?! MAAAAAOOOOO",
    "Haddocken!",
    "Criiiiiiiiiio Jenkins!",
    "I have come here to chew chum and fish bass, and I'm all out of chum",
    "I'm going to *Qweek* your ass!",
];

type Error = Box<dyn std::error::Error + Send + Sync>;

struct TrapConfig {
    channel_id: serenity::ChannelId,
    purge_window_secs: u64,
    fallback_timeout_min: u64,
}

impl TrapConfig {
    fn from_env() -> Option<Self> {
        let channel_id =
            serenity::ChannelId::new(u64::from_str(&env::var("TRAP_CHANNEL_ID").ok()?).ok()?);
        let purge_window_secs = env::var("TRAP_PURGE_WINDOW_S")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(60);
        let fallback_timeout_min = env::var("TRAP_FALLBACK_TIMEOUTM")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(9999); // Timeout if kick doesnt work
        Some(Self {
            channel_id,
            purge_window_secs,
            fallback_timeout_min,
        })
    }
}

pub async fn trap_event_handler(
    ctx: &serenity::Context,
    event: &FullEvent,
    index: &UserRecentIndex,
) -> Result<(), Error> {
    match event {
        FullEvent::Ready { data_about_bot, .. } => {
            if let Some(cfg) = TrapConfig::from_env() {
                tracing::info!(
                    "Trap active in channel {} as @{} — purge={}s, fallback_timeout={}m",
                    cfg.channel_id,
                    data_about_bot.user.name,
                    cfg.purge_window_secs,
                    cfg.fallback_timeout_min
                );
            } else {
                tracing::info!("Trap inactive: TRAP_CHANNEL_ID not set");
            }
        }
        FullEvent::Message { new_message } => {
            let Some(cfg) = TrapConfig::from_env() else {
                return Ok(());
            };
            if new_message.channel_id != cfg.channel_id {
                return Ok(());
            }
            if new_message.author.bot || new_message.author.id == ctx.cache.current_user().id {
                return Ok(());
            }
            let Some(guild_id) = new_message.guild_id else {
                return Ok(());
            };

            // 1) Kick (fallback to timeout if hierarchy/perms block)
            let mut action = "kick";
            if let Err(e) = guild_id
                .kick_with_reason(&ctx.http, new_message.author.id, "Spam")
                .await
            {
                tracing::warn!("Kick failed for {}: {:?}", new_message.author.id, e);
                action = "timeout";
                if let Err(e2) = timeout_member(
                    ctx,
                    guild_id,
                    new_message.author.id,
                    cfg.fallback_timeout_min,
                    "Trap channel violation",
                )
                    .await
                {
                    tracing::warn!(
                        "Timeout fallback failed for {}: {:?}",
                        new_message.author.id,
                        e2
                    );
                    action = "none";
                }
            }

            // 2) Reply to the offending message
            let reply = CreateMessage::new()
                .reference_message(new_message)
                .add_sticker_id(1411742747093766315) // Crio Lesner sticker
                .content(pick_phrase(PHRASES));
            if let Err(e) = reply
                .execute(ctx, (new_message.channel_id, new_message.guild_id))
                .await
            {
                tracing::warn!("Failed to reply: {e:?}");
            }

            // 3) Purge recent messages (guild-wide) using the index
            if let Err(e) = index
                .purge_recent(
                    &ctx.http,
                    new_message.author.id,
                    cfg.purge_window_secs,
                    None
                )
                .await
            {
                tracing::warn!("Purge after trap failed: {:?}", e);
            }

            // 4) Notify moderators
            let _ = notify_moderators(
                &ctx.http,
                format!(
                    "[TRAP] Action: **{}** | User: <@{}> | Channel: <#{}> | Purge window: {}s",
                    action, new_message.author.id, new_message.channel_id, cfg.purge_window_secs
                ),
            )
            .await;
        }
        _ => {}
    }
    Ok(())
}

async fn timeout_member(
    ctx: &serenity::Context,
    guild_id: serenity::GuildId,
    user_id: serenity::UserId,
    minutes: u64,
    _reason: &str,
) -> Result<(), Error> {
    use serenity::builder::EditMember;
    use serenity::model::timestamp::Timestamp;

    let until = chrono::Utc::now() + chrono::Duration::minutes(minutes as i64);
    let until_ts =
        Timestamp::from_unix_timestamp(until.timestamp()).unwrap_or_else(|_| Timestamp::now());

    guild_id
        .edit_member(
            &ctx.http,
            user_id,
            EditMember::default().disable_communication_until_datetime(until_ts),
        )
        .await?;
    Ok(())
}
