use fishystuff_api::models::trade::{
    ExcludedTradeNpc, TradeNpcCatalogResponse, TradeNpcCatalogSummary, TradeNpcDestination,
    TradeNpcSourceDescriptor, TradeNpcSpawn, TradeNpcTradeInfo, TradeOriginRegion,
    TradeRegionWaypointRef, TradeZoneOriginRegion, TradeZoneOriginRegions,
};
use mysql::prelude::Queryable;

use crate::error::{AppError, AppResult};
use crate::store::validate_dolt_ref;

use super::util::{db_unavailable, is_missing_table, normalize_optional_string, row_string};
use super::DoltMySqlStore;

const TRADE_NPC_CATALOG_META_TABLE: &str = "trade_npc_catalog_meta";
const TRADE_NPC_CATALOG_SOURCES_TABLE: &str = "trade_npc_catalog_sources";
const TRADE_ORIGIN_REGIONS_TABLE: &str = "trade_origin_regions";
const TRADE_ZONE_ORIGIN_REGIONS_TABLE: &str = "trade_zone_origin_regions";
const TRADE_NPC_DESTINATIONS_TABLE: &str = "trade_npc_destinations";
const TRADE_NPC_EXCLUDED_TABLE: &str = "trade_npc_excluded";

type TradeNpcSourceRow = (Option<String>, Option<String>, Option<String>);
type TradeOriginRegionRow = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);
type TradeZoneOriginRegionRow = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);
type TradeNpcExcludedRow = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

impl DoltMySqlStore {
    pub(super) fn query_trade_npc_catalog(
        &self,
        ref_id: Option<&str>,
    ) -> AppResult<TradeNpcCatalogResponse> {
        let as_of = dolt_as_of(ref_id)?;
        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;

        let meta = query_trade_meta(&mut conn, &as_of)?;
        let sources = query_trade_sources(&mut conn, &as_of)?;
        let origin_regions = query_trade_origin_regions(&mut conn, &as_of)?;
        let zone_origin_regions = query_trade_zone_origin_regions(&mut conn, &as_of)?;
        let destinations = query_trade_destinations(&mut conn, &as_of)?;
        let excluded = query_trade_excluded(&mut conn, &as_of)?;

        Ok(TradeNpcCatalogResponse {
            schema: meta.0,
            version: meta.1,
            coordinate_space: meta.2,
            sources,
            summary: meta.3,
            origin_regions,
            zone_origin_regions,
            destinations,
            excluded,
        })
    }
}

fn query_trade_meta(
    conn: &mut mysql::PooledConn,
    as_of: &str,
) -> AppResult<(String, u32, String, TradeNpcCatalogSummary)> {
    let query = format!(
        "SELECT \
            `catalog_schema`, \
            CAST(`version` AS CHAR), \
            `coordinate_space`, \
            CAST(`character_function_trade_rows` AS CHAR), \
            CAST(`character_function_barter_rows` AS CHAR), \
            CAST(`character_function_trade_barter_overlap_rows` AS CHAR), \
            CAST(`selling_to_npc_rows` AS CHAR), \
            CAST(`title_trade_manager_rows` AS CHAR), \
            CAST(`candidate_npcs` AS CHAR), \
            CAST(`origin_regions` AS CHAR), \
            CAST(`zone_origin_regions` AS CHAR), \
            CAST(`destinations` AS CHAR), \
            CAST(`excluded_missing_spawn` AS CHAR), \
            CAST(`excluded_missing_trade_origin` AS CHAR) \
         FROM {TRADE_NPC_CATALOG_META_TABLE}{as_of} \
         ORDER BY `catalog_schema` \
         LIMIT 1"
    );
    let row: Option<mysql::Row> = query_trade_rows(conn, &query, TRADE_NPC_CATALOG_META_TABLE)?
        .into_iter()
        .next();
    let Some(row) = row else {
        return Err(AppError::unavailable(
            "trade NPC Dolt catalog is empty; import trade NPC catalog tables",
        ));
    };
    Ok((
        required_string(row_string(&row, 0), "trade_npc_catalog_meta.catalog_schema")?,
        required_u32(row_string(&row, 1), "trade_npc_catalog_meta.version")?,
        required_string(
            row_string(&row, 2),
            "trade_npc_catalog_meta.coordinate_space",
        )?,
        TradeNpcCatalogSummary {
            character_function_trade_rows: required_usize(
                row_string(&row, 3),
                "trade_npc_catalog_meta.character_function_trade_rows",
            )?,
            character_function_barter_rows: required_usize(
                row_string(&row, 4),
                "trade_npc_catalog_meta.character_function_barter_rows",
            )?,
            character_function_trade_barter_overlap_rows: required_usize(
                row_string(&row, 5),
                "trade_npc_catalog_meta.character_function_trade_barter_overlap_rows",
            )?,
            selling_to_npc_rows: required_usize(
                row_string(&row, 6),
                "trade_npc_catalog_meta.selling_to_npc_rows",
            )?,
            title_trade_manager_rows: required_usize(
                row_string(&row, 7),
                "trade_npc_catalog_meta.title_trade_manager_rows",
            )?,
            candidate_npcs: required_usize(
                row_string(&row, 8),
                "trade_npc_catalog_meta.candidate_npcs",
            )?,
            origin_regions: required_usize(
                row_string(&row, 9),
                "trade_npc_catalog_meta.origin_regions",
            )?,
            zone_origin_regions: required_usize(
                row_string(&row, 10),
                "trade_npc_catalog_meta.zone_origin_regions",
            )?,
            destinations: required_usize(
                row_string(&row, 11),
                "trade_npc_catalog_meta.destinations",
            )?,
            excluded_missing_spawn: required_usize(
                row_string(&row, 12),
                "trade_npc_catalog_meta.excluded_missing_spawn",
            )?,
            excluded_missing_trade_origin: required_usize(
                row_string(&row, 13),
                "trade_npc_catalog_meta.excluded_missing_trade_origin",
            )?,
        },
    ))
}

