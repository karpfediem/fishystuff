//! Notification utilities for moderation actions (no threads).
//! - post a summary ("headline") in #mod-info
//! - forward evidence messages as references into #mod-info
//! - optionally post plain text into #mod-info
//!
//! The traits here are small and mockable for tests.

use poise::serenity_prelude as serenity;
use poise::serenity_prelude::{
    Builder, ChannelId, CreateMessage, CreateThread, GuildChannel, Http, Message, MessageId,
    MessageReference, MessageReferenceKind,
};

/// Abstraction over posting — mockable.
#[async_trait::async_trait]
pub trait MessagePoster: Send + Sync {
    /// Post a message and return the created message.
    async fn post_to(
        &self,
        http: &Http,
        parent: ChannelId,
        content: String,
    ) -> serenity::Result<Message>;

    /// Post a forward reference (original author preserved)
    async fn forward_to(
        &self,
        http: &Http,
        parent: ChannelId,
        src_channel: ChannelId,
        src_message: MessageId,
    ) -> serenity::Result<()>;

    async fn post_to_mod_info(&self, http: &Http, content: String) -> serenity::Result<Message>;
    async fn forward_to_mod_info(
        &self,
        http: &Http,
        src_channel: ChannelId,
        src_message: MessageId,
    ) -> serenity::Result<()>;
}

/// Default poster backed by Serenity HTTP.
pub struct SerenityPoster {
    cfg: ModInfoConfig,
}

/// Where summaries are posted (config).
#[derive(Clone, Copy)]
pub struct ModInfoConfig {
    pub mod_info_channel_id: ChannelId,
}

impl ModInfoConfig {
    pub fn new(id: u64) -> Self {
        Self {
            mod_info_channel_id: ChannelId::new(id),
        }
    }
}

impl Default for ModInfoConfig {
    fn default() -> Self {
        Self {
            mod_info_channel_id: ChannelId::new(211092999835222017),
        }
    }
}
impl SerenityPoster {
    pub fn new(cfg: ModInfoConfig) -> Self {
        Self { cfg }
    }
}

#[async_trait::async_trait]
impl MessagePoster for SerenityPoster {
    async fn post_to(
        &self,
        http: &Http,
        channel: ChannelId,
        content: String,
    ) -> serenity::Result<Message> {
        let msg = channel
            .send_message(http, CreateMessage::new().content(content))
            .await?;
        Ok(msg)
    }

    async fn forward_to(
        &self,
        http: &Http,
        channel: ChannelId,
        src_channel: ChannelId,
        src_message: MessageId,
    ) -> serenity::Result<()> {
        let forward = MessageReference::new(MessageReferenceKind::Forward, src_channel)
            .message_id(src_message);
        let _ = channel
            .send_message(http, CreateMessage::new().reference_message(forward))
            .await?;
        Ok(())
    }

    async fn post_to_mod_info(&self, http: &Http, content: String) -> serenity::Result<Message> {
        self.post_to(http, self.cfg.mod_info_channel_id, content)
            .await
    }

    async fn forward_to_mod_info(
        &self,
        http: &Http,
        src_channel: ChannelId,
        src_message: MessageId,
    ) -> serenity::Result<()> {
        self.forward_to(http, self.cfg.mod_info_channel_id, src_channel, src_message)
            .await
    }
}

pub async fn create_thread(
    http: &Http,
    thread_name: impl Into<String>,
    parent_channel_id: ChannelId,
    parent_message_id: Option<MessageId>,
) -> poise::serenity_prelude::Result<GuildChannel> {
    CreateThread::new(thread_name)
        .rate_limit_per_user(0)
        .invitable(false)
        .execute(http, (parent_channel_id, parent_message_id))
        .await
}

// ----------------------- formatting helpers (shared) -----------------------

/// One-liner headline suitable for the parent #mod-info channel.
pub fn make_parent_summary(
    action_label: &str,
    offending_message: &Message,
    extra_note: impl Into<Option<String>>,
) -> String {
    // Example: "[BURST] kick — <@uid> in <#chan>; Distinct channels: 3 | Total msgs: 7 | Purge window: 60s"
    let mut s = format!(
        "{} — <@{}> in <#{}>",
        action_label,
        offending_message.author.id.get(),
        offending_message.channel_id.get(),
    );

    if let Some(note) = extra_note.into() {
        if !note.is_empty() {
            s.push_str("; ");
            s.push_str(&note);
        }
    }
    s
}
