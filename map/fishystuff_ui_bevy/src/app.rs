use bevy::asset::{AssetMetaCheck, AssetPlugin, UnapprovedPathMode};
use bevy::prelude::*;
use bevy::render::{settings::WgpuSettings, RenderPlugin};
use bevy::window::{Window, WindowPlugin};
use bevy_flair::prelude::FlairPlugin;

#[cfg(target_arch = "wasm32")]
use crate::bridge::host::BrowserBridgePlugin;
use crate::map::terrain::runtime::Terrain3dPlugin;
use crate::plugins::api::{
    ApiBootstrapState, FishCatalog, FishFilterState, MapDisplayState, PatchFilterState,
    RemoteImageCache, RemoteImageEpoch,
};
#[cfg(target_arch = "wasm32")]
use crate::plugins::camera::initial_resolution;
use crate::plugins::points::PointsPlugin;
use crate::plugins::raster::RasterPlugin;
use crate::plugins::ui::UiPointerCapture;
use crate::plugins::vector_layers::VectorLayersPlugin;
#[cfg(target_arch = "wasm32")]
use crate::plugins::FishystuffPlugins;
use crate::plugins::{camera::CameraPlugin, input::InputPlugin};
#[cfg(target_arch = "wasm32")]
use bevy::asset::io::web::WebAssetPlugin;

#[derive(Debug, Clone)]
pub struct NativeAppOptions {
    pub asset_root: String,
    pub width: u32,
    pub height: u32,
    pub visible: bool,
    pub renderless: bool,
}

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
        .add_plugins(BrowserBridgePlugin)
        .add_plugins(FishystuffPlugins)
        .run();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn build_native_app(options: &NativeAppOptions) -> App {
    let mut app = App::new();
    let mut plugins = DefaultPlugins
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
        });

    if options.renderless {
        plugins = plugins.set(RenderPlugin {
            render_creation: WgpuSettings {
                backends: None,
                ..default()
            }
            .into(),
            ..default()
        });
    }

    app.add_plugins(plugins)
        .add_plugins(FlairPlugin)
        .init_resource::<UiPointerCapture>()
        .init_resource::<ApiBootstrapState>()
        .init_resource::<PatchFilterState>()
        .init_resource::<FishFilterState>()
        .init_resource::<MapDisplayState>()
        .init_resource::<FishCatalog>()
        .init_resource::<RemoteImageCache>()
        .init_resource::<RemoteImageEpoch>()
        .add_plugins(CameraPlugin)
        .add_plugins(Terrain3dPlugin)
        .add_plugins(InputPlugin)
        .add_plugins(RasterPlugin)
        .add_plugins(VectorLayersPlugin)
        .add_plugins(PointsPlugin);
    app
}
