use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use clap::{Parser, Subcommand, ValueEnum};
use fishystuff_core::terrain::{
    chunk_grid_dims_for_level, chunk_map_bounds, decode_terrain_chunk, encode_terrain_chunk,
    key_for_map_px, normalized_height_to_u16, packed_rgb24_norm_from_rgb,
    sample_chunk_norm_at_map_px, world_height_from_normalized, TerrainChunkData,
    TerrainChunkLodKey, TerrainDrapeLayerKind, TerrainDrapeManifest, TerrainHeightEncoding,
    TerrainLevelManifest, TerrainManifest,
};
use image::{ImageReader, RgbaImage};

#[derive(Parser, Debug)]
#[command(name = "fishystuff_terrain_pyramid")]
#[command(about = "Build terrain geometry and drape chunk pyramids", long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    BuildTerrainPyramid(BuildTerrainPyramidArgs),
    BuildTerrainDrapePyramid(BuildTerrainDrapePyramidArgs),
}

#[derive(Parser, Debug, Clone)]
struct BuildTerrainPyramidArgs {
    /// Full-resolution terrain source tile directory (e.g. terraintiles/7/)
    #[arg(long)]
    source_root: PathBuf,
    /// Output directory containing manifest.json and levels/<z>/*.thc
    #[arg(long)]
    out_dir: PathBuf,
    /// Runtime revision identifier
    #[arg(long, default_value = "v1")]
    revision: String,
    /// Public root URL for runtime chunk fetches
    #[arg(long, default_value = "/images/terrain/v1")]
    root_url: String,
    /// Chunk path template relative to root URL
    #[arg(long, default_value = "levels/{level}/{x}_{y}.thc")]
    chunk_path: String,
    /// Canonical map width in MapSpace pixels
    #[arg(long, default_value_t = 11_560)]
    map_width: u32,
    /// Canonical map height in MapSpace pixels
    #[arg(long, default_value_t = 10_540)]
    map_height: u32,
    /// Base chunk footprint in MapSpace pixels at level 0
    #[arg(long, default_value_t = 256)]
    chunk_map_px: u32,
    /// Grid edge resolution per chunk (samples = edge*edge)
    #[arg(long, default_value_t = 65)]
    grid_size: u16,
    /// Maximum generated level (0 finest, larger = coarser)
    #[arg(long, default_value_t = 7)]
    max_level: u8,
    /// Terrain bounding-box minimum Y used to decode packed RGB24 heights
    #[arg(long, default_value_t = -9_500.0)]
    bbox_y_min: f32,
    /// Terrain bounding-box maximum Y used to decode packed RGB24 heights
    #[arg(long, default_value_t = 24_000.0)]
    bbox_y_max: f32,
    /// Optional explicit source tile size. If omitted, auto-detected.
    #[arg(long)]
    source_tile_size: Option<u32>,
    /// Max number of decoded source tiles kept in memory while sampling
    #[arg(long, default_value_t = 96)]
    source_tile_cache: usize,
}

#[derive(Parser, Debug, Clone)]
struct BuildTerrainDrapePyramidArgs {
    /// Terrain manifest produced by build-terrain-pyramid
    #[arg(long)]
    terrain_manifest: PathBuf,
    /// Canonical MapSpace source image for the drape layer
    #[arg(long)]
    source_image: PathBuf,
    /// Output directory containing manifest.json and levels/<z>/*.png
    #[arg(long)]
    out_dir: PathBuf,
    /// Runtime revision identifier
    #[arg(long, default_value = "v1")]
    revision: String,
    /// Layer key (e.g. minimap, zone_mask)
    #[arg(long, default_value = "minimap")]
    layer: String,
    /// Public root URL for runtime drape chunk fetches
    #[arg(long, default_value = "/images/terrain_drape/minimap/v1")]
    root_url: String,
    /// Chunk path template relative to root URL
    #[arg(long, default_value = "levels/{level}/{x}_{y}.png")]
    chunk_path: String,
    /// Output texture size per chunk
    #[arg(long, default_value_t = 256)]
    texture_px: u16,
    /// Layer kind for runtime sampling semantics
    #[arg(long, value_enum, default_value_t = DrapeKindArg::RasterVisual)]
    kind: DrapeKindArg,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum DrapeKindArg {
    RasterVisual,
    RasterData,
}

impl From<DrapeKindArg> for TerrainDrapeLayerKind {
    fn from(value: DrapeKindArg) -> Self {
        match value {
            DrapeKindArg::RasterVisual => TerrainDrapeLayerKind::RasterVisual,
            DrapeKindArg::RasterData => TerrainDrapeLayerKind::RasterData,
        }
    }
}

#[derive(Debug, Clone)]
struct SourceTileIndex {
    tiles: HashMap<(i32, i32), PathBuf>,
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
    tile_size: u32,
    width_px: u32,
    height_px: u32,
    has_lossy_webp: bool,
}

#[derive(Debug, Default, Clone, Copy)]
struct SamplingStats {
    missing_samples: u64,
}

#[derive(Debug, Clone)]
struct SourceTilePixels {
    width: u32,
    height: u32,
    rgb: Vec<u8>,
}

#[derive(Debug)]
struct SourceTileSampler {
    index: SourceTileIndex,
    cache_limit: usize,
    cache: HashMap<(i32, i32), SourceTilePixels>,
    cache_order: Vec<(i32, i32)>,
}

impl SourceTileSampler {
    fn new(index: SourceTileIndex, cache_limit: usize) -> Self {
        Self {
            index,
            cache_limit: cache_limit.max(8),
            cache: HashMap::new(),
            cache_order: Vec::new(),
        }
    }

