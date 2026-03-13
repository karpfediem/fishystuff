pub fn tile_z(z_base: f32, max_level: u8, level: i32) -> f32 {
    let max_level_hint = max_level as i32;
    let lod_bias = ((max_level_hint - level).max(0) as f32) * 0.001;
    z_base + lod_bias
}
