mod catalog;
mod layers;
mod stats;
mod util;
mod zone_profile_v2;

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use async_trait::async_trait;
use fishystuff_api::error::ApiError;
use fishystuff_api::ids::{MapVersionId, Rgb};
use fishystuff_api::models::effort::{EffortGridRequest, EffortGridResponse};
use fishystuff_api::models::events::{
    EventPointCompact, EventSourceKind, EventsSnapshotMetaResponse, EventsSnapshotResponse,
};
use fishystuff_api::models::fish::{FishEntry, FishListResponse};
use fishystuff_api::models::layers::{
    LayerDescriptor, LayerUiInfo, LayersResponse, LodPolicyDto, TilesetRef,
};
use fishystuff_api::models::meta::{
    CanonicalMapInfo, MapVersionInfo, MetaDefaults, MetaResponse, PatchInfo,
};
use fishystuff_api::models::region_groups::{RegionGroupDescriptor, RegionGroupsResponse};
use fishystuff_api::models::zone_profile_v2::{ZoneProfileV2Request, ZoneProfileV2Response};
use fishystuff_api::models::zone_stats::{
    DriftInfo, ZoneConfidence, ZoneFishEvidence, ZoneStatsRequest, ZoneStatsResponse,
    ZoneStatsWindow, ZoneStatus,
};
use fishystuff_api::models::zones::ZoneEntry;
use fishystuff_api::version::API_VERSION;
use fishystuff_core::constants::{MAP_HEIGHT, MAP_WIDTH};
use fishystuff_core::prob::js_divergence;
use fishystuff_core::tile::tile_dimensions;
use mysql::prelude::Queryable;
use mysql::OptsBuilder;
use mysql::{Opts, Pool, PoolConstraints, PoolOpts, Row};

use crate::config::ZoneStatusConfig;
use crate::error::{AppError, AppResult};
use crate::store::queries;
use crate::store::{validate_dolt_ref, FishLang, Store};
use catalog::{
    encyclopedia_icon_id_from_db, fish_catch_methods_from_description, fish_grade_from_db,
    fish_is_dried, merge_fish_catalog_row, parse_positive_i64,
};
use layers::{
    normalize_pick_mode, parse_layer_kind, parse_layer_transform, parse_vector_source,
    resolve_layer_asset_url, substitute_map_version, VectorSourceFields,
};
use stats::{
    align_alpha, align_probs, beta_ci, compute_status, gaussian_blur_grid, pixel_to_tile_index,
    sample_dirichlet, seed_from_drift, seed_from_params, time_weight, XorShift64,
};
use util::{
    clamp_i64_to_i32, clamp_i64_to_u32, clamp_i64_to_u8, clamp_i64_to_usize, db_unavailable,
    epoch_to_mysql_datetime, event_source_kind_from_db, events_schema_or_db_unavailable,
    is_layers_schema_error, is_missing_table, normalize_optional_string, row_f64, row_i64,
    row_opt_f64, row_string, row_u32_opt, synthetic_events_snapshot_revision,
    synthetic_fish_revision, synthetic_layers_revision, synthetic_region_groups_revision,
};

const EPS: f64 = 1e-9;
const EPS_FISH: f64 = 1e-9;
const SOURCE_KIND_RANKING: i32 = 1;
const DOLT_POOL_MIN_CONNECTIONS: usize = 0;
const DOLT_POOL_MAX_CONNECTIONS: usize = 16;
const DOLT_TCP_CONNECT_TIMEOUT_SECS: u64 = 3;
const DOLT_SOCKET_READ_TIMEOUT_SECS: u64 = 10;
const DOLT_SOCKET_WRITE_TIMEOUT_SECS: u64 = 10;
const DOLT_TCP_KEEPALIVE_TIME_MS: u32 = 5_000;
#[cfg(any(target_os = "linux", target_os = "macos"))]
const DOLT_TCP_KEEPALIVE_PROBE_INTERVAL_SECS: u32 = 5;
#[cfg(any(target_os = "linux", target_os = "macos"))]
const DOLT_TCP_KEEPALIVE_PROBE_COUNT: u32 = 3;
#[cfg(target_os = "linux")]
const DOLT_TCP_USER_TIMEOUT_MS: u32 = 10_000;

#[derive(Clone)]
pub struct DoltMySqlStore {
    pool: Pool,
    defaults: MetaDefaults,
}

#[derive(Debug, Clone)]
struct QueryParams {
    map_version: String,
    from_ts_utc: i64,
    to_ts_utc: i64,
    half_life_days: Option<f64>,
    tile_px: u32,
    sigma_tiles: f64,
    fish_norm: bool,
    alpha0: f64,
    top_k: usize,
    drift_boundary_ts: Option<i64>,
}

#[derive(Debug, Clone)]
struct EventZoneRow {
    ts_utc: i64,
    fish_id: i32,
    sample_px_x: i32,
    sample_px_y: i32,
    zone_rgb_u32: u32,
}

#[derive(Debug, Clone)]
struct DerivedEvent {
    ts_utc: i64,
    fish_id: i32,
    zone_rgb_u32: u32,
    w_time: f64,
}

#[derive(Debug, Clone)]
struct WindowSummary {
    alpha_total: f64,
    alpha_by_fish: HashMap<i32, f64>,
    p_mean_by_fish: HashMap<i32, f64>,
    c_zone: HashMap<i32, f64>,
    ess: f64,
    total_weight: f64,
    last_seen: Option<i64>,
}

#[derive(Debug, Clone)]
struct FishIdentityEntry {
    encyclopedia_key: i32,
    item_id: i32,
    encyclopedia_id: Option<i32>,
    name: Option<String>,
}

#[derive(Debug, Clone)]
struct FishIdentityIndex {
    by_encyclopedia: HashMap<i32, FishIdentityEntry>,
}

