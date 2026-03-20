use super::super::super::*;

pub(in crate::bridge::host::snapshot) fn effective_ui_state(
    bridge_input: &FishyMapInputState,
    display_state: &MapDisplayState,
    diagnostics_open: bool,
) -> crate::bridge::contract::FishyMapUiState {
    crate::bridge::contract::FishyMapUiState {
        diagnostics_open,
        legend_open: bridge_input.ui.legend_open,
        left_panel_open: bridge_input.ui.left_panel_open,
        show_points: display_state.show_points,
        show_point_icons: display_state.show_point_icons,
        point_icon_scale: display_state
            .point_icon_scale
            .clamp(POINT_ICON_SCALE_MIN, POINT_ICON_SCALE_MAX),
        bookmarks: bridge_input.ui.bookmarks.clone(),
    }
}
