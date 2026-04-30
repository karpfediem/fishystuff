use bevy::asset::RenderAssetUsages;
use bevy::color::Alpha;
use bevy::ecs::system::SystemParam;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::text::{Justify, TextLayout};
use bevy::window::PrimaryWindow;
use bevy_flair::prelude::{ClassList, NodeStyleSheet};

use crate::bridge::contract::FishyMapThemeColors;
use crate::bridge::theme::parse_css_color;
use crate::map::camera::mode::{ViewMode, ViewModeState};
use crate::map::layers::{LayerRegistry, LayerRuntime};
use crate::plugins::api::{HoverInfo, HoverState, SelectedInfo, SelectionState};
use crate::plugins::camera::Map2dCamera;
use crate::plugins::render_domain::{world_2d_layers, World2dRenderEntity};
use crate::plugins::svg_icons::{UiSvgIconAssets, UiSvgIconKind};
use crate::plugins::ui::{UiFonts, UiRoot};

#[cfg(target_arch = "wasm32")]
use crate::bridge::host::BrowserBridgeState;

const HOVER_MARKER_Z: f32 = 40.6;
const HOVER_LABEL_SIZE_PX: f32 = 12.0;
const HOVER_LABEL_COLOR: Color = Color::srgb(0.98, 0.97, 0.94);
const HOVER_CALLOUT_MIN_WIDTH_SCREEN_PX: f32 = 56.0;
const HOVER_CALLOUT_HEIGHT_SCREEN_PX: f32 = 28.0;
const HOVER_CALLOUT_PADDING_X_SCREEN_PX: f32 = 6.0;
const HOVER_CALLOUT_PADDING_LEFT_SCREEN_PX: f32 =
    HOVER_CALLOUT_PADDING_X_SCREEN_PX + HOVER_ICON_SIZE_SCREEN_PX + HOVER_ICON_GAP_SCREEN_PX;
const HOVER_CALLOUT_PADDING_RIGHT_SCREEN_PX: f32 = HOVER_CALLOUT_PADDING_X_SCREEN_PX;
const HOVER_CALLOUT_BORDER_SCREEN_PX: f32 = 2.0;
const HOVER_CALLOUT_CORNER_RADIUS_SCREEN_PX: f32 = 10.0;
const HOVER_ICON_SIZE_SCREEN_PX: f32 = 14.0;
const HOVER_ICON_GAP_SCREEN_PX: f32 = 8.0;
const HOVER_ICON_INSET_LEFT_SCREEN_PX: f32 = HOVER_CALLOUT_PADDING_X_SCREEN_PX;
const HOVER_ICON_TOP_SCREEN_PX: f32 =
    (HOVER_CALLOUT_HEIGHT_SCREEN_PX - HOVER_ICON_SIZE_SCREEN_PX) * 0.5 - 2.5;
const HOVER_PLAIN_TEXT_WIDTH_FACTOR: f32 = 0.62;
const HOVER_PLAIN_TEXT_WIDTH_SLACK_SCREEN_PX: f32 = 4.0;
const HOVER_PREFIX_TEXT_WIDTH_FACTOR: f32 = 0.54;
const HOVER_CODE_TEXT_WIDTH_FACTOR: f32 = 0.60;
const HOVER_NAME_TEXT_WIDTH_FACTOR: f32 = 0.56;
const HOVER_SEMANTIC_GAP_SCREEN_PX: f32 = 5.0;
const HOVER_CHIP_CODE_PADDING_X_SCREEN_PX: f32 = 7.0;
const HOVER_CHIP_NAME_PADDING_X_SCREEN_PX: f32 = 9.0;
const HOVER_SEMANTIC_WIDTH_SLACK_SCREEN_PX: f32 = 2.0;
const HOVER_CALLOUT_BORDER_COLOR: Color = Color::srgb(0.14, 0.15, 0.19);
const HOVER_CALLOUT_PANEL_COLOR: Color = Color::srgb(0.17, 0.18, 0.22);
const HOVER_CHIP_REGION_GROUP_BORDER_COLOR: Color = Color::srgb(0.24, 0.66, 0.84);
const HOVER_CHIP_REGION_GROUP_PANEL_COLOR: Color = Color::srgb(0.17, 0.18, 0.22);
const HOVER_CHIP_REGION_GROUP_CODE_BG_COLOR: Color = Color::srgb(0.24, 0.66, 0.84);
const HOVER_CHIP_REGION_GROUP_CODE_TEXT_COLOR: Color = Color::srgb(0.97, 0.99, 1.0);
const HOVER_CHIP_REGION_GROUP_NAME_TEXT_COLOR: Color = Color::srgb(0.98, 0.97, 0.94);
const HOVER_CHIP_REGION_BORDER_COLOR: Color = Color::srgb(0.98, 0.78, 0.30);
const HOVER_CHIP_REGION_CODE_BG_COLOR: Color = Color::srgb(0.98, 0.78, 0.30);
const HOVER_CHIP_REGION_CODE_TEXT_COLOR: Color = Color::srgb(0.18, 0.11, 0.0);

const HOVER_TEXTURE_WIDTH_PX: usize = 32;
const HOVER_TEXTURE_HEIGHT_PX: usize = 32;
const HOVER_RING_RADIUS_PX: f32 = 12.0;
const HOVER_RING_THICKNESS_PX: f32 = 3.0;
const HOVER_CORE_RADIUS_PX: f32 = 4.5;
const EDGE_FEATHER_PX: f32 = 1.2;

const RESOURCE_BAR_MARKER_SIZE_SCREEN_PX: f32 = 28.0;
const ORIGIN_NODE_MARKER_SIZE_SCREEN_PX: f32 = 20.0;
const REGION_NODE_MARKER_SIZE_SCREEN_PX: f32 = 20.0;
const RESOURCE_BAR_LABEL_OFFSET_SCREEN_PX: f32 = 20.0;
const ORIGIN_NODE_LABEL_OFFSET_SCREEN_PX: f32 = 36.0;
const REGION_NODE_LABEL_OFFSET_SCREEN_PX: f32 = 28.0;
const VIEW_EDGE_PADDING_SCREEN_PX: f32 = 12.0;

