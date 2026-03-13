use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Map2D,
    Terrain3D,
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewModeState {
    pub mode: ViewMode,
    pub terrain_initialized: bool,
}

impl Default for ViewModeState {
    fn default() -> Self {
        Self {
            mode: ViewMode::Map2D,
            terrain_initialized: false,
        }
    }
}
