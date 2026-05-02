use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::color::Alpha;
use bevy::ecs::system::SystemParam;
use bevy::image::Image;
use bevy::input::touch::Touches;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::bridge::contract::{FishyMapBookmarkEntry, FishyMapHoverLayerSampleSnapshot};
use crate::config::DRAG_THRESHOLD;
use crate::map::camera::map2d::Map2dViewState;
use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::exact_lookup::ExactLookupCache;
use crate::map::field_metadata::FieldMetadataCache;
use crate::map::layer_query::{sample_semantic_layers_at_world_point, LayerQuerySample};
use crate::map::layers::{LayerRegistry, LayerRuntime};
use crate::map::raster::{cache::clip_mask_allows_world_point, RasterTileCache};
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::WorldPoint;
use crate::plugins::api::{
    ApiBootstrapState, HoverState, LayerEffectiveFilterState, ZoneMembershipFilter,
};
use crate::plugins::camera::Map2dCamera;
use crate::plugins::input::PanState;
use crate::plugins::render_domain::{world_2d_layers, World2dRenderEntity};
use crate::plugins::vector_layers::VectorLayerRuntime;
use fishystuff_core::field_metadata::{preferred_detail_fact_value, FieldDetailSection};

const BOOKMARK_MARKER_SIZE_SCREEN_PX: f32 = 22.0;
const BOOKMARK_MARKER_Z: f32 = 40.4;
const BOOKMARK_TEXTURE_WIDTH_PX: usize = 32;
const BOOKMARK_TEXTURE_HEIGHT_PX: usize = 32;
const BOOKMARK_RING_RADIUS_PX: f32 = 12.0;
const BOOKMARK_RING_THICKNESS_PX: f32 = 4.0;
const BOOKMARK_CORE_RADIUS_PX: f32 = 5.0;
const BOOKMARK_COLOR: [u8; 3] = [239, 92, 31];
const BOOKMARK_CORE_COLOR: [u8; 3] = [255, 242, 214];
const EDGE_FEATHER_PX: f32 = 1.2;

pub struct BookmarksPlugin;

impl Plugin for BookmarksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BookmarkState>()
            .init_resource::<BookmarkRenderAssets>()
            .init_resource::<BookmarkMarkerPool>()
            .init_resource::<BookmarkDetailProgress>()
            .add_systems(
                Update,
                (
                    ensure_bookmark_render_assets,
                    enrich_next_bookmark_details,
                    sync_bookmark_markers,
                )
                    .chain(),
            );
    }
}

#[derive(Resource, Default)]
pub struct BookmarkState {
    pub entries: Vec<FishyMapBookmarkEntry>,
    pub selected_ids: Vec<String>,
}

#[derive(Component)]
struct BookmarkMarker;

#[derive(Clone, Copy, Debug)]
struct BookmarkVisualSet {
    marker: Entity,
}

#[derive(Resource, Default)]
struct BookmarkRenderAssets {
    marker_texture: Option<Handle<Image>>,
}

#[derive(Resource, Default)]
struct BookmarkMarkerPool {
    markers: Vec<BookmarkVisualSet>,
}

#[derive(Resource, Default)]
struct BookmarkDetailProgress {
    cursor: usize,
}

#[derive(SystemParam)]
struct BookmarkRenderContext<'w, 's> {
    layer_registry: Res<'w, LayerRegistry>,
    layer_runtime: Res<'w, LayerRuntime>,
    exact_lookups: Res<'w, ExactLookupCache>,
    tile_cache: Res<'w, RasterTileCache>,
    vector_runtime: Res<'w, VectorLayerRuntime>,
    layer_filters: Res<'w, LayerEffectiveFilterState>,
    render_assets: Res<'w, BookmarkRenderAssets>,
    _marker: std::marker::PhantomData<&'s ()>,
}

#[derive(SystemParam)]
struct BookmarkDetailContext<'w> {
    mouse_buttons: Res<'w, ButtonInput<MouseButton>>,
    touches: Res<'w, Touches>,
    pan: Res<'w, PanState>,
    hover: Res<'w, HoverState>,
    map_view: Res<'w, Map2dViewState>,
    bootstrap: Res<'w, ApiBootstrapState>,
    layer_registry: Res<'w, LayerRegistry>,
    layer_runtime: Res<'w, LayerRuntime>,
    exact_lookups: Res<'w, ExactLookupCache>,
    field_metadata: Res<'w, FieldMetadataCache>,
}

