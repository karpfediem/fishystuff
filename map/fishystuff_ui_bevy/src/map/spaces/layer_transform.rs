use crate::map::spaces::affine::Affine2D;
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::{LayerPoint, LayerRect, MapPoint, WorldPoint, WorldRect};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayerTransform {
    IdentityMapSpace,
    AffineToMap(Affine2D),
    AffineToWorld(Affine2D),
}

impl LayerTransform {
    pub fn layer_to_world_affine(self, map_to_world: MapToWorld) -> Affine2D {
        match self {
            Self::IdentityMapSpace => map_to_world.map_to_world_affine(),
            Self::AffineToMap(affine) => {
                Affine2D::compose(map_to_world.map_to_world_affine(), affine)
            }
            Self::AffineToWorld(affine) => affine,
        }
    }

    pub fn world_to_layer_affine(self, map_to_world: MapToWorld) -> Option<Affine2D> {
        self.layer_to_world_affine(map_to_world).inverse()
    }

    pub fn layer_to_map_affine(self, map_to_world: MapToWorld) -> Option<Affine2D> {
        let layer_to_world = self.layer_to_world_affine(map_to_world);
        Some(Affine2D::compose(
            map_to_world.world_to_map_affine(),
            layer_to_world,
        ))
    }

    pub fn map_to_layer_affine(self, map_to_world: MapToWorld) -> Option<Affine2D> {
        let world_to_layer = self.world_to_layer_affine(map_to_world)?;
        Some(Affine2D::compose(
            world_to_layer,
            map_to_world.map_to_world_affine(),
        ))
    }

    pub fn layer_to_map(self, map_to_world: MapToWorld, layer: LayerPoint) -> Option<MapPoint> {
        let layer_to_map = self.layer_to_map_affine(map_to_world)?;
        let (x, y) = layer_to_map.apply(layer.x, layer.y);
        Some(MapPoint::new(x, y))
    }

