use std::collections::{HashMap, HashSet};

use async_channel::{Receiver, TryRecvError};
use bevy::prelude::*;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::layers::{
    GeometrySpace, LayerId, LayerManifestStatus, LayerRegistry, LayerRuntime,
};
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::MapPoint;
use crate::plugins::api::{MapDisplayState, POINT_ICON_SCALE_MAX, POINT_ICON_SCALE_MIN};
use crate::plugins::camera::Map2dCamera;
use crate::plugins::render_domain::{world_2d_layers, World2dRenderEntity};
use crate::plugins::svg_icons::{UiSvgIconAssets, UiSvgIconKind};
use crate::runtime_io;

const WAYPOINT_MARKER_SIZE_SCREEN_PX: f32 = 18.0;
const WAYPOINT_Z_OFFSET: f32 = 0.05;

pub struct WaypointLayersPlugin;

impl Plugin for WaypointLayersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WaypointLayerRuntime>()
            .add_systems(Update, (update_waypoint_layers, sync_waypoint_markers));
    }
}

#[derive(Resource, Default)]
struct WaypointLayerRuntime {
    states: HashMap<LayerId, WaypointLayerState>,
}

#[derive(Default)]
struct WaypointLayerState {
    source_key: Option<String>,
    pending: Option<Receiver<Result<WaypointFeatureCollection, String>>>,
    markers: Vec<WaypointMarkerRecord>,
    entities: Vec<Entity>,
}

#[derive(Debug, Clone)]
struct WaypointMarkerRecord {
    world_x: f32,
    world_z: f32,
    label: Option<String>,
    feature_id: Option<String>,
}

#[derive(Component)]
struct WaypointLayerMarker {
    layer_id: LayerId,
    world_x: f32,
    world_z: f32,
}

#[derive(Deserialize)]
struct WaypointFeatureCollection {
    #[serde(default)]
    features: Vec<WaypointFeature>,
}

#[derive(Deserialize)]
struct WaypointFeature {
    #[serde(default)]
    properties: Map<String, Value>,
    #[serde(default)]
    geometry: Option<WaypointGeometry>,
}

#[derive(Deserialize)]
struct WaypointGeometry {
    #[serde(rename = "type")]
    geometry_type: String,
    #[serde(default)]
    coordinates: Vec<f64>,
}

fn update_waypoint_layers(
    mut commands: Commands,
    registry: Res<LayerRegistry>,
    mut layer_runtime: ResMut<LayerRuntime>,
    icon_assets: Res<UiSvgIconAssets>,
    mut runtime: ResMut<WaypointLayerRuntime>,
) {
    layer_runtime.sync_to_registry(&registry);
    prune_stale_waypoint_layers(&registry, &mut runtime, &mut commands);

    let icon = icon_assets.handle(UiSvgIconKind::MapPin);

    for layer in registry.ordered() {
        let Some(runtime_state) = layer_runtime.get_mut(layer.id) else {
            continue;
        };
        if !layer.is_waypoints() {
            runtime_state.manifest_status = LayerManifestStatus::Missing;
            continue;
        }
        let Some(source) = layer.waypoint_source.as_ref() else {
            runtime_state.manifest_status = LayerManifestStatus::Ready;
            clear_waypoint_layer_entities(
                &mut runtime.states.entry(layer.id).or_default().entities,
                &mut commands,
            );
            continue;
        };

        let state = runtime.states.entry(layer.id).or_default();
        let source_key = format!("{}#{}", source.url, source.revision);
        if state.source_key.as_deref() != Some(source_key.as_str()) {
            state.source_key = Some(source_key);
            state.pending = Some(runtime_io::spawn_json_request(source.url.clone()));
            state.markers.clear();
            clear_waypoint_layer_entities(&mut state.entities, &mut commands);
            runtime_state.manifest_status = LayerManifestStatus::Loading;
            continue;
        }

        if let Some(receiver) = state.pending.as_ref() {
            match receiver.try_recv() {
                Ok(result) => {
                    state.pending = None;
                    clear_waypoint_layer_entities(&mut state.entities, &mut commands);
                    match result {
                        Ok(collection) => {
                            state.markers = parse_waypoint_markers(source, collection);
                            runtime_state.manifest_status = LayerManifestStatus::Ready;
                        }
                        Err(err) => {
                            bevy::log::warn!(
                                "failed to load waypoint layer {} from {}: {}",
                                layer.key,
                                source.url,
                                err
                            );
                            state.markers.clear();
                            runtime_state.manifest_status = LayerManifestStatus::Failed;
                        }
                    }
                }
                Err(TryRecvError::Closed) => {
                    state.pending = None;
                    state.markers.clear();
                    clear_waypoint_layer_entities(&mut state.entities, &mut commands);
                    runtime_state.manifest_status = LayerManifestStatus::Failed;
                }
                Err(TryRecvError::Empty) => {
                    runtime_state.manifest_status = LayerManifestStatus::Loading;
                }
            }
        }

        if runtime_state.manifest_status == LayerManifestStatus::Ready
            && state.entities.is_empty()
            && !state.markers.is_empty()
        {
            if let Some(icon) = icon.clone() {
                state.entities =
                    spawn_waypoint_marker_entities(&mut commands, layer.id, &state.markers, icon);
            }
        }
    }
}