fn ensure_bookmark_render_assets(
    mut render_assets: ResMut<BookmarkRenderAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    if render_assets.marker_texture.is_none() {
        render_assets.marker_texture = Some(images.add(build_bookmark_marker_texture()));
    }
}

fn enrich_next_bookmark_details(
    mut bookmarks: ResMut<BookmarkState>,
    mut progress: ResMut<BookmarkDetailProgress>,
    context: BookmarkDetailContext<'_>,
) {
    if bookmark_details_should_yield(&context) || bookmarks.entries.is_empty() {
        return;
    }

    if progress.cursor >= bookmarks.entries.len() {
        progress.cursor = 0;
    }

    for offset in 0..bookmarks.entries.len() {
        let index = (progress.cursor + offset) % bookmarks.entries.len();
        if !bookmark_needs_runtime_details(&bookmarks.entries[index], &context) {
            continue;
        }
        let mut next = bookmarks.entries[index].clone();
        enrich_bookmark_runtime_details(&mut next, &context);
        progress.cursor = (index + 1) % bookmarks.entries.len();
        if next != bookmarks.entries[index] {
            bookmarks.entries[index] = next;
        }
        return;
    }
}

fn bookmark_details_should_yield(context: &BookmarkDetailContext<'_>) -> bool {
    context.mouse_buttons.pressed(MouseButton::Left)
        || context.mouse_buttons.just_released(MouseButton::Left)
        || context.touches.iter().next().is_some()
        || context.touches.any_just_released()
        || context.pan.drag_distance > DRAG_THRESHOLD
        || context.hover.is_changed()
        || context.map_view.is_changed()
}

fn bookmark_needs_runtime_details(
    bookmark: &FishyMapBookmarkEntry,
    context: &BookmarkDetailContext<'_>,
) -> bool {
    if bookmark.layer_samples.is_empty() {
        return true;
    }
    bookmark.point_label
        != preferred_bookmark_point_label(
            &bookmark.layer_samples,
            &context.layer_registry,
            &context.layer_runtime,
            Some(&context.bootstrap.zones),
        )
}

fn enrich_bookmark_runtime_details(
    bookmark: &mut FishyMapBookmarkEntry,
    context: &BookmarkDetailContext<'_>,
) {
    if bookmark.layer_samples.is_empty() {
        let layer_samples = sample_semantic_layers_at_world_point(
            &context.layer_registry,
            &context.exact_lookups,
            &context.field_metadata,
            WorldPoint::new(bookmark.world_x, bookmark.world_z),
            MapToWorld::default(),
        );
        bookmark.layer_samples = hover_layer_samples_snapshot(&layer_samples);
        if bookmark.zone_rgb.is_none() {
            bookmark.zone_rgb = zone_rgb_from_bookmark_samples(&bookmark.layer_samples);
        }
    }
    bookmark.point_label = preferred_bookmark_point_label(
        &bookmark.layer_samples,
        &context.layer_registry,
        &context.layer_runtime,
        Some(&context.bootstrap.zones),
    );
}

fn hover_layer_samples_snapshot(
    samples: &[LayerQuerySample],
) -> Vec<FishyMapHoverLayerSampleSnapshot> {
    samples
        .iter()
        .map(|sample| FishyMapHoverLayerSampleSnapshot {
            layer_id: sample.layer_id.clone(),
            layer_name: sample.layer_name.clone(),
            kind: sample.kind.clone(),
            rgb: sample.rgb.as_array(),
            rgb_u32: sample.rgb_u32,
            field_id: sample.field_id,
            targets: sample.targets.clone(),
            detail_pane: sample.detail_pane.clone(),
            detail_sections: sample.detail_sections.clone(),
        })
        .collect()
}

fn zone_rgb_from_bookmark_samples(samples: &[FishyMapHoverLayerSampleSnapshot]) -> Option<u32> {
    samples
        .iter()
        .find(|sample| sample.layer_id == "zone_mask")
        .map(|sample| sample.rgb_u32)
}

