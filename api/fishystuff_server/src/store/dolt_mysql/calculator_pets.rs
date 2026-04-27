use std::collections::{BTreeMap, HashMap, HashSet};

use fishystuff_api::models::calculator::{
    CalculatorOptionEntry, CalculatorPetCatalog, CalculatorPetEntry, CalculatorPetOptionEntry,
    CalculatorPetTierEntry,
};
use mysql::{prelude::Queryable, PooledConn};

use crate::error::AppResult;
use crate::store::{validate_dolt_ref, FishLang};

use super::util::{db_unavailable, is_missing_table};
use super::DoltMySqlStore;

type CalculatorPetDbRow = (
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
);
type CalculatorPetSkillIndexDbRow = (Option<String>, Option<String>);
type CalculatorPetEquipSkillDbRow = (Option<String>, Option<String>, Option<String>);
type CalculatorPetAcquireSkillRateDbRow = (Option<String>, Option<String>, Option<String>);
type CalculatorLanguagedataDbRow = (Option<String>, Option<String>);
type CalculatorPetSkilltypeDbRow = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);
type CalculatorPetSpecialSkillDbRow = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

#[derive(Debug, Clone)]
struct RawPetRow {
    character_key: String,
    skin_key: Option<String>,
    icon_image_file: Option<String>,
    race: String,
    kind: String,
    tier_source: u8,
    special_skill_no: Option<String>,
    base_skill_index: Option<String>,
    acquire_key: Option<String>,
}

#[derive(Debug, Clone)]
struct PetSpecialSkillMeta {
    skill_no: String,
    skill_type: String,
    param0: Option<String>,
    param1: Option<String>,
}

#[derive(Debug, Clone)]
struct PetEquipSkillRow {
    index: String,
    group_no: String,
    skill_no: String,
}

