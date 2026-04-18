use bevy::prelude::World;
use clap::ValueEnum;

use crate::bridge::contract::{
    FishyMapSearchExpressionNode, FishyMapSearchExpressionOperator, FishyMapSearchTerm,
};
use crate::map::camera::map2d::Map2dViewState;
use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::camera::terrain3d::Terrain3dViewState;
use crate::map::layers::{LayerRegistry, LayerRuntime};
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::MapPoint;
use crate::plugins::api::{
    FishFilterState, LayerEffectiveFilterState, MapDisplayState, SearchExpressionState,
};

const MAP_MIN: f32 = 256.0;
const MAP_MAX: f32 = 1792.0;
const MAP_CENTER: f32 = 1024.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ScenarioName {
    #[value(name = "load_map")]
    LoadMap,
    #[value(name = "raster_2d_pan_zoom")]
    Raster2dPanZoom,
    #[value(name = "points_overlay_filtering")]
    PointsOverlayFiltering,
    #[value(name = "zone_mask_hover_filtering")]
    ZoneMaskHoverFiltering,
    #[value(name = "vector_region_groups_enable")]
    VectorRegionGroupsEnable,
    #[value(name = "terrain3d_enter_and_orbit")]
    Terrain3dEnterAndOrbit,
    #[value(name = "mode_switch_2d_3d_2d")]
    ModeSwitch2d3d2d,
}

