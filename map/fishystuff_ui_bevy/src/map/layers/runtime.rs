use std::collections::{HashMap, HashSet};

use bevy::prelude::Resource;

use super::{LayerId, LayerRegistry, LayerSpec, LayerVectorStatus, FISH_EVIDENCE_LAYER_KEY};

const POINT_ICON_SCALE_MIN: f32 = 1.0;
const POINT_ICON_SCALE_MAX: f32 = 5.0;
const POINT_ICON_SCALE_DEFAULT: f32 = 2.0;

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
    pub waypoint_connections_visible: bool,
    pub waypoint_labels_visible: bool,
    pub point_icons_visible: bool,
    pub point_icon_scale: f32,
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
            if let Some(source) = spec.waypoint_source.as_ref() {
                if !source.supports_connections {
                    state.waypoint_connections_visible = false;
                }
                if !source.supports_labels {
                    state.waypoint_labels_visible = false;
                }
            } else {
                state.waypoint_connections_visible = false;
                state.waypoint_labels_visible = false;
            }
            if !supports_point_icons(spec) {
                state.point_icons_visible = false;
                state.point_icon_scale = POINT_ICON_SCALE_DEFAULT;
            } else {
                state.point_icon_scale = state
                    .point_icon_scale
                    .clamp(POINT_ICON_SCALE_MIN, POINT_ICON_SCALE_MAX);
            }
            // Preserve runtime stack overrides across repeated syncs. Browser-applied
            // layer order/stack changes are stored in runtime state and should not be
            // reset to catalog defaults by unrelated per-frame registry syncs.
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

    pub fn waypoint_connections_visible(&self, id: LayerId) -> bool {
        self.get(id)
            .map(|state| state.waypoint_connections_visible)
            .unwrap_or(false)
    }

    pub fn set_waypoint_connections_visible(&mut self, id: LayerId, visible: bool) {
        if let Some(value) = self.states.get_mut(&id) {
            value.waypoint_connections_visible = visible;
        }
    }

    pub fn waypoint_labels_visible(&self, id: LayerId) -> bool {
        self.get(id)
            .map(|state| state.waypoint_labels_visible)
            .unwrap_or(false)
    }

    pub fn set_waypoint_labels_visible(&mut self, id: LayerId, visible: bool) {
        if let Some(value) = self.states.get_mut(&id) {
            value.waypoint_labels_visible = visible;
        }
    }

    pub fn point_icons_visible(&self, id: LayerId) -> bool {
        self.get(id)
            .map(|state| state.point_icons_visible)
            .unwrap_or(false)
    }

    pub fn set_point_icons_visible(&mut self, id: LayerId, visible: bool) {
        if let Some(value) = self.states.get_mut(&id) {
            value.point_icons_visible = visible;
        }
    }

    pub fn point_icon_scale(&self, id: LayerId) -> f32 {
        self.get(id)
            .map(|state| state.point_icon_scale)
            .unwrap_or(POINT_ICON_SCALE_DEFAULT)
    }

    pub fn set_point_icon_scale(&mut self, id: LayerId, scale: f32) {
        if let Some(value) = self.states.get_mut(&id) {
            value.point_icon_scale = scale.clamp(POINT_ICON_SCALE_MIN, POINT_ICON_SCALE_MAX);
        }
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
        waypoint_connections_visible: spec
            .waypoint_source
            .as_ref()
            .is_some_and(|source| source.supports_connections && source.show_connections_default),
        waypoint_labels_visible: spec
            .waypoint_source
            .as_ref()
            .is_some_and(|source| source.supports_labels && source.show_labels_default),
        point_icons_visible: supports_point_icons(spec),
        point_icon_scale: POINT_ICON_SCALE_DEFAULT,
        z_base: spec.z_base,
        display_order: spec.display_order,
        current_base_lod: None,
        current_detail_lod: None,
        last_view_update_frame: 0,
        visible_tile_count: 0,
        resident_tile_count: 0,
        pending_count: 0,
        inflight_count: 0,
        manifest_status: if spec.is_waypoints() && spec.waypoint_source.is_none() {
            LayerManifestStatus::Ready
        } else {
            LayerManifestStatus::Missing
        },
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

fn supports_point_icons(spec: &LayerSpec) -> bool {
    spec.key == FISH_EVIDENCE_LAYER_KEY
}

pub type LayerSettings = LayerRuntime;
