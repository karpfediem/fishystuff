use std::collections::HashMap;

use fishystuff_api::ids::Rgb;
use fishystuff_api::models::calculator::{
    CalculatorMasteryPrizeRateEntry, CalculatorZoneGroupRateEntry,
};
use mysql::prelude::Queryable;

use crate::error::AppResult;
use crate::store::validate_dolt_ref;

use super::util::db_unavailable;
use super::DoltMySqlStore;

impl DoltMySqlStore {
    #[tracing::instrument(name = "store.calculator_catalog.query.mastery_prize_curve", skip_all)]
    pub(super) fn query_calculator_mastery_prize_curve(
        &self,
        ref_id: Option<&str>,
    ) -> AppResult<Vec<CalculatorMasteryPrizeRateEntry>> {
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };
        let query = format!(
            "SELECT \
                CAST(fishing_mastery AS SIGNED), \
                CAST(high_drop_rate_raw AS SIGNED), \
                CAST(high_drop_rate AS DOUBLE) \
             FROM calculator_fishing_mastery_high_drop_curve{as_of} \
             ORDER BY fishing_mastery"
        );

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<(i64, i64, f64)> = conn.query(query).map_err(db_unavailable)?;

        Ok(rows
            .into_iter()
            .filter_map(|(fishing_mastery, high_drop_rate_raw, high_drop_rate)| {
                Some(CalculatorMasteryPrizeRateEntry {
                    fishing_mastery: i32::try_from(fishing_mastery).ok()?,
                    high_drop_rate_raw: i32::try_from(high_drop_rate_raw).ok()?,
                    high_drop_rate: high_drop_rate as f32,
                })
            })
            .collect())
    }

    #[tracing::instrument(name = "store.calculator_catalog.query.zone_group_rates", skip_all)]
    pub(super) fn query_calculator_zone_group_rates(
        &self,
        ref_id: Option<&str>,
    ) -> AppResult<Vec<CalculatorZoneGroupRateEntry>> {
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };
        let query = format!(
            "SELECT \
                CAST(R AS SIGNED), \
                CAST(G AS SIGNED), \
                CAST(B AS SIGNED), \
                CAST(slot_idx AS SIGNED), \
                COALESCE(CAST(drop_rate AS SIGNED), 0), \
                CAST(item_main_group_key AS SIGNED) \
             FROM fishing_zone_slots{as_of} \
             ORDER BY R, G, B, slot_idx"
        );

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<(i64, i64, i64, i64, i64, Option<i64>)> =
            conn.query(query).map_err(db_unavailable)?;

        let mut grouped = HashMap::<String, CalculatorZoneGroupRateEntry>::new();
        for (r, g, b, slot_idx, drop_rate, item_main_group_key) in rows {
            let Ok(r) = u8::try_from(r) else {
                continue;
            };
            let Ok(g) = u8::try_from(g) else {
                continue;
            };
            let Ok(b) = u8::try_from(b) else {
                continue;
            };
            let rgb = Rgb { r, g, b };
            let zone_rgb_key = rgb.key().0;
            let entry = grouped.entry(zone_rgb_key.clone()).or_insert_with(|| {
                CalculatorZoneGroupRateEntry {
                    zone_rgb_key,
                    ..CalculatorZoneGroupRateEntry::default()
                }
            });
            let drop_rate = i32::try_from(drop_rate).unwrap_or(0).max(0);
            match slot_idx {
                1 => {
                    entry.prize_main_group_key = item_main_group_key
                        .and_then(|value| i32::try_from(value).ok())
                        .filter(|value| *value > 0);
                }
                2 => entry.rare_rate_raw = drop_rate,
                3 => entry.high_quality_rate_raw = drop_rate,
                4 => entry.general_rate_raw = drop_rate,
                5 => entry.trash_rate_raw = drop_rate,
                _ => {}
            }
        }

        let mut entries = grouped.into_values().collect::<Vec<_>>();
        entries.sort_by(|left, right| left.zone_rgb_key.cmp(&right.zone_rgb_key));
        Ok(entries)
    }
}
