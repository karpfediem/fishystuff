use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Result};

const DISCRETE_FIELD_MAGIC: &[u8; 8] = b"FSZLKP01";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldRgbaChunk {
    width: u16,
    height: u16,
    data: Vec<u8>,
}

impl FieldRgbaChunk {
    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn into_data(self) -> Vec<u8> {
        self.data
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldRowSpan {
    pub start_x: u32,
    pub end_x: u32,
    pub id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscreteFieldRows {
    width: u16,
    height: u16,
    row_offsets: Vec<u32>,
    row_end_xs: Vec<u16>,
    row_ids: Vec<u32>,
}

pub type ZoneLookupRows = DiscreteFieldRows;

impl DiscreteFieldRows {
    pub fn from_u32_grid(width: u32, height: u32, data: &[u32]) -> Result<Self> {
        let width = u16::try_from(width)
            .map_err(|_| anyhow::anyhow!("field width {} exceeds u16", width))?;
        let height = u16::try_from(height)
            .map_err(|_| anyhow::anyhow!("field height {} exceeds u16", height))?;
        if width == 0 || height == 0 {
            bail!("field dimensions must be non-zero");
        }
        let expected_len = usize::from(width)
            .checked_mul(usize::from(height))
            .ok_or_else(|| anyhow::anyhow!("field data length overflow"))?;
        if data.len() != expected_len {
            bail!(
                "field data length mismatch: {} != {}",
                data.len(),
                expected_len
            );
        }

        let mut row_offsets = Vec::with_capacity(height as usize + 1);
        let mut row_end_xs = Vec::new();
        let mut row_ids = Vec::new();
        let row_stride = usize::from(width);

        for row in data.chunks_exact(row_stride) {
            row_offsets.push(row_end_xs.len() as u32);
            let mut current_id = row[0];
            for x in 1..row_stride {
                let id = row[x];
                if id == current_id {
                    continue;
                }
                row_end_xs.push(u16::try_from(x).expect("x fits in u16"));
                row_ids.push(current_id);
                current_id = id;
            }
            row_end_xs.push(width);
            row_ids.push(current_id);
        }
        row_offsets.push(row_end_xs.len() as u32);

        Ok(Self {
            width,
            height,
            row_offsets,
            row_end_xs,
            row_ids,
        })
    }

    pub fn from_rgba(width: u32, height: u32, data: &[u8]) -> Result<Self> {
        let width_u16 = u16::try_from(width)
            .map_err(|_| anyhow::anyhow!("field width {} exceeds u16", width))?;
        let height_u16 = u16::try_from(height)
            .map_err(|_| anyhow::anyhow!("field height {} exceeds u16", height))?;
        if width_u16 == 0 || height_u16 == 0 {
            bail!("field dimensions must be non-zero");
        }
        let expected_len = usize::from(width_u16)
            .checked_mul(usize::from(height_u16))
            .and_then(|value| value.checked_mul(4))
            .ok_or_else(|| anyhow::anyhow!("field rgba length overflow"))?;
        if data.len() != expected_len {
            bail!(
                "field rgba length mismatch: {} != {}",
                data.len(),
                expected_len
            );
        }

        let mut ids = Vec::with_capacity(usize::from(width_u16) * usize::from(height_u16));
        for pixel in data.chunks_exact(4) {
            ids.push(((pixel[0] as u32) << 16) | ((pixel[1] as u32) << 8) | pixel[2] as u32);
        }
        Self::from_u32_grid(u32::from(width_u16), u32::from(height_u16), &ids)
    }

    pub fn from_row_spans<I>(width: u32, height: u32, rows: I) -> Result<Self>
    where
        I: IntoIterator<Item = Vec<FieldRowSpan>>,
    {
        let width = u16::try_from(width)
            .map_err(|_| anyhow::anyhow!("field width {} exceeds u16", width))?;
        let height = u16::try_from(height)
            .map_err(|_| anyhow::anyhow!("field height {} exceeds u16", height))?;
        if width == 0 || height == 0 {
            bail!("field dimensions must be non-zero");
        }

        let mut row_offsets = Vec::with_capacity(height as usize + 1);
        let mut row_end_xs = Vec::new();
        let mut row_ids = Vec::new();
        let mut row_count = 0usize;

        for spans in rows {
            if row_count >= height as usize {
                bail!(
                    "field row count exceeds height: {} > {}",
                    row_count + 1,
                    height
                );
            }

            row_offsets.push(row_end_xs.len() as u32);
            let mut cursor_x = 0u32;
            let mut last_id: Option<u32> = None;

            for span in spans {
                if span.start_x > span.end_x {
                    bail!(
                        "field row {} has reversed span [{}..{})",
                        row_count,
                        span.start_x,
                        span.end_x
                    );
                }
                if span.end_x > u32::from(width) {
                    bail!(
                        "field row {} span end {} exceeds width {}",
                        row_count,
                        span.end_x,
                        width
                    );
                }
                if span.start_x < cursor_x {
                    bail!(
                        "field row {} spans overlap or are unsorted at x {}",
                        row_count,
                        span.start_x
                    );
                }
                if span.start_x > cursor_x {
                    push_row_segment(&mut row_end_xs, &mut row_ids, &mut last_id, span.start_x, 0);
                }
                cursor_x = span.end_x;
                if span.start_x < span.end_x {
                    push_row_segment(
                        &mut row_end_xs,
                        &mut row_ids,
                        &mut last_id,
                        span.end_x,
                        span.id,
                    );
                }
            }

            if cursor_x < u32::from(width) || row_end_xs.len() == row_offsets[row_count] as usize {
                push_row_segment(
                    &mut row_end_xs,
                    &mut row_ids,
                    &mut last_id,
                    u32::from(width),
                    0,
                );
            }

            row_count += 1;
        }

        if row_count != height as usize {
            bail!("field row count mismatch: {} != {}", row_count, height);
        }
        row_offsets.push(row_end_xs.len() as u32);
        validate_rows(width, height, &row_offsets, &row_end_xs)?;

        Ok(Self {
            width,
            height,
            row_offsets,
            row_end_xs,
            row_ids,
        })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 20 {
            bail!("field payload too short: {}", bytes.len());
        }
        if &bytes[..8] != DISCRETE_FIELD_MAGIC {
            bail!("invalid field header");
        }
        let width = u16::from_le_bytes([bytes[8], bytes[9]]);
        let height = u16::from_le_bytes([bytes[10], bytes[11]]);
        let row_offset_count =
            u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;
        let segment_count =
            u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]) as usize;
        let expected = 20 + row_offset_count * 4 + segment_count * 2 + segment_count * 4;
        if bytes.len() != expected {
            bail!(
                "field payload length mismatch: {} != {}",
                bytes.len(),
                expected
            );
        }
        if row_offset_count != height as usize + 1 {
            bail!(
                "field row offset count mismatch: {} != {}",
                row_offset_count,
                height as usize + 1
            );
        }

        let mut cursor = 20;
        let mut row_offsets = Vec::with_capacity(row_offset_count);
        for _ in 0..row_offset_count {
            row_offsets.push(u32::from_le_bytes([
                bytes[cursor],
                bytes[cursor + 1],
                bytes[cursor + 2],
                bytes[cursor + 3],
            ]));
            cursor += 4;
        }

        let mut row_end_xs = Vec::with_capacity(segment_count);
        for _ in 0..segment_count {
            row_end_xs.push(u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]));
            cursor += 2;
        }

