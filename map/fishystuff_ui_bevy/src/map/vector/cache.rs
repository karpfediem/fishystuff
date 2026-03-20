use std::collections::{HashMap, VecDeque};

use bevy::prelude::{
    Assets, Color, ColorMaterial, Commands, Entity, Handle, StandardMaterial, Transform, Vec3,
    Visibility,
};
use serde_json::{Map, Value};

use crate::map::layers::LayerId;

#[derive(Debug, Clone, Copy, Default)]
pub struct VectorLayerStats {
    pub fetched_bytes: u32,
    pub feature_count: u32,
    pub features_processed: u32,
    pub polygon_count: u32,
    pub multipolygon_count: u32,
    pub hole_ring_count: u32,
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub build_ms: f32,
    pub last_frame_build_ms: f32,
    pub progress: f32,
    pub mesh_count: u32,
    pub chunked_bucket_count: u32,
}

#[derive(Debug, Clone)]
pub struct BuiltVectorChunk {
    pub color_rgba: [u8; 4],
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub min_world_x: f32,
    pub max_world_x: f32,
    pub min_world_z: f32,
    pub max_world_z: f32,
}

#[derive(Debug)]
pub struct BuiltVectorGeometry {
    pub chunks: Vec<BuiltVectorChunk>,
    pub hover_features: Vec<HoverFeature>,
    pub stats: VectorLayerStats,
}

#[derive(Debug)]
pub struct VectorMeshChunk {
    pub entity_2d: Entity,
    pub material_2d: Handle<ColorMaterial>,
    pub entity_3d: Entity,
    pub material_3d: Handle<StandardMaterial>,
}

#[derive(Debug, Default)]
pub struct VectorMeshBundleSet {
    pub chunks: Vec<VectorMeshChunk>,
    pub hover_chunks: Vec<BuiltVectorChunk>,
    pub hover_features: Vec<HoverFeature>,
    pub stats: VectorLayerStats,
}

#[derive(Debug, Clone)]
pub struct HoverFeature {
    pub properties: Map<String, Value>,
    pub polygons: Vec<HoverPolygon>,
    pub min_world_x: f32,
    pub max_world_x: f32,
    pub min_world_z: f32,
    pub max_world_z: f32,
}

#[derive(Debug, Clone)]
pub struct HoverPolygon {
    pub rings: Vec<Vec<[f32; 2]>>,
    pub min_world_x: f32,
    pub max_world_x: f32,
    pub min_world_z: f32,
    pub max_world_z: f32,
}

impl VectorMeshBundleSet {
    pub fn set_depth(&self, commands: &mut Commands, z_base: f32, y_base: f32) {
        for chunk in &self.chunks {
            commands
                .entity(chunk.entity_2d)
                .insert(Transform::from_translation(Vec3::new(0.0, 0.0, z_base)));
            commands
                .entity(chunk.entity_3d)
                .insert(Transform::from_translation(Vec3::new(
                    0.0,
                    y_base + z_base,
                    0.0,
                )));
        }
    }

    pub fn set_visibility(&self, commands: &mut Commands, visible: bool) {
        let visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        for chunk in &self.chunks {
            commands.entity(chunk.entity_2d).insert(visibility);
            commands.entity(chunk.entity_3d).insert(visibility);
        }
    }

    pub fn set_opacity(
        &self,
        materials_2d: &mut Assets<ColorMaterial>,
        materials_3d: &mut Assets<StandardMaterial>,
        opacity: f32,
    ) {
        let alpha = opacity.clamp(0.0, 1.0);
        for chunk in &self.chunks {
            if let Some(material) = materials_2d.get_mut(&chunk.material_2d) {
                let base = material.color.to_srgba();
                material.color = Color::srgba(base.red, base.green, base.blue, alpha);
            }
            if let Some(material) = materials_3d.get_mut(&chunk.material_3d) {
                let base = material.base_color.to_srgba();
                material.base_color = Color::srgba(base.red, base.green, base.blue, alpha);
                material.alpha_mode = if alpha >= 0.999 {
                    bevy::prelude::AlphaMode::Opaque
                } else {
                    bevy::prelude::AlphaMode::Blend
                };
            }
        }
    }

