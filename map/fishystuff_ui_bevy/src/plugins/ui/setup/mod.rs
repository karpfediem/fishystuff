use super::*;

mod styles;

pub(super) fn load_fonts(commands: Commands, fonts: ResMut<Assets<Font>>) {
    styles::load_fonts(commands, fonts);
}

pub(super) fn setup_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        UiRoot,
        UiRenderEntity,
        ui_layers(),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            ..default()
        },
        NodeStyleSheet::new(asset_server.load("/map/ui/fishystuff.css")),
        ClassList::new("fs-overlay"),
    ));
}
