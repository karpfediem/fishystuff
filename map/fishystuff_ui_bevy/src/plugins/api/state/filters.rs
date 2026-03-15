use fishystuff_api::models::meta::PatchInfo;

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
