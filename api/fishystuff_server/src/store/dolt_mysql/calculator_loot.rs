use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use fishystuff_api::error::ApiErrorCode;
use fishystuff_api::ids::RgbKey;
use fishystuff_core::fish_icons::{fish_icon_path_from_asset_file, parse_fish_icon_asset_id};
use mysql::prelude::Queryable;
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::store::{
    queries, validate_dolt_ref, CalculatorZoneLootEntry, CalculatorZoneLootEvidence,
    CalculatorZoneLootOverlayMeta, DataLang,
};

use super::catalog::{item_grade_from_db, parse_positive_i64};
use super::util::{db_unavailable, is_missing_table, normalize_optional_string};
use super::DoltMySqlStore;

fn calculator_loot_item_icon_path(icon_id: i32) -> String {
    format!("/images/items/{icon_id:08}.webp")
}

fn resolve_calculator_loot_item_icon(item_id: i32, icon_file: Option<&str>) -> String {
    icon_file
        .and_then(fish_icon_path_from_asset_file)
        .or_else(|| {
            icon_file
                .and_then(parse_fish_icon_asset_id)
                .map(calculator_loot_item_icon_path)
        })
        .unwrap_or_else(|| calculator_loot_item_icon_path(item_id))
}

const COMMUNITY_PRIZE_GUESS_SOURCE_ID: &str = "community_prize_fish_guesses_workbook";
const MANUAL_COMMUNITY_GUESS_SOURCE_ID: &str = "manual_community_zone_fish_guess";
const GROUP_RATE_SCALE: f64 = 1_000_000.0;
const COMBINED_GROUP_RATE_SCALE: f64 = GROUP_RATE_SCALE * GROUP_RATE_SCALE;
const HARPOON_SLOT_IDX: u8 = 6;
const CALCULATOR_ZONE_LOOT_RETRY_BASE_DELAY: Duration = Duration::from_millis(250);
const CALCULATOR_ZONE_LOOT_RETRY_MAX_DELAY: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, Deserialize)]
struct CommunityPrizeGuessMeta {
    slot_idx: u8,
    guessed_rate: f64,
    subgroup_key: Option<i64>,
}

#[derive(Debug, Clone, Copy, Default)]
struct CommunitySupportMeta {
    slot_idx: Option<u8>,
    guessed_rate: Option<f64>,
    item_main_group_key: Option<i64>,
    subgroup_key: Option<i64>,
}

#[derive(Debug, Clone)]
struct CommunityPresenceMeta {
    source_id: String,
    item_id: i32,
    support_status: String,
    claim_count: u32,
    slot_idx: Option<u8>,
    item_main_group_key: Option<i64>,
    subgroup_key: Option<i64>,
}

#[derive(Debug, Clone, Copy, Default)]
struct RankingPresenceMeta {
    full_count: u32,
    partial_count: u32,
}

fn parse_community_support_notes(notes: &str) -> CommunitySupportMeta {
    let mut meta = CommunitySupportMeta::default();
    for part in notes.split(';') {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        match key.trim() {
            "slot_idx" => {
                meta.slot_idx = value.trim().parse::<u8>().ok().filter(|value| *value > 0)
            }
            "guessed_rate" => {
                meta.guessed_rate = value
                    .trim()
                    .parse::<f64>()
                    .ok()
                    .filter(|value| *value > 0.0)
            }
            "item_main_group_key" | "main_group_key" => {
                meta.item_main_group_key =
                    value.trim().parse::<i64>().ok().filter(|value| *value > 0)
            }
            "item_sub_group_key" | "subgroup_key" => {
                meta.subgroup_key = value.trim().parse::<i64>().ok().filter(|value| *value > 0)
            }
            _ => {}
        }
    }
    meta
}

fn parse_community_prize_guess_notes(notes: &str) -> Option<CommunityPrizeGuessMeta> {
    let meta = parse_community_support_notes(notes);
    let slot_idx = meta.slot_idx?;
    let guessed_rate = meta.guessed_rate?;
    (guessed_rate > 0.0).then_some(CommunityPrizeGuessMeta {
        slot_idx,
        guessed_rate,
        subgroup_key: meta.subgroup_key,
    })
}

fn is_community_guess_source_id(source_id: &str) -> bool {
    matches!(
        source_id,
        COMMUNITY_PRIZE_GUESS_SOURCE_ID | MANUAL_COMMUNITY_GUESS_SOURCE_ID
    )
}

fn community_presence_scope(meta: &CommunityPresenceMeta) -> &'static str {
    if meta.subgroup_key.is_some() {
        "subgroup"
    } else if meta.item_main_group_key.is_some() || meta.slot_idx.is_some() {
        "group"
    } else {
        "zone"
    }
}

fn community_presence_specificity(meta: &CommunityPresenceMeta) -> u8 {
    match community_presence_scope(meta) {
        "subgroup" => 3,
        "group" => 2,
        _ => 1,
    }
}

fn community_status_priority(status: &str) -> u8 {
    match status.trim().to_ascii_lowercase().as_str() {
        "confirmed" => 3,
        "guessed" => 2,
        "unconfirmed" => 1,
        "data_incomplete" => 0,
        _ => 0,
    }
}

fn community_presence_matches_row(
    meta: &CommunityPresenceMeta,
    slot_idx: u8,
    item_main_group_key: i64,
    subgroup_keys: &[i64],
) -> bool {
    if let Some(expected_slot_idx) = meta.slot_idx {
        if expected_slot_idx != slot_idx {
            return false;
        }
    }
    if let Some(expected_item_main_group_key) = meta.item_main_group_key {
        if expected_item_main_group_key != item_main_group_key {
            return false;
        }
    }
    if let Some(expected_subgroup_key) = meta.subgroup_key {
        if !subgroup_keys.contains(&expected_subgroup_key) {
            return false;
        }
    }
    true
}

