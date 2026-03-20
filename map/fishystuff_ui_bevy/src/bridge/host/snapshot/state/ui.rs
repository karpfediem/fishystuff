use serde_json::Value;

use super::super::super::*;
use crate::bridge::contract::FishyMapBookmarkEntry;
use crate::map::layers::{LayerRegistry, VectorSourceSpec};
use crate::plugins::bookmarks::BookmarkState;
use crate::plugins::vector_layers::VectorLayerRuntime;

pub(in crate::bridge::host::snapshot) fn effective_ui_state(
    bridge_input: &FishyMapInputState,
    display_state: &MapDisplayState,
    diagnostics_open: bool,
    bookmarks: &BookmarkState,
    layer_registry: &LayerRegistry,
    vector_runtime: &VectorLayerRuntime,
) -> crate::bridge::contract::FishyMapUiState {
    crate::bridge::contract::FishyMapUiState {
        diagnostics_open,
        legend_open: bridge_input.ui.legend_open,
        left_panel_open: bridge_input.ui.left_panel_open,
        show_points: display_state.show_points,
        show_point_icons: display_state.show_point_icons,
        point_icon_scale: display_state
            .point_icon_scale
            .clamp(POINT_ICON_SCALE_MIN, POINT_ICON_SCALE_MAX),
        bookmarks: effective_ui_bookmarks(bookmarks, layer_registry, vector_runtime),
    }
}

fn effective_ui_bookmarks(
    bookmarks: &BookmarkState,
    layer_registry: &LayerRegistry,
    vector_runtime: &VectorLayerRuntime,
) -> Vec<FishyMapBookmarkEntry> {
    bookmarks
        .entries
        .iter()
        .map(|bookmark| enrich_bookmark_entry(bookmark, layer_registry, vector_runtime))
        .collect()
}

fn enrich_bookmark_entry(
    bookmark: &FishyMapBookmarkEntry,
    layer_registry: &LayerRegistry,
    vector_runtime: &VectorLayerRuntime,
) -> FishyMapBookmarkEntry {
    let mut enriched = bookmark.clone();
    if enriched.resource_name.is_some() && enriched.origin_name.is_some() {
        return enriched;
    }
    let Some(derived_names) =
        sample_region_bookmark_names(bookmark, layer_registry, vector_runtime)
    else {
        return enriched;
    };
    if enriched.resource_name.is_none() {
        enriched.resource_name = derived_names.resource_name;
    }
    if enriched.origin_name.is_none() {
        enriched.origin_name = derived_names.origin_name;
    }
    enriched
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BookmarkDerivedNames {
    resource_name: Option<String>,
    origin_name: Option<String>,
}

fn sample_region_bookmark_names(
    bookmark: &FishyMapBookmarkEntry,
    layer_registry: &LayerRegistry,
    vector_runtime: &VectorLayerRuntime,
) -> Option<BookmarkDerivedNames> {
    let regions_layer = layer_registry.get_by_key("regions")?;
    let source = regions_layer.vector_source.as_ref()?;
    let revision = resolved_vector_revision(source, layer_registry.map_version_id());
    let bundle = vector_runtime
        .finished
        .get_ref(&(regions_layer.id, revision))?;
    let properties = bundle.sample_properties(bookmark.world_x as f32, bookmark.world_z as f32)?;
    Some(BookmarkDerivedNames {
        resource_name: derive_resource_name(properties),
        origin_name: derive_origin_name(properties),
    })
}

fn derive_resource_name(properties: &serde_json::Map<String, Value>) -> Option<String> {
    let region_group_id = json_u32(properties.get("rg"));
    let region_id = json_u32(properties.get("r"));
    let origin_name = json_string(properties.get("on"));
    if !has_waypoint_assignment(
        properties.get("rgwp"),
        properties.get("rgx"),
        properties.get("rgz"),
    ) {
        return region_group_id.map(|id| format!("RG{id}"));
    }
    if has_waypoint_assignment(
        properties.get("owp"),
        properties.get("ox"),
        properties.get("oz"),
    ) {
        return origin_name;
    }
    region_id.map(|id| format!("R{id}")).or(origin_name)
}

fn derive_origin_name(properties: &serde_json::Map<String, Value>) -> Option<String> {
    let region_id = json_u32(properties.get("r"));
    let origin_name = json_string(properties.get("on"));
    if has_waypoint_assignment(
        properties.get("owp"),
        properties.get("ox"),
        properties.get("oz"),
    ) {
        return origin_name;
    }
    region_id.map(|id| format!("R{id}")).or(origin_name)
}

fn has_waypoint_assignment(
    waypoint_id: Option<&Value>,
    world_x: Option<&Value>,
    world_z: Option<&Value>,
) -> bool {
    json_u32(waypoint_id).is_some() || json_f64(world_x).is_some() || json_f64(world_z).is_some()
}

fn json_u32(value: Option<&Value>) -> Option<u32> {
    match value {
        Some(Value::Number(number)) => number.as_u64().and_then(|raw| u32::try_from(raw).ok()),
        Some(Value::String(text)) => text.trim().parse::<u32>().ok(),
        _ => None,
    }
}

fn json_string(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(text)) => {
            let trimmed = text.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        _ => None,
    }
}

