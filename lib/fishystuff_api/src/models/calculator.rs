use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalculatorItemEntry {
    pub key: String,
    pub name: String,
    pub r#type: String,
    pub afr: Option<f32>,
    pub bonus_rare: Option<f32>,
    pub bonus_big: Option<f32>,
    pub durability: Option<i32>,
    pub drr: Option<f32>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalculatorCatalogResponse {
    #[serde(default)]
    pub items: Vec<CalculatorItemEntry>,
    #[serde(default)]
    pub lifeskill_levels: Vec<CalculatorLifeskillLevelEntry>,
}

fn default_level() -> i32 {
    5
}

fn default_lifeskill_level() -> String {
    "100".to_string()
}

fn default_zone() -> String {
    "240,74,74".to_string()
}

fn default_rod() -> String {
    "item:16162".to_string()
}

fn default_chair() -> String {
    "item:705539".to_string()
}

fn default_lightstone_set() -> String {
    "effect:blacksmith-s-blessing".to_string()
}

fn default_backpack() -> String {
    "item:830150".to_string()
}

fn default_outfit() -> Vec<String> {
    vec![
        "effect:8-piece-outfit-set-effect".to_string(),
        "effect:awakening-weapon-outfit".to_string(),
        "effect:mainhand-weapon-outfit".to_string(),
    ]
}

fn default_food() -> Vec<String> {
    vec!["item:9359".to_string()]
}

fn default_buff() -> Vec<String> {
    vec!["".to_string(), "item:721092".to_string()]
}

fn default_pet_one() -> CalculatorPetSignals {
    CalculatorPetSignals {
        tier: "5".to_string(),
        special: "auto_fishing_time_reduction".to_string(),
        talent: "durability_reduction_resistance".to_string(),
        skills: vec!["fishing_exp".to_string()],
    }
}

fn default_pet_other() -> CalculatorPetSignals {
    CalculatorPetSignals {
        tier: "4".to_string(),
        special: String::new(),
        talent: "durability_reduction_resistance".to_string(),
        skills: vec!["fishing_exp".to_string()],
    }
}

fn default_catch_time_active() -> f64 {
    17.5
}

fn default_catch_time_afk() -> f64 {
    6.5
}

fn default_timespan_amount() -> f64 {
    8.0
}

fn default_timespan_unit() -> String {
    "hours".to_string()
}

fn default_brand() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CalculatorPetSignals {
    pub tier: String,
    pub special: String,
    pub talent: String,
    pub skills: Vec<String>,
}

impl Default for CalculatorPetSignals {
    fn default() -> Self {
        default_pet_other()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CalculatorSignals {
    #[serde(default = "default_level")]
    pub level: i32,
    #[serde(default = "default_lifeskill_level")]
    pub lifeskill_level: String,
    #[serde(default = "default_zone")]
    pub zone: String,
    pub resources: f64,
    #[serde(default = "default_rod")]
    pub rod: String,
    pub float: String,
    #[serde(default = "default_chair")]
    pub chair: String,
    #[serde(default = "default_lightstone_set")]
    pub lightstone_set: String,
    #[serde(default = "default_backpack")]
    pub backpack: String,
    #[serde(default = "default_outfit")]
    pub outfit: Vec<String>,
    #[serde(default = "default_food")]
    pub food: Vec<String>,
    #[serde(default = "default_buff")]
    pub buff: Vec<String>,
    #[serde(default = "default_pet_one")]
    pub pet1: CalculatorPetSignals,
    #[serde(default = "default_pet_other")]
    pub pet2: CalculatorPetSignals,
    #[serde(default = "default_pet_other")]
    pub pet3: CalculatorPetSignals,
    #[serde(default = "default_pet_other")]
    pub pet4: CalculatorPetSignals,
    #[serde(default = "default_pet_other")]
    pub pet5: CalculatorPetSignals,
    #[serde(default = "default_catch_time_active", rename = "catchTimeActive")]
    pub catch_time_active: f64,
    #[serde(default = "default_catch_time_afk", rename = "catchTimeAfk")]
    pub catch_time_afk: f64,
    #[serde(default = "default_timespan_amount", rename = "timespanAmount")]
    pub timespan_amount: f64,
    #[serde(default = "default_timespan_unit", rename = "timespanUnit")]
    pub timespan_unit: String,
    #[serde(default = "default_brand")]
    pub brand: bool,
    pub active: bool,
    pub debug: bool,
}

impl Default for CalculatorSignals {
    fn default() -> Self {
        Self {
            level: default_level(),
            lifeskill_level: default_lifeskill_level(),
            zone: default_zone(),
            resources: 0.0,
            rod: default_rod(),
            float: String::new(),
            chair: default_chair(),
            lightstone_set: default_lightstone_set(),
            backpack: default_backpack(),
            outfit: default_outfit(),
            food: default_food(),
            buff: default_buff(),
            pet1: default_pet_one(),
            pet2: default_pet_other(),
            pet3: default_pet_other(),
            pet4: default_pet_other(),
            pet5: default_pet_other(),
            catch_time_active: default_catch_time_active(),
            catch_time_afk: default_catch_time_afk(),
            timespan_amount: default_timespan_amount(),
            timespan_unit: default_timespan_unit(),
            brand: default_brand(),
            active: false,
            debug: false,
        }
    }
}