    fn sample_norm_bilinear(&mut self, x: f32, y: f32) -> Option<f32> {
        if self.index.width_px == 0 || self.index.height_px == 0 {
            return None;
        }
        let fx = x.clamp(0.0, self.index.width_px.saturating_sub(1) as f32);
        let fy = y.clamp(0.0, self.index.height_px.saturating_sub(1) as f32);
        let x0 = fx.floor() as i32;
        let y0 = fy.floor() as i32;
        let x1 = (x0 + 1).min(self.index.width_px as i32 - 1);
        let y1 = (y0 + 1).min(self.index.height_px as i32 - 1);
        let tx = (fx - x0 as f32).clamp(0.0, 1.0);
        let ty = (fy - y0 as f32).clamp(0.0, 1.0);

        let h00 = self.sample_norm_nearest(x0, y0)?;
        let h10 = self.sample_norm_nearest(x1, y0)?;
        let h01 = self.sample_norm_nearest(x0, y1)?;
        let h11 = self.sample_norm_nearest(x1, y1)?;

        let top = h00 + (h10 - h00) * tx;
        let bottom = h01 + (h11 - h01) * tx;
        Some(top + (bottom - top) * ty)
    }

    fn sample_norm_nearest(&mut self, x: i32, y: i32) -> Option<f32> {
        let rgb = self.sample_rgb_nearest(x, y)?;
        Some(packed_rgb24_norm_from_rgb(rgb))
    }

    fn sample_rgb_nearest(&mut self, x: i32, y: i32) -> Option<[u8; 3]> {
        if x < 0
            || y < 0
            || x >= self.index.width_px as i32
            || y >= self.index.height_px as i32
            || self.index.tile_size == 0
        {
            return None;
        }
        let tile_size = self.index.tile_size as i32;
        let tile_x = x.div_euclid(tile_size) + self.index.min_x;
        let tile_y = y.div_euclid(tile_size) + self.index.min_y;
        let local_x = x.rem_euclid(tile_size) as u32;
        let local_y = y.rem_euclid(tile_size) as u32;

        let tile = self.get_tile(tile_x, tile_y)?;
        if local_x >= tile.width || local_y >= tile.height {
            return None;
        }
        let idx = (local_y as usize * tile.width as usize + local_x as usize) * 3;
        Some([
            *tile.rgb.get(idx)?,
            *tile.rgb.get(idx + 1)?,
            *tile.rgb.get(idx + 2)?,
        ])
    }

    fn get_tile(&mut self, tile_x: i32, tile_y: i32) -> Option<&SourceTilePixels> {
        if self.cache.contains_key(&(tile_x, tile_y)) {
            self.touch((tile_x, tile_y));
            return self.cache.get(&(tile_x, tile_y));
        }

        let path = self.index.tiles.get(&(tile_x, tile_y))?;
        let image = ImageReader::open(path)
            .ok()?
            .with_guessed_format()
            .ok()?
            .decode()
            .ok()?
            .to_rgb8();
        let tile = SourceTilePixels {
            width: image.width(),
            height: image.height(),
            rgb: image.into_raw(),
        };
        self.cache.insert((tile_x, tile_y), tile);
        self.touch((tile_x, tile_y));
        self.evict_if_needed();
        self.cache.get(&(tile_x, tile_y))
    }

    fn touch(&mut self, key: (i32, i32)) {
        if let Some(pos) = self.cache_order.iter().position(|entry| *entry == key) {
            self.cache_order.remove(pos);
        }
        self.cache_order.push(key);
    }

