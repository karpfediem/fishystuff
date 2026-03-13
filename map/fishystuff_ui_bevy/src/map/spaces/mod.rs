pub mod affine;
pub mod layer_transform;
pub mod world;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapPoint {
    pub x: f64,
    pub y: f64,
}

impl MapPoint {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayerPoint {
    pub x: f64,
    pub y: f64,
}

impl LayerPoint {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldPoint {
    pub x: f64,
    pub z: f64,
}

impl WorldPoint {
    pub const fn new(x: f64, z: f64) -> Self {
        Self { x, z }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapRect {
    pub min: MapPoint,
    pub max: MapPoint,
}

impl MapRect {
    pub fn corners(self) -> [MapPoint; 4] {
        [
            MapPoint::new(self.min.x, self.min.y),
            MapPoint::new(self.max.x, self.min.y),
            MapPoint::new(self.max.x, self.max.y),
            MapPoint::new(self.min.x, self.max.y),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldRect {
    pub min: WorldPoint,
    pub max: WorldPoint,
}

impl WorldRect {
    pub fn corners(self) -> [WorldPoint; 4] {
        [
            WorldPoint::new(self.min.x, self.min.z),
            WorldPoint::new(self.max.x, self.min.z),
            WorldPoint::new(self.max.x, self.max.z),
            WorldPoint::new(self.min.x, self.max.z),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayerRect {
    pub min: LayerPoint,
    pub max: LayerPoint,
}

impl LayerRect {
    pub fn corners(self) -> [LayerPoint; 4] {
        [
            LayerPoint::new(self.min.x, self.min.y),
            LayerPoint::new(self.max.x, self.min.y),
            LayerPoint::new(self.max.x, self.max.y),
            LayerPoint::new(self.min.x, self.max.y),
        ]
    }
}
