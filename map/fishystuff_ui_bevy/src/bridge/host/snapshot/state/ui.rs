use super::super::super::*;
use crate::bridge::contract::FishyMapBookmarkEntry;
use crate::map::exact_lookup::ExactLookupCache;
use crate::map::field_metadata::FieldMetadataCache;
use crate::map::layer_query::sample_semantic_layers_at_world_point;
use crate::map::layers::LayerRegistry;
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::WorldPoint;
use crate::plugins::bookmarks::BookmarkState;

use super::hover_layer_samples_snapshot;

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
        active_detail_pane_id: bridge_input.ui.active_detail_pane_id.clone(),
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
    if !enriched.layer_samples.is_empty() {
        return enriched;
    }
    enriched.layer_samples =
        collect_bookmark_layer_samples(bookmark, layer_registry, exact_lookups, field_metadata);
    enriched
}

fn collect_bookmark_layer_samples(
    bookmark: &FishyMapBookmarkEntry,
    layer_registry: &LayerRegistry,
    exact_lookups: &ExactLookupCache,
    field_metadata: &FieldMetadataCache,
) -> Vec<crate::bridge::contract::FishyMapHoverLayerSampleSnapshot> {
    let samples = sample_semantic_layers_at_world_point(
        layer_registry,
        exact_lookups,
        field_metadata,
        WorldPoint::new(bookmark.world_x, bookmark.world_z),
        MapToWorld::default(),
    );
    hover_layer_samples_snapshot(&samples)
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
        FieldDetailFact, FieldDetailSection, FieldHoverMetadataAsset, FieldHoverMetadataEntry,
    };

    fn metadata_entry(
        key: &str,
        label: &str,
        value: &str,
        icon: &str,
        status_icon: Option<&str>,
    ) -> FieldHoverMetadataEntry {
        FieldHoverMetadataEntry {
            targets: Vec::new(),
            detail_pane: None,
            detail_sections: vec![FieldDetailSection {
                id: key.to_string(),
                kind: "facts".to_string(),
                title: Some(label.to_string()),
                facts: vec![FieldDetailFact {
                    key: key.to_string(),
                    label: label.to_string(),
                    value: value.to_string(),
                    icon: Some(icon.to_string()),
                    status_icon: status_icon.map(ToOwned::to_owned),
                    status_icon_tone: None,
                }],
                targets: Vec::new(),
            }],
        }
    }

    fn field_layer_descriptor(
        layer_id: &str,
        name: &str,
        display_order: i32,
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
            ui: fishystuff_api::models::layers::LayerUiInfo {
                display_order,
                ..Default::default()
            },
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
                field_layer_descriptor("zone_mask", "Zone Mask", 20),
                field_layer_descriptor("region_groups", "Region Groups", 30),
                field_layer_descriptor("regions", "Regions", 40),
            ],
        });
        registry
    }

    #[test]
    fn bookmark_enrichment_fills_missing_semantic_layer_samples() {
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
                    metadata_entry("zone", "Zone", "Mediah", "hover-zone", None),
                )]),
            },
        );
        field_metadata.insert_ready(
            regions_layer.id,
            "/fields/regions.v1.meta.json".to_string(),
            FieldHoverMetadataAsset {
                entries: std::collections::BTreeMap::from([(
                    76,
                    metadata_entry("origin_region", "Region", "Tarif", "hover-origin", None),
                )]),
            },
        );
        field_metadata.insert_ready(
            region_groups_layer.id,
            "/fields/region_groups.v1.meta.json".to_string(),
            FieldHoverMetadataAsset {
                entries: std::collections::BTreeMap::from([(
                    16,
                    metadata_entry(
                        "resource_region",
                        "Containing region",
                        "Tarif",
                        "hover-resources",
                        None,
                    ),
                )]),
            },
        );

        let bookmark = FishyMapBookmarkEntry {
            id: "bookmark-a".to_string(),
            label: Some("Test".to_string()),
            world_x: 5.0,
            world_z: 5.0,
            layer_samples: Vec::new(),
            zone_rgb: None,
            created_at: None,
        };

        let enriched = enrich_bookmark_entry(&bookmark, &registry, &exact_lookups, &field_metadata);

        assert_eq!(
            enriched
                .layer_samples
                .iter()
                .map(|sample| sample.layer_id.as_str())
                .collect::<Vec<_>>(),
            vec!["zone_mask", "region_groups", "regions"]
        );
    }

    #[test]
    fn bookmark_enrichment_preserves_fact_based_semantic_samples() {
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
                    metadata_entry("zone", "Zone", "Zone 0x445566", "hover-zone", None),
                )]),
            },
        );
        field_metadata.insert_ready(
            regions_layer.id,
            "/fields/regions.v1.meta.json".to_string(),
            FieldHoverMetadataAsset {
                entries: std::collections::BTreeMap::from([(
                    76,
                    metadata_entry(
                        "origin_region",
                        "Region",
                        "R76",
                        "hover-origin",
                        Some("question-mark"),
                    ),
                )]),
            },
        );
        field_metadata.insert_ready(
            region_groups_layer.id,
            "/fields/region_groups.v1.meta.json".to_string(),
            FieldHoverMetadataAsset {
                entries: std::collections::BTreeMap::from([(
                    16,
                    metadata_entry(
                        "resource_region",
                        "Containing region",
                        "RG16",
                        "hover-resources",
                        Some("question-mark"),
                    ),
                )]),
            },
        );

        let bookmark = FishyMapBookmarkEntry {
            id: "bookmark-a".to_string(),
            label: Some("Test".to_string()),
            world_x: 5.0,
            world_z: 5.0,
            layer_samples: Vec::new(),
            zone_rgb: None,
            created_at: None,
        };

        let enriched = enrich_bookmark_entry(&bookmark, &registry, &exact_lookups, &field_metadata);

        assert_eq!(
            enriched
                .layer_samples
                .iter()
                .filter_map(|sample| sample.detail_sections.first())
                .filter_map(|section| section.facts.first())
                .map(|fact| fact.value.as_str())
                .collect::<Vec<_>>(),
            vec!["Zone 0x445566", "RG16", "R76"]
        );
    }
}
