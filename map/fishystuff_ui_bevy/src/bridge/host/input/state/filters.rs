use crate::bridge::contract::FishyMapInputState;
use crate::map::ui_layers::LayerDebugSettings;
use crate::plugins::api::{
    LayerFilterBindingOverrideState, MapDisplayState, POINT_ICON_SCALE_MAX, POINT_ICON_SCALE_MIN,
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

pub(super) fn apply_layer_filter_binding_overrides(
    input: &FishyMapInputState,
    overrides: &mut LayerFilterBindingOverrideState,
) {
    overrides.set_disabled_binding_ids_by_layer(
        input
            .filters
            .layer_filter_binding_ids_disabled_by_layer
            .clone()
            .unwrap_or_default(),
    );
}

#[cfg(test)]
mod tests {
    use super::super::super::super::persistence::apply_patch_range_override;
    use crate::bridge::contract::FishyMapInputState;
    use crate::plugins::api::{Patch, PatchFilterState};
    use chrono::NaiveDate;
    use fishystuff_api::ids::PatchId;
    use fishystuff_api::models::meta::PatchInfo;

    fn patch(patch_id: &str, start_ts_utc: i64, patch_name: Option<&str>) -> Patch {
        PatchInfo {
            patch_id: PatchId(patch_id.to_string()),
            start_ts_utc,
            patch_name: patch_name.map(str::to_string),
        }
    }

    fn unix_day_start(value: &str) -> i64 {
        NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp()
    }

    #[test]
    fn apply_patch_filters_clears_existing_range_when_input_has_no_patch_bounds() {
        let input = FishyMapInputState::default();
        let mut patch_filter = PatchFilterState {
            patches: vec![
                patch("2026-03-12", unix_day_start("2026-03-12"), Some("New Era")),
                patch(
                    "2026-04-24",
                    unix_day_start("2026-04-24"),
                    Some("Silver Tide"),
                ),
            ],
            from_ts: Some(unix_day_start("2026-03-12")),
            to_ts: Some(unix_day_start("2026-04-25").saturating_sub(1)),
            selected_patch: Some("2026-03-12".to_string()),
        };

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
        apply_patch_range_override(&mut patch_filter, from_patch_id, to_patch_id);

        assert_eq!(patch_filter.selected_patch, None);
        assert_eq!(patch_filter.from_ts, None);
        assert_eq!(patch_filter.to_ts, None);
    }
}
