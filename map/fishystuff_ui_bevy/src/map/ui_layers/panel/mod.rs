use super::*;
use bevy::ecs::system::SystemParam;

mod debug_row;
mod layer_rows;
mod view_rows;

pub(super) fn setup_layer_ui(mut commands: Commands, resources: LayerPanelResources<'_, '_>) {
    spawn_layer_panel(&mut commands, resources.context());
}

pub(super) fn rebuild_layer_ui_on_registry_change(
    mut commands: Commands,
    resources: LayerPanelResources<'_, '_>,
    panel_q: Query<Entity, With<LayerPanel>>,
) {
    if !resources.registry.is_changed() {
        return;
    }
    for entity in &panel_q {
        commands.entity(entity).despawn();
    }
    spawn_layer_panel(&mut commands, resources.context());
}

fn spawn_layer_panel(commands: &mut Commands, context: LayerPanelContext<'_>) {
    let LayerPanelContext {
        registry,
        fonts,
        display_state,
        view_mode,
        terrain_cfg,
        tile_debug,
        asset_server,
    } = context;
    let font = fonts.regular.clone();
    commands
        .spawn((
            LayerPanel,
            UiPointerBlocker,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                bottom: Val::Px(16.0),
                width: Val::Px(LAYER_MENU_WIDTH),
                height: Val::Px(LAYER_MENU_HEIGHT),
                display: Display::None,
                min_width: Val::Px(LAYER_MENU_WIDTH),
                max_width: Val::Px(LAYER_MENU_WIDTH),
                min_height: Val::Px(LAYER_MENU_HEIGHT),
                max_height: Val::Px(LAYER_MENU_HEIGHT),
                padding: UiRect::all(Val::Px(10.0)),
                row_gap: Val::Px(8.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                overflow: Overflow::clip_y(),
                ..default()
            },
            FocusPolicy::Block,
            Visibility::Hidden,
            NodeStyleSheet::new(asset_server.load("/map/ui/fishystuff.css")),
            GlobalZIndex(1200),
            ClassList::new("panel layers-panel"),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("Layers"),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.95, 0.98)),
                ClassList::new("panel-title"),
            ));

            view_rows::spawn_view_rows(panel, display_state, view_mode, terrain_cfg, &font);
            layer_rows::spawn_layer_rows(panel, registry, &font);
            debug_row::spawn_debug_controls(panel, tile_debug, &font);
        });
}

#[derive(SystemParam)]
pub(super) struct LayerPanelResources<'w, 's> {
    registry: Res<'w, LayerRegistry>,
    fonts: Res<'w, UiFonts>,
    display_state: Res<'w, MapDisplayState>,
    view_mode: Res<'w, ViewModeState>,
    terrain_cfg: Res<'w, Terrain3dConfig>,
    tile_debug: Res<'w, TileDebugControls>,
    asset_server: Res<'w, AssetServer>,
    _marker: std::marker::PhantomData<&'s ()>,
}

impl<'w, 's> LayerPanelResources<'w, 's> {
    fn context(&self) -> LayerPanelContext<'_> {
        LayerPanelContext {
            registry: &self.registry,
            fonts: &self.fonts,
            display_state: &self.display_state,
            view_mode: &self.view_mode,
            terrain_cfg: &self.terrain_cfg,
            tile_debug: &self.tile_debug,
            asset_server: &self.asset_server,
        }
    }
}

#[derive(Clone, Copy)]
struct LayerPanelContext<'a> {
    registry: &'a LayerRegistry,
    fonts: &'a UiFonts,
    display_state: &'a MapDisplayState,
    view_mode: &'a ViewModeState,
    terrain_cfg: &'a Terrain3dConfig,
    tile_debug: &'a TileDebugControls,
    asset_server: &'a AssetServer,
}
