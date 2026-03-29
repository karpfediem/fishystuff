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
    pub treasure_rate_raw: i32,
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
    pub tiers: Vec<CalculatorOptionEntry>,
    #[serde(default)]
    pub specials: Vec<CalculatorOptionEntry>,
    #[serde(default)]
    pub talents: Vec<CalculatorOptionEntry>,
    #[serde(default)]
    pub skills: Vec<CalculatorOptionEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CalculatorPetSignals {
    pub tier: String,
    pub special: String,
    pub talent: String,
    pub skills: Vec<String>,
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
    pub rod: String,
    pub float: String,
    pub chair: String,
    pub lightstone_set: String,
    pub backpack: String,
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
