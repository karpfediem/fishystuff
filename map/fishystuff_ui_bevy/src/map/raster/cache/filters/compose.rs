use fishystuff_core::masks::pack_rgb_u32;
use fishystuff_core::masks::ZoneLookupRows;

use crate::map::exact_lookup::ExactLookupCache;
use crate::map::field_view::{loaded_field_layer, FieldLayerView};
use crate::map::layers::{LayerId, LayerRegistry, LayerRuntime, LayerSpec};
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::LayerPoint;
use crate::plugins::api::ZoneMembershipFilter;
use crate::plugins::vector_layers::VectorLayerRuntime;

use super::super::super::TileKey;
use super::super::{RasterTileCache, TilePixelData};
use super::clip_mask::clip_mask_allows_world_point;

const HOVER_HIGHLIGHT_RGB: [u8; 3] = [48, 255, 96];

pub(super) struct RasterVisualComposeContext<'a> {
    pub(super) key: TileKey,
    pub(super) layer: &'a LayerSpec,
    pub(super) filter: &'a ZoneMembershipFilter,
    pub(super) requires_pixel_filter: bool,
    pub(super) hover_zone_rgb: Option<u32>,
    pub(super) clip_mask_layer: Option<LayerId>,
    pub(super) layer_registry: &'a LayerRegistry,
    pub(super) layer_runtime: &'a LayerRuntime,
    pub(super) exact_lookups: &'a ExactLookupCache,
    pub(super) tile_cache: &'a RasterTileCache,
    pub(super) vector_runtime: &'a VectorLayerRuntime,
    pub(super) map_version: Option<&'a str>,
}

pub(super) fn restore_rgba_in_place(source: &TilePixelData, image_data: &mut [u8]) {
    if image_data.len() != source.data.len() {
        return;
    }
    image_data.copy_from_slice(&source.data);
}

pub(super) fn update_hover_highlight_in_place(
    source: &TilePixelData,
    image_data: &mut [u8],
    zone_rows: &ZoneLookupRows,
    previous_hover_zone_rgb: Option<u32>,
    next_hover_zone_rgb: Option<u32>,
) {
    if image_data.len() != source.data.len() || previous_hover_zone_rgb == next_hover_zone_rgb {
        return;
    }
    if let Some(previous_rgb) = previous_hover_zone_rgb {
        restore_zone_rgb_spans(source, image_data, zone_rows, previous_rgb);
    }
    if let Some(next_rgb) = next_hover_zone_rgb {
        apply_zone_rgb_highlight(image_data, zone_rows, next_rgb);
    }
}

pub(super) fn compose_raster_visuals_in_place(
    source: &TilePixelData,
    image_data: &mut [u8],
    context: &RasterVisualComposeContext<'_>,
) {
    let RasterVisualComposeContext {
        key,
        layer,
        filter,
        requires_pixel_filter,
        hover_zone_rgb,
        clip_mask_layer,
        layer_registry,
        layer_runtime,
        exact_lookups,
        tile_cache,
        vector_runtime,
        map_version,
    } = context;
    if image_data.len() != source.data.len() {
        return;
    }
    let Some(target_transform) = layer.world_transform(MapToWorld::default()) else {
        restore_rgba_in_place(source, image_data);
        return;
    };
    let tile_px = f64::from(layer.tile_px.max(1));
    let px_scale_x = tile_px / f64::from(source.width.max(1));
    let px_scale_y = tile_px / f64::from(source.height.max(1));

    for (row_idx, (src_row, dst_row)) in source
        .data
        .chunks_exact((source.width * 4) as usize)
        .zip(image_data.chunks_exact_mut((source.width * 4) as usize))
        .enumerate()
    {
        for (col_idx, (src, dst)) in src_row
            .chunks_exact(4)
            .zip(dst_row.chunks_exact_mut(4))
            .enumerate()
        {
            dst[0] = src[0];
            dst[1] = src[1];
            dst[2] = src[2];
            dst[3] = src[3];

            let source_rgb = pack_rgb_u32(src[0], src[1], src[2]);
            let layer_point = LayerPoint::new(
                f64::from(key.tx) * tile_px + (col_idx as f64 + 0.5) * px_scale_x,
                f64::from(key.ty) * tile_px + (row_idx as f64 + 0.5) * px_scale_y,
            );
            let filter_rgb = sample_zone_filter_rgb_at_layer_point(
                layer,
                exact_lookups,
                layer_point,
                source_rgb,
            );

            if *requires_pixel_filter
                && !filter_rgb.is_some_and(|rgb| filter.zone_rgbs.contains(&rgb))
            {
                dst[3] = 0;
                continue;
            }

            if *hover_zone_rgb == filter_rgb {
                dst[0] = HOVER_HIGHLIGHT_RGB[0];
                dst[1] = HOVER_HIGHLIGHT_RGB[1];
                dst[2] = HOVER_HIGHLIGHT_RGB[2];
            }

            let Some(mask_layer_id) = *clip_mask_layer else {
                continue;
            };
            let world_point = target_transform.layer_to_world(layer_point);
            let Some(allowed) = clip_mask_allows_world_point(
                mask_layer_id,
                world_point,
                layer_registry,
                layer_runtime,
                exact_lookups,
                tile_cache,
                vector_runtime,
                filter,
                *map_version,
            ) else {
                continue;
            };
            if !allowed {
                dst[3] = 0;
            }
        }
    }
}

