use std::collections::{HashMap, HashSet};

use mysql::prelude::Queryable;

use crate::error::AppResult;
use crate::store::validate_dolt_ref;

use super::catalog::item_grade_from_db;
use super::util::{db_unavailable, is_missing_table, normalize_optional_string};
use super::DoltMySqlStore;

#[derive(Debug, Clone, Default)]
pub(super) struct ItemSourceMetadata {
    pub(super) name_ko: Option<String>,
    pub(super) name_en: Option<String>,
    pub(super) normalized_name_ko: Option<String>,
    pub(super) item_type: Option<String>,
    pub(super) durability: Option<i32>,
    pub(super) grade: Option<String>,
    pub(super) icon_path: Option<String>,
    pub(super) icon_id: Option<i32>,
}

pub(super) fn normalize_source_owned_item_name(name: &str) -> String {
    name.replace("[의상] ", "")
        .replace("[이벤트] ", "")
        .replace("의 낚시 배낭", " 낚시 배낭")
        .replace("의 낚시복", " 낚시복")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn item_type_from_equip_type(equip_type: Option<&str>) -> Option<&'static str> {
    match equip_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse::<i32>().ok())
    {
        Some(22) => Some("outfit"),
        Some(44) => Some("rod"),
        Some(46) => Some("chair"),
        Some(59) => Some("float"),
        Some(111) => Some("backpack"),
        _ => None,
    }
}

fn normalized_item_name_expr() -> &'static str {
    "TRIM(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(COALESCE(it.`ItemName`, ''), '[의상] ', ''), '[이벤트] ', ''), '의 낚시 배낭', ' 낚시 배낭'), '의 낚시복', ' 낚시복'), '  ', ' '))"
}

fn dedupe_non_empty_values(values: &[String]) -> Vec<String> {
    let mut out = values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    out.sort_unstable();
    out.dedup();
    out
}

