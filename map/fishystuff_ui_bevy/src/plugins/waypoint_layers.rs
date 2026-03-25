use std::collections::{HashMap, HashSet};

use async_channel::{Receiver, TryRecvError};
use bevy::image::Image;
use bevy::prelude::*;
use bevy::text::{Font, Justify, TextLayout};
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::layers::{
    GeometrySpace, LayerId, LayerManifestStatus, LayerRegistry, LayerRuntime, WaypointSourceSpec,
};
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::MapPoint;
use crate::plugins::api::{MapDisplayState, POINT_ICON_SCALE_MAX, POINT_ICON_SCALE_MIN};
use crate::plugins::camera::CameraZoomBounds;
use crate::plugins::camera::Map2dCamera;
use crate::plugins::render_domain::{world_2d_layers, World2dRenderEntity};
use crate::plugins::svg_icons::{UiSvgIconAssets, UiSvgIconKind};
use crate::plugins::ui::{UiFonts, UiRoot};
use crate::runtime_io;

const WAYPOINT_MARKER_SIZE_SCREEN_PX: f32 = 18.0;
const WAYPOINT_CONNECTION_THICKNESS_SCREEN_PX: f32 = 2.0;
const WAYPOINT_LABEL_FONT_SIZE_SCREEN_PX: f32 = 7.0;
const WAYPOINT_LABEL_GAP_SCREEN_PX: f32 = 1.0;
const WAYPOINT_LABEL_SHADOW_OFFSET_X_SCREEN_PX: f32 = 0.75;
const WAYPOINT_LABEL_SHADOW_OFFSET_Y_SCREEN_PX: f32 = 0.75;
const WAYPOINT_LABEL_MAX_ZOOM_OUT_RATIO_OF_FIT: f32 = 0.15;
const WAYPOINT_LABEL_Z_INDEX: i32 = 1180;
const WAYPOINT_Z_OFFSET: f32 = 0.05;
const WAYPOINT_CONNECTION_Z_OFFSET: f32 = 0.02;
const WAYPOINT_LABEL_COLOR: Color = Color::srgb(0.98, 0.97, 0.94);
const WAYPOINT_LABEL_SHADOW_COLOR: Color = Color::srgba(0.08, 0.09, 0.11, 0.95);
const WAYPOINT_CONNECTION_COLOR: Color = Color::srgb(0.92, 0.88, 0.70);

pub struct WaypointLayersPlugin;

impl Plugin for WaypointLayersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WaypointLayerRuntime>()
            .add_systems(Update, (update_waypoint_layers, sync_waypoint_entities));
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
    features: ParsedWaypointFeatures,
    entities: Vec<Entity>,
}

#[derive(Default)]
struct ParsedWaypointFeatures {
    points: Vec<WaypointPointRecord>,
    connections: Vec<WaypointConnectionRecord>,
}

#[derive(Debug, Clone)]
struct WaypointPointRecord {
    world_x: f32,
    world_z: f32,
    label: Option<String>,
    map_label: Option<String>,
    feature_id: Option<String>,
    node_type: Option<String>,
    node_type_label: Option<String>,
    default_visible: bool,
}

#[derive(Debug, Clone, Copy)]
struct WaypointConnectionRecord {
    start_world_x: f32,
    start_world_z: f32,
    end_world_x: f32,
    end_world_z: f32,
    default_visible: bool,
}

#[derive(Component)]
struct WaypointLayerMarker {
    layer_id: LayerId,
    world_x: f32,
    world_z: f32,
    default_visible: bool,
}

#[derive(Component)]
struct WaypointLayerConnection {
    layer_id: LayerId,
    center_x: f32,
    center_z: f32,
    length_world: f32,
    angle_radians: f32,
    default_visible: bool,
}

#[derive(Component)]
struct WaypointLayerLabel {
    layer_id: LayerId,
    world_x: f32,
    world_z: f32,
    default_visible: bool,
}

#[derive(Component)]
struct WaypointLayerLabelRoot;

#[derive(Component)]
struct WaypointLayerLabelShadowText;

#[derive(Component)]
struct WaypointLayerLabelText;

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
    coordinates: Value,
}