fn community_presence_slot_idx(
    meta: &CommunityPresenceMeta,
    slot_main_group_by_idx: &HashMap<u8, i64>,
    slot_subgroup_select_rate: &HashMap<(u8, i64), i64>,
) -> Option<u8> {
    meta.slot_idx
        .filter(|slot_idx| *slot_idx > 0)
        .or_else(|| {
            meta.item_main_group_key.and_then(|item_main_group_key| {
                slot_main_group_by_idx
                    .iter()
                    .find_map(|(slot_idx, candidate)| {
                        (*candidate == item_main_group_key).then_some(*slot_idx)
                    })
            })
        })
        .or_else(|| {
            meta.subgroup_key.and_then(|subgroup_key| {
                slot_subgroup_select_rate
                    .keys()
                    .find_map(|(slot_idx, candidate)| {
                        (*candidate == subgroup_key).then_some(*slot_idx)
                    })
            })
        })
}

fn build_community_presence_evidence(meta: &CommunityPresenceMeta) -> CalculatorZoneLootEvidence {
    CalculatorZoneLootEvidence {
        source_family: "community".to_string(),
        claim_kind: "presence".to_string(),
        scope: community_presence_scope(meta).to_string(),
        rate: None,
        normalized_rate: None,
        status: Some(meta.support_status.clone()),
        claim_count: Some(meta.claim_count),
        source_id: Some(meta.source_id.clone()),
        slot_idx: meta.slot_idx,
        item_main_group_key: meta.item_main_group_key,
        subgroup_key: meta.subgroup_key,
    }
}

fn push_ranking_presence_evidence(
    evidence: &mut Vec<CalculatorZoneLootEvidence>,
    meta: RankingPresenceMeta,
    layer_revision_id: &str,
) {
    if meta.full_count > 0 {
        evidence.push(CalculatorZoneLootEvidence {
            source_family: "ranking".to_string(),
            claim_kind: "presence".to_string(),
            scope: "ring_full".to_string(),
            rate: None,
            normalized_rate: None,
            status: Some("observed".to_string()),
            claim_count: Some(meta.full_count),
            source_id: Some(format!("layer_revision:{layer_revision_id}")),
            ..CalculatorZoneLootEvidence::default()
        });
    }
    if meta.partial_count > 0 {
        evidence.push(CalculatorZoneLootEvidence {
            source_family: "ranking".to_string(),
            claim_kind: "presence".to_string(),
            scope: "ring_partial".to_string(),
            rate: None,
            normalized_rate: None,
            status: Some("observed".to_string()),
            claim_count: Some(meta.partial_count),
            source_id: Some(format!("layer_revision:{layer_revision_id}")),
            ..CalculatorZoneLootEvidence::default()
        });
    }
}

fn zone_loot_slot_sort_key(slot_idx: u8) -> u8 {
    if slot_idx == 0 {
        u8::MAX
    } else {
        slot_idx
    }
}

fn should_retry_calculator_zone_loot_error(err: &AppError) -> bool {
    matches!(err.0.code, ApiErrorCode::Unavailable)
}

fn calculator_zone_loot_retry_delay(failures: u32) -> Duration {
    let multiplier = 1u32
        .checked_shl(failures.saturating_sub(1).min(31))
        .unwrap_or(u32::MAX);
    CALCULATOR_ZONE_LOOT_RETRY_BASE_DELAY
        .checked_mul(multiplier)
        .unwrap_or(CALCULATOR_ZONE_LOOT_RETRY_MAX_DELAY)
        .min(CALCULATOR_ZONE_LOOT_RETRY_MAX_DELAY)
}

fn apply_community_guess_weights(
    aggregate_weights: &HashMap<(u8, i32), f64>,
    community_guess_by_key: &HashMap<(u8, i32), CommunityPrizeGuessMeta>,
    slot_subgroup_select_rate: &HashMap<(u8, i64), i64>,
    slot_option_count: &HashMap<u8, usize>,
) -> HashMap<(u8, i32), f64> {
    let mut effective_weights = aggregate_weights.clone();
    for ((slot_idx, item_id), guess) in community_guess_by_key {
        let select_rate = guess
            .subgroup_key
            .and_then(|subgroup_key| {
                slot_subgroup_select_rate
                    .get(&(*slot_idx, subgroup_key))
                    .copied()
            })
            .or_else(|| {
                if slot_option_count.get(slot_idx).copied().unwrap_or_default() == 1 {
                    slot_subgroup_select_rate.iter().find_map(
                        |((candidate_slot_idx, _), select_rate)| {
                            (*candidate_slot_idx == *slot_idx).then_some(*select_rate)
                        },
                    )
                } else {
                    None
                }
            });
        let Some(select_rate) = select_rate else {
            continue;
        };
        let guessed_weight = guess.guessed_rate * GROUP_RATE_SCALE * (select_rate as f64);
        if guessed_weight <= 0.0 {
            continue;
        }
        effective_weights
            .entry((*slot_idx, *item_id))
            .or_insert(guessed_weight);
    }
    effective_weights
}

impl DoltMySqlStore {
    fn calculator_zone_loot_cache_key(
        lang: &DataLang,
        ref_id: Option<&str>,
        zone_rgb_key: &str,
    ) -> String {
        let lang = lang.code();
        match ref_id {
            Some(ref_id) => format!("{lang}:{ref_id}:{zone_rgb_key}"),
            None => format!("{lang}:head:{zone_rgb_key}"),
        }
    }

