use std::borrow::Cow;
use std::collections::HashMap;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::{ChannelId, Message};

/// What the action is operating on and how.
pub struct PurgeParams<'a> {
    pub offending_message: &'a Message,
    pub window_secs: u64,
    pub reference_now_secs: i64,
    pub action_label: Cow<'a, str>,
    pub extra_note: Option<Cow<'a, str>>,
    pub channel_allowlist: Option<Vec<ChannelId>>,
    pub max_total: Option<usize>,
}

impl<'a> PurgeParams<'a> {
    pub fn new(
        offending_message: &'a Message,
        window_secs: u64,
        reference_now_secs: i64,
        action_label: impl Into<Cow<'a, str>>,
    ) -> Self {
        Self {
            offending_message,
            window_secs,
            reference_now_secs,
            action_label: action_label.into(),
            extra_note: None,
            channel_allowlist: None,
            max_total: None,
        }
    }
    pub fn extra_note(mut self, note: impl Into<Cow<'a, str>>) -> Self {
        self.extra_note = Some(note.into());
        self
    }
    pub fn channel_allowlist(mut self, channels: &[ChannelId]) -> Self {
        self.channel_allowlist = Some(channels.to_vec());
        self
    }
    pub fn max_total(mut self, cap: usize) -> Self {
        self.max_total = Some(cap);
        self
    }
}

/// Per-channel list of message IDs to act on (newest-first).
pub type PerChannelTargets = HashMap<ChannelId, Vec<serenity::MessageId>>;

/// Summary of what we deleted.
#[derive(Clone, Copy, Debug, Default)]
pub struct PurgeStats {
    pub targeted: usize,
    pub deleted: usize,
    pub channels_touched: usize,
}
impl PurgeStats {
    pub fn add_channel(&mut self) { self.channels_touched += 1; }
    pub fn add_targeted(&mut self, n: usize) { self.targeted += n; }
    pub fn add_deleted(&mut self, n: usize) { self.deleted += n; }
}