const RESOURCE_BAR_MARKER_COLOR: [u8; 3] = [77, 211, 255];
const ORIGIN_NODE_MARKER_COLOR: [u8; 3] = [255, 196, 66];
const REGION_NODE_MARKER_COLOR: [u8; 3] = [244, 240, 232];
const TERRITORY_DETAIL_PANE_ID: &str = "territory";

pub struct HoverTargetsPlugin;

impl Plugin for HoverTargetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HoverTargetMarkerAssets>()
            .init_resource::<HoverTargetMarkerPool>()
            .add_systems(
                Update,
                (ensure_hover_target_marker_assets, sync_hover_targets).chain(),
            );
    }
}

#[derive(Component)]
struct HoverTargetMarker;

#[derive(Component)]
struct HoverTargetLabelRoot;

#[derive(Component)]
struct HoverTargetLabelText;

#[derive(Component)]
struct HoverTargetLabelIcon;

#[derive(Component)]
struct HoverTargetSemanticInline;

#[derive(Component)]
struct HoverTargetSemanticPrefixText;

#[derive(Component)]
struct HoverTargetSemanticChip;

#[derive(Component)]
struct HoverTargetSemanticChipCodeBox;

#[derive(Component)]
struct HoverTargetSemanticChipCodeText;

#[derive(Component)]
struct HoverTargetSemanticChipNameBox;

#[derive(Component)]
struct HoverTargetSemanticChipNameText;

