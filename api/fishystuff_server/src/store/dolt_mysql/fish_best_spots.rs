use std::collections::{BTreeSet, HashMap};

use fishystuff_api::ids::Rgb;
use fishystuff_api::models::fish::{FishBestSpotEntry, FishBestSpotsResponse};
use mysql::prelude::Queryable;

use crate::error::AppResult;
use crate::store::{validate_dolt_ref, FishLang};

use super::util::{db_unavailable, is_missing_table, normalize_optional_string};
use super::{DoltMySqlStore, EventZoneSupportMode, SOURCE_KIND_RANKING};

const COMMUNITY_PRIZE_GUESS_SOURCE_ID: &str = "community_prize_fish_guesses_workbook";
const MANUAL_COMMUNITY_GUESS_SOURCE_ID: &str = "manual_community_zone_fish_guess";

#[derive(Debug, Clone, Copy)]
struct CommunityPrizeGuessMeta {
    slot_idx: u8,
}

#[derive(Debug, Clone, Default)]
struct FishBestSpotAccumulator {
    zone_rgb: String,
    zone_name: String,
    db_groups: BTreeSet<String>,
    community_groups: BTreeSet<String>,
    has_ranking_presence: bool,
    ranking_observation_count: u32,
}

fn fish_group_label(slot_idx: u8) -> Option<&'static str> {
    match slot_idx {
        1 => Some("Prize"),
        2 => Some("Rare"),
        3 => Some("High-Quality"),
        4 => Some("General"),
        5 => Some("Trash"),
        _ => None,
    }
}

fn fish_group_rank(label: &str) -> u8 {
    match label {
        "Prize" => 0,
        "Rare" => 1,
        "High-Quality" => 2,
        "General" => 3,
        "Trash" => 4,
        _ => u8::MAX,
    }
}

fn is_community_guess_source_id(source_id: &str) -> bool {
    matches!(
        source_id,
        COMMUNITY_PRIZE_GUESS_SOURCE_ID | MANUAL_COMMUNITY_GUESS_SOURCE_ID
    )
}

fn parse_community_prize_guess_notes(notes: &str) -> Option<CommunityPrizeGuessMeta> {
    for part in notes.split(';') {
        let (key, value) = part.split_once('=')?;
        if key.trim() == "slot_idx" {
            return value
                .trim()
                .parse::<u8>()
                .ok()
                .map(|slot_idx| CommunityPrizeGuessMeta { slot_idx });
        }
    }
    None
}

impl DoltMySqlStore {
    pub(super) fn query_fish_best_spots(
        &self,
        _lang: FishLang,
        ref_id: Option<&str>,
        item_id: i32,
    ) -> AppResult<FishBestSpotsResponse> {
        let zones = self
            .query_zones(ref_id)?
            .into_iter()
            .map(|zone| (zone.rgb_u32, zone))
            .collect::<HashMap<_, _>>();
        let mut spots = HashMap::<u32, FishBestSpotAccumulator>::new();

        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };

        let zone_meta = |zone_rgb_u32: u32| -> (String, String) {
            let zone = zones.get(&zone_rgb_u32);
            (
                Rgb::from_u32(zone_rgb_u32).key().to_string(),
                zone.and_then(|value| normalize_optional_string(value.name.clone()))
                    .unwrap_or_else(|| Rgb::from_u32(zone_rgb_u32).key().to_string()),
            )
        };

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;

