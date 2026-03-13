use crate::prelude::*;

pub fn make_drape_material(
    materials: &mut Assets<StandardMaterial>,
    texture: Handle<Image>,
    alpha: f32,
) -> Handle<StandardMaterial> {
    let alpha = alpha.clamp(0.0, 1.0);
    materials.add(StandardMaterial {
        base_color_texture: Some(texture),
        base_color: Color::srgba(1.0, 1.0, 1.0, alpha),
        alpha_mode: drape_alpha_mode(alpha),
        depth_bias: 2.0,
        unlit: true,
        cull_mode: None,
        ..default()
    })
}

pub fn apply_drape_material_alpha(material: &mut StandardMaterial, alpha: f32) {
    let alpha = alpha.clamp(0.0, 1.0);
    material.base_color = Color::srgba(1.0, 1.0, 1.0, alpha);
    material.alpha_mode = drape_alpha_mode(alpha);
    material.depth_bias = 2.0;
    material.cull_mode = None;
}

fn drape_alpha_mode(alpha: f32) -> AlphaMode {
    if alpha >= 0.999 {
        AlphaMode::Opaque
    } else {
        AlphaMode::Blend
    }
}
