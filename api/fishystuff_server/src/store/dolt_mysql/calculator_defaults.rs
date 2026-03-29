use fishystuff_api::models::calculator::{
    CalculatorLifeskillLevelEntry, CalculatorOptionEntry, CalculatorPetSignals,
    CalculatorSessionPresetEntry, CalculatorSignals,
};

use crate::store::FishLang;

pub(super) fn lifeskill_level_drr_from_index(index: i32) -> f32 {
    (0.1 + 0.005 * index as f32).min(0.6)
}

fn build_calculator_default_pet(tier: &str, special: &str) -> CalculatorPetSignals {
    CalculatorPetSignals {
        tier: tier.to_string(),
        special: special.to_string(),
        talent: "durability_reduction_resistance".to_string(),
        skills: vec!["fishing_exp".to_string()],
    }
}

fn localized_label(lang: FishLang, en: impl Into<String>, ko: impl Into<String>) -> String {
    match lang {
        FishLang::En => en.into(),
        FishLang::Ko => ko.into(),
    }
}

pub(super) fn build_calculator_default_signals() -> CalculatorSignals {
    CalculatorSignals {
        level: 5,
        lifeskill_level: "100".to_string(),
        mastery: 0.0,
        trade_level: "73".to_string(),
        zone: "240,74,74".to_string(),
        resources: 0.0,
        rod: "item:16162".to_string(),
        float: String::new(),
        chair: "item:705539".to_string(),
        lightstone_set: "lightstone-set:30".to_string(),
        backpack: "item:830150".to_string(),
        outfit: vec![
            "effect:8-piece-outfit-set-effect".to_string(),
            "effect:mainhand-weapon-outfit".to_string(),
            "effect:awakening-weapon-outfit".to_string(),
            "item:14330".to_string(),
        ],
        food: vec!["item:9359".to_string()],
        buff: vec!["".to_string(), "item:721092".to_string()],
        pet1: build_calculator_default_pet("5", "auto_fishing_time_reduction"),
        pet2: build_calculator_default_pet("4", ""),
        pet3: build_calculator_default_pet("4", ""),
        pet4: build_calculator_default_pet("4", ""),
        pet5: build_calculator_default_pet("4", ""),
        trade_distance_bonus: 134.15,
        trade_price_curve: 120.0,
        catch_time_active: 17.5,
        catch_time_afk: 6.5,
        timespan_amount: 8.0,
        timespan_unit: "hours".to_string(),
        apply_trade_modifiers: true,
        show_silver_amounts: false,
        discard_grade: "none".to_string(),
        brand: true,
        active: false,
        debug: false,
    }
}

pub(super) fn build_calculator_fishing_levels(lang: FishLang) -> Vec<CalculatorOptionEntry> {
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

pub(super) fn build_calculator_session_units(lang: FishLang) -> Vec<CalculatorOptionEntry> {
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

pub(super) fn build_calculator_trade_levels(lang: FishLang) -> Vec<CalculatorOptionEntry> {
    const TIERS: [(&str, &str, i32); 7] = [
        ("Beginner", "초급", 10),
        ("Apprentice", "견습", 10),
        ("Skilled", "숙련", 10),
        ("Professional", "전문", 10),
        ("Artisan", "장인", 10),
        ("Master", "명장", 30),
        ("Guru", "도인", 100),
    ];

    let mut levels = Vec::new();
    let mut order = 0i32;
    for (en_tier, ko_tier, max_level) in TIERS {
        for level in 1..=max_level {
            order += 1;
            levels.push(CalculatorOptionEntry {
                key: order.to_string(),
                label: localized_label(
                    lang,
                    &format!("{en_tier} {level}"),
                    &format!("{ko_tier} {level}"),
                ),
            });
        }
    }
    levels
}

pub(super) fn build_calculator_session_presets(
    lang: FishLang,
) -> Vec<CalculatorSessionPresetEntry> {
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

pub(super) fn build_calculator_lifeskill_levels() -> Vec<CalculatorLifeskillLevelEntry> {
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
                lifeskill_level_drr: lifeskill_level_drr_from_index(order.min(130)),
            });
        }
    }
    levels
}
