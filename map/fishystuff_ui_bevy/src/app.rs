#[cfg(not(target_arch = "wasm32"))]
use bevy::app::{PanicHandlerPlugin, TaskPoolPlugin};
#[cfg(not(target_arch = "wasm32"))]
use bevy::asset::AssetApp;
use bevy::asset::{AssetMetaCheck, AssetPlugin, UnapprovedPathMode};
#[cfg(not(target_arch = "wasm32"))]
use bevy::diagnostic::{DiagnosticsPlugin, FrameCountPlugin};
#[cfg(not(target_arch = "wasm32"))]
use bevy::image::ImagePlugin;
#[cfg(not(target_arch = "wasm32"))]
use bevy::input::InputPlugin as BevyInputPlugin;
#[cfg(not(target_arch = "wasm32"))]
use bevy::light::GlobalAmbientLight;
#[cfg(not(target_arch = "wasm32"))]
use bevy::log::LogPlugin;
#[cfg(not(target_arch = "wasm32"))]
use bevy::mesh::MeshPlugin;
use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy::time::TimePlugin;
#[cfg(not(target_arch = "wasm32"))]
use bevy::transform::TransformPlugin;
#[cfg(not(target_arch = "wasm32"))]
use bevy::window::ExitCondition;
use bevy::window::{Window, WindowPlugin};
use bevy_flair::prelude::FlairPlugin;

#[cfg(target_arch = "wasm32")]
use crate::bridge::host::BrowserBridgePlugin;
#[cfg(not(target_arch = "wasm32"))]
use crate::map::terrain::runtime::Terrain3dPlugin;
#[cfg(not(target_arch = "wasm32"))]
use crate::plugins::bookmarks::BookmarksPlugin;
#[cfg(target_arch = "wasm32")]
use crate::plugins::camera::initial_resolution;
#[cfg(not(target_arch = "wasm32"))]
use crate::plugins::points::PointsPlugin;
#[cfg(not(target_arch = "wasm32"))]
use crate::plugins::raster::RasterPlugin;
#[cfg(not(target_arch = "wasm32"))]
use crate::plugins::ui::{UiFonts, UiPointerCapture};
#[cfg(not(target_arch = "wasm32"))]
use crate::plugins::vector_layers::VectorLayersPlugin;
#[cfg(target_arch = "wasm32")]
use crate::plugins::FishystuffPlugins;
#[cfg(not(target_arch = "wasm32"))]
use crate::plugins::{api::ApiPlugin, camera::CameraPlugin, input::InputPlugin};
#[cfg(target_arch = "wasm32")]
use crate::profiling::browser::BrowserProfilingPlugin;
#[cfg(target_arch = "wasm32")]
use bevy::asset::io::web::WebAssetPlugin;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub struct NativeAppOptions {
    pub asset_root: String,
    pub width: u32,
    pub height: u32,
    pub visible: bool,
    pub renderless: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for NativeAppOptions {
    fn default() -> Self {
        Self {
            asset_root: ".".to_string(),
            width: 1280,
            height: 720,
            visible: false,
            renderless: false,
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn run_browser() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WebAssetPlugin {
                    silence_startup_warning: true,
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "FishyStuff Zones".to_string(),
                        canvas: Some("#bevy".to_string()),
                        resolution: initial_resolution(),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: "".to_string(),
                    meta_check: AssetMetaCheck::Never,
                    unapproved_path_mode: UnapprovedPathMode::Allow,
                    ..default()
                }),
        )
        .add_plugins(FlairPlugin)
        .add_plugins(BrowserProfilingPlugin)
        .add_plugins(BrowserBridgePlugin)
        .add_plugins(FishystuffPlugins)
        .run();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn build_native_app(options: &NativeAppOptions) -> App {
    let mut app = if options.renderless {
        build_headless_native_app(options)
    } else {
        build_windowed_native_app(options)
    };
    app.init_resource::<UiPointerCapture>()
        .init_resource::<UiFonts>()
        .add_plugins(ApiPlugin)
        .add_plugins(CameraPlugin)
        .add_plugins(Terrain3dPlugin)
        .add_plugins(InputPlugin)
        .add_plugins(RasterPlugin)
        .add_plugins(VectorLayersPlugin)
        .add_plugins(BookmarksPlugin)
        .add_plugins(PointsPlugin);
    app
}

#[cfg(not(target_arch = "wasm32"))]
fn build_windowed_native_app(options: &NativeAppOptions) -> App {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "FishyStuff Zones Profiling".to_string(),
                    resolution: (options.width, options.height).into(),
                    visible: options.visible,
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: options.asset_root.clone(),
                meta_check: AssetMetaCheck::Never,
                unapproved_path_mode: UnapprovedPathMode::Allow,
                ..default()
            }),
    )
    .add_plugins(FlairPlugin);
    app
}

#[cfg(not(target_arch = "wasm32"))]
fn build_headless_native_app(options: &NativeAppOptions) -> App {
    let mut app = App::new();
    app.add_plugins((
        PanicHandlerPlugin,
        LogPlugin::default(),
        TaskPoolPlugin::default(),
        FrameCountPlugin,
        TimePlugin,
        TransformPlugin,
        DiagnosticsPlugin,
        BevyInputPlugin,
        WindowPlugin {
            primary_window: Some(Window {
                title: "FishyStuff Zones Profiling".to_string(),
                resolution: (options.width, options.height).into(),
                visible: false,
                ..default()
            }),
            exit_condition: ExitCondition::DontExit,
            close_when_requested: false,
            ..default()
        },
        AssetPlugin {
            file_path: options.asset_root.clone(),
            meta_check: AssetMetaCheck::Never,
            unapproved_path_mode: UnapprovedPathMode::Allow,
            ..default()
        },
        ImagePlugin::default(),
        MeshPlugin,
    ))
    .add_plugins(FlairPlugin)
    .init_resource::<GlobalAmbientLight>()
    .init_asset::<Font>()
    .init_asset::<ColorMaterial>()
    .init_asset::<StandardMaterial>();
    app
}

#[cfg(test)]
mod tests {
    use super::{build_native_app, NativeAppOptions};
    use crate::map::ui_layers::LayerUiPlugin;
    use crate::plugins::api::ApiPlugin;
    use crate::plugins::mask::MaskPlugin;
    use crate::plugins::ui::UiPlugin;

    #[test]
    fn ui_and_layer_plugins_start_without_access_panics() {
        let mut app = build_native_app(&NativeAppOptions {
            renderless: true,
            ..NativeAppOptions::default()
        });
        app.add_plugins(ApiPlugin)
            .add_plugins(MaskPlugin)
            .add_plugins(UiPlugin)
            .add_plugins(LayerUiPlugin);

        app.update();
    }
}
