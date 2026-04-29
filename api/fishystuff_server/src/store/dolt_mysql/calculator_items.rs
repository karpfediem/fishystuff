use std::collections::HashMap;

use fishystuff_api::models::calculator::CalculatorItemEntry;
use fishystuff_core::fish_icons::{fish_icon_path_from_asset_file, parse_fish_icon_asset_id};

use crate::error::AppResult;
use crate::store::DataLang;

use super::calculator_effects::CalculatorItemEffectValues;
use super::calculator_sources::{
    CalculatorCatalogSourceData, CalculatorItemDbRow, CalculatorSourceBackedItemRow,
};
use super::item_metadata::ItemSourceMetadata;
use super::util::normalize_optional_string;
use super::DoltMySqlStore;

fn calculator_item_icon_path(icon_id: i32) -> String {
    format!("/images/items/{icon_id:08}.webp")
}

fn source_backed_icon_path(
    item_icon_file: Option<&str>,
    metadata: Option<&ItemSourceMetadata>,
) -> Option<String> {
    item_icon_file
        .and_then(fish_icon_path_from_asset_file)
        .or_else(|| metadata.and_then(|meta| meta.icon_path.clone()))
}

fn canonical_source_icon_id(
    item_icon_file: Option<&str>,
    metadata: Option<&ItemSourceMetadata>,
) -> Option<i32> {
    item_icon_file
        .and_then(parse_fish_icon_asset_id)
        .or_else(|| metadata.and_then(|meta| meta.icon_id))
}

fn resolve_calculator_item_icon(
    item_id: Option<i32>,
    explicit_icon_id: Option<i32>,
    item_icon_file: Option<&str>,
    metadata: Option<&ItemSourceMetadata>,
) -> (Option<i32>, Option<String>) {
    let source_icon_path = source_backed_icon_path(item_icon_file, metadata);
    let source_icon_id = canonical_source_icon_id(item_icon_file, metadata);
    let icon_id = if source_icon_path.is_some() {
        source_icon_id
    } else {
        explicit_icon_id.or(source_icon_id).or(item_id)
    };
    let icon = source_icon_path
        .clone()
        .or_else(|| icon_id.map(calculator_item_icon_path))
        .or_else(|| item_id.map(calculator_item_icon_path));
    (icon_id, icon)
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
    _lang: &DataLang,
    item_id: i32,
    item_type: &str,
    source_name_en: Option<&str>,
    source_name_ko: Option<&str>,
    item_icon_file: Option<&str>,
    icon_id: Option<i32>,
    metadata: Option<&ItemSourceMetadata>,
    fish_multiplier: Option<f32>,
    source_durability: Option<i32>,
    override_values: CalculatorItemEffectValues,
) -> CalculatorItemEntry {
    let name = source_name_en
        .map(ToOwned::to_owned)
        .or_else(|| metadata.and_then(|metadata| metadata.display_name()))
        .or_else(|| source_name_ko.map(ToOwned::to_owned))
        .or_else(|| metadata.and_then(|metadata| metadata.name_ko.clone()))
        .unwrap_or_else(|| format!("item:{item_id}"));
    let (icon_id, icon) =
        resolve_calculator_item_icon(Some(item_id), icon_id, item_icon_file, metadata);

    CalculatorItemEntry {
        key: format!("item:{item_id}"),
        name,
        r#type: item_type.to_string(),
        buff_category_key: None,
        buff_category_id: None,
        buff_category_level: None,
        afr: override_values.afr,
        bonus_rare: override_values.bonus_rare,
        bonus_big: override_values.bonus_big,
        durability: source_durability.or_else(|| metadata.and_then(|metadata| metadata.durability)),
        item_drr: override_values.item_drr,
        fish_multiplier,
        exp_fish: override_values.exp_fish,
        exp_life: override_values.exp_life,
        grade: None,
        item_id: Some(item_id),
        icon_id,
        icon,
    }
}

