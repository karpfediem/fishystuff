use super::super::super::*;
use crate::bridge::contract::FishyMapBookmarkEntry;
use crate::map::exact_lookup::{sample_field_layer_id_u32, ExactLookupCache};
use crate::map::field_metadata::FieldMetadataCache;
use crate::map::layers::LayerRegistry;
use crate::map::spaces::world::MapToWorld;
use crate::plugins::bookmarks::BookmarkState;

pub(in crate::bridge::host::snapshot) fn effective_ui_state(
    bridge_input: &FishyMapInputState,
    display_state: &MapDisplayState,
    diagnostics_open: bool,
    bookmarks: &BookmarkState,
    layer_registry: &LayerRegistry,
    exact_lookups: &ExactLookupCache,
    field_metadata: &FieldMetadataCache,
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
        bookmark_selected_ids: bookmarks.selected_ids.clone(),
        bookmarks: effective_ui_bookmarks(bookmarks, layer_registry, exact_lookups, field_metadata),
    }
}

fn effective_ui_bookmarks(
    bookmarks: &BookmarkState,
    layer_registry: &LayerRegistry,
    exact_lookups: &ExactLookupCache,
    field_metadata: &FieldMetadataCache,
) -> Vec<FishyMapBookmarkEntry> {
    bookmarks
        .entries
        .iter()
        .map(|bookmark| {
            enrich_bookmark_entry(bookmark, layer_registry, exact_lookups, field_metadata)
        })
        .collect()
}

fn enrich_bookmark_entry(
    bookmark: &FishyMapBookmarkEntry,
    layer_registry: &LayerRegistry,
    exact_lookups: &ExactLookupCache,
    field_metadata: &FieldMetadataCache,
) -> FishyMapBookmarkEntry {
    let mut enriched = bookmark.clone();
    if enriched.resource_name.is_some() && enriched.origin_name.is_some() {
        return enriched;
    }
    let Some(derived_names) =
        sample_region_bookmark_names(bookmark, layer_registry, exact_lookups, field_metadata)
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
    exact_lookups: &ExactLookupCache,
    field_metadata: &FieldMetadataCache,
) -> Option<BookmarkDerivedNames> {
    let regions_layer = layer_registry.get_by_key("regions")?;
    let metadata_url = regions_layer.field_metadata_url()?;
    let map =
        MapToWorld::default().world_to_map(WorldPoint::new(bookmark.world_x, bookmark.world_z));
    let map_px_x = map.x.floor() as i32;
    let map_px_y = map.y.floor() as i32;
    let field_id = sample_field_layer_id_u32(regions_layer, exact_lookups, map_px_x, map_px_y)?;
    let entry = field_metadata.entry(regions_layer.id, &metadata_url, field_id)?;
    Some(BookmarkDerivedNames {
        resource_name: derive_resource_name(entry),
        origin_name: derive_origin_name(entry),
    })
}

fn derive_resource_name(
    entry: &fishystuff_core::field_metadata::FieldHoverMetadataEntry,
) -> Option<String> {
    if !has_waypoint_assignment(
        entry.resource_bar_waypoint,
        entry.resource_bar_world_x,
        entry.resource_bar_world_z,
    ) {
        return entry.region_group.map(|id| format!("RG{id}"));
    }
    if has_waypoint_assignment(
        entry.origin_waypoint,
        entry.origin_world_x,
        entry.origin_world_z,
    ) {
        return entry.region_name.clone();
    }
    entry
        .region_id
        .map(|id| format!("R{id}"))
        .or_else(|| entry.region_name.clone())
}

fn derive_origin_name(
    entry: &fishystuff_core::field_metadata::FieldHoverMetadataEntry,
) -> Option<String> {
    if has_waypoint_assignment(
        entry.origin_waypoint,
        entry.origin_world_x,
        entry.origin_world_z,
    ) {
        return entry.region_name.clone();
    }
    entry
        .region_id
        .map(|id| format!("R{id}"))
        .or_else(|| entry.region_name.clone())
}

fn has_waypoint_assignment(
    waypoint_id: Option<u32>,
    world_x: Option<f64>,
    world_z: Option<f64>,
) -> bool {
    waypoint_id.is_some() || world_x.is_some() || world_z.is_some()
}

