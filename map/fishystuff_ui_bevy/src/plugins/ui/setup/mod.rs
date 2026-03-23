use super::*;

mod panel_shell;
mod patch_shell;
mod search_shell;
mod styles;

#[derive(Clone)]
struct SetupTextStyles {
    title_style: UiTextStyle,
    label_style: UiTextStyle,
    value_style: UiTextStyle,
    small_style: UiTextStyle,
}

impl SetupTextStyles {
    fn new(font: Handle<Font>) -> Self {
        Self {
            title_style: text_style(20.0, Color::srgb(0.95, 0.95, 0.98), font.clone()),
            label_style: text_style(14.0, Color::srgb(0.82, 0.82, 0.86), font.clone()),
            value_style: text_style(13.0, Color::srgb(0.9, 0.9, 0.92), font.clone()),
            small_style: text_style(12.0, Color::srgb(0.75, 0.75, 0.8), font),
        }
    }
}

pub(super) fn text_style(size: f32, color: Color, font: Handle<Font>) -> UiTextStyle {
    styles::text_style(size, color, font)
}

pub(super) fn load_fonts(commands: Commands, fonts: ResMut<Assets<Font>>) {
    styles::load_fonts(commands, fonts);
}

pub(super) fn setup_ui(
    mut commands: Commands,
    fonts: Res<UiFonts>,
    asset_server: Res<AssetServer>,
) {
    let styles = SetupTextStyles::new(fonts.regular.clone());

    commands
        .spawn((
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
        ))
        .with_children(|root| {
            panel_shell::spawn_selection_panel(root, &styles);
            patch_shell::spawn_patch_panel(root, &styles);
            search_shell::spawn_search_anchor(root, &styles);
        });
}
