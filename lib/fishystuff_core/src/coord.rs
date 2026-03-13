use crate::constants::{
    DEFAULT_PIXEL_CENTER_OFFSET, LEFT, MAP_HEIGHT, MAP_WIDTH, SECTOR_PER_PIXEL, SECTOR_SCALE, TOP,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pixel {
    pub x: i32,
    pub y: i32,
}

pub fn pixel_in_bounds(px: i32, py: i32) -> bool {
    (0..MAP_WIDTH).contains(&px) && (0..MAP_HEIGHT).contains(&py)
}

pub fn pixel_if_in_bounds(px: i32, py: i32) -> Option<Pixel> {
    if pixel_in_bounds(px, py) {
        Some(Pixel { x: px, y: py })
    } else {
        None
    }
}

pub fn pixel_to_world_with_offset(px: f64, py: f64, pixel_center_offset: f64) -> (f64, f64) {
    let world_x = (px * SECTOR_PER_PIXEL + LEFT) * SECTOR_SCALE;
    let world_z = (-(py + pixel_center_offset) * SECTOR_PER_PIXEL + TOP) * SECTOR_SCALE;
    (world_x, world_z)
}

pub fn pixel_to_world(px: f64, py: f64) -> (f64, f64) {
    pixel_to_world_with_offset(px, py, DEFAULT_PIXEL_CENTER_OFFSET)
}

pub fn world_to_pixel_f_with_offset(
    world_x: f64,
    world_z: f64,
    pixel_center_offset: f64,
) -> (f64, f64) {
    let px = ((world_x / SECTOR_SCALE) - LEFT) / SECTOR_PER_PIXEL;
    let py = ((TOP - (world_z / SECTOR_SCALE)) / SECTOR_PER_PIXEL) - pixel_center_offset;
    (px, py)
}

pub fn world_to_pixel_f(world_x: f64, world_z: f64) -> (f64, f64) {
    world_to_pixel_f_with_offset(world_x, world_z, DEFAULT_PIXEL_CENTER_OFFSET)
}

pub fn world_to_pixel_round(world_x: f64, world_z: f64) -> Pixel {
    let (px, py) = world_to_pixel_f(world_x, world_z);
    Pixel {
        x: px.round() as i32,
        y: py.round() as i32,
    }
}
