use std::collections::{HashMap, HashSet};

use mysql::prelude::Queryable;

use crate::error::AppResult;
use crate::store::validate_dolt_ref;

use super::util::{db_unavailable, is_missing_table, normalize_optional_string};
use super::DoltMySqlStore;

type CalculatorConsumableEffectDbRow =
    (Option<i32>, Option<String>, Option<String>, Option<String>);

type CalculatorLightstoneEffectDbRow = (Option<String>, Option<String>);

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct CalculatorItemEffectValues {
    pub(super) afr: Option<f32>,
    pub(super) bonus_rare: Option<f32>,
    pub(super) bonus_big: Option<f32>,
    pub(super) drr: Option<f32>,
    pub(super) exp_fish: Option<f32>,
    pub(super) exp_life: Option<f32>,
}

impl CalculatorItemEffectValues {
    fn has_any(self) -> bool {
        self.afr.is_some()
            || self.bonus_rare.is_some()
            || self.bonus_big.is_some()
            || self.drr.is_some()
            || self.exp_fish.is_some()
            || self.exp_life.is_some()
    }
}

fn add_effect_value(slot: &mut Option<f32>, value: Option<f32>) {
    let Some(value) = value else {
        return;
    };
    *slot = Some(slot.unwrap_or(0.0) + value);
}

pub(super) fn extract_first_number(text: &str) -> Option<f32> {
    let chars: Vec<char> = text.chars().collect();
    let mut idx = 0;
    while idx < chars.len() {
        if chars[idx] == '+' || chars[idx] == '-' || chars[idx].is_ascii_digit() {
            let start = idx;
            idx += 1;
            let mut seen_digit = chars[start].is_ascii_digit();
            while idx < chars.len() && (chars[idx].is_ascii_digit() || chars[idx] == '.') {
                seen_digit |= chars[idx].is_ascii_digit();
                idx += 1;
            }
            if seen_digit {
                let candidate = chars[start..idx].iter().collect::<String>();
                if let Ok(value) = candidate.parse::<f32>() {
                    return Some(value);
                }
            }
        } else {
            idx += 1;
        }
    }
    None
}

fn extract_percent_ratio(text: &str) -> Option<f32> {
    extract_first_number(text).map(|value| value.abs() / 100.0)
}

fn parse_calculator_effect_line(values: &mut CalculatorItemEffectValues, line: &str) {
    let line = line.trim();
    if line.is_empty() {
        return;
    }
    if line.contains("자동 낚시") {
        add_effect_value(&mut values.afr, extract_percent_ratio(line));
    }
    if line.contains("희귀 어종") {
        add_effect_value(&mut values.bonus_rare, extract_percent_ratio(line));
    }
    if line.contains("대형 어종") {
        add_effect_value(&mut values.bonus_big, extract_percent_ratio(line));
    }
    if line.contains("내구도 소모 감소 저항") {
        add_effect_value(&mut values.drr, extract_percent_ratio(line));
    }
    if line.contains("낚시 경험치") {
        add_effect_value(&mut values.exp_fish, extract_percent_ratio(line));
    }
    if line.contains("생활 경험치") {
        add_effect_value(&mut values.exp_life, extract_percent_ratio(line));
    }
}

pub(super) fn parse_calculator_effect_text(values: &mut CalculatorItemEffectValues, text: &str) {
    for line in text.lines() {
        parse_calculator_effect_line(values, line);
    }
}

pub(super) fn legacy_lightstone_name_for_source_name_ko(name_ko: &str) -> Option<&'static str> {
    match name_ko.trim() {
        "신의 입질" => Some("Nibbles"),
        "고래의 입" => Some("Whaling"),
        "예리한 갈매기" => Some("Sharp-Eyed Seagull"),
        "선택과 집중 : 낚시" => Some("Choice & Focus: Fishing"),
        "대장장이의 축복" => Some("Blacksmith's Blessing"),
        _ => None,
    }
}

