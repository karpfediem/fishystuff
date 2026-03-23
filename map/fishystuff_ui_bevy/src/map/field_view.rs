use fishystuff_api::Rgb;
use fishystuff_core::field::{DiscreteFieldRows, FieldRgbaChunk};

use crate::map::exact_lookup::ExactLookupCache;
use crate::map::layers::{FieldColorMode, LayerSpec};

pub trait FieldLayerView {
    fn width(&self) -> u16;
    fn height(&self) -> u16;
    fn field_id_at_map_px(&self, map_px_x: i32, map_px_y: i32) -> Option<u32>;

    fn contains_at_map_px(&self, map_px_x: i32, map_px_y: i32) -> bool {
        self.field_id_at_map_px(map_px_x, map_px_y)
            .map(|id| id != 0)
            .unwrap_or(false)
    }

    fn rgb_at_map_px(&self, map_px_x: i32, map_px_y: i32) -> Option<Rgb>;

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
    use fishystuff_api::Rgb;
    use fishystuff_core::field::DiscreteFieldRows;

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
}