#[derive(Debug, Clone, Default)]
struct PetOptionEffects {
    auto_fishing_time_reduction: Option<f32>,
    durability_reduction_resistance: Option<f32>,
    fishing_exp: Option<f32>,
    life_exp: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PetOptionKind {
    Special,
    Talent,
    Skill,
}

#[derive(Debug, Clone)]
struct CalculatorPetOptionRecord {
    entry: CalculatorPetOptionEntry,
}

#[derive(Debug, Clone)]
struct BuiltPetEntry {
    entry: CalculatorPetEntry,
    alias_keys: Vec<String>,
}

fn localized_label(lang: FishLang, en: impl Into<String>, ko: impl Into<String>) -> String {
    match lang {
        FishLang::En => en.into(),
        FishLang::Ko => ko.into(),
    }
}

fn build_calculator_pet_catalog(lang: FishLang) -> CalculatorPetCatalog {
    let tiers = (1..=5)
        .map(|tier| CalculatorOptionEntry {
            key: tier.to_string(),
            label: match lang {
                FishLang::En => format!("Tier {tier}"),
                FishLang::Ko => format!("{tier}세대"),
            },
        })
        .collect();

    CalculatorPetCatalog {
        slots: 5,
        pets: Vec::new(),
        tiers,
        specials: Vec::new(),
        talents: Vec::new(),
        skills: Vec::new(),
    }
}

fn trim_optional_string(value: Option<String>) -> Option<String> {
    let value = value?;
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("null")
        || trimmed.eq_ignore_ascii_case("<null>")
    {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_u8(value: Option<&str>) -> Option<u8> {
    value?.trim().parse::<u8>().ok()
}

fn parse_f32(value: Option<&str>) -> Option<f32> {
    value?.trim().parse::<f32>().ok()
}

fn is_looting_pet_type(value: Option<&str>) -> bool {
    matches!(value.map(str::trim), None | Some("") | Some("0"))
}

fn parse_asset_stem(raw_path: &str) -> Option<String> {
    let normalized = raw_path.trim();
    if normalized.is_empty() {
        return None;
    }
    let basename = normalized
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(normalized)
        .trim();
    if basename.is_empty() {
        return None;
    }
    let stem = basename
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(basename)
        .trim();
    (!stem.is_empty()).then(|| stem.to_ascii_lowercase())
}

fn pet_image_url(raw_path: Option<&str>) -> Option<String> {
    raw_path
        .and_then(parse_asset_stem)
        .map(|stem| format!("/images/pets/{stem}.webp"))
}

fn pet_visual_group_key(row: &RawPetRow) -> String {
    row.icon_image_file
        .as_deref()
        .and_then(parse_asset_stem)
        .or_else(|| {
            row.skin_key
                .as_ref()
                .map(|value| value.to_ascii_lowercase())
        })
        .unwrap_or_else(|| row.character_key.to_ascii_lowercase())
}

fn parse_acquire_rate(value: Option<&str>) -> Option<f32> {
    value
        .and_then(|raw| raw.trim().parse::<f64>().ok())
        .filter(|rate| *rate > 0.0)
        .map(|rate| (rate / 1_000_000.0) as f32)
}

fn parse_first_number(value: &str) -> Option<f32> {
    let mut current = String::new();
    let mut seen_digit = false;
    for ch in value.chars() {
        if ch.is_ascii_digit()
            || (ch == '.' && seen_digit)
            || ((ch == '+' || ch == '-') && current.is_empty())
        {
            current.push(ch);
            seen_digit |= ch.is_ascii_digit();
            continue;
        }
        if seen_digit {
            break;
        }
        current.clear();
    }
    if current.is_empty() {
        None
    } else {
        current.parse::<f32>().ok().map(f32::abs)
    }
}

fn parse_pet_option_effects(
    english_label: Option<&str>,
    korean_label: Option<&str>,
    korean_description: Option<&str>,
) -> PetOptionEffects {
    let mut effects = PetOptionEffects::default();
    let mut texts = Vec::new();
    if let Some(value) = english_label.filter(|value| !value.trim().is_empty()) {
        texts.push(value.to_string());
    }
    if texts.is_empty() {
        if let Some(value) = korean_label.filter(|value| !value.trim().is_empty()) {
            texts.push(value.to_string());
        }
        if let Some(value) = korean_description.filter(|value| !value.trim().is_empty()) {
            texts.push(value.to_string());
        }
    }

    for text in texts {
        for segment in text.split(',') {
            let normalized = segment.trim().to_ascii_lowercase();
            if normalized.is_empty() {
                continue;
            }
            if normalized.contains("fishing exp") || normalized.contains("낚시 경험치") {
                if let Some(value) = parse_first_number(segment) {
                    effects.fishing_exp = Some(value / 100.0);
                }
                continue;
            }
            if normalized.contains("life exp") || normalized.contains("생활 경험치") {
                if let Some(value) = parse_first_number(segment) {
                    effects.life_exp = Some(value / 100.0);
                }
                continue;
            }
            if normalized.contains("durability reduction resistance")
                || normalized.contains("내구도 감소 저항")
                || normalized.contains("내구도 소모 감소 저항")
            {
                if let Some(value) = parse_first_number(segment) {
                    effects.durability_reduction_resistance = Some(value / 100.0);
                }
                continue;
            }
            if normalized.contains("auto-fishing time") || normalized.contains("자동 낚시") {
                if normalized.contains("sec") || normalized.contains('초') {
                    if let Some(value) = parse_first_number(segment) {
                        effects.auto_fishing_time_reduction = Some(value / 180.0);
                    }
                } else if normalized.contains('%') {
                    if let Some(value) = parse_first_number(segment) {
                        effects.auto_fishing_time_reduction = Some(value / 100.0);
                    }
                }
            }
        }
    }

    effects
}

fn pet_option_kind(effects: &PetOptionEffects) -> Option<PetOptionKind> {
    if effects.auto_fishing_time_reduction.is_some() {
        return Some(PetOptionKind::Special);
    }
    if effects.durability_reduction_resistance.is_some() || effects.life_exp.is_some() {
        return Some(PetOptionKind::Talent);
    }
    if effects.fishing_exp.is_some() {
        return Some(PetOptionKind::Skill);
    }
    None
}

fn localized_pet_option_label(
    lang: FishLang,
    skill_no: &str,
    english_label: Option<&str>,
    korean_label: Option<&str>,
    korean_description: Option<&str>,
) -> String {
    match lang {
        FishLang::En => english_label
            .or(korean_label)
            .or(korean_description)
            .unwrap_or(skill_no)
            .to_string(),
        FishLang::Ko => korean_label
            .or(korean_description)
            .or(english_label)
            .unwrap_or(skill_no)
            .to_string(),
    }
}

fn quoted_sql_values(values: &[String]) -> String {
    values
        .iter()
        .map(|value| format!("'{}'", value.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(",")
}

fn query_pet_rows(conn: &mut PooledConn, as_of: &str) -> Result<Vec<RawPetRow>, mysql::Error> {
    let query = format!(
        "SELECT \
            `CharacterKey`, \
            `PetChangeLookKey`, \
            `IconImageFile1`, \
            `Race`, \
            `Kind`, \
            `Tier`, \
            `Skill_1`, \
            `BaseSkill`, \
            `EquipSkillAquireKey`, \
            `PetType` \
         FROM pet_table{as_of}"
    );
    let rows: Vec<CalculatorPetDbRow> = conn.query(query)?;
    Ok(rows
        .into_iter()
        .filter_map(
            |(
                character_key,
                skin_key,
                icon_image_file,
                race,
                kind,
                tier_source,
                special_skill_no,
                base_skill_index,
                acquire_key,
                pet_type,
            )| {
                let character_key = trim_optional_string(character_key)?;
                let race = trim_optional_string(race)?;
                let kind = trim_optional_string(kind)?;
                if !is_looting_pet_type(trim_optional_string(pet_type).as_deref()) {
                    return None;
                }
                let tier_source = parse_u8(trim_optional_string(tier_source).as_deref())?;
                Some(RawPetRow {
                    character_key,
                    skin_key: trim_optional_string(skin_key).filter(|value| value != "0"),
                    icon_image_file: trim_optional_string(icon_image_file),
                    race,
                    kind,
                    tier_source,
                    special_skill_no: trim_optional_string(special_skill_no)
                        .filter(|value| value != "0"),
                    base_skill_index: trim_optional_string(base_skill_index),
                    acquire_key: trim_optional_string(acquire_key).filter(|value| value != "0"),
                })
            },
        )
        .collect())
}

fn query_skill_map(
    conn: &mut PooledConn,
    as_of: &str,
    table_name: &str,
) -> Result<HashMap<String, String>, mysql::Error> {
    let query = format!("SELECT `Index`, `SkillNo` FROM {table_name}{as_of}");
    let rows: Vec<CalculatorPetSkillIndexDbRow> = conn.query(query)?;
    Ok(rows
        .into_iter()
        .filter_map(|(index, skill_no)| {
            let index = trim_optional_string(index)?;
            let skill_no = trim_optional_string(skill_no)?;
            Some((index, skill_no))
        })
        .collect())
}

fn query_pet_equip_skill_rows(
    conn: &mut PooledConn,
    as_of: &str,
) -> Result<Vec<PetEquipSkillRow>, mysql::Error> {
    let query = format!("SELECT `Index`, `GroupNo`, `SkillNo` FROM pet_equipskill_table{as_of}");
    let rows: Vec<CalculatorPetEquipSkillDbRow> = conn.query(query)?;
    Ok(rows
        .into_iter()
        .filter_map(|(index, group_no, skill_no)| {
            Some(PetEquipSkillRow {
                index: trim_optional_string(index)?,
                group_no: trim_optional_string(group_no)?,
                skill_no: trim_optional_string(skill_no)?,
            })
        })
        .collect())
}

fn pet_equip_skill_row_sort_key(row: &PetEquipSkillRow) -> (i64, &str) {
    (
        row.index.parse::<i64>().unwrap_or(i64::MAX),
        row.index.as_str(),
    )
}

fn pet_main_learned_skill_slot(row_count: usize) -> Option<usize> {
    match row_count {
        0 => None,
        1 => Some(0),
        _ => Some(1),
    }
}

fn build_pet_learned_skill_by_index(
    equip_skill_rows: &[PetEquipSkillRow],
) -> HashMap<String, String> {
    let mut rows_by_group = HashMap::<String, Vec<&PetEquipSkillRow>>::new();
    for row in equip_skill_rows {
        rows_by_group
            .entry(row.group_no.clone())
            .or_default()
            .push(row);
    }
    for rows in rows_by_group.values_mut() {
        rows.sort_by(|left, right| {
            pet_equip_skill_row_sort_key(left).cmp(&pet_equip_skill_row_sort_key(right))
        });
    }

    let mut result = HashMap::new();
    for row in equip_skill_rows {
        let Some(group_rows) = rows_by_group.get(&row.group_no) else {
            continue;
        };
        let Some(skill_row) =
            pet_main_learned_skill_slot(group_rows.len()).and_then(|slot| group_rows.get(slot))
        else {
            continue;
        };
        result.insert(row.index.clone(), skill_row.skill_no.clone());
    }
    result
}

fn pet_acquire_skill_rate_select(rate_index: usize, as_of: &str) -> String {
    let equip_index = rate_index.saturating_sub(1);
    format!(
        "SELECT `Key`, '{equip_index}', `AquireRate_{rate_index}` \
         FROM pet_equipskill_aquire_table{as_of} \
         WHERE `AquireRate_{rate_index}` IS NOT NULL \
           AND `AquireRate_{rate_index}` <> '' \
           AND `AquireRate_{rate_index}` <> '0'",
    )
}

fn query_acquire_skill_rates(
    conn: &mut PooledConn,
    as_of: &str,
) -> Result<HashMap<String, HashMap<String, f32>>, mysql::Error> {
    let query = (1..=42)
        .map(|rate_index| pet_acquire_skill_rate_select(rate_index, as_of))
        .collect::<Vec<_>>()
        .join(" UNION ALL ");
    let rows: Vec<CalculatorPetAcquireSkillRateDbRow> = conn.query(query)?;
    let mut result = HashMap::<String, HashMap<String, f32>>::new();
    for (acquire_key, equip_index, raw_rate) in rows {
        let Some(acquire_key) = trim_optional_string(acquire_key) else {
            continue;
        };
        let Some(equip_index) = trim_optional_string(equip_index) else {
            continue;
        };
        if let Some(chance) = parse_acquire_rate(trim_optional_string(raw_rate).as_deref()) {
            result
                .entry(acquire_key)
                .or_default()
                .entry(equip_index)
                .or_insert(chance);
        }
    }
    Ok(result)
}

fn query_languagedata_texts(
    conn: &mut PooledConn,
    as_of: &str,
    ids: &HashSet<String>,
    unk: &str,
) -> Result<HashMap<String, String>, mysql::Error> {
    let mut values = ids.iter().cloned().collect::<Vec<_>>();
    values.sort();
    let mut result = HashMap::new();
    for chunk in values.chunks(400) {
        let query = format!(
            "SELECT `id`, `text` \
             FROM languagedata_en{as_of} \
             WHERE `unk` = '{}' \
               AND `id` IN ({})",
            unk.replace('\'', "''"),
            quoted_sql_values(chunk),
        );
        let rows: Vec<CalculatorLanguagedataDbRow> = conn.query(query)?;
        for (id, text) in rows {
            let Some(id) = trim_optional_string(id) else {
                continue;
            };
            let Some(text) = trim_optional_string(text) else {
                continue;
            };
            result.entry(id).or_insert(text);
        }
    }
    Ok(result)
}

fn query_skilltype_meta(
    conn: &mut PooledConn,
    as_of: &str,
    skill_ids: &HashSet<String>,
) -> Result<HashMap<String, (Option<String>, Option<String>, Option<String>)>, mysql::Error> {
    let mut values = skill_ids.iter().cloned().collect::<Vec<_>>();
    values.sort();
    let mut result = HashMap::new();
    for chunk in values.chunks(400) {
        let query = format!(
            "SELECT `SkillNo`, `SkillName`, `Desc`, `IconImageFile` \
             FROM skilltype_table_new{as_of} \
             WHERE `SkillNo` IN ({})",
            quoted_sql_values(chunk),
        );
        let rows: Vec<CalculatorPetSkilltypeDbRow> = conn.query(query)?;
        for (skill_no, skill_name_ko, skill_description_ko, icon) in rows {
            let Some(skill_no) = trim_optional_string(skill_no) else {
                continue;
            };
            result.insert(
                skill_no,
                (
                    trim_optional_string(skill_name_ko),
                    trim_optional_string(skill_description_ko),
                    trim_optional_string(icon),
                ),
            );
        }
    }
    Ok(result)
}

fn query_pet_special_skill_meta(
    conn: &mut PooledConn,
    as_of: &str,
    skill_ids: &HashSet<String>,
) -> Result<HashMap<String, PetSpecialSkillMeta>, mysql::Error> {
    let mut values = skill_ids.iter().cloned().collect::<Vec<_>>();
    values.sort();
    let mut result = HashMap::new();
    for chunk in values.chunks(400) {
        let query = format!(
            "SELECT `PetSkillNo`, `PetSkillType`, `Param0`, `Param1` \
             FROM pet_skill_table{as_of} \
             WHERE `Level` = '1' \
               AND `PetSkillNo` IN ({})",
            quoted_sql_values(chunk),
        );
        let rows: Vec<CalculatorPetSpecialSkillDbRow> = conn.query(query)?;
        for (skill_no, skill_type, param0, param1) in rows {
            let Some(skill_no) = trim_optional_string(skill_no) else {
                continue;
            };
            let Some(skill_type) = trim_optional_string(skill_type) else {
                continue;
            };
            result.insert(
                skill_no.clone(),
                PetSpecialSkillMeta {
                    skill_no,
                    skill_type,
                    param0: trim_optional_string(param0),
                    param1: trim_optional_string(param1),
                },
            );
        }
    }
    Ok(result)
}

fn choose_pet_tier_representative<'a>(rows: &'a [&'a RawPetRow]) -> Option<&'a RawPetRow> {
    rows.iter().copied().min_by(|left, right| {
        let left_has_acquire = left.acquire_key.is_some();
        let right_has_acquire = right.acquire_key.is_some();
        right_has_acquire
            .cmp(&left_has_acquire)
            .then_with(|| {
                right
                    .icon_image_file
                    .is_some()
                    .cmp(&left.icon_image_file.is_some())
            })
            .then_with(|| right.skin_key.is_some().cmp(&left.skin_key.is_some()))
            .then_with(|| {
                let left_id = left.character_key.parse::<i64>().unwrap_or(i64::MAX);
                let right_id = right.character_key.parse::<i64>().unwrap_or(i64::MAX);
                left_id.cmp(&right_id)
            })
    })
}

fn choose_pet_base_label(
    rows: &[RawPetRow],
    names_by_character_key: &HashMap<String, String>,
) -> String {
    let mut counts = HashMap::<String, usize>::new();
    for row in rows {
        if let Some(name) = names_by_character_key.get(&row.character_key) {
            *counts.entry(name.clone()).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .max_by(|(left_label, left_count), (right_label, right_count)| {
            left_count
                .cmp(right_count)
                .then_with(|| left_label.cmp(right_label))
        })
        .map(|(label, _)| label)
        .unwrap_or_else(|| format!("Pet {}:{}", rows[0].race, rows[0].kind))
}

fn build_pet_skin_key(rows: &[RawPetRow]) -> Option<String> {
    let mut counts = HashMap::<String, usize>::new();
    for row in rows {
        if let Some(skin_key) = row.skin_key.as_ref() {
            *counts.entry(skin_key.clone()).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .max_by(|(left_key, left_count), (right_key, right_count)| {
            left_count
                .cmp(right_count)
                .then_with(|| left_key.cmp(right_key))
        })
        .map(|(skin_key, _)| skin_key)
}

fn pet_lineage_key(row: &RawPetRow) -> String {
    row.skin_key
        .as_ref()
        .map(|skin_key| {
            format!(
                "change-look:{}:{}:{}",
                skin_key.trim(),
                row.race.trim(),
                row.kind.trim()
            )
        })
        .unwrap_or_else(|| {
            format!(
                "visual:{}:{}:{}",
                row.race.trim(),
                row.kind.trim(),
                pet_visual_group_key(row)
            )
        })
}

fn pet_variant_group_label_key(label: &str) -> String {
    label.trim().to_ascii_lowercase()
}

fn pet_variant_group_key(row: &RawPetRow, base_label: &str) -> Option<String> {
    row.skin_key
        .as_ref()
        .map(|skin_key| {
            format!(
                "change-look:{}:{}:{}",
                skin_key.trim(),
                row.race.trim(),
                row.kind.trim()
            )
        })
        .or_else(|| {
            let label_key = pet_variant_group_label_key(base_label);
            (!label_key.is_empty()).then(|| {
                format!(
                    "visual-label:{}:{}:{}",
                    row.race.trim(),
                    pet_visual_group_key(row),
                    label_key
                )
            })
        })
}

fn build_pet_lineage_keys(rows: &[RawPetRow]) -> Vec<String> {
    let mut keys = rows.iter().map(pet_lineage_key).collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    keys
}

fn build_pet_variant_group_keys(rows: &[RawPetRow], base_label: &str) -> Vec<String> {
    let mut keys = rows
        .iter()
        .filter_map(|row| pet_variant_group_key(row, base_label))
        .collect::<Vec<_>>();
    keys.sort();
    keys.dedup();
    keys
}

fn sorted_pet_row_refs<'a>(rows: &[&'a RawPetRow]) -> Vec<&'a RawPetRow> {
    let mut sorted = rows.to_vec();
    sorted.sort_by(|left, right| {
        let left_id = left.character_key.parse::<i64>().unwrap_or(i64::MAX);
        let right_id = right.character_key.parse::<i64>().unwrap_or(i64::MAX);
        left_id
            .cmp(&right_id)
            .then_with(|| left.character_key.cmp(&right.character_key))
    });
    sorted
}

fn pet_tier_candidate_rows<'a>(
    representative: &'a RawPetRow,
    candidates: &[&'a RawPetRow],
) -> Vec<&'a RawPetRow> {
    let mut rows = Vec::with_capacity(candidates.len().max(1));
    rows.push(representative);
    for candidate in sorted_pet_row_refs(candidates) {
        if candidate.character_key != representative.character_key {
            rows.push(candidate);
        }
    }
    rows
}

fn dedupe_strings_preserve_order(values: &mut Vec<String>) {
    let mut seen = HashSet::new();
    values.retain(|value| seen.insert(value.clone()));
}

fn build_tier_entry(
    lang: FishLang,
    tier_source: u8,
    representative: &RawPetRow,
    candidates: &[&RawPetRow],
    base_skill_by_index: &HashMap<String, String>,
    acquire_skill_rates: &HashMap<String, HashMap<String, f32>>,
    equip_skill_rows: &[PetEquipSkillRow],
    options_by_key: &HashMap<String, CalculatorPetOptionRecord>,
) -> CalculatorPetTierEntry {
    let mut specials = Vec::new();
    let mut talents = Vec::new();
    let mut skills = Vec::new();
    let mut skill_chances = BTreeMap::new();
    let candidate_rows = pet_tier_candidate_rows(representative, candidates);
    let learned_skill_by_index = build_pet_learned_skill_by_index(equip_skill_rows);

    if let Some(special_key) = candidate_rows.iter().find_map(|candidate| {
        candidate.special_skill_no.as_ref().and_then(|skill_no| {
            let special_key = pet_special_option_key(skill_no);
            options_by_key
                .contains_key(&special_key)
                .then_some(special_key)
        })
    }) {
        specials.push(special_key);
    }

    if let Some(talent_key) = candidate_rows.iter().find_map(|candidate| {
        candidate.base_skill_index.as_ref().and_then(|skill_index| {
            base_skill_by_index
                .get(skill_index)
                .filter(|skill_no| options_by_key.contains_key(*skill_no))
                .cloned()
        })
    }) {
        talents.push(talent_key);
    }

    for candidate in candidate_rows {
        if let Some(acquire_key) = candidate.acquire_key.as_ref() {
            if let Some(rates) = acquire_skill_rates.get(acquire_key) {
                let mut option_chances = BTreeMap::<String, f32>::new();
                for (skill_no, chance) in rates
                    .iter()
                    .filter_map(|(index, chance)| {
                        learned_skill_by_index
                            .get(index)
                            .map(|skill_no| (skill_no, *chance))
                    })
                    .filter(|(skill_no, _)| options_by_key.contains_key(*skill_no))
                {
                    *option_chances.entry(skill_no.clone()).or_default() += chance;
                }
                let mut option_ids = option_chances.into_iter().collect::<Vec<_>>();
                option_ids.sort_by(|left, right| {
                    let left_label = options_by_key
                        .get(&left.0)
                        .map(|option| option.entry.label.as_str())
                        .unwrap_or(left.0.as_str());
                    let right_label = options_by_key
                        .get(&right.0)
                        .map(|option| option.entry.label.as_str())
                        .unwrap_or(right.0.as_str());
                    left_label
                        .cmp(right_label)
                        .then_with(|| left.0.cmp(&right.0))
                });
                for (option_id, chance) in option_ids {
                    skill_chances.entry(option_id.clone()).or_insert(chance);
                    skills.push(option_id);
                }
            }
        }
    }
    dedupe_strings_preserve_order(&mut specials);
    dedupe_strings_preserve_order(&mut talents);
    dedupe_strings_preserve_order(&mut skills);

    CalculatorPetTierEntry {
        key: (tier_source + 1).to_string(),
        label: localized_label(
            lang,
            format!("Tier {}", tier_source + 1),
            format!("{}세대", tier_source + 1),
        ),
        specials,
        talents,
        skills,
        skill_chances,
    }
}

fn pet_entry_dedup_signature(entry: &CalculatorPetEntry) -> String {
    format!(
        "{}\u{1f}{}\u{1f}{}",
        entry.label.trim().to_ascii_lowercase(),
        entry.image_url.as_deref().unwrap_or_default().trim(),
        serde_json::to_string(&entry.tiers).unwrap_or_default(),
    )
}

fn dedupe_built_pet_entries(entries: Vec<BuiltPetEntry>) -> Vec<BuiltPetEntry> {
    let mut deduped = BTreeMap::<String, BuiltPetEntry>::new();
    for mut built in entries {
        let signature = pet_entry_dedup_signature(&built.entry);
        built.alias_keys.push(built.entry.key.clone());
        match deduped.get_mut(&signature) {
            Some(existing) => {
                existing.alias_keys.extend(built.alias_keys);
                if existing.entry.skin_key.is_none() {
                    existing.entry.skin_key = built.entry.skin_key.clone();
                }
                existing
                    .entry
                    .lineage_keys
                    .extend(built.entry.lineage_keys.clone());
                existing
                    .entry
                    .variant_group_keys
                    .extend(built.entry.variant_group_keys.clone());
            }
            None => {
                deduped.insert(signature, built);
            }
        }
    }

    deduped
        .into_values()
        .map(|mut built| {
            built.alias_keys.sort();
            built.alias_keys.dedup();
            built.entry.alias_keys = built.alias_keys.clone();
            built.entry.lineage_keys.sort();
            built.entry.lineage_keys.dedup();
            built.entry.variant_group_keys.sort();
            built.entry.variant_group_keys.dedup();
            built
        })
        .collect()
}

fn calculator_pet_option_records(
    lang: FishLang,
    skill_ids: &HashSet<String>,
    base_talent_skill_ids: &HashSet<String>,
    learned_skill_ids: &HashSet<String>,
    english_labels: &HashMap<String, String>,
    skilltype_meta: &HashMap<String, (Option<String>, Option<String>, Option<String>)>,
) -> HashMap<String, CalculatorPetOptionRecord> {
    let mut records = HashMap::new();
    for skill_id in skill_ids {
        let english_label = english_labels.get(skill_id).cloned();
        let (korean_label, korean_description, _icon) = skilltype_meta
            .get(skill_id)
            .cloned()
            .unwrap_or((None, None, None));
        let effects = parse_pet_option_effects(
            english_label.as_deref(),
            korean_label.as_deref(),
            korean_description.as_deref(),
        );
        let Some(_kind) = pet_option_kind(&effects)
            .or_else(|| {
                base_talent_skill_ids
                    .contains(skill_id)
                    .then_some(PetOptionKind::Talent)
            })
            .or_else(|| {
                learned_skill_ids
                    .contains(skill_id)
                    .then_some(PetOptionKind::Skill)
            })
        else {
            continue;
        };
        records.insert(
            skill_id.clone(),
            CalculatorPetOptionRecord {
                entry: CalculatorPetOptionEntry {
                    key: skill_id.clone(),
                    label: localized_pet_option_label(
                        lang,
                        skill_id,
                        english_label.as_deref(),
                        korean_label.as_deref(),
                        korean_description.as_deref(),
                    ),
                    icon: None,
                    auto_fishing_time_reduction: effects.auto_fishing_time_reduction,
                    durability_reduction_resistance: effects.durability_reduction_resistance,
                    fishing_exp: effects.fishing_exp,
                    life_exp: effects.life_exp,
                },
            },
        );
    }
    records
}

fn pet_special_option_key(skill_no: &str) -> String {
    format!("pet-special:{}", skill_no.trim())
}

fn format_whole_number(value: f32) -> String {
    if (value - value.round()).abs() < 0.001 {
        format!("{}", value.round() as i32)
    } else {
        format!("{value:.1}")
    }
}

fn pet_special_percent(value: Option<&str>) -> Option<f32> {
    parse_f32(value).map(|raw| raw / 1_000_000.0)
}

fn pet_special_seconds(value: Option<&str>) -> Option<f32> {
    parse_f32(value).map(|raw| raw / 1000.0)
}

fn pet_special_range_meters(value: Option<&str>) -> Option<f32> {
    parse_f32(value).map(|raw| raw / 100.0)
}

fn pet_special_interval_suffix(meta: &PetSpecialSkillMeta) -> String {
    pet_special_seconds(meta.param1.as_deref())
        .or_else(|| pet_special_seconds(meta.param0.as_deref()))
        .map(|seconds| format!(" every {}s", format_whole_number(seconds)))
        .unwrap_or_default()
}

fn pet_special_detection_suffix(meta: &PetSpecialSkillMeta) -> String {
    let range = pet_special_range_meters(meta.param0.as_deref());
    let seconds = pet_special_seconds(meta.param1.as_deref());
    match (range, seconds) {
        (Some(range), Some(seconds)) => format!(
            " ({}m / {}s)",
            format_whole_number(range),
            format_whole_number(seconds)
        ),
        (Some(range), None) => format!(" ({}m)", format_whole_number(range)),
        (None, Some(seconds)) => format!(" ({}s)", format_whole_number(seconds)),
        (None, None) => String::new(),
    }
}

fn pet_special_gathering_chance(meta: &PetSpecialSkillMeta) -> Option<f32> {
    match meta.skill_no.trim() {
        "41" => Some(0.30),
        "42" => Some(0.40),
        "43" => Some(0.50),
        _ => None,
    }
}

fn pet_special_option_label(lang: FishLang, meta: &PetSpecialSkillMeta) -> String {
    let special = |en: String, ko: String| localized_label(lang, en, ko);
    match meta.skill_type.trim() {
        "2" => special(
            format!(
                "Special: Resource Detection{}",
                pet_special_detection_suffix(meta)
            ),
            "특기: 자원 탐지".to_string(),
        ),
        "3" => special(
            format!(
                "Special: Hostility Detection{}",
                pet_special_detection_suffix(meta)
            ),
            "특기: 적대 모험가 감지".to_string(),
        ),
        "5" => special(
            format!(
                "Special: Monster Taunt{}",
                pet_special_interval_suffix(meta)
            ),
            "특기: 몬스터 도발".to_string(),
        ),
        "6" => special(
            format!(
                "Special: Rare Monster Detection{}",
                pet_special_interval_suffix(meta)
            ),
            "특기: 희귀 몬스터 탐지".to_string(),
        ),
        "7" => {
            let percent = pet_special_percent(meta.param0.as_deref()).unwrap_or_default() * 100.0;
            special(
                format!(
                    "Special: Auto-Fishing Time Reduction -{}%",
                    format_whole_number(percent)
                ),
                format!(
                    "특기: 자동 낚시 시간 감소 -{}%",
                    format_whole_number(percent)
                ),
            )
        }
        "8" => {
            let percent = pet_special_percent(meta.param0.as_deref()).unwrap_or_default() * 100.0;
            special(
                format!(
                    "Special: Desert Illness Resistance +{}%",
                    format_whole_number(percent)
                ),
                format!("특기: 사막 질병 저항 +{}%", format_whole_number(percent)),
            )
        }
        "9" => {
            let percent = pet_special_gathering_chance(meta).unwrap_or_default() * 100.0;
            special(
                format!(
                    "Special: Additional Gathering Resources {}% chance",
                    format_whole_number(percent)
                ),
                format!(
                    "특기: 채집물 추가 획득 확률 {}%",
                    format_whole_number(percent)
                ),
            )
        }
        "10" => special(
            "Special: Expanded Item Pickup Range".to_string(),
            "특기: 아이템 줍기 범위 증가".to_string(),
        ),
        _ => special(
            format!("Special: Pet Skill {}", meta.skill_no),
            format!("특기: 펫 기술 {}", meta.skill_no),
        ),
    }
}

fn calculator_pet_special_option_records(
    lang: FishLang,
    meta_by_skill_no: &HashMap<String, PetSpecialSkillMeta>,
) -> HashMap<String, CalculatorPetOptionRecord> {
    let mut records = HashMap::new();
    for meta in meta_by_skill_no.values() {
        let auto_fishing_time_reduction = (meta.skill_type.trim() == "7")
            .then(|| pet_special_percent(meta.param0.as_deref()))
            .flatten();
        let key = pet_special_option_key(&meta.skill_no);
        records.insert(
            key.clone(),
            CalculatorPetOptionRecord {
                entry: CalculatorPetOptionEntry {
                    key,
                    label: pet_special_option_label(lang, meta),
                    icon: None,
                    auto_fishing_time_reduction,
                    durability_reduction_resistance: None,
                    fishing_exp: None,
                    life_exp: None,
                },
            },
        );
    }
    records
}

impl DoltMySqlStore {
    pub(super) fn query_calculator_pet_catalog(
        &self,
        lang: FishLang,
        ref_id: Option<&str>,
    ) -> AppResult<CalculatorPetCatalog> {
        let mut catalog = build_calculator_pet_catalog(lang);
        let as_of = if let Some(ref_id) = ref_id {
            validate_dolt_ref(ref_id)?;
            format!(" AS OF '{}'", ref_id.replace('\'', "''"))
        } else {
            String::new()
        };
        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;

        let pet_rows = match query_pet_rows(&mut conn, &as_of) {
            Ok(rows) => rows,
            Err(err)
                if is_missing_table(&err, "pet_table")
                    || is_missing_table(&err, "languagedata_en") =>
            {
                return Ok(catalog);
            }
            Err(err) => return Err(db_unavailable(err)),
        };
        if pet_rows.is_empty() {
            return Ok(catalog);
        }

        let base_skill_by_index = match query_skill_map(&mut conn, &as_of, "pet_base_skill_table") {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "pet_base_skill_table") => HashMap::new(),
            Err(err) => return Err(db_unavailable(err)),
        };
        let equip_skill_rows = match query_pet_equip_skill_rows(&mut conn, &as_of) {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "pet_equipskill_table") => Vec::new(),
            Err(err) => return Err(db_unavailable(err)),
        };
        let acquire_skill_rates = match query_acquire_skill_rates(&mut conn, &as_of) {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "pet_equipskill_aquire_table") => HashMap::new(),
            Err(err) => return Err(db_unavailable(err)),
        };

        let pet_character_ids = pet_rows
            .iter()
            .map(|row| row.character_key.clone())
            .collect::<HashSet<_>>();
        let names_by_character_key =
            match query_languagedata_texts(&mut conn, &as_of, &pet_character_ids, "6") {
                Ok(rows) => rows,
                Err(err) if is_missing_table(&err, "languagedata_en") => HashMap::new(),
                Err(err) => return Err(db_unavailable(err)),
            };

        let mut skill_ids = HashSet::new();
        let mut base_talent_skill_ids = HashSet::new();
        let mut learned_skill_ids = HashSet::new();
        let mut pet_special_skill_ids = HashSet::new();
        let learned_skill_by_index = build_pet_learned_skill_by_index(&equip_skill_rows);
        for row in &pet_rows {
            if let Some(skill_no) = row.special_skill_no.as_ref() {
                pet_special_skill_ids.insert(skill_no.clone());
            }
            if let Some(skill_index) = row.base_skill_index.as_ref() {
                if let Some(skill_no) = base_skill_by_index.get(skill_index) {
                    skill_ids.insert(skill_no.clone());
                    base_talent_skill_ids.insert(skill_no.clone());
                }
            }
            if let Some(acquire_key) = row.acquire_key.as_ref() {
                if let Some(rates) = acquire_skill_rates.get(acquire_key) {
                    for index in rates.keys() {
                        if let Some(skill_no) = learned_skill_by_index.get(index) {
                            skill_ids.insert(skill_no.clone());
                            learned_skill_ids.insert(skill_no.clone());
                        }
                    }
                }
            }
        }

        let english_skill_labels =
            match query_languagedata_texts(&mut conn, &as_of, &skill_ids, "10") {
                Ok(rows) => rows,
                Err(err) if is_missing_table(&err, "languagedata_en") => HashMap::new(),
                Err(err) => return Err(db_unavailable(err)),
            };
        let skilltype_meta = match query_skilltype_meta(&mut conn, &as_of, &skill_ids) {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "skilltype_table_new") => HashMap::new(),
            Err(err) => return Err(db_unavailable(err)),
        };
        let pet_special_skill_meta =
            match query_pet_special_skill_meta(&mut conn, &as_of, &pet_special_skill_ids) {
                Ok(rows) => rows,
                Err(err) => return Err(db_unavailable(err)),
            };
        let mut options_by_key = calculator_pet_option_records(
            lang,
            &skill_ids,
            &base_talent_skill_ids,
            &learned_skill_ids,
            &english_skill_labels,
            &skilltype_meta,
        );
        options_by_key.extend(calculator_pet_special_option_records(
            lang,
            &pet_special_skill_meta,
        ));

        let mut grouped_rows = BTreeMap::<String, Vec<RawPetRow>>::new();
        for row in pet_rows {
            grouped_rows
                .entry(format!(
                    "pet:{}:{}:{}",
                    row.race,
                    row.kind,
                    pet_visual_group_key(&row)
                ))
                .or_default()
                .push(row);
        }

        let built_pets = grouped_rows
            .into_iter()
            .map(|(key, rows)| {
                let base_label = choose_pet_base_label(&rows, &names_by_character_key);
                let skin_key = build_pet_skin_key(&rows);
                let lineage_keys = build_pet_lineage_keys(&rows);
                let variant_group_keys = build_pet_variant_group_keys(&rows, &base_label);
                let mut tiers_by_source = BTreeMap::<u8, Vec<&RawPetRow>>::new();
                for row in &rows {
                    tiers_by_source
                        .entry(row.tier_source)
                        .or_default()
                        .push(row);
                }
                let tiers = tiers_by_source
                    .into_iter()
                    .filter_map(|(tier_source, candidates)| {
                        choose_pet_tier_representative(&candidates).map(|representative| {
                            build_tier_entry(
                                lang,
                                tier_source,
                                representative,
                                &candidates,
                                &base_skill_by_index,
                                &acquire_skill_rates,
                                &equip_skill_rows,
                                &options_by_key,
                            )
                        })
                    })
                    .collect::<Vec<_>>();
                BuiltPetEntry {
                    entry: CalculatorPetEntry {
                        key,
                        label: base_label,
                        skin_key,
                        image_url: rows
                            .iter()
                            .find_map(|row| pet_image_url(row.icon_image_file.as_deref())),
                        alias_keys: Vec::new(),
                        lineage_keys,
                        variant_group_keys,
                        tiers,
                    },
                    alias_keys: Vec::new(),
                }
            })
            .collect::<Vec<_>>();

        let mut built_pets = dedupe_built_pet_entries(built_pets);

        built_pets.sort_by(|left, right| {
            left.entry
                .label
                .cmp(&right.entry.label)
                .then_with(|| left.entry.key.cmp(&right.entry.key))
        });

        let mut special_keys = HashSet::<String>::new();
        let mut talent_keys = HashSet::<String>::new();
        let mut skill_keys = HashSet::<String>::new();
        for pet in &built_pets {
            for tier in &pet.entry.tiers {
                special_keys.extend(tier.specials.iter().cloned());
                talent_keys.extend(tier.talents.iter().cloned());
                skill_keys.extend(tier.skills.iter().cloned());
            }
        }

        let mut specials = special_keys
            .into_iter()
            .filter_map(|key| options_by_key.get(&key).map(|entry| entry.entry.clone()))
            .collect::<Vec<_>>();
        let mut talents = talent_keys
            .into_iter()
            .filter_map(|key| options_by_key.get(&key).map(|entry| entry.entry.clone()))
            .collect::<Vec<_>>();
        let mut skills = skill_keys
            .into_iter()
            .filter_map(|key| options_by_key.get(&key).map(|entry| entry.entry.clone()))
            .collect::<Vec<_>>();
        let sort_options = |left: &CalculatorPetOptionEntry, right: &CalculatorPetOptionEntry| {
            left.label
                .cmp(&right.label)
                .then_with(|| left.key.cmp(&right.key))
        };
        specials.sort_by(sort_options);
        talents.sort_by(sort_options);
        skills.sort_by(sort_options);

        catalog.pets = built_pets.into_iter().map(|pet| pet.entry).collect();
        catalog.specials = specials;
        catalog.talents = talents;
        catalog.skills = skills;
        Ok(catalog)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use fishystuff_api::models::calculator::{
        CalculatorPetEntry, CalculatorPetOptionEntry, CalculatorPetTierEntry,
    };

    use crate::store::FishLang;

    use super::{
        build_pet_learned_skill_by_index, build_pet_variant_group_keys, build_tier_entry,
        calculator_pet_option_records, calculator_pet_special_option_records,
        dedupe_built_pet_entries, is_looting_pet_type, localized_pet_option_label,
        parse_acquire_rate, parse_asset_stem, parse_pet_option_effects,
        pet_acquire_skill_rate_select, pet_image_url, pet_main_learned_skill_slot, pet_option_kind,
        pet_special_option_label, BuiltPetEntry, CalculatorPetOptionRecord, PetEquipSkillRow,
        PetOptionKind, PetSpecialSkillMeta, RawPetRow,
    };

    #[test]
    fn parse_acquire_rate_normalizes_source_weight_to_chance() {
        let chance = parse_acquire_rate(Some("150000")).expect("chance");
        assert!((chance - 0.15).abs() < 0.0001);
        assert_eq!(parse_acquire_rate(Some("0")), None);
        assert_eq!(parse_acquire_rate(Some("")), None);
    }

    #[test]
    fn pet_acquire_skill_rate_select_maps_one_based_rate_columns_to_zero_based_equip_indexes() {
        let query = pet_acquire_skill_rate_select(1, " AS OF 'test'");
        assert!(query.contains("SELECT `Key`, '0', `AquireRate_1`"));
        assert!(query.contains("pet_equipskill_aquire_table AS OF 'test'"));

        let query = pet_acquire_skill_rate_select(4, "");
        assert!(query.contains("SELECT `Key`, '3', `AquireRate_4`"));
    }

    #[test]
    fn pet_main_learned_skill_slot_uses_source_main_row() {
        assert_eq!(pet_main_learned_skill_slot(0), None);
        assert_eq!(pet_main_learned_skill_slot(1), Some(0));
        assert_eq!(pet_main_learned_skill_slot(2), Some(1));
        assert_eq!(pet_main_learned_skill_slot(3), Some(1));
    }

    #[test]
    fn learned_skill_indexes_resolve_through_main_skill_group() {
        let rows = vec![
            PetEquipSkillRow {
                index: "3".to_string(),
                group_no: "2".to_string(),
                skill_no: "combat_exp_passive".to_string(),
            },
            PetEquipSkillRow {
                index: "4".to_string(),
                group_no: "2".to_string(),
                skill_no: "combat_exp_main".to_string(),
            },
            PetEquipSkillRow {
                index: "5".to_string(),
                group_no: "2".to_string(),
                skill_no: "combat_exp_duplicate".to_string(),
            },
            PetEquipSkillRow {
                index: "10".to_string(),
                group_no: "5".to_string(),
                skill_no: "fishing_speed".to_string(),
            },
        ];

        let learned_skill_by_index = build_pet_learned_skill_by_index(&rows);
        assert_eq!(
            learned_skill_by_index.get("3").map(String::as_str),
            Some("combat_exp_main")
        );
        assert_eq!(
            learned_skill_by_index.get("4").map(String::as_str),
            Some("combat_exp_main")
        );
        assert_eq!(
            learned_skill_by_index.get("10").map(String::as_str),
            Some("fishing_speed")
        );
    }

    #[test]
    fn parse_pet_option_effects_reads_percent_and_seconds_sources() {
        let fishing = parse_pet_option_effects(Some("Fishing EXP +7%"), None, None);
        assert_eq!(fishing.fishing_exp, Some(0.07));
        assert_eq!(pet_option_kind(&fishing), Some(PetOptionKind::Skill));

        let special = parse_pet_option_effects(Some("Auto-Fishing Time -10 sec"), None, None);
        assert_eq!(special.auto_fishing_time_reduction, Some(10.0 / 180.0));
        assert_eq!(pet_option_kind(&special), Some(PetOptionKind::Special));

        let combined =
            parse_pet_option_effects(Some("Fishing EXP +10%, Auto-fishing Time -5%"), None, None);
        assert_eq!(combined.fishing_exp, Some(0.10));
        assert_eq!(combined.auto_fishing_time_reduction, Some(0.05));
    }

    #[test]
    fn localized_pet_option_label_prefers_requested_language() {
        assert_eq!(
            localized_pet_option_label(
                FishLang::En,
                "49022",
                Some("Fishing EXP +5%"),
                Some("낚시 경험치 획득량 증가 +5%"),
                None,
            ),
            "Fishing EXP +5%"
        );
        assert_eq!(
            localized_pet_option_label(
                FishLang::Ko,
                "49085",
                Some("Durability Reduction Resistance +5%"),
                Some("내구도 감소 저항 +5%"),
                None,
            ),
            "내구도 감소 저항 +5%"
        );
    }

    #[test]
    fn calculator_pet_option_records_keeps_unmodeled_base_talents() {
        let skill_ids = HashSet::from([
            "base:combat_exp".to_string(),
            "equip:ignored".to_string(),
            "equip:fishing_exp".to_string(),
        ]);
        let base_talent_skill_ids = HashSet::from(["base:combat_exp".to_string()]);
        let english_labels = HashMap::from([
            ("base:combat_exp".to_string(), "Combat EXP +5%".to_string()),
            ("equip:ignored".to_string(), "Ignore Me +5%".to_string()),
            (
                "equip:fishing_exp".to_string(),
                "Fishing EXP +7%".to_string(),
            ),
        ]);

        let records = calculator_pet_option_records(
            FishLang::En,
            &skill_ids,
            &base_talent_skill_ids,
            &HashSet::new(),
            &english_labels,
            &HashMap::new(),
        );

        let base_record = records.get("base:combat_exp").expect("base talent");
        assert_eq!(base_record.entry.label, "Combat EXP +5%");
        assert_eq!(
            records.get("equip:fishing_exp").unwrap().entry.label,
            "Fishing EXP +7%"
        );
        assert!(!records.contains_key("equip:ignored"));
    }

    #[test]
    fn calculator_pet_option_records_keeps_source_backed_learned_skills_without_modeled_effects() {
        let skill_ids = HashSet::from(["skill:combat_exp".to_string()]);
        let learned_skill_ids = HashSet::from(["skill:combat_exp".to_string()]);
        let english_labels =
            HashMap::from([("skill:combat_exp".to_string(), "Combat EXP +5%".to_string())]);

        let records = calculator_pet_option_records(
            FishLang::En,
            &skill_ids,
            &HashSet::new(),
            &learned_skill_ids,
            &english_labels,
            &HashMap::new(),
        );

        let record = records
            .get("skill:combat_exp")
            .expect("source learned skill");
        assert_eq!(record.entry.label, "Combat EXP +5%");
    }

    #[test]
    fn build_tier_entry_scans_all_candidate_rows_for_fixed_talent() {
        let representative = RawPetRow {
            character_key: "100".to_string(),
            skin_key: Some("200".to_string()),
            icon_image_file: None,
            race: "3".to_string(),
            kind: "1".to_string(),
            tier_source: 4,
            special_skill_no: None,
            base_skill_index: None,
            acquire_key: None,
        };
        let candidate_with_talent = RawPetRow {
            character_key: "101".to_string(),
            skin_key: Some("200".to_string()),
            icon_image_file: None,
            race: "3".to_string(),
            kind: "1".to_string(),
            tier_source: 4,
            special_skill_no: None,
            base_skill_index: Some("base-index".to_string()),
            acquire_key: None,
        };
        let base_skill_by_index =
            HashMap::from([("base-index".to_string(), "talent:combat_exp".to_string())]);
        let options_by_key = HashMap::from([(
            "talent:combat_exp".to_string(),
            CalculatorPetOptionRecord {
                entry: CalculatorPetOptionEntry {
                    key: "talent:combat_exp".to_string(),
                    label: "Combat EXP +5%".to_string(),
                    ..CalculatorPetOptionEntry::default()
                },
            },
        )]);
        let candidates = vec![&representative, &candidate_with_talent];

        let tier = build_tier_entry(
            FishLang::En,
            4,
            &representative,
            &candidates,
            &base_skill_by_index,
            &HashMap::new(),
            &[],
            &options_by_key,
        );

        assert_eq!(tier.key, "5");
        assert_eq!(tier.talents, vec!["talent:combat_exp".to_string()]);
    }

    #[test]
    fn build_tier_entry_uses_main_learned_skill_rows_and_sums_duplicate_effect_odds() {
        let representative = RawPetRow {
            character_key: "100".to_string(),
            skin_key: None,
            icon_image_file: None,
            race: "38".to_string(),
            kind: "38".to_string(),
            tier_source: 0,
            special_skill_no: None,
            base_skill_index: None,
            acquire_key: Some("301".to_string()),
        };
        let equip_skill_rows = vec![
            PetEquipSkillRow {
                index: "6".to_string(),
                group_no: "3".to_string(),
                skill_no: "gathering_exp_passive".to_string(),
            },
            PetEquipSkillRow {
                index: "7".to_string(),
                group_no: "3".to_string(),
                skill_no: "gathering_exp_main".to_string(),
            },
            PetEquipSkillRow {
                index: "8".to_string(),
                group_no: "3".to_string(),
                skill_no: "gathering_exp_duplicate".to_string(),
            },
            PetEquipSkillRow {
                index: "14".to_string(),
                group_no: "8".to_string(),
                skill_no: "fishing_exp_passive".to_string(),
            },
            PetEquipSkillRow {
                index: "15".to_string(),
                group_no: "8".to_string(),
                skill_no: "fishing_exp_main".to_string(),
            },
            PetEquipSkillRow {
                index: "16".to_string(),
                group_no: "8".to_string(),
                skill_no: "fishing_exp_duplicate".to_string(),
            },
        ];
        let acquire_skill_rates = HashMap::from([(
            "301".to_string(),
            HashMap::from([
                ("6".to_string(), 0.15),
                ("8".to_string(), 0.03),
                ("14".to_string(), 0.15),
            ]),
        )]);
        let options_by_key = HashMap::from([
            (
                "gathering_exp_main".to_string(),
                CalculatorPetOptionRecord {
                    entry: CalculatorPetOptionEntry {
                        key: "gathering_exp_main".to_string(),
                        label: "Gathering EXP +5%".to_string(),
                        ..CalculatorPetOptionEntry::default()
                    },
                },
            ),
            (
                "fishing_exp_main".to_string(),
                CalculatorPetOptionRecord {
                    entry: CalculatorPetOptionEntry {
                        key: "fishing_exp_main".to_string(),
                        label: "Fishing EXP +5%".to_string(),
                        ..CalculatorPetOptionEntry::default()
                    },
                },
            ),
        ]);
        let candidates = vec![&representative];

        let tier = build_tier_entry(
            FishLang::En,
            0,
            &representative,
            &candidates,
            &HashMap::new(),
            &acquire_skill_rates,
            &equip_skill_rows,
            &options_by_key,
        );

        assert_eq!(
            tier.skills,
            vec![
                "fishing_exp_main".to_string(),
                "gathering_exp_main".to_string()
            ]
        );
        assert_eq!(tier.skill_chances.get("fishing_exp_main"), Some(&0.15));
        assert!((tier.skill_chances["gathering_exp_main"] - 0.18).abs() < 0.0001);
    }

    #[test]
    fn pet_special_option_label_formats_auto_fishing_time_reduction() {
        let meta = PetSpecialSkillMeta {
            skill_no: "37".to_string(),
            skill_type: "7".to_string(),
            param0: Some("300000".to_string()),
            param1: None,
        };

        assert_eq!(
            pet_special_option_label(FishLang::En, &meta),
            "Special: Auto-Fishing Time Reduction -30%"
        );

        let records = calculator_pet_special_option_records(
            FishLang::En,
            &HashMap::from([("37".to_string(), meta)]),
        );
        let record = records.get("pet-special:37").expect("special option");
        assert_eq!(record.entry.auto_fishing_time_reduction, Some(0.30));
    }

    #[test]
    fn pet_special_option_label_formats_detection_metadata() {
        let meta = PetSpecialSkillMeta {
            skill_no: "21".to_string(),
            skill_type: "2".to_string(),
            param0: Some("3400".to_string()),
            param1: Some("10000".to_string()),
        };

        assert_eq!(
            pet_special_option_label(FishLang::En, &meta),
            "Special: Resource Detection (34m / 10s)"
        );
    }

    #[test]
    fn pet_image_url_maps_source_dds_to_cdn_webp() {
        assert_eq!(
            parse_asset_stem(r#"New_UI_Common_forLua\Window\Stable\Pet\Pet_Hawk_0014.dds"#),
            Some("pet_hawk_0014".to_string())
        );
        assert_eq!(
            pet_image_url(Some(
                r#"New_UI_Common_forLua\Window\Stable\Pet\Pet_Hawk_0014.dds"#
            )),
            Some("/images/pets/pet_hawk_0014.webp".to_string())
        );
        assert_eq!(pet_image_url(None), None);
    }

    #[test]
    fn is_looting_pet_type_excludes_fairies() {
        assert!(is_looting_pet_type(None));
        assert!(is_looting_pet_type(Some("")));
        assert!(is_looting_pet_type(Some("0")));
        assert!(!is_looting_pet_type(Some("1")));
    }

    #[test]
    fn build_pet_variant_group_keys_groups_unskinned_same_label_visual_variants() {
        let rows = vec![
            RawPetRow {
                character_key: "56626".to_string(),
                skin_key: None,
                icon_image_file: Some(
                    r#"New_UI_Common_forLua\Window\Stable\Pet\Pet_BlueDragon_0001.dds"#.to_string(),
                ),
                race: "38".to_string(),
                kind: "38".to_string(),
                tier_source: 4,
                special_skill_no: Some("26".to_string()),
                base_skill_index: Some("73".to_string()),
                acquire_key: Some("304".to_string()),
            },
            RawPetRow {
                character_key: "56631".to_string(),
                skin_key: None,
                icon_image_file: Some(
                    r#"New_UI_Common_forLua\Window\Stable\Pet\Pet_BlueDragon_0001.dds"#.to_string(),
                ),
                race: "38".to_string(),
                kind: "39".to_string(),
                tier_source: 4,
                special_skill_no: Some("26".to_string()),
                base_skill_index: Some("58".to_string()),
                acquire_key: Some("304".to_string()),
            },
        ];

        assert_eq!(
            build_pet_variant_group_keys(&rows, "Young Azure Dragon"),
            vec!["visual-label:38:pet_bluedragon_0001:young azure dragon".to_string()]
        );
    }

    #[test]
    fn dedupe_built_pet_entries_collapses_identical_pet_variants() {
        let duplicated = vec![
            BuiltPetEntry {
                entry: CalculatorPetEntry {
                    key: "pet:azure:38".to_string(),
                    label: "Young Azure Dragon".to_string(),
                    image_url: Some("/images/pets/pet_blue_dragon_0001.webp".to_string()),
                    lineage_keys: vec!["change-look:blue:38".to_string()],
                    variant_group_keys: vec!["variant:38".to_string()],
                    tiers: vec![CalculatorPetTierEntry {
                        key: "5".to_string(),
                        label: "Tier 5".to_string(),
                        specials: vec!["special_a".to_string()],
                        talents: vec!["talent_a".to_string()],
                        skills: vec!["skill_a".to_string()],
                        skill_chances: Default::default(),
                    }],
                    ..CalculatorPetEntry::default()
                },
                alias_keys: Vec::new(),
            },
            BuiltPetEntry {
                entry: CalculatorPetEntry {
                    key: "pet:azure:43".to_string(),
                    label: "Young Azure Dragon".to_string(),
                    image_url: Some("/images/pets/pet_blue_dragon_0001.webp".to_string()),
                    lineage_keys: vec!["change-look:blue:43".to_string()],
                    variant_group_keys: vec!["variant:43".to_string()],
                    tiers: vec![CalculatorPetTierEntry {
                        key: "5".to_string(),
                        label: "Tier 5".to_string(),
                        specials: vec!["special_a".to_string()],
                        talents: vec!["talent_a".to_string()],
                        skills: vec!["skill_a".to_string()],
                        skill_chances: Default::default(),
                    }],
                    ..CalculatorPetEntry::default()
                },
                alias_keys: Vec::new(),
            },
        ];

        let deduped = dedupe_built_pet_entries(duplicated);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].entry.key, "pet:azure:38");
        assert_eq!(
            deduped[0].entry.alias_keys,
            vec!["pet:azure:38".to_string(), "pet:azure:43".to_string()]
        );
        assert_eq!(
            deduped[0].entry.lineage_keys,
            vec![
                "change-look:blue:38".to_string(),
                "change-look:blue:43".to_string()
            ]
        );
        assert_eq!(
            deduped[0].entry.variant_group_keys,
            vec!["variant:38".to_string(), "variant:43".to_string()]
        );
    }
}
