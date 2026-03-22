use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use clap::Parser;
use serde::Serialize;

#[derive(Parser, Debug)]
#[command(name = "fishystuff_single_level_tileset")]
#[command(about = "Write a single-level tileset manifest for a fully covered raster grid")]
struct Args {
    #[arg(long)]
    out: PathBuf,
    #[arg(long)]
    tile_px: u32,
    #[arg(long)]
    map_width: u32,
    #[arg(long)]
    map_height: u32,
    #[arg(long)]
    root_url: String,
    #[arg(long, default_value = "{z}/{x}_{y}.png")]
    path: String,
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
    let tile_px = args.tile_px.max(1);
    let width = args.map_width.div_ceil(tile_px);
    let height = args.map_height.div_ceil(tile_px);
    let tile_count = width as usize * height as usize;
    let occupancy = vec![0xff_u8; tile_count.div_ceil(8)];

    let manifest = TilesetManifest {
        version: 1,
        map_size_px: [args.map_width, args.map_height],
        tile_size_px: tile_px,
        root: args.root_url,
        levels: vec![LevelManifest {
            z: 0,
            min_x: 0,
            min_y: 0,
            width,
            height,
            tile_count,
            path: args.path,
            occupancy_b64: BASE64_STANDARD.encode(occupancy),
        }],
    };

    if let Some(parent) = args.out.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(&manifest).context("serialize tileset manifest")?;
    fs::write(&args.out, bytes).with_context(|| format!("write {}", args.out.display()))?;
    Ok(())
}
