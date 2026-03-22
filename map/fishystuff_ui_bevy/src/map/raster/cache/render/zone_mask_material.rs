use bevy::app::{App, Plugin};
use bevy_asset::{embedded_asset, embedded_path, Asset, AssetPath, Handle};
use bevy_image::Image;
use bevy_math::Vec4;
use bevy_reflect::TypePath;
use bevy_render::render_resource::AsBindGroup;
use bevy_shader::ShaderRef;
use bevy_sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};

const HOVER_HIGHLIGHT_RGB: [f32; 3] = [64.0 / 255.0, 1.0, 128.0 / 255.0];

pub(crate) struct ZoneMaskMaterialPlugin;

impl Plugin for ZoneMaskMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "zone_mask_material.wgsl");
        app.add_plugins(Material2dPlugin::<ZoneMaskMaterial>::default());
    }
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
pub(crate) struct ZoneMaskMaterial {
    #[uniform(0)]
    pub(crate) opacity: f32,
    #[uniform(1)]
    pub(crate) hover_rgb: Vec4,
    #[uniform(2)]
    pub(crate) highlight_rgb: Vec4,
    #[texture(3)]
    #[sampler(4)]
    pub(crate) texture: Handle<Image>,
}

impl ZoneMaskMaterial {
    pub(crate) fn new(texture: Handle<Image>, opacity: f32, hover_zone_rgb: Option<u32>) -> Self {
        let mut material = Self {
            opacity: 1.0,
            hover_rgb: Vec4::ZERO,
            highlight_rgb: Vec4::new(
                HOVER_HIGHLIGHT_RGB[0],
                HOVER_HIGHLIGHT_RGB[1],
                HOVER_HIGHLIGHT_RGB[2],
                1.0,
            ),
            texture,
        };
        material.sync_visual_state(opacity, hover_zone_rgb);
        material
    }

    pub(crate) fn sync_visual_state(&mut self, opacity: f32, hover_zone_rgb: Option<u32>) -> bool {
        let next_opacity = opacity.clamp(0.0, 1.0);
        let next_hover_rgb = hover_zone_rgb
            .map(|rgb| {
                Vec4::new(
                    ((rgb >> 16) & 0xff) as f32 / 255.0,
                    ((rgb >> 8) & 0xff) as f32 / 255.0,
                    (rgb & 0xff) as f32 / 255.0,
                    1.0,
                )
            })
            .unwrap_or(Vec4::ZERO);
        if self.opacity == next_opacity && self.hover_rgb == next_hover_rgb {
            return false;
        }
        self.opacity = next_opacity;
        self.hover_rgb = next_hover_rgb;
        true
    }
}

impl Material2d for ZoneMaskMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("zone_mask_material.wgsl"))
                .with_source("embedded"),
        )
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[cfg(test)]
mod tests {
    use super::ZoneMaskMaterial;
    use bevy_asset::Handle;
    use bevy_image::Image;

    #[test]
    fn sync_visual_state_only_mutates_when_inputs_change() {
        let mut material = ZoneMaskMaterial::new(Handle::<Image>::default(), 0.55, Some(0x123456));
        assert_eq!(material.opacity, 0.55);
        assert_eq!(material.hover_rgb.w, 1.0);
        assert!(!material.sync_visual_state(0.55, Some(0x123456)));
        assert!(material.sync_visual_state(0.25, Some(0x654321)));
        assert_eq!(material.opacity, 0.25);
        assert!((material.hover_rgb.x - (0x65 as f32 / 255.0)).abs() < f32::EPSILON);
        assert!(material.sync_visual_state(0.25, None));
        assert_eq!(material.hover_rgb, bevy_math::Vec4::ZERO);
    }
}
