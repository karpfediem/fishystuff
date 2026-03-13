use bevy::prelude::*;

pub struct RasterPlugin;

impl Plugin for RasterPlugin {
    fn build(&self, app: &mut App) {
        crate::map::raster::build_plugin(app);
    }
}
