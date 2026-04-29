use std::collections::HashMap;

use fishystuff_api::ids::Rgb;
use mysql::prelude::Queryable;

use crate::error::AppResult;
use crate::store::validate_dolt_ref;

use super::super::{db_unavailable, is_missing_table, DoltMySqlStore};

#[derive(Debug, Clone)]
pub(super) struct LegacyZoneFishSupport {
    pub(super) item_id: i32,
    pub(super) encyclopedia_key: Option<i32>,
    pub(super) encyclopedia_id: Option<i32>,
    pub(super) fish_name: Option<String>,
    pub(super) aggregate_weight: f64,
    pub(super) lineages: Vec<LegacyZoneFishLineage>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct LegacyZoneFishLineage {
    pub(super) slot_idx: u8,
    pub(super) drop_rate: i64,
    pub(super) item_main_group_key: i64,
    pub(super) option_idx: u8,
    pub(super) select_rate: i64,
    pub(super) subgroup_key: i64,
}

#[derive(Debug, Clone, Default)]
pub(super) struct LegacyZoneSupportSummary {
    pub(super) evaluated: bool,
    pub(super) fish: Vec<LegacyZoneFishSupport>,
    pub(super) notes: Vec<String>,
}

impl DoltMySqlStore {
    pub(super) fn query_legacy_zone_support(
        &self,
        zone_rgb: Rgb,
        ref_id: Option<&str>,
        fish_names: &HashMap<i32, String>,
        fish_identities: &HashMap<i32, (i32, Option<i32>, Option<i32>)>,
    ) -> AppResult<LegacyZoneSupportSummary> {
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let zone_query = format!(
            "SELECT \
                DropRate1, DropID1, \
                DropRate2, DropID2, \
                DropRate3, DropID3, \
                DropRate4, DropID4, \
                DropRate5, DropID5 \
             FROM fishing_table{as_of} \
             WHERE R = ? AND G = ? AND B = ?"
        );

        let zone_row: Option<(
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
        )> = match conn.exec_first(zone_query, (zone_rgb.r, zone_rgb.g, zone_rgb.b)) {
            Ok(row) => row,
            Err(err) if is_missing_table(&err, "fishing_table") => {
                return Ok(LegacyZoneSupportSummary {
                    evaluated: false,
                    fish: Vec::new(),
                    notes: vec![
                        "legacy fishing tables are unavailable in the current runtime".to_string(),
                    ],
                });
            }
            Err(err) => return Err(db_unavailable(err)),
        };

        let Some(zone_row) = zone_row else {
            return Ok(LegacyZoneSupportSummary {
                evaluated: true,
                fish: Vec::new(),
                notes: vec!["legacy fishing tables have no RGB entry for this zone".to_string()],
            });
        };

        let slot_rows: Vec<(u8, i64, i64)> = [
            (1_u8, zone_row.0, zone_row.1),
            (2_u8, zone_row.2, zone_row.3),
            (3_u8, zone_row.4, zone_row.5),
            (4_u8, zone_row.6, zone_row.7),
            (5_u8, zone_row.8, zone_row.9),
        ]
        .into_iter()
        .filter_map(|(slot_idx, drop_rate, item_main_group_key)| {
            let drop_rate = drop_rate?;
            let item_main_group_key = item_main_group_key?;
            if drop_rate <= 0 || item_main_group_key <= 0 {
                return None;
            }
            Some((slot_idx, drop_rate, item_main_group_key))
        })
        .collect();

        if slot_rows.is_empty() {
            return Ok(LegacyZoneSupportSummary {
                evaluated: true,
                fish: Vec::new(),
                notes: vec![
                    "legacy fishing tables have no positive slot/group rows for this zone"
                        .to_string(),
                ],
            });
        }

        let main_group_id_csv = slot_rows
            .iter()
            .map(|(_, _, item_main_group_key)| item_main_group_key.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let main_group_query = format!(
            "SELECT \
                ItemMainGroupKey, \
                SelectRate0, ItemSubGroupKey0, \
                SelectRate1, ItemSubGroupKey1, \
                SelectRate2, ItemSubGroupKey2, \
                SelectRate3, ItemSubGroupKey3 \
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
        )> = match conn.query(main_group_query) {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "item_main_group_table") => {
                return Ok(LegacyZoneSupportSummary {
                    evaluated: false,
                    fish: Vec::new(),
                    notes: vec![
                        "legacy main-group tables are unavailable in the current runtime"
                            .to_string(),
                    ],
                });
            }
            Err(err) => return Err(db_unavailable(err)),
        };

        let mut subgroup_options: HashMap<i64, Vec<(u8, i64, i64)>> = HashMap::new();
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
            for (option_idx, select_rate, subgroup_key) in [
                (0_u8, select_rate0, subgroup0),
                (1_u8, select_rate1, subgroup1),
                (2_u8, select_rate2, subgroup2),
                (3_u8, select_rate3, subgroup3),
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
                    .push((option_idx, select_rate, subgroup_key));
            }
        }

