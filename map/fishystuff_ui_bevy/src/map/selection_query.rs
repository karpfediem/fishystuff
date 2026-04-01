use crate::bridge::contract::FishyMapSelectionPointKind;
use crate::map::field_metadata::FieldMetadataCache;
use crate::map::field_semantics::semantic_sample_for_field_id;
use crate::map::field_view::sample_rgb_for_field_id;
use crate::map::hover_query::WorldPointQueryContext;
use crate::map::layer_query::{sample_semantic_layers_at_world_point, LayerQuerySample};
use crate::map::layers::LayerRegistry;
use crate::map::spaces::WorldPoint;
use crate::plugins::api::{HoverInfo, SelectedInfo};
use fishystuff_core::field_metadata::{detail_facts, preferred_detail_fact_value};
use std::collections::HashMap;

fn normalized_point_label(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn preferred_sample_point_label(
    sample: &LayerQuerySample,
    zone_names: Option<&HashMap<u32, Option<String>>>,
) -> Option<String> {
    if let Some(value) = preferred_detail_fact_value(detail_facts(&sample.detail_sections)) {
        return normalized_point_label(Some(value));
    }
    if sample.layer_id == "zone_mask" {
        if let Some(value) = zone_names
            .and_then(|zones| zones.get(&sample.rgb_u32))
            .and_then(|value| value.as_deref())
        {
            return normalized_point_label(Some(value));
        }
    }
    sample
        .targets
        .iter()
        .find_map(|target| normalized_point_label(Some(target.label.as_str())))
}

fn preferred_point_label_from_layer_samples(
    layer_samples: &[LayerQuerySample],
    fallback_point_label: Option<&str>,
    zone_names: Option<&HashMap<u32, Option<String>>>,
) -> Option<String> {
    layer_samples
        .iter()
        .find_map(|sample| preferred_sample_point_label(sample, zone_names))
        .or_else(|| normalized_point_label(fallback_point_label))
}

pub fn selected_info_from_hover(hover: &HoverInfo) -> Option<SelectedInfo> {
    if hover.zone_rgb().is_none() && hover.layer_samples.is_empty() {
        return None;
    }
    Some(SelectedInfo {
        map_px: hover.map_px,
        map_py: hover.map_py,
        world_x: hover.world_x,
        world_z: hover.world_z,
        sampled_world_point: true,
        point_kind: Some(FishyMapSelectionPointKind::Clicked),
        point_label: preferred_point_label_from_layer_samples(&hover.layer_samples, None, None),
        layer_samples: hover.layer_samples.clone(),
    })
}

pub fn selected_info_at_world_point(
    world_point: WorldPoint,
    context: &WorldPointQueryContext<'_>,
    point_kind: FishyMapSelectionPointKind,
    point_label: Option<&str>,
    zone_names: Option<&HashMap<u32, Option<String>>>,
) -> Option<SelectedInfo> {
    let map = context.map_to_world.world_to_map(world_point);
    let map_x = map.x as f32;
    let map_y = map.y as f32;
    if map_x < 0.0
        || map_y < 0.0
        || map_x >= context.map_to_world.image_size_x as f32
        || map_y >= context.map_to_world.image_size_y as f32
    {
        return None;
    }
    let layer_samples = sample_semantic_layers_at_world_point(
        context.layer_registry,
        context.exact_lookups,
        context.field_metadata,
        world_point,
        context.map_to_world,
    );
    if layer_samples.is_empty() {
        return None;
    }
    Some(SelectedInfo {
        map_px: map_x.floor() as i32,
        map_py: map_y.floor() as i32,
        world_x: world_point.x,
        world_z: world_point.z,
        sampled_world_point: true,
        point_kind: Some(point_kind),
        point_label: preferred_point_label_from_layer_samples(
            &layer_samples,
            point_label,
            zone_names,
        ),
        layer_samples,
    })
}

pub fn selected_info_for_zone_rgb(
    layer_registry: &LayerRegistry,
    field_metadata: &FieldMetadataCache,
    zone_rgb: u32,
    zone_names: Option<&HashMap<u32, Option<String>>>,
) -> SelectedInfo {
    let layer_samples: Vec<LayerQuerySample> =
        semantic_layer_sample_for_field_id(layer_registry, field_metadata, "zone_mask", zone_rgb)
            .into_iter()
            .collect();
    SelectedInfo {
        map_px: 0,
        map_py: 0,
        world_x: f64::NAN,
        world_z: f64::NAN,
        sampled_world_point: false,
        point_kind: None,
        point_label: preferred_point_label_from_layer_samples(&layer_samples, None, zone_names),
        layer_samples,
    }
}

pub fn selected_info_for_semantic_field(
    layer_registry: &LayerRegistry,
    field_metadata: &FieldMetadataCache,
    layer_key: &str,
    field_id: u32,
    zone_names: Option<&HashMap<u32, Option<String>>>,
) -> Option<SelectedInfo> {
    let layer_sample =
        semantic_layer_sample_for_field_id(layer_registry, field_metadata, layer_key, field_id)?;
    let layer_samples = vec![layer_sample];
    Some(SelectedInfo {
        map_px: 0,
        map_py: 0,
        world_x: f64::NAN,
        world_z: f64::NAN,
        sampled_world_point: false,
        point_kind: None,
        point_label: preferred_point_label_from_layer_samples(&layer_samples, None, zone_names),
        layer_samples,
    })
}

pub fn semantic_layer_sample_for_field_id(
    layer_registry: &LayerRegistry,
    field_metadata: &FieldMetadataCache,
    layer_key: &str,
    field_id: u32,
) -> Option<LayerQuerySample> {
    let layer = layer_registry.get_by_key(layer_key)?;
    let rgb = sample_rgb_for_field_id(field_id, layer.field_color_mode()?);
    let semantic = semantic_sample_for_field_id(layer, field_metadata, field_id, rgb);
    Some(LayerQuerySample {
        layer_id: layer.key.clone(),
        layer_name: layer.name.clone(),
        kind: "field".to_string(),
        rgb: semantic.rgb,
        rgb_u32: semantic.rgb_u32,
        field_id: Some(semantic.field_id),
        targets: semantic.targets,
        detail_pane: semantic.detail_pane,
        detail_sections: semantic.detail_sections,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        selected_info_at_world_point, selected_info_for_semantic_field, selected_info_for_zone_rgb,
        selected_info_from_hover, semantic_layer_sample_for_field_id,
    };
    use crate::bridge::contract::FishyMapSelectionPointKind;
    use crate::map::exact_lookup::ExactLookupCache;
    use crate::map::field_metadata::FieldMetadataCache;
    use crate::map::hover_query::WorldPointQueryContext;
    use crate::map::layer_query::LayerQuerySample;
    use crate::map::layers::{LayerRegistry, LayerRuntime};
    use crate::map::raster::RasterTileCache;
    use crate::map::spaces::world::MapToWorld;
    use crate::map::spaces::MapPoint;
    use crate::plugins::api::HoverInfo;
    use crate::plugins::vector_layers::VectorLayerRuntime;
    use fishystuff_api::Rgb;
    use fishystuff_core::field::DiscreteFieldRows;
    use fishystuff_core::field_metadata::{
        FieldDetailFact, FieldDetailSection, FieldHoverMetadataAsset, FieldHoverMetadataEntry,
        FIELD_DETAIL_FACT_KEY_ORIGIN_REGION, FIELD_DETAIL_FACT_KEY_ZONE,
    };

    fn metadata_entry(key: &str, label: &str, value: &str, icon: &str) -> FieldHoverMetadataEntry {
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
                    status_icon: None,
                    status_icon_tone: None,
                }],
                targets: Vec::new(),
            }],
        }
    }

    fn zone_registry() -> LayerRegistry {
        let mut registry = LayerRegistry::default();
        registry.apply_layers_response(fishystuff_api::models::layers::LayersResponse {
            revision: "rev".to_string(),
            map_version_id: None,
            layers: vec![fishystuff_api::models::layers::LayerDescriptor {
                layer_id: "zone_mask".to_string(),
                name: "Zone Mask".to_string(),
                enabled: true,
                kind: fishystuff_api::models::layers::LayerKind::TiledRaster,
                transform: fishystuff_api::models::layers::LayerTransformDto::IdentityMapSpace,
                tileset: fishystuff_api::models::layers::TilesetRef::default(),
                tile_px: 512,
                max_level: 0,
                y_flip: false,
                field_source: Some(fishystuff_api::models::layers::FieldSourceRef {
                    url: "/images/exact_lookup/zone_mask.v1.bin".to_string(),
                    revision: "zone-field-v1".to_string(),
                    color_mode: fishystuff_api::models::layers::FieldColorMode::RgbU24,
                }),
                field_metadata_source: Some(
                    fishystuff_api::models::layers::FieldMetadataSourceRef {
                        url: "/fields/zone_mask.v1.meta.json".to_string(),
                        revision: "zone-meta-v1".to_string(),
                    },
                ),
                vector_source: None,
                lod_policy: fishystuff_api::models::layers::LodPolicyDto::default(),
                ui: fishystuff_api::models::layers::LayerUiInfo::default(),
                request_weight: 1.0,
                pick_mode: "exact_tile_pixel".to_string(),
            }],
        });
        registry
    }

    fn semantic_registry() -> LayerRegistry {
        let mut registry = LayerRegistry::default();
        registry.apply_layers_response(fishystuff_api::models::layers::LayersResponse {
            revision: "rev".to_string(),
            map_version_id: None,
            layers: vec![
                fishystuff_api::models::layers::LayerDescriptor {
                    layer_id: "zone_mask".to_string(),
                    name: "Zone Mask".to_string(),
                    enabled: true,
                    kind: fishystuff_api::models::layers::LayerKind::TiledRaster,
                    transform: fishystuff_api::models::layers::LayerTransformDto::IdentityMapSpace,
                    tileset: fishystuff_api::models::layers::TilesetRef::default(),
                    tile_px: 512,
                    max_level: 0,
                    y_flip: false,
                    field_source: Some(fishystuff_api::models::layers::FieldSourceRef {
                        url: "/fields/zone_mask.v1.bin".to_string(),
                        revision: "zone-field-v1".to_string(),
                        color_mode: fishystuff_api::models::layers::FieldColorMode::RgbU24,
                    }),
                    field_metadata_source: Some(
                        fishystuff_api::models::layers::FieldMetadataSourceRef {
                            url: "/fields/zone_mask.v1.meta.json".to_string(),
                            revision: "zone-meta-v1".to_string(),
                        },
                    ),
                    vector_source: None,
                    lod_policy: fishystuff_api::models::layers::LodPolicyDto::default(),
                    ui: fishystuff_api::models::layers::LayerUiInfo {
                        display_order: 20,
                        ..Default::default()
                    },
                    request_weight: 1.0,
                    pick_mode: "exact_tile_pixel".to_string(),
                },
                fishystuff_api::models::layers::LayerDescriptor {
                    layer_id: "regions".to_string(),
                    name: "Regions".to_string(),
                    enabled: true,
                    kind: fishystuff_api::models::layers::LayerKind::TiledRaster,
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
                    ui: fishystuff_api::models::layers::LayerUiInfo {
                        display_order: 40,
                        ..Default::default()
                    },
                    request_weight: 1.0,
                    pick_mode: "none".to_string(),
                },
            ],
        });
        registry
    }

    #[test]
    fn selected_info_from_hover_preserves_hover_selection_payload() {
        let hover = HoverInfo {
            map_px: 12,
            map_py: 34,
            world_x: 1.25,
            world_z: 2.5,
            layer_samples: vec![LayerQuerySample {
                layer_id: "zone_mask".to_string(),
                layer_name: "Zone Mask".to_string(),
                kind: "field".to_string(),
                rgb: Rgb::from_u32(0x123456),
                rgb_u32: 0x123456,
                field_id: Some(0x123456),
                targets: Vec::new(),
                detail_pane: None,
                detail_sections: Vec::new(),
            }],
        };

        let selected = selected_info_from_hover(&hover).expect("selected info");
        assert_eq!(selected.map_px, 12);
        assert_eq!(selected.map_py, 34);
        assert_eq!(selected.zone_rgb_u32(), Some(0x123456));
        assert!(selected.sampled_world_point);
        assert_eq!(
            selected.point_kind,
            Some(FishyMapSelectionPointKind::Clicked)
        );
        assert_eq!(selected.point_label, None);
        assert_eq!(selected.world_x, 1.25);
        assert_eq!(selected.world_z, 2.5);
        assert_eq!(selected.layer_samples, hover.layer_samples);
    }

    #[test]
    fn selected_info_from_hover_keeps_non_zone_field_selection() {
        let hover = HoverInfo {
            map_px: 7,
            map_py: 9,
            world_x: 3.5,
            world_z: 4.5,
            layer_samples: vec![LayerQuerySample {
                layer_id: "regions".to_string(),
                layer_name: "Regions".to_string(),
                kind: "field".to_string(),
                rgb: Rgb::from_u32(0x223344),
                rgb_u32: 0x223344,
                field_id: Some(76),
                targets: Vec::new(),
                detail_pane: None,
                detail_sections: Vec::new(),
            }],
        };

        let selected = selected_info_from_hover(&hover).expect("selected info");
        assert_eq!(selected.zone_rgb_u32(), None);
        assert!(selected.sampled_world_point);
        assert_eq!(
            selected.point_kind,
            Some(FishyMapSelectionPointKind::Clicked)
        );
        assert_eq!(selected.layer_samples, hover.layer_samples);
    }

    #[test]
    fn selected_info_at_world_point_uses_semantic_layers_even_when_hidden() {
        let registry = semantic_registry();
        let zone_layer = registry.get_by_key("zone_mask").expect("zone layer");
        let regions_layer = registry.get_by_key("regions").expect("regions layer");
        let mut runtime = LayerRuntime::default();
        runtime.sync_to_registry(&registry);
        runtime.set_visible(zone_layer.id, false);
        runtime.set_visible(regions_layer.id, false);

        let mut exact_lookups = ExactLookupCache::default();
        exact_lookups.insert_ready(
            zone_layer.id,
            "/fields/zone_mask.v1.bin".to_string(),
            DiscreteFieldRows::from_u32_grid(2, 2, &[0x123456; 4]).expect("zone field"),
        );
        exact_lookups.insert_ready(
            regions_layer.id,
            "/fields/regions.v1.bin".to_string(),
            DiscreteFieldRows::from_u32_grid(2, 2, &[76; 4]).expect("region field"),
        );

        let mut field_metadata = FieldMetadataCache::default();
        field_metadata.insert_ready(
            zone_layer.id,
            "/fields/zone_mask.v1.meta.json".to_string(),
            FieldHoverMetadataAsset {
                entries: std::collections::BTreeMap::from([(
                    0x123456,
                    metadata_entry(
                        FIELD_DETAIL_FACT_KEY_ZONE,
                        "Zone",
                        "Velia Bay",
                        "hover-zone",
                    ),
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
                        FIELD_DETAIL_FACT_KEY_ORIGIN_REGION,
                        "Region",
                        "Tarif",
                        "hover-origin",
                    ),
                )]),
            },
        );

        let map_to_world = MapToWorld::default();
        let selected = selected_info_at_world_point(
            map_to_world.map_to_world(MapPoint::new(0.5, 0.5)),
            &WorldPointQueryContext {
                layer_registry: &registry,
                layer_runtime: &runtime,
                exact_lookups: &exact_lookups,
                field_metadata: &field_metadata,
                tile_cache: &RasterTileCache::default(),
                vector_runtime: &VectorLayerRuntime::default(),
                map_to_world,
            },
            FishyMapSelectionPointKind::Clicked,
            None,
            None,
        )
        .expect("selected info");

        assert_eq!(selected.zone_rgb_u32(), Some(0x123456));
        assert_eq!(selected.layer_samples.len(), 2);
        assert_eq!(selected.point_label.as_deref(), Some("Velia Bay"));
        assert!(selected
            .layer_samples
            .iter()
            .any(|sample| sample.layer_id == "zone_mask"));
        assert!(selected
            .layer_samples
            .iter()
            .any(|sample| sample.layer_id == "regions"));
    }

    #[test]
    fn semantic_layer_sample_for_field_id_reads_zone_row_from_field_metadata() {
        let registry = zone_registry();
        let zone_layer = registry.get_by_key("zone_mask").expect("zone layer");
        let mut field_metadata = FieldMetadataCache::default();
        field_metadata.insert_ready(
            zone_layer.id,
            "/fields/zone_mask.v1.meta.json".to_string(),
            FieldHoverMetadataAsset {
                entries: std::collections::BTreeMap::from([(
                    0x123456,
                    metadata_entry(
                        FIELD_DETAIL_FACT_KEY_ZONE,
                        "Zone",
                        "Velia Bay",
                        "hover-zone",
                    ),
                )]),
            },
        );
        let sample =
            semantic_layer_sample_for_field_id(&registry, &field_metadata, "zone_mask", 0x123456)
                .expect("zone sample");
        assert_eq!(sample.layer_id, "zone_mask");
        assert_eq!(sample.rgb_u32, 0x123456);
        assert_eq!(sample.field_id, Some(0x123456));
        assert_eq!(sample.detail_sections[0].facts[0].value, "Velia Bay");
    }

    #[test]
    fn selected_info_for_zone_rgb_uses_shared_zone_mask_layer_sample() {
        let registry = zone_registry();
        let zone_layer = registry.get_by_key("zone_mask").expect("zone layer");
        let mut field_metadata = FieldMetadataCache::default();
        field_metadata.insert_ready(
            zone_layer.id,
            "/fields/zone_mask.v1.meta.json".to_string(),
            FieldHoverMetadataAsset {
                entries: std::collections::BTreeMap::from([(
                    0x223344,
                    metadata_entry(
                        FIELD_DETAIL_FACT_KEY_ZONE,
                        "Zone",
                        "Cron Islands",
                        "hover-zone",
                    ),
                )]),
            },
        );

        let selected = selected_info_for_zone_rgb(&registry, &field_metadata, 0x223344, None);
        assert_eq!(selected.zone_rgb_u32(), Some(0x223344));
        assert!(!selected.sampled_world_point);
        assert_eq!(selected.point_kind, None);
        assert_eq!(selected.point_label.as_deref(), Some("Cron Islands"));
        assert!(!selected.world_x.is_finite());
        assert!(!selected.world_z.is_finite());
        assert_eq!(selected.layer_samples.len(), 1);
        assert_eq!(selected.layer_samples[0].layer_id, "zone_mask");
        assert_eq!(
            selected.layer_samples[0].detail_sections[0].facts[0].value,
            "Cron Islands"
        );
    }

    #[test]
    fn selected_info_for_semantic_field_keeps_non_zone_layer_sample() {
        let mut registry = LayerRegistry::default();
        registry.apply_layers_response(fishystuff_api::models::layers::LayersResponse {
            revision: "rev".to_string(),
            map_version_id: None,
            layers: vec![fishystuff_api::models::layers::LayerDescriptor {
                layer_id: "regions".to_string(),
                name: "Regions".to_string(),
                enabled: true,
                kind: fishystuff_api::models::layers::LayerKind::TiledRaster,
                transform: fishystuff_api::models::layers::LayerTransformDto::IdentityMapSpace,
                tileset: fishystuff_api::models::layers::TilesetRef::default(),
                tile_px: 512,
                max_level: 0,
                y_flip: false,
                field_source: Some(fishystuff_api::models::layers::FieldSourceRef {
                    url: "/fields/regions.v1.bin".to_string(),
                    revision: "regions-v1".to_string(),
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
        let region_layer = registry.get_by_key("regions").expect("regions layer");
        let mut field_metadata = FieldMetadataCache::default();
        field_metadata.insert_ready(
            region_layer.id,
            "/fields/regions.v1.meta.json".to_string(),
            FieldHoverMetadataAsset {
                entries: std::collections::BTreeMap::from([(
                    76,
                    metadata_entry(
                        FIELD_DETAIL_FACT_KEY_ORIGIN_REGION,
                        "Region",
                        "Grana",
                        "hover-origin",
                    ),
                )]),
            },
        );

        let selected =
            selected_info_for_semantic_field(&registry, &field_metadata, "regions", 76, None)
                .expect("selected info");
        assert_eq!(selected.zone_rgb_u32(), None);
        assert_eq!(selected.layer_samples.len(), 1);
        assert_eq!(selected.layer_samples[0].layer_id, "regions");
        assert_eq!(selected.layer_samples[0].field_id, Some(76));
        assert_eq!(selected.point_label.as_deref(), Some("Grana"));
        assert_eq!(
            selected.layer_samples[0].detail_sections[0].facts[0].value,
            "Grana"
        );
    }

    #[test]
    fn selected_info_at_world_point_prefers_lowest_layer_label_before_target_fallback() {
        let registry = semantic_registry();
        let zone_layer = registry.get_by_key("zone_mask").expect("zone layer");
        let regions_layer = registry.get_by_key("regions").expect("regions layer");

        let mut exact_lookups = ExactLookupCache::default();
        exact_lookups.insert_ready(
            zone_layer.id,
            "/fields/zone_mask.v1.bin".to_string(),
            DiscreteFieldRows::from_u32_grid(2, 2, &[0x123456; 4]).expect("zone field"),
        );
        exact_lookups.insert_ready(
            regions_layer.id,
            "/fields/regions.v1.bin".to_string(),
            DiscreteFieldRows::from_u32_grid(2, 2, &[76; 4]).expect("region field"),
        );

        let mut field_metadata = FieldMetadataCache::default();
        field_metadata.insert_ready(
            zone_layer.id,
            "/fields/zone_mask.v1.meta.json".to_string(),
            FieldHoverMetadataAsset {
                entries: std::collections::BTreeMap::from([(
                    0x123456,
                    metadata_entry(
                        FIELD_DETAIL_FACT_KEY_ZONE,
                        "Zone",
                        "Margoria South",
                        "hover-zone",
                    ),
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
                        FIELD_DETAIL_FACT_KEY_ORIGIN_REGION,
                        "Region",
                        "Margoria (RG218)",
                        "hover-origin",
                    ),
                )]),
            },
        );

        let map_to_world = MapToWorld::default();
        let selected = selected_info_at_world_point(
            map_to_world.map_to_world(MapPoint::new(0.5, 0.5)),
            &WorldPointQueryContext {
                layer_registry: &registry,
                layer_runtime: &LayerRuntime::default(),
                exact_lookups: &exact_lookups,
                field_metadata: &field_metadata,
                tile_cache: &RasterTileCache::default(),
                vector_runtime: &VectorLayerRuntime::default(),
                map_to_world,
            },
            FishyMapSelectionPointKind::Clicked,
            Some("Bookmark fallback"),
            None,
        )
        .expect("selected info");

        assert_eq!(selected.point_label.as_deref(), Some("Margoria South"));
    }
}
