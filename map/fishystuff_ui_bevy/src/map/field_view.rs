use fishystuff_api::Rgb;
use fishystuff_core::field::{DiscreteFieldRows, FieldRgbaChunk};

use crate::map::exact_lookup::ExactLookupCache;
use crate::map::layers::{FieldColorMode, LayerSpec};
use crate::map::spaces::layer_transform::WorldTransform;
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::{LayerPoint, WorldPoint};

pub trait FieldLayerView {
    fn width(&self) -> u16;
    fn height(&self) -> u16;
    fn field_id_at_map_px(&self, map_px_x: i32, map_px_y: i32) -> Option<u32>;

    fn field_id_at_layer_point(&self, layer_point: LayerPoint) -> Option<u32> {
        let (layer_px_x, layer_px_y) = layer_point_to_px(layer_point)?;
        self.field_id_at_map_px(layer_px_x, layer_px_y)
    }

    fn contains_at_map_px(&self, map_px_x: i32, map_px_y: i32) -> bool {
        self.field_id_at_map_px(map_px_x, map_px_y)
            .map(|id| id != 0)
            .unwrap_or(false)
    }

    fn contains_at_layer_point(&self, layer_point: LayerPoint) -> bool {
        self.field_id_at_layer_point(layer_point)
            .map(|id| id != 0)
            .unwrap_or(false)
    }

    fn field_id_at_world_point(
        &self,
        layer: &LayerSpec,
        map_to_world: MapToWorld,
        world_point: WorldPoint,
    ) -> Option<u32> {
        let world_transform = layer.world_transform(map_to_world)?;
        self.field_id_at_world_point_with_transform(world_transform, world_point)
    }

    fn field_id_at_world_point_with_transform(
        &self,
        world_transform: WorldTransform,
        world_point: WorldPoint,
    ) -> Option<u32> {
        self.field_id_at_layer_point(world_transform.world_to_layer(world_point))
    }

    fn rgb_at_map_px(&self, map_px_x: i32, map_px_y: i32) -> Option<Rgb>;

    fn rgb_at_layer_point(&self, layer_point: LayerPoint) -> Option<Rgb> {
        let (layer_px_x, layer_px_y) = layer_point_to_px(layer_point)?;
        self.rgb_at_map_px(layer_px_x, layer_px_y)
    }

    fn rgb_at_world_point(
        &self,
        layer: &LayerSpec,
        map_to_world: MapToWorld,
        world_point: WorldPoint,
    ) -> Option<Rgb> {
        let world_transform = layer.world_transform(map_to_world)?;
        self.rgb_at_world_point_with_transform(world_transform, world_point)
    }

    fn rgb_at_world_point_with_transform(
        &self,
        world_transform: WorldTransform,
        world_point: WorldPoint,
    ) -> Option<Rgb> {
        self.rgb_at_layer_point(world_transform.world_to_layer(world_point))
    }

    fn contains_at_world_point(
        &self,
        layer: &LayerSpec,
        map_to_world: MapToWorld,
        world_point: WorldPoint,
    ) -> bool {
        let Some(world_transform) = layer.world_transform(map_to_world) else {
            return false;
        };
        self.contains_at_world_point_with_transform(world_transform, world_point)
    }

    fn contains_at_world_point_with_transform(
        &self,
        world_transform: WorldTransform,
        world_point: WorldPoint,
    ) -> bool {
        self.contains_at_layer_point(world_transform.world_to_layer(world_point))
    }

    fn render_rgba_chunk(
        &self,
        source_origin_x: i32,
        source_origin_y: i32,
        source_width: u32,
        source_height: u32,
        output_width: u16,
        output_height: u16,
    ) -> FieldRgbaChunk;
}

#[derive(Debug, Clone, Copy)]
pub struct LoadedFieldLayer<'a> {
    field: &'a DiscreteFieldRows,
    color_mode: FieldColorMode,
}

pub fn loaded_field_layer<'a>(
    layer: &'a LayerSpec,
    exact_lookups: &'a ExactLookupCache,
) -> Option<LoadedFieldLayer<'a>> {
    let url = layer.field_url()?;
    let color_mode = layer.field_color_mode()?;
    let field = exact_lookups.get(layer.id, &url)?;
    Some(LoadedFieldLayer { field, color_mode })
}

impl FieldLayerView for LoadedFieldLayer<'_> {
    fn width(&self) -> u16 {
        self.field.width()
    }

    fn height(&self) -> u16 {
        self.field.height()
    }

    fn field_id_at_map_px(&self, map_px_x: i32, map_px_y: i32) -> Option<u32> {
        self.field.cell_id_u32(map_px_x, map_px_y)
    }

    fn rgb_at_map_px(&self, map_px_x: i32, map_px_y: i32) -> Option<Rgb> {
        let id = self.field_id_at_map_px(map_px_x, map_px_y)?;
        let [r, g, b] = sample_rgb_bytes_for_field_id(id, self.color_mode);
        Some(Rgb::new(r, g, b))
    }

    fn render_rgba_chunk(
        &self,
        source_origin_x: i32,
        source_origin_y: i32,
        source_width: u32,
        source_height: u32,
        output_width: u16,
        output_height: u16,
    ) -> FieldRgbaChunk {
        self.field.render_rgba_resampled_chunk(
            source_origin_x,
            source_origin_y,
            source_width,
            source_height,
            output_width,
            output_height,
            |id| visual_rgba_for_field_id(id, self.color_mode),
        )
    }
}

