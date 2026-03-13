use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use clap::Parser;
use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(name = "fishystuff_region_groups_raster")]
#[command(about = "Rasterize region-group GeoJSON directly into level-0 tiles + tileset manifest")]
struct Args {
    /// GeoJSON feature collection with polygon/multipolygon region groups.
    #[arg(long)]
    geojson: PathBuf,
    /// Output directory for level-0 tiles: <...>/<level=0>/<x>_<y>.png.
    #[arg(long)]
    out_dir: PathBuf,
    /// Output path for tileset.json.
    #[arg(long)]
    tileset_out: PathBuf,
    /// Canonical map width in pixels.
    #[arg(long, default_value_t = 11_560)]
    map_width: u32,
    /// Canonical map height in pixels.
    #[arg(long, default_value_t = 10_540)]
    map_height: u32,
    /// Tile size in pixels.
    #[arg(long, default_value_t = 512)]
    tile_size: u32,
    /// Output alpha channel for filled polygons.
    #[arg(long, default_value_t = 255)]
    alpha: u8,
}

#[derive(Debug, Deserialize)]
struct GeoCollection {
    #[serde(default)]
    features: Vec<GeoFeature>,
}

#[derive(Debug, Deserialize)]
struct GeoFeature {
    #[serde(default)]
    properties: GeoProperties,
    geometry: GeoGeometry,
}

#[derive(Debug, Default, Deserialize)]
struct GeoProperties {
    #[serde(default)]
    c: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "coordinates")]
enum GeoGeometry {
    Polygon(Vec<Vec<[f64; 2]>>),
    MultiPolygon(Vec<Vec<Vec<[f64; 2]>>>),
}

#[derive(Debug, Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, Copy)]
struct BBox {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl BBox {
    fn intersects_tile(self, x0: i32, y0: i32, x1: i32, y1: i32) -> bool {
        let tx0 = x0 as f64;
        let ty0 = y0 as f64;
        let tx1 = x1 as f64;
        let ty1 = y1 as f64;
        !(self.max_x < tx0 || self.min_x > tx1 || self.max_y < ty0 || self.min_y > ty1)
    }
}

#[derive(Debug, Clone)]
struct PolygonData {
    outer: Vec<Point>,
    holes: Vec<Vec<Point>>,
    bbox: BBox,
}

#[derive(Debug, Clone)]
struct FeatureData {
    polygons: Vec<PolygonData>,
    bbox: BBox,
    color: [u8; 4],
}

#[derive(Debug, Serialize)]
struct TilesetManifest {
    tile_size_px: u32,
    levels: Vec<LevelManifest>,
}

#[derive(Debug, Serialize)]
struct LevelManifest {
    z: u32,
    min_x: i32,
    min_y: i32,
    width: u32,
    height: u32,
    tile_count: usize,
    occupancy_b64: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.tile_size == 0 {
        bail!("--tile-size must be > 0");
    }
    if args.map_width == 0 || args.map_height == 0 {
        bail!("--map-width and --map-height must be > 0");
    }

    let bytes = fs::read(&args.geojson)
        .with_context(|| format!("read geojson {}", args.geojson.display()))?;
    let collection: GeoCollection =
        serde_json::from_slice(&bytes).context("parse region-group geojson")?;
    let features = compile_features(&collection, args.alpha);
    if features.is_empty() {
        bail!("geojson did not contain rasterizable features");
    }

    fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("create tile output dir {}", args.out_dir.display()))?;
    if let Some(parent) = args.tileset_out.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create tileset dir {}", parent.display()))?;
    }

    let mut filled_tiles = HashSet::new();
    let tiles_x = args.map_width.div_ceil(args.tile_size);
    let tiles_y = args.map_height.div_ceil(args.tile_size);
    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let x0 = (tx * args.tile_size) as i32;
            let y0 = (ty * args.tile_size) as i32;
            let tile_w = args.tile_size.min(args.map_width - tx * args.tile_size);
            let tile_h = args.tile_size.min(args.map_height - ty * args.tile_size);
            let x1 = x0 + tile_w as i32 - 1;
            let y1 = y0 + tile_h as i32 - 1;

            let mut tile = RgbaImage::from_pixel(tile_w, tile_h, Rgba([0, 0, 0, 0]));
            rasterize_tile(&mut tile, x0, y0, x1, y1, &features);

            if tile.pixels().all(|px| px[3] == 0) {
                continue;
            }
            let out_path = args.out_dir.join(format!("{}_{}.png", tx, ty));
            tile.save(&out_path)
                .with_context(|| format!("write tile {}", out_path.display()))?;
            filled_tiles.insert((tx as i32, ty as i32));
        }
    }

    if filled_tiles.is_empty() {
        bail!("rasterization produced no non-empty tiles");
    }

    let (min_x, min_y, width, height, bitset) = build_occupancy(filled_tiles.iter().copied())?;
    let manifest = TilesetManifest {
        tile_size_px: args.tile_size,
        levels: vec![LevelManifest {
            z: 0,
            min_x,
            min_y,
            width,
            height,
            tile_count: filled_tiles.len(),
            occupancy_b64: BASE64_STANDARD.encode(bitset),
        }],
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).context("serialize tileset json")?;
    fs::write(&args.tileset_out, manifest_bytes)
        .with_context(|| format!("write {}", args.tileset_out.display()))?;

    println!(
        "region-groups rasterized: features={} tiles={}/{} output={}",
        features.len(),
        filled_tiles.len(),
        (tiles_x * tiles_y),
        args.out_dir.display()
    );
    Ok(())
}

fn compile_features(collection: &GeoCollection, alpha: u8) -> Vec<FeatureData> {
    let mut out = Vec::new();
    for feature in &collection.features {
        let color = feature_color(&feature.properties, alpha);
        let mut polygons = Vec::new();
        match &feature.geometry {
            GeoGeometry::Polygon(rings) => {
                if let Some(poly) = compile_polygon(rings) {
                    polygons.push(poly);
                }
            }
            GeoGeometry::MultiPolygon(polys) => {
                for rings in polys {
                    if let Some(poly) = compile_polygon(rings) {
                        polygons.push(poly);
                    }
                }
            }
        }
        if polygons.is_empty() {
            continue;
        }
        let bbox = merge_bboxes(polygons.iter().map(|poly| poly.bbox));
        out.push(FeatureData {
            polygons,
            bbox,
            color,
        });
    }
    out
}

fn compile_polygon(rings: &[Vec<[f64; 2]>]) -> Option<PolygonData> {
    if rings.is_empty() {
        return None;
    }
    let outer = compile_ring(&rings[0])?;
    let mut holes = Vec::new();
    for ring in &rings[1..] {
        if let Some(hole) = compile_ring(ring) {
            holes.push(hole);
        }
    }
    let bbox = ring_bbox(&outer)?;
    Some(PolygonData { outer, holes, bbox })
}

fn compile_ring(points: &[[f64; 2]]) -> Option<Vec<Point>> {
    let mut ring: Vec<Point> = points.iter().map(|p| Point { x: p[0], y: p[1] }).collect();
    if ring.len() < 3 {
        return None;
    }
    if let (Some(first), Some(last)) = (ring.first().copied(), ring.last().copied()) {
        if (first.x - last.x).abs() < f64::EPSILON && (first.y - last.y).abs() < f64::EPSILON {
            ring.pop();
        }
    }
    if ring.len() < 3 {
        return None;
    }
    Some(ring)
}

fn feature_color(props: &GeoProperties, alpha: u8) -> [u8; 4] {
    let r = props.c.first().copied().unwrap_or(255);
    let g = props.c.get(1).copied().unwrap_or(255);
    let b = props.c.get(2).copied().unwrap_or(255);
    [r, g, b, alpha]
}

fn merge_bboxes<I>(mut boxes: I) -> BBox
where
    I: Iterator<Item = BBox>,
{
    let first = boxes.next().unwrap_or(BBox {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 0.0,
        max_y: 0.0,
    });
    boxes.fold(first, |acc, bbox| BBox {
        min_x: acc.min_x.min(bbox.min_x),
        min_y: acc.min_y.min(bbox.min_y),
        max_x: acc.max_x.max(bbox.max_x),
        max_y: acc.max_y.max(bbox.max_y),
    })
}

fn ring_bbox(points: &[Point]) -> Option<BBox> {
    let first = *points.first()?;
    let mut min_x = first.x;
    let mut min_y = first.y;
    let mut max_x = first.x;
    let mut max_y = first.y;
    for p in points.iter().copied().skip(1) {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }
    Some(BBox {
        min_x,
        min_y,
        max_x,
        max_y,
    })
}