fn source_backed_effect_values(row: &CalculatorSourceBackedItemRow) -> CalculatorItemEffectValues {
    let values = CalculatorItemEffectValues {
        afr: row.afr,
        bonus_rare: row.bonus_rare,
        bonus_big: row.bonus_big,
        item_drr: row.item_drr,
        exp_fish: row.exp_fish,
        exp_life: row.exp_life,
    };
    if row.source_kind == "lightstone_set" {
        return values;
    }
    let mut values = values;
    if let Some(effect_description) = row.effect_description_ko.as_deref() {
        let mut parsed = CalculatorItemEffectValues::default();
        super::calculator_effects::parse_unique_calculator_effect_text(
            &mut parsed,
            effect_description,
        );
        values.afr = values.afr.or(parsed.afr);
        values.bonus_rare = values.bonus_rare.or(parsed.bonus_rare);
        values.bonus_big = values.bonus_big.or(parsed.bonus_big);
        values.item_drr = values.item_drr.or(parsed.item_drr);
        values.exp_fish = values.exp_fish.or(parsed.exp_fish);
        values.exp_life = values.exp_life.or(parsed.exp_life);
    }
    values
}

pub(super) fn build_source_lightstone_item(
    _lang: &DataLang,
    source_key: &str,
    source_name_en: Option<&str>,
    name_ko: Option<&str>,
    item_type: &str,
    item_icon_file: Option<&str>,
    durability: Option<i32>,
    fish_multiplier: Option<f32>,
    override_values: CalculatorItemEffectValues,
) -> CalculatorItemEntry {
    let name = source_name_en
        .map(ToOwned::to_owned)
        .or_else(|| name_ko.map(ToOwned::to_owned))
        .unwrap_or_else(|| source_key.to_string());
    let (icon_id, icon) = resolve_calculator_item_icon(None, None, item_icon_file, None);

    CalculatorItemEntry {
        key: source_key.to_string(),
        name,
        r#type: item_type.to_string(),
        buff_category_key: None,
        buff_category_id: None,
        buff_category_level: None,
        afr: override_values.afr,
        bonus_rare: override_values.bonus_rare,
        bonus_big: override_values.bonus_big,
        durability,
        item_drr: override_values.item_drr,
        fish_multiplier,
        exp_fish: override_values.exp_fish,
        exp_life: override_values.exp_life,
        grade: None,
        item_id: None,
        icon_id,
        icon,
    }
}

impl DoltMySqlStore {
    fn build_legacy_calculator_items(
        &self,
        _lang: &DataLang,
        rows: Vec<CalculatorItemDbRow>,
        item_source_metadata: &HashMap<i32, ItemSourceMetadata>,
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
                .and_then(|item_id| item_source_metadata.get(&item_id))
                .and_then(|metadata| metadata.display_name())
                .unwrap_or_else(|| legacy_name.clone());
            let key = if let Some(item_id) = item_id {
                format!("item:{item_id}")
            } else {
                format!("effect:{}", slugify_calculator_effect_key(&legacy_name))
            };
            let metadata = item_id.and_then(|item_id| item_source_metadata.get(&item_id));
            let (icon_id, icon) = resolve_calculator_item_icon(item_id, icon_id, None, metadata);
            items.push(CalculatorItemEntry {
                key,
                name: display_name,
                r#type: item_type,
                buff_category_key: None,
                buff_category_id: None,
                buff_category_level: None,
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
                item_drr: drr,
                fish_multiplier,
                exp_fish,
                exp_life,
                grade: item_id.and_then(|item_id| {
                    item_source_metadata
                        .get(&item_id)
                        .and_then(|metadata| metadata.grade.clone())
                }),
                item_id,
                icon_id,
                icon,
            });
        }

