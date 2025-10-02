//! Purge helper that deletes by message IDs collected from RecentIndex.
//! Uses per-channel bulk delete (2..=100) and falls back to singles on error.

use crate::moderation::purge::UserRecentIndex;
use poise::serenity_prelude as serenity;

type Error = Box<dyn std::error::Error + Send + Sync>;

impl UserRecentIndex {
    /// Delete messages for `user_id` newer than `window_secs` using only IDs from `index`.
    /// If `exclude` is Some(id), that ID is skipped (e.g., the triggering message already deleted).
    pub async fn purge_recent(
        &self,
        http: &serenity::Http,
        user_id: serenity::UserId,
        window_secs: u64,
        exclude: Option<serenity::model::id::MessageId>,
    ) -> Result<usize, Error> {
        let mut total = 0usize;

        let mut per_channel = self.collect_since(user_id, window_secs).await;

        if let Some(ex) = exclude {
            if let Some(v) = per_channel.get_mut(&serenity::ChannelId::new(ex.get() >> 22)) {
                // The above guess is not reliable; safer: just remove from all channels.
            }
            for v in per_channel.values_mut() {
                v.retain(|&id| id != ex);
            }
        }

        for (chan_id, mut ids) in per_channel {
            if ids.is_empty() {
                continue;
            }

            // Delete in chunks up to 100
            while !ids.is_empty() {
                let chunk: Vec<_> = ids.drain(0..ids.len().min(100)).collect();

                if chunk.len() >= 2 {
                    match chan_id.delete_messages(http, chunk.clone()).await {
                        Ok(_) => total += chunk.len(),
                        Err(e) => {
                            tracing::warn!(
                                "Bulk delete failed in {}: {e:?}; falling back",
                                chan_id
                            );
                            for id in chunk {
                                if chan_id.delete_message(http, id).await.is_ok() {
                                    total += 1;
                                }
                            }
                        }
                    }
                } else {
                    let id = chunk[0];
                    if chan_id.delete_message(http, id).await.is_ok() {
                        total += 1;
                    }
                }
            }
        }

        Ok(total)
    }
}