fn normalized_bookmark_label(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn preferred_bookmark_sample_point_label(
    sample: &FishyMapHoverLayerSampleSnapshot,
    zone_names: Option<&HashMap<u32, Option<String>>>,
) -> Option<String> {
    if let Some(value) = preferred_detail_fact_value(detail_sections(sample)) {
        return normalized_bookmark_label(Some(value));
    }
    if sample.layer_id == "zone_mask" {
        if let Some(value) = zone_names
            .and_then(|zones| zones.get(&sample.rgb_u32))
            .and_then(|value| value.as_deref())
        {
            return normalized_bookmark_label(Some(value));
        }
    }
    sample
        .targets
        .iter()
        .find_map(|target| normalized_bookmark_label(Some(target.label.as_str())))
}

fn preferred_bookmark_point_label(
    layer_samples: &[FishyMapHoverLayerSampleSnapshot],
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
    zone_names: Option<&HashMap<u32, Option<String>>>,
) -> Option<String> {
    let mut ordered_samples: Vec<(usize, &FishyMapHoverLayerSampleSnapshot)> = layer_samples
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            let layer_order = layer_registry
                .ordered()
                .iter()
                .find(|layer| layer.key == sample.layer_id)
                .map(|layer| layer_runtime.display_order(layer.id))
                .map(|order| order as usize)
                .unwrap_or(1000 + index);
            (layer_order, sample)
        })
        .collect();
    ordered_samples.sort_by_key(|(order, _sample)| *order);
    ordered_samples
        .into_iter()
        .find_map(|(_order, sample)| preferred_bookmark_sample_point_label(sample, zone_names))
}

