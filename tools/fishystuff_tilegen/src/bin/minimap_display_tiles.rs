use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use clap::Parser;
use fishystuff_core::constants::{
    DEFAULT_PIXEL_CENTER_OFFSET, LEFT, MAP_HEIGHT, MAP_WIDTH, SECTOR_PER_PIXEL, SECTOR_SCALE, TOP,
};
use image::imageops::{overlay, resize, FilterType};
use image::ImageReader;
use image::{Rgba, RgbaImage};
use serde::Serialize;

#[derive(Parser, Debug)]
#[command(name = "fishystuff_minimap_display_tiles")]
#[command(
    about = "Build a map-space minimap display pyramid that keeps full source detail at high zoom"
)]
struct Args {
    #[arg(long)]
    input_dir: PathBuf,
    #[arg(long)]
    out_dir: PathBuf,
    /// Logical map-space tile span for the finest level.
    #[arg(long, default_value_t = 1280)]
    tile_px: u32,
    /// Output texture size for every pyramid level. Defaults to source-equivalent resolution.
    #[arg(long)]
    output_px: Option<u32>,
    #[arg(long, default_value_t = 4)]
    max_level: u32,
    #[arg(long, default_value_t = 128)]
    source_tile_px: u32,
    #[arg(long, default_value_t = MAP_WIDTH as u32)]
    map_width: u32,
    #[arg(long, default_value_t = MAP_HEIGHT as u32)]
    map_height: u32,
    #[arg(long, default_value_t = 100.0)]
    layer_to_world_a: f64,
    #[arg(long, default_value_t = 0.0)]
    layer_to_world_b: f64,
    #[arg(long, default_value_t = 0.0)]
    layer_to_world_tx: f64,
    #[arg(long, default_value_t = 0.0)]
    layer_to_world_c: f64,
    #[arg(long, default_value_t = 100.0)]
    layer_to_world_d: f64,
    #[arg(long, default_value_t = 0.0)]
    layer_to_world_ty: f64,
    #[arg(long, default_value_t = true)]
    source_y_flip: bool,
    #[arg(long, default_value = "/images/tiles/minimap_visual/v1")]
    root_url: String,
}

type ParentChildren = Vec<(u32, u32, u32, u32)>;

#[derive(Debug, Clone, Copy)]
struct Affine2D {
    a: f64,
    b: f64,
    tx: f64,
    c: f64,
    d: f64,
    ty: f64,
}

impl Affine2D {
    fn apply(self, x: f64, y: f64) -> (f64, f64) {
        (
            self.a * x + self.b * y + self.tx,
            self.c * x + self.d * y + self.ty,
        )
    }

    fn inverse(self) -> Result<Self> {
        let det = self.a * self.d - self.b * self.c;
        if det.abs() <= f64::EPSILON {
            bail!("non-invertible affine transform");
        }
        let inv_det = 1.0 / det;
        let a = self.d * inv_det;
        let b = -self.b * inv_det;
        let c = -self.c * inv_det;
        let d = self.a * inv_det;
        let tx = -(a * self.tx + b * self.ty);
        let ty = -(c * self.tx + d * self.ty);
        Ok(Self { a, b, tx, c, d, ty })
    }

    fn compose(lhs: Self, rhs: Self) -> Self {
        Self {
            a: lhs.a * rhs.a + lhs.b * rhs.c,
            b: lhs.a * rhs.b + lhs.b * rhs.d,
            tx: lhs.a * rhs.tx + lhs.b * rhs.ty + lhs.tx,
            c: lhs.c * rhs.a + lhs.d * rhs.c,
            d: lhs.c * rhs.b + lhs.d * rhs.d,
            ty: lhs.c * rhs.tx + lhs.d * rhs.ty + lhs.ty,
        }
    }
}

#[derive(Serialize)]
struct TilesetManifest {
    version: u32,
    map_size_px: [u32; 2],
    tile_size_px: u32,
    root: String,
    levels: Vec<LevelManifest>,
}