    pub fn despawn(self, commands: &mut Commands) {
        for chunk in self.chunks {
            commands.entity(chunk.entity_2d).despawn();
            commands.entity(chunk.entity_3d).despawn();
        }
    }

    pub fn sample_rgb(&self, world_x: f32, world_z: f32) -> Option<[u8; 4]> {
        for chunk in self.hover_chunks.iter().rev() {
            if chunk.color_rgba[3] == 0
                || world_x < chunk.min_world_x
                || world_x > chunk.max_world_x
                || world_z < chunk.min_world_z
                || world_z > chunk.max_world_z
            {
                continue;
            }
            if chunk_contains_point(chunk, world_x, world_z) {
                return Some(chunk.color_rgba);
            }
        }
        None
    }

    pub fn sample_properties(&self, world_x: f32, world_z: f32) -> Option<&Map<String, Value>> {
        for feature in self.hover_features.iter().rev() {
            if world_x < feature.min_world_x
                || world_x > feature.max_world_x
                || world_z < feature.min_world_z
                || world_z > feature.max_world_z
            {
                continue;
            }
            if feature_contains_point(feature, world_x, world_z) {
                return Some(&feature.properties);
            }
        }
        None
    }
}

fn chunk_contains_point(chunk: &BuiltVectorChunk, world_x: f32, world_z: f32) -> bool {
    for triangle in chunk.indices.chunks_exact(3) {
        let Some(a) = chunk.positions.get(triangle[0] as usize) else {
            continue;
        };
        let Some(b) = chunk.positions.get(triangle[1] as usize) else {
            continue;
        };
        let Some(c) = chunk.positions.get(triangle[2] as usize) else {
            continue;
        };
        if point_in_triangle_2d(world_x, world_z, a, b, c) {
            return true;
        }
    }
    false
}

fn feature_contains_point(feature: &HoverFeature, world_x: f32, world_z: f32) -> bool {
    feature
        .polygons
        .iter()
        .any(|polygon| polygon_contains_point(polygon, world_x, world_z))
}

fn polygon_contains_point(polygon: &HoverPolygon, world_x: f32, world_z: f32) -> bool {
    if world_x < polygon.min_world_x
        || world_x > polygon.max_world_x
        || world_z < polygon.min_world_z
        || world_z > polygon.max_world_z
    {
        return false;
    }
    let Some(outer_ring) = polygon.rings.first() else {
        return false;
    };
    if !point_in_ring_2d(world_x, world_z, outer_ring) {
        return false;
    }
    !polygon
        .rings
        .iter()
        .skip(1)
        .any(|ring| point_in_ring_2d(world_x, world_z, ring))
}

fn point_in_triangle_2d(px: f32, pz: f32, a: &[f32; 3], b: &[f32; 3], c: &[f32; 3]) -> bool {
    let area = edge_fn(a[0], a[1], b[0], b[1], c[0], c[1]);
    if area.abs() <= f32::EPSILON {
        return false;
    }
    let w0 = edge_fn(b[0], b[1], c[0], c[1], px, pz);
    let w1 = edge_fn(c[0], c[1], a[0], a[1], px, pz);
    let w2 = edge_fn(a[0], a[1], b[0], b[1], px, pz);
    let has_neg = w0 < 0.0 || w1 < 0.0 || w2 < 0.0;
    let has_pos = w0 > 0.0 || w1 > 0.0 || w2 > 0.0;
    !(has_neg && has_pos)
}

fn edge_fn(ax: f32, az: f32, bx: f32, bz: f32, px: f32, pz: f32) -> f32 {
    (px - ax) * (bz - az) - (pz - az) * (bx - ax)
}

