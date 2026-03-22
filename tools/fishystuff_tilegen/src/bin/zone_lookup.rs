use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use fishystuff_core::masks::{ZoneLookupRows, ZoneMask};

#[derive(Parser, Debug)]
#[command(name = "fishystuff_zone_lookup")]
#[command(about = "Build a compact exact zone-lookup asset from the canonical zone mask PNG")]
struct Args {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mask = ZoneMask::load_png(&args.input)
        .with_context(|| format!("load zone mask {}", args.input.display()))?;
    let lookup = ZoneLookupRows::from_zone_mask(&mask).context("build zone lookup rows")?;
    let bytes = lookup.to_bytes();
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create output dir {}", parent.display()))?;
    }
    fs::write(&args.output, &bytes)
        .with_context(|| format!("write zone lookup {}", args.output.display()))?;
    println!(
        "wrote {} ({}x{}, {} row segments, {} bytes)",
        args.output.display(),
        lookup.width(),
        lookup.height(),
        lookup.segment_count(),
        bytes.len()
    );
    Ok(())
}