        items
    }

    fn build_source_backed_items(
        lang: &DataLang,
        source_backed_rows: &[CalculatorSourceBackedItemRow],
        item_source_metadata: &HashMap<i32, ItemSourceMetadata>,
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
                    let mut item = build_source_item(
                        lang,
                        item_id,
                        &row.item_type,
                        row.source_name_en.as_deref(),
                        row.source_name_ko.as_deref(),
                        row.item_icon_file.as_deref(),
                        row.icon_id,
                        item_source_metadata.get(&item_id),
                        row.fish_multiplier,
                        row.durability,
                        override_values,
                    );
                    item.buff_category_key = row.buff_category_key.clone();
                    item.buff_category_id = row.buff_category_id;
                    item.buff_category_level = row.buff_category_level;
                    item.grade = item
                        .item_id
                        .and_then(|item_id| item_source_metadata.get(&item_id))
                        .and_then(|metadata| metadata.grade.clone());
                    items.push(item);
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

    #[tracing::instrument(name = "store.calculator_catalog.assemble_items", skip_all)]
    pub(super) fn build_calculator_items_from_source_data(
        &self,
        lang: &DataLang,
        source_data: CalculatorCatalogSourceData,
    ) -> AppResult<Vec<CalculatorItemEntry>> {
        let CalculatorCatalogSourceData {
            legacy_rows,
            item_source_metadata,
            source_backed_rows,
        } = source_data;
        let legacy_items =
            self.build_legacy_calculator_items(lang, legacy_rows, &item_source_metadata);
        let sourced_items =
            Self::build_source_backed_items(lang, &source_backed_rows, &item_source_metadata)?;
        Ok(self.merge_calculator_items(legacy_items, sourced_items))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::store::DataLang;

    use super::super::calculator_effects::CalculatorItemEffectValues;
    use super::super::item_metadata::ItemSourceMetadata;
    use super::{
        build_source_item, build_source_lightstone_item, resolve_calculator_item_icon,
        source_backed_effect_values, CalculatorSourceBackedItemRow, DoltMySqlStore,
    };

    fn kr_data_lang() -> DataLang {
        DataLang::from_code("kr").expect("valid test data language")
    }

    #[test]
    fn source_consumable_item_prefers_source_metadata() {
        let sourced = build_source_item(
            &kr_data_lang(),
            9359,
            "food",
            Some("Balacs Lunchbox"),
            Some("발락스 도시락"),
            Some("00009359.dds"),
            Some(42),
            None,
            Some(1.5),
            Some(11),
            CalculatorItemEffectValues {
                afr: Some(0.07),
                exp_fish: Some(0.10),
                ..CalculatorItemEffectValues::default()
            },
        );

        assert_eq!(sourced.key, "item:9359");
        assert_eq!(sourced.name, "Balacs Lunchbox");
        assert_eq!(sourced.r#type, "food");
        assert_eq!(sourced.durability, Some(11));
        assert_eq!(sourced.icon_id, Some(9359));
        assert_eq!(sourced.icon.as_deref(), Some("/images/items/00009359.webp"));
        assert_eq!(sourced.fish_multiplier, Some(1.5));
        assert_eq!(sourced.afr, Some(0.07));
        assert_eq!(sourced.exp_fish, Some(0.10));
    }

    #[test]
    fn source_backed_item_rows_can_use_direct_numeric_effects() {
        let items = DoltMySqlStore::build_source_backed_items(
            &DataLang::En,
            &[CalculatorSourceBackedItemRow {
                source_key: "item:16162".to_string(),
                source_kind: "item".to_string(),
                item_id: Some(16162),
                item_type: "rod".to_string(),
                buff_category_key: None,
                buff_category_id: None,
                buff_category_level: None,
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
                item_drr: None,
                exp_fish: None,
                exp_life: None,
            }],
            &HashMap::new(),
        )
        .expect("source-backed rows should build");

        assert_eq!(items.len(), 1);
        let sourced = &items[0];
        assert_eq!(sourced.key, "item:16162");
        assert_eq!(sourced.name, "Balenos Fishing Rod");
        assert_eq!(sourced.r#type, "rod");
        assert_eq!(sourced.icon_id, Some(16162));
        assert_eq!(sourced.icon.as_deref(), Some("/images/items/00016162.webp"));
        assert_eq!(sourced.durability, Some(100));
        assert_eq!(sourced.afr, Some(0.25));
    }

    #[test]
    fn source_backed_items_prefer_explicit_source_stems_over_legacy_icon_ids() {
        let items = DoltMySqlStore::build_source_backed_items(
            &DataLang::En,
            &[CalculatorSourceBackedItemRow {
                source_key: "item:24277".to_string(),
                source_kind: "item".to_string(),
                item_id: Some(24277),
                item_type: "buff".to_string(),
                buff_category_key: None,
                buff_category_id: None,
                buff_category_level: None,
                source_name_en: Some("[11th Anniversary] Celebration Cake (Life)".to_string()),
                source_name_ko: Some("[11주년 기념] 케이크".to_string()),
                item_icon_file: Some(
                    "New_Icon/03_ETC/06_Housing/InHouse_DPFO_birthdayCake_01.dds".to_string(),
                ),
                icon_id: Some(1),
                durability: None,
                fish_multiplier: None,
                effect_description_ko: Some("생활 경험치 획득량 +10%".to_string()),
                afr: None,
                bonus_rare: None,
                bonus_big: None,
                item_drr: None,
                exp_fish: None,
                exp_life: None,
            }],
            &HashMap::new(),
        )
        .expect("source-backed rows should build");

        assert_eq!(items.len(), 1);
        let sourced = &items[0];
        assert_eq!(sourced.icon_id, None);
        assert_eq!(
            sourced.icon.as_deref(),
            Some("/images/items/InHouse_DPFO_birthdayCake_01.webp")
        );
    }

    #[test]
    fn source_backed_items_key_icons_by_item_id() {
        let items = DoltMySqlStore::build_source_backed_items(
            &DataLang::En,
            &[CalculatorSourceBackedItemRow {
                source_key: "item:14330".to_string(),
                source_kind: "item".to_string(),
                item_id: Some(14330),
                item_type: "outfit".to_string(),
                buff_category_key: None,
                buff_category_id: None,
                buff_category_level: None,
                source_name_en: Some("Professional Fisher's Uniform (Costume)".to_string()),
                source_name_ko: Some("[의상] 전문 낚시복".to_string()),
                item_icon_file: Some(
                    "New_Icon/06_PC_EquipItem/00_Common/09_Upperbody/00014071.dds".to_string(),
                ),
                icon_id: Some(14071),
                durability: None,
                fish_multiplier: None,
                effect_description_ko: None,
                afr: None,
                bonus_rare: None,
                bonus_big: None,
                item_drr: None,
                exp_fish: Some(0.10),
                exp_life: None,
            }],
            &HashMap::new(),
        )
        .expect("source-backed rows should build");

        assert_eq!(items.len(), 1);
        let sourced = &items[0];
        assert_eq!(sourced.icon_id, Some(14071));
        assert_eq!(sourced.icon.as_deref(), Some("/images/items/00014071.webp"));
    }

    #[test]
    fn source_backed_items_preserve_buff_category_metadata() {
        let items = DoltMySqlStore::build_source_backed_items(
            &DataLang::En,
            &[CalculatorSourceBackedItemRow {
                source_key: "item:9359".to_string(),
                source_kind: "item".to_string(),
                item_id: Some(9359),
                item_type: "food".to_string(),
                buff_category_key: Some("buff-category:1".to_string()),
                buff_category_id: Some(1),
                buff_category_level: Some(0),
                source_name_en: Some("Balacs Lunchbox".to_string()),
                source_name_ko: Some("발락스 도시락".to_string()),
                item_icon_file: Some("00009359.dds".to_string()),
                icon_id: None,
                durability: None,
                fish_multiplier: None,
                effect_description_ko: Some("자동 낚시 시간 감소 +7%".to_string()),
                afr: None,
                bonus_rare: None,
                bonus_big: None,
                item_drr: None,
                exp_fish: None,
                exp_life: None,
            }],
            &HashMap::new(),
        )
        .expect("source-backed rows should build");

        assert_eq!(items.len(), 1);
        let sourced = &items[0];
        assert_eq!(
            sourced.buff_category_key.as_deref(),
            Some("buff-category:1")
        );
        assert_eq!(sourced.buff_category_id, Some(1));
        assert_eq!(sourced.buff_category_level, Some(0));
    }

    #[test]
    fn source_backed_items_apply_grade_from_item_metadata() {
        let items = DoltMySqlStore::build_source_backed_items(
            &DataLang::En,
            &[CalculatorSourceBackedItemRow {
                source_key: "item:9307".to_string(),
                source_kind: "item".to_string(),
                item_id: Some(9307),
                item_type: "buff".to_string(),
                buff_category_key: None,
                buff_category_id: None,
                buff_category_level: None,
                source_name_en: Some("Verdure Draught".to_string()),
                source_name_ko: Some("신록의 영약".to_string()),
                item_icon_file: Some("00009307.dds".to_string()),
                icon_id: Some(9307),
                durability: None,
                fish_multiplier: None,
                effect_description_ko: Some("자동 낚시 시간 감소 +5%".to_string()),
                afr: None,
                bonus_rare: None,
                bonus_big: None,
                item_drr: None,
                exp_fish: None,
                exp_life: None,
            }],
            &HashMap::from([(
                9307,
                ItemSourceMetadata {
                    grade: Some("Rare".to_string()),
                    ..ItemSourceMetadata::default()
                },
            )]),
        )
        .expect("source-backed rows should build");

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].grade.as_deref(), Some("Rare"));
    }

    #[test]
    fn source_backed_effect_values_merge_direct_and_text_effects() {
        let values = source_backed_effect_values(&CalculatorSourceBackedItemRow {
            source_key: "item:1".to_string(),
            source_kind: "item".to_string(),
            item_id: Some(1),
            item_type: "buff".to_string(),
            buff_category_key: None,
            buff_category_id: None,
            buff_category_level: None,
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
            item_drr: None,
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
            buff_category_key: None,
            buff_category_id: None,
            buff_category_level: None,
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
            item_drr: None,
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
    fn source_backed_lightstone_values_do_not_parse_set_tooltip_fallback() {
        let values = source_backed_effect_values(&CalculatorSourceBackedItemRow {
            source_key: "lightstone-set:30".to_string(),
            source_kind: "lightstone_set".to_string(),
            item_id: None,
            item_type: "lightstone_set".to_string(),
            buff_category_key: None,
            buff_category_id: None,
            buff_category_level: None,
            source_name_en: Some("Blacksmith's Blessing".to_string()),
            source_name_ko: Some("대장장이의 축복".to_string()),
            item_icon_file: None,
            icon_id: None,
            durability: None,
            fish_multiplier: None,
            effect_description_ko: Some(
                "[대장장이의 축복]\n몬스터 추가 공격력 +5\n몬스터 피해 감소 +5\n장비 내구도 감소 저항 +30%"
                    .to_string(),
            ),
            afr: None,
            bonus_rare: None,
            bonus_big: None,
            item_drr: None,
            exp_fish: None,
            exp_life: None,
        });

        assert_eq!(values, CalculatorItemEffectValues::default());
    }

    #[test]
    fn source_backed_effect_values_deduplicate_repeated_effect_lines() {
        let values = source_backed_effect_values(&CalculatorSourceBackedItemRow {
            source_key: "item:59335".to_string(),
            source_kind: "item".to_string(),
            item_id: Some(59335),
            item_type: "buff".to_string(),
            buff_category_key: None,
            buff_category_id: None,
            buff_category_level: None,
            source_name_en: Some("Treant's Tear".to_string()),
            source_name_ko: Some("엔트의 눈물".to_string()),
            item_icon_file: None,
            icon_id: None,
            durability: None,
            fish_multiplier: None,
            effect_description_ko: Some(
                "생활 경험치 획득량 +30%\n생활 경험치 획득량 +30%\n낚시 경험치 획득량 +10%"
                    .to_string(),
            ),
            afr: None,
            bonus_rare: None,
            bonus_big: None,
            item_drr: None,
            exp_fish: None,
            exp_life: None,
        });

        assert_eq!(values.exp_life, Some(0.30));
        assert_eq!(values.exp_fish, Some(0.10));
    }

    #[test]
    fn source_backed_items_skip_rows_without_supported_calculator_effects() {
        let items = DoltMySqlStore::build_source_backed_items(
            &DataLang::En,
            &[
                CalculatorSourceBackedItemRow {
                    source_key: "item:14069".to_string(),
                    source_kind: "item".to_string(),
                    item_id: Some(14069),
                    item_type: "outfit".to_string(),
                    buff_category_key: None,
                    buff_category_id: None,
                    buff_category_level: None,
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
                    item_drr: None,
                    exp_fish: None,
                    exp_life: None,
                },
                CalculatorSourceBackedItemRow {
                    source_key: "lightstone-set:151".to_string(),
                    source_kind: "lightstone_set".to_string(),
                    item_id: None,
                    item_type: "lightstone_set".to_string(),
                    buff_category_key: None,
                    buff_category_id: None,
                    buff_category_level: None,
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
                    item_drr: None,
                    exp_fish: None,
                    exp_life: None,
                },
            ],
            &HashMap::new(),
        )
        .expect("source-backed rows should build");

        assert!(items.is_empty());
    }

    #[test]
    fn source_lightstone_item_uses_source_owned_identity() {
        let sourced = build_source_lightstone_item(
            &kr_data_lang(),
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
        assert_eq!(sourced.icon.as_deref(), Some("/images/items/00000721.webp"));
        assert_eq!(sourced.durability, Some(9));
        assert_eq!(sourced.fish_multiplier, Some(1.25));
        assert_eq!(sourced.bonus_rare, Some(0.05));
        assert_eq!(sourced.exp_fish, Some(0.10));
    }

    #[test]
    fn source_lightstone_item_keeps_non_numeric_source_icon_stems() {
        let sourced = build_source_lightstone_item(
            &DataLang::En,
            "lightstone-set:999",
            Some("Event Buff"),
            Some("이벤트 버프"),
            "lightstone_set",
            Some("ui_texture/icon/new_icon/04_pc_skill/03_buff/event_item_00790580.dds"),
            None,
            None,
            CalculatorItemEffectValues {
                afr: Some(0.15),
                ..CalculatorItemEffectValues::default()
            },
        );

        assert_eq!(sourced.icon_id, None);
        assert_eq!(
            sourced.icon.as_deref(),
            Some("/images/items/event_item_00790580.webp")
        );
    }

    #[test]
    fn metadata_icon_paths_override_legacy_numeric_icon_ids() {
        let metadata = ItemSourceMetadata {
            icon_path: Some("/images/items/InHouse_DPFO_birthdayCake_01.webp".to_string()),
            icon_id: None,
            ..ItemSourceMetadata::default()
        };

        let (icon_id, icon) =
            resolve_calculator_item_icon(Some(24277), Some(1), None, Some(&metadata));

        assert_eq!(icon_id, None);
        assert_eq!(
            icon.as_deref(),
            Some("/images/items/InHouse_DPFO_birthdayCake_01.webp")
        );
    }

    #[test]
    fn source_lightstone_item_uses_source_owned_english_name() {
        let sourced = build_source_lightstone_item(
            &DataLang::En,
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
