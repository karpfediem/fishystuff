pub mod api;
pub mod camera;
#[cfg(target_arch = "wasm32")]
pub mod diagnostics;
pub mod input;
pub mod mask;
pub mod points;
pub mod raster;
pub mod render_domain;
pub mod ui;
pub mod vector_layers;

#[cfg(target_arch = "wasm32")]
use bevy::app::PluginGroupBuilder;
#[cfg(target_arch = "wasm32")]
use bevy::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::map::terrain::runtime::Terrain3dPlugin;
#[cfg(target_arch = "wasm32")]
use crate::map::ui_layers::LayerUiPlugin;

#[cfg(target_arch = "wasm32")]
pub struct FishystuffPlugins;

#[cfg(target_arch = "wasm32")]
impl PluginGroup for FishystuffPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(camera::CameraPlugin)
            .add(Terrain3dPlugin)
            .add(input::InputPlugin)
            .add(raster::RasterPlugin)
            .add(vector_layers::VectorLayersPlugin)
            .add(api::ApiPlugin)
            .add(points::PointsPlugin)
            .add(mask::MaskPlugin)
            .add(ui::UiPlugin)
            .add(LayerUiPlugin)
            .add(diagnostics::DiagnosticsPlugin)
    }
}
