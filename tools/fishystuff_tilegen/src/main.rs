use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::Parser;
use image::{DynamicImage, GenericImageView, ImageReader};

#[derive(Parser, Debug)]
#[command(name = "fishystuff_tilegen")]
#[command(about = "Generate tiled map + zone mask assets", long_about = None)]
struct Args {
    /// Path to the input image
    #[arg(long)]
    input: PathBuf,
    /// Output directory for tiles
    #[arg(long)]
    out_dir: PathBuf,
    /// Tile size in pixels (default: 512)
    #[arg(long, default_value_t = 512)]
    tile_size: u32,
    /// Optional expected width (guards against wrong inputs)
    #[arg(long)]
    expect_width: Option<u32>,
    /// Optional expected height (guards against wrong inputs)
    #[arg(long)]
    expect_height: Option<u32>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.expect_width.is_some() ^ args.expect_height.is_some() {
        bail!("--expect-width and --expect-height must be provided together");
    }

    let img = load_image(&args.input).with_context(|| "load input image")?;
    validate_dimensions(&img, args.expect_width, args.expect_height)?;
    write_tiles(&img, &args.out_dir, args.tile_size)?;

    Ok(())
}

fn load_image(path: &Path) -> Result<DynamicImage> {
    let img = ImageReader::open(path)
        .with_context(|| format!("open image: {}", path.display()))?
        .with_guessed_format()
        .context("guess image format")?
        .decode()
        .context("decode image")?;
    Ok(img)
}

fn validate_dimensions(
    img: &DynamicImage,
    expect_width: Option<u32>,
    expect_height: Option<u32>,
) -> Result<()> {
    if let (Some(w), Some(h)) = (expect_width, expect_height) {
        let (img_w, img_h) = img.dimensions();
        if img_w != w || img_h != h {
            bail!(
                "dimensions mismatch: {}x{} (expected {}x{})",
                img_w,
                img_h,
                w,
                h
            );
        }
    }
    Ok(())
}

fn write_tiles(img: &DynamicImage, out_dir: &Path, tile_size: u32) -> Result<()> {
    let (width, height) = img.dimensions();
    let tiles_x = width.div_ceil(tile_size);
    let tiles_y = height.div_ceil(tile_size);

    fs::create_dir_all(out_dir)
        .with_context(|| format!("create tile dir: {}", out_dir.display()))?;

    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let x0 = tx * tile_size;
            let y0 = ty * tile_size;
            let tile_w = (width - x0).min(tile_size);
            let tile_h = (height - y0).min(tile_size);
            let sub = img.crop_imm(x0, y0, tile_w, tile_h);
            let out_path = out_dir.join(format!("{}_{}.png", tx, ty));
            sub.save(&out_path)
                .with_context(|| format!("write tile: {}", out_path.display()))?;
        }
    }

    Ok(())
}
