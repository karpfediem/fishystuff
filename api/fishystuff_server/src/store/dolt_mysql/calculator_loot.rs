use std::collections::HashMap;

use fishystuff_api::ids::RgbKey;
use fishystuff_core::fish_icons::parse_fish_icon_asset_id;
use mysql::prelude::Queryable;
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::store::{
    validate_dolt_ref, CalculatorZoneLootEntry, CalculatorZoneLootEvidence, FishLang,
};

use super::catalog::{fish_grade_from_db, parse_positive_i64};
use super::util::{db_unavailable, is_missing_table, normalize_optional_string};
use super::DoltMySqlStore;

fn calculator_loot_item_icon_path(icon_id: i32) -> String {
    format!("/images/items/{icon_id:08}.webp")
}

const COMMUNITY_PRIZE_GUESS_SOURCE_ID: &str = "community_prize_fish_guesses_workbook";
const GROUP_RATE_SCALE: f64 = 1_000_000.0;
const COMBINED_GROUP_RATE_SCALE: f64 = GROUP_RATE_SCALE * GROUP_RATE_SCALE;

#[derive(Debug, Clone, Copy, Deserialize)]
struct CommunityPrizeGuessMeta {
    slot_idx: u8,
    guessed_rate: f64,
    subgroup_key: Option<i64>,
}

