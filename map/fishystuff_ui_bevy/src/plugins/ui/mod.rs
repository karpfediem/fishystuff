use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::image::Image;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::input::{ButtonInput, ButtonState};
use bevy::text::{Font, TextColor, TextFont};
use bevy::ui::{
    FocusPolicy, GlobalZIndex, OverflowAxis, RelativeCursorPosition, UiTargetCamera, ZIndex,
};
use bevy::window::PrimaryWindow;
use bevy_flair::prelude::*;

use crate::map::layers::{LayerRegistry, LayerSettings, PickMode};
use crate::plugins::api::{
    fish_item_icon_url, remote_image_handle, FishCatalog, FishFilterState, MapDisplayState, Patch,
    PatchFilterState, RemoteImageCache, RemoteImageEpoch, RemoteImageStatus, SelectionState,
    POINT_ICON_SCALE_MAX, POINT_ICON_SCALE_MIN,
};
use crate::plugins::camera::UiCamera;
use crate::plugins::render_domain::{ui_layers, UiRenderEntity};
use crate::prelude::*;

mod panel;
mod patches;
mod scroll;
mod search;
mod setup;
mod toggles;

#[cfg(target_arch = "wasm32")]
pub(crate) use patches::patch_index_for_timestamp;

const AUTOCOMPLETE_MAX: usize = 8;
const ZONE_MENU_WIDTH: f32 = 360.0;
const ZONE_MENU_HEIGHT: f32 = 680.0;
const PATCH_MENU_WIDTH: f32 = 240.0;
const PATCH_MENU_HEIGHT: f32 = 180.0;
const PATCH_MENU_RIGHT: f32 = 16.0;
const PATCH_MENU_BOTTOM: f32 = 540.0;
const PATCH_DROPDOWN_OPEN_HEIGHT: f32 = 140.0;
const SCROLL_LINE_HEIGHT: f32 = 7.0;
const SCROLL_PIXEL_MULTIPLIER: f32 = 0.35;
const EVIDENCE_SCROLLBAR_MIN_THUMB: f32 = 18.0;
const EVIDENCE_LIST_ROW_GAP: f32 = 6.0;
const EVIDENCE_ROW_HEIGHT_ESTIMATE: f32 = 38.0;
const EVIDENCE_SCROLL_PADDING_Y: f32 = 12.0;
const AUTOCOMPLETE_DROPDOWN_MAX_HEIGHT: f32 = 172.0;
const AUTOCOMPLETE_SCROLLBAR_MIN_THUMB: f32 = 14.0;
const AUTOCOMPLETE_LIST_ROW_GAP: f32 = 4.0;
const AUTOCOMPLETE_ROW_HEIGHT_ESTIMATE: f32 = 26.0;
const AUTOCOMPLETE_SCROLL_PADDING_Y: f32 = 0.0;
const PATCH_DROPDOWN_SCROLLBAR_MIN_THUMB: f32 = 14.0;
const PATCH_DROPDOWN_LIST_ROW_GAP: f32 = 2.0;
const PATCH_DROPDOWN_ROW_HEIGHT_ESTIMATE: f32 = 26.0;
const PATCH_DROPDOWN_SCROLL_PADDING_Y: f32 = 0.0;
const POINT_ICON_SIZE_SLIDER_MIN_THUMB: f32 = 14.0;
const LEGACY_PATCH_UI_ENABLED: bool = false;

#[derive(Component)]
struct UiRoot;

#[derive(Component)]
struct PanelRoot;

#[derive(Component)]
struct PanelTitleText;

#[derive(Component)]
struct SelectionSummaryText;

#[derive(Component)]
struct SelectionOverviewList;

#[derive(Component)]
struct ZoneEvidenceScroll;

#[derive(Component)]
struct ZoneEvidenceList;

#[derive(Component)]
struct ZoneEvidenceScrollbarTrack;

#[derive(Component)]
struct ZoneEvidenceScrollbarThumb;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum PatchBound {
    From,
    To,
}

#[derive(Component)]
struct PatchRangeButton {
    bound: PatchBound,
}