#[cfg(test)]
mod tests {
    use super::enrich_bookmark_entry;
    use crate::bridge::contract::FishyMapBookmarkEntry;
    use crate::map::exact_lookup::ExactLookupCache;
    use crate::map::field_metadata::FieldMetadataCache;
    use crate::map::layers::LayerRegistry;
    use fishystuff_core::field::DiscreteFieldRows;
    use fishystuff_core::field_metadata::{FieldHoverMetadataAsset, FieldHoverMetadataEntry};

    fn regions_registry() -> LayerRegistry {
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
                field_source: Some(fishystuff_api::models::layers::FieldSourceRef {
                    url: "/fields/regions.v1.bin".to_string(),
                    revision: "regions-field-v1".to_string(),
                    color_mode: fishystuff_api::models::layers::FieldColorMode::DebugHash,
                }),
                field_metadata_source: Some(
                    fishystuff_api::models::layers::FieldMetadataSourceRef {
                        url: "/fields/regions.v1.meta.json".to_string(),
                        revision: "regions-meta-v1".to_string(),
                    },
                ),
                vector_source: None,
                lod_policy: fishystuff_api::models::layers::LodPolicyDto::default(),
                ui: fishystuff_api::models::layers::LayerUiInfo::default(),
                request_weight: 1.0,
                pick_mode: "none".to_string(),
            }],
        });
        registry
    }

    #[test]
    fn bookmark_enrichment_fills_missing_resource_and_origin_names() {
        let registry = regions_registry();
        let regions_layer = registry.get_by_key("regions").expect("regions layer");
        let mut exact_lookups = ExactLookupCache::default();
        exact_lookups.insert_ready(
            regions_layer.id,
            "/fields/regions.v1.bin".to_string(),
            DiscreteFieldRows::from_u32_grid(10, 10, &[76; 100]).expect("field"),
        );
        let mut field_metadata = FieldMetadataCache::default();
        field_metadata.insert_ready(
            regions_layer.id,
            "/fields/regions.v1.meta.json".to_string(),
            FieldHoverMetadataAsset {
                entries: std::collections::BTreeMap::from([(
                    76,
                    FieldHoverMetadataEntry {
                        region_id: Some(76),
                        region_group: Some(16),
                        region_name: Some("Tarif".to_string()),
                        resource_bar_waypoint: Some(306),
                        resource_bar_world_x: None,
                        resource_bar_world_z: None,
                        origin_waypoint: Some(1437),
                        origin_world_x: None,
                        origin_world_z: None,
                    },
                )]),
            },
        );

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

        let enriched = enrich_bookmark_entry(&bookmark, &registry, &exact_lookups, &field_metadata);

        assert_eq!(enriched.resource_name.as_deref(), Some("Tarif"));
        assert_eq!(enriched.origin_name.as_deref(), Some("Tarif"));
    }

    #[test]
    fn bookmark_enrichment_falls_back_to_region_group_and_region_ids_when_assignments_are_missing()
    {
        let registry = regions_registry();
        let regions_layer = registry.get_by_key("regions").expect("regions layer");
        let mut exact_lookups = ExactLookupCache::default();
        exact_lookups.insert_ready(
            regions_layer.id,
            "/fields/regions.v1.bin".to_string(),
            DiscreteFieldRows::from_u32_grid(10, 10, &[76; 100]).expect("field"),
        );
        let mut field_metadata = FieldMetadataCache::default();
        field_metadata.insert_ready(
            regions_layer.id,
            "/fields/regions.v1.meta.json".to_string(),
            FieldHoverMetadataAsset {
                entries: std::collections::BTreeMap::from([(
                    76,
                    FieldHoverMetadataEntry {
                        region_id: Some(76),
                        region_group: Some(16),
                        region_name: None,
                        resource_bar_waypoint: None,
                        resource_bar_world_x: None,
                        resource_bar_world_z: None,
                        origin_waypoint: None,
                        origin_world_x: None,
                        origin_world_z: None,
                    },
                )]),
            },
        );

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

        let enriched = enrich_bookmark_entry(&bookmark, &registry, &exact_lookups, &field_metadata);

        assert_eq!(enriched.resource_name.as_deref(), Some("RG16"));
        assert_eq!(enriched.origin_name.as_deref(), Some("R76"));
    }
}