impl ScenarioName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LoadMap => "load_map",
            Self::Raster2dPanZoom => "raster_2d_pan_zoom",
            Self::PointsOverlayFiltering => "points_overlay_filtering",
            Self::ZoneMaskHoverFiltering => "zone_mask_hover_filtering",
            Self::VectorRegionGroupsEnable => "vector_region_groups_enable",
            Self::Terrain3dEnterAndOrbit => "terrain3d_enter_and_orbit",
            Self::ModeSwitch2d3d2d => "mode_switch_2d_3d_2d",
        }
    }

    pub fn default_frames(self) -> u64 {
        match self {
            Self::LoadMap => 240,
            Self::Raster2dPanZoom => 600,
            Self::PointsOverlayFiltering => 600,
            Self::ZoneMaskHoverFiltering => 600,
            Self::VectorRegionGroupsEnable => 480,
            Self::Terrain3dEnterAndOrbit => 540,
            Self::ModeSwitch2d3d2d => 540,
        }
    }

    pub fn apply(self, world: &mut World, frame: u64, total_frames: u64) {
        let total_frames = total_frames.max(1);
        match self {
            Self::LoadMap => {
                let _ = frame;
                let _ = total_frames;
                configure_common_layers(world, false, false);
                set_map_2d_view(world, MAP_CENTER, MAP_CENTER, 1.0);
            }
            Self::Raster2dPanZoom => {
                configure_common_layers(world, false, false);
                let progress = frame as f32 / total_frames as f32;
                let map_x = sweep(progress, MAP_MIN, MAP_MAX);
                let map_y = sweep(progress * 1.7, MAP_MIN, MAP_MAX);
                let zoom = 0.75 + oscillate(progress * 2.5) * 2.75;
                set_map_2d_view(world, map_x, map_y, zoom.max(0.35));
            }
            Self::PointsOverlayFiltering => {
                configure_common_layers(world, true, false);
                let progress = frame as f32 / total_frames as f32;
                let map_x = sweep(progress * 1.2, MAP_MIN, MAP_MAX);
                let map_y = sweep(progress * 0.8, MAP_MIN, MAP_MAX);
                let zoom = 0.65 + oscillate(progress * 3.0) * 1.4;
                set_map_2d_view(world, map_x, map_y, zoom.max(0.35));

                let selected_fish_ids: &[i32] = match ((frame / 120) % 4) as usize {
                    0 => &[],
                    1 => &[101],
                    2 => &[202],
                    _ => &[101, 202],
                };
                set_selected_fish_ids(world, selected_fish_ids);
            }
            Self::ZoneMaskHoverFiltering => {
                configure_common_layers(world, false, false);
                set_map_2d_view(world, MAP_CENTER, MAP_CENTER, 1.0);
                set_search_expression(world, broad_color_expression());
                set_hovered_zone_from_effective_zone_mask_filter(world, frame / 30);
            }
            Self::VectorRegionGroupsEnable => {
                configure_common_layers(world, false, true);
                let progress = frame as f32 / total_frames as f32;
                let map_x = sweep(progress * 0.9, MAP_MIN, MAP_MAX);
                let map_y = sweep(progress * 1.4, MAP_MIN, MAP_MAX);
                let zoom = 0.9 + oscillate(progress * 2.0) * 1.8;
                set_map_2d_view(world, map_x, map_y, zoom.max(0.45));

                let enable_after = total_frames / 4;
                set_layer_visibility(world, "region_groups", frame >= enable_after);
            }
            Self::Terrain3dEnterAndOrbit => {
                configure_common_layers(world, false, false);
                let enter_after = total_frames / 5;
                if frame < enter_after {
                    set_map_2d_view(world, MAP_CENTER, MAP_CENTER, 0.9);
                } else {
                    let orbit_progress = (frame.saturating_sub(enter_after)) as f32
                        / (total_frames - enter_after).max(1) as f32;
                    let yaw = orbit_progress * std::f32::consts::TAU * 0.8;
                    let pitch = -0.55 + oscillate(orbit_progress * 1.3) * 0.28;
                    let distance = 60_000.0 + oscillate(orbit_progress * 1.8) * 20_000.0;
                    set_terrain_view(world, MAP_CENTER, MAP_CENTER, yaw, pitch, distance);
                }
            }
            Self::ModeSwitch2d3d2d => {
                configure_common_layers(world, false, true);
                let first = total_frames / 3;
                let second = (total_frames * 2) / 3;
                if frame < first {
                    let progress = frame as f32 / first.max(1) as f32;
                    set_map_2d_view(
                        world,
                        sweep(progress, MAP_MIN, MAP_MAX),
                        MAP_CENTER,
                        0.8 + oscillate(progress) * 1.4,
                    );
                    set_layer_visibility(world, "region_groups", false);
                } else if frame < second {
                    let progress = (frame - first) as f32 / (second - first).max(1) as f32;
                    set_layer_visibility(world, "region_groups", true);
                    set_terrain_view(
                        world,
                        MAP_CENTER,
                        MAP_CENTER,
                        progress * std::f32::consts::TAU * 0.6,
                        -0.6 + oscillate(progress * 1.2) * 0.22,
                        70_000.0 + oscillate(progress * 2.2) * 15_000.0,
                    );
                } else {
                    let progress = (frame - second) as f32 / (total_frames - second).max(1) as f32;
                    set_map_2d_view(
                        world,
                        MAP_CENTER,
                        sweep(progress, MAP_MAX, MAP_MIN),
                        0.7 + oscillate(progress * 2.0) * 2.2,
                    );
                    set_layer_visibility(world, "region_groups", true);
                }
            }
        }
    }
}

fn configure_common_layers(world: &mut World, show_points: bool, allow_vector: bool) {
    let needs_display_update = {
        let display = world.resource::<MapDisplayState>();
        display.show_points != show_points
            || display.show_point_icons
            || !display.show_zone_mask
            || (display.zone_mask_opacity - 0.5).abs() > f32::EPSILON
    };
    if needs_display_update {
        let mut display = world.resource_mut::<MapDisplayState>();
        display.show_points = show_points;
        display.show_point_icons = false;
        display.show_zone_mask = true;
        display.zone_mask_opacity = 0.5;
    }
    set_layer_visibility(world, "minimap", true);
    set_layer_visibility(world, "zone_mask", true);
    if !allow_vector {
        set_layer_visibility(world, "region_groups", false);
    }
}

fn set_map_2d_view(world: &mut World, map_x: f32, map_y: f32, zoom: f32) {
    let map_to_world = MapToWorld::default();
    let world_point = map_to_world.map_to_world(MapPoint::new(map_x as f64, map_y as f64));
    if world.resource::<ViewModeState>().mode != ViewMode::Map2D {
        let mut mode = world.resource_mut::<ViewModeState>();
        mode.mode = ViewMode::Map2D;
    }
    let desired_view = Map2dViewState {
        center_world_x: world_point.x as f32,
        center_world_z: world_point.z as f32,
        zoom: zoom.max(0.25),
    };
    if *world.resource::<Map2dViewState>() != desired_view {
        *world.resource_mut::<Map2dViewState>() = desired_view;
    }
}

