use crate::map::spaces::affine::Affine2D;
use crate::map::spaces::{MapPoint, MapRect, WorldPoint, WorldRect};
use fishystuff_core::constants::{
    BOTTOM, DEFAULT_PIXEL_CENTER_OFFSET, DISTANCE_PER_PIXEL, LEFT, MAP_HEIGHT, MAP_WIDTH, RIGHT,
    SECTOR_PER_PIXEL, SECTOR_SCALE, TOP,
};
use fishystuff_core::coord::{pixel_to_world_with_offset, world_to_pixel_f_with_offset};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapToWorld {
    pub left: f64,
    pub right: f64,
    pub bottom: f64,
    pub top: f64,
    pub image_size_x: u32,
    pub image_size_y: u32,
    pub sector_per_pixel: f64,
    pub distance_per_pixel: f64,
    pub world_position_factor: f64,
    pub pixel_center_offset: f64,
}

impl Default for MapToWorld {
    fn default() -> Self {
        Self {
            left: LEFT,
            right: RIGHT,
            bottom: BOTTOM,
            top: TOP,
            image_size_x: MAP_WIDTH as u32,
            image_size_y: MAP_HEIGHT as u32,
            sector_per_pixel: SECTOR_PER_PIXEL,
            distance_per_pixel: DISTANCE_PER_PIXEL,
            world_position_factor: SECTOR_SCALE,
            pixel_center_offset: DEFAULT_PIXEL_CENTER_OFFSET,
        }
    }
}

impl MapToWorld {
    pub fn map_to_world(self, map: MapPoint) -> WorldPoint {
        let (wx, wz) = pixel_to_world_with_offset(map.x, map.y, self.pixel_center_offset);
        WorldPoint::new(wx, wz)
    }

    pub fn world_to_map(self, world: WorldPoint) -> MapPoint {
        let (mx, my) = world_to_pixel_f_with_offset(world.x, world.z, self.pixel_center_offset);
        MapPoint::new(mx, my)
    }

    pub fn map_to_world_affine(self) -> Affine2D {
        Affine2D::new(
            self.sector_per_pixel * self.world_position_factor,
            0.0,
            self.left * self.world_position_factor,
            0.0,
            -self.sector_per_pixel * self.world_position_factor,
            (self.top - self.pixel_center_offset * self.sector_per_pixel)
                * self.world_position_factor,
        )
    }

    pub fn world_to_map_affine(self) -> Affine2D {
        self.map_to_world_affine()
            .inverse()
            .expect("map->world affine must be invertible")
    }

    pub fn map_bounds(self) -> MapRect {
        MapRect {
            min: MapPoint::new(0.0, 0.0),
            max: MapPoint::new(self.image_size_x as f64, self.image_size_y as f64),
        }
    }

    pub fn world_bounds(self) -> WorldRect {
        WorldRect {
            min: WorldPoint::new(
                self.left * self.world_position_factor,
                self.bottom * self.world_position_factor,
            ),
            max: WorldPoint::new(
                self.right * self.world_position_factor,
                self.top * self.world_position_factor,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MapToWorld;
    use crate::map::spaces::MapPoint;
    use fishystuff_core::constants::{MAP_HEIGHT, MAP_WIDTH};

    #[test]
    fn map_world_roundtrip() {
        let map_to_world = MapToWorld::default();
        let samples = [
            MapPoint::new(0.0, 0.0),
            MapPoint::new((MAP_WIDTH - 1) as f64, (MAP_HEIGHT - 1) as f64),
            MapPoint::new(50.5, 20.25),
            MapPoint::new(5000.0, 8000.0),
        ];
        for map in samples {
            let world = map_to_world.map_to_world(map);
            let back = map_to_world.world_to_map(world);
            assert!((back.x - map.x).abs() < 1e-7);
            assert!((back.y - map.y).abs() < 1e-7);
        }
    }

    #[test]
    fn affine_and_formula_match() {
        let map_to_world = MapToWorld::default();
        let affine = map_to_world.map_to_world_affine();
        let map = MapPoint::new(7331.125, 912.75);
        let world = map_to_world.map_to_world(map);
        let (x, z) = affine.apply(map.x, map.y);
        assert!((x - world.x).abs() < 1e-9);
        assert!((z - world.z).abs() < 1e-9);
    }
}
