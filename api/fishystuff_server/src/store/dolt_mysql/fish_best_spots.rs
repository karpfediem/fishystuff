use std::collections::{BTreeSet, HashMap};

use fishystuff_api::error::ApiErrorCode;
use fishystuff_api::ids::Rgb;
use fishystuff_api::models::fish::{
    CommunityFishZoneSupportEntry, CommunityFishZoneSupportResponse, FishBestSpotEntry,
    FishBestSpotsResponse,
};
use mysql::prelude::Queryable;

use crate::error::AppResult;
use crate::store::{validate_dolt_ref, DataLang};

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

fn support_status_implies_presence(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "confirmed" | "guessed"
    )
}

impl DoltMySqlStore {
    fn fish_best_spots_index_cache_key(lang: &DataLang, ref_id: Option<&str>) -> String {
        let lang = lang.code();
        match ref_id {
            Some(ref_id) => format!("{lang}:{ref_id}"),
            None => format!("{lang}:head"),
        }
    }

    fn community_fish_zone_support_cache_key(ref_id: Option<&str>) -> String {
        match ref_id {
            Some(ref_id) => ref_id.to_string(),
            None => "head".to_string(),
        }
    }

    pub(super) fn query_community_fish_zone_support_cached(
        &self,
        ref_id: Option<&str>,
    ) -> AppResult<CommunityFishZoneSupportResponse> {
        let cache_key = Self::community_fish_zone_support_cache_key(ref_id);
        loop {
            if let Ok(cache) = self.community_fish_zone_support_cache.lock() {
                if let Some(cached) = cache.get(&cache_key) {
                    return Ok(cached.clone());
                }
            }

            let (inflight_lock, inflight_cvar) = &*self.community_fish_zone_support_inflight;
            let mut inflight = inflight_lock
                .lock()
                .expect("community fish zone support inflight lock poisoned");
            if !inflight.contains(&cache_key) {
                inflight.insert(cache_key.clone());
                drop(inflight);
                break;
            }
            inflight = inflight_cvar
                .wait(inflight)
                .expect("community fish zone support inflight wait poisoned");
            drop(inflight);
        }

        let result = self.query_community_fish_zone_support(ref_id);

        let (inflight_lock, inflight_cvar) = &*self.community_fish_zone_support_inflight;
        let mut inflight = inflight_lock
            .lock()
            .expect("community fish zone support inflight lock poisoned");
        inflight.remove(&cache_key);
        inflight_cvar.notify_all();
        drop(inflight);

        let response = result?;

        if let Ok(mut cache) = self.community_fish_zone_support_cache.lock() {
            cache.insert(cache_key, response.clone());
        }

        Ok(response)
    }

    pub(super) fn query_community_fish_zone_support(
        &self,
        ref_id: Option<&str>,
    ) -> AppResult<CommunityFishZoneSupportResponse> {
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };

