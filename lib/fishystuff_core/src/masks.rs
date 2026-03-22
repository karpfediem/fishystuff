use std::path::Path;

use anyhow::{bail, Context, Result};
use image::{ImageReader, RgbImage};

use crate::transform::{MapToWaterTransform, TransformKind};

pub fn pack_rgb_u32(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

pub fn unpack_rgb_u32(rgb: u32) -> (u8, u8, u8) {
    (
        ((rgb >> 16) & 0xff) as u8,
        ((rgb >> 8) & 0xff) as u8,
        (rgb & 0xff) as u8,
    )
}

pub fn format_rgb_u32(rgb: u32) -> String {
    let (r, g, b) = unpack_rgb_u32(rgb);
    format!("{},{},{}", r, g, b)
}

const ZONE_LOOKUP_MAGIC: &[u8; 8] = b"FSZLKP01";

#[derive(Debug, Clone)]
pub struct WaterMask {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl WaterMask {
    pub fn load_png(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let img = ImageReader::open(path)
            .with_context(|| format!("open watermask: {}", path.display()))?
            .with_guessed_format()
            .context("guess watermask format")?
            .decode()
            .context("decode watermask")?
            .into_rgb8();
        let width = img.width();
        let height = img.height();
        let data = img.into_raw();
        Ok(Self {
            width,
            height,
            data,
        })
    }

    pub fn from_rgb(width: u32, height: u32, data: Vec<u8>) -> Result<Self> {
        let expected = width
            .checked_mul(height)
            .and_then(|v| v.checked_mul(3))
            .ok_or_else(|| anyhow::anyhow!("watermask dimensions overflow"))?
            as usize;
        if data.len() != expected {
            bail!(
                "watermask data length mismatch: {} != {}",
                data.len(),
                expected
            );
        }
        Ok(Self {
            width,
            height,
            data,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    fn idx(&self, px: i32, py: i32) -> Option<usize> {
        if px < 0 || py < 0 {
            return None;
        }
        let px = px as u32;
        let py = py as u32;
        if px >= self.width || py >= self.height {
            return None;
        }
        let idx = (py * self.width + px) as usize * 3;
        Some(idx)
    }

    pub fn is_water(&self, px: i32, py: i32) -> bool {
        let Some(idx) = self.idx(px, py) else {
            return false;
        };
        self.data[idx] == 0 && self.data[idx + 1] == 0 && self.data[idx + 2] == 255
    }
}

pub trait WaterQuery {
    fn is_water_at_map_px(&self, map_x: i32, map_y: i32) -> bool;
}

impl WaterQuery for WaterMask {
    fn is_water_at_map_px(&self, map_x: i32, map_y: i32) -> bool {
        self.is_water(map_x, map_y)
    }
}

#[derive(Debug, Clone)]
pub struct WaterSampler {
    img: RgbImage,
    width: u32,
    height: u32,
    xform: TransformKind,
}

impl WaterSampler {
    pub fn from_png(path: impl AsRef<Path>, xform: TransformKind) -> Result<Self> {
        let path = path.as_ref();
        let img = ImageReader::open(path)
            .with_context(|| format!("open watermap: {}", path.display()))?
            .with_guessed_format()
            .context("guess watermap format")?
            .decode()
            .context("decode watermap")?
            .into_rgb8();
        let width = img.width();
        let height = img.height();
        Ok(Self {
            img,
            width,
            height,
            xform,
        })
    }

    pub fn from_image(img: RgbImage, xform: TransformKind) -> Self {
        let width = img.width();
        let height = img.height();
        Self {
            img,
            width,
            height,
            xform,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn is_water_at_map_px(&self, map_x: i32, map_y: i32) -> bool {
        if self.width == 0 || self.height == 0 {
            return false;
        }
        let (wx, wy) = self.xform.map_to_water(map_x as f64, map_y as f64);
        let xi = clamp_i32(wx.round() as i32, 0, self.width as i32 - 1) as u32;
        let yi = clamp_i32(wy.round() as i32, 0, self.height as i32 - 1) as u32;
        let pixel = self.img.get_pixel(xi, yi);
        pixel[0] == 0 && pixel[1] == 0 && pixel[2] == 255
    }

    pub fn sample_rgb_bilinear_at_map_px(&self, map_x: f64, map_y: f64) -> [u8; 3] {
        if self.width == 0 || self.height == 0 {
            return [0, 0, 0];
        }
        let (wx, wy) = self.xform.map_to_water(map_x, map_y);
        self.sample_rgb_bilinear_at_water(wx, wy)
    }

    fn sample_rgb_bilinear_at_water(&self, wx: f64, wy: f64) -> [u8; 3] {
        let max_x = (self.width - 1) as f64;
        let max_y = (self.height - 1) as f64;
        let x = wx.clamp(0.0, max_x);
        let y = wy.clamp(0.0, max_y);

        let x0 = x.floor() as u32;
        let y0 = y.floor() as u32;
        let x1 = (x0 + 1).min(self.width - 1);
        let y1 = (y0 + 1).min(self.height - 1);

        let tx = x - x0 as f64;
        let ty = y - y0 as f64;
        let w00 = (1.0 - tx) * (1.0 - ty);
        let w10 = tx * (1.0 - ty);
        let w01 = (1.0 - tx) * ty;
        let w11 = tx * ty;

        let p00 = self.img.get_pixel(x0, y0);
        let p10 = self.img.get_pixel(x1, y0);
        let p01 = self.img.get_pixel(x0, y1);
        let p11 = self.img.get_pixel(x1, y1);

        let mut out = [0u8; 3];
        for ch in 0..3 {
            let v = p00[ch] as f64 * w00
                + p10[ch] as f64 * w10
                + p01[ch] as f64 * w01
                + p11[ch] as f64 * w11;
            out[ch] = v.round().clamp(0.0, 255.0) as u8;
        }
        out
    }

    pub fn transform(&self) -> &TransformKind {
        &self.xform
    }
}

impl WaterQuery for WaterSampler {
    fn is_water_at_map_px(&self, map_x: i32, map_y: i32) -> bool {
        self.is_water_at_map_px(map_x, map_y)
    }
}

fn clamp_i32(v: i32, min: i32, max: i32) -> i32 {
    if v < min {
        min
    } else if v > max {
        max
    } else {
        v
    }
}

#[derive(Debug, Clone)]
pub struct ZoneMask {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl ZoneMask {
    pub fn load_png(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let img = ImageReader::open(path)
            .with_context(|| format!("open zonemask: {}", path.display()))?
            .with_guessed_format()
            .context("guess zonemask format")?
            .decode()
            .context("decode zonemask")?
            .into_rgb8();
        let width = img.width();
        let height = img.height();
        let data = img.into_raw();
        Ok(Self {
            width,
            height,
            data,
        })
    }

    pub fn from_rgb(width: u32, height: u32, data: Vec<u8>) -> Result<Self> {
        let expected = width
            .checked_mul(height)
            .and_then(|v| v.checked_mul(3))
            .ok_or_else(|| anyhow::anyhow!("zonemask dimensions overflow"))?
            as usize;
        if data.len() != expected {
            bail!(
                "zonemask data length mismatch: {} != {}",
                data.len(),
                expected
            );
        }
        Ok(Self {
            width,
            height,
            data,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    fn idx(&self, px: i32, py: i32) -> Option<usize> {
        if px < 0 || py < 0 {
            return None;
        }
        let px = px as u32;
        let py = py as u32;
        if px >= self.width || py >= self.height {
            return None;
        }
        let idx = (py * self.width + px) as usize * 3;
        Some(idx)
    }

    pub fn rgb_u32(&self, px: i32, py: i32) -> Option<u32> {
        let idx = self.idx(px, py)?;
        let r = self.data[idx] as u32;
        let g = self.data[idx + 1] as u32;
        let b = self.data[idx + 2] as u32;
        Some((r << 16) | (g << 8) | b)
    }

    pub fn sample_rgb_u32_clamped(&self, px: i32, py: i32) -> u32 {
        if self.width == 0 || self.height == 0 {
            return 0;
        }
        let max_x = self.width as i32 - 1;
        let max_y = self.height as i32 - 1;
        let cx = px.clamp(0, max_x) as u32;
        let cy = py.clamp(0, max_y) as u32;
        let idx = (cy * self.width + cx) as usize * 3;
        let r = self.data[idx];
        let g = self.data[idx + 1];
        let b = self.data[idx + 2];
        pack_rgb_u32(r, g, b)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneLookupRows {
    width: u16,
    height: u16,
    row_offsets: Vec<u32>,
    row_end_xs: Vec<u16>,
    row_rgbs: Vec<u32>,
}

impl ZoneLookupRows {
    pub fn from_zone_mask(mask: &ZoneMask) -> Result<Self> {
        let width = u16::try_from(mask.width)
            .map_err(|_| anyhow::anyhow!("zone lookup width {} exceeds u16", mask.width))?;
        let height = u16::try_from(mask.height)
            .map_err(|_| anyhow::anyhow!("zone lookup height {} exceeds u16", mask.height))?;
        if width == 0 || height == 0 {
            bail!("zone lookup dimensions must be non-zero");
        }
        let mut row_offsets = Vec::with_capacity(height as usize + 1);
        let mut row_end_xs = Vec::new();
        let mut row_rgbs = Vec::new();

        for y in 0..mask.height {
            row_offsets.push(row_end_xs.len() as u32);
            let mut current_rgb = mask.sample_rgb_u32_clamped(0, y as i32);
            for x in 1..mask.width {
                let rgb = mask.sample_rgb_u32_clamped(x as i32, y as i32);
                if rgb == current_rgb {
                    continue;
                }
                row_end_xs.push(u16::try_from(x).expect("x fits in u16"));
                row_rgbs.push(current_rgb);
                current_rgb = rgb;
            }
            row_end_xs.push(width);
            row_rgbs.push(current_rgb);
        }
        row_offsets.push(row_end_xs.len() as u32);

        Ok(Self {
            width,
            height,
            row_offsets,
            row_end_xs,
            row_rgbs,
        })
    }

    pub fn from_rgba(width: u32, height: u32, data: &[u8]) -> Result<Self> {
        let width = u16::try_from(width)
            .map_err(|_| anyhow::anyhow!("zone lookup width {} exceeds u16", width))?;
        let height = u16::try_from(height)
            .map_err(|_| anyhow::anyhow!("zone lookup height {} exceeds u16", height))?;
        if width == 0 || height == 0 {
            bail!("zone lookup dimensions must be non-zero");
        }
        let expected_len = usize::from(width)
            .checked_mul(usize::from(height))
            .and_then(|value| value.checked_mul(4))
            .ok_or_else(|| anyhow::anyhow!("zone lookup rgba length overflow"))?;
        if data.len() != expected_len {
            bail!(
                "zone lookup rgba length mismatch: {} != {}",
                data.len(),
                expected_len
            );
        }

        let mut row_offsets = Vec::with_capacity(height as usize + 1);
        let mut row_end_xs = Vec::new();
        let mut row_rgbs = Vec::new();
        let row_stride = usize::from(width) * 4;

        for row in data.chunks_exact(row_stride) {
            row_offsets.push(row_end_xs.len() as u32);
            let mut current_rgb = pack_rgb_u32(row[0], row[1], row[2]);
            for x in 1..usize::from(width) {
                let pixel_offset = x * 4;
                let rgb = pack_rgb_u32(
                    row[pixel_offset],
                    row[pixel_offset + 1],
                    row[pixel_offset + 2],
                );
                if rgb == current_rgb {
                    continue;
                }
                row_end_xs.push(u16::try_from(x).expect("x fits in u16"));
                row_rgbs.push(current_rgb);
                current_rgb = rgb;
            }
            row_end_xs.push(width);
            row_rgbs.push(current_rgb);
        }
        row_offsets.push(row_end_xs.len() as u32);

        Ok(Self {
            width,
            height,
            row_offsets,
            row_end_xs,
            row_rgbs,
        })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 20 {
            bail!("zone lookup payload too short: {}", bytes.len());
        }
        if &bytes[..8] != ZONE_LOOKUP_MAGIC {
            bail!("invalid zone lookup header");
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
                "zone lookup payload length mismatch: {} != {}",
                bytes.len(),
                expected
            );
        }
        if row_offset_count != height as usize + 1 {
            bail!(
                "zone lookup row offset count mismatch: {} != {}",
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

        let mut row_rgbs = Vec::with_capacity(segment_count);
        for _ in 0..segment_count {
            row_rgbs.push(u32::from_le_bytes([
                bytes[cursor],
                bytes[cursor + 1],
                bytes[cursor + 2],
                bytes[cursor + 3],
            ]));
            cursor += 4;
        }

        if *row_offsets.first().unwrap_or(&1) != 0 {
            bail!("zone lookup row offsets must start at 0");
        }
        if *row_offsets.last().unwrap_or(&0) as usize != segment_count {
            bail!("zone lookup row offsets must end at segment count");
        }
        if row_offsets.windows(2).any(|pair| pair[0] > pair[1]) {
            bail!("zone lookup row offsets must be monotonic");
        }
        for y in 0..height as usize {
            let start = row_offsets[y] as usize;
            let end = row_offsets[y + 1] as usize;
            if start == end {
                bail!("zone lookup row {} has no coverage", y);
            }
            let row = &row_end_xs[start..end];
            if row.windows(2).any(|pair| pair[0] >= pair[1]) {
                bail!("zone lookup row {} has non-increasing segment ends", y);
            }
            if row.last().copied() != Some(width) {
                bail!("zone lookup row {} must terminate at width {}", y, width);
            }
        }

        Ok(Self {
            width,
            height,
            row_offsets,
            row_end_xs,
            row_rgbs,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(
            20 + self.row_offsets.len() * 4 + self.row_end_xs.len() * 2 + self.row_rgbs.len() * 4,
        );
        out.extend_from_slice(ZONE_LOOKUP_MAGIC);
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
        for rgb in &self.row_rgbs {
            out.extend_from_slice(&rgb.to_le_bytes());
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

    pub fn for_each_span_matching(&self, target_rgb: u32, mut visit: impl FnMut(u16, u16, u16)) {
        for y in 0..self.height as usize {
            let start = self.row_offsets[y] as usize;
            let end = self.row_offsets[y + 1] as usize;
            let mut span_start = 0_u16;
            for idx in start..end {
                let span_end = self.row_end_xs[idx];
                if self.row_rgbs[idx] == target_rgb {
                    visit(y as u16, span_start, span_end);
                }
                span_start = span_end;
            }
        }
    }

    pub fn rgb_u32(&self, px: i32, py: i32) -> Option<u32> {
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
        self.row_rgbs.get(start + idx).copied()
    }

    pub fn sample_rgb_u32_clamped(&self, px: i32, py: i32) -> u32 {
        if self.width == 0 || self.height == 0 {
            return 0;
        }
        let x = px.clamp(0, self.width as i32 - 1);
        let y = py.clamp(0, self.height as i32 - 1);
        self.rgb_u32(x, y).unwrap_or(0)
    }
}