fn rasterize_tile(
    tile: &mut RgbaImage,
    tile_x0: i32,
    tile_y0: i32,
    tile_x1: i32,
    tile_y1: i32,
    features: &[FeatureData],
) {
    for feature in features {
        if !feature
            .bbox
            .intersects_tile(tile_x0, tile_y0, tile_x1, tile_y1)
        {
            continue;
        }
        let color = Rgba(feature.color);
        for polygon in &feature.polygons {
            if !polygon
                .bbox
                .intersects_tile(tile_x0, tile_y0, tile_x1, tile_y1)
            {
                continue;
            }
            fill_ring(tile, tile_x0, tile_y0, &polygon.outer, color);
            for hole in &polygon.holes {
                fill_ring(tile, tile_x0, tile_y0, hole, Rgba([0, 0, 0, 0]));
            }
        }
    }
}

fn fill_ring(tile: &mut RgbaImage, tile_x0: i32, tile_y0: i32, ring: &[Point], color: Rgba<u8>) {
    if ring.len() < 3 {
        return;
    }
    let Some(bbox) = ring_bbox(ring) else {
        return;
    };
    let tile_w = tile.width() as i32;
    let tile_h = tile.height() as i32;
    let x_min = tile_x0;
    let y_min = tile_y0;
    let x_max = tile_x0 + tile_w - 1;
    let y_max = tile_y0 + tile_h - 1;

    if !bbox.intersects_tile(x_min, y_min, x_max, y_max) {
        return;
    }

    let row_start = ((bbox.min_y - 0.5).ceil() as i32).max(y_min);
    let row_end = ((bbox.max_y - 0.5).floor() as i32).min(y_max);
    if row_end < row_start {
        return;
    }

    let mut intersections = Vec::with_capacity(ring.len());
    for gy in row_start..=row_end {
        intersections.clear();
        let y_center = gy as f64 + 0.5;

        let mut prev = ring[ring.len() - 1];
        for &curr in ring {
            if (prev.y <= y_center && curr.y > y_center)
                || (curr.y <= y_center && prev.y > y_center)
            {
                let t = (y_center - prev.y) / (curr.y - prev.y);
                let x = prev.x + t * (curr.x - prev.x);
                intersections.push(x);
            }
            prev = curr;
        }

        intersections.sort_by(|a, b| a.total_cmp(b));
        for segment in intersections.chunks_exact(2) {
            let left = segment[0].min(segment[1]);
            let right = segment[0].max(segment[1]);
            let col_start = ((left - 0.5).ceil() as i32).max(x_min);
            let col_end = ((right - 0.5).floor() as i32).min(x_max);
            if col_end < col_start {
                continue;
            }

            let local_y = (gy - tile_y0) as u32;
            for gx in col_start..=col_end {
                let local_x = (gx - tile_x0) as u32;
                tile.put_pixel(local_x, local_y, color);
            }
        }
    }
}

fn build_occupancy<I>(coords: I) -> Result<(i32, i32, u32, u32, Vec<u8>)>
where
    I: IntoIterator<Item = (i32, i32)>,
{
    let coords: Vec<(i32, i32)> = coords.into_iter().collect();
    let Some(&(first_x, first_y)) = coords.first() else {
        bail!("cannot build occupancy for empty level");
    };

    let mut min_x = first_x;
    let mut max_x = first_x;
    let mut min_y = first_y;
    let mut max_y = first_y;
    for (x, y) in &coords {
        min_x = min_x.min(*x);
        max_x = max_x.max(*x);
        min_y = min_y.min(*y);
        max_y = max_y.max(*y);
    }
    let width = (max_x - min_x + 1) as u32;
    let height = (max_y - min_y + 1) as u32;
    let len_bits = width as usize * height as usize;
    let mut bitset = vec![0_u8; len_bits.div_ceil(8)];
    for (x, y) in coords {
        let gx = (x - min_x) as usize;
        let gy = (y - min_y) as usize;
        let idx = gy * width as usize + gx;
        bitset[idx >> 3] |= 1_u8 << (idx & 7);
    }
    Ok((min_x, min_y, width, height, bitset))
}
