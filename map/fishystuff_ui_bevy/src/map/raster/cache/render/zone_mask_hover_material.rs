use bevy::app::{App, Plugin};
use bevy::asset::{
    embedded_asset, embedded_path, Asset, AssetApp, AssetPath, Handle, UntypedAssetId,
    VisitAssetDependencies,
};
use bevy::color::{Color, ColorToComponents, LinearRgba};
use bevy::image::Image;
use bevy::math::Vec4;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::render::RenderApp;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};

const HOVER_OVERLAY_ALPHA: f32 = 0.42;
const HOVER_HIGHLIGHT_RGB: [u8; 3] = [64, 255, 128];

pub(crate) struct ZoneMaskHoverMaterialPlugin;

impl Plugin for ZoneMaskHoverMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "zone_mask_hover_material.wgsl");
        app.init_asset::<ZoneMaskHoverMaterial>();
        if app.get_sub_app(RenderApp).is_some() {
            app.add_plugins(Material2dPlugin::<ZoneMaskHoverMaterial>::default());
        }
    }
}

#[derive(AsBindGroup, Debug, Clone)]
pub(crate) struct ZoneMaskHoverMaterial {
    #[uniform(0)]
    pub(crate) params: ZoneMaskHoverMaterialUniform,
    #[texture(1)]
    #[sampler(2)]
    pub(crate) texture: Handle<Image>,
}

#[derive(Clone, Copy, Debug, ShaderType)]
pub(crate) struct ZoneMaskHoverMaterialUniform {
    pub(crate) hover_rgb: Vec4,
    pub(crate) highlight_rgba: Vec4,
}

impl ZoneMaskHoverMaterial {
    pub(crate) fn new(texture: Handle<Image>, hover_zone_rgb: u32, layer_alpha: f32) -> Self {
        let (r, g, b) = fishystuff_core::masks::unpack_rgb_u32(hover_zone_rgb);
        let hover_rgb = LinearRgba::from(Color::srgb_u8(r, g, b))
            .to_f32_array()
            .into();
        let highlight_rgba = LinearRgba::from(Color::srgba_u8(
            HOVER_HIGHLIGHT_RGB[0],
            HOVER_HIGHLIGHT_RGB[1],
            HOVER_HIGHLIGHT_RGB[2],
            (layer_alpha.clamp(0.0, 1.0) * HOVER_OVERLAY_ALPHA * 255.0)
                .round()
                .clamp(0.0, 255.0) as u8,
        ))
        .to_f32_array()
        .into();
        Self {
            params: ZoneMaskHoverMaterialUniform {
                hover_rgb,
                highlight_rgba,
            },
            texture,
        }
    }

    pub(crate) fn update(&mut self, hover_zone_rgb: u32, layer_alpha: f32) {
        *self = Self::new(self.texture.clone(), hover_zone_rgb, layer_alpha);
    }
}

impl VisitAssetDependencies for ZoneMaskHoverMaterial {
    fn visit_dependencies(&self, visit: &mut impl FnMut(UntypedAssetId)) {
        visit(self.texture.id().untyped());
    }
}

impl TypePath for ZoneMaskHoverMaterial {
    fn type_path() -> &'static str {
        "fishystuff_ui_bevy::map::raster::cache::render::zone_mask_hover_material::ZoneMaskHoverMaterial"
    }

    fn short_type_path() -> &'static str {
        "ZoneMaskHoverMaterial"
    }

    fn type_ident() -> Option<&'static str> {
        Some("ZoneMaskHoverMaterial")
    }

    fn crate_name() -> Option<&'static str> {
        Some("fishystuff_ui_bevy")
    }

    fn module_path() -> Option<&'static str> {
        Some("map::raster::cache::render::zone_mask_hover_material")
    }
}

impl Asset for ZoneMaskHoverMaterial {}

impl Material2d for ZoneMaskHoverMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("zone_mask_hover_material.wgsl"))
                .with_source("embedded"),
        )
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}
