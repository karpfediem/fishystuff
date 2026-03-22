use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use clap::Parser;
use image::imageops::{overlay, resize, FilterType};
use image::{ImageReader, Rgba, RgbaImage};
use serde::Serialize;

type ParentChildren = Vec<(i32, i32, u32, u32)>;

#[derive(Parser, Debug)]
#[command(name = "fishystuff_mask_pyramid")]
#[command(about = "Build a multi-resolution zone-mask tile pyramid and tileset manifest")]
struct Args {
    /// Directory containing level-0 source tiles named <x>_<y>.png
    #[arg(long)]
    input_dir: PathBuf,
    /// Output directory containing level folders (0/,1/,...) and tileset.json
    #[arg(long)]
    out_dir: PathBuf,
    /// Tile size for both input and output levels
    #[arg(long, default_value_t = 512)]
    tile_px: u32,
    /// Maximum generated level (inclusive)
    #[arg(long, default_value_t = 4)]
    max_level: u32,
    /// Canonical map width in pixels
    #[arg(long, default_value_t = 11_560)]
    map_width: u32,
    /// Canonical map height in pixels
    #[arg(long, default_value_t = 10_540)]
    map_height: u32,
    /// Root URL used by the runtime to resolve tile paths
    #[arg(long, default_value = "/images/tiles/mask/v1")]
    root_url: String,
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

fn main() -> Result<()> {
    let args = Args::parse();
    if args.tile_px == 0 {
        bail!("--tile-px must be > 0");
    }
    if !args.input_dir.is_dir() {
        bail!("input directory not found: {}", args.input_dir.display());
    }
    fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("create output dir: {}", args.out_dir.display()))?;

    let level0 = collect_level0(&args.input_dir)?;
    if level0.is_empty() {
        bail!("no mask tiles found in {}", args.input_dir.display());
    }

    copy_level0(&args.input_dir, &args.out_dir.join("0"), &level0)?;

    let mut levels: Vec<HashSet<(i32, i32)>> = Vec::new();
    levels.push(level0);

    for z in 1..=args.max_level {
        let prev = levels[(z - 1) as usize].clone();
        if prev.is_empty() {
            levels.push(HashSet::new());
            continue;
        }
        let next = build_next_level(
            &args.out_dir.join((z - 1).to_string()),
            &args.out_dir.join(z.to_string()),
            &prev,
            args.tile_px,
        )?;
        levels.push(next);
    }

    let mut manifest_levels = Vec::new();
    for (z, coords) in levels.into_iter().enumerate() {
        if coords.is_empty() {
            continue;
        }
        let (min_x, min_y, width, height, bitset) = build_occupancy(coords.iter().copied())?;
        manifest_levels.push(LevelManifest {
            z: z as u32,
            min_x,
            min_y,
            width,
            height,
            tile_count: coords.len(),
            path: "{z}/{x}_{y}.png".to_string(),
            occupancy_b64: BASE64_STANDARD.encode(bitset),
        });
    }

    let manifest = TilesetManifest {
        version: 1,
        map_size_px: [args.map_width, args.map_height],
        tile_size_px: args.tile_px,
        root: args.root_url,
        levels: manifest_levels,
    };
    let out_path = args.out_dir.join("tileset.json");
    let bytes = serde_json::to_vec_pretty(&manifest).context("serialize tileset manifest")?;
    fs::write(&out_path, bytes).with_context(|| format!("write {}", out_path.display()))?;
    Ok(())
}

fn collect_level0(input_dir: &Path) -> Result<HashSet<(i32, i32)>> {
    let mut out = HashSet::new();
    for entry in fs::read_dir(input_dir).with_context(|| format!("read {}", input_dir.display()))? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if let Some(coord) = parse_tile_name(&name) {
            out.insert(coord);
        }
    }
    Ok(out)
}

fn copy_level0(input_dir: &Path, out_dir: &Path, coords: &HashSet<(i32, i32)>) -> Result<()> {
    fs::create_dir_all(out_dir).with_context(|| format!("create {}", out_dir.display()))?;
    for &(x, y) in coords {
        let src = input_dir.join(tile_name(x, y));
        let dst = out_dir.join(tile_name(x, y));
        fs::copy(&src, &dst)
            .with_context(|| format!("copy {} -> {}", src.display(), dst.display()))?;
    }
    Ok(())
}

