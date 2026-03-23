use axum::extract::{rejection::QueryRejection, Extension, Query, State};
use axum::Json;
use serde::Deserialize;

use fishystuff_api::ids::MapVersionId;
use fishystuff_api::models::layers::{LayerDescriptor, LayersResponse};

use crate::error::{with_timeout, AppResult};
use crate::routes::meta::map_request_id;
use crate::routes::public_assets::normalize_public_asset_url;
use crate::state::{RequestId, SharedState};

#[derive(Debug, Deserialize)]
pub struct LayersQuery {
    pub map_version: Option<String>,
}

pub async fn get_layers(
    State(state): State<SharedState>,
    query: Result<Query<LayersQuery>, QueryRejection>,
    Extension(request_id): Extension<RequestId>,
) -> AppResult<Json<LayersResponse>> {
    let Query(query) = query.map_err(|err| {
        crate::error::AppError::invalid_argument(err.to_string())
            .with_request_id(request_id.0.clone())
    })?;

    let map_version = query
        .map_version
        .and_then(|raw| {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .or_else(|| {
            state
                .config
                .defaults
                .map_version_id
                .as_ref()
                .map(|id| id.0.clone())
        });
    let mut response = with_timeout(
        state.config.request_timeout_secs,
        state.store.get_layers(map_version.clone()),
    )
    .await
    .map_err(|err| map_request_id(err, &request_id))?;

    if response.map_version_id.is_none() {
        response.map_version_id = map_version.map(MapVersionId);
    }
    normalize_layer_asset_urls(&mut response);

    Ok(Json(response))
}

fn normalize_layer_asset_urls(response: &mut LayersResponse) {
    for layer in &mut response.layers {
        normalize_layer_descriptor(layer);
    }
}

fn normalize_layer_descriptor(layer: &mut LayerDescriptor) {
    layer.tileset.manifest_url = normalize_public_asset_url(&layer.tileset.manifest_url);
    layer.tileset.tile_url_template = normalize_public_asset_url(&layer.tileset.tile_url_template);
    if let Some(field_source) = layer.field_source.as_mut() {
        field_source.url = normalize_public_asset_url(&field_source.url);
    }
    if let Some(field_metadata_source) = layer.field_metadata_source.as_mut() {
        field_metadata_source.url = normalize_public_asset_url(&field_metadata_source.url);
    }
    if let Some(vector_source) = layer.vector_source.as_mut() {
        vector_source.url = normalize_public_asset_url(&vector_source.url);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::extract::{Extension, Query, State};
    use axum::Json;

    use fishystuff_api::ids::MapVersionId;
    use fishystuff_api::models::effort::{EffortGridRequest, EffortGridResponse};
    use fishystuff_api::models::events::{EventsSnapshotMetaResponse, EventsSnapshotResponse};
    use fishystuff_api::models::fish::FishListResponse;
    use fishystuff_api::models::layers::{
        GeometrySpace, LayerDescriptor, LayerKind, LayerTransformDto, LayerUiInfo, LayersResponse,
        LodPolicyDto, StyleMode, TilesetRef, VectorSourceRef,
    };
    use fishystuff_api::models::meta::{MetaDefaults, MetaResponse};
    use fishystuff_api::models::region_groups::RegionGroupsResponse;
    use fishystuff_api::models::zone_profile_v2::{ZoneProfileV2Request, ZoneProfileV2Response};
    use fishystuff_api::models::zone_stats::{ZoneStatsRequest, ZoneStatsResponse};
    use fishystuff_api::models::zones::ZoneEntry;

    use crate::config::{AppConfig, ZoneStatusConfig};
    use crate::error::AppResult;
    use crate::state::{AppState, RequestId};
    use crate::store::{FishLang, Store};

    use super::{get_layers, LayersQuery};

    struct MockStore;

    #[async_trait]
    impl Store for MockStore {
        async fn get_meta(&self) -> AppResult<MetaResponse> {
            panic!("unused in test")
        }

        async fn get_layers(&self, map_version_id: Option<String>) -> AppResult<LayersResponse> {
            Ok(LayersResponse {
                revision: "test-rev".to_string(),
                map_version_id: map_version_id.map(MapVersionId),
                layers: vec![LayerDescriptor {
                    layer_id: "region_groups".to_string(),
                    name: "Region Groups".to_string(),
                    enabled: true,
                    kind: LayerKind::VectorGeoJson,
                    transform: LayerTransformDto::IdentityMapSpace,
                    tileset: TilesetRef::default(),
                    tile_px: 512,
                    max_level: 0,
                    y_flip: false,
                    field_source: None,
                    field_metadata_source: None,
                    vector_source: Some(VectorSourceRef {
                        url: "/region_groups/v1.geojson".to_string(),
                        revision: "rg-v1".to_string(),
                        geometry_space: GeometrySpace::MapPixels,
                        style_mode: StyleMode::FeaturePropertyPalette,
                        feature_id_property: Some("id".to_string()),
                        color_property: Some("c".to_string()),
                    }),
                    lod_policy: LodPolicyDto::default(),
                    ui: LayerUiInfo::default(),
                    request_weight: 1.0,
                    pick_mode: "none".to_string(),
                }],
            })
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
            _request: ZoneProfileV2Request,
            _status_cfg: ZoneStatusConfig,
        ) -> AppResult<ZoneProfileV2Response> {
            panic!("unused in test")
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

    #[tokio::test]
    async fn layers_route_returns_vector_metadata_shape() {
        let config = AppConfig {
            bind: "127.0.0.1:0".to_string(),
            database_url: "mysql://unused".to_string(),
            cors_allowed_origins: vec!["https://fishystuff.fish".to_string()],
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
        let state = AppState::for_tests(config, Arc::new(MockStore));

        let Json(response) = get_layers(
            State(state),
            Ok(Query(LayersQuery {
                map_version: Some("v1".to_string()),
            })),
            Extension(RequestId("req-test".to_string())),
        )
        .await
        .expect("layers response");

        assert_eq!(response.layers.len(), 1);
        let layer = &response.layers[0];
        assert_eq!(layer.kind, LayerKind::VectorGeoJson);
        let vector = layer.vector_source.as_ref().expect("vector_source");
        assert_eq!(vector.url, "/region_groups/v1.geojson");
        assert_eq!(vector.geometry_space, GeometrySpace::MapPixels);
        assert_eq!(vector.style_mode, StyleMode::FeaturePropertyPalette);
    }

    #[tokio::test]
    async fn layers_route_normalizes_public_asset_paths() {
        let config = AppConfig {
            bind: "127.0.0.1:0".to_string(),
            database_url: "mysql://unused".to_string(),
            cors_allowed_origins: vec!["https://fishystuff.fish".to_string()],
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
        let state = AppState::for_tests(config, Arc::new(MockStore));

        let Json(response) = get_layers(
            State(state),
            Ok(Query(LayersQuery {
                map_version: Some("v1".to_string()),
            })),
            Extension(RequestId("req-test".to_string())),
        )
        .await
        .expect("layers response");

        let vector = response.layers[0]
            .vector_source
            .as_ref()
            .expect("vector_source");
        assert_eq!(vector.url, "/region_groups/v1.geojson");
    }
}
