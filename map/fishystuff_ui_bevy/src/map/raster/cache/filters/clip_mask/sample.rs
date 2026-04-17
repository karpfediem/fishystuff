use fishystuff_core::masks::pack_rgb_u32;
use std::collections::HashSet;

use crate::map::exact_lookup::ExactLookupCache;
use crate::map::field_view::{loaded_field_layer, FieldLayerView};
use crate::map::layers::{LayerSpec, PickMode};
use crate::map::raster::{layer_map_version, TileKey};
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::WorldPoint;
use crate::plugins::api::ZoneMembershipFilter;
use crate::plugins::vector_layers::VectorLayerRuntime;

use super::super::super::RasterTileCache;

pub(crate) fn clip_mask_allows_world_point(
    layer_id: crate::map::layers::LayerId,
    world_point: WorldPoint,
    layer_registry: &crate::map::layers::LayerRegistry,
    layer_runtime: &crate::map::layers::LayerRuntime,
    exact_lookups: &ExactLookupCache,
    tile_cache: &RasterTileCache,
    vector_runtime: &VectorLayerRuntime,
    filter: &ZoneMembershipFilter,
    map_version: Option<&str>,
) -> Option<bool> {
    clip_mask_allows_world_point_inner(
        layer_id,
        world_point,
        layer_registry,
        layer_runtime,
        exact_lookups,
        tile_cache,
        vector_runtime,
        filter,
        map_version,
        &mut HashSet::new(),
    )
}

fn clip_mask_allows_world_point_inner(
    layer_id: crate::map::layers::LayerId,
    world_point: WorldPoint,
    layer_registry: &crate::map::layers::LayerRegistry,
    layer_runtime: &crate::map::layers::LayerRuntime,
    exact_lookups: &ExactLookupCache,
    tile_cache: &RasterTileCache,
    vector_runtime: &VectorLayerRuntime,
    filter: &ZoneMembershipFilter,
    map_version: Option<&str>,
    visited: &mut HashSet<crate::map::layers::LayerId>,
) -> Option<bool> {
    if !visited.insert(layer_id) {
        return Some(false);
    }

    let layer = layer_registry.get(layer_id)?;
    let visible_here = if let Some(field) = loaded_field_layer(layer, exact_lookups) {
        sample_field_clip_mask(layer, field, world_point, filter)
    } else if layer.is_raster() {
        sample_raster_clip_mask(layer, world_point, tile_cache, filter, map_version)
    } else if layer.is_vector() {
        sample_vector_clip_mask(
            layer,
            world_point,
            vector_runtime,
            layer_registry.map_version_id(),
        )
    } else {
        Some(true)
    }?;
    if !visible_here {
        return Some(false);
    }

    let Some(mask_layer_id) = layer_runtime.clip_mask_layer(layer_id) else {
        return Some(true);
    };
    clip_mask_allows_world_point_inner(
        mask_layer_id,
        world_point,
        layer_registry,
        layer_runtime,
        exact_lookups,
        tile_cache,
        vector_runtime,
        filter,
        map_version,
        visited,
    )
}

fn zone_filter_applies(layer: &LayerSpec, filter: &ZoneMembershipFilter) -> bool {
    filter.active && layer.pick_mode == PickMode::ExactTilePixel && layer.key == "zone_mask"
}

fn sample_field_clip_mask(
    layer: &LayerSpec,
    field: impl FieldLayerView,
    world_point: WorldPoint,
    filter: &ZoneMembershipFilter,
) -> Option<bool> {
    let map_to_world = MapToWorld::default();
    if !field.contains_at_world_point(layer, map_to_world, world_point) {
        return Some(false);
    }
    if zone_filter_applies(layer, filter) {
        let rgb = field.rgb_at_world_point(layer, map_to_world, world_point)?;
        return Some(filter.zone_rgbs.contains(&rgb.to_u32()));
    }
    Some(true)
}

