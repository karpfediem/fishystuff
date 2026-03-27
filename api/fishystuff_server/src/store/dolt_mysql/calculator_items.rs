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

pub(super) fn build_source_consumable_item(
    lang: FishLang,
    item_id: i32,
    item_type: &str,
    legacy_name_en: Option<&str>,
    fish_multiplier: Option<f32>,
    source_durability: Option<i32>,
    metadata: Option<&CalculatorItemSourceMetadata>,
    override_values: CalculatorItemEffectValues,
) -> CalculatorItemEntry {
    let name = if matches!(lang, FishLang::Ko) {
        metadata
            .and_then(|metadata| metadata.name_ko.clone())
            .or_else(|| legacy_name_en.map(ToOwned::to_owned))
            .unwrap_or_else(|| format!("item:{item_id}"))
    } else {
        legacy_name_en
            .map(ToOwned::to_owned)
            .or_else(|| metadata.and_then(|metadata| metadata.name_ko.clone()))
            .unwrap_or_else(|| format!("item:{item_id}"))
    };
    let icon_id = metadata
        .and_then(|metadata| metadata.icon_id)
        .or(Some(item_id));

    CalculatorItemEntry {
        key: format!("item:{item_id}"),
        name,
        r#type: item_type.to_string(),
        afr: override_values.afr,
        bonus_rare: override_values.bonus_rare,
        bonus_big: override_values.bonus_big,
        durability: metadata
            .and_then(|metadata| metadata.durability)
            .or(source_durability),
        drr: override_values.drr,
        fish_multiplier,
        exp_fish: override_values.exp_fish,
        exp_life: override_values.exp_life,
        item_id: Some(item_id),
        icon_id,
        icon: icon_id.map(calculator_item_icon_path),
    }
}

pub(super) fn build_source_lightstone_item(
    lang: FishLang,
    legacy_name_en: &str,
    name_ko: Option<&str>,
    item_type: &str,
    icon_id: Option<i32>,
    durability: Option<i32>,
    fish_multiplier: Option<f32>,
    override_values: CalculatorItemEffectValues,
) -> CalculatorItemEntry {
    let name = if matches!(lang, FishLang::Ko) {
        name_ko
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| legacy_name_en.to_string())
    } else {
        legacy_name_en.to_string()
    };

    CalculatorItemEntry {
        key: format!("effect:{}", slugify_calculator_effect_key(legacy_name_en)),
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
        item_source_metadata: &HashMap<i32, CalculatorItemSourceMetadata>,
        source_backed_rows: &[CalculatorSourceBackedItemRow],
    ) -> AppResult<Vec<CalculatorItemEntry>> {
        let mut items = Vec::new();
        for row in source_backed_rows {
            let Some(effect_description) = row.effect_description_ko.as_deref() else {
                continue;
            };
            let mut override_values = CalculatorItemEffectValues::default();
            super::calculator_effects::parse_calculator_effect_text(
                &mut override_values,
                effect_description,
            );
            if override_values == CalculatorItemEffectValues::default() {
                continue;
            }
            match row.source_kind.as_str() {
                "item" => {
                    let Some(item_id) = row.item_id else {
                        continue;
                    };
                    items.push(build_source_consumable_item(
                        lang,
                        item_id,
                        &row.item_type,
                        row.legacy_name_en.as_deref(),
                        row.fish_multiplier,
                        row.durability,
                        item_source_metadata.get(&item_id),
                        override_values,
                    ));
                }
                "lightstone_set" => {
                    let Some(legacy_name_en) = row.legacy_name_en.as_deref() else {
                        continue;
                    };
                    let icon_id = row
                        .item_icon_file
                        .as_deref()
                        .and_then(parse_fish_icon_asset_id)
                        .or(row.legacy_icon_id);
                    items.push(build_source_lightstone_item(
                        lang,
                        legacy_name_en,
                        row.source_name_ko.as_deref(),
                        &row.item_type,
                        icon_id,
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
        let sourced_items =
            Self::build_source_backed_items(lang, &item_source_metadata, &source_backed_rows)?;
        Ok(self.merge_calculator_items(legacy_items, sourced_items))
    }
}

#[cfg(test)]
mod tests {
    use crate::store::FishLang;

    use super::super::calculator_effects::CalculatorItemEffectValues;
    use super::{
        build_source_consumable_item, build_source_lightstone_item, CalculatorItemSourceMetadata,
    };

    #[test]
    fn source_consumable_item_prefers_source_metadata() {
        let metadata = CalculatorItemSourceMetadata {
            name_ko: Some("발락스 도시락".to_string()),
            durability: Some(11),
            icon_id: Some(9359),
        };
        let sourced = build_source_consumable_item(
            FishLang::Ko,
            9359,
            "food",
            Some("Balacs Lunchbox"),
            Some(1.5),
            Some(11),
            Some(&metadata),
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
    fn source_lightstone_item_uses_localized_name_but_keeps_legacy_identity() {
        let sourced = build_source_lightstone_item(
            FishLang::Ko,
            "Sharp-Eyed Seagull",
            Some("예리한 갈매기"),
            "lightstone_set",
            Some(721),
            Some(9),
            Some(1.25),
            CalculatorItemEffectValues {
                bonus_rare: Some(0.05),
                exp_fish: Some(0.10),
                ..CalculatorItemEffectValues::default()
            },
        );

        assert_eq!(sourced.key, "effect:sharp-eyed-seagull");
        assert_eq!(sourced.name, "예리한 갈매기");
        assert_eq!(sourced.r#type, "lightstone_set");
        assert_eq!(sourced.icon_id, Some(721));
        assert_eq!(sourced.icon.as_deref(), Some("/img/items/00000721.webp"));
        assert_eq!(sourced.durability, Some(9));
        assert_eq!(sourced.fish_multiplier, Some(1.25));
        assert_eq!(sourced.bonus_rare, Some(0.05));
        assert_eq!(sourced.exp_fish, Some(0.10));
    }
}
