use bevy::asset::RenderAssetUsages;
use bevy::color::Alpha;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::text::{Justify, TextLayout};
use bevy::window::PrimaryWindow;

use crate::bridge::contract::FishyMapThemeColors;
use crate::bridge::theme::parse_css_color;
use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::layers::{LayerRegistry, LayerRuntime};
use crate::plugins::api::{HoverInfo, HoverState};
use crate::plugins::camera::Map2dCamera;
use crate::plugins::render_domain::{world_2d_layers, World2dRenderEntity};
use crate::plugins::ui::UiFonts;

#[cfg(target_arch = "wasm32")]
use crate::bridge::host::BrowserBridgeState;

const HOVER_MARKER_Z: f32 = 40.6;
const HOVER_LABEL_SIZE_PX: f32 = 12.0;
const HOVER_LABEL_COLOR: Color = Color::srgb(0.98, 0.97, 0.94);
const HOVER_CALLOUT_MIN_WIDTH_SCREEN_PX: f32 = 96.0;
const HOVER_CALLOUT_HEIGHT_SCREEN_PX: f32 = 28.0;
const HOVER_CALLOUT_PADDING_X_SCREEN_PX: f32 = 12.0;
const HOVER_CALLOUT_BORDER_SCREEN_PX: f32 = 2.0;
const HOVER_CALLOUT_CORNER_RADIUS_SCREEN_PX: f32 = 10.0;
const HOVER_TEXT_WIDTH_FACTOR: f32 = 0.72;
const HOVER_TEXT_WIDTH_SLACK_SCREEN_PX: f32 = 10.0;
const HOVER_CALLOUT_BORDER_COLOR: Color = Color::srgba(0.74, 0.78, 0.86, 0.96);
const HOVER_CALLOUT_PANEL_COLOR: Color = Color::srgba(0.07, 0.09, 0.12, 0.95);

const HOVER_TEXTURE_WIDTH_PX: usize = 32;
const HOVER_TEXTURE_HEIGHT_PX: usize = 32;
const HOVER_RING_RADIUS_PX: f32 = 12.0;
const HOVER_RING_THICKNESS_PX: f32 = 3.0;
const HOVER_CORE_RADIUS_PX: f32 = 4.5;
const EDGE_FEATHER_PX: f32 = 1.2;

const RESOURCE_BAR_MARKER_SIZE_SCREEN_PX: f32 = 28.0;
const ORIGIN_NODE_MARKER_SIZE_SCREEN_PX: f32 = 20.0;
const RESOURCE_BAR_LABEL_OFFSET_SCREEN_PX: f32 = 20.0;
const ORIGIN_NODE_LABEL_OFFSET_SCREEN_PX: f32 = 36.0;
const VIEW_EDGE_PADDING_SCREEN_PX: f32 = 12.0;

const RESOURCE_BAR_MARKER_COLOR: [u8; 3] = [77, 211, 255];
const ORIGIN_NODE_MARKER_COLOR: [u8; 3] = [255, 196, 66];

pub struct HoverTargetsPlugin;

impl Plugin for HoverTargetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HoverTargetMarkerAssets>()
            .init_resource::<HoverTargetMarkerPool>()
            .add_systems(
                Update,
                (ensure_hover_target_marker_assets, sync_hover_targets),
            );
    }
}

#[derive(Component)]
struct HoverTargetMarker;

#[derive(Component)]
struct HoverTargetLabelRoot;

#[derive(Component)]
struct HoverTargetLabelText;

#[derive(Clone, Copy, Debug)]
struct HoverTargetVisualPair {
    marker: Entity,
    label_root: Entity,
    label_text: Entity,
}

#[derive(Resource, Default)]
struct HoverTargetMarkerAssets {
    texture: Option<Handle<Image>>,
}

#[derive(Resource, Default)]
struct HoverTargetMarkerPool {
    markers: Vec<HoverTargetVisualPair>,
}

#[derive(Debug, Clone, PartialEq)]
struct HoverTargetVisual {
    world_x: f32,
    world_z: f32,
    label: String,
    marker_size_screen_px: f32,
    label_offset_screen_px: f32,
    color_rgb: [u8; 3],
}

#[derive(Debug, Clone, Copy)]
struct Map2dViewportBounds {
    min_x: f32,
    max_x: f32,
    min_z: f32,
    max_z: f32,
    scale: f32,
}