fn json_f64(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Number(number)) => number.as_f64(),
        Some(Value::String(text)) => text.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn resolved_vector_revision(source: &VectorSourceSpec, map_version_id: Option<&str>) -> String {
    let mut url = source.url.clone();
    if url.contains("{map_version}") {
        let version = map_version_id
            .filter(|value| !value.trim().is_empty() && *value != "0v0")
            .unwrap_or("v1");
        url = url.replace("{map_version}", version);
    }
    let revision = source.revision.trim();
    if revision.is_empty() {
        format!("url:{url}")
    } else {
        revision.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::enrich_bookmark_entry;
    use crate::bridge::contract::FishyMapBookmarkEntry;
    use crate::map::layers::LayerRegistry;
    use crate::map::vector::cache::{
        HoverFeature, HoverPolygon, VectorFinishedCache, VectorLayerStats, VectorMeshBundleSet,
    };
    use crate::plugins::vector_layers::VectorLayerRuntime;
    use serde_json::{Map, Value};

    #[test]
    fn bookmark_enrichment_fills_missing_resource_and_origin_names() {
        let mut registry = LayerRegistry::default();
        registry.apply_layers_response(fishystuff_api::models::layers::LayersResponse {
            revision: "rev".to_string(),
            map_version_id: None,
            layers: vec![fishystuff_api::models::layers::LayerDescriptor {
                layer_id: "regions".to_string(),
                name: "Regions".to_string(),
                enabled: true,
                kind: fishystuff_api::models::layers::LayerKind::VectorGeoJson,
                transform: fishystuff_api::models::layers::LayerTransformDto::IdentityMapSpace,
                tileset: fishystuff_api::models::layers::TilesetRef::default(),
                tile_px: 512,
                max_level: 0,
                y_flip: false,
                vector_source: Some(fishystuff_api::models::layers::VectorSourceRef {
                    url: "/region_groups/regions.v1.geojson".to_string(),
                    revision: "regions-v1".to_string(),
                    geometry_space: fishystuff_api::models::layers::GeometrySpace::MapPixels,
                    style_mode: fishystuff_api::models::layers::StyleMode::FeaturePropertyPalette,
                    feature_id_property: Some("id".to_string()),
                    color_property: Some("c".to_string()),
                }),
                lod_policy: fishystuff_api::models::layers::LodPolicyDto::default(),
                ui: fishystuff_api::models::layers::LayerUiInfo::default(),
                request_weight: 1.0,
                pick_mode: "none".to_string(),
            }],
        });
        let regions_layer = registry.get_by_key("regions").expect("regions layer");
        let mut properties = Map::new();
        properties.insert("r".to_string(), Value::from(76u32));
        properties.insert("rg".to_string(), Value::from(16u32));
        properties.insert("on".to_string(), Value::String("Tarif".to_string()));
        properties.insert("rgwp".to_string(), Value::from(306u32));
        properties.insert("owp".to_string(), Value::from(1437u32));
        let bundle = VectorMeshBundleSet {
            chunks: Vec::new(),
            hover_chunks: Vec::new(),
            stats: VectorLayerStats::default(),
            hover_features: vec![HoverFeature {
                properties,
                polygons: vec![HoverPolygon {
                    rings: vec![vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]],
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
        };
        let mut runtime = VectorLayerRuntime {
            states: std::collections::HashMap::new(),
            finished: VectorFinishedCache::with_capacity(4),
        };
        runtime
            .finished
            .insert((regions_layer.id, "regions-v1".to_string()), bundle);

        let bookmark = FishyMapBookmarkEntry {
            id: "bookmark-a".to_string(),
            label: Some("Test".to_string()),
            world_x: 5.0,
            world_z: 5.0,
            zone_name: None,
            resource_name: None,
            origin_name: None,
            zone_rgb: None,
            created_at: None,
        };

        let enriched = enrich_bookmark_entry(&bookmark, &registry, &runtime);

        assert_eq!(enriched.resource_name.as_deref(), Some("Tarif"));
        assert_eq!(enriched.origin_name.as_deref(), Some("Tarif"));
    }

    #[test]
    fn bookmark_enrichment_falls_back_to_region_group_and_region_ids_when_assignments_are_missing()
    {
        let mut registry = LayerRegistry::default();
        registry.apply_layers_response(fishystuff_api::models::layers::LayersResponse {
            revision: "rev".to_string(),
            map_version_id: None,
            layers: vec![fishystuff_api::models::layers::LayerDescriptor {
                layer_id: "regions".to_string(),
                name: "Regions".to_string(),
                enabled: true,
                kind: fishystuff_api::models::layers::LayerKind::VectorGeoJson,
                transform: fishystuff_api::models::layers::LayerTransformDto::IdentityMapSpace,
                tileset: fishystuff_api::models::layers::TilesetRef::default(),
                tile_px: 512,
                max_level: 0,
                y_flip: false,
                vector_source: Some(fishystuff_api::models::layers::VectorSourceRef {
                    url: "/region_groups/regions.v1.geojson".to_string(),
                    revision: "regions-v1".to_string(),
                    geometry_space: fishystuff_api::models::layers::GeometrySpace::MapPixels,
                    style_mode: fishystuff_api::models::layers::StyleMode::FeaturePropertyPalette,
                    feature_id_property: Some("id".to_string()),
                    color_property: Some("c".to_string()),
                }),
                lod_policy: fishystuff_api::models::layers::LodPolicyDto::default(),
                ui: fishystuff_api::models::layers::LayerUiInfo::default(),
                request_weight: 1.0,
                pick_mode: "none".to_string(),
            }],
        });
        let regions_layer = registry.get_by_key("regions").expect("regions layer");
        let mut properties = Map::new();
        properties.insert("r".to_string(), Value::from(76u32));
        properties.insert("rg".to_string(), Value::from(16u32));
        let bundle = VectorMeshBundleSet {
            chunks: Vec::new(),
            hover_chunks: Vec::new(),
            stats: VectorLayerStats::default(),
            hover_features: vec![HoverFeature {
                properties,
                polygons: vec![HoverPolygon {
                    rings: vec![vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]],
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
        };
        let mut runtime = VectorLayerRuntime {
            states: std::collections::HashMap::new(),
            finished: VectorFinishedCache::with_capacity(4),
        };
        runtime
            .finished
            .insert((regions_layer.id, "regions-v1".to_string()), bundle);

        let bookmark = FishyMapBookmarkEntry {
            id: "bookmark-a".to_string(),
            label: Some("Test".to_string()),
            world_x: 5.0,
            world_z: 5.0,
            zone_name: None,
            resource_name: None,
            origin_name: None,
            zone_rgb: None,
            created_at: None,
        };

        let enriched = enrich_bookmark_entry(&bookmark, &registry, &runtime);

        assert_eq!(enriched.resource_name.as_deref(), Some("RG16"));
        assert_eq!(enriched.origin_name.as_deref(), Some("R76"));
    }
}
