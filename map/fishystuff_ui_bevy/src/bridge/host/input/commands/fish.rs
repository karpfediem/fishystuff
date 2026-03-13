use crate::bridge::host::BrowserBridgeState;
use crate::plugins::api::{FishCatalog, FishFilterState};

pub(super) fn apply_focus_fish_command(
    bridge: &mut BrowserBridgeState,
    fish: &FishCatalog,
    fish_filter: &mut FishFilterState,
    fish_id: i32,
) {
    bridge.input.filters.fish_ids.retain(|id| *id != fish_id);
    bridge.input.filters.fish_ids.push(fish_id);
    fish_filter.selected_fish_ids = bridge.input.filters.fish_ids.clone();
    fish_filter.selected_fish = Some(fish_id);
    fish_filter.selected_fish_name = fish
        .entries
        .iter()
        .find(|entry| entry.id == fish_id)
        .map(|entry| entry.name.clone());
}
