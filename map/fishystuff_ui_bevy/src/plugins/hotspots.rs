use std::collections::{HashMap, HashSet};

use async_channel::{Receiver, TryRecvError};
use bevy::color::Alpha;
use bevy::prelude::*;
use fishystuff_api::Rgb;
use fishystuff_core::field_metadata::{FieldDetailFact, FieldDetailSection, FieldHoverTarget};
use serde::Deserialize;
use serde_json::Value;

use crate::bridge::contract::{FishyMapSearchExpressionNode, FishyMapSharedFishState};
use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::layer_query::LayerQuerySample;
use crate::map::layers::{
    LayerId, LayerManifestStatus, LayerRegistry, LayerRuntime, HOTSPOTS_LAYER_KEY,
};
use crate::map::search_filters::{
    fish_id_matches_search_expression, project_expression_for_fish_selection,
};
use crate::map::spaces::WorldPoint;
use crate::plugins::api::{
    fish_item_icon_url, remote_image_handle, FishCatalog, HoverState, MapDisplayState,
    RemoteImageCache, RemoteImageStatus, SearchExpressionState,
};
use crate::plugins::camera::Map2dCamera;
use crate::plugins::render_domain::{world_2d_layers, World2dRenderEntity};
use crate::plugins::waypoint_layers::{WaypointLayerInteractionSample, WaypointSampleOptions};
use crate::runtime_io;

pub(crate) const HOTSPOT_TARGET_KEY: &str = "hotspot";

const HOTSPOT_FILL_Z_OFFSET: f32 = 0.03;
const HOTSPOT_BORDER_Z_OFFSET: f32 = 0.04;
const HOTSPOT_ICON_Z_OFFSET: f32 = 0.08;
const HOTSPOT_FILL_COLOR: Color = Color::srgba(1.0, 0.70, 0.22, 0.20);
const HOTSPOT_BORDER_COLOR: Color = Color::srgba(1.0, 0.70, 0.22, 0.90);
const HOTSPOT_BORDER_THICKNESS_SCREEN_PX: f32 = 2.0;
const HOTSPOT_ICON_SIZE_SCREEN_PX: f32 = 18.0;
const HOTSPOT_SAMPLE_RGB: Rgb = Rgb::new(255, 179, 56);

pub struct HotspotLayersPlugin;

impl Plugin for HotspotLayersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HotspotLayerRuntime>()
            .add_systems(Update, (update_hotspot_layers, sync_hotspot_entities));
    }
}

#[derive(Resource, Default)]
pub(crate) struct HotspotLayerRuntime {
    states: HashMap<LayerId, HotspotLayerState>,
}

#[derive(Default)]
struct HotspotLayerState {
    source_key: Option<String>,
    pending: Option<Receiver<Result<HotspotAsset, String>>>,
    hotspots: Vec<HotspotRecord>,
    entities: Vec<Entity>,
}

#[derive(Component, Clone, Copy)]
struct HotspotLayerFeature {
    layer_id: LayerId,
    hotspot_id: u32,
    min_x: f32,
    min_z: f32,
    max_x: f32,
    max_z: f32,
    center_x: f32,
    center_z: f32,
    primary_fish_item_id: Option<u32>,
}

#[derive(Component)]
struct HotspotLayerFill;

#[derive(Component)]
struct HotspotLayerBorder {
    edge: HotspotBorderEdge,
}