fn update_waypoint_layers(
    mut commands: Commands,
    registry: Res<LayerRegistry>,
    mut layer_runtime: ResMut<LayerRuntime>,
    icon_assets: Res<UiSvgIconAssets>,
    fonts: Res<UiFonts>,
    ui_root_q: Query<Entity, With<UiRoot>>,
    mut runtime: ResMut<WaypointLayerRuntime>,
) {
    layer_runtime.sync_to_registry(&registry);
    prune_stale_waypoint_layers(&registry, &mut runtime, &mut commands);

    let icon = icon_assets.handle(UiSvgIconKind::MapPin);
    let ui_root = ui_root_q.single().ok();

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
            state.features = ParsedWaypointFeatures::default();
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
                            state.features = parse_waypoint_features(source, collection);
                            runtime_state.manifest_status = LayerManifestStatus::Ready;
                        }
                        Err(err) => {
                            bevy::log::warn!(
                                "failed to load waypoint layer {} from {}: {}",
                                layer.key,
                                source.url,
                                err
                            );
                            state.features = ParsedWaypointFeatures::default();
                            runtime_state.manifest_status = LayerManifestStatus::Failed;
                        }
                    }
                }
                Err(TryRecvError::Closed) => {
                    state.pending = None;
                    state.features = ParsedWaypointFeatures::default();
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
            && (!state.features.points.is_empty() || !state.features.connections.is_empty())
        {
            if let Some(icon) = icon.clone() {
                state.entities = spawn_waypoint_entities(
                    &mut commands,
                    layer.id,
                    &state.features,
                    icon,
                    fonts.regular.clone(),
                    ui_root,
                );
            }
        }
    }
}

