use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fishystuff_ui_bevy::profiling::bench_support;

fn bench_raster_visible_tiles(c: &mut Criterion) {
    let fixture = bench_support::raster_fixture();
    c.bench_function("raster_visible_tile_computation", |b| {
        b.iter(|| bench_support::raster_visible_tile_computation(black_box(&fixture)))
    });
}

fn bench_raster_desired_set(c: &mut Criterion) {
    let fixture = bench_support::raster_fixture();
    c.bench_function("raster_desired_set_build", |b| {
        b.iter(|| bench_support::raster_desired_set_build(black_box(&fixture)))
    });
}

fn bench_raster_eviction(c: &mut Criterion) {
    let fixture = bench_support::raster_fixture();
    c.bench_function("raster_eviction_scoring", |b| {
        b.iter(|| bench_support::raster_eviction_score_sum(black_box(&fixture)))
    });
}

fn bench_vector_triangulation(c: &mut Criterion) {
    let fixture = bench_support::vector_fixture();
    c.bench_function("vector_triangulation", |b| {
        b.iter(|| bench_support::vector_triangulation(black_box(&fixture)))
    });
}

fn bench_terrain_mesh(c: &mut Criterion) {
    let fixture = bench_support::terrain_fixture();
    c.bench_function("terrain_chunk_mesh_generation", |b| {
        b.iter(|| bench_support::terrain_mesh_build(black_box(&fixture)))
    });
}

fn bench_events_bbox_query(c: &mut Criterion) {
    let fixture = bench_support::events_fixture();
    c.bench_function("events_snapshot_bbox_query", |b| {
        b.iter(|| bench_support::event_bbox_query(black_box(&fixture)))
    });
}

fn bench_events_clustering(c: &mut Criterion) {
    let fixture = bench_support::events_fixture();
    c.bench_function("events_clustering", |b| {
        b.iter(|| bench_support::event_clustering(black_box(&fixture)))
    });
}

criterion_group!(
    perf_hotpaths,
    bench_raster_visible_tiles,
    bench_raster_desired_set,
    bench_raster_eviction,
    bench_vector_triangulation,
    bench_terrain_mesh,
    bench_events_bbox_query,
    bench_events_clustering,
);
criterion_main!(perf_hotpaths);
