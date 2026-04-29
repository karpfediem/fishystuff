use mysql::prelude::Queryable;
use mysql::Row;
use std::collections::HashMap;

use crate::error::{AppError, AppResult};
use crate::store::{validate_dolt_ref, DataLang};

use super::calculator_effects::{
    normalized_effect_lines, parse_unique_calculator_effect_text, CalculatorItemEffectValues,
};
use super::item_metadata::{normalize_source_owned_item_name, ItemSourceMetadata};
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

#[derive(Debug, Clone)]
struct CalculatorEnchantEffectEntryRow {
    item_type: String,
    item_name_ko: String,
    normalized_item_name_ko: String,
    enchant_level: i32,
    skill_no: Option<String>,
    durability: Option<i32>,
    source_rule_values: CalculatorItemEffectValues,
    source_text_ko: Option<String>,
    source_text_values: CalculatorItemEffectValues,
}

#[derive(Debug, Clone, Default)]
struct CalculatorSkillEffectBundle {
    effect_description_ko: Option<String>,
    values: CalculatorItemEffectValues,
}

#[derive(Debug, Clone)]
struct RawEnchantSkillCandidateRow {
    item_name_ko: String,
    normalized_item_name_ko: String,
    enchant_level: i32,
    skill_no: String,
}

#[derive(Debug, Clone)]
pub(super) struct CalculatorCatalogSourceData {
    pub(super) legacy_rows: Vec<CalculatorItemDbRow>,
    pub(super) item_source_metadata: HashMap<i32, ItemSourceMetadata>,
    pub(super) source_backed_rows: Vec<CalculatorSourceBackedItemRow>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct CalculatorSourceEffectEvidence {
    // Values decoded from structured source surfaces, including source macros
    // such as PatternDescription and prepared source views.
    pub(super) source_rule_values: CalculatorItemEffectValues,
    // Original Korean text evidence plus values inferred by parsing that text.
    // Keep these separate from source_rule_values so text-derived enrichment
    // never masquerades as a structured source fact.
    pub(super) source_text_ko: Option<String>,
    pub(super) source_text_values: CalculatorItemEffectValues,
    // Narrow hand-maintained enrichments for source gaps that are not yet
    // recoverable from the imported source set.
    pub(super) manual_values: CalculatorItemEffectValues,
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
    pub(super) effect_evidence: CalculatorSourceEffectEvidence,
}

#[derive(Debug, Clone)]
struct CalculatorLightstoneSourceMetadataRow {
    source_key: String,
    lightstone_set_id: String,
    skill_no: String,
    set_name_ko: Option<String>,
    skill_icon_file: Option<String>,
}

fn parse_optional_i32(value: Option<String>) -> Option<i32> {
    normalize_optional_string(value).and_then(|value| value.parse::<i32>().ok())
}

fn parse_optional_f32(value: Option<String>) -> Option<f32> {
    normalize_optional_string(value).and_then(|value| value.parse::<f32>().ok())
}

fn collect_calculator_item_metadata_ids(
    legacy_rows: &[CalculatorItemDbRow],
    source_backed_rows: &[CalculatorSourceBackedItemRow],
) -> Vec<i32> {
    let mut item_ids = legacy_rows
        .iter()
        .filter_map(|row| row.10)
        .chain(source_backed_rows.iter().filter_map(|row| row.item_id))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    item_ids.sort_unstable();
    item_ids
}

// Remaining manual source fallbacks.
//
// These are intentionally narrow and only cover values we have not yet been
// able to prove from the current intermediate dump. They should stay small and
// disappear as stronger source-backed paths are found.
fn manually_maintained_source_fish_multiplier(item_id: i32, item_type: &str) -> Option<f32> {
    if item_type != "rod" {
        return None;
    }
    match item_id {
        // Multi-catch rods. The current source set lets us prove the rod family
        // identity, but not a structured numeric multiplier field like `1.6`.
        // Keep this maintained manually until an original-source numeric path is
        // found.
        16153 | 767158 | 767187 | 767671 => Some(1.6),
        _ => None,
    }
}

fn manually_maintained_source_effect_values(
    item_id: i32,
    item_type: &str,
) -> CalculatorItemEffectValues {
    if item_type != "rod" {
        return CalculatorItemEffectValues::default();
    }

    match item_id {
        // Base Triple-Float Fishing Rod. The current intermediate source dump
        // exposes the multi-catch family link, but not the hidden Rare/HQ
        // rates that are visible in-game after the tooltip rewrite.
        16153 => CalculatorItemEffectValues {
            bonus_rare: Some(0.02),
            bonus_big: Some(0.05),
            ..CalculatorItemEffectValues::default()
        },
        _ => CalculatorItemEffectValues::default(),
    }
}

fn manually_maintained_source_item_type(item_id: i32) -> Option<&'static str> {
    match item_id {
        16153 => Some("rod"),
        _ => None,
    }
}

fn extract_bracketed_name(text: &str) -> Option<String> {
    let start = text.find('[')?;
    let end = text[start + 1..].find(']')?;
    let name = text[start + 1..start + 1 + end].trim();
    (!name.is_empty()).then(|| name.to_string())
}