impl DoltMySqlStore {
    pub(super) fn query_calculator_consumable_effect_overrides(
        &self,
        ref_id: Option<&str>,
        item_ids: &[i32],
    ) -> AppResult<HashMap<i32, CalculatorItemEffectValues>> {
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
                item_description_ko, \
                skill_description_ko, \
                buff_description_ko \
             FROM calculator_consumable_effects{as_of} \
             WHERE item_id IN ({id_list})"
        );

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<CalculatorConsumableEffectDbRow> = match conn.query(query) {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "calculator_consumable_effects") => {
                return Ok(HashMap::new());
            }
            Err(err) if is_missing_table(&err, "skill_table_new") => return Ok(HashMap::new()),
            Err(err) if is_missing_table(&err, "buff_table") => return Ok(HashMap::new()),
            Err(err) => return Err(db_unavailable(err)),
        };

        let mut description_lines = HashMap::<i32, HashSet<String>>::new();
        let mut item_descriptions = HashMap::<i32, String>::new();
        for (item_id, item_description, skill_description, buff_description) in rows {
            let Some(item_id) = item_id else {
                continue;
            };
            if let Some(item_description) = normalize_optional_string(item_description) {
                item_descriptions.entry(item_id).or_insert(item_description);
            }
            let entry = description_lines.entry(item_id).or_default();
            for description in [buff_description, skill_description] {
                let Some(description) = normalize_optional_string(description) else {
                    continue;
                };
                for line in description.lines() {
                    let line = line.trim();
                    if !line.is_empty() {
                        entry.insert(line.to_string());
                    }
                }
            }
        }

        let mut overrides = HashMap::new();
        for item_id in item_ids.iter().copied() {
            let mut values = CalculatorItemEffectValues::default();
            let mut had_effect_lines = false;
            if let Some(lines) = description_lines.get(&item_id) {
                had_effect_lines = !lines.is_empty();
                for line in lines {
                    parse_calculator_effect_line(&mut values, line);
                }
            }
            if !had_effect_lines {
                if let Some(description) = item_descriptions.get(&item_id) {
                    parse_calculator_effect_text(&mut values, description);
                }
            }
            if values.has_any() {
                overrides.insert(item_id, values);
            }
        }

        Ok(overrides)
    }

    pub(super) fn query_calculator_lightstone_effect_overrides(
        &self,
        ref_id: Option<&str>,
    ) -> AppResult<HashMap<String, CalculatorItemEffectValues>> {
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };
        let query = format!(
            "SELECT \
                set_name_ko, \
                effect_description_ko \
             FROM calculator_lightstone_set_effects{as_of}"
        );

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<CalculatorLightstoneEffectDbRow> = match conn.query(query) {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "calculator_lightstone_set_effects") => {
                return Ok(HashMap::new());
            }
            Err(err) if is_missing_table(&err, "lightstone_set_option") => {
                return Ok(HashMap::new());
            }
            Err(err) => return Err(db_unavailable(err)),
        };

        let mut overrides = HashMap::new();
        for (set_name_ko, effect_description_ko) in rows {
            let Some(set_name_ko) = normalize_optional_string(set_name_ko) else {
                continue;
            };
            let Some(legacy_name) = legacy_lightstone_name_for_source_name_ko(&set_name_ko) else {
                continue;
            };
            let Some(effect_description_ko) = normalize_optional_string(effect_description_ko)
            else {
                continue;
            };
            let mut values = CalculatorItemEffectValues::default();
            parse_calculator_effect_text(&mut values, &effect_description_ko);
            if values.has_any() {
                overrides.insert(legacy_name.to_string(), values);
            }
        }

        Ok(overrides)
    }

    pub(super) fn query_calculator_lightstone_name_overrides_ko(
        &self,
        ref_id: Option<&str>,
    ) -> AppResult<HashMap<String, String>> {
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };
        let query = format!(
            "SELECT \
                set_name_ko, \
                effect_description_ko \
             FROM calculator_lightstone_set_effects{as_of}"
        );

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<CalculatorLightstoneEffectDbRow> = match conn.query(query) {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "calculator_lightstone_set_effects") => {
                return Ok(HashMap::new());
            }
            Err(err) if is_missing_table(&err, "lightstone_set_option") => {
                return Ok(HashMap::new());
            }
            Err(err) => return Err(db_unavailable(err)),
        };

        let mut overrides = HashMap::new();
        for (set_name_ko, _) in rows {
            let Some(set_name_ko) = normalize_optional_string(set_name_ko) else {
                continue;
            };
            let Some(legacy_name) = legacy_lightstone_name_for_source_name_ko(&set_name_ko) else {
                continue;
            };
            overrides.insert(legacy_name.to_string(), set_name_ko);
        }
        Ok(overrides)
    }
}
