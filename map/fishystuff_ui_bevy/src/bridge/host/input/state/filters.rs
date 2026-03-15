use super::super::super::persistence::apply_patch_range_override;
use crate::bridge::contract::FishyMapInputState;
use crate::map::ui_layers::LayerDebugSettings;
use crate::plugins::api::{
    FishFilterState, MapDisplayState, PatchFilterState, POINT_ICON_SCALE_MAX, POINT_ICON_SCALE_MIN,
};

pub(super) fn apply_display_flags(
    input: &FishyMapInputState,
    display_state: &mut MapDisplayState,
    debug_layers: &mut LayerDebugSettings,
) {
    debug_layers.enabled = input.ui.diagnostics_open;
    display_state.show_points = input.ui.show_points;
    display_state.show_point_icons = input.ui.show_point_icons;
    display_state.point_icon_scale = input
        .ui
        .point_icon_scale
        .clamp(POINT_ICON_SCALE_MIN, POINT_ICON_SCALE_MAX);
}

pub(super) fn apply_fish_filters(input: &FishyMapInputState, fish_filter: &mut FishFilterState) {
    fish_filter.selected_fish_ids = input.filters.fish_ids.clone();
}

pub(super) fn apply_patch_filters(input: &FishyMapInputState, patch_filter: &mut PatchFilterState) {
    let from_patch_id = input
        .filters
        .from_patch_id
        .as_deref()
        .or(input.filters.patch_id.as_deref());
    let to_patch_id = input
        .filters
        .to_patch_id
        .as_deref()
        .or(input.filters.patch_id.as_deref());
    if from_patch_id.is_some() || to_patch_id.is_some() {
        apply_patch_range_override(patch_filter, from_patch_id, to_patch_id);
    }
}
