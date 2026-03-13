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
