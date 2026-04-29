use crate::error::AppResult;
use crate::store::validate_dolt_ref;
use mysql::prelude::Queryable;

use super::super::{db_unavailable, is_missing_table, DoltMySqlStore};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CommunitySupportStatus {
    Confirmed,
    Guessed,
    Unconfirmed,
    DataIncomplete,
}

#[derive(Debug, Clone)]
pub(super) struct CommunityZoneFishSupport {
    pub(super) item_id: i32,
    pub(super) fish_name: Option<String>,
    pub(super) status: CommunitySupportStatus,
    pub(super) claim_count: u32,
    pub(super) source_id: String,
    pub(super) slot_idx: Option<u8>,
    pub(super) item_main_group_key: Option<i64>,
    pub(super) subgroup_key: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct CommunityZoneSupportSummary {
    pub(super) evaluated: bool,
    pub(super) fish: Vec<CommunityZoneFishSupport>,
    pub(super) notes: Vec<String>,
}

fn parse_support_status(value: &str) -> CommunitySupportStatus {
    match value.trim().to_ascii_lowercase().as_str() {
        "confirmed" => CommunitySupportStatus::Confirmed,
        "guessed" => CommunitySupportStatus::Guessed,
        "data_incomplete" => CommunitySupportStatus::DataIncomplete,
        _ => CommunitySupportStatus::Unconfirmed,
    }
}

fn status_priority(status: CommunitySupportStatus) -> u8 {
    match status {
        CommunitySupportStatus::Confirmed => 3,
        CommunitySupportStatus::Guessed => 2,
        CommunitySupportStatus::Unconfirmed => 1,
        CommunitySupportStatus::DataIncomplete => 0,
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct CommunitySupportLineage {
    slot_idx: Option<u8>,
    item_main_group_key: Option<i64>,
    subgroup_key: Option<i64>,
}

fn parse_community_support_notes(notes: &str) -> CommunitySupportLineage {
    let mut lineage = CommunitySupportLineage::default();
    for part in notes.split(';') {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        match key.trim() {
            "slot_idx" => {
                lineage.slot_idx = value.trim().parse::<u8>().ok().filter(|value| *value > 0);
            }
            "item_main_group_key" | "main_group_key" => {
                lineage.item_main_group_key =
                    value.trim().parse::<i64>().ok().filter(|value| *value > 0);
            }
            "item_sub_group_key" | "subgroup_key" => {
                lineage.subgroup_key = value.trim().parse::<i64>().ok().filter(|value| *value > 0);
            }
            _ => {}
        }
    }
    lineage
}

impl DoltMySqlStore {
    pub(super) fn query_community_zone_support(
        &self,
        zone_rgb_u32: u32,
        ref_id: Option<&str>,
    ) -> AppResult<CommunityZoneSupportSummary> {
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };

        let query = format!(
            "SELECT source_id, item_id, fish_name, support_status, claim_count, notes \
             FROM community_zone_fish_support{as_of} \
             WHERE zone_rgb = ?"
        );

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<(String, i64, Option<String>, String, i64, Option<String>)> =
            match conn.exec(query, (zone_rgb_u32,)) {
                Ok(rows) => rows,
                Err(err) if is_missing_table(&err, "community_zone_fish_support") => {
                    return Ok(CommunityZoneSupportSummary {
                        evaluated: false,
                        fish: Vec::new(),
                        notes: vec![
                            "community support tables are unavailable in the current runtime"
                                .to_string(),
                        ],
                    });
                }
                Err(err) => return Err(db_unavailable(err)),
            };

        if rows.is_empty() {
            let total_rows: Option<i64> = match conn.exec_first(
                format!("SELECT COUNT(*) FROM community_zone_fish_support{as_of}"),
                (),
            ) {
                Ok(value) => value,
                Err(err) if is_missing_table(&err, "community_zone_fish_support") => {
                    return Ok(CommunityZoneSupportSummary {
                        evaluated: false,
                        fish: Vec::new(),
                        notes: vec![
                            "community support tables are unavailable in the current runtime"
                                .to_string(),
                        ],
                    });
                }
                Err(err) => return Err(db_unavailable(err)),
            };

            if total_rows.unwrap_or(0) == 0 {
                return Ok(CommunityZoneSupportSummary {
                    evaluated: false,
                    fish: Vec::new(),
                    notes: vec![
                        "community support table exists but has not been populated in the current runtime"
                            .to_string(),
                    ],
                });
            }

            return Ok(CommunityZoneSupportSummary {
                evaluated: true,
                fish: Vec::new(),
                notes: vec![
                    "community support tables have no fish rows for this zone RGB".to_string(),
                ],
            });
        }

        let mut fish = Vec::with_capacity(rows.len());
        for (source_id, item_id, fish_name, support_status, claim_count, notes) in rows {
            let Ok(item_id) = i32::try_from(item_id) else {
                continue;
            };
            let claim_count = u32::try_from(claim_count).unwrap_or(u32::MAX);
            let lineage = notes
                .as_deref()
                .map(parse_community_support_notes)
                .unwrap_or_default();
            fish.push(CommunityZoneFishSupport {
                item_id,
                fish_name,
                status: parse_support_status(&support_status),
                claim_count,
                source_id,
                slot_idx: lineage.slot_idx,
                item_main_group_key: lineage.item_main_group_key,
                subgroup_key: lineage.subgroup_key,
            });
        }

        fish.sort_by(|left, right| {
            status_priority(right.status)
                .cmp(&status_priority(left.status))
                .then_with(|| right.claim_count.cmp(&left.claim_count))
                .then_with(|| left.item_id.cmp(&right.item_id))
        });

        Ok(CommunityZoneSupportSummary {
            evaluated: true,
            fish,
            notes: vec![
                "community support is sourced from curated zone/fish workbook imports"
                    .to_string(),
                "community support is a curated hint layer and does not imply a measured catch rate"
                    .to_string(),
            ],
        })
    }
}