fn query_trade_sources(
    conn: &mut mysql::PooledConn,
    as_of: &str,
) -> AppResult<Vec<TradeNpcSourceDescriptor>> {
    let query = format!(
        "SELECT `source_id`, `file`, `role` \
         FROM {TRADE_NPC_CATALOG_SOURCES_TABLE}{as_of} \
         ORDER BY `source_id`"
    );
    query_trade_table::<TradeNpcSourceRow>(conn, &query, TRADE_NPC_CATALOG_SOURCES_TABLE)?
        .into_iter()
        .map(|(id, file, role)| {
            Ok(TradeNpcSourceDescriptor {
                id: required_string(id, "trade_npc_catalog_sources.source_id")?,
                file: required_string(file, "trade_npc_catalog_sources.file")?,
                role: required_string(role, "trade_npc_catalog_sources.role")?,
            })
        })
        .collect()
}

fn query_trade_origin_regions(
    conn: &mut mysql::PooledConn,
    as_of: &str,
) -> AppResult<Vec<TradeOriginRegion>> {
    let query = format!(
        "SELECT \
            CAST(`region_id` AS CHAR), \
            `region_name`, \
            CAST(`waypoint_id` AS CHAR), \
            `waypoint_name`, \
            CAST(`world_x` AS CHAR), \
            CAST(`world_z` AS CHAR) \
         FROM {TRADE_ORIGIN_REGIONS_TABLE}{as_of} \
         ORDER BY `region_name`, `region_id`"
    );
    query_trade_table::<TradeOriginRegionRow>(conn, &query, TRADE_ORIGIN_REGIONS_TABLE)?
        .into_iter()
        .map(|row| {
            Ok(TradeOriginRegion {
                region_id: required_u32(row.0, "trade_origin_regions.region_id")?,
                region_name: normalize_optional_string(row.1),
                waypoint_id: optional_u32(row.2, "trade_origin_regions.waypoint_id")?,
                waypoint_name: normalize_optional_string(row.3),
                world_x: required_f64(row.4, "trade_origin_regions.world_x")?,
                world_z: required_f64(row.5, "trade_origin_regions.world_z")?,
            })
        })
        .collect()
}

fn query_trade_zone_origin_regions(
    conn: &mut mysql::PooledConn,
    as_of: &str,
) -> AppResult<Vec<TradeZoneOriginRegions>> {
    let query = format!(
        "SELECT \
            `zone_rgb_key`, \
            CAST(`zone_rgb_u32` AS CHAR), \
            CAST(`origin_region_id` AS CHAR), \
            CAST(`pixel_count` AS CHAR) \
         FROM {TRADE_ZONE_ORIGIN_REGIONS_TABLE}{as_of} \
         ORDER BY `zone_rgb_u32`, `pixel_count` DESC, `origin_region_id`"
    );
    let mut by_zone = Vec::<TradeZoneOriginRegions>::new();
    for row in query_trade_table::<TradeZoneOriginRegionRow>(
        conn,
        &query,
        TRADE_ZONE_ORIGIN_REGIONS_TABLE,
    )? {
        let zone_rgb_key = required_string(row.0, "trade_zone_origin_regions.zone_rgb_key")?;
        let zone_rgb_u32 = required_u32(row.1, "trade_zone_origin_regions.zone_rgb_u32")?;
        let origin = TradeZoneOriginRegion {
            region_id: required_u32(row.2, "trade_zone_origin_regions.origin_region_id")?,
            pixel_count: required_u32(row.3, "trade_zone_origin_regions.pixel_count")?,
        };
        if let Some(existing) = by_zone
            .last_mut()
            .filter(|entry| entry.zone_rgb_u32 == zone_rgb_u32)
        {
            existing.origins.push(origin);
        } else {
            by_zone.push(TradeZoneOriginRegions {
                zone_rgb_key,
                zone_rgb_u32,
                origins: vec![origin],
            });
        }
    }
    Ok(by_zone)
}

