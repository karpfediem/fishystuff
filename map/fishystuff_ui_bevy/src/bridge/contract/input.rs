use std::collections::{BTreeMap, BTreeSet};

use fishystuff_core::field_metadata::FieldHoverRow;
use serde::{Deserialize, Serialize};

use super::snapshot::FishyMapHoverLayerSampleSnapshot;

use super::normalize::{
    deserialize_nullable_string_field, normalize_i32_list, normalize_layer_clip_mask_map,
    normalize_layer_opacity_map, normalize_string_list, normalize_u32_list, normalize_u32_map,
};
use super::snapshot::FishyMapSelectionPointKind;
use super::{
    default_contract_version, FishyMapViewSnapshot, FISHYMAP_CONTRACT_VERSION,
    FISHYMAP_POINT_ICON_SCALE_MAX, FISHYMAP_POINT_ICON_SCALE_MIN,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FishyMapViewMode {
    #[default]
    #[serde(rename = "2d")]
    TwoD,
    #[serde(rename = "3d")]
    ThreeD,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapThemeColors {
    pub base100: Option<String>,
    pub base200: Option<String>,
    pub base300: Option<String>,
    pub base_content: Option<String>,
    pub primary: Option<String>,
    pub primary_content: Option<String>,
    pub secondary: Option<String>,
    pub secondary_content: Option<String>,
    pub accent: Option<String>,
    pub accent_content: Option<String>,
    pub neutral: Option<String>,
    pub neutral_content: Option<String>,
    pub info: Option<String>,
    pub success: Option<String>,
    pub warning: Option<String>,
    pub error: Option<String>,
}

impl FishyMapThemeColors {
    pub fn merge_from(&mut self, patch: FishyMapThemeColors) {
        if patch.base100.is_some() {
            self.base100 = patch.base100;
        }
        if patch.base200.is_some() {
            self.base200 = patch.base200;
        }
        if patch.base300.is_some() {
            self.base300 = patch.base300;
        }
        if patch.base_content.is_some() {
            self.base_content = patch.base_content;
        }
        if patch.primary.is_some() {
            self.primary = patch.primary;
        }
        if patch.primary_content.is_some() {
            self.primary_content = patch.primary_content;
        }
        if patch.secondary.is_some() {
            self.secondary = patch.secondary;
        }
        if patch.secondary_content.is_some() {
            self.secondary_content = patch.secondary_content;
        }
        if patch.accent.is_some() {
            self.accent = patch.accent;
        }
        if patch.accent_content.is_some() {
            self.accent_content = patch.accent_content;
        }
        if patch.neutral.is_some() {
            self.neutral = patch.neutral;
        }
        if patch.neutral_content.is_some() {
            self.neutral_content = patch.neutral_content;
        }
        if patch.info.is_some() {
            self.info = patch.info;
        }
        if patch.success.is_some() {
            self.success = patch.success;
        }
        if patch.warning.is_some() {
            self.warning = patch.warning;
        }
        if patch.error.is_some() {
            self.error = patch.error;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapThemePatch {
    pub name: Option<String>,
    pub colors: Option<FishyMapThemeColors>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapThemeState {
    pub name: Option<String>,
    pub colors: FishyMapThemeColors,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapFiltersPatch {
    pub fish_ids: Option<Vec<i32>>,
    pub zone_rgbs: Option<Vec<u32>>,
    pub semantic_field_ids_by_layer: Option<BTreeMap<String, Vec<u32>>>,
    pub search_text: Option<String>,
    pub prize_only: Option<bool>,
    #[serde(default, deserialize_with = "deserialize_nullable_string_field")]
    pub patch_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_string_field")]
    pub from_patch_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_string_field")]
    pub to_patch_id: Option<Option<String>>,
    pub layer_ids_visible: Option<Vec<String>>,
    pub layer_ids_ordered: Option<Vec<String>>,
    pub layer_opacities: Option<BTreeMap<String, f32>>,
    pub layer_clip_masks: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapFiltersState {
    pub fish_ids: Vec<i32>,
    pub zone_rgbs: Vec<u32>,
    pub semantic_field_ids_by_layer: BTreeMap<String, Vec<u32>>,
    pub search_text: String,
    pub prize_only: bool,
    pub patch_id: Option<String>,
    pub from_patch_id: Option<String>,
    pub to_patch_id: Option<String>,
    pub layer_ids_visible: Option<Vec<String>>,
    pub layer_ids_ordered: Option<Vec<String>>,
    pub layer_opacities: Option<BTreeMap<String, f32>>,
    pub layer_clip_masks: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapBookmarkEntry {
    pub id: String,
    pub label: Option<String>,
    pub world_x: f64,
    pub world_z: f64,
    pub layer_samples: Vec<FishyMapHoverLayerSampleSnapshot>,
    pub rows: Vec<FieldHoverRow>,
    pub zone_rgb: Option<u32>,
    pub created_at: Option<String>,
}

impl FishyMapBookmarkEntry {
    fn normalize(self) -> Option<Self> {
        let id = self.id.trim().to_string();
        if id.is_empty() || !self.world_x.is_finite() || !self.world_z.is_finite() {
            return None;
        }
        let normalize_optional = |value: Option<String>| {
            value.and_then(|value| {
                let trimmed = value.trim().to_string();
                (!trimmed.is_empty()).then_some(trimmed)
            })
        };
        Some(Self {
            id,
            label: normalize_optional(self.label),
            world_x: self.world_x,
            world_z: self.world_z,
            layer_samples: self.layer_samples,
            rows: normalize_hover_rows(self.rows),
            zone_rgb: self.zone_rgb,
            created_at: normalize_optional(self.created_at),
        })
    }
}

fn normalize_hover_rows(rows: Vec<FieldHoverRow>) -> Vec<FieldHoverRow> {
    rows.into_iter()
        .filter_map(|row| {
            let key = row.key.trim().to_string();
            let icon = row.icon.trim().to_string();
            let label = row.label.trim().to_string();
            let value = row.value.trim().to_string();
            let hide_label = row.hide_label;
            if icon.is_empty() || value.is_empty() || (!hide_label && label.is_empty()) {
                return None;
            }
            let normalize_optional = |value: Option<String>| {
                value.and_then(|value| {
                    let trimmed = value.trim().to_string();
                    (!trimmed.is_empty()).then_some(trimmed)
                })
            };
            Some(FieldHoverRow {
                key,
                icon,
                label,
                value,
                hide_label,
                status_icon: normalize_optional(row.status_icon),
                status_icon_tone: normalize_optional(row.status_icon_tone),
            })
        })
        .collect()
}

fn normalize_bookmarks(bookmarks: Vec<FishyMapBookmarkEntry>) -> Vec<FishyMapBookmarkEntry> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::with_capacity(bookmarks.len());
    for bookmark in bookmarks {
        let Some(bookmark) = bookmark.normalize() else {
            continue;
        };
        if !seen.insert(bookmark.id.clone()) {
            continue;
        }
        normalized.push(bookmark);
    }
    normalized
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapUiState {
    pub diagnostics_open: bool,
    pub legend_open: bool,
    pub left_panel_open: bool,
    pub show_points: bool,
    pub show_point_icons: bool,
    pub point_icon_scale: f32,
    pub active_detail_pane_id: Option<String>,
    pub bookmark_selected_ids: Vec<String>,
    pub bookmarks: Vec<FishyMapBookmarkEntry>,
}

impl Default for FishyMapUiState {
    fn default() -> Self {
        Self {
            diagnostics_open: false,
            legend_open: false,
            left_panel_open: true,
            show_points: true,
            show_point_icons: true,
            point_icon_scale: FISHYMAP_POINT_ICON_SCALE_MIN,
            active_detail_pane_id: None,
            bookmark_selected_ids: Vec::new(),
            bookmarks: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapUiPatch {
    pub diagnostics_open: Option<bool>,
    pub legend_open: Option<bool>,
    pub left_panel_open: Option<bool>,
    pub show_points: Option<bool>,
    pub show_point_icons: Option<bool>,
    pub point_icon_scale: Option<f32>,
    #[serde(default, deserialize_with = "deserialize_nullable_string_field")]
    pub active_detail_pane_id: Option<Option<String>>,
    pub bookmark_selected_ids: Option<Vec<String>>,
    pub bookmarks: Option<Vec<FishyMapBookmarkEntry>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapWorldPointCommand {
    pub world_x: f64,
    pub world_z: f64,
    pub point_kind: Option<FishyMapSelectionPointKind>,
    pub point_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapSelectSemanticFieldCommand {
    pub layer_id: String,
    pub field_id: u32,
    pub target_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapCommands {
    pub reset_view: Option<bool>,
    pub set_view_mode: Option<FishyMapViewMode>,
    pub select_zone_rgb: Option<u32>,
    pub select_semantic_field: Option<FishyMapSelectSemanticFieldCommand>,
    pub select_world_point: Option<FishyMapWorldPointCommand>,
    pub restore_view: Option<FishyMapViewSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapStatePatch {
    #[serde(default = "default_contract_version")]
    pub version: u8,
    pub theme: Option<FishyMapThemePatch>,
    pub filters: Option<FishyMapFiltersPatch>,
    pub ui: Option<FishyMapUiPatch>,
    pub commands: Option<FishyMapCommands>,
}

impl Default for FishyMapStatePatch {
    fn default() -> Self {
        Self {
            version: FISHYMAP_CONTRACT_VERSION,
            theme: None,
            filters: None,
            ui: None,
            commands: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FishyMapInputState {
    #[serde(default = "default_contract_version")]
    pub version: u8,
    pub theme: FishyMapThemeState,
    pub filters: FishyMapFiltersState,
    pub ui: FishyMapUiState,
}

impl Default for FishyMapInputState {
    fn default() -> Self {
        Self {
            version: FISHYMAP_CONTRACT_VERSION,
            theme: FishyMapThemeState::default(),
            filters: FishyMapFiltersState::default(),
            ui: FishyMapUiState::default(),
        }
    }
}

impl FishyMapInputState {
    pub fn apply_patch(&mut self, patch: FishyMapStatePatch) -> FishyMapCommands {
        if let Some(theme) = patch.theme {
            if let Some(name) = theme.name {
                self.theme.name = Some(name);
            }
            if let Some(colors) = theme.colors {
                self.theme.colors.merge_from(colors);
            }
        }

        if let Some(filters) = patch.filters {
            let mut patch_selection_updated = false;
            if let Some(fish_ids) = filters.fish_ids {
                self.filters.fish_ids = normalize_i32_list(fish_ids);
            }
            if let Some(zone_rgbs) = filters.zone_rgbs {
                self.filters.zone_rgbs = normalize_u32_list(zone_rgbs);
                self.filters
                    .semantic_field_ids_by_layer
                    .insert("zone_mask".to_string(), self.filters.zone_rgbs.clone());
            }
            if let Some(semantic_field_ids_by_layer) = filters.semantic_field_ids_by_layer {
                self.filters.semantic_field_ids_by_layer =
                    normalize_u32_map(semantic_field_ids_by_layer);
                self.filters.zone_rgbs = self
                    .filters
                    .semantic_field_ids_by_layer
                    .get("zone_mask")
                    .cloned()
                    .unwrap_or_default();
            }
            if let Some(search_text) = filters.search_text {
                self.filters.search_text = search_text;
            }
            if let Some(prize_only) = filters.prize_only {
                self.filters.prize_only = prize_only;
            }
            if let Some(patch_id) = filters.patch_id {
                self.filters.patch_id = patch_id;
                if filters.from_patch_id.is_none() && filters.to_patch_id.is_none() {
                    self.filters.from_patch_id = self.filters.patch_id.clone();
                    self.filters.to_patch_id = self.filters.patch_id.clone();
                }
                patch_selection_updated = true;
            }
            if let Some(from_patch_id) = filters.from_patch_id {
                self.filters.from_patch_id = from_patch_id;
                patch_selection_updated = true;
            }
            if let Some(to_patch_id) = filters.to_patch_id {
                self.filters.to_patch_id = to_patch_id;
                patch_selection_updated = true;
            }
            if patch_selection_updated {
                self.filters.patch_id =
                    match (&self.filters.from_patch_id, &self.filters.to_patch_id) {
                        (Some(from_patch_id), Some(to_patch_id))
                            if from_patch_id == to_patch_id =>
                        {
                            Some(from_patch_id.clone())
                        }
                        _ => None,
                    };
            }
            if let Some(layer_ids_visible) = filters.layer_ids_visible {
                self.filters.layer_ids_visible = Some(normalize_string_list(layer_ids_visible));
            }
            if let Some(layer_ids_ordered) = filters.layer_ids_ordered {
                self.filters.layer_ids_ordered = Some(normalize_string_list(layer_ids_ordered));
            }
            if let Some(layer_opacities) = filters.layer_opacities {
                let normalized = normalize_layer_opacity_map(layer_opacities);
                self.filters.layer_opacities = (!normalized.is_empty()).then_some(normalized);
            }
            if let Some(layer_clip_masks) = filters.layer_clip_masks {
                let normalized = normalize_layer_clip_mask_map(layer_clip_masks);
                self.filters.layer_clip_masks = (!normalized.is_empty()).then_some(normalized);
            }
        }

        if let Some(ui) = patch.ui {
            if let Some(value) = ui.diagnostics_open {
                self.ui.diagnostics_open = value;
            }
            if let Some(value) = ui.legend_open {
                self.ui.legend_open = value;
            }
            if let Some(value) = ui.left_panel_open {
                self.ui.left_panel_open = value;
            }
            if let Some(value) = ui.show_points {
                self.ui.show_points = value;
            }
            if let Some(value) = ui.show_point_icons {
                self.ui.show_point_icons = value;
            }
            if let Some(value) = ui.point_icon_scale {
                self.ui.point_icon_scale =
                    value.clamp(FISHYMAP_POINT_ICON_SCALE_MIN, FISHYMAP_POINT_ICON_SCALE_MAX);
            }
            if let Some(active_detail_pane_id) = ui.active_detail_pane_id {
                self.ui.active_detail_pane_id = active_detail_pane_id.and_then(|value| {
                    let trimmed = value.trim().to_string();
                    (!trimmed.is_empty()).then_some(trimmed)
                });
            }
            if let Some(bookmark_selected_ids) = ui.bookmark_selected_ids {
                self.ui.bookmark_selected_ids = normalize_string_list(bookmark_selected_ids);
            }
            if let Some(bookmarks) = ui.bookmarks {
                self.ui.bookmarks = normalize_bookmarks(bookmarks);
            }
        }

        patch.commands.unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::{FishyMapInputState, FishyMapStatePatch, FishyMapUiPatch};

    #[test]
    fn ui_patch_can_set_and_clear_active_detail_pane_id() {
        let mut state = FishyMapInputState::default();
        state.apply_patch(FishyMapStatePatch {
            ui: Some(FishyMapUiPatch {
                active_detail_pane_id: Some(Some("territory".to_string())),
                ..Default::default()
            }),
            ..Default::default()
        });
        assert_eq!(state.ui.active_detail_pane_id.as_deref(), Some("territory"));

        state.apply_patch(FishyMapStatePatch {
            ui: Some(FishyMapUiPatch {
                active_detail_pane_id: Some(None),
                ..Default::default()
            }),
            ..Default::default()
        });
        assert_eq!(state.ui.active_detail_pane_id, None);
    }
}
