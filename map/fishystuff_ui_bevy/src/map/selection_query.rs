use fishystuff_core::field_metadata::FIELD_HOVER_ROW_KEY_ZONE;

use crate::map::field_metadata::FieldMetadataCache;
use crate::map::field_semantics::field_row_value_for_id;
use crate::map::hover_query::{hover_info_at_world_point, WorldPointQueryContext};
use crate::map::layers::LayerRegistry;
use crate::map::spaces::WorldPoint;
use crate::plugins::api::{HoverInfo, SelectedInfo};

pub fn selected_info_from_hover(hover: &HoverInfo) -> Option<SelectedInfo> {
    let (rgb, rgb_u32) = (hover.rgb?, hover.rgb_u32?);
    Some(SelectedInfo {
        map_px: hover.map_px,
        map_py: hover.map_py,
        rgb,
        rgb_u32,
        zone_name: hover.zone_name.clone(),
        world_x: hover.world_x,
        world_z: hover.world_z,
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
    let rgb = fishystuff_api::Rgb::from_u32(zone_rgb);
    SelectedInfo {
        map_px: 0,
        map_py: 0,
        rgb,
        rgb_u32: zone_rgb,
        zone_name: resolve_zone_name(layer_registry, field_metadata, zone_rgb),
        world_x: 0.0,
        world_z: 0.0,
        layer_samples: Vec::new(),
    }
}

fn resolve_zone_name(
    layer_registry: &LayerRegistry,
    field_metadata: &FieldMetadataCache,
    zone_rgb: u32,
) -> Option<String> {
    let layer = layer_registry.get_by_key("zone_mask")?;
    field_row_value_for_id(layer, field_metadata, zone_rgb, FIELD_HOVER_ROW_KEY_ZONE)
}

#[cfg(test)]
mod tests {
    use super::{resolve_zone_name, selected_info_for_zone_rgb, selected_info_from_hover};
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
            rgb: Some(Rgb::from_u32(0x123456)),
            rgb_u32: Some(0x123456),
            zone_name: Some("Olvia Coast".to_string()),
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
        assert_eq!(selected.rgb_u32, 0x123456);
        assert_eq!(selected.zone_name.as_deref(), Some("Olvia Coast"));
        assert_eq!(selected.world_x, 1.25);
        assert_eq!(selected.world_z, 2.5);
        assert_eq!(selected.layer_samples, hover.layer_samples);
    }

    #[test]
    fn resolve_zone_name_reads_zone_row_from_field_metadata() {
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
        assert_eq!(
            resolve_zone_name(&registry, &field_metadata, 0x123456),
            Some("Velia Bay".to_string())
        );
    }

    #[test]
    fn selected_info_for_zone_rgb_uses_shared_zone_name_lookup() {
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
        assert_eq!(selected.rgb_u32, 0x223344);
        assert_eq!(selected.zone_name.as_deref(), Some("Cron Islands"));
        assert_eq!(selected.world_x, 0.0);
        assert_eq!(selected.world_z, 0.0);
        assert!(selected.layer_samples.is_empty());
    }
}