type FishCatalogDbRow = (
    i64,
    Option<i64>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

type FishIdentityDbRow = (i64, i64, Option<String>, Option<String>, Option<String>);

type EventsSnapshotMetaDbRow = (u64, Option<i64>, Option<i64>, Option<String>);

type EventPointCompactDbRow = (
    i64,
    i64,
    i64,
    i64,
    i64,
    i64,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    i64,
    Option<String>,
);

#[derive(Debug, Clone)]
struct FishCatalogRow {
    item_id: i32,
    encyclopedia_key: Option<i32>,
    encyclopedia_id: Option<i32>,
    name: String,
    grade: Option<String>,
    grade_rank: Option<u8>,
    is_prize: Option<bool>,
    is_dried: bool,
    catch_methods: Vec<String>,
    vendor_price: Option<i64>,
}

impl QueryParams {
    fn validate(&self) -> AppResult<()> {
        if self.from_ts_utc >= self.to_ts_utc {
            return Err(AppError::invalid_argument(
                "from_ts_utc must be < to_ts_utc",
            ));
        }
        if self.tile_px == 0 {
            return Err(AppError::invalid_argument("tile_px must be > 0"));
        }
        if self.sigma_tiles <= 0.0 {
            return Err(AppError::invalid_argument("sigma_tiles must be > 0"));
        }
        if let Some(half) = self.half_life_days {
            if half <= 0.0 {
                return Err(AppError::invalid_argument("half_life_days must be > 0"));
            }
        }
        if self.alpha0 <= 0.0 {
            return Err(AppError::invalid_argument("alpha0 must be > 0"));
        }
        if self.top_k == 0 {
            return Err(AppError::invalid_argument("top_k must be > 0"));
        }
        if let Some(boundary) = self.drift_boundary_ts {
            if boundary <= self.from_ts_utc || boundary >= self.to_ts_utc {
                return Err(AppError::invalid_argument(
                    "drift_boundary_ts must be within (from_ts_utc, to_ts_utc)",
                ));
            }
        }
        Ok(())
    }
}

impl DoltMySqlStore {
    pub fn new(database_url: String, defaults: MetaDefaults) -> AppResult<Self> {
        let opts = Opts::from_url(&database_url).map_err(db_unavailable)?;
        let constraints =
            PoolConstraints::new(DOLT_POOL_MIN_CONNECTIONS, DOLT_POOL_MAX_CONNECTIONS)
                .ok_or_else(|| AppError::internal("invalid Dolt pool constraints"))?;
        let pool_opts = PoolOpts::default().with_constraints(constraints);
        let mut builder = OptsBuilder::from_opts(opts)
            .pool_opts(pool_opts)
            .tcp_connect_timeout(Some(Duration::from_secs(DOLT_TCP_CONNECT_TIMEOUT_SECS)))
            .read_timeout(Some(Duration::from_secs(DOLT_SOCKET_READ_TIMEOUT_SECS)))
            .write_timeout(Some(Duration::from_secs(DOLT_SOCKET_WRITE_TIMEOUT_SECS)))
            .tcp_keepalive_time_ms(Some(DOLT_TCP_KEEPALIVE_TIME_MS));
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            builder = builder
                .tcp_keepalive_probe_interval_secs(Some(DOLT_TCP_KEEPALIVE_PROBE_INTERVAL_SECS))
                .tcp_keepalive_probe_count(Some(DOLT_TCP_KEEPALIVE_PROBE_COUNT));
        }
        #[cfg(target_os = "linux")]
        {
            builder = builder.tcp_user_timeout_ms(Some(DOLT_TCP_USER_TIMEOUT_MS));
        }
        let pool = Pool::new(builder).map_err(db_unavailable)?;
        Ok(Self { pool, defaults })
    }

    fn query_patches(&self) -> AppResult<Vec<PatchInfo>> {
        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<(String, i64, Option<String>)> =
            conn.query(queries::PATCHES_SQL).map_err(db_unavailable)?;

        let mut patches = Vec::with_capacity(rows.len());
        for (patch_id, start_ts_utc, patch_name) in rows {
            patches.push(PatchInfo {
                patch_id: patch_id.into(),
                start_ts_utc,
                patch_name: normalize_optional_string(patch_name),
            });
        }
        Ok(patches)
    }

    fn query_map_versions(&self) -> AppResult<Vec<MapVersionInfo>> {
        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<(String, Option<String>, Option<i64>)> = match conn
            .query(queries::MAP_VERSIONS_SQL)
        {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "map_versions") => {
                return Err(AppError::not_found(
                    "map_versions table is missing; apply api/sql/migrations/20260301_vector_geojson_layers.sql",
                ));
            }
            Err(err) => return Err(db_unavailable(err)),
        };

        let mut versions = Vec::with_capacity(rows.len());
        for (map_version_id, name, is_default) in rows {
            versions.push(MapVersionInfo {
                map_version_id: MapVersionId(map_version_id),
                name: normalize_optional_string(name),
                is_default: is_default.unwrap_or(0) != 0,
            });
        }

        if versions.is_empty() {
            return Err(AppError::not_found(
                "map_versions table is empty; seed map_versions before starting the server",
            ));
        }

        if !versions.iter().any(|entry| entry.is_default) {
            let default_id = self.defaults.map_version_id.as_ref();
            let mut found = false;
            for entry in &mut versions {
                entry.is_default = default_id
                    .map(|id| id == &entry.map_version_id)
                    .unwrap_or(false);
                found |= entry.is_default;
            }
            if !found {
                if let Some(first) = versions.first_mut() {
                    first.is_default = true;
                }
            }
        }

        Ok(versions)
    }

    fn query_layer_descriptors(
        &self,
        map_version_id: Option<&str>,
    ) -> AppResult<Vec<LayerDescriptor>> {
        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<Row> = if let Some(map_version_id) = map_version_id {
            match conn.exec(queries::LAYERS_WITH_CONFIG_SQL, (map_version_id,)) {
                Ok(rows) => rows,
                Err(err) if is_missing_table(&err, "layer_configs") => {
                    return Err(AppError::not_found(
                        "layer_configs table is missing; apply api/sql/migrations/20260301_vector_geojson_layers.sql",
                    ));
                }
                Err(err) if is_missing_table(&err, "layers") => {
                    return Err(AppError::not_found(
                        "layers table is missing; apply api/sql/migrations/20260228_layers.sql",
                    ));
                }
                Err(err) if is_layers_schema_error(&err) => {
                    return Err(AppError::not_found(
                        "layers schema is outdated; apply api/sql/migrations/20260301_vector_geojson_layers.sql",
                    ));
                }
                Err(err) => return Err(db_unavailable(err)),
            }
        } else {
            match conn.query(queries::LAYERS_SQL) {
                Ok(rows) => rows,
                Err(err) if is_missing_table(&err, "layers") => {
                    return Err(AppError::not_found(
                        "layers table is missing; apply api/sql/migrations/20260228_layers.sql",
                    ));
                }
                Err(err) if is_layers_schema_error(&err) => {
                    return Err(AppError::not_found(
                        "layers schema is outdated; apply api/sql/migrations/20260301_vector_geojson_layers.sql",
                    ));
                }
                Err(err) => return Err(db_unavailable(err)),
            }
        };

        let mut descriptors = Vec::with_capacity(rows.len());
        for row in rows {
            let layer_id = row_string(&row, 0).unwrap_or_default();
            let name = row_string(&row, 1).unwrap_or_else(|| layer_id.clone());
            let enabled = row_i64(&row, 2, 0);
            if enabled == 0 {
                continue;
            }
            let transform_kind =
                row_string(&row, 3).unwrap_or_else(|| "identity_map_space".to_string());
            let affine_a = row_opt_f64(&row, 4);
            let affine_b = row_opt_f64(&row, 5);
            let affine_tx = row_opt_f64(&row, 6);
            let affine_c = row_opt_f64(&row, 7);
            let affine_d = row_opt_f64(&row, 8);
            let affine_ty = row_opt_f64(&row, 9);
            let manifest_url = row_string(&row, 10).unwrap_or_default();
            let tile_url_template = row_string(&row, 11).unwrap_or_default();
            let tileset_version = row_string(&row, 12).unwrap_or_default();
            let tile_px = row_i64(&row, 13, 512);
            let max_level = row_i64(&row, 14, 0);
            let y_flip = row_i64(&row, 15, 0);
            let lod_target_tiles = row_i64(&row, 16, 220);
            let lod_hysteresis_hi = row_f64(&row, 17, 280.0);
            let lod_hysteresis_lo = row_f64(&row, 18, 160.0);
            let lod_margin_tiles = row_i64(&row, 19, 1);
            let lod_enable_refine = row_i64(&row, 20, 0);
            let lod_refine_debounce_ms = row_i64(&row, 21, 0);
            let lod_max_detail_tiles = row_i64(&row, 22, 0);
            let visible_default = row_i64(&row, 23, 1);
            let opacity_default = row_f64(&row, 24, 1.0);
            let z_base = row_f64(&row, 25, 0.0);
            let display_order = row_i64(&row, 26, 0);
            let request_weight = row_f64(&row, 27, 1.0);
            let pick_mode = row_string(&row, 28).unwrap_or_else(|| "none".to_string());
            let layer_kind = row_string(&row, 29).unwrap_or_else(|| "tiled_raster".to_string());
            let vector_source_url = row_string(&row, 30);
            let vector_source_revision = row_string(&row, 31);
            let vector_geometry_space = row_string(&row, 32);
            let vector_style_mode = row_string(&row, 33);
            let vector_feature_id_property = row_string(&row, 34);
            let vector_color_property = row_string(&row, 35);

            let transform = parse_layer_transform(
                &transform_kind,
                affine_a,
                affine_b,
                affine_tx,
                affine_c,
                affine_d,
                affine_ty,
            );
            let manifest_url =
                resolve_layer_asset_url(&substitute_map_version(&manifest_url, map_version_id));
            let tile_url_template = resolve_layer_asset_url(&substitute_map_version(
                &tile_url_template,
                map_version_id,
            ));
            let kind = parse_layer_kind(&layer_id, &layer_kind)?;
            let vector_source = parse_vector_source(
                &layer_id,
                kind,
                VectorSourceFields {
                    source_url: vector_source_url,
                    source_revision: vector_source_revision,
                    geometry_space: vector_geometry_space,
                    style_mode: vector_style_mode,
                    feature_id_property: vector_feature_id_property,
                    color_property: vector_color_property,
                },
                map_version_id,
            )?;
            descriptors.push(LayerDescriptor {
                layer_id,
                name,
                enabled: true,
                kind,
                transform,
                tileset: TilesetRef {
                    manifest_url,
                    tile_url_template,
                    version: tileset_version,
                },
                tile_px: clamp_i64_to_u32(tile_px, 1).max(1),
                max_level: clamp_i64_to_u8(max_level, 0),
                y_flip: y_flip != 0,
                vector_source,
                lod_policy: LodPolicyDto {
                    target_tiles: clamp_i64_to_usize(lod_target_tiles, 1),
                    hysteresis_hi: lod_hysteresis_hi as f32,
                    hysteresis_lo: lod_hysteresis_lo as f32,
                    margin_tiles: clamp_i64_to_i32(lod_margin_tiles, 0),
                    enable_refine: lod_enable_refine != 0,
                    refine_debounce_ms: clamp_i64_to_u32(lod_refine_debounce_ms, 0),
                    max_detail_tiles: clamp_i64_to_usize(lod_max_detail_tiles, 0),
                    ..LodPolicyDto::default()
                },
                ui: LayerUiInfo {
                    visible_default: visible_default != 0,
                    opacity_default: opacity_default as f32,
                    z_base: z_base as f32,
                    display_order: clamp_i64_to_i32(display_order, 0),
                },
                request_weight: request_weight as f32,
                pick_mode: normalize_pick_mode(pick_mode),
            });
        }

        Ok(descriptors)
    }

    fn query_region_groups(&self, map_version_id: &str) -> AppResult<Vec<RegionGroupDescriptor>> {
        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;

        let meta_rows: Vec<Row> = match conn.exec(queries::REGION_GROUP_META_SQL, (map_version_id,))
        {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "region_group_meta") => {
                return Err(AppError::not_found(
                    "region_group_meta table is missing; apply api/sql/migrations/20260301_region_groups.sql",
                ));
            }
            Err(err) => return Err(db_unavailable(err)),
        };

        let region_rows: Vec<(i64, i64)> = match conn
            .exec(queries::REGION_GROUP_REGIONS_SQL, (map_version_id,))
        {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "region_group_regions") => {
                return Err(AppError::not_found(
                    "region_group_regions table is missing; apply api/sql/migrations/20260301_region_groups.sql",
                ));
            }
            Err(err) => return Err(db_unavailable(err)),
        };

        let mut groups: BTreeMap<u32, RegionGroupDescriptor> = BTreeMap::new();
        for row in meta_rows {
            let group_id = clamp_i64_to_u32(row_i64(&row, 1, 0), 0);
            if group_id == 0 {
                continue;
            }

            groups.insert(
                group_id,
                RegionGroupDescriptor {
                    region_group_id: group_id,
                    feature_count: clamp_i64_to_u32(row_i64(&row, 3, 0), 0),
                    region_count: clamp_i64_to_u32(row_i64(&row, 4, 0), 0),
                    accessible_region_count: clamp_i64_to_u32(row_i64(&row, 5, 0), 0),
                    color_rgb_u32: row_u32_opt(&row, 2),
                    bbox_min_x: row_opt_f64(&row, 6),
                    bbox_min_y: row_opt_f64(&row, 7),
                    bbox_max_x: row_opt_f64(&row, 8),
                    bbox_max_y: row_opt_f64(&row, 9),
                    graph_world_x: row_opt_f64(&row, 10),
                    graph_world_z: row_opt_f64(&row, 11),
                    source: row_string(&row, 12).unwrap_or_default(),
                    region_ids: Vec::new(),
                },
            );
        }

        for (region_group_id, region_id) in region_rows {
            let region_group_id = clamp_i64_to_u32(region_group_id, 0);
            let region_id = clamp_i64_to_u32(region_id, 0);
            if region_group_id == 0 || region_id == 0 {
                continue;
            }
            groups
                .entry(region_group_id)
                .or_insert_with(|| RegionGroupDescriptor {
                    region_group_id,
                    ..RegionGroupDescriptor::default()
                })
                .region_ids
                .push(region_id);
        }

        let mut out = Vec::with_capacity(groups.len());
        for mut group in groups.into_values() {
            group.region_ids.sort_unstable();
            group.region_ids.dedup();
            if group.region_count == 0 {
                group.region_count = u32::try_from(group.region_ids.len()).unwrap_or(0);
            }
            out.push(group);
        }
        Ok(out)
    }

    fn resolve_layer_revision_id(
        &self,
        explicit_layer_revision_id: Option<&str>,
        map_version_id: Option<&MapVersionId>,
        layer_id: Option<&str>,
        patch_id: Option<&str>,
        at_ts_utc: Option<i64>,
        window_to_ts_utc: i64,
    ) -> AppResult<String> {
        if let Some(value) = explicit_layer_revision_id {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }
        if let Some(value) = map_version_id {
            let trimmed = value.0.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_string());
            }
        }

        let layer_id = layer_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppError::invalid_argument(
                    "layer_revision_id is required (or provide layer_id with patch_id/at_ts_utc)",
                )
            })?;

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        if let Some(patch_id) = patch_id.map(str::trim).filter(|value| !value.is_empty()) {
            let row: Option<String> = match conn.exec_first(
                "SELECT layer_revision_id \
                 FROM layer_revisions \
                 WHERE layer_id = ? AND patch_id = ? \
                 ORDER BY created_at DESC \
                 LIMIT 1",
                (layer_id, patch_id),
            ) {
                Ok(value) => value,
                Err(err) if is_missing_table(&err, "layer_revisions") => {
                    return Err(AppError::unavailable(
                        "layer_revisions table missing; apply evidence layer revision migration",
                    ));
                }
                Err(err) => return Err(db_unavailable(err)),
            };
            return row.ok_or_else(|| {
                AppError::not_found(format!(
                    "no layer revision for layer_id={} patch_id={}",
                    layer_id, patch_id
                ))
            });
        }

        let at_ts = at_ts_utc.unwrap_or(window_to_ts_utc);
        let at_dt = epoch_to_mysql_datetime(at_ts)?;
        let row: Option<String> = match conn.exec_first(
            "SELECT layer_revision_id \
             FROM layer_revisions \
             WHERE layer_id = ? \
               AND (effective_from_utc IS NULL OR effective_from_utc <= ?) \
               AND (effective_to_utc IS NULL OR effective_to_utc > ?) \
             ORDER BY effective_from_utc DESC, created_at DESC \
             LIMIT 1",
            (layer_id, at_dt.as_str(), at_dt.as_str()),
        ) {
            Ok(value) => value,
            Err(err) if is_missing_table(&err, "layer_revisions") => {
                return Err(AppError::unavailable(
                    "layer_revisions table missing; apply evidence layer revision migration",
                ));
            }
            Err(err) => return Err(db_unavailable(err)),
        };
        row.ok_or_else(|| {
            AppError::not_found(format!(
                "no effective layer revision for layer_id={} at_ts_utc={}",
                layer_id, at_ts
            ))
        })
    }

    fn query_dolt_head_revision(&self) -> Option<String> {
        self.query_dolt_revision(None)
    }

    fn query_dolt_revision(&self, ref_id: Option<&str>) -> Option<String> {
        let ref_id = ref_id.map(str::trim).filter(|value| !value.is_empty());
        if let Some(value) = ref_id {
            validate_dolt_ref(value).ok()?;
        }
        let ref_id = ref_id.unwrap_or("HEAD").replace('\'', "''");
        let mut conn = self.pool.get_conn().ok()?;
        let query = format!("SELECT HASHOF('{ref_id}')");
        let hash: Option<String> = conn.query_first(query).ok()?;
        let hash = hash?;
        let hash = hash.trim();
        if hash.is_empty() {
            None
        } else {
            Some(format!("dolt:{hash}"))
        }
    }

    fn query_fish_names(
        &self,
        lang: FishLang,
        ref_id: Option<&str>,
    ) -> AppResult<HashMap<i32, String>> {
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };
        let fish_name_expr = match lang {
            FishLang::En => "en.`text`",
            FishLang::Ko => "k.name_ko",
        };
        let query = format!(
            "SELECT \
                k.fish_id, \
                {fish_name_expr} AS fish_name \
             FROM fish_names_ko{as_of} k \
             JOIN languagedata_en{as_of} en ON en.`id` = k.fish_id \
               AND en.`format` = 'A' \
               AND COALESCE(en.`unk`, '') = '' \
               AND NULLIF(TRIM(en.`text`), '') IS NOT NULL"
        );

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<(i64, Option<String>)> = conn.query(query).map_err(db_unavailable)?;

        let mut out = HashMap::new();
        for (fish_id, name) in rows {
            let fish_id = match i32::try_from(fish_id) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let Some(name) = normalize_optional_string(name) else {
                continue;
            };
            out.insert(fish_id, name);
        }

        Ok(out)
    }

    fn query_fish_catalog(
        &self,
        lang: FishLang,
        ref_id: Option<&str>,
    ) -> AppResult<Vec<FishCatalogRow>> {
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };
        let fish_name_expr = match lang {
            FishLang::En => "en.`text`",
            FishLang::Ko => "COALESCE(NULLIF(TRIM(f.name_ko), ''), en.`text`)",
        };
        // fish_names_ko can lag newer releases, so union the fish_table-only rows.
        let query = format!(
            "SELECT \
                f.fish_id, \
                ft.encyclopedia_key, \
                {fish_name_expr} AS fish_name, \
                it.`GradeType` AS grade_type, \
                NULLIF(ft.icon, '') AS fish_table_icon_file, \
                NULLIF(it.`IconImageFile`, '') AS item_icon_file, \
                NULLIF(ft.encyclopedia_icon, '') AS encyclopedia_icon_file, \
                it.`ItemName` AS item_name, \
                it.`Description` AS item_description, \
                it.`OriginalPrice` AS original_price \
             FROM fish_names_ko{as_of} f \
             JOIN languagedata_en{as_of} en ON en.`id` = f.fish_id \
               AND en.`format` = 'A' \
               AND COALESCE(en.`unk`, '') = '' \
               AND NULLIF(TRIM(en.`text`), '') IS NOT NULL \
             JOIN item_table{as_of} it ON it.`Index` = f.fish_id \
             LEFT JOIN fish_table{as_of} ft ON ft.item_key = f.fish_id \
             UNION ALL \
             SELECT \
                ft.item_key AS fish_id, \
                ft.encyclopedia_key, \
                {fish_name_expr} AS fish_name, \
                it.`GradeType` AS grade_type, \
                NULLIF(ft.icon, '') AS fish_table_icon_file, \
                NULLIF(it.`IconImageFile`, '') AS item_icon_file, \
                NULLIF(ft.encyclopedia_icon, '') AS encyclopedia_icon_file, \
                it.`ItemName` AS item_name, \
                it.`Description` AS item_description, \
                it.`OriginalPrice` AS original_price \
             FROM fish_table{as_of} ft \
             JOIN languagedata_en{as_of} en ON en.`id` = ft.item_key \
               AND en.`format` = 'A' \
               AND COALESCE(en.`unk`, '') = '' \
               AND NULLIF(TRIM(en.`text`), '') IS NOT NULL \
             LEFT JOIN item_table{as_of} it ON it.`Index` = ft.item_key \
             LEFT JOIN fish_names_ko{as_of} f ON f.fish_id = ft.item_key \
             WHERE f.fish_id IS NULL"
        );

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<FishCatalogDbRow> = conn.query(query).map_err(db_unavailable)?;

        let mut out = BTreeMap::new();
        for (
            fish_id,
            encyclopedia_key,
            name,
            grade_type,
            _fish_table_icon_file,
            _item_icon_file,
            encyclopedia_icon_file,
            item_name,
            description,
            original_price,
        ) in rows
        {
            let item_id = match i32::try_from(fish_id) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let encyclopedia_key = encyclopedia_key.and_then(|value| i32::try_from(value).ok());
            let Some(name) = normalize_optional_string(name) else {
                continue;
            };
            let item_name = normalize_optional_string(item_name);
            let (grade, grade_rank, is_prize) = fish_grade_from_db(grade_type);
            let encyclopedia_id = encyclopedia_icon_id_from_db(encyclopedia_icon_file);
            let is_dried = fish_is_dried(Some(name.as_str()), item_name.as_deref());
            let catch_methods = fish_catch_methods_from_description(description);
            let vendor_price = parse_positive_i64(original_price);

            merge_fish_catalog_row(
                &mut out,
                FishCatalogRow {
                    item_id,
                    encyclopedia_key,
                    encyclopedia_id,
                    name,
                    grade,
                    grade_rank,
                    is_prize,
                    is_dried,
                    catch_methods,
                    vendor_price,
                },
            );
        }

        Ok(out.into_values().collect())
    }

    fn query_zones(&self, ref_id: Option<&str>) -> AppResult<Vec<ZoneEntry>> {
        let mut query = queries::ZONES_SQL.to_string();
        if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            query.push_str(&format!(" AS OF '{}'", ref_id));
        }

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<(i64, i64, i64, Option<String>)> =
            conn.query(query).map_err(db_unavailable)?;

        let mut zones = Vec::with_capacity(rows.len());
        for (r, g, b, name) in rows {
            let r =
                u8::try_from(r).map_err(|_| AppError::internal("zones_merged R out of range"))?;
            let g =
                u8::try_from(g).map_err(|_| AppError::internal("zones_merged G out of range"))?;
            let b =
                u8::try_from(b).map_err(|_| AppError::internal("zones_merged B out of range"))?;
            let rgb = Rgb { r, g, b };
            zones.push(ZoneEntry {
                rgb_u32: rgb.to_u32(),
                rgb,
                rgb_key: rgb.key(),
                name: normalize_optional_string(name),
            });
        }
        zones.sort_by_key(|zone| zone.rgb_u32);
        Ok(zones)
    }

    fn query_fish_identities(&self, ref_id: Option<&str>) -> AppResult<FishIdentityIndex> {
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };
        let query = format!(
            "SELECT \
                ft.encyclopedia_key, \
                ft.item_key, \
                en.`text` AS localized_name, \
                ft.icon, \
                ft.encyclopedia_icon \
             FROM fish_table{as_of} ft \
             JOIN languagedata_en{as_of} en ON en.`id` = ft.item_key \
               AND en.`format` = 'A' \
               AND COALESCE(en.`unk`, '') = '' \
               AND NULLIF(TRIM(en.`text`), '') IS NOT NULL"
        );

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<FishIdentityDbRow> = conn.query(query).map_err(db_unavailable)?;

        let mut by_encyclopedia = HashMap::new();
        for (enc, item, name, _icon, encyclopedia_icon) in rows {
            let enc = match i32::try_from(enc) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let item = match i32::try_from(item) {
                Ok(value) => value,
                Err(_) => continue,
            };

            let entry = FishIdentityEntry {
                encyclopedia_key: enc,
                item_id: item,
                encyclopedia_id: encyclopedia_icon_id_from_db(encyclopedia_icon),
                name: normalize_optional_string(name),
            };
            by_encyclopedia.insert(enc, entry);
        }

        Ok(FishIdentityIndex { by_encyclopedia })
    }

    fn has_event_zone_assignment(&self, layer_revision_id: &str) -> AppResult<bool> {
        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let count: Option<i64> = match conn.exec_first(
            queries::EVENT_ZONE_ASSIGNMENT_COUNT_SQL,
            (layer_revision_id,),
        ) {
            Ok(count) => count,
            Err(err) if is_missing_table(&err, "event_zone_assignment") => return Ok(false),
            Err(err) => return Err(db_unavailable(err)),
        };
        Ok(count.unwrap_or(0) > 0)
    }

    fn load_water_tiles(&self, map_version: &str, tile_px: i32) -> AppResult<(i32, i32, Vec<u32>)> {
        if tile_px <= 0 {
            return Err(AppError::invalid_argument("tile_px must be > 0"));
        }

        let (grid_w, grid_h) = tile_dimensions(MAP_WIDTH, MAP_HEIGHT, tile_px);
        let len = (grid_w * grid_h) as usize;
        let mut values: Vec<Option<u32>> = vec![None; len];

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<(i64, i64, i64)> =
            match conn.exec(queries::WATER_TILES_SQL, (map_version, tile_px)) {
                Ok(rows) => rows,
                Err(err) if is_missing_table(&err, "water_tiles") => {
                    return Err(AppError::not_found(format!(
                        "water_tiles missing for map_version={} tile_px={tile_px}",
                        map_version
                    )));
                }
                Err(err) => return Err(db_unavailable(err)),
            };

        if rows.is_empty() {
            return Err(AppError::not_found(format!(
                "water_tiles missing for map_version={} tile_px={tile_px}",
                map_version
            )));
        }

        for (tile_x, tile_y, water_count) in rows {
            let tile_x =
                i32::try_from(tile_x).map_err(|_| AppError::internal("tile_x out of range"))?;
            let tile_y =
                i32::try_from(tile_y).map_err(|_| AppError::internal("tile_y out of range"))?;
            let water_count = u32::try_from(water_count)
                .map_err(|_| AppError::internal("water_count out of range"))?;

            if tile_x < 0 || tile_y < 0 || tile_x >= grid_w || tile_y >= grid_h {
                return Err(AppError::internal(format!(
                    "water_tiles out of bounds: tile_x={tile_x}, tile_y={tile_y}, grid={grid_w}x{grid_h}"
                )));
            }

            let idx = (tile_y * grid_w + tile_x) as usize;
            values[idx] = Some(water_count);
        }

        if values.iter().any(Option::is_none) {
            return Err(AppError::internal(format!(
                "water_tiles incomplete for tile_px={tile_px}"
            )));
        }

        Ok((
            grid_w,
            grid_h,
            values.into_iter().map(|v| v.unwrap_or_default()).collect(),
        ))
    }

    // Ranking-derived analytics must stay source-filtered so future event kinds
    // cannot silently contaminate ranking evidence semantics.
    fn load_ranking_events_with_zone_in_window(
        &self,
        layer_revision_id: &str,
        from_ts_utc: i64,
        to_ts_utc: i64,
    ) -> AppResult<Vec<EventZoneRow>> {
        let from_dt = epoch_to_mysql_datetime(from_ts_utc)?;
        let to_dt = epoch_to_mysql_datetime(to_ts_utc)?;
        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<(i64, i64, i64, i64, i64)> = conn
            .exec(
                queries::RANKING_EVENTS_WITH_ZONE_SQL,
                (layer_revision_id, SOURCE_KIND_RANKING, from_dt, to_dt),
            )
            .map_err(events_schema_or_db_unavailable)?;

        let mut out = Vec::with_capacity(rows.len());
        for (ts_utc, fish_id, sample_px_x, sample_px_y, zone_rgb_u32) in rows {
            out.push(EventZoneRow {
                ts_utc,
                fish_id: i32::try_from(fish_id)
                    .map_err(|_| AppError::internal("fish_id out of range"))?,
                sample_px_x: i32::try_from(sample_px_x)
                    .map_err(|_| AppError::internal("sample_px_x out of range"))?,
                sample_px_y: i32::try_from(sample_px_y)
                    .map_err(|_| AppError::internal("sample_px_y out of range"))?,
                zone_rgb_u32: u32::try_from(zone_rgb_u32)
                    .map_err(|_| AppError::internal("zone_rgb_u32 out of range"))?,
            });
        }
        Ok(out)
    }

    fn query_events_snapshot_meta(&self) -> AppResult<EventsSnapshotMetaResponse> {
        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let row: Option<EventsSnapshotMetaDbRow> = conn
            .exec_first(
                "SELECT \
                    COUNT(1) AS event_count, \
                    MAX(CAST(TIMESTAMPDIFF(SECOND, '1970-01-01 00:00:00', e.ts_utc) AS SIGNED)) AS max_ts_utc, \
                    MAX(e.event_id) AS max_event_id, \
                    DATE_FORMAT(MAX(e.ts_utc), '%Y-%m-%d %H:%i:%s.%f') AS last_updated_utc \
                 FROM events e \
                 WHERE e.water_ok = 1 \
                   AND e.source_kind = ?",
                (SOURCE_KIND_RANKING,),
            )
            .map_err(events_schema_or_db_unavailable)?;

        let (event_count, max_ts_utc, max_event_id, last_updated_utc) =
            row.unwrap_or((0, None, None, None));
        let event_count = usize::try_from(event_count)
            .map_err(|_| AppError::internal("event_count out of range"))?;
        let source_revision = self.query_dolt_head_revision();
        let revision = synthetic_events_snapshot_revision(
            source_revision.as_deref(),
            event_count,
            max_ts_utc,
            max_event_id,
        );

        Ok(EventsSnapshotMetaResponse {
            revision: revision.clone(),
            event_count,
            source_kind: EventSourceKind::Ranking,
            last_updated_utc: normalize_optional_string(last_updated_utc),
            snapshot_url: format!("/api/v1/events_snapshot?revision={revision}"),
        })
    }

    fn load_events_snapshot(&self) -> AppResult<Vec<EventPointCompact>> {
        let layer_revision_id = self
            .defaults
            .map_version_id
            .as_ref()
            .map(|id| id.0.as_str())
            .unwrap_or("v1");
        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<EventPointCompactDbRow> = conn
            .exec(
                "SELECT \
                    e.event_id, \
                    e.fish_id, \
                    CAST(TIMESTAMPDIFF(SECOND, '1970-01-01 00:00:00', e.ts_utc) AS SIGNED) AS ts_utc, \
                    e.length_milli, \
                    e.map_px_x, \
                    e.map_px_y, \
                    e.world_x, \
                    e.world_z, \
                    z.zone_rgb, \
                    e.source_kind, \
                    e.source_id \
                 FROM events e \
                 LEFT JOIN event_zone_assignment z \
                   ON z.event_id = e.event_id \
                  AND z.layer_revision_id = ? \
                 WHERE e.water_ok = 1 \
                   AND e.source_kind = ? \
                 ORDER BY e.ts_utc, e.event_id",
                (layer_revision_id, SOURCE_KIND_RANKING),
            )
            .map_err(events_schema_or_db_unavailable)?;

        let mut out = Vec::with_capacity(rows.len());
        for (
            event_id,
            fish_id,
            ts_utc,
            length_milli,
            map_px_x,
            map_px_y,
            world_x,
            world_z,
            zone_rgb_u32,
            source_kind,
            source_id,
        ) in rows
        {
            out.push(EventPointCompact {
                event_id,
                fish_id: i32::try_from(fish_id)
                    .map_err(|_| AppError::internal("fish_id out of range"))?,
                ts_utc,
                map_px_x: i32::try_from(map_px_x)
                    .map_err(|_| AppError::internal("map_px_x out of range"))?,
                map_px_y: i32::try_from(map_px_y)
                    .map_err(|_| AppError::internal("map_px_y out of range"))?,
                length_milli: i32::try_from(length_milli)
                    .map_err(|_| AppError::internal("length_milli out of range"))?,
                world_x: world_x
                    .map(|value| {
                        i32::try_from(value).map_err(|_| AppError::internal("world_x out of range"))
                    })
                    .transpose()?,
                world_z: world_z
                    .map(|value| {
                        i32::try_from(value).map_err(|_| AppError::internal("world_z out of range"))
                    })
                    .transpose()?,
                zone_rgb_u32: zone_rgb_u32
                    .map(|value| {
                        u32::try_from(value)
                            .map_err(|_| AppError::internal("zone_rgb_u32 out of range"))
                    })
                    .transpose()?,
                source_kind: event_source_kind_from_db(source_kind),
                source_id: normalize_optional_string(source_id),
            });
        }
        Ok(out)
    }

    fn build_event_fish_names(
        item_names: &HashMap<i32, String>,
        fish_table: &FishIdentityIndex,
    ) -> HashMap<i32, String> {
        let mut out = item_names.clone();
        for entry in fish_table.by_encyclopedia.values() {
            let resolved_name = item_names
                .get(&entry.item_id)
                .cloned()
                .or_else(|| entry.name.clone());
            let Some(name) = resolved_name else {
                continue;
            };
            out.entry(entry.item_id).or_insert_with(|| name.clone());
            out.insert(entry.encyclopedia_key, name);
        }
        out
    }

    fn build_event_fish_identity_map(
        fish_table: &FishIdentityIndex,
    ) -> HashMap<i32, (i32, Option<i32>, Option<i32>)> {
        let mut out = HashMap::new();
        for entry in fish_table.by_encyclopedia.values() {
            let identity = (
                entry.item_id,
                Some(entry.encyclopedia_key),
                entry.encyclopedia_id,
            );
            out.insert(entry.encyclopedia_key, identity);
            out.entry(entry.item_id).or_insert(identity);
        }
        out
    }

    fn compute_effort_grid(&self, params: &QueryParams) -> AppResult<EffortGridResponse> {
        params.validate()?;
        if !self.has_event_zone_assignment(&params.map_version)? {
            return Err(AppError::not_found(format!(
                "event_zone_assignment missing for layer_revision_id={}",
                params.map_version
            )));
        }

        let tile_px = i32::try_from(params.tile_px)
            .map_err(|_| AppError::invalid_argument("tile_px out of range"))?;
        let (grid_w, grid_h) = tile_dimensions(MAP_WIDTH, MAP_HEIGHT, tile_px);
        let water_counts = match self.load_water_tiles(&params.map_version, tile_px) {
            Ok((_, _, counts)) => counts,
            Err(_) => vec![1_u32; (grid_w * grid_h) as usize],
        };
        let events = self.load_ranking_events_with_zone_in_window(
            &params.map_version,
            params.from_ts_utc,
            params.to_ts_utc,
        )?;

        let len = (grid_w * grid_h) as usize;
        let mut e_raw = vec![0.0f64; len];
        for ev in &events {
            let Some(idx) =
                pixel_to_tile_index(grid_w, grid_h, tile_px, ev.sample_px_x, ev.sample_px_y)
            else {
                continue;
            };
            let w_time = time_weight(params, ev.ts_utc)?;
            e_raw[idx] += w_time;
        }

        let m: Vec<f64> = water_counts.into_iter().map(|v| v as f64).collect();
        let e_blur =
            gaussian_blur_grid(&e_raw, grid_w as usize, grid_h as usize, params.sigma_tiles);
        let m_blur = gaussian_blur_grid(&m, grid_w as usize, grid_h as usize, params.sigma_tiles);

        let mut effort = Vec::with_capacity(len);
        for idx in 0..len {
            effort.push(e_blur[idx] / m_blur[idx].max(EPS));
        }

        Ok(EffortGridResponse {
            tile_px: params.tile_px,
            grid_w,
            grid_h,
            sigma_tiles: params.sigma_tiles,
            values: effort,
        })
    }

    fn compute_zone_stats(
        &self,
        zones_meta: &HashMap<u32, ZoneEntry>,
        fish_names: &HashMap<i32, String>,
        fish_identities: &HashMap<i32, (i32, Option<i32>, Option<i32>)>,
        params: &QueryParams,
        zone_rgb_u32: u32,
        status_cfg: &ZoneStatusConfig,
    ) -> AppResult<ZoneStatsResponse> {
        params.validate()?;
        let window = ZoneStatsWindow {
            from_ts_utc: params.from_ts_utc,
            to_ts_utc: params.to_ts_utc,
            half_life_days: params.half_life_days,
            fish_norm: params.fish_norm,
            tile_px: params.tile_px,
            sigma_tiles: params.sigma_tiles,
            alpha0: params.alpha0,
        };

        if !self.has_event_zone_assignment(&params.map_version)? {
            return Err(AppError::not_found(format!(
                "event_zone_assignment missing for layer_revision_id={}",
                params.map_version
            )));
        }

        let summary = self.compute_window_summary(params, zone_rgb_u32)?;
        if summary.alpha_by_fish.is_empty() || summary.alpha_total <= 0.0 {
            return Ok(ZoneStatsResponse {
                zone_rgb_u32,
                zone_rgb: Rgb::from_u32(zone_rgb_u32).key(),
                zone_name: zones_meta
                    .get(&zone_rgb_u32)
                    .and_then(|zone| zone.name.clone()),
                window,
                confidence: ZoneConfidence {
                    ess: 0.0,
                    total_weight: summary.total_weight,
                    last_seen_ts_utc: summary.last_seen,
                    age_days_last: None,
                    status: ZoneStatus::Unknown,
                    notes: vec!["no evidence in window".to_string()],
                    drift: None,
                },
                distribution: Vec::new(),
            });
        }

        let mut distribution = Vec::new();
        for fish_id in zone_distribution_fish_ids(&summary) {
            let p_mean = summary.p_mean_by_fish.get(&fish_id).copied().unwrap_or(0.0);
            let evidence = summary.c_zone.get(&fish_id).copied().unwrap_or(0.0);
            let (item_id, encyclopedia_key, encyclopedia_id) = fish_identities
                .get(&fish_id)
                .copied()
                .unwrap_or((fish_id, None, None));

            distribution.push(ZoneFishEvidence {
                fish_id,
                item_id,
                encyclopedia_key,
                encyclopedia_id,
                fish_name: fish_names.get(&fish_id).cloned(),
                evidence_weight: evidence,
                p_mean,
                ci_low: None,
                ci_high: None,
            });
        }

        distribution.sort_by(|left, right| {
            right
                .p_mean
                .partial_cmp(&left.p_mean)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.fish_id.cmp(&right.fish_id))
        });
        if distribution.len() > params.top_k {
            distribution.truncate(params.top_k);
        }

        let mut with_ci = Vec::with_capacity(distribution.len());
        for mut fish in distribution {
            let alpha = summary
                .alpha_by_fish
                .get(&fish.fish_id)
                .copied()
                .unwrap_or(0.0);
            let beta = (summary.alpha_total - alpha).max(0.0);
            if alpha > 0.0 && beta > 0.0 {
                let seed = seed_from_params(
                    &params.map_version,
                    zone_rgb_u32,
                    fish.fish_id,
                    params.from_ts_utc,
                    params.to_ts_utc,
                );
                let (low, high) = beta_ci(alpha, beta, seed, 2000);
                fish.ci_low = Some(low);
                fish.ci_high = Some(high);
            }
            with_ci.push(fish);
        }

        let ess = summary.ess;
        let (mut status, age_days_last, mut notes) = compute_status(
            summary.total_weight,
            summary.last_seen,
            params.to_ts_utc,
            ess,
            status_cfg,
        );

        let mut drift = None;
        if let Some(boundary) = params.drift_boundary_ts {
            let (drift_info, drifting, drift_note) =
                self.compute_drift_info(params, zone_rgb_u32, boundary, status_cfg)?;
            drift = drift_info;
            if let Some(note) = drift_note {
                notes.push(note);
            }
            if status != ZoneStatus::Unknown && drifting {
                status = ZoneStatus::Drifting;
            }
        }

        Ok(ZoneStatsResponse {
            zone_rgb_u32,
            zone_rgb: Rgb::from_u32(zone_rgb_u32).key(),
            zone_name: zones_meta
                .get(&zone_rgb_u32)
                .and_then(|zone| zone.name.clone()),
            window,
            confidence: ZoneConfidence {
                ess,
                total_weight: summary.total_weight,
                last_seen_ts_utc: summary.last_seen,
                age_days_last,
                status,
                notes,
                drift,
            },
            distribution: with_ci,
        })
    }

    fn compute_window_summary(
        &self,
        params: &QueryParams,
        zone_rgb_u32: u32,
    ) -> AppResult<WindowSummary> {
        let events = self.load_ranking_events_with_zone_in_window(
            &params.map_version,
            params.from_ts_utc,
            params.to_ts_utc,
        )?;

        if events.is_empty() {
            return Ok(WindowSummary {
                alpha_total: 0.0,
                alpha_by_fish: HashMap::new(),
                p_mean_by_fish: HashMap::new(),
                c_zone: HashMap::new(),
                ess: 0.0,
                total_weight: 0.0,
                last_seen: None,
            });
        }

        let mut derived: Vec<DerivedEvent> = Vec::with_capacity(events.len());
        let mut fish_time: HashMap<i32, f64> = HashMap::new();

        for event in events {
            let w_time = time_weight(params, event.ts_utc)?;
            *fish_time.entry(event.fish_id).or_insert(0.0) += w_time;
            derived.push(DerivedEvent {
                ts_utc: event.ts_utc,
                fish_id: event.fish_id,
                zone_rgb_u32: event.zone_rgb_u32,
                w_time,
            });
        }

        let mut fish_norm = HashMap::new();
        if params.fish_norm {
            for (fish_id, sum) in fish_time {
                fish_norm.insert(fish_id, 1.0 / sum.max(EPS_FISH));
            }
        }

        let mut c_global: HashMap<i32, f64> = HashMap::new();
        let mut c_zone: HashMap<i32, f64> = HashMap::new();
        let mut w_sum = 0.0;
        let mut w2_sum = 0.0;
        let mut last_seen: Option<i64> = None;

        for event in derived {
            let u = event.w_time;
            let w = if params.fish_norm {
                u * fish_norm.get(&event.fish_id).copied().unwrap_or(0.0)
            } else {
                u
            };

            *c_global.entry(event.fish_id).or_insert(0.0) += w;
            if event.zone_rgb_u32 == zone_rgb_u32 {
                *c_zone.entry(event.fish_id).or_insert(0.0) += w;
                w_sum += u;
                w2_sum += u * u;
                last_seen = Some(last_seen.map_or(event.ts_utc, |prev| prev.max(event.ts_utc)));
            }
        }

        let total_global: f64 = c_global.values().sum();
        if total_global <= 0.0 {
            let ess = if w2_sum > 0.0 {
                (w_sum * w_sum) / w2_sum.max(EPS)
            } else {
                0.0
            };
            return Ok(WindowSummary {
                alpha_total: 0.0,
                alpha_by_fish: HashMap::new(),
                p_mean_by_fish: HashMap::new(),
                c_zone,
                ess,
                total_weight: w_sum,
                last_seen,
            });
        }

        let mut fish_ids: Vec<i32> = c_global.keys().copied().collect();
        fish_ids.sort_unstable();

        let mut alpha_total = params.alpha0;
        let mut alpha_by_fish = HashMap::new();
        let mut p_mean_by_fish = HashMap::new();
        for fish_id in &fish_ids {
            let p0 = c_global.get(fish_id).copied().unwrap_or(0.0) / total_global;
            let c = c_zone.get(fish_id).copied().unwrap_or(0.0);
            let alpha = params.alpha0 * p0 + c;
            alpha_total += c;
            alpha_by_fish.insert(*fish_id, alpha);
        }

        for (fish_id, alpha) in &alpha_by_fish {
            p_mean_by_fish.insert(*fish_id, *alpha / alpha_total);
        }

        let ess = if w2_sum > 0.0 {
            (w_sum * w_sum) / w2_sum.max(EPS)
        } else {
            0.0
        };

        Ok(WindowSummary {
            alpha_total,
            alpha_by_fish,
            p_mean_by_fish,
            c_zone,
            ess,
            total_weight: w_sum,
            last_seen,
        })
    }

    fn compute_drift_info(
        &self,
        params: &QueryParams,
        zone_rgb_u32: u32,
        boundary: i64,
        cfg: &ZoneStatusConfig,
    ) -> AppResult<(Option<DriftInfo>, bool, Option<String>)> {
        let mut old_params = params.clone();
        old_params.to_ts_utc = boundary;
        old_params.drift_boundary_ts = None;

        let mut new_params = params.clone();
        new_params.from_ts_utc = boundary;
        new_params.drift_boundary_ts = None;

        let old = self.compute_window_summary(&old_params, zone_rgb_u32)?;
        let new = self.compute_window_summary(&new_params, zone_rgb_u32)?;

        let mut union: Vec<i32> = old
            .alpha_by_fish
            .keys()
            .chain(new.alpha_by_fish.keys())
            .copied()
            .collect();
        union.sort_unstable();
        union.dedup();

        if union.is_empty() {
            return Ok((None, false, Some("drift skipped: no evidence".to_string())));
        }

        let p_old = align_probs(&old.p_mean_by_fish, &union);
        let p_new = align_probs(&new.p_mean_by_fish, &union);
        let jsd_mean = js_divergence(&p_old, &p_new);

        let mut p_drift = 0.0;
        let mut drifting = false;
        let mut note = None;
        if old.ess >= cfg.drift_min_ess && new.ess >= cfg.drift_min_ess {
            let alpha_old = align_alpha(&old.alpha_by_fish, &union);
            let alpha_new = align_alpha(&new.alpha_by_fish, &union);
            let seed = seed_from_drift(
                &params.map_version,
                zone_rgb_u32,
                boundary,
                params.from_ts_utc,
                params.to_ts_utc,
            );
            let mut rng = XorShift64::new(seed);
            let mut count = 0usize;
            for _ in 0..cfg.drift_samples {
                let s_old = sample_dirichlet(&alpha_old, &mut rng);
                let s_new = sample_dirichlet(&alpha_new, &mut rng);
                let jsd = js_divergence(&s_old, &s_new);
                if jsd > cfg.drift_jsd_threshold {
                    count += 1;
                }
            }
            p_drift = count as f64 / cfg.drift_samples as f64;
            drifting = p_drift >= cfg.drift_prob_threshold;
        } else {
            note = Some("drift skipped: insufficient ESS".to_string());
        }

        let info = DriftInfo {
            boundary_ts_utc: boundary,
            jsd_mean,
            p_drift,
            ess_old: old.ess,
            ess_new: new.ess,
            samples: cfg.drift_samples,
            jsd_threshold: cfg.drift_jsd_threshold,
        };
        Ok((Some(info), drifting, note))
    }
}