        let query = format!(
            "SELECT CAST(item_id AS SIGNED), CAST(zone_rgb AS UNSIGNED), support_status \
             FROM community_zone_fish_support{as_of} \
             ORDER BY item_id, zone_rgb"
        );

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<(i64, u64, String)> = match conn.exec(query, ()) {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "community_zone_fish_support") => Vec::new(),
            Err(err) => return Err(db_unavailable(err)),
        };

        let mut fish = Vec::<CommunityFishZoneSupportEntry>::new();
        for (item_id, zone_rgb, support_status) in rows {
            if !support_status_implies_presence(&support_status) {
                continue;
            }
            let Ok(item_id) = i32::try_from(item_id) else {
                continue;
            };
            let Ok(zone_rgb) = u32::try_from(zone_rgb) else {
                continue;
            };
            match fish.last_mut() {
                Some(current) if current.item_id == item_id => {
                    if current.zone_rgbs.last().copied() != Some(zone_rgb) {
                        current.zone_rgbs.push(zone_rgb);
                    }
                }
                _ => fish.push(CommunityFishZoneSupportEntry {
                    item_id,
                    zone_rgbs: vec![zone_rgb],
                }),
            }
        }

        Ok(CommunityFishZoneSupportResponse {
            revision: self
                .query_dolt_revision(ref_id)
                .unwrap_or_else(|| synthetic_community_fish_zone_support_revision(ref_id, &fish)),
            count: fish.len(),
            fish,
        })
    }

    fn fish_best_spots_cache_key(lang: &DataLang, ref_id: Option<&str>, item_id: i32) -> String {
        let lang = lang.code();
        match ref_id {
            Some(ref_id) => format!("{lang}:{ref_id}:{item_id}"),
            None => format!("{lang}:head:{item_id}"),
        }
    }

    pub(super) fn query_fish_best_spots_cached(
        &self,
        lang: DataLang,
        ref_id: Option<&str>,
        item_id: i32,
    ) -> AppResult<FishBestSpotsResponse> {
        self.validate_data_lang_available(&lang, ref_id)?;
        let cache_key = Self::fish_best_spots_cache_key(&lang, ref_id, item_id);
        loop {
            if let Ok(cache) = self.fish_best_spots_cache.lock() {
                if let Some(cached) = cache.get(&cache_key) {
                    return Ok(cached.clone());
                }
            }

            let (inflight_lock, inflight_cvar) = &*self.fish_best_spots_inflight;
            let mut inflight = inflight_lock
                .lock()
                .expect("fish best spots inflight lock poisoned");
            if !inflight.contains(&cache_key) {
                inflight.insert(cache_key.clone());
                drop(inflight);
                break;
            }
            inflight = inflight_cvar
                .wait(inflight)
                .expect("fish best spots inflight wait poisoned");
            drop(inflight);
        }

        let result = self.query_fish_best_spots(lang, ref_id, item_id);

        let (inflight_lock, inflight_cvar) = &*self.fish_best_spots_inflight;
        let mut inflight = inflight_lock
            .lock()
            .expect("fish best spots inflight lock poisoned");
        inflight.remove(&cache_key);
        inflight_cvar.notify_all();
        drop(inflight);

        let response = result?;

        if let Ok(mut cache) = self.fish_best_spots_cache.lock() {
            cache.insert(cache_key, response.clone());
        }

        Ok(response)
    }

    pub(super) fn query_fish_best_spots_index_cached(
        &self,
        lang: &DataLang,
        ref_id: Option<&str>,
    ) -> AppResult<HashMap<i32, Vec<FishBestSpotEntry>>> {
        let cache_key = Self::fish_best_spots_index_cache_key(lang, ref_id);
        loop {
            if let Ok(cache) = self.fish_best_spots_index_cache.lock() {
                if let Some(cached) = cache.get(&cache_key) {
                    return Ok(cached.clone());
                }
            }

            let (inflight_lock, inflight_cvar) = &*self.fish_best_spots_index_inflight;
            let mut inflight = inflight_lock
                .lock()
                .expect("fish best spots index inflight lock poisoned");
            if !inflight.contains(&cache_key) {
                inflight.insert(cache_key.clone());
                drop(inflight);
                break;
            }
            inflight = inflight_cvar
                .wait(inflight)
                .expect("fish best spots index inflight wait poisoned");
            drop(inflight);
        }

        let result = self.query_fish_best_spots_index(lang, ref_id);

        let (inflight_lock, inflight_cvar) = &*self.fish_best_spots_index_inflight;
        let mut inflight = inflight_lock
            .lock()
            .expect("fish best spots index inflight lock poisoned");
        inflight.remove(&cache_key);
        inflight_cvar.notify_all();
        drop(inflight);

        let response = result?;

        if let Ok(mut cache) = self.fish_best_spots_index_cache.lock() {
            cache.insert(cache_key, response.clone());
        }

        Ok(response)
    }

    fn query_fish_best_spots_index(
        &self,
        lang: &DataLang,
        ref_id: Option<&str>,
    ) -> AppResult<HashMap<i32, Vec<FishBestSpotEntry>>> {
        let zones = self
            .query_zones(ref_id)?
            .into_iter()
            .map(|zone| (zone.rgb_u32, zone))
            .collect::<HashMap<_, _>>();
        let mut spots_by_item = HashMap::<i32, HashMap<u32, FishBestSpotAccumulator>>::new();

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
            "SELECT DISTINCT CAST(sg.ItemKey AS SIGNED), CAST(fz.zone_rgb AS UNSIGNED), CAST(fz.slot_idx AS SIGNED) \
             FROM flockfish_zone_group_slots{as_of} fz \
             JOIN item_main_group_table{as_of} mg ON mg.ItemMainGroupKey = fz.item_main_group_key \
             JOIN item_sub_group_table{as_of} sg ON sg.ItemSubGroupKey = mg.ItemSubGroupKey0 \
             WHERE fz.resolution_status = 'numeric' \
             UNION \
             SELECT DISTINCT CAST(sg.ItemKey AS SIGNED), CAST(fz.zone_rgb AS UNSIGNED), CAST(fz.slot_idx AS SIGNED) \
             FROM flockfish_zone_group_slots{as_of} fz \
             JOIN item_main_group_table{as_of} mg ON mg.ItemMainGroupKey = fz.item_main_group_key \
             JOIN item_sub_group_table{as_of} sg ON sg.ItemSubGroupKey = mg.ItemSubGroupKey1 \
             WHERE fz.resolution_status = 'numeric' \
             UNION \
             SELECT DISTINCT CAST(sg.ItemKey AS SIGNED), CAST(fz.zone_rgb AS UNSIGNED), CAST(fz.slot_idx AS SIGNED) \
             FROM flockfish_zone_group_slots{as_of} fz \
             JOIN item_main_group_table{as_of} mg ON mg.ItemMainGroupKey = fz.item_main_group_key \
             JOIN item_sub_group_table{as_of} sg ON sg.ItemSubGroupKey = mg.ItemSubGroupKey2 \
             WHERE fz.resolution_status = 'numeric' \
             UNION \
             SELECT DISTINCT CAST(sg.ItemKey AS SIGNED), CAST(fz.zone_rgb AS UNSIGNED), CAST(fz.slot_idx AS SIGNED) \
             FROM flockfish_zone_group_slots{as_of} fz \
             JOIN item_main_group_table{as_of} mg ON mg.ItemMainGroupKey = fz.item_main_group_key \
             JOIN item_sub_group_table{as_of} sg ON sg.ItemSubGroupKey = mg.ItemSubGroupKey3 \
             WHERE fz.resolution_status = 'numeric'"
        );
        let db_rows: Vec<(i64, u64, i64)> = conn.query(db_query).map_err(db_unavailable)?;
        for (item_id, zone_rgb_u32, slot_idx) in db_rows {
            let Ok(item_id) = i32::try_from(item_id) else {
                continue;
            };
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
            let spot = spots_by_item
                .entry(item_id)
                .or_default()
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
                CAST(item_id AS SIGNED), \
                CAST(zone_rgb AS UNSIGNED), \
                COALESCE(source_id, ''), \
                COALESCE(notes, '') \
             FROM community_zone_fish_support{as_of}"
        );
        let community_rows: Vec<(i64, u64, String, String)> = match conn.query(community_query) {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "community_zone_fish_support") => Vec::new(),
            Err(err) => return Err(db_unavailable(err)),
        };
        for (item_id, zone_rgb_u32, source_id, notes) in community_rows {
            let Ok(item_id) = i32::try_from(item_id) else {
                continue;
            };
            let Ok(zone_rgb_u32) = u32::try_from(zone_rgb_u32) else {
                continue;
            };
            if !is_community_guess_source_id(&source_id) {
                continue;
            }
            let Some(meta) = parse_community_prize_guess_notes(&notes) else {
                continue;
            };
            let Some(group_label) = fish_group_label(meta.slot_idx) else {
                continue;
            };
            let (zone_rgb, zone_name) = zone_meta(zone_rgb_u32);
            let spot = spots_by_item
                .entry(item_id)
                .or_default()
                .entry(zone_rgb_u32)
                .or_insert_with(|| FishBestSpotAccumulator {
                    zone_rgb,
                    zone_name,
                    ..FishBestSpotAccumulator::default()
                });
            spot.community_groups.insert(group_label.to_string());
        }

        if let Some(map_version_id) = self.defaults.map_version_id.as_ref() {
            let layer_revision_id = match self.resolve_layer_revision_id(
                None,
                Some(map_version_id),
                Some(super::ZONE_MASK_LAYER_ID),
                None,
                None,
                0,
            ) {
                Ok(value) => Some(value),
                Err(err) if err.0.code == ApiErrorCode::NotFound => None,
                Err(err) => return Err(err),
            };
            if let Some(layer_revision_id) = layer_revision_id {
                if let Some(support_mode) =
                    self.resolve_event_zone_support_mode(&layer_revision_id)?
                {
                    let fish_identities = self.query_fish_identities(lang, ref_id)?;
                    let event_fish_identities =
                        Self::build_event_fish_identity_map(&fish_identities);
                    let ranking_query = match support_mode {
                        EventZoneSupportMode::Assignment => {
                            "SELECT CAST(e.fish_id AS SIGNED), CAST(z.zone_rgb AS UNSIGNED), COUNT(1) \
                             FROM events e \
                             JOIN event_zone_assignment z ON z.event_id = e.event_id AND z.layer_revision_id = ? \
                             WHERE e.water_ok = 1 \
                               AND e.source_kind = ? \
                             GROUP BY e.fish_id, z.zone_rgb"
                                .to_string()
                        }
                        EventZoneSupportMode::RingSupport => {
                            "SELECT CAST(e.fish_id AS SIGNED), CAST(ring.zone_rgb AS UNSIGNED), COUNT(DISTINCT e.event_id) \
                             FROM events e \
                             JOIN event_zone_ring_support ring ON ring.event_id = e.event_id AND ring.layer_revision_id = ? \
                             WHERE e.water_ok = 1 \
                               AND e.source_kind = ? \
                             GROUP BY e.fish_id, ring.zone_rgb"
                                .to_string()
                        }
                    };
                    let ranking_rows: Vec<(i64, u64, u64)> =
                        match conn.exec(ranking_query, (&layer_revision_id, SOURCE_KIND_RANKING)) {
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
                    for (fish_id, zone_rgb_u32, observation_count) in ranking_rows {
                        let Ok(fish_id) = i32::try_from(fish_id) else {
                            continue;
                        };
                        let Ok(zone_rgb_u32) = u32::try_from(zone_rgb_u32) else {
                            continue;
                        };
                        let item_id = event_fish_identities
                            .get(&fish_id)
                            .map(|(item_id, _, _)| *item_id)
                            .unwrap_or(fish_id);
                        let (zone_rgb, zone_name) = zone_meta(zone_rgb_u32);
                        let spot = spots_by_item
                            .entry(item_id)
                            .or_default()
                            .entry(zone_rgb_u32)
                            .or_insert_with(|| FishBestSpotAccumulator {
                                zone_rgb,
                                zone_name,
                                ..FishBestSpotAccumulator::default()
                            });
                        spot.has_ranking_presence = true;
                        let observation_count =
                            u32::try_from(observation_count).unwrap_or(u32::MAX);
                        spot.ranking_observation_count = spot
                            .ranking_observation_count
                            .saturating_add(observation_count);
                    }
                }
            }
        }

        let mut out = HashMap::new();
        for (item_id, item_spots) in spots_by_item {
            let mut spots = item_spots
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
            out.insert(item_id, spots);
        }

        Ok(out)
    }

    pub(super) fn query_fish_best_spots(
        &self,
        lang: DataLang,
        ref_id: Option<&str>,
        item_id: i32,
    ) -> AppResult<FishBestSpotsResponse> {
        self.validate_data_lang_available(&lang, ref_id)?;
        let spots = self
            .query_fish_best_spots_index_cached(&lang, ref_id)?
            .get(&item_id)
            .cloned()
            .unwrap_or_default();

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

fn synthetic_community_fish_zone_support_revision(
    source_revision: Option<&str>,
    fish: &[CommunityFishZoneSupportEntry],
) -> String {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source_revision.unwrap_or("").hash(&mut hasher);
    for entry in fish {
        entry.item_id.hash(&mut hasher);
        entry.zone_rgbs.hash(&mut hasher);
    }
    format!("fish-community-zone-support-{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::support_status_implies_presence;

    #[test]
    fn support_status_implies_presence_for_confirmed_and_guessed_rows() {
        assert!(support_status_implies_presence("confirmed"));
        assert!(support_status_implies_presence(" guessed "));
        assert!(!support_status_implies_presence("unconfirmed"));
        assert!(!support_status_implies_presence("data_incomplete"));
    }
}
