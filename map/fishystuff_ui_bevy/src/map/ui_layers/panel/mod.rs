use super::*;

mod debug_row;
mod layer_rows;
mod view_rows;

pub(super) fn setup_layer_ui(
    mut commands: Commands,
    registry: Res<LayerRegistry>,
    fonts: Res<UiFonts>,
    display_state: Res<MapDisplayState>,
    view_mode: Res<ViewModeState>,
    terrain_cfg: Res<Terrain3dConfig>,
    tile_debug: Res<TileDebugControls>,
    asset_server: Res<AssetServer>,
) {
    spawn_layer_panel(
        &mut commands,
        &registry,
        &fonts,
        &display_state,
        &view_mode,
        &terrain_cfg,
        &tile_debug,
        &asset_server,
    );
}

pub(super) fn rebuild_layer_ui_on_registry_change(
    mut commands: Commands,
    registry: Res<LayerRegistry>,
    fonts: Res<UiFonts>,
    display_state: Res<MapDisplayState>,
    view_mode: Res<ViewModeState>,
    terrain_cfg: Res<Terrain3dConfig>,
    tile_debug: Res<TileDebugControls>,
    asset_server: Res<AssetServer>,
    panel_q: Query<Entity, With<LayerPanel>>,
) {
    if !registry.is_changed() {
        return;
    }
    for entity in &panel_q {
        commands.entity(entity).despawn();
    }
    spawn_layer_panel(
        &mut commands,
        &registry,
        &fonts,
        &display_state,
        &view_mode,
        &terrain_cfg,
        &tile_debug,
        &asset_server,
    );
}

fn spawn_layer_panel(
    commands: &mut Commands,
    registry: &LayerRegistry,
    fonts: &UiFonts,
    display_state: &MapDisplayState,
    view_mode: &ViewModeState,
    terrain_cfg: &Terrain3dConfig,
    tile_debug: &TileDebugControls,
    asset_server: &AssetServer,
) {
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
            NodeStyleSheet::new(asset_server.load("map/ui/fishystuff.css")),
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
