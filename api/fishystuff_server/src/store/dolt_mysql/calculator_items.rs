use std::collections::HashMap;

use fishystuff_api::models::calculator::CalculatorItemEntry;
use fishystuff_core::fish_icons::parse_fish_icon_asset_id;

use crate::error::AppResult;
use crate::store::FishLang;

use super::calculator_effects::CalculatorItemEffectValues;
use super::calculator_sources::{
    CalculatorCatalogSourceData, CalculatorItemDbRow, CalculatorItemSourceMetadata,
    CalculatorSourceBackedItemRow,
};
use super::util::normalize_optional_string;
use super::DoltMySqlStore;

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

pub(super) fn build_source_item(
    lang: FishLang,
    item_id: i32,
    item_type: &str,
    source_name_en: Option<&str>,
    source_name_ko: Option<&str>,
    item_icon_file: Option<&str>,
    icon_id: Option<i32>,
    fish_multiplier: Option<f32>,
    source_durability: Option<i32>,
    override_values: CalculatorItemEffectValues,
) -> CalculatorItemEntry {
    let name = if matches!(lang, FishLang::Ko) {
        source_name_ko
            .map(ToOwned::to_owned)
            .or_else(|| source_name_en.map(ToOwned::to_owned))
            .unwrap_or_else(|| format!("item:{item_id}"))
    } else {
        source_name_en
            .map(ToOwned::to_owned)
            .or_else(|| source_name_ko.map(ToOwned::to_owned))
            .unwrap_or_else(|| format!("item:{item_id}"))
    };
    let icon_id = item_icon_file
        .and_then(parse_fish_icon_asset_id)
        .or(icon_id)
        .or(Some(item_id));

    CalculatorItemEntry {
        key: format!("item:{item_id}"),
        name,
        r#type: item_type.to_string(),
        afr: override_values.afr,
        bonus_rare: override_values.bonus_rare,
        bonus_big: override_values.bonus_big,
        durability: source_durability,
        drr: override_values.drr,
        fish_multiplier,
        exp_fish: override_values.exp_fish,
        exp_life: override_values.exp_life,
        item_id: Some(item_id),
        icon_id,
        icon: icon_id.map(calculator_item_icon_path),
    }
}

fn source_backed_effect_values(row: &CalculatorSourceBackedItemRow) -> CalculatorItemEffectValues {
    let mut values = CalculatorItemEffectValues {
        afr: row.afr,
        bonus_rare: row.bonus_rare,
        bonus_big: row.bonus_big,
        drr: row.drr,
        exp_fish: row.exp_fish,
        exp_life: row.exp_life,
    };
    if let Some(effect_description) = row.effect_description_ko.as_deref() {
        let mut parsed = CalculatorItemEffectValues::default();
        super::calculator_effects::parse_calculator_effect_text(&mut parsed, effect_description);
        values.afr = values.afr.or(parsed.afr);
        values.bonus_rare = values.bonus_rare.or(parsed.bonus_rare);
        values.bonus_big = values.bonus_big.or(parsed.bonus_big);
        values.drr = values.drr.or(parsed.drr);
        values.exp_fish = values.exp_fish.or(parsed.exp_fish);
        values.exp_life = values.exp_life.or(parsed.exp_life);
    }
    values
}

pub(super) fn build_source_lightstone_item(
    lang: FishLang,
    source_key: &str,
    source_name_en: Option<&str>,
    name_ko: Option<&str>,
    item_type: &str,
    item_icon_file: Option<&str>,
    durability: Option<i32>,
    fish_multiplier: Option<f32>,
    override_values: CalculatorItemEffectValues,
) -> CalculatorItemEntry {
    let name = if matches!(lang, FishLang::Ko) {
        name_ko.map(ToOwned::to_owned).unwrap_or_else(|| {
            source_name_en
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| source_key.to_string())
        })
    } else {
        source_name_en
            .map(ToOwned::to_owned)
            .or_else(|| name_ko.map(ToOwned::to_owned))
            .unwrap_or_else(|| source_key.to_string())
    };
    let icon_id = item_icon_file.and_then(parse_fish_icon_asset_id);

    CalculatorItemEntry {
        key: source_key.to_string(),
        name,
        r#type: item_type.to_string(),
        afr: override_values.afr,
        bonus_rare: override_values.bonus_rare,
        bonus_big: override_values.bonus_big,
        durability,
        drr: override_values.drr,
        fish_multiplier,
        exp_fish: override_values.exp_fish,
        exp_life: override_values.exp_life,
        item_id: None,
        icon_id,
        icon: icon_id.map(calculator_item_icon_path),
    }
}

impl DoltMySqlStore {
    fn build_legacy_calculator_items(
        &self,
        lang: FishLang,
        rows: Vec<CalculatorItemDbRow>,
        item_source_metadata: &HashMap<i32, CalculatorItemSourceMetadata>,
    ) -> Vec<CalculatorItemEntry> {
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

        items
    }

