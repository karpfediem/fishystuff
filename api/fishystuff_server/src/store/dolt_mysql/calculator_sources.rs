use std::collections::HashMap;

use mysql::prelude::Queryable;

use crate::error::AppResult;
use crate::store::validate_dolt_ref;

use super::calculator_effects::{
    CalculatorEffectSourceData, CalculatorItemEffectValues, CalculatorLightstoneSourceEntry,
};
use super::util::{db_unavailable, is_missing_table, normalize_optional_string};
use super::DoltMySqlStore;

pub(super) type CalculatorItemDbRow = (
    Option<String>,
    Option<String>,
    Option<f32>,
    Option<f32>,
    Option<f32>,
    Option<i32>,
    Option<f32>,
    Option<f32>,
    Option<f32>,
    Option<f32>,
    Option<i32>,
    Option<i32>,
);

#[derive(Debug, Clone, Default)]
pub(super) struct CalculatorItemSourceMetadata {
    pub(super) name_ko: Option<String>,
    pub(super) durability: Option<i32>,
    pub(super) icon_id: Option<i32>,
}

pub(super) struct CalculatorCatalogSourceData {
    pub(super) legacy_rows: Vec<CalculatorItemDbRow>,
    pub(super) item_source_metadata: HashMap<i32, CalculatorItemSourceMetadata>,
    pub(super) lightstone_sources: HashMap<String, CalculatorLightstoneSourceEntry>,
    pub(super) consumable_overrides: HashMap<i32, CalculatorItemEffectValues>,
}

impl DoltMySqlStore {
    fn query_calculator_item_table_metadata(
        &self,
        ref_id: Option<&str>,
        item_ids: &[i32],
    ) -> AppResult<HashMap<i32, CalculatorItemSourceMetadata>> {
        if item_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };
        let id_list = item_ids
            .iter()
            .map(i32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let query = format!(
            "SELECT \
                item_id, \
                item_name_ko, \
                item_icon_file, \
                endurance_limit \
             FROM calculator_item_source_metadata{as_of} \
             WHERE item_id IN ({id_list})"
        );
        let raw_item_table_query = format!(
            "SELECT \
                CAST(it.`Index` AS SIGNED), \
                it.`ItemName`, \
                it.`IconImageFile`, \
                CASE \
                    WHEN TRIM(COALESCE(it.`EnduranceLimit`, '')) REGEXP '^[0-9]+$' \
                    THEN CAST(it.`EnduranceLimit` AS SIGNED) \
                    ELSE NULL \
                END AS endurance_limit \
             FROM item_table{as_of} it \
             WHERE it.`Index` IN ({id_list})"
        );

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<(i64, Option<String>, Option<String>, Option<i64>)> = match conn.query(query)
        {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "calculator_item_source_metadata") => {
                match conn.query(raw_item_table_query) {
                    Ok(rows) => rows,
                    Err(err) if is_missing_table(&err, "item_table") => return Ok(HashMap::new()),
                    Err(err) => return Err(db_unavailable(err)),
                }
            }
            Err(err) if is_missing_table(&err, "item_table") => return Ok(HashMap::new()),
            Err(err) => return Err(db_unavailable(err)),
        };
        let mut out = HashMap::new();
        for (item_id, name, icon_file, durability) in rows {
            let Ok(item_id) = i32::try_from(item_id) else {
                continue;
            };
            out.insert(
                item_id,
                CalculatorItemSourceMetadata {
                    name_ko: normalize_optional_string(name),
                    durability: durability.and_then(|value| i32::try_from(value).ok()),
                    icon_id: normalize_optional_string(icon_file).and_then(|value| {
                        fishystuff_core::fish_icons::parse_fish_icon_asset_id(&value)
                    }),
                },
            );
        }
        Ok(out)
    }

    fn query_legacy_calculator_item_rows(
        &self,
        ref_id: Option<&str>,
    ) -> AppResult<Vec<CalculatorItemDbRow>> {
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };
        let query = format!(
            "SELECT \
                name, \
                type, \
                afr, \
                bonus_rare, \
                bonus_big, \
                durability, \
                drr, \
                fish_multiplier, \
                exp_fish, \
                exp_life, \
                id, \
                icon_id \
             FROM items{as_of}"
        );

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        conn.query(query).map_err(db_unavailable)
    }

    pub(super) fn query_calculator_catalog_source_data(
        &self,
        ref_id: Option<&str>,
    ) -> AppResult<CalculatorCatalogSourceData> {
        let legacy_rows = self.query_legacy_calculator_item_rows(ref_id)?;
        let item_ids = legacy_rows
            .iter()
            .filter_map(|row| row.10)
            .collect::<Vec<_>>();
        let item_source_metadata = self.query_calculator_item_table_metadata(ref_id, &item_ids)?;
        let override_item_ids = legacy_rows
            .iter()
            .filter_map(|row| {
                let item_type = normalize_optional_string(row.1.clone())?;
                if matches!(item_type.as_str(), "food" | "buff") {
                    row.10
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let CalculatorEffectSourceData {
            consumable_overrides,
            lightstone_sources,
        } = self.query_calculator_effect_source_data(ref_id, &override_item_ids)?;

        Ok(CalculatorCatalogSourceData {
            legacy_rows,
            item_source_metadata,
            lightstone_sources,
            consumable_overrides,
        })
    }
}