#[derive(Clone, Copy, Debug)]
struct HoverTargetVisualPair {
    marker: Entity,
    label_icon: Entity,
    label_root: Entity,
    plain_text: Entity,
    semantic_inline: Entity,
    semantic_prefix: Entity,
    semantic_chip: Entity,
    semantic_chip_code_box: Entity,
    semantic_chip_code_text: Entity,
    semantic_chip_name_box: Entity,
    semantic_chip_name_text: Entity,
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
    offscreen: bool,
    marker_size_screen_px: f32,
    label_offset_screen_px: f32,
    color_rgb: [u8; 3],
    icon_kind: UiSvgIconKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SemanticIdentityKind {
    Region,
    RegionGroup,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemanticIdentityLabel {
    prefix: String,
    code: String,
    name: String,
    kind: SemanticIdentityKind,
}

#[derive(Debug, Clone, Copy)]
struct Map2dViewportBounds {
    min_x: f32,
    max_x: f32,
    min_z: f32,
    max_z: f32,
    scale: f32,
}

#[derive(SystemParam)]
struct HoverTargetSyncQueries<'w, 's> {
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    camera_q: Query<
        'w,
        's,
        (
            &'static Camera,
            &'static GlobalTransform,
            &'static Projection,
        ),
        (
            With<Map2dCamera>,
            Without<HoverTargetMarker>,
            Without<HoverTargetLabelRoot>,
            Without<HoverTargetLabelText>,
        ),
    >,
    markers: Query<
        'w,
        's,
        (
            &'static mut Transform,
            &'static mut Visibility,
            &'static mut Sprite,
        ),
        (
            With<HoverTargetMarker>,
            Without<HoverTargetLabelRoot>,
            Without<HoverTargetLabelText>,
        ),
    >,
    label_icons: Query<
        'w,
        's,
        &'static mut ImageNode,
        (
            With<HoverTargetLabelIcon>,
            Without<HoverTargetMarker>,
            Without<HoverTargetLabelRoot>,
            Without<HoverTargetLabelText>,
            Without<HoverTargetSemanticInline>,
            Without<HoverTargetSemanticPrefixText>,
            Without<HoverTargetSemanticChip>,
            Without<HoverTargetSemanticChipCodeBox>,
            Without<HoverTargetSemanticChipCodeText>,
            Without<HoverTargetSemanticChipNameBox>,
            Without<HoverTargetSemanticChipNameText>,
        ),
    >,
    label_roots: Query<
        'w,
        's,
        (
            &'static mut Node,
            &'static mut Visibility,
            &'static mut BackgroundColor,
            &'static mut BorderColor,
            &'static ComputedNode,
        ),
        (
            With<HoverTargetLabelRoot>,
            Without<HoverTargetMarker>,
            Without<HoverTargetLabelText>,
            Without<HoverTargetLabelIcon>,
            Without<HoverTargetSemanticInline>,
            Without<HoverTargetSemanticPrefixText>,
            Without<HoverTargetSemanticChip>,
            Without<HoverTargetSemanticChipCodeBox>,
            Without<HoverTargetSemanticChipCodeText>,
            Without<HoverTargetSemanticChipNameBox>,
            Without<HoverTargetSemanticChipNameText>,
        ),
    >,
    label_parts: ParamSet<
        'w,
        's,
        (
            Query<
                'w,
                's,
                (
                    &'static mut Text,
                    &'static mut TextFont,
                    &'static mut TextColor,
                ),
                (
                    With<HoverTargetLabelText>,
                    Without<HoverTargetMarker>,
                    Without<HoverTargetLabelRoot>,
                ),
            >,
            Query<
                'w,
                's,
                (
                    &'static mut Text,
                    &'static mut TextFont,
                    &'static mut TextColor,
                ),
                (
                    With<HoverTargetSemanticPrefixText>,
                    Without<HoverTargetMarker>,
                    Without<HoverTargetLabelRoot>,
                ),
            >,
            Query<
                'w,
                's,
                (&'static mut BackgroundColor, &'static mut BorderColor),
                (
                    With<HoverTargetSemanticChip>,
                    Without<HoverTargetMarker>,
                    Without<HoverTargetLabelRoot>,
                ),
            >,
            Query<
                'w,
                's,
                &'static mut BackgroundColor,
                (
                    With<HoverTargetSemanticChipCodeBox>,
                    Without<HoverTargetMarker>,
                    Without<HoverTargetLabelRoot>,
                ),
            >,
            Query<
                'w,
                's,
                (
                    &'static mut Text,
                    &'static mut TextFont,
                    &'static mut TextColor,
                ),
                (
                    With<HoverTargetSemanticChipCodeText>,
                    Without<HoverTargetMarker>,
                    Without<HoverTargetLabelRoot>,
                ),
            >,
            Query<
                'w,
                's,
                &'static mut BackgroundColor,
                (
                    With<HoverTargetSemanticChipNameBox>,
                    Without<HoverTargetMarker>,
                    Without<HoverTargetLabelRoot>,
                ),
            >,
            Query<
                'w,
                's,
                (
                    &'static mut Text,
                    &'static mut TextFont,
                    &'static mut TextColor,
                ),
                (
                    With<HoverTargetSemanticChipNameText>,
                    Without<HoverTargetMarker>,
                    Without<HoverTargetLabelRoot>,
                ),
            >,
        ),
    >,
    child_visibility: Query<
        'w,
        's,
        &'static mut Visibility,
        (Without<HoverTargetMarker>, Without<HoverTargetLabelRoot>),
    >,
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
    selection: Res<SelectionState>,
    view_mode: Res<ViewModeState>,
    #[cfg(target_arch = "wasm32")] bridge: Res<BrowserBridgeState>,
    ui_assets: (Res<AssetServer>, Res<UiFonts>),
    layer_state: (Res<LayerRegistry>, Res<LayerRuntime>),
    icon_assets: (Res<HoverTargetMarkerAssets>, Res<UiSvgIconAssets>),
    mut marker_pool: ResMut<HoverTargetMarkerPool>,
    ui_root_q: Query<Entity, With<UiRoot>>,
    mut sync: HoverTargetSyncQueries,
) {
    #[cfg(target_arch = "wasm32")]
    let active_detail_pane_id = bridge
        .input
        .ui
        .active_detail_pane_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    #[cfg(not(target_arch = "wasm32"))]
    let active_detail_pane_id: Option<&str> = None;

    let targets = effective_targets(
        view_mode.mode,
        active_detail_pane_id,
        hover.info.as_ref(),
        selection.info.as_ref(),
        &layer_state.0,
        &layer_state.1,
    );

    if targets.is_empty() {
        hide_hover_targets(&marker_pool, &mut sync);
        return;
    }

    let Some(texture) = icon_assets.0.texture.as_ref() else {
        return;
    };
    let Some(default_icon_handle) = icon_assets.1.handle(UiSvgIconKind::MapPin) else {
        return;
    };
    let Ok(ui_root) = ui_root_q.single() else {
        hide_hover_targets(&marker_pool, &mut sync);
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
        let mut label_icon = Entity::PLACEHOLDER;
        let mut plain_text = Entity::PLACEHOLDER;
        let mut semantic_inline = Entity::PLACEHOLDER;
        let mut semantic_prefix = Entity::PLACEHOLDER;
        let mut semantic_chip = Entity::PLACEHOLDER;
        let mut semantic_chip_code_box = Entity::PLACEHOLDER;
        let mut semantic_chip_code_text = Entity::PLACEHOLDER;
        let mut semantic_chip_name_box = Entity::PLACEHOLDER;
        let mut semantic_chip_name_text = Entity::PLACEHOLDER;
        let mut label_root_entity = commands.spawn((
            HoverTargetLabelRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                height: Val::Px(HOVER_CALLOUT_HEIGHT_SCREEN_PX),
                padding: UiRect {
                    left: Val::Px(HOVER_CALLOUT_PADDING_LEFT_SCREEN_PX),
                    right: Val::Px(HOVER_CALLOUT_PADDING_RIGHT_SCREEN_PX),
                    top: Val::Px(0.0),
                    bottom: Val::Px(0.0),
                },
                border: UiRect::all(Val::Px(HOVER_CALLOUT_BORDER_SCREEN_PX)),
                border_radius: BorderRadius::all(Val::Px(HOVER_CALLOUT_CORNER_RADIUS_SCREEN_PX)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(callout_panel_color),
            BorderColor::all(callout_border_color),
            GlobalZIndex(1400),
            Visibility::Hidden,
            NodeStyleSheet::new(ui_assets.0.load("/map/ui/fishystuff.css")),
            ClassList::new("marker-callout"),
        ));
        let label_root = label_root_entity
            .with_children(|parent| {
                label_icon = parent
                    .spawn((
                        HoverTargetLabelIcon,
                        ImageNode::new(default_icon_handle.clone()),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(HOVER_ICON_INSET_LEFT_SCREEN_PX),
                            top: Val::Px(HOVER_ICON_TOP_SCREEN_PX),
                            width: Val::Px(HOVER_ICON_SIZE_SCREEN_PX),
                            height: Val::Px(HOVER_ICON_SIZE_SCREEN_PX),
                            ..default()
                        },
                        Visibility::Hidden,
                    ))
                    .id();
                plain_text = parent
                    .spawn((
                        HoverTargetLabelText,
                        Text::new(""),
                        TextFont {
                            font: ui_assets.1.regular.clone(),
                            font_size: HOVER_LABEL_SIZE_PX,
                            ..default()
                        },
                        TextLayout::new_with_no_wrap().with_justify(Justify::Center),
                        TextColor(label_color),
                        Visibility::Hidden,
                        ClassList::new("marker-callout-text"),
                    ))
                    .id();
                semantic_inline = parent
                    .spawn((
                        HoverTargetSemanticInline,
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(5.0),
                            ..default()
                        },
                        Visibility::Hidden,
                        ClassList::new("marker-callout-inline"),
                    ))
                    .with_children(|inline| {
                        semantic_prefix = inline
                            .spawn((
                                HoverTargetSemanticPrefixText,
                                Text::new(""),
                                TextFont {
                                    font: ui_assets.1.regular.clone(),
                                    font_size: HOVER_LABEL_SIZE_PX,
                                    ..default()
                                },
                                TextLayout::new_with_no_wrap(),
                                TextColor(label_color),
                                Visibility::Hidden,
                                ClassList::new("marker-callout-prefix"),
                            ))
                            .id();
                        semantic_chip = inline
                            .spawn((
                                HoverTargetSemanticChip,
                                Node {
                                    display: Display::Flex,
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Stretch,
                                    overflow: Overflow::clip(),
                                    border: UiRect::all(Val::Px(1.0)),
                                    border_radius: BorderRadius::all(Val::Px(999.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                                BorderColor::all(HOVER_CHIP_REGION_GROUP_BORDER_COLOR),
                                Visibility::Visible,
                                ClassList::new("marker-callout-chip"),
                            ))
                            .with_children(|chip| {
                                semantic_chip_code_box = chip
                                    .spawn((
                                        HoverTargetSemanticChipCodeBox,
                                        Node {
                                            display: Display::Flex,
                                            align_items: AlignItems::Center,
                                            justify_content: JustifyContent::Center,
                                            padding: UiRect::axes(Val::Px(7.0), Val::Px(1.0)),
                                            border_radius: BorderRadius {
                                                top_left: Val::Px(999.0),
                                                bottom_left: Val::Px(999.0),
                                                ..default()
                                            },
                                            ..default()
                                        },
                                        BackgroundColor(HOVER_CHIP_REGION_GROUP_CODE_BG_COLOR),
                                        Visibility::Visible,
                                        ClassList::new("marker-callout-chip-code-box"),
                                    ))
                                    .with_children(|code_box| {
                                        semantic_chip_code_text = code_box
                                            .spawn((
                                                HoverTargetSemanticChipCodeText,
                                                Text::new(""),
                                                TextFont {
                                                    font: ui_assets.1.regular.clone(),
                                                    font_size: 10.0,
                                                    ..default()
                                                },
                                                TextLayout::new_with_no_wrap(),
                                                TextColor(HOVER_CHIP_REGION_GROUP_CODE_TEXT_COLOR),
                                                Visibility::Hidden,
                                                ClassList::new("marker-callout-chip-code"),
                                            ))
                                            .id();
                                    })
                                    .id();
                                semantic_chip_name_box = chip
                                    .spawn((
                                        HoverTargetSemanticChipNameBox,
                                        Node {
                                            display: Display::Flex,
                                            align_items: AlignItems::Center,
                                            padding: UiRect::axes(Val::Px(9.0), Val::Px(1.0)),
                                            border_radius: BorderRadius {
                                                top_right: Val::Px(999.0),
                                                bottom_right: Val::Px(999.0),
                                                ..default()
                                            },
                                            ..default()
                                        },
                                        BackgroundColor(HOVER_CHIP_REGION_GROUP_PANEL_COLOR),
                                        Visibility::Visible,
                                        ClassList::new("marker-callout-chip-name-box"),
                                    ))
                                    .with_children(|name_box| {
                                        semantic_chip_name_text = name_box
                                            .spawn((
                                                HoverTargetSemanticChipNameText,
                                                Text::new(""),
                                                TextFont {
                                                    font: ui_assets.1.regular.clone(),
                                                    font_size: HOVER_LABEL_SIZE_PX,
                                                    ..default()
                                                },
                                                TextLayout::new_with_no_wrap(),
                                                TextColor(label_color),
                                                Visibility::Hidden,
                                                ClassList::new("marker-callout-chip-name"),
                                            ))
                                            .id();
                                    })
                                    .id();
                            })
                            .id();
                    })
                    .id();
            })
            .id();
        commands.entity(ui_root).add_child(label_root);
        marker_pool.markers.push(HoverTargetVisualPair {
            marker,
            label_icon,
            label_root,
            plain_text,
            semantic_inline,
            semantic_prefix,
            semantic_chip,
            semantic_chip_code_box,
            semantic_chip_code_text,
            semantic_chip_name_box,
            semantic_chip_name_text,
        });
    }

    let viewport_bounds = map_2d_viewport_bounds(&sync.windows, &sync.camera_q);
    let scale = viewport_bounds
        .map(|bounds| bounds.scale)
        .unwrap_or_else(|| camera_scale(&sync.camera_q));
    for (index, target) in targets.iter().enumerate() {
        let target = viewport_bounds
            .map(|bounds| clamp_hover_target_to_viewport(target, bounds))
            .unwrap_or_else(|| target.clone());
        let pair = marker_pool.markers[index];
        if let Ok((mut transform, mut visibility, mut sprite)) = sync.markers.get_mut(pair.marker) {
            transform.translation.x = target.world_x;
            transform.translation.y = target.world_z;
            transform.translation.z = HOVER_MARKER_Z;
            sprite.image = texture.clone();
            sprite.color = color_from_rgb(target.color_rgb);
            sprite.custom_size = Some(Vec2::splat(target.marker_size_screen_px * scale));
            *visibility = Visibility::Visible;
        }
        let viewport_position = {
            let Ok((camera, camera_transform, _)) = sync.camera_q.single() else {
                hide_hover_target_label(pair, &mut sync);
                continue;
            };
            world_to_viewport(
                camera,
                camera_transform,
                Vec3::new(target.world_x, target.world_z, 0.0),
            )
        };
        let Some(viewport_position) = viewport_position else {
            hide_hover_target_label(pair, &mut sync);
            continue;
        };
        let panel_size_px = sync
            .label_roots
            .get(pair.label_root)
            .ok()
            .map(|(_, _, _, _, computed)| {
                let inv = computed.inverse_scale_factor();
                computed.size() * inv
            })
            .filter(|size| size.x > 1.0 && size.y > 1.0)
            .unwrap_or_else(|| hover_callout_size_px(&target.label));
        let (left_px, top_px) = if let Ok(window) = sync.windows.single() {
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
        if let Some(icon_handle) = icon_assets.1.handle(target.icon_kind) {
            if let Ok(mut image_node) = sync.label_icons.get_mut(pair.label_icon) {
                image_node.image = icon_handle;
                image_node.color = label_color;
            }
            if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.label_icon) {
                *visibility = Visibility::Visible;
            }
        }
        if let Ok((mut node, mut visibility, mut background, mut border, _)) =
            sync.label_roots.get_mut(pair.label_root)
        {
            node.display = Display::Flex;
            node.left = Val::Px(left_px);
            node.top = Val::Px(top_px);
            node.height = Val::Px(panel_size_px.y);
            node.border = UiRect::all(Val::Px(HOVER_CALLOUT_BORDER_SCREEN_PX));
            node.border_radius = BorderRadius::all(Val::Px(HOVER_CALLOUT_CORNER_RADIUS_SCREEN_PX));
            *background = BackgroundColor(callout_panel_color);
            *border = BorderColor::all(callout_border_color);
            *visibility = Visibility::Visible;
        }
        let semantic_identity = parse_semantic_identity_label(&target.label);
        if let Ok((mut text, mut text_font, mut text_color)) =
            sync.label_parts.p0().get_mut(pair.plain_text)
        {
            text.0 = target.label.clone();
            text_font.font = ui_assets.1.regular.clone();
            text_font.font_size = HOVER_LABEL_SIZE_PX;
            text_color.0 = label_color;
        }
        if let Some(identity) = semantic_identity.as_ref() {
            if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.plain_text) {
                *visibility = Visibility::Hidden;
            }
            if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.semantic_inline) {
                *visibility = Visibility::Visible;
            }
            if let Ok((mut text, mut text_font, mut text_color)) =
                sync.label_parts.p1().get_mut(pair.semantic_prefix)
            {
                text.0 = identity.prefix.clone();
                text_font.font = ui_assets.1.regular.clone();
                text_font.font_size = HOVER_LABEL_SIZE_PX;
                text_color.0 = label_color;
            }
            if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.semantic_prefix) {
                *visibility = if identity.prefix.is_empty() {
                    Visibility::Hidden
                } else {
                    Visibility::Visible
                };
            }
            if let Ok((mut background, mut border)) =
                sync.label_parts.p2().get_mut(pair.semantic_chip)
            {
                *background = BackgroundColor(Color::NONE);
                *border = BorderColor::all(chip_border_color(identity.kind, theme_colors));
            }
            if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.semantic_chip) {
                *visibility = Visibility::Visible;
            }
            if let Ok(mut background) = sync.label_parts.p3().get_mut(pair.semantic_chip_code_box) {
                *background = BackgroundColor(chip_code_box_color(identity.kind, theme_colors));
            }
            if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.semantic_chip_code_box) {
                *visibility = Visibility::Visible;
            }
            if let Ok((mut text, mut text_font, mut text_color)) =
                sync.label_parts.p4().get_mut(pair.semantic_chip_code_text)
            {
                text.0 = identity.code.clone();
                text_font.font = ui_assets.1.regular.clone();
                text_font.font_size = 10.0;
                text_color.0 = chip_code_text_color(identity.kind, theme_colors);
            }
            if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.semantic_chip_code_text)
            {
                *visibility = Visibility::Visible;
            }
            if let Ok(mut background) = sync.label_parts.p5().get_mut(pair.semantic_chip_name_box) {
                *background = BackgroundColor(chip_name_box_color(theme_colors));
            }
            if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.semantic_chip_name_box) {
                *visibility = if identity.name.is_empty() {
                    Visibility::Hidden
                } else {
                    Visibility::Visible
                };
            }
            if let Ok((mut text, mut text_font, mut text_color)) =
                sync.label_parts.p6().get_mut(pair.semantic_chip_name_text)
            {
                text.0 = identity.name.clone();
                text_font.font = ui_assets.1.regular.clone();
                text_font.font_size = HOVER_LABEL_SIZE_PX;
                text_color.0 = chip_name_text_color(theme_colors);
            }
            if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.semantic_chip_name_text)
            {
                *visibility = if identity.name.is_empty() {
                    Visibility::Hidden
                } else {
                    Visibility::Visible
                };
            }
            if let Ok((mut text, _, mut text_color)) =
                sync.label_parts.p0().get_mut(pair.plain_text)
            {
                text.0.clear();
                text_color.0 = label_color;
            }
        } else {
            if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.semantic_inline) {
                *visibility = Visibility::Hidden;
            }
            if let Ok((mut text, mut text_font, mut text_color)) =
                sync.label_parts.p0().get_mut(pair.plain_text)
            {
                text.0 = target.label.clone();
                text_font.font = ui_assets.1.regular.clone();
                text_font.font_size = HOVER_LABEL_SIZE_PX;
                text_color.0 = label_color;
            }
            if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.plain_text) {
                *visibility = Visibility::Visible;
            }
            if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.semantic_prefix) {
                *visibility = Visibility::Hidden;
            }
            if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.semantic_chip) {
                *visibility = Visibility::Hidden;
            }
            if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.semantic_chip_code_box) {
                *visibility = Visibility::Hidden;
            }
            if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.semantic_chip_code_text)
            {
                *visibility = Visibility::Hidden;
            }
            if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.semantic_chip_name_box) {
                *visibility = Visibility::Hidden;
            }
            if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.semantic_chip_name_text)
            {
                *visibility = Visibility::Hidden;
            }
        }
    }

    for pair in marker_pool.markers.iter().skip(targets.len()) {
        if let Ok((_, mut visibility, _)) = sync.markers.get_mut(pair.marker) {
            *visibility = Visibility::Hidden;
        }
        hide_hover_target_label(*pair, &mut sync);
    }
}

