use std::collections::HashMap;

use mysql::prelude::Queryable;
use mysql::Row;

use crate::error::AppResult;
use crate::store::validate_dolt_ref;

use super::calculator_effects::normalized_effect_lines;
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
    item_drr: Option<f32>,
    exp_fish: Option<f32>,
    exp_life: Option<f32>,
}

pub(super) struct CalculatorCatalogSourceData {
    pub(super) legacy_rows: Vec<CalculatorItemDbRow>,
    pub(super) item_source_metadata: HashMap<i32, CalculatorItemSourceMetadata>,
    pub(super) source_backed_rows: Vec<CalculatorSourceBackedItemRow>,
}

#[derive(Debug, Clone)]
pub(super) struct CalculatorSourceBackedItemRow {
    pub(super) source_key: String,
    pub(super) source_kind: String,
    pub(super) item_id: Option<i32>,
    pub(super) item_type: String,
    pub(super) buff_category_key: Option<String>,
    pub(super) buff_category_id: Option<i32>,
    pub(super) buff_category_level: Option<i32>,
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
    pub(super) item_drr: Option<f32>,
    pub(super) exp_fish: Option<f32>,
    pub(super) exp_life: Option<f32>,
}

#[derive(Debug, Clone)]
struct CalculatorConsumableItemRow {
    item_id: i32,
    item_classify: Option<String>,
    skill_no: Option<String>,
    sub_skill_no: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct CalculatorBuffSourceMetadata {
    category_id: Option<i32>,
    category_level: Option<i32>,
}

#[derive(Debug, Clone)]
struct CalculatorBuffTextRow {
    text: String,
    has_description: bool,
}

fn parse_optional_i32(value: Option<String>) -> Option<i32> {
    normalize_optional_string(value).and_then(|value| value.parse::<i32>().ok())
}

fn buff_category_key(category_id: Option<i32>) -> Option<String> {
    category_id.map(|category_id| format!("buff-category:{category_id}"))
}

fn fallback_consumable_family_key(
    primary_skill_id: Option<&str>,
    primary_skill_counts: &HashMap<String, usize>,
) -> Option<String> {
    let skill_id = primary_skill_id?;
    (primary_skill_counts.get(skill_id).copied().unwrap_or(0) > 1)
        .then(|| format!("skill-family:{skill_id}"))
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

fn select_consumable_category_metadata(
    primary_skill_id: Option<&str>,
    fallback_skill_id: Option<&str>,
    buff_categories_by_skill: &HashMap<String, CalculatorBuffSourceMetadata>,
) -> CalculatorBuffSourceMetadata {
    if let Some(primary) = primary_skill_id
        .and_then(|skill_id| buff_categories_by_skill.get(skill_id))
        .filter(|metadata| metadata.category_id.is_some())
    {
        return primary.clone();
    }
    if let Some(fallback) = fallback_skill_id
        .and_then(|skill_id| buff_categories_by_skill.get(skill_id))
        .filter(|metadata| metadata.category_id.is_some())
    {
        return fallback.clone();
    }
    let categories = [primary_skill_id, fallback_skill_id]
        .into_iter()
        .flatten()
        .filter_map(|skill_id| buff_categories_by_skill.get(skill_id))
        .filter_map(|metadata| {
            metadata
                .category_id
                .map(|category_id| (category_id, metadata.category_level))
        })
        .collect::<Vec<_>>();
    let Some((category_id, category_level)) = categories
        .iter()
        .max_by_key(|(category_id, category_level)| (*category_level, -*category_id))
        .copied()
    else {
        return CalculatorBuffSourceMetadata::default();
    };
    CalculatorBuffSourceMetadata {
        category_id: Some(category_id),
        category_level,
    }
}

fn select_consumable_effect_texts(
    skill_id: &str,
    buff_ids: &[String],
    buff_text_rows: &HashMap<String, CalculatorBuffTextRow>,
    skill_descriptions: &HashMap<String, String>,
) -> Vec<String> {
    let buff_rows = buff_ids
        .iter()
        .filter_map(|buff_id| buff_text_rows.get(buff_id))
        .collect::<Vec<_>>();
    let composite_rows = buff_rows
        .iter()
        .filter(|row| row.has_description && normalized_effect_lines(&row.text).len() > 1)
        .map(|row| row.text.clone())
        .collect::<Vec<_>>();
    if !composite_rows.is_empty() {
        return composite_rows;
    }
    let leaf_rows = buff_rows
        .iter()
        .map(|row| row.text.clone())
        .collect::<Vec<_>>();
    if !leaf_rows.is_empty() {
        return leaf_rows;
    }
    skill_descriptions
        .get(skill_id)
        .cloned()
        .into_iter()
        .collect()
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
                `BuffName`, \
                `Description` \
             FROM buff_table{as_of} \
             WHERE ({}) OR ({})",
            keyword_predicate("`Description`"),
            keyword_predicate("`BuffName`")
        );
        let buff_desc_rows: Vec<(String, Option<String>, Option<String>)> =
            match conn.query(buff_desc_query) {
                Ok(rows) => rows,
                Err(err) if is_missing_table(&err, "buff_table") => Vec::new(),
                Err(err) => return Err(db_unavailable(err)),
            };
        let buff_text_rows = buff_desc_rows
            .into_iter()
            .filter_map(|(buff_id, buff_name, description)| {
                let normalized_description = normalize_optional_string(description);
                let normalized_buff_name = normalize_optional_string(buff_name);
                Some((
                    buff_id,
                    CalculatorBuffTextRow {
                        text: normalized_description.clone().or(normalized_buff_name)?,
                        has_description: normalized_description.is_some(),
                    },
                ))
            })
            .collect::<HashMap<_, _>>();

        if skill_descriptions.is_empty() && buff_text_rows.is_empty() {
            return Ok(Vec::new());
        }

        let skill_ids = skill_descriptions.keys().cloned().collect::<Vec<_>>();
        let buff_ids = buff_text_rows.keys().cloned().collect::<Vec<_>>();
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
                let selected_texts = skill_buffs
                    .get(&candidate_skill)
                    .map(|buff_ids| {
                        select_consumable_effect_texts(
                            &candidate_skill,
                            buff_ids,
                            &buff_text_rows,
                            &skill_descriptions,
                        )
                    })
                    .filter(|texts| !texts.is_empty())
                    .unwrap_or_else(|| {
                        skill_descriptions
                            .get(&candidate_skill)
                            .cloned()
                            .into_iter()
                            .collect()
                    });
                for text in selected_texts {
                    effect_lines.push((item_id, text));
                }
            }
        }

        Ok(effect_lines)
    }

    fn query_consumable_item_rows(
        &self,
        ref_id: Option<&str>,
        item_ids: &[i32],
    ) -> AppResult<Vec<CalculatorConsumableItemRow>> {
        if item_ids.is_empty() {
            return Ok(Vec::new());
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
                CAST(`Index` AS SIGNED) AS item_id, \
                `ItemClassify`, \
                TRIM(COALESCE(`SkillNo`, '')) AS skill_no, \
                TRIM(COALESCE(`SubSkillNo`, '')) AS sub_skill_no \
             FROM item_table{as_of} \
             WHERE `Index` IN ({id_list})"
        );

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<(i64, Option<String>, Option<String>, Option<String>)> =
            match conn.query(query) {
                Ok(rows) => rows,
                Err(err) if is_missing_table(&err, "item_table") => return Ok(Vec::new()),
                Err(err) => return Err(db_unavailable(err)),
            };

        Ok(rows
            .into_iter()
            .filter_map(|(item_id, item_classify, skill_no, sub_skill_no)| {
                let Ok(item_id) = i32::try_from(item_id) else {
                    return None;
                };
                Some(CalculatorConsumableItemRow {
                    item_id,
                    item_classify: normalize_optional_string(item_classify),
                    skill_no: normalize_optional_string(skill_no),
                    sub_skill_no: normalize_optional_string(sub_skill_no),
                })
            })
            .collect())
    }

    fn query_consumable_skill_buff_categories(
        &self,
        ref_id: Option<&str>,
        skill_ids: &[String],
    ) -> AppResult<HashMap<String, CalculatorBuffSourceMetadata>> {
        if skill_ids.is_empty() {
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
        let skill_id_list = quote_list(skill_ids);

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let skill_query = format!(
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
             WHERE TRIM(COALESCE(`SkillNo`, '')) IN ({skill_id_list})"
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
        )> = match conn.query(skill_query) {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "skill_table_new") => return Ok(HashMap::new()),
            Err(err) => return Err(db_unavailable(err)),
        };

        let mut buff_ids = Vec::<String>::new();
        let mut skill_buffs = HashMap::<String, Vec<String>>::new();
        for (skill_no, buff0, buff1, buff2, buff3, buff4, buff5, buff6, buff7, buff8, buff9) in
            skill_rows
        {
            let entry = skill_buffs.entry(skill_no).or_default();
            for buff_id in [
                buff0, buff1, buff2, buff3, buff4, buff5, buff6, buff7, buff8, buff9,
            ]
            .into_iter()
            .filter_map(normalize_optional_string)
            {
                if !entry.iter().any(|existing| existing == &buff_id) {
                    entry.push(buff_id.clone());
                }
                if !buff_ids.iter().any(|existing| existing == &buff_id) {
                    buff_ids.push(buff_id);
                }
            }
        }

        if buff_ids.is_empty() {
            return Ok(HashMap::new());
        }

        let buff_id_list = quote_list(&buff_ids);
        let buff_query = format!(
            "SELECT \
                TRIM(COALESCE(`Index`, '')) AS buff_id, \
                `Category`, \
                `CategoryLevel` \
             FROM buff_table{as_of} \
             WHERE TRIM(COALESCE(`Index`, '')) IN ({buff_id_list})"
        );
        let buff_rows: Vec<(String, Option<String>, Option<String>)> = match conn.query(buff_query)
        {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "buff_table") => return Ok(HashMap::new()),
            Err(err) => return Err(db_unavailable(err)),
        };
        let buff_metadata = buff_rows
            .into_iter()
            .map(|(buff_id, category, category_level)| {
                (
                    buff_id,
                    CalculatorBuffSourceMetadata {
                        category_id: parse_optional_i32(category),
                        category_level: parse_optional_i32(category_level),
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        let mut out = HashMap::new();
        for (skill_id, buff_ids) in skill_buffs {
            let categories = buff_ids
                .iter()
                .filter_map(|buff_id| buff_metadata.get(buff_id))
                .filter_map(|metadata| {
                    metadata
                        .category_id
                        .filter(|category_id| *category_id > 0)
                        .map(|category_id| (category_id, metadata.category_level))
                })
                .collect::<Vec<_>>();
            let Some((category_id, occurrences)) = categories
                .iter()
                .fold(
                    HashMap::<i32, usize>::new(),
                    |mut counts, (category_id, _)| {
                        *counts.entry(*category_id).or_default() += 1;
                        counts
                    },
                )
                .into_iter()
                .max_by_key(|(category_id, count)| (*count, -(*category_id)))
            else {
                continue;
            };
            let _ = occurrences;
            let category_level = categories
                .iter()
                .filter(|(candidate_id, _)| *candidate_id == category_id)
                .filter_map(|(_, category_level)| *category_level)
                .max();
            out.insert(
                skill_id,
                CalculatorBuffSourceMetadata {
                    category_id: Some(category_id),
                    category_level,
                },
            );
        }

        Ok(out)
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

    fn query_consumable_source_backed_item_rows(
        &self,
        ref_id: Option<&str>,
    ) -> AppResult<Vec<CalculatorSourceBackedItemRow>> {
        let mut effect_lines_by_item_id = HashMap::<i32, Vec<String>>::new();
        for (item_id, effect_line) in self.query_consumable_effect_line_rows(ref_id)? {
            let lines = effect_lines_by_item_id.entry(item_id).or_default();
            for normalized_line in normalized_effect_lines(&effect_line) {
                if !lines.iter().any(|existing| existing == &normalized_line) {
                    lines.push(normalized_line);
                }
            }
        }

        if effect_lines_by_item_id.is_empty() {
            return Ok(Vec::new());
        }

        let item_ids = effect_lines_by_item_id.keys().copied().collect::<Vec<_>>();
        let item_rows = self.query_consumable_item_rows(ref_id, &item_ids)?;
        let item_source_metadata = self.query_calculator_item_table_metadata(ref_id, &item_ids)?;
        let primary_skill_counts = item_rows
            .iter()
            .filter_map(|row| row.skill_no.clone())
            .fold(HashMap::<String, usize>::new(), |mut counts, skill_id| {
                *counts.entry(skill_id).or_default() += 1;
                counts
            });
        let skill_ids = item_rows
            .iter()
            .flat_map(|row| [row.skill_no.clone(), row.sub_skill_no.clone()])
            .flatten()
            .collect::<Vec<_>>();
        let buff_categories_by_skill =
            self.query_consumable_skill_buff_categories(ref_id, &skill_ids)?;

        let mut source_backed_rows = item_rows
            .into_iter()
            .filter_map(|row| {
                let effect_lines = effect_lines_by_item_id.remove(&row.item_id)?;
                let source_meta = item_source_metadata.get(&row.item_id);
                let category_metadata = select_consumable_category_metadata(
                    row.skill_no.as_deref(),
                    row.sub_skill_no.as_deref(),
                    &buff_categories_by_skill,
                );
                let source_name_en = source_meta.and_then(|meta| meta.name_en.clone());
                let source_name_ko = source_meta.and_then(|meta| meta.name_ko.clone());
                let buff_category_key =
                    buff_category_key(category_metadata.category_id).or_else(|| {
                        fallback_consumable_family_key(
                            row.skill_no.as_deref(),
                            &primary_skill_counts,
                        )
                    });
                let item_type = match (category_metadata.category_id, row.item_classify.as_deref())
                {
                    (Some(1), _) | (None, Some("8")) => "food",
                    _ => "buff",
                };
                Some(CalculatorSourceBackedItemRow {
                    source_key: format!("item:{}", row.item_id),
                    source_kind: "item".to_string(),
                    item_id: Some(row.item_id),
                    item_type: item_type.to_string(),
                    buff_category_key,
                    buff_category_id: category_metadata.category_id,
                    buff_category_level: category_metadata.category_level,
                    source_name_en,
                    source_name_ko,
                    item_icon_file: None,
                    icon_id: source_meta.and_then(|meta| meta.icon_id),
                    durability: source_meta.and_then(|meta| meta.durability),
                    fish_multiplier: None,
                    effect_description_ko: Some(effect_lines.join("\n")),
                    afr: None,
                    bonus_rare: None,
                    bonus_big: None,
                    item_drr: None,
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
                buff_category_key: None,
                buff_category_id: None,
                buff_category_level: None,
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
                item_drr: drr,
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

        let mut chosen_effects =
            HashMap::<(String, String), CalculatorEnchantEffectEntryRow>::new();
        for row in rows {
            let item_type =
                normalize_optional_string(row.get::<String, _>("item_type")).unwrap_or_default();
            if !matches!(
                item_type.as_str(),
                "rod" | "float" | "chair" | "backpack" | "outfit"
            ) {
                continue;
            }
            let Some(item_name_ko) =
                normalize_optional_string(row.get::<String, _>("item_name_ko"))
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
                item_drr: row.get::<Option<f32>, _>("drr").flatten(),
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
                    existing.bonus_rare = max_opt_f32(existing.bonus_rare, effect_row.bonus_rare);
                    existing.bonus_big = max_opt_f32(existing.bonus_big, effect_row.bonus_big);
                    existing.item_drr = max_opt_f32(existing.item_drr, effect_row.item_drr);
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

        let mut exact_metadata_by_name =
            HashMap::<String, Vec<(i32, CalculatorItemSourceMetadata)>>::new();
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
                    buff_category_key: None,
                    buff_category_id: None,
                    buff_category_level: None,
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
                    item_drr: row.item_drr,
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
        let mut source_backed_rows = self.query_consumable_source_backed_item_rows(ref_id)?;
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{
        fallback_consumable_family_key, select_consumable_category_metadata,
        select_consumable_effect_texts, CalculatorBuffSourceMetadata, CalculatorBuffTextRow,
    };

    #[test]
    fn consumable_effect_texts_prefer_composite_buff_rows_over_leaf_rows() {
        let mut buff_text_rows = HashMap::new();
        buff_text_rows.insert(
            "55426".to_string(),
            CalculatorBuffTextRow {
                text: "엔트의 눈물\n생활 경험치 획득량 +30%\n낚시 속도 잠재력 +2단계".to_string(),
                has_description: true,
            },
        );
        buff_text_rows.insert(
            "55427".to_string(),
            CalculatorBuffTextRow {
                text: "생활 경험치 획득량 +30%".to_string(),
                has_description: false,
            },
        );

        let selected = select_consumable_effect_texts(
            "59335",
            &["55426".to_string(), "55427".to_string()],
            &buff_text_rows,
            &HashMap::new(),
        );

        assert_eq!(
            selected,
            vec!["엔트의 눈물\n생활 경험치 획득량 +30%\n낚시 속도 잠재력 +2단계".to_string()]
        );
    }

    #[test]
    fn consumable_effect_texts_fall_back_to_leaf_rows_without_composite_text() {
        let mut buff_text_rows = HashMap::new();
        buff_text_rows.insert(
            "55948".to_string(),
            CalculatorBuffTextRow {
                text: "낚시 경험치 획득량 +10%".to_string(),
                has_description: true,
            },
        );
        buff_text_rows.insert(
            "55942".to_string(),
            CalculatorBuffTextRow {
                text: "자동 낚시 시간 감소 7%".to_string(),
                has_description: true,
            },
        );

        let selected = select_consumable_effect_texts(
            "55570",
            &["55948".to_string(), "55942".to_string()],
            &buff_text_rows,
            &HashMap::new(),
        );

        assert_eq!(
            selected,
            vec![
                "낚시 경험치 획득량 +10%".to_string(),
                "자동 낚시 시간 감소 7%".to_string(),
            ]
        );
    }

    #[test]
    fn consumable_category_prefers_primary_skill_over_buff_removal_subskill() {
        let by_skill = HashMap::from([
            (
                "55595".to_string(),
                CalculatorBuffSourceMetadata {
                    category_id: Some(1),
                    category_level: Some(1),
                },
            ),
            (
                "51349".to_string(),
                CalculatorBuffSourceMetadata {
                    category_id: Some(10),
                    category_level: Some(1),
                },
            ),
        ]);

        let selected = select_consumable_category_metadata(Some("55595"), Some("51349"), &by_skill);

        assert_eq!(selected.category_id, Some(1));
        assert_eq!(selected.category_level, Some(1));
    }

    #[test]
    fn fallback_consumable_family_uses_skill_family_for_duplicate_skills() {
        let counts = HashMap::from([("12345".to_string(), 3usize)]);

        let key = fallback_consumable_family_key(Some("12345"), &counts);

        assert_eq!(key.as_deref(), Some("skill-family:12345"));
    }

    #[test]
    fn fallback_consumable_family_is_none_for_unique_skills_without_category() {
        let counts = HashMap::from([("59778".to_string(), 1usize)]);

        let key = fallback_consumable_family_key(Some("59778"), &counts);

        assert_eq!(key, None);
    }
}