    fn build_source_backed_items(
        lang: FishLang,
        source_backed_rows: &[CalculatorSourceBackedItemRow],
    ) -> AppResult<Vec<CalculatorItemEntry>> {
        let mut items = Vec::new();
        for row in source_backed_rows {
            let override_values = source_backed_effect_values(row);
            if override_values == CalculatorItemEffectValues::default() {
                continue;
            }
            match row.source_kind.as_str() {
                "item" => {
                    let Some(item_id) = row.item_id else {
                        continue;
                    };
                    items.push(build_source_item(
                        lang,
                        item_id,
                        &row.item_type,
                        row.source_name_en.as_deref(),
                        row.source_name_ko.as_deref(),
                        row.item_icon_file.as_deref(),
                        row.icon_id,
                        row.fish_multiplier,
                        row.durability,
                        override_values,
                    ));
                }
                "lightstone_set" => {
                    items.push(build_source_lightstone_item(
                        lang,
                        &row.source_key,
                        row.source_name_en.as_deref(),
                        row.source_name_ko.as_deref(),
                        &row.item_type,
                        row.item_icon_file.as_deref(),
                        row.durability,
                        row.fish_multiplier,
                        override_values,
                    ));
                }
                _ => {}
            }
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

    pub(super) fn query_calculator_items(
        &self,
        lang: FishLang,
        ref_id: Option<&str>,
    ) -> AppResult<Vec<CalculatorItemEntry>> {
        let CalculatorCatalogSourceData {
            legacy_rows,
            item_source_metadata,
            source_backed_rows,
        } = self.query_calculator_catalog_source_data(ref_id)?;
        let legacy_items =
            self.build_legacy_calculator_items(lang, legacy_rows, &item_source_metadata);
        let sourced_items = Self::build_source_backed_items(lang, &source_backed_rows)?;
        Ok(self.merge_calculator_items(legacy_items, sourced_items))
    }
}

#[cfg(test)]
mod tests {
    use crate::store::FishLang;

    use super::super::calculator_effects::CalculatorItemEffectValues;
    use super::{
        build_source_item, build_source_lightstone_item, source_backed_effect_values,
        CalculatorSourceBackedItemRow, DoltMySqlStore,
    };

    #[test]
    fn source_consumable_item_prefers_source_metadata() {
        let sourced = build_source_item(
            FishLang::Ko,
            9359,
            "food",
            Some("Balacs Lunchbox"),
            Some("발락스 도시락"),
            Some("00009359.dds"),
            Some(42),
            Some(1.5),
            Some(11),
            CalculatorItemEffectValues {
                afr: Some(0.07),
                exp_fish: Some(0.10),
                ..CalculatorItemEffectValues::default()
            },
        );

        assert_eq!(sourced.key, "item:9359");
        assert_eq!(sourced.name, "발락스 도시락");
        assert_eq!(sourced.r#type, "food");
        assert_eq!(sourced.durability, Some(11));
        assert_eq!(sourced.icon_id, Some(9359));
        assert_eq!(sourced.icon.as_deref(), Some("/img/items/00009359.webp"));
        assert_eq!(sourced.fish_multiplier, Some(1.5));
        assert_eq!(sourced.afr, Some(0.07));
        assert_eq!(sourced.exp_fish, Some(0.10));
    }

    #[test]
    fn source_backed_item_rows_can_use_direct_numeric_effects() {
        let items = DoltMySqlStore::build_source_backed_items(
            FishLang::En,
            &[CalculatorSourceBackedItemRow {
                source_key: "item:16162".to_string(),
                source_kind: "item".to_string(),
                item_id: Some(16162),
                item_type: "rod".to_string(),
                source_name_en: Some("Balenos Fishing Rod".to_string()),
                source_name_ko: Some("발레노스 낚싯대".to_string()),
                item_icon_file: Some(
                    "New_Icon/06_PC_EquipItem/00_Common/00_ETC/00016162.dds".to_string(),
                ),
                icon_id: Some(1),
                durability: Some(100),
                fish_multiplier: None,
                effect_description_ko: None,
                afr: Some(0.25),
                bonus_rare: None,
                bonus_big: None,
                drr: None,
                exp_fish: None,
                exp_life: None,
            }],
        )
        .expect("source-backed rows should build");

        assert_eq!(items.len(), 1);
        let sourced = &items[0];
        assert_eq!(sourced.key, "item:16162");
        assert_eq!(sourced.name, "Balenos Fishing Rod");
        assert_eq!(sourced.r#type, "rod");
        assert_eq!(sourced.icon_id, Some(16162));
        assert_eq!(sourced.durability, Some(100));
        assert_eq!(sourced.afr, Some(0.25));
    }

    #[test]
    fn source_backed_effect_values_merge_direct_and_text_effects() {
        let values = source_backed_effect_values(&CalculatorSourceBackedItemRow {
            source_key: "item:1".to_string(),
            source_kind: "item".to_string(),
            item_id: Some(1),
            item_type: "buff".to_string(),
            source_name_en: None,
            source_name_ko: None,
            item_icon_file: None,
            icon_id: None,
            durability: None,
            fish_multiplier: None,
            effect_description_ko: Some("낚시 경험치 획득량 +10%".to_string()),
            afr: Some(0.05),
            bonus_rare: None,
            bonus_big: None,
            drr: None,
            exp_fish: None,
            exp_life: None,
        });

        assert_eq!(
            values,
            CalculatorItemEffectValues {
                afr: Some(0.05),
                exp_fish: Some(0.10),
                ..CalculatorItemEffectValues::default()
            }
        );
    }

    #[test]
    fn source_backed_effect_values_prefer_direct_numeric_values() {
        let values = source_backed_effect_values(&CalculatorSourceBackedItemRow {
            source_key: "lightstone-set:160".to_string(),
            source_kind: "lightstone_set".to_string(),
            item_id: None,
            item_type: "lightstone_set".to_string(),
            source_name_en: Some("Nibbles".to_string()),
            source_name_ko: Some("신의 입질".to_string()),
            item_icon_file: None,
            icon_id: None,
            durability: None,
            fish_multiplier: None,
            effect_description_ko: Some(
                "자동 낚시 시간 -15%\n낚시 경험치 획득량 +10%\n낚시 숙련도 +20".to_string(),
            ),
            afr: Some(0.15),
            bonus_rare: None,
            bonus_big: None,
            drr: None,
            exp_fish: Some(0.10),
            exp_life: None,
        });

        assert_eq!(
            values,
            CalculatorItemEffectValues {
                afr: Some(0.15),
                exp_fish: Some(0.10),
                ..CalculatorItemEffectValues::default()
            }
        );
    }

    #[test]
    fn source_backed_items_skip_rows_without_supported_calculator_effects() {
        let items = DoltMySqlStore::build_source_backed_items(
            FishLang::En,
            &[
                CalculatorSourceBackedItemRow {
                    source_key: "item:14069".to_string(),
                    source_kind: "item".to_string(),
                    item_id: Some(14069),
                    item_type: "outfit".to_string(),
                    source_name_en: Some("Apprentice Fisher's Uniform".to_string()),
                    source_name_ko: Some("수습 낚시복".to_string()),
                    item_icon_file: Some("00014069.dds".to_string()),
                    icon_id: None,
                    durability: Some(0),
                    fish_multiplier: None,
                    effect_description_ko: None,
                    afr: None,
                    bonus_rare: None,
                    bonus_big: None,
                    drr: None,
                    exp_fish: None,
                    exp_life: None,
                },
                CalculatorSourceBackedItemRow {
                    source_key: "lightstone-set:151".to_string(),
                    source_kind: "lightstone_set".to_string(),
                    item_id: None,
                    item_type: "lightstone_set".to_string(),
                    source_name_en: None,
                    source_name_ko: Some("낚시꾼의 비기".to_string()),
                    item_icon_file: None,
                    icon_id: None,
                    durability: None,
                    fish_multiplier: None,
                    effect_description_ko: Some("낚시 숙련도 +30".to_string()),
                    afr: None,
                    bonus_rare: None,
                    bonus_big: None,
                    drr: None,
                    exp_fish: None,
                    exp_life: None,
                },
            ],
        )
        .expect("source-backed rows should build");

        assert!(items.is_empty());
    }

    #[test]
    fn source_lightstone_item_uses_source_owned_identity() {
        let sourced = build_source_lightstone_item(
            FishLang::Ko,
            "lightstone-set:162",
            None,
            Some("예리한 갈매기"),
            "lightstone_set",
            Some("00000721.dds"),
            Some(9),
            Some(1.25),
            CalculatorItemEffectValues {
                bonus_rare: Some(0.05),
                exp_fish: Some(0.10),
                ..CalculatorItemEffectValues::default()
            },
        );

        assert_eq!(sourced.key, "lightstone-set:162");
        assert_eq!(sourced.name, "예리한 갈매기");
        assert_eq!(sourced.r#type, "lightstone_set");
        assert_eq!(sourced.icon_id, Some(721));
        assert_eq!(sourced.icon.as_deref(), Some("/img/items/00000721.webp"));
        assert_eq!(sourced.durability, Some(9));
        assert_eq!(sourced.fish_multiplier, Some(1.25));
        assert_eq!(sourced.bonus_rare, Some(0.05));
        assert_eq!(sourced.exp_fish, Some(0.10));
    }

    #[test]
    fn source_lightstone_item_uses_source_owned_english_name() {
        let sourced = build_source_lightstone_item(
            FishLang::En,
            "lightstone-set:160",
            Some("Nibbles"),
            Some("신의 입질"),
            "lightstone_set",
            None,
            None,
            None,
            CalculatorItemEffectValues {
                afr: Some(0.15),
                exp_fish: Some(0.10),
                ..CalculatorItemEffectValues::default()
            },
        );

        assert_eq!(sourced.key, "lightstone-set:160");
        assert_eq!(sourced.name, "Nibbles");
        assert_eq!(sourced.r#type, "lightstone_set");
        assert_eq!(sourced.afr, Some(0.15));
        assert_eq!(sourced.exp_fish, Some(0.10));
    }
}
