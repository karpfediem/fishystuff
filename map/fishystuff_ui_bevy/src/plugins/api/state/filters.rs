use fishystuff_api::models::meta::PatchInfo;
use std::collections::BTreeMap;

use crate::prelude::*;

pub type Patch = PatchInfo;
pub const POINT_ICON_SCALE_MIN: f32 = 1.0;
pub const POINT_ICON_SCALE_MAX: f32 = 3.0;

#[derive(Resource, Default)]
pub struct PatchFilterState {
    pub from_ts: Option<i64>,
    pub to_ts: Option<i64>,
    pub patches: Vec<Patch>,
    pub selected_patch: Option<String>,
}

#[derive(Resource, Default)]
pub struct FishFilterState {
    pub selected_fish_ids: Vec<i32>,
}

#[derive(Resource, Default)]
pub struct SemanticFieldFilterState {
    pub selected_field_ids_by_layer: BTreeMap<String, Vec<u32>>,
}

impl SemanticFieldFilterState {
    pub const ZONE_MASK_LAYER_ID: &str = "zone_mask";

    pub fn field_ids_for_layer(&self, layer_id: &str) -> &[u32] {
        self.selected_field_ids_by_layer
            .get(layer_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn set_field_ids_for_layer(
        &mut self,
        layer_id: impl Into<String>,
        mut field_ids: Vec<u32>,
    ) {
        field_ids.sort_unstable();
        field_ids.dedup();
        let layer_id = layer_id.into();
        if field_ids.is_empty() {
            self.selected_field_ids_by_layer.remove(&layer_id);
            return;
        }
        self.selected_field_ids_by_layer.insert(layer_id, field_ids);
    }

    pub fn selected_zone_rgbs(&self) -> &[u32] {
        self.field_ids_for_layer(Self::ZONE_MASK_LAYER_ID)
    }
}

#[cfg(test)]
mod tests {
    use super::SemanticFieldFilterState;

    #[test]
    fn semantic_field_filter_state_normalizes_and_clears_layer_ids() {
        let mut filter = SemanticFieldFilterState::default();
        filter.set_field_ids_for_layer("regions", vec![8, 3, 8, 5]);
        assert_eq!(filter.field_ids_for_layer("regions"), &[3, 5, 8]);

        filter.set_field_ids_for_layer("regions", Vec::new());
        assert!(filter.field_ids_for_layer("regions").is_empty());
    }

    #[test]
    fn semantic_field_filter_state_exposes_zone_mask_ids() {
        let mut filter = SemanticFieldFilterState::default();
        filter.set_field_ids_for_layer(SemanticFieldFilterState::ZONE_MASK_LAYER_ID, vec![9, 4]);
        assert_eq!(filter.selected_zone_rgbs(), &[4, 9]);
    }
}

#[derive(Resource)]
pub struct MapDisplayState {
    pub show_effort: bool,
    pub show_points: bool,
    pub show_point_icons: bool,
    pub point_icon_scale: f32,
    pub show_drift: bool,
    pub show_zone_mask: bool,
    pub zone_mask_opacity: f32,
    pub hovered_zone_rgb: Option<u32>,
}

impl Default for MapDisplayState {
    fn default() -> Self {
        Self {
            show_effort: true,
            show_points: true,
            show_point_icons: true,
            point_icon_scale: POINT_ICON_SCALE_MIN,
            show_drift: false,
            show_zone_mask: true,
            zone_mask_opacity: 0.55,
            hovered_zone_rgb: None,
        }
    }
}
