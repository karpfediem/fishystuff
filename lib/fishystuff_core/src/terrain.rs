use std::collections::HashSet;

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine as _;
use serde::{Deserialize, Serialize};

pub const PACKED_RGB24_MAX_U32: u32 = 16_777_215;
pub const U16_HEIGHT_MAX_U16: u16 = 65_535;

pub const TERRAIN_CHUNK_MAGIC: [u8; 4] = *b"THC1";
pub const TERRAIN_CHUNK_VERSION: u16 = 1;
pub const TERRAIN_CHUNK_HEADER_LEN: usize = 24;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerrainHeightEncoding {
    #[default]
    U16Norm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TerrainChunkLodKey {
    pub level: u8,
    pub cx: i32,
    pub cy: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainManifest {
    pub revision: String,
    pub map_width: u32,
    pub map_height: u32,
    pub chunk_map_px: u32,
    pub grid_size: u16,
    pub max_level: u8,
    pub bbox_y_min: f32,
    pub bbox_y_max: f32,
    pub encoding: TerrainHeightEncoding,
    pub root: String,
    pub chunk_path: String,
    pub levels: Vec<TerrainLevelManifest>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerrainDrapeLayerKind {
    RasterVisual,
    RasterData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainDrapeManifest {
    pub revision: String,
    pub layer: String,
    pub map_width: u32,
    pub map_height: u32,
    pub chunk_map_px: u32,
    pub max_level: u8,
    pub texture_px: u16,
    pub format: String,
    pub kind: TerrainDrapeLayerKind,
    pub root: String,
    pub chunk_path: String,
    pub levels: Vec<TerrainLevelManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainLevelManifest {
    pub level: u8,
    pub min_x: i32,
    pub min_y: i32,
    pub width: u32,
    pub height: u32,
    pub tile_count: usize,
    pub occupancy_b64: String,
}

#[derive(Debug, Clone)]
pub struct DecodedTerrainLevel {
    pub level: u8,
    pub min_x: i32,
    pub min_y: i32,
    pub width: u32,
    pub height: u32,
    pub tile_count: usize,
    pub occupancy: Vec<u8>,
}

impl TerrainManifest {
    pub fn level(&self, level: u8) -> Option<&TerrainLevelManifest> {
        self.levels.iter().find(|entry| entry.level == level)
    }

    pub fn decode_level(&self, level: u8) -> Result<DecodedTerrainLevel> {
        let Some(raw) = self.level(level) else {
            bail!("terrain level {} missing from manifest", level);
        };
        raw.decode()
    }

    pub fn chunk_url(&self, key: TerrainChunkLodKey) -> String {
        let path = self
            .chunk_path
            .replace("{level}", &key.level.to_string())
            .replace("{x}", &key.cx.to_string())
            .replace("{y}", &key.cy.to_string());
        if self.root.ends_with('/') {
            format!("{}{}", self.root, path)
        } else {
            format!("{}/{}", self.root, path)
        }
    }
}

impl TerrainDrapeManifest {
    pub fn level(&self, level: u8) -> Option<&TerrainLevelManifest> {
        self.levels.iter().find(|entry| entry.level == level)
    }

    pub fn decode_level(&self, level: u8) -> Result<DecodedTerrainLevel> {
        let Some(raw) = self.level(level) else {
            bail!("terrain drape level {} missing from manifest", level);
        };
        raw.decode()
    }

    pub fn chunk_url(&self, key: TerrainChunkLodKey) -> String {
        let path = self
            .chunk_path
            .replace("{level}", &key.level.to_string())
            .replace("{x}", &key.cx.to_string())
            .replace("{y}", &key.cy.to_string());
        if self.root.ends_with('/') {
            format!("{}{}", self.root, path)
        } else {
            format!("{}/{}", self.root, path)
        }
    }
}

impl TerrainLevelManifest {
    pub fn max_x(&self) -> i32 {
        self.min_x + self.width.saturating_sub(1) as i32
    }

    pub fn max_y(&self) -> i32 {
        self.min_y + self.height.saturating_sub(1) as i32
    }

    pub fn decode(&self) -> Result<DecodedTerrainLevel> {
        let expected_len = (self.width as usize)
            .saturating_mul(self.height as usize)
            .div_ceil(8);
        let occupancy = base64::engine::general_purpose::STANDARD
            .decode(self.occupancy_b64.as_bytes())
            .with_context(|| format!("decode occupancy for terrain level {}", self.level))?;
        if occupancy.len() < expected_len {
            bail!(
                "terrain level {} occupancy too short: expected at least {}, got {}",
                self.level,
                expected_len,
                occupancy.len()
            );
        }
        Ok(DecodedTerrainLevel {
            level: self.level,
            min_x: self.min_x,
            min_y: self.min_y,
            width: self.width,
            height: self.height,
            tile_count: self.tile_count,
            occupancy,
        })
    }
}

impl DecodedTerrainLevel {
    pub fn contains(&self, cx: i32, cy: i32) -> bool {
        if cx < self.min_x
            || cy < self.min_y
            || cx > self.max_x()
            || cy > self.max_y()
            || self.width == 0
            || self.height == 0
        {
            return false;
        }
        let gx = (cx - self.min_x) as usize;
        let gy = (cy - self.min_y) as usize;
        let idx = gy.saturating_mul(self.width as usize).saturating_add(gx);
        let byte = idx >> 3;
        let bit = idx & 7;
        self.occupancy
            .get(byte)
            .map(|value| (value & (1_u8 << bit)) != 0)
            .unwrap_or(false)
    }

    pub fn max_x(&self) -> i32 {
        self.min_x + self.width.saturating_sub(1) as i32
    }

    pub fn max_y(&self) -> i32 {
        self.min_y + self.height.saturating_sub(1) as i32
    }
}

pub fn packed_rgb24_to_u32(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

pub fn packed_rgb24_norm_from_rgb(rgb: [u8; 3]) -> f32 {
    packed_rgb24_to_u32(rgb[0], rgb[1], rgb[2]) as f32 / PACKED_RGB24_MAX_U32 as f32
}

pub fn packed_rgb24_norm_from_rgba(rgba: [u8; 4]) -> f32 {
    packed_rgb24_norm_from_rgb([rgba[0], rgba[1], rgba[2]])
}

pub fn world_height_from_normalized(norm: f32, bbox_y_min: f32, bbox_y_max: f32) -> f32 {
    bbox_y_min + norm.clamp(0.0, 1.0) * (bbox_y_max - bbox_y_min)
}

pub fn normalized_height_to_u16(norm: f32) -> u16 {
    (norm.clamp(0.0, 1.0) * U16_HEIGHT_MAX_U16 as f32)
        .round()
        .clamp(0.0, U16_HEIGHT_MAX_U16 as f32) as u16
}

pub fn u16_to_normalized_height(value: u16) -> f32 {
    value as f32 / U16_HEIGHT_MAX_U16 as f32
}

pub fn world_height_from_u16(value: u16, bbox_y_min: f32, bbox_y_max: f32) -> f32 {
    world_height_from_normalized(u16_to_normalized_height(value), bbox_y_min, bbox_y_max)
}

pub fn chunk_span_map_px(chunk_map_px: u32, level: u8) -> u32 {
    let factor = 1_u32.checked_shl(level as u32).unwrap_or(u32::MAX);
    chunk_map_px.max(1).saturating_mul(factor).max(1)
}

pub fn chunk_grid_dims_for_level(
    map_width: u32,
    map_height: u32,
    chunk_map_px: u32,
    level: u8,
) -> (i32, i32) {
    let span = chunk_span_map_px(chunk_map_px, level).max(1);
    let tiles_x = map_width.max(1).div_ceil(span) as i32;
    let tiles_y = map_height.max(1).div_ceil(span) as i32;
    (tiles_x.max(1), tiles_y.max(1))
}

pub fn chunk_map_bounds(
    map_width: u32,
    map_height: u32,
    chunk_map_px: u32,
    key: TerrainChunkLodKey,
) -> (f32, f32, f32, f32) {
    let span = chunk_span_map_px(chunk_map_px, key.level) as f32;
    let x0 = (key.cx as f32 * span).max(0.0);
    let y0 = (key.cy as f32 * span).max(0.0);
    let x1 = (((key.cx + 1) as f32) * span).min(map_width.saturating_sub(1) as f32);
    let y1 = (((key.cy + 1) as f32) * span).min(map_height.saturating_sub(1) as f32);
    (x0, y0, x1, y1)
}

pub fn key_for_map_px(map_x: f32, map_y: f32, chunk_map_px: u32, level: u8) -> TerrainChunkLodKey {
    let span = chunk_span_map_px(chunk_map_px, level).max(1) as f32;
    TerrainChunkLodKey {
        level,
        cx: (map_x / span).floor() as i32,
        cy: (map_y / span).floor() as i32,
    }
}

pub fn parent_chunk_key(key: TerrainChunkLodKey, max_level: u8) -> Option<TerrainChunkLodKey> {
    if key.level >= max_level {
        return None;
    }
    Some(TerrainChunkLodKey {
        level: key.level + 1,
        cx: key.cx.div_euclid(2),
        cy: key.cy.div_euclid(2),
    })
}

pub fn child_chunk_keys(key: TerrainChunkLodKey) -> Option<[TerrainChunkLodKey; 4]> {
    if key.level == 0 {
        return None;
    }
    let level = key.level - 1;
    let x = key.cx.saturating_mul(2);
    let y = key.cy.saturating_mul(2);
    Some([
        TerrainChunkLodKey {
            level,
            cx: x,
            cy: y,
        },
        TerrainChunkLodKey {
            level,
            cx: x + 1,
            cy: y,
        },
        TerrainChunkLodKey {
            level,
            cx: x,
            cy: y + 1,
        },
        TerrainChunkLodKey {
            level,
            cx: x + 1,
            cy: y + 1,
        },
    ])
}

pub fn nearest_available_ancestor<F>(
    mut key: TerrainChunkLodKey,
    max_level: u8,
    mut is_available: F,
) -> Option<TerrainChunkLodKey>
where
    F: FnMut(TerrainChunkLodKey) -> bool,
{
    loop {
        if is_available(key) {
            return Some(key);
        }
        key = parent_chunk_key(key, max_level)?;
    }
}

pub fn resolve_fallback_render_set<F>(
    desired: impl IntoIterator<Item = TerrainChunkLodKey>,
    max_level: u8,
    mut is_available: F,
) -> Vec<TerrainChunkLodKey>
where
    F: FnMut(TerrainChunkLodKey) -> bool,
{
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for key in desired {
        if let Some(ancestor) = nearest_available_ancestor(key, max_level, &mut is_available) {
            if seen.insert(ancestor) {
                out.push(ancestor);
            }
        }
    }
    out
}

pub fn lod_for_view_distance(
    camera_distance_world: f32,
    distance_per_map_px: f32,
    chunk_map_px: u32,
    target_chunks_across_radius: f32,
    max_level: u8,
) -> u8 {
    let px_per_map = distance_per_map_px.max(1e-6);
    let radius_map_px = camera_distance_world.max(1.0) / px_per_map;
    let target_span =
        (radius_map_px / target_chunks_across_radius.max(1.0)).max(chunk_map_px.max(1) as f32);
    let mut level = 0_u8;
    while level < max_level {
        let next = level + 1;
        if chunk_span_map_px(chunk_map_px, next) as f32 > target_span {
            break;
        }
        level = next;
    }
    level
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainChunkData {
    pub key: TerrainChunkLodKey,
    pub grid_size: u16,
    pub encoding: TerrainHeightEncoding,
    pub heights: Vec<u16>,
}

impl TerrainChunkData {
    pub fn expected_samples(&self) -> usize {
        expected_sample_count(self.grid_size)
    }

    pub fn validate(&self) -> Result<()> {
        let expected = self.expected_samples();
        if self.heights.len() != expected {
            bail!(
                "terrain chunk sample count mismatch: {} != {}",
                self.heights.len(),
                expected
            );
        }
        Ok(())
    }
}

pub fn expected_sample_count(grid_size: u16) -> usize {
    let edge = grid_size.max(2) as usize;
    edge.saturating_mul(edge)
}

pub fn encode_terrain_chunk(chunk: &TerrainChunkData) -> Result<Vec<u8>> {
    chunk.validate()?;
    let mut out =
        Vec::with_capacity(TERRAIN_CHUNK_HEADER_LEN + chunk.heights.len().saturating_mul(2));
    out.extend_from_slice(&TERRAIN_CHUNK_MAGIC);
    out.extend_from_slice(&TERRAIN_CHUNK_VERSION.to_le_bytes());
    out.push(chunk.key.level);
    out.push(match chunk.encoding {
        TerrainHeightEncoding::U16Norm => 1,
    });
    out.extend_from_slice(&chunk.grid_size.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(&chunk.key.cx.to_le_bytes());
    out.extend_from_slice(&chunk.key.cy.to_le_bytes());
    out.extend_from_slice(&(chunk.heights.len() as u32).to_le_bytes());
    for value in &chunk.heights {
        out.extend_from_slice(&value.to_le_bytes());
    }
    Ok(out)
}

pub fn decode_terrain_chunk(bytes: &[u8]) -> Result<TerrainChunkData> {
    if bytes.len() < TERRAIN_CHUNK_HEADER_LEN {
        bail!(
            "terrain chunk too short: {} < {}",
            bytes.len(),
            TERRAIN_CHUNK_HEADER_LEN
        );
    }
    let magic: [u8; 4] = bytes[0..4]
        .try_into()
        .map_err(|_| anyhow!("terrain chunk magic read"))?;
    if magic != TERRAIN_CHUNK_MAGIC {
        bail!("terrain chunk magic mismatch");
    }
    let version = u16::from_le_bytes(
        bytes[4..6]
            .try_into()
            .map_err(|_| anyhow!("terrain chunk version read"))?,
    );
    if version != TERRAIN_CHUNK_VERSION {
        bail!(
            "unsupported terrain chunk version: {} (expected {})",
            version,
            TERRAIN_CHUNK_VERSION
        );
    }
    let level = bytes[6];
    let encoding = match bytes[7] {
        1 => TerrainHeightEncoding::U16Norm,
        other => bail!("unsupported terrain chunk encoding byte {}", other),
    };
    let grid_size = u16::from_le_bytes(
        bytes[8..10]
            .try_into()
            .map_err(|_| anyhow!("terrain chunk grid_size read"))?,
    );
    let chunk_x = i32::from_le_bytes(
        bytes[12..16]
            .try_into()
            .map_err(|_| anyhow!("terrain chunk x read"))?,
    );
    let chunk_y = i32::from_le_bytes(
        bytes[16..20]
            .try_into()
            .map_err(|_| anyhow!("terrain chunk y read"))?,
    );
    let sample_count = u32::from_le_bytes(
        bytes[20..24]
            .try_into()
            .map_err(|_| anyhow!("terrain chunk sample_count read"))?,
    ) as usize;

    let expected_samples = expected_sample_count(grid_size);
    if sample_count != expected_samples {
        bail!(
            "terrain chunk sample_count mismatch: {} != expected {}",
            sample_count,
            expected_samples
        );
    }
    let expected_bytes = TERRAIN_CHUNK_HEADER_LEN + sample_count.saturating_mul(2);
    if bytes.len() != expected_bytes {
        bail!(
            "terrain chunk payload length mismatch: {} != {}",
            bytes.len(),
            expected_bytes
        );
    }

    let mut heights = Vec::with_capacity(sample_count);
    let mut cursor = TERRAIN_CHUNK_HEADER_LEN;
    for _ in 0..sample_count {
        let value = u16::from_le_bytes(
            bytes[cursor..cursor + 2]
                .try_into()
                .map_err(|_| anyhow!("terrain chunk sample read"))?,
        );
        heights.push(value);
        cursor += 2;
    }

    Ok(TerrainChunkData {
        key: TerrainChunkLodKey {
            level,
            cx: chunk_x,
            cy: chunk_y,
        },
        grid_size,
        encoding,
        heights,
    })
}

pub fn bilinear_sample_u16_grid(heights: &[u16], grid_size: u16, u: f32, v: f32) -> Option<f32> {
    let edge = grid_size.max(2) as usize;
    let expected = edge.saturating_mul(edge);
    if heights.len() != expected {
        return None;
    }
    let u = u.clamp(0.0, 1.0);
    let v = v.clamp(0.0, 1.0);
    let fx = u * (edge - 1) as f32;
    let fy = v * (edge - 1) as f32;
    let x0 = fx.floor() as usize;
    let y0 = fy.floor() as usize;
    let x1 = (x0 + 1).min(edge - 1);
    let y1 = (y0 + 1).min(edge - 1);
    let tx = (fx - x0 as f32).clamp(0.0, 1.0);
    let ty = (fy - y0 as f32).clamp(0.0, 1.0);
    let idx = |x: usize, y: usize| y * edge + x;
    let h00 = u16_to_normalized_height(heights[idx(x0, y0)]);
    let h10 = u16_to_normalized_height(heights[idx(x1, y0)]);
    let h01 = u16_to_normalized_height(heights[idx(x0, y1)]);
    let h11 = u16_to_normalized_height(heights[idx(x1, y1)]);
    let top = h00 + (h10 - h00) * tx;
    let bot = h01 + (h11 - h01) * tx;
    Some(top + (bot - top) * ty)
}

pub fn sample_chunk_norm_at_map_px(
    map_width: u32,
    map_height: u32,
    chunk_map_px: u32,
    chunk: &TerrainChunkData,
    map_x: f32,
    map_y: f32,
) -> Option<f32> {
    let (x0, y0, x1, y1) = chunk_map_bounds(map_width, map_height, chunk_map_px, chunk.key);
    if map_x < x0 || map_x > x1 || map_y < y0 || map_y > y1 {
        return None;
    }
    let span_x = (x1 - x0).max(1.0);
    let span_y = (y1 - y0).max(1.0);
    let u = (map_x - x0) / span_x;
    let v = (map_y - y0) / span_y;
    bilinear_sample_u16_grid(&chunk.heights, chunk.grid_size, u, v)
}

pub fn chunk_vertex_positions<F>(
    map_width: u32,
    map_height: u32,
    chunk_map_px: u32,
    bbox_y_min: f32,
    bbox_y_max: f32,
    chunk: &TerrainChunkData,
    mut map_to_world_xz: F,
) -> Option<Vec<[f32; 3]>>
where
    F: FnMut(f32, f32) -> (f32, f32),
{
    let edge = chunk.grid_size.max(2) as usize;
    if chunk.heights.len() != edge * edge {
        return None;
    }
    let (x0, y0, x1, y1) = chunk_map_bounds(map_width, map_height, chunk_map_px, chunk.key);
    let mut out = Vec::with_capacity(edge * edge);
    for gy in 0..edge {
        let v = gy as f32 / (edge - 1) as f32;
        let map_y = x0_mul_add(y0, y1, v);
        for gx in 0..edge {
            let u = gx as f32 / (edge - 1) as f32;
            let map_x = x0_mul_add(x0, x1, u);
            let idx = gy * edge + gx;
            let world_y = world_height_from_u16(chunk.heights[idx], bbox_y_min, bbox_y_max);
            let (world_x, world_z) = map_to_world_xz(map_x, map_y);
            out.push([world_x, world_y, world_z]);
        }
    }
    Some(out)
}

pub fn chunk_local_uvs(grid_size: u16) -> Vec<[f32; 2]> {
    let edge = grid_size.max(2) as usize;
    let mut out = Vec::with_capacity(edge * edge);
    for gy in 0..edge {
        let v = gy as f32 / (edge - 1) as f32;
        for gx in 0..edge {
            let u = gx as f32 / (edge - 1) as f32;
            out.push([u, v]);
        }
    }
    out
}

fn x0_mul_add(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{
        bilinear_sample_u16_grid, chunk_grid_dims_for_level, chunk_map_bounds, chunk_span_map_px,
        chunk_vertex_positions, decode_terrain_chunk, encode_terrain_chunk, lod_for_view_distance,
        nearest_available_ancestor, packed_rgb24_norm_from_rgb, packed_rgb24_to_u32,
        resolve_fallback_render_set, sample_chunk_norm_at_map_px, world_height_from_u16,
        TerrainChunkData, TerrainChunkLodKey, TerrainHeightEncoding, U16_HEIGHT_MAX_U16,
    };

    fn test_chunk(level: u8, cx: i32, cy: i32, grid_size: u16, fill: u16) -> TerrainChunkData {
        let samples = (grid_size as usize).saturating_mul(grid_size as usize);
        TerrainChunkData {
            key: TerrainChunkLodKey { level, cx, cy },
            grid_size,
            encoding: TerrainHeightEncoding::U16Norm,
            heights: vec![fill; samples],
        }
    }

    #[test]
    fn packed_rgb24_decode_matches_expected() {
        assert_eq!(packed_rgb24_to_u32(0, 0, 0), 0);
        assert_eq!(packed_rgb24_to_u32(255, 255, 255), 16_777_215);
        let mid = packed_rgb24_norm_from_rgb([128, 0, 0]);
        assert!((mid - (8_388_608.0 / 16_777_215.0)).abs() < 1e-7);
    }

    #[test]
    fn chunk_codec_roundtrips() {
        let mut chunk = test_chunk(2, -3, 4, 5, 1234);
        chunk.heights[0] = 0;
        chunk.heights[1] = U16_HEIGHT_MAX_U16;
        let bytes = encode_terrain_chunk(&chunk).expect("encode");
        let decoded = decode_terrain_chunk(&bytes).expect("decode");
        assert_eq!(chunk, decoded);
    }

    #[test]
    fn chunk_math_scales_with_level() {
        assert_eq!(chunk_span_map_px(256, 0), 256);
        assert_eq!(chunk_span_map_px(256, 1), 512);
        assert_eq!(chunk_span_map_px(256, 3), 2048);
        let (nx0, ny0) = chunk_grid_dims_for_level(11560, 10540, 256, 0);
        let (nx1, ny1) = chunk_grid_dims_for_level(11560, 10540, 256, 1);
        assert!(nx1 < nx0);
        assert!(ny1 < ny0);
    }

    #[test]
    fn nearest_ancestor_finds_loaded_parent() {
        let loaded: HashSet<TerrainChunkLodKey> = [
            TerrainChunkLodKey {
                level: 3,
                cx: 0,
                cy: 0,
            },
            TerrainChunkLodKey {
                level: 2,
                cx: 1,
                cy: 1,
            },
        ]
        .into_iter()
        .collect();
        let key = TerrainChunkLodKey {
            level: 0,
            cx: 7,
            cy: 7,
        };
        let found =
            nearest_available_ancestor(key, 3, |candidate| loaded.contains(&candidate)).unwrap();
        assert_eq!(
            found,
            TerrainChunkLodKey {
                level: 2,
                cx: 1,
                cy: 1
            }
        );
    }

    #[test]
    fn no_gap_fallback_uses_common_ancestor() {
        let parent = TerrainChunkLodKey {
            level: 1,
            cx: 10,
            cy: 12,
        };
        let desired = [
            TerrainChunkLodKey {
                level: 0,
                cx: 20,
                cy: 24,
            },
            TerrainChunkLodKey {
                level: 0,
                cx: 21,
                cy: 24,
            },
            TerrainChunkLodKey {
                level: 0,
                cx: 20,
                cy: 25,
            },
            TerrainChunkLodKey {
                level: 0,
                cx: 21,
                cy: 25,
            },
        ];
        let resolved = resolve_fallback_render_set(desired, 4, |key| key == parent);
        assert_eq!(resolved, vec![parent]);
    }

    #[test]
    fn lod_choice_increases_with_distance() {
        let near = lod_for_view_distance(2_000.0, 301.17648, 256, 5.0, 7);
        let mid = lod_for_view_distance(40_000.0, 301.17648, 256, 5.0, 7);
        let far = lod_for_view_distance(200_000.0, 301.17648, 256, 5.0, 7);
        assert!(near <= mid);
        assert!(mid <= far);
    }

    #[test]
    fn bilinear_grid_sampling_is_scalar_space() {
        let heights = vec![0, U16_HEIGHT_MAX_U16, U16_HEIGHT_MAX_U16, 0];
        let value = bilinear_sample_u16_grid(&heights, 2, 0.5, 0.5).expect("sample");
        assert!((value - 0.5).abs() < 1e-6);
    }

    #[test]
    fn sample_chunk_map_px_returns_expected_value() {
        let chunk = TerrainChunkData {
            key: TerrainChunkLodKey {
                level: 0,
                cx: 0,
                cy: 0,
            },
            grid_size: 2,
            encoding: TerrainHeightEncoding::U16Norm,
            heights: vec![0, U16_HEIGHT_MAX_U16, U16_HEIGHT_MAX_U16, 0],
        };
        let value =
            sample_chunk_norm_at_map_px(11560, 10540, 256, &chunk, 128.0, 128.0).expect("sample");
        assert!((value - 0.5).abs() < 1e-6);
    }

    #[test]
    fn world_vertex_generation_uses_u16_decoding() {
        let chunk = test_chunk(0, 0, 0, 2, U16_HEIGHT_MAX_U16);
        let vertices =
            chunk_vertex_positions(11560, 10540, 256, -100.0, 900.0, &chunk, |x, y| (x, y))
                .expect("vertices");
        assert_eq!(vertices.len(), 4);
        for vertex in vertices {
            assert!((vertex[1] - 900.0).abs() < 1e-5);
        }
    }

    #[test]
    fn world_height_decoding_matches_extrema() {
        assert!((world_height_from_u16(0, -500.0, 1000.0) - -500.0).abs() < 1e-6);
        assert!((world_height_from_u16(U16_HEIGHT_MAX_U16, -500.0, 1000.0) - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn chunk_bounds_match_expected_extent() {
        let bounds = chunk_map_bounds(
            11560,
            10540,
            256,
            TerrainChunkLodKey {
                level: 2,
                cx: 1,
                cy: 1,
            },
        );
        assert!((bounds.0 - 1024.0).abs() < 1e-6);
        assert!((bounds.1 - 1024.0).abs() < 1e-6);
    }
}
