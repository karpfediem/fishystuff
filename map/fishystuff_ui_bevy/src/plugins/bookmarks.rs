use std::collections::HashSet;

use bevy::asset::RenderAssetUsages;
use bevy::color::Alpha;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::bridge::contract::{FishyMapBookmarkEntry, FishyMapThemeColors};
use crate::bridge::theme::parse_css_color;
use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::plugins::api::{HoverInfo, HoverState};
use crate::plugins::camera::Map2dCamera;
use crate::plugins::render_domain::{world_2d_layers, World2dRenderEntity};
use crate::plugins::ui::UiFonts;

#[cfg(target_arch = "wasm32")]
use crate::bridge::host::BrowserBridgeState;

const BOOKMARK_MARKER_SIZE_SCREEN_PX: f32 = 22.0;
const BOOKMARK_HOVER_RADIUS_SCREEN_PX: f32 = 14.0;
const BOOKMARK_MARKER_Z: f32 = 40.4;
const BOOKMARK_CALLOUT_LABEL_SIZE_PX: f32 = 12.0;
const BOOKMARK_CALLOUT_MIN_WIDTH_SCREEN_PX: f32 = 76.0;
const BOOKMARK_CALLOUT_HEIGHT_SCREEN_PX: f32 = 28.0;
const BOOKMARK_CALLOUT_PADDING_X_SCREEN_PX: f32 = 12.0;
const BOOKMARK_CALLOUT_GAP_SCREEN_PX: f32 = 10.0;
const BOOKMARK_CALLOUT_BORDER_SCREEN_PX: f32 = 2.0;
const BOOKMARK_CALLOUT_CORNER_RADIUS_SCREEN_PX: f32 = 10.0;
const BOOKMARK_TEXT_WIDTH_FACTOR: f32 = 0.58;
const BOOKMARK_TEXTURE_WIDTH_PX: usize = 32;
const BOOKMARK_TEXTURE_HEIGHT_PX: usize = 32;
const BOOKMARK_RING_RADIUS_PX: f32 = 12.0;
const BOOKMARK_RING_THICKNESS_PX: f32 = 4.0;
const BOOKMARK_CORE_RADIUS_PX: f32 = 5.0;
const BOOKMARK_COLOR: [u8; 3] = [239, 92, 31];
const BOOKMARK_CORE_COLOR: [u8; 3] = [255, 242, 214];
const BOOKMARK_CALLOUT_BORDER_COLOR: Color = Color::srgba(0.94, 0.56, 0.27, 0.98);
const BOOKMARK_CALLOUT_PANEL_COLOR: Color = Color::srgba(0.07, 0.09, 0.12, 0.95);
const BOOKMARK_CALLOUT_LABEL_COLOR: Color = Color::srgb(0.98, 0.97, 0.94);
const EDGE_FEATHER_PX: f32 = 1.2;

pub struct BookmarksPlugin;

