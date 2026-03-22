use fishystuff_core::masks::pack_rgb_u32;
use fishystuff_core::masks::ZoneLookupRows;

use crate::map::exact_lookup::ExactLookupCache;
use crate::map::layers::{LayerId, LayerRegistry, LayerSpec};
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::LayerPoint;
use crate::plugins::points::EvidenceZoneFilter;
use crate::plugins::vector_layers::VectorLayerRuntime;

use super::super::super::TileKey;
use super::super::{RasterTileCache, TilePixelData};
use super::clip_mask::clip_mask_allows_world_point;

const HOVER_HIGHLIGHT_RGB: [u8; 3] = [48, 255, 96];

pub(super) struct RasterVisualComposeContext<'a> {
    pub(super) key: TileKey,
    pub(super) layer: &'a LayerSpec,
    pub(super) filter: &'a EvidenceZoneFilter,
    pub(super) requires_pixel_filter: bool,
    pub(super) hover_zone_rgb: Option<u32>,
    pub(super) clip_mask_layer: Option<LayerId>,
    pub(super) layer_registry: &'a LayerRegistry,
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

            let rgb = pack_rgb_u32(src[0], src[1], src[2]);
            if *requires_pixel_filter && !filter.zone_rgbs.contains(&rgb) {
                dst[3] = 0;
                continue;
            }

            if *hover_zone_rgb == Some(rgb) {
                dst[0] = HOVER_HIGHLIGHT_RGB[0];
                dst[1] = HOVER_HIGHLIGHT_RGB[1];
                dst[2] = HOVER_HIGHLIGHT_RGB[2];
            }

            let Some(mask_layer_id) = *clip_mask_layer else {
                continue;
            };
            let layer_point = LayerPoint::new(
                f64::from(key.tx) * tile_px + (col_idx as f64 + 0.5) * px_scale_x,
                f64::from(key.ty) * tile_px + (row_idx as f64 + 0.5) * px_scale_y,
            );
            let world_point = target_transform.layer_to_world(layer_point);
            let Some(allowed) = clip_mask_allows_world_point(
                mask_layer_id,
                world_point,
                layer_registry,
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
    use fishystuff_core::masks::pack_rgb_u32;

    use super::{update_hover_highlight_in_place, HOVER_HIGHLIGHT_RGB};
    use crate::map::raster::cache::TilePixelData;

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
}
