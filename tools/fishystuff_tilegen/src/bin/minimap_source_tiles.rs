use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

use anyhow::{anyhow, bail, Context, Result};
use chrono::{SecondsFormat, Utc};
use clap::Parser;
use pazifista::{
    canonical_archive_input_path, open_archive_index_path, ArchiveIndex, FileEntry,
    MINIMAP_TILE_ARCHIVE_FILTER,
};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(name = "minimap_source_tiles")]
#[command(about = "Build source-backed raw minimap rader_*.png tiles from original PAZ archives")]
struct Args {
    #[arg(long, default_value = "data/scratch/paz")]
    source_archive: PathBuf,
    #[arg(long, default_value = "data/scratch/minimap/source_tiles")]
    out_dir: PathBuf,
    #[arg(long)]
    force: bool,
    #[arg(long)]
    magick: Option<PathBuf>,
    #[arg(long)]
    convert_concurrency: Option<usize>,
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
struct SourceManifest<'a> {
    generated_at_utc: String,
    source_archive: &'a str,
    resolved_archive_file: &'a str,
    archive_filter: &'a str,
    tile_count: usize,
}

#[derive(Deserialize)]
struct SourceManifestOwned {
    source_archive: String,
    resolved_archive_file: String,
    archive_filter: String,
    tile_count: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let source_archive_input = render_input_path(&args.source_archive)?;
    let resolved_archive_file = canonical_archive_input_path(&args.source_archive)?;
    let resolved_archive_string = resolved_archive_file.display().to_string();
    let output_dir = normalize_output_dir(&args.out_dir)?;
    let magick = args
        .magick
        .clone()
        .unwrap_or_else(|| PathBuf::from("magick"));

    ensure_magick_available(&magick)?;
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;

    let entries = load_minimap_entries(&resolved_archive_file)?;
    if entries.is_empty() {
        bail!("no archive entries matched {}", MINIMAP_TILE_ARCHIVE_FILTER);
    }

    let mut expected_output_names = HashSet::with_capacity(entries.len());
    let mut pending_entries = Vec::new();
    for entry in entries {
        let output_name = output_name_for_entry(&entry)
            .with_context(|| format!("unexpected minimap tile path {}", entry.file_path))?;
        expected_output_names.insert(output_name.clone());
        let output_path = output_dir.join(&output_name);
        if args.force || !output_path.is_file() {
            pending_entries.push(entry);
        }
    }

    if !args.quiet {
        println!(
            "resolved {} source-backed minimap tiles from {}",
            expected_output_names.len(),
            source_archive_input
        );
        if pending_entries.is_empty() {
            println!("raw minimap tile set is already current");
        } else {
            println!(
                "converting {} pending raw minimap tiles into {}",
                pending_entries.len(),
                output_dir.display()
            );
        }
    }

    let converted_count = convert_pending_entries(
        &resolved_archive_file,
        &output_dir,
        &magick,
        pending_entries,
        args.convert_concurrency,
        args.quiet,
    )?;
    let pruned_count = prune_stale_raw_tiles(&output_dir, &expected_output_names, args.quiet)?;

    let manifest = SourceManifest {
        generated_at_utc: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        source_archive: &source_archive_input,
        resolved_archive_file: &resolved_archive_string,
        archive_filter: MINIMAP_TILE_ARCHIVE_FILTER,
        tile_count: expected_output_names.len(),
    };
    write_source_manifest(&output_dir.join("source-manifest.json"), &manifest)?;

    if !args.quiet {
        println!(
            "raw minimap tile set ready under {} (converted {}, pruned {})",
            output_dir.display(),
            converted_count,
            pruned_count
        );
    }

    Ok(())
}

fn render_input_path(path: &Path) -> Result<String> {
    if path.is_absolute() {
        return Ok(path.display().to_string());
    }
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let joined = cwd.join(path);
    let rendered = joined
        .strip_prefix(&cwd)
        .unwrap_or(&joined)
        .display()
        .to_string();
    Ok(rendered)
}

fn normalize_output_dir(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()
            .context("failed to read current directory")?
            .join(path))
    }
}

fn ensure_magick_available(magick: &Path) -> Result<()> {
    let output = Command::new(magick)
        .arg("-version")
        .output()
        .with_context(|| format!("failed to execute {}", magick.display()))?;
    if !output.status.success() {
        bail!(
            "{} -version failed with status {}",
            magick.display(),
            output.status
        );
    }
    Ok(())
}

fn load_minimap_entries(source_archive: &Path) -> Result<Vec<FileEntry>> {
    let archive = open_archive_index_path(source_archive, true)?;
    let entries = archive.matching_entries(&[MINIMAP_TILE_ARCHIVE_FILTER.to_string()]);
    Ok(entries)
}

fn output_name_for_entry(entry: &FileEntry) -> Option<String> {
    let basename = Path::new(&entry.file_path).file_name()?.to_str()?;
    let stem = basename.strip_suffix(".dds")?;
    if !stem.starts_with("rader_") {
        return None;
    }
    Some(format!("{stem}.png"))
}

