use serde::{Deserialize, Serialize};

use crate::ids::RgbKey;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ZoneLootSummaryRequest {
    pub rgb: RgbKey,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ZoneLootSummaryResponse {
    pub available: bool,
    pub zone_name: Option<String>,
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
    pub presence_text: Option<String>,
    pub presence_source_kind: String,
    pub presence_tooltip: String,
}