impl Plugin for BookmarksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BookmarkState>()
            .init_resource::<BookmarkRenderAssets>()
            .init_resource::<BookmarkMarkerPool>()
            .add_systems(
                Update,
                (ensure_bookmark_render_assets, sync_bookmark_markers).chain(),
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

#[derive(Component)]
struct BookmarkCalloutRoot;

#[derive(Component)]
struct BookmarkCalloutText;

#[derive(Clone, Copy, Debug)]
struct BookmarkVisualSet {
    marker: Entity,
    callout_root: Entity,
    callout_text: Entity,
}

#[derive(Resource, Default)]
struct BookmarkRenderAssets {
    marker_texture: Option<Handle<Image>>,
}

#[derive(Resource, Default)]
struct BookmarkMarkerPool {
    markers: Vec<BookmarkVisualSet>,
}

fn ensure_bookmark_render_assets(
    mut render_assets: ResMut<BookmarkRenderAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    if render_assets.marker_texture.is_none() {
        render_assets.marker_texture = Some(images.add(build_bookmark_marker_texture()));
    }
}

fn sync_bookmark_markers(
    mut commands: Commands,
    bookmarks: Res<BookmarkState>,
    hover: Res<HoverState>,
    view_mode: Res<ViewModeState>,
    #[cfg(target_arch = "wasm32")] bridge: Res<BrowserBridgeState>,
    fonts: Res<UiFonts>,
    render_assets: Res<BookmarkRenderAssets>,
    mut marker_pool: ResMut<BookmarkMarkerPool>,
    camera_q: Query<(&Camera, &GlobalTransform, &Projection), With<Map2dCamera>>,
    mut markers: Query<
        (&mut Transform, &mut Visibility, &mut Sprite),
        (With<BookmarkMarker>, Without<BookmarkCalloutRoot>),
    >,
    mut callout_roots: Query<
        (
            &mut Node,
            &mut Visibility,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (
            With<BookmarkCalloutRoot>,
            Without<BookmarkMarker>,
            Without<BookmarkCalloutText>,
        ),
    >,
    mut callout_texts: Query<
        (&mut Text, &mut TextFont, &mut TextColor),
        (
            With<BookmarkCalloutText>,
            Without<BookmarkMarker>,
            Without<BookmarkCalloutRoot>,
        ),
    >,
) {
    if view_mode.mode != ViewMode::Map2D || bookmarks.entries.is_empty() {
        hide_bookmark_visuals(&marker_pool, &mut markers, &mut callout_roots);
        return;
    }

    let Some(marker_texture) = render_assets.marker_texture.as_ref() else {
        return;
    };
    let Ok((camera, camera_transform, projection)) = camera_q.single() else {
        hide_bookmark_visuals(&marker_pool, &mut markers, &mut callout_roots);
        return;
    };
    let current_scale = match projection {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => 1.0,
    }
    .max(f32::EPSILON);

    let marker_size_world = BOOKMARK_MARKER_SIZE_SCREEN_PX * current_scale;
    let hovered_index =
        hovered_bookmark_index(&bookmarks.entries, hover.info.as_ref(), current_scale);
    let selected_ids = bookmarks
        .selected_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    #[cfg(target_arch = "wasm32")]
    let theme_colors = Some(&bridge.input.theme.colors);
    #[cfg(not(target_arch = "wasm32"))]
    let theme_colors: Option<&FishyMapThemeColors> = None;
    let callout_border_color = theme_colors
        .and_then(bookmark_callout_border_color)
        .unwrap_or(BOOKMARK_CALLOUT_BORDER_COLOR);
    let callout_panel_color = theme_colors
        .and_then(bookmark_callout_panel_color)
        .unwrap_or(BOOKMARK_CALLOUT_PANEL_COLOR);
    let callout_label_color = theme_colors
        .and_then(bookmark_callout_label_color)
        .unwrap_or(BOOKMARK_CALLOUT_LABEL_COLOR);

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

        let mut callout_text = Entity::PLACEHOLDER;
        let callout_root = commands
            .spawn((
                BookmarkCalloutRoot,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Px(BOOKMARK_CALLOUT_MIN_WIDTH_SCREEN_PX),
                    height: Val::Px(BOOKMARK_CALLOUT_HEIGHT_SCREEN_PX),
                    padding: UiRect::axes(
                        Val::Px(BOOKMARK_CALLOUT_PADDING_X_SCREEN_PX),
                        Val::Px(0.0),
                    ),
                    border: UiRect::all(Val::Px(BOOKMARK_CALLOUT_BORDER_SCREEN_PX)),
                    border_radius: BorderRadius::all(Val::Px(
                        BOOKMARK_CALLOUT_CORNER_RADIUS_SCREEN_PX,
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
                callout_text = parent
                    .spawn((
                        BookmarkCalloutText,
                        Text::new(""),
                        TextFont {
                            font: fonts.regular.clone(),
                            font_size: BOOKMARK_CALLOUT_LABEL_SIZE_PX,
                            ..default()
                        },
                        TextColor(callout_label_color),
                    ))
                    .id();
            })
            .id();

        marker_pool.markers.push(BookmarkVisualSet {
            marker,
            callout_root,
            callout_text,
        });
    }

    for (index, bookmark) in bookmarks.entries.iter().enumerate() {
        let visual = marker_pool.markers[index];
        let world_x = bookmark.world_x as f32;
        let world_z = bookmark.world_z as f32;

        if let Ok((mut transform, mut visibility, mut sprite)) = markers.get_mut(visual.marker) {
            transform.translation = Vec3::new(world_x, world_z, BOOKMARK_MARKER_Z);
            sprite.image = marker_texture.clone();
            sprite.custom_size = Some(Vec2::splat(marker_size_world));
            *visibility = Visibility::Visible;
        }

        let callout_visible =
            selected_ids.contains(bookmark.id.as_str()) || hovered_index == Some(index);
        if !callout_visible {
            hide_bookmark_callout(visual, &mut callout_roots);
            continue;
        }

        let Some(viewport_position) =
            world_to_viewport(camera, camera_transform, Vec3::new(world_x, world_z, 0.0))
        else {
            hide_bookmark_callout(visual, &mut callout_roots);
            continue;
        };

        let display_text = format!("{}: {}", index + 1, bookmark_display_label(bookmark, index));
        let panel_size_px = bookmark_callout_size_px(&display_text);
        let top_px = viewport_position.y
            - BOOKMARK_MARKER_SIZE_SCREEN_PX * 0.5
            - BOOKMARK_CALLOUT_GAP_SCREEN_PX
            - panel_size_px.y;
        let left_px = viewport_position.x - panel_size_px.x * 0.5;

        if let Ok((mut node, mut visibility, mut background, mut border)) =
            callout_roots.get_mut(visual.callout_root)
        {
            node.left = Val::Px(left_px);
            node.top = Val::Px(top_px);
            node.width = Val::Px(panel_size_px.x);
            node.height = Val::Px(panel_size_px.y);
            node.border = UiRect::all(Val::Px(BOOKMARK_CALLOUT_BORDER_SCREEN_PX));
            node.border_radius =
                BorderRadius::all(Val::Px(BOOKMARK_CALLOUT_CORNER_RADIUS_SCREEN_PX));
            *background = BackgroundColor(callout_panel_color);
            *border = BorderColor::all(callout_border_color);
            *visibility = Visibility::Visible;
        }
        if let Ok((mut text, mut text_font, mut text_color)) =
            callout_texts.get_mut(visual.callout_text)
        {
            text.0 = display_text;
            text_font.font = fonts.regular.clone();
            text_font.font_size = BOOKMARK_CALLOUT_LABEL_SIZE_PX;
            text_color.0 = callout_label_color;
        }
    }

    for visual in marker_pool.markers.iter().skip(bookmarks.entries.len()) {
        if let Ok((_, mut visibility, _)) = markers.get_mut(visual.marker) {
            *visibility = Visibility::Hidden;
        }
        hide_bookmark_callout(*visual, &mut callout_roots);
    }
}

fn hide_bookmark_visuals(
    marker_pool: &BookmarkMarkerPool,
    markers: &mut Query<
        (&mut Transform, &mut Visibility, &mut Sprite),
        (With<BookmarkMarker>, Without<BookmarkCalloutRoot>),
    >,
    callout_roots: &mut Query<
        (
            &mut Node,
            &mut Visibility,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (
            With<BookmarkCalloutRoot>,
            Without<BookmarkMarker>,
            Without<BookmarkCalloutText>,
        ),
    >,
) {
    for visual in &marker_pool.markers {
        if let Ok((_, mut visibility, _)) = markers.get_mut(visual.marker) {
            *visibility = Visibility::Hidden;
        }
        hide_bookmark_callout(*visual, callout_roots);
    }
}

fn hide_bookmark_callout(
    visual: BookmarkVisualSet,
    callout_roots: &mut Query<
        (
            &mut Node,
            &mut Visibility,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (
            With<BookmarkCalloutRoot>,
            Without<BookmarkMarker>,
            Without<BookmarkCalloutText>,
        ),
    >,
) {
    if let Ok((_, mut visibility, _, _)) = callout_roots.get_mut(visual.callout_root) {
        *visibility = Visibility::Hidden;
    }
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

fn bookmark_callout_border_color(colors: &FishyMapThemeColors) -> Option<Color> {
    colors
        .primary
        .as_deref()
        .or(colors.base300.as_deref())
        .or(colors.base200.as_deref())
        .and_then(parse_css_color)
        .map(|color| color.with_alpha(0.96))
}

fn bookmark_callout_panel_color(colors: &FishyMapThemeColors) -> Option<Color> {
    colors
        .base200
        .as_deref()
        .or(colors.base100.as_deref())
        .and_then(parse_css_color)
        .map(|color| color.with_alpha(0.95))
}

fn bookmark_callout_label_color(colors: &FishyMapThemeColors) -> Option<Color> {
    colors
        .base_content
        .as_deref()
        .or(colors.primary_content.as_deref())
        .and_then(parse_css_color)
        .map(|color| color.with_alpha(0.98))
}

fn hovered_bookmark_index(
    entries: &[FishyMapBookmarkEntry],
    hover: Option<&HoverInfo>,
    current_scale: f32,
) -> Option<usize> {
    let Some(hover) = hover else {
        return None;
    };
    let max_distance_sq = (BOOKMARK_HOVER_RADIUS_SCREEN_PX * current_scale).powi(2);
    entries
        .iter()
        .enumerate()
        .filter_map(|(index, bookmark)| {
            let dx = bookmark.world_x as f32 - hover.world_x as f32;
            let dz = bookmark.world_z as f32 - hover.world_z as f32;
            let distance_sq = dx * dx + dz * dz;
            (distance_sq <= max_distance_sq).then_some((index, distance_sq))
        })
        .min_by(|lhs, rhs| lhs.1.total_cmp(&rhs.1))
        .map(|(index, _)| index)
}

fn bookmark_display_label(bookmark: &FishyMapBookmarkEntry, index: usize) -> String {
    bookmark
        .label
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            bookmark
                .zone_name
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| format!("Bookmark {}", index + 1))
}

fn bookmark_callout_size_px(display_text: &str) -> Vec2 {
    let text_width_px = display_text.chars().count() as f32
        * BOOKMARK_CALLOUT_LABEL_SIZE_PX
        * BOOKMARK_TEXT_WIDTH_FACTOR;
    let width_px = (text_width_px + BOOKMARK_CALLOUT_PADDING_X_SCREEN_PX * 2.0)
        .max(BOOKMARK_CALLOUT_MIN_WIDTH_SCREEN_PX);
    Vec2::new(width_px, BOOKMARK_CALLOUT_HEIGHT_SCREEN_PX)
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

    #[test]
    fn hovered_bookmark_index_prefers_the_closest_bookmark_in_radius() {
        let entries = vec![
            FishyMapBookmarkEntry {
                id: "bookmark-a".to_string(),
                label: Some("A".to_string()),
                world_x: 100.0,
                world_z: 100.0,
                zone_name: None,
                resource_name: None,
                origin_name: None,
                zone_rgb: None,
                created_at: None,
            },
            FishyMapBookmarkEntry {
                id: "bookmark-b".to_string(),
                label: Some("B".to_string()),
                world_x: 108.0,
                world_z: 108.0,
                zone_name: None,
                resource_name: None,
                origin_name: None,
                zone_rgb: None,
                created_at: None,
            },
        ];

        let hover = HoverInfo {
            map_px: 0,
            map_py: 0,
            rgb: None,
            rgb_u32: None,
            zone_name: None,
            world_x: 101.0,
            world_z: 101.0,
            layer_samples: Vec::new(),
        };

        assert_eq!(hovered_bookmark_index(&entries, Some(&hover), 1.0), Some(0));
        assert_eq!(hovered_bookmark_index(&entries, Some(&hover), 0.2), None);
    }
}