fn build_next_level(
    prev_dir: &Path,
    next_dir: &Path,
    prev_coords: &HashSet<(i32, i32)>,
    tile_px: u32,
) -> Result<HashSet<(i32, i32)>> {
    fs::create_dir_all(next_dir).with_context(|| format!("create {}", next_dir.display()))?;

    let mut parents: HashMap<(i32, i32), ParentChildren> = HashMap::new();
    for &(x, y) in prev_coords {
        let (px, py, qx, qy) = tile_parent_quadrant(x, y);
        parents.entry((px, py)).or_default().push((x, y, qx, qy));
    }

    let mut next_coords = HashSet::with_capacity(parents.len());
    for ((px, py), children) in parents {
        let mut canvas = RgbaImage::from_pixel(tile_px * 2, tile_px * 2, Rgba([0, 0, 0, 0]));
        let mut has_child = false;
        let mut occupied_width = 0_u32;
        let mut occupied_height = 0_u32;
        for (cx, cy, qx, qy) in children {
            let child_path = prev_dir.join(tile_name(cx, cy));
            let child = ImageReader::open(&child_path)
                .with_context(|| format!("open child tile {}", child_path.display()))?
                .with_guessed_format()
                .with_context(|| format!("guess format for {}", child_path.display()))?
                .decode()
                .with_context(|| format!("decode child tile {}", child_path.display()))?
                .to_rgba8();
            let draw_x = qx * tile_px;
            let draw_y = qy * tile_px;
            overlay(&mut canvas, &child, draw_x as i64, draw_y as i64);
            occupied_width = occupied_width.max(draw_x + child.width());
            occupied_height = occupied_height.max(draw_y + child.height());
            has_child = true;
        }
        if !has_child {
            continue;
        }
        let occupied_width = occupied_width.max(1);
        let occupied_height = occupied_height.max(1);
        let occupied =
            image::imageops::crop_imm(&canvas, 0, 0, occupied_width, occupied_height).to_image();
        let down = resize(
            &occupied,
            occupied_width.div_ceil(2),
            occupied_height.div_ceil(2),
            FilterType::Triangle,
        );
        let out_path = next_dir.join(tile_name(px, py));
        down.save(&out_path)
            .with_context(|| format!("write parent tile {}", out_path.display()))?;
        next_coords.insert((px, py));
    }

    Ok(next_coords)
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

fn tile_name(x: i32, y: i32) -> String {
    format!("{}_{}.png", x, y)
}

fn parse_tile_name(name: &str) -> Option<(i32, i32)> {
    let stem = name.strip_suffix(".png")?;
    let mut parts = stem.split('_');
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((x, y))
}

fn tile_parent_quadrant(x: i32, y: i32) -> (i32, i32, u32, u32) {
    let px = x.div_euclid(2);
    let py = y.div_euclid(2);
    let qx = x.rem_euclid(2) as u32;
    let qy = y.rem_euclid(2) as u32;
    (px, py, qx, qy)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use image::{ImageReader, Rgba, RgbaImage};

    use super::{build_next_level, tile_name, tile_parent_quadrant};

    static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let unique = format!(
                "fishystuff-mask-pyramid-{}-{}",
                std::process::id(),
                TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed)
            );
            let path = std::env::temp_dir().join(unique);
            fs::create_dir_all(&path).expect("create temp test directory");
            Self { path }
        }

        fn join(&self, name: &str) -> PathBuf {
            self.path.join(name)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn write_tile(path: &Path, width: u32, height: u32) {
        let tile = RgbaImage::from_pixel(width, height, Rgba([255, 0, 0, 255]));
        tile.save(path).expect("write test tile");
    }

    #[test]
    fn parent_quadrants_handle_positive_tiles() {
        assert_eq!(tile_parent_quadrant(0, 0), (0, 0, 0, 0));
        assert_eq!(tile_parent_quadrant(1, 0), (0, 0, 1, 0));
        assert_eq!(tile_parent_quadrant(0, 1), (0, 0, 0, 1));
        assert_eq!(tile_parent_quadrant(3, 2), (1, 1, 1, 0));
    }

    #[test]
    fn partial_edge_children_preserve_parent_edge_dimensions() {
        let root = TestDir::new();
        let prev_dir = root.join("0");
        let next_dir = root.join("1");
        fs::create_dir_all(&prev_dir).expect("create prev dir");

        write_tile(&prev_dir.join(tile_name(22, 20)), 296, 300);

        let prev_coords = HashSet::from([(22, 20)]);
        let next_coords =
            build_next_level(&prev_dir, &next_dir, &prev_coords, 512).expect("build next level");
        assert_eq!(next_coords, HashSet::from([(11, 10)]));

        let parent = ImageReader::open(next_dir.join(tile_name(11, 10)))
            .expect("open parent tile")
            .decode()
            .expect("decode parent tile");
        assert_eq!(parent.width(), 148);
        assert_eq!(parent.height(), 150);
    }

    #[test]
    fn full_children_still_generate_full_parent_tiles() {
        let root = TestDir::new();
        let prev_dir = root.join("0");
        let next_dir = root.join("1");
        fs::create_dir_all(&prev_dir).expect("create prev dir");

        for &(x, y) in &[(0, 0), (1, 0), (0, 1), (1, 1)] {
            write_tile(&prev_dir.join(tile_name(x, y)), 512, 512);
        }

        let prev_coords = HashSet::from([(0, 0), (1, 0), (0, 1), (1, 1)]);
        let next_coords =
            build_next_level(&prev_dir, &next_dir, &prev_coords, 512).expect("build next level");
        assert_eq!(next_coords, HashSet::from([(0, 0)]));

        let parent = ImageReader::open(next_dir.join(tile_name(0, 0)))
            .expect("open parent tile")
            .decode()
            .expect("decode parent tile");
        assert_eq!(parent.width(), 512);
        assert_eq!(parent.height(), 512);
    }
}
