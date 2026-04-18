use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

pub const LAYER_WORLD_2D: usize = 0;
pub const LAYER_UI: usize = 1;

#[derive(Component, Debug)]
pub struct World2dRenderEntity;

#[derive(Component, Debug)]
pub struct UiRenderEntity;

pub fn world_2d_layers() -> RenderLayers {
    RenderLayers::layer(LAYER_WORLD_2D)
}

pub fn ui_layers() -> RenderLayers {
    RenderLayers::layer(LAYER_UI)
}

#[cfg(test)]
mod tests {
    use super::{ui_layers, world_2d_layers};

    #[test]
    fn ui_layers_are_isolated_from_world_layers() {
        assert!(!ui_layers().intersects(&world_2d_layers()));
    }
}
