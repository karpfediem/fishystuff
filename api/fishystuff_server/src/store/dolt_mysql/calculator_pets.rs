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
);
type CalculatorPetSkillIndexDbRow = (Option<String>, Option<String>);
type CalculatorLanguagedataDbRow = (Option<String>, Option<String>);
type CalculatorPetSkilltypeDbRow = (
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
    base_skill_index: Option<String>,
    acquire_key: Option<String>,
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
             WHERE TRIM(COALESCE(`unk`, '')) = '{}' \
               AND TRIM(COALESCE(`id`, '')) IN ({})",
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
             WHERE TRIM(COALESCE(`SkillNo`, '')) IN ({})",
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

fn build_tier_entry(
    lang: FishLang,
    tier_source: u8,
    representative: &RawPetRow,
    base_skill_by_index: &HashMap<String, String>,
    acquire_skill_indexes: &HashMap<String, HashSet<String>>,
    equip_skill_by_index: &HashMap<String, String>,
    options_by_key: &HashMap<String, CalculatorPetOptionRecord>,
) -> CalculatorPetTierEntry {
    let mut specials = Vec::new();
    let mut talents = Vec::new();
    let mut skills = Vec::new();

    if let Some(skill_index) = representative.base_skill_index.as_ref() {
        if let Some(skill_no) = base_skill_by_index.get(skill_index) {
            if options_by_key.contains_key(skill_no) {
                talents.push(skill_no.clone());
            }
        }
    }

    if let Some(acquire_key) = representative.acquire_key.as_ref() {
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
            built
        })
        .collect()
}

fn calculator_pet_option_records(
    lang: FishLang,
    skill_ids: &HashSet<String>,
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
        let Some(kind) = pet_option_kind(&effects) else {
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
        for row in &pet_rows {
            if let Some(skill_index) = row.base_skill_index.as_ref() {
                if let Some(skill_no) = base_skill_by_index.get(skill_index) {
                    skill_ids.insert(skill_no.clone());
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
        let options_by_key =
            calculator_pet_option_records(lang, &skill_ids, &english_skill_labels, &skilltype_meta);

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
    use fishystuff_api::models::calculator::{CalculatorPetEntry, CalculatorPetTierEntry};

    use crate::store::FishLang;

    use super::{
        dedupe_built_pet_entries, is_looting_pet_type, localized_pet_option_label,
        parse_asset_stem, parse_pet_option_effects, pet_image_url, pet_option_kind, BuiltPetEntry,
        PetOptionKind,
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
    }
}