fn parse_community_prize_guess_notes(notes: &str) -> Option<CommunityPrizeGuessMeta> {
    let mut slot_idx = None;
    let mut guessed_rate = None;
    let mut subgroup_key = None;
    for part in notes.split(';') {
        let (key, value) = part.split_once('=')?;
        match key.trim() {
            "slot_idx" => slot_idx = value.trim().parse::<u8>().ok(),
            "guessed_rate" => guessed_rate = value.trim().parse::<f64>().ok(),
            "subgroup_key" => subgroup_key = value.trim().parse::<i64>().ok(),
            _ => {}
        }
    }
    let slot_idx = slot_idx?;
    let guessed_rate = guessed_rate?;
    (guessed_rate > 0.0).then_some(CommunityPrizeGuessMeta {
        slot_idx,
        guessed_rate,
        subgroup_key,
    })
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
        lang: FishLang,
        ref_id: Option<&str>,
        zone_rgb_key: &str,
    ) -> String {
        let lang = match lang {
            FishLang::En => "en",
            FishLang::Ko => "ko",
        };
        match ref_id {
            Some(ref_id) => format!("{lang}:{ref_id}:{zone_rgb_key}"),
            None => format!("{lang}:head:{zone_rgb_key}"),
        }
    }

    pub(super) fn query_calculator_zone_loot_cached(
        &self,
        lang: FishLang,
        ref_id: Option<&str>,
        zone_rgb_key: &str,
    ) -> AppResult<Vec<CalculatorZoneLootEntry>> {
        let cache_key = Self::calculator_zone_loot_cache_key(lang, ref_id, zone_rgb_key);
        loop {
            if let Ok(cache) = self.calculator_zone_loot_cache.lock() {
                if let Some(cached) = cache.get(&cache_key) {
                    return Ok(cached.clone());
                }
            }

            let (inflight_lock, inflight_cvar) = &*self.calculator_zone_loot_inflight;
            let mut inflight = inflight_lock
                .lock()
                .expect("calculator zone loot inflight lock poisoned");
            if !inflight.contains(&cache_key) {
                inflight.insert(cache_key.clone());
                drop(inflight);
                break;
            }
            inflight = inflight_cvar
                .wait(inflight)
                .expect("calculator zone loot inflight wait poisoned");
            drop(inflight);
        }

        let result = self.query_calculator_zone_loot(lang, ref_id, zone_rgb_key);

        let (inflight_lock, inflight_cvar) = &*self.calculator_zone_loot_inflight;
        let mut inflight = inflight_lock
            .lock()
            .expect("calculator zone loot inflight lock poisoned");
        inflight.remove(&cache_key);
        inflight_cvar.notify_all();
        drop(inflight);

        let rows = result?;
        if let Ok(mut cache) = self.calculator_zone_loot_cache.lock() {
            cache.insert(cache_key, rows.clone());
        }
        Ok(rows)
    }

    fn query_calculator_zone_loot(
        &self,
        lang: FishLang,
        ref_id: Option<&str>,
        zone_rgb_key: &str,
    ) -> AppResult<Vec<CalculatorZoneLootEntry>> {
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
        let slot_rows = zone_rows
            .into_iter()
            .filter_map(|(slot_idx, item_main_group_key)| {
                let slot_idx = u8::try_from(slot_idx).ok()?;
                (item_main_group_key > 0).then_some((slot_idx, item_main_group_key))
            })
            .collect::<Vec<_>>();
        if slot_rows.is_empty() {
            return Ok(Vec::new());
        }

        let main_group_id_csv = slot_rows
            .iter()
            .map(|(_, item_main_group_key)| item_main_group_key.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let main_group_query = format!(
            "SELECT \
                CAST(ItemMainGroupKey AS SIGNED), \
                CAST(SelectRate0 AS SIGNED), CAST(ItemSubGroupKey0 AS SIGNED), \
                CAST(SelectRate1 AS SIGNED), CAST(ItemSubGroupKey1 AS SIGNED), \
                CAST(SelectRate2 AS SIGNED), CAST(ItemSubGroupKey2 AS SIGNED), \
                CAST(SelectRate3 AS SIGNED), CAST(ItemSubGroupKey3 AS SIGNED) \
             FROM item_main_group_table{as_of} \
             WHERE ItemMainGroupKey IN ({main_group_id_csv})"
        );
        let main_group_rows: Vec<(
            i64,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
        )> = conn.query(main_group_query).map_err(db_unavailable)?;

        let mut subgroup_options = HashMap::<i64, Vec<(i64, i64)>>::new();
        for (
            item_main_group_key,
            select_rate0,
            subgroup0,
            select_rate1,
            subgroup1,
            select_rate2,
            subgroup2,
            select_rate3,
            subgroup3,
        ) in main_group_rows
        {
            for (select_rate, subgroup_key) in [
                (select_rate0, subgroup0),
                (select_rate1, subgroup1),
                (select_rate2, subgroup2),
                (select_rate3, subgroup3),
            ] {
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
        if subgroup_ids.is_empty() {
            return Ok(Vec::new());
        }

        let subgroup_id_csv = subgroup_ids
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let subgroup_query = format!(
            "SELECT \
                CAST(ItemSubGroupKey AS SIGNED), \
                CAST(ItemKey AS SIGNED), \
                CAST(SelectRate_0 AS SIGNED), \
                CAST(SelectRate_1 AS SIGNED), \
                CAST(SelectRate_2 AS SIGNED) \
             FROM item_sub_group_table{as_of} \
             WHERE ItemSubGroupKey IN ({subgroup_id_csv})"
        );
        let subgroup_rows: Vec<(i64, i64, Option<i64>, Option<i64>, Option<i64>)> =
            conn.query(subgroup_query).map_err(db_unavailable)?;

        let mut subgroup_variants = HashMap::<i64, Vec<(i32, i64)>>::new();
        for (item_sub_group_key, item_key, select_rate_0, select_rate_1, select_rate_2) in
            subgroup_rows
        {
            let Ok(item_id) = i32::try_from(item_key) else {
                continue;
            };
            if item_id <= 0 {
                continue;
            }
            let select_rate = [select_rate_0, select_rate_1, select_rate_2]
                .into_iter()
                .flatten()
                .find(|select_rate| *select_rate > 0);
            if let Some(select_rate) = select_rate {
                subgroup_variants
                    .entry(item_sub_group_key)
                    .or_default()
                    .push((item_id, select_rate));
            }
        }

        let mut aggregate_weights = HashMap::<(u8, i32), f64>::new();
        for (slot_idx, item_main_group_key) in &slot_rows {
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
                }
            }
        }
        let mut slot_subgroup_select_rate = HashMap::<(u8, i64), i64>::new();
        let mut slot_option_count = HashMap::<u8, usize>::new();
        for (slot_idx, item_main_group_key) in &slot_rows {
            let Some(options) = subgroup_options.get(item_main_group_key) else {
                continue;
            };
            slot_option_count.insert(*slot_idx, options.len());
            for (select_rate, subgroup_key) in options {
                slot_subgroup_select_rate.insert((*slot_idx, *subgroup_key), *select_rate);
            }
        }

        let mut community_presence_by_item = HashMap::<i32, (String, u32)>::new();
        let mut community_guess_by_key = HashMap::<(u8, i32), CommunityPrizeGuessMeta>::new();
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
            if source_id == COMMUNITY_PRIZE_GUESS_SOURCE_ID {
                let Some(notes) = normalize_optional_string(notes) else {
                    continue;
                };
                let Some(guess) = parse_community_prize_guess_notes(&notes) else {
                    continue;
                };
                community_guess_by_key.insert((guess.slot_idx, item_id), guess);
                continue;
            }
            let claim_count = u32::try_from(claim_count.max(0)).unwrap_or(u32::MAX);
            community_presence_by_item.insert(
                item_id,
                (support_status.trim().to_ascii_lowercase(), claim_count),
            );
        }

        let effective_weights = apply_community_guess_weights(
            &aggregate_weights,
            &community_guess_by_key,
            &slot_subgroup_select_rate,
            &slot_option_count,
        );
        if effective_weights.is_empty() {
            return Ok(Vec::new());
        }

        let mut slot_totals = HashMap::<u8, f64>::new();
        for ((slot_idx, _), weight) in &effective_weights {
            *slot_totals.entry(*slot_idx).or_default() += *weight;
        }
        let mut slot_membership_count = HashMap::<i32, usize>::new();
        for (_, item_id) in effective_weights.keys() {
            *slot_membership_count.entry(*item_id).or_default() += 1;
        }

        let item_id_csv = effective_weights
            .keys()
            .map(|(_, item_id)| item_id.to_string())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(",");
        let item_name_expr = match lang {
            FishLang::En => {
                "COALESCE(NULLIF(TRIM(en.`text`), ''), NULLIF(TRIM(it.`ItemName`), ''))"
            }
            FishLang::Ko => {
                "COALESCE(NULLIF(TRIM(it.`ItemName`), ''), NULLIF(TRIM(en.`text`), ''))"
            }
        };
        let item_query = format!(
            "SELECT \
                CAST(it.`Index` AS SIGNED), \
                {item_name_expr} AS item_name, \
                NULLIF(TRIM(it.`IconImageFile`), '') AS icon_file, \
                it.`GradeType`, \
                it.`OriginalPrice`, \
                CASE WHEN ft.item_key IS NULL THEN 0 ELSE 1 END AS is_fish \
             FROM item_table{as_of} it \
             LEFT JOIN fish_table{as_of} ft ON ft.item_key = it.`Index` \
             LEFT JOIN languagedata_en{as_of} en ON en.`id` = it.`Index` \
               AND en.`format` = 'A' \
               AND COALESCE(en.`unk`, '') = '' \
               AND NULLIF(TRIM(en.`text`), '') IS NOT NULL \
             WHERE it.`Index` IN ({item_id_csv})"
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
            let icon = normalize_optional_string(icon_file)
                .and_then(|value| parse_fish_icon_asset_id(&value))
                .map(calculator_loot_item_icon_path)
                .or_else(|| Some(calculator_loot_item_icon_path(item_id)));
            let (grade, _, _is_prize) = fish_grade_from_db(grade_type);
            let vendor_price = parse_positive_i64(original_price);
            item_meta.insert(item_id, (name, icon, grade, vendor_price, is_fish > 0));
        }

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
                    });
                }
                if let Some(guess) = community_guess_by_key.get(&(slot_idx, item_id)) {
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
                    });
                }
                if let Some((support_status, claim_count)) =
                    community_presence_by_item.get(&item_id)
                {
                    let scope = if slot_membership_count
                        .get(&item_id)
                        .copied()
                        .unwrap_or_default()
                        <= 1
                    {
                        "group_inferred"
                    } else {
                        "zone"
                    };
                    evidence.push(CalculatorZoneLootEvidence {
                        source_family: "community".to_string(),
                        claim_kind: "presence".to_string(),
                        scope: scope.to_string(),
                        rate: None,
                        normalized_rate: None,
                        status: Some(support_status.clone()),
                        claim_count: Some(*claim_count),
                    });
                }
                Some(CalculatorZoneLootEntry {
                    slot_idx,
                    item_id,
                    name,
                    icon,
                    vendor_price,
                    grade,
                    is_fish,
                    within_group_rate,
                    evidence,
                })
            })
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| {
            left.slot_idx
                .cmp(&right.slot_idx)
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

    use super::{
        apply_community_guess_weights, parse_community_prize_guess_notes, CommunityPrizeGuessMeta,
    };

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
}