        let db_query = format!(
            "SELECT DISTINCT zone_rgb, slot_idx FROM (\
                 SELECT CAST(fz.zone_rgb AS UNSIGNED) AS zone_rgb, CAST(fz.slot_idx AS SIGNED) AS slot_idx \
                 FROM flockfish_zone_group_slots{as_of} fz \
                 JOIN item_main_group_table{as_of} mg ON mg.ItemMainGroupKey = fz.item_main_group_key \
                 JOIN item_sub_group_table{as_of} sg ON sg.ItemSubGroupKey = mg.ItemSubGroupKey0 \
                 WHERE fz.resolution_status = 'numeric' AND sg.ItemKey = ? \
                 UNION \
                 SELECT CAST(fz.zone_rgb AS UNSIGNED) AS zone_rgb, CAST(fz.slot_idx AS SIGNED) AS slot_idx \
                 FROM flockfish_zone_group_slots{as_of} fz \
                 JOIN item_main_group_table{as_of} mg ON mg.ItemMainGroupKey = fz.item_main_group_key \
                 JOIN item_sub_group_table{as_of} sg ON sg.ItemSubGroupKey = mg.ItemSubGroupKey1 \
                 WHERE fz.resolution_status = 'numeric' AND sg.ItemKey = ? \
                 UNION \
                 SELECT CAST(fz.zone_rgb AS UNSIGNED) AS zone_rgb, CAST(fz.slot_idx AS SIGNED) AS slot_idx \
                 FROM flockfish_zone_group_slots{as_of} fz \
                 JOIN item_main_group_table{as_of} mg ON mg.ItemMainGroupKey = fz.item_main_group_key \
                 JOIN item_sub_group_table{as_of} sg ON sg.ItemSubGroupKey = mg.ItemSubGroupKey2 \
                 WHERE fz.resolution_status = 'numeric' AND sg.ItemKey = ? \
                 UNION \
                 SELECT CAST(fz.zone_rgb AS UNSIGNED) AS zone_rgb, CAST(fz.slot_idx AS SIGNED) AS slot_idx \
                 FROM flockfish_zone_group_slots{as_of} fz \
                 JOIN item_main_group_table{as_of} mg ON mg.ItemMainGroupKey = fz.item_main_group_key \
                 JOIN item_sub_group_table{as_of} sg ON sg.ItemSubGroupKey = mg.ItemSubGroupKey3 \
                 WHERE fz.resolution_status = 'numeric' AND sg.ItemKey = ? \
             ) matches \
             ORDER BY zone_rgb, slot_idx"
        );
        let db_rows: Vec<(u64, i64)> = conn
            .exec(db_query, (item_id, item_id, item_id, item_id))
            .map_err(db_unavailable)?;
        for (zone_rgb_u32, slot_idx) in db_rows {
            let Ok(zone_rgb_u32) = u32::try_from(zone_rgb_u32) else {
                continue;
            };
            let Ok(slot_idx) = u8::try_from(slot_idx) else {
                continue;
            };
            let Some(group_label) = fish_group_label(slot_idx) else {
                continue;
            };
            let (zone_rgb, zone_name) = zone_meta(zone_rgb_u32);
            let spot = spots
                .entry(zone_rgb_u32)
                .or_insert_with(|| FishBestSpotAccumulator {
                    zone_rgb,
                    zone_name,
                    ..FishBestSpotAccumulator::default()
                });
            spot.db_groups.insert(group_label.to_string());
        }

        let community_query = format!(
            "SELECT \
                CAST(zone_rgb AS UNSIGNED), \
                COALESCE(source_id, ''), \
                COALESCE(notes, '') \
             FROM community_zone_fish_support{as_of} \
             WHERE item_id = ?"
        );
        let community_rows: Vec<(u64, String, String)> =
            match conn.exec(community_query, (item_id,)) {
                Ok(rows) => rows,
                Err(err) if is_missing_table(&err, "community_zone_fish_support") => Vec::new(),
                Err(err) => return Err(db_unavailable(err)),
            };
        for (zone_rgb_u32, source_id, notes) in community_rows {
            let Ok(zone_rgb_u32) = u32::try_from(zone_rgb_u32) else {
                continue;
            };
            if !is_community_guess_source_id(&source_id) {
                continue;
            }
            let (zone_rgb, zone_name) = zone_meta(zone_rgb_u32);
            let spot = spots
                .entry(zone_rgb_u32)
                .or_insert_with(|| FishBestSpotAccumulator {
                    zone_rgb,
                    zone_name,
                    ..FishBestSpotAccumulator::default()
                });

            if let Some(meta) = parse_community_prize_guess_notes(&notes) {
                if let Some(group_label) = fish_group_label(meta.slot_idx) {
                    spot.community_groups.insert(group_label.to_string());
                }
            }
        }

