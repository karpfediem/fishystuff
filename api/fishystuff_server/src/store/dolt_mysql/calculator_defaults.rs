use std::cmp::Ordering;

use fishystuff_api::models::calculator::{
    CalculatorLifeskillLevelEntry, CalculatorOptionEntry, CalculatorPetCatalog,
    CalculatorPetOptionEntry, CalculatorPetSignals, CalculatorPetTierEntry,
    CalculatorSessionPresetEntry, CalculatorSignals,
};

use crate::store::FishLang;

pub(super) fn lifeskill_level_drr_from_index(index: i32) -> f32 {
    (0.1 + 0.005 * index as f32).min(0.6)
}

const DEFAULT_LAHTRON_PET_KEY: &str = "pet:65:1:pet_ato_ratron_0001";

fn pet_option_by_key<'a>(
    options: &'a [CalculatorPetOptionEntry],
    key: &str,
) -> Option<&'a CalculatorPetOptionEntry> {
    options.iter().find(|option| option.key == key)
}

fn pet_skill_limit_for_tier_key(tier_key: &str) -> usize {
    match tier_key.trim() {
        "1" | "2" => 1,
        "3" => 2,
        "4" | "5" => 3,
        _ => 1,
    }
}

fn compare_chance_desc(left: f32, right: f32) -> Ordering {
    right.partial_cmp(&left).unwrap_or(Ordering::Equal)
}