    fn evict_if_needed(&mut self) {
        while self.cache.len() > self.cache_limit {
            let Some(oldest) = self.cache_order.first().copied() else {
                break;
            };
            self.cache_order.remove(0);
            self.cache.remove(&oldest);
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.cmd {
        Command::BuildTerrainPyramid(cmd) => build_terrain_pyramid(cmd),
        Command::BuildTerrainDrapePyramid(cmd) => build_terrain_drape_pyramid(cmd),
    }
}

fn build_terrain_pyramid(args: BuildTerrainPyramidArgs) -> Result<()> {
    if args.grid_size < 2 {
        bail!("--grid-size must be >= 2");
    }
    if args.chunk_map_px == 0 {
        bail!("--chunk-map-px must be > 0");
    }
    if args.map_width < 2 || args.map_height < 2 {
        bail!("--map-width and --map-height must be >= 2");
    }
    if !matches!(
        args.bbox_y_max.partial_cmp(&args.bbox_y_min),
        Some(std::cmp::Ordering::Greater)
    ) {
        bail!("bbox range must be strictly increasing");
    }

    let index = build_source_tile_index(&args.source_root, args.source_tile_size)?;
    if index.has_lossy_webp {
        eprintln!(
            "warning: source tiles are .webp (lossy). Final production terrain should use lossless source tiles if available."
        );
    }
    let expected_coverage =
        (index.max_x - index.min_x + 1) as usize * (index.max_y - index.min_y + 1) as usize;
    if expected_coverage != index.tiles.len() {
        eprintln!(
            "warning: source coverage has holes: detected={} expected_dense={}",
            index.tiles.len(),
            expected_coverage
        );
    }

    fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("create output dir {}", args.out_dir.display()))?;

    let mut sampler = SourceTileSampler::new(index.clone(), args.source_tile_cache);
    let mut levels: Vec<HashMap<(i32, i32), TerrainChunkData>> =
        Vec::with_capacity(args.max_level as usize + 1);
    let mut min_norm = f32::INFINITY;
    let mut max_norm = f32::NEG_INFINITY;
    let mut stats = SamplingStats::default();

    let level0 = build_finest_level_chunks(
        &mut sampler,
        &args,
        &index,
        &mut min_norm,
        &mut max_norm,
        &mut stats,
    )?;
    levels.push(level0);

    for level in 1..=args.max_level {
        let next =
            build_coarser_level_chunks(level, &levels[level as usize - 1], &args, &mut stats)?;
        levels.push(next);
    }

    let mut manifest_levels = Vec::with_capacity(levels.len());
    let mut chunk_counts = Vec::with_capacity(levels.len());
    for (level, chunks) in levels.iter().enumerate() {
        let level_dir = args.out_dir.join("levels").join(level.to_string());
        fs::create_dir_all(&level_dir)
            .with_context(|| format!("create {}", level_dir.display()))?;
        for ((cx, cy), chunk) in chunks {
            let out_path = level_dir.join(format!("{}_{}.thc", cx, cy));
            let bytes = encode_terrain_chunk(chunk)?;
            fs::write(&out_path, bytes).with_context(|| format!("write {}", out_path.display()))?;
            let decoded = decode_terrain_chunk(&fs::read(&out_path)?)?;
            if decoded.grid_size != chunk.grid_size || decoded.key != chunk.key {
                bail!("roundtrip mismatch for {}", out_path.display());
            }
        }
        let occupancy = build_occupancy(chunks.keys().copied())?;
        manifest_levels.push(TerrainLevelManifest {
            level: level as u8,
            min_x: occupancy.min_x,
            min_y: occupancy.min_y,
            width: occupancy.width,
            height: occupancy.height,
            tile_count: chunks.len(),
            occupancy_b64: BASE64_STANDARD.encode(&occupancy.bits),
        });
        chunk_counts.push((level as u8, chunks.len()));
    }

    let manifest = TerrainManifest {
        revision: args.revision.clone(),
        map_width: args.map_width,
        map_height: args.map_height,
        chunk_map_px: args.chunk_map_px,
        grid_size: args.grid_size,
        max_level: args.max_level,
        bbox_y_min: args.bbox_y_min,
        bbox_y_max: args.bbox_y_max,
        encoding: TerrainHeightEncoding::U16Norm,
        root: args.root_url.clone(),
        chunk_path: args.chunk_path.clone(),
        levels: manifest_levels,
    };
    let manifest_path = args.out_dir.join("manifest.json");
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).context("serialize terrain manifest")?,
    )
    .with_context(|| format!("write {}", manifest_path.display()))?;