fn hide_hover_targets(marker_pool: &HoverTargetMarkerPool, sync: &mut HoverTargetSyncQueries) {
    for pair in &marker_pool.markers {
        if let Ok((_, mut visibility, _)) = sync.markers.get_mut(pair.marker) {
            *visibility = Visibility::Hidden;
        }
        hide_hover_target_label(*pair, sync);
    }
}

fn hide_hover_target_label(pair: HoverTargetVisualPair, sync: &mut HoverTargetSyncQueries) {
    if let Ok((mut node, mut visibility, _, _, _)) = sync.label_roots.get_mut(pair.label_root) {
        node.display = Display::None;
        *visibility = Visibility::Hidden;
    }
    if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.label_icon) {
        *visibility = Visibility::Hidden;
    }
    if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.plain_text) {
        *visibility = Visibility::Hidden;
    }
    if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.semantic_inline) {
        *visibility = Visibility::Hidden;
    }
    if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.semantic_prefix) {
        *visibility = Visibility::Hidden;
    }
    if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.semantic_chip) {
        *visibility = Visibility::Hidden;
    }
    if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.semantic_chip_code_box) {
        *visibility = Visibility::Hidden;
    }
    if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.semantic_chip_code_text) {
        *visibility = Visibility::Hidden;
    }
    if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.semantic_chip_name_box) {
        *visibility = Visibility::Hidden;
    }
    if let Ok(mut visibility) = sync.child_visibility.get_mut(pair.semantic_chip_name_text) {
        *visibility = Visibility::Hidden;
    }
}

