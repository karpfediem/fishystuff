use super::super::super::*;
use crate::map::layers::LayerManifestStatus;

fn is_fish_evidence_layer(layer: &crate::map::layers::LayerSpec) -> bool {
    layer.key == crate::map::layers::FISH_EVIDENCE_LAYER_KEY
}

pub(in crate::bridge::host) fn current_layer_order<'a>(
    layer_registry: &'a LayerRegistry,
    layer_runtime: &LayerRuntime,
) -> Vec<&'a crate::map::layers::LayerSpec> {
    let mut ordered = layer_registry.ordered().iter().collect::<Vec<_>>();
    ordered.sort_by(|lhs, rhs| {
        layer_runtime
            .get(rhs.id)
            .map(|state| state.display_order)
            .unwrap_or(rhs.display_order)
            .cmp(
                &layer_runtime
                    .get(lhs.id)
                    .map(|state| state.display_order)
                    .unwrap_or(lhs.display_order),
            )
            .then_with(|| rhs.display_order.cmp(&lhs.display_order))
            .then_with(|| lhs.key.cmp(&rhs.key))
    });
    ordered
}

pub(in crate::bridge::host::snapshot) fn current_layer_summaries(
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
    filter_binding_overrides: &LayerFilterBindingOverrideState,
) -> Vec<FishyMapLayerSummary> {
    current_layer_order(layer_registry, layer_runtime)
        .into_iter()
        .map(|layer| {
            let runtime_state = layer_runtime.get(layer.id);
            let is_fish_evidence = is_fish_evidence_layer(layer);
            FishyMapLayerSummary {
                layer_id: layer.key.clone(),
                name: layer.name.clone(),
                visible: runtime_state
                    .map(|state| state.visible)
                    .unwrap_or(layer.visible_default),
                opacity: runtime_state
                    .map(|state| state.opacity)
                    .unwrap_or(layer.opacity_default),
                opacity_default: layer.opacity_default,
                display_order: runtime_state
                    .map(|state| state.display_order)
                    .unwrap_or(layer.display_order),
                kind: if is_fish_evidence {
                    "fish-evidence".to_string()
                } else {
                    match layer.kind {
                        LayerKind::TiledRaster => "tiled-raster".to_string(),
                        LayerKind::VectorGeoJson => "vector-geojson".to_string(),
                        LayerKind::Waypoints => "waypoints".to_string(),
                    }
                },
                visible_tile_count: runtime_state
                    .map(|state| state.visible_tile_count)
                    .unwrap_or_default(),
                resident_tile_count: runtime_state
                    .map(|state| state.resident_tile_count)
                    .unwrap_or_default(),
                pending_count: runtime_state
                    .map(|state| state.pending_count)
                    .unwrap_or_default(),
                inflight_count: runtime_state
                    .map(|state| state.inflight_count)
                    .unwrap_or_default(),
                manifest_status: runtime_state
                    .map(|state| manifest_status_label(state.manifest_status).to_string())
                    .unwrap_or_else(|| "missing".to_string()),
                vector_status: runtime_state
                    .map(|state| vector_status_label(state.vector_status).to_string())
                    .unwrap_or_else(|| "inactive".to_string()),
                vector_progress: runtime_state
                    .map(|state| state.vector_progress)
                    .unwrap_or_default(),
                vector_feature_count: runtime_state
                    .map(|state| state.vector_feature_count)
                    .unwrap_or_default(),
                vector_vertex_count: runtime_state
                    .map(|state| state.vector_vertex_count)
                    .unwrap_or_default(),
                vector_triangle_count: runtime_state
                    .map(|state| state.vector_triangle_count)
                    .unwrap_or_default(),
                vector_mesh_count: runtime_state
                    .map(|state| state.vector_mesh_count)
                    .unwrap_or_default(),
                vector_chunked_bucket_count: runtime_state
                    .map(|state| state.vector_chunked_bucket_count)
                    .unwrap_or_default(),
                vector_build_ms: runtime_state
                    .map(|state| state.vector_build_ms)
                    .unwrap_or_default(),
                vector_last_frame_build_ms: runtime_state
                    .map(|state| state.vector_last_frame_build_ms)
                    .unwrap_or_default(),
                vector_cache_entries: runtime_state
                    .map(|state| state.vector_cache_entries)
                    .unwrap_or_default(),
                supports_waypoint_connections: layer
                    .waypoint_source
                    .as_ref()
                    .is_some_and(|source| source.supports_connections),
                waypoint_connections_visible: runtime_state
                    .map(|state| state.waypoint_connections_visible)
                    .unwrap_or_else(|| {
                        layer.waypoint_source.as_ref().is_some_and(|source| {
                            source.supports_connections && source.show_connections_default
                        })
                    }),
                waypoint_connections_default: layer.waypoint_source.as_ref().is_some_and(
                    |source| source.supports_connections && source.show_connections_default,
                ),
                supports_waypoint_labels: layer
                    .waypoint_source
                    .as_ref()
                    .is_some_and(|source| source.supports_labels),
                waypoint_labels_visible: runtime_state
                    .map(|state| state.waypoint_labels_visible)
                    .unwrap_or_else(|| {
                        layer.waypoint_source.as_ref().is_some_and(|source| {
                            source.supports_labels && source.show_labels_default
                        })
                    }),
                waypoint_labels_default: layer
                    .waypoint_source
                    .as_ref()
                    .is_some_and(|source| source.supports_labels && source.show_labels_default),
                supports_point_icons: is_fish_evidence,
                point_icons_visible: runtime_state
                    .map(|state| state.point_icons_visible)
                    .unwrap_or(is_fish_evidence),
                point_icons_default: is_fish_evidence,
                point_icon_scale: runtime_state
                    .map(|state| state.point_icon_scale)
                    .unwrap_or(crate::bridge::contract::FISHYMAP_POINT_ICON_SCALE_MIN),
                point_icon_scale_default: crate::bridge::contract::FISHYMAP_POINT_ICON_SCALE_MIN,
                filter_bindings: layer
                    .filter_bindings
                    .iter()
                    .map(|binding| FishyMapLayerFilterBindingSummary {
                        binding_id: binding.binding_id.clone(),
                        source: layer_filter_source_label(binding.source).to_string(),
                        target: layer_filter_target_label(binding.target).to_string(),
                        enabled: filter_binding_overrides.is_binding_enabled(layer, binding),
                        default_enabled: binding.default_enabled,
                    })
                    .collect(),
            }
        })
        .collect()
}