    pub(super) fn query_calculator_zone_loot_cached(
        &self,
        lang: DataLang,
        ref_id: Option<&str>,
        zone_rgb_key: &str,
    ) -> AppResult<Vec<CalculatorZoneLootEntry>> {
        self.validate_data_lang_available(&lang, ref_id)?;
        let cache_key = Self::calculator_zone_loot_cache_key(&lang, ref_id, zone_rgb_key);
        loop {
            if let Ok(cache) = self.calculator_zone_loot_cache.lock() {
                if let Some(cached) = cache.get(&cache_key) {
                    return Ok(cached.clone());
                }
            }

            let (load_state_lock, load_state_cvar) = &*self.calculator_zone_loot_load_state;
            let mut load_state = load_state_lock
                .lock()
                .expect("calculator zone loot load state lock poisoned");
            if load_state.inflight.contains(&cache_key) {
                load_state = load_state_cvar
                    .wait(load_state)
                    .expect("calculator zone loot load state wait poisoned");
                drop(load_state);
                continue;
            }
            if let Some(retry_backoff) = load_state.retry_backoff.get(&cache_key) {
                let now = Instant::now();
                if now < retry_backoff.retry_at {
                    let wait = retry_backoff.retry_at.saturating_duration_since(now);
                    let (guard, _) = load_state_cvar
                        .wait_timeout(load_state, wait)
                        .expect("calculator zone loot backoff wait poisoned");
                    drop(guard);
                    continue;
                }
            }
            if !load_state.inflight.contains(&cache_key) {
                load_state.inflight.insert(cache_key.clone());
                drop(load_state);
                break;
            }
        }

        let result = self.query_calculator_zone_loot(lang, ref_id, zone_rgb_key);

        if let Ok(rows) = &result {
            if let Ok(mut cache) = self.calculator_zone_loot_cache.lock() {
                cache.insert(cache_key.clone(), rows.clone());
            }
        }

        let (load_state_lock, load_state_cvar) = &*self.calculator_zone_loot_load_state;
        let mut load_state = load_state_lock
            .lock()
            .expect("calculator zone loot load state lock poisoned");
        load_state.inflight.remove(&cache_key);
        match &result {
            Ok(_) => {
                load_state.retry_backoff.remove(&cache_key);
            }
            Err(err) if should_retry_calculator_zone_loot_error(err) => {
                let failures = load_state
                    .retry_backoff
                    .get(&cache_key)
                    .map_or(1, |retry_backoff| retry_backoff.failures.saturating_add(1));
                let delay = calculator_zone_loot_retry_delay(failures);
                load_state.retry_backoff.insert(
                    cache_key.clone(),
                    super::CalculatorZoneLootRetryBackoff {
                        failures,
                        retry_at: Instant::now() + delay,
                    },
                );
                tracing::warn!(
                    cache.key = %cache_key,
                    retry.failures = failures,
                    retry.delay.ms = delay.as_millis() as u64,
                    error.message = %err.0.message,
                    "calculator zone loot cache fill failed; backing off before retry"
                );
            }
            Err(_) => {
                load_state.retry_backoff.remove(&cache_key);
            }
        }
        load_state_cvar.notify_all();
        drop(load_state);

        result
    }

