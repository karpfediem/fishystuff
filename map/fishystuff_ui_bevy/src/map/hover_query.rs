use fishystuff_core::field_metadata::{FieldHoverRow, FIELD_HOVER_ROW_KEY_ZONE};

use crate::map::exact_lookup::ExactLookupCache;
use crate::map::field_metadata::FieldMetadataCache;
use crate::map::layer_query::{
    sample_layers_at_world_point, LayerQuerySample, LayerSamplingContext,
};
use crate::map::layers::{LayerRegistry, LayerRuntime, LayerSpec};
use crate::map::raster::RasterTileCache;
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::WorldPoint;
use crate::plugins::api::HoverInfo;
use crate::plugins::vector_layers::VectorLayerRuntime;

pub struct WorldPointQueryContext<'a> {
    pub layer_registry: &'a LayerRegistry,
    pub layer_runtime: &'a LayerRuntime,
    pub exact_lookups: &'a ExactLookupCache,
    pub field_metadata: &'a FieldMetadataCache,
    pub tile_cache: &'a RasterTileCache,
    pub vector_runtime: &'a VectorLayerRuntime,
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
        &hover_layers,
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
    let zone_sample = zone_mask_hover_sample(&layer_samples);
    let zone_name = zone_sample.and_then(|sample| zone_name_from_hover_rows(&sample.rows));
    let zone_rgb = zone_sample.as_ref().map(|sample| sample.rgb);
    let zone_rgb_u32 = zone_sample.as_ref().map(|sample| sample.rgb_u32);
    Some(HoverInfo {
        map_px,
        map_py,
        rgb: zone_rgb,
        rgb_u32: zone_rgb_u32,
        zone_name,
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
        .filter(|layer| layer.key != "minimap" && layer_runtime.visible(layer.id))
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

fn zone_mask_hover_sample(layer_samples: &[LayerQuerySample]) -> Option<&LayerQuerySample> {
    layer_samples
        .iter()
        .find(|sample| sample.layer_id == "zone_mask")
}

fn zone_name_from_hover_rows(rows: &[FieldHoverRow]) -> Option<String> {
    rows.iter()
        .find(|row| row.key == FIELD_HOVER_ROW_KEY_ZONE)
        .map(|row| row.value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::{hover_info_at_world_point, zone_mask_hover_sample, zone_name_from_hover_rows};
    use crate::map::exact_lookup::ExactLookupCache;
    use crate::map::field_metadata::FieldMetadataCache;
    use crate::map::layer_query::LayerQuerySample;
    use crate::map::layers::{LayerRegistry, LayerRuntime};
    use crate::map::raster::RasterTileCache;
    use crate::map::spaces::world::MapToWorld;
    use crate::map::spaces::MapPoint;
    use crate::plugins::vector_layers::VectorLayerRuntime;
    use fishystuff_api::models::layers::{
        LayerDescriptor, LayerKind as LayerKindDto, LayerTransformDto, LayerUiInfo, LayersResponse,
        LodPolicyDto, TilesetRef,
    };
    use fishystuff_api::Rgb;
    use fishystuff_core::field_metadata::{FieldHoverRow, FIELD_HOVER_ROW_KEY_ZONE};

    #[test]
    fn zone_name_from_hover_rows_reads_zone_row() {
        let rows = vec![FieldHoverRow {
            key: FIELD_HOVER_ROW_KEY_ZONE.to_string(),
            icon: "hover-zone".to_string(),
            label: "Zone".to_string(),
            value: "Olvia Coast".to_string(),
            hide_label: false,
            status_icon: None,
            status_icon_tone: None,
        }];
        assert_eq!(
            zone_name_from_hover_rows(&rows),
            Some("Olvia Coast".to_string())
        );
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
                rows: Vec::new(),
                targets: Vec::new(),
            },
            LayerQuerySample {
                layer_id: "zone_mask".to_string(),
                layer_name: "Zone Mask".to_string(),
                kind: "field".to_string(),
                rgb: Rgb::from_u32(0x445566),
                rgb_u32: 0x445566,
                field_id: Some(0x445566),
                rows: Vec::new(),
                targets: Vec::new(),
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
            field_source: None,
            field_metadata_source: None,
            vector_source: None,
            lod_policy: LodPolicyDto::default(),
            ui: LayerUiInfo {
                visible_default,
                opacity_default: 1.0,
                z_base: 0.0,
                display_order,
            },
            request_weight: 1.0,
            pick_mode: "none".to_string(),
        }
    }

    #[test]
    fn hover_info_at_world_point_ignores_minimap_and_hidden_layers() {
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
        runtime.set_visible(region_groups.id, false);

        let map_to_world = MapToWorld::default();
        let info = hover_info_at_world_point(
            map_to_world.map_to_world(MapPoint::new(0.0, 0.0)),
            &super::WorldPointQueryContext {
                layer_registry: &registry,
                layer_runtime: &runtime,
                exact_lookups: &ExactLookupCache::default(),
                field_metadata: &FieldMetadataCache::default(),
                tile_cache: &RasterTileCache::default(),
                vector_runtime: &VectorLayerRuntime::default(),
                map_to_world,
            },
        )
        .expect("hover info");

        assert!(info.layer_samples.is_empty());
        assert_eq!(info.map_px, 0);
        assert_eq!(info.map_py, 0);
    }
}
