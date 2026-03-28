use std::collections::HashMap;

use mysql::prelude::Queryable;
use mysql::Row;

use crate::error::AppResult;
use crate::store::validate_dolt_ref;

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
    pub(super) name_en: Option<String>,
    pub(super) normalized_name_ko: Option<String>,
    pub(super) durability: Option<i32>,
    pub(super) icon_id: Option<i32>,
}

#[derive(Debug, Clone)]
struct CalculatorEnchantEffectEntryRow {
    item_type: String,
    item_name_ko: String,
    normalized_item_name_ko: String,
    enchant_level: i32,
    durability: Option<i32>,
    afr: Option<f32>,
    bonus_rare: Option<f32>,
    bonus_big: Option<f32>,
    drr: Option<f32>,
    exp_fish: Option<f32>,
    exp_life: Option<f32>,
}

pub(super) struct CalculatorCatalogSourceData {
    pub(super) legacy_rows: Vec<CalculatorItemDbRow>,
    pub(super) item_source_metadata: HashMap<i32, CalculatorItemSourceMetadata>,
    pub(super) source_backed_rows: Vec<CalculatorSourceBackedItemRow>,
}

pub(super) struct CalculatorSourceBackedItemRow {
    pub(super) source_key: String,
    pub(super) source_kind: String,
    pub(super) item_id: Option<i32>,
    pub(super) item_type: String,
    pub(super) source_name_en: Option<String>,
    pub(super) source_name_ko: Option<String>,
    pub(super) item_icon_file: Option<String>,
    pub(super) icon_id: Option<i32>,
    pub(super) durability: Option<i32>,
    pub(super) fish_multiplier: Option<f32>,
    pub(super) effect_description_ko: Option<String>,
    pub(super) afr: Option<f32>,
    pub(super) bonus_rare: Option<f32>,
    pub(super) bonus_big: Option<f32>,
    pub(super) drr: Option<f32>,
    pub(super) exp_fish: Option<f32>,
    pub(super) exp_life: Option<f32>,
}