#[derive(Serialize)]
struct LevelManifest {
    z: u32,
    min_x: i32,
    min_y: i32,
    width: u32,
    height: u32,
    tile_count: usize,
    path: String,
    occupancy_b64: String,
}

#[derive(Debug, Clone, Copy)]
struct PyramidLevel {
    z: u32,
    width: u32,
    height: u32,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.tile_px == 0 || args.source_tile_px == 0 {
        bail!("tile sizes must be > 0");
    }
    if !args.input_dir.is_dir() {
        bail!("input directory not found: {}", args.input_dir.display());
    }

    let layer_to_world = Affine2D {
        a: args.layer_to_world_a,
        b: args.layer_to_world_b,
        tx: args.layer_to_world_tx,
        c: args.layer_to_world_c,
        d: args.layer_to_world_d,
        ty: args.layer_to_world_ty,
    };
    let world_to_layer = layer_to_world.inverse()?;
    let map_to_world = Affine2D {
        a: SECTOR_PER_PIXEL * SECTOR_SCALE,
        b: 0.0,
        tx: LEFT * SECTOR_SCALE,
        c: 0.0,
        d: -(SECTOR_PER_PIXEL * SECTOR_SCALE),
        ty: (TOP - DEFAULT_PIXEL_CENTER_OFFSET * SECTOR_PER_PIXEL) * SECTOR_SCALE,
    };
    let map_to_layer = Affine2D::compose(world_to_layer, map_to_world);
    let output_px = args
        .output_px
        .unwrap_or_else(|| default_output_px(map_to_layer, args.tile_px));
    let source_tile_px =
        i32::try_from(args.source_tile_px).context("source tile_px out of range")?;
    let levels = build_levels(
        args.map_width,
        args.map_height,
        args.tile_px,
        args.max_level,
    );

    for level in &levels {
        fs::create_dir_all(args.out_dir.join(level.z.to_string()))
            .with_context(|| format!("create level dir {}", level.z))?;
    }

    build_level0(&args, &levels[0], output_px, source_tile_px, map_to_layer)?;
    for level in levels.iter().skip(1) {
        build_parent_level(&args.out_dir, level.z, output_px)?;
    }

    let manifest_levels = levels
        .iter()
        .map(|level| LevelManifest {
            z: level.z,
            min_x: 0,
            min_y: 0,
            width: level.width,
            height: level.height,
            tile_count: (level.width * level.height) as usize,
            path: "{z}/{x}_{y}.png".to_string(),
            occupancy_b64: full_occupancy(level.width, level.height),
        })
        .collect();
    let manifest = TilesetManifest {
        version: 1,
        map_size_px: [args.map_width, args.map_height],
        tile_size_px: args.tile_px,
        root: args.root_url,
        levels: manifest_levels,
    };
    let manifest_path = args.out_dir.join("tileset.json");
    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)
        .with_context(|| format!("write {}", manifest_path.display()))?;
    Ok(())
}

fn build_levels(
    map_width: u32,
    map_height: u32,
    tile_px: u32,
    max_level: u32,
) -> Vec<PyramidLevel> {
    let mut levels = Vec::with_capacity(max_level as usize + 1);
    let mut width = map_width.div_ceil(tile_px);
    let mut height = map_height.div_ceil(tile_px);
    for z in 0..=max_level {
        levels.push(PyramidLevel { z, width, height });
        if width == 1 && height == 1 {
            break;
        }
        width = width.div_ceil(2);
        height = height.div_ceil(2);
    }
    levels
}

fn default_output_px(map_to_layer: Affine2D, tile_px: u32) -> u32 {
    let scale_x = map_to_layer.a.hypot(map_to_layer.c);
    let scale_y = map_to_layer.b.hypot(map_to_layer.d);
    let source_px_per_map_px = scale_x.max(scale_y).max(1.0);
    (f64::from(tile_px) * source_px_per_map_px).round().max(1.0) as u32
}