fn sample_raster_clip_mask(
    layer: &LayerSpec,
    world_point: WorldPoint,
    tile_cache: &RasterTileCache,
    filter: &ZoneMembershipFilter,
    map_version: Option<&str>,
) -> Option<bool> {
    let world_transform = layer.world_transform(MapToWorld::default())?;
    let layer_px = world_transform.world_to_layer(world_point);
    if layer_px.x < 0.0 || layer_px.y < 0.0 {
        return Some(false);
    }
    let (map_version_id, _) = layer_map_version(layer, map_version)?;
    let layer_ix = layer_px.x.floor() as u32;
    let layer_iy = layer_px.y.floor() as u32;
    let [r, g, b, a] =
        sample_ready_raster_rgba(layer, map_version_id, layer_ix, layer_iy, tile_cache)?;
    if a == 0 {
        return Some(false);
    }
    if zone_filter_applies(layer, filter) {
        let rgb = pack_rgb_u32(r, g, b);
        return Some(filter.zone_rgbs.contains(&rgb));
    }
    Some(true)
}

fn sample_ready_raster_rgba(
    layer: &LayerSpec,
    map_version_id: u64,
    layer_ix: u32,
    layer_iy: u32,
    tile_cache: &RasterTileCache,
) -> Option<[u8; 4]> {
    let tile_px = u64::from(layer.tile_px.max(1));
    for z in 0..=i32::from(layer.max_level) {
        let downsample = 1_u64.checked_shl(z as u32)?;
        let tile_span = tile_px.checked_mul(downsample)?;
        let tx = u64::from(layer_ix) / tile_span;
        let ty = u64::from(layer_iy) / tile_span;
        let local_x = (u64::from(layer_ix) / downsample).checked_sub(tx.checked_mul(tile_px)?)?;
        let local_y = (u64::from(layer_iy) / downsample).checked_sub(ty.checked_mul(tile_px)?)?;
        let key = TileKey {
            layer: layer.id,
            map_version: map_version_id,
            z,
            tx: tx as i32,
            ty: ty as i32,
        };
        let Some(tile) = tile_cache.get_ready_pixel_data(&key) else {
            continue;
        };
        let local_x = local_x as u32;
        let local_y = local_y as u32;
        if local_x >= tile.width || local_y >= tile.height {
            continue;
        }
        let idx = ((local_y * tile.width + local_x) * 4) as usize;
        if idx + 3 >= tile.data.len() {
            continue;
        }
        return Some([
            tile.data[idx],
            tile.data[idx + 1],
            tile.data[idx + 2],
            tile.data[idx + 3],
        ]);
    }
    None
}

fn sample_vector_clip_mask(
    layer: &LayerSpec,
    world_point: WorldPoint,
    vector_runtime: &VectorLayerRuntime,
    registry_map_version_id: Option<&str>,
) -> Option<bool> {
    let source = layer.vector_source.as_ref()?;
    let revision = resolved_vector_revision_for_clip_mask(source, registry_map_version_id);
    let bundle = vector_runtime.finished.get_ref(&(layer.id, revision))?;
    Some(
        bundle
            .sample_rgb(world_point.x as f32, world_point.z as f32)
            .is_some_and(|rgba| rgba[3] > 0),
    )
}

fn resolved_vector_revision_for_clip_mask(
    source: &crate::map::layers::VectorSourceSpec,
    map_version_id: Option<&str>,
) -> String {
    let mut url = source.url.clone();
    if url.contains("{map_version}") {
        let version = map_version_id
            .filter(|value| !value.trim().is_empty() && *value != "0v0")
            .unwrap_or("v1");
        url = url.replace("{map_version}", version);
    }
    let revision = source.revision.trim();
    if revision.is_empty() {
        format!("url:{url}")
    } else {
        revision.to_string()
    }
}

#[cfg(test)]
mod tests {
    use fishystuff_api::Rgb;

    use crate::map::field_view::FieldLayerView;
    use crate::map::layers::{LayerId, LayerKind, LayerSpec, LodPolicy, PickMode};
    use crate::map::raster::cache::{RasterTileEntry, TilePixelData, TileState};
    use crate::map::spaces::layer_transform::LayerTransform;
    use crate::prelude::*;

    use super::*;