    pub fn map_to_layer(self, map_to_world: MapToWorld, map: MapPoint) -> Option<LayerPoint> {
        let inv = self.map_to_layer_affine(map_to_world)?;
        let (x, y) = inv.apply(map.x, map.y);
        Some(LayerPoint::new(x, y))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileSpace {
    pub tile_px: u32,
    pub y_flip: bool,
}

impl TileSpace {
    pub const fn new(tile_px: u32, y_flip: bool) -> Self {
        Self { tile_px, y_flip }
    }

    pub fn local_to_layer_px(self, tile_x: i32, tile_y: i32, u: f64, v: f64) -> LayerPoint {
        let tile = self.tile_px as f64;
        let sx = tile_x as f64 * tile + u;
        let sy = if self.y_flip {
            tile_y as f64 * tile + (tile - 1.0 - v)
        } else {
            tile_y as f64 * tile + v
        };
        LayerPoint::new(sx, sy)
    }

    pub fn tile_span_px(self, level: i32) -> Option<f64> {
        if !(0..=30).contains(&level) || self.tile_px == 0 {
            return None;
        }
        let scale = 1_i64.checked_shl(level as u32)? as f64;
        Some(scale * self.tile_px as f64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldTransform {
    pub layer_to_map: Affine2D,
    pub map_to_layer: Affine2D,
    pub layer_to_world: Affine2D,
    pub world_to_layer: Affine2D,
    pub map_to_world: MapToWorld,
}

impl WorldTransform {
    pub fn new(layer: LayerTransform, map_to_world: MapToWorld) -> Option<Self> {
        let map_to_world_affine = map_to_world.map_to_world_affine();
        let world_to_map_affine = map_to_world.world_to_map_affine();
        let layer_to_world = layer.layer_to_world_affine(map_to_world);
        let world_to_layer = layer_to_world.inverse()?;
        let layer_to_map = Affine2D::compose(world_to_map_affine, layer_to_world);
        let map_to_layer = Affine2D::compose(world_to_layer, map_to_world_affine);
        debug_assert!(Affine2D::compose(world_to_map_affine, map_to_world_affine)
            .approx_eq(Affine2D::IDENTITY, 1e-8));
        Some(Self {
            layer_to_map,
            map_to_layer,
            layer_to_world,
            world_to_layer,
            map_to_world,
        })
    }

    pub fn layer_to_world(self, layer: LayerPoint) -> WorldPoint {
        let (x, z) = self.layer_to_world.apply(layer.x, layer.y);
        WorldPoint::new(x, z)
    }

    pub fn world_to_layer(self, world: WorldPoint) -> LayerPoint {
        let (x, y) = self.world_to_layer.apply(world.x, world.z);
        LayerPoint::new(x, y)
    }

    pub fn layer_to_map(self, layer: LayerPoint) -> MapPoint {
        let (x, y) = self.layer_to_map.apply(layer.x, layer.y);
        MapPoint::new(x, y)
    }

    pub fn map_to_layer(self, map: MapPoint) -> LayerPoint {
        let (x, y) = self.map_to_layer.apply(map.x, map.y);
        LayerPoint::new(x, y)
    }

    pub fn map_to_world(self, map: MapPoint) -> WorldPoint {
        self.map_to_world.map_to_world(map)
    }

    pub fn world_to_map(self, world: WorldPoint) -> MapPoint {
        self.map_to_world.world_to_map(world)
    }

    pub fn world_rect_to_layer_aabb(self, world: WorldRect) -> LayerRect {
        let corners = world.corners().map(|corner| self.world_to_layer(corner));
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for corner in corners {
            min_x = min_x.min(corner.x);
            min_y = min_y.min(corner.y);
            max_x = max_x.max(corner.x);
            max_y = max_y.max(corner.y);
        }
        LayerRect {
            min: LayerPoint::new(min_x, min_y),
            max: LayerPoint::new(max_x, max_y),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LayerTransform, TileSpace, WorldTransform};
    use crate::map::spaces::affine::Affine2D;
    use crate::map::spaces::world::MapToWorld;
    use crate::map::spaces::{LayerPoint, WorldPoint, WorldRect};

    const EPS: f64 = 1e-6;

    fn minimap_world_affine(map_to_world: MapToWorld, tile_px: u32) -> Affine2D {
        let scale = map_to_world.world_position_factor / tile_px as f64;
        Affine2D::new(scale, 0.0, 0.0, 0.0, scale, 0.0)
    }

    #[test]
    fn minimap_source_world_matches_direct_formula() {
        let map_to_world = MapToWorld::default();
        let tile_space = TileSpace::new(128, true);
        let layer_transform =
            LayerTransform::AffineToWorld(minimap_world_affine(map_to_world, tile_space.tile_px));
        let world_transform = WorldTransform::new(layer_transform, map_to_world).expect("invert");
        let source = tile_space.local_to_layer_px(-100, 90, 47.0, 61.0);
        let world = world_transform.layer_to_world(source);
        let expected = WorldPoint::new(
            source.x / tile_space.tile_px as f64 * map_to_world.world_position_factor,
            source.y / tile_space.tile_px as f64 * map_to_world.world_position_factor,
        );
        assert!((world.x - expected.x).abs() < EPS);
        assert!((world.z - expected.z).abs() < EPS);

        let map = world_transform.layer_to_map(source);
        let world_via_map = map_to_world.map_to_world(map);
        assert!((world.x - world_via_map.x).abs() < EPS);
        assert!((world.z - world_via_map.z).abs() < EPS);
    }

    #[test]
    fn layer_inverse_roundtrip_minimap_and_mask() {
        let map_to_world = MapToWorld::default();
        let tile_space = TileSpace::new(128, true);
        let minimap = WorldTransform::new(
            LayerTransform::AffineToWorld(minimap_world_affine(map_to_world, tile_space.tile_px)),
            map_to_world,
        )
        .expect("invertible minimap");
        let mask = WorldTransform::new(LayerTransform::IdentityMapSpace, map_to_world)
            .expect("invertible mask");

        let minimap_sample = LayerPoint::new(-2048.25, 8192.75);
        let minimap_world = minimap.layer_to_world(minimap_sample);
        let minimap_back = minimap.world_to_layer(minimap_world);
        assert!((minimap_back.x - minimap_sample.x).abs() < EPS);
        assert!((minimap_back.y - minimap_sample.y).abs() < EPS);

        let mask_sample = LayerPoint::new(2500.0, 4400.0);
        let mask_world = mask.layer_to_world(mask_sample);
        let mask_back = mask.world_to_layer(mask_world);
        assert!((mask_back.x - mask_sample.x).abs() < EPS);
        assert!((mask_back.y - mask_sample.y).abs() < EPS);
    }

    #[test]
    fn minimap_landmarks_follow_world_formula() {
        let map_to_world = MapToWorld::default();
        let tile_space = TileSpace::new(128, true);
        let minimap = WorldTransform::new(
            LayerTransform::AffineToWorld(minimap_world_affine(map_to_world, tile_space.tile_px)),
            map_to_world,
        )
        .expect("invertible minimap");

        let landmarks = [
            tile_space.local_to_layer_px(-134, 140, 0.5, 0.5),
            tile_space.local_to_layer_px(-15, 64, 27.0, 88.0),
            tile_space.local_to_layer_px(92, -30, 96.0, 17.0),
        ];

        for source_landmark in landmarks {
            let world = minimap.layer_to_world(source_landmark);
            let expected = WorldPoint::new(
                source_landmark.x / tile_space.tile_px as f64 * map_to_world.world_position_factor,
                source_landmark.y / tile_space.tile_px as f64 * map_to_world.world_position_factor,
            );
            assert!((world.x - expected.x).abs() < EPS);
            assert!((world.z - expected.z).abs() < EPS);
        }
    }

    #[test]
    fn visible_world_corners_generate_layer_aabb() {
        let map_to_world = MapToWorld::default();
        let tile_space = TileSpace::new(128, true);
        let minimap = WorldTransform::new(
            LayerTransform::AffineToWorld(minimap_world_affine(map_to_world, tile_space.tile_px)),
            map_to_world,
        )
        .expect("invertible minimap");

        let world_rect = WorldRect {
            min: WorldPoint::new(-250_000.0, 410_000.0),
            max: WorldPoint::new(120_000.0, 690_000.0),
        };
        let layer_aabb = minimap.world_rect_to_layer_aabb(world_rect);

        assert!(layer_aabb.min.x.is_finite());
        assert!(layer_aabb.min.y.is_finite());
        assert!(layer_aabb.max.x.is_finite());
        assert!(layer_aabb.max.y.is_finite());
        assert!(layer_aabb.max.x >= layer_aabb.min.x);
        assert!(layer_aabb.max.y >= layer_aabb.min.y);
    }
}
