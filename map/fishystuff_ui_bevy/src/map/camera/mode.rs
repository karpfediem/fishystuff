use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    #[default]
    Map2D,
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ViewModeState {
    pub mode: ViewMode,
}
