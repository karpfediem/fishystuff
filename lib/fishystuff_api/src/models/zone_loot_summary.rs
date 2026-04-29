use serde::{Deserialize, Serialize};

use crate::ids::RgbKey;
use crate::models::calculator::CalculatorUserOverlaySignals;

fn default_show_normalized_select_rates() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ZoneLootSummaryRequest {
    pub rgb: RgbKey,
    #[serde(default)]
    pub overlay: CalculatorUserOverlaySignals,
    #[serde(default = "default_show_normalized_select_rates")]
    pub show_normalized_select_rates: bool,
}

impl Default for ZoneLootSummaryRequest {
    fn default() -> Self {
        Self {
            rgb: RgbKey::default(),
            overlay: CalculatorUserOverlaySignals::default(),
            show_normalized_select_rates: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ZoneLootSummaryResponse {
    pub available: bool,
    pub zone_name: Option<String>,
    pub data_quality_note: String,
    pub note: String,
    pub profile_label: String,
    pub groups: Vec<ZoneLootSummaryGroupRow>,
    pub species_rows: Vec<ZoneLootSummarySpeciesRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ZoneLootSummaryGroupRow {
    pub slot_idx: u8,
    pub label: String,
    pub fill_color: String,
    pub stroke_color: String,
    pub text_color: String,
    pub drop_rate_text: String,
    pub drop_rate_source_kind: String,
    pub drop_rate_tooltip: String,
    #[serde(default)]
    pub raw_drop_rate_text: String,
    #[serde(default)]
    pub raw_drop_rate_tooltip: String,
    #[serde(default)]
    pub normalized_drop_rate_text: String,
    #[serde(default)]
    pub normalized_drop_rate_tooltip: String,
    pub condition_text: String,
    pub condition_tooltip: String,
    #[serde(default)]
    pub catch_methods: Vec<String>,
    #[serde(default)]
    pub condition_options: Vec<ZoneLootSummaryConditionOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ZoneLootSummaryConditionOption {
    pub condition_text: String,
    pub condition_tooltip: String,
    pub drop_rate_text: String,
    pub drop_rate_source_kind: String,
    pub drop_rate_tooltip: String,
    #[serde(default)]
    pub raw_drop_rate_text: String,
    #[serde(default)]
    pub raw_drop_rate_tooltip: String,
    #[serde(default)]
    pub normalized_drop_rate_text: String,
    #[serde(default)]
    pub normalized_drop_rate_tooltip: String,
    pub active: bool,
    pub species_rows: Vec<ZoneLootSummarySpeciesRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ZoneLootSummarySpeciesRow {
    pub slot_idx: u8,
    pub group_label: String,
    pub label: String,
    pub icon_url: Option<String>,
    pub icon_grade_tone: String,
    pub fill_color: String,
    pub stroke_color: String,
    pub text_color: String,
    pub drop_rate_text: String,
    pub drop_rate_source_kind: String,
    pub drop_rate_tooltip: String,
    #[serde(default)]
    pub raw_drop_rate_text: String,
    #[serde(default)]
    pub raw_drop_rate_tooltip: String,
    #[serde(default)]
    pub normalized_drop_rate_text: String,
    #[serde(default)]
    pub normalized_drop_rate_tooltip: String,
    pub presence_text: Option<String>,
    pub presence_source_kind: String,
    pub presence_tooltip: String,
    #[serde(default)]
    pub catch_methods: Vec<String>,
}
