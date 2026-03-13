use super::super::*;

pub(super) fn text_style(size: f32, color: Color, font: Handle<Font>) -> UiTextStyle {
    UiTextStyle { font, size, color }
}

pub(super) fn load_fonts(mut commands: Commands, mut fonts: ResMut<Assets<Font>>) {
    let bytes = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../site/assets/css/fonts/Comfortaa/static/Comfortaa-Regular.ttf"
    ))
    .to_vec();
    let font = Font::try_from_bytes(bytes).expect("load ui font");
    let handle = fonts.add(font);
    commands.insert_resource(UiFonts { regular: handle });
}
