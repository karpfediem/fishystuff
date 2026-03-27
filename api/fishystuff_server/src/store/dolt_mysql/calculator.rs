use std::collections::HashMap;

use fishystuff_api::models::calculator::{
    CalculatorCatalogResponse, CalculatorItemEntry, CalculatorLifeskillLevelEntry,
    CalculatorOptionEntry, CalculatorPetCatalog, CalculatorPetSignals,
    CalculatorSessionPresetEntry, CalculatorSignals,
};
use fishystuff_core::fish_icons::parse_fish_icon_asset_id;
use mysql::prelude::Queryable;

use crate::error::AppResult;
use crate::store::{validate_dolt_ref, FishLang};

use super::util::{db_unavailable, is_missing_table, normalize_optional_string};
use super::DoltMySqlStore;

fn build_calculator_default_pet(tier: &str, special: &str) -> CalculatorPetSignals {
    CalculatorPetSignals {
        tier: tier.to_string(),
        special: special.to_string(),
        talent: "durability_reduction_resistance".to_string(),
        skills: vec!["fishing_exp".to_string()],
    }
}

fn build_calculator_default_signals() -> CalculatorSignals {
    CalculatorSignals {
        level: 5,
        lifeskill_level: "100".to_string(),
        zone: "240,74,74".to_string(),
        resources: 0.0,
        rod: "item:16162".to_string(),
        float: String::new(),
        chair: "item:705539".to_string(),
        lightstone_set: "effect:blacksmith-s-blessing".to_string(),
        backpack: "item:830150".to_string(),
        outfit: vec![
            "effect:8-piece-outfit-set-effect".to_string(),
            "effect:awakening-weapon-outfit".to_string(),
            "effect:mainhand-weapon-outfit".to_string(),
        ],
        food: vec!["item:9359".to_string()],
        buff: vec!["".to_string(), "item:721092".to_string()],
        pet1: build_calculator_default_pet("5", "auto_fishing_time_reduction"),
        pet2: build_calculator_default_pet("4", ""),
        pet3: build_calculator_default_pet("4", ""),
        pet4: build_calculator_default_pet("4", ""),
        pet5: build_calculator_default_pet("4", ""),
        catch_time_active: 17.5,
        catch_time_afk: 6.5,
        timespan_amount: 8.0,
        timespan_unit: "hours".to_string(),
        brand: true,
        active: false,
        debug: false,
    }
}

type CalculatorItemDbRow = (
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

type CalculatorPetOptionDbRow = (Option<String>, Option<String>, Option<String>);

#[derive(Debug, Clone, Default)]
struct CalculatorItemSourceMetadata {
    name_ko: Option<String>,
    durability: Option<i32>,
    icon_id: Option<i32>,
}

fn localized_label(lang: FishLang, en: &'static str, ko: &'static str) -> String {
    match lang {
        FishLang::En => en.to_string(),
        FishLang::Ko => ko.to_string(),
    }
}

fn push_unique_option(options: &mut Vec<CalculatorOptionEntry>, key: &str, label: String) {
    if options.iter().any(|option| option.key == key) {
        return;
    }
    options.push(CalculatorOptionEntry {
        key: key.to_string(),
        label,
    });
}

fn canonical_pet_option_key(
    option_kind: &str,
    skill_name_ko: Option<&str>,
    skill_description_ko: Option<&str>,
) -> Option<&'static str> {
    let text = [
        skill_name_ko.unwrap_or(""),
        skill_description_ko.unwrap_or(""),
    ]
    .join("\n");
    if text.contains("자동 낚시") {
        return Some("auto_fishing_time_reduction");
    }
    if text.contains("내구도 소모 감소 저항") {
        return Some("durability_reduction_resistance");
    }
    if text.contains("낚시 경험치") {
        return Some("fishing_exp");
    }
    if text.contains("생활 경험치") {
        return Some("life_exp");
    }
    if option_kind == "special" {
        return Some("auto_fishing_time_reduction");
    }
    None
}

