use bevy::asset::{AssetMetaCheck, AssetPlugin, UnapprovedPathMode};
use bevy::asset::io::web::WebAssetPlugin;
use bevy::prelude::*;
use bevy::window::{Window, WindowPlugin};
use bevy_flair::prelude::FlairPlugin;

use crate::bridge::host::BrowserBridgePlugin;
use crate::plugins::camera::initial_resolution;
use crate::plugins::FishystuffPlugins;

pub fn run() {
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