fn layer_filter_source_label(source: crate::map::layers::LayerFilterSourceKind) -> &'static str {
    match source {
        crate::map::layers::LayerFilterSourceKind::FishSelection => "fish_selection",
        crate::map::layers::LayerFilterSourceKind::ZoneSelection => "zone_selection",
        crate::map::layers::LayerFilterSourceKind::SemanticSelection => "semantic_selection",
    }
}

fn layer_filter_target_label(target: crate::map::layers::LayerFilterTargetKind) -> &'static str {
    match target {
        crate::map::layers::LayerFilterTargetKind::ZoneMembership => "zone_membership",
        crate::map::layers::LayerFilterTargetKind::SemanticFieldSelection => {
            "semantic_field_selection"
        }
    }
}

fn manifest_status_label(status: LayerManifestStatus) -> &'static str {
    match status {
        LayerManifestStatus::Missing => "missing",
        LayerManifestStatus::Loading => "loading",
        LayerManifestStatus::Ready => "ready",
        LayerManifestStatus::Failed => "failed",
    }
}

fn vector_status_label(status: crate::map::layers::LayerVectorStatus) -> &'static str {
    match status {
        crate::map::layers::LayerVectorStatus::Inactive => "inactive",
        crate::map::layers::LayerVectorStatus::NotRequested => "not-requested",
        crate::map::layers::LayerVectorStatus::Fetching => "fetching",
        crate::map::layers::LayerVectorStatus::Parsing => "parsing",
        crate::map::layers::LayerVectorStatus::Building => "building",
        crate::map::layers::LayerVectorStatus::Ready => "ready",
        crate::map::layers::LayerVectorStatus::Failed => "failed",
    }
}

