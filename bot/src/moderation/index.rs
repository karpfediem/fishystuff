//! Rolling in-memory index of recent messages per user (concurrent, O(k) pruning).
//! Side-effect free: no forwarding, no deletion.

use std::collections::{HashMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};

use dashmap::DashMap;
use poise::serenity_prelude as serenity;
use crate::moderation::now_unix;

const PER_USER_HARD_CAP: usize = 20_000;

#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub channel_id: serenity::ChannelId,
    pub message_id: serenity::model::id::MessageId,
    pub arrival_secs: i64,
    pub msg_ts_secs: i64,
}

/// “Read-only” trait for consumers (actions) to depend on.
#[async_trait::async_trait]
pub trait RecentIndex: Send + Sync {
    async fn record(&self, msg: &serenity::Message);
    async fn collect_since_at(
        &self,
        user_id: serenity::UserId,
        seconds: u64,
        reference_now_secs: i64,
    ) -> HashMap<serenity::ChannelId, Vec<serenity::MessageId>>;
    async fn counts_since_at(
        &self,
        user_id: serenity::UserId,
        seconds: u64,
        reference_now_secs: i64,
    ) -> HashMap<serenity::ChannelId, u64>;
}

/// Concrete implementation (DashMap + VecDeque).
#[derive(Clone)]
pub struct DashRecentIndex {
    retention_secs: i64,
    inner: DashMap<serenity::UserId, VecDeque<Entry>>,
}

impl DashRecentIndex {
    pub fn new(retention_secs: u64) -> Self {
        Self {
            retention_secs: retention_secs as i64,
            inner: DashMap::new(),
        }
    }
}

#[async_trait::async_trait]
impl RecentIndex for DashRecentIndex {
    async fn record(&self, msg: &serenity::Message) {
        if msg.author.bot { return; }
        let arrival_secs = now_unix();
        let msg_ts_secs = msg.timestamp.unix_timestamp();
        let cutoff = arrival_secs - self.retention_secs;

        use dashmap::mapref::entry::Entry as E;
        match self.inner.entry(msg.author.id) {
            E::Occupied(mut occ) => {
                let dq = occ.get_mut();
                while let Some(front) = dq.front() {
                    if front.arrival_secs >= cutoff { break; }
                    dq.pop_front();
                }
                dq.push_back(Entry {
                    channel_id: msg.channel_id,
                    message_id: msg.id,
                    arrival_secs,
                    msg_ts_secs,
                });
                if dq.len() > PER_USER_HARD_CAP {
                    let drop_n = dq.len() - PER_USER_HARD_CAP;
                    for _ in 0..drop_n { dq.pop_front(); }
                }
            }
            E::Vacant(vac) => {
                let mut dq = VecDeque::with_capacity(64);
                dq.push_back(Entry {
                    channel_id: msg.channel_id,
                    message_id: msg.id,
                    arrival_secs,
                    msg_ts_secs,
                });
                vac.insert(dq);
            }
        }
    }

    async fn collect_since_at(
        &self,
        user_id: serenity::UserId,
        seconds: u64,
        reference_now_secs: i64,
    ) -> HashMap<serenity::ChannelId, Vec<serenity::MessageId>> {
        let cutoff = reference_now_secs - seconds as i64;
        let Some(dq) = self.inner.get(&user_id) else { return HashMap::new(); };

        let mut per_channel: HashMap<serenity::ChannelId, Vec<serenity::MessageId>> = HashMap::new();
        for e in dq.iter().rev() {
            if e.arrival_secs < cutoff { break; }
            per_channel.entry(e.channel_id).or_default().push(e.message_id);
        }
        per_channel
    }

    async fn counts_since_at(
        &self,
        user_id: serenity::UserId,
        seconds: u64,
        reference_now_secs: i64,
    ) -> HashMap<serenity::ChannelId, u64> {
        let cutoff = reference_now_secs - seconds as i64;
        let Some(dq) = self.inner.get(&user_id) else { return HashMap::new(); };

        let mut per_channel: HashMap<serenity::ChannelId, u64> = HashMap::new();
        for e in dq.iter().rev() {
            if e.arrival_secs < cutoff { break; }
            *per_channel.entry(e.channel_id).or_default() += 1;
        }
        per_channel
    }
}

#[async_trait::async_trait]
impl RecentIndex for std::sync::Arc<DashRecentIndex> {
    async fn record(&self, msg: &serenity::Message) {
        (**self).record(msg).await
    }
    async fn collect_since_at(
        &self,
        user_id: serenity::UserId,
        seconds: u64,
        reference_now_secs: i64,
    ) -> std::collections::HashMap<serenity::ChannelId, Vec<serenity::MessageId>> {
        (**self).collect_since_at(user_id, seconds, reference_now_secs).await
    }
    async fn counts_since_at(
        &self,
        user_id: serenity::UserId,
        seconds: u64,
        reference_now_secs: i64,
    ) -> std::collections::HashMap<serenity::ChannelId, u64> {
        (**self).counts_since_at(user_id, seconds, reference_now_secs).await
    }
}