fn quote_sql_string_list(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("'{}'", value.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(",")
}

impl DoltMySqlStore {
    pub(super) fn query_item_table_metadata(
        &self,
        ref_id: Option<&str>,
        item_ids: &[i32],
    ) -> AppResult<HashMap<i32, ItemSourceMetadata>> {
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
                CAST(it.`Index` AS SIGNED), \
                it.`ItemName`, \
                l.item_name_en, \
                it.`EquipType`, \
                it.`IconImageFile`, \
                it.`GradeType`, \
                CASE \
                    WHEN TRIM(COALESCE(it.`EnduranceLimit`, '')) REGEXP '^[0-9]+$' \
                    THEN CAST(it.`EnduranceLimit` AS SIGNED) \
                    ELSE NULL \
                END AS endurance_limit \
             FROM item_table{as_of} it \
             LEFT JOIN ( \
                 SELECT \
                     CAST(l.`id` AS SIGNED) AS item_id, \
                     MAX(NULLIF(TRIM(l.`text`), '')) AS item_name_en \
                 FROM languagedata_en{as_of} l \
                 WHERE l.`id` IN ({id_list}) \
                   AND l.`format` = 'A' \
                   AND l.`unk` IS NULL \
                 GROUP BY CAST(l.`id` AS SIGNED) \
             ) l \
               ON l.item_id = CAST(it.`Index` AS SIGNED) \
             WHERE it.`Index` IN ({id_list})"
        );

        self.query_item_table_metadata_from_query(query)
    }

    pub(super) fn query_item_table_metadata_by_names(
        &self,
        ref_id: Option<&str>,
        exact_names: &[String],
        normalized_names: &[String],
    ) -> AppResult<HashMap<i32, ItemSourceMetadata>> {
        let exact_names = dedupe_non_empty_values(exact_names);
        let normalized_names = dedupe_non_empty_values(normalized_names);
        if exact_names.is_empty() && normalized_names.is_empty() {
            return Ok(HashMap::new());
        }
        let item_ids =
            self.query_item_table_matching_ids_by_names(ref_id, &exact_names, &normalized_names)?;
        self.query_item_table_metadata(ref_id, &item_ids)
    }

    fn query_item_table_matching_ids_by_names(
        &self,
        ref_id: Option<&str>,
        exact_names: &[String],
        normalized_names: &[String],
    ) -> AppResult<Vec<i32>> {
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };
        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let mut item_ids = HashSet::<i32>::new();

        if !exact_names.is_empty() {
            let exact_query = format!(
                "SELECT CAST(it.`Index` AS SIGNED) \
                 FROM item_table{as_of} it \
                 WHERE it.`ItemName` IN ({})",
                quote_sql_string_list(exact_names)
            );
            let rows: Vec<i64> = match conn.query(exact_query) {
                Ok(rows) => rows,
                Err(err) if is_missing_table(&err, "item_table") => return Ok(Vec::new()),
                Err(err) => return Err(db_unavailable(err)),
            };
            item_ids.extend(
                rows.into_iter()
                    .filter_map(|item_id| i32::try_from(item_id).ok()),
            );
        }

        if !normalized_names.is_empty() {
            let normalized_query = format!(
                "SELECT CAST(it.`Index` AS SIGNED) \
                 FROM item_table{as_of} it \
                 WHERE {} IN ({})",
                normalized_item_name_expr(),
                quote_sql_string_list(normalized_names)
            );
            let rows: Vec<i64> = match conn.query(normalized_query) {
                Ok(rows) => rows,
                Err(err) if is_missing_table(&err, "item_table") => return Ok(Vec::new()),
                Err(err) => return Err(db_unavailable(err)),
            };
            item_ids.extend(
                rows.into_iter()
                    .filter_map(|item_id| i32::try_from(item_id).ok()),
            );
        }

        let mut item_ids = item_ids.into_iter().collect::<Vec<_>>();
        item_ids.sort_unstable();
        Ok(item_ids)
    }

    fn query_item_table_metadata_from_query(
        &self,
        query: String,
    ) -> AppResult<HashMap<i32, ItemSourceMetadata>> {
        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<(
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<i64>,
        )> = match conn.query(query) {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "item_table") => return Ok(HashMap::new()),
            Err(err) => return Err(db_unavailable(err)),
        };

        Ok(rows
            .into_iter()
            .filter_map(
                |(item_id, name_ko, name_en, equip_type, icon_file, grade_type, durability)| {
                    let item_id = i32::try_from(item_id).ok()?;
                    let (grade, _, _) = item_grade_from_db(grade_type);
                    let icon_file = normalize_optional_string(icon_file);
                    Some((
                        item_id,
                        ItemSourceMetadata {
                            normalized_name_ko: normalize_optional_string(
                                name_ko.as_ref().cloned(),
                            )
                            .map(|value| normalize_source_owned_item_name(&value)),
                            name_ko: normalize_optional_string(name_ko),
                            name_en: normalize_optional_string(name_en),
                            item_type: item_type_from_equip_type(equip_type.as_deref())
                                .map(str::to_string),
                            durability: durability.and_then(|value| i32::try_from(value).ok()),
                            grade,
                            icon_path: icon_file.as_deref().and_then(
                                fishystuff_core::fish_icons::fish_icon_path_from_asset_file,
                            ),
                            icon_id: icon_file
                                .as_deref()
                                .and_then(fishystuff_core::fish_icons::parse_fish_icon_asset_id),
                        },
                    ))
                },
            )
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::{item_type_from_equip_type, normalize_source_owned_item_name};

    #[test]
    fn item_type_from_equip_type_maps_supported_gear_categories() {
        assert_eq!(item_type_from_equip_type(Some("22")), Some("outfit"));
        assert_eq!(item_type_from_equip_type(Some("44")), Some("rod"));
        assert_eq!(item_type_from_equip_type(Some("46")), Some("chair"));
        assert_eq!(item_type_from_equip_type(Some("59")), Some("float"));
        assert_eq!(item_type_from_equip_type(Some("111")), Some("backpack"));
        assert_eq!(item_type_from_equip_type(Some("15")), None);
        assert_eq!(item_type_from_equip_type(None), None);
    }

    #[test]
    fn normalize_source_owned_item_name_strips_known_costume_prefixes() {
        assert_eq!(
            normalize_source_owned_item_name("[의상] 전문의 낚시복"),
            "전문 낚시복"
        );
        assert_eq!(
            normalize_source_owned_item_name("[이벤트] 장인의 낚시 배낭"),
            "장인 낚시 배낭"
        );
    }
}