fn sync_waypoint_entities(
    display_state: Res<MapDisplayState>,
    view_mode: Res<ViewModeState>,
    zoom_bounds: Res<CameraZoomBounds>,
    layer_runtime: Res<LayerRuntime>,
    camera_q: Query<(&Camera, &GlobalTransform, &Projection), With<Map2dCamera>>,
    mut markers: Query<
        (
            &WaypointLayerMarker,
            &mut Transform,
            &mut Visibility,
            &mut Sprite,
        ),
        (
            Without<WaypointLayerConnection>,
            Without<WaypointLayerLabelRoot>,
        ),
    >,
    mut connections: Query<
        (
            &WaypointLayerConnection,
            &mut Transform,
            &mut Visibility,
            &mut Sprite,
        ),
        (
            Without<WaypointLayerMarker>,
            Without<WaypointLayerLabelRoot>,
        ),
    >,
    mut label_roots: Query<
        (
            &WaypointLayerLabel,
            &mut Node,
            &mut Visibility,
            &ComputedNode,
            &Children,
        ),
        (
            With<WaypointLayerLabelRoot>,
            Without<WaypointLayerMarker>,
            Without<WaypointLayerConnection>,
        ),
    >,
    mut label_colors: Query<
        (
            &mut TextColor,
            Has<WaypointLayerLabelText>,
            Has<WaypointLayerLabelShadowText>,
        ),
        (
            Without<WaypointLayerLabelRoot>,
            Without<WaypointLayerMarker>,
            Without<WaypointLayerConnection>,
        ),
    >,
) {
    let (camera, camera_transform, projection) = match camera_q.single() {
        Ok(value) => value,
        Err(_) => return,
    };
    let camera_scale = match projection {
        Projection::Orthographic(ortho) => ortho.scale.max(f32::EPSILON),
        _ => 1.0,
    };
    let user_scale = display_state
        .point_icon_scale
        .clamp(POINT_ICON_SCALE_MIN, POINT_ICON_SCALE_MAX);
    let marker_size = WAYPOINT_MARKER_SIZE_SCREEN_PX * camera_scale * user_scale;
    let marker_screen_size = WAYPOINT_MARKER_SIZE_SCREEN_PX * user_scale;
    let connection_thickness = WAYPOINT_CONNECTION_THICKNESS_SCREEN_PX * camera_scale * user_scale;
    let ui_visible = view_mode.mode == ViewMode::Map2D;
    let label_max_scale = (zoom_bounds.fit_scale * WAYPOINT_LABEL_MAX_ZOOM_OUT_RATIO_OF_FIT)
        .max(zoom_bounds.min_scale);
    let labels_allowed_by_zoom = camera_scale <= label_max_scale;

    for (marker, mut transform, mut visibility, mut sprite) in &mut markers {
        let Some(state) = layer_runtime.get(marker.layer_id) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let layer_visible = ui_visible && state.visible && marker.default_visible;
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

    for (connection, mut transform, mut visibility, mut sprite) in &mut connections {
        let Some(state) = layer_runtime.get(connection.layer_id) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let layer_visible = ui_visible
            && state.visible
            && state.waypoint_connections_visible
            && connection.default_visible;
        transform.translation = Vec3::new(
            connection.center_x,
            connection.center_z,
            state.z_base + WAYPOINT_CONNECTION_Z_OFFSET,
        );
        transform.rotation = Quat::from_rotation_z(connection.angle_radians);
        sprite.custom_size = Some(Vec2::new(connection.length_world, connection_thickness));
        sprite.color = WAYPOINT_CONNECTION_COLOR.with_alpha(state.opacity.clamp(0.0, 1.0) * 0.65);
        *visibility = if layer_visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    for (label, mut node, mut visibility, computed, children) in &mut label_roots {
        let Some(state) = layer_runtime.get(label.layer_id) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let layer_visible = ui_visible
            && state.visible
            && state.waypoint_labels_visible
            && labels_allowed_by_zoom
            && label.default_visible;
        let Some(viewport_position) = world_to_viewport(
            camera,
            camera_transform,
            Vec3::new(label.world_x, label.world_z, 0.0),
        ) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        if !layer_visible {
            *visibility = Visibility::Hidden;
            continue;
        }
        let panel_size_px = {
            let inv = computed.inverse_scale_factor();
            computed.size() * inv
        };
        let left_px = viewport_position.x - panel_size_px.x * 0.5;
        let top_px = viewport_position.y + marker_screen_size * 0.5 + WAYPOINT_LABEL_GAP_SCREEN_PX;
        node.left = Val::Px(left_px);
        node.top = Val::Px(top_px);
        let alpha = state.opacity.clamp(0.0, 1.0);
        for child in children.iter() {
            if let Ok((mut text_color, is_label_text, is_label_shadow)) =
                label_colors.get_mut(child)
            {
                if is_label_text {
                    text_color.0 = WAYPOINT_LABEL_COLOR.with_alpha(alpha);
                } else if is_label_shadow {
                    text_color.0 = WAYPOINT_LABEL_SHADOW_COLOR.with_alpha(alpha);
                }
            }
        }
        *visibility = Visibility::Visible;
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

fn spawn_waypoint_entities(
    commands: &mut Commands,
    layer_id: LayerId,
    features: &ParsedWaypointFeatures,
    icon: Handle<Image>,
    font: Handle<Font>,
    ui_root: Option<Entity>,
) -> Vec<Entity> {
    let mut entities = Vec::with_capacity(features.points.len() * 2 + features.connections.len());

    for point in &features.points {
        let name = point
            .label
            .clone()
            .or_else(|| point.feature_id.clone())
            .unwrap_or_else(|| "Waypoint".to_string());
        let node_type_suffix = point
            .node_type_label
            .clone()
            .or_else(|| point.node_type.clone());
        let entity_name = node_type_suffix
            .map(|node_type_label| format!("Waypoint {name} ({node_type_label})"))
            .unwrap_or_else(|| format!("Waypoint {name}"));
        let marker = commands
            .spawn((
                Name::new(entity_name),
                WaypointLayerMarker {
                    layer_id,
                    world_x: point.world_x,
                    world_z: point.world_z,
                    default_visible: point.default_visible,
                },
                World2dRenderEntity,
                world_2d_layers(),
                Sprite {
                    image: icon.clone(),
                    custom_size: Some(Vec2::splat(WAYPOINT_MARKER_SIZE_SCREEN_PX)),
                    ..default()
                },
                Transform::from_xyz(point.world_x, point.world_z, 0.0),
                Visibility::Hidden,
            ))
            .id();
        entities.push(marker);

        if let (Some(ui_root), Some(map_label)) = (
            ui_root,
            point
                .map_label
                .as_deref()
                .map(str::trim)
                .filter(|label| !label.is_empty()),
        ) {
            let label_root = commands
                .spawn((
                    Name::new(format!("Waypoint Label {map_label}")),
                    WaypointLayerLabel {
                        layer_id,
                        world_x: point.world_x,
                        world_z: point.world_z,
                        default_visible: point.default_visible,
                    },
                    WaypointLayerLabelRoot,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    GlobalZIndex(WAYPOINT_LABEL_Z_INDEX),
                    Visibility::Hidden,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        WaypointLayerLabelShadowText,
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(WAYPOINT_LABEL_SHADOW_OFFSET_X_SCREEN_PX),
                            top: Val::Px(WAYPOINT_LABEL_SHADOW_OFFSET_Y_SCREEN_PX),
                            ..default()
                        },
                        Text::new(map_label.to_string()),
                        TextFont {
                            font: font.clone(),
                            font_size: WAYPOINT_LABEL_FONT_SIZE_SCREEN_PX,
                            ..default()
                        },
                        TextLayout::new_with_no_wrap().with_justify(Justify::Center),
                        TextColor(WAYPOINT_LABEL_SHADOW_COLOR),
                    ));
                    parent.spawn((
                        WaypointLayerLabelText,
                        Text::new(map_label.to_string()),
                        TextFont {
                            font: font.clone(),
                            font_size: WAYPOINT_LABEL_FONT_SIZE_SCREEN_PX,
                            ..default()
                        },
                        TextLayout::new_with_no_wrap().with_justify(Justify::Center),
                        TextColor(WAYPOINT_LABEL_COLOR),
                    ));
                })
                .id();
            commands.entity(ui_root).add_child(label_root);
            entities.push(label_root);
        }
    }

    for connection in &features.connections {
        let dx = connection.end_world_x - connection.start_world_x;
        let dz = connection.end_world_z - connection.start_world_z;
        let length_world = dx.hypot(dz);
        if length_world <= f32::EPSILON {
            continue;
        }
        let entity = commands
            .spawn((
                Name::new("Waypoint Connection"),
                WaypointLayerConnection {
                    layer_id,
                    center_x: (connection.start_world_x + connection.end_world_x) * 0.5,
                    center_z: (connection.start_world_z + connection.end_world_z) * 0.5,
                    length_world,
                    angle_radians: dz.atan2(dx),
                    default_visible: connection.default_visible,
                },
                World2dRenderEntity,
                world_2d_layers(),
                Sprite::from_color(
                    WAYPOINT_CONNECTION_COLOR,
                    Vec2::new(length_world, WAYPOINT_CONNECTION_THICKNESS_SCREEN_PX),
                ),
                Transform::from_xyz(0.0, 0.0, 0.0),
                Visibility::Hidden,
            ))
            .id();
        entities.push(entity);
    }

    entities
}

fn parse_waypoint_features(
    source: &WaypointSourceSpec,
    collection: WaypointFeatureCollection,
) -> ParsedWaypointFeatures {
    let mut parsed = ParsedWaypointFeatures::default();
    for feature in collection.features {
        let WaypointFeature {
            properties,
            geometry,
        } = feature;
        let Some(geometry) = geometry else {
            continue;
        };
        match geometry.geometry_type.as_str() {
            "Point" => {
                if let Some(point) =
                    waypoint_point_from_feature(source, properties, geometry.coordinates)
                {
                    parsed.points.push(point);
                }
            }
            "LineString" => {
                if let Some(connection) = waypoint_connection_from_feature(
                    source.geometry_space,
                    properties,
                    geometry.coordinates,
                ) {
                    parsed.connections.push(connection);
                }
            }
            _ => {}
        }
    }
    parsed
}

fn waypoint_point_from_feature(
    source: &WaypointSourceSpec,
    properties: Map<String, Value>,
    coordinates: Value,
) -> Option<WaypointPointRecord> {
    let coordinates = coordinates.as_array()?;
    if coordinates.len() < 2 {
        return None;
    }
    let x = coordinates.first()?.as_f64()?;
    let y = coordinates.get(1)?.as_f64()?;
    let (world_x, world_z) = waypoint_world_position(source.geometry_space, x, y);
    let label = source
        .label_property
        .as_ref()
        .and_then(|key| string_property(&properties, key));
    let map_label = source
        .name_property
        .as_ref()
        .and_then(|key| string_property(&properties, key))
        .or_else(|| label.clone());
    let feature_id = source
        .feature_id_property
        .as_ref()
        .and_then(|key| string_property(&properties, key));
    let node_type = string_property(&properties, "node_type");
    let node_type_label = string_property(&properties, "node_type_label");
    let default_visible = bool_property(&properties, "default_visible").unwrap_or(true);
    Some(WaypointPointRecord {
        world_x,
        world_z,
        label,
        map_label,
        feature_id,
        node_type,
        node_type_label,
        default_visible,
    })
}

fn waypoint_connection_from_feature(
    geometry_space: GeometrySpace,
    properties: Map<String, Value>,
    coordinates: Value,
) -> Option<WaypointConnectionRecord> {
    let coordinates = coordinates.as_array()?;
    if coordinates.len() < 2 {
        return None;
    }
    let start = coordinates.first()?.as_array()?;
    let end = coordinates.last()?.as_array()?;
    let start_x = start.first()?.as_f64()?;
    let start_y = start.get(1)?.as_f64()?;
    let end_x = end.first()?.as_f64()?;
    let end_y = end.get(1)?.as_f64()?;
    let (start_world_x, start_world_z) = waypoint_world_position(geometry_space, start_x, start_y);
    let (end_world_x, end_world_z) = waypoint_world_position(geometry_space, end_x, end_y);
    let default_visible = bool_property(&properties, "default_visible").unwrap_or(true);
    Some(WaypointConnectionRecord {
        start_world_x,
        start_world_z,
        end_world_x,
        end_world_z,
        default_visible,
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

fn bool_property(properties: &Map<String, Value>, key: &str) -> Option<bool> {
    properties.get(key).and_then(Value::as_bool)
}

fn world_to_viewport(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    world_position: Vec3,
) -> Option<Vec2> {
    camera
        .world_to_viewport(camera_transform, world_position)
        .ok()
}

#[cfg(test)]
mod tests {
    use super::{parse_waypoint_features, GeometrySpace, WaypointFeatureCollection};
    use crate::map::layers::WaypointSourceSpec;

    #[test]
    fn parse_waypoint_features_reads_world_space_points_and_connections() {
        let collection: WaypointFeatureCollection = serde_json::from_str(
            r#"{
                "features": [
                    {
                        "properties": {
                            "r": 204,
                            "label": "Stonebeak Shore (R204)",
                            "name": "Stonebeak Shore",
                            "node_type": "connection",
                            "node_type_label": "Connection",
                            "default_visible": false
                        },
                        "geometry": {"type": "Point", "coordinates": [303191.0, -1694.35]}
                    },
                    {
                        "properties": {"from_r": 204, "to_r": 205, "default_visible": false},
                        "geometry": {"type": "LineString", "coordinates": [[303191.0, -1694.35], [301000.0, 1200.0]]}
                    }
                ]
            }"#,
        )
        .expect("parse");
        let features = parse_waypoint_features(
            &WaypointSourceSpec {
                url: "/waypoints/region_nodes.v1.geojson".to_string(),
                revision: "region-nodes-v1".to_string(),
                geometry_space: GeometrySpace::World,
                feature_id_property: Some("r".to_string()),
                label_property: Some("label".to_string()),
                name_property: Some("name".to_string()),
                supports_connections: true,
                supports_labels: true,
                show_connections_default: true,
                show_labels_default: true,
            },
            collection,
        );

        assert_eq!(features.points.len(), 1);
        assert_eq!(features.connections.len(), 1);
        assert_eq!(features.points[0].feature_id.as_deref(), Some("204"));
        assert_eq!(
            features.points[0].label.as_deref(),
            Some("Stonebeak Shore (R204)")
        );
        assert_eq!(
            features.points[0].map_label.as_deref(),
            Some("Stonebeak Shore")
        );
        assert_eq!(features.points[0].node_type.as_deref(), Some("connection"));
        assert_eq!(
            features.points[0].node_type_label.as_deref(),
            Some("Connection")
        );
        assert!(!features.points[0].default_visible);
        assert!((features.points[0].world_x - 303191.0).abs() < f32::EPSILON);
        assert!((features.points[0].world_z + 1694.35).abs() < 0.01);
        assert!(!features.connections[0].default_visible);
        assert!((features.connections[0].start_world_x - 303191.0).abs() < f32::EPSILON);
        assert!((features.connections[0].end_world_z - 1200.0).abs() < 0.01);
    }
}