fn query_trade_destinations(
    conn: &mut mysql::PooledConn,
    as_of: &str,
) -> AppResult<Vec<TradeNpcDestination>> {
    let query = format!(
        "SELECT \
            `destination_id`, \
            CAST(`npc_key` AS CHAR), \
            `npc_name`, \
            `role_source`, \
            `source_tags_json`, \
            CAST(`item_main_group_key` AS CHAR), \
            `trade_group_type`, \
            CAST(`npc_spawn_region_id` AS CHAR), \
            `npc_spawn_region_name`, \
            CAST(`npc_spawn_world_x` AS CHAR), \
            CAST(`npc_spawn_world_y` AS CHAR), \
            CAST(`npc_spawn_world_z` AS CHAR), \
            CAST(`assigned_region_id` AS CHAR), \
            `assigned_region_name`, \
            CAST(`assigned_waypoint_id` AS CHAR), \
            `assigned_waypoint_name`, \
            CAST(`assigned_world_x` AS CHAR), \
            CAST(`assigned_world_z` AS CHAR), \
            CAST(`sell_origin_region_id` AS CHAR), \
            `sell_origin_region_name`, \
            CAST(`sell_origin_waypoint_id` AS CHAR), \
            `sell_origin_waypoint_name`, \
            CAST(`sell_origin_world_x` AS CHAR), \
            CAST(`sell_origin_world_z` AS CHAR) \
         FROM {TRADE_NPC_DESTINATIONS_TABLE}{as_of} \
         ORDER BY `assigned_region_name`, `npc_name`, `npc_key`, `destination_id`"
    );
    query_trade_rows(conn, &query, TRADE_NPC_DESTINATIONS_TABLE)?
        .into_iter()
        .map(|row| {
            Ok(TradeNpcDestination {
                id: required_string(row_string(&row, 0), "trade_npc_destinations.destination_id")?,
                npc_key: required_u32(row_string(&row, 1), "trade_npc_destinations.npc_key")?,
                npc_name: required_string(row_string(&row, 2), "trade_npc_destinations.npc_name")?,
                role_source: required_string(
                    row_string(&row, 3),
                    "trade_npc_destinations.role_source",
                )?,
                source_tags: source_tags(
                    row_string(&row, 4),
                    "trade_npc_destinations.source_tags_json",
                )?,
                trade: TradeNpcTradeInfo {
                    item_main_group_key: optional_u32(
                        row_string(&row, 5),
                        "trade_npc_destinations.item_main_group_key",
                    )?,
                    trade_group_type: normalize_optional_string(row_string(&row, 6)),
                },
                npc_spawn: TradeNpcSpawn {
                    region_id: required_u32(
                        row_string(&row, 7),
                        "trade_npc_destinations.npc_spawn_region_id",
                    )?,
                    region_name: normalize_optional_string(row_string(&row, 8)),
                    world_x: required_f64(
                        row_string(&row, 9),
                        "trade_npc_destinations.npc_spawn_world_x",
                    )?,
                    world_y: required_f64(
                        row_string(&row, 10),
                        "trade_npc_destinations.npc_spawn_world_y",
                    )?,
                    world_z: required_f64(
                        row_string(&row, 11),
                        "trade_npc_destinations.npc_spawn_world_z",
                    )?,
                },
                assigned_region: TradeRegionWaypointRef {
                    region_id: optional_u32(
                        row_string(&row, 12),
                        "trade_npc_destinations.assigned_region_id",
                    )?,
                    region_name: normalize_optional_string(row_string(&row, 13)),
                    waypoint_id: optional_u32(
                        row_string(&row, 14),
                        "trade_npc_destinations.assigned_waypoint_id",
                    )?,
                    waypoint_name: normalize_optional_string(row_string(&row, 15)),
                    world_x: optional_f64(
                        row_string(&row, 16),
                        "trade_npc_destinations.assigned_world_x",
                    )?,
                    world_z: optional_f64(
                        row_string(&row, 17),
                        "trade_npc_destinations.assigned_world_z",
                    )?,
                },
                sell_destination_trade_origin: TradeRegionWaypointRef {
                    region_id: optional_u32(
                        row_string(&row, 18),
                        "trade_npc_destinations.sell_origin_region_id",
                    )?,
                    region_name: normalize_optional_string(row_string(&row, 19)),
                    waypoint_id: optional_u32(
                        row_string(&row, 20),
                        "trade_npc_destinations.sell_origin_waypoint_id",
                    )?,
                    waypoint_name: normalize_optional_string(row_string(&row, 21)),
                    world_x: optional_f64(
                        row_string(&row, 22),
                        "trade_npc_destinations.sell_origin_world_x",
                    )?,
                    world_z: optional_f64(
                        row_string(&row, 23),
                        "trade_npc_destinations.sell_origin_world_z",
                    )?,
                },
            })
        })
        .collect()
}

