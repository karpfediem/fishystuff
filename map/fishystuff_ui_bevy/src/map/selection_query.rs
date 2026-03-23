use crate::map::field_metadata::FieldMetadataCache;
use crate::map::field_semantics::semantic_sample_for_field_id;
use crate::map::field_view::sample_rgb_for_field_id;
use crate::map::hover_query::{hover_info_at_world_point, WorldPointQueryContext};
use crate::map::layer_query::LayerQuerySample;
use crate::map::layers::LayerRegistry;
use crate::map::spaces::WorldPoint;
use crate::plugins::api::{HoverInfo, SelectedInfo};

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
        layer_samples: hover.layer_samples.clone(),
    })
}

pub fn selected_info_at_world_point(
    world_point: WorldPoint,
    context: &WorldPointQueryContext<'_>,
) -> Option<SelectedInfo> {
    let hover = hover_info_at_world_point(world_point, context)?;
    selected_info_from_hover(&hover)
}

pub fn selected_info_for_zone_rgb(
    layer_registry: &LayerRegistry,
    field_metadata: &FieldMetadataCache,
    zone_rgb: u32,
) -> SelectedInfo {
    let layer_samples =
        semantic_layer_sample_for_field_id(layer_registry, field_metadata, "zone_mask", zone_rgb)
            .into_iter()
            .collect();
    SelectedInfo {
        map_px: 0,
        map_py: 0,
        world_x: f64::NAN,
        world_z: f64::NAN,
        sampled_world_point: false,
        layer_samples,
    }
}

pub fn selected_info_for_semantic_field(
    layer_registry: &LayerRegistry,
    field_metadata: &FieldMetadataCache,
    layer_key: &str,
    field_id: u32,
) -> Option<SelectedInfo> {
    let layer_sample =
        semantic_layer_sample_for_field_id(layer_registry, field_metadata, layer_key, field_id)?;
    Some(SelectedInfo {
        map_px: 0,
        map_py: 0,
        world_x: f64::NAN,
        world_z: f64::NAN,
        sampled_world_point: false,
        layer_samples: vec![layer_sample],
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
        rows: semantic.rows,
        targets: semantic.targets,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        selected_info_for_semantic_field, selected_info_for_zone_rgb, selected_info_from_hover,
        semantic_layer_sample_for_field_id,
    };
    use crate::map::field_metadata::FieldMetadataCache;
    use crate::map::layer_query::LayerQuerySample;
    use crate::map::layers::LayerRegistry;
    use crate::plugins::api::HoverInfo;
    use fishystuff_api::Rgb;
    use fishystuff_core::field_metadata::{
        FieldHoverMetadataAsset, FieldHoverMetadataEntry, FieldHoverRow, FIELD_HOVER_ROW_KEY_ZONE,
    };

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
                rows: Vec::new(),
                targets: Vec::new(),
            }],
        };

        let selected = selected_info_from_hover(&hover).expect("selected info");
        assert_eq!(selected.map_px, 12);
        assert_eq!(selected.map_py, 34);
        assert_eq!(selected.zone_rgb_u32(), Some(0x123456));
        assert!(selected.sampled_world_point);
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
                rows: Vec::new(),
                targets: Vec::new(),
            }],
        };

        let selected = selected_info_from_hover(&hover).expect("selected info");
        assert_eq!(selected.zone_rgb_u32(), None);
        assert!(selected.sampled_world_point);
        assert_eq!(selected.layer_samples, hover.layer_samples);
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
                    FieldHoverMetadataEntry {
                        rows: vec![FieldHoverRow {
                            key: FIELD_HOVER_ROW_KEY_ZONE.to_string(),
                            icon: "hover-zone".to_string(),
                            label: "Zone".to_string(),
                            value: "Velia Bay".to_string(),
                            hide_label: false,
                            status_icon: None,
                            status_icon_tone: None,
                        }],
                        targets: Vec::new(),
                    },
                )]),
            },
        );
        let sample =
            semantic_layer_sample_for_field_id(&registry, &field_metadata, "zone_mask", 0x123456)
                .expect("zone sample");
        assert_eq!(sample.layer_id, "zone_mask");
        assert_eq!(sample.rgb_u32, 0x123456);
        assert_eq!(sample.field_id, Some(0x123456));
        assert_eq!(sample.rows.len(), 1);
        assert_eq!(sample.rows[0].value, "Velia Bay");
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
                    FieldHoverMetadataEntry {
                        rows: vec![FieldHoverRow {
                            key: FIELD_HOVER_ROW_KEY_ZONE.to_string(),
                            icon: "hover-zone".to_string(),
                            label: "Zone".to_string(),
                            value: "Cron Islands".to_string(),
                            hide_label: false,
                            status_icon: None,
                            status_icon_tone: None,
                        }],
                        targets: Vec::new(),
                    },
                )]),
            },
        );

        let selected = selected_info_for_zone_rgb(&registry, &field_metadata, 0x223344);
        assert_eq!(selected.zone_rgb_u32(), Some(0x223344));
        assert!(!selected.sampled_world_point);
        assert!(!selected.world_x.is_finite());
        assert!(!selected.world_z.is_finite());
        assert_eq!(selected.layer_samples.len(), 1);
        assert_eq!(selected.layer_samples[0].layer_id, "zone_mask");
        assert_eq!(selected.layer_samples[0].rows[0].value, "Cron Islands");
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
                    FieldHoverMetadataEntry {
                        rows: vec![FieldHoverRow {
                            key: "origin".to_string(),
                            icon: "hover-origin".to_string(),
                            label: "Origin".to_string(),
                            value: "Grana".to_string(),
                            hide_label: false,
                            status_icon: None,
                            status_icon_tone: None,
                        }],
                        targets: Vec::new(),
                    },
                )]),
            },
        );

        let selected = selected_info_for_semantic_field(&registry, &field_metadata, "regions", 76)
            .expect("selected info");
        assert_eq!(selected.zone_rgb_u32(), None);
        assert_eq!(selected.layer_samples.len(), 1);
        assert_eq!(selected.layer_samples[0].layer_id, "regions");
        assert_eq!(selected.layer_samples[0].field_id, Some(76));
        assert_eq!(selected.layer_samples[0].rows[0].value, "Grana");
    }
}