fn default_pet_skill_keys(
    catalog: &CalculatorPetCatalog,
    tier: &CalculatorPetTierEntry,
) -> Vec<String> {
    let skill_limit = pet_skill_limit_for_tier_key(&tier.key);
    let mut selected = Vec::new();

    let mut fishing_candidates = tier
        .skills
        .iter()
        .filter_map(|key| {
            let option = pet_option_by_key(&catalog.skills, key)?;
            let fishing_exp = option.fishing_exp.filter(|value| *value > 0.0)?;
            Some((
                key.as_str(),
                fishing_exp,
                tier.skill_chances.get(key).copied().unwrap_or_default(),
            ))
        })
        .collect::<Vec<_>>();
    fishing_candidates.sort_by(|left, right| {
        compare_chance_desc(left.1, right.1)
            .then_with(|| compare_chance_desc(left.2, right.2))
            .then_with(|| left.0.cmp(right.0))
    });
    if let Some((key, _, _)) = fishing_candidates.first() {
        selected.push((*key).to_string());
    }

    let mut candidates = tier
        .skills
        .iter()
        .filter(|key| !selected.iter().any(|selected_key| selected_key == *key))
        .map(|key| {
            (
                key.as_str(),
                tier.skill_chances.get(key).copied().unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        compare_chance_desc(left.1, right.1).then_with(|| left.0.cmp(right.0))
    });
    selected.extend(
        candidates
            .into_iter()
            .map(|(key, _)| key.to_string())
            .take(skill_limit.saturating_sub(selected.len())),
    );
    selected.truncate(skill_limit);
    selected
}

fn build_calculator_default_lahtron_pet(
    catalog: &CalculatorPetCatalog,
    tier_key: &str,
    pack_leader: bool,
) -> CalculatorPetSignals {
    let mut pet = CalculatorPetSignals {
        tier: tier_key.to_string(),
        ..CalculatorPetSignals::default()
    };
    let Some(entry) = catalog
        .pets
        .iter()
        .find(|entry| entry.key == DEFAULT_LAHTRON_PET_KEY)
    else {
        return pet;
    };
    let Some(tier) = entry.tiers.iter().find(|tier| tier.key == tier_key) else {
        return pet;
    };

    pet.pet = entry.key.clone();
    pet.pack_leader = pack_leader && tier.key == "5";
    pet.special = tier.specials.first().cloned().unwrap_or_default();
    pet.talent = tier.talents.first().cloned().unwrap_or_default();
    pet.skills = default_pet_skill_keys(catalog, tier);
    pet
}

fn localized_label(lang: FishLang, en: impl Into<String>, ko: impl Into<String>) -> String {
    match lang {
        FishLang::En => en.into(),
        FishLang::Ko => ko.into(),
    }
}

pub(super) fn build_calculator_default_signals(pets: &CalculatorPetCatalog) -> CalculatorSignals {
    CalculatorSignals {
        level: 5,
        lifeskill_level: "100".to_string(),
        mastery: 2500.0,
        trade_level: "73".to_string(),
        zone: "240,74,74".to_string(),
        resources: 0.0,
        fishing_mode: "rod".to_string(),
        rod: "item:16162".to_string(),
        float: String::new(),
        chair: "item:705539".to_string(),
        lightstone_set: "lightstone-set:30".to_string(),
        backpack: "item:830150".to_string(),
        target_fish: String::new(),
        target_fish_amount: 1.0,
        target_fish_pmf_count: 0.0,
        outfit: vec![
            "effect:8-piece-outfit-set-effect".to_string(),
            "effect:mainhand-weapon-outfit".to_string(),
            "effect:awakening-weapon-outfit".to_string(),
            "item:14330".to_string(),
        ],
        food: vec!["item:9359".to_string()],
        buff: vec!["".to_string(), "item:721092".to_string()],
        pet1: build_calculator_default_lahtron_pet(pets, "5", true),
        pet2: build_calculator_default_lahtron_pet(pets, "4", false),
        pet3: build_calculator_default_lahtron_pet(pets, "4", false),
        pet4: build_calculator_default_lahtron_pet(pets, "4", false),
        pet5: build_calculator_default_lahtron_pet(pets, "4", false),
        trade_distance_bonus: 134.15,
        trade_price_curve: 120.0,
        price_overrides: Default::default(),
        overlay: Default::default(),
        catch_time_active: 17.5,
        catch_time_afk: 6.5,
        timespan_amount: 8.0,
        timespan_unit: "hours".to_string(),
        apply_trade_modifiers: true,
        show_silver_amounts: true,
        show_normalized_select_rates: true,
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

#[cfg(test)]
mod tests {
    use fishystuff_api::models::calculator::CalculatorPetEntry;

    use super::*;

    #[test]
    fn calculator_default_signals_use_lahtron_pet_setup() {
        let catalog = CalculatorPetCatalog {
            pets: vec![CalculatorPetEntry {
                key: DEFAULT_LAHTRON_PET_KEY.to_string(),
                label: "Lahtron".to_string(),
                tiers: vec![
                    CalculatorPetTierEntry {
                        key: "4".to_string(),
                        specials: vec!["pet-special:37".to_string()],
                        talents: vec!["49084".to_string()],
                        skills: vec![
                            "49033".to_string(),
                            "49018".to_string(),
                            "49024".to_string(),
                            "49015".to_string(),
                            "49021".to_string(),
                        ],
                        skill_chances: [
                            ("49033".to_string(), 0.01),
                            ("49018".to_string(), 0.15),
                            ("49024".to_string(), 0.15),
                            ("49015".to_string(), 0.16),
                            ("49021".to_string(), 0.15),
                        ]
                        .into(),
                        ..CalculatorPetTierEntry::default()
                    },
                    CalculatorPetTierEntry {
                        key: "5".to_string(),
                        specials: vec!["pet-special:37".to_string()],
                        talents: vec!["49084".to_string()],
                        skills: vec![
                            "49033".to_string(),
                            "49018".to_string(),
                            "49024".to_string(),
                            "49015".to_string(),
                            "49021".to_string(),
                        ],
                        skill_chances: [
                            ("49033".to_string(), 0.01),
                            ("49018".to_string(), 0.15),
                            ("49024".to_string(), 0.15),
                            ("49015".to_string(), 0.16),
                            ("49021".to_string(), 0.15),
                        ]
                        .into(),
                        ..CalculatorPetTierEntry::default()
                    },
                ],
                ..CalculatorPetEntry::default()
            }],
            skills: vec![
                CalculatorPetOptionEntry {
                    key: "49033".to_string(),
                    label: "Alchemy EXP +7%".to_string(),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "49018".to_string(),
                    label: "Combat EXP +7%".to_string(),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "49024".to_string(),
                    label: "Fishing EXP +7%".to_string(),
                    fishing_exp: Some(0.07),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "49015".to_string(),
                    label: "Karma Recovery +7%".to_string(),
                    ..CalculatorPetOptionEntry::default()
                },
                CalculatorPetOptionEntry {
                    key: "49021".to_string(),
                    label: "Gathering EXP +7%".to_string(),
                    ..CalculatorPetOptionEntry::default()
                },
            ],
            ..CalculatorPetCatalog::default()
        };
        let defaults = build_calculator_default_signals(&catalog);
        let pets = [
            &defaults.pet1,
            &defaults.pet2,
            &defaults.pet3,
            &defaults.pet4,
            &defaults.pet5,
        ];

        assert_eq!(defaults.pet1.tier, "5");
        assert!(defaults.pet1.pack_leader);
        for pet in pets {
            assert_eq!(pet.pet, DEFAULT_LAHTRON_PET_KEY);
            assert_eq!(pet.special, "pet-special:37");
            assert_eq!(pet.talent, "49084");
            assert_eq!(
                pet.skills,
                vec![
                    "49024".to_string(),
                    "49015".to_string(),
                    "49018".to_string(),
                ]
            );
        }
        for pet in [
            &defaults.pet2,
            &defaults.pet3,
            &defaults.pet4,
            &defaults.pet5,
        ] {
            assert_eq!(pet.tier, "4");
            assert!(!pet.pack_leader);
        }
    }
}