fn hover_target_border_color(colors: &FishyMapThemeColors) -> Option<Color> {
    colors
        .base200
        .as_deref()
        .or(colors.base300.as_deref())
        .and_then(parse_css_color)
}

fn hover_target_panel_color(colors: &FishyMapThemeColors) -> Option<Color> {
    colors
        .base100
        .as_deref()
        .or(colors.base200.as_deref())
        .and_then(parse_css_color)
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

    targets_from_samples(&info.layer_samples, layer_registry, layer_runtime)
}

fn effective_targets(
    view_mode: ViewMode,
    active_detail_pane_id: Option<&str>,
    hover_info: Option<&HoverInfo>,
    selection_info: Option<&SelectedInfo>,
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
) -> Vec<HoverTargetVisual> {
    if view_mode != ViewMode::Map2D {
        return Vec::new();
    }

    match active_detail_pane_id {
        Some(TERRITORY_DETAIL_PANE_ID) => {
            selection_targets_from_info(selection_info, layer_registry, layer_runtime)
        }
        Some(_) => Vec::new(),
        None => hover_targets_from_info(hover_info, layer_registry, layer_runtime),
    }
}

fn selection_targets_from_info(
    info: Option<&SelectedInfo>,
    _layer_registry: &LayerRegistry,
    _layer_runtime: &LayerRuntime,
) -> Vec<HoverTargetVisual> {
    let Some(info) = info else {
        return Vec::new();
    };

    info.layer_samples
        .iter()
        .flat_map(hover_targets_from_sample)
        .collect()
}