        if let Some(layer_revision_id) = self
            .defaults
            .map_version_id
            .as_ref()
            .map(|id| id.0.as_str())
        {
            if let Some(support_mode) = self.resolve_event_zone_support_mode(layer_revision_id)? {
                let fish_identities = self.query_fish_identities(ref_id)?;
                let event_fish_ids = Self::build_event_fish_identity_map(&fish_identities)
                    .into_iter()
                    .filter_map(|(fish_id, (mapped_item_id, _, _))| {
                        (mapped_item_id == item_id).then_some(fish_id)
                    })
                    .chain(std::iter::once(item_id))
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();

                if !event_fish_ids.is_empty() {
                    let fish_id_csv = event_fish_ids
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(",");
                    let ranking_query = match support_mode {
                        EventZoneSupportMode::Assignment => format!(
                            "SELECT CAST(z.zone_rgb AS UNSIGNED), COUNT(1) \
                             FROM events e \
                             JOIN event_zone_assignment z ON z.event_id = e.event_id AND z.layer_revision_id = ? \
                             WHERE e.water_ok = 1 \
                               AND e.source_kind = ? \
                               AND e.fish_id IN ({fish_id_csv}) \
                             GROUP BY z.zone_rgb"
                        ),
                        EventZoneSupportMode::RingSupport => format!(
                            "SELECT CAST(ring.zone_rgb AS UNSIGNED), COUNT(DISTINCT e.event_id) \
                             FROM events e \
                             JOIN event_zone_ring_support ring ON ring.event_id = e.event_id AND ring.layer_revision_id = ? \
                             WHERE e.water_ok = 1 \
                               AND e.source_kind = ? \
                               AND e.fish_id IN ({fish_id_csv}) \
                             GROUP BY ring.zone_rgb"
                        ),
                    };
                    let ranking_rows: Vec<(u64, u64)> =
                        match conn.exec(ranking_query, (layer_revision_id, SOURCE_KIND_RANKING)) {
                            Ok(rows) => rows,
                            Err(err)
                                if support_mode == EventZoneSupportMode::Assignment
                                    && is_missing_table(&err, "event_zone_assignment") =>
                            {
                                Vec::new()
                            }
                            Err(err)
                                if support_mode == EventZoneSupportMode::RingSupport
                                    && is_missing_table(&err, "event_zone_ring_support") =>
                            {
                                Vec::new()
                            }
                            Err(err) => return Err(db_unavailable(err)),
                        };
                    for (zone_rgb_u32, observation_count) in ranking_rows {
                        let Ok(zone_rgb_u32) = u32::try_from(zone_rgb_u32) else {
                            continue;
                        };
                        let (zone_rgb, zone_name) = zone_meta(zone_rgb_u32);
                        let spot =
                            spots
                                .entry(zone_rgb_u32)
                                .or_insert_with(|| FishBestSpotAccumulator {
                                    zone_rgb,
                                    zone_name,
                                    ..FishBestSpotAccumulator::default()
                                });
                        spot.has_ranking_presence = true;
                        spot.ranking_observation_count =
                            u32::try_from(observation_count).unwrap_or(u32::MAX);
                    }
                }
            }
        }

        let mut spots = spots
            .into_values()
            .map(|spot| FishBestSpotEntry {
                zone_rgb: spot.zone_rgb,
                zone_name: spot.zone_name,
                db_groups: spot.db_groups.into_iter().collect(),
                community_groups: spot.community_groups.into_iter().collect(),
                has_ranking_presence: spot.has_ranking_presence,
                ranking_observation_count: (spot.ranking_observation_count > 0)
                    .then_some(spot.ranking_observation_count),
            })
            .collect::<Vec<_>>();

        spots.sort_by(|left, right| {
            let left_tier = if !left.db_groups.is_empty() {
                0
            } else if !left.community_groups.is_empty() {
                1
            } else {
                2
            };
            let right_tier = if !right.db_groups.is_empty() {
                0
            } else if !right.community_groups.is_empty() {
                1
            } else {
                2
            };
            let left_group_rank = left
                .db_groups
                .iter()
                .chain(left.community_groups.iter())
                .map(|group| fish_group_rank(group))
                .min()
                .unwrap_or(u8::MAX);
            let right_group_rank = right
                .db_groups
                .iter()
                .chain(right.community_groups.iter())
                .map(|group| fish_group_rank(group))
                .min()
                .unwrap_or(u8::MAX);
            left_tier
                .cmp(&right_tier)
                .then_with(|| left_group_rank.cmp(&right_group_rank))
                .then_with(|| {
                    right
                        .ranking_observation_count
                        .unwrap_or(0)
                        .cmp(&left.ranking_observation_count.unwrap_or(0))
                })
                .then_with(|| {
                    left.zone_name
                        .to_lowercase()
                        .cmp(&right.zone_name.to_lowercase())
                })
                .then_with(|| left.zone_rgb.cmp(&right.zone_rgb))
        });

        Ok(FishBestSpotsResponse {
            revision: self
                .query_dolt_revision(ref_id)
                .unwrap_or_else(|| "dolt:unknown".to_string()),
            item_id,
            count: spots.len(),
            spots,
        })
    }
}
