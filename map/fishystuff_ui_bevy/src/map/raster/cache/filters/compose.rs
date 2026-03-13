use fishystuff_core::masks::pack_rgb_u32;

use crate::map::layers::{LayerId, LayerRegistry, LayerSpec};
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::LayerPoint;
use crate::plugins::points::EvidenceZoneFilter;
use crate::plugins::vector_layers::VectorLayerRuntime;

use super::super::super::TileKey;
use super::super::{RasterTileCache, TilePixelData};
use super::clip_mask::clip_mask_allows_world_point;

const HOVER_HIGHLIGHT_RGB: [u8; 3] = [64, 255, 128];

pub(super) fn restore_rgba_in_place(source: &TilePixelData, image_data: &mut [u8]) {
    if image_data.len() != source.data.len() {
        return;
    }
    image_data.copy_from_slice(&source.data);
}

pub(super) fn compose_raster_visuals_in_place(
    source: &TilePixelData,
    image_data: &mut [u8],
    key: TileKey,
    layer: &LayerSpec,
    filter: &EvidenceZoneFilter,
    requires_pixel_filter: bool,
    hover_zone_rgb: Option<u32>,
    clip_mask_layer: Option<LayerId>,
    layer_registry: &LayerRegistry,
    tile_cache: &RasterTileCache,
    vector_runtime: &VectorLayerRuntime,
    map_version: Option<&str>,
) {
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
            if requires_pixel_filter && !filter.zone_rgbs.contains(&rgb) {
                dst[3] = 0;
                continue;
            }

            if hover_zone_rgb == Some(rgb) {
                dst[0] = HOVER_HIGHLIGHT_RGB[0];
                dst[1] = HOVER_HIGHLIGHT_RGB[1];
                dst[2] = HOVER_HIGHLIGHT_RGB[2];
            }

            let Some(mask_layer_id) = clip_mask_layer else {
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
                tile_cache,
                vector_runtime,
                filter,
                map_version,
            ) else {
                continue;
            };
            if !allowed {
                dst[3] = 0;
            }
        }
    }
}
