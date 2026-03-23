use super::super::super::*;
use crate::bridge::contract::FishyMapBookmarkEntry;
use crate::map::exact_lookup::ExactLookupCache;
use crate::map::field_metadata::FieldMetadataCache;
use crate::map::field_semantics::{loaded_semantic_field_layer, SemanticFieldLayerView};
use crate::map::layers::LayerRegistry;
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::WorldPoint;
use crate::plugins::bookmarks::BookmarkState;
use fishystuff_core::field_metadata::FieldHoverRow;

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
    if !enriched.rows.is_empty() {
        return enriched;
    }
    enriched.rows = collect_bookmark_rows(bookmark, layer_registry, exact_lookups, field_metadata);
    enriched
}

fn collect_bookmark_rows(
    bookmark: &FishyMapBookmarkEntry,
    layer_registry: &LayerRegistry,
    exact_lookups: &ExactLookupCache,
    field_metadata: &FieldMetadataCache,
) -> Vec<FieldHoverRow> {
    ["zone_mask", "region_groups", "regions"]
        .into_iter()
        .flat_map(|layer_key| {
            sample_bookmark_layer_rows(
                bookmark,
                layer_key,
                layer_registry,
                exact_lookups,
                field_metadata,
            )
        })
        .collect()
}

fn sample_bookmark_layer_rows(
    bookmark: &FishyMapBookmarkEntry,
    layer_key: &str,
    layer_registry: &LayerRegistry,
    exact_lookups: &ExactLookupCache,
    field_metadata: &FieldMetadataCache,
) -> Vec<FieldHoverRow> {
    let Some(layer) = layer_registry.get_by_key(layer_key) else {
        return Vec::new();
    };
    let world_point = WorldPoint::new(bookmark.world_x, bookmark.world_z);
    loaded_semantic_field_layer(layer, exact_lookups, field_metadata)
        .and_then(|field| {
            field.semantic_sample_at_world_point(layer, MapToWorld::default(), world_point)
        })
        .map(|sample| sample.rows)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::enrich_bookmark_entry;
    use crate::bridge::contract::FishyMapBookmarkEntry;
    use crate::map::exact_lookup::ExactLookupCache;
    use crate::map::field_metadata::FieldMetadataCache;
    use crate::map::layers::LayerRegistry;
    use fishystuff_core::field::DiscreteFieldRows;
    use fishystuff_core::field_metadata::{
        FieldHoverMetadataAsset, FieldHoverMetadataEntry, FieldHoverRow,
    };

    fn field_layer_descriptor(
        layer_id: &str,
        name: &str,
    ) -> fishystuff_api::models::layers::LayerDescriptor {
        fishystuff_api::models::layers::LayerDescriptor {
            layer_id: layer_id.to_string(),
            name: name.to_string(),
            enabled: true,
            kind: fishystuff_api::models::layers::LayerKind::TiledRaster,
            transform: fishystuff_api::models::layers::LayerTransformDto::IdentityMapSpace,
            tileset: fishystuff_api::models::layers::TilesetRef::default(),
            tile_px: 512,
            max_level: 0,
            y_flip: false,
            field_source: Some(fishystuff_api::models::layers::FieldSourceRef {
                url: format!("/fields/{layer_id}.v1.bin"),
                revision: format!("{layer_id}-field-v1"),
                color_mode: fishystuff_api::models::layers::FieldColorMode::DebugHash,
            }),
            field_metadata_source: Some(fishystuff_api::models::layers::FieldMetadataSourceRef {
                url: format!("/fields/{layer_id}.v1.meta.json"),
                revision: format!("{layer_id}-meta-v1"),
            }),
            vector_source: None,
            lod_policy: fishystuff_api::models::layers::LodPolicyDto::default(),
            ui: fishystuff_api::models::layers::LayerUiInfo::default(),
            request_weight: 1.0,
            pick_mode: "none".to_string(),
        }
    }

    fn field_registry() -> LayerRegistry {
        let mut registry = LayerRegistry::default();
        registry.apply_layers_response(fishystuff_api::models::layers::LayersResponse {
            revision: "rev".to_string(),
            map_version_id: None,
            layers: vec![
                field_layer_descriptor("zone_mask", "Zone Mask"),
                field_layer_descriptor("regions", "Regions"),
                field_layer_descriptor("region_groups", "Region Groups"),
            ],
        });
        registry
    }

    #[test]
    fn bookmark_enrichment_fills_missing_semantic_rows() {
        let registry = field_registry();
        let zone_layer = registry.get_by_key("zone_mask").expect("zone layer");
        let regions_layer = registry.get_by_key("regions").expect("regions layer");
        let region_groups_layer = registry
            .get_by_key("region_groups")
            .expect("region_groups layer");
        let mut exact_lookups = ExactLookupCache::default();
        exact_lookups.insert_ready(
            zone_layer.id,
            "/fields/zone_mask.v1.bin".to_string(),
            DiscreteFieldRows::from_u32_grid(10, 10, &[0x123456; 100]).expect("field"),
        );
        exact_lookups.insert_ready(
            regions_layer.id,
            "/fields/regions.v1.bin".to_string(),
            DiscreteFieldRows::from_u32_grid(10, 10, &[76; 100]).expect("field"),
        );
        exact_lookups.insert_ready(
            region_groups_layer.id,
            "/fields/region_groups.v1.bin".to_string(),
            DiscreteFieldRows::from_u32_grid(10, 10, &[16; 100]).expect("field"),
        );
        let mut field_metadata = FieldMetadataCache::default();
        field_metadata.insert_ready(
            zone_layer.id,
            "/fields/zone_mask.v1.meta.json".to_string(),
            FieldHoverMetadataAsset {
                entries: std::collections::BTreeMap::from([(
                    0x123456,
                    FieldHoverMetadataEntry {
                        rows: vec![FieldHoverRow {
                            key: "zone".to_string(),
                            icon: "hover-zone".to_string(),
                            label: "Zone".to_string(),
                            value: "Mediah".to_string(),
                            hide_label: false,
                            status_icon: None,
                            status_icon_tone: None,
                        }],
                        targets: Vec::new(),
                    },
                )]),
            },
        );
        field_metadata.insert_ready(
            regions_layer.id,
            "/fields/regions.v1.meta.json".to_string(),
            FieldHoverMetadataAsset {
                entries: std::collections::BTreeMap::from([(
                    76,
                    FieldHoverMetadataEntry {
                        rows: vec![FieldHoverRow {
                            key: "origin".to_string(),
                            icon: "hover-origin".to_string(),
                            label: "Origin".to_string(),
                            value: "Tarif".to_string(),
                            hide_label: false,
                            status_icon: None,
                            status_icon_tone: None,
                        }],
                        targets: Vec::new(),
                    },
                )]),
            },
        );
        field_metadata.insert_ready(
            region_groups_layer.id,
            "/fields/region_groups.v1.meta.json".to_string(),
            FieldHoverMetadataAsset {
                entries: std::collections::BTreeMap::from([(
                    16,
                    FieldHoverMetadataEntry {
                        rows: vec![FieldHoverRow {
                            key: "resources".to_string(),
                            icon: "hover-resources".to_string(),
                            label: "Resources".to_string(),
                            value: "Tarif".to_string(),
                            hide_label: false,
                            status_icon: None,
                            status_icon_tone: None,
                        }],
                        targets: Vec::new(),
                    },
                )]),
            },
        );

        let bookmark = FishyMapBookmarkEntry {
            id: "bookmark-a".to_string(),
            label: Some("Test".to_string()),
            world_x: 5.0,
            world_z: 5.0,
            rows: Vec::new(),
            zone_rgb: None,
            created_at: None,
        };

        let enriched = enrich_bookmark_entry(&bookmark, &registry, &exact_lookups, &field_metadata);

        assert_eq!(
            enriched
                .rows
                .iter()
                .map(|row| row.value.as_str())
                .collect::<Vec<_>>(),
            vec!["Mediah", "Tarif", "Tarif"]
        );
    }

    #[test]
    fn bookmark_enrichment_preserves_fallback_semantic_rows() {
        let registry = field_registry();
        let zone_layer = registry.get_by_key("zone_mask").expect("zone layer");
        let regions_layer = registry.get_by_key("regions").expect("regions layer");
        let region_groups_layer = registry
            .get_by_key("region_groups")
            .expect("region_groups layer");
        let mut exact_lookups = ExactLookupCache::default();
        exact_lookups.insert_ready(
            zone_layer.id,
            "/fields/zone_mask.v1.bin".to_string(),
            DiscreteFieldRows::from_u32_grid(10, 10, &[0x445566; 100]).expect("field"),
        );
        exact_lookups.insert_ready(
            regions_layer.id,
            "/fields/regions.v1.bin".to_string(),
            DiscreteFieldRows::from_u32_grid(10, 10, &[76; 100]).expect("field"),
        );
        exact_lookups.insert_ready(
            region_groups_layer.id,
            "/fields/region_groups.v1.bin".to_string(),
            DiscreteFieldRows::from_u32_grid(10, 10, &[16; 100]).expect("field"),
        );
        let mut field_metadata = FieldMetadataCache::default();
        field_metadata.insert_ready(
            zone_layer.id,
            "/fields/zone_mask.v1.meta.json".to_string(),
            FieldHoverMetadataAsset {
                entries: std::collections::BTreeMap::from([(
                    0x445566,
                    FieldHoverMetadataEntry {
                        rows: vec![FieldHoverRow {
                            key: "zone".to_string(),
                            icon: "hover-zone".to_string(),
                            label: "Zone".to_string(),
                            value: "Zone 0x445566".to_string(),
                            hide_label: false,
                            status_icon: None,
                            status_icon_tone: None,
                        }],
                        targets: Vec::new(),
                    },
                )]),
            },
        );
        field_metadata.insert_ready(
            regions_layer.id,
            "/fields/regions.v1.meta.json".to_string(),
            FieldHoverMetadataAsset {
                entries: std::collections::BTreeMap::from([(
                    76,
                    FieldHoverMetadataEntry {
                        rows: vec![FieldHoverRow {
                            key: "origin".to_string(),
                            icon: "hover-origin".to_string(),
                            label: "Origin".to_string(),
                            value: "R76".to_string(),
                            hide_label: false,
                            status_icon: Some("question-mark".to_string()),
                            status_icon_tone: None,
                        }],
                        targets: Vec::new(),
                    },
                )]),
            },
        );
        field_metadata.insert_ready(
            region_groups_layer.id,
            "/fields/region_groups.v1.meta.json".to_string(),
            FieldHoverMetadataAsset {
                entries: std::collections::BTreeMap::from([(
                    16,
                    FieldHoverMetadataEntry {
                        rows: vec![FieldHoverRow {
                            key: "resources".to_string(),
                            icon: "hover-resources".to_string(),
                            label: "Resources".to_string(),
                            value: "RG16".to_string(),
                            hide_label: false,
                            status_icon: Some("question-mark".to_string()),
                            status_icon_tone: None,
                        }],
                        targets: Vec::new(),
                    },
                )]),
            },
        );

        let bookmark = FishyMapBookmarkEntry {
            id: "bookmark-a".to_string(),
            label: Some("Test".to_string()),
            world_x: 5.0,
            world_z: 5.0,
            rows: Vec::new(),
            zone_rgb: None,
            created_at: None,
        };

        let enriched = enrich_bookmark_entry(&bookmark, &registry, &exact_lookups, &field_metadata);

        assert_eq!(
            enriched
                .rows
                .iter()
                .map(|row| row.value.as_str())
                .collect::<Vec<_>>(),
            vec!["Zone 0x445566", "RG16", "R76"]
        );
    }
}