fn point_in_ring_2d(px: f32, pz: f32, ring: &[[f32; 2]]) -> bool {
    if ring.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = ring[ring.len() - 1];
    for current in ring {
        let x0 = previous[0];
        let z0 = previous[1];
        let x1 = current[0];
        let z1 = current[1];
        let z_delta = z1 - z0;
        let intersects = ((z0 > pz) != (z1 > pz))
            && (px
                < (x1 - x0) * (pz - z0)
                    / if z_delta.abs() <= f32::EPSILON {
                        f32::EPSILON
                    } else {
                        z_delta
                    }
                    + x0);
        if intersects {
            inside = !inside;
        }
        previous = *current;
    }
    inside
}

#[derive(Debug, Clone, Copy, Default)]
pub struct VectorCacheTelemetry {
    pub hits: u64,
    pub misses: u64,
}

#[derive(Debug, Default)]
pub struct VectorFinishedCache {
    max_entries: usize,
    entries: HashMap<(LayerId, String), VectorMeshBundleSet>,
    lru: VecDeque<(LayerId, String)>,
    telemetry: VectorCacheTelemetry,
}

impl VectorFinishedCache {
    pub fn with_capacity(max_entries: usize) -> Self {
        Self {
            max_entries,
            entries: HashMap::new(),
            lru: VecDeque::new(),
            telemetry: VectorCacheTelemetry::default(),
        }
    }

    pub fn set_max_entries(&mut self, max_entries: usize) {
        self.max_entries = max_entries;
    }

    pub fn telemetry(&self) -> VectorCacheTelemetry {
        self.telemetry
    }

    pub fn get(&mut self, key: &(LayerId, String)) -> Option<&VectorMeshBundleSet> {
        if self.entries.contains_key(key) {
            self.telemetry.hits = self.telemetry.hits.saturating_add(1);
            self.touch(key);
            self.entries.get(key)
        } else {
            self.telemetry.misses = self.telemetry.misses.saturating_add(1);
            None
        }
    }

    pub fn get_ref(&self, key: &(LayerId, String)) -> Option<&VectorMeshBundleSet> {
        self.entries.get(key)
    }

    pub fn insert(
        &mut self,
        key: (LayerId, String),
        value: VectorMeshBundleSet,
    ) -> Option<VectorMeshBundleSet> {
        self.touch(&key);
        self.entries.insert(key, value)
    }

    pub fn remove(&mut self, key: &(LayerId, String)) -> Option<VectorMeshBundleSet> {
        self.lru.retain(|candidate| candidate != key);
        self.entries.remove(key)
    }

    pub fn clear(&mut self) -> Vec<VectorMeshBundleSet> {
        self.lru.clear();
        self.entries.drain().map(|(_, value)| value).collect()
    }

    pub fn keys_for_layer(&self, layer_id: LayerId) -> Vec<(LayerId, String)> {
        self.entries
            .keys()
            .filter(|(candidate_id, _)| *candidate_id == layer_id)
            .cloned()
            .collect()
    }