fn sync_bookmark_markers(
    mut commands: Commands,
    bookmarks: Res<BookmarkState>,
    view_mode: Res<ViewModeState>,
    render_context: BookmarkRenderContext<'_, '_>,
    mut marker_pool: ResMut<BookmarkMarkerPool>,
    camera_q: Query<&Projection, With<Map2dCamera>>,
    mut markers: Query<(&mut Transform, &mut Visibility, &mut Sprite), With<BookmarkMarker>>,
) {
    let bookmark_layer = render_context.layer_registry.get_by_key("bookmarks");
    let bookmark_layer_id = bookmark_layer.map(|layer| layer.id);
    let bookmark_layer_state =
        bookmark_layer.and_then(|layer| render_context.layer_runtime.get(layer.id));
    let bookmark_layer_visible = bookmark_layer_state
        .map(|state| state.visible)
        .unwrap_or(true);
    let bookmark_layer_opacity = bookmark_layer_state
        .map(|state| state.opacity.clamp(0.0, 1.0))
        .unwrap_or(1.0);
    let bookmark_layer_z = bookmark_layer_state
        .map(|state| state.z_base)
        .unwrap_or(BOOKMARK_MARKER_Z);

    if view_mode.mode != ViewMode::Map2D || bookmarks.entries.is_empty() || !bookmark_layer_visible
    {
        hide_bookmark_visuals(&marker_pool, &mut markers);
        return;
    }

    let Some(marker_texture) = render_context.render_assets.marker_texture.as_ref() else {
        return;
    };
    let Ok(projection) = camera_q.single() else {
        hide_bookmark_visuals(&marker_pool, &mut markers);
        return;
    };
    let current_scale = match projection {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => 1.0,
    }
    .max(f32::EPSILON);

    let marker_size_world = BOOKMARK_MARKER_SIZE_SCREEN_PX * current_scale;

    while marker_pool.markers.len() < bookmarks.entries.len() {
        let marker = commands
            .spawn((
                BookmarkMarker,
                World2dRenderEntity,
                world_2d_layers(),
                Sprite {
                    image: marker_texture.clone(),
                    custom_size: Some(Vec2::splat(marker_size_world)),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, BOOKMARK_MARKER_Z),
                Visibility::Hidden,
            ))
            .id();
        marker_pool.markers.push(BookmarkVisualSet { marker });
    }

    for (index, bookmark) in bookmarks.entries.iter().enumerate() {
        let visual = marker_pool.markers[index];
        let world_x = bookmark.world_x as f32;
        let world_z = bookmark.world_z as f32;
        let world_point = WorldPoint::new(world_x as f64, world_z as f64);
        let bookmark_visible_here = bookmark_layer_id.is_none_or(|layer_id| {
            bookmark_visible_in_layer_clip(layer_id, world_point, &render_context)
        });

        if let Ok((mut transform, mut visibility, mut sprite)) = markers.get_mut(visual.marker) {
            transform.translation = Vec3::new(world_x, world_z, bookmark_layer_z);
            sprite.image = marker_texture.clone();
            sprite.custom_size = Some(Vec2::splat(marker_size_world));
            sprite.color = Color::WHITE.with_alpha(bookmark_layer_opacity);
            *visibility = if bookmark_visible_here {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }

    for visual in marker_pool.markers.iter().skip(bookmarks.entries.len()) {
        if let Ok((_, mut visibility, _)) = markers.get_mut(visual.marker) {
            *visibility = Visibility::Hidden;
        }
    }
}

fn hide_bookmark_visuals(
    marker_pool: &BookmarkMarkerPool,
    markers: &mut Query<(&mut Transform, &mut Visibility, &mut Sprite), With<BookmarkMarker>>,
) {
    for visual in &marker_pool.markers {
        if let Ok((_, mut visibility, _)) = markers.get_mut(visual.marker) {
            *visibility = Visibility::Hidden;
        }
    }
}

fn bookmark_visible_in_layer_clip(
    layer_id: crate::map::layers::LayerId,
    world_point: WorldPoint,
    render_context: &BookmarkRenderContext<'_, '_>,
) -> bool {
    let inactive_filter = ZoneMembershipFilter::default();
    let zone_filter = render_context
        .layer_registry
        .get(layer_id)
        .and_then(|layer| {
            render_context
                .layer_filters
                .zone_membership_filter(layer.key.as_str())
        })
        .unwrap_or(&inactive_filter);
    !matches!(
        clip_mask_allows_world_point(
            layer_id,
            world_point,
            &render_context.layer_registry,
            &render_context.layer_runtime,
            &render_context.exact_lookups,
            &render_context.tile_cache,
            &render_context.vector_runtime,
            zone_filter,
            render_context.layer_registry.map_version_id(),
        ),
        Some(false)
    )
}

fn detail_sections(
    sample: &crate::bridge::contract::FishyMapHoverLayerSampleSnapshot,
) -> impl Iterator<Item = &fishystuff_core::field_metadata::FieldDetailFact> {
    sample
        .detail_sections
        .iter()
        .flat_map(|section: &FieldDetailSection| section.facts.iter())
}

fn build_bookmark_marker_texture() -> Image {
    let width = BOOKMARK_TEXTURE_WIDTH_PX;
    let height = BOOKMARK_TEXTURE_HEIGHT_PX;
    let center_x = (width as f32 - 1.0) * 0.5;
    let center_y = (height as f32 - 1.0) * 0.5;

    let mut texture_data = vec![0_u8; width * height * 4];
    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let distance = (dx * dx + dy * dy).sqrt();

            let ring_alpha = ring_alpha(distance);
            let core_alpha = circle_alpha(distance, BOOKMARK_CORE_RADIUS_PX);
            let alpha = ring_alpha.max(core_alpha);
            if alpha <= 0.0 {
                continue;
            }

            let color = if core_alpha > 0.0 {
                BOOKMARK_CORE_COLOR
            } else {
                BOOKMARK_COLOR
            };
            let offset = (y * width + x) * 4;
            texture_data[offset] = color[0];
            texture_data[offset + 1] = color[1];
            texture_data[offset + 2] = color[2];
            texture_data[offset + 3] = (alpha * 255.0).round() as u8;
        }
    }

    Image::new_fill(
        Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

fn ring_alpha(distance: f32) -> f32 {
    let half_thickness = BOOKMARK_RING_THICKNESS_PX * 0.5;
    let edge_distance = (distance - BOOKMARK_RING_RADIUS_PX).abs();
    if edge_distance <= half_thickness {
        return 1.0;
    }
    if edge_distance <= half_thickness + EDGE_FEATHER_PX {
        return 1.0 - (edge_distance - half_thickness) / EDGE_FEATHER_PX;
    }
    0.0
}

fn circle_alpha(distance: f32, radius: f32) -> f32 {
    if distance <= radius {
        return 1.0;
    }
    if distance <= radius + EDGE_FEATHER_PX {
        return 1.0 - (distance - radius) / EDGE_FEATHER_PX;
    }
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::{IntoSystem, System};

    #[test]
    fn sync_bookmark_markers_initializes_without_query_access_conflicts() {
        let mut world = World::new();
        let mut system = IntoSystem::into_system(sync_bookmark_markers);
        system.initialize(&mut world);
    }
}