    let min_world = world_height_from_normalized(min_norm, args.bbox_y_min, args.bbox_y_max);
    let max_world = world_height_from_normalized(max_norm, args.bbox_y_min, args.bbox_y_max);
    eprintln!(
        "terrain bake diagnostics: source_tiles={} tile_size={} source_dims={}x{} map_dims={}x{}",
        index.tiles.len(),
        index.tile_size,
        index.width_px,
        index.height_px,
        args.map_width,
        args.map_height
    );
    eprintln!(
        "terrain bake diagnostics: decoded_height_norm={:.6}..{:.6} world={:.2}..{:.2}",
        min_norm, max_norm, min_world, max_world
    );
    eprintln!(
        "terrain bake diagnostics: missing_samples={} levels={:?}",
        stats.missing_samples, chunk_counts
    );
    eprintln!("wrote terrain manifest: {}", manifest_path.display());

    Ok(())
}

fn build_terrain_drape_pyramid(args: BuildTerrainDrapePyramidArgs) -> Result<()> {
    if args.texture_px < 2 {
        bail!("--texture-px must be >= 2");
    }
    let manifest_bytes = fs::read(&args.terrain_manifest)
        .with_context(|| format!("read {}", args.terrain_manifest.display()))?;
    let terrain_manifest: TerrainManifest =
        serde_json::from_slice(&manifest_bytes).context("parse terrain manifest")?;

    let source_img = ImageReader::open(&args.source_image)
        .with_context(|| format!("open source image {}", args.source_image.display()))?
        .with_guessed_format()
        .context("guess source image format")?
        .decode()
        .context("decode source image")?
        .to_rgba8();

    fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("create output dir {}", args.out_dir.display()))?;

    let mut output_levels = Vec::new();
    for level in &terrain_manifest.levels {
        let decoded = level.decode()?;
        let level_dir = args.out_dir.join("levels").join(level.level.to_string());
        fs::create_dir_all(&level_dir)
            .with_context(|| format!("create {}", level_dir.display()))?;

        let mut written = 0usize;
        for cy in decoded.min_y..=decoded.max_y() {
            for cx in decoded.min_x..=decoded.max_x() {
                if !decoded.contains(cx, cy) {
                    continue;
                }
                let key = TerrainChunkLodKey {
                    level: level.level,
                    cx,
                    cy,
                };
                let image = render_drape_chunk_image(
                    &source_img,
                    terrain_manifest.map_width,
                    terrain_manifest.map_height,
                    terrain_manifest.chunk_map_px,
                    key,
                    args.texture_px,
                );
                let out = level_dir.join(format!("{}_{}.png", cx, cy));
                image
                    .save(&out)
                    .with_context(|| format!("write {}", out.display()))?;
                written += 1;
            }
        }
        output_levels.push(TerrainLevelManifest {
            level: level.level,
            min_x: level.min_x,
            min_y: level.min_y,
            width: level.width,
            height: level.height,
            tile_count: written,
            occupancy_b64: level.occupancy_b64.clone(),
        });
    }

    let drape_manifest = TerrainDrapeManifest {
        revision: args.revision.clone(),
        layer: args.layer.clone(),
        map_width: terrain_manifest.map_width,
        map_height: terrain_manifest.map_height,
        chunk_map_px: terrain_manifest.chunk_map_px,
        max_level: terrain_manifest.max_level,
        texture_px: args.texture_px,
        format: "png".to_string(),
        kind: args.kind.into(),
        root: args.root_url.clone(),
        chunk_path: args.chunk_path.clone(),
        levels: output_levels,
    };
    let manifest_path = args.out_dir.join("manifest.json");
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&drape_manifest).context("serialize drape manifest")?,
    )
    .with_context(|| format!("write {}", manifest_path.display()))?;
    eprintln!(
        "terrain drape bake diagnostics: source={} dims={}x{} levels={} manifest={}",
        args.source_image.display(),
        source_img.width(),
        source_img.height(),
        drape_manifest.levels.len(),
        manifest_path.display()
    );
    Ok(())
}

