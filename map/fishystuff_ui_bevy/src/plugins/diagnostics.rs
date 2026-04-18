use crate::bridge::contract::FishyMapThemeColors;
use crate::bridge::host::BrowserBridgeState;
use crate::bridge::theme::parse_css_color;
use crate::plugins::ui::{UiFonts, UiStartupSet};
use crate::prelude::*;
use bevy::color::Alpha;
use bevy::text::{TextColor, TextFont};
use bevy_flair::prelude::*;

pub struct DiagnosticsPlugin;

impl Plugin for DiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DiagnosticsState>()
            .add_systems(Startup, setup_diagnostics_ui.after(UiStartupSet))
            .add_systems(Update, (update_diagnostics_theme, update_diagnostics_text));
    }
}

#[derive(Component)]
struct DiagnosticsText;

#[derive(Component)]
struct DiagnosticsRoot;

#[derive(Resource, Default)]
struct DiagnosticsState {
    accum_time: f64,
    frame_count: u32,
}

fn setup_diagnostics_ui(mut commands: Commands, fonts: Res<UiFonts>) {
    let font = fonts.regular.clone();

    commands
        .spawn((
            DiagnosticsRoot,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                top: Val::Px(12.0),
                padding: UiRect::all(Val::Px(8.0)),
                border_radius: BorderRadius::all(Val::Px(999.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.75)),
            BorderColor::all(Color::srgba(1.0, 1.0, 1.0, 0.10)),
            ClassList::new("diagnostics"),
        ))
        .with_children(|root| {
            root.spawn((
                DiagnosticsText,
                Text::new("fps: --"),
                TextFont {
                    font,
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.86, 0.9, 0.94)),
                ClassList::new("diagnostics-text"),
            ));
        });
}

fn update_diagnostics_text(
    time: Res<Time>,
    mut state: ResMut<DiagnosticsState>,
    mut query: Query<&mut Text, With<DiagnosticsText>>,
) {
    state.accum_time += time.delta_secs_f64();
    state.frame_count = state.frame_count.saturating_add(1);
    if state.accum_time < 0.5 {
        return;
    }
    let fps = state.frame_count as f64 / state.accum_time.max(0.001);
    state.accum_time = 0.0;
    state.frame_count = 0;
    crate::perf_last!("bevy.diagnostics.fps", fps);
    crate::perf_last!(
        "bevy.diagnostics.frame_time_ms",
        if fps > 0.0 { 1000.0 / fps } else { 0.0 }
    );

    if let Ok(mut text_comp) = query.single_mut() {
        text_comp.0 = format!("fps: {:>5.1}", fps);
    }
}

fn update_diagnostics_theme(
    bridge: Res<BrowserBridgeState>,
    mut root_query: Query<
        (&mut BackgroundColor, &mut BorderColor, &mut Node),
        With<DiagnosticsRoot>,
    >,
    mut text_query: Query<&mut TextColor, With<DiagnosticsText>>,
) {
    if !bridge.is_changed() {
        return;
    }

    let colors = &bridge.input.theme.colors;
    let background = diagnostics_background_color(colors)
        .unwrap_or_else(|| Color::srgba(0.05, 0.06, 0.08, 0.78));
    let border =
        diagnostics_border_color(colors).unwrap_or_else(|| Color::srgba(1.0, 1.0, 1.0, 0.10));
    let text = diagnostics_text_color(colors).unwrap_or_else(|| Color::srgb(0.86, 0.9, 0.94));

    if let Ok((mut background_color, mut border_color, mut node)) = root_query.single_mut() {
        background_color.0 = background;
        *border_color = BorderColor::all(border);
        node.border_radius = BorderRadius::all(Val::Px(999.0));
    }

    if let Ok(mut text_color) = text_query.single_mut() {
        text_color.0 = text;
    }
}

fn diagnostics_background_color(colors: &FishyMapThemeColors) -> Option<Color> {
    colors
        .base200
        .as_deref()
        .or(colors.base100.as_deref())
        .and_then(parse_css_color)
        .map(|color| color.with_alpha(0.92))
}

fn diagnostics_border_color(colors: &FishyMapThemeColors) -> Option<Color> {
    colors
        .base300
        .as_deref()
        .or(colors.base200.as_deref())
        .or(colors.base100.as_deref())
        .and_then(parse_css_color)
        .map(|color| color.with_alpha(0.92))
}

fn diagnostics_text_color(colors: &FishyMapThemeColors) -> Option<Color> {
    colors
        .base_content
        .as_deref()
        .and_then(parse_css_color)
        .map(|color| color.with_alpha(0.92))
}
