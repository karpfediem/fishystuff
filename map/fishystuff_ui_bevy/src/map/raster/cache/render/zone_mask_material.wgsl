#import bevy_sprite::mesh2d_vertex_output::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material_opacity: f32;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> material_hover_rgb: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> material_highlight_rgb: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var texture_color: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var texture_sampler: sampler;

const HOVER_MATCH_TOLERANCE: f32 = 0.5 / 255.0;
const HOVER_BLEND: f32 = 0.82;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    var output_color = textureSample(texture_color, texture_sampler, mesh.uv);
    if material_hover_rgb.w > 0.5 {
        let delta = abs(output_color.rgb - material_hover_rgb.rgb);
        if delta.r <= HOVER_MATCH_TOLERANCE
            && delta.g <= HOVER_MATCH_TOLERANCE
            && delta.b <= HOVER_MATCH_TOLERANCE
        {
            output_color.rgb = mix(output_color.rgb, material_highlight_rgb.rgb, HOVER_BLEND);
        }
    }
    output_color.a = output_color.a * material_opacity;
    return output_color;
}