pub(in crate::bridge::host::snapshot::filters) fn current_layer_opacity_overrides(
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
) -> Option<BTreeMap<String, f32>> {
    let mut overrides = BTreeMap::new();
    for layer in current_layer_order(layer_registry, layer_runtime) {
        let opacity = layer_runtime.opacity(layer.id).clamp(0.0, 1.0);
        if (opacity - layer.opacity_default).abs() <= f32::EPSILON {
            continue;
        }
        overrides.insert(layer.key.clone(), opacity);
    }
    (!overrides.is_empty()).then_some(overrides)
}

pub(in crate::bridge::host::snapshot::filters) fn current_layer_clip_mask_overrides(
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
) -> Option<BTreeMap<String, String>> {
    let mut overrides = BTreeMap::new();
    for layer in current_layer_order(layer_registry, layer_runtime) {
        let Some(mask_layer_id) = layer_runtime.clip_mask_layer(layer.id) else {
            continue;
        };
        let Some(mask_layer) = layer_registry.get(mask_layer_id) else {
            continue;
        };
        if mask_layer.id == layer.id {
            continue;
        }
        overrides.insert(layer.key.clone(), mask_layer.key.clone());
    }
    (!overrides.is_empty()).then_some(overrides)
}

pub(in crate::bridge::host::snapshot::filters) fn current_layer_waypoint_connection_overrides(
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
) -> Option<BTreeMap<String, bool>> {
    let mut overrides = BTreeMap::new();
    for layer in current_layer_order(layer_registry, layer_runtime) {
        let Some(source) = layer.waypoint_source.as_ref() else {
            continue;
        };
        if !source.supports_connections {
            continue;
        }
        let visible = layer_runtime.waypoint_connections_visible(layer.id);
        if visible == source.show_connections_default {
            continue;
        }
        overrides.insert(layer.key.clone(), visible);
    }
    (!overrides.is_empty()).then_some(overrides)
}

pub(in crate::bridge::host::snapshot::filters) fn current_layer_waypoint_label_overrides(
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
) -> Option<BTreeMap<String, bool>> {
    let mut overrides = BTreeMap::new();
    for layer in current_layer_order(layer_registry, layer_runtime) {
        let Some(source) = layer.waypoint_source.as_ref() else {
            continue;
        };
        if !source.supports_labels {
            continue;
        }
        let visible = layer_runtime.waypoint_labels_visible(layer.id);
        if visible == source.show_labels_default {
            continue;
        }
        overrides.insert(layer.key.clone(), visible);
    }
    (!overrides.is_empty()).then_some(overrides)
}

pub(in crate::bridge::host::snapshot::filters) fn current_layer_point_icon_visibility_overrides(
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
) -> Option<BTreeMap<String, bool>> {
    let mut overrides = BTreeMap::new();
    for layer in current_layer_order(layer_registry, layer_runtime) {
        if !is_fish_evidence_layer(layer) {
            continue;
        }
        let visible = layer_runtime.point_icons_visible(layer.id);
        if visible {
            continue;
        }
        overrides.insert(layer.key.clone(), visible);
    }
    (!overrides.is_empty()).then_some(overrides)
}

pub(in crate::bridge::host::snapshot::filters) fn current_layer_point_icon_scale_overrides(
    layer_registry: &LayerRegistry,
    layer_runtime: &LayerRuntime,
) -> Option<BTreeMap<String, f32>> {
    let mut overrides = BTreeMap::new();
    for layer in current_layer_order(layer_registry, layer_runtime) {
        if !is_fish_evidence_layer(layer) {
            continue;
        }
        let scale = layer_runtime.point_icon_scale(layer.id);
        if (scale - crate::bridge::contract::FISHYMAP_POINT_ICON_SCALE_MIN).abs() <= f32::EPSILON {
            continue;
        }
        overrides.insert(layer.key.clone(), scale);
    }
    (!overrides.is_empty()).then_some(overrides)
}
