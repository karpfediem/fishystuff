use std::ops::RangeInclusive;

use fishystuff_core::terrain::{
    chunk_grid_dims_for_level, chunk_map_bounds as core_chunk_map_bounds,
    chunk_span_map_px as core_chunk_span_map_px, key_for_map_px as core_key_for_map_px,
    TerrainChunkLodKey,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerrainChunkKey(pub TerrainChunkLodKey);

impl TerrainChunkKey {
    pub fn new(level: u8, cx: i32, cy: i32) -> Self {
        Self(TerrainChunkLodKey { level, cx, cy })
    }

    pub fn level(self) -> u8 {
        self.0.level
    }

    pub fn cx(self) -> i32 {
        self.0.cx
    }

    pub fn cy(self) -> i32 {
        self.0.cy
    }

    pub fn raw(self) -> TerrainChunkLodKey {
        self.0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TerrainChunkState {
    #[default]
    NotRequested,
    Building,
    Ready,
    Failed,
}

pub fn chunk_span_map_px(chunk_map_px: u32, level: u8) -> u32 {
    core_chunk_span_map_px(chunk_map_px, level)
}

pub fn chunk_grid_dims(map_w: u32, map_h: u32, chunk_map_px: u32, level: u8) -> (i32, i32) {
    chunk_grid_dims_for_level(map_w, map_h, chunk_map_px, level)
}

pub fn chunk_map_bounds(
    key: TerrainChunkKey,
    map_w: u32,
    map_h: u32,
    chunk_map_px: u32,
) -> (f32, f32, f32, f32) {
    core_chunk_map_bounds(map_w, map_h, chunk_map_px, key.0)
}

pub fn key_for_map_px(map_x: f32, map_y: f32, chunk_map_px: u32, level: u8) -> TerrainChunkKey {
    TerrainChunkKey(core_key_for_map_px(map_x, map_y, chunk_map_px, level))
}

pub fn visible_chunk_range(
    center_map_x: f32,
    center_map_y: f32,
    radius_map_px: f32,
    map_w: u32,
    map_h: u32,
    chunk_map_px: u32,
    level: u8,
) -> (RangeInclusive<i32>, RangeInclusive<i32>) {
    let chunk = core_chunk_span_map_px(chunk_map_px, level).max(1) as f32;
    let (nx, ny) = chunk_grid_dims_for_level(map_w, map_h, chunk_map_px, level);
    let min_x = ((center_map_x - radius_map_px) / chunk).floor() as i32;
    let max_x = ((center_map_x + radius_map_px) / chunk).floor() as i32;
    let min_y = ((center_map_y - radius_map_px) / chunk).floor() as i32;
    let max_y = ((center_map_y + radius_map_px) / chunk).floor() as i32;

    (
        min_x.clamp(0, nx - 1)..=max_x.clamp(0, nx - 1),
        min_y.clamp(0, ny - 1)..=max_y.clamp(0, ny - 1),
    )
}

#[cfg(test)]
mod tests {
    use super::{chunk_grid_dims, key_for_map_px, visible_chunk_range};

    #[test]
    fn chunk_grid_matches_expected_counts() {
        let (nx, ny) = chunk_grid_dims(11560, 10540, 512, 0);
        assert_eq!(nx, 23);
        assert_eq!(ny, 21);
    }

    #[test]
    fn map_px_maps_to_chunk_key() {
        let key = key_for_map_px(1024.0, 1537.0, 512, 0);
        assert_eq!(key.cx(), 2);
        assert_eq!(key.cy(), 3);
    }

    #[test]
    fn visible_range_clamps_to_grid() {
        let (xs, ys) = visible_chunk_range(100.0, 100.0, 600.0, 2000, 2000, 512, 0);
        assert_eq!(*xs.start(), 0);
        assert_eq!(*ys.start(), 0);
        assert!(*xs.end() <= 3);
        assert!(*ys.end() <= 3);
    }
}
