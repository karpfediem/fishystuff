use std::hash::{Hash, Hasher};

use chrono::TimeZone;
use fishystuff_api::models::events::EventSourceKind;
use fishystuff_api::models::region_groups::RegionGroupDescriptor;
use mysql::Row;

use crate::error::{AppError, AppResult};

use super::FishCatalogRow;

pub(super) fn clamp_i64_to_u32(value: i64, fallback: u32) -> u32 {
    u32::try_from(value.max(0)).unwrap_or(fallback)
}

pub(super) fn synthetic_events_snapshot_revision(
    source_revision: Option<&str>,
    event_count: usize,
    max_ts_utc: Option<i64>,
    max_event_id: Option<i64>,
) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source_revision.unwrap_or("").hash(&mut hasher);
    event_count.hash(&mut hasher);
    max_ts_utc.unwrap_or_default().hash(&mut hasher);
    max_event_id.unwrap_or_default().hash(&mut hasher);
    format!("events-{:016x}", hasher.finish())
}

pub(super) fn synthetic_fish_revision(
    source_revision: Option<&str>,
    fish: &[FishCatalogRow],
) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source_revision.unwrap_or("").hash(&mut hasher);
    for entry in fish {
        entry.item_id.hash(&mut hasher);
        entry.encyclopedia_key.hash(&mut hasher);
        entry.encyclopedia_id.hash(&mut hasher);
        entry.name.hash(&mut hasher);
        entry.grade.hash(&mut hasher);
        entry.is_prize.hash(&mut hasher);
        entry.is_dried.hash(&mut hasher);
        entry.catch_methods.hash(&mut hasher);
        entry.vendor_price.hash(&mut hasher);
    }
    format!("fish-{:016x}", hasher.finish())
}

pub(super) fn synthetic_region_groups_revision(
    map_version_id: Option<&str>,
    groups: &[RegionGroupDescriptor],
) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    map_version_id.unwrap_or("").hash(&mut hasher);
    for group in groups {
        if let Ok(serialized) = serde_json::to_string(group) {
            serialized.hash(&mut hasher);
        } else {
            group.region_group_id.hash(&mut hasher);
        }
    }
    format!("synthetic:{:016x}", hasher.finish())
}

pub(super) fn event_source_kind_from_db(source_kind: i64) -> Option<EventSourceKind> {
    match source_kind {
        1 => Some(EventSourceKind::Ranking),
        _ => None,
    }
}

pub(super) fn row_string(row: &Row, idx: usize) -> Option<String> {
    row.get::<Option<String>, _>(idx)
        .flatten()
        .map(|value| value.trim().to_string())
}

pub(super) fn row_u32_opt(row: &Row, idx: usize) -> Option<u32> {
    row.get::<Option<i64>, _>(idx)
        .flatten()
        .and_then(|value| u32::try_from(value).ok())
}

pub(super) fn row_opt_f64(row: &Row, idx: usize) -> Option<f64> {
    row.get::<Option<f64>, _>(idx).flatten()
}

pub(super) fn row_i64(row: &Row, idx: usize, fallback: i64) -> i64 {
    row.get::<Option<i64>, _>(idx).flatten().unwrap_or(fallback)
}

pub(super) fn epoch_to_mysql_datetime(ts_utc: i64) -> AppResult<String> {
    let dt = chrono::Utc
        .timestamp_opt(ts_utc, 0)
        .single()
        .ok_or_else(|| AppError::invalid_argument(format!("invalid timestamp: {ts_utc}")))?;
    Ok(dt.format("%Y-%m-%d %H:%M:%S%.6f").to_string())
}

pub(super) fn db_unavailable(err: impl std::fmt::Display) -> AppError {
    AppError::unavailable(format!("database unavailable: {err}"))
}

pub(super) fn is_missing_table(err: &mysql::Error, table: &str) -> bool {
    matches!(
        err,
        mysql::Error::MySqlError(server)
            if server.code == 1146 && server.message.contains(table)
    )
}

fn is_unknown_column(err: &mysql::Error, column: &str) -> bool {
    match err {
        mysql::Error::MySqlError(server) => {
            let message = server.message.to_ascii_lowercase();
            let has_column = message.contains(&column.to_ascii_lowercase());
            let unknown_column_code = server.code == 1054;
            let dolt_unknown_column = message.contains("does not have column");
            has_column && (unknown_column_code || dolt_unknown_column)
        }
        _ => false,
    }
}

fn is_events_schema_error(err: &mysql::Error) -> bool {
    is_unknown_column(err, "map_px_x")
        || is_unknown_column(err, "map_px_y")
        || is_unknown_column(err, "sample_px_x")
        || is_unknown_column(err, "sample_px_y")
        || is_unknown_column(err, "event_id")
        || is_unknown_column(err, "source_kind")
        || is_unknown_column(err, "source_id")
        || is_missing_table(err, "event_zone_assignment")
}

pub(super) fn events_schema_or_db_unavailable(err: mysql::Error) -> AppError {
    if is_events_schema_error(&err) {
        return AppError::unavailable(
            "events schema is outdated for /api/v1/events_snapshot and /api/v1/zone_stats. \
             Use a Dolt commit or branch that contains the current events schema \
             (and rebuild events tables and re-import ranking if needed), then rebuild event zone assignments.",
        );
    }
    db_unavailable(err)
}

pub(super) fn normalize_optional_string(value: Option<String>) -> Option<String> {
    let value = value?;
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("null")
        || trimmed.eq_ignore_ascii_case("<null>")
    {
        None
    } else {
        Some(trimmed.to_string())
    }
}