fn targets_from_samples(
    samples: &[crate::map::layer_query::LayerQuerySample],
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
) -> Vec<HoverTargetVisual> {
    samples
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
    sample: &crate::map::layer_query::LayerQuerySample,
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
    let (marker_size_screen_px, label_offset_screen_px, color_rgb, icon_kind) =
        match target.key.as_str() {
            "resource_node" => (
                RESOURCE_BAR_MARKER_SIZE_SCREEN_PX,
                RESOURCE_BAR_LABEL_OFFSET_SCREEN_PX,
                RESOURCE_BAR_MARKER_COLOR,
                UiSvgIconKind::HoverResources,
            ),
            "region_node" => (
                REGION_NODE_MARKER_SIZE_SCREEN_PX,
                REGION_NODE_LABEL_OFFSET_SCREEN_PX,
                REGION_NODE_MARKER_COLOR,
                UiSvgIconKind::MapPin,
            ),
            "origin_node" => (
                ORIGIN_NODE_MARKER_SIZE_SCREEN_PX,
                ORIGIN_NODE_LABEL_OFFSET_SCREEN_PX,
                ORIGIN_NODE_MARKER_COLOR,
                UiSvgIconKind::TradeOrigin,
            ),
            _ => return None,
        };
    Some(HoverTargetVisual {
        world_x: target.world_x as f32,
        world_z: target.world_z as f32,
        label: target.label.clone(),
        offscreen: false,
        marker_size_screen_px,
        label_offset_screen_px,
        color_rgb,
        icon_kind,
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
    next.offscreen = (clamped_x - target.world_x).abs() > f32::EPSILON
        || (clamped_z - target.world_z).abs() > f32::EPSILON;
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
    let content_width_px = parse_semantic_identity_label(display_text)
        .map(semantic_identity_width_px)
        .unwrap_or_else(|| plain_label_width_px(display_text.trim()));
    let width_px = (content_width_px
        + HOVER_CALLOUT_PADDING_LEFT_SCREEN_PX
        + HOVER_CALLOUT_PADDING_RIGHT_SCREEN_PX
        + HOVER_CALLOUT_BORDER_SCREEN_PX * 2.0)
        .max(HOVER_CALLOUT_MIN_WIDTH_SCREEN_PX);
    Vec2::new(width_px, HOVER_CALLOUT_HEIGHT_SCREEN_PX)
}

fn plain_label_width_px(text: &str) -> f32 {
    text.chars().count() as f32 * HOVER_LABEL_SIZE_PX * HOVER_PLAIN_TEXT_WIDTH_FACTOR
        + HOVER_PLAIN_TEXT_WIDTH_SLACK_SCREEN_PX
}

fn semantic_identity_width_px(identity: SemanticIdentityLabel) -> f32 {
    let prefix_width = if identity.prefix.is_empty() {
        0.0
    } else {
        identity.prefix.chars().count() as f32
            * HOVER_LABEL_SIZE_PX
            * HOVER_PREFIX_TEXT_WIDTH_FACTOR
    };
    let code_width = identity.code.chars().count() as f32 * 10.0 * HOVER_CODE_TEXT_WIDTH_FACTOR
        + HOVER_CHIP_CODE_PADDING_X_SCREEN_PX * 2.0;
    let name_width = if identity.name.is_empty() {
        0.0
    } else {
        identity.name.chars().count() as f32 * HOVER_LABEL_SIZE_PX * HOVER_NAME_TEXT_WIDTH_FACTOR
            + HOVER_CHIP_NAME_PADDING_X_SCREEN_PX * 2.0
    };
    let gap_width = if prefix_width > 0.0 {
        HOVER_SEMANTIC_GAP_SCREEN_PX
    } else {
        0.0
    };
    prefix_width + gap_width + code_width + name_width + 2.0 + HOVER_SEMANTIC_WIDTH_SLACK_SCREEN_PX
}

fn color_from_rgb([red, green, blue]: [u8; 3]) -> Color {
    Color::srgb_u8(red, green, blue)
}

fn parse_semantic_identity_label(value: &str) -> Option<SemanticIdentityLabel> {
    let trimmed = value.trim();
    let (head, raw_code) = trimmed.rsplit_once(" (")?;
    let code = raw_code.strip_suffix(')')?.trim();
    let kind = if code.starts_with("RG") {
        SemanticIdentityKind::RegionGroup
    } else if code.starts_with('R') {
        SemanticIdentityKind::Region
    } else {
        return None;
    };
    let (prefix, raw_name) = head
        .split_once(':')
        .map(|(prefix, name)| (prefix.trim(), name.trim()))
        .unwrap_or(("", head.trim()));
    Some(SemanticIdentityLabel {
        prefix: prefix.to_string(),
        code: code.to_string(),
        name: raw_name.to_string(),
        kind,
    })
}

fn chip_border_color(kind: SemanticIdentityKind, colors: Option<&FishyMapThemeColors>) -> Color {
    match kind {
        SemanticIdentityKind::RegionGroup => colors
            .and_then(|colors| colors.info.as_deref())
            .and_then(parse_css_color)
            .unwrap_or(HOVER_CHIP_REGION_GROUP_BORDER_COLOR),
        SemanticIdentityKind::Region => colors
            .and_then(|colors| colors.warning.as_deref())
            .and_then(parse_css_color)
            .unwrap_or(HOVER_CHIP_REGION_BORDER_COLOR),
    }
}

fn chip_code_box_color(kind: SemanticIdentityKind, colors: Option<&FishyMapThemeColors>) -> Color {
    match kind {
        SemanticIdentityKind::RegionGroup => colors
            .and_then(|colors| colors.info.as_deref())
            .and_then(parse_css_color)
            .unwrap_or(HOVER_CHIP_REGION_GROUP_CODE_BG_COLOR),
        SemanticIdentityKind::Region => colors
            .and_then(|colors| colors.warning.as_deref())
            .and_then(parse_css_color)
            .unwrap_or(HOVER_CHIP_REGION_CODE_BG_COLOR),
    }
}

fn chip_code_text_color(kind: SemanticIdentityKind, colors: Option<&FishyMapThemeColors>) -> Color {
    match kind {
        SemanticIdentityKind::RegionGroup => colors
            .and_then(|colors| colors.info_content.as_deref())
            .and_then(parse_css_color)
            .unwrap_or(HOVER_CHIP_REGION_GROUP_CODE_TEXT_COLOR),
        SemanticIdentityKind::Region => colors
            .and_then(|colors| colors.warning_content.as_deref())
            .and_then(parse_css_color)
            .unwrap_or(HOVER_CHIP_REGION_CODE_TEXT_COLOR),
    }
}

fn chip_name_box_color(colors: Option<&FishyMapThemeColors>) -> Color {
    colors
        .and_then(|colors| colors.base100.as_deref())
        .and_then(parse_css_color)
        .unwrap_or(HOVER_CHIP_REGION_GROUP_PANEL_COLOR)
}

fn chip_name_text_color(colors: Option<&FishyMapThemeColors>) -> Color {
    colors
        .and_then(|colors| colors.base_content.as_deref())
        .and_then(parse_css_color)
        .unwrap_or(HOVER_CHIP_REGION_GROUP_NAME_TEXT_COLOR)
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
        clamp_hover_target_to_viewport, effective_targets, hover_targets_from_info,
        parse_semantic_identity_label, selection_targets_from_info, HoverTargetVisual,
        Map2dViewportBounds, SemanticIdentityKind, ORIGIN_NODE_LABEL_OFFSET_SCREEN_PX,
        ORIGIN_NODE_MARKER_COLOR, ORIGIN_NODE_MARKER_SIZE_SCREEN_PX,
        REGION_NODE_LABEL_OFFSET_SCREEN_PX, REGION_NODE_MARKER_COLOR,
        REGION_NODE_MARKER_SIZE_SCREEN_PX, RESOURCE_BAR_LABEL_OFFSET_SCREEN_PX,
        RESOURCE_BAR_MARKER_COLOR, RESOURCE_BAR_MARKER_SIZE_SCREEN_PX, TERRITORY_DETAIL_PANE_ID,
    };
    use crate::map::camera::mode::ViewMode;
    use crate::map::layer_query::LayerQuerySample;
    use crate::map::layers::{LayerRegistry, LayerRuntime};
    use crate::plugins::api::{HoverInfo, SelectedInfo};
    use crate::plugins::svg_icons::UiSvgIconKind;
    use fishystuff_api::models::layers::{
        GeometrySpace, LayerDescriptor, LayerKind as LayerKindDto, LayerTransformDto, LayerUiInfo,
        LayersResponse, LodPolicyDto, StyleMode, TilesetRef, VectorSourceRef,
    };
    use fishystuff_api::Rgb;
    use fishystuff_core::field_metadata::FieldHoverTarget;

    fn sample(layer_id: &str) -> LayerQuerySample {
        LayerQuerySample {
            layer_id: layer_id.to_string(),
            layer_name: layer_id.to_string(),
            kind: "vector-geojson".to_string(),
            rgb: Rgb::new(0, 0, 0),
            rgb_u32: 0,
            field_id: None,
            targets: Vec::new(),
            detail_pane: None,
            detail_sections: Vec::new(),
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
            filter_bindings: Vec::new(),
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
            world_x: 0.0,
            world_z: 0.0,
            layer_samples: vec![region_group, regions],
            point_samples: Vec::new(),
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
                    offscreen: false,
                    marker_size_screen_px: RESOURCE_BAR_MARKER_SIZE_SCREEN_PX,
                    label_offset_screen_px: RESOURCE_BAR_LABEL_OFFSET_SCREEN_PX,
                    color_rgb: RESOURCE_BAR_MARKER_COLOR,
                    icon_kind: UiSvgIconKind::HoverResources,
                },
                HoverTargetVisual {
                    world_x: 789.0,
                    world_z: 321.0,
                    label: "Origin: Tarif".to_string(),
                    offscreen: false,
                    marker_size_screen_px: ORIGIN_NODE_MARKER_SIZE_SCREEN_PX,
                    label_offset_screen_px: ORIGIN_NODE_LABEL_OFFSET_SCREEN_PX,
                    color_rgb: ORIGIN_NODE_MARKER_COLOR,
                    icon_kind: UiSvgIconKind::TradeOrigin,
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
            world_x: 0.0,
            world_z: 0.0,
            layer_samples: vec![region_group, region],
            point_samples: Vec::new(),
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
    fn selection_targets_follow_selected_layer_samples() {
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

        let info = SelectedInfo {
            map_px: 0,
            map_py: 0,
            world_x: 0.0,
            world_z: 0.0,
            layer_samples: vec![region_group, region],
            sampled_world_point: true,
            point_kind: None,
            point_label: None,
            point_samples: Vec::new(),
        };

        let layer_registry = LayerRegistry::default();
        let layer_runtime = LayerRuntime::default();
        let targets = selection_targets_from_info(Some(&info), &layer_registry, &layer_runtime);
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
            world_x: 0.0,
            world_z: 0.0,
            layer_samples: vec![region_group.clone(), region.clone()],
            point_samples: Vec::new(),
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
                offscreen: false,
                marker_size_screen_px: RESOURCE_BAR_MARKER_SIZE_SCREEN_PX,
                label_offset_screen_px: RESOURCE_BAR_LABEL_OFFSET_SCREEN_PX,
                color_rgb: RESOURCE_BAR_MARKER_COLOR,
                icon_kind: UiSvgIconKind::HoverResources,
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
                offscreen: false,
                marker_size_screen_px: ORIGIN_NODE_MARKER_SIZE_SCREEN_PX,
                label_offset_screen_px: ORIGIN_NODE_LABEL_OFFSET_SCREEN_PX,
                color_rgb: ORIGIN_NODE_MARKER_COLOR,
                icon_kind: UiSvgIconKind::TradeOrigin,
            }]
        );
    }

    #[test]
    fn selection_targets_ignore_layer_visibility() {
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

        let info = SelectedInfo {
            map_px: 0,
            map_py: 0,
            world_x: 0.0,
            world_z: 0.0,
            sampled_world_point: true,
            point_kind: None,
            point_label: None,
            layer_samples: vec![region_group, region],
            point_samples: Vec::new(),
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
        layer_runtime.set_visible(region_groups_id, false);
        layer_runtime.set_visible(regions_id, false);

        let targets = selection_targets_from_info(Some(&info), &layer_registry, &layer_runtime);
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].world_x, 10.0);
        assert_eq!(targets[1].world_x, 30.0);
    }

    #[test]
    fn non_territory_detail_panes_hide_territory_targets() {
        let mut region_group = sample("region_groups");
        region_group.targets.push(FieldHoverTarget {
            key: "resource_node".to_string(),
            label: "Resources: Tarif".to_string(),
            world_x: 10.0,
            world_z: 20.0,
        });

        let hover_info = HoverInfo {
            map_px: 0,
            map_py: 0,
            world_x: 0.0,
            world_z: 0.0,
            layer_samples: vec![region_group.clone()],
            point_samples: Vec::new(),
        };

        let selection_info = SelectedInfo {
            map_px: 0,
            map_py: 0,
            world_x: 0.0,
            world_z: 0.0,
            sampled_world_point: true,
            point_kind: None,
            point_label: None,
            layer_samples: vec![region_group],
            point_samples: Vec::new(),
        };

        let (layer_registry, layer_runtime) = hover_layer_state();

        assert!(effective_targets(
            ViewMode::Map2D,
            Some("zone_mask"),
            Some(&hover_info),
            Some(&selection_info),
            &layer_registry,
            &layer_runtime,
        )
        .is_empty());

        assert_eq!(
            effective_targets(
                ViewMode::Map2D,
                Some(TERRITORY_DETAIL_PANE_ID),
                Some(&hover_info),
                Some(&selection_info),
                &layer_registry,
                &layer_runtime,
            )
            .len(),
            1
        );
    }

    #[test]
    fn clamp_hover_target_to_viewport_marks_offscreen_targets() {
        let target = HoverTargetVisual {
            world_x: 500.0,
            world_z: 800.0,
            label: "Origin: Tarif".to_string(),
            offscreen: false,
            marker_size_screen_px: ORIGIN_NODE_MARKER_SIZE_SCREEN_PX,
            label_offset_screen_px: ORIGIN_NODE_LABEL_OFFSET_SCREEN_PX,
            color_rgb: ORIGIN_NODE_MARKER_COLOR,
            icon_kind: UiSvgIconKind::TradeOrigin,
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
        assert_eq!(clamped.label, "Origin: Tarif");
        assert!(clamped.offscreen);
    }

    #[test]
    fn parse_semantic_identity_label_reads_prefix_kind_and_name() {
        let parsed = parse_semantic_identity_label("Origin: Tarif (R216)").expect("semantic chip");
        assert_eq!(parsed.prefix, "Origin");
        assert_eq!(parsed.code, "R216");
        assert_eq!(parsed.name, "Tarif");
        assert_eq!(parsed.kind, SemanticIdentityKind::Region);

        let parsed = parse_semantic_identity_label("Resources: Tarif (RG58)")
            .expect("resource semantic chip");
        assert_eq!(parsed.prefix, "Resources");
        assert_eq!(parsed.code, "RG58");
        assert_eq!(parsed.name, "Tarif");
        assert_eq!(parsed.kind, SemanticIdentityKind::RegionGroup);
    }

    #[test]
    fn hover_targets_include_region_node_targets() {
        let sample = crate::map::layer_query::LayerQuerySample {
            layer_id: "region_groups".to_string(),
            layer_name: "Region Groups".to_string(),
            kind: "field".to_string(),
            rgb: fishystuff_api::Rgb { r: 0, g: 0, b: 0 },
            rgb_u32: 0,
            field_id: Some(58),
            detail_pane: None,
            detail_sections: Vec::new(),
            targets: vec![FieldHoverTarget {
                key: "region_node".to_string(),
                label: "Node: Kasula Farm (R213)".to_string(),
                world_x: 1.0,
                world_z: 2.0,
            }],
        };

        assert_eq!(
            super::hover_targets_from_sample(&sample),
            vec![HoverTargetVisual {
                world_x: 1.0,
                world_z: 2.0,
                label: "Node: Kasula Farm (R213)".to_string(),
                offscreen: false,
                marker_size_screen_px: REGION_NODE_MARKER_SIZE_SCREEN_PX,
                label_offset_screen_px: REGION_NODE_LABEL_OFFSET_SCREEN_PX,
                color_rgb: REGION_NODE_MARKER_COLOR,
                icon_kind: UiSvgIconKind::MapPin,
            }]
        );
    }
}
