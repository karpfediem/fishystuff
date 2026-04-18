use crate::bridge::contract::FishyMapInputState;
use crate::plugins::camera::Map2dCamera;
use crate::prelude::*;

pub(super) fn apply_theme_background(
    input: &FishyMapInputState,
    clear_color: &mut ClearColor,
    map_camera_q: &mut Query<&mut Camera, With<Map2dCamera>>,
) {
    if let Some(color) = super::super::super::parse_theme_background_color(&input.theme.colors) {
        clear_color.0 = color;
        if let Ok(mut map_camera) = map_camera_q.single_mut() {
            map_camera.clear_color = ClearColorConfig::Custom(color);
        }
    }
}