fn layer_point_to_px(layer_point: LayerPoint) -> Option<(i32, i32)> {
    if !layer_point.x.is_finite() || !layer_point.y.is_finite() {
        return None;
    }
    Some((layer_point.x.floor() as i32, layer_point.y.floor() as i32))
}

fn sample_rgb_bytes_for_field_id(id: u32, color_mode: FieldColorMode) -> [u8; 3] {
    match color_mode {
        FieldColorMode::RgbU24 => [
            ((id >> 16) & 0xff) as u8,
            ((id >> 8) & 0xff) as u8,
            (id & 0xff) as u8,
        ],
        FieldColorMode::DebugHash => {
            let hash = hash_u32(id);
            [
                ((hash >> 16) & 0xff) as u8,
                ((hash >> 8) & 0xff) as u8,
                (hash & 0xff) as u8,
            ]
        }
    }
}

fn visual_rgba_for_field_id(id: u32, color_mode: FieldColorMode) -> [u8; 4] {
    match color_mode {
        FieldColorMode::RgbU24 => [
            ((id >> 16) & 0xff) as u8,
            ((id >> 8) & 0xff) as u8,
            (id & 0xff) as u8,
            255,
        ],
        FieldColorMode::DebugHash => {
            let hash = hash_u32(id);
            [
                ((hash >> 16) & 0xff) as u8,
                ((hash >> 8) & 0xff) as u8,
                (hash & 0xff) as u8,
                255,
            ]
            .map(|channel| channel.max(32))
        }
    }
}

fn hash_u32(mut value: u32) -> u32 {
    value ^= value >> 16;
    value = value.wrapping_mul(0x7feb_352d);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846c_a68b);
    value ^ (value >> 16)
}

#[cfg(test)]
mod tests {
    use super::{FieldLayerView, LoadedFieldLayer};
    use crate::map::layers::FieldColorMode;
    use crate::map::layers::{LayerId, LayerKind, LayerSpec, LodPolicy, PickMode};
    use crate::map::spaces::affine::Affine2D;
    use crate::map::spaces::layer_transform::LayerTransform;
    use crate::map::spaces::world::MapToWorld;
    use crate::map::spaces::WorldPoint;
    use fishystuff_api::Rgb;
    use fishystuff_core::field::DiscreteFieldRows;

    fn test_field_layer_spec(transform: LayerTransform) -> LayerSpec {
        LayerSpec {
            id: LayerId::from_raw(0),
            key: "test".to_string(),
            name: "Test".to_string(),
            visible_default: true,
            opacity_default: 1.0,
            z_base: 0.0,
            kind: LayerKind::TiledRaster,
            tileset_url: String::new(),
            tile_url_template: String::new(),
            tileset_version: "v1".to_string(),
            vector_source: None,
            transform,
            tile_px: 512,
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
            pick_mode: PickMode::None,
            display_order: 0,
        }
    }

    #[test]
    fn loaded_field_layer_contains_only_nonzero_ids() {
        let field = DiscreteFieldRows::from_u32_grid(2, 1, &[0, 7]).expect("field");
        let view = LoadedFieldLayer {
            field: &field,
            color_mode: FieldColorMode::RgbU24,
        };
        assert!(!view.contains_at_map_px(0, 0));
        assert!(view.contains_at_map_px(1, 0));
        assert_eq!(view.rgb_at_map_px(1, 0), Some(Rgb::from_u32(7)));
    }

    #[test]
    fn loaded_field_layer_samples_world_point_through_layer_transform() {
        let field = DiscreteFieldRows::from_u32_grid(2, 2, &[0, 7, 11, 13]).expect("field");
        let layer = test_field_layer_spec(LayerTransform::AffineToWorld(Affine2D::IDENTITY));
        let view = LoadedFieldLayer {
            field: &field,
            color_mode: FieldColorMode::RgbU24,
        };
        let world_point = WorldPoint::new(1.25, 0.25);
        assert_eq!(
            view.field_id_at_world_point(&layer, MapToWorld::default(), world_point),
            Some(7)
        );
        assert_eq!(
            view.rgb_at_world_point(&layer, MapToWorld::default(), world_point),
            Some(Rgb::from_u32(7))
        );
        assert!(view.contains_at_world_point(&layer, MapToWorld::default(), world_point));
    }
}