fn sample_zone_filter_rgb_at_layer_point(
    layer: &LayerSpec,
    exact_lookups: &ExactLookupCache,
    layer_point: LayerPoint,
    source_rgb: u32,
) -> Option<u32> {
    if !layer.is_zone_mask_visual_layer() {
        return Some(source_rgb);
    }
    let Some(field) = loaded_field_layer(layer, exact_lookups) else {
        return Some(source_rgb);
    };
    field
        .rgb_at_layer_point(layer_point)
        .map(|rgb| rgb.to_u32())
}

fn restore_zone_rgb_spans(
    source: &TilePixelData,
    image_data: &mut [u8],
    zone_rows: &ZoneLookupRows,
    target_rgb: u32,
) {
    let row_stride = source.width as usize * 4;
    zone_rows.for_each_span_matching(target_rgb, |row, start_x, end_x| {
        let row_offset = row as usize * row_stride;
        let start = row_offset + start_x as usize * 4;
        let end = row_offset + end_x as usize * 4;
        image_data[start..end].copy_from_slice(&source.data[start..end]);
    });
}

fn apply_zone_rgb_highlight(image_data: &mut [u8], zone_rows: &ZoneLookupRows, target_rgb: u32) {
    let row_stride = zone_rows.width() as usize * 4;
    zone_rows.for_each_span_matching(target_rgb, |row, start_x, end_x| {
        let row_offset = row as usize * row_stride;
        let start = row_offset + start_x as usize * 4;
        let end = row_offset + end_x as usize * 4;
        for pixel in image_data[start..end].chunks_exact_mut(4) {
            pixel[0] = HOVER_HIGHLIGHT_RGB[0];
            pixel[1] = HOVER_HIGHLIGHT_RGB[1];
            pixel[2] = HOVER_HIGHLIGHT_RGB[2];
        }
    });
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use fishystuff_core::masks::pack_rgb_u32;
    use fishystuff_core::masks::ZoneLookupRows;

    use super::{
        compose_raster_visuals_in_place, sample_zone_filter_rgb_at_layer_point,
        update_hover_highlight_in_place, HOVER_HIGHLIGHT_RGB,
    };
    use crate::map::exact_lookup::ExactLookupCache;
    use crate::map::layers::{
        LayerId, LayerKind, LayerRegistry, LayerRuntime, LodPolicy, PickMode,
    };
    use crate::map::raster::cache::TilePixelData;
    use crate::map::raster::{RasterTileCache, TileKey};
    use crate::map::spaces::layer_transform::LayerTransform;
    use crate::map::spaces::LayerPoint;
    use crate::plugins::api::ZoneMembershipFilter;
    use crate::plugins::vector_layers::VectorLayerRuntime;

    fn zone_mask_layer() -> crate::map::layers::LayerSpec {
        crate::map::layers::LayerSpec {
            id: LayerId::from_raw(1),
            key: "zone_mask".to_string(),
            name: "Zone Mask".to_string(),
            visible_default: true,
            opacity_default: 1.0,
            z_base: 0.0,
            kind: LayerKind::TiledRaster,
            tileset_url: "/images/tiles/zone_mask_visual/v1/tileset.json".to_string(),
            tile_url_template: "/images/tiles/zone_mask_visual/v1/{z}/{x}_{y}.png".to_string(),
            tileset_version: "v1".to_string(),
            vector_source: None,
            waypoint_source: None,
            transform: LayerTransform::IdentityMapSpace,
            tile_px: 1,
            max_level: 0,
            y_flip: false,
            field_source: None,
            field_metadata_source: None,
            lod_policy: LodPolicy {
                target_tiles: 64,
                hysteresis_hi: 80.0,
                hysteresis_lo: 40.0,
                margin_tiles: 0,
                enable_refine: true,
                refine_debounce_ms: 0,
                max_detail_tiles: 128,
                max_resident_tiles: 256,
                pinned_coarse_levels: 2,
                coarse_pin_min_level: None,
                warm_margin_tiles: 1,
                protected_margin_tiles: 0,
                detail_eviction_weight: 4.0,
                max_detail_requests_while_camera_moving: 1,
                motion_suppresses_refine: true,
            },
            request_weight: 1.0,
            pick_mode: PickMode::ExactTilePixel,
            display_order: 0,
            filter_bindings: Vec::new(),
        }
    }

    #[test]
    fn hover_highlight_delta_only_mutates_old_and_new_zone_spans() {
        let source = TilePixelData {
            width: 4,
            height: 1,
            data: vec![1, 2, 3, 255, 1, 2, 3, 255, 4, 5, 6, 255, 4, 5, 6, 255],
        };
        let zone_rows = fishystuff_core::masks::ZoneLookupRows::from_rgba(
            source.width,
            source.height,
            &source.data,
        )
        .expect("zone rows");
        let mut image_data = source.data.clone();

        update_hover_highlight_in_place(
            &source,
            &mut image_data,
            &zone_rows,
            None,
            Some(pack_rgb_u32(4, 5, 6)),
        );
        assert_eq!(&image_data[0..8], &source.data[0..8]);
        assert_eq!(&image_data[8..11], &HOVER_HIGHLIGHT_RGB);
        assert_eq!(&image_data[12..15], &HOVER_HIGHLIGHT_RGB);

        update_hover_highlight_in_place(
            &source,
            &mut image_data,
            &zone_rows,
            Some(pack_rgb_u32(4, 5, 6)),
            Some(pack_rgb_u32(1, 2, 3)),
        );
        assert_eq!(&image_data[8..16], &source.data[8..16]);
        assert_eq!(&image_data[0..3], &HOVER_HIGHLIGHT_RGB);
        assert_eq!(&image_data[4..7], &HOVER_HIGHLIGHT_RGB);
    }

    #[test]
    fn zone_mask_filter_sampling_prefers_exact_lookup_rgb() {
        let layer = zone_mask_layer();
        let mut exact_lookups = ExactLookupCache::default();
        exact_lookups.insert_ready(
            layer.id,
            layer.exact_lookup_url().expect("exact lookup url"),
            ZoneLookupRows::from_rgba(1, 1, &[10, 20, 30, 255]).expect("zone rows"),
        );

        let sampled = sample_zone_filter_rgb_at_layer_point(
            &layer,
            &exact_lookups,
            LayerPoint::new(0.5, 0.5),
            pack_rgb_u32(1, 2, 3),
        );

        assert_eq!(sampled, Some(pack_rgb_u32(10, 20, 30)));
    }

    #[test]
    fn compose_filters_zone_mask_visuals_using_exact_lookup_rgb() {
        let layer = zone_mask_layer();
        let mut exact_lookups = ExactLookupCache::default();
        exact_lookups.insert_ready(
            layer.id,
            layer.exact_lookup_url().expect("exact lookup url"),
            ZoneLookupRows::from_rgba(1, 1, &[10, 20, 30, 255]).expect("zone rows"),
        );
        let source = TilePixelData {
            width: 1,
            height: 1,
            data: vec![1, 2, 3, 255],
        };
        let mut image_data = source.data.clone();
        let filter = ZoneMembershipFilter {
            active: true,
            zone_rgbs: HashSet::from([pack_rgb_u32(10, 20, 30)]),
            revision: 1,
        };
        let layer_registry = LayerRegistry::default();
        let layer_runtime = LayerRuntime::default();
        let tile_cache = RasterTileCache::default();
        let vector_runtime = VectorLayerRuntime::default();

        compose_raster_visuals_in_place(
            &source,
            &mut image_data,
            &super::RasterVisualComposeContext {
                key: TileKey {
                    layer: layer.id,
                    map_version: 0,
                    z: 0,
                    tx: 0,
                    ty: 0,
                },
                layer: &layer,
                filter: &filter,
                requires_pixel_filter: true,
                hover_zone_rgb: None,
                clip_mask_layer: None,
                layer_registry: &layer_registry,
                layer_runtime: &layer_runtime,
                exact_lookups: &exact_lookups,
                tile_cache: &tile_cache,
                vector_runtime: &vector_runtime,
                map_version: None,
            },
        );

        assert_eq!(image_data, source.data);
    }
}