fn sync_waypoint_markers(
    display_state: Res<MapDisplayState>,
    view_mode: Res<ViewModeState>,
    layer_runtime: Res<LayerRuntime>,
    camera_q: Query<&Projection, With<Map2dCamera>>,
    mut markers: Query<(
        &WaypointLayerMarker,
        &mut Transform,
        &mut Visibility,
        &mut Sprite,
    )>,
) {
    let camera_scale = camera_q
        .single()
        .ok()
        .and_then(|projection| match projection {
            Projection::Orthographic(ortho) => Some(ortho.scale.max(f32::EPSILON)),
            _ => None,
        })
        .unwrap_or(1.0);
    let user_scale = display_state
        .point_icon_scale
        .clamp(POINT_ICON_SCALE_MIN, POINT_ICON_SCALE_MAX);
    let marker_size = WAYPOINT_MARKER_SIZE_SCREEN_PX * camera_scale * user_scale;
    let ui_visible = view_mode.mode == ViewMode::Map2D;

    for (marker, mut transform, mut visibility, mut sprite) in &mut markers {
        let Some(state) = layer_runtime.get(marker.layer_id) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let layer_visible = ui_visible && state.visible;
        transform.translation = Vec3::new(
            marker.world_x,
            marker.world_z,
            state.z_base + WAYPOINT_Z_OFFSET,
        );
        sprite.custom_size = Some(Vec2::splat(marker_size));
        sprite.color = Color::srgba(1.0, 1.0, 1.0, state.opacity.clamp(0.0, 1.0));
        *visibility = if layer_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn prune_stale_waypoint_layers(
    registry: &LayerRegistry,
    runtime: &mut WaypointLayerRuntime,
    commands: &mut Commands,
) {
    let valid_ids: HashSet<LayerId> = registry
        .ordered()
        .iter()
        .filter(|layer| layer.is_waypoints())
        .map(|layer| layer.id)
        .collect();
    runtime.states.retain(|layer_id, state| {
        if valid_ids.contains(layer_id) {
            true
        } else {
            clear_waypoint_layer_entities(&mut state.entities, commands);
            false
        }
    });
}

fn clear_waypoint_layer_entities(entities: &mut Vec<Entity>, commands: &mut Commands) {
    for entity in entities.drain(..) {
        commands.entity(entity).despawn();
    }
}

fn spawn_waypoint_marker_entities(
    commands: &mut Commands,
    layer_id: LayerId,
    markers: &[WaypointMarkerRecord],
    icon: Handle<Image>,
) -> Vec<Entity> {
    let mut entities = Vec::with_capacity(markers.len());
    for marker in markers {
        let name = marker
            .label
            .clone()
            .or_else(|| marker.feature_id.clone())
            .unwrap_or_else(|| "Waypoint".to_string());
        let entity = commands
            .spawn((
                Name::new(format!("Waypoint {name}")),
                WaypointLayerMarker {
                    layer_id,
                    world_x: marker.world_x,
                    world_z: marker.world_z,
                },
                World2dRenderEntity,
                world_2d_layers(),
                Sprite {
                    image: icon.clone(),
                    custom_size: Some(Vec2::splat(WAYPOINT_MARKER_SIZE_SCREEN_PX)),
                    ..default()
                },
                Transform::from_xyz(marker.world_x, marker.world_z, 0.0),
                Visibility::Hidden,
            ))
            .id();
        entities.push(entity);
    }
    entities
}

fn parse_waypoint_markers(
    source: &crate::map::layers::WaypointSourceSpec,
    collection: WaypointFeatureCollection,
) -> Vec<WaypointMarkerRecord> {
    collection
        .features
        .into_iter()
        .filter_map(|feature| waypoint_record_from_feature(source, feature))
        .collect()
}

fn waypoint_record_from_feature(
    source: &crate::map::layers::WaypointSourceSpec,
    feature: WaypointFeature,
) -> Option<WaypointMarkerRecord> {
    let geometry = feature.geometry?;
    if geometry.geometry_type != "Point" || geometry.coordinates.len() < 2 {
        return None;
    }
    let (world_x, world_z) = waypoint_world_position(
        source.geometry_space,
        geometry.coordinates[0],
        geometry.coordinates[1],
    );
    let label = source
        .label_property
        .as_ref()
        .and_then(|key| string_property(&feature.properties, key));
    let feature_id = source
        .feature_id_property
        .as_ref()
        .and_then(|key| string_property(&feature.properties, key));
    Some(WaypointMarkerRecord {
        world_x,
        world_z,
        label,
        feature_id,
    })
}

fn waypoint_world_position(geometry_space: GeometrySpace, x: f64, y: f64) -> (f32, f32) {
    match geometry_space {
        GeometrySpace::World => (x as f32, y as f32),
        GeometrySpace::MapPixels => {
            let world = MapToWorld::default().map_to_world(MapPoint::new(x, y));
            (world.x as f32, world.z as f32)
        }
    }
}

fn string_property(properties: &Map<String, Value>, key: &str) -> Option<String> {
    let value = properties.get(key)?;
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_waypoint_markers, GeometrySpace, WaypointFeatureCollection};
    use crate::map::layers::WaypointSourceSpec;

    #[test]
    fn parse_waypoint_markers_reads_world_space_points() {
        let collection: WaypointFeatureCollection = serde_json::from_str(
            r#"{
                "features": [
                    {
                        "properties": {"r": 204, "label": "Stonebeak Shore (R204)"},
                        "geometry": {"type": "Point", "coordinates": [303191.0, -1694.35]}
                    }
                ]
            }"#,
        )
        .expect("parse");
        let markers = parse_waypoint_markers(
            &WaypointSourceSpec {
                url: "/waypoints/region_nodes.v1.geojson".to_string(),
                revision: "region-nodes-v1".to_string(),
                geometry_space: GeometrySpace::World,
                feature_id_property: Some("r".to_string()),
                label_property: Some("label".to_string()),
            },
            collection,
        );

        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].feature_id.as_deref(), Some("204"));
        assert_eq!(markers[0].label.as_deref(), Some("Stonebeak Shore (R204)"));
        assert!((markers[0].world_x - 303191.0).abs() < f32::EPSILON);
        assert!((markers[0].world_z + 1694.35).abs() < 0.01);
    }
}