#[derive(Component)]
struct PatchRangeButtonText {
    bound: PatchBound,
}

#[derive(Component)]
struct PatchDropdownList {
    bound: PatchBound,
}

#[derive(Component)]
struct PatchDropdownScrollbarTrack {
    bound: PatchBound,
}

#[derive(Component)]
struct PatchDropdownScrollbarThumb {
    bound: PatchBound,
}

#[derive(Component)]
struct PatchEntry {
    bound: PatchBound,
    patch_id: String,
}

#[derive(Component)]
struct PointIconSizeSliderTrack;

#[derive(Component)]
struct PointIconSizeSliderThumb;

#[derive(Component)]
struct PointIconSizeValueText;

#[derive(Component, Default)]
struct ToggleEffort;

#[derive(Component, Default)]
struct TogglePoints;

#[derive(Component, Default)]
struct ToggleDrift;

#[derive(Component, Default)]
struct ToggleZoneMask;

#[derive(Component)]
struct MaskOpacityDown;

#[derive(Component)]
struct MaskOpacityUp;

#[derive(Component)]
struct MaskOpacityText;

#[derive(Component)]
struct FishSearchInput;

#[derive(Component)]
struct FishSearchText;

#[derive(Component)]
struct FishSearchTags;

#[derive(Component)]
struct FishSearchTag {
    fish_id: i32,
}

#[derive(Component)]
struct FishAutocompleteFrame;

#[derive(Component)]
struct FishAutocompleteScroll;

#[derive(Component)]
struct FishAutocompleteList;

#[derive(Component)]
struct FishAutocompleteEntry {
    idx: usize,
}

#[derive(Component)]
struct FishAutocompleteEntryText;

#[derive(Component)]
struct FishAutocompleteEntryIcon;

#[derive(Component)]
struct FishAutocompleteScrollbarTrack;

#[derive(Component)]
struct FishAutocompleteScrollbarThumb;

#[derive(Resource, Default)]
struct FocusedInput {
    entity: Option<Entity>,
}

#[derive(Resource, Default)]
struct SearchState {
    query: String,
    results: Vec<usize>,
    selected: usize,
    open: bool,
    selected_fish_ids: Vec<i32>,
}

#[derive(Resource, Default)]
struct PatchDropdownState {
    open: Option<PatchBound>,
    from_patch_id: Option<String>,
    to_patch_id: Option<String>,
    last_hash: u64,
}

#[derive(Resource, Default)]
struct ZoneEvidenceScrollbarDragState {
    active: bool,
    grab_offset_px: f32,
}

#[derive(Resource, Default)]
struct FishAutocompleteScrollbarDragState {
    active: bool,
    grab_offset_px: f32,
}

#[derive(Resource, Default)]
struct PatchDropdownScrollbarDragState {
    active_bound: Option<PatchBound>,
    grab_offset_px: f32,
}

#[derive(Resource, Default)]
struct PointIconSizeSliderDragState {
    active: bool,
    grab_offset_px: f32,
}

#[derive(Clone)]
struct UiTextStyle {
    font: Handle<Font>,
    size: f32,
    color: Color,
}

#[derive(Bundle, Clone)]
struct UiTextBundle {
    text: Text,
    font: TextFont,
    color: TextColor,
}

impl UiTextBundle {
    fn new(text: impl Into<String>, style: &UiTextStyle) -> Self {
        Self {
            text: Text::new(text),
            font: TextFont {
                font: style.font.clone(),
                font_size: style.size,
                ..default()
            },
            color: TextColor(style.color),
        }
    }
}

fn fish_icon_handle(item_id: i32, remote_images: &mut RemoteImageCache) -> Option<Handle<Image>> {
    let url = fish_item_icon_url(item_id)?;
    match remote_image_handle(&url, remote_images) {
        RemoteImageStatus::Ready(handle) => Some(handle),
        RemoteImageStatus::Pending | RemoteImageStatus::Failed(_) => None,
    }
}

pub struct UiPlugin;

#[derive(Component, Debug, Default)]
pub struct UiPointerBlocker;