fn calculator_pet_option_label(lang: FishLang, key: &str) -> String {
    match key {
        "auto_fishing_time_reduction" => {
            localized_label(lang, "Auto-Fishing Time Reduction", "자동 낚시 시간 감소")
        }
        "durability_reduction_resistance" => localized_label(
            lang,
            "Durability Reduction Resistance",
            "내구도 소모 감소 저항",
        ),
        "life_exp" => localized_label(lang, "Life EXP", "생활 경험치"),
        "fishing_exp" => localized_label(lang, "Fishing EXP", "낚시 경험치"),
        _ => key.to_string(),
    }
}

fn build_calculator_fishing_levels(lang: FishLang) -> Vec<CalculatorOptionEntry> {
    (0..=5)
        .map(|level| CalculatorOptionEntry {
            key: level.to_string(),
            label: match lang {
                FishLang::En => format!("Level {level}"),
                FishLang::Ko => format!("낚시 {level}단계"),
            },
        })
        .collect()
}

fn build_calculator_session_units(lang: FishLang) -> Vec<CalculatorOptionEntry> {
    [
        ("minutes", "Minutes", "분"),
        ("hours", "Hours", "시간"),
        ("days", "Days", "일"),
        ("weeks", "Weeks", "주"),
    ]
    .into_iter()
    .map(|(key, en, ko)| CalculatorOptionEntry {
        key: key.to_string(),
        label: localized_label(lang, en, ko),
    })
    .collect()
}

fn build_calculator_session_presets(lang: FishLang) -> Vec<CalculatorSessionPresetEntry> {
    [
        ("1 hour", "1시간", 1.0, "hours"),
        ("8 hours", "8시간", 8.0, "hours"),
        ("10 hours", "10시간", 10.0, "hours"),
        ("12 hours", "12시간", 12.0, "hours"),
        ("1 day", "1일", 1.0, "days"),
    ]
    .into_iter()
    .map(|(en, ko, amount, unit)| CalculatorSessionPresetEntry {
        label: localized_label(lang, en, ko),
        amount,
        unit: unit.to_string(),
    })
    .collect()
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
    let specials = vec![
        CalculatorOptionEntry {
            key: String::new(),
            label: localized_label(lang, "None", "없음"),
        },
        CalculatorOptionEntry {
            key: "auto_fishing_time_reduction".to_string(),
            label: localized_label(lang, "Auto-Fishing Time Reduction", "자동 낚시 시간 감소"),
        },
    ];
    let talents = vec![
        CalculatorOptionEntry {
            key: String::new(),
            label: localized_label(lang, "None", "없음"),
        },
        CalculatorOptionEntry {
            key: "durability_reduction_resistance".to_string(),
            label: localized_label(
                lang,
                "Durability Reduction Resistance",
                "내구도 소모 감소 저항",
            ),
        },
    ];
    let skills = vec![CalculatorOptionEntry {
        key: "fishing_exp".to_string(),
        label: localized_label(lang, "Fishing EXP", "낚시 경험치"),
    }];

    CalculatorPetCatalog {
        slots: 5,
        tiers,
        specials,
        talents,
        skills,
    }
}

fn calculator_item_icon_path(icon_id: i32) -> String {
    format!("/img/items/{icon_id:08}.webp")
}