        let subgroup_ids = subgroup_options
            .values()
            .flat_map(|options| options.iter().map(|(_, _, subgroup_key)| *subgroup_key))
            .collect::<Vec<_>>();
        if subgroup_ids.is_empty() {
            return Ok(LegacyZoneSupportSummary {
                evaluated: true,
                fish: Vec::new(),
                notes: vec![
                    "legacy fishing tables have zone slots for this RGB, but no positive subgroup options"
                        .to_string(),
                ],
            });
        }

        let subgroup_id_csv = subgroup_ids
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let subgroup_query = format!(
            "SELECT \
                ItemSubGroupKey, ItemKey, EnchantLevel, \
                SelectRate_0, SelectRate_1, SelectRate_2 \
             FROM item_sub_group_table{as_of} \
             WHERE ItemSubGroupKey IN ({subgroup_id_csv})"
        );
        let subgroup_rows: Vec<(i64, i64, i64, Option<i64>, Option<i64>, Option<i64>)> = match conn
            .query(subgroup_query)
        {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "item_sub_group_table") => {
                return Ok(LegacyZoneSupportSummary {
                    evaluated: false,
                    fish: Vec::new(),
                    notes: vec![
                        "legacy subgroup tables are unavailable in the current runtime".to_string(),
                    ],
                });
            }
            Err(err) => return Err(db_unavailable(err)),
        };

        let mut subgroup_variants: HashMap<i64, Vec<(i64, i64)>> = HashMap::new();
        for (
            item_sub_group_key,
            item_key,
            _enchant_level,
            select_rate_0,
            select_rate_1,
            select_rate_2,
        ) in subgroup_rows
        {
            if item_key <= 0 {
                continue;
            }
            for select_rate in [select_rate_0, select_rate_1, select_rate_2] {
                let Some(select_rate) = select_rate else {
                    continue;
                };
                if select_rate <= 0 {
                    continue;
                }
                subgroup_variants
                    .entry(item_sub_group_key)
                    .or_default()
                    .push((item_key, select_rate));
            }
        }

        if subgroup_variants.is_empty() {
            return Ok(LegacyZoneSupportSummary {
                evaluated: true,
                fish: Vec::new(),
                notes: vec![
                    "legacy fishing tables have subgroup references for this zone, but no positive item variants"
                        .to_string(),
                ],
            });
        }

        let mut fish_support: HashMap<i32, LegacyZoneFishSupport> = HashMap::new();
        for (slot_idx, drop_rate, item_main_group_key) in slot_rows {
            let Some(options) = subgroup_options.get(&item_main_group_key) else {
                continue;
            };
            for (option_idx, select_rate, item_sub_group_key) in options {
                let Some(variants) = subgroup_variants.get(item_sub_group_key) else {
                    continue;
                };
                let lineage = LegacyZoneFishLineage {
                    slot_idx,
                    drop_rate,
                    item_main_group_key,
                    option_idx: *option_idx,
                    select_rate: *select_rate,
                    subgroup_key: *item_sub_group_key,
                };
                for (item_key, variant_rate) in variants {
                    let Ok(raw_fish_id) = i32::try_from(*item_key) else {
                        continue;
                    };
                    let (item_id, encyclopedia_key, encyclopedia_id) = fish_identities
                        .get(&raw_fish_id)
                        .copied()
                        .unwrap_or((raw_fish_id, None, None));
                    let aggregate_weight = (drop_rate as f64 / 1_000_000.0)
                        * (*select_rate as f64 / 1_000_000.0)
                        * (*variant_rate as f64 / 1_000_000.0);
                    let fish_name = fish_names
                        .get(&item_id)
                        .cloned()
                        .or_else(|| fish_names.get(&raw_fish_id).cloned());
                    fish_support
                        .entry(item_id)
                        .and_modify(|entry| {
                            entry.aggregate_weight += aggregate_weight;
                            if !entry.lineages.contains(&lineage) {
                                entry.lineages.push(lineage.clone());
                            }
                            if entry.fish_name.is_none() {
                                entry.fish_name = fish_name.clone();
                            }
                            if entry.encyclopedia_key.is_none() {
                                entry.encyclopedia_key = encyclopedia_key;
                            }
                            if entry.encyclopedia_id.is_none() {
                                entry.encyclopedia_id = encyclopedia_id;
                            }
                        })
                        .or_insert(LegacyZoneFishSupport {
                            item_id,
                            encyclopedia_key,
                            encyclopedia_id,
                            fish_name,
                            aggregate_weight,
                            lineages: vec![lineage.clone()],
                        });
                }
            }
        }

        let mut fish = fish_support.into_values().collect::<Vec<_>>();
        fish.sort_by(|left, right| {
            right
                .aggregate_weight
                .total_cmp(&left.aggregate_weight)
                .then_with(|| left.item_id.cmp(&right.item_id))
        });

        let mut notes = vec![
            "legacy reference support is derived from fishing_table -> item_main_group_table -> item_sub_group_table"
                .to_string(),
            "legacy reference support is not freshness-aware and does not imply current drop rates"
                .to_string(),
        ];
        if fish.is_empty() {
            notes.push(
                "legacy fishing tables were evaluated, but they did not yield resolvable fish rows for this zone"
                    .to_string(),
            );
        }

        Ok(LegacyZoneSupportSummary {
            evaluated: true,
            fish,
            notes,
        })
    }
}
