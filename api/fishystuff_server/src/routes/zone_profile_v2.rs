use axum::extract::{rejection::JsonRejection, Extension, State};
use axum::Json;

use fishystuff_api::models::zone_profile_v2::{ZoneProfileV2Request, ZoneProfileV2Response};

use crate::error::{with_timeout, AppError, AppResult};
use crate::routes::meta::map_request_id;
use crate::state::{RequestId, SharedState};

pub async fn zone_profile_v2(
    State(state): State<SharedState>,
    Extension(request_id): Extension<RequestId>,
    payload: Result<Json<ZoneProfileV2Request>, JsonRejection>,
) -> AppResult<Json<ZoneProfileV2Response>> {
    let Json(request) = payload.map_err(|err| {
        AppError::invalid_argument(err.to_string()).with_request_id(request_id.0.clone())
    })?;

    let response = with_timeout(
        state.config.request_timeout_secs,
        state
            .store
            .zone_profile_v2(request, state.config.status_cfg.clone()),
    )
    .await
    .map_err(|err| map_request_id(err, &request_id))?;

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::extract::{Extension, State};
    use axum::Json;
    use fishystuff_api::ids::{MapVersionId, RgbKey};
    use fishystuff_api::models::effort::{EffortGridRequest, EffortGridResponse};
    use fishystuff_api::models::events::{EventsSnapshotMetaResponse, EventsSnapshotResponse};
    use fishystuff_api::models::fish::FishListResponse;
    use fishystuff_api::models::layers::LayersResponse;
    use fishystuff_api::models::meta::{MetaDefaults, MetaResponse};
    use fishystuff_api::models::region_groups::RegionGroupsResponse;
    use fishystuff_api::models::zone_profile_v2::{
        ZoneAssignment, ZoneBorderAssessment, ZoneBorderClass, ZoneBorderMethod,
        ZoneCatchRateSummary, ZoneDiagnostics, ZoneMetricAvailability, ZonePresenceState,
        ZonePresenceSupport, ZoneProfileV2Request, ZoneProfileV2Response, ZonePublicState,
        ZoneRankingEvidence, ZoneRankingShareKind, ZoneRankingStatus, ZoneSourceFamily,
    };
    use fishystuff_api::models::zone_stats::{ZoneStatsRequest, ZoneStatsResponse};
    use fishystuff_api::models::zones::ZoneEntry;

    use crate::config::{AppConfig, ZoneStatusConfig};
    use crate::error::AppResult;
    use crate::state::{AppState, RequestId};
    use crate::store::{FishLang, Store};

    use super::zone_profile_v2;

    struct MockStore;

    #[async_trait]
    impl Store for MockStore {
        async fn get_meta(&self) -> AppResult<MetaResponse> {
            panic!("unused in test")
        }

        async fn get_layers(&self, _map_version_id: Option<String>) -> AppResult<LayersResponse> {
            panic!("unused in test")
        }

        async fn get_region_groups(
            &self,
            _map_version_id: Option<String>,
        ) -> AppResult<RegionGroupsResponse> {
            panic!("unused in test")
        }

        async fn list_fish(
            &self,
            _lang: FishLang,
            _ref_id: Option<String>,
        ) -> AppResult<FishListResponse> {
            panic!("unused in test")
        }

        async fn list_zones(&self, _ref_id: Option<String>) -> AppResult<Vec<ZoneEntry>> {
            panic!("unused in test")
        }

        async fn zone_stats(
            &self,
            _request: ZoneStatsRequest,
            _status_cfg: ZoneStatusConfig,
        ) -> AppResult<ZoneStatsResponse> {
            panic!("unused in test")
        }

        async fn zone_profile_v2(
            &self,
            request: ZoneProfileV2Request,
            _status_cfg: ZoneStatusConfig,
        ) -> AppResult<ZoneProfileV2Response> {
            Ok(ZoneProfileV2Response {
                assignment: ZoneAssignment {
                    zone_rgb_u32: request.rgb.to_u32().expect("rgb"),
                    zone_rgb: request.rgb,
                    zone_name: Some("Test Zone".to_string()),
                    point: None,
                    border: ZoneBorderAssessment {
                        class: ZoneBorderClass::Unavailable,
                        nearest_border_distance_px: None,
                        method: ZoneBorderMethod::Unavailable,
                        warnings: vec!["border analysis not implemented".to_string()],
                    },
                    neighboring_zones: Vec::new(),
                },
                presence_support: ZonePresenceSupport {
                    state: ZonePresenceState::InsufficientEvidence,
                    evaluated_sources: vec![ZoneSourceFamily::Ranking],
                    fish: Vec::new(),
                    notes: vec!["missing ranking evidence is not evidence of absence".to_string()],
                },
                ranking_evidence: ZoneRankingEvidence {
                    availability: ZoneMetricAvailability::Available,
                    source_family: ZoneSourceFamily::Ranking,
                    share_kind: ZoneRankingShareKind::PosteriorMeanEvidenceShare,
                    total_weight: 0.0,
                    ess: 0.0,
                    raw_event_count: None,
                    last_seen_ts_utc: None,
                    age_days_last: None,
                    status: ZoneRankingStatus::Unknown,
                    drift: None,
                    notes: vec!["ranking evidence share is not a catch/drop rate".to_string()],
                    fish: Vec::new(),
                },
                catch_rates: ZoneCatchRateSummary {
                    source_family: ZoneSourceFamily::Logs,
                    availability: ZoneMetricAvailability::PendingSource,
                    fish: Vec::new(),
                    notes: vec!["player-log catch rates not yet available".to_string()],
                },
                diagnostics: ZoneDiagnostics {
                    public_state: ZonePublicState::InsufficientEvidence,
                    insufficient_evidence: true,
                    border_sensitive: None,
                    border_stress: None,
                    notes: vec!["ranking-only first slice".to_string()],
                },
            })
        }

        async fn effort_grid(&self, _request: EffortGridRequest) -> AppResult<EffortGridResponse> {
            panic!("unused in test")
        }

        async fn events_snapshot_meta(&self) -> AppResult<EventsSnapshotMetaResponse> {
            panic!("unused in test")
        }

        async fn events_snapshot(
            &self,
            _requested_revision: Option<String>,
        ) -> AppResult<EventsSnapshotResponse> {
            panic!("unused in test")
        }

        async fn healthcheck(&self) -> AppResult<()> {
            panic!("unused in test")
        }
    }

    fn test_state() -> Arc<AppState> {
        let config = AppConfig {
            bind: "127.0.0.1:0".to_string(),
            database_url: "mysql://unused".to_string(),
            cors_allowed_origins: vec!["https://fishystuff.fish".to_string()],
            images_dir: PathBuf::from("data/cdn/public/images"),
            terrain_manifest_url: None,
            terrain_drape_manifest_url: None,
            terrain_height_tiles_url: None,
            defaults: MetaDefaults {
                tile_px: 32,
                sigma_tiles: 3.0,
                half_life_days: None,
                alpha0: 1.0,
                top_k: 30,
                map_version_id: Some(MapVersionId("v1".to_string())),
            },
            status_cfg: ZoneStatusConfig::default(),
            cache_zone_stats_max: 4,
            cache_effort_max: 4,
            cache_log: false,
            request_timeout_secs: 5,
        };
        AppState::for_tests(config, Arc::new(MockStore))
    }

    #[tokio::test]
    async fn zone_profile_v2_route_returns_explicit_placeholder_sections() {
        let Json(response) = zone_profile_v2(
            State(test_state()),
            Extension(RequestId("req-test".to_string())),
            Ok(Json(ZoneProfileV2Request {
                layer_revision_id: None,
                layer_id: None,
                patch_id: None,
                at_ts_utc: None,
                map_version_id: None,
                rgb: RgbKey("1,2,3".to_string()),
                map_px_x: None,
                map_px_y: None,
                from_ts_utc: 100,
                to_ts_utc: 200,
                tile_px: 32,
                sigma_tiles: 3.0,
                fish_norm: false,
                alpha0: 1.0,
                top_k: 30,
                half_life_days: None,
                drift_boundary_ts_utc: None,
                ref_id: None,
                lang: None,
            })),
        )
        .await
        .expect("zone profile response");

        assert_eq!(
            response.assignment.border.class,
            ZoneBorderClass::Unavailable
        );
        assert_eq!(
            response.presence_support.state,
            ZonePresenceState::InsufficientEvidence
        );
        assert_eq!(
            response.ranking_evidence.availability,
            ZoneMetricAvailability::Available
        );
        assert_eq!(
            response.catch_rates.availability,
            ZoneMetricAvailability::PendingSource
        );
    }
}