    fn test_layer(max_level: u8) -> LayerSpec {
        LayerSpec {
            id: LayerId::from_raw(0),
            key: "test".to_string(),
            name: "Test".to_string(),
            visible_default: true,
            opacity_default: 1.0,
            z_base: 0.0,
            kind: LayerKind::TiledRaster,
            tileset_url: "/tileset.json".to_string(),
            tile_url_template: "/tiles/{z}/{x}_{y}.png".to_string(),
            tileset_version: "v1".to_string(),
            field_source: None,
            field_metadata_source: None,
            vector_source: None,
            waypoint_source: None,
            transform: LayerTransform::IdentityMapSpace,
            tile_px: 512,
            max_level,
            y_flip: false,
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
            pick_mode: PickMode::None,
            display_order: 0,
            filter_bindings: Vec::new(),
        }
    }

    fn ready_entry(last_used: u64) -> RasterTileEntry {
        RasterTileEntry {
            handle: Handle::default(),
            entity: None,
            material: None,
            state: TileState::Ready,
            visible: false,
            alpha: 1.0,
            depth: 0.0,
            last_used,
            exact_quad: false,
            sprite_size: None,
            pixel_data: None,
            zone_rgbs: Vec::new(),
            zone_lookup_rows: None,
            filter_active: false,
            filter_revision: 0,
            pixel_filtered: false,
            hover_highlight_zone: None,
            clip_mask_layer: None,
            clip_mask_revision: 0,
            clip_mask_applied: false,
            linger_until_frame: 0,
        }
    }

    #[derive(Clone, Copy)]
    struct FakeFieldView {
        id: u32,
        present: bool,
    }

    impl FieldLayerView for FakeFieldView {
        fn width(&self) -> u16 {
            1
        }

        fn height(&self) -> u16 {
            1
        }

        fn field_id_at_map_px(&self, _map_px_x: i32, _map_px_y: i32) -> Option<u32> {
            self.present.then_some(self.id)
        }

        fn rgb_at_map_px(&self, _map_px_x: i32, _map_px_y: i32) -> Option<Rgb> {
            self.present.then_some(Rgb::from_u32(self.id))
        }

        fn render_rgba_chunk(
            &self,
            _source_origin_x: i32,
            _source_origin_y: i32,
            _source_width: u32,
            _source_height: u32,
            _output_width: u16,
            _output_height: u16,
        ) -> fishystuff_core::field::FieldRgbaChunk {
            unreachable!("not used in clip mask tests")
        }
    }

    #[test]
    fn raster_clip_mask_sampling_uses_loaded_ancestor_tiles() {
        let mut layer = test_layer(1);
        layer.tile_px = 2;
        let mut cache = RasterTileCache::default();
        let mut entry = ready_entry(1);
        entry.pixel_data = Some(TilePixelData {
            width: 2,
            height: 2,
            data: vec![
                1, 2, 3, 255, 4, 5, 6, 255, //
                7, 8, 9, 255, 10, 20, 30, 255,
            ],
        });
        cache.entries.insert(
            TileKey {
                layer: layer.id,
                map_version: 1,
                z: 1,
                tx: 0,
                ty: 0,
            },
            entry,
        );

        let rgba = sample_ready_raster_rgba(&layer, 1, 2, 2, &cache);
        assert_eq!(rgba, Some([10, 20, 30, 255]));
    }

    #[test]
    fn field_clip_mask_uses_shared_field_view_contract() {
        let mut layer = test_layer(0);
        layer.key = "zone_mask".to_string();
        layer.pick_mode = PickMode::ExactTilePixel;
        let filter = EvidenceZoneFilter {
            active: true,
            zone_rgbs: std::collections::HashSet::from([0x123456]),
            revision: 1,
        };
        let world_point = WorldPoint::new(10.0, 10.0);

        assert_eq!(
            sample_field_clip_mask(
                &layer,
                FakeFieldView {
                    id: 0,
                    present: true
                },
                world_point,
                &filter
            ),
            Some(false)
        );
        assert_eq!(
            sample_field_clip_mask(
                &layer,
                FakeFieldView {
                    id: 0x123456,
                    present: true,
                },
                world_point,
                &filter,
            ),
            Some(true)
        );
        assert_eq!(
            sample_field_clip_mask(
                &layer,
                FakeFieldView {
                    id: 0x654321,
                    present: true,
                },
                world_point,
                &filter,
            ),
            Some(false)
        );
    }
}
