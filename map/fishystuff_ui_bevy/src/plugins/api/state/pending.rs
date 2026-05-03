use async_channel::Receiver;
use fishystuff_api::models::fish::CommunityFishZoneSupportResponse;
use fishystuff_api::models::meta::MetaResponse;
use fishystuff_api::models::zone_stats::ZoneStatsResponse;
use fishystuff_api::models::zones::ZonesResponse;
use std::time::Duration;

use crate::prelude::*;

use super::catalog::FishCatalogPayload;

const API_REQUEST_RETRY_BASE_DELAY: Duration = Duration::from_secs(1);
const API_REQUEST_RETRY_MAX_DELAY: Duration = Duration::from_secs(30);

#[derive(Resource, Default)]
pub struct PendingRequests {
    pub meta: Option<Receiver<Result<MetaResponse, String>>>,
    pub zones: Option<Receiver<Result<ZonesResponse, String>>>,
    pub zone_stats: Option<(u32, Receiver<Result<ZoneStatsResponse, String>>)>,
    pub(crate) fish_catalog: Option<Receiver<Result<FishCatalogPayload, String>>>,
    pub(crate) community_fish_zone_support:
        Option<Receiver<Result<CommunityFishZoneSupportResponse, String>>>,
    meta_retry: ApiRequestRetryState,
    zones_retry: ApiRequestRetryState,
    fish_catalog_retry: ApiRequestRetryState,
    community_fish_zone_support_retry: ApiRequestRetryState,
}

impl PendingRequests {
    pub(crate) fn can_request_meta(&self, now_secs: f64) -> bool {
        self.meta_retry.can_attempt_at(now_secs)
    }

    pub(crate) fn record_meta_success(&mut self) {
        self.meta_retry.record_success();
    }

    pub(crate) fn record_meta_failure(&mut self, now_secs: f64) -> Duration {
        self.meta_retry.record_failure_at(now_secs)
    }

    pub(crate) fn can_request_zones(&self, now_secs: f64) -> bool {
        self.zones_retry.can_attempt_at(now_secs)
    }

    pub(crate) fn record_zones_success(&mut self) {
        self.zones_retry.record_success();
    }

    pub(crate) fn record_zones_failure(&mut self, now_secs: f64) -> Duration {
        self.zones_retry.record_failure_at(now_secs)
    }

    pub(crate) fn can_request_fish_catalog(&self, now_secs: f64) -> bool {
        self.fish_catalog_retry.can_attempt_at(now_secs)
    }

    pub(crate) fn record_fish_catalog_success(&mut self) {
        self.fish_catalog_retry.record_success();
    }

    pub(crate) fn record_fish_catalog_failure(&mut self, now_secs: f64) -> Duration {
        self.fish_catalog_retry.record_failure_at(now_secs)
    }

    pub(crate) fn can_request_community_fish_zone_support(&self, now_secs: f64) -> bool {
        self.community_fish_zone_support_retry
            .can_attempt_at(now_secs)
    }

    pub(crate) fn record_community_fish_zone_support_success(&mut self) {
        self.community_fish_zone_support_retry.record_success();
    }

    pub(crate) fn record_community_fish_zone_support_failure(&mut self, now_secs: f64) -> Duration {
        self.community_fish_zone_support_retry
            .record_failure_at(now_secs)
    }
}

#[derive(Debug, Default)]
struct ApiRequestRetryState {
    failure_count: u32,
    next_attempt_at_secs: Option<f64>,
}

impl ApiRequestRetryState {
    fn can_attempt_at(&self, now_secs: f64) -> bool {
        self.next_attempt_at_secs
            .map(|next_attempt_at_secs| now_secs >= next_attempt_at_secs)
            .unwrap_or(true)
    }

    fn record_success(&mut self) {
        self.failure_count = 0;
        self.next_attempt_at_secs = None;
    }

    fn record_failure_at(&mut self, now_secs: f64) -> Duration {
        self.failure_count = self.failure_count.saturating_add(1);
        let delay = retry_delay_for_failure_count(self.failure_count);
        self.next_attempt_at_secs = Some(now_secs + delay.as_secs_f64());
        delay
    }
}

fn retry_delay_for_failure_count(failure_count: u32) -> Duration {
    if failure_count == 0 {
        return Duration::ZERO;
    }
    let multiplier = 1u32 << failure_count.saturating_sub(1).min(8);
    API_REQUEST_RETRY_BASE_DELAY
        .saturating_mul(multiplier)
        .min(API_REQUEST_RETRY_MAX_DELAY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_request_retry_state_blocks_until_next_attempt() {
        let now = 10.0;
        let mut state = ApiRequestRetryState::default();

        assert!(state.can_attempt_at(now));
        assert_eq!(state.record_failure_at(now), Duration::from_secs(1));
        assert!(!state.can_attempt_at(now + 0.999));
        assert!(state.can_attempt_at(now + 1.0));
    }

    #[test]
    fn api_request_retry_state_backs_off_and_resets_after_success() {
        let now = 10.0;
        let mut state = ApiRequestRetryState::default();

        assert_eq!(state.record_failure_at(now), Duration::from_secs(1));
        assert_eq!(state.record_failure_at(now + 1.0), Duration::from_secs(2));
        assert_eq!(state.record_failure_at(now + 3.0), Duration::from_secs(4));

        state.record_success();

        assert!(state.can_attempt_at(now + 3.0));
        assert_eq!(state.record_failure_at(now + 3.0), Duration::from_secs(1));
    }

    #[test]
    fn retry_delay_caps_at_max_delay() {
        assert_eq!(retry_delay_for_failure_count(1), Duration::from_secs(1));
        assert_eq!(retry_delay_for_failure_count(2), Duration::from_secs(2));
        assert_eq!(retry_delay_for_failure_count(6), Duration::from_secs(30));
        assert_eq!(retry_delay_for_failure_count(99), Duration::from_secs(30));
    }
}
