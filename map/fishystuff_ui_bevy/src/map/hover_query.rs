use crate::map::exact_lookup::ExactLookupCache;
use crate::map::field_metadata::FieldMetadataCache;
use crate::map::layer_query::{sample_layers_at_world_point, LayerSamplingContext};
use crate::map::layers::{LayerRegistry, LayerRuntime, LayerSpec};
use crate::map::raster::cache::clip_mask_allows_world_point;
use crate::map::raster::RasterTileCache;
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::WorldPoint;
use crate::plugins::api::{HoverInfo, LayerEffectiveFilterState};
use crate::plugins::points::EvidenceZoneFilter;
use crate::plugins::vector_layers::VectorLayerRuntime;

pub struct WorldPointQueryContext<'a> {
    pub layer_registry: &'a LayerRegistry,
    pub layer_runtime: &'a LayerRuntime,
    pub exact_lookups: &'a ExactLookupCache,
    pub field_metadata: &'a FieldMetadataCache,
    pub tile_cache: &'a RasterTileCache,
    pub vector_runtime: &'a VectorLayerRuntime,
    pub layer_filters: &'a LayerEffectiveFilterState,
    pub map_to_world: MapToWorld,
}

pub fn hover_info_at_world_point(
    world_point: WorldPoint,
    context: &WorldPointQueryContext<'_>,
) -> Option<HoverInfo> {
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

    let map_px = map_x.floor() as i32;
    let map_py = map_y.floor() as i32;
    let hover_layers = current_hover_layers(context.layer_registry, context.layer_runtime);
    let layer_samples = sample_layers_at_world_point(
        &hover_layers
            .into_iter()
            .filter(|layer| {
                let inactive_filter = EvidenceZoneFilter::default();
                let zone_filter = context
                    .layer_filters
                    .zone_membership_filter(layer.key.as_str())
                    .unwrap_or(&inactive_filter);
                !matches!(
                    clip_mask_allows_world_point(
                        layer.id,
                        world_point,
                        context.layer_registry,
                        context.layer_runtime,
                        context.exact_lookups,
                        context.tile_cache,
                        context.vector_runtime,
                        zone_filter,
                        context.layer_registry.map_version_id(),
                    ),
                    Some(false)
                )
            })
            .collect::<Vec<_>>(),
        &LayerSamplingContext {
            exact_lookups: context.exact_lookups,
            field_metadata: context.field_metadata,
            tile_cache: context.tile_cache,
            vector_runtime: context.vector_runtime,
            world_point,
            map_to_world: context.map_to_world,
            map_version_id: context.layer_registry.map_version_id(),
        },
    );
    Some(HoverInfo {
        map_px,
        map_py,
        world_x: world_point.x,
        world_z: world_point.z,
        layer_samples,
    })
}

