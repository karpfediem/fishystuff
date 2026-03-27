use std::collections::HashMap;

use fishystuff_api::models::calculator::CalculatorItemEntry;

use crate::error::AppResult;
use crate::store::FishLang;

use super::calculator_effects::{CalculatorItemEffectValues, CalculatorLightstoneSourceEntry};
use super::calculator_sources::{
    CalculatorCatalogSourceData, CalculatorItemDbRow, CalculatorItemSourceMetadata,
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
    legacy: &CalculatorItemEntry,
    metadata: Option<&CalculatorItemSourceMetadata>,
    override_values: CalculatorItemEffectValues,
) -> CalculatorItemEntry {
    let name = if matches!(lang, FishLang::Ko) {
        metadata
            .and_then(|metadata| metadata.name_ko.clone())
            .unwrap_or_else(|| legacy.name.clone())
    } else {
        legacy.name.clone()
    };
    let icon_id = metadata
        .and_then(|metadata| metadata.icon_id)
        .or(legacy.icon_id)
        .or(Some(item_id));

    CalculatorItemEntry {
        key: format!("item:{item_id}"),
        name,
        r#type: legacy.r#type.clone(),
        afr: override_values.afr,
        bonus_rare: override_values.bonus_rare,
        bonus_big: override_values.bonus_big,
        durability: metadata
            .and_then(|metadata| metadata.durability)
            .or(legacy.durability),
        drr: override_values.drr,
        fish_multiplier: legacy.fish_multiplier,
        exp_fish: override_values.exp_fish,
        exp_life: override_values.exp_life,
        item_id: Some(item_id),
        icon_id,
        icon: icon_id.map(calculator_item_icon_path),
    }
}

pub(super) fn build_source_lightstone_item(
    lang: FishLang,
    legacy: &CalculatorItemEntry,
    name_ko: Option<&str>,
    override_values: CalculatorItemEffectValues,
) -> CalculatorItemEntry {
    let name = if matches!(lang, FishLang::Ko) {
        name_ko
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| legacy.name.clone())
    } else {
        legacy.name.clone()
    };

    CalculatorItemEntry {
        key: legacy.key.clone(),
        name,
        r#type: legacy.r#type.clone(),
        afr: override_values.afr,
        bonus_rare: override_values.bonus_rare,
        bonus_big: override_values.bonus_big,
        durability: legacy.durability,
        drr: override_values.drr,
        fish_multiplier: legacy.fish_multiplier,
        exp_fish: override_values.exp_fish,
        exp_life: override_values.exp_life,
        item_id: legacy.item_id,
        icon_id: legacy.icon_id,
        icon: legacy.icon.clone(),
    }
}

