use std::collections::{HashMap, HashSet};

use bevy::prelude::Resource;

use super::{LayerId, LayerRegistry, LayerSpec, LayerVectorStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayerManifestStatus {
    #[default]
    Missing,
    Loading,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Copy)]
pub struct LayerRuntimeState {
    pub visible: bool,
    pub opacity: f32,
    pub clip_mask_layer: Option<LayerId>,
    pub z_base: f32,
    pub display_order: i32,
    pub current_base_lod: Option<u8>,
    pub current_detail_lod: Option<u8>,
    pub last_view_update_frame: u64,
    pub visible_tile_count: u32,
    pub resident_tile_count: u32,
    pub pending_count: u32,
    pub inflight_count: u32,
    pub manifest_status: LayerManifestStatus,
    pub vector_status: LayerVectorStatus,
    pub vector_progress: f32,
    pub vector_fetched_bytes: u32,
    pub vector_feature_count: u32,
    pub vector_features_processed: u32,
    pub vector_polygon_count: u32,
    pub vector_multipolygon_count: u32,
    pub vector_hole_ring_count: u32,
    pub vector_vertex_count: u32,
    pub vector_triangle_count: u32,
    pub vector_mesh_count: u32,
    pub vector_chunked_bucket_count: u32,
    pub vector_build_ms: f32,
    pub vector_last_frame_build_ms: f32,
    pub vector_cache_hits: u32,
    pub vector_cache_misses: u32,
    pub vector_cache_last_hit: bool,
    pub vector_cache_entries: u32,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct LayerRuntime {
    states: HashMap<LayerId, LayerRuntimeState>,
}

impl LayerRuntime {
    pub fn get(&self, id: LayerId) -> Option<LayerRuntimeState> {
        self.states.get(&id).copied()
    }

    pub fn get_mut(&mut self, id: LayerId) -> Option<&mut LayerRuntimeState> {
        self.states.get_mut(&id)
    }

    pub fn sync_to_registry(&mut self, registry: &LayerRegistry) {
        let valid_ids = registry
            .ordered()
            .iter()
            .map(|spec| spec.id)
            .collect::<HashSet<_>>();
        self.states
            .retain(|layer_id, _| valid_ids.contains(layer_id));

        for spec in registry.ordered() {
            let state = self
                .states
                .entry(spec.id)
                .or_insert_with(|| default_state_for_spec(spec));
            if spec.is_vector() {
                if state.vector_status == LayerVectorStatus::Inactive {
                    state.vector_status = LayerVectorStatus::NotRequested;
                }
            } else {
                state.vector_status = LayerVectorStatus::Inactive;
            }
            state.z_base = spec.z_base;
            state.display_order = spec.display_order;
        }
    }

    pub fn reset_from_registry(&mut self, registry: &LayerRegistry) {
        let mut states = HashMap::with_capacity(registry.ordered().len());
        for spec in registry.ordered() {
            states.insert(spec.id, default_state_for_spec(spec));
        }
        self.states = states;
    }

    pub fn visible(&self, id: LayerId) -> bool {
        self.get(id).map(|s| s.visible).unwrap_or(false)
    }

    pub fn opacity(&self, id: LayerId) -> f32 {
        self.get(id).map(|s| s.opacity).unwrap_or(1.0)
    }

    pub fn set_visible(&mut self, id: LayerId, visible: bool) {
        if let Some(value) = self.states.get_mut(&id) {
            value.visible = visible;
        }
    }

    pub fn set_opacity(&mut self, id: LayerId, opacity: f32) {
        if let Some(value) = self.states.get_mut(&id) {
            value.opacity = opacity.clamp(0.0, 1.0);
        }
    }

    pub fn clip_mask_layer(&self, id: LayerId) -> Option<LayerId> {
        self.get(id).and_then(|state| state.clip_mask_layer)
    }

    pub fn set_clip_mask(&mut self, id: LayerId, clip_mask_layer: Option<LayerId>) {
        if let Some(value) = self.states.get_mut(&id) {
            value.clip_mask_layer = clip_mask_layer.filter(|mask_layer| *mask_layer != id);
        }
    }

    pub fn clear_clip_masks(&mut self) {
        for value in self.states.values_mut() {
            value.clip_mask_layer = None;
        }
    }

    pub fn clip_mask_source_ids(&self) -> HashSet<LayerId> {
        self.states
            .values()
            .filter_map(|state| state.clip_mask_layer)
            .collect()
    }

    pub fn z_base(&self, id: LayerId) -> f32 {
        self.get(id).map(|s| s.z_base).unwrap_or(0.0)
    }

    pub fn display_order(&self, id: LayerId) -> i32 {
        self.get(id).map(|s| s.display_order).unwrap_or_default()
    }

    pub fn set_stack(&mut self, id: LayerId, display_order: i32, z_base: f32) {
        if let Some(value) = self.states.get_mut(&id) {
            value.display_order = display_order;
            value.z_base = z_base;
        }
    }

    pub fn reset_runtime_metrics(&mut self) {
        for state in self.states.values_mut() {
            state.current_base_lod = None;
            state.current_detail_lod = None;
            state.visible_tile_count = 0;
            state.resident_tile_count = 0;
            state.pending_count = 0;
            state.inflight_count = 0;
            state.vector_progress = 0.0;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (LayerId, LayerRuntimeState)> + '_ {
        self.states.iter().map(|(id, value)| (*id, *value))
    }
}

fn default_state_for_spec(spec: &LayerSpec) -> LayerRuntimeState {
    LayerRuntimeState {
        visible: spec.visible_default,
        opacity: spec.opacity_default,
        clip_mask_layer: None,
        z_base: spec.z_base,
        display_order: spec.display_order,
        current_base_lod: None,
        current_detail_lod: None,
        last_view_update_frame: 0,
        visible_tile_count: 0,
        resident_tile_count: 0,
        pending_count: 0,
        inflight_count: 0,
        manifest_status: LayerManifestStatus::Missing,
        vector_status: if spec.is_vector() {
            LayerVectorStatus::NotRequested
        } else {
            LayerVectorStatus::Inactive
        },
        vector_progress: 0.0,
        vector_fetched_bytes: 0,
        vector_feature_count: 0,
        vector_features_processed: 0,
        vector_polygon_count: 0,
        vector_multipolygon_count: 0,
        vector_hole_ring_count: 0,
        vector_vertex_count: 0,
        vector_triangle_count: 0,
        vector_mesh_count: 0,
        vector_chunked_bucket_count: 0,
        vector_build_ms: 0.0,
        vector_last_frame_build_ms: 0.0,
        vector_cache_hits: 0,
        vector_cache_misses: 0,
        vector_cache_last_hit: false,
        vector_cache_entries: 0,
    }
}

pub type LayerSettings = LayerRuntime;