fn build_level0(
    args: &Args,
    level: &PyramidLevel,
    output_px: u32,
    source_tile_px: i32,
    map_to_layer: Affine2D,
) -> Result<()> {
    let level0_dir = args.out_dir.join("0");
    for ty in 0..level.height {
        for tx in 0..level.width {
            let mut out = RgbaImage::from_pixel(output_px, output_px, Rgba([0, 0, 0, 0]));
            let map_x0 = tx * args.tile_px;
            let map_y0 = ty * args.tile_px;
            let map_x1 = (map_x0 + args.tile_px).min(args.map_width);
            let map_y1 = (map_y0 + args.tile_px).min(args.map_height);
            let map_span_x = f64::from(map_x1 - map_x0);
            let map_span_y = f64::from(map_y1 - map_y0);

            let source_bounds =
                source_tile_bounds(map_to_layer, map_x0, map_y0, map_x1, map_y1, source_tile_px);
            let source_tiles =
                load_source_tiles(&args.input_dir, source_bounds, args.source_tile_px)?;

            let sample_step_x = map_span_x / f64::from(output_px);
            let sample_step_y = map_span_y / f64::from(output_px);
            for oy in 0..output_px {
                let map_y = f64::from(map_y0) + (f64::from(oy) + 0.5) * sample_step_y;
                let mut map_x = f64::from(map_x0) + 0.5 * sample_step_x;
                for ox in 0..output_px {
                    if let Some((sx, sy)) = apply_if_in_bounds(
                        map_x,
                        map_y,
                        f64::from(map_x0),
                        f64::from(map_x1),
                        f64::from(map_y0),
                        f64::from(map_y1),
                        map_to_layer,
                    ) {
                        if let Some(pixel) = sample_source_nearest(
                            &source_tiles,
                            sx,
                            sy,
                            source_tile_px,
                            args.source_y_flip,
                        ) {
                            out.put_pixel(ox, oy, pixel);
                        }
                    }
                    map_x += sample_step_x;
                }
            }

            let out_path = level0_dir.join(format!("{}_{}.png", tx, ty));
            out.save(&out_path)
                .with_context(|| format!("write {}", out_path.display()))?;
        }
    }
    Ok(())
}

fn build_parent_level(out_dir: &Path, z: u32, output_px: u32) -> Result<()> {
    let prev_dir = out_dir.join((z - 1).to_string());
    let next_dir = out_dir.join(z.to_string());
    let prev_files = collect_level_tiles(&prev_dir)?;
    let mut parents: HashMap<(u32, u32), ParentChildren> = HashMap::new();
    for &(x, y) in &prev_files {
        let parent = (x / 2, y / 2);
        let quadrant = (x % 2, y % 2);
        parents
            .entry(parent)
            .or_default()
            .push((x, y, quadrant.0, quadrant.1));
    }

    for ((px, py), children) in parents {
        let mut canvas = RgbaImage::from_pixel(output_px * 2, output_px * 2, Rgba([0, 0, 0, 0]));
        for (cx, cy, qx, qy) in children {
            let child_path = prev_dir.join(format!("{}_{}.png", cx, cy));
            let child = ImageReader::open(&child_path)
                .with_context(|| format!("open {}", child_path.display()))?
                .with_guessed_format()
                .with_context(|| format!("guess format for {}", child_path.display()))?
                .decode()
                .with_context(|| format!("decode {}", child_path.display()))?
                .to_rgba8();
            overlay(
                &mut canvas,
                &child,
                i64::from(qx * output_px),
                i64::from(qy * output_px),
            );
        }
        let down = resize(&canvas, output_px, output_px, FilterType::Triangle);
        let out_path = next_dir.join(format!("{}_{}.png", px, py));
        down.save(&out_path)
            .with_context(|| format!("write {}", out_path.display()))?;
    }
    Ok(())
}

fn collect_level_tiles(dir: &Path) -> Result<Vec<(u32, u32)>> {
    let mut tiles = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if let Some(coords) = parse_level_tile_name(&name) {
            tiles.push(coords);
        }
    }
    Ok(tiles)
}

fn parse_level_tile_name(name: &str) -> Option<(u32, u32)> {
    let stem = name.strip_suffix(".png")?;
    let mut parts = stem.split('_');
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((x, y))
}