fn build_source_tile_index(
    source_root: &Path,
    tile_size_override: Option<u32>,
) -> Result<SourceTileIndex> {
    if !source_root.is_dir() {
        bail!("source tile root not found: {}", source_root.display());
    }

    let mut tiles: HashMap<(i32, i32), PathBuf> = HashMap::new();
    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;
    let mut tile_size = 0_u32;
    let mut has_lossy_webp = false;

    for entry in
        fs::read_dir(source_root).with_context(|| format!("read {}", source_root.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let path = entry.path();
        let ext = path
            .extension()
            .and_then(OsStr::to_str)
            .map(|value| value.to_ascii_lowercase())
            .unwrap_or_default();
        if ext != "webp" && ext != "png" {
            continue;
        }
        if ext == "webp" {
            has_lossy_webp = true;
        }
        let Some(stem) = path.file_stem().and_then(OsStr::to_str) else {
            continue;
        };
        let Some((x, y)) = parse_coord_stem(stem) else {
            continue;
        };

        let dims = ImageReader::open(&path)
            .with_context(|| format!("open {}", path.display()))?
            .with_guessed_format()
            .with_context(|| format!("guess image format {}", path.display()))?
            .into_dimensions()
            .with_context(|| format!("read dimensions {}", path.display()))?;
        if dims.0 != dims.1 {
            bail!(
                "non-square source tile {} has dims {}x{}",
                path.display(),
                dims.0,
                dims.1
            );
        }
        let detected = dims.0;
        if tile_size == 0 {
            tile_size = detected;
        } else if detected != tile_size {
            bail!(
                "inconsistent source tile size: {} has {}, expected {}",
                path.display(),
                detected,
                tile_size
            );
        }

        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
        tiles.insert((x, y), path);
    }

    if tiles.is_empty() {
        bail!(
            "no source terrain tiles found under {}",
            source_root.display()
        );
    }
    if let Some(override_size) = tile_size_override {
        if tile_size != override_size {
            bail!(
                "source tile size mismatch: detected {}, override {}",
                tile_size,
                override_size
            );
        }
    }
    let width_tiles = (max_x - min_x + 1) as u32;
    let height_tiles = (max_y - min_y + 1) as u32;
    let width_px = width_tiles.saturating_mul(tile_size);
    let height_px = height_tiles.saturating_mul(tile_size);
    Ok(SourceTileIndex {
        tiles,
        min_x,
        min_y,
        max_x,
        max_y,
        tile_size,
        width_px,
        height_px,
        has_lossy_webp,
    })
}

fn parse_coord_stem(stem: &str) -> Option<(i32, i32)> {
    let mut parts = stem.split('_');
    let x = parts.next()?.parse::<i32>().ok()?;
    let y = parts.next()?.parse::<i32>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((x, y))
}

fn build_finest_level_chunks(
    sampler: &mut SourceTileSampler,
    args: &BuildTerrainPyramidArgs,
    source_index: &SourceTileIndex,
    min_norm: &mut f32,
    max_norm: &mut f32,
    stats: &mut SamplingStats,
) -> Result<HashMap<(i32, i32), TerrainChunkData>> {
    let (tiles_x, tiles_y) =
        chunk_grid_dims_for_level(args.map_width, args.map_height, args.chunk_map_px, 0);
    let mut out = HashMap::new();
    let map_w_max = args.map_width.saturating_sub(1).max(1) as f32;
    let map_h_max = args.map_height.saturating_sub(1).max(1) as f32;
    let src_w_max = source_index.width_px.saturating_sub(1).max(1) as f32;
    let src_h_max = source_index.height_px.saturating_sub(1).max(1) as f32;

    for cy in 0..tiles_y {
        for cx in 0..tiles_x {
            let key = TerrainChunkLodKey { level: 0, cx, cy };
            let (x0, y0, x1, y1) =
                chunk_map_bounds(args.map_width, args.map_height, args.chunk_map_px, key);
            let heights = sample_chunk_grid(args.grid_size, |u, v| {
                let map_x = lerp(x0, x1, u);
                let map_y = lerp(y0, y1, v);
                let src_x = (map_x / map_w_max) * src_w_max;
                let src_y = (map_y / map_h_max) * src_h_max;
                if let Some(value) = sampler.sample_norm_bilinear(src_x, src_y) {
                    *min_norm = min_norm.min(value);
                    *max_norm = max_norm.max(value);
                    normalized_height_to_u16(value)
                } else {
                    stats.missing_samples = stats.missing_samples.saturating_add(1);
                    normalized_height_to_u16(0.0)
                }
            });
            out.insert(
                (cx, cy),
                TerrainChunkData {
                    key,
                    grid_size: args.grid_size,
                    encoding: TerrainHeightEncoding::U16Norm,
                    heights,
                },
            );
        }
    }
    Ok(out)
}

fn build_coarser_level_chunks(
    level: u8,
    prev_level: &HashMap<(i32, i32), TerrainChunkData>,
    args: &BuildTerrainPyramidArgs,
    stats: &mut SamplingStats,
) -> Result<HashMap<(i32, i32), TerrainChunkData>> {
    let prev_level_num = level.saturating_sub(1);
    let (tiles_x, tiles_y) =
        chunk_grid_dims_for_level(args.map_width, args.map_height, args.chunk_map_px, level);
    let mut out = HashMap::new();
    for cy in 0..tiles_y {
        for cx in 0..tiles_x {
            let key = TerrainChunkLodKey { level, cx, cy };
            let (x0, y0, x1, y1) =
                chunk_map_bounds(args.map_width, args.map_height, args.chunk_map_px, key);
            let heights = sample_chunk_grid(args.grid_size, |u, v| {
                let map_x = lerp(x0, x1, u);
                let map_y = lerp(y0, y1, v);
                if let Some(norm) = sample_norm_from_chunk_level(
                    prev_level,
                    args.map_width,
                    args.map_height,
                    args.chunk_map_px,
                    prev_level_num,
                    map_x,
                    map_y,
                ) {
                    normalized_height_to_u16(norm)
                } else {
                    stats.missing_samples = stats.missing_samples.saturating_add(1);
                    normalized_height_to_u16(0.0)
                }
            });
            out.insert(
                (cx, cy),
                TerrainChunkData {
                    key,
                    grid_size: args.grid_size,
                    encoding: TerrainHeightEncoding::U16Norm,
                    heights,
                },
            );
        }
    }
    Ok(out)
}

fn sample_norm_from_chunk_level(
    chunks: &HashMap<(i32, i32), TerrainChunkData>,
    map_width: u32,
    map_height: u32,
    chunk_map_px: u32,
    level: u8,
    map_x: f32,
    map_y: f32,
) -> Option<f32> {
    let key = key_for_map_px(map_x, map_y, chunk_map_px, level);
    let (tiles_x, tiles_y) = chunk_grid_dims_for_level(map_width, map_height, chunk_map_px, level);
    let cx = key.cx.clamp(0, tiles_x - 1);
    let cy = key.cy.clamp(0, tiles_y - 1);

    if let Some(chunk) = chunks.get(&(cx, cy)) {
        if let Some(value) =
            sample_chunk_norm_at_map_px(map_width, map_height, chunk_map_px, chunk, map_x, map_y)
        {
            return Some(value);
        }
    }

    let offsets = [
        (-1, 0),
        (1, 0),
        (0, -1),
        (0, 1),
        (-1, -1),
        (1, -1),
        (-1, 1),
        (1, 1),
    ];
    for (dx, dy) in offsets {
        let nx = (cx + dx).clamp(0, tiles_x - 1);
        let ny = (cy + dy).clamp(0, tiles_y - 1);
        if let Some(chunk) = chunks.get(&(nx, ny)) {
            if let Some(value) = sample_chunk_norm_at_map_px(
                map_width,
                map_height,
                chunk_map_px,
                chunk,
                map_x,
                map_y,
            ) {
                return Some(value);
            }
        }
    }
    None
}

fn sample_chunk_grid(mut grid_size: u16, mut sample: impl FnMut(f32, f32) -> u16) -> Vec<u16> {
    grid_size = grid_size.max(2);
    let edge = grid_size as usize;
    let mut out = Vec::with_capacity(edge.saturating_mul(edge));
    for gy in 0..edge {
        let v = gy as f32 / (edge - 1) as f32;
        for gx in 0..edge {
            let u = gx as f32 / (edge - 1) as f32;
            out.push(sample(u, v));
        }
    }
    out
}

fn render_drape_chunk_image(
    source: &RgbaImage,
    map_width: u32,
    map_height: u32,
    chunk_map_px: u32,
    key: TerrainChunkLodKey,
    texture_px: u16,
) -> RgbaImage {
    let edge = texture_px.max(2);
    let mut out = RgbaImage::new(edge as u32, edge as u32);
    let (x0, y0, x1, y1) = chunk_map_bounds(map_width, map_height, chunk_map_px, key);
    let map_w_max = map_width.saturating_sub(1).max(1) as f32;
    let map_h_max = map_height.saturating_sub(1).max(1) as f32;
    let src_w_max = source.width().saturating_sub(1).max(1) as f32;
    let src_h_max = source.height().saturating_sub(1).max(1) as f32;

    for py in 0..edge {
        let v = py as f32 / (edge - 1) as f32;
        let map_y = lerp(y0, y1, v);
        let src_y = (map_y / map_h_max) * src_h_max;
        for px in 0..edge {
            let u = px as f32 / (edge - 1) as f32;
            let map_x = lerp(x0, x1, u);
            let src_x = (map_x / map_w_max) * src_w_max;
            let rgba = sample_rgba_bilinear(source, src_x, src_y);
            out.put_pixel(px as u32, py as u32, image::Rgba(rgba));
        }
    }
    out
}

fn sample_rgba_bilinear(source: &RgbaImage, x: f32, y: f32) -> [u8; 4] {
    let max_x = source.width().saturating_sub(1) as f32;
    let max_y = source.height().saturating_sub(1) as f32;
    let fx = x.clamp(0.0, max_x);
    let fy = y.clamp(0.0, max_y);
    let x0 = fx.floor() as u32;
    let y0 = fy.floor() as u32;
    let x1 = (x0 + 1).min(source.width().saturating_sub(1));
    let y1 = (y0 + 1).min(source.height().saturating_sub(1));
    let tx = (fx - x0 as f32).clamp(0.0, 1.0);
    let ty = (fy - y0 as f32).clamp(0.0, 1.0);
    let p00 = source.get_pixel(x0, y0).0;
    let p10 = source.get_pixel(x1, y0).0;
    let p01 = source.get_pixel(x0, y1).0;
    let p11 = source.get_pixel(x1, y1).0;
    let mut out = [0_u8; 4];
    for ch in 0..4 {
        let top = p00[ch] as f32 + (p10[ch] as f32 - p00[ch] as f32) * tx;
        let bottom = p01[ch] as f32 + (p11[ch] as f32 - p01[ch] as f32) * tx;
        let value = top + (bottom - top) * ty;
        out[ch] = value.round().clamp(0.0, 255.0) as u8;
    }
    out
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[derive(Debug)]
struct OccupancyBits {
    min_x: i32,
    min_y: i32,
    width: u32,
    height: u32,
    bits: Vec<u8>,
}

fn build_occupancy<I>(keys: I) -> Result<OccupancyBits>
where
    I: IntoIterator<Item = (i32, i32)>,
{
    let keys: Vec<(i32, i32)> = keys.into_iter().collect();
    let Some(&(first_x, first_y)) = keys.first() else {
        bail!("cannot build occupancy for empty level");
    };
    let mut min_x = first_x;
    let mut max_x = first_x;
    let mut min_y = first_y;
    let mut max_y = first_y;
    for (x, y) in &keys {
        min_x = min_x.min(*x);
        max_x = max_x.max(*x);
        min_y = min_y.min(*y);
        max_y = max_y.max(*y);
    }
    let width = (max_x - min_x + 1) as u32;
    let height = (max_y - min_y + 1) as u32;
    let bit_count = width as usize * height as usize;
    let mut bits = vec![0_u8; bit_count.div_ceil(8)];
    for (x, y) in keys {
        let gx = (x - min_x) as usize;
        let gy = (y - min_y) as usize;
        let idx = gy * width as usize + gx;
        bits[idx >> 3] |= 1_u8 << (idx & 7);
    }
    Ok(OccupancyBits {
        min_x,
        min_y,
        width,
        height,
        bits,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{
        build_coarser_level_chunks, build_finest_level_chunks, build_occupancy,
        build_source_tile_index, render_drape_chunk_image, sample_rgba_bilinear,
        BuildTerrainPyramidArgs, SamplingStats, SourceTileSampler,
    };
    use fishystuff_core::terrain::{TerrainChunkData, TerrainChunkLodKey, TerrainHeightEncoding};
    use image::{Rgb, RgbImage, Rgba, RgbaImage};

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{}_{}", prefix, stamp));
        std::fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn packed_rgb(norm: f32) -> [u8; 3] {
        let packed = (norm.clamp(0.0, 1.0) * 16_777_215.0).round() as u32;
        [
            ((packed >> 16) & 0xFF) as u8,
            ((packed >> 8) & 0xFF) as u8,
            (packed & 0xFF) as u8,
        ]
    }

    #[test]
    fn scalar_bilinear_sampling_across_source_tiles() {
        let dir = temp_dir("terrain_source_tiles");
        let mut left = RgbImage::new(2, 2);
        let mut right = RgbImage::new(2, 2);
        let lo = packed_rgb(0.0);
        let hi = packed_rgb(1.0);
        for y in 0..2 {
            for x in 0..2 {
                left.put_pixel(x, y, Rgb(lo));
                right.put_pixel(x, y, Rgb(hi));
            }
        }
        left.save(dir.join("0_0.png")).expect("write left");
        right.save(dir.join("1_0.png")).expect("write right");

        let index = build_source_tile_index(&dir, None).expect("index");
        let mut sampler = SourceTileSampler::new(index, 8);
        let mid = sampler.sample_norm_bilinear(1.5, 0.5).expect("sample");
        assert!((mid - 0.5).abs() < 0.05);
    }

    #[test]
    fn finest_level_chunk_generation_from_synthetic_source() {
        let dir = temp_dir("terrain_finest_chunks");
        let mut tile = RgbImage::new(4, 4);
        for y in 0..4 {
            for x in 0..4 {
                let norm = ((x + y) as f32 / 6.0).clamp(0.0, 1.0);
                tile.put_pixel(x, y, Rgb(packed_rgb(norm)));
            }
        }
        tile.save(dir.join("0_0.png")).expect("write tile");

        let args = BuildTerrainPyramidArgs {
            source_root: dir.clone(),
            out_dir: dir.join("out"),
            revision: "vtest".to_string(),
            root_url: "/map/terrain/vtest".to_string(),
            chunk_path: "levels/{level}/{x}_{y}.thc".to_string(),
            map_width: 4,
            map_height: 4,
            chunk_map_px: 4,
            grid_size: 2,
            max_level: 1,
            bbox_y_min: 0.0,
            bbox_y_max: 100.0,
            source_tile_size: None,
            source_tile_cache: 8,
        };
        let index = build_source_tile_index(&args.source_root, None).expect("index");
        let mut sampler = SourceTileSampler::new(index.clone(), 8);
        let mut min_norm = f32::INFINITY;
        let mut max_norm = f32::NEG_INFINITY;
        let mut stats = SamplingStats::default();
        let level0 = build_finest_level_chunks(
            &mut sampler,
            &args,
            &index,
            &mut min_norm,
            &mut max_norm,
            &mut stats,
        )
        .expect("build level0");
        assert_eq!(level0.len(), 1);
        assert!(max_norm > min_norm);
        assert_eq!(stats.missing_samples, 0);
    }

    #[test]
    fn coarse_level_generation_downsamples_scalar_heights() {
        let mut prev: HashMap<(i32, i32), TerrainChunkData> = HashMap::new();
        prev.insert(
            (0, 0),
            TerrainChunkData {
                key: TerrainChunkLodKey {
                    level: 0,
                    cx: 0,
                    cy: 0,
                },
                grid_size: 2,
                encoding: TerrainHeightEncoding::U16Norm,
                heights: vec![0, 65_535, 65_535, 0],
            },
        );
        let args = BuildTerrainPyramidArgs {
            source_root: std::path::PathBuf::new(),
            out_dir: std::path::PathBuf::new(),
            revision: "vtest".to_string(),
            root_url: "/map/terrain/vtest".to_string(),
            chunk_path: "levels/{level}/{x}_{y}.thc".to_string(),
            map_width: 4,
            map_height: 4,
            chunk_map_px: 4,
            grid_size: 2,
            max_level: 1,
            bbox_y_min: 0.0,
            bbox_y_max: 1.0,
            source_tile_size: None,
            source_tile_cache: 8,
        };
        let mut stats = SamplingStats::default();
        let coarse = build_coarser_level_chunks(1, &prev, &args, &mut stats).expect("coarse");
        assert_eq!(coarse.len(), 1);
        let chunk = coarse.get(&(0, 0)).expect("chunk");
        assert_eq!(chunk.heights.len(), 4);
    }

    #[test]
    fn occupancy_bitset_roundtrip() {
        let bits = build_occupancy([(0, 0), (1, 0), (0, 1), (1, 1)]).expect("occupancy");
        assert_eq!(bits.width, 2);
        assert_eq!(bits.height, 2);
        assert_eq!(bits.bits.len(), 1);
        assert_eq!(bits.bits[0] & 0x0F, 0x0F);
    }

    #[test]
    fn drape_chunk_alignment_identity_map() {
        let mut source = RgbaImage::new(4, 4);
        for y in 0..4 {
            for x in 0..4 {
                source.put_pixel(x, y, Rgba([x as u8 * 10, y as u8 * 20, 99, 255]));
            }
        }
        let chunk = render_drape_chunk_image(
            &source,
            4,
            4,
            4,
            TerrainChunkLodKey {
                level: 0,
                cx: 0,
                cy: 0,
            },
            4,
        );
        assert_eq!(chunk.get_pixel(0, 0).0, [0, 0, 99, 255]);
        assert_eq!(chunk.get_pixel(3, 3).0, [30, 60, 99, 255]);
    }

    #[test]
    fn rgba_bilinear_sampling_is_stable() {
        let mut source = RgbaImage::new(2, 2);
        source.put_pixel(0, 0, Rgba([0, 0, 0, 255]));
        source.put_pixel(1, 0, Rgba([100, 0, 0, 255]));
        source.put_pixel(0, 1, Rgba([0, 100, 0, 255]));
        source.put_pixel(1, 1, Rgba([100, 100, 0, 255]));
        let sample = sample_rgba_bilinear(&source, 0.5, 0.5);
        assert_eq!(sample, [50, 50, 0, 255]);
    }
}