fn parse_lightstone_set_name(
    primary_text: Option<&str>,
    description_text: Option<&str>,
) -> Option<String> {
    primary_text.and_then(extract_bracketed_name).or_else(|| {
        description_text
            .and_then(|description| normalized_effect_lines(description).into_iter().next())
            .and_then(|line| extract_bracketed_name(&line))
    })
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

fn effect_values_from_source_text(text: Option<&str>) -> CalculatorItemEffectValues {
    let mut values = CalculatorItemEffectValues::default();
    if let Some(text) = text {
        parse_unique_calculator_effect_text(&mut values, text);
    }
    values
}

fn effect_values_from_fields(
    afr: Option<f32>,
    bonus_rare: Option<f32>,
    bonus_big: Option<f32>,
    item_drr: Option<f32>,
    exp_fish: Option<f32>,
    exp_life: Option<f32>,
) -> CalculatorItemEffectValues {
    CalculatorItemEffectValues {
        afr,
        bonus_rare,
        bonus_big,
        item_drr,
        exp_fish,
        exp_life,
    }
}

fn max_effect_values(
    left: CalculatorItemEffectValues,
    right: CalculatorItemEffectValues,
) -> CalculatorItemEffectValues {
    CalculatorItemEffectValues {
        afr: max_opt_f32(left.afr, right.afr),
        bonus_rare: max_opt_f32(left.bonus_rare, right.bonus_rare),
        bonus_big: max_opt_f32(left.bonus_big, right.bonus_big),
        item_drr: max_opt_f32(left.item_drr, right.item_drr),
        exp_fish: max_opt_f32(left.exp_fish, right.exp_fish),
        exp_life: max_opt_f32(left.exp_life, right.exp_life),
    }
}

fn merge_unique_effect_texts(left: Option<String>, right: Option<String>) -> Option<String> {
    let mut lines = Vec::<String>::new();
    for text in [left, right].into_iter().flatten() {
        for line in normalized_effect_lines(&text) {
            if !lines.iter().any(|existing| existing == &line) {
                lines.push(line);
            }
        }
    }
    (!lines.is_empty()).then(|| lines.join("\n"))
}

fn merge_skill_effect_bundle(
    target: &mut CalculatorEnchantEffectEntryRow,
    bundle: &CalculatorSkillEffectBundle,
) {
    target.source_text_ko = merge_unique_effect_texts(
        target.source_text_ko.clone(),
        bundle.effect_description_ko.clone(),
    );
    target.source_text_values = max_effect_values(target.source_text_values, bundle.values);
}

impl DoltMySqlStore {
    #[tracing::instrument(
        name = "store.calculator_catalog.query.lightstone_source_rows",
        skip_all
    )]
    fn query_lightstone_source_rows(
        &self,
        lang: &DataLang,
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
        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;

        let metadata_query = format!(
            "SELECT \
                CONCAT('lightstone-set:', TRIM(ls.`Index`)) AS source_key, \
                TRIM(ls.`Index`) AS lightstone_set_id, \
                ls.`SetOptionSkillNo` AS skill_no, \
                NULLIF(TRIM(stype.`SkillName`), '') AS skill_name_ko, \
                NULLIF(TRIM(ls.`Description`), '') AS description_ko, \
                NULLIF(TRIM(stype.`IconImageFile`), '') AS skill_icon_file \
             FROM lightstone_set_option{as_of} ls \
             LEFT JOIN skilltype_table_new{as_of} stype \
               ON stype.`SkillNo` = ls.`SetOptionSkillNo` \
             WHERE NULLIF(ls.`SetOptionSkillNo`, '') IS NOT NULL"
        );
        let metadata_rows: Vec<(
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
        )> = match conn.query(metadata_query) {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "lightstone_set_option") => return Ok(Vec::new()),
            Err(err) => return Err(db_unavailable(err)),
        };

        let metadata_rows = metadata_rows
            .into_iter()
            .filter_map(
                |(
                    source_key,
                    _lightstone_set_id,
                    skill_no,
                    skill_name_ko,
                    description_ko,
                    skill_icon_file,
                )| {
                    let skill_no = normalize_optional_string(Some(skill_no))?;
                    Some(CalculatorLightstoneSourceMetadataRow {
                        source_key,
                        lightstone_set_id: _lightstone_set_id,
                        skill_no,
                        set_name_ko: parse_lightstone_set_name(
                            skill_name_ko.as_deref(),
                            description_ko.as_deref(),
                        ),
                        skill_icon_file: normalize_optional_string(skill_icon_file),
                    })
                },
            )
            .collect::<Vec<_>>();

        if metadata_rows.is_empty() {
            return Ok(Vec::new());
        }

        let quote_list = |values: &[String]| {
            values
                .iter()
                .map(|value| format!("'{}'", value.replace('\'', "''")))
                .collect::<Vec<_>>()
                .join(",")
        };

        let localized_lightstone_names = {
            let lightstone_set_ids = metadata_rows
                .iter()
                .map(|row| row.lightstone_set_id.clone())
                .collect::<Vec<_>>();
            let names_query = format!(
                "SELECT \
                        CAST(`id` AS CHAR), \
                        MAX(NULLIF(TRIM(TRAILING ']' FROM SUBSTRING_INDEX(SUBSTRING_INDEX(NULLIF(TRIM(`text`), ''), ']', 1), '[', -1)), '')) \
                     FROM languagedata{as_of} \
                     WHERE `lang` = '{}' \
                       AND `format` = 'B' \
                       AND `category` = '113' \
                       AND `id` IN ({}) \
                       AND NULLIF(TRIM(`text`), '') IS NOT NULL \
                       AND `text` LIKE '%[%' \
                       AND `text` LIKE '%]%' \
                     GROUP BY CAST(`id` AS CHAR)",
                lang.code().replace('\'', "''"),
                quote_list(&lightstone_set_ids)
            );
            let rows: Vec<(String, Option<String>)> = match conn.query(names_query) {
                Ok(rows) => rows,
                Err(err) => return Err(db_unavailable(err)),
            };
            rows.into_iter()
                .filter_map(|(set_id, name)| {
                    normalize_optional_string(name).map(|name| (set_id, name))
                })
                .collect::<HashMap<_, _>>()
        };

        let skill_nos = metadata_rows
            .iter()
            .map(|row| row.skill_no.clone())
            .collect::<Vec<_>>();
        let skill_rows_query = format!(
            "SELECT \
                `SkillNo` AS skill_no, \
                `Buff0` AS buff0, \
                `Buff1` AS buff1, \
                `Buff2` AS buff2, \
                `Buff3` AS buff3, \
                `Buff4` AS buff4, \
                `Buff5` AS buff5, \
                `Buff6` AS buff6, \
                `Buff7` AS buff7, \
                `Buff8` AS buff8, \
                `Buff9` AS buff9 \
             FROM skill_table_new{as_of} \
             WHERE `SkillNo` IN ({})",
            quote_list(&skill_nos)
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
            Err(err) if is_missing_table(&err, "skill_table_new") => return Ok(Vec::new()),
            Err(err) => return Err(db_unavailable(err)),
        };

        let mut buff_ids_by_skill = HashMap::<String, Vec<String>>::new();
        for (skill_no, buff0, buff1, buff2, buff3, buff4, buff5, buff6, buff7, buff8, buff9) in
            skill_rows
        {
            let entry = buff_ids_by_skill.entry(skill_no).or_default();
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

        let buff_ids = buff_ids_by_skill
            .values()
            .flat_map(|buff_ids| buff_ids.iter().cloned())
            .collect::<Vec<_>>();
        if buff_ids.is_empty() {
            return Ok(Vec::new());
        }

        let buff_rows_query = format!(
            "SELECT \
                `Index` AS buff_id, \
                `BuffName`, \
                `Description` \
             FROM buff_table{as_of} \
             WHERE `Index` IN ({})",
            quote_list(&buff_ids)
        );
        let buff_rows: Vec<(String, Option<String>, Option<String>)> =
            match conn.query(buff_rows_query) {
                Ok(rows) => rows,
                Err(err) if is_missing_table(&err, "buff_table") => return Ok(Vec::new()),
                Err(err) => return Err(db_unavailable(err)),
            };

        let buff_text_by_id = buff_rows
            .into_iter()
            .filter_map(|(buff_id, buff_name, description)| {
                normalize_optional_string(description)
                    .or_else(|| normalize_optional_string(buff_name))
                    .map(|text| (buff_id, text))
            })
            .collect::<HashMap<_, _>>();

        let mut out = Vec::new();
        for row in metadata_rows {
            let Some(buff_ids) = buff_ids_by_skill.get(&row.skill_no) else {
                continue;
            };
            let mut effect_lines = Vec::new();
            for buff_id in buff_ids {
                let Some(text) = buff_text_by_id.get(buff_id) else {
                    continue;
                };
                for normalized_line in normalized_effect_lines(text) {
                    if !effect_lines
                        .iter()
                        .any(|existing| existing == &normalized_line)
                    {
                        effect_lines.push(normalized_line);
                    }
                }
            }
            if effect_lines.is_empty() {
                continue;
            }

            let effect_description_ko = effect_lines.join("\n");
            let mut values = CalculatorItemEffectValues::default();
            parse_unique_calculator_effect_text(&mut values, &effect_description_ko);
            if values == CalculatorItemEffectValues::default() {
                continue;
            }

            out.push((
                row.source_key,
                row.set_name_ko,
                localized_lightstone_names
                    .get(&row.lightstone_set_id)
                    .cloned(),
                row.skill_icon_file,
                Some(effect_description_ko),
                values.afr,
                values.bonus_rare,
                values.bonus_big,
                values.item_drr,
                values.exp_fish,
                values.exp_life,
            ));
        }

        Ok(out)
    }

    #[tracing::instrument(
        name = "store.calculator_catalog.query.skill_buff_effect_bundles",
        skip_all,
        fields(skill_count = skill_nos.len())
    )]
    fn query_skill_buff_effect_bundles(
        &self,
        ref_id: Option<&str>,
        skill_nos: &[String],
    ) -> AppResult<HashMap<String, CalculatorSkillEffectBundle>> {
        if skill_nos.is_empty() {
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

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let skill_rows_query = format!(
            "SELECT \
                `SkillNo` AS skill_no, \
                `Buff0` AS buff0, \
                `Buff1` AS buff1, \
                `Buff2` AS buff2, \
                `Buff3` AS buff3, \
                `Buff4` AS buff4, \
                `Buff5` AS buff5, \
                `Buff6` AS buff6, \
                `Buff7` AS buff7, \
                `Buff8` AS buff8, \
                `Buff9` AS buff9 \
             FROM skill_table_new{as_of} \
             WHERE `SkillNo` IN ({})",
            quote_list(skill_nos)
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
            Err(err) if is_missing_table(&err, "skill_table_new") => return Ok(HashMap::new()),
            Err(err) => return Err(db_unavailable(err)),
        };

        let mut buff_ids_by_skill = HashMap::<String, Vec<String>>::new();
        let mut buff_ids = Vec::<String>::new();
        for (skill_no, buff0, buff1, buff2, buff3, buff4, buff5, buff6, buff7, buff8, buff9) in
            skill_rows
        {
            let entry = buff_ids_by_skill.entry(skill_no).or_default();
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

        let buff_rows_query = format!(
            "SELECT \
                `Index` AS buff_id, \
                `BuffName`, \
                `Description` \
             FROM buff_table{as_of} \
             WHERE `Index` IN ({})",
            quote_list(&buff_ids)
        );
        let buff_rows: Vec<(String, Option<String>, Option<String>)> =
            match conn.query(buff_rows_query) {
                Ok(rows) => rows,
                Err(err) if is_missing_table(&err, "buff_table") => return Ok(HashMap::new()),
                Err(err) => return Err(db_unavailable(err)),
            };

        let mut buff_lines_by_id = HashMap::<String, Vec<String>>::new();
        for (buff_id, buff_name, description) in buff_rows {
            let entry = buff_lines_by_id.entry(buff_id).or_default();
            for text in [description, buff_name]
                .into_iter()
                .filter_map(normalize_optional_string)
            {
                for normalized_line in normalized_effect_lines(&text) {
                    if !entry.iter().any(|existing| existing == &normalized_line) {
                        entry.push(normalized_line);
                    }
                }
            }
        }

        let mut out = HashMap::new();
        for (skill_no, buff_ids) in buff_ids_by_skill {
            let mut effect_lines = Vec::new();
            for buff_id in buff_ids {
                let Some(lines) = buff_lines_by_id.get(&buff_id) else {
                    continue;
                };
                for line in lines {
                    if !effect_lines.iter().any(|existing| existing == line) {
                        effect_lines.push(line.clone());
                    }
                }
            }
            if effect_lines.is_empty() {
                continue;
            }
            let effect_description_ko = Some(effect_lines.join("\n"));
            let mut values = CalculatorItemEffectValues::default();
            if let Some(text) = effect_description_ko.as_deref() {
                parse_unique_calculator_effect_text(&mut values, text);
            }
            out.insert(
                skill_no,
                CalculatorSkillEffectBundle {
                    effect_description_ko,
                    values,
                },
            );
        }

        Ok(out)
    }

    #[tracing::instrument(
        name = "store.calculator_catalog.query.raw_enchant_skill_map",
        skip_all,
        fields(item_name_count = item_names.len())
    )]
    fn query_raw_enchant_skill_map(
        &self,
        ref_id: Option<&str>,
        item_names: &[String],
    ) -> AppResult<HashMap<(String, i32), String>> {
        if item_names.is_empty() {
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
        let name_list = quote_list(item_names);

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let mut candidates = HashMap::<(String, i32), Vec<String>>::new();
        for table_name in ["enchant_equipment", "enchant_lifeequipment", "enchant_cash"] {
            let query = format!(
                "SELECT \
                    NULLIF(TRIM(`ItemName`), '') AS item_name_ko, \
                    CAST(TRIM(COALESCE(`Enchant`, '0')) AS SIGNED) AS enchant_level, \
                    `SkillNo` AS skill_no \
                 FROM {table_name}{as_of} \
                 WHERE NULLIF(TRIM(`ItemName`), '') IN ({name_list}) \
                   AND NULLIF(`SkillNo`, '') IS NOT NULL \
                   AND `SkillNo` <> '0'"
            );
            let rows: Vec<(Option<String>, i64, Option<String>)> = match conn.query(query) {
                Ok(rows) => rows,
                Err(err) if is_missing_table(&err, table_name) => Vec::new(),
                Err(err) => return Err(db_unavailable(err)),
            };
            for (item_name_ko, enchant_level, skill_no) in rows {
                let Some(item_name_ko) = normalize_optional_string(item_name_ko) else {
                    continue;
                };
                let Some(skill_no) = normalize_optional_string(skill_no) else {
                    continue;
                };
                let Ok(enchant_level) = i32::try_from(enchant_level) else {
                    continue;
                };
                let entry = candidates.entry((item_name_ko, enchant_level)).or_default();
                if !entry.iter().any(|existing| existing == &skill_no) {
                    entry.push(skill_no);
                }
            }
        }

        Ok(candidates
            .into_iter()
            .filter_map(|(key, skill_nos)| {
                (skill_nos.len() == 1).then(|| (key, skill_nos.into_iter().next().unwrap()))
            })
            .collect())
    }

    #[tracing::instrument(
        name = "store.calculator_catalog.query.raw_enchant_effect_text_map",
        skip_all,
        fields(item_name_count = item_names.len())
    )]
    fn query_raw_enchant_effect_text_map(
        &self,
        ref_id: Option<&str>,
        item_names: &[String],
    ) -> AppResult<HashMap<(String, i32), String>> {
        if item_names.is_empty() {
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
        let name_list = quote_list(item_names);

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let mut effect_text_by_key = HashMap::<(String, i32), String>::new();
        for table_name in ["enchant_equipment", "enchant_lifeequipment", "enchant_cash"] {
            let query = format!(
                "SELECT \
                    NULLIF(TRIM(`ItemName`), '') AS item_name_ko, \
                    CAST(TRIM(COALESCE(`Enchant`, '0')) AS SIGNED) AS enchant_level, \
                    NULLIF(TRIM(COALESCE(`PatternDescription`, '')), '') AS pattern_description, \
                    NULLIF(TRIM(COALESCE(`Description`, '')), '') AS description \
                 FROM {table_name}{as_of} \
                 WHERE NULLIF(TRIM(`ItemName`), '') IN ({name_list})"
            );
            let rows: Vec<(Option<String>, i64, Option<String>, Option<String>)> =
                match conn.query(query) {
                    Ok(rows) => rows,
                    Err(err) if is_missing_table(&err, table_name) => Vec::new(),
                    Err(err) => return Err(db_unavailable(err)),
                };
            for (item_name_ko, enchant_level, pattern_description, description) in rows {
                let Some(item_name_ko) = normalize_optional_string(item_name_ko) else {
                    continue;
                };
                let Ok(enchant_level) = i32::try_from(enchant_level) else {
                    continue;
                };
                let effect_text = merge_unique_effect_texts(
                    normalize_optional_string(pattern_description),
                    normalize_optional_string(description),
                );
                let Some(effect_text) = effect_text else {
                    continue;
                };
                let key = (item_name_ko, enchant_level);
                let merged =
                    merge_unique_effect_texts(effect_text_by_key.remove(&key), Some(effect_text))
                        .expect("merged raw enchant effect text should exist");
                effect_text_by_key.insert(key, merged);
            }
        }

        Ok(effect_text_by_key)
    }

    #[tracing::instrument(
        name = "store.calculator_catalog.query.raw_enchant_skill_only_candidates",
        skip_all
    )]
    fn query_raw_enchant_skill_only_candidates(
        &self,
        ref_id: Option<&str>,
    ) -> AppResult<Vec<RawEnchantSkillCandidateRow>> {
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let mut raw_rows = Vec::<RawEnchantSkillCandidateRow>::new();
        for table_name in ["enchant_equipment", "enchant_lifeequipment", "enchant_cash"] {
            let query = format!(
                "SELECT \
                    NULLIF(TRIM(`ItemName`), '') AS item_name_ko, \
                    CAST(TRIM(COALESCE(`Enchant`, '0')) AS SIGNED) AS enchant_level, \
                    `SkillNo` AS skill_no \
                 FROM {table_name}{as_of} \
                 WHERE NULLIF(`SkillNo`, '') IS NOT NULL \
                   AND `SkillNo` <> '0' \
                   AND NULLIF(TRIM(`ItemName`), '') IS NOT NULL"
            );
            let rows: Vec<(Option<String>, i64, Option<String>)> = match conn.query(query) {
                Ok(rows) => rows,
                Err(err) if is_missing_table(&err, table_name) => Vec::new(),
                Err(err) => return Err(db_unavailable(err)),
            };
            for (item_name_ko, enchant_level, skill_no) in rows {
                let Some(item_name_ko) = normalize_optional_string(item_name_ko) else {
                    continue;
                };
                let Some(skill_no) = normalize_optional_string(skill_no) else {
                    continue;
                };
                let Ok(enchant_level) = i32::try_from(enchant_level) else {
                    continue;
                };
                raw_rows.push(RawEnchantSkillCandidateRow {
                    normalized_item_name_ko: normalize_source_owned_item_name(&item_name_ko),
                    item_name_ko,
                    enchant_level,
                    skill_no,
                });
            }
        }

        if raw_rows.is_empty() {
            return Ok(Vec::new());
        }

        Ok(raw_rows)
    }

    #[tracing::instrument(
        name = "store.calculator_catalog.query.legacy_item_rows",
        skip_all,
        fields(
            excluded_item_count = excluded_item_ids.len(),
            excluded_effect_count = excluded_effect_names.len(),
        )
    )]
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

    #[tracing::instrument(
        name = "store.calculator_catalog.query.consumable_source_item_effect_evidence",
        skip_all
    )]
    fn query_consumable_source_item_effect_evidence_rows(
        &self,
        ref_id: Option<&str>,
    ) -> AppResult<Vec<CalculatorSourceBackedItemRow>> {
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };
        let query = format!(
            "SELECT \
                source_key, \
                item_id, \
                item_type, \
                buff_category_key, \
                buff_category_id, \
                buff_category_level, \
                source_text_ko, \
                source_text_afr, \
                source_text_bonus_rare, \
                source_text_bonus_big, \
                source_text_item_drr, \
                source_text_exp_fish, \
                source_text_exp_life \
             FROM calculator_consumable_source_item_effect_evidence{as_of} \
             WHERE COALESCE(\
                source_text_afr, \
                source_text_bonus_rare, \
                source_text_bonus_big, \
                source_text_item_drr, \
                source_text_exp_fish, \
                source_text_exp_life \
             ) IS NOT NULL"
        );
        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<Row> = conn.query(query).map_err(db_unavailable)?;

        Ok(rows
            .into_iter()
            .filter_map(|row| {
                let source_key = normalize_optional_string(
                    row.get::<Option<String>, _>("source_key").flatten(),
                )?;
                let item_id =
                    parse_optional_i32(row.get::<Option<String>, _>("item_id").flatten())?;
                Some(CalculatorSourceBackedItemRow {
                    source_key,
                    source_kind: "item".to_string(),
                    item_id: Some(item_id),
                    item_type: normalize_optional_string(
                        row.get::<Option<String>, _>("item_type").flatten(),
                    )
                    .unwrap_or_else(|| "buff".to_string()),
                    buff_category_key: normalize_optional_string(
                        row.get::<Option<String>, _>("buff_category_key").flatten(),
                    ),
                    buff_category_id: parse_optional_i32(
                        row.get::<Option<String>, _>("buff_category_id").flatten(),
                    ),
                    buff_category_level: parse_optional_i32(
                        row.get::<Option<String>, _>("buff_category_level")
                            .flatten(),
                    ),
                    source_name_en: None,
                    source_name_ko: None,
                    item_icon_file: None,
                    icon_id: None,
                    durability: None,
                    fish_multiplier: None,
                    effect_evidence: CalculatorSourceEffectEvidence {
                        source_text_ko: normalize_optional_string(
                            row.get::<Option<String>, _>("source_text_ko").flatten(),
                        ),
                        source_text_values: effect_values_from_fields(
                            parse_optional_f32(
                                row.get::<Option<String>, _>("source_text_afr").flatten(),
                            ),
                            parse_optional_f32(
                                row.get::<Option<String>, _>("source_text_bonus_rare")
                                    .flatten(),
                            ),
                            parse_optional_f32(
                                row.get::<Option<String>, _>("source_text_bonus_big")
                                    .flatten(),
                            ),
                            parse_optional_f32(
                                row.get::<Option<String>, _>("source_text_item_drr")
                                    .flatten(),
                            ),
                            parse_optional_f32(
                                row.get::<Option<String>, _>("source_text_exp_fish")
                                    .flatten(),
                            ),
                            parse_optional_f32(
                                row.get::<Option<String>, _>("source_text_exp_life")
                                    .flatten(),
                            ),
                        ),
                        ..CalculatorSourceEffectEvidence::default()
                    },
                })
            })
            .collect())
    }

    #[tracing::instrument(
        name = "store.calculator_catalog.query.consumable_source_backed_items",
        skip_all
    )]
    fn query_consumable_source_backed_item_rows(
        &self,
        lang: &DataLang,
        ref_id: Option<&str>,
    ) -> AppResult<Vec<CalculatorSourceBackedItemRow>> {
        let mut source_backed_rows =
            self.query_consumable_source_item_effect_evidence_rows(ref_id)?;

        source_backed_rows.extend(
            self.query_lightstone_source_rows(lang, ref_id)?
                .into_iter()
                .map(
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
                        effect_evidence: CalculatorSourceEffectEvidence {
                            source_text_ko: effect_description_ko,
                            source_text_values: effect_values_from_fields(
                                afr, bonus_rare, bonus_big, drr, exp_fish, exp_life,
                            ),
                            ..CalculatorSourceEffectEvidence::default()
                        },
                    },
                ),
        );

        Ok(source_backed_rows)
    }

    #[tracing::instrument(
        name = "store.calculator_catalog.query.enchant_source_backed_items",
        skip_all
    )]
    fn query_source_owned_enchant_source_backed_item_rows(
        &self,
        lang: &DataLang,
        ref_id: Option<&str>,
    ) -> AppResult<Vec<CalculatorSourceBackedItemRow>> {
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };
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
        let worker_span = tracing::Span::current();
        let (rows, skill_only_candidates) = std::thread::scope(|scope| -> AppResult<_> {
            let rows_handle = scope.spawn({
                let query = query.clone();
                let worker_span = worker_span.clone();
                move || -> AppResult<Option<Vec<Row>>> {
                    let _worker = worker_span.enter();
                    let _span = tracing::info_span!(
                        "store.calculator_catalog.query.enchant_item_effect_entries"
                    )
                    .entered();
                    let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
                    let rows: Option<Vec<Row>> = match conn.query(query) {
                        Ok(rows) => Ok(Some(rows)),
                        Err(err)
                            if is_missing_table(&err, "calculator_enchant_item_effect_entries") =>
                        {
                            Ok(None)
                        }
                        Err(err) => Err(db_unavailable(err)),
                    }?;
                    Ok(rows)
                }
            });
            let skill_only_handle = scope.spawn({
                let worker_span = worker_span.clone();
                move || {
                    let _worker = worker_span.enter();
                    self.query_raw_enchant_skill_only_candidates(ref_id)
                }
            });

            let rows = rows_handle.join().map_err(|_| {
                AppError::internal("calculator catalog enchant effect worker panicked")
            })??;
            let skill_only_candidates = skill_only_handle.join().map_err(|_| {
                AppError::internal("calculator catalog enchant skill-only worker panicked")
            })??;
            let Some(rows) = rows else {
                return Ok((Vec::new(), Vec::new()));
            };
            Ok((rows, skill_only_candidates))
        })?;
        let mut chosen_effects = HashMap::<String, CalculatorEnchantEffectEntryRow>::new();
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
            let enchant_level = normalize_optional_string(row.get::<String, _>("enchant_level"))
                .and_then(|value| value.parse::<i32>().ok())
                .unwrap_or_default();
            let effect_row = CalculatorEnchantEffectEntryRow {
                item_type: item_type.clone(),
                normalized_item_name_ko: normalize_source_owned_item_name(&item_name_ko),
                item_name_ko: item_name_ko.clone(),
                enchant_level,
                skill_no: None,
                durability: row.get::<Option<i32>, _>("durability").flatten(),
                source_rule_values: effect_values_from_fields(
                    row.get::<Option<f32>, _>("afr").flatten(),
                    row.get::<Option<f32>, _>("bonus_rare").flatten(),
                    row.get::<Option<f32>, _>("bonus_big").flatten(),
                    row.get::<Option<f32>, _>("drr").flatten(),
                    row.get::<Option<f32>, _>("exp_fish").flatten(),
                    None,
                ),
                source_text_ko: None,
                source_text_values: CalculatorItemEffectValues::default(),
            };

            let key = effect_row.item_name_ko.clone();
            match chosen_effects.get_mut(&key) {
                Some(existing) if effect_row.enchant_level > existing.enchant_level => {
                    *existing = effect_row;
                }
                Some(existing) if effect_row.enchant_level == existing.enchant_level => {
                    existing.durability = max_opt_i32(existing.durability, effect_row.durability);
                    existing.source_rule_values = max_effect_values(
                        existing.source_rule_values,
                        effect_row.source_rule_values,
                    );
                    existing.source_text_ko = merge_unique_effect_texts(
                        existing.source_text_ko.clone(),
                        effect_row.source_text_ko,
                    );
                    existing.source_text_values = max_effect_values(
                        existing.source_text_values,
                        effect_row.source_text_values,
                    );
                }
                Some(_) => {}
                None => {
                    chosen_effects.insert(key, effect_row);
                }
            }
        }

        for candidate in skill_only_candidates {
            let key = candidate.item_name_ko.clone();
            let candidate = CalculatorEnchantEffectEntryRow {
                item_type: String::new(),
                item_name_ko: candidate.item_name_ko,
                normalized_item_name_ko: candidate.normalized_item_name_ko,
                enchant_level: candidate.enchant_level,
                skill_no: Some(candidate.skill_no),
                durability: None,
                source_rule_values: CalculatorItemEffectValues::default(),
                source_text_ko: None,
                source_text_values: CalculatorItemEffectValues::default(),
            };
            match chosen_effects.get_mut(&key) {
                Some(existing) if candidate.enchant_level > existing.enchant_level => {
                    *existing = candidate;
                }
                Some(_) => {}
                None => {
                    chosen_effects.insert(key, candidate);
                }
            }
        }

        let mut chosen_effects = chosen_effects.into_values().collect::<Vec<_>>();
        if chosen_effects.is_empty() {
            return Ok(Vec::new());
        }

        let skill_no_by_item = self.query_raw_enchant_skill_map(
            ref_id,
            &chosen_effects
                .iter()
                .map(|row| row.item_name_ko.clone())
                .collect::<Vec<_>>(),
        )?;
        let raw_effect_text_by_item = self.query_raw_enchant_effect_text_map(
            ref_id,
            &chosen_effects
                .iter()
                .map(|row| row.item_name_ko.clone())
                .collect::<Vec<_>>(),
        )?;
        for effect_row in &mut chosen_effects {
            effect_row.skill_no = skill_no_by_item
                .get(&(effect_row.item_name_ko.clone(), effect_row.enchant_level))
                .cloned();
            let raw_effect_text = raw_effect_text_by_item
                .get(&(effect_row.item_name_ko.clone(), effect_row.enchant_level))
                .cloned();
            effect_row.source_text_ko =
                merge_unique_effect_texts(effect_row.source_text_ko.clone(), raw_effect_text);
            effect_row.source_text_values =
                effect_values_from_source_text(effect_row.source_text_ko.as_deref());
        }

        let skill_nos = chosen_effects
            .iter()
            .filter_map(|row| row.skill_no.clone())
            .collect::<Vec<_>>();
        let skill_effects = self.query_skill_buff_effect_bundles(ref_id, &skill_nos)?;
        for effect_row in &mut chosen_effects {
            let Some(skill_no) = effect_row.skill_no.as_deref() else {
                continue;
            };
            let Some(bundle) = skill_effects.get(skill_no) else {
                continue;
            };
            merge_skill_effect_bundle(effect_row, bundle);
        }

        let exact_names = chosen_effects
            .iter()
            .map(|row| row.item_name_ko.clone())
            .collect::<Vec<_>>();
        let normalized_names = chosen_effects
            .iter()
            .map(|row| row.normalized_item_name_ko.clone())
            .collect::<Vec<_>>();
        let metadata_candidates =
            self.query_item_table_metadata_by_names(lang, ref_id, &exact_names, &normalized_names)?;

        let mut exact_metadata_by_name = HashMap::<String, Vec<(i32, ItemSourceMetadata)>>::new();
        let mut normalized_metadata_by_name =
            HashMap::<String, Vec<(i32, ItemSourceMetadata)>>::new();
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

        let mut source_backed_rows = chosen_effects
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
                let item_type = if row.item_type.is_empty() {
                    metadata.item_type.clone()?
                } else {
                    row.item_type
                };
                if !matches!(
                    item_type.as_str(),
                    "rod" | "float" | "chair" | "backpack" | "outfit"
                ) {
                    return None;
                }
                // Manual values stay as enrichment evidence until final catalog
                // assembly. They cover known source gaps without changing the
                // structured/text evidence recovered here.
                let fish_multiplier =
                    manually_maintained_source_fish_multiplier(item_id, &item_type);
                let manual_values = manually_maintained_source_effect_values(item_id, &item_type);

                Some(CalculatorSourceBackedItemRow {
                    source_key: format!("item:{item_id}"),
                    source_kind: "item".to_string(),
                    item_id: Some(item_id),
                    item_type,
                    buff_category_key: None,
                    buff_category_id: None,
                    buff_category_level: None,
                    source_name_en: metadata.display_name(),
                    source_name_ko: metadata.name_ko,
                    item_icon_file: None,
                    icon_id: metadata.icon_id,
                    durability: row.durability.or(metadata.durability),
                    fish_multiplier,
                    effect_evidence: CalculatorSourceEffectEvidence {
                        source_rule_values: row.source_rule_values,
                        source_text_ko: row.source_text_ko,
                        source_text_values: row.source_text_values,
                        manual_values,
                    },
                })
            })
            .collect::<Vec<_>>();

        let existing_item_ids = source_backed_rows
            .iter()
            .filter_map(|row| row.item_id)
            .collect::<std::collections::HashSet<_>>();
        // Manual-only rows are a last resort for items that should still be in
        // the calculator but currently have no recoverable source-backed effect
        // row in the intermediate dump.
        let manual_item_ids = [16153];
        let manual_metadata = self.query_item_table_metadata(lang, ref_id, &manual_item_ids)?;
        for item_id in manual_item_ids {
            if existing_item_ids.contains(&item_id) {
                continue;
            }
            let Some(item_type) = manually_maintained_source_item_type(item_id) else {
                continue;
            };
            let manual_values = manually_maintained_source_effect_values(item_id, item_type);
            let fish_multiplier = manually_maintained_source_fish_multiplier(item_id, item_type);
            if manual_values == CalculatorItemEffectValues::default() && fish_multiplier.is_none() {
                continue;
            }
            let Some(metadata) = manual_metadata.get(&item_id) else {
                continue;
            };
            source_backed_rows.push(CalculatorSourceBackedItemRow {
                source_key: format!("item:{item_id}"),
                source_kind: "item".to_string(),
                item_id: Some(item_id),
                item_type: item_type.to_string(),
                buff_category_key: None,
                buff_category_id: None,
                buff_category_level: None,
                source_name_en: metadata.display_name(),
                source_name_ko: metadata.name_ko.clone(),
                item_icon_file: None,
                icon_id: metadata.icon_id,
                durability: metadata.durability,
                fish_multiplier,
                effect_evidence: CalculatorSourceEffectEvidence {
                    manual_values,
                    ..CalculatorSourceEffectEvidence::default()
                },
            });
        }

        Ok(source_backed_rows)
    }

    pub(super) fn query_calculator_catalog_source_data_at_revision(
        &self,
        lang: &DataLang,
        revision: &str,
        resolved_ref: &str,
    ) -> AppResult<CalculatorCatalogSourceData> {
        let cache_key = format!("{}:{revision}", lang.code());
        loop {
            if let Ok(cache) = self.calculator_source_data_cache.lock() {
                if let Some(cached) = cache.get(&cache_key) {
                    return Ok(cached.clone());
                }
            }

            let (inflight_lock, inflight_cvar) = &*self.calculator_source_data_inflight;
            let mut inflight = inflight_lock
                .lock()
                .expect("calculator source data inflight lock poisoned");
            if !inflight.contains(&cache_key) {
                inflight.insert(cache_key.clone());
                drop(inflight);
                break;
            }
            inflight = inflight_cvar
                .wait(inflight)
                .expect("calculator source data inflight wait poisoned");
            drop(inflight);
        }

        let query_ref = Some(resolved_ref);
        let result: AppResult<CalculatorCatalogSourceData> = (|| {
            let worker_span = tracing::Span::current();
            let (all_legacy_rows, source_backed_rows) =
                std::thread::scope(|scope| -> AppResult<_> {
                    let legacy_rows_handle = scope.spawn({
                        let worker_span = worker_span.clone();
                        move || {
                            let _worker = worker_span.enter();
                            self.query_legacy_calculator_item_rows(query_ref, &[], &[])
                        }
                    });
                    let consumable_rows_handle = scope.spawn({
                        let worker_span = worker_span.clone();
                        move || {
                            let _worker = worker_span.enter();
                            self.query_consumable_source_backed_item_rows(lang, query_ref)
                        }
                    });
                    let enchant_rows_handle = scope.spawn({
                        let worker_span = worker_span.clone();
                        move || {
                            let _worker = worker_span.enter();
                            self.query_source_owned_enchant_source_backed_item_rows(lang, query_ref)
                        }
                    });

                    let all_legacy_rows = legacy_rows_handle.join().map_err(|_| {
                        AppError::internal("calculator catalog legacy item worker panicked")
                    })??;
                    let mut source_backed_rows =
                        consumable_rows_handle.join().map_err(|_| {
                            AppError::internal("calculator catalog consumable item worker panicked")
                        })??;
                    source_backed_rows.extend(enchant_rows_handle.join().map_err(|_| {
                        AppError::internal("calculator catalog enchant item worker panicked")
                    })??);
                    Ok((all_legacy_rows, source_backed_rows))
                })?;
            let item_source_metadata = self.query_item_table_metadata(
                lang,
                query_ref,
                &collect_calculator_item_metadata_ids(&all_legacy_rows, &source_backed_rows),
            )?;

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
                        Some(item_type)
                            if has_source_lightstones && item_type == "lightstone_set" =>
                        {
                            false
                        }
                        _ => true,
                    };
                    keep_item && keep_effect
                })
                .collect::<Vec<_>>();

            let source_data = CalculatorCatalogSourceData {
                legacy_rows,
                item_source_metadata,
                source_backed_rows,
            };
            Ok(source_data)
        })();

        let (inflight_lock, inflight_cvar) = &*self.calculator_source_data_inflight;
        let mut inflight = inflight_lock
            .lock()
            .expect("calculator source data inflight lock poisoned");
        inflight.remove(&cache_key);
        inflight_cvar.notify_all();
        drop(inflight);

        let source_data = result?;

        if let Ok(mut cache) = self.calculator_source_data_cache.lock() {
            cache.insert(cache_key, source_data.clone());
        }

        Ok(source_data)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        collect_calculator_item_metadata_ids, manually_maintained_source_effect_values,
        manually_maintained_source_fish_multiplier, merge_unique_effect_texts,
        parse_lightstone_set_name, CalculatorSourceBackedItemRow, CalculatorSourceEffectEvidence,
    };

    #[test]
    fn collect_calculator_item_metadata_ids_includes_source_backed_items() {
        let legacy_rows = vec![
            (
                Some("Legacy Food".to_string()),
                Some("food".to_string()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(9359),
                Some(9359),
            ),
            (
                Some("Legacy Effect".to_string()),
                Some("buff".to_string()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
        ];
        let source_backed_rows = vec![
            CalculatorSourceBackedItemRow {
                source_key: "item:9307".to_string(),
                source_kind: "item".to_string(),
                item_id: Some(9307),
                item_type: "buff".to_string(),
                buff_category_key: None,
                buff_category_id: None,
                buff_category_level: None,
                source_name_en: None,
                source_name_ko: None,
                item_icon_file: None,
                icon_id: None,
                durability: None,
                fish_multiplier: None,
                effect_evidence: CalculatorSourceEffectEvidence::default(),
            },
            CalculatorSourceBackedItemRow {
                source_key: "item:9359".to_string(),
                source_kind: "item".to_string(),
                item_id: Some(9359),
                item_type: "food".to_string(),
                buff_category_key: None,
                buff_category_id: None,
                buff_category_level: None,
                source_name_en: None,
                source_name_ko: None,
                item_icon_file: None,
                icon_id: None,
                durability: None,
                fish_multiplier: None,
                effect_evidence: CalculatorSourceEffectEvidence::default(),
            },
            CalculatorSourceBackedItemRow {
                source_key: "lightstone-set:1".to_string(),
                source_kind: "lightstone_set".to_string(),
                item_id: None,
                item_type: "lightstone_set".to_string(),
                buff_category_key: None,
                buff_category_id: None,
                buff_category_level: None,
                source_name_en: None,
                source_name_ko: None,
                item_icon_file: None,
                icon_id: None,
                durability: None,
                fish_multiplier: None,
                effect_evidence: CalculatorSourceEffectEvidence::default(),
            },
        ];

        assert_eq!(
            collect_calculator_item_metadata_ids(&legacy_rows, &source_backed_rows),
            vec![9307, 9359]
        );
    }

    #[test]
    fn parse_lightstone_set_name_prefers_skill_name() {
        let name = parse_lightstone_set_name(
            Some("160.[신의 입질]"),
            Some("[대장장이의 축복]\n장비 내구도 감소 저항 +30%"),
        );

        assert_eq!(name.as_deref(), Some("신의 입질"));
    }

    #[test]
    fn parse_lightstone_set_name_falls_back_to_description() {
        let name = parse_lightstone_set_name(
            None,
            Some("<PAColor0xffd2ffad>[대장장이의 축복]<PAOldColor>\\n장비 내구도 감소 저항 +30%"),
        );

        assert_eq!(name.as_deref(), Some("대장장이의 축복"));
    }

    #[test]
    fn parse_lightstone_set_name_reads_loc_category_113_text() {
        let name = parse_lightstone_set_name(
            None,
            Some(
                "<PAColor0xffd2ffad>[Nibbles]<PAOldColor>\nAuto-fishing Time -15%\nFishing EXP +10%",
            ),
        );

        assert_eq!(name.as_deref(), Some("Nibbles"));
    }

    #[test]
    fn manually_maintained_source_fish_multiplier_covers_known_multi_catch_rods() {
        assert_eq!(
            manually_maintained_source_fish_multiplier(16153, "rod"),
            Some(1.6)
        );
        assert_eq!(
            manually_maintained_source_fish_multiplier(767158, "rod"),
            Some(1.6)
        );
        assert_eq!(
            manually_maintained_source_fish_multiplier(767187, "rod"),
            Some(1.6)
        );
        assert_eq!(
            manually_maintained_source_fish_multiplier(767671, "rod"),
            Some(1.6)
        );
        assert_eq!(
            manually_maintained_source_fish_multiplier(16162, "rod"),
            None
        );
        assert_eq!(
            manually_maintained_source_fish_multiplier(760976, "rod"),
            None
        );
        assert_eq!(
            manually_maintained_source_fish_multiplier(830150, "backpack"),
            None
        );
    }

    #[test]
    fn manually_maintained_source_effect_values_cover_base_triple_float_hidden_rates() {
        let values = manually_maintained_source_effect_values(16153, "rod");
        assert_eq!(values.bonus_rare, Some(0.02));
        assert_eq!(values.bonus_big, Some(0.05));

        let unrelated = manually_maintained_source_effect_values(767158, "rod");
        assert_eq!(unrelated.bonus_rare, None);
        assert_eq!(unrelated.bonus_big, None);

        let wrong_type = manually_maintained_source_effect_values(16153, "backpack");
        assert_eq!(wrong_type.bonus_rare, None);
        assert_eq!(wrong_type.bonus_big, None);
    }

    #[test]
    fn merge_unique_effect_texts_combines_raw_and_skill_lines_without_duplicates() {
        let merged = merge_unique_effect_texts(
            Some(
                "AUTO_FISHING_REDUCE_TIME_DOWN_2(10)\nCHANCE_RARE_SPECIES_FISH_INCRE(5)"
                    .to_string(),
            ),
            Some("희귀 확률 증가(5%)\nCHANCE_RARE_SPECIES_FISH_INCRE(5)".to_string()),
        );

        assert_eq!(
            merged.as_deref(),
            Some(
                "AUTO_FISHING_REDUCE_TIME_DOWN_2(10)\nCHANCE_RARE_SPECIES_FISH_INCRE(5)\n희귀 확률 증가(5%)"
            )
        );
    }
}