fn query_trade_excluded(
    conn: &mut mysql::PooledConn,
    as_of: &str,
) -> AppResult<Vec<ExcludedTradeNpc>> {
    let query = format!(
        "SELECT CAST(`npc_key` AS CHAR), `npc_name`, `reason`, `source_tags_json` \
         FROM {TRADE_NPC_EXCLUDED_TABLE}{as_of} \
         ORDER BY `reason`, `npc_name`, `npc_key`"
    );
    query_trade_table::<TradeNpcExcludedRow>(conn, &query, TRADE_NPC_EXCLUDED_TABLE)?
        .into_iter()
        .map(|row| {
            Ok(ExcludedTradeNpc {
                npc_key: required_u32(row.0, "trade_npc_excluded.npc_key")?,
                npc_name: required_string(row.1, "trade_npc_excluded.npc_name")?,
                reason: required_string(row.2, "trade_npc_excluded.reason")?,
                source_tags: source_tags(row.3, "trade_npc_excluded.source_tags_json")?,
            })
        })
        .collect()
}

fn query_trade_table<T>(conn: &mut mysql::PooledConn, query: &str, table: &str) -> AppResult<Vec<T>>
where
    T: mysql::prelude::FromRow,
{
    match conn.query(query) {
        Ok(rows) => Ok(rows),
        Err(err) if is_missing_table(&err, table) => Err(AppError::unavailable(format!(
            "trade NPC Dolt table `{table}` is missing; import trade NPC catalog tables"
        ))),
        Err(err) => Err(db_unavailable(err)),
    }
}

fn query_trade_rows(
    conn: &mut mysql::PooledConn,
    query: &str,
    table: &str,
) -> AppResult<Vec<mysql::Row>> {
    match conn.query(query) {
        Ok(rows) => Ok(rows),
        Err(err) if is_missing_table(&err, table) => Err(AppError::unavailable(format!(
            "trade NPC Dolt table `{table}` is missing; import trade NPC catalog tables"
        ))),
        Err(err) => Err(db_unavailable(err)),
    }
}

fn dolt_as_of(ref_id: Option<&str>) -> AppResult<String> {
    let Some(ref_id) = ref_id.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(String::new());
    };
    validate_dolt_ref(ref_id)?;
    Ok(format!(" AS OF '{}'", ref_id.replace('\'', "''")))
}

fn required_string(value: Option<String>, field: &str) -> AppResult<String> {
    normalize_optional_string(value)
        .ok_or_else(|| AppError::internal(format!("trade NPC catalog field {field} is empty")))
}

fn required_u32(value: Option<String>, field: &str) -> AppResult<u32> {
    let value = required_string(value, field)?;
    value
        .parse::<u32>()
        .map_err(|err| AppError::internal(format!("parse trade NPC catalog {field}: {err}")))
}

fn required_usize(value: Option<String>, field: &str) -> AppResult<usize> {
    let value = required_string(value, field)?;
    value
        .parse::<usize>()
        .map_err(|err| AppError::internal(format!("parse trade NPC catalog {field}: {err}")))
}

fn optional_u32(value: Option<String>, field: &str) -> AppResult<Option<u32>> {
    let Some(value) = normalize_optional_string(value) else {
        return Ok(None);
    };
    value
        .parse::<u32>()
        .map(Some)
        .map_err(|err| AppError::internal(format!("parse trade NPC catalog {field}: {err}")))
}

fn required_f64(value: Option<String>, field: &str) -> AppResult<f64> {
    let value = required_string(value, field)?;
    value
        .parse::<f64>()
        .map_err(|err| AppError::internal(format!("parse trade NPC catalog {field}: {err}")))
}

fn optional_f64(value: Option<String>, field: &str) -> AppResult<Option<f64>> {
    let Some(value) = normalize_optional_string(value) else {
        return Ok(None);
    };
    value
        .parse::<f64>()
        .map(Some)
        .map_err(|err| AppError::internal(format!("parse trade NPC catalog {field}: {err}")))
}

fn source_tags(value: Option<String>, field: &str) -> AppResult<Vec<String>> {
    let Some(value) = normalize_optional_string(value) else {
        return Ok(Vec::new());
    };
    serde_json::from_str::<Vec<String>>(&value)
        .map_err(|err| AppError::internal(format!("parse trade NPC catalog {field}: {err}")))
}