fn full_occupancy(width: u32, height: u32) -> String {
    let occupancy = vec![0xff_u8; (width as usize * height as usize).div_ceil(8)];
    BASE64_STANDARD.encode(occupancy)
}

fn apply_if_in_bounds(
    map_x: f64,
    map_y: f64,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    map_to_layer: Affine2D,
) -> Option<(f64, f64)> {
    if map_x < min_x || map_x >= max_x || map_y < min_y || map_y >= max_y {
        return None;
    }
    Some(map_to_layer.apply(map_x, map_y))
}

fn source_tile_bounds(
    map_to_layer: Affine2D,
    map_x0: u32,
    map_y0: u32,
    map_x1: u32,
    map_y1: u32,
    source_tile_px: i32,
) -> (i32, i32, i32, i32) {
    let corners = [
        map_to_layer.apply(map_x0 as f64, map_y0 as f64),
        map_to_layer.apply(map_x1 as f64, map_y0 as f64),
        map_to_layer.apply(map_x0 as f64, map_y1 as f64),
        map_to_layer.apply(map_x1 as f64, map_y1 as f64),
    ];
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for (x, y) in corners {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    (
        (min_x.floor() as i32).div_euclid(source_tile_px) - 1,
        (max_x.ceil() as i32).div_euclid(source_tile_px) + 1,
        (min_y.floor() as i32).div_euclid(source_tile_px) - 1,
        (max_y.ceil() as i32).div_euclid(source_tile_px) + 1,
    )
}

fn load_source_tiles(
    input_dir: &Path,
    bounds: (i32, i32, i32, i32),
    source_tile_px: u32,
) -> Result<HashMap<(i32, i32), RgbaImage>> {
    let (min_tx, max_tx, min_ty, max_ty) = bounds;
    let mut tiles = HashMap::new();
    for ty in min_ty..=max_ty {
        for tx in min_tx..=max_tx {
            let path = input_dir.join(format!("rader_{}_{}.png", tx, ty));
            if !path.is_file() {
                continue;
            }
            let image = ImageReader::open(&path)
                .with_context(|| format!("open {}", path.display()))?
                .with_guessed_format()
                .with_context(|| format!("guess format for {}", path.display()))?
                .decode()
                .with_context(|| format!("decode {}", path.display()))?
                .to_rgba8();
            if image.width() != source_tile_px || image.height() != source_tile_px {
                bail!(
                    "source tile {} has unexpected size {}x{} (expected {}x{})",
                    path.display(),
                    image.width(),
                    image.height(),
                    source_tile_px,
                    source_tile_px
                );
            }
            tiles.insert((tx, ty), image);
        }
    }
    Ok(tiles)
}

fn sample_source_nearest(
    tiles: &HashMap<(i32, i32), RgbaImage>,
    sx: f64,
    sy: f64,
    source_tile_px: i32,
    source_y_flip: bool,
) -> Option<Rgba<u8>> {
    let px = sx.round() as i32;
    let py = sy.round() as i32;
    let tx = px.div_euclid(source_tile_px);
    let ty = py.div_euclid(source_tile_px);
    let local_x = px.rem_euclid(source_tile_px) as u32;
    let layer_local_y = py.rem_euclid(source_tile_px) as u32;
    let local_y = if source_y_flip {
        (source_tile_px as u32 - 1).saturating_sub(layer_local_y)
    } else {
        layer_local_y
    };
    tiles
        .get(&(tx, ty))
        .map(|tile| *tile.get_pixel(local_x, local_y))
}

#[cfg(test)]
mod tests {
    use super::{default_output_px, Affine2D};

    #[test]
    fn default_output_px_preserves_minimap_source_density() {
        let map_to_layer = Affine2D {
            a: 3.011_764_764_785_766_4,
            b: 0.0,
            tx: 0.0,
            c: 0.0,
            d: -3.011_764_764_785_766_4,
            ty: 0.0,
        };
        assert_eq!(default_output_px(map_to_layer, 1024), 3084);
        assert_eq!(default_output_px(map_to_layer, 1280), 3855);
        assert_eq!(default_output_px(map_to_layer, 2048), 6168);
    }
}
