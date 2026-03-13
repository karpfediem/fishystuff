use crate::constants::{LEFT, SECTOR_PER_PIXEL, SECTOR_SCALE, TOP};

pub trait MapToWaterTransform {
    fn map_to_water(&self, map_x: f64, map_y: f64) -> (f64, f64);
}

#[derive(Debug, Clone)]
pub enum TransformKind {
    ScaleToFit {
        map_w: u32,
        map_h: u32,
        water_w: u32,
        water_h: u32,
    },
    ScaleOffset {
        sx: f64,
        sy: f64,
        ox: f64,
        oy: f64,
    },
    WorldExtent {
        world_left: f64,
        world_right: f64,
        world_bottom: f64,
        world_top: f64,
        map_pixel_center_offset: f64,
        water_w: u32,
        water_h: u32,
    },
}

impl MapToWaterTransform for TransformKind {
    fn map_to_water(&self, map_x: f64, map_y: f64) -> (f64, f64) {
        match *self {
            TransformKind::ScaleToFit {
                map_w,
                map_h,
                water_w,
                water_h,
            } => {
                let sx = if map_w > 1 && water_w > 1 {
                    (water_w as f64 - 1.0) / (map_w as f64 - 1.0)
                } else {
                    0.0
                };
                let sy = if map_h > 1 && water_h > 1 {
                    (water_h as f64 - 1.0) / (map_h as f64 - 1.0)
                } else {
                    0.0
                };
                (map_x * sx, map_y * sy)
            }
            TransformKind::ScaleOffset { sx, sy, ox, oy } => (map_x * sx + ox, map_y * sy + oy),
            TransformKind::WorldExtent {
                world_left,
                world_right,
                world_bottom,
                world_top,
                map_pixel_center_offset,
                water_w,
                water_h,
            } => {
                let world_x = (LEFT + map_x * SECTOR_PER_PIXEL) * SECTOR_SCALE;
                let world_z =
                    (TOP - (map_y + map_pixel_center_offset) * SECTOR_PER_PIXEL) * SECTOR_SCALE;
                let extent_w = world_right - world_left;
                let extent_h = world_top - world_bottom;
                let ux = if extent_w.abs() > f64::EPSILON {
                    (world_x - world_left) / extent_w
                } else {
                    0.0
                };
                let uy = if extent_h.abs() > f64::EPSILON {
                    (world_top - world_z) / extent_h
                } else {
                    0.0
                };
                let sx = ux * (water_w.saturating_sub(1) as f64);
                let sy = uy * (water_h.saturating_sub(1) as f64);
                (sx, sy)
            }
        }
    }
}
