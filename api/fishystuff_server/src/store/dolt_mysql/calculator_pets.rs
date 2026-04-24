use std::collections::{BTreeMap, HashMap, HashSet};

use fishystuff_api::models::calculator::{
    CalculatorOptionEntry, CalculatorPetCatalog, CalculatorPetEntry, CalculatorPetOptionEntry,
    CalculatorPetTierEntry,
};
use mysql::{prelude::Queryable, PooledConn, Row};

use crate::error::AppResult;
use crate::store::{validate_dolt_ref, FishLang};

use super::util::{db_unavailable, is_missing_table, row_string};
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
    kind: PetOptionKind,
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

fn parse_nonzero_rate(value: Option<&str>) -> bool {
    value
        .and_then(|raw| raw.trim().parse::<f64>().ok())
        .is_some_and(|rate| rate > 0.0)
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

fn query_acquire_skill_indexes(
    conn: &mut PooledConn,
    as_of: &str,
) -> Result<HashMap<String, HashSet<String>>, mysql::Error> {
    let query = format!("SELECT * FROM pet_equipskill_aquire_table{as_of}");
    let rows: Vec<Row> = conn.query(query)?;
    let mut result = HashMap::<String, HashSet<String>>::new();
    for row in rows {
        let Some(acquire_key) = row_string(&row, 0).filter(|value| !value.is_empty()) else {
            continue;
        };
        let entry = result.entry(acquire_key).or_default();
        for equip_index in 0..=42 {
            if parse_nonzero_rate(row_string(&row, equip_index + 2).as_deref()) {
                entry.insert(equip_index.to_string());
            }
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

fn build_pet_lineage_keys(rows: &[RawPetRow]) -> Vec<String> {
    let mut keys = rows.iter().map(pet_lineage_key).collect::<Vec<_>>();
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
    acquire_skill_indexes: &HashMap<String, HashSet<String>>,
    equip_skill_by_index: &HashMap<String, String>,
    options_by_key: &HashMap<String, CalculatorPetOptionRecord>,
) -> CalculatorPetTierEntry {
    let mut specials = Vec::new();
    let mut talents = Vec::new();
    let mut skills = Vec::new();
    let candidate_rows = pet_tier_candidate_rows(representative, candidates);

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
            if let Some(indexes) = acquire_skill_indexes.get(acquire_key) {
                let mut option_ids = indexes
                    .iter()
                    .filter_map(|index| equip_skill_by_index.get(index))
                    .filter(|skill_no| options_by_key.contains_key(*skill_no))
                    .cloned()
                    .collect::<Vec<_>>();
                option_ids.sort_by(|left, right| {
                    let left_label = options_by_key
                        .get(left)
                        .map(|option| option.entry.label.as_str())
                        .unwrap_or(left.as_str());
                    let right_label = options_by_key
                        .get(right)
                        .map(|option| option.entry.label.as_str())
                        .unwrap_or(right.as_str());
                    left_label.cmp(right_label).then_with(|| left.cmp(right))
                });
                option_ids.dedup();
                for option_id in option_ids {
                    match options_by_key
                        .get(&option_id)
                        .map(|option| option.kind)
                        .unwrap_or(PetOptionKind::Skill)
                    {
                        PetOptionKind::Special => specials.push(option_id),
                        PetOptionKind::Talent => talents.push(option_id),
                        PetOptionKind::Skill => skills.push(option_id),
                    }
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
            built
        })
        .collect()
}

fn calculator_pet_option_records(
    lang: FishLang,
    skill_ids: &HashSet<String>,
    base_talent_skill_ids: &HashSet<String>,
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
        let Some(kind) = pet_option_kind(&effects).or_else(|| {
            base_talent_skill_ids
                .contains(skill_id)
                .then_some(PetOptionKind::Talent)
        }) else {
            continue;
        };
        records.insert(
            skill_id.clone(),
            CalculatorPetOptionRecord {
                kind,
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
                kind: PetOptionKind::Special,
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
        let equip_skill_by_index = match query_skill_map(&mut conn, &as_of, "pet_equipskill_table")
        {
            Ok(rows) => rows,
            Err(err) if is_missing_table(&err, "pet_equipskill_table") => HashMap::new(),
            Err(err) => return Err(db_unavailable(err)),
        };
        let acquire_skill_indexes = match query_acquire_skill_indexes(&mut conn, &as_of) {
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
        let mut pet_special_skill_ids = HashSet::new();
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
                if let Some(indexes) = acquire_skill_indexes.get(acquire_key) {
                    for index in indexes {
                        if let Some(skill_no) = equip_skill_by_index.get(index) {
                            skill_ids.insert(skill_no.clone());
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
                                &acquire_skill_indexes,
                                &equip_skill_by_index,
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
        build_tier_entry, calculator_pet_option_records, calculator_pet_special_option_records,
        dedupe_built_pet_entries, is_looting_pet_type, localized_pet_option_label,
        parse_asset_stem, parse_pet_option_effects, pet_image_url, pet_option_kind,
        pet_special_option_label, BuiltPetEntry, CalculatorPetOptionRecord, PetOptionKind,
        PetSpecialSkillMeta, RawPetRow,
    };

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
            &english_labels,
            &HashMap::new(),
        );

        let base_record = records.get("base:combat_exp").expect("base talent");
        assert_eq!(base_record.kind, PetOptionKind::Talent);
        assert_eq!(base_record.entry.label, "Combat EXP +5%");
        assert_eq!(
            records.get("equip:fishing_exp").unwrap().kind,
            PetOptionKind::Skill
        );
        assert!(!records.contains_key("equip:ignored"));
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
                kind: PetOptionKind::Talent,
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
            &HashMap::new(),
            &options_by_key,
        );

        assert_eq!(tier.key, "5");
        assert_eq!(tier.talents, vec!["talent:combat_exp".to_string()]);
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
        assert_eq!(record.kind, PetOptionKind::Special);
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
    fn dedupe_built_pet_entries_collapses_identical_pet_variants() {
        let duplicated = vec![
            BuiltPetEntry {
                entry: CalculatorPetEntry {
                    key: "pet:azure:38".to_string(),
                    label: "Young Azure Dragon".to_string(),
                    image_url: Some("/images/pets/pet_blue_dragon_0001.webp".to_string()),
                    lineage_keys: vec!["change-look:blue:38".to_string()],
                    tiers: vec![CalculatorPetTierEntry {
                        key: "5".to_string(),
                        label: "Tier 5".to_string(),
                        specials: vec!["special_a".to_string()],
                        talents: vec!["talent_a".to_string()],
                        skills: vec!["skill_a".to_string()],
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
                    tiers: vec![CalculatorPetTierEntry {
                        key: "5".to_string(),
                        label: "Tier 5".to_string(),
                        specials: vec!["special_a".to_string()],
                        talents: vec!["talent_a".to_string()],
                        skills: vec!["skill_a".to_string()],
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
    }
}