fn convert_pending_entries(
    source_archive: &Path,
    output_dir: &Path,
    magick: &Path,
    pending_entries: Vec<FileEntry>,
    requested_concurrency: Option<usize>,
    quiet: bool,
) -> Result<usize> {
    if pending_entries.is_empty() {
        return Ok(0);
    }

    let total = pending_entries.len();
    let worker_count = requested_concurrency
        .unwrap_or_else(default_convert_concurrency)
        .max(1)
        .min(total);
    let mut buckets = vec![Vec::new(); worker_count];
    for (index, entry) in pending_entries.into_iter().enumerate() {
        buckets[index % worker_count].push(entry);
    }

    let completed = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::with_capacity(worker_count);
    for (worker_index, bucket) in buckets.into_iter().enumerate() {
        if bucket.is_empty() {
            continue;
        }
        let completed = Arc::clone(&completed);
        let source_archive = source_archive.to_path_buf();
        let output_dir = output_dir.to_path_buf();
        let magick = magick.to_path_buf();
        handles.push(thread::spawn(move || -> Result<usize> {
            let mut archive = open_archive_index_path(&source_archive, true)?;
            let worker_temp_dir = std::env::temp_dir().join(format!(
                "fishystuff-minimap-source-{}-{}",
                std::process::id(),
                worker_index
            ));
            if worker_temp_dir.exists() {
                fs::remove_dir_all(&worker_temp_dir)
                    .with_context(|| format!("failed to clear {}", worker_temp_dir.display()))?;
            }
            fs::create_dir_all(&worker_temp_dir)
                .with_context(|| format!("failed to create {}", worker_temp_dir.display()))?;

            let result = convert_bucket(
                &mut archive,
                &bucket,
                &output_dir,
                &magick,
                &worker_temp_dir,
                &completed,
                total,
                quiet,
            );
            let cleanup_result = fs::remove_dir_all(&worker_temp_dir);
            if let Err(err) = cleanup_result {
                if result.is_ok() {
                    return Err(err).with_context(|| {
                        format!("failed to remove {}", worker_temp_dir.display())
                    });
                }
            }
            result
        }));
    }

    let mut converted = 0usize;
    for handle in handles {
        let worker_result = handle
            .join()
            .map_err(|panic| anyhow!("minimap source worker panicked: {panic:?}"))??;
        converted += worker_result;
    }
    Ok(converted)
}

fn convert_bucket(
    archive: &mut ArchiveIndex,
    entries: &[FileEntry],
    output_dir: &Path,
    magick: &Path,
    worker_temp_dir: &Path,
    completed: &AtomicUsize,
    total: usize,
    quiet: bool,
) -> Result<usize> {
    let mut converted = 0usize;
    for entry in entries {
        let output_name = output_name_for_entry(entry)
            .with_context(|| format!("unexpected minimap tile path {}", entry.file_path))?;
        let output_path = output_dir.join(&output_name);
        let temp_input_path = worker_temp_dir.join(output_name.replace(".png", ".dds"));
        archive
            .write_entry(entry, &temp_input_path, true)
            .with_context(|| format!("failed to extract {}", entry.file_path))?;
        convert_dds_to_png(magick, &temp_input_path, &output_path)?;
        fs::remove_file(&temp_input_path)
            .with_context(|| format!("failed to remove {}", temp_input_path.display()))?;
        converted += 1;
        let completed_now = completed.fetch_add(1, Ordering::Relaxed) + 1;
        if !quiet && (completed_now == total || completed_now % 250 == 0) {
            println!("converted {completed_now}/{total} raw minimap tiles");
        }
    }
    Ok(converted)
}

fn convert_dds_to_png(magick: &Path, input_path: &Path, output_path: &Path) -> Result<()> {
    let status = Command::new(magick)
        .arg(input_path)
        .arg("-strip")
        .arg(format!("PNG32:{}", output_path.display()))
        .status()
        .with_context(|| format!("failed to execute {}", magick.display()))?;
    if !status.success() {
        bail!(
            "{} failed to convert {} to {} with status {}",
            magick.display(),
            input_path.display(),
            output_path.display(),
            status
        );
    }
    Ok(())
}

fn prune_stale_raw_tiles(
    output_dir: &Path,
    expected_output_names: &HashSet<String>,
    quiet: bool,
) -> Result<usize> {
    let mut removed = 0usize;
    for entry in fs::read_dir(output_dir)
        .with_context(|| format!("failed to read {}", output_dir.display()))?
    {
        let entry = entry.with_context(|| format!("failed to read {}", output_dir.display()))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with("rader_") || !name.ends_with(".png") {
            continue;
        }
        if expected_output_names.contains(name) {
            continue;
        }
        fs::remove_file(&path).with_context(|| format!("failed to remove {}", path.display()))?;
        removed += 1;
        if !quiet {
            println!("removed stale raw minimap tile {}", path.display());
        }
    }
    Ok(removed)
}

fn write_source_manifest(path: &Path, manifest: &SourceManifest<'_>) -> Result<()> {
    if let Ok(current) = fs::read(path) {
        if let Ok(existing) = serde_json::from_slice::<SourceManifestOwned>(&current) {
            if existing.source_archive == manifest.source_archive
                && existing.resolved_archive_file == manifest.resolved_archive_file
                && existing.archive_filter == manifest.archive_filter
                && existing.tile_count == manifest.tile_count
            {
                return Ok(());
            }
        }
    }
    let next = serde_json::to_vec_pretty(manifest).context("serialize source manifest")?;
    fs::write(path, next).with_context(|| format!("failed to write {}", path.display()))
}

fn default_convert_concurrency() -> usize {
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(4)
        .clamp(2, 8)
}

#[cfg(test)]
mod tests {
    use super::output_name_for_entry;
    use pazifista::FileEntry;

    #[test]
    fn output_name_maps_rader_dds_to_png() {
        let entry = FileEntry {
            paz_num: 1,
            offset: 0,
            compressed_size: 0,
            original_size: 0,
            file_name: "rader_24_-14.dds".to_string(),
            file_path:
                "ui_texture/new_ui_common_forlua/widget/rader/minimap_data_pack/rader_24_-14.dds"
                    .to_string(),
        };
        assert_eq!(
            output_name_for_entry(&entry).as_deref(),
            Some("rader_24_-14.png")
        );
    }
}