        let mut row_ids = Vec::with_capacity(segment_count);
        for _ in 0..segment_count {
            row_ids.push(u32::from_le_bytes([
                bytes[cursor],
                bytes[cursor + 1],
                bytes[cursor + 2],
                bytes[cursor + 3],
            ]));
            cursor += 4;
        }

        validate_rows(width, height, &row_offsets, &row_end_xs)?;

        Ok(Self {
            width,
            height,
            row_offsets,
            row_end_xs,
            row_ids,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(
            20 + self.row_offsets.len() * 4 + self.row_end_xs.len() * 2 + self.row_ids.len() * 4,
        );
        out.extend_from_slice(DISCRETE_FIELD_MAGIC);
        out.extend_from_slice(&self.width.to_le_bytes());
        out.extend_from_slice(&self.height.to_le_bytes());
        out.extend_from_slice(&(self.row_offsets.len() as u32).to_le_bytes());
        out.extend_from_slice(&(self.row_end_xs.len() as u32).to_le_bytes());
        for offset in &self.row_offsets {
            out.extend_from_slice(&offset.to_le_bytes());
        }
        for end_x in &self.row_end_xs {
            out.extend_from_slice(&end_x.to_le_bytes());
        }
        for id in &self.row_ids {
            out.extend_from_slice(&id.to_le_bytes());
        }
        out
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn segment_count(&self) -> usize {
        self.row_end_xs.len()
    }

    pub fn unique_nonzero_ids(&self) -> BTreeSet<u32> {
        self.row_ids.iter().copied().filter(|id| *id != 0).collect()
    }

    pub fn for_each_span_matching(&self, target_id: u32, mut visit: impl FnMut(u16, u16, u16)) {
        for y in 0..self.height as usize {
            let start = self.row_offsets[y] as usize;
            let end = self.row_offsets[y + 1] as usize;
            let mut span_start = 0_u16;
            for idx in start..end {
                let span_end = self.row_end_xs[idx];
                if self.row_ids[idx] == target_id {
                    visit(y as u16, span_start, span_end);
                }
                span_start = span_end;
            }
        }
    }

    pub fn for_each_merged_rect_matching(
        &self,
        target_id: u32,
        mut visit: impl FnMut(u16, u16, u16, u16),
    ) {
        let mut active = BTreeMap::<(u16, u16), u16>::new();

        for y in 0..self.height {
            let mut current = Vec::<(u16, u16)>::new();
            self.for_each_row_span_matching(y, target_id, |start_x, end_x| {
                current.push((start_x, end_x));
            });

            let mut next_active = BTreeMap::<(u16, u16), u16>::new();
            for span in current {
                let start_y = active.remove(&span).unwrap_or(y);
                next_active.insert(span, start_y);
            }

            for ((start_x, end_x), start_y) in active {
                visit(start_y, y, start_x, end_x);
            }

            active = next_active;
        }

        for ((start_x, end_x), start_y) in active {
            visit(start_y, self.height, start_x, end_x);
        }
    }

    pub fn cell_id_u32(&self, px: i32, py: i32) -> Option<u32> {
        if px < 0 || py < 0 {
            return None;
        }
        let x = u16::try_from(px).ok()?;
        let y = u16::try_from(py).ok()?;
        if x >= self.width || y >= self.height {
            return None;
        }
        let start = self.row_offsets[y as usize] as usize;
        let end = self.row_offsets[y as usize + 1] as usize;
        let row = &self.row_end_xs[start..end];
        let idx = row.partition_point(|end_x| *end_x <= x);
        self.row_ids.get(start + idx).copied()
    }

    pub fn sample_cell_id_u32_clamped(&self, px: i32, py: i32) -> u32 {
        if self.width == 0 || self.height == 0 {
            return 0;
        }
        let x = px.clamp(0, self.width as i32 - 1);
        let y = py.clamp(0, self.height as i32 - 1);
        self.cell_id_u32(x, y).unwrap_or(0)
    }

    pub fn rgb_u32(&self, px: i32, py: i32) -> Option<u32> {
        self.cell_id_u32(px, py)
    }

    pub fn sample_rgb_u32_clamped(&self, px: i32, py: i32) -> u32 {
        self.sample_cell_id_u32_clamped(px, py)
    }

    pub fn render_rgba_chunk(
        &self,
        origin_x: i32,
        origin_y: i32,
        width: u16,
        height: u16,
        mut color_for_id: impl FnMut(u32) -> [u8; 4],
    ) -> FieldRgbaChunk {
        let mut data = vec![0u8; usize::from(width) * usize::from(height) * 4];
        if width == 0 || height == 0 {
            return FieldRgbaChunk {
                width,
                height,
                data,
            };
        }

        let chunk_max_x = origin_x + i32::from(width);
        for local_y in 0..usize::from(height) {
            let source_y = origin_y + local_y as i32;
            let Some(source_y_u16) = u16::try_from(source_y).ok() else {
                continue;
            };
            if source_y_u16 >= self.height {
                continue;
            }
            let row_start = self.row_offsets[source_y_u16 as usize] as usize;
            let row_end = self.row_offsets[source_y_u16 as usize + 1] as usize;
            let mut span_start_x = 0_i32;
            for idx in row_start..row_end {
                let span_end_x = i32::from(self.row_end_xs[idx]);
                let fill_start = span_start_x.max(origin_x);
                let fill_end = span_end_x.min(chunk_max_x);
                if fill_start < fill_end {
                    let rgba = color_for_id(self.row_ids[idx]);
                    let local_start_x = usize::try_from(fill_start - origin_x).unwrap_or(0);
                    let local_end_x = usize::try_from(fill_end - origin_x).unwrap_or(local_start_x);
                    let row_offset = local_y * usize::from(width) * 4;
                    for local_x in local_start_x..local_end_x {
                        let pixel_offset = row_offset + local_x * 4;
                        data[pixel_offset..pixel_offset + 4].copy_from_slice(&rgba);
                    }
                }
                span_start_x = span_end_x;
            }
        }

        FieldRgbaChunk {
            width,
            height,
            data,
        }
    }

    pub fn render_rgba_resampled_chunk(
        &self,
        source_origin_x: i32,
        source_origin_y: i32,
        source_width: u32,
        source_height: u32,
        output_width: u16,
        output_height: u16,
        mut color_for_id: impl FnMut(u32) -> [u8; 4],
    ) -> FieldRgbaChunk {
        let mut data = vec![0u8; usize::from(output_width) * usize::from(output_height) * 4];
        if output_width == 0 || output_height == 0 || source_width == 0 || source_height == 0 {
            return FieldRgbaChunk {
                width: output_width,
                height: output_height,
                data,
            };
        }

        for local_y in 0..usize::from(output_height) {
            let source_y = source_origin_y
                + ((local_y as u64 * source_height as u64) / output_height as u64) as i32;
            let row_offset = local_y * usize::from(output_width) * 4;
            for local_x in 0..usize::from(output_width) {
                let source_x = source_origin_x
                    + ((local_x as u64 * source_width as u64) / output_width as u64) as i32;
                let Some(id) = self.cell_id_u32(source_x, source_y) else {
                    continue;
                };
                let rgba = color_for_id(id);
                let pixel_offset = row_offset + local_x * 4;
                data[pixel_offset..pixel_offset + 4].copy_from_slice(&rgba);
            }
        }

        FieldRgbaChunk {
            width: output_width,
            height: output_height,
            data,
        }
    }

    fn for_each_row_span_matching(&self, y: u16, target_id: u32, mut visit: impl FnMut(u16, u16)) {
        let start = self.row_offsets[y as usize] as usize;
        let end = self.row_offsets[y as usize + 1] as usize;
        let mut span_start = 0_u16;
        for idx in start..end {
            let span_end = self.row_end_xs[idx];
            if self.row_ids[idx] == target_id {
                visit(span_start, span_end);
            }
            span_start = span_end;
        }
    }
}

fn push_row_segment(
    row_end_xs: &mut Vec<u16>,
    row_ids: &mut Vec<u32>,
    last_id: &mut Option<u32>,
    end_x: u32,
    id: u32,
) {
    let end_x = u16::try_from(end_x).expect("field span end fits in u16");
    match last_id {
        Some(previous_id) if *previous_id == id => {
            if let Some(previous_end) = row_end_xs.last_mut() {
                *previous_end = end_x;
            }
        }
        _ => {
            row_end_xs.push(end_x);
            row_ids.push(id);
            *last_id = Some(id);
        }
    }
}

fn validate_rows(width: u16, height: u16, row_offsets: &[u32], row_end_xs: &[u16]) -> Result<()> {
    let segment_count = row_end_xs.len();
    if *row_offsets.first().unwrap_or(&1) != 0 {
        bail!("field row offsets must start at 0");
    }
    if *row_offsets.last().unwrap_or(&0) as usize != segment_count {
        bail!("field row offsets must end at segment count");
    }
    if row_offsets.windows(2).any(|pair| pair[0] > pair[1]) {
        bail!("field row offsets must be monotonic");
    }
    for y in 0..height as usize {
        let start = row_offsets[y] as usize;
        let end = row_offsets[y + 1] as usize;
        if start == end {
            bail!("field row {} has no coverage", y);
        }
        let row = &row_end_xs[start..end];
        if row.windows(2).any(|pair| pair[0] >= pair[1]) {
            bail!("field row {} has non-increasing segment ends", y);
        }
        if row.last().copied() != Some(width) {
            bail!("field row {} must terminate at width {}", y, width);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{DiscreteFieldRows, FieldRowSpan};

    #[test]
    fn u32_grid_roundtrips_and_samples() {
        let field =
            DiscreteFieldRows::from_u32_grid(4, 2, &[1, 1, 2, 2, 3, 3, 2, 2]).expect("field");
        assert_eq!(field.segment_count(), 4);
        assert_eq!(field.cell_id_u32(0, 0), Some(1));
        assert_eq!(field.cell_id_u32(3, 0), Some(2));
        assert_eq!(field.cell_id_u32(1, 1), Some(3));

        let bytes = field.to_bytes();
        let decoded = DiscreteFieldRows::from_bytes(&bytes).expect("decode");
        assert_eq!(decoded, field);
    }

    #[test]
    fn row_spans_fill_background_gaps_and_merge_adjacent_ids() {
        let field = DiscreteFieldRows::from_row_spans(
            5,
            2,
            vec![
                vec![
                    FieldRowSpan {
                        start_x: 1,
                        end_x: 3,
                        id: 7,
                    },
                    FieldRowSpan {
                        start_x: 3,
                        end_x: 5,
                        id: 7,
                    },
                ],
                vec![FieldRowSpan {
                    start_x: 0,
                    end_x: 2,
                    id: 9,
                }],
            ],
        )
        .expect("field");

        assert_eq!(field.segment_count(), 4);
        assert_eq!(field.cell_id_u32(0, 0), Some(0));
        assert_eq!(field.cell_id_u32(1, 0), Some(7));
        assert_eq!(field.cell_id_u32(4, 0), Some(7));
        assert_eq!(field.cell_id_u32(0, 1), Some(9));
        assert_eq!(field.cell_id_u32(4, 1), Some(0));
    }

    #[test]
    fn render_rgba_chunk_rasterizes_spans_and_preserves_transparent_oob() {
        let field =
            DiscreteFieldRows::from_u32_grid(4, 2, &[1, 1, 2, 2, 3, 3, 2, 2]).expect("field");
        let chunk = field.render_rgba_chunk(-1, 0, 4, 3, |id| match id {
            1 => [10, 20, 30, 255],
            2 => [40, 50, 60, 255],
            3 => [70, 80, 90, 255],
            _ => [0, 0, 0, 0],
        });

        assert_eq!(chunk.width(), 4);
        assert_eq!(chunk.height(), 3);
        let data = chunk.data();

        assert_eq!(&data[0..4], &[0, 0, 0, 0]);
        assert_eq!(&data[4..8], &[10, 20, 30, 255]);
        assert_eq!(&data[8..12], &[10, 20, 30, 255]);
        assert_eq!(&data[12..16], &[40, 50, 60, 255]);

        let row_1 = 4 * 4;
        assert_eq!(&data[row_1..row_1 + 4], &[0, 0, 0, 0]);
        assert_eq!(&data[row_1 + 4..row_1 + 8], &[70, 80, 90, 255]);
        assert_eq!(&data[row_1 + 8..row_1 + 12], &[70, 80, 90, 255]);
        assert_eq!(&data[row_1 + 12..row_1 + 16], &[40, 50, 60, 255]);

        let row_2 = 2 * 4 * 4;
        assert_eq!(&data[row_2..row_2 + 16], &[0; 16]);
    }

    #[test]
    fn render_rgba_resampled_chunk_downsamples_source_field() {
        let field = DiscreteFieldRows::from_u32_grid(
            4,
            4,
            &[
                1, 1, 2, 2, //
                1, 1, 2, 2, //
                3, 3, 4, 4, //
                3, 3, 4, 4,
            ],
        )
        .expect("field");
        let chunk = field.render_rgba_resampled_chunk(0, 0, 4, 4, 2, 2, |id| match id {
            1 => [10, 0, 0, 255],
            2 => [20, 0, 0, 255],
            3 => [30, 0, 0, 255],
            4 => [40, 0, 0, 255],
            _ => [0, 0, 0, 0],
        });

        assert_eq!(chunk.width(), 2);
        assert_eq!(chunk.height(), 2);
        assert_eq!(
            chunk.data(),
            &[
                10, 0, 0, 255, 20, 0, 0, 255, //
                30, 0, 0, 255, 40, 0, 0, 255,
            ]
        );
    }

    #[test]
    fn merged_rects_coalesce_identical_spans_across_rows() {
        let field = DiscreteFieldRows::from_u32_grid(
            4,
            4,
            &[
                0, 7, 7, 0, //
                0, 7, 7, 0, //
                0, 7, 0, 0, //
                0, 7, 0, 0,
            ],
        )
        .expect("field");

        let mut rects = Vec::new();
        field.for_each_merged_rect_matching(7, |start_y, end_y, start_x, end_x| {
            rects.push((start_y, end_y, start_x, end_x));
        });

        assert_eq!(rects, vec![(0, 2, 1, 3), (2, 4, 1, 2)]);
    }
}
