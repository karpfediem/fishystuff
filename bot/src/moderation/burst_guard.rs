//! Burst + Single-Channel Spam enforcement.
//!
//! Triggers on either:
//!   A) Cross-channel burst: user posts in >= MIN_CHANNELS distinct channels within WINDOW_SECS,
//!      and >= MIN_MESSAGES total within that window.
//!      -> Kick (fallback to timeout) -> Reply (sticker message text) -> Notify mods (forward) -> Purge via index
//!
//!   B) Single-channel spam: user posts >= CHAN_SPAM_MIN_MESSAGES in the same channel within
//!      CHAN_SPAM_WINDOW_SECS.
//!      -> Timeout X minutes -> Reply (sticker message text) -> Notify mods (forward) -> Purge via index
//!
//! Permissions: Manage Messages, Kick Members, Moderate Members, Send Messages (#mod-info)

use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use poise::serenity_prelude as serenity;
use poise::serenity_prelude::{Builder, CreateMessage};
use serenity::FullEvent;
use tokio::sync::RwLock;
use crate::moderation::{forward_to_mod_info, timeout_member, Error};
use crate::poke::pick_phrase;

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


#[derive(Clone)]
pub struct BurstState {
    // Separate cooldowns for cross-channel and single-channel paths
    last_cross_trigger: Arc<RwLock<HashMap<serenity::UserId, i64>>>,
    last_chan_trigger: Arc<RwLock<HashMap<serenity::UserId, i64>>>,
    cfg: Arc<BurstConfig>,
}

#[derive(Debug, Clone)]
struct BurstConfig {
    // Cross-channel
    window_secs: u64,
    min_channels: u64,
    min_messages: u64,
    cooldown_secs: u64,
    purge_window_secs: u64,
    timeout_minutes: u64, // fallback when kick fails

    // Single-channel
    chan_window_secs: u64,
    chan_min_messages: u64,
    chan_cooldown_secs: u64,
    chan_timeout_minutes: u64,
}

