use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use bevy::prelude::World;
use bevy::time::TimeUpdateStrategy;
use clap::Parser;

use crate::app::{build_native_app, NativeAppOptions};
use crate::map::events::EventsSnapshotState;
use crate::map::layers::{LayerRegistry, LayerRuntime};
use crate::map::raster::{RasterTileCache, TileStats};
use crate::plugins::points::PointsState;
use crate::profiling::fixtures::FixtureData;
use crate::profiling::scenario::ScenarioName;
use crate::profiling::{self, ReportMetadata};

#[derive(Debug, Parser)]
pub struct HarnessCli {
    #[arg(long, value_enum)]
    pub scenario: ScenarioName,
    #[arg(long)]
    pub output: PathBuf,
    #[arg(long)]
    pub frames: Option<u64>,
    #[arg(long)]
    pub seconds: Option<f64>,
    #[arg(long, default_value_t = 120)]
    pub warmup_frames: u64,
    #[arg(long)]
    pub fixture_root: Option<PathBuf>,
    #[arg(long)]
    pub trace_chrome: Option<PathBuf>,
    #[arg(long, default_value_t = false)]
    pub headless: bool,
    #[arg(long, default_value_t = 1280)]
    pub window_width: u32,
    #[arg(long, default_value_t = 720)]
    pub window_height: u32,
}

pub fn run(cli: HarnessCli) -> Result<()> {
    let fixture_root = cli.fixture_root.unwrap_or_else(default_fixture_root);
    let fixture_root = fixture_root
        .canonicalize()
        .with_context(|| format!("canonicalize fixture root {}", fixture_root.display()))?;
    let fixtures = FixtureData::load(&fixture_root)?;

    let measured_frames = cli.frames.unwrap_or_else(|| {
        cli.seconds
            .map(|seconds| (seconds * 60.0).round() as u64)
            .unwrap_or_else(|| cli.scenario.default_frames())
    });
    let total_frames = cli.warmup_frames.saturating_add(measured_frames);

    let mut app = build_native_app(&NativeAppOptions {
        asset_root: fixture_root.to_string_lossy().to_string(),
        width: cli.window_width,
        height: cli.window_height,
        visible: !cli.headless,
        renderless: cli.headless,
    });
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )));
    fixtures.seed_world(app.world_mut());

    profiling::reset(profiling::ProfilingConfig {
        enabled: true,
        capture_after_frame: cli.warmup_frames,
        capture_trace: cli.trace_chrome.is_some(),
    });

    let started_at = Instant::now();
    for frame in 0..total_frames {
        profiling::begin_frame(frame);
        cli.scenario.apply(app.world_mut(), frame, total_frames);
        pin_snapshot_cache(app.world_mut());
        app.update();
        sample_counters(app.world());
        profiling::end_frame(frame);
    }

    let summary = profiling::report(ReportMetadata {
        scenario: cli.scenario.as_str().to_string(),
        bevy_version: "0.18.0".to_string(),
        git_revision: git_revision(),
        build_profile: profile_name(),
        frames: measured_frames,
        warmup_frames: cli.warmup_frames,
        wall_clock_ms: started_at.elapsed().as_secs_f64() * 1000.0,
    });

    write_summary(&cli.output, &summary)?;
    if let Some(trace_path) = cli.trace_chrome.as_ref() {
        if let Some(parent) = trace_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create trace dir {}", parent.display()))?;
        }
        profiling::write_trace(trace_path)?;
    }

    println!("{}", hotspot_summary(&summary, 8));
    Ok(())
}

fn default_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/profiling")
}

fn profile_name() -> String {
    std::env::var("PROFILE").unwrap_or_else(|_| "dev".to_string())
}

fn git_revision() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn write_summary(path: &Path, summary: &profiling::ProfileSummary) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create output dir {}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(summary).context("encode profiling summary")?;
    std::fs::write(path, bytes).with_context(|| format!("write summary {}", path.display()))?;
    Ok(())
}

fn pin_snapshot_cache(world: &mut World) {
    let now = world.resource::<bevy::prelude::Time>().elapsed_secs_f64();
    let mut snapshot = world.resource_mut::<EventsSnapshotState>();
    if snapshot.loaded {
        snapshot.last_meta_poll_at_secs = now;
    }
}

