//! Rolling in-memory index of recent messages per user.
//! Stores (channel_id, message_id, timestamp) and prunes by a short retention window.
//!
//! Call `record(&Message)` on every incoming message (for all channels).
//! Use `collect_since(user_id, seconds)` to get a per-channel map of message IDs.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use poise::serenity_prelude as serenity;
use tokio::sync::RwLock;

#[derive(Clone, Copy, Debug)]
struct Entry {
    channel_id: serenity::ChannelId,
    message_id: serenity::model::id::MessageId,
    ts_secs: i64,
}

/// Stores recent messages per user with periodic pruning.
pub struct UserRecentIndex {
    retention_secs: i64,
    inner: RwLock<HashMap<serenity::UserId, Vec<Entry>>>,
}

impl UserRecentIndex {
    /// `retention_secs` should be small (e.g., 120–300) to bound memory.
    pub fn new(retention_secs: u64) -> Self {
        Self {
            retention_secs: retention_secs as i64,
            inner: RwLock::new(HashMap::new()),
        }
    }

    /// Record message id from any channel/thread in a bucket for the user.
    pub async fn record(&self, msg: &serenity::Message) {
        if msg.author.bot {
            return;
        }
        let now = now_unix();
        let ts_secs = msg.timestamp.unix_timestamp();

        let mut map = self.inner.write().await;
        let bucket = map.entry(msg.author.id).or_default();

        bucket.push(Entry {
            channel_id: msg.channel_id,
            message_id: msg.id,
            ts_secs,
        });

        let cutoff = now - self.retention_secs;
        bucket.retain(|e| e.ts_secs >= cutoff);

        // Optional safety bound if someone floods massively
        if bucket.len() > 20_000 {
            let keep_from = bucket.len().saturating_sub(20_000);
            bucket.drain(0..keep_from);
        }
    }

    /// Get messages by `user_id` newer than now - `seconds`, grouped by channel.
    pub async fn collect_since(
        &self,
        user_id: serenity::UserId,
        seconds: u64,
    ) -> HashMap<serenity::ChannelId, Vec<serenity::model::id::MessageId>> {
        let now = now_unix();
        let cutoff = now - seconds as i64;

        let map = self.inner.read().await;
        let bucket = match map.get(&user_id) {
            Some(b) => b,
            None => return HashMap::new(),
        };

        let mut per_channel: HashMap<serenity::ChannelId, Vec<serenity::model::id::MessageId>> =
            HashMap::new();

        // Iterate newest-first for a fast cutoff
        for e in bucket.iter().rev() {
            if e.ts_secs < cutoff {
                break;
            }
            per_channel.entry(e.channel_id).or_default().push(e.message_id);
        }

        per_channel
    }
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