fn slugify_calculator_effect_key(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut last_was_dash = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

fn build_calculator_lifeskill_levels() -> Vec<CalculatorLifeskillLevelEntry> {
    const TIERS: [(&str, i32); 7] = [
        ("Beginner", 10),
        ("Apprentice", 10),
        ("Skilled", 10),
        ("Professional", 10),
        ("Artisan", 10),
        ("Master", 30),
        ("Guru", 100),
    ];

    let mut levels = Vec::new();
    let mut order = 0i32;
    for (tier_name, max_level) in TIERS {
        for level in 1..=max_level {
            order += 1;
            levels.push(CalculatorLifeskillLevelEntry {
                key: order.to_string(),
                name: format!("{tier_name} {level}"),
                index: order.min(130),
                order,
            });
        }
    }
    levels
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
                    icon_id: normalize_optional_string(icon_file)
                        .and_then(|value| parse_fish_icon_asset_id(&value)),
                },
            );
        }
        Ok(out)
    }

    fn query_legacy_calculator_items(
        &self,
        lang: FishLang,
        ref_id: Option<&str>,
    ) -> AppResult<Vec<CalculatorItemEntry>> {
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
        let rows: Vec<CalculatorItemDbRow> = conn.query(query).map_err(db_unavailable)?;

        let item_ids = rows.iter().filter_map(|row| row.10).collect::<Vec<_>>();
        let item_source_metadata = self.query_calculator_item_table_metadata(ref_id, &item_ids)?;
        let lightstone_names_ko = if matches!(lang, FishLang::Ko) {
            self.query_calculator_lightstone_name_overrides_ko(ref_id)?
        } else {
            HashMap::new()
        };

        let mut items = Vec::with_capacity(rows.len());
        for (
            name,
            item_type,
            afr,
            bonus_rare,
            bonus_big,
            durability,
            drr,
            fish_multiplier,
            exp_fish,
            exp_life,
            item_id,
            icon_id,
        ) in rows
        {
            let Some(legacy_name) = normalize_optional_string(name) else {
                continue;
            };
            let item_type = normalize_optional_string(item_type).unwrap_or_default();
            let display_name = item_id
                .and_then(|item_id| {
                    if matches!(lang, FishLang::Ko) {
                        item_source_metadata
                            .get(&item_id)
                            .and_then(|metadata| metadata.name_ko.clone())
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    if item_type == "lightstone_set" {
                        lightstone_names_ko.get(&legacy_name).cloned()
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| legacy_name.clone());
            let key = if let Some(item_id) = item_id {
                format!("item:{item_id}")
            } else {
                format!("effect:{}", slugify_calculator_effect_key(&legacy_name))
            };
            let icon_id = item_id
                .and_then(|item_id| {
                    item_source_metadata
                        .get(&item_id)
                        .and_then(|metadata| metadata.icon_id)
                })
                .or(icon_id)
                .or(item_id);
            let icon = icon_id.map(calculator_item_icon_path);
            items.push(CalculatorItemEntry {
                key,
                name: display_name,
                r#type: item_type,
                afr,
                bonus_rare,
                bonus_big,
                durability: item_id
                    .and_then(|item_id| {
                        item_source_metadata
                            .get(&item_id)
                            .and_then(|metadata| metadata.durability)
                    })
                    .or(durability),
                drr,
                fish_multiplier,
                exp_fish,
                exp_life,
                item_id,
                icon_id,
                icon,
            });
        }

        Ok(items)
    }

    fn build_source_consumable_items(
        &self,
        ref_id: Option<&str>,
        legacy_items: &[CalculatorItemEntry],
    ) -> AppResult<Vec<CalculatorItemEntry>> {
        let override_item_ids = legacy_items
            .iter()
            .filter(|item| matches!(item.r#type.as_str(), "food" | "buff"))
            .filter_map(|item| item.item_id)
            .collect::<Vec<_>>();
        let consumable_overrides =
            self.query_calculator_consumable_effect_overrides(ref_id, &override_item_ids)?;
        let mut items = Vec::new();
        for item in legacy_items {
            let Some(item_id) = item.item_id else {
                continue;
            };
            let Some(override_values) = consumable_overrides.get(&item_id).copied() else {
                continue;
            };
            let mut sourced = item.clone();
            sourced.afr = override_values.afr;
            sourced.bonus_rare = override_values.bonus_rare;
            sourced.bonus_big = override_values.bonus_big;
            sourced.drr = override_values.drr;
            sourced.exp_fish = override_values.exp_fish;
            sourced.exp_life = override_values.exp_life;
            items.push(sourced);
        }
        Ok(items)
    }

    fn build_source_lightstone_items(
        &self,
        lang: FishLang,
        ref_id: Option<&str>,
        legacy_items: &[CalculatorItemEntry],
    ) -> AppResult<Vec<CalculatorItemEntry>> {
        let lightstone_overrides = self.query_calculator_lightstone_effect_overrides(ref_id)?;
        let lightstone_names_ko = if matches!(lang, FishLang::Ko) {
            self.query_calculator_lightstone_name_overrides_ko(ref_id)?
        } else {
            HashMap::new()
        };
        let mut items = Vec::new();
        for item in legacy_items {
            if item.r#type != "lightstone_set" {
                continue;
            }
            let Some(override_values) = lightstone_overrides.get(item.name.as_str()).copied()
            else {
                continue;
            };
            let mut sourced = item.clone();
            if let Some(name_ko) = lightstone_names_ko.get(item.name.as_str()) {
                sourced.name = name_ko.clone();
            }
            sourced.afr = override_values.afr;
            sourced.bonus_rare = override_values.bonus_rare;
            sourced.bonus_big = override_values.bonus_big;
            sourced.drr = override_values.drr;
            sourced.exp_fish = override_values.exp_fish;
            sourced.exp_life = override_values.exp_life;
            items.push(sourced);
        }
        Ok(items)
    }

    fn merge_calculator_items(
        &self,
        legacy_items: Vec<CalculatorItemEntry>,
        sourced_items: Vec<CalculatorItemEntry>,
    ) -> Vec<CalculatorItemEntry> {
        let mut merged = HashMap::<String, CalculatorItemEntry>::new();
        for item in legacy_items {
            merged.insert(item.key.clone(), item);
        }
        for item in sourced_items {
            merged.insert(item.key.clone(), item);
        }

        let mut items = merged.into_values().collect::<Vec<_>>();

        items.sort_by(|left, right| {
            left.r#type
                .cmp(&right.r#type)
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
                .then_with(|| left.key.cmp(&right.key))
        });

        items
    }

    fn query_calculator_items(
        &self,
        lang: FishLang,
        ref_id: Option<&str>,
    ) -> AppResult<Vec<CalculatorItemEntry>> {
        let legacy_items = self.query_legacy_calculator_items(lang, ref_id)?;
        let mut sourced_items = self.build_source_consumable_items(ref_id, &legacy_items)?;
        sourced_items.extend(self.build_source_lightstone_items(lang, ref_id, &legacy_items)?);
        Ok(self.merge_calculator_items(legacy_items, sourced_items))
    }

    fn query_calculator_pet_catalog(
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
        let query = format!(
            "SELECT \
                option_kind, \
                skill_name_ko, \
                skill_description_ko \
             FROM calculator_pet_skill_options{as_of}"
        );

        let mut conn = self.pool.get_conn().map_err(db_unavailable)?;
        let rows: Vec<CalculatorPetOptionDbRow> = match conn.query(query) {
            Ok(rows) => rows,
            Err(err)
                if is_missing_table(&err, "calculator_pet_skill_options")
                    || is_missing_table(&err, "skilltype_table_new")
                    || is_missing_table(&err, "pet_base_skill_table")
                    || is_missing_table(&err, "pet_equipskill_table")
                    || is_missing_table(&err, "pet_setstats_table") =>
            {
                return Ok(catalog);
            }
            Err(err) => return Err(db_unavailable(err)),
        };

        for (option_kind, skill_name_ko, skill_description_ko) in rows {
            let option_kind = normalize_optional_string(option_kind).unwrap_or_default();
            let skill_name_ko = normalize_optional_string(skill_name_ko);
            let skill_description_ko = normalize_optional_string(skill_description_ko);
            let Some(key) = canonical_pet_option_key(
                &option_kind,
                skill_name_ko.as_deref(),
                skill_description_ko.as_deref(),
            ) else {
                continue;
            };
            let label = calculator_pet_option_label(lang, key);
            match key {
                "auto_fishing_time_reduction" => {
                    push_unique_option(&mut catalog.specials, key, label);
                }
                "durability_reduction_resistance" | "life_exp" => {
                    push_unique_option(&mut catalog.talents, key, label);
                }
                "fishing_exp" => {
                    push_unique_option(&mut catalog.skills, key, label);
                }
                _ => {}
            }
        }

        Ok(catalog)
    }

    pub(super) fn query_calculator_catalog(
        &self,
        lang: FishLang,
        ref_id: Option<&str>,
    ) -> AppResult<CalculatorCatalogResponse> {
        Ok(CalculatorCatalogResponse {
            items: self.query_calculator_items(lang, ref_id)?,
            lifeskill_levels: build_calculator_lifeskill_levels(),
            fishing_levels: build_calculator_fishing_levels(lang),
            session_units: build_calculator_session_units(lang),
            session_presets: build_calculator_session_presets(lang),
            pets: self.query_calculator_pet_catalog(lang, ref_id)?,
            defaults: build_calculator_default_signals(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::store::FishLang;

    use super::super::calculator_effects::{
        extract_first_number, legacy_lightstone_name_for_source_name_ko,
        parse_calculator_effect_text, CalculatorItemEffectValues,
    };
    use super::{calculator_pet_option_label, canonical_pet_option_key};

    #[test]
    fn extract_first_number_handles_signed_percent_lines() {
        assert_eq!(extract_first_number("자동 낚시 시간 -15%"), Some(-15.0));
        assert_eq!(extract_first_number("낚시 경험치 획득량 +10%"), Some(10.0));
        assert_eq!(extract_first_number("생활 숙련도 +20"), Some(20.0));
        assert_eq!(extract_first_number("효과 없음"), None);
    }

    #[test]
    fn calculator_effect_text_parses_balacs_style_lines() {
        let mut values = CalculatorItemEffectValues::default();
        parse_calculator_effect_text(
            &mut values,
            "자동 낚시 시간 감소 7%\n낚시 경험치 획득량 +10%",
        );

        assert_eq!(
            values,
            CalculatorItemEffectValues {
                afr: Some(0.07),
                exp_fish: Some(0.10),
                ..CalculatorItemEffectValues::default()
            }
        );
    }

    #[test]
    fn calculator_effect_text_parses_event_food_and_housekeeper_lines() {
        let mut values = CalculatorItemEffectValues::default();
        parse_calculator_effect_text(&mut values, "생활 숙련도 +50\n생활 경험치 획득량 +20%");

        assert_eq!(
            values,
            CalculatorItemEffectValues {
                exp_life: Some(0.20),
                ..CalculatorItemEffectValues::default()
            }
        );

        let mut event_food = CalculatorItemEffectValues::default();
        parse_calculator_effect_text(
            &mut event_food,
            "자동 낚시 시간 -10%\n생활 경험치 획득량 +50%\n생활 숙련도 +100",
        );

        assert_eq!(
            event_food,
            CalculatorItemEffectValues {
                afr: Some(0.10),
                exp_life: Some(0.50),
                ..CalculatorItemEffectValues::default()
            }
        );
    }

    #[test]
    fn legacy_lightstone_names_map_to_current_calculator_entries() {
        assert_eq!(
            legacy_lightstone_name_for_source_name_ko("신의 입질"),
            Some("Nibbles")
        );
        assert_eq!(
            legacy_lightstone_name_for_source_name_ko("고래의 입"),
            Some("Whaling")
        );
        assert_eq!(
            legacy_lightstone_name_for_source_name_ko("예리한 갈매기"),
            Some("Sharp-Eyed Seagull")
        );
        assert_eq!(
            legacy_lightstone_name_for_source_name_ko("선택과 집중 : 낚시"),
            Some("Choice & Focus: Fishing")
        );
        assert_eq!(legacy_lightstone_name_for_source_name_ko("없는 세트"), None);
    }

    #[test]
    fn canonical_pet_option_key_maps_current_source_rows() {
        assert_eq!(
            canonical_pet_option_key("skill", Some("낚시 경험치 획득량 증가 +5%"), None),
            Some("fishing_exp")
        );
        assert_eq!(
            canonical_pet_option_key("talent", Some("생활 경험치 획득량 증가 +5%"), None),
            Some("life_exp")
        );
        assert_eq!(
            canonical_pet_option_key("special", Some("자동 낚시 시간 감소 5%"), None),
            Some("auto_fishing_time_reduction")
        );
        assert_eq!(
            canonical_pet_option_key("talent", Some("내구도 소모 감소 저항 +5%"), None),
            Some("durability_reduction_resistance")
        );
        assert_eq!(
            canonical_pet_option_key("other", Some("낚시 1단계 상승"), None),
            None
        );
    }

    #[test]
    fn calculator_pet_option_label_localizes_known_keys() {
        assert_eq!(
            calculator_pet_option_label(FishLang::En, "fishing_exp"),
            "Fishing EXP"
        );
        assert_eq!(
            calculator_pet_option_label(FishLang::Ko, "life_exp"),
            "생활 경험치"
        );
    }
}