    pub fn keys(&self) -> Vec<(LayerId, String)> {
        self.entries.keys().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn layer_len(&self, layer_id: LayerId) -> usize {
        self.entries
            .keys()
            .filter(|(candidate_id, _)| *candidate_id == layer_id)
            .count()
    }

    pub fn evict_lru_non_visible<F>(&mut self, mut is_visible: F) -> Vec<VectorMeshBundleSet>
    where
        F: FnMut(&(LayerId, String)) -> bool,
    {
        let mut evicted = Vec::new();
        while self.entries.len() > self.max_entries {
            let Some(index) = self.lru.iter().position(|key| !is_visible(key)) else {
                break;
            };
            let Some(key) = self.lru.remove(index) else {
                break;
            };
            if let Some(bundle) = self.entries.remove(&key) {
                evicted.push(bundle);
            }
        }
        evicted
    }

    pub fn remove_layer_except(
        &mut self,
        layer_id: LayerId,
        keep_revision: &str,
    ) -> Vec<VectorMeshBundleSet> {
        let keys = self.keys_for_layer(layer_id);
        let mut removed = Vec::new();
        for key in keys {
            if key.1 != keep_revision {
                if let Some(bundle) = self.remove(&key) {
                    removed.push(bundle);
                }
            }
        }
        removed
    }

    fn touch(&mut self, key: &(LayerId, String)) {
        self.lru.retain(|candidate| candidate != key);
        self.lru.push_back(key.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::{HoverFeature, HoverPolygon, VectorFinishedCache, VectorMeshBundleSet};
    use crate::map::layers::LayerId;
    use serde_json::{Map, Value};

    #[test]
    fn evicts_lru_non_visible_entry_when_over_capacity() {
        let mut cache = VectorFinishedCache::with_capacity(1);
        let k1 = (LayerId::from_raw(1), "rev-a".to_string());
        let k2 = (LayerId::from_raw(2), "rev-b".to_string());
        cache.insert(k1.clone(), VectorMeshBundleSet::default());
        cache.insert(k2.clone(), VectorMeshBundleSet::default());
        let evicted = cache.evict_lru_non_visible(|key| key == &k2);
        assert_eq!(evicted.len(), 1);
        assert!(cache.get_ref(&k1).is_none());
        assert!(cache.get_ref(&k2).is_some());
    }

    #[test]
    fn visibility_toggle_can_reuse_cached_entry_without_rebuild() {
        let mut cache = VectorFinishedCache::with_capacity(2);
        let key = (LayerId::from_raw(7), "rev".to_string());
        cache.insert(key.clone(), VectorMeshBundleSet::default());
        assert!(cache.get(&key).is_some());
        assert!(cache.get_ref(&key).is_some());
        let evicted = cache.evict_lru_non_visible(|_| true);
        assert!(evicted.is_empty());
        assert!(cache.get_ref(&key).is_some());
    }

    #[test]
    fn remove_layer_except_keeps_only_active_revision() {
        let mut cache = VectorFinishedCache::with_capacity(4);
        let layer_id = LayerId::from_raw(3);
        let k1 = (layer_id, "rev-a".to_string());
        let k2 = (layer_id, "rev-b".to_string());
        let k3 = (LayerId::from_raw(9), "rev-z".to_string());
        cache.insert(k1.clone(), VectorMeshBundleSet::default());
        cache.insert(k2.clone(), VectorMeshBundleSet::default());
        cache.insert(k3.clone(), VectorMeshBundleSet::default());

        let removed = cache.remove_layer_except(layer_id, "rev-b");
        assert_eq!(removed.len(), 1);
        assert!(cache.get_ref(&k1).is_none());
        assert!(cache.get_ref(&k2).is_some());
        assert!(cache.get_ref(&k3).is_some());
    }

    #[test]
    fn hover_feature_sampling_respects_polygon_holes() {
        let mut properties = Map::new();
        properties.insert("rg".to_string(), Value::from(118u32));
        let bundle = VectorMeshBundleSet {
            hover_features: vec![HoverFeature {
                properties,
                polygons: vec![HoverPolygon {
                    rings: vec![
                        vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
                        vec![[3.0, 3.0], [7.0, 3.0], [7.0, 7.0], [3.0, 7.0]],
                    ],
                    min_world_x: 0.0,
                    max_world_x: 10.0,
                    min_world_z: 0.0,
                    max_world_z: 10.0,
                }],
                min_world_x: 0.0,
                max_world_x: 10.0,
                min_world_z: 0.0,
                max_world_z: 10.0,
            }],
            ..VectorMeshBundleSet::default()
        };

        assert!(bundle.sample_properties(1.0, 1.0).is_some());
        assert!(bundle.sample_properties(5.0, 5.0).is_none());
    }
}