#[derive(Component)]
struct HotspotLayerIcon;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum HotspotBorderEdge {
    Top,
    Right,
    Bottom,
    Left,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct HotspotAsset {
    schema: String,
    version: u32,
    hotspots: Vec<HotspotRecord>,
}

impl Default for HotspotAsset {
    fn default() -> Self {
        Self {
            schema: String::new(),
            version: 0,
            hotspots: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct HotspotRecord {
    id: u32,
    point_size: f64,
    start_x: f64,
    start_y: f64,
    start_z: f64,
    end_x: f64,
    end_z: f64,
    min_x: f64,
    min_z: f64,
    max_x: f64,
    max_z: f64,
    center_x: f64,
    center_z: f64,
    primary_fish_item_id: Option<u32>,
    primary_fish_name: Option<String>,
    loot_items: Vec<Value>,
    loot_groups: Vec<Value>,
    fishing_group_key: u32,
    spawn_rate: Option<u32>,
    spawn_character_key: Option<u32>,
    spawn_action_index: Option<u32>,
    point_contents_group_key: Option<u32>,
    fishing_contents_group_key: Option<u32>,
    drop_groups: Vec<HotspotDropGroup>,
    min_wait_time: Option<u32>,
    max_wait_time: Option<u32>,
    point_remain_time: Option<u32>,
    min_fish_count: Option<u32>,
    max_fish_count: Option<u32>,
    available_fishing_level: Option<u32>,
    observe_fishing_level: Option<u32>,
    source_stats: HotspotSourceStats,
    imported_metadata: Option<HotspotImportedMetadata>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct HotspotDropGroup {
    slot: u8,
    drop_rate: Option<u32>,
    group_key: u32,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct HotspotSourceStats {
    min_wait_time: u32,
    max_wait_time: u32,
    point_remain_time: u32,
    min_fish_count: u32,
    max_fish_count: u32,
    available_fishing_level: u32,
    observe_fishing_level: u32,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct HotspotImportedMetadata {
    source_id: String,
    source_hotspot_id: u32,
    min_wait_time: Option<u32>,
    max_wait_time: Option<u32>,
    point_remain_time: Option<u32>,
    min_fish_count: Option<u32>,
    max_fish_count: Option<u32>,
    available_fishing_level: Option<u32>,
    observe_fishing_level: Option<u32>,
}

fn update_hotspot_layers(
    mut commands: Commands,
    registry: Res<LayerRegistry>,
    mut layer_runtime: ResMut<LayerRuntime>,
    mut runtime: ResMut<HotspotLayerRuntime>,
) {
    layer_runtime.sync_to_registry(&registry);
    prune_stale_hotspot_layers(&registry, &mut runtime, &mut commands);

    for layer in registry.ordered() {
        if layer.key != HOTSPOTS_LAYER_KEY {
            continue;
        }
        let Some(runtime_state) = layer_runtime.get_mut(layer.id) else {
            continue;
        };
        let Some(source) = layer.waypoint_source.as_ref() else {
            runtime_state.manifest_status = LayerManifestStatus::Missing;
            continue;
        };

        let request_url = resolve_hotspot_source_url(&source.url);
        let state = runtime.states.entry(layer.id).or_default();
        let source_key = format!("{}#{}", request_url, source.revision);
        if state.source_key.as_deref() != Some(source_key.as_str()) {
            state.source_key = Some(source_key);
            state.pending = Some(runtime_io::spawn_json_request(request_url));
            state.hotspots.clear();
            clear_hotspot_layer_entities(&mut state.entities, &mut commands);
            runtime_state.manifest_status = LayerManifestStatus::Loading;
            continue;
        }

        if let Some(receiver) = state.pending.as_ref() {
            match receiver.try_recv() {
                Ok(result) => {
                    state.pending = None;
                    clear_hotspot_layer_entities(&mut state.entities, &mut commands);
                    match result.and_then(validate_hotspot_asset) {
                        Ok(mut asset) => {
                            asset.hotspots.sort_by_key(|hotspot| hotspot.id);
                            state.hotspots = asset.hotspots;
                            runtime_state.manifest_status = LayerManifestStatus::Ready;
                        }
                        Err(err) => {
                            bevy::log::warn!(
                                "failed to load hotspot layer {} from {}: {}",
                                layer.key,
                                resolve_hotspot_source_url(&source.url),
                                err
                            );
                            state.hotspots.clear();
                            runtime_state.manifest_status = LayerManifestStatus::Failed;
                        }
                    }
                }
                Err(TryRecvError::Closed) => {
                    state.pending = None;
                    state.hotspots.clear();
                    clear_hotspot_layer_entities(&mut state.entities, &mut commands);
                    runtime_state.manifest_status = LayerManifestStatus::Failed;
                }
                Err(TryRecvError::Empty) => {
                    runtime_state.manifest_status = LayerManifestStatus::Loading;
                }
            }
        }

        if runtime_state.manifest_status == LayerManifestStatus::Ready
            && state.entities.is_empty()
            && !state.hotspots.is_empty()
        {
            state.entities = spawn_hotspot_entities(&mut commands, layer.id, &state.hotspots);
        }
    }
}

fn sync_hotspot_entities(
    display_state: Res<MapDisplayState>,
    view_mode: Res<ViewModeState>,
    hover_state: Res<HoverState>,
    layer_runtime: Res<LayerRuntime>,
    fish_catalog: Res<FishCatalog>,
    search_expression: Res<SearchExpressionState>,
    mut remote_images: ResMut<RemoteImageCache>,
    camera_q: Query<&Projection, With<Map2dCamera>>,
    mut queries: ParamSet<(
        Query<
            (
                &HotspotLayerFeature,
                &mut Transform,
                &mut Visibility,
                &mut Sprite,
            ),
            With<HotspotLayerFill>,
        >,
        Query<(
            &HotspotLayerFeature,
            &HotspotLayerBorder,
            &mut Transform,
            &mut Visibility,
            &mut Sprite,
        )>,
        Query<
            (
                &HotspotLayerFeature,
                &mut Transform,
                &mut Visibility,
                &mut Sprite,
            ),
            With<HotspotLayerIcon>,
        >,
    )>,
) {
    let ui_visible = view_mode.mode == ViewMode::Map2D;
    let camera_scale = camera_world_scale(&camera_q);
    let border_thickness = HOTSPOT_BORDER_THICKNESS_SCREEN_PX * camera_scale;
    let icon_size = hotspot_icon_world_size(&display_state, camera_scale);
    let hovered_hotspot_id = hovered_hotspot_id(&hover_state);
    let projected_search_expression =
        project_expression_for_fish_selection(&search_expression.expression);

    for (feature, mut transform, mut visibility, mut sprite) in &mut queries.p0() {
        let Some(state) = layer_runtime.get(feature.layer_id) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let width = feature.width();
        let height = feature.height();
        let hovered = hovered_hotspot_id == Some(feature.hotspot_id);
        if !ui_visible
            || !state.visible
            || width <= f32::EPSILON
            || height <= f32::EPSILON
            || !hotspot_feature_matches_search_expression(
                feature,
                projected_search_expression.as_ref(),
                &fish_catalog,
                &search_expression.shared_fish_state,
            )
        {
            *visibility = Visibility::Hidden;
            continue;
        }
        transform.translation = Vec3::new(
            feature.center_x,
            feature.center_z,
            state.z_base + HOTSPOT_FILL_Z_OFFSET,
        );
        sprite.custom_size = Some(Vec2::new(width, height));
        sprite.color = HOTSPOT_FILL_COLOR
            .with_alpha(HOTSPOT_FILL_COLOR.alpha() * state.opacity.clamp(0.0, 1.0));
        *visibility = if hovered {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    for (feature, border, mut transform, mut visibility, mut sprite) in &mut queries.p1() {
        let Some(state) = layer_runtime.get(feature.layer_id) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let width = feature.width();
        let height = feature.height();
        if !ui_visible
            || !state.visible
            || width <= f32::EPSILON
            || height <= f32::EPSILON
            || !hotspot_feature_matches_search_expression(
                feature,
                projected_search_expression.as_ref(),
                &fish_catalog,
                &search_expression.shared_fish_state,
            )
        {
            *visibility = Visibility::Hidden;
            continue;
        }
        let (translation, size) = border_transform_and_size(feature, border.edge, border_thickness);
        transform.translation = Vec3::new(
            translation.x,
            translation.y,
            state.z_base + HOTSPOT_BORDER_Z_OFFSET,
        );
        sprite.custom_size = Some(size);
        sprite.color = HOTSPOT_BORDER_COLOR
            .with_alpha(HOTSPOT_BORDER_COLOR.alpha() * state.opacity.clamp(0.0, 1.0));
        *visibility = Visibility::Visible;
    }

    for (feature, mut transform, mut visibility, mut sprite) in &mut queries.p2() {
        let Some(state) = layer_runtime.get(feature.layer_id) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        if !ui_visible
            || !state.visible
            || !hotspot_feature_matches_search_expression(
                feature,
                projected_search_expression.as_ref(),
                &fish_catalog,
                &search_expression.shared_fish_state,
            )
        {
            *visibility = Visibility::Hidden;
            continue;
        }
        let Some(item_id) = feature.primary_fish_item_id else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let Some(url) = fish_item_icon_url(item_id as i32) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let RemoteImageStatus::Ready(handle) = remote_image_handle(&url, &mut remote_images) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        sprite.image = handle;
        sprite.custom_size = Some(Vec2::splat(icon_size));
        sprite.color = Color::WHITE.with_alpha(state.opacity.clamp(0.0, 1.0));
        transform.translation = Vec3::new(
            feature.center_x,
            feature.center_z,
            state.z_base + HOTSPOT_ICON_Z_OFFSET,
        );
        *visibility = Visibility::Visible;
    }
}

fn validate_hotspot_asset(asset: HotspotAsset) -> Result<HotspotAsset, String> {
    if asset.schema != "fishystuff.hotspots" {
        return Err(format!("unexpected schema `{}`", asset.schema));
    }
    if asset.version != 1 {
        return Err(format!(
            "unsupported hotspot asset version {}",
            asset.version
        ));
    }
    Ok(asset)
}

fn prune_stale_hotspot_layers(
    registry: &LayerRegistry,
    runtime: &mut HotspotLayerRuntime,
    commands: &mut Commands,
) {
    let valid_ids: HashSet<LayerId> = registry
        .ordered()
        .iter()
        .filter(|layer| layer.key == HOTSPOTS_LAYER_KEY)
        .map(|layer| layer.id)
        .collect();
    runtime.states.retain(|layer_id, state| {
        if valid_ids.contains(layer_id) {
            true
        } else {
            clear_hotspot_layer_entities(&mut state.entities, commands);
            false
        }
    });
}

fn clear_hotspot_layer_entities(entities: &mut Vec<Entity>, commands: &mut Commands) {
    for entity in entities.drain(..) {
        commands.entity(entity).despawn();
    }
}

fn spawn_hotspot_entities(
    commands: &mut Commands,
    layer_id: LayerId,
    hotspots: &[HotspotRecord],
) -> Vec<Entity> {
    let mut entities = Vec::with_capacity(hotspots.len().saturating_mul(6));
    for hotspot in hotspots {
        let width = (hotspot.max_x - hotspot.min_x).abs() as f32;
        let height = (hotspot.max_z - hotspot.min_z).abs() as f32;
        if width <= f32::EPSILON || height <= f32::EPSILON {
            continue;
        }
        let feature = HotspotLayerFeature {
            layer_id,
            hotspot_id: hotspot.id,
            min_x: hotspot.min_x as f32,
            min_z: hotspot.min_z as f32,
            max_x: hotspot.max_x as f32,
            max_z: hotspot.max_z as f32,
            center_x: hotspot.center_x as f32,
            center_z: hotspot.center_z as f32,
            primary_fish_item_id: hotspot.primary_fish_item_id,
        };
        let fill = commands
            .spawn((
                Name::new(format!("Hotspot #{}", hotspot.id)),
                feature,
                HotspotLayerFill,
                World2dRenderEntity,
                world_2d_layers(),
                Sprite::from_color(HOTSPOT_FILL_COLOR, Vec2::new(width, height)),
                Transform::from_xyz(hotspot.center_x as f32, hotspot.center_z as f32, 0.0),
                Visibility::Hidden,
            ))
            .id();
        entities.push(fill);
        for edge in [
            HotspotBorderEdge::Top,
            HotspotBorderEdge::Right,
            HotspotBorderEdge::Bottom,
            HotspotBorderEdge::Left,
        ] {
            let border = commands
                .spawn((
                    Name::new(format!("Hotspot #{} Border", hotspot.id)),
                    feature,
                    HotspotLayerBorder { edge },
                    World2dRenderEntity,
                    world_2d_layers(),
                    Sprite::from_color(HOTSPOT_BORDER_COLOR, Vec2::ZERO),
                    Transform::default(),
                    Visibility::Hidden,
                ))
                .id();
            entities.push(border);
        }
        let icon = commands
            .spawn((
                Name::new(format!("Hotspot #{} Fish Icon", hotspot.id)),
                feature,
                HotspotLayerIcon,
                World2dRenderEntity,
                world_2d_layers(),
                Sprite::default(),
                Transform::from_xyz(hotspot.center_x as f32, hotspot.center_z as f32, 0.0),
                Visibility::Hidden,
            ))
            .id();
        entities.push(icon);
    }
    entities
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn hotspot_sample_at_world_point_with_options(
    world_point: WorldPoint,
    hotspot_runtime: &HotspotLayerRuntime,
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
    fish_catalog: &FishCatalog,
    search_expression: &SearchExpressionState,
    options: WaypointSampleOptions,
) -> Option<WaypointLayerInteractionSample> {
    hotspot_samples_at_world_point_with_options(
        world_point,
        hotspot_runtime,
        layer_registry,
        layer_runtime,
        fish_catalog,
        search_expression,
        options,
    )
    .into_iter()
    .next()
}

pub(crate) fn hotspot_samples_at_world_point_with_options(
    world_point: WorldPoint,
    hotspot_runtime: &HotspotLayerRuntime,
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
    fish_catalog: &FishCatalog,
    search_expression: &SearchExpressionState,
    options: WaypointSampleOptions,
) -> Vec<WaypointLayerInteractionSample> {
    if options
        .target_key
        .is_some_and(|target_key| target_key != HOTSPOT_TARGET_KEY)
    {
        return Vec::new();
    }
    let mut hits = Vec::new();
    let projected_search_expression =
        project_expression_for_fish_selection(&search_expression.expression);
    for layer in layer_registry.ordered() {
        if layer.key != HOTSPOTS_LAYER_KEY {
            continue;
        }
        if !options.include_hidden_layers && !layer_runtime.visible(layer.id) {
            continue;
        }
        let Some(layer_state) = hotspot_runtime.states.get(&layer.id) else {
            continue;
        };
        if layer_state.pending.is_some() {
            continue;
        }
        for hotspot in &layer_state.hotspots {
            if !hotspot_matches_search_expression(
                hotspot,
                projected_search_expression.as_ref(),
                fish_catalog,
                &search_expression.shared_fish_state,
            ) {
                continue;
            }
            if !hotspot_contains_world_point(hotspot, world_point) {
                continue;
            }
            hits.push(HotspotHit {
                area: hotspot_area(hotspot),
                center_distance_sq: hotspot_center_distance_sq(hotspot, world_point),
                display_order: layer_runtime
                    .get(layer.id)
                    .map(|state| state.display_order)
                    .unwrap_or(layer.display_order),
                sample: hotspot_interaction_sample(layer, hotspot),
            });
        }
    }
    hits.sort_by(|left, right| {
        right
            .display_order
            .cmp(&left.display_order)
            .then_with(|| left.area.total_cmp(&right.area))
            .then_with(|| left.center_distance_sq.total_cmp(&right.center_distance_sq))
            .then_with(|| {
                left.sample
                    .point_label
                    .as_deref()
                    .unwrap_or("")
                    .cmp(right.sample.point_label.as_deref().unwrap_or(""))
            })
    });
    hits.into_iter().map(|hit| hit.sample).collect()
}

pub(crate) fn hotspot_layers_pending(
    hotspot_runtime: &HotspotLayerRuntime,
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
    options: WaypointSampleOptions,
) -> bool {
    if options
        .target_key
        .is_some_and(|target_key| target_key != HOTSPOT_TARGET_KEY)
    {
        return false;
    }
    layer_registry.ordered().iter().any(|layer| {
        layer.key == HOTSPOTS_LAYER_KEY
            && (options.include_hidden_layers || layer_runtime.visible(layer.id))
            && hotspot_runtime
                .states
                .get(&layer.id)
                .and_then(|state| state.pending.as_ref())
                .is_some()
    })
}

struct HotspotHit {
    area: f64,
    center_distance_sq: f64,
    display_order: i32,
    sample: WaypointLayerInteractionSample,
}

impl HotspotLayerFeature {
    fn width(self) -> f32 {
        (self.max_x - self.min_x).abs()
    }

    fn height(self) -> f32 {
        (self.max_z - self.min_z).abs()
    }
}

fn hotspot_feature_matches_search_expression(
    feature: &HotspotLayerFeature,
    projected_expression: Option<&FishyMapSearchExpressionNode>,
    fish_catalog: &FishCatalog,
    shared_fish_state: &FishyMapSharedFishState,
) -> bool {
    fish_item_id_matches_search_expression(
        feature.primary_fish_item_id,
        projected_expression,
        fish_catalog,
        shared_fish_state,
    )
}

fn hotspot_matches_search_expression(
    hotspot: &HotspotRecord,
    projected_expression: Option<&FishyMapSearchExpressionNode>,
    fish_catalog: &FishCatalog,
    shared_fish_state: &FishyMapSharedFishState,
) -> bool {
    fish_item_id_matches_search_expression(
        hotspot.primary_fish_item_id,
        projected_expression,
        fish_catalog,
        shared_fish_state,
    )
}

fn fish_item_id_matches_search_expression(
    fish_item_id: Option<u32>,
    projected_expression: Option<&FishyMapSearchExpressionNode>,
    fish_catalog: &FishCatalog,
    shared_fish_state: &FishyMapSharedFishState,
) -> bool {
    let Some(expression) = projected_expression else {
        return true;
    };
    let Some(fish_id) = fish_item_id.and_then(|item_id| i32::try_from(item_id).ok()) else {
        return false;
    };
    fish_id_matches_search_expression(fish_catalog, fish_id, expression, shared_fish_state)
}

fn camera_world_scale(camera_q: &Query<&Projection, With<Map2dCamera>>) -> f32 {
    camera_q
        .single()
        .ok()
        .and_then(|projection| match projection {
            Projection::Orthographic(ortho) => Some(ortho.scale),
            _ => None,
        })
        .unwrap_or(1.0)
        .max(f32::EPSILON)
}

fn hotspot_icon_world_size(display_state: &MapDisplayState, camera_scale: f32) -> f32 {
    HOTSPOT_ICON_SIZE_SCREEN_PX
        * camera_scale
        * display_state.point_icon_scale.clamp(
            crate::plugins::api::POINT_ICON_SCALE_MIN,
            crate::plugins::api::POINT_ICON_SCALE_MAX,
        )
}

fn hovered_hotspot_id(hover_state: &HoverState) -> Option<u32> {
    hover_state
        .info
        .as_ref()?
        .layer_samples
        .iter()
        .find(|sample| sample.layer_id == HOTSPOTS_LAYER_KEY && sample.kind == HOTSPOT_TARGET_KEY)
        .and_then(|sample| sample.field_id)
}

fn border_transform_and_size(
    feature: &HotspotLayerFeature,
    edge: HotspotBorderEdge,
    thickness: f32,
) -> (Vec2, Vec2) {
    let thickness = thickness.max(f32::EPSILON);
    let width = feature.width().max(thickness);
    let height = feature.height().max(thickness);
    match edge {
        HotspotBorderEdge::Top => (
            Vec2::new(feature.center_x, feature.max_z),
            Vec2::new(width + thickness, thickness),
        ),
        HotspotBorderEdge::Right => (
            Vec2::new(feature.max_x, feature.center_z),
            Vec2::new(thickness, height + thickness),
        ),
        HotspotBorderEdge::Bottom => (
            Vec2::new(feature.center_x, feature.min_z),
            Vec2::new(width + thickness, thickness),
        ),
        HotspotBorderEdge::Left => (
            Vec2::new(feature.min_x, feature.center_z),
            Vec2::new(thickness, height + thickness),
        ),
    }
}

fn hotspot_contains_world_point(hotspot: &HotspotRecord, world_point: WorldPoint) -> bool {
    world_point.x >= hotspot.min_x
        && world_point.x <= hotspot.max_x
        && world_point.z >= hotspot.min_z
        && world_point.z <= hotspot.max_z
}

fn hotspot_area(hotspot: &HotspotRecord) -> f64 {
    (hotspot.max_x - hotspot.min_x).abs() * (hotspot.max_z - hotspot.min_z).abs()
}

fn hotspot_center_distance_sq(hotspot: &HotspotRecord, world_point: WorldPoint) -> f64 {
    let dx = world_point.x - hotspot.center_x;
    let dz = world_point.z - hotspot.center_z;
    dx * dx + dz * dz
}

fn hotspot_interaction_sample(
    layer: &crate::map::layers::LayerSpec,
    hotspot: &HotspotRecord,
) -> WaypointLayerInteractionSample {
    let label = hotspot_label(hotspot);
    WaypointLayerInteractionSample {
        world_x: hotspot.center_x,
        world_z: hotspot.center_z,
        point_label: Some(label.clone()),
        layer_sample: LayerQuerySample {
            layer_id: layer.key.clone(),
            layer_name: layer.name.clone(),
            kind: HOTSPOT_TARGET_KEY.to_string(),
            rgb: HOTSPOT_SAMPLE_RGB,
            rgb_u32: HOTSPOT_SAMPLE_RGB.to_u32(),
            field_id: Some(hotspot.id),
            targets: vec![FieldHoverTarget {
                key: HOTSPOT_TARGET_KEY.to_string(),
                label,
                world_x: hotspot.center_x,
                world_z: hotspot.center_z,
            }],
            detail_pane: None,
            detail_sections: hotspot_detail_sections(hotspot),
        },
    }
}

fn hotspot_label(hotspot: &HotspotRecord) -> String {
    hotspot
        .primary_fish_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| format!("{name} Hotspot #{}", hotspot.id))
        .unwrap_or_else(|| format!("Hotspot #{}", hotspot.id))
}

fn hotspot_detail_sections(hotspot: &HotspotRecord) -> Vec<FieldDetailSection> {
    let mut hotspot_facts = vec![
        detail_fact("hotspot_id", "Hotspot", hotspot.id.to_string(), "map-pin"),
        detail_fact(
            HOTSPOT_TARGET_KEY,
            "Hotspot",
            format!("#{}", hotspot.id),
            "map-pin",
        ),
    ];
    if let Some(primary_fish_name) = hotspot
        .primary_fish_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        hotspot_facts.push(detail_fact(
            "primary_fish",
            "Fish",
            primary_fish_name,
            "fish-fill",
        ));
    }
    if let Some(primary_fish_item_id) = hotspot.primary_fish_item_id {
        hotspot_facts.push(detail_fact(
            "primary_fish_item_id",
            "Fish Item",
            primary_fish_item_id.to_string(),
            "fish-fill",
        ));
    }
    let imported_metadata = hotspot.imported_metadata.as_ref();
    if let Some(metadata) = imported_metadata {
        hotspot_facts.push(detail_fact(
            "metadata_source",
            "Metadata Source",
            metadata_source_label(metadata.source_id.as_str()),
            "information-circle",
        ));
        hotspot_facts.push(detail_fact(
            "metadata_source_hotspot_id",
            "Metadata Source Hotspot",
            metadata.source_hotspot_id.to_string(),
            "map-pin",
        ));
        hotspot_facts.push(detail_fact(
            "source_metadata_stats",
            "Source Table Metadata",
            source_stats_summary(&hotspot.source_stats),
            "source-database",
        ));
    }
    push_optional_detail_fact(
        &mut hotspot_facts,
        "min_fish_count",
        "Min. Catches",
        imported_metadata
            .and_then(|metadata| metadata.min_fish_count)
            .or(hotspot.min_fish_count),
        "information-circle",
    );
    push_optional_detail_fact(
        &mut hotspot_facts,
        "max_fish_count",
        "Max. Catches",
        imported_metadata
            .and_then(|metadata| metadata.max_fish_count)
            .or(hotspot.max_fish_count),
        "information-circle",
    );
    push_optional_detail_fact(
        &mut hotspot_facts,
        "available_fishing_level",
        "Catchable at",
        imported_metadata
            .and_then(|metadata| metadata.available_fishing_level)
            .or(hotspot.available_fishing_level),
        "information-circle",
    );
    push_optional_detail_fact(
        &mut hotspot_facts,
        "observe_fishing_level",
        "Visible at",
        imported_metadata
            .and_then(|metadata| metadata.observe_fishing_level)
            .or(hotspot.observe_fishing_level),
        "information-circle",
    );
    push_optional_detail_fact(
        &mut hotspot_facts,
        "min_wait_time_ms",
        "Bite Time Minimum",
        imported_metadata
            .and_then(|metadata| metadata.min_wait_time)
            .or(hotspot.min_wait_time),
        "stopwatch",
    );
    push_optional_detail_fact(
        &mut hotspot_facts,
        "max_wait_time_ms",
        "Bite Time Maximum",
        imported_metadata
            .and_then(|metadata| metadata.max_wait_time)
            .or(hotspot.max_wait_time),
        "stopwatch",
    );
    push_optional_detail_fact(
        &mut hotspot_facts,
        "point_remain_time_ms",
        "Lifetime",
        imported_metadata
            .and_then(|metadata| metadata.point_remain_time)
            .or(hotspot.point_remain_time),
        "time-fill",
    );
    hotspot_facts.push(detail_fact(
        "fishing_group_key",
        "Fishing Group",
        hotspot.fishing_group_key.to_string(),
        "information-circle",
    ));
    if !hotspot.drop_groups.is_empty() {
        hotspot_facts.push(detail_fact(
            "drop_groups",
            "Drop Groups",
            hotspot_drop_group_summary(hotspot),
            "information-circle",
        ));
    }
    if let Some(contents_group_key) = hotspot
        .point_contents_group_key
        .or(hotspot.fishing_contents_group_key)
    {
        hotspot_facts.push(detail_fact(
            "contents_group_key",
            "Contents Group",
            contents_group_key.to_string(),
            "information-circle",
        ));
    }
    if let Some(spawn_character_key) = hotspot.spawn_character_key {
        let spawn_value = hotspot
            .spawn_action_index
            .map(|action| format!("{spawn_character_key} / action {action}"))
            .unwrap_or_else(|| spawn_character_key.to_string());
        hotspot_facts.push(detail_fact(
            "spawn_character",
            "Spawn Character",
            spawn_value,
            "information-circle",
        ));
    }
    hotspot_facts.push(detail_fact(
        "point_size",
        "Point Size",
        format_source_number(hotspot.point_size),
        "information-circle",
    ));
    hotspot_facts.push(detail_fact(
        "bounds",
        "Bounds",
        format!(
            "x {}..{}, z {}..{}",
            format_source_number(hotspot.min_x),
            format_source_number(hotspot.max_x),
            format_source_number(hotspot.min_z),
            format_source_number(hotspot.max_z),
        ),
        "information-circle",
    ));
    if !hotspot.loot_groups.is_empty() {
        for loot_group in &hotspot.loot_groups {
            if let Ok(value) = serde_json::to_string(loot_group) {
                hotspot_facts.push(detail_fact("loot_group", "Loot Group", value, "fish-fill"));
            }
        }
    } else {
        for loot_item in &hotspot.loot_items {
            if let Ok(value) = serde_json::to_string(loot_item) {
                hotspot_facts.push(detail_fact("loot_item", "Loot", value, "fish-fill"));
            }
        }
    }

    Vec::from([FieldDetailSection {
        id: "hotspot".to_string(),
        kind: "hotspot".to_string(),
        title: Some("Hotspot".to_string()),
        facts: hotspot_facts,
        targets: Vec::new(),
    }])
}

fn push_optional_detail_fact(
    facts: &mut Vec<FieldDetailFact>,
    key: &'static str,
    label: &'static str,
    value: Option<u32>,
    icon: &'static str,
) {
    if let Some(value) = value {
        facts.push(detail_fact(key, label, value.to_string(), icon));
    }
}

fn metadata_source_label(source_id: &str) -> &'static str {
    match source_id {
        "bdolytics_community_hotspot_metadata" => "bdolytics community snapshot",
        _ => "imported hotspot metadata",
    }
}

fn source_stats_summary(stats: &HotspotSourceStats) -> String {
    if stats.min_wait_time == 0
        && stats.max_wait_time == 0
        && stats.point_remain_time == 0
        && stats.min_fish_count == 0
        && stats.max_fish_count == 0
        && stats.available_fishing_level == 0
        && stats.observe_fishing_level == 0
    {
        return "FloatFishing_Table stat columns are 0".to_string();
    }
    format!(
        "FloatFishing_Table stats: minWait={} maxWait={} lifetime={} minCount={} maxCount={} availableLevel={} visibleLevel={}",
        stats.min_wait_time,
        stats.max_wait_time,
        stats.point_remain_time,
        stats.min_fish_count,
        stats.max_fish_count,
        stats.available_fishing_level,
        stats.observe_fishing_level,
    )
}

fn detail_fact(
    key: impl Into<String>,
    label: impl Into<String>,
    value: impl Into<String>,
    icon: impl Into<String>,
) -> FieldDetailFact {
    FieldDetailFact {
        key: key.into(),
        label: label.into(),
        value: value.into(),
        icon: Some(icon.into()),
        status_icon: None,
        status_icon_tone: None,
    }
}

fn hotspot_drop_group_summary(hotspot: &HotspotRecord) -> String {
    hotspot
        .drop_groups
        .iter()
        .map(|group| {
            group
                .drop_rate
                .map(|rate| format!("{} ({rate})", group.group_key))
                .unwrap_or_else(|| group.group_key.to_string())
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_source_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

fn resolve_hotspot_source_url(url: &str) -> String {
    if is_api_path(url) {
        #[cfg(target_arch = "wasm32")]
        {
            return crate::plugins::api::resolve_api_request_url(url);
        }
    }
    url.to_string()
}

fn is_api_path(url: &str) -> bool {
    let value = url.trim_start();
    value.starts_with("/api/")
}

#[cfg(test)]
mod tests {
    use super::{
        border_transform_and_size, hotspot_contains_world_point, hotspot_detail_sections,
        hotspot_label, hotspot_matches_search_expression, hovered_hotspot_id, HotspotBorderEdge,
        HotspotDropGroup, HotspotImportedMetadata, HotspotLayerFeature, HotspotRecord,
        HOTSPOT_TARGET_KEY,
    };
    use crate::bridge::contract::{
        FishyMapSearchExpressionNode, FishyMapSearchExpressionOperator, FishyMapSearchTerm,
        FishyMapSharedFishState,
    };
    use crate::map::layer_query::LayerQuerySample;
    use crate::map::layers::HOTSPOTS_LAYER_KEY;
    use crate::map::spaces::WorldPoint;
    use crate::plugins::api::{FishCatalog, FishEntry, HoverInfo, HoverState};
    use fishystuff_api::Rgb;
    use serde_json::json;

    fn hotspot() -> HotspotRecord {
        HotspotRecord {
            id: 2,
            point_size: 2000.0,
            start_x: -73592.0,
            start_y: -8208.0,
            start_z: 253493.0,
            end_x: -33080.0,
            end_z: 198722.0,
            min_x: -73592.0,
            min_z: 198722.0,
            max_x: -33080.0,
            max_z: 253493.0,
            center_x: -53336.0,
            center_z: 226107.5,
            primary_fish_item_id: Some(8452),
            primary_fish_name: Some("Coelacanth".to_string()),
            loot_items: vec![json!({
                "itemId": 8452,
                "name": "Coelacanth",
                "label": "Coelacanth",
                "selectRate": 1_000_000,
                "gradeType": 3,
                "iconItemId": 8452,
                "iconImage": "New_Icon/03_ETC/07_ProductMaterial/00008452",
            })],
            loot_groups: vec![json!({
                "slotIdx": 2,
                "label": "Group 1",
                "conditionOptionKey": "hotspot:2:2:10944",
                "conditionOptions": [
                    {
                        "conditionKey": "getLifeLevel(1)>80;",
                        "conditionText": "Fishing Level Guru 1+",
                        "conditionTooltip": "getLifeLevel(1)>80;",
                        "active": true,
                        "speciesRows": [
                            {
                                "itemId": 8452,
                                "name": "Coelacanth",
                                "label": "Coelacanth",
                                "selectRate": 999_950,
                                "gradeType": 3,
                                "iconItemId": 8452,
                                "iconImage": "New_Icon/03_ETC/07_ProductMaterial/00008452"
                            }
                        ]
                    }
                ]
            })],
            fishing_group_key: 2,
            min_fish_count: Some(1),
            max_fish_count: Some(1),
            spawn_character_key: Some(917),
            spawn_action_index: Some(4),
            drop_groups: vec![HotspotDropGroup {
                slot: 2,
                drop_rate: Some(1_000_000),
                group_key: 10944,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn hotspot_hit_test_uses_source_bounds() {
        let hotspot = hotspot();
        assert!(hotspot_contains_world_point(
            &hotspot,
            WorldPoint::new(-53336.0, 226107.5)
        ));
        assert!(!hotspot_contains_world_point(
            &hotspot,
            WorldPoint::new(-80000.0, 226107.5)
        ));
    }

    #[test]
    fn hotspot_detail_sections_use_source_group_metadata() {
        let sections = hotspot_detail_sections(&hotspot());
        assert_eq!(sections.len(), 1);
        let facts = &sections[0].facts;
        assert!(facts
            .iter()
            .any(|fact| fact.key == "drop_groups" && fact.value == "10944 (1000000)"));
        assert!(facts
            .iter()
            .any(|fact| fact.key == "primary_fish" && fact.value == "Coelacanth"));
        assert!(facts
            .iter()
            .any(|fact| fact.key == "primary_fish_item_id" && fact.value == "8452"));
        assert!(facts
            .iter()
            .any(|fact| fact.key == "min_fish_count" && fact.value == "1"));
        assert!(facts.iter().any(|fact| fact.key == "loot_group"
            && fact
                .value
                .contains("\"conditionText\":\"Fishing Level Guru 1+\"")
            && fact.value.contains("\"itemId\":8452")));
        assert!(facts
            .iter()
            .any(|fact| fact.key == "spawn_character" && fact.value == "917 / action 4"));
    }

    #[test]
    fn hotspot_detail_sections_use_imported_metadata_with_provenance() {
        let mut hotspot = hotspot();
        hotspot.min_fish_count = None;
        hotspot.max_fish_count = None;
        hotspot.imported_metadata = Some(HotspotImportedMetadata {
            source_id: "bdolytics_community_hotspot_metadata".to_string(),
            source_hotspot_id: 2,
            min_wait_time: Some(79_496),
            max_wait_time: Some(109_496),
            point_remain_time: Some(600_000),
            min_fish_count: Some(2),
            max_fish_count: Some(4),
            available_fishing_level: Some(1),
            observe_fishing_level: Some(1),
        });

        let sections = hotspot_detail_sections(&hotspot);
        let facts = &sections[0].facts;
        assert!(facts
            .iter()
            .any(|fact| fact.key == "metadata_source"
                && fact.value == "bdolytics community snapshot"));
        assert!(facts.iter().any(|fact| fact.key == "source_metadata_stats"
            && fact.value == "FloatFishing_Table stat columns are 0"));
        assert!(facts
            .iter()
            .any(|fact| fact.key == "min_fish_count" && fact.value == "2"));
        assert!(facts
            .iter()
            .any(|fact| fact.key == "max_wait_time_ms" && fact.value == "109496"));
    }

    #[test]
    fn hotspot_label_uses_fish_identity_when_available() {
        assert_eq!(hotspot_label(&hotspot()), "Coelacanth Hotspot #2");
    }

    #[test]
    fn hover_fill_uses_first_hotspot_sample_id() {
        let hover_state = HoverState {
            info: Some(HoverInfo {
                map_px: 1,
                map_py: 1,
                world_x: 10.0,
                world_z: 20.0,
                layer_samples: vec![hotspot_layer_sample(7), hotspot_layer_sample(8)],
                point_samples: Vec::new(),
            }),
        };
        assert_eq!(hovered_hotspot_id(&hover_state), Some(7));
    }

    #[test]
    fn border_segments_follow_source_bounds() {
        let feature = HotspotLayerFeature {
            layer_id: crate::map::layers::LayerId::from_raw(7),
            hotspot_id: 2,
            min_x: -100.0,
            min_z: 25.0,
            max_x: 300.0,
            max_z: 225.0,
            center_x: 100.0,
            center_z: 125.0,
            primary_fish_item_id: Some(8452),
        };
        let (translation, size) = border_transform_and_size(&feature, HotspotBorderEdge::Top, 4.0);
        assert_eq!(translation, bevy::prelude::Vec2::new(100.0, 225.0));
        assert_eq!(size, bevy::prelude::Vec2::new(404.0, 4.0));

        let (translation, size) = border_transform_and_size(&feature, HotspotBorderEdge::Left, 4.0);
        assert_eq!(translation, bevy::prelude::Vec2::new(-100.0, 125.0));
        assert_eq!(size, bevy::prelude::Vec2::new(4.0, 204.0));
    }

    #[test]
    fn hotspot_search_filter_matches_primary_fish_item() {
        let mut fish_catalog = FishCatalog::default();
        fish_catalog.replace(vec![FishEntry {
            id: 452,
            item_id: 8452,
            encyclopedia_key: Some(452),
            encyclopedia_id: Some(8452),
            name: "Coelacanth".to_string(),
            name_lower: "coelacanth".to_string(),
            grade: Some("Rare".to_string()),
            is_prize: false,
        }]);
        let expression = FishyMapSearchExpressionNode::Group {
            operator: FishyMapSearchExpressionOperator::And,
            children: vec![
                FishyMapSearchExpressionNode::Term {
                    term: FishyMapSearchTerm::Fish { fish_id: 452 },
                    negated: false,
                },
                FishyMapSearchExpressionNode::Term {
                    term: FishyMapSearchTerm::FishFilter {
                        term: "yellow".to_string(),
                    },
                    negated: false,
                },
            ],
            negated: false,
        };

        assert!(hotspot_matches_search_expression(
            &hotspot(),
            Some(&expression),
            &fish_catalog,
            &FishyMapSharedFishState::default(),
        ));
        assert!(!hotspot_matches_search_expression(
            &hotspot(),
            Some(&FishyMapSearchExpressionNode::Term {
                term: FishyMapSearchTerm::Fish { fish_id: 777 },
                negated: false,
            }),
            &fish_catalog,
            &FishyMapSharedFishState::default(),
        ));
    }

    fn hotspot_layer_sample(id: u32) -> LayerQuerySample {
        LayerQuerySample {
            layer_id: HOTSPOTS_LAYER_KEY.to_string(),
            layer_name: "Hotspots".to_string(),
            kind: HOTSPOT_TARGET_KEY.to_string(),
            rgb: Rgb::new(255, 179, 56),
            rgb_u32: Rgb::new(255, 179, 56).to_u32(),
            field_id: Some(id),
            targets: Vec::new(),
            detail_pane: None,
            detail_sections: Vec::new(),
        }
    }
}