fn current_hover_layers<'a>(
    layer_registry: &'a LayerRegistry,
    layer_runtime: &LayerRuntime,
) -> Vec<&'a LayerSpec> {
    let mut layers = layer_registry
        .ordered()
        .iter()
        .filter(|layer| {
            layer.key != "minimap"
                && ((layer.field_url().is_some() && layer.field_metadata_url().is_some())
                    || layer_runtime.visible(layer.id))
        })
        .collect::<Vec<_>>();
    layers.sort_by(|lhs, rhs| {
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
    layers
}

#[cfg(test)]
mod tests {
    use super::hover_info_at_world_point;
    use crate::map::exact_lookup::ExactLookupCache;
    use crate::map::field_metadata::FieldMetadataCache;
    use crate::map::layer_query::LayerQuerySample;
    use crate::map::layers::{LayerRegistry, LayerRuntime};
    use crate::map::raster::RasterTileCache;
    use crate::map::spaces::world::MapToWorld;
    use crate::map::spaces::MapPoint;
    use crate::plugins::api::LayerEffectiveFilterState;
    use crate::plugins::points::EvidenceZoneFilter;
    use crate::plugins::vector_layers::VectorLayerRuntime;
    use fishystuff_api::models::layers::{
        LayerDescriptor, LayerKind as LayerKindDto, LayerTransformDto, LayerUiInfo, LayersResponse,
        LodPolicyDto, TilesetRef,
    };
    use fishystuff_api::Rgb;
    use fishystuff_core::field_metadata::{FieldDetailFact, FieldDetailSection};

    fn metadata_entry(
        key: &str,
        label: &str,
        value: &str,
        icon: &str,
    ) -> fishystuff_core::field_metadata::FieldHoverMetadataEntry {
        fishystuff_core::field_metadata::FieldHoverMetadataEntry {
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

    fn zone_mask_hover_sample(layer_samples: &[LayerQuerySample]) -> Option<&LayerQuerySample> {
        layer_samples
            .iter()
            .find(|sample| sample.layer_id == "zone_mask")
    }

    #[test]
    fn zone_mask_hover_sample_prefers_zone_mask_layer_id() {
        let samples = vec![
            LayerQuerySample {
                layer_id: "regions".to_string(),
                layer_name: "Regions".to_string(),
                kind: "field".to_string(),
                rgb: Rgb::from_u32(0x112233),
                rgb_u32: 0x112233,
                field_id: Some(88),
                targets: Vec::new(),
                detail_pane: None,
                detail_sections: Vec::new(),
            },
            LayerQuerySample {
                layer_id: "zone_mask".to_string(),
                layer_name: "Zone Mask".to_string(),
                kind: "field".to_string(),
                rgb: Rgb::from_u32(0x445566),
                rgb_u32: 0x445566,
                field_id: Some(0x445566),
                targets: Vec::new(),
                detail_pane: None,
                detail_sections: Vec::new(),
            },
        ];
        assert_eq!(
            zone_mask_hover_sample(&samples).map(|sample| sample.rgb_u32),
            Some(0x445566)
        );
    }

    fn layer_descriptor(
        layer_id: &str,
        display_order: i32,
        visible_default: bool,
    ) -> LayerDescriptor {
        LayerDescriptor {
            layer_id: layer_id.to_string(),
            name: layer_id.to_string(),
            enabled: true,
            kind: LayerKindDto::TiledRaster,
            transform: LayerTransformDto::IdentityMapSpace,
            tileset: TilesetRef::default(),
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
            lod_policy: LodPolicyDto::default(),
            ui: LayerUiInfo {
                visible_default,
                opacity_default: 1.0,
                z_base: 0.0,
                display_order,
            },
            filter_bindings: Vec::new(),
            request_weight: 1.0,
            pick_mode: "none".to_string(),
        }
    }

    #[test]
    fn hover_info_at_world_point_keeps_hidden_semantic_layers() {
        let mut registry = LayerRegistry::default();
        registry.apply_layers_response(LayersResponse {
            revision: "rev".to_string(),
            map_version_id: None,
            layers: vec![
                layer_descriptor("minimap", 10, true),
                layer_descriptor("regions", 40, true),
                layer_descriptor("region_groups", 30, true),
            ],
        });
        let mut runtime = LayerRuntime::default();
        runtime.sync_to_registry(&registry);
        let region_groups = registry
            .get_by_key("region_groups")
            .expect("region_groups layer");
        let regions = registry.get_by_key("regions").expect("regions layer");
        runtime.set_visible(region_groups.id, false);
        runtime.set_visible(regions.id, false);

        let mut exact_lookups = ExactLookupCache::default();
        exact_lookups.insert_ready(
            region_groups.id,
            "/fields/region_groups.v1.bin".to_string(),
            fishystuff_core::field::DiscreteFieldRows::from_u32_grid(1, 1, &[16]).expect("field"),
        );
        exact_lookups.insert_ready(
            regions.id,
            "/fields/regions.v1.bin".to_string(),
            fishystuff_core::field::DiscreteFieldRows::from_u32_grid(1, 1, &[76]).expect("field"),
        );
        let mut field_metadata = FieldMetadataCache::default();
        field_metadata.insert_ready(
            region_groups.id,
            "/fields/region_groups.v1.meta.json".to_string(),
            fishystuff_core::field_metadata::FieldHoverMetadataAsset {
                entries: std::collections::BTreeMap::from([(
                    16,
                    metadata_entry(
                        "resource_region",
                        "Containing region",
                        "Tarif",
                        "hover-resources",
                    ),
                )]),
            },
        );
        field_metadata.insert_ready(
            regions.id,
            "/fields/regions.v1.meta.json".to_string(),
            fishystuff_core::field_metadata::FieldHoverMetadataAsset {
                entries: std::collections::BTreeMap::from([(
                    76,
                    metadata_entry("origin_region", "Region", "Tarif", "hover-origin"),
                )]),
            },
        );

        let map_to_world = MapToWorld::default();
        let _evidence_zone_filter = EvidenceZoneFilter::default();
        let layer_filters = LayerEffectiveFilterState::default();
        let info = hover_info_at_world_point(
            map_to_world.map_to_world(MapPoint::new(0.0, 0.0)),
            &super::WorldPointQueryContext {
                layer_registry: &registry,
                layer_runtime: &runtime,
                exact_lookups: &exact_lookups,
                field_metadata: &field_metadata,
                tile_cache: &RasterTileCache::default(),
                vector_runtime: &VectorLayerRuntime::default(),
                layer_filters: &layer_filters,
                map_to_world,
            },
        )
        .expect("hover info");

        assert_eq!(info.layer_samples.len(), 2);
        assert_eq!(info.zone_rgb_u32(), None);
        assert_eq!(info.map_px, 0);
        assert_eq!(info.map_py, 0);
    }
}
