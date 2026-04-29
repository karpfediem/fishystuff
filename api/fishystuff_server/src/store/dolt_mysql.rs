mod calculator;
mod calculator_defaults;
mod calculator_effects;
mod calculator_items;
mod calculator_loot;
mod calculator_pets;
mod calculator_progression;
mod calculator_sources;
mod catalog;
mod fish_best_spots;
mod item_metadata;
mod stats;
mod util;
mod zone_profile_v2;

#[cfg(test)]
mod layers;

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use fishystuff_api::error::ApiError;
use fishystuff_api::ids::{MapVersionId, Rgb};
use fishystuff_api::models::calculator::CalculatorCatalogResponse;
use fishystuff_api::models::effort::{EffortGridRequest, EffortGridResponse};
use fishystuff_api::models::events::{
    EventPointCompact, EventSourceKind, EventsSnapshotMetaResponse, EventsSnapshotResponse,
};
use fishystuff_api::models::fish::{
    CommunityFishZoneSupportResponse, FishBestSpotEntry, FishBestSpotsResponse, FishEntry,
    FishListResponse,
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
use mysql::{params, Opts, Pool, PoolConstraints, PoolOpts, Row};
use tracing::instrument;

use crate::config::ZoneStatusConfig;
use crate::error::{AppError, AppResult};
use crate::store::queries;
use crate::store::{validate_dolt_ref, CalculatorZoneLootEntry, DataLang, Store};
use calculator_sources::CalculatorCatalogSourceData;
use catalog::{
    encyclopedia_icon_id_from_db, fish_catch_methods_from_description, fish_is_dried,
    item_grade_from_db, merge_fish_catalog_row, parse_positive_i64,
};
#[cfg(test)]
use layers::{parse_layer_kind, parse_vector_source, resolve_layer_asset_url, VectorSourceFields};
use stats::{
    align_alpha, align_probs, beta_ci, compute_status, gaussian_blur_grid, pixel_to_tile_index,
    sample_dirichlet, seed_from_drift, seed_from_params, time_weight, XorShift64,
};
use util::{
    clamp_i64_to_u32, db_unavailable, epoch_to_mysql_datetime, event_source_kind_from_db,
    events_schema_or_db_unavailable, is_missing_table, normalize_optional_string, row_i64,
    row_opt_f64, row_string, row_u32_opt, synthetic_events_snapshot_revision,
    synthetic_fish_revision, synthetic_region_groups_revision,
};

const EPS: f64 = 1e-9;
const EPS_FISH: f64 = 1e-9;
const SOURCE_KIND_RANKING: i32 = 1;
const ZONE_MASK_LAYER_ID: &str = "zone_mask";
const DOLT_POOL_MIN_CONNECTIONS: usize = 0;
const DOLT_POOL_MAX_CONNECTIONS: usize = 16;
const DOLT_TCP_CONNECT_TIMEOUT_SECS: u64 = 3;
const DOLT_SOCKET_TIMEOUT_FLOOR_SECS: u64 = 60;
const DOLT_SOCKET_TIMEOUT_EXTRA_SECS: u64 = 30;
const DOLT_TCP_KEEPALIVE_TIME_MS: u32 = 5_000;
#[cfg(any(target_os = "linux", target_os = "macos"))]
const DOLT_TCP_KEEPALIVE_PROBE_INTERVAL_SECS: u32 = 5;
#[cfg(any(target_os = "linux", target_os = "macos"))]
const DOLT_TCP_KEEPALIVE_PROBE_COUNT: u32 = 3;
#[cfg(target_os = "linux")]
const DOLT_TCP_USER_TIMEOUT_MS: u32 = 10_000;
const META_CACHE_TTL_SECS: u64 = 60;
const DATA_LANG_AVAILABLE_CACHE_TTL_SECS: u64 = 60;

#[derive(Clone)]
pub struct DoltMySqlStore {
    pool: Pool,
    defaults: MetaDefaults,
    meta_cache: Arc<Mutex<Option<(Instant, MetaResponse)>>>,
    data_lang_available_cache: Arc<Mutex<HashMap<String, (Instant, bool)>>>,
    dolt_revision_cache: Arc<Mutex<HashMap<String, String>>>,
    layer_revision_id_cache: Arc<Mutex<HashMap<String, String>>>,
    event_zone_assignment_exists_cache: Arc<Mutex<HashMap<String, bool>>>,
    event_zone_ring_support_exists_cache: Arc<Mutex<HashMap<String, bool>>>,
    event_zone_support_mode_cache: Arc<Mutex<HashMap<String, Option<EventZoneSupportMode>>>>,
    calculator_catalog_cache: Arc<Mutex<HashMap<String, CalculatorCatalogResponse>>>,
    calculator_catalog_inflight: Arc<(Mutex<HashSet<String>>, Condvar)>,
    calculator_source_data_cache: Arc<Mutex<HashMap<String, CalculatorCatalogSourceData>>>,
    calculator_source_data_inflight: Arc<(Mutex<HashSet<String>>, Condvar)>,
    calculator_zone_loot_cache: Arc<Mutex<HashMap<String, Vec<CalculatorZoneLootEntry>>>>,
    calculator_zone_loot_load_state: Arc<(Mutex<CalculatorZoneLootLoadState>, Condvar)>,
    fish_list_cache: Arc<Mutex<HashMap<String, FishListResponse>>>,
    fish_list_inflight: Arc<(Mutex<HashSet<String>>, Condvar)>,
    fish_best_spots_cache: Arc<Mutex<HashMap<String, FishBestSpotsResponse>>>,
    fish_best_spots_inflight: Arc<(Mutex<HashSet<String>>, Condvar)>,
    fish_best_spots_index_cache: Arc<Mutex<HashMap<String, HashMap<i32, Vec<FishBestSpotEntry>>>>>,
    fish_best_spots_index_inflight: Arc<(Mutex<HashSet<String>>, Condvar)>,
    community_fish_zone_support_cache:
        Arc<Mutex<HashMap<String, CommunityFishZoneSupportResponse>>>,
    community_fish_zone_support_inflight: Arc<(Mutex<HashSet<String>>, Condvar)>,
    zones_cache: Arc<Mutex<HashMap<String, Vec<ZoneEntry>>>>,
    zones_inflight: Arc<(Mutex<HashSet<String>>, Condvar)>,
    fish_names_cache: Arc<Mutex<HashMap<String, HashMap<i32, String>>>>,
    fish_names_inflight: Arc<(Mutex<HashSet<String>>, Condvar)>,
    fish_identity_cache: Arc<Mutex<HashMap<String, FishIdentityIndex>>>,
    fish_identity_inflight: Arc<(Mutex<HashSet<String>>, Condvar)>,
    zone_stats_global_weights_cache: Arc<Mutex<HashMap<String, HashMap<i32, f64>>>>,
    zone_stats_global_weights_inflight: Arc<(Mutex<HashSet<String>>, Condvar)>,
    zone_stats_zone_weights_cache: Arc<Mutex<HashMap<String, ZoneWeightSummary>>>,
    zone_stats_zone_weights_inflight: Arc<(Mutex<HashSet<String>>, Condvar)>,
}

#[derive(Debug, Default)]
struct CalculatorZoneLootLoadState {
    inflight: HashSet<String>,
    retry_backoff: HashMap<String, CalculatorZoneLootRetryBackoff>,
}

#[derive(Debug, Clone)]
struct CalculatorZoneLootRetryBackoff {
    failures: u32,
    retry_at: Instant,
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
    sample_px_x: i32,
    sample_px_y: i32,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct EventZoneSupportRow {
    event_id: i64,
    ts_utc: i64,
    fish_id: i32,
    zone_rgbs: Vec<u32>,
}

#[derive(Debug, Clone, Default)]
struct ZoneWeightSummary {
    weights_by_fish: HashMap<i32, f64>,
    weight_sum: f64,
    weight2_sum: f64,
    last_seen: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventZoneSupportMode {
    Assignment,
    RingSupport,
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

type EventPointCompactBaseDbRow = (
    i64,
    i64,
    i64,
    i64,
    i64,
    i64,
    Option<i64>,
    Option<i64>,
    i64,
    Option<String>,
);

type EventZoneMembershipDbRow = (i64, i64);

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

#[cfg(test)]
fn group_event_zone_support_rows(
    rows: &[(i64, i64, i64, i64)],
) -> AppResult<Vec<EventZoneSupportRow>> {
    let mut out: Vec<EventZoneSupportRow> = Vec::new();
    for &(event_id, ts_utc, fish_id, zone_rgb_u32) in rows {
        let fish_id =
            i32::try_from(fish_id).map_err(|_| AppError::internal("fish_id out of range"))?;
        let zone_rgb_u32 = u32::try_from(zone_rgb_u32)
            .map_err(|_| AppError::internal("zone_rgb_u32 out of range"))?;

        match out.last_mut() {
            Some(current) if current.event_id == event_id => {
                if !current.zone_rgbs.contains(&zone_rgb_u32) {
                    current.zone_rgbs.push(zone_rgb_u32);
                }
            }
            _ => out.push(EventZoneSupportRow {
                event_id,
                ts_utc,
                fish_id,
                zone_rgbs: vec![zone_rgb_u32],
            }),
        }
    }
    Ok(out)
}

fn group_event_zone_membership_rows(
    rows: &[EventZoneMembershipDbRow],
) -> AppResult<HashMap<i64, Vec<u32>>> {
    let mut out: HashMap<i64, Vec<u32>> = HashMap::new();
    for &(event_id, zone_rgb_u32) in rows {
        let zone_rgb_u32 = u32::try_from(zone_rgb_u32)
            .map_err(|_| AppError::internal("zone_rgb_u32 out of range"))?;
        let zones = out.entry(event_id).or_default();
        if zones.last().copied() != Some(zone_rgb_u32) {
            zones.push(zone_rgb_u32);
        }
    }
    Ok(out)
}

fn zone_stats_weight_expr(params: &QueryParams) -> &'static str {
    if params.half_life_days.is_some() {
        "POW(2.0, -TIMESTAMPDIFF(SECOND, e.ts_utc, :half_life_to_dt) / (86400.0 * :half_life_days))"
    } else {
        "1.0"
    }
}

fn zone_stats_weight2_expr(params: &QueryParams) -> &'static str {
    if params.half_life_days.is_some() {
        "POW(2.0, -2.0 * TIMESTAMPDIFF(SECOND, e.ts_utc, :half_life_to_dt) / (86400.0 * :half_life_days))"
    } else {
        "1.0"
    }
}

fn zone_stats_weights_cache_key(
    params: &QueryParams,
    support_mode: EventZoneSupportMode,
    zone_rgb_u32: Option<u32>,
) -> String {
    let half_life = params
        .half_life_days
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string());
    format!(
        "layer={}|from={}|to={}|half={}|support={support_mode:?}|zone={}",
        params.map_version,
        params.from_ts_utc,
        params.to_ts_utc,
        half_life,
        zone_rgb_u32
            .map(|value| value.to_string())
            .unwrap_or_else(|| "global".to_string())
    )
}

fn fish_weight_rows_to_map(rows: Vec<(i64, f64)>) -> AppResult<HashMap<i32, f64>> {
    let mut out = HashMap::with_capacity(rows.len());
    for (fish_id, weight) in rows {
        if !weight.is_finite() || weight <= 0.0 {
            continue;
        }
        let fish_id =
            i32::try_from(fish_id).map_err(|_| AppError::internal("fish_id out of range"))?;
        out.insert(fish_id, weight);
    }
    Ok(out)
}

fn zone_weight_rows_to_summary(
    rows: Vec<(i64, f64, f64, Option<i64>)>,
) -> AppResult<ZoneWeightSummary> {
    let mut summary = ZoneWeightSummary::default();
    for (fish_id, weight, weight2, last_seen) in rows {
        if !weight.is_finite() || weight <= 0.0 {
            continue;
        }
        let fish_id =
            i32::try_from(fish_id).map_err(|_| AppError::internal("fish_id out of range"))?;
        summary.weights_by_fish.insert(fish_id, weight);
        summary.weight_sum += weight;
        if weight2.is_finite() && weight2 > 0.0 {
            summary.weight2_sum += weight2;
        }
        if let Some(last_seen) = last_seen {
            summary.last_seen = Some(
                summary
                    .last_seen
                    .map_or(last_seen, |prev| prev.max(last_seen)),
            );
        }
    }
    Ok(summary)
}

fn event_zone_assignment_map(rows: &[EventZoneMembershipDbRow]) -> AppResult<HashMap<i64, u32>> {
    let mut out = HashMap::new();
    for &(event_id, zone_rgb_u32) in rows {
        let zone_rgb_u32 = u32::try_from(zone_rgb_u32)
            .map_err(|_| AppError::internal("zone_rgb_u32 out of range"))?;
        out.entry(event_id).or_insert(zone_rgb_u32);
    }
    Ok(out)
}

fn revision_database_name(database_name: &str, ref_id: &str) -> String {
    let base_database_name = database_name
        .split_once('/')
        .map(|(base, _)| base)
        .unwrap_or(database_name);
    format!("{base_database_name}/{ref_id}")
}

fn dolt_socket_timeout_secs(request_timeout_secs: u64) -> u64 {
    request_timeout_secs
        .saturating_add(DOLT_SOCKET_TIMEOUT_EXTRA_SECS)
        .max(DOLT_SOCKET_TIMEOUT_FLOOR_SECS)
}

impl DoltMySqlStore {
    pub fn new(
        database_url: String,
        defaults: MetaDefaults,
        request_timeout_secs: u64,
    ) -> AppResult<Self> {
        let opts = Opts::from_url(&database_url).map_err(db_unavailable)?;
        let mut builder = OptsBuilder::from_opts(opts.clone());
        if let Some(default_ref_id) = defaults.dolt_ref_id.as_deref() {
            validate_dolt_ref(default_ref_id)?;
            let database_name = opts.get_db_name().ok_or_else(|| {
                AppError::internal(
                    "database_url must include a database name when defaults.dolt_ref_id is set",
                )
            })?;
            if database_name.is_empty() {
                return Err(AppError::internal(
                    "database_url must include a non-empty database name when defaults.dolt_ref_id is set",
                ));
            }
            builder = builder.db_name(Some(revision_database_name(database_name, default_ref_id)));
        }
        let constraints =
            PoolConstraints::new(DOLT_POOL_MIN_CONNECTIONS, DOLT_POOL_MAX_CONNECTIONS)
                .ok_or_else(|| AppError::internal("invalid Dolt pool constraints"))?;
        let pool_opts = PoolOpts::default().with_constraints(constraints);
        let socket_timeout = Duration::from_secs(dolt_socket_timeout_secs(request_timeout_secs));
        builder = builder
            .pool_opts(pool_opts)
            .tcp_connect_timeout(Some(Duration::from_secs(DOLT_TCP_CONNECT_TIMEOUT_SECS)))
            .read_timeout(Some(socket_timeout))
            .write_timeout(Some(socket_timeout))
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
        let store = Self {
            pool,
            defaults,
            meta_cache: Arc::new(Mutex::new(None)),
            data_lang_available_cache: Arc::new(Mutex::new(HashMap::new())),
            dolt_revision_cache: Arc::new(Mutex::new(HashMap::new())),
            layer_revision_id_cache: Arc::new(Mutex::new(HashMap::new())),
            event_zone_assignment_exists_cache: Arc::new(Mutex::new(HashMap::new())),
            event_zone_ring_support_exists_cache: Arc::new(Mutex::new(HashMap::new())),
            event_zone_support_mode_cache: Arc::new(Mutex::new(HashMap::new())),
            calculator_catalog_cache: Arc::new(Mutex::new(HashMap::new())),
            calculator_catalog_inflight: Arc::new((Mutex::new(HashSet::new()), Condvar::new())),
            calculator_source_data_cache: Arc::new(Mutex::new(HashMap::new())),
            calculator_source_data_inflight: Arc::new((Mutex::new(HashSet::new()), Condvar::new())),
            calculator_zone_loot_cache: Arc::new(Mutex::new(HashMap::new())),
            calculator_zone_loot_load_state: Arc::new((
                Mutex::new(CalculatorZoneLootLoadState::default()),
                Condvar::new(),
            )),
            fish_list_cache: Arc::new(Mutex::new(HashMap::new())),
            fish_list_inflight: Arc::new((Mutex::new(HashSet::new()), Condvar::new())),
            fish_best_spots_cache: Arc::new(Mutex::new(HashMap::new())),
            fish_best_spots_inflight: Arc::new((Mutex::new(HashSet::new()), Condvar::new())),
            fish_best_spots_index_cache: Arc::new(Mutex::new(HashMap::new())),
            fish_best_spots_index_inflight: Arc::new((Mutex::new(HashSet::new()), Condvar::new())),
            community_fish_zone_support_cache: Arc::new(Mutex::new(HashMap::new())),
            community_fish_zone_support_inflight: Arc::new((
                Mutex::new(HashSet::new()),
                Condvar::new(),
            )),
            zones_cache: Arc::new(Mutex::new(HashMap::new())),
            zones_inflight: Arc::new((Mutex::new(HashSet::new()), Condvar::new())),
            fish_names_cache: Arc::new(Mutex::new(HashMap::new())),
            fish_names_inflight: Arc::new((Mutex::new(HashSet::new()), Condvar::new())),
            fish_identity_cache: Arc::new(Mutex::new(HashMap::new())),
            fish_identity_inflight: Arc::new((Mutex::new(HashSet::new()), Condvar::new())),
            zone_stats_global_weights_cache: Arc::new(Mutex::new(HashMap::new())),
            zone_stats_global_weights_inflight: Arc::new((
                Mutex::new(HashSet::new()),
                Condvar::new(),
            )),
            zone_stats_zone_weights_cache: Arc::new(Mutex::new(HashMap::new())),
            zone_stats_zone_weights_inflight: Arc::new((
                Mutex::new(HashSet::new()),
                Condvar::new(),
            )),
        };
        Ok(store)
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
                    "map_versions table is missing; use a Dolt commit or branch that contains the current map schema",
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

    fn query_region_groups(&self, map_version_id: &str) -> AppResult<Vec<RegionGroupDescriptor>> {
        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;

        let meta_rows: Vec<Row> = match conn.exec(queries::REGION_GROUP_META_SQL, (map_version_id,))
        {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "region_group_meta") => {
                return Err(AppError::not_found(
                    "region_group_meta table is missing; use a Dolt commit or branch that contains the current region-group schema",
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
                    "region_group_regions table is missing; use a Dolt commit or branch that contains the current region-group schema",
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
        let requested_map_version = map_version_id
            .map(|value| value.0.trim())
            .filter(|value| !value.is_empty());
        let requested_layer_id = layer_id.map(str::trim).filter(|value| !value.is_empty());
        let cache_key = format!(
            "map={}|layer={}|patch={}|at={}|window={}",
            requested_map_version.unwrap_or(""),
            requested_layer_id.unwrap_or(""),
            patch_id
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(""),
            at_ts_utc.map(|value| value.to_string()).unwrap_or_default(),
            window_to_ts_utc,
        );
        if let Ok(cache) = self.layer_revision_id_cache.lock() {
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        let resolved = (|| -> AppResult<String> {
            if let (Some(map_version_id), Some(layer_id)) =
                (requested_map_version, requested_layer_id)
            {
                let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
                let row: Option<String> = match conn.exec_first(
                    "SELECT layer_revision_id \
                     FROM layer_revisions \
                     WHERE layer_id = ? AND map_version_id = ? \
                     ORDER BY created_at DESC \
                     LIMIT 1",
                    (layer_id, map_version_id),
                ) {
                    Ok(value) => value,
                    Err(err) if is_missing_table(&err, "layer_revisions") => {
                        return Err(AppError::unavailable(
                            "layer_revisions table missing; use a Dolt commit or branch that contains the current evidence schema",
                        ));
                    }
                    Err(err) => return Err(db_unavailable(err)),
                };
                return row.ok_or_else(|| {
                    AppError::not_found(format!(
                        "no layer revision for layer_id={} map_version_id={}",
                        layer_id, map_version_id
                    ))
                });
            }
            if let Some(map_version_id) = requested_map_version {
                return Ok(map_version_id.to_string());
            }

            let layer_id = requested_layer_id.ok_or_else(|| {
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
                            "layer_revisions table missing; use a Dolt commit or branch that contains the current evidence schema",
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
                        "layer_revisions table missing; use a Dolt commit or branch that contains the current evidence schema",
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
        })()?;

        if let Ok(mut cache) = self.layer_revision_id_cache.lock() {
            cache.insert(cache_key, resolved.clone());
        }

        Ok(resolved)
    }

    fn query_dolt_head_revision(&self) -> Option<String> {
        self.query_dolt_revision(None)
    }

    fn query_dolt_revision_uncached(&self, ref_id: Option<&str>) -> AppResult<String> {
        let ref_id = ref_id.map(str::trim).filter(|value| !value.is_empty());
        if let Some(value) = ref_id {
            validate_dolt_ref(value)?;
        }
        let ref_id = ref_id.unwrap_or("HEAD").replace('\'', "''");
        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let query = format!("SELECT HASHOF('{ref_id}')");
        let hash: Option<String> = conn.query_first(query).map_err(db_unavailable)?;
        let hash = hash
            .map(|hash| hash.trim().to_string())
            .filter(|hash| !hash.is_empty())
            .ok_or_else(|| AppError::not_found("Dolt ref did not resolve to a revision"))?;
        Ok(format!("dolt:{hash}"))
    }

    fn query_dolt_revision(&self, ref_id: Option<&str>) -> Option<String> {
        let cache_key = Self::revision_cache_key(ref_id);
        if let Ok(cache) = self.dolt_revision_cache.lock() {
            if let Some(cached) = cache.get(&cache_key) {
                return Some(cached.clone());
            }
        }

        let revision = self.query_dolt_revision_uncached(ref_id).ok()?;
        if let Ok(mut cache) = self.dolt_revision_cache.lock() {
            cache.insert(cache_key, revision.clone());
        }
        Some(revision)
    }

    fn data_lang_available_cache_key(lang: &DataLang, ref_id: Option<&str>) -> String {
        let lang = lang.code();
        match ref_id {
            Some(ref_id) => format!("{lang}:{ref_id}"),
            None => format!("{lang}:head"),
        }
    }

    fn unsupported_data_lang_error(lang: &DataLang) -> AppError {
        AppError::invalid_argument(format!("unsupported data language code: {}", lang.code()))
    }

    pub(super) fn validate_data_lang_available(
        &self,
        lang: &DataLang,
        ref_id: Option<&str>,
    ) -> AppResult<()> {
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };
        let cache_key = Self::data_lang_available_cache_key(lang, ref_id);
        let cache_ttl = Duration::from_secs(DATA_LANG_AVAILABLE_CACHE_TTL_SECS);
        if let Ok(cache) = self.data_lang_available_cache.lock() {
            if let Some((cached_at, available)) = cache.get(&cache_key) {
                if cached_at.elapsed() < cache_ttl {
                    return if *available {
                        Ok(())
                    } else {
                        Err(Self::unsupported_data_lang_error(lang))
                    };
                }
            }
        }

        let query = format!(
            "SELECT 1 FROM languagedata{as_of} \
             WHERE `lang` = '{}' \
             LIMIT 1",
            lang.code().replace('\'', "''")
        );

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        match conn.query_first::<u8, _>(query) {
            Ok(Some(_)) => {
                if let Ok(mut cache) = self.data_lang_available_cache.lock() {
                    cache.insert(cache_key, (Instant::now(), true));
                }
                Ok(())
            }
            Ok(None) => {
                if let Ok(mut cache) = self.data_lang_available_cache.lock() {
                    cache.insert(cache_key, (Instant::now(), false));
                }
                Err(Self::unsupported_data_lang_error(lang))
            }
            Err(err) if is_missing_table(&err, "languagedata") => {
                if let Ok(mut cache) = self.data_lang_available_cache.lock() {
                    cache.insert(cache_key, (Instant::now(), false));
                }
                Err(Self::unsupported_data_lang_error(lang))
            }
            Err(err) => Err(db_unavailable(err)),
        }
    }

    fn query_data_languages(&self) -> AppResult<Vec<String>> {
        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let langs: Vec<String> = conn
            .query("SELECT DISTINCT `lang` FROM languagedata")
            .map_err(db_unavailable)?;
        let mut languages = BTreeSet::new();
        for code in langs {
            if let Some(lang) = DataLang::from_code(&code) {
                languages.insert(lang.code().to_string());
            }
        }
        Ok(languages.into_iter().collect())
    }

    fn query_fish_names(
        &self,
        lang: &DataLang,
        ref_id: Option<&str>,
    ) -> AppResult<HashMap<i32, String>> {
        let cache_key = Self::fish_names_cache_key(lang, ref_id);
        loop {
            if let Ok(cache) = self.fish_names_cache.lock() {
                if let Some(cached) = cache.get(&cache_key) {
                    return Ok(cached.clone());
                }
            }

            let (inflight_lock, inflight_cvar) = &*self.fish_names_inflight;
            let mut inflight = inflight_lock
                .lock()
                .expect("fish names inflight lock poisoned");
            if !inflight.contains(&cache_key) {
                inflight.insert(cache_key.clone());
                drop(inflight);
                break;
            }
            inflight = inflight_cvar
                .wait(inflight)
                .expect("fish names inflight wait poisoned");
            drop(inflight);
        }

        let result = self.query_fish_names_uncached(lang, ref_id);

        let (inflight_lock, inflight_cvar) = &*self.fish_names_inflight;
        let mut inflight = inflight_lock
            .lock()
            .expect("fish names inflight lock poisoned");
        inflight.remove(&cache_key);
        inflight_cvar.notify_all();
        drop(inflight);

        let names = result?;

        if let Ok(mut cache) = self.fish_names_cache.lock() {
            cache.insert(cache_key, names.clone());
        }

        Ok(names)
    }

    fn query_fish_names_uncached(
        &self,
        lang: &DataLang,
        ref_id: Option<&str>,
    ) -> AppResult<HashMap<i32, String>> {
        self.validate_data_lang_available(lang, ref_id)?;
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };
        let query = format!(
            "SELECT \
                k.fish_id, \
                loc.`text` AS fish_name \
             FROM fish_names_ko{as_of} k \
             JOIN languagedata{as_of} loc \
               ON loc.`lang` = '{}' \
              AND loc.`id` = CAST(k.fish_id AS SIGNED) \
              AND loc.`format` = 'A' \
              AND loc.`category` = '' \
              AND NULLIF(TRIM(loc.`text`), '') IS NOT NULL",
            lang.code().replace('\'', "''")
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

    fn fish_names_cache_key(lang: &DataLang, ref_id: Option<&str>) -> String {
        match ref_id {
            Some(ref_id) => format!("{}:{ref_id}", lang.code()),
            None => format!("{}:head", lang.code()),
        }
    }

    fn query_fish_catalog(
        &self,
        lang: &DataLang,
        ref_id: Option<&str>,
    ) -> AppResult<Vec<FishCatalogRow>> {
        self.validate_data_lang_available(lang, ref_id)?;
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };
        // fish_names_ko can lag newer releases, so union the fish_table-only rows.
        let query = format!(
            "SELECT \
                f.fish_id, \
                ft.encyclopedia_key, \
                loc.`text` AS fish_name, \
                it.`GradeType` AS grade_type, \
                NULLIF(ft.icon, '') AS fish_table_icon_file, \
                NULLIF(it.`IconImageFile`, '') AS item_icon_file, \
                NULLIF(ft.encyclopedia_icon, '') AS encyclopedia_icon_file, \
                it.`ItemName` AS item_name, \
                it.`Description` AS item_description, \
                it.`OriginalPrice` AS original_price \
             FROM fish_names_ko{as_of} f \
             JOIN languagedata{as_of} loc \
               ON loc.`lang` = '{item_lang}' \
              AND loc.`id` = CAST(f.fish_id AS SIGNED) \
              AND loc.`format` = 'A' \
              AND loc.`category` = '' \
              AND NULLIF(TRIM(loc.`text`), '') IS NOT NULL \
             JOIN item_table{as_of} it ON it.`Index` = f.fish_id \
             LEFT JOIN fish_table{as_of} ft ON ft.item_key = f.fish_id \
             UNION ALL \
             SELECT \
                ft.item_key AS fish_id, \
                ft.encyclopedia_key, \
                loc.`text` AS fish_name, \
                it.`GradeType` AS grade_type, \
                NULLIF(ft.icon, '') AS fish_table_icon_file, \
                NULLIF(it.`IconImageFile`, '') AS item_icon_file, \
                NULLIF(ft.encyclopedia_icon, '') AS encyclopedia_icon_file, \
                it.`ItemName` AS item_name, \
                it.`Description` AS item_description, \
                it.`OriginalPrice` AS original_price \
             FROM fish_table{as_of} ft \
             JOIN languagedata{as_of} loc \
               ON loc.`lang` = '{item_lang}' \
              AND loc.`id` = CAST(ft.item_key AS SIGNED) \
              AND loc.`format` = 'A' \
              AND loc.`category` = '' \
              AND NULLIF(TRIM(loc.`text`), '') IS NOT NULL \
             LEFT JOIN item_table{as_of} it ON it.`Index` = ft.item_key \
             LEFT JOIN fish_names_ko{as_of} f ON f.fish_id = ft.item_key \
             WHERE f.fish_id IS NULL",
            item_lang = lang.code().replace('\'', "''")
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
            let (grade, grade_rank, is_prize) = item_grade_from_db(grade_type);
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

    fn fish_list_cache_key(lang: &DataLang, ref_id: Option<&str>) -> String {
        let lang = lang.code();
        match ref_id {
            Some(ref_id) => format!("{lang}:{ref_id}"),
            None => format!("{lang}:head"),
        }
    }

    fn query_fish_list_cached(
        &self,
        lang: DataLang,
        ref_id: Option<&str>,
    ) -> AppResult<FishListResponse> {
        let cache_key = Self::fish_list_cache_key(&lang, ref_id);
        loop {
            if let Ok(cache) = self.fish_list_cache.lock() {
                if let Some(cached) = cache.get(&cache_key) {
                    return Ok(cached.clone());
                }
            }

            let (inflight_lock, inflight_cvar) = &*self.fish_list_inflight;
            let mut inflight = inflight_lock
                .lock()
                .expect("fish list inflight lock poisoned");
            if !inflight.contains(&cache_key) {
                inflight.insert(cache_key.clone());
                drop(inflight);
                break;
            }
            inflight = inflight_cvar
                .wait(inflight)
                .expect("fish list inflight wait poisoned");
            drop(inflight);
        }

        let result: AppResult<FishListResponse> = (|| {
            let mut fish = self.query_fish_catalog(&lang, ref_id)?;
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
            let revision = self
                .query_dolt_revision(ref_id)
                .unwrap_or_else(|| synthetic_fish_revision(ref_id, &fish));
            let entries = fish
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
                .collect::<Vec<_>>();
            Ok(FishListResponse {
                revision,
                count: entries.len(),
                fish: entries,
            })
        })();

        let (inflight_lock, inflight_cvar) = &*self.fish_list_inflight;
        let mut inflight = inflight_lock
            .lock()
            .expect("fish list inflight lock poisoned");
        inflight.remove(&cache_key);
        inflight_cvar.notify_all();
        drop(inflight);

        let response = result?;

        if let Ok(mut cache) = self.fish_list_cache.lock() {
            cache.insert(cache_key, response.clone());
        }

        Ok(response)
    }

    fn revision_cache_key(ref_id: Option<&str>) -> String {
        match ref_id {
            Some(ref_id) => ref_id.to_string(),
            None => "head".to_string(),
        }
    }

    fn query_zones(&self, ref_id: Option<&str>) -> AppResult<Vec<ZoneEntry>> {
        let cache_key = Self::revision_cache_key(ref_id);
        loop {
            if let Ok(cache) = self.zones_cache.lock() {
                if let Some(cached) = cache.get(&cache_key) {
                    return Ok(cached.clone());
                }
            }

            let (inflight_lock, inflight_cvar) = &*self.zones_inflight;
            let mut inflight = inflight_lock.lock().expect("zones inflight lock poisoned");
            if !inflight.contains(&cache_key) {
                inflight.insert(cache_key.clone());
                drop(inflight);
                break;
            }
            inflight = inflight_cvar
                .wait(inflight)
                .expect("zones inflight wait poisoned");
            drop(inflight);
        }

        let result = self.query_zones_uncached(ref_id);

        let (inflight_lock, inflight_cvar) = &*self.zones_inflight;
        let mut inflight = inflight_lock.lock().expect("zones inflight lock poisoned");
        inflight.remove(&cache_key);
        inflight_cvar.notify_all();
        drop(inflight);

        let zones = result?;

        if let Ok(mut cache) = self.zones_cache.lock() {
            cache.insert(cache_key, zones.clone());
        }

        Ok(zones)
    }

    fn query_zones_uncached(&self, ref_id: Option<&str>) -> AppResult<Vec<ZoneEntry>> {
        let mut query = queries::ZONES_SQL.to_string();
        if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            query.push_str(&format!(" AS OF '{}'", ref_id));
        }

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<(
            i64,
            i64,
            i64,
            Option<String>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
        )> = conn.query(query).map_err(db_unavailable)?;

        let mut zones = Vec::with_capacity(rows.len());
        for (r, g, b, name, active, confirmed, index, bite_time_min, bite_time_max) in rows {
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
                active: active.map(|value| value != 0),
                confirmed: confirmed.map(|value| value != 0),
                index: index
                    .map(|value| {
                        u32::try_from(value)
                            .map_err(|_| AppError::internal("zones_merged index out of range"))
                    })
                    .transpose()?,
                bite_time_min: bite_time_min
                    .map(|value| {
                        u32::try_from(value).map_err(|_| {
                            AppError::internal("zones_merged bite_time_min out of range")
                        })
                    })
                    .transpose()?,
                bite_time_max: bite_time_max
                    .map(|value| {
                        u32::try_from(value).map_err(|_| {
                            AppError::internal("zones_merged bite_time_max out of range")
                        })
                    })
                    .transpose()?,
            });
        }
        zones.sort_by_key(|zone| zone.rgb_u32);
        Ok(zones)
    }

    fn fish_identity_cache_key(lang: &DataLang, ref_id: Option<&str>) -> String {
        match ref_id {
            Some(ref_id) => format!("{}:{ref_id}", lang.code()),
            None => format!("{}:head", lang.code()),
        }
    }

    fn query_fish_identities(
        &self,
        lang: &DataLang,
        ref_id: Option<&str>,
    ) -> AppResult<FishIdentityIndex> {
        let cache_key = Self::fish_identity_cache_key(lang, ref_id);
        loop {
            if let Ok(cache) = self.fish_identity_cache.lock() {
                if let Some(cached) = cache.get(&cache_key) {
                    return Ok(cached.clone());
                }
            }

            let (inflight_lock, inflight_cvar) = &*self.fish_identity_inflight;
            let mut inflight = inflight_lock
                .lock()
                .expect("fish identity inflight lock poisoned");
            if !inflight.contains(&cache_key) {
                inflight.insert(cache_key.clone());
                drop(inflight);
                break;
            }
            inflight = inflight_cvar
                .wait(inflight)
                .expect("fish identity inflight wait poisoned");
            drop(inflight);
        }

        let result = self.query_fish_identities_uncached(lang, ref_id);

        let (inflight_lock, inflight_cvar) = &*self.fish_identity_inflight;
        let mut inflight = inflight_lock
            .lock()
            .expect("fish identity inflight lock poisoned");
        inflight.remove(&cache_key);
        inflight_cvar.notify_all();
        drop(inflight);

        let identities = result?;

        if let Ok(mut cache) = self.fish_identity_cache.lock() {
            cache.insert(cache_key, identities.clone());
        }

        Ok(identities)
    }

    fn query_fish_identities_uncached(
        &self,
        lang: &DataLang,
        ref_id: Option<&str>,
    ) -> AppResult<FishIdentityIndex> {
        self.validate_data_lang_available(lang, ref_id)?;
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
                loc.`text` AS localized_name, \
                ft.icon, \
                ft.encyclopedia_icon \
             FROM fish_table{as_of} ft \
             JOIN languagedata{as_of} loc \
               ON loc.`lang` = '{}' \
              AND loc.`id` = ft.item_key \
              AND loc.`format` = 'A' \
              AND loc.`category` = '' \
              AND NULLIF(TRIM(loc.`text`), '') IS NOT NULL",
            lang.code().replace('\'', "''")
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
        if let Ok(cache) = self.event_zone_assignment_exists_cache.lock() {
            if let Some(cached) = cache.get(layer_revision_id) {
                return Ok(*cached);
            }
        }

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let exists: Option<u8> = match conn.exec_first(
            queries::EVENT_ZONE_ASSIGNMENT_EXISTS_SQL,
            (layer_revision_id,),
        ) {
            Ok(value) => value,
            Err(err) if is_missing_table(&err, "event_zone_assignment") => return Ok(false),
            Err(err) => return Err(db_unavailable(err)),
        };
        let exists = exists.is_some();
        if let Ok(mut cache) = self.event_zone_assignment_exists_cache.lock() {
            cache.insert(layer_revision_id.to_string(), exists);
        }
        Ok(exists)
    }

    fn has_event_zone_ring_support(&self, layer_revision_id: &str) -> AppResult<bool> {
        if let Ok(cache) = self.event_zone_ring_support_exists_cache.lock() {
            if let Some(cached) = cache.get(layer_revision_id) {
                return Ok(*cached);
            }
        }

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let exists: Option<u8> = match conn.exec_first(
            queries::EVENT_ZONE_RING_SUPPORT_EXISTS_SQL,
            (layer_revision_id,),
        ) {
            Ok(value) => value,
            Err(err) if is_missing_table(&err, "event_zone_ring_support") => return Ok(false),
            Err(err) => return Err(db_unavailable(err)),
        };
        let exists = exists.is_some();
        if let Ok(mut cache) = self.event_zone_ring_support_exists_cache.lock() {
            cache.insert(layer_revision_id.to_string(), exists);
        }
        Ok(exists)
    }

    fn resolve_event_zone_support_mode(
        &self,
        layer_revision_id: &str,
    ) -> AppResult<Option<EventZoneSupportMode>> {
        if let Ok(cache) = self.event_zone_support_mode_cache.lock() {
            if let Some(cached) = cache.get(layer_revision_id) {
                return Ok(*cached);
            }
        }

        let support_mode = if self.has_event_zone_ring_support(layer_revision_id)? {
            Some(EventZoneSupportMode::RingSupport)
        } else if self.has_event_zone_assignment(layer_revision_id)? {
            Some(EventZoneSupportMode::Assignment)
        } else {
            None
        };

        if let Ok(mut cache) = self.event_zone_support_mode_cache.lock() {
            cache.insert(layer_revision_id.to_string(), support_mode);
        }

        Ok(support_mode)
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
        for (ts_utc, _fish_id, sample_px_x, sample_px_y, _zone_rgb_u32) in rows {
            out.push(EventZoneRow {
                ts_utc,
                sample_px_x: i32::try_from(sample_px_x)
                    .map_err(|_| AppError::internal("sample_px_x out of range"))?,
                sample_px_y: i32::try_from(sample_px_y)
                    .map_err(|_| AppError::internal("sample_px_y out of range"))?,
            });
        }
        Ok(out)
    }

    fn load_events_snapshot_assignment_map(
        &self,
        layer_revision_id: &str,
    ) -> AppResult<HashMap<i64, u32>> {
        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<EventZoneMembershipDbRow> = conn
            .exec(
                queries::EVENTS_SNAPSHOT_ASSIGNMENT_SQL,
                (layer_revision_id, SOURCE_KIND_RANKING),
            )
            .map_err(events_schema_or_db_unavailable)?;
        event_zone_assignment_map(&rows)
    }

    fn load_events_snapshot_ring_support_map(
        &self,
        layer_revision_id: &str,
        fully_contained_only: bool,
    ) -> AppResult<HashMap<i64, Vec<u32>>> {
        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<EventZoneMembershipDbRow> = conn
            .exec(
                if fully_contained_only {
                    queries::EVENTS_SNAPSHOT_FULL_RING_SUPPORT_SQL
                } else {
                    queries::EVENTS_SNAPSHOT_RING_SUPPORT_SQL
                },
                (layer_revision_id, SOURCE_KIND_RANKING),
            )
            .map_err(events_schema_or_db_unavailable)?;
        group_event_zone_membership_rows(&rows)
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
        let layer_revision_id = self.resolve_layer_revision_id(
            None,
            self.defaults.map_version_id.as_ref(),
            Some(ZONE_MASK_LAYER_ID),
            None,
            None,
            0,
        )?;
        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<EventPointCompactBaseDbRow> = conn
            .exec(queries::EVENTS_SNAPSHOT_BASE_SQL, (SOURCE_KIND_RANKING,))
            .map_err(events_schema_or_db_unavailable)?;
        let has_assignment = self.has_event_zone_assignment(&layer_revision_id)?;
        let support_mode = self.resolve_event_zone_support_mode(&layer_revision_id)?;
        let assignment_by_event = if has_assignment {
            self.load_events_snapshot_assignment_map(&layer_revision_id)?
        } else {
            HashMap::new()
        };
        let zone_support_by_event = match support_mode {
            Some(EventZoneSupportMode::RingSupport) => {
                self.load_events_snapshot_ring_support_map(&layer_revision_id, false)?
            }
            Some(EventZoneSupportMode::Assignment) => assignment_by_event
                .iter()
                .map(|(&event_id, &zone_rgb)| (event_id, vec![zone_rgb]))
                .collect(),
            None => HashMap::new(),
        };
        let full_zone_support_by_event = match support_mode {
            Some(EventZoneSupportMode::RingSupport) => {
                self.load_events_snapshot_ring_support_map(&layer_revision_id, true)?
            }
            Some(EventZoneSupportMode::Assignment) => assignment_by_event
                .iter()
                .map(|(&event_id, &zone_rgb)| (event_id, vec![zone_rgb]))
                .collect(),
            None => HashMap::new(),
        };

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
            source_kind,
            source_id,
        ) in rows
        {
            let zone_rgb_u32 = assignment_by_event.get(&event_id).copied();
            let zone_rgbs = zone_support_by_event
                .get(&event_id)
                .cloned()
                .unwrap_or_else(|| zone_rgb_u32.into_iter().collect());
            let full_zone_rgbs = full_zone_support_by_event
                .get(&event_id)
                .cloned()
                .unwrap_or_default();
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
                zone_rgb_u32,
                zone_rgbs,
                full_zone_rgbs,
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

        let Some(support_mode) = self.resolve_event_zone_support_mode(&params.map_version)? else {
            return Err(AppError::not_found(format!(
                "event_zone_assignment/event_zone_ring_support missing for layer_revision_id={}",
                params.map_version
            )));
        };

        let summary = self.compute_window_summary(params, zone_rgb_u32, support_mode)?;
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
                self.compute_drift_info(params, zone_rgb_u32, boundary, status_cfg, support_mode)?;
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

    fn load_zone_stats_global_weights(
        &self,
        params: &QueryParams,
        support_mode: EventZoneSupportMode,
    ) -> AppResult<HashMap<i32, f64>> {
        let cache_key = zone_stats_weights_cache_key(params, support_mode, None);
        loop {
            if let Ok(cache) = self.zone_stats_global_weights_cache.lock() {
                if let Some(cached) = cache.get(&cache_key) {
                    return Ok(cached.clone());
                }
            }

            let (inflight_lock, inflight_cvar) = &*self.zone_stats_global_weights_inflight;
            let mut inflight = inflight_lock
                .lock()
                .expect("zone stats global weights inflight lock poisoned");
            if !inflight.contains(&cache_key) {
                inflight.insert(cache_key.clone());
                drop(inflight);
                break;
            }
            inflight = inflight_cvar
                .wait(inflight)
                .expect("zone stats global weights inflight wait poisoned");
            drop(inflight);
        }

        let result = self.load_zone_stats_global_weights_uncached(params, support_mode);

        let (inflight_lock, inflight_cvar) = &*self.zone_stats_global_weights_inflight;
        let mut inflight = inflight_lock
            .lock()
            .expect("zone stats global weights inflight lock poisoned");
        inflight.remove(&cache_key);
        inflight_cvar.notify_all();
        drop(inflight);

        let weights = result?;

        if let Ok(mut cache) = self.zone_stats_global_weights_cache.lock() {
            cache.insert(cache_key, weights.clone());
        }

        Ok(weights)
    }

    fn load_zone_stats_global_weights_uncached(
        &self,
        params: &QueryParams,
        support_mode: EventZoneSupportMode,
    ) -> AppResult<HashMap<i32, f64>> {
        let from_dt = epoch_to_mysql_datetime(params.from_ts_utc)?;
        let to_dt = epoch_to_mysql_datetime(params.to_ts_utc)?;
        let support_exists = match support_mode {
            EventZoneSupportMode::Assignment => {
                "EXISTS (\
                    SELECT 1 \
                    FROM event_zone_assignment z \
                    WHERE z.layer_revision_id = :layer_revision_id \
                      AND z.event_id = e.event_id\
                )"
            }
            EventZoneSupportMode::RingSupport => {
                "EXISTS (\
                    SELECT 1 \
                    FROM event_zone_ring_support ring \
                    WHERE ring.layer_revision_id = :layer_revision_id \
                      AND ring.event_id = e.event_id\
                )"
            }
        };
        let weight_expr = zone_stats_weight_expr(params);
        let query = format!(
            "SELECT \
                CAST(e.fish_id AS SIGNED), \
                CAST(SUM({weight_expr}) AS DOUBLE) \
             FROM events e \
             WHERE e.water_ok = 1 \
               AND e.source_kind = :source_kind \
               AND e.ts_utc >= :from_dt \
               AND e.ts_utc < :to_dt \
               AND {support_exists} \
             GROUP BY e.fish_id"
        );

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<(i64, f64)> = if let Some(half_life_days) = params.half_life_days {
            conn.exec(
                query,
                mysql::params! {
                    "layer_revision_id" => params.map_version.as_str(),
                    "source_kind" => SOURCE_KIND_RANKING,
                    "from_dt" => from_dt.as_str(),
                    "to_dt" => to_dt.as_str(),
                    "half_life_to_dt" => to_dt.as_str(),
                    "half_life_days" => half_life_days,
                },
            )
        } else {
            conn.exec(
                query,
                mysql::params! {
                    "layer_revision_id" => params.map_version.as_str(),
                    "source_kind" => SOURCE_KIND_RANKING,
                    "from_dt" => from_dt.as_str(),
                    "to_dt" => to_dt.as_str(),
                },
            )
        }
        .map_err(events_schema_or_db_unavailable)?;

        fish_weight_rows_to_map(rows)
    }

    fn load_zone_stats_zone_weights(
        &self,
        params: &QueryParams,
        zone_rgb_u32: u32,
        support_mode: EventZoneSupportMode,
    ) -> AppResult<ZoneWeightSummary> {
        let cache_key = zone_stats_weights_cache_key(params, support_mode, Some(zone_rgb_u32));
        loop {
            if let Ok(cache) = self.zone_stats_zone_weights_cache.lock() {
                if let Some(cached) = cache.get(&cache_key) {
                    return Ok(cached.clone());
                }
            }

            let (inflight_lock, inflight_cvar) = &*self.zone_stats_zone_weights_inflight;
            let mut inflight = inflight_lock
                .lock()
                .expect("zone stats zone weights inflight lock poisoned");
            if !inflight.contains(&cache_key) {
                inflight.insert(cache_key.clone());
                drop(inflight);
                break;
            }
            inflight = inflight_cvar
                .wait(inflight)
                .expect("zone stats zone weights inflight wait poisoned");
            drop(inflight);
        }

        let result = self.load_zone_stats_zone_weights_uncached(params, zone_rgb_u32, support_mode);

        let (inflight_lock, inflight_cvar) = &*self.zone_stats_zone_weights_inflight;
        let mut inflight = inflight_lock
            .lock()
            .expect("zone stats zone weights inflight lock poisoned");
        inflight.remove(&cache_key);
        inflight_cvar.notify_all();
        drop(inflight);

        let weights = result?;

        if let Ok(mut cache) = self.zone_stats_zone_weights_cache.lock() {
            cache.insert(cache_key, weights.clone());
        }

        Ok(weights)
    }

    fn load_zone_stats_zone_weights_uncached(
        &self,
        params: &QueryParams,
        zone_rgb_u32: u32,
        support_mode: EventZoneSupportMode,
    ) -> AppResult<ZoneWeightSummary> {
        let from_dt = epoch_to_mysql_datetime(params.from_ts_utc)?;
        let to_dt = epoch_to_mysql_datetime(params.to_ts_utc)?;
        let support_table = match support_mode {
            EventZoneSupportMode::Assignment => "event_zone_assignment",
            EventZoneSupportMode::RingSupport => "event_zone_ring_support",
        };
        let weight_expr = zone_stats_weight_expr(params);
        let weight2_expr = zone_stats_weight2_expr(params);
        let query = format!(
            "SELECT \
                CAST(e.fish_id AS SIGNED), \
                CAST(SUM({weight_expr}) AS DOUBLE), \
                CAST(SUM({weight2_expr}) AS DOUBLE), \
                MAX(CAST(TIMESTAMPDIFF(SECOND, '1970-01-01 00:00:00', e.ts_utc) AS SIGNED)) \
             FROM {support_table} z \
             JOIN events e ON e.event_id = z.event_id \
             WHERE z.layer_revision_id = :layer_revision_id \
               AND z.zone_rgb = :zone_rgb \
               AND e.water_ok = 1 \
               AND e.source_kind = :source_kind \
               AND e.ts_utc >= :from_dt \
               AND e.ts_utc < :to_dt \
             GROUP BY e.fish_id"
        );

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<(i64, f64, f64, Option<i64>)> =
            if let Some(half_life_days) = params.half_life_days {
                conn.exec(
                    query,
                    mysql::params! {
                        "layer_revision_id" => params.map_version.as_str(),
                        "zone_rgb" => zone_rgb_u32,
                        "source_kind" => SOURCE_KIND_RANKING,
                        "from_dt" => from_dt.as_str(),
                        "to_dt" => to_dt.as_str(),
                        "half_life_to_dt" => to_dt.as_str(),
                        "half_life_days" => half_life_days,
                    },
                )
            } else {
                conn.exec(
                    query,
                    mysql::params! {
                        "layer_revision_id" => params.map_version.as_str(),
                        "zone_rgb" => zone_rgb_u32,
                        "source_kind" => SOURCE_KIND_RANKING,
                        "from_dt" => from_dt.as_str(),
                        "to_dt" => to_dt.as_str(),
                    },
                )
            }
            .map_err(events_schema_or_db_unavailable)?;

        zone_weight_rows_to_summary(rows)
    }

    fn window_summary_from_weights(
        params: &QueryParams,
        global_weights: HashMap<i32, f64>,
        zone_weights: ZoneWeightSummary,
    ) -> WindowSummary {
        if global_weights.is_empty() {
            return WindowSummary {
                alpha_total: 0.0,
                alpha_by_fish: HashMap::new(),
                p_mean_by_fish: HashMap::new(),
                c_zone: HashMap::new(),
                ess: 0.0,
                total_weight: zone_weights.weight_sum,
                last_seen: zone_weights.last_seen,
            };
        }

        let mut c_global: HashMap<i32, f64> = HashMap::with_capacity(global_weights.len());
        let mut c_zone: HashMap<i32, f64> =
            HashMap::with_capacity(zone_weights.weights_by_fish.len());
        if params.fish_norm {
            for (&fish_id, &fish_weight) in &global_weights {
                if fish_weight > 0.0 {
                    c_global.insert(fish_id, 1.0);
                }
            }
            for (&fish_id, &zone_weight) in &zone_weights.weights_by_fish {
                let fish_weight = global_weights.get(&fish_id).copied().unwrap_or(0.0);
                if fish_weight > 0.0 {
                    c_zone.insert(fish_id, zone_weight / fish_weight.max(EPS_FISH));
                }
            }
        } else {
            c_global = global_weights;
            c_zone = zone_weights.weights_by_fish;
        }

        let total_global: f64 = c_global.values().sum();
        if total_global <= 0.0 {
            let ess = if zone_weights.weight2_sum > 0.0 {
                (zone_weights.weight_sum * zone_weights.weight_sum)
                    / zone_weights.weight2_sum.max(EPS)
            } else {
                0.0
            };
            return WindowSummary {
                alpha_total: 0.0,
                alpha_by_fish: HashMap::new(),
                p_mean_by_fish: HashMap::new(),
                c_zone,
                ess,
                total_weight: zone_weights.weight_sum,
                last_seen: zone_weights.last_seen,
            };
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

        let ess = if zone_weights.weight2_sum > 0.0 {
            (zone_weights.weight_sum * zone_weights.weight_sum) / zone_weights.weight2_sum.max(EPS)
        } else {
            0.0
        };

        WindowSummary {
            alpha_total,
            alpha_by_fish,
            p_mean_by_fish,
            c_zone,
            ess,
            total_weight: zone_weights.weight_sum,
            last_seen: zone_weights.last_seen,
        }
    }

    fn compute_window_summary(
        &self,
        params: &QueryParams,
        zone_rgb_u32: u32,
        support_mode: EventZoneSupportMode,
    ) -> AppResult<WindowSummary> {
        let global_weights = self.load_zone_stats_global_weights(params, support_mode)?;
        let zone_weights = self.load_zone_stats_zone_weights(params, zone_rgb_u32, support_mode)?;
        Ok(Self::window_summary_from_weights(
            params,
            global_weights,
            zone_weights,
        ))
    }

    fn compute_drift_info(
        &self,
        params: &QueryParams,
        zone_rgb_u32: u32,
        boundary: i64,
        cfg: &ZoneStatusConfig,
        support_mode: EventZoneSupportMode,
    ) -> AppResult<(Option<DriftInfo>, bool, Option<String>)> {
        let mut old_params = params.clone();
        old_params.to_ts_utc = boundary;
        old_params.drift_boundary_ts = None;

        let mut new_params = params.clone();
        new_params.from_ts_utc = boundary;
        new_params.drift_boundary_ts = None;

        let old = self.compute_window_summary(&old_params, zone_rgb_u32, support_mode)?;
        let new = self.compute_window_summary(&new_params, zone_rgb_u32, support_mode)?;

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
    #[instrument(name = "store.get_meta", skip_all)]
    async fn get_meta(&self) -> AppResult<MetaResponse> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            if let Ok(cache) = this.meta_cache.lock() {
                if let Some((cached_at, cached)) = cache.as_ref() {
                    if cached_at.elapsed() <= Duration::from_secs(META_CACHE_TTL_SECS) {
                        return Ok(cached.clone());
                    }
                }
            }

            let patches = this.query_patches()?;
            let map_versions = this.query_map_versions()?;
            let default_patch = patches.last().cloned();
            let canonical_map = CanonicalMapInfo::default();
            let meta = MetaResponse {
                api_version: API_VERSION.to_string(),
                canonical_map,
                patches,
                default_patch,
                map_versions,
                data_languages: this.query_data_languages()?,
                defaults: this.defaults.clone(),
            };
            if let Ok(mut cache) = this.meta_cache.lock() {
                *cache = Some((Instant::now(), meta.clone()));
            }
            Ok(meta)
        })
        .await
        .map_err(|err| AppError::internal(err.to_string()))?
    }

    #[instrument(name = "store.get_region_groups", skip_all)]
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

    #[instrument(name = "store.list_fish", skip_all)]
    async fn list_fish(
        &self,
        lang: DataLang,
        ref_id: Option<String>,
    ) -> AppResult<FishListResponse> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.query_fish_list_cached(lang, ref_id.as_deref()))
            .await
            .map_err(|err| AppError::internal(err.to_string()))?
    }

    #[instrument(name = "store.fish_best_spots", skip_all)]
    async fn fish_best_spots(
        &self,
        lang: DataLang,
        ref_id: Option<String>,
        item_id: i32,
    ) -> AppResult<FishBestSpotsResponse> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            this.query_fish_best_spots_cached(lang, ref_id.as_deref(), item_id)
        })
        .await
        .map_err(|err| AppError::internal(err.to_string()))?
    }

    #[instrument(name = "store.community_fish_zone_support", skip_all)]
    async fn community_fish_zone_support(
        &self,
        ref_id: Option<String>,
    ) -> AppResult<CommunityFishZoneSupportResponse> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            this.query_community_fish_zone_support_cached(ref_id.as_deref())
        })
        .await
        .map_err(|err| AppError::internal(err.to_string()))?
    }

    #[instrument(name = "store.calculator_catalog", skip_all)]
    async fn calculator_catalog(
        &self,
        lang: DataLang,
        ref_id: Option<String>,
    ) -> AppResult<CalculatorCatalogResponse> {
        let this = self.clone();
        let span = tracing::Span::current();
        tokio::task::spawn_blocking(move || {
            let _span = span.enter();
            this.query_calculator_catalog(lang, ref_id.as_deref())
        })
        .await
        .map_err(|err| AppError::internal(err.to_string()))?
    }

    #[instrument(name = "store.calculator_zone_loot", skip_all)]
    async fn calculator_zone_loot(
        &self,
        lang: DataLang,
        ref_id: Option<String>,
        zone_rgb_key: String,
    ) -> AppResult<Vec<CalculatorZoneLootEntry>> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            this.query_calculator_zone_loot_cached(lang, ref_id.as_deref(), &zone_rgb_key)
        })
        .await
        .map_err(|err| AppError::internal(err.to_string()))?
    }

    #[instrument(name = "store.list_zones", skip_all)]
    async fn list_zones(&self, ref_id: Option<String>) -> AppResult<Vec<ZoneEntry>> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.query_zones(ref_id.as_deref()))
            .await
            .map_err(|err| AppError::internal(err.to_string()))?
    }

    #[instrument(name = "store.zone_stats", skip_all)]
    async fn zone_stats(
        &self,
        request: ZoneStatsRequest,
        status_cfg: ZoneStatusConfig,
    ) -> AppResult<ZoneStatsResponse> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            let zone_rgb_u32 = request.rgb.to_u32().map_err(AppError::invalid_argument)?;
            let layer_id = request.layer_id.as_deref().or(Some(ZONE_MASK_LAYER_ID));
            let layer_revision_id = this.resolve_layer_revision_id(
                request.layer_revision_id.as_deref(),
                request.map_version_id.as_ref(),
                layer_id,
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

            let lang = DataLang::from_param(request.lang.as_deref())?;
            let fish_names = this.query_fish_names(&lang, request.ref_id.as_deref())?;
            let fish_table = this.query_fish_identities(&lang, request.ref_id.as_deref())?;
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

    #[instrument(name = "store.zone_profile_v2", skip_all)]
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

    #[instrument(name = "store.effort_grid", skip_all)]
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

    #[instrument(name = "store.events_snapshot_meta", skip_all)]
    async fn events_snapshot_meta(&self) -> AppResult<EventsSnapshotMetaResponse> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || this.query_events_snapshot_meta())
            .await
            .map_err(|err| AppError::internal(err.to_string()))?
    }

    #[instrument(name = "store.events_snapshot", skip_all)]
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

    #[instrument(name = "store.healthcheck", skip_all)]
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
        compute_status, dolt_socket_timeout_secs, event_source_kind_from_db,
        event_zone_assignment_map, fish_catch_methods_from_description, fish_is_dried,
        group_event_zone_membership_rows, group_event_zone_support_rows, merge_fish_catalog_row,
        parse_layer_kind, parse_positive_i64, parse_vector_source, pixel_to_tile_index,
        resolve_layer_asset_url, revision_database_name, synthetic_events_snapshot_revision,
        zone_distribution_fish_ids, DoltMySqlStore, EventZoneSupportRow, FishCatalogRow,
        FishIdentityEntry, FishIdentityIndex, QueryParams, VectorSourceFields, WindowSummary,
        ZoneWeightSummary,
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

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn dolt_socket_timeout_tracks_request_budget_with_floor() {
        assert_eq!(dolt_socket_timeout_secs(5), 60);
        assert_eq!(dolt_socket_timeout_secs(15), 60);
        assert_eq!(dolt_socket_timeout_secs(45), 75);
        assert_eq!(dolt_socket_timeout_secs(90), 120);
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
    fn revision_database_name_appends_ref_to_base_database() {
        assert_eq!(
            revision_database_name("fishystuff", "beta"),
            "fishystuff/beta"
        );
        assert_eq!(
            revision_database_name("fishystuff/main", "beta"),
            "fishystuff/beta"
        );
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
                Some("/vectors/region_groups.v1.geojson"),
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
                Some("/vectors/region_groups.{map_version}.geojson"),
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

        assert_eq!(source.url, "/vectors/region_groups.v1.geojson");
    }

    #[test]
    fn layer_asset_url_resolution_handles_root_and_relative_paths() {
        assert_eq!(
            resolve_layer_asset_url("/images/tiles/minimap_visual/v1/tileset.json"),
            "/images/tiles/minimap_visual/v1/tileset.json"
        );
        assert_eq!(
            resolve_layer_asset_url("images/tiles/minimap_visual/v1/tileset.json"),
            "images/tiles/minimap_visual/v1/tileset.json"
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

    #[test]
    fn window_summary_from_weights_preserves_zone_evidence_math() {
        let params = QueryParams {
            map_version: "layer-1".to_string(),
            from_ts_utc: 0,
            to_ts_utc: 200,
            half_life_days: None,
            tile_px: 32,
            sigma_tiles: 3.0,
            fish_norm: false,
            alpha0: 1.0,
            top_k: 10,
            drift_boundary_ts: None,
        };
        let global = HashMap::from([(1, 10.0), (2, 30.0)]);
        let zone = ZoneWeightSummary {
            weights_by_fish: HashMap::from([(1, 3.0), (2, 1.0)]),
            weight_sum: 4.0,
            weight2_sum: 4.0,
            last_seen: Some(100),
        };

        let summary = DoltMySqlStore::window_summary_from_weights(&params, global, zone);

        assert_eq!(summary.c_zone.get(&1).copied(), Some(3.0));
        assert_eq!(summary.c_zone.get(&2).copied(), Some(1.0));
        assert_close(summary.alpha_total, 5.0);
        assert_close(summary.alpha_by_fish.get(&1).copied().unwrap(), 3.25);
        assert_close(summary.alpha_by_fish.get(&2).copied().unwrap(), 1.75);
        assert_close(summary.p_mean_by_fish.get(&1).copied().unwrap(), 0.65);
        assert_close(summary.p_mean_by_fish.get(&2).copied().unwrap(), 0.35);
        assert_close(summary.ess, 4.0);
        assert_close(summary.total_weight, 4.0);
        assert_eq!(summary.last_seen, Some(100));
    }

    #[test]
    fn window_summary_from_weights_applies_fish_normalization_to_counts_only() {
        let params = QueryParams {
            map_version: "layer-1".to_string(),
            from_ts_utc: 0,
            to_ts_utc: 200,
            half_life_days: None,
            tile_px: 32,
            sigma_tiles: 3.0,
            fish_norm: true,
            alpha0: 1.0,
            top_k: 10,
            drift_boundary_ts: None,
        };
        let global = HashMap::from([(1, 10.0), (2, 30.0)]);
        let zone = ZoneWeightSummary {
            weights_by_fish: HashMap::from([(1, 5.0), (2, 15.0)]),
            weight_sum: 20.0,
            weight2_sum: 250.0,
            last_seen: Some(150),
        };

        let summary = DoltMySqlStore::window_summary_from_weights(&params, global, zone);

        assert_eq!(summary.c_zone.get(&1).copied(), Some(0.5));
        assert_eq!(summary.c_zone.get(&2).copied(), Some(0.5));
        assert_close(summary.alpha_total, 2.0);
        assert_close(summary.p_mean_by_fish.get(&1).copied().unwrap(), 0.5);
        assert_close(summary.p_mean_by_fish.get(&2).copied().unwrap(), 0.5);
        assert_close(summary.ess, 1.6);
        assert_close(summary.total_weight, 20.0);
        assert_eq!(summary.last_seen, Some(150));
    }

    #[test]
    fn group_event_zone_support_rows_merges_multiple_zone_rows_per_event() {
        let grouped = group_event_zone_support_rows(&[
            (10, 1_700_000_000, 8201, 0x010203),
            (10, 1_700_000_000, 8201, 0x040506),
            (11, 1_700_000_010, 8202, 0x070809),
        ])
        .expect("group rows");

        assert_eq!(
            grouped,
            vec![
                EventZoneSupportRow {
                    event_id: 10,
                    ts_utc: 1_700_000_000,
                    fish_id: 8201,
                    zone_rgbs: vec![0x010203, 0x040506],
                },
                EventZoneSupportRow {
                    event_id: 11,
                    ts_utc: 1_700_000_010,
                    fish_id: 8202,
                    zone_rgbs: vec![0x070809],
                },
            ]
        );
    }

    #[test]
    fn group_event_zone_support_rows_deduplicates_duplicate_zone_rows() {
        let grouped = group_event_zone_support_rows(&[
            (10, 1_700_000_000, 8201, 0x010203),
            (10, 1_700_000_000, 8201, 0x010203),
        ])
        .expect("group rows");

        assert_eq!(
            grouped,
            vec![EventZoneSupportRow {
                event_id: 10,
                ts_utc: 1_700_000_000,
                fish_id: 8201,
                zone_rgbs: vec![0x010203],
            }]
        );
    }

    #[test]
    fn group_event_zone_membership_rows_merges_and_deduplicates_zone_sets() {
        let grouped = group_event_zone_membership_rows(&[
            (10, 0x010203),
            (10, 0x010203),
            (10, 0x040506),
            (11, 0x070809),
        ])
        .expect("group rows");

        assert_eq!(
            grouped,
            HashMap::from([(10_i64, vec![0x010203, 0x040506]), (11_i64, vec![0x070809]),]),
        );
    }

    #[test]
    fn event_zone_assignment_map_keeps_first_zone_per_event() {
        let assignment =
            event_zone_assignment_map(&[(10, 0x010203), (10, 0x040506), (11, 0x070809)])
                .expect("assignment map");

        assert_eq!(
            assignment,
            HashMap::from([(10_i64, 0x010203_u32), (11_i64, 0x070809_u32)]),
        );
    }
}
