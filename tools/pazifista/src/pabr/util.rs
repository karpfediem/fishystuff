use anyhow::{Context, Result};

pub(crate) fn sample_source_row(output_y: u32, output_height: u32, source_rows: usize) -> usize {
    let numerator = (u64::from(output_y) * 2 + 1) * source_rows as u64;
    let denominator = u64::from(output_height) * 2;
    ((numerator / denominator) as usize).min(source_rows.saturating_sub(1))
}

pub(crate) fn sample_local_x(output_x: usize, output_width: u32, native_width: u32) -> u32 {
    let numerator = (output_x as u64 * 2 + 1) * u64::from(native_width);
    let denominator = u64::from(output_width) * 2;
    ((numerator / denominator) as u32).min(native_width.saturating_sub(1))
}

pub(crate) fn color_bgr_for_region_id(region_id: u16) -> [u8; 3] {
    let [red, green, blue] = color_rgb_for_value(u32::from(region_id));
    [blue, green, red]
}

pub(crate) fn color_rgb_for_value(value: u32) -> [u8; 3] {
    let mut state = value;
    state ^= state >> 16;
    state = state.wrapping_mul(0x7FEB_352D);
    state ^= state >> 15;
    state = state.wrapping_mul(0x846C_A68B);
    state ^= state >> 16;

    let red = 64 + ((state >> 16) as u8 & 0x7F);
    let green = 64 + ((state >> 8) as u8 & 0x7F);
    let blue = 64 + (state as u8 & 0x7F);
    [red, green, blue]
}

pub(crate) fn mode_region_id(region_ids: &[u16]) -> u16 {
    let mut best_id = region_ids[0];
    let mut best_count = 1usize;

    for &candidate in region_ids {
        let mut count = 0usize;
        for &value in region_ids {
            if value == candidate {
                count += 1;
            }
        }

        if count > best_count || (count == best_count && candidate < best_id) {
            best_id = candidate;
            best_count = count;
        }
    }

    best_id
}

pub(crate) fn mode_value(values: &[u32]) -> u32 {
    let mut best_value = values[0];
    let mut best_count = 1usize;

    for &candidate in values {
        let mut count = 0usize;
        for &value in values {
            if value == candidate {
                count += 1;
            }
        }

        if count > best_count || (count == best_count && candidate < best_value) {
            best_value = candidate;
            best_count = count;
        }
    }

    best_value
}

pub(crate) fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let end = offset
        .checked_add(2)
        .context("u16 offset overflow while parsing PABR data")?;
    let slice = bytes
        .get(offset..end)
        .with_context(|| format!("u16 read at offset {} is out of bounds", offset))?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

pub(crate) fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let end = offset
        .checked_add(4)
        .context("u32 offset overflow while parsing PABR data")?;
    let slice = bytes
        .get(offset..end)
        .with_context(|| format!("u32 read at offset {} is out of bounds", offset))?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

#[cfg(test)]
mod tests {
    use super::{
        color_bgr_for_region_id, mode_region_id, mode_value, sample_local_x, sample_source_row,
    };

    #[test]
    fn sampling_uses_pixel_centers() {
        assert_eq!(sample_local_x(0, 4, 11560), 1445);
        assert_eq!(sample_local_x(3, 4, 11560), 10115);
        assert_eq!(sample_source_row(0, 4, 2), 0);
        assert_eq!(sample_source_row(3, 4, 2), 1);
    }

    #[test]
    fn false_color_is_deterministic_and_non_black() {
        let a = color_bgr_for_region_id(4);
        let b = color_bgr_for_region_id(4);
        let c = color_bgr_for_region_id(5);

        assert_eq!(a, b);
        assert_ne!(a, [0, 0, 0]);
        assert_ne!(a, c);
    }

    #[test]
    fn mode_prefers_majority_then_lowest_value() {
        assert_eq!(mode_region_id(&[9, 4, 9, 4, 4]), 4);
        assert_eq!(mode_region_id(&[9, 4]), 4);
        assert_eq!(mode_value(&[12, 8, 12, 8, 8]), 8);
        assert_eq!(mode_value(&[12, 8]), 8);
    }
}