impl Default for BurstConfig {
    fn default() -> Self {
        let burst_timeout_min = env::var("BURST_TIMEOUT_MINUTES").ok().and_then(|s| s.parse().ok()).unwrap_or(60);
        Self {
            // Cross-channel
            window_secs: env::var("BURST_WINDOW_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(5),
            min_channels: env::var("BURST_MIN_CHANNELS").ok().and_then(|s| s.parse().ok()).unwrap_or(3),
            min_messages: env::var("BURST_MIN_MESSAGES").ok().and_then(|s| s.parse().ok()).unwrap_or(3),
            cooldown_secs: env::var("BURST_COOLDOWN_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(30),
            purge_window_secs: env::var("BURST_PURGE_WINDOW_S").ok().and_then(|s| s.parse().ok()).unwrap_or(60),
            timeout_minutes: burst_timeout_min,

            // Single-channel
            chan_window_secs: env::var("CHAN_SPAM_WINDOW_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(4),
            chan_min_messages: env::var("CHAN_SPAM_MIN_MESSAGES").ok().and_then(|s| s.parse().ok()).unwrap_or(8),
            chan_cooldown_secs: env::var("CHAN_SPAM_COOLDOWN_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(30),
            chan_timeout_minutes: env::var("CHAN_SPAM_TIMEOUT_MINUTES").ok().and_then(|s| s.parse().ok()).unwrap_or(5),
        }
    }
}

impl BurstState {
    pub fn new() -> Self {
        Self {
            last_cross_trigger: Arc::new(RwLock::new(HashMap::new())),
            last_chan_trigger: Arc::new(RwLock::new(HashMap::new())),
            cfg: Arc::new(BurstConfig::default()),
        }
    }
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Call from the global event handler AFTER recording the message into UserRecentIndex.
pub async fn burst_event_handler(
    ctx: &serenity::Context,
    event: &FullEvent,
    state: &BurstState,
    index: &crate::moderation::purge::UserRecentIndex,
) -> Result<(), Error> {
    let FullEvent::Message { new_message } = event else { return Ok(()); };
    if new_message.author.bot { return Ok(()); }
    let Some(guild_id) = new_message.guild_id else { return Ok(()); };

    // Gather recent activity for this author (use max of the two windows so we don't double-fetch)
    let window_for_fetch = state.cfg.window_secs.max(state.cfg.chan_window_secs);
    let per_channel = index.collect_since(new_message.author.id, window_for_fetch).await;
    if per_channel.is_empty() { return Ok(()); }

    // ----- A) Cross-channel burst -----
    let distinct_channels = per_channel.len() as u64;
    let total_msgs: u64 = per_channel.values().map(|v| v.len() as u64).sum();

    if distinct_channels >= state.cfg.min_channels && total_msgs >= state.cfg.min_messages {
        // cooldown
        let now = now_unix();
        {
            let map = state.last_cross_trigger.read().await;
            if let Some(&last) = map.get(&new_message.author.id) {
                if now - last < state.cfg.cooldown_secs as i64 { return Ok(()); }
            }
        }
        { state.last_cross_trigger.write().await.insert(new_message.author.id, now); }

        // 1) Kick (fallback to timeout)
        let mut action = "kick";
        if let Err(e) = guild_id.kick_with_reason(&ctx.http, new_message.author.id, "Spam").await {
            tracing::warn!("Kick failed for {}: {:?}", new_message.author.id, e);
            action = "timeout";
            if let Err(e2) = timeout_member(ctx, guild_id, new_message.author.id, state.cfg.timeout_minutes, "Spam: Burst across multiple channels").await {
                tracing::warn!("Timeout fallback failed for {}: {:?}", new_message.author.id, e2);
                action = "none";
            }
        }

        // 2) Reply to the triggering message (sticker message text)
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

        // 3) Notify moderators
        let action_notice =format!(
            "[BURST] Action: **{}** | User: <@{}> | Distinct channels: {} | Total msgs: {} | Window: {}s | Trigger in <#{}> | Purge Window: {}s ",
            action, new_message.author.id, distinct_channels, total_msgs, state.cfg.window_secs, new_message.channel_id, state.cfg.purge_window_secs
        );
        if let Err(e) = forward_to_mod_info(&ctx.http, new_message, &action_notice).await {
            tracing::warn!("Failed to notify mods: {:?} | Original notice: {}", e, action_notice);
        };


        // 4) Purge via index (include triggering message)
        if let Err(e) = index.purge_recent(&ctx.http, new_message.author.id, state.cfg.purge_window_secs, None).await {
            tracing::warn!("Purge after cross-channel burst failed: {:?}", e);
        }

        return Ok(());
    }

    // ----- B) Single-channel spam -----
    let per_channel_single = index.collect_since(new_message.author.id, state.cfg.chan_window_secs).await;
    if !per_channel_single.is_empty() {
        let (top_chan, top_count) = per_channel_single
            .iter()
            .map(|(ch, ids)| (*ch, ids.len() as u64))
            .max_by_key(|(_, c)| *c)
            .unwrap();

        if top_count >= state.cfg.chan_min_messages {
            // cooldown
            let now = now_unix();
            {
                let map = state.last_chan_trigger.read().await;
                if let Some(&last) = map.get(&new_message.author.id) {
                    if now - last < state.cfg.chan_cooldown_secs as i64 { return Ok(()); }
                }
            }
            { state.last_chan_trigger.write().await.insert(new_message.author.id, now); }

            // 1) Reply to the triggering message
            let _ = new_message.reply_ping(ctx, "**Please slow down**").await;

            // 2) Timeout (mute) for configured minutes
            if let Err(e) = timeout_member(
                ctx, guild_id, new_message.author.id, state.cfg.chan_timeout_minutes, "Spam: Rapid spam in a single channel"
            ).await {
                tracing::warn!("Timeout failed for {}: {:?}", new_message.author.id, e);
            }

            // 3) Notify moderators
            let action_notice = format!(
                "[SPAM] Muted <@{}> for {}m | {} msgs in <#{}> within {}s | Purge window: {}s",
                new_message.author.id,
                state.cfg.chan_timeout_minutes,
                top_count,
                top_chan,
                state.cfg.chan_window_secs,
                state.cfg.purge_window_secs
            );
            if let Err(e) = forward_to_mod_info(&ctx.http, new_message, &action_notice).await {
                tracing::warn!("Failed to notify mods: {:?} | Original notice: {}", e, action_notice);
            };

            // 4) Purge (use the broader purge window)
            if let Err(e) = index.purge_recent(&ctx.http, new_message.author.id, state.cfg.purge_window_secs, None).await {
                tracing::warn!("Purge after single-channel spam failed: {:?}", e);
            }
        }
    }

    Ok(())
}