fn ensure_hover_target_marker_assets(
    mut marker_assets: ResMut<HoverTargetMarkerAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    if marker_assets.texture.is_some() {
        return;
    }
    marker_assets.texture = Some(images.add(build_hover_marker_texture()));
}

fn sync_hover_targets(
    mut commands: Commands,
    hover: Res<HoverState>,
    view_mode: Res<ViewModeState>,
    #[cfg(target_arch = "wasm32")] bridge: Res<BrowserBridgeState>,
    fonts: Res<UiFonts>,
    layer_registry: Res<LayerRegistry>,
    layer_runtime: Res<LayerRuntime>,
    marker_assets: Res<HoverTargetMarkerAssets>,
    mut marker_pool: ResMut<HoverTargetMarkerPool>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<
        (&Camera, &GlobalTransform, &Projection),
        (
            With<Map2dCamera>,
            Without<HoverTargetMarker>,
            Without<HoverTargetLabelRoot>,
            Without<HoverTargetLabelText>,
        ),
    >,
    mut markers: Query<
        (&mut Transform, &mut Visibility, &mut Sprite),
        (
            With<HoverTargetMarker>,
            Without<HoverTargetLabelRoot>,
            Without<HoverTargetLabelText>,
        ),
    >,
    mut label_roots: Query<
        (
            &mut Node,
            &mut Visibility,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (
            With<HoverTargetLabelRoot>,
            Without<HoverTargetMarker>,
            Without<HoverTargetLabelText>,
        ),
    >,
    mut label_texts: Query<
        (&mut Text, &mut TextFont, &mut TextColor),
        (
            With<HoverTargetLabelText>,
            Without<HoverTargetMarker>,
            Without<HoverTargetLabelRoot>,
        ),
    >,
) {
    let targets = if view_mode.mode == ViewMode::Map2D {
        hover_targets_from_info(hover.info.as_ref(), &layer_registry, &layer_runtime)
    } else {
        Vec::new()
    };

    if targets.is_empty() {
        hide_hover_targets(&marker_pool, &mut markers, &mut label_roots);
        return;
    }

    let Some(texture) = marker_assets.texture.as_ref() else {
        return;
    };
    let Ok((camera, camera_transform, _)) = camera_q.single() else {
        hide_hover_targets(&marker_pool, &mut markers, &mut label_roots);
        return;
    };
    #[cfg(target_arch = "wasm32")]
    let theme_colors = Some(&bridge.input.theme.colors);
    #[cfg(not(target_arch = "wasm32"))]
    let theme_colors: Option<&FishyMapThemeColors> = None;
    let label_color = theme_colors
        .and_then(hover_target_label_color)
        .unwrap_or(HOVER_LABEL_COLOR);
    let callout_border_color = theme_colors
        .and_then(hover_target_border_color)
        .unwrap_or(HOVER_CALLOUT_BORDER_COLOR);
    let callout_panel_color = theme_colors
        .and_then(hover_target_panel_color)
        .unwrap_or(HOVER_CALLOUT_PANEL_COLOR);

    while marker_pool.markers.len() < targets.len() {
        let marker = commands
            .spawn((
                HoverTargetMarker,
                World2dRenderEntity,
                world_2d_layers(),
                Sprite {
                    image: texture.clone(),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, HOVER_MARKER_Z),
                Visibility::Hidden,
            ))
            .id();
        let mut label_text = Entity::PLACEHOLDER;
        let label_root = commands
            .spawn((
                HoverTargetLabelRoot,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(HOVER_CALLOUT_MIN_WIDTH_SCREEN_PX),
                    height: Val::Px(HOVER_CALLOUT_HEIGHT_SCREEN_PX),
                    padding: UiRect::axes(Val::Px(HOVER_CALLOUT_PADDING_X_SCREEN_PX), Val::Px(0.0)),
                    border: UiRect::all(Val::Px(HOVER_CALLOUT_BORDER_SCREEN_PX)),
                    border_radius: BorderRadius::all(Val::Px(
                        HOVER_CALLOUT_CORNER_RADIUS_SCREEN_PX,
                    )),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(callout_panel_color),
                BorderColor::all(callout_border_color),
                Visibility::Hidden,
            ))
            .with_children(|parent| {
                label_text = parent
                    .spawn((
                        HoverTargetLabelText,
                        Text::new(""),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: HOVER_LABEL_SIZE_PX,
                            ..default()
                        },
                        TextLayout::new_with_no_wrap().with_justify(Justify::Center),
                        TextColor(label_color),
                    ))
                    .id();
            })
            .id();
        marker_pool.markers.push(HoverTargetVisualPair {
            marker,
            label_root,
            label_text,
        });
    }

    let viewport_bounds = map_2d_viewport_bounds(&windows, &camera_q);
    let scale = viewport_bounds
        .map(|bounds| bounds.scale)
        .unwrap_or_else(|| camera_scale(&camera_q));
    for (index, target) in targets.iter().enumerate() {
        let target = viewport_bounds
            .map(|bounds| clamp_hover_target_to_viewport(target, bounds))
            .unwrap_or_else(|| target.clone());
        let pair = marker_pool.markers[index];
        if let Ok((mut transform, mut visibility, mut sprite)) = markers.get_mut(pair.marker) {
            transform.translation.x = target.world_x;
            transform.translation.y = target.world_z;
            transform.translation.z = HOVER_MARKER_Z;
            sprite.image = texture.clone();
            sprite.color = color_from_rgb(target.color_rgb);
            sprite.custom_size = Some(Vec2::splat(target.marker_size_screen_px * scale));
            *visibility = Visibility::Visible;
        }
        let Some(viewport_position) = world_to_viewport(
            camera,
            camera_transform,
            Vec3::new(target.world_x, target.world_z, 0.0),
        ) else {
            hide_hover_target_label(pair, &mut label_roots);
            continue;
        };
        let panel_size_px = hover_callout_size_px(&target.label);
        let (left_px, top_px) = if let Ok(window) = windows.single() {
            let max_left = (window.width() - panel_size_px.x).max(0.0);
            let max_top = (window.height() - panel_size_px.y).max(0.0);
            (
                (viewport_position.x - panel_size_px.x * 0.5).clamp(0.0, max_left),
                (viewport_position.y - target.label_offset_screen_px - panel_size_px.y)
                    .clamp(0.0, max_top),
            )
        } else {
            (
                viewport_position.x - panel_size_px.x * 0.5,
                viewport_position.y - target.label_offset_screen_px - panel_size_px.y,
            )
        };
        if let Ok((mut node, mut visibility, mut background, mut border)) =
            label_roots.get_mut(pair.label_root)
        {
            node.left = Val::Px(left_px);
            node.top = Val::Px(top_px);
            node.width = Val::Px(panel_size_px.x);
            node.height = Val::Px(panel_size_px.y);
            node.border = UiRect::all(Val::Px(HOVER_CALLOUT_BORDER_SCREEN_PX));
            node.border_radius = BorderRadius::all(Val::Px(HOVER_CALLOUT_CORNER_RADIUS_SCREEN_PX));
            *background = BackgroundColor(callout_panel_color);
            *border = BorderColor::all(callout_border_color);
            *visibility = Visibility::Visible;
        }
        if let Ok((mut text, mut text_font, mut text_color)) = label_texts.get_mut(pair.label_text)
        {
            text.0 = target.label.clone();
            text_font.font = fonts.regular.clone();
            text_font.font_size = HOVER_LABEL_SIZE_PX;
            text_color.0 = label_color;
        }
    }

    for pair in marker_pool.markers.iter().skip(targets.len()) {
        if let Ok((_, mut visibility, _)) = markers.get_mut(pair.marker) {
            *visibility = Visibility::Hidden;
        }
        hide_hover_target_label(*pair, &mut label_roots);
    }
}

