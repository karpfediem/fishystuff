use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy_resvg::resvg;

const ICON_SPRITE_SVG: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../site/assets/img/icons.svg"
));

pub struct UiSvgIconsPlugin;

impl Plugin for UiSvgIconsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiSvgIconAssets>()
            .add_systems(Update, ensure_ui_svg_icon_assets);
    }
}

#[derive(Resource, Default)]
pub struct UiSvgIconAssets {
    map_pin: Option<Handle<Image>>,
    fish_fill: Option<Handle<Image>>,
    crosshair: Option<Handle<Image>>,
    hover_resources: Option<Handle<Image>>,
    trade_origin: Option<Handle<Image>>,
    bookmark: Option<Handle<Image>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiSvgIconKind {
    MapPin,
    FishFill,
    Crosshair,
    HoverResources,
    TradeOrigin,
    Bookmark,
}

impl UiSvgIconAssets {
    pub fn handle(&self, kind: UiSvgIconKind) -> Option<Handle<Image>> {
        match kind {
            UiSvgIconKind::MapPin => self.map_pin.clone(),
            UiSvgIconKind::FishFill => self.fish_fill.clone(),
            UiSvgIconKind::Crosshair => self.crosshair.clone(),
            UiSvgIconKind::HoverResources => self.hover_resources.clone(),
            UiSvgIconKind::TradeOrigin => self.trade_origin.clone(),
            UiSvgIconKind::Bookmark => self.bookmark.clone(),
        }
    }
}

fn ensure_ui_svg_icon_assets(
    mut icon_assets: ResMut<UiSvgIconAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    if icon_assets.map_pin.is_some()
        && icon_assets.fish_fill.is_some()
        && icon_assets.crosshair.is_some()
        && icon_assets.hover_resources.is_some()
        && icon_assets.trade_origin.is_some()
        && icon_assets.bookmark.is_some()
    {
        return;
    }

    icon_assets.map_pin = Some(add_sprite_icon_asset(
        &mut images,
        "fishy-map-pin",
        "map-pin",
        "#ffffff",
    ));
    icon_assets.fish_fill = Some(add_sprite_icon_asset(
        &mut images,
        "fishy-fish-fill",
        "fish-fill",
        "#ffffff",
    ));
    icon_assets.crosshair = Some(add_sprite_icon_asset(
        &mut images,
        "fishy-crosshair",
        "crosshair",
        "#ffffff",
    ));
    icon_assets.hover_resources = Some(add_sprite_icon_asset(
        &mut images,
        "fishy-hover-resources",
        "hover-resources",
        "#ffffff",
    ));
    icon_assets.trade_origin = Some(add_sprite_icon_asset(
        &mut images,
        "fishy-trade-origin",
        "trade-origin",
        "#ffffff",
    ));
    icon_assets.bookmark = Some(add_sprite_icon_asset(
        &mut images,
        "fishy-bookmark",
        "bookmark",
        "#ffffff",
    ));
}

fn add_sprite_icon_asset(
    images: &mut Assets<Image>,
    symbol_id: &str,
    debug_name: &str,
    color_css: &str,
) -> Handle<Image> {
    images.add(render_sprite_icon(symbol_id, debug_name, color_css))
}

fn render_sprite_icon(symbol_id: &str, debug_name: &str, color_css: &str) -> Image {
    let svg_markup = extract_sprite_symbol_svg(ICON_SPRITE_SVG, symbol_id)
        .unwrap_or_else(|| panic!("missing svg symbol `{symbol_id}` in shared icon sprite"));
    let svg_markup = inject_svg_color(&svg_markup, color_css);
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(svg_markup.as_bytes(), &options)
        .unwrap_or_else(|err| panic!("failed to parse `{debug_name}` svg: {err}"));
    let size = tree.size().to_int_size();
    let (width, height) = size.dimensions();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .unwrap_or_else(|| panic!("failed to allocate pixmap for `{debug_name}`"));
    resvg::render(
        &tree,
        resvg::usvg::Transform::default(),
        &mut pixmap.as_mut(),
    );
    Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixmap.take(),
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    )
}

fn extract_sprite_symbol_svg(sprite_svg: &str, symbol_id: &str) -> Option<String> {
    let id_token = format!("id=\"{symbol_id}\"");
    let id_index = sprite_svg.find(&id_token)?;
    let symbol_start = sprite_svg[..id_index].rfind("<symbol")?;
    let open_end = symbol_start + sprite_svg[symbol_start..].find('>')?;
    let open_tag = &sprite_svg[symbol_start..=open_end];
    let view_box = extract_attribute_value(open_tag, "viewBox")?;
    let content_start = open_end + 1;
    let content_end = content_start + sprite_svg[content_start..].find("</symbol>")?;
    let symbol_content = &sprite_svg[content_start..content_end];
    Some(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{view_box}\">{symbol_content}</svg>"
    ))
}

fn inject_svg_color(svg: &str, color_css: &str) -> String {
    svg.replacen("<svg ", &format!("<svg color=\"{color_css}\" "), 1)
}

fn extract_attribute_value<'a>(tag: &'a str, attribute: &str) -> Option<&'a str> {
    let prefix = format!("{attribute}=\"");
    let value_start = tag.find(&prefix)? + prefix.len();
    let value_end = value_start + tag[value_start..].find('"')?;
    Some(&tag[value_start..value_end])
}
