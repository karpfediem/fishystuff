use fishystuff_api::Rgb;
use fishystuff_core::field_metadata::FIELD_HOVER_ROW_KEY_ZONE;

use crate::map::field_metadata::FieldMetadataCache;
use crate::map::field_semantics::field_row_value_for_id;
use crate::map::layers::LayerRegistry;
use crate::plugins::api::{
    build_zone_stats_request, spawn_zone_stats_request, ApiBootstrapState, PatchFilterState,
    PendingRequests, SelectionState,
};

pub(super) fn apply_zone_selection_command(
    bootstrap: &ApiBootstrapState,
    patch_filter: &PatchFilterState,
    layer_registry: &LayerRegistry,
    field_metadata: &FieldMetadataCache,
    selection: &mut SelectionState,
    pending: &mut PendingRequests,
    zone_rgb: u32,
) {
    let rgb = Rgb::from_u32(zone_rgb);
    selection.info = Some(crate::plugins::api::SelectedInfo {
        map_px: 0,
        map_py: 0,
        rgb,
        rgb_u32: zone_rgb,
        zone_name: resolve_zone_name(layer_registry, field_metadata, zone_rgb),
        world_x: 0.0,
        world_z: 0.0,
    });
    selection.zone_stats = None;
    selection.zone_stats_status = "zone stats: loading".to_string();
    if let Some(request) = build_zone_stats_request(bootstrap, patch_filter, rgb) {
        pending.zone_stats = Some((zone_rgb, spawn_zone_stats_request(request)));
    } else {
        selection.zone_stats_status = "zone stats: missing defaults".to_string();
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
    use super::resolve_zone_name;
    use crate::map::field_metadata::FieldMetadataCache;
    use crate::map::layers::LayerRegistry;
    use fishystuff_core::field_metadata::{
        FieldHoverMetadataAsset, FieldHoverMetadataEntry, FieldHoverRow,
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
}
