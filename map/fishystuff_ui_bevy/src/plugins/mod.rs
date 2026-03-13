pub mod api;
pub mod camera;
pub mod diagnostics;
pub mod input;
pub mod mask;
pub mod points;
pub mod raster;
pub mod render_domain;
pub mod ui;
pub mod vector_layers;

use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

use crate::map::terrain::runtime::Terrain3dPlugin;
use crate::map::ui_layers::LayerUiPlugin;

pub struct FishystuffPlugins;

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