fn sample_counters(world: &World) {
    let registry = world.resource::<LayerRegistry>();
    let layer_runtime = world.resource::<LayerRuntime>();
    let tile_stats = world.resource::<TileStats>();
    let tile_cache = world.resource::<RasterTileCache>();
    let points = world.resource::<PointsState>();
    let snapshot = world.resource::<EventsSnapshotState>();

    let mut raster_visible = 0_u64;
    let mut raster_desired = 0_u64;
    let mut field_visible = 0_u64;
    let mut field_desired = 0_u64;
    let mut vector_features = 0_u64;
    let mut vector_triangles = 0_u64;
    for (layer_id, state) in layer_runtime.iter() {
        let Some(spec) = registry.get(layer_id) else {
            continue;
        };
        if spec.is_raster() {
            let desired_tiles = state.visible_tile_count as u64
                + state.pending_count as u64
                + state.inflight_count as u64;
            raster_visible = raster_visible.saturating_add(state.visible_tile_count as u64);
            raster_desired = raster_desired.saturating_add(desired_tiles);
            crate::perf_gauge!(
                format!("raster.visible_tiles.layer.{}", spec.key),
                state.visible_tile_count
            );
            crate::perf_gauge!(
                format!("raster.desired_tiles.layer.{}", spec.key),
                desired_tiles
            );
            crate::perf_gauge!(
                format!("raster.resident_tiles.layer.{}", spec.key),
                state.resident_tile_count
            );
        } else if spec.is_field() {
            let desired_tiles = state.visible_tile_count as u64
                + state.pending_count as u64
                + state.inflight_count as u64;
            field_visible = field_visible.saturating_add(state.visible_tile_count as u64);
            field_desired = field_desired.saturating_add(desired_tiles);
            crate::perf_gauge!(
                format!("field.visible_tiles.layer.{}", spec.key),
                state.visible_tile_count
            );
            crate::perf_gauge!(
                format!("field.desired_tiles.layer.{}", spec.key),
                desired_tiles
            );
            crate::perf_gauge!(
                format!("field.resident_tiles.layer.{}", spec.key),
                state.resident_tile_count
            );
        } else if spec.is_vector() {
            vector_features = vector_features.saturating_add(state.vector_feature_count as u64);
            vector_triangles = vector_triangles.saturating_add(state.vector_triangle_count as u64);
            crate::perf_gauge!(
                format!("vector.feature_count.layer.{}", spec.key),
                state.vector_feature_count
            );
            crate::perf_gauge!(
                format!("vector.triangle_count.layer.{}", spec.key),
                state.vector_triangle_count
            );
            crate::perf_gauge!(
                format!("vector.cache_entries.layer.{}", spec.key),
                state.vector_cache_entries
            );
        }
    }

    crate::perf_gauge!("raster.visible_tiles", raster_visible);
    crate::perf_gauge!("raster.desired_tiles", raster_desired);
    crate::perf_gauge!("field.visible_tiles", field_visible);
    crate::perf_gauge!("field.desired_tiles", field_desired);
    crate::perf_gauge!("raster.cache_entries", tile_cache.len());
    crate::perf_gauge!("raster.blank_visible_tiles", tile_stats.blank_visible_tiles);
    crate::perf_gauge!(
        "raster.fallback_visible_tiles",
        tile_stats.fallback_visible_tiles
    );
    crate::perf_gauge!("vector.feature_count", vector_features);
    crate::perf_gauge!("vector.triangle_count", vector_triangles);
    crate::perf_gauge!("events.snapshot_size", snapshot.event_count);
    crate::perf_gauge!("events.candidate_count", points.candidate_count);
    crate::perf_gauge!("events.rendered_clusters", points.rendered_cluster_count);
    crate::perf_gauge!("events.rendered_points", points.rendered_point_count);
    crate::perf_last!("raster.cache_entries_current", tile_cache.len());
    crate::perf_last!("events.snapshot_size_current", snapshot.event_count);
}

pub fn hotspot_summary(summary: &profiling::ProfileSummary, limit: usize) -> String {
    let mut spans = summary.named_spans.iter().collect::<Vec<_>>();
    spans.sort_by(|left, right| right.1.total_ms.total_cmp(&left.1.total_ms));
    let mut lines = vec![format!(
        "scenario={} frames={} warmup={} frame_avg_ms={:.3} p95_ms={:.3}",
        summary.scenario,
        summary.frames,
        summary.warmup_frames,
        summary.frame_time_ms.avg,
        summary.frame_time_ms.p95
    )];
    for (name, stats) in spans.into_iter().take(limit) {
        lines.push(format!(
            "{} total_ms={:.3} avg_ms={:.3} p95_ms={:.3} count={}",
            name, stats.total_ms, stats.avg_ms, stats.p95_ms, stats.count
        ));
    }
    lines.join("\n")
}