    fn query_calculator_zone_loot(
        &self,
        lang: DataLang,
        ref_id: Option<&str>,
        zone_rgb_key: &str,
    ) -> AppResult<Vec<CalculatorZoneLootEntry>> {
        self.validate_data_lang_available(&lang, ref_id)?;
        let zone_rgb = zone_rgb_key
            .parse::<RgbKey>()
            .map_err(AppError::invalid_argument)?
            .as_rgb()
            .map_err(AppError::invalid_argument)?;
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let zone_query = format!(
            "SELECT \
                CAST(slot_idx AS SIGNED), \
                CAST(item_main_group_key AS SIGNED) \
             FROM flockfish_zone_group_slots{as_of} \
             WHERE zone_rgb = ? \
               AND resolution_status = 'numeric' \
             ORDER BY slot_idx"
        );
        let zone_rows: Vec<(i64, i64)> = conn
            .exec(zone_query, (zone_rgb.to_u32(),))
            .map_err(db_unavailable)?;
        let mut slot_rows = zone_rows
            .into_iter()
            .filter_map(|(slot_idx, item_main_group_key)| {
                let slot_idx = u8::try_from(slot_idx).ok()?;
                (item_main_group_key > 0).then_some((slot_idx, item_main_group_key, "rod"))
            })
            .collect::<Vec<_>>();
        let harpoon_query = format!(
            "SELECT CAST(DropIDHarpoon AS SIGNED) \
             FROM fishing_table{as_of} \
             WHERE R = ? AND G = ? AND B = ? \
               AND DropIDHarpoon IS NOT NULL \
             LIMIT 1"
        );
        let harpoon_main_group_key: Option<i64> = conn
            .exec_first(harpoon_query, (zone_rgb.r, zone_rgb.g, zone_rgb.b))
            .map_err(db_unavailable)?;
        if let Some(item_main_group_key) = harpoon_main_group_key.filter(|value| *value > 0) {
            slot_rows.push((HARPOON_SLOT_IDX, item_main_group_key, "harpoon"));
        }
        slot_rows.sort_by_key(|(slot_idx, _, _)| *slot_idx);
        let slot_main_group_by_idx = slot_rows
            .iter()
            .map(|(slot_idx, item_main_group_key, _)| (*slot_idx, *item_main_group_key))
            .collect::<HashMap<u8, i64>>();
        let slot_method_by_idx = slot_rows
            .iter()
            .map(|(slot_idx, _, method)| (*slot_idx, (*method).to_string()))
            .collect::<HashMap<u8, String>>();

        let mut subgroup_options = HashMap::<i64, Vec<(i64, i64)>>::new();
        let mut group_conditions_raw = HashMap::<i64, Vec<String>>::new();
        if !slot_rows.is_empty() {
            let main_group_ids = slot_rows
                .iter()
                .map(|(_, item_main_group_key, _)| *item_main_group_key)
                .collect::<HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            let main_group_id_csv = main_group_ids
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");
            let main_group_query = format!(
                "SELECT \
                    CAST(item_main_group_key AS SIGNED), \
                    CAST(select_rate AS SIGNED), \
                    NULLIF(TRIM(condition_raw), '') AS condition_raw, \
                    CAST(item_sub_group_key AS SIGNED) \
                 FROM item_main_group_options{as_of} \
                 WHERE item_main_group_key IN ({main_group_id_csv}) \
                 ORDER BY item_main_group_key, option_idx"
            );
            let main_group_rows: Vec<(i64, Option<i64>, Option<String>, Option<i64>)> =
                conn.query(main_group_query).map_err(db_unavailable)?;

            for (item_main_group_key, select_rate, condition_raw, subgroup_key) in main_group_rows {
                if let Some(condition_raw) = normalize_optional_string(condition_raw) {
                    let conditions = group_conditions_raw.entry(item_main_group_key).or_default();
                    if !conditions.contains(&condition_raw) {
                        conditions.push(condition_raw);
                    }
                }
                let Some(select_rate) = select_rate else {
                    continue;
                };
                let Some(subgroup_key) = subgroup_key else {
                    continue;
                };
                if select_rate <= 0 || subgroup_key <= 0 {
                    continue;
                }
                subgroup_options
                    .entry(item_main_group_key)
                    .or_default()
                    .push((select_rate, subgroup_key));
            }
        }

        let subgroup_ids = subgroup_options
            .values()
            .flat_map(|options| options.iter().map(|(_, subgroup_key)| *subgroup_key))
            .collect::<Vec<_>>();
        let mut subgroup_variants = HashMap::<i64, Vec<(i32, i64)>>::new();
        if !subgroup_ids.is_empty() {
            let subgroup_id_csv = subgroup_ids
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");
            let subgroup_query = format!(
                "SELECT \
                    CAST(item_sub_group_key AS SIGNED), \
                    CAST(item_key AS SIGNED), \
                    CAST(select_rate AS SIGNED) \
                 FROM item_sub_group_item_variants{as_of} \
                 WHERE item_sub_group_key IN ({subgroup_id_csv})"
            );
            let subgroup_rows: Vec<(i64, i64, Option<i64>)> =
                conn.query(subgroup_query).map_err(db_unavailable)?;

            for (item_sub_group_key, item_key, select_rate) in subgroup_rows {
                let Ok(item_id) = i32::try_from(item_key) else {
                    continue;
                };
                if item_id <= 0 {
                    continue;
                }
                let Some(select_rate) = select_rate.filter(|value| *value > 0) else {
                    continue;
                };
                subgroup_variants
                    .entry(item_sub_group_key)
                    .or_default()
                    .push((item_id, select_rate));
            }
        }

        let mut aggregate_weights = HashMap::<(u8, i32), f64>::new();
        let mut slot_item_subgroups = HashMap::<(u8, i32), Vec<i64>>::new();
        for (slot_idx, item_main_group_key, _) in &slot_rows {
            let Some(options) = subgroup_options.get(item_main_group_key) else {
                continue;
            };
            for (select_rate, subgroup_key) in options {
                let Some(variants) = subgroup_variants.get(subgroup_key) else {
                    continue;
                };
                for (item_id, variant_rate) in variants {
                    let weight = (*select_rate as f64) * (*variant_rate as f64);
                    *aggregate_weights.entry((*slot_idx, *item_id)).or_default() += weight;
                    let subgroup_keys = slot_item_subgroups
                        .entry((*slot_idx, *item_id))
                        .or_default();
                    if !subgroup_keys.contains(subgroup_key) {
                        subgroup_keys.push(*subgroup_key);
                    }
                }
            }
        }
        let mut slot_subgroup_select_rate = HashMap::<(u8, i64), i64>::new();
        let mut slot_option_count = HashMap::<u8, usize>::new();
        for (slot_idx, item_main_group_key, _) in &slot_rows {
            let Some(options) = subgroup_options.get(item_main_group_key) else {
                continue;
            };
            slot_option_count.insert(*slot_idx, options.len());
            for (select_rate, subgroup_key) in options {
                slot_subgroup_select_rate.insert((*slot_idx, *subgroup_key), *select_rate);
            }
        }

        let mut community_presence_rows = Vec::<CommunityPresenceMeta>::new();
        let mut community_guess_by_key =
            HashMap::<(u8, i32), (String, CommunityPrizeGuessMeta)>::new();
        let community_query = format!(
            "SELECT source_id, CAST(item_id AS SIGNED), support_status, CAST(claim_count AS SIGNED), notes \
             FROM community_zone_fish_support{as_of} \
             WHERE zone_rgb = ?"
        );
        let community_rows: Vec<(String, i64, String, i64, Option<String>)> =
            match conn.exec(community_query, (zone_rgb.to_u32(),)) {
                Ok(rows) => rows,
                Err(err) if is_missing_table(&err, "community_zone_fish_support") => Vec::new(),
                Err(err) => return Err(db_unavailable(err)),
            };
        for (source_id, item_id, support_status, claim_count, notes) in community_rows {
            let Ok(item_id) = i32::try_from(item_id) else {
                continue;
            };
            let support_status = support_status.trim().to_ascii_lowercase();
            let claim_count = u32::try_from(claim_count.max(0)).unwrap_or(u32::MAX);
            if is_community_guess_source_id(&source_id) {
                let Some(notes) = normalize_optional_string(notes) else {
                    continue;
                };
                let parsed_meta = parse_community_support_notes(&notes);
                let Some(guess) = parse_community_prize_guess_notes(&notes) else {
                    continue;
                };
                community_guess_by_key
                    .insert((guess.slot_idx, item_id), (source_id.clone(), guess));
                community_presence_rows.push(CommunityPresenceMeta {
                    source_id,
                    item_id,
                    support_status,
                    claim_count,
                    slot_idx: parsed_meta.slot_idx.or(Some(guess.slot_idx)),
                    item_main_group_key: parsed_meta.item_main_group_key,
                    subgroup_key: parsed_meta.subgroup_key.or(guess.subgroup_key),
                });
            } else {
                let meta = notes
                    .as_deref()
                    .map(parse_community_support_notes)
                    .unwrap_or_default();
                community_presence_rows.push(CommunityPresenceMeta {
                    source_id,
                    item_id,
                    support_status,
                    claim_count,
                    slot_idx: meta.slot_idx,
                    item_main_group_key: meta.item_main_group_key,
                    subgroup_key: meta.subgroup_key,
                });
            }
        }
        let fish_identities = self.query_fish_identities(ref_id)?;
        let layer_revision_id = match self.resolve_layer_revision_id(
            None,
            self.defaults.map_version_id.as_ref(),
            Some(super::ZONE_MASK_LAYER_ID),
            None,
            None,
            0,
        ) {
            Ok(value) => Some(value),
            Err(err) if err.0.code == ApiErrorCode::NotFound => None,
            Err(err) => return Err(err),
        };
        let ranking_presence_rows: Vec<(i64, i64, i64)> = match layer_revision_id {
            Some(ref layer_revision_id) => match conn.exec(
                queries::RANKING_RING_SUPPORT_BY_ZONE_SQL,
                (
                    layer_revision_id,
                    super::SOURCE_KIND_RANKING,
                    zone_rgb.to_u32(),
                ),
            ) {
                Ok(rows) => rows,
                Err(err) if is_missing_table(&err, "event_zone_ring_support") => Vec::new(),
                Err(err) => return Err(db_unavailable(err)),
            },
            None => Vec::new(),
        };
        let mut ranking_presence_by_item = HashMap::<i32, RankingPresenceMeta>::new();
        for (fish_id, full_count, partial_count) in ranking_presence_rows {
            let Ok(fish_id) = i32::try_from(fish_id) else {
                continue;
            };
            let item_id = fish_identities
                .by_encyclopedia
                .get(&fish_id)
                .map(|entry| entry.item_id)
                .unwrap_or(fish_id);
            let meta = ranking_presence_by_item.entry(item_id).or_default();
            meta.full_count = meta
                .full_count
                .saturating_add(u32::try_from(full_count.max(0)).unwrap_or(u32::MAX));
            meta.partial_count = meta
                .partial_count
                .saturating_add(u32::try_from(partial_count.max(0)).unwrap_or(u32::MAX));
        }

        let community_guess_meta_by_key = community_guess_by_key
            .iter()
            .map(|(key, (_, guess))| (*key, *guess))
            .collect::<HashMap<_, _>>();

        let effective_weights = apply_community_guess_weights(
            &aggregate_weights,
            &community_guess_meta_by_key,
            &slot_subgroup_select_rate,
            &slot_option_count,
        );

        let mut slot_totals = HashMap::<u8, f64>::new();
        for ((slot_idx, _), weight) in &effective_weights {
            *slot_totals.entry(*slot_idx).or_default() += *weight;
        }

        let item_ids = effective_weights
            .keys()
            .map(|(_, item_id)| item_id.to_string())
            .chain(
                community_presence_rows
                    .iter()
                    .map(|meta| meta.item_id.to_string()),
            )
            .chain(
                ranking_presence_by_item
                    .keys()
                    .copied()
                    .map(|item_id| item_id.to_string()),
            )
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if item_ids.is_empty() {
            return Ok(Vec::new());
        }
        let item_id_csv = item_ids.join(",");
        let item_query = format!(
            "SELECT \
                CAST(it.`Index` AS SIGNED), \
                NULLIF(TRIM(item_name.`name`), '') AS item_name, \
                NULLIF(TRIM(it.`IconImageFile`), '') AS icon_file, \
                it.`GradeType`, \
                it.`OriginalPrice`, \
                CASE WHEN ft.item_key IS NULL THEN 0 ELSE 1 END AS is_fish \
             FROM item_table{as_of} it \
             LEFT JOIN fish_table{as_of} ft ON ft.item_key = it.`Index` \
             LEFT JOIN calculator_item_names{as_of} item_name \
               ON item_name.`item_id` = CAST(it.`Index` AS SIGNED) \
              AND item_name.`lang` = '{}' \
             WHERE it.`Index` IN ({item_id_csv})",
            lang.code().replace('\'', "''")
        );
        let item_rows: Vec<(
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            i64,
        )> = conn.query(item_query).map_err(db_unavailable)?;
        let mut item_meta =
            HashMap::<i32, (String, Option<String>, Option<String>, Option<i64>, bool)>::new();
        for (item_id, name, icon_file, grade_type, original_price, is_fish) in item_rows {
            let Ok(item_id) = i32::try_from(item_id) else {
                continue;
            };
            let name = normalize_optional_string(name).unwrap_or_else(|| item_id.to_string());
            let icon_file = normalize_optional_string(icon_file);
            let icon = Some(resolve_calculator_loot_item_icon(
                item_id,
                icon_file.as_deref(),
            ));
            let (grade, _, _is_prize) = item_grade_from_db(grade_type);
            let vendor_price = parse_positive_i64(original_price);
            item_meta.insert(item_id, (name, icon, grade, vendor_price, is_fish > 0));
        }

        let mut matched_community_presence_indexes = HashSet::<usize>::new();
        let mut matched_ranking_items = HashSet::<i32>::new();
        let mut entries = effective_weights
            .into_iter()
            .filter_map(|((slot_idx, item_id), weight)| {
                let total_weight = slot_totals.get(&slot_idx).copied().unwrap_or_default();
                if total_weight <= 0.0 || weight <= 0.0 {
                    return None;
                }
                let within_group_rate = weight / total_weight;
                let (name, icon, grade, vendor_price, is_fish) =
                    item_meta.get(&item_id).cloned().unwrap_or_else(|| {
                        (
                            item_id.to_string(),
                            Some(calculator_loot_item_icon_path(item_id)),
                            None,
                            None,
                            false,
                        )
                    });
                let catch_methods = slot_method_by_idx
                    .get(&slot_idx)
                    .cloned()
                    .map(|method| vec![method])
                    .unwrap_or_else(|| vec!["rod".to_string()]);
                let mut evidence = Vec::new();
                if let Some(db_weight) = aggregate_weights.get(&(slot_idx, item_id)).copied() {
                    evidence.push(CalculatorZoneLootEvidence {
                        source_family: "database".to_string(),
                        claim_kind: "in_group_rate".to_string(),
                        scope: "group".to_string(),
                        rate: Some(db_weight / COMBINED_GROUP_RATE_SCALE),
                        normalized_rate: Some(db_weight / total_weight),
                        status: Some("best_effort".to_string()),
                        claim_count: None,
                        slot_idx: Some(slot_idx),
                        item_main_group_key: slot_main_group_by_idx.get(&slot_idx).copied(),
                        ..CalculatorZoneLootEvidence::default()
                    });
                }
                if let Some((guess_source_id, guess)) =
                    community_guess_by_key.get(&(slot_idx, item_id))
                {
                    let guess_weight = slot_subgroup_select_rate
                        .get(&(slot_idx, guess.subgroup_key.unwrap_or_default()))
                        .copied()
                        .map(|select_rate| {
                            guess.guessed_rate * GROUP_RATE_SCALE * (select_rate as f64)
                        })
                        .or_else(|| {
                            if slot_option_count
                                .get(&slot_idx)
                                .copied()
                                .unwrap_or_default()
                                == 1
                            {
                                slot_subgroup_select_rate.iter().find_map(
                                    |((candidate_slot_idx, _), select_rate)| {
                                        (*candidate_slot_idx == slot_idx).then_some(
                                            guess.guessed_rate
                                                * GROUP_RATE_SCALE
                                                * (*select_rate as f64),
                                        )
                                    },
                                )
                            } else {
                                None
                            }
                        });
                    evidence.push(CalculatorZoneLootEvidence {
                        source_family: "community".to_string(),
                        claim_kind: "guessed_in_group_rate".to_string(),
                        scope: "group".to_string(),
                        rate: Some(guess.guessed_rate),
                        normalized_rate: guess_weight.map(|weight| weight / total_weight),
                        status: Some("guessed".to_string()),
                        claim_count: None,
                        source_id: Some(guess_source_id.clone()),
                        slot_idx: Some(guess.slot_idx),
                        item_main_group_key: slot_main_group_by_idx.get(&slot_idx).copied(),
                        subgroup_key: guess.subgroup_key,
                        ..CalculatorZoneLootEvidence::default()
                    });
                }
                let item_main_group_key = slot_main_group_by_idx
                    .get(&slot_idx)
                    .copied()
                    .unwrap_or_default();
                let group_conditions_raw = group_conditions_raw
                    .get(&item_main_group_key)
                    .cloned()
                    .unwrap_or_default();
                let subgroup_keys = slot_item_subgroups
                    .get(&(slot_idx, item_id))
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let mut matching_presence = community_presence_rows
                    .iter()
                    .enumerate()
                    .filter(|meta| {
                        meta.1.item_id == item_id
                            && community_presence_matches_row(
                                meta.1,
                                slot_idx,
                                item_main_group_key,
                                subgroup_keys,
                            )
                    })
                    .collect::<Vec<_>>();
                matching_presence.sort_by(|left, right| {
                    community_presence_specificity(right.1)
                        .cmp(&community_presence_specificity(left.1))
                        .then_with(|| {
                            community_status_priority(&right.1.support_status)
                                .cmp(&community_status_priority(&left.1.support_status))
                        })
                        .then_with(|| right.1.claim_count.cmp(&left.1.claim_count))
                        .then_with(|| left.1.source_id.cmp(&right.1.source_id))
                });
                for (index, meta) in matching_presence {
                    matched_community_presence_indexes.insert(index);
                    evidence.push(build_community_presence_evidence(meta));
                }
                if let Some(meta) = ranking_presence_by_item.get(&item_id).copied() {
                    matched_ranking_items.insert(item_id);
                    if let Some(layer_revision_id) = layer_revision_id.as_deref() {
                        push_ranking_presence_evidence(&mut evidence, meta, layer_revision_id);
                    }
                }
                Some(CalculatorZoneLootEntry {
                    slot_idx,
                    item_id,
                    name,
                    icon,
                    vendor_price,
                    grade,
                    is_fish,
                    catch_methods,
                    group_conditions_raw,
                    within_group_rate,
                    evidence,
                    overlay: CalculatorZoneLootOverlayMeta::default(),
                })
            })
            .collect::<Vec<_>>();
        let mut synthetic_evidence_by_key =
            HashMap::<(u8, i32), Vec<CalculatorZoneLootEvidence>>::new();
        let mut synthetic_keys_by_item = HashMap::<i32, Vec<(u8, i32)>>::new();
        for (index, meta) in community_presence_rows.iter().enumerate() {
            if matched_community_presence_indexes.contains(&index) {
                continue;
            }
            let slot_idx = community_presence_slot_idx(
                meta,
                &slot_main_group_by_idx,
                &slot_subgroup_select_rate,
            )
            .unwrap_or(0);
            let key = (slot_idx, meta.item_id);
            synthetic_evidence_by_key
                .entry(key)
                .or_default()
                .push(build_community_presence_evidence(meta));
            synthetic_keys_by_item
                .entry(meta.item_id)
                .or_default()
                .push(key);
        }
        for keys in synthetic_keys_by_item.values_mut() {
            keys.sort_unstable();
            keys.dedup();
        }
        for (item_id, meta) in ranking_presence_by_item {
            if matched_ranking_items.contains(&item_id) {
                continue;
            }
            let key = synthetic_keys_by_item
                .get(&item_id)
                .filter(|keys| keys.len() == 1)
                .and_then(|keys| keys.first().copied())
                .unwrap_or((0, item_id));
            push_ranking_presence_evidence(
                synthetic_evidence_by_key.entry(key).or_default(),
                meta,
                layer_revision_id
                    .as_deref()
                    .expect("ranking presence rows require a resolved layer revision id"),
            );
        }
        for ((slot_idx, item_id), evidence) in synthetic_evidence_by_key {
            let (name, icon, grade, vendor_price, is_fish) =
                item_meta.get(&item_id).cloned().unwrap_or_else(|| {
                    (
                        item_id.to_string(),
                        Some(calculator_loot_item_icon_path(item_id)),
                        None,
                        None,
                        false,
                    )
                });
            let item_main_group_key = slot_main_group_by_idx
                .get(&slot_idx)
                .copied()
                .or_else(|| {
                    evidence
                        .iter()
                        .find_map(|row| row.item_main_group_key.filter(|value| *value > 0))
                })
                .unwrap_or_default();
            let catch_methods = slot_method_by_idx
                .get(&slot_idx)
                .cloned()
                .map(|method| vec![method])
                .unwrap_or_else(|| vec!["rod".to_string()]);
            entries.push(CalculatorZoneLootEntry {
                slot_idx,
                item_id,
                name,
                icon,
                vendor_price,
                grade,
                is_fish,
                catch_methods,
                group_conditions_raw: group_conditions_raw
                    .get(&item_main_group_key)
                    .cloned()
                    .unwrap_or_default(),
                within_group_rate: 0.0,
                evidence,
                overlay: CalculatorZoneLootOverlayMeta::default(),
            });
        }
        entries.sort_by(|left, right| {
            zone_loot_slot_sort_key(left.slot_idx)
                .cmp(&zone_loot_slot_sort_key(right.slot_idx))
                .then_with(|| {
                    right
                        .within_group_rate
                        .partial_cmp(&left.within_group_rate)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
                .then_with(|| left.item_id.cmp(&right.item_id))
        });
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::Duration;

    use crate::error::AppError;

    use super::{
        apply_community_guess_weights, community_presence_matches_row, community_presence_scope,
        community_presence_slot_idx, community_status_priority, is_community_guess_source_id,
        parse_community_prize_guess_notes, parse_community_support_notes,
        resolve_calculator_loot_item_icon, CommunityPresenceMeta, CommunityPrizeGuessMeta,
        MANUAL_COMMUNITY_GUESS_SOURCE_ID,
    };

    #[test]
    fn loot_icons_prefer_explicit_source_stems() {
        assert_eq!(
            resolve_calculator_loot_item_icon(
                24277,
                Some("New_Icon/03_ETC/06_Housing/InHouse_DPFO_birthdayCake_01.dds"),
            ),
            "/images/items/InHouse_DPFO_birthdayCake_01.webp"
        );
    }

    #[test]
    fn loot_icons_fall_back_to_item_ids_when_no_source_icon_file_exists() {
        assert_eq!(
            resolve_calculator_loot_item_icon(9307, None),
            "/images/items/00009307.webp"
        );
    }

    #[test]
    fn parse_community_prize_guess_notes_reads_slot_and_rate() {
        let parsed =
            parse_community_prize_guess_notes("slot_idx=1;guessed_rate=0.02;subgroup_key=11054")
                .expect("guess notes should parse");

        assert_eq!(parsed.slot_idx, 1);
        assert!((parsed.guessed_rate - 0.02).abs() < f64::EPSILON);
        assert_eq!(parsed.subgroup_key, Some(11054));
    }

    #[test]
    fn parse_community_prize_guess_notes_rejects_missing_fields() {
        assert!(parse_community_prize_guess_notes("guessed_rate=0.02").is_none());
        assert!(parse_community_prize_guess_notes("slot_idx=1").is_none());
    }

    #[test]
    fn manual_community_guess_source_id_is_recognized() {
        assert!(is_community_guess_source_id(
            MANUAL_COMMUNITY_GUESS_SOURCE_ID
        ));
    }

    #[test]
    fn parse_community_support_notes_reads_structural_keys() {
        let meta =
            parse_community_support_notes("slot_idx=1;item_main_group_key=9001;subgroup_key=11054");

        assert_eq!(meta.slot_idx, Some(1));
        assert_eq!(meta.item_main_group_key, Some(9001));
        assert_eq!(meta.subgroup_key, Some(11054));
    }

    #[test]
    fn community_status_priority_treats_guessed_as_presence_support() {
        assert!(community_status_priority("confirmed") > community_status_priority("guessed"));
        assert!(community_status_priority("guessed") > community_status_priority("unconfirmed"));
    }

    #[test]
    fn community_presence_scope_prefers_explicit_group_lineage() {
        let zone = CommunityPresenceMeta {
            source_id: "community_sheet".to_string(),
            item_id: 8201,
            support_status: "confirmed".to_string(),
            claim_count: 2,
            slot_idx: None,
            item_main_group_key: None,
            subgroup_key: None,
        };
        let group = CommunityPresenceMeta {
            item_main_group_key: Some(9001),
            ..zone.clone()
        };
        let subgroup = CommunityPresenceMeta {
            slot_idx: Some(1),
            subgroup_key: Some(11054),
            ..zone.clone()
        };

        assert_eq!(community_presence_scope(&zone), "zone");
        assert_eq!(community_presence_scope(&group), "group");
        assert_eq!(community_presence_scope(&subgroup), "subgroup");
    }

    #[test]
    fn community_presence_match_requires_structural_overlap() {
        let meta = CommunityPresenceMeta {
            source_id: "community_sheet".to_string(),
            item_id: 8201,
            support_status: "confirmed".to_string(),
            claim_count: 2,
            slot_idx: Some(1),
            item_main_group_key: Some(9001),
            subgroup_key: Some(11054),
        };

        assert!(community_presence_matches_row(
            &meta,
            1,
            9001,
            &[11054, 11055]
        ));
        assert!(!community_presence_matches_row(
            &meta,
            2,
            9001,
            &[11054, 11055]
        ));
        assert!(!community_presence_matches_row(
            &meta,
            1,
            9002,
            &[11054, 11055]
        ));
        assert!(!community_presence_matches_row(&meta, 1, 9001, &[11055]));
    }

    #[test]
    fn community_presence_slot_idx_derives_slot_from_lineage() {
        let meta = CommunityPresenceMeta {
            source_id: "community_sheet".to_string(),
            item_id: 8201,
            support_status: "confirmed".to_string(),
            claim_count: 2,
            slot_idx: None,
            item_main_group_key: Some(9001),
            subgroup_key: Some(11054),
        };

        assert_eq!(
            community_presence_slot_idx(
                &meta,
                &HashMap::from([(4_u8, 9001_i64)]),
                &HashMap::from([((1_u8, 11054_i64), 1_000_000_i64)]),
            ),
            Some(4)
        );
        assert_eq!(
            community_presence_slot_idx(
                &CommunityPresenceMeta {
                    item_main_group_key: None,
                    ..meta.clone()
                },
                &HashMap::new(),
                &HashMap::from([((1_u8, 11054_i64), 1_000_000_i64)]),
            ),
            Some(1)
        );
    }

    #[test]
    fn community_prize_guess_uses_subgroup_weight_scale() {
        let aggregate_weights = HashMap::from([
            ((1_u8, 8201_i32), 10_000_000_000.0),
            ((1_u8, 8473_i32), 300_000_000_000.0),
            ((1_u8, 8476_i32), 100_000_000_000.0),
        ]);
        let community_guess_by_key = HashMap::from([(
            (1_u8, 820985_i32),
            CommunityPrizeGuessMeta {
                slot_idx: 1,
                guessed_rate: 0.02,
                subgroup_key: Some(11054),
            },
        )]);
        let slot_subgroup_select_rate = HashMap::from([((1_u8, 11054_i64), 1_000_000_i64)]);
        let slot_option_count = HashMap::from([(1_u8, 1_usize)]);

        let effective_weights = apply_community_guess_weights(
            &aggregate_weights,
            &community_guess_by_key,
            &slot_subgroup_select_rate,
            &slot_option_count,
        );

        let total = effective_weights.values().sum::<f64>();
        let yellow = effective_weights
            .get(&(1, 8473))
            .copied()
            .unwrap_or_default()
            / total;
        let blue = effective_weights
            .get(&(1, 8476))
            .copied()
            .unwrap_or_default()
            / total;
        let mud = effective_weights
            .get(&(1, 8201))
            .copied()
            .unwrap_or_default()
            / total;
        let silver = effective_weights
            .get(&(1, 820985))
            .copied()
            .unwrap_or_default()
            / total;

        assert!((yellow - 0.6976744186).abs() < 1e-9);
        assert!((blue - 0.2325581395).abs() < 1e-9);
        assert!((mud - 0.0232558139).abs() < 1e-9);
        assert!((silver - 0.0465116279).abs() < 1e-9);
    }

    #[test]
    fn database_group_rate_uses_raw_source_scale() {
        let yellow_weight = 300_000_000_000.0_f64;
        let blue_weight = 100_000_000_000.0_f64;
        let mud_weight = 10_000_000_000.0_f64;

        assert!((yellow_weight / super::COMBINED_GROUP_RATE_SCALE - 0.30_f64).abs() < 1e-9);
        assert!((blue_weight / super::COMBINED_GROUP_RATE_SCALE - 0.10_f64).abs() < 1e-9);
        assert!((mud_weight / super::COMBINED_GROUP_RATE_SCALE - 0.01_f64).abs() < 1e-9);
    }

    #[test]
    fn calculator_zone_loot_retry_delay_exponentially_backs_off_with_cap() {
        assert_eq!(
            super::calculator_zone_loot_retry_delay(1),
            Duration::from_millis(250)
        );
        assert_eq!(
            super::calculator_zone_loot_retry_delay(2),
            Duration::from_millis(500)
        );
        assert_eq!(
            super::calculator_zone_loot_retry_delay(3),
            Duration::from_secs(1)
        );
        assert_eq!(
            super::calculator_zone_loot_retry_delay(4),
            Duration::from_secs(2)
        );
        assert_eq!(
            super::calculator_zone_loot_retry_delay(5),
            Duration::from_secs(4)
        );
        assert_eq!(
            super::calculator_zone_loot_retry_delay(6),
            Duration::from_secs(5)
        );
        assert_eq!(
            super::calculator_zone_loot_retry_delay(12),
            Duration::from_secs(5)
        );
    }

    #[test]
    fn calculator_zone_loot_retry_policy_only_retries_transient_db_failures() {
        assert!(super::should_retry_calculator_zone_loot_error(
            &AppError::unavailable("database unavailable")
        ));
        assert!(!super::should_retry_calculator_zone_loot_error(
            &AppError::invalid_argument("bad zone")
        ));
        assert!(!super::should_retry_calculator_zone_loot_error(
            &AppError::not_found("not found")
        ));
        assert!(!super::should_retry_calculator_zone_loot_error(
            &AppError::internal("bug")
        ));
    }
}