fn normalize_source_owned_item_name(name: &str) -> String {
    name.replace("[의상] ", "")
        .replace("[이벤트] ", "")
        .replace("의 낚시 배낭", " 낚시 배낭")
        .replace("의 낚시복", " 낚시복")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn max_opt_i32(left: Option<i32>, right: Option<i32>) -> Option<i32> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn max_opt_f32(left: Option<f32>, right: Option<f32>) -> Option<f32> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

impl DoltMySqlStore {
    fn query_consumable_effect_line_rows(
        &self,
        ref_id: Option<&str>,
    ) -> AppResult<Vec<(i32, String)>> {
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };
        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let keyword_predicate = |column: &str| {
            [
                "낚시",
                "자동 낚시",
                "희귀 어종",
                "대형 어종",
                "낚시 경험치",
                "생활 경험치",
                "낚시 숙련도",
                "생활 숙련도",
                "내구도 소모 감소 저항",
            ]
            .into_iter()
            .map(|keyword| format!("COALESCE({column}, '') LIKE '%{keyword}%'"))
            .collect::<Vec<_>>()
            .join(" OR ")
        };
        let quote_list = |values: &[String]| {
            values
                .iter()
                .map(|value| format!("'{}'", value.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join(",")
        };

        let skill_desc_query = format!(
            "SELECT \
                TRIM(COALESCE(`SkillNo`, '')) AS skill_no, \
                `Desc` \
             FROM skilltype_table_new{as_of} \
             WHERE ({})",
            keyword_predicate("`Desc`")
        );
        let skill_desc_rows: Vec<(String, Option<String>)> = match conn.query(skill_desc_query) {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "skilltype_table_new") => Vec::new(),
            Err(err) => return Err(db_unavailable(err)),
        };
        let skill_descriptions = skill_desc_rows
            .into_iter()
            .filter_map(|(skill_no, description)| {
                Some((skill_no, normalize_optional_string(description)?))
            })
            .collect::<HashMap<_, _>>();

        let buff_desc_query = format!(
            "SELECT \
                TRIM(COALESCE(`Index`, '')) AS buff_id, \
                `Description` \
             FROM buff_table{as_of} \
             WHERE ({})",
            keyword_predicate("`Description`")
        );
        let buff_desc_rows: Vec<(String, Option<String>)> = match conn.query(buff_desc_query) {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "buff_table") => Vec::new(),
            Err(err) => return Err(db_unavailable(err)),
        };
        let buff_descriptions = buff_desc_rows
            .into_iter()
            .filter_map(|(buff_id, description)| {
                Some((buff_id, normalize_optional_string(description)?))
            })
            .collect::<HashMap<_, _>>();

        if skill_descriptions.is_empty() && buff_descriptions.is_empty() {
            return Ok(Vec::new());
        }

        let skill_ids = skill_descriptions.keys().cloned().collect::<Vec<_>>();
        let buff_ids = buff_descriptions.keys().cloned().collect::<Vec<_>>();
        let skill_filter = if skill_ids.is_empty() {
            String::from("FALSE")
        } else {
            format!(
                "TRIM(COALESCE(`SkillNo`, '')) IN ({})",
                quote_list(&skill_ids)
            )
        };
        let buff_filter = if buff_ids.is_empty() {
            String::from("FALSE")
        } else {
            format!(
                "TRIM(COALESCE(`Buff0`, '')) IN ({ids}) \
                 OR TRIM(COALESCE(`Buff1`, '')) IN ({ids}) \
                 OR TRIM(COALESCE(`Buff2`, '')) IN ({ids}) \
                 OR TRIM(COALESCE(`Buff3`, '')) IN ({ids}) \
                 OR TRIM(COALESCE(`Buff4`, '')) IN ({ids}) \
                 OR TRIM(COALESCE(`Buff5`, '')) IN ({ids}) \
                 OR TRIM(COALESCE(`Buff6`, '')) IN ({ids}) \
                 OR TRIM(COALESCE(`Buff7`, '')) IN ({ids}) \
                 OR TRIM(COALESCE(`Buff8`, '')) IN ({ids}) \
                 OR TRIM(COALESCE(`Buff9`, '')) IN ({ids})",
                ids = quote_list(&buff_ids)
            )
        };

        let skill_rows_query = format!(
            "SELECT \
                TRIM(COALESCE(`SkillNo`, '')) AS skill_no, \
                TRIM(COALESCE(`Buff0`, '')) AS buff0, \
                TRIM(COALESCE(`Buff1`, '')) AS buff1, \
                TRIM(COALESCE(`Buff2`, '')) AS buff2, \
                TRIM(COALESCE(`Buff3`, '')) AS buff3, \
                TRIM(COALESCE(`Buff4`, '')) AS buff4, \
                TRIM(COALESCE(`Buff5`, '')) AS buff5, \
                TRIM(COALESCE(`Buff6`, '')) AS buff6, \
                TRIM(COALESCE(`Buff7`, '')) AS buff7, \
                TRIM(COALESCE(`Buff8`, '')) AS buff8, \
                TRIM(COALESCE(`Buff9`, '')) AS buff9 \
             FROM skill_table_new{as_of} \
             WHERE ({skill_filter}) OR ({buff_filter})"
        );
        let skill_rows: Vec<(
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        )> = match conn.query(skill_rows_query) {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "skill_table_new") => Vec::new(),
            Err(err) => return Err(db_unavailable(err)),
        };

        let mut relevant_skill_ids = Vec::<String>::new();
        let mut skill_buffs = HashMap::<String, Vec<String>>::new();
        for (skill_no, buff0, buff1, buff2, buff3, buff4, buff5, buff6, buff7, buff8, buff9) in
            skill_rows
        {
            if !relevant_skill_ids
                .iter()
                .any(|existing| existing == &skill_no)
            {
                relevant_skill_ids.push(skill_no.clone());
            }
            let entry = skill_buffs.entry(skill_no).or_default();
            for buff_id in [
                buff0, buff1, buff2, buff3, buff4, buff5, buff6, buff7, buff8, buff9,
            ]
            .into_iter()
            .filter_map(normalize_optional_string)
            {
                if !entry.iter().any(|existing| existing == &buff_id) {
                    entry.push(buff_id);
                }
            }
        }

        if relevant_skill_ids.is_empty() {
            return Ok(Vec::new());
        }

        let quoted_skill_ids = quote_list(&relevant_skill_ids);
        let item_query = format!(
            "SELECT \
                CAST(`Index` AS SIGNED) AS item_id, \
                TRIM(COALESCE(`SkillNo`, '')) AS skill_no, \
                TRIM(COALESCE(`SubSkillNo`, '')) AS sub_skill_no \
             FROM item_table{as_of} \
             WHERE TRIM(COALESCE(`SkillNo`, '')) IN ({quoted_skill_ids}) \
                OR TRIM(COALESCE(`SubSkillNo`, '')) IN ({quoted_skill_ids})"
        );
        let item_rows: Vec<(i64, Option<String>, Option<String>)> = match conn.query(item_query) {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "item_table") => Vec::new(),
            Err(err) => return Err(db_unavailable(err)),
        };

        let mut effect_lines = Vec::new();
        for (item_id, skill_no, sub_skill_no) in item_rows {
            let Ok(item_id) = i32::try_from(item_id) else {
                continue;
            };
            for candidate_skill in [skill_no, sub_skill_no]
                .into_iter()
                .filter_map(normalize_optional_string)
            {
                if let Some(description) = skill_descriptions.get(&candidate_skill) {
                    effect_lines.push((item_id, description.clone()));
                }
                if let Some(buff_ids) = skill_buffs.get(&candidate_skill) {
                    for buff_id in buff_ids {
                        if let Some(description) = buff_descriptions.get(buff_id) {
                            effect_lines.push((item_id, description.clone()));
                        }
                    }
                }
            }
        }

        Ok(effect_lines)
    }

    fn query_lightstone_source_rows(
        &self,
        ref_id: Option<&str>,
    ) -> AppResult<
        Vec<(
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<f32>,
            Option<f32>,
            Option<f32>,
            Option<f32>,
            Option<f32>,
            Option<f32>,
        )>,
    > {
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };
        let query = format!(
            "SELECT \
                source_key, \
                set_name_ko, \
                source_name_en, \
                skill_icon_file, \
                effect_description_ko, \
                afr, \
                bonus_rare, \
                bonus_big, \
                drr, \
                exp_fish, \
                exp_life \
             FROM calculator_lightstone_effect_sources{as_of}"
        );

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<(
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<f32>,
            Option<f32>,
            Option<f32>,
            Option<f32>,
            Option<f32>,
            Option<f32>,
        )> = match conn.query(query) {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "calculator_lightstone_effect_sources") => {
                return Ok(Vec::new());
            }
            Err(err) => return Err(db_unavailable(err)),
        };

        Ok(rows)
    }

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
        let raw_item_table_query = format!(
            "SELECT \
                CAST(it.`Index` AS SIGNED), \
                it.`ItemName`, \
                MAX( \
                    CASE \
                        WHEN COALESCE(l.`format`, '') = 'A' \
                         AND COALESCE(l.`unk`, '') = '' \
                        THEN NULLIF(TRIM(l.`text`), '') \
                        ELSE NULL \
                    END \
                ) AS item_name_en, \
                it.`IconImageFile`, \
                CASE \
                    WHEN TRIM(COALESCE(it.`EnduranceLimit`, '')) REGEXP '^[0-9]+$' \
                    THEN CAST(it.`EnduranceLimit` AS SIGNED) \
                    ELSE NULL \
                END AS endurance_limit \
             FROM item_table{as_of} it \
             LEFT JOIN languagedata_en{as_of} l \
               ON l.`id` = CAST(it.`Index` AS SIGNED) \
             WHERE it.`Index` IN ({id_list}) \
             GROUP BY CAST(it.`Index` AS SIGNED), \
                      it.`ItemName`, \
                      it.`IconImageFile`, \
                      CASE \
                          WHEN TRIM(COALESCE(it.`EnduranceLimit`, '')) REGEXP '^[0-9]+$' \
                          THEN CAST(it.`EnduranceLimit` AS SIGNED) \
                          ELSE NULL \
                      END"
        );

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<(
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<i64>,
        )> = match conn.query(raw_item_table_query) {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "item_table") => return Ok(HashMap::new()),
            Err(err) => return Err(db_unavailable(err)),
        };
        let mut out = HashMap::new();
        for (item_id, name_ko, name_en, icon_file, durability) in rows {
            let Ok(item_id) = i32::try_from(item_id) else {
                continue;
            };
            out.insert(
                item_id,
                CalculatorItemSourceMetadata {
                    normalized_name_ko: normalize_optional_string(name_ko.as_ref().cloned())
                        .map(|value| normalize_source_owned_item_name(&value)),
                    name_ko: normalize_optional_string(name_ko),
                    name_en: normalize_optional_string(name_en),
                    durability: durability.and_then(|value| i32::try_from(value).ok()),
                    icon_id: normalize_optional_string(icon_file).and_then(|value| {
                        fishystuff_core::fish_icons::parse_fish_icon_asset_id(&value)
                    }),
                },
            );
        }
        Ok(out)
    }

    fn query_calculator_item_table_metadata_by_names(
        &self,
        ref_id: Option<&str>,
        exact_names: &[String],
        normalized_names: &[String],
    ) -> AppResult<HashMap<i32, CalculatorItemSourceMetadata>> {
        if exact_names.is_empty() && normalized_names.is_empty() {
            return Ok(HashMap::new());
        }
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };
        let quote_list = |values: &[String]| {
            values
                .iter()
                .map(|value| format!("'{}'", value.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join(",")
        };
        let normalized_name_expr = "TRIM(REPLACE(REPLACE(REPLACE(REPLACE(REPLACE(COALESCE(it.`ItemName`, ''), '[의상] ', ''), '[이벤트] ', ''), '의 낚시 배낭', ' 낚시 배낭'), '의 낚시복', ' 낚시복'), '  ', ' '))";
        let exact_filter = if exact_names.is_empty() {
            String::from("FALSE")
        } else {
            format!(
                "NULLIF(TRIM(it.`ItemName`), '') IN ({})",
                quote_list(exact_names)
            )
        };
        let normalized_filter = if normalized_names.is_empty() {
            String::from("FALSE")
        } else {
            format!(
                "{normalized_name_expr} IN ({})",
                quote_list(normalized_names)
            )
        };
        let query = format!(
            "SELECT \
                CAST(it.`Index` AS SIGNED), \
                it.`ItemName`, \
                MAX( \
                    CASE \
                        WHEN COALESCE(l.`format`, '') = 'A' \
                         AND COALESCE(l.`unk`, '') = '' \
                        THEN NULLIF(TRIM(l.`text`), '') \
                        ELSE NULL \
                    END \
                ) AS item_name_en, \
                it.`IconImageFile`, \
                CASE \
                    WHEN TRIM(COALESCE(it.`EnduranceLimit`, '')) REGEXP '^[0-9]+$' \
                    THEN CAST(it.`EnduranceLimit` AS SIGNED) \
                    ELSE NULL \
                END AS endurance_limit \
             FROM item_table{as_of} it \
             LEFT JOIN languagedata_en{as_of} l \
               ON l.`id` = CAST(it.`Index` AS SIGNED) \
             WHERE ({exact_filter}) OR ({normalized_filter}) \
             GROUP BY CAST(it.`Index` AS SIGNED), \
                      it.`ItemName`, \
                      it.`IconImageFile`, \
                      CASE \
                          WHEN TRIM(COALESCE(it.`EnduranceLimit`, '')) REGEXP '^[0-9]+$' \
                          THEN CAST(it.`EnduranceLimit` AS SIGNED) \
                          ELSE NULL \
                      END"
        );

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<(
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<i64>,
        )> = match conn.query(query) {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "item_table") => return Ok(HashMap::new()),
            Err(err) => return Err(db_unavailable(err)),
        };

        let mut out = HashMap::new();
        for (item_id, name_ko, name_en, icon_file, durability) in rows {
            let Ok(item_id) = i32::try_from(item_id) else {
                continue;
            };
            out.insert(
                item_id,
                CalculatorItemSourceMetadata {
                    normalized_name_ko: normalize_optional_string(name_ko.as_ref().cloned())
                        .map(|value| normalize_source_owned_item_name(&value)),
                    name_ko: normalize_optional_string(name_ko),
                    name_en: normalize_optional_string(name_en),
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
        excluded_item_ids: &[i32],
        excluded_effect_names: &[String],
    ) -> AppResult<Vec<CalculatorItemDbRow>> {
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };
        let mut where_clauses = Vec::new();
        if !excluded_item_ids.is_empty() {
            let id_list = excluded_item_ids
                .iter()
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(",");
            where_clauses.push(format!("(id IS NULL OR id NOT IN ({id_list}))"));
        }
        if !excluded_effect_names.is_empty() {
            let escaped_names = excluded_effect_names
                .iter()
                .map(|name| format!("'{}'", name.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join(",");
            where_clauses.push(format!(
                "NOT (type = 'lightstone_set' AND name IN ({escaped_names}))"
            ));
        }
        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_clauses.join(" AND "))
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
             FROM items{as_of}{where_sql}"
        );

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        conn.query(query).map_err(db_unavailable)
    }

    fn query_text_source_backed_item_rows(
        &self,
        ref_id: Option<&str>,
        legacy_rows: &[CalculatorItemDbRow],
        item_source_metadata: &HashMap<i32, CalculatorItemSourceMetadata>,
    ) -> AppResult<Vec<CalculatorSourceBackedItemRow>> {
        let legacy_rows_by_item_id = legacy_rows
            .iter()
            .filter_map(|row| Some((row.10?, row)))
            .collect::<HashMap<_, _>>();

        let mut effect_lines_by_item_id = HashMap::<i32, Vec<String>>::new();
        for (item_id, effect_line) in self.query_consumable_effect_line_rows(ref_id)? {
            let lines = effect_lines_by_item_id.entry(item_id).or_default();
            if !lines.iter().any(|existing| existing == &effect_line) {
                lines.push(effect_line);
            }
        }

        let mut source_backed_rows = effect_lines_by_item_id
            .into_iter()
            .filter_map(|(item_id, effect_lines)| {
                let legacy_row = legacy_rows_by_item_id.get(&item_id)?;
                let item_type = normalize_optional_string(legacy_row.1.clone())
                    .unwrap_or_else(|| "buff".into());
                let source_meta = item_source_metadata.get(&item_id);
                Some(CalculatorSourceBackedItemRow {
                    source_key: format!("item:{item_id}"),
                    source_kind: "item".to_string(),
                    item_id: Some(item_id),
                    item_type,
                    source_name_en: source_meta.and_then(|meta| meta.name_en.clone()),
                    source_name_ko: source_meta.and_then(|meta| meta.name_ko.clone()),
                    item_icon_file: None,
                    icon_id: source_meta.and_then(|meta| meta.icon_id).or(legacy_row.11),
                    durability: source_meta
                        .and_then(|meta| meta.durability)
                        .or(legacy_row.5),
                    fish_multiplier: legacy_row.7,
                    effect_description_ko: Some(effect_lines.join("\n")),
                    afr: None,
                    bonus_rare: None,
                    bonus_big: None,
                    drr: None,
                    exp_fish: None,
                    exp_life: None,
                })
            })
            .collect::<Vec<_>>();

        source_backed_rows.extend(self.query_lightstone_source_rows(ref_id)?.into_iter().map(
            |(
                source_key,
                source_name_ko,
                source_name_en,
                item_icon_file,
                effect_description_ko,
                afr,
                bonus_rare,
                bonus_big,
                drr,
                exp_fish,
                exp_life,
            )| CalculatorSourceBackedItemRow {
                source_key,
                source_kind: "lightstone_set".to_string(),
                item_id: None,
                item_type: "lightstone_set".to_string(),
                source_name_en,
                source_name_ko,
                item_icon_file,
                icon_id: None,
                durability: None,
                fish_multiplier: None,
                effect_description_ko,
                afr,
                bonus_rare,
                bonus_big,
                drr,
                exp_fish,
                exp_life,
            },
        ));

        Ok(source_backed_rows)
    }

    fn query_source_owned_enchant_source_backed_item_rows(
        &self,
        ref_id: Option<&str>,
        legacy_rows: &[CalculatorItemDbRow],
    ) -> AppResult<Vec<CalculatorSourceBackedItemRow>> {
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };
        let fish_multiplier_by_item_id = legacy_rows
            .iter()
            .filter_map(|row| Some((row.10?, row.7)))
            .collect::<HashMap<_, _>>();

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let query = format!(
            "SELECT \
                item_type, \
                item_name_ko, \
                enchant_level, \
                durability, \
                afr, \
                bonus_rare, \
                bonus_big, \
                drr, \
                exp_fish \
             FROM calculator_enchant_item_effect_entries{as_of}"
        );
        let rows: Vec<Row> = match conn.query(query) {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "calculator_enchant_item_effect_entries") => {
                return Ok(Vec::new());
            }
            Err(err) => return Err(db_unavailable(err)),
        };

        let mut chosen_effects = HashMap::<(String, String), CalculatorEnchantEffectEntryRow>::new();
        for row in rows {
            let item_type = normalize_optional_string(row.get::<String, _>("item_type"))
                .unwrap_or_default();
            if !matches!(
                item_type.as_str(),
                "rod" | "float" | "chair" | "backpack" | "outfit"
            ) {
                continue;
            }
            let Some(item_name_ko) = normalize_optional_string(row.get::<String, _>("item_name_ko"))
            else {
                continue;
            };
            let effect_row = CalculatorEnchantEffectEntryRow {
                item_type: item_type.clone(),
                normalized_item_name_ko: normalize_source_owned_item_name(&item_name_ko),
                item_name_ko: item_name_ko.clone(),
                enchant_level: normalize_optional_string(row.get::<String, _>("enchant_level"))
                    .and_then(|value| value.parse::<i32>().ok())
                    .unwrap_or_default(),
                durability: row.get::<Option<i32>, _>("durability").flatten(),
                afr: row.get::<Option<f32>, _>("afr").flatten(),
                bonus_rare: row.get::<Option<f32>, _>("bonus_rare").flatten(),
                bonus_big: row.get::<Option<f32>, _>("bonus_big").flatten(),
                drr: row.get::<Option<f32>, _>("drr").flatten(),
                exp_fish: row.get::<Option<f32>, _>("exp_fish").flatten(),
                exp_life: None,
            };

            let key = (item_type, item_name_ko);
            match chosen_effects.get_mut(&key) {
                Some(existing) if effect_row.enchant_level > existing.enchant_level => {
                    *existing = effect_row;
                }
                Some(existing) if effect_row.enchant_level == existing.enchant_level => {
                    existing.durability = max_opt_i32(existing.durability, effect_row.durability);
                    existing.afr = max_opt_f32(existing.afr, effect_row.afr);
                    existing.bonus_rare =
                        max_opt_f32(existing.bonus_rare, effect_row.bonus_rare);
                    existing.bonus_big = max_opt_f32(existing.bonus_big, effect_row.bonus_big);
                    existing.drr = max_opt_f32(existing.drr, effect_row.drr);
                    existing.exp_fish = max_opt_f32(existing.exp_fish, effect_row.exp_fish);
                    existing.exp_life = max_opt_f32(existing.exp_life, effect_row.exp_life);
                }
                Some(_) => {}
                None => {
                    chosen_effects.insert(key, effect_row);
                }
            }
        }

        let chosen_effects = chosen_effects.into_values().collect::<Vec<_>>();
        if chosen_effects.is_empty() {
            return Ok(Vec::new());
        }

        let exact_names = chosen_effects
            .iter()
            .map(|row| row.item_name_ko.clone())
            .collect::<Vec<_>>();
        let normalized_names = chosen_effects
            .iter()
            .map(|row| row.normalized_item_name_ko.clone())
            .collect::<Vec<_>>();
        let metadata_candidates = self.query_calculator_item_table_metadata_by_names(
            ref_id,
            &exact_names,
            &normalized_names,
        )?;

        let mut exact_metadata_by_name = HashMap::<String, Vec<(i32, CalculatorItemSourceMetadata)>>::new();
        let mut normalized_metadata_by_name =
            HashMap::<String, Vec<(i32, CalculatorItemSourceMetadata)>>::new();
        for (item_id, metadata) in metadata_candidates {
            if let Some(name_ko) = metadata.name_ko.clone() {
                exact_metadata_by_name
                    .entry(name_ko)
                    .or_default()
                    .push((item_id, metadata.clone()));
            }
            if let Some(normalized_name_ko) = metadata.normalized_name_ko.clone() {
                normalized_metadata_by_name
                    .entry(normalized_name_ko)
                    .or_default()
                    .push((item_id, metadata));
            }
        }

        Ok(chosen_effects
            .into_iter()
            .filter_map(|row| {
                let exact_match = exact_metadata_by_name
                    .get(&row.item_name_ko)
                    .and_then(|matches| matches.iter().min_by_key(|(item_id, _)| *item_id))
                    .cloned();
                let resolved = exact_match.or_else(|| {
                    normalized_metadata_by_name
                        .get(&row.normalized_item_name_ko)
                        .filter(|matches| matches.len() == 1)
                        .and_then(|matches| matches.first().cloned())
                })?;
                let (item_id, metadata) = resolved;

                Some(CalculatorSourceBackedItemRow {
                    source_key: format!("item:{item_id}"),
                    source_kind: "item".to_string(),
                    item_id: Some(item_id),
                    item_type: row.item_type,
                    source_name_en: metadata.name_en,
                    source_name_ko: metadata.name_ko,
                    item_icon_file: None,
                    icon_id: metadata.icon_id,
                    durability: row.durability.or(metadata.durability),
                    fish_multiplier: fish_multiplier_by_item_id.get(&item_id).copied().flatten(),
                    effect_description_ko: None,
                    afr: row.afr,
                    bonus_rare: row.bonus_rare,
                    bonus_big: row.bonus_big,
                    drr: row.drr,
                    exp_fish: row.exp_fish,
                    exp_life: row.exp_life,
                })
            })
            .collect())
    }

    pub(super) fn query_calculator_catalog_source_data(
        &self,
        ref_id: Option<&str>,
    ) -> AppResult<CalculatorCatalogSourceData> {
        let all_legacy_rows = self.query_legacy_calculator_item_rows(ref_id, &[], &[])?;
        let all_item_ids = all_legacy_rows
            .iter()
            .filter_map(|row| row.10)
            .collect::<Vec<_>>();
        let item_source_metadata =
            self.query_calculator_item_table_metadata(ref_id, &all_item_ids)?;
        let mut source_backed_rows = self.query_text_source_backed_item_rows(
            ref_id,
            &all_legacy_rows,
            &item_source_metadata,
        )?;
        source_backed_rows.extend(
            self.query_source_owned_enchant_source_backed_item_rows(ref_id, &all_legacy_rows)?,
        );

        let excluded_item_ids = source_backed_rows
            .iter()
            .filter_map(|row| (row.source_kind == "item").then_some(row.item_id).flatten())
            .collect::<Vec<_>>();
        let excluded_item_ids = excluded_item_ids
            .into_iter()
            .collect::<std::collections::HashSet<_>>();
        let has_source_lightstones = source_backed_rows
            .iter()
            .any(|row| row.source_kind == "lightstone_set");
        let legacy_rows = all_legacy_rows
            .into_iter()
            .filter(|row| {
                let keep_item = row
                    .10
                    .map(|item_id| !excluded_item_ids.contains(&item_id))
                    .unwrap_or(true);
                let keep_effect = match normalize_optional_string(row.1.clone()) {
                    Some(item_type) if has_source_lightstones && item_type == "lightstone_set" => {
                        false
                    }
                    _ => true,
                };
                keep_item && keep_effect
            })
            .collect::<Vec<_>>();

        Ok(CalculatorCatalogSourceData {
            legacy_rows,
            item_source_metadata,
            source_backed_rows,
        })
    }
}
