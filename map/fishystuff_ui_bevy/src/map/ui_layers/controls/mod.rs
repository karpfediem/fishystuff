mod debug;
mod layers;
mod view;

pub(in crate::map::ui_layers) use debug::{
    handle_debug_toggle_clicks, handle_eviction_toggle_clicks, sync_debug_toggle_label,
    sync_eviction_toggle_label,
};
pub(in crate::map::ui_layers) use layers::{
    handle_layer_opacity_clicks, handle_layer_toggle_clicks, sync_layer_labels,
};
pub(in crate::map::ui_layers) use view::{
    handle_terrain_tuning_clicks, handle_view_mode_clicks, handle_view_toggle_clicks,
    show_drape_label, sync_terrain_tuning_labels, sync_view_mode_labels, sync_view_toggle_labels,
};
