use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalculatorItemEntry {
    pub key: String,
    pub name: String,
    pub r#type: String,
    pub buff_category_key: Option<String>,
    pub buff_category_id: Option<i32>,
    pub buff_category_level: Option<i32>,
    pub afr: Option<f32>,
    pub bonus_rare: Option<f32>,
    pub bonus_big: Option<f32>,
    pub durability: Option<i32>,
    pub item_drr: Option<f32>,
    pub fish_multiplier: Option<f32>,
    pub exp_fish: Option<f32>,
    pub exp_life: Option<f32>,
    pub grade: Option<String>,
    pub item_id: Option<i32>,
    pub icon_id: Option<i32>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalculatorLifeskillLevelEntry {
    pub key: String,
    pub name: String,
    pub index: i32,
    pub order: i32,
    pub lifeskill_level_drr: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalculatorOptionEntry {
    pub key: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalculatorMasteryPrizeRateEntry {
    pub fishing_mastery: i32,
    pub high_drop_rate_raw: i32,
    pub high_drop_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalculatorZoneGroupRateEntry {
    pub zone_rgb_key: String,
    pub prize_main_group_key: Option<i32>,
    pub rare_rate_raw: i32,
    pub high_quality_rate_raw: i32,
    pub general_rate_raw: i32,
    pub trash_rate_raw: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalculatorSessionPresetEntry {
    pub label: String,
    pub amount: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalculatorPetCatalog {
    pub slots: usize,
    #[serde(default)]
    pub pets: Vec<CalculatorPetEntry>,
    #[serde(default)]
    pub tiers: Vec<CalculatorOptionEntry>,
    #[serde(default)]
    pub specials: Vec<CalculatorPetOptionEntry>,
    #[serde(default)]
    pub talents: Vec<CalculatorPetOptionEntry>,
    #[serde(default)]
    pub skills: Vec<CalculatorPetOptionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalculatorPetEntry {
    pub key: String,
    pub label: String,
    pub skin_key: Option<String>,
    pub image_url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alias_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lineage_keys: Vec<String>,
    #[serde(default)]
    pub tiers: Vec<CalculatorPetTierEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalculatorPetTierEntry {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub specials: Vec<String>,
    #[serde(default)]
    pub talents: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub skill_chances: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalculatorPetOptionEntry {
    pub key: String,
    pub label: String,
    pub icon: Option<String>,
    pub auto_fishing_time_reduction: Option<f32>,
    pub durability_reduction_resistance: Option<f32>,
    pub fishing_exp: Option<f32>,
    pub life_exp: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CalculatorPetSignals {
    pub pet: String,
    pub tier: String,
    #[serde(rename = "packLeader")]
    pub pack_leader: bool,
    pub special: String,
    pub talent: String,
    pub skills: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CalculatorPriceOverrideSignals {
    #[serde(rename = "tradePriceCurvePercent")]
    pub trade_price_curve_percent: Option<f64>,
    #[serde(rename = "basePrice")]
    pub base_price: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct CalculatorZoneGroupOverlaySignals {
    pub present: Option<bool>,
    #[serde(rename = "rawRatePercent")]
    pub raw_rate_percent: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct CalculatorZoneLootOverlaySignals {
    pub present: Option<bool>,
    #[serde(rename = "slotIdx")]
    pub slot_idx: Option<u8>,
    #[serde(rename = "rawRatePercent")]
    pub raw_rate_percent: Option<f64>,
    pub name: Option<String>,
    pub grade: Option<String>,
    #[serde(rename = "isFish")]
    pub is_fish: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct CalculatorZoneOverlaySignals {
    #[serde(default)]
    pub groups: BTreeMap<String, CalculatorZoneGroupOverlaySignals>,
    #[serde(default)]
    pub items: BTreeMap<String, CalculatorZoneLootOverlaySignals>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct CalculatorUserOverlaySignals {
    #[serde(default)]
    pub zones: BTreeMap<String, CalculatorZoneOverlaySignals>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CalculatorSignals {
    pub level: i32,
    pub lifeskill_level: String,
    pub mastery: f64,
    pub trade_level: String,
    pub zone: String,
    pub resources: f64,
    #[serde(rename = "fishingMode")]
    pub fishing_mode: String,
    pub rod: String,
    pub float: String,
    pub chair: String,
    pub lightstone_set: String,
    pub backpack: String,
    #[serde(rename = "targetFish")]
    pub target_fish: String,
    #[serde(rename = "targetFishAmount")]
    pub target_fish_amount: f64,
    #[serde(rename = "targetFishPmfCount")]
    pub target_fish_pmf_count: f64,
    pub outfit: Vec<String>,
    pub food: Vec<String>,
    pub buff: Vec<String>,
    pub pet1: CalculatorPetSignals,
    pub pet2: CalculatorPetSignals,
    pub pet3: CalculatorPetSignals,
    pub pet4: CalculatorPetSignals,
    pub pet5: CalculatorPetSignals,
    #[serde(rename = "tradeDistanceBonus")]
    pub trade_distance_bonus: f64,
    #[serde(rename = "tradePriceCurve")]
    pub trade_price_curve: f64,
    #[serde(rename = "priceOverrides")]
    pub price_overrides: BTreeMap<String, CalculatorPriceOverrideSignals>,
    #[serde(default)]
    pub overlay: CalculatorUserOverlaySignals,
    #[serde(rename = "catchTimeActive")]
    pub catch_time_active: f64,
    #[serde(rename = "catchTimeAfk")]
    pub catch_time_afk: f64,
    #[serde(rename = "timespanAmount")]
    pub timespan_amount: f64,
    #[serde(rename = "timespanUnit")]
    pub timespan_unit: String,
    #[serde(rename = "applyTradeModifiers")]
    pub apply_trade_modifiers: bool,
    #[serde(rename = "showSilverAmounts")]
    pub show_silver_amounts: bool,
    #[serde(rename = "showNormalizedSelectRates")]
    pub show_normalized_select_rates: bool,
    #[serde(rename = "discardGrade")]
    pub discard_grade: String,
    pub brand: bool,
    pub active: bool,
    pub debug: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalculatorCatalogResponse {
    #[serde(default)]
    pub items: Vec<CalculatorItemEntry>,
    #[serde(default)]
    pub lifeskill_levels: Vec<CalculatorLifeskillLevelEntry>,
    #[serde(default)]
    pub mastery_prize_curve: Vec<CalculatorMasteryPrizeRateEntry>,
    #[serde(default)]
    pub zone_group_rates: Vec<CalculatorZoneGroupRateEntry>,
    #[serde(default)]
    pub fishing_levels: Vec<CalculatorOptionEntry>,
    #[serde(default)]
    pub trade_levels: Vec<CalculatorOptionEntry>,
    #[serde(default)]
    pub session_units: Vec<CalculatorOptionEntry>,
    #[serde(default)]
    pub session_presets: Vec<CalculatorSessionPresetEntry>,
    #[serde(default)]
    pub pets: CalculatorPetCatalog,
    #[serde(default)]
    pub defaults: CalculatorSignals,
}