impl DoltMySqlStore {
    fn build_legacy_calculator_items(
        &self,
        lang: FishLang,
        rows: Vec<CalculatorItemDbRow>,
        item_source_metadata: &HashMap<i32, CalculatorItemSourceMetadata>,
        lightstone_sources: &HashMap<String, CalculatorLightstoneSourceEntry>,
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
                .or_else(|| {
                    if item_type == "lightstone_set" {
                        lightstone_sources
                            .get(&legacy_name)
                            .map(|source| source.name_ko.clone())
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

    fn build_source_consumable_items(
        lang: FishLang,
        legacy_items: &[CalculatorItemEntry],
        item_source_metadata: &HashMap<i32, CalculatorItemSourceMetadata>,
        consumable_overrides: &HashMap<i32, CalculatorItemEffectValues>,
    ) -> AppResult<Vec<CalculatorItemEntry>> {
        let legacy_by_item_id = legacy_items
            .iter()
            .filter_map(|item| item.item_id.map(|item_id| (item_id, item)))
            .collect::<HashMap<_, _>>();
        let mut ordered_item_ids = consumable_overrides.keys().copied().collect::<Vec<_>>();
        ordered_item_ids.sort_unstable();
        let mut items = Vec::new();
        for item_id in ordered_item_ids {
            let Some(legacy) = legacy_by_item_id.get(&item_id).copied() else {
                continue;
            };
            let Some(override_values) = consumable_overrides.get(&item_id).copied() else {
                continue;
            };
            items.push(build_source_consumable_item(
                lang,
                item_id,
                legacy,
                item_source_metadata.get(&item_id),
                override_values,
            ));
        }
        Ok(items)
    }

    fn build_source_lightstone_items(
        lang: FishLang,
        legacy_items: &[CalculatorItemEntry],
        lightstone_sources: &HashMap<String, CalculatorLightstoneSourceEntry>,
    ) -> AppResult<Vec<CalculatorItemEntry>> {
        let legacy_by_name = legacy_items
            .iter()
            .filter(|item| item.r#type == "lightstone_set")
            .map(|item| (item.name.clone(), item))
            .collect::<HashMap<_, _>>();
        let mut ordered_names = lightstone_sources.keys().cloned().collect::<Vec<_>>();
        ordered_names.sort_unstable();
        let mut items = Vec::new();
        for legacy_name in ordered_names {
            let Some(legacy) = legacy_by_name.get(&legacy_name).copied() else {
                continue;
            };
            let Some(source) = lightstone_sources.get(legacy_name.as_str()) else {
                continue;
            };
            let Some(override_values) = source.values else {
                continue;
            };
            items.push(build_source_lightstone_item(
                lang,
                legacy,
                Some(source.name_ko.as_str()),
                override_values,
            ));
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
            lightstone_sources,
            consumable_overrides,
        } = self.query_calculator_catalog_source_data(ref_id)?;
        let legacy_items = self.build_legacy_calculator_items(
            lang,
            legacy_rows,
            &item_source_metadata,
            &lightstone_sources,
        );
        let mut sourced_items = Self::build_source_consumable_items(
            lang,
            &legacy_items,
            &item_source_metadata,
            &consumable_overrides,
        )?;
        sourced_items.extend(Self::build_source_lightstone_items(
            lang,
            &legacy_items,
            &lightstone_sources,
        )?);
        Ok(self.merge_calculator_items(legacy_items, sourced_items))
    }
}

#[cfg(test)]
mod tests {
    use fishystuff_api::models::calculator::CalculatorItemEntry;

    use crate::store::FishLang;

    use super::super::calculator_effects::CalculatorItemEffectValues;
    use super::{
        build_source_consumable_item, build_source_lightstone_item, CalculatorItemSourceMetadata,
    };

    #[test]
    fn source_consumable_item_prefers_source_metadata() {
        let legacy = CalculatorItemEntry {
            key: "item:9359".to_string(),
            name: "Balacs Lunchbox".to_string(),
            r#type: "food".to_string(),
            durability: Some(7),
            fish_multiplier: Some(1.5),
            item_id: Some(9359),
            icon_id: Some(42),
            icon: Some("/img/items/00000042.webp".to_string()),
            ..CalculatorItemEntry::default()
        };
        let metadata = CalculatorItemSourceMetadata {
            name_ko: Some("발락스 도시락".to_string()),
            durability: Some(11),
            icon_id: Some(9359),
        };
        let sourced = build_source_consumable_item(
            FishLang::Ko,
            9359,
            &legacy,
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
        let legacy = CalculatorItemEntry {
            key: "effect:sharp-eyed-seagull".to_string(),
            name: "Sharp-Eyed Seagull".to_string(),
            r#type: "lightstone_set".to_string(),
            fish_multiplier: Some(1.25),
            icon_id: Some(721),
            icon: Some("/img/items/00000721.webp".to_string()),
            ..CalculatorItemEntry::default()
        };
        let sourced = build_source_lightstone_item(
            FishLang::Ko,
            &legacy,
            Some("예리한 갈매기"),
            CalculatorItemEffectValues {
                bonus_rare: Some(0.05),
                exp_fish: Some(0.10),
                ..CalculatorItemEffectValues::default()
            },
        );

        assert_eq!(sourced.key, legacy.key);
        assert_eq!(sourced.name, "예리한 갈매기");
        assert_eq!(sourced.r#type, "lightstone_set");
        assert_eq!(sourced.icon_id, legacy.icon_id);
        assert_eq!(sourced.icon, legacy.icon);
        assert_eq!(sourced.fish_multiplier, Some(1.25));
        assert_eq!(sourced.bonus_rare, Some(0.05));
        assert_eq!(sourced.exp_fish, Some(0.10));
    }
}