fn zone_distribution_fish_ids(summary: &WindowSummary) -> Vec<i32> {
    let mut fish_ids: Vec<i32> = summary.c_zone.keys().copied().collect();
    fish_ids.sort_unstable();
    fish_ids
}

#[async_trait]
impl Store for DoltMySqlStore {
    async fn get_meta(&self) -> AppResult<MetaResponse> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            let patches = this.query_patches()?;
            let map_versions = this.query_map_versions()?;
            let default_patch = patches.last().cloned();
            let canonical_map = CanonicalMapInfo::default();
            Ok(MetaResponse {
                api_version: API_VERSION.to_string(),
                terrain_manifest_url: None,
                terrain_drape_manifest_url: None,
                terrain_height_tiles_url: None,
                canonical_map,
                patches,
                default_patch,
                map_versions,
                defaults: this.defaults.clone(),
            })
        })
        .await
        .map_err(|err| AppError::internal(err.to_string()))?
    }

    async fn get_layers(&self, map_version_id: Option<String>) -> AppResult<LayersResponse> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            let selected_map_version =
                map_version_id.or_else(|| this.defaults.map_version_id.clone().map(|id| id.0));
            let layers = this.query_layer_descriptors(selected_map_version.as_deref())?;
            let revision = this.query_dolt_head_revision().unwrap_or_else(|| {
                synthetic_layers_revision(selected_map_version.as_deref(), &layers)
            });
            Ok(LayersResponse {
                revision,
                map_version_id: selected_map_version.map(MapVersionId),
                layers,
            })
        })
        .await
        .map_err(|err| AppError::internal(err.to_string()))?
    }

    async fn get_region_groups(
        &self,
        map_version_id: Option<String>,
    ) -> AppResult<RegionGroupsResponse> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            let selected_map_version =
                map_version_id.or_else(|| this.defaults.map_version_id.clone().map(|id| id.0));
            let groups = if let Some(map_version) = selected_map_version.as_deref() {
                this.query_region_groups(map_version)?
            } else {
                Vec::new()
            };
            let revision = this.query_dolt_head_revision().unwrap_or_else(|| {
                synthetic_region_groups_revision(selected_map_version.as_deref(), &groups)
            });
            Ok(RegionGroupsResponse {
                revision,
                map_version_id: selected_map_version.map(MapVersionId),
                groups,
            })
        })
        .await
        .map_err(|err| AppError::internal(err.to_string()))?
    }

    async fn list_fish(
        &self,
        lang: FishLang,
        ref_id: Option<String>,
    ) -> AppResult<FishListResponse> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            let mut fish = this.query_fish_catalog(lang, ref_id.as_deref())?;
            fish.sort_by(|left, right| {
                right
                    .is_prize
                    .unwrap_or(false)
                    .cmp(&left.is_prize.unwrap_or(false))
                    .then_with(|| {
                        right
                            .grade_rank
                            .unwrap_or_default()
                            .cmp(&left.grade_rank.unwrap_or_default())
                    })
                    .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
                    .then_with(|| left.item_id.cmp(&right.item_id))
            });
            let revision = this
                .query_dolt_revision(ref_id.as_deref())
                .unwrap_or_else(|| synthetic_fish_revision(ref_id.as_deref(), &fish));
            let entries: Vec<FishEntry> = fish
                .into_iter()
                .map(|entry| FishEntry {
                    item_id: entry.item_id,
                    encyclopedia_key: entry.encyclopedia_key,
                    encyclopedia_id: entry.encyclopedia_id,
                    name: entry.name,
                    grade: entry.grade,
                    is_prize: entry.is_prize,
                    is_dried: entry.is_dried,
                    catch_methods: entry.catch_methods,
                    vendor_price: entry.vendor_price,
                })
                .collect();
            Ok(FishListResponse {
                revision,
                count: entries.len(),
                fish: entries,
            })
        })
        .await
        .map_err(|err| AppError::internal(err.to_string()))?
    }

    async fn list_zones(&self, ref_id: Option<String>) -> AppResult<Vec<ZoneEntry>> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.query_zones(ref_id.as_deref()))
            .await
            .map_err(|err| AppError::internal(err.to_string()))?
    }

    async fn zone_stats(
        &self,
        request: ZoneStatsRequest,
        status_cfg: ZoneStatusConfig,
    ) -> AppResult<ZoneStatsResponse> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            let zone_rgb_u32 = request.rgb.to_u32().map_err(AppError::invalid_argument)?;
            let layer_revision_id = this.resolve_layer_revision_id(
                request.layer_revision_id.as_deref(),
                request.map_version_id.as_ref(),
                request.layer_id.as_deref(),
                request.patch_id.as_deref(),
                request.at_ts_utc,
                request.to_ts_utc,
            )?;

            let params = QueryParams {
                map_version: layer_revision_id,
                from_ts_utc: request.from_ts_utc,
                to_ts_utc: request.to_ts_utc,
                half_life_days: request.half_life_days,
                tile_px: request.tile_px,
                sigma_tiles: request.sigma_tiles,
                fish_norm: request.fish_norm,
                alpha0: request.alpha0,
                top_k: request.top_k,
                drift_boundary_ts: request.drift_boundary_ts_utc,
            };
            params.validate()?;

            let lang = FishLang::from_param(request.lang.as_deref());
            let fish_names = this.query_fish_names(lang, request.ref_id.as_deref())?;
            let fish_table = this.query_fish_identities(request.ref_id.as_deref())?;
            let zones_vec = this.query_zones(request.ref_id.as_deref())?;
            let zones: HashMap<u32, ZoneEntry> = zones_vec
                .into_iter()
                .map(|zone| (zone.rgb_u32, zone))
                .collect();
            let event_fish_names = DoltMySqlStore::build_event_fish_names(&fish_names, &fish_table);
            let event_fish_identities = DoltMySqlStore::build_event_fish_identity_map(&fish_table);
            this.compute_zone_stats(
                &zones,
                &event_fish_names,
                &event_fish_identities,
                &params,
                zone_rgb_u32,
                &status_cfg,
            )
        })
        .await
        .map_err(|err| AppError::internal(err.to_string()))?
    }

    async fn zone_profile_v2(
        &self,
        request: ZoneProfileV2Request,
        status_cfg: ZoneStatusConfig,
    ) -> AppResult<ZoneProfileV2Response> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.compute_zone_profile_v2(request, status_cfg))
            .await
            .map_err(|err| AppError::internal(err.to_string()))?
    }

    async fn effort_grid(&self, request: EffortGridRequest) -> AppResult<EffortGridResponse> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            let params = QueryParams {
                map_version: request.map_version_id.0,
                from_ts_utc: request.from_ts_utc,
                to_ts_utc: request.to_ts_utc,
                half_life_days: request.half_life_days,
                tile_px: request.tile_px,
                sigma_tiles: request.sigma_tiles,
                fish_norm: false,
                alpha0: 1.0,
                top_k: 30,
                drift_boundary_ts: None,
            };
            this.compute_effort_grid(&params)
        })
        .await
        .map_err(|err| AppError::internal(err.to_string()))?
    }

    async fn events_snapshot_meta(&self) -> AppResult<EventsSnapshotMetaResponse> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.query_events_snapshot_meta())
            .await
            .map_err(|err| AppError::internal(err.to_string()))?
    }

    async fn events_snapshot(
        &self,
        requested_revision: Option<String>,
    ) -> AppResult<EventsSnapshotResponse> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            let meta = this.query_events_snapshot_meta()?;
            if let Some(requested_revision) = requested_revision {
                if requested_revision != meta.revision {
                    return Err(AppError(ApiError::conflict(format!(
                        "snapshot revision mismatch: requested={} current={}",
                        requested_revision, meta.revision
                    ))));
                }
            }
            let events = this.load_events_snapshot()?;
            Ok(EventsSnapshotResponse {
                revision: meta.revision,
                event_count: meta.event_count,
                source_kind: EventSourceKind::Ranking,
                events,
            })
        })
        .await
        .map_err(|err| AppError::internal(err.to_string()))?
    }

    async fn healthcheck(&self) -> AppResult<()> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = this.pool.get_conn().map_err(db_unavailable)?;
            let _: Vec<(i32,)> = conn
                .query(queries::HEALTHCHECK_SQL)
                .map_err(db_unavailable)?;
            Ok(())
        })
        .await
        .map_err(|err| AppError::internal(err.to_string()))?
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use fishystuff_api::error::ApiErrorCode;
    use fishystuff_api::models::layers::{GeometrySpace, LayerKind, StyleMode};
    use fishystuff_api::models::zone_stats::ZoneStatus;

    use crate::config::ZoneStatusConfig;

    use super::{
        catalog::{encyclopedia_icon_id_from_db, is_web_icon_path},
        compute_status, event_source_kind_from_db, fish_catch_methods_from_description,
        fish_is_dried, merge_fish_catalog_row, parse_layer_kind, parse_positive_i64,
        parse_vector_source, pixel_to_tile_index, resolve_layer_asset_url,
        synthetic_events_snapshot_revision, zone_distribution_fish_ids, DoltMySqlStore,
        FishCatalogRow, FishIdentityEntry, FishIdentityIndex, VectorSourceFields, WindowSummary,
    };

    fn vector_source_fields(
        source_url: Option<&str>,
        source_revision: Option<&str>,
        geometry_space: Option<&str>,
        style_mode: Option<&str>,
        feature_id_property: Option<&str>,
        color_property: Option<&str>,
    ) -> VectorSourceFields {
        VectorSourceFields {
            source_url: source_url.map(str::to_string),
            source_revision: source_revision.map(str::to_string),
            geometry_space: geometry_space.map(str::to_string),
            style_mode: style_mode.map(str::to_string),
            feature_id_property: feature_id_property.map(str::to_string),
            color_property: color_property.map(str::to_string),
        }
    }

    #[test]
    fn vector_layer_requires_non_empty_source_url() {
        let err = parse_vector_source(
            "water",
            LayerKind::VectorGeoJson,
            vector_source_fields(
                None,
                Some("wm-v1"),
                Some("map_pixels"),
                Some("feature_property_palette"),
                None,
                Some("c"),
            ),
            Some("v1"),
        )
        .expect_err("expected invalid vector source");
        assert_eq!(err.0.code, ApiErrorCode::InvalidArgument);
        assert!(err.0.message.contains("vector_source_url"));
    }

    #[test]
    fn fish_catalog_dedup_keeps_one_entry_per_fish() {
        let mut rows = std::collections::BTreeMap::new();
        merge_fish_catalog_row(
            &mut rows,
            FishCatalogRow {
                item_id: 8475,
                encyclopedia_key: Some(8475),
                encyclopedia_id: Some(9475),
                name: "Albino Coelacanth".to_string(),
                grade: Some("Prize".to_string()),
                grade_rank: Some(4),
                is_prize: Some(true),
                is_dried: false,
                catch_methods: vec!["harpoon".to_string()],
                vendor_price: Some(120_000_000),
            },
        );
        merge_fish_catalog_row(
            &mut rows,
            FishCatalogRow {
                item_id: 8475,
                encyclopedia_key: Some(8475),
                encyclopedia_id: None,
                name: "Albino Coelacanth".to_string(),
                grade: Some("General".to_string()),
                grade_rank: Some(1),
                is_prize: Some(false),
                is_dried: false,
                catch_methods: vec!["rod".to_string()],
                vendor_price: Some(88_800),
            },
        );

        let row = rows.get(&8475).expect("deduped fish row");
        assert_eq!(rows.len(), 1);
        assert_eq!(row.grade.as_deref(), Some("Prize"));
        assert_eq!(row.is_prize, Some(true));
        assert_eq!(row.encyclopedia_id, Some(9475));
        assert_eq!(
            row.catch_methods,
            vec!["rod".to_string(), "harpoon".to_string()]
        );
        assert_eq!(row.vendor_price, Some(120_000_000));
    }

    #[test]
    fn fish_is_dried_detects_actual_dried_items() {
        assert!(fish_is_dried(Some("Dried Tuna"), Some("말린 참치")));
        assert!(fish_is_dried(None, Some("말린 랍스터")));
        assert!(!fish_is_dried(Some("Yellow Corvina"), Some("참조기")));
    }

    #[test]
    fn fish_catch_methods_collect_rod_and_harpoon_tags() {
        assert_eq!(
            fish_catch_methods_from_description(Some(
                "물가에서 낚시를 하여 구할 수 있으며\n- 희귀 어종".to_string()
            )),
            vec!["rod".to_string()]
        );
        assert_eq!(
            fish_catch_methods_from_description(Some(
                "물가에서 낚시를 하여 구할 수 있으며\n- 작살 어종".to_string()
            )),
            vec!["harpoon".to_string()]
        );
        assert_eq!(
            fish_catch_methods_from_description(Some(
                "물가에서 낚시를 하여 구할 수 있으며\n- 보물 어종\n- 작살 어종".to_string()
            )),
            vec!["rod".to_string(), "harpoon".to_string()]
        );
        assert_eq!(
            fish_catch_methods_from_description(None),
            vec!["rod".to_string()]
        );
    }

    #[test]
    fn parse_positive_i64_ignores_zero_and_invalid_values() {
        assert_eq!(
            parse_positive_i64(Some("1200000".to_string())),
            Some(1_200_000)
        );
        assert_eq!(parse_positive_i64(Some("0".to_string())), None);
        assert_eq!(parse_positive_i64(Some("abc".to_string())), None);
        assert_eq!(parse_positive_i64(None), None);
    }

    #[test]
    fn fish_catalog_discards_non_web_icon_files() {
        assert!(is_web_icon_path("00008475.png"));
        assert!(!is_web_icon_path(
            "New_Icon/03_ETC/07_ProductMaterial/00008518.dds"
        ));
        assert_eq!(
            encyclopedia_icon_id_from_db(Some("00008475.png".to_string())),
            Some(8475)
        );
        assert_eq!(
            encyclopedia_icon_id_from_db(Some(
                "New_Icon/03_ETC/07_ProductMaterial/00008518.dds".to_string()
            )),
            None
        );
    }

    #[test]
    fn event_fish_identity_map_resolves_item_ids_for_encyclopedia_keys() {
        let fish_table = FishIdentityIndex {
            by_encyclopedia: HashMap::from([(
                821015,
                FishIdentityEntry {
                    encyclopedia_key: 821015,
                    item_id: 820115,
                    encyclopedia_id: Some(9015),
                    name: Some("Blue Bat Star".to_string()),
                },
            )]),
        };
        let event_identities = DoltMySqlStore::build_event_fish_identity_map(&fish_table);

        assert_eq!(
            event_identities.get(&821015).copied(),
            Some((820115, Some(821015), Some(9015)))
        );
    }

    #[test]
    fn event_fish_names_fall_back_to_fish_table_name_for_encyclopedia_keys() {
        let fish_table = FishIdentityIndex {
            by_encyclopedia: HashMap::from([(
                821015,
                FishIdentityEntry {
                    encyclopedia_key: 821015,
                    item_id: 820115,
                    encyclopedia_id: Some(9015),
                    name: Some("Blue Bat Star".to_string()),
                },
            )]),
        };
        let item_names = HashMap::new();

        let event_fish_names = DoltMySqlStore::build_event_fish_names(&item_names, &fish_table);

        assert_eq!(
            event_fish_names.get(&821015).map(String::as_str),
            Some("Blue Bat Star")
        );
        assert_eq!(
            event_fish_names.get(&820115).map(String::as_str),
            Some("Blue Bat Star")
        );
    }

    #[test]
    fn vector_layer_rejects_unknown_geometry_space() {
        let err = parse_vector_source(
            "region_groups",
            LayerKind::VectorGeoJson,
            vector_source_fields(
                Some("/region_groups/v1.geojson"),
                Some("rg-v1"),
                Some("screen_pixels"),
                Some("feature_property_palette"),
                None,
                Some("c"),
            ),
            Some("v1"),
        )
        .expect_err("expected geometry_space validation error");
        assert_eq!(err.0.code, ApiErrorCode::InvalidArgument);
        assert!(err.0.message.contains("vector_geometry_space"));
    }

    #[test]
    fn vector_layer_accepts_world_geometry_space() {
        let source = parse_vector_source(
            "water",
            LayerKind::VectorGeoJson,
            vector_source_fields(
                Some("/water/v1.geojson"),
                Some("wm-v1"),
                Some("world"),
                Some("feature_property_palette"),
                None,
                Some("c"),
            ),
            Some("v1"),
        )
        .expect("valid source")
        .expect("source");
        assert_eq!(source.geometry_space, GeometrySpace::World);
        assert_eq!(source.style_mode, StyleMode::FeaturePropertyPalette);
    }

    #[test]
    fn vector_layer_source_url_uses_normalized_path() {
        let source = parse_vector_source(
            "region_groups",
            LayerKind::VectorGeoJson,
            vector_source_fields(
                Some("/region_groups/{map_version}.geojson"),
                Some("rg-v1"),
                Some("map_pixels"),
                Some("feature_property_palette"),
                Some("id"),
                Some("c"),
            ),
            Some("v1"),
        )
        .expect("valid source")
        .expect("source");

        assert_eq!(source.url, "/region_groups/v1.geojson");
    }

    #[test]
    fn layer_asset_url_resolution_handles_root_and_relative_paths() {
        assert_eq!(
            resolve_layer_asset_url("/images/tiles/minimap/v1/tileset.json"),
            "/images/tiles/minimap/v1/tileset.json"
        );
        assert_eq!(
            resolve_layer_asset_url("images/tiles/minimap/v1/tileset.json"),
            "images/tiles/minimap/v1/tileset.json"
        );
        assert_eq!(
            resolve_layer_asset_url("https://static.example.com/a.png"),
            "https://static.example.com/a.png"
        );
    }

    #[test]
    fn layer_kind_rejects_unknown_value() {
        let err = parse_layer_kind("water", "vector_tiles").expect_err("expected invalid kind");
        assert_eq!(err.0.code, ApiErrorCode::InvalidArgument);
        assert!(err.0.message.contains("layer_kind"));
    }

    #[test]
    fn pixel_to_tile_index_returns_none_out_of_bounds() {
        let idx = pixel_to_tile_index(10, 10, 32, 64, 64).expect("index");
        assert_eq!(idx, 22);
        assert!(pixel_to_tile_index(10, 10, 32, -33, 64).is_none());
        assert!(pixel_to_tile_index(10, 10, 32, 100_000, 64).is_none());
    }

    #[test]
    fn compute_status_not_unknown_when_weighted_evidence_exists() {
        let cfg = ZoneStatusConfig::default();
        let (status, _, _) = compute_status(5.0, Some(1_700_000_000), 1_700_086_400, 20.0, &cfg);
        assert_ne!(status, ZoneStatus::Unknown);
    }

    #[test]
    fn zone_stats_status_is_not_unknown_when_evidence_exists() {
        let cfg = ZoneStatusConfig::default();
        let (status, age_days, notes) =
            compute_status(1.0, Some(1_700_000_000), 1_700_043_200, 12.0, &cfg);
        assert_ne!(status, ZoneStatus::Unknown);
        assert!(age_days.is_some());
        assert!(notes.is_empty());
    }

    #[test]
    fn events_snapshot_revision_changes_when_input_changes() {
        let rev_a = synthetic_events_snapshot_revision(Some("dolt:abc"), 100, Some(10), Some(20));
        let rev_b = synthetic_events_snapshot_revision(Some("dolt:abc"), 101, Some(10), Some(20));
        let rev_c = synthetic_events_snapshot_revision(Some("dolt:def"), 100, Some(10), Some(20));
        assert_ne!(rev_a, rev_b);
        assert_ne!(rev_a, rev_c);
    }

    #[test]
    fn event_source_kind_maps_ranking_code() {
        assert_eq!(
            event_source_kind_from_db(1),
            Some(fishystuff_api::models::events::EventSourceKind::Ranking)
        );
        assert_eq!(event_source_kind_from_db(99), None);
    }

    #[test]
    fn zone_distribution_fish_ids_excludes_prior_only_fish() {
        let summary = WindowSummary {
            alpha_total: 10.0,
            alpha_by_fish: HashMap::from([(1, 5.0), (2, 5.0)]),
            p_mean_by_fish: HashMap::from([(1, 0.5), (2, 0.5)]),
            c_zone: HashMap::from([(1, 4.0)]),
            ess: 4.0,
            total_weight: 4.0,
            last_seen: Some(100),
        };

        assert_eq!(zone_distribution_fish_ids(&summary), vec![1]);
    }
}