fn hide_hover_targets(
    marker_pool: &HoverTargetMarkerPool,
    markers: &mut Query<
        (&mut Transform, &mut Visibility, &mut Sprite),
        (
            With<HoverTargetMarker>,
            Without<HoverTargetLabelRoot>,
            Without<HoverTargetLabelText>,
        ),
    >,
    label_roots: &mut Query<
        (
            &mut Node,
            &mut Visibility,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (
            With<HoverTargetLabelRoot>,
            Without<HoverTargetMarker>,
            Without<HoverTargetLabelText>,
        ),
    >,
) {
    for pair in &marker_pool.markers {
        if let Ok((_, mut visibility, _)) = markers.get_mut(pair.marker) {
            *visibility = Visibility::Hidden;
        }
        hide_hover_target_label(*pair, label_roots);
    }
}

fn hide_hover_target_label(
    pair: HoverTargetVisualPair,
    label_roots: &mut Query<
        (
            &mut Node,
            &mut Visibility,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (
            With<HoverTargetLabelRoot>,
            Without<HoverTargetMarker>,
            Without<HoverTargetLabelText>,
        ),
    >,
) {
    if let Ok((_, mut visibility, _, _)) = label_roots.get_mut(pair.label_root) {
        *visibility = Visibility::Hidden;
    }
}

fn hover_target_border_color(colors: &FishyMapThemeColors) -> Option<Color> {
    colors
        .base300
        .as_deref()
        .or(colors.primary.as_deref())
        .or(colors.base200.as_deref())
        .and_then(parse_css_color)
        .map(|color| color.with_alpha(0.96))
}

fn hover_target_panel_color(colors: &FishyMapThemeColors) -> Option<Color> {
    colors
        .base200
        .as_deref()
        .or(colors.base100.as_deref())
        .and_then(parse_css_color)
        .map(|color| color.with_alpha(0.95))
}

fn hover_target_label_color(colors: &FishyMapThemeColors) -> Option<Color> {
    colors
        .base_content
        .as_deref()
        .or(colors.primary_content.as_deref())
        .and_then(parse_css_color)
        .map(|color| color.with_alpha(0.98))
}

fn hover_targets_from_info(
    info: Option<&HoverInfo>,
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
) -> Vec<HoverTargetVisual> {
    let Some(info) = info else {
        return Vec::new();
    };

    info.layer_samples
        .iter()
        .filter(|sample| layer_visible_by_key(&sample.layer_id, layer_registry, layer_runtime))
        .flat_map(hover_targets_from_sample)
        .collect()
}

fn layer_visible_by_key(
    layer_key: &str,
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
) -> bool {
    layer_registry
        .get_by_key(layer_key)
        .map(|layer| layer_runtime.visible(layer.id))
        .unwrap_or(true)
}

fn hover_targets_from_sample(
    sample: &crate::plugins::api::HoverLayerSample,
) -> Vec<HoverTargetVisual> {
    sample
        .targets
        .iter()
        .filter_map(hover_target_visual)
        .collect()
}

fn hover_target_visual(
    target: &fishystuff_core::field_metadata::FieldHoverTarget,
) -> Option<HoverTargetVisual> {
    let (marker_size_screen_px, label_offset_screen_px, color_rgb) = match target.key.as_str() {
        "resource_node" => (
            RESOURCE_BAR_MARKER_SIZE_SCREEN_PX,
            RESOURCE_BAR_LABEL_OFFSET_SCREEN_PX,
            RESOURCE_BAR_MARKER_COLOR,
        ),
        "origin_node" => (
            ORIGIN_NODE_MARKER_SIZE_SCREEN_PX,
            ORIGIN_NODE_LABEL_OFFSET_SCREEN_PX,
            ORIGIN_NODE_MARKER_COLOR,
        ),
        _ => (
            ORIGIN_NODE_MARKER_SIZE_SCREEN_PX,
            ORIGIN_NODE_LABEL_OFFSET_SCREEN_PX,
            ORIGIN_NODE_MARKER_COLOR,
        ),
    };
    Some(HoverTargetVisual {
        world_x: target.world_x as f32,
        world_z: target.world_z as f32,
        label: target.label.clone(),
        marker_size_screen_px,
        label_offset_screen_px,
        color_rgb,
    })
}

fn map_2d_viewport_bounds(
    windows: &Query<&Window, With<PrimaryWindow>>,
    camera_q: &Query<
        (&Camera, &GlobalTransform, &Projection),
        (
            With<Map2dCamera>,
            Without<HoverTargetMarker>,
            Without<HoverTargetLabelRoot>,
            Without<HoverTargetLabelText>,
        ),
    >,
) -> Option<Map2dViewportBounds> {
    let window = windows.single().ok()?;
    let (_, camera_transform, projection) = camera_q.single().ok()?;
    let Projection::Orthographic(orthographic) = projection else {
        return None;
    };
    let scale = orthographic.scale.max(f32::EPSILON);
    let half_width = window.width().max(1.0) * 0.5 * scale;
    let half_height = window.height().max(1.0) * 0.5 * scale;
    let translation = camera_transform.translation();
    Some(Map2dViewportBounds {
        min_x: translation.x - half_width,
        max_x: translation.x + half_width,
        min_z: translation.y - half_height,
        max_z: translation.y + half_height,
        scale,
    })
}

fn clamp_hover_target_to_viewport(
    target: &HoverTargetVisual,
    viewport: Map2dViewportBounds,
) -> HoverTargetVisual {
    let margin_world =
        (target.marker_size_screen_px * 0.5 + VIEW_EDGE_PADDING_SCREEN_PX) * viewport.scale;
    let clamped_x = target
        .world_x
        .clamp(viewport.min_x + margin_world, viewport.max_x - margin_world);
    let clamped_z = target
        .world_z
        .clamp(viewport.min_z + margin_world, viewport.max_z - margin_world);
    let mut next = target.clone();
    if (clamped_x - target.world_x).abs() > f32::EPSILON
        || (clamped_z - target.world_z).abs() > f32::EPSILON
    {
        next.label = format!("{} (offscreen)", next.label);
    }
    next.world_x = clamped_x;
    next.world_z = clamped_z;
    next
}

fn camera_scale(
    camera_q: &Query<
        (&Camera, &GlobalTransform, &Projection),
        (
            With<Map2dCamera>,
            Without<HoverTargetMarker>,
            Without<HoverTargetLabelRoot>,
            Without<HoverTargetLabelText>,
        ),
    >,
) -> f32 {
    camera_q
        .single()
        .ok()
        .and_then(|(_, _, projection)| match projection {
            Projection::Orthographic(ortho) => Some(ortho.scale),
            _ => None,
        })
        .unwrap_or(1.0)
        .max(f32::EPSILON)
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

fn hover_callout_size_px(display_text: &str) -> Vec2 {
    let text_width_px =
        display_text.chars().count() as f32 * HOVER_LABEL_SIZE_PX * HOVER_TEXT_WIDTH_FACTOR
            + HOVER_TEXT_WIDTH_SLACK_SCREEN_PX;
    let width_px = (text_width_px + HOVER_CALLOUT_PADDING_X_SCREEN_PX * 2.0)
        .max(HOVER_CALLOUT_MIN_WIDTH_SCREEN_PX);
    Vec2::new(width_px, HOVER_CALLOUT_HEIGHT_SCREEN_PX)
}

fn color_from_rgb([red, green, blue]: [u8; 3]) -> Color {
    Color::srgb_u8(red, green, blue)
}

fn build_hover_marker_texture() -> Image {
    let width = HOVER_TEXTURE_WIDTH_PX;
    let height = HOVER_TEXTURE_HEIGHT_PX;
    let center_x = (width as f32 - 1.0) * 0.5;
    let center_y = (height as f32 - 1.0) * 0.5;

    let mut texture_data = vec![0_u8; width * height * 4];
    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 - center_x;
            let dy = y as f32 - center_y;
            let distance = (dx * dx + dy * dy).sqrt();

            let ring_alpha = ring_alpha(distance);
            let core_alpha = circle_alpha(distance, HOVER_CORE_RADIUS_PX);
            let alpha = ring_alpha.max(core_alpha);
            if alpha <= 0.0 {
                continue;
            }

            let offset = (y * width + x) * 4;
            texture_data[offset] = 255;
            texture_data[offset + 1] = 255;
            texture_data[offset + 2] = 255;
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
    let half_thickness = HOVER_RING_THICKNESS_PX * 0.5;
    let edge_distance = (distance - HOVER_RING_RADIUS_PX).abs();
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
    use super::{
        clamp_hover_target_to_viewport, hover_targets_from_info, HoverTargetVisual,
        Map2dViewportBounds, ORIGIN_NODE_LABEL_OFFSET_SCREEN_PX, ORIGIN_NODE_MARKER_COLOR,
        ORIGIN_NODE_MARKER_SIZE_SCREEN_PX, RESOURCE_BAR_LABEL_OFFSET_SCREEN_PX,
        RESOURCE_BAR_MARKER_COLOR, RESOURCE_BAR_MARKER_SIZE_SCREEN_PX,
    };
    use crate::map::layers::{LayerRegistry, LayerRuntime};
    use crate::plugins::api::{HoverInfo, HoverLayerSample};
    use fishystuff_api::models::layers::{
        GeometrySpace, LayerDescriptor, LayerKind as LayerKindDto, LayerTransformDto, LayerUiInfo,
        LayersResponse, LodPolicyDto, StyleMode, TilesetRef, VectorSourceRef,
    };
    use fishystuff_api::Rgb;
    use fishystuff_core::field_metadata::FieldHoverTarget;

    fn sample(layer_id: &str) -> HoverLayerSample {
        HoverLayerSample {
            layer_id: layer_id.to_string(),
            layer_name: layer_id.to_string(),
            kind: "vector-geojson".to_string(),
            rgb: Rgb::new(0, 0, 0),
            rgb_u32: 0,
            field_id: None,
            rows: Vec::new(),
            targets: Vec::new(),
        }
    }

    fn vector_descriptor(layer_id: &str, name: &str, display_order: i32) -> LayerDescriptor {
        LayerDescriptor {
            layer_id: layer_id.to_string(),
            name: name.to_string(),
            enabled: true,
            kind: LayerKindDto::VectorGeoJson,
            transform: LayerTransformDto::IdentityMapSpace,
            tileset: TilesetRef::default(),
            tile_px: 512,
            max_level: 0,
            y_flip: false,
            field_source: None,
            field_metadata_source: None,
            vector_source: Some(VectorSourceRef {
                url: format!("/{layer_id}/v1.geojson"),
                revision: format!("{layer_id}-v1"),
                geometry_space: GeometrySpace::MapPixels,
                style_mode: StyleMode::FeaturePropertyPalette,
                feature_id_property: Some("id".to_string()),
                color_property: Some("c".to_string()),
            }),
            lod_policy: LodPolicyDto::default(),
            ui: LayerUiInfo {
                visible_default: true,
                opacity_default: 1.0,
                z_base: 30.0,
                display_order,
            },
            request_weight: 1.0,
            pick_mode: "none".to_string(),
        }
    }

    fn hover_layer_state() -> (LayerRegistry, LayerRuntime) {
        let mut registry = LayerRegistry::default();
        registry.apply_layers_response(LayersResponse {
            revision: "rev".to_string(),
            map_version_id: None,
            layers: vec![
                vector_descriptor("region_groups", "Region Groups", 30),
                vector_descriptor("regions", "Regions", 40),
            ],
        });
        let mut runtime = LayerRuntime::default();
        runtime.sync_to_registry(&registry);
        (registry, runtime)
    }

    #[test]
    fn hover_targets_include_resource_bar_and_origin_node() {
        let mut region_group = sample("region_groups");
        region_group.targets.push(FieldHoverTarget {
            key: "resource_node".to_string(),
            label: "Resource node: Tarif".to_string(),
            world_x: 123.0,
            world_z: 456.0,
        });

        let mut regions = sample("regions");
        regions.targets.push(FieldHoverTarget {
            key: "origin_node".to_string(),
            label: "Origin: Tarif".to_string(),
            world_x: 789.0,
            world_z: 321.0,
        });

        let info = HoverInfo {
            map_px: 0,
            map_py: 0,
            rgb: None,
            rgb_u32: None,
            zone_name: None,
            world_x: 0.0,
            world_z: 0.0,
            layer_samples: vec![region_group, regions],
        };

        let layer_registry = LayerRegistry::default();
        let layer_runtime = LayerRuntime::default();
        assert_eq!(
            hover_targets_from_info(Some(&info), &layer_registry, &layer_runtime),
            vec![
                HoverTargetVisual {
                    world_x: 123.0,
                    world_z: 456.0,
                    label: "Resource node: Tarif".to_string(),
                    marker_size_screen_px: RESOURCE_BAR_MARKER_SIZE_SCREEN_PX,
                    label_offset_screen_px: RESOURCE_BAR_LABEL_OFFSET_SCREEN_PX,
                    color_rgb: RESOURCE_BAR_MARKER_COLOR,
                },
                HoverTargetVisual {
                    world_x: 789.0,
                    world_z: 321.0,
                    label: "Origin: Tarif".to_string(),
                    marker_size_screen_px: ORIGIN_NODE_MARKER_SIZE_SCREEN_PX,
                    label_offset_screen_px: ORIGIN_NODE_LABEL_OFFSET_SCREEN_PX,
                    color_rgb: ORIGIN_NODE_MARKER_COLOR,
                },
            ]
        );
    }

    #[test]
    fn hover_targets_fall_back_to_separate_samples() {
        let mut region_group = sample("region_groups");
        region_group.targets.push(FieldHoverTarget {
            key: "resource_node".to_string(),
            label: "Resource node".to_string(),
            world_x: 10.0,
            world_z: 20.0,
        });

        let mut region = sample("regions");
        region.targets.push(FieldHoverTarget {
            key: "origin_node".to_string(),
            label: "Origin node".to_string(),
            world_x: 30.0,
            world_z: 40.0,
        });

        let info = HoverInfo {
            map_px: 0,
            map_py: 0,
            rgb: None,
            rgb_u32: None,
            zone_name: None,
            world_x: 0.0,
            world_z: 0.0,
            layer_samples: vec![region_group, region],
        };

        let layer_registry = LayerRegistry::default();
        let layer_runtime = LayerRuntime::default();
        let targets = hover_targets_from_info(Some(&info), &layer_registry, &layer_runtime);
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].world_x, 10.0);
        assert_eq!(targets[0].world_z, 20.0);
        assert_eq!(targets[1].world_x, 30.0);
        assert_eq!(targets[1].world_z, 40.0);
    }

    #[test]
    fn hover_targets_only_show_markers_for_visible_layers() {
        let mut region_group = sample("region_groups");
        region_group.targets.push(FieldHoverTarget {
            key: "resource_node".to_string(),
            label: "Resource node".to_string(),
            world_x: 10.0,
            world_z: 20.0,
        });

        let mut region = sample("regions");
        region.targets.push(FieldHoverTarget {
            key: "origin_node".to_string(),
            label: "Origin: Tarif".to_string(),
            world_x: 30.0,
            world_z: 40.0,
        });

        let info = HoverInfo {
            map_px: 0,
            map_py: 0,
            rgb: None,
            rgb_u32: None,
            zone_name: None,
            world_x: 0.0,
            world_z: 0.0,
            layer_samples: vec![region_group.clone(), region.clone()],
        };

        let (layer_registry, mut layer_runtime) = hover_layer_state();
        let region_groups_id = layer_registry
            .get_by_key("region_groups")
            .expect("region_groups layer")
            .id;
        let regions_id = layer_registry
            .get_by_key("regions")
            .expect("regions layer")
            .id;
        layer_runtime.set_visible(region_groups_id, true);
        layer_runtime.set_visible(regions_id, false);
        assert_eq!(
            hover_targets_from_info(Some(&info), &layer_registry, &layer_runtime),
            vec![HoverTargetVisual {
                world_x: 10.0,
                world_z: 20.0,
                label: "Resource node".to_string(),
                marker_size_screen_px: RESOURCE_BAR_MARKER_SIZE_SCREEN_PX,
                label_offset_screen_px: RESOURCE_BAR_LABEL_OFFSET_SCREEN_PX,
                color_rgb: RESOURCE_BAR_MARKER_COLOR,
            }]
        );

        layer_runtime.set_visible(region_groups_id, false);
        layer_runtime.set_visible(regions_id, true);
        assert_eq!(
            hover_targets_from_info(Some(&info), &layer_registry, &layer_runtime),
            vec![HoverTargetVisual {
                world_x: 30.0,
                world_z: 40.0,
                label: "Origin: Tarif".to_string(),
                marker_size_screen_px: ORIGIN_NODE_MARKER_SIZE_SCREEN_PX,
                label_offset_screen_px: ORIGIN_NODE_LABEL_OFFSET_SCREEN_PX,
                color_rgb: ORIGIN_NODE_MARKER_COLOR,
            }]
        );
    }

    #[test]
    fn clamp_hover_target_to_viewport_marks_offscreen_targets() {
        let target = HoverTargetVisual {
            world_x: 500.0,
            world_z: 800.0,
            label: "Origin: Tarif".to_string(),
            marker_size_screen_px: ORIGIN_NODE_MARKER_SIZE_SCREEN_PX,
            label_offset_screen_px: ORIGIN_NODE_LABEL_OFFSET_SCREEN_PX,
            color_rgb: ORIGIN_NODE_MARKER_COLOR,
        };
        let clamped = clamp_hover_target_to_viewport(
            &target,
            Map2dViewportBounds {
                min_x: 0.0,
                max_x: 100.0,
                min_z: 0.0,
                max_z: 100.0,
                scale: 1.0,
            },
        );
        assert!(clamped.world_x <= 100.0);
        assert!(clamped.world_z <= 100.0);
        assert_eq!(clamped.label, "Origin: Tarif (offscreen)");
    }
}
