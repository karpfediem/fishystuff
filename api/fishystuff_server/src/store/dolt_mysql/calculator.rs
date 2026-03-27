use fishystuff_api::models::calculator::{
    CalculatorCatalogResponse, CalculatorLifeskillLevelEntry, CalculatorOptionEntry,
    CalculatorPetSignals, CalculatorSessionPresetEntry, CalculatorSignals,
};

use crate::error::AppResult;
use crate::store::FishLang;

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

fn localized_label(lang: FishLang, en: &'static str, ko: &'static str) -> String {
    match lang {
        FishLang::En => en.to_string(),
        FishLang::Ko => ko.to_string(),
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
    use super::super::calculator_effects::{
        extract_first_number, legacy_lightstone_name_for_source_name_ko,
        parse_calculator_effect_text, CalculatorItemEffectValues,
    };

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
}
