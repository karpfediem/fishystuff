mod controls;
mod diagnostics;
mod panel;

use std::collections::BTreeMap;

use bevy::prelude::*;
use bevy::ui::FocusPolicy;
use bevy_flair::prelude::{ClassList, NodeStyleSheet};

use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::camera::terrain3d::{reset_terrain3d_view, Terrain3dViewState};
use crate::map::events::EventsSnapshotState;
use crate::map::layers::{LayerId, LayerKind, LayerRegistry, LayerSettings, VectorSourceSpec};
use crate::map::raster::{RasterTileCache, TileDebugControls, TileStats};
use crate::map::terrain::Terrain3dConfig;
use crate::plugins::api::MapDisplayState;
use crate::plugins::points::{PointIconCache, PointsState};
use crate::plugins::ui::{UiFonts, UiPointerBlocker, UiStartupSet};

pub struct LayerUiPlugin;
const LAYER_MENU_WIDTH: f32 = 300.0;
const LAYER_MENU_HEIGHT: f32 = 560.0;

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct LayerDebugSettings {
    pub enabled: bool,
}

#[derive(Component)]
struct LayerPanel;

#[derive(Component)]
struct LayerToggleButton {
    id: LayerId,
}

#[derive(Component)]
struct LayerToggleText {
    id: LayerId,
}

#[derive(Component)]
struct LayerOpacityDown {
    id: LayerId,
}

#[derive(Component)]
struct LayerOpacityUp {
    id: LayerId,
}

#[derive(Component)]
struct LayerLabel {
    id: LayerId,
}

#[derive(Component)]
struct LayerDebugText;

#[derive(Component)]
struct LayerDebugToggleButton;

#[derive(Component)]
struct LayerDebugToggleText;

#[derive(Component)]
struct LayerEvictionToggleButton;

#[derive(Component)]
struct LayerEvictionToggleText;

#[derive(Clone, Copy, Debug)]
enum ViewToggleKind {
    Effort,
    Points,
    PointIcons,
    Drift,
}

#[derive(Component)]
struct ViewToggleButton {
    kind: ViewToggleKind,
}

#[derive(Component)]
struct ViewToggleText {
    kind: ViewToggleKind,
}

#[derive(Component)]
struct ViewModeButton {
    mode: ViewMode,
}

#[derive(Component)]
struct ViewModeText {
    mode: ViewMode,
}

#[derive(Component)]
struct Reset3dViewButton;

#[derive(Component)]
struct TerrainShowDrapeToggle;

#[derive(Component)]
struct TerrainShowDrapeText;

impl Plugin for LayerUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LayerSettings>()
            .init_resource::<LayerDebugSettings>()
            .add_systems(Startup, panel::setup_layer_ui.after(UiStartupSet))
            .add_systems(
                Update,
                (
                    panel::rebuild_layer_ui_on_registry_change,
                    controls::handle_view_toggle_clicks,
                    controls::handle_view_mode_clicks,
                    controls::handle_terrain_tuning_clicks,
                    controls::handle_layer_toggle_clicks,
                    controls::handle_layer_opacity_clicks,
                    controls::handle_debug_toggle_clicks,
                    controls::handle_eviction_toggle_clicks,
                    controls::sync_view_toggle_labels,
                ),
            )
            .add_systems(Update, controls::sync_view_mode_labels)
            .add_systems(Update, controls::sync_terrain_tuning_labels)
            .add_systems(Update, controls::sync_debug_toggle_label)
            .add_systems(Update, controls::sync_eviction_toggle_label)
            .add_systems(Update, controls::sync_layer_labels)
            .add_systems(Update, diagnostics::sync_layer_debug);
    }
}
