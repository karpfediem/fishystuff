use crate::moderation::actions::notify::create_thread;
use crate::moderation::index::{DashRecentIndex, RecentIndex};
use crate::moderation::types::{PerChannelTargets, PurgeParams, PurgeStats};
use crate::serenity;
use poise::serenity_prelude::{ChannelId, Http};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use tracing_subscriber::fmt::format;

pub mod notify;
pub mod purge;
pub mod timeout;

pub struct ModeratorActions {
    index: Arc<DashRecentIndex>,
    poster: Box<dyn notify::MessagePoster>,
    purger: Box<dyn purge::Purger>,
}

impl ModeratorActions {
    pub fn new(
        index: Arc<DashRecentIndex>,
        poster: impl notify::MessagePoster + 'static,
        purger: impl purge::Purger + 'static,
    ) -> Self {
        Self {
            index,
            poster: Box::new(poster),
            purger: Box::new(purger),
        }
    }
}

impl ModeratorActions {
    /// 1) query index; 2) post summary in #mod-info; 3) forward evidence (to #mod-info);
    /// 4) purge (bulk where possible); 5) post footer with stats (to #mod-info).
    ///
    /// Note: this function **does not** mutate the index to forget deleted IDs.
    pub async fn forward_then_purge(
        &self,
        http: &Http,
        params: PurgeParams<'_>,
    ) -> serenity::Result<PurgeStats> {
        let user_id = params.offending_message.author.id;

        // 1) Gather targets (arrival-based window).
        let mut per_channel = self
            .index
            .collect_since_at(user_id, params.window_secs, params.reference_now_secs)
            .await;

        // Optional allowlist (e.g., single-channel spam).
        if let Some(allow) = &params.channel_allowlist {
            let allowset: std::collections::HashSet<_> = allow.iter().copied().collect();
            per_channel.retain(|ch, _| allowset.contains(ch));
        }

        // Optional cap (preserve newest-first by channel).
        if let Some(max_total) = params.max_total {
            let mut flat: Vec<(ChannelId, serenity::MessageId)> = per_channel
                .iter()
                .flat_map(|(ch, ids)| ids.iter().map(move |id| (*ch, *id)))
                .collect();
            flat.truncate(max_total);
            let mut capped: PerChannelTargets = HashMap::new();
            for (ch, id) in flat {
                capped.entry(ch).or_default().push(id);
            }
            per_channel = capped;
        }

        let total_targets: usize = per_channel.values().map(|v| v.len()).sum();

        tracing::debug!(
            target: "mod.actions",
            "forward_then_purge: user={} targets_total={} channels={}",
            user_id.get(),
            total_targets,
            per_channel.len()
        );

        for (ch, ids) in &per_channel {
            tracing::debug!(target: "mod.actions", "channel={} count={}", ch.get(), ids.len());
        }

        // 2) Post summary directly in #mod-info.
        let summary = notify::make_parent_summary(
            &params.action_label,
            params.offending_message,
            params.extra_note.map(Cow::into_owned),
        );
        let summary_msg = self.poster.post_to_mod_info(http, summary).await?;


        // 3) Forward evidence to #mod-info thread (oldest → newest for readability).
        //    NOTE: We forward right after the summary. Moderators can click through even if deletes
        //    complete before all forwards land.

        // Create new thread on summary message
        let thread = create_thread(
            http,
            format!(
                "[{}] Evidence for user `{}` at `{}`",
                params.action_label, params.offending_message.author, params.reference_now_secs
            ),
            summary_msg.channel_id,
            Some(summary_msg.id),
        )
            .await?;

        let mut forwarded = 0usize;
        tracing::debug!(target:"mod.actions","forwarding start total={}", total_targets);

        for (ch, ids) in &per_channel {
            for mid in ids.iter().rev() {
                let res = self.poster.forward_to(http, thread.id, *ch, *mid).await;
                // discord/serenity freaks out if we spam messages so try sleeping...
                tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
                match res {
                    Ok(()) => forwarded += 1,
                    Err(_) => {}
                }
            }
        }
        tracing::debug!(target:"mod.actions","forwarding done forwarded={}", forwarded);

        // 4) Purge: bulk where possible (2..100), fallback to single deletes.
        let mut stats = PurgeStats::default();
        tracing::debug!(target:"mod.actions","purge start");

        for (ch, ids) in per_channel.iter_mut() {
            if ids.is_empty() {
                continue;
            }
            stats.add_channel();

            let mut i = 0usize;
            while i < ids.len() {
                let remaining = ids.len() - i;

                if remaining >= 2 {
                    let take = remaining.min(100);
                    let slice = &ids[i..i + take];
                    match self.purger.bulk_delete(http, *ch, slice).await {
                        Ok(()) => {
                            stats.add_targeted(take);
                            stats.add_deleted(take);
                            i += take;
                        }
                        Err(e) => {
                            tracing::warn!(
                                target: "mod.actions",
                                "bulk_delete failed ch={} count={} err={:?}; falling back to single",
                                ch.get(), take, e
                            );
                            for mid in slice {
                                match self.purger.single_delete(http, *ch, *mid).await {
                                    Ok(()) => {
                                        stats.add_targeted(1);
                                        stats.add_deleted(1);
                                    }
                                    Err(se) => {
                                        stats.add_targeted(1);
                                        tracing::warn!(
                                            target: "mod.actions",
                                            "single_delete failed ch={} msg_id={} err={:?}",
                                            ch.get(), mid.get(), se
                                        );
                                    }
                                }
                            }
                            i += take;
                        }
                    }
                } else {
                    let mid = ids[i];
                    match self.purger.single_delete(http, *ch, mid).await {
                        Ok(()) => {
                            stats.add_targeted(1);
                            stats.add_deleted(1);
                        }
                        Err(e) => {
                            stats.add_targeted(1);
                            tracing::warn!(
                                target: "mod.actions",
                                "single_delete failed ch={} msg_id={} err={:?}",
                                ch.get(), mid.get(), e
                            );
                        }
                    }
                    i += 1;
                }
            }
        }

        tracing::debug!(
            target: "mod.actions",
            "purge_done: user={} targeted={} deleted={} channels={}",
            user_id.get(),
            stats.targeted,
            stats.deleted,
            stats.channels_touched
        );

        // 5) Footer with stats (post in #mod-info, under the summary).
        let footer = if forwarded != stats.targeted {
            format!(
                "Forwarded {} message(s); targeted {}; deleted {} across {} channel(s).",
                forwarded, stats.targeted, stats.deleted, stats.channels_touched
            )
        } else {
            format!(
                "Deleted {} / {} across {} channel(s).",
                stats.deleted, stats.targeted, stats.channels_touched
            )
        };
        let _ = self.poster.post_to_mod_info(http, footer).await;

        // return the place we posted to (parent) for consistency with previous API
        Ok(stats)
    }
}
