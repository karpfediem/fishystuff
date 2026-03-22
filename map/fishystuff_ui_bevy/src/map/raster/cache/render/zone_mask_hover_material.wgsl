#import bevy_sprite::{
    mesh2d_vertex_output::VertexOutput,
    mesh2d_view_bindings::view,
}

#ifdef TONEMAP_IN_SHADER
#import bevy_core_pipeline::tonemapping
#endif

struct ZoneMaskHoverMaterial {
    hover_rgb: vec4<f32>,
    highlight_rgba: vec4<f32>,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: ZoneMaskHoverMaterial;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var texture_color: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var texture_sampler: sampler;

const COLOR_EPSILON: f32 = 0.0015;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let sampled = textureSample(texture_color, texture_sampler, mesh.uv);
    var output_color = vec4<f32>(0.0);
    if distance(sampled.rgb, material.hover_rgb.rgb) <= COLOR_EPSILON {
        output_color = material.highlight_rgba;
    }

#ifdef TONEMAP_IN_SHADER
    output_color = tonemapping::tone_mapping(output_color, view.color_grading);
#endif
    return output_color;
}