fn set_terrain_view(
    world: &mut World,
    map_x: f32,
    map_y: f32,
    yaw: f32,
    pitch: f32,
    distance: f32,
) {
    let map_to_world = MapToWorld::default();
    let world_point = map_to_world.map_to_world(MapPoint::new(map_x as f64, map_y as f64));
    let needs_mode_update = {
        let mode = world.resource::<ViewModeState>();
        mode.mode != ViewMode::Terrain3D || !mode.terrain_initialized
    };
    if needs_mode_update {
        let mut mode = world.resource_mut::<ViewModeState>();
        mode.mode = ViewMode::Terrain3D;
        mode.terrain_initialized = true;
    }
    let mut desired_view = *world.resource::<Terrain3dViewState>();
    desired_view.pivot_world =
        bevy::math::Vec3::new(world_point.x as f32, 0.0, world_point.z as f32);
    desired_view.yaw = yaw;
    desired_view.pitch = pitch;
    desired_view.set_distance_clamped(distance);
    if *world.resource::<Terrain3dViewState>() != desired_view {
        *world.resource_mut::<Terrain3dViewState>() = desired_view;
    }
}

fn set_layer_visibility(world: &mut World, key: &str, visible: bool) {
    let layer_id = world.resource::<LayerRegistry>().id_by_key(key);
    let Some(layer_id) = layer_id else {
        return;
    };
    if world.resource::<LayerRuntime>().visible(layer_id) != visible {
        world
            .resource_mut::<LayerRuntime>()
            .set_visible(layer_id, visible);
    }
}

fn set_selected_fish_ids(world: &mut World, selected_fish_ids: &[i32]) {
    if world
        .resource::<FishFilterState>()
        .selected_fish_ids
        .as_slice()
        != selected_fish_ids
    {
        world.resource_mut::<FishFilterState>().selected_fish_ids = selected_fish_ids.to_vec();
    }
}

fn set_search_expression(world: &mut World, expression: FishyMapSearchExpressionNode) {
    if world.resource::<SearchExpressionState>().expression != expression {
        world.resource_mut::<SearchExpressionState>().expression = expression;
    }
}

fn set_hovered_zone_from_effective_zone_mask_filter(world: &mut World, step: u64) {
    let zone_rgbs = world
        .resource::<LayerEffectiveFilterState>()
        .zone_membership_filter("zone_mask")
        .filter(|filter| filter.active && !filter.zone_rgbs.is_empty())
        .map(|filter| {
            let mut zone_rgbs = filter.zone_rgbs.iter().copied().collect::<Vec<_>>();
            zone_rgbs.sort_unstable();
            zone_rgbs
        })
        .unwrap_or_default();
    let hovered_zone_rgb = if zone_rgbs.is_empty() {
        None
    } else {
        Some(zone_rgbs[step as usize % zone_rgbs.len()])
    };
    if world.resource::<MapDisplayState>().hovered_zone_rgb != hovered_zone_rgb {
        world.resource_mut::<MapDisplayState>().hovered_zone_rgb = hovered_zone_rgb;
    }
}

fn broad_color_expression() -> FishyMapSearchExpressionNode {
    FishyMapSearchExpressionNode::Group {
        operator: FishyMapSearchExpressionOperator::Or,
        children: ["red", "yellow", "blue", "green", "white"]
            .into_iter()
            .map(|term| FishyMapSearchExpressionNode::Term {
                term: FishyMapSearchTerm::FishFilter {
                    term: term.to_string(),
                },
                negated: false,
            })
            .collect(),
        negated: false,
    }
}

fn oscillate(value: f32) -> f32 {
    ((value * std::f32::consts::TAU).sin() * 0.5) + 0.5
}

fn sweep(progress: f32, min: f32, max: f32) -> f32 {
    let width = max - min;
    let triangle = if progress.fract() < 0.5 {
        progress.fract() * 2.0
    } else {
        2.0 - progress.fract() * 2.0
    };
    min + width * triangle.clamp(0.0, 1.0)
}