#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct UiPointerCapture {
    pub blocked: bool,
    pub text_input_active: bool,
}

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiFonts>()
            .init_resource::<UiPointerCapture>()
            .init_resource::<FocusedInput>()
            .init_resource::<SearchState>()
            .init_resource::<PatchDropdownState>()
            .init_resource::<ZoneEvidenceScrollbarDragState>()
            .init_resource::<FishAutocompleteScrollbarDragState>()
            .init_resource::<PatchDropdownScrollbarDragState>()
            .init_resource::<PointIconSizeSliderDragState>()
            .add_systems(Startup, setup::load_fonts.in_set(UiStartupSet))
            .add_systems(Startup, setup::setup_ui.after(UiStartupSet))
            .add_systems(
                Update,
                (
                    tag_new_ui_entities_with_ui_layer,
                    bind_ui_roots_to_ui_camera,
                    scroll::handle_ui_scroll_wheel,
                    patches::sync_patch_defaults,
                    patches::handle_patch_dropdown_toggle,
                    patches::sync_patch_dropdown_visibility,
                    patches::sync_patch_list,
                    patches::handle_patch_dropdown_scrollbar_drag,
                    patches::sync_patch_dropdown_scrollbar,
                    patches::handle_patch_entry_click,
                    patches::update_patch_button_texts,
                    patches::sync_patch_entry_selection,
                    patches::handle_point_icon_size_slider_drag,
                    patches::sync_point_icon_size_slider,
                    patches::sync_point_icon_size_text,
                ),
            )
            .add_systems(
                Update,
                (
                    toggles::handle_toggle_buttons,
                    toggles::sync_toggle_visuals,
                    toggles::handle_mask_opacity_buttons,
                    toggles::sync_mask_opacity_text,
                    search::handle_search_focus,
                    search::handle_text_input,
                    search::refresh_search_results,
                    search::handle_autocomplete_click,
                    search::update_search_text,
                    search::update_autocomplete_ui,
                    panel::update_panel_title,
                    panel::update_selected_text,
                    panel::sync_selection_overview_list,
                    panel::sync_zone_evidence_list,
                    scroll::handle_autocomplete_scrollbar_drag,
                    scroll::sync_autocomplete_scrollbar,
                    scroll::handle_zone_evidence_scrollbar_drag,
                    scroll::sync_zone_evidence_scrollbar,
                    search::sync_ui_input_capture_state,
                ),
            )
            .add_systems(
                Update,
                (search::handle_search_tag_click, search::sync_search_tags),
            );
    }
}

fn tag_new_ui_entities_with_ui_layer(mut commands: Commands, entities: NewUiEntityQuery<'_, '_>) {
    for entity in &entities {
        commands
            .entity(entity)
            .queue_silenced(|mut entity: EntityWorldMut| {
                entity.insert((UiRenderEntity, ui_layers()));
            });
    }
}

fn bind_ui_roots_to_ui_camera(
    mut commands: Commands,
    ui_camera_q: Query<Entity, With<UiCamera>>,
    ui_roots_q: Query<(Entity, Option<&UiTargetCamera>), With<UiRoot>>,
) {
    let Ok(ui_camera) = ui_camera_q.single() else {
        return;
    };
    for (entity, current_target) in &ui_roots_q {
        let already_bound = current_target
            .map(UiTargetCamera::entity)
            .map(|entity| entity == ui_camera)
            .unwrap_or(false);
        if !already_bound {
            commands
                .entity(entity)
                .queue_silenced(move |mut entity: EntityWorldMut| {
                    entity.insert(UiTargetCamera(ui_camera));
                });
        }
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct UiStartupSet;

type NewUiEntityQuery<'w, 's> = Query<
    'w,
    's,
    Entity,
    (
        Or<(Added<Node>, Added<Button>, Added<Text>, Added<ImageNode>)>,
        Without<UiRenderEntity>,
    ),
>;

#[derive(Resource, Clone, Default)]
pub struct UiFonts {
    pub regular: Handle<Font>,
}
