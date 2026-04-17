mod field_layers;
mod mysql_store;
mod ranking;
mod region_groups;
mod region_layers;

use std::collections::BTreeSet;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::{TimeZone, Utc};
use clap::{Parser, Subcommand};
use csv::{ReaderBuilder, StringRecord};
use fishystuff_config::load_api_database_url_from_secretspec;
use sha2::{Digest, Sha256};

use crate::mysql_store::{
    EventZoneInsertRow, EventZoneRingSupportInsertRow, MySqlIngestStore, RankingEventRow,
};
use fishystuff_analytics::{
    compute_zone_stats, compute_zone_stats_with_config, zone_stats_to_json, QueryParams,
    ZoneStatusConfig,
};
use fishystuff_core::constants::{DISTANCE_PER_PIXEL, MAP_HEIGHT, MAP_WIDTH, SECTOR_SCALE};
use fishystuff_core::coord::{pixel_if_in_bounds, world_to_pixel_f, world_to_pixel_round};
use fishystuff_core::masks::{pack_rgb_u32, WaterSampler, ZoneMask};
use fishystuff_core::snap::snap_to_water;
use fishystuff_core::tile::{pixel_to_tile, tile_dimensions};
use fishystuff_core::transform::TransformKind;
use fishystuff_store::sqlite::SqliteStore;
use fishystuff_store::{Event, WaterTile};
use fishystuff_zones_meta::{DoltZonesMetaProvider, ZonesMetaProvider};
use ranking::{parse_datetime_utc, RankingRow};

const SOURCE_KIND_RANKING: u8 = 1;
const RANKING_RING_RADIUS_WORLD_UNITS: f64 = 500.0;
const RANKING_RING_SAMPLE_COUNT: usize = 64;

#[derive(Parser)]
#[command(name = "fishystuff_ingest")]
struct Cli {
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Ingest {
        #[arg(long)]
        ranking_csv: PathBuf,
        #[arg(long)]
        watermap: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        ignore_watermap: bool,
        #[arg(long)]
        out_db: PathBuf,
        #[arg(long, default_value_t = 32)]
        tile_px: i32,
        #[arg(long, default_value_t = 32)]
        snap_radius: i32,
        #[arg(long)]
        watermap_transform: Option<String>,
        #[arg(long)]
        watermap_sx: Option<f64>,
        #[arg(long)]
        watermap_sy: Option<f64>,
        #[arg(long)]
        watermap_ox: Option<f64>,
        #[arg(long)]
        watermap_oy: Option<f64>,
    },
    IngestMysql {
        #[arg(long)]
        ranking_csv: PathBuf,
        #[arg(long)]
        map_version: Option<String>,
        #[arg(long)]
        watermap: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        ignore_watermap: bool,
        #[arg(long, default_value_t = 32)]
        tile_px: i32,
        #[arg(long, default_value_t = 32)]
        snap_radius: i32,
        #[arg(long)]
        watermap_transform: Option<String>,
        #[arg(long)]
        watermap_sx: Option<f64>,
        #[arg(long)]
        watermap_sy: Option<f64>,
        #[arg(long)]
        watermap_ox: Option<f64>,
        #[arg(long)]
        watermap_oy: Option<f64>,
    },
    ImportRanking {
        #[arg(long)]
        ranking_csv: PathBuf,
        #[arg(long)]
        map_version: Option<String>,
    },
    IndexWaterTiles {
        #[arg(long)]
        watermap: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        ignore_watermap: bool,
        #[arg(long, default_value_t = 32)]
        tile_px: i32,
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        watermap_transform: Option<String>,
        #[arg(long)]
        watermap_sx: Option<f64>,
        #[arg(long)]
        watermap_sy: Option<f64>,
        #[arg(long)]
        watermap_ox: Option<f64>,
        #[arg(long)]
        watermap_oy: Option<f64>,
    },
    IndexWaterTilesMysql {
        #[arg(long)]
        map_version: Option<String>,
        #[arg(long)]
        watermap: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        ignore_watermap: bool,
        #[arg(long, default_value_t = 32)]
        tile_px: i32,
        #[arg(long)]
        watermap_transform: Option<String>,
        #[arg(long)]
        watermap_sx: Option<f64>,
        #[arg(long)]
        watermap_sy: Option<f64>,
        #[arg(long)]
        watermap_ox: Option<f64>,
        #[arg(long)]
        watermap_oy: Option<f64>,
    },
    IndexZoneMask {
        #[arg(long)]
        db: PathBuf,
        #[arg(long)]
        map_version: String,
        #[arg(long)]
        zone_mask: PathBuf,
        #[arg(long, default_value_t = false)]
        overwrite: bool,
    },
    IndexZoneMaskMysql {
        #[arg(long)]
        map_version: Option<String>,
        #[arg(long)]
        zone_mask: PathBuf,
        #[arg(long, default_value_t = false)]
        overwrite: bool,
    },
    BuildEventZoneAssignment {
        #[arg(long)]
        layer_revision_id: String,
        #[arg(long, default_value = "zone_mask")]
        layer_id: String,
        #[arg(long)]
        zone_mask_root: PathBuf,
        #[arg(long, default_value_t = 4096)]
        batch_size: usize,
    },
    ImportRegionGroupsMysql {
        #[arg(long)]
        map_version: Option<String>,
        #[arg(long)]
        geojson: PathBuf,
        #[arg(long)]
        regioninfo_bss: PathBuf,
        #[arg(long)]
        regiongroupinfo_bss: PathBuf,
        #[arg(long, default_value = "original")]
        source: String,
    },
    BuildDetailedRegionsGeojson {
        #[arg(long)]
        regions_geojson: PathBuf,
        #[arg(long)]
        regioninfo_bss: PathBuf,
        #[arg(long)]
        regiongroupinfo_bss: PathBuf,
        #[arg(long, help = "Original localization .loc file")]
        loc: PathBuf,
        #[arg(long = "waypoint-xml", required = true)]
        waypoint_xml: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    BuildRegionGroupsGeojson {
        #[arg(long)]
        region_groups_geojson: PathBuf,
        #[arg(long)]
        regioninfo_bss: PathBuf,
        #[arg(long)]
        regiongroupinfo_bss: PathBuf,
        #[arg(long, help = "Original localization .loc file")]
        loc: PathBuf,
        #[arg(long = "waypoint-xml", required = true)]
        waypoint_xml: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    BuildRegionNodesGeojson {
        #[arg(long)]
        regioninfo_bss: PathBuf,
        #[arg(long)]
        regiongroupinfo_bss: PathBuf,
        #[arg(long, help = "Original localization .loc file")]
        loc: PathBuf,
        #[arg(long = "waypoint-xml", required = true)]
        waypoint_xml: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    BuildRegionsFieldMetadata {
        #[arg(long)]
        field: PathBuf,
        #[arg(long)]
        regioninfo_bss: PathBuf,
        #[arg(long)]
        regiongroupinfo_bss: PathBuf,
        #[arg(long, help = "Original localization .loc file")]
        loc: PathBuf,
        #[arg(long = "waypoint-xml", required = true)]
        waypoint_xml: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    BuildRegionGroupsFieldMetadata {
        #[arg(long)]
        field: PathBuf,
        #[arg(long)]
        regions_field: PathBuf,
        #[arg(long)]
        regioninfo_bss: PathBuf,
        #[arg(long)]
        regiongroupinfo_bss: PathBuf,
        #[arg(long, help = "Original localization .loc file")]
        loc: PathBuf,
        #[arg(long = "waypoint-xml", required = true)]
        waypoint_xml: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    BuildZoneMaskFieldMetadata {
        #[arg(long)]
        field: PathBuf,
        #[arg(long)]
        out: PathBuf,
    },
    DebugWatermapProjection {
        #[arg(long)]
        watermap: Option<PathBuf>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        landmarks_csv: Option<PathBuf>,
        #[arg(long, default_value = "rgb")]
        projection_mode: String,
        #[arg(long)]
        watermap_transform: Option<String>,
        #[arg(long)]
        watermap_sx: Option<f64>,
        #[arg(long)]
        watermap_sy: Option<f64>,
        #[arg(long)]
        watermap_ox: Option<f64>,
        #[arg(long)]
        watermap_oy: Option<f64>,
    },
    ZoneStats {
        #[arg(long)]
        db: Option<PathBuf>,
        #[arg(long)]
        map_version: Option<String>,
        #[arg(long)]
        rgb: String,
        #[arg(long)]
        from_ts: i64,
        #[arg(long)]
        to_ts: i64,
        #[arg(long)]
        tile_px: Option<u32>,
        #[arg(long)]
        sigma_tiles: Option<f64>,
        #[arg(long)]
        fish_norm: bool,
        #[arg(long, default_value_t = 1.0)]
        alpha0: f64,
        #[arg(long, default_value_t = 30)]
        top_k: usize,
        #[arg(long)]
        fish_names: Option<PathBuf>,
        #[arg(long)]
        dolt_repo: Option<PathBuf>,
        #[arg(long)]
        dolt_ref: Option<String>,
        #[arg(long)]
        half_life_days: Option<f64>,
        #[arg(long)]
        drift_boundary_ts: Option<i64>,
    },
}

struct IngestCommand {
    ranking_csv: PathBuf,
    watermap: Option<PathBuf>,
    ignore_watermap: bool,
    out_db: PathBuf,
    tile_px: i32,
    snap_radius: i32,
    water_xform: WatermapTransformArgs,
}

struct ZoneStatsCommand {
    db: Option<PathBuf>,
    map_version: Option<String>,
    rgb: String,
    from_ts: i64,
    to_ts: i64,
    tile_px: Option<u32>,
    sigma_tiles: Option<f64>,
    fish_norm: bool,
    alpha0: f64,
    top_k: usize,
    fish_names: Option<PathBuf>,
    dolt_repo: Option<PathBuf>,
    dolt_ref: Option<String>,
    half_life_days: Option<f64>,
    drift_boundary_ts: Option<i64>,
    config: Option<fishystuff_config::Config>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = if let Some(path) = &cli.config {
        Some(fishystuff_config::load_config(path)?)
    } else {
        None
    };
    match cli.command {
        Commands::Ingest {
            ranking_csv,
            watermap,
            ignore_watermap,
            out_db,
            tile_px,
            snap_radius,
            watermap_transform,
            watermap_sx,
            watermap_sy,
            watermap_ox,
            watermap_oy,
        } => run_ingest(
            IngestCommand {
                ranking_csv,
                watermap,
                ignore_watermap,
                out_db,
                tile_px,
                snap_radius,
                water_xform: WatermapTransformArgs::new(
                    watermap_transform,
                    watermap_sx,
                    watermap_sy,
                    watermap_ox,
                    watermap_oy,
                ),
            },
            config.as_ref(),
        ),
        Commands::IngestMysql {
            ranking_csv,
            map_version,
            ..
        } => run_ingest_mysql(ranking_csv, map_version, config.as_ref()),
        Commands::ImportRanking {
            ranking_csv,
            map_version,
        } => run_ingest_mysql(ranking_csv, map_version, config.as_ref()),
        Commands::IndexWaterTiles {
            watermap,
            ignore_watermap,
            tile_px,
            db,
            watermap_transform,
            watermap_sx,
            watermap_sy,
            watermap_ox,
            watermap_oy,
        } => run_index_water_tiles(
            watermap,
            ignore_watermap,
            tile_px,
            db,
            WatermapTransformArgs::new(
                watermap_transform,
                watermap_sx,
                watermap_sy,
                watermap_ox,
                watermap_oy,
            ),
            config.as_ref(),
        ),
        Commands::IndexWaterTilesMysql {
            map_version,
            watermap,
            ignore_watermap,
            tile_px,
            watermap_transform,
            watermap_sx,
            watermap_sy,
            watermap_ox,
            watermap_oy,
        } => run_index_water_tiles_mysql(
            map_version,
            watermap,
            ignore_watermap,
            tile_px,
            WatermapTransformArgs::new(
                watermap_transform,
                watermap_sx,
                watermap_sy,
                watermap_ox,
                watermap_oy,
            ),
            config.as_ref(),
        ),
        Commands::IndexZoneMask {
            db,
            map_version,
            zone_mask,
            overwrite,
        } => run_index_zone_mask(db, map_version, zone_mask, overwrite),
        Commands::IndexZoneMaskMysql {
            map_version,
            zone_mask,
            ..
        } => run_index_zone_mask_mysql(map_version, zone_mask, 4096, config.as_ref()),
        Commands::BuildEventZoneAssignment {
            layer_revision_id,
            layer_id,
            zone_mask_root,
            batch_size,
        } => run_build_event_zone_assignment_mysql(
            layer_revision_id,
            layer_id,
            zone_mask_root,
            batch_size,
        ),
        Commands::ImportRegionGroupsMysql {
            map_version,
            geojson,
            regioninfo_bss,
            regiongroupinfo_bss,
            source,
        } => run_import_region_groups_mysql(
            map_version,
            geojson,
            regioninfo_bss,
            regiongroupinfo_bss,
            source,
            config.as_ref(),
        ),
        Commands::BuildDetailedRegionsGeojson {
            regions_geojson,
            regioninfo_bss,
            regiongroupinfo_bss,
            loc,
            waypoint_xml,
            out,
        } => run_build_detailed_regions_geojson(
            regions_geojson,
            regioninfo_bss,
            regiongroupinfo_bss,
            loc,
            waypoint_xml,
            out,
        ),
        Commands::BuildRegionGroupsGeojson {
            region_groups_geojson,
            regioninfo_bss,
            regiongroupinfo_bss,
            loc,
            waypoint_xml,
            out,
        } => run_build_region_groups_geojson(
            region_groups_geojson,
            regioninfo_bss,
            regiongroupinfo_bss,
            loc,
            waypoint_xml,
            out,
        ),
        Commands::BuildRegionNodesGeojson {
            regioninfo_bss,
            regiongroupinfo_bss,
            loc,
            waypoint_xml,
            out,
        } => run_build_region_nodes_geojson(
            regioninfo_bss,
            regiongroupinfo_bss,
            loc,
            waypoint_xml,
            out,
        ),
        Commands::BuildRegionsFieldMetadata {
            field,
            regioninfo_bss,
            regiongroupinfo_bss,
            loc,
            waypoint_xml,
            out,
        } => run_build_regions_field_metadata(
            field,
            regioninfo_bss,
            regiongroupinfo_bss,
            loc,
            waypoint_xml,
            out,
        ),
        Commands::BuildRegionGroupsFieldMetadata {
            field,
            regions_field,
            regioninfo_bss,
            regiongroupinfo_bss,
            loc,
            waypoint_xml,
            out,
        } => run_build_region_groups_field_metadata(
            field,
            regions_field,
            regioninfo_bss,
            regiongroupinfo_bss,
            loc,
            waypoint_xml,
            out,
        ),
        Commands::BuildZoneMaskFieldMetadata { field, out } => {
            run_build_zone_mask_field_metadata(field, out)
        }
        Commands::DebugWatermapProjection {
            watermap,
            out,
            landmarks_csv,
            projection_mode,
            watermap_transform,
            watermap_sx,
            watermap_sy,
            watermap_ox,
            watermap_oy,
        } => run_debug_watermap_projection(
            watermap,
            out,
            landmarks_csv,
            projection_mode,
            WatermapTransformArgs::new(
                watermap_transform,
                watermap_sx,
                watermap_sy,
                watermap_ox,
                watermap_oy,
            ),
            config.as_ref(),
        ),
        Commands::ZoneStats {
            db,
            map_version,
            rgb,
            from_ts,
            to_ts,
            tile_px,
            sigma_tiles,
            fish_norm,
            alpha0,
            top_k,
            fish_names,
            dolt_repo,
            dolt_ref,
            half_life_days,
            drift_boundary_ts,
        } => run_zone_stats(ZoneStatsCommand {
            db,
            map_version,
            rgb,
            from_ts,
            to_ts,
            tile_px,
            sigma_tiles,
            fish_norm,
            alpha0,
            top_k,
            fish_names,
            dolt_repo,
            dolt_ref,
            half_life_days,
            drift_boundary_ts,
            config,
        }),
    }
}

fn run_ingest(command: IngestCommand, config: Option<&fishystuff_config::Config>) -> Result<()> {
    let IngestCommand {
        ranking_csv,
        watermap,
        ignore_watermap,
        out_db,
        tile_px,
        snap_radius,
        water_xform,
    } = command;
    if tile_px <= 0 {
        bail!("tile_px must be > 0");
    }
    if snap_radius < 0 {
        bail!("snap_radius must be >= 0");
    }
    let water = if ignore_watermap {
        None
    } else {
        Some(load_water_sampler(watermap, &water_xform, config)?)
    };

    let mut store = SqliteStore::open(&out_db).context("open db")?;
    let file = File::open(&ranking_csv)
        .with_context(|| format!("open ranking csv: {}", ranking_csv.display()))?;
    let mut rdr = ReaderBuilder::new().delimiter(b';').from_reader(file);

    let mut batch: Vec<Event> = Vec::with_capacity(1024);
    for result in rdr.deserialize::<RankingRow>() {
        let row = result.context("read ranking row")?;
        let ts_utc = parse_datetime_utc(&row.date)?;
        let mut px = None;
        let mut py = None;
        let mut water_px = None;
        let mut water_py = None;
        let mut tile_x = None;
        let mut tile_y = None;
        let mut water_ok = false;

        let pixel = world_to_pixel_round(row.x, row.z);
        if let Some(pix) = pixel_if_in_bounds(pixel.x, pixel.y) {
            px = Some(pix.x);
            py = Some(pix.y);
            if let Some(water) = &water {
                let snap = snap_to_water(water, pix.x, pix.y, snap_radius);
                water_ok = snap.water_ok;
                water_px = snap.water_px;
                water_py = snap.water_py;
            } else {
                water_ok = true;
                water_px = Some(pix.x);
                water_py = Some(pix.y);
            }
            if let (Some(wx), Some(wy)) = (water_px, water_py) {
                let (tx, ty) = pixel_to_tile(wx, wy, tile_px);
                tile_x = Some(tx);
                tile_y = Some(ty);
            }
        }

        batch.push(Event {
            ts_utc,
            fish_id: row.encyclopedia_key,
            world_x: row.x,
            world_z: row.z,
            px,
            py,
            water_px,
            water_py,
            tile_x,
            tile_y,
            water_ok,
        });

        if batch.len() >= 1024 {
            store.insert_events(&batch).context("insert events")?;
            batch.clear();
        }
    }
    if !batch.is_empty() {
        store.insert_events(&batch).context("insert events")?;
    }

    Ok(())
}

fn run_ingest_mysql(
    ranking_csv: PathBuf,
    map_version: Option<String>,
    config: Option<&fishystuff_config::Config>,
) -> Result<()> {
    let database_url = resolve_database_url()?;
    let map_version = resolve_map_version(map_version, config)?;
    let store = MySqlIngestStore::open(&database_url).context("open mysql store")?;
    let input_sha256 = sha256_file(&ranking_csv)?;

    let ingest_run_id = store
        .start_ingest_run(SOURCE_KIND_RANKING, &map_version, &input_sha256)
        .context("start ingest run")?;

    let file = File::open(&ranking_csv)
        .with_context(|| format!("open ranking csv: {}", ranking_csv.display()))?;
    let mut rdr = ReaderBuilder::new().delimiter(b';').from_reader(file);

    let mut rows_seen = 0u64;
    let mut rows_inserted = 0u64;
    let mut rows_skipped = 0u64;
    let mut batch: Vec<RankingEventRow> = Vec::with_capacity(1024);
    for result in rdr.deserialize::<RankingRow>() {
        rows_seen += 1;
        let row = result.context("read ranking row")?;
        match ranking_row_to_event_row(&row) {
            Ok(event) => batch.push(event),
            Err(_) => {
                rows_skipped += 1;
                continue;
            }
        }

        if batch.len() >= 1024 {
            rows_inserted += store.insert_events(&batch).context("insert mysql events")?;
            batch.clear();
        }
    }

    if !batch.is_empty() {
        rows_inserted += store.insert_events(&batch).context("insert mysql events")?;
    }

    let rows_deduped = ranking_rows_deduped(rows_seen, rows_inserted, rows_skipped);
    let run_notes = format!(
        "source=ranking_csv skipped={} input_sha256={}",
        rows_skipped, input_sha256
    );
    store
        .finish_ingest_run(
            ingest_run_id,
            rows_seen,
            rows_inserted,
            rows_deduped,
            Some(&run_notes),
        )
        .context("finish ingest run")?;

    println!(
        "import-ranking: seen={} inserted={} deduped={} skipped={} map_version={}",
        rows_seen, rows_inserted, rows_deduped, rows_skipped, map_version
    );
    Ok(())
}

fn ranking_rows_deduped(rows_seen: u64, rows_inserted: u64, rows_skipped: u64) -> u64 {
    rows_seen
        .saturating_sub(rows_skipped)
        .saturating_sub(rows_inserted)
}

fn ranking_row_to_event_row(row: &RankingRow) -> Result<RankingEventRow> {
    let ts_utc_epoch = parse_datetime_utc(&row.date)?;
    let ts_utc = epoch_to_mysql_datetime(ts_utc_epoch)?;
    let world_x = row.x.round() as i32;
    let world_y = row.y.round() as i32;
    let world_z = row.z.round() as i32;
    let pixel = world_to_pixel_round(row.x, row.z);
    let map_px_x = pixel.x;
    let map_px_y = pixel.y;
    let length_milli = ((row.length.max(0.0)) * 1000.0).round() as i32;
    let event_uid = ranking_event_uid(
        ts_utc_epoch,
        row.encyclopedia_key,
        length_milli,
        world_x,
        world_y,
        world_z,
    );

    Ok(RankingEventRow {
        event_uid,
        source_kind: SOURCE_KIND_RANKING,
        source_id: None,
        ts_utc,
        fish_id: row.encyclopedia_key,
        length_milli,
        world_x,
        world_y,
        world_z,
        map_px_x,
        map_px_y,
        snap_px_x: map_px_x,
        snap_px_y: map_px_y,
        snap_dist_px: 0,
        water_ok: true,
    })
}

fn ranking_event_uid(
    ts_utc_epoch: i64,
    fish_id: i32,
    length_milli: i32,
    world_x: i32,
    world_y: i32,
    world_z: i32,
) -> String {
    // Normalize to second / millimeter / integer world-unit precision so reruns
    // of equivalent ranking exports collapse to one canonical event id.
    let mut hasher = Sha256::new();
    hasher.update(ts_utc_epoch.to_le_bytes());
    hasher.update(fish_id.to_le_bytes());
    hasher.update(length_milli.to_le_bytes());
    hasher.update(world_x.to_le_bytes());
    hasher.update(world_y.to_le_bytes());
    hasher.update(world_z.to_le_bytes());
    let digest = hasher.finalize();
    to_hex(&digest[..16])
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn epoch_to_mysql_datetime(ts_utc: i64) -> Result<String> {
    let dt = Utc
        .timestamp_opt(ts_utc, 0)
        .single()
        .ok_or_else(|| anyhow::anyhow!("invalid UTC timestamp: {ts_utc}"))?;
    Ok(dt.format("%Y-%m-%d %H:%M:%S%.6f").to_string())
}

fn sha256_file(path: &PathBuf) -> Result<String> {
    let mut file = File::open(path).with_context(|| format!("open file: {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("read file: {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(to_hex(&hasher.finalize()))
}

fn run_index_water_tiles(
    watermap: Option<PathBuf>,
    ignore_watermap: bool,
    tile_px: i32,
    db: PathBuf,
    water_xform: WatermapTransformArgs,
    config: Option<&fishystuff_config::Config>,
) -> Result<()> {
    let mut store = SqliteStore::open(&db).context("open db")?;

    if tile_px <= 0 {
        bail!("tile_px must be > 0");
    }

    let (tiles_x, tiles_y, counts) = if ignore_watermap {
        full_map_tile_counts(tile_px)
    } else {
        let water = load_water_sampler(watermap, &water_xform, config)?;
        let (tiles_x, tiles_y) = tile_dimensions(MAP_WIDTH, MAP_HEIGHT, tile_px);
        let mut counts = vec![0i32; (tiles_x * tiles_y) as usize];
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                if water.is_water_at_map_px(x, y) {
                    let (tx, ty) = pixel_to_tile(x, y, tile_px);
                    let idx = (ty * tiles_x + tx) as usize;
                    counts[idx] += 1;
                }
            }
        }
        (tiles_x, tiles_y, counts)
    };

    let mut tiles = Vec::with_capacity(counts.len());
    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let idx = (ty * tiles_x + tx) as usize;
            tiles.push(WaterTile {
                tile_px,
                tile_x: tx,
                tile_y: ty,
                water_count: counts[idx],
            });
        }
    }

    store
        .upsert_water_tiles(&tiles)
        .context("store water tiles")?;
    Ok(())
}

fn run_index_water_tiles_mysql(
    map_version: Option<String>,
    watermap: Option<PathBuf>,
    ignore_watermap: bool,
    tile_px: i32,
    water_xform: WatermapTransformArgs,
    config: Option<&fishystuff_config::Config>,
) -> Result<()> {
    let database_url = resolve_database_url()?;
    let map_version = resolve_map_version(map_version, config)?;
    let store = MySqlIngestStore::open(&database_url).context("open mysql store")?;

    if tile_px <= 0 {
        bail!("tile_px must be > 0");
    }

    let (tiles_x, tiles_y, counts) = if ignore_watermap {
        full_map_tile_counts(tile_px)
    } else {
        let water = load_water_sampler(watermap, &water_xform, config)?;
        let (tiles_x, tiles_y) = tile_dimensions(MAP_WIDTH, MAP_HEIGHT, tile_px);
        let mut counts = vec![0i32; (tiles_x * tiles_y) as usize];
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                if water.is_water_at_map_px(x, y) {
                    let (tx, ty) = pixel_to_tile(x, y, tile_px);
                    let idx = (ty * tiles_x + tx) as usize;
                    counts[idx] += 1;
                }
            }
        }
        (tiles_x, tiles_y, counts)
    };

    let mut tiles = Vec::with_capacity(counts.len());
    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let idx = (ty * tiles_x + tx) as usize;
            tiles.push(WaterTile {
                tile_px,
                tile_x: tx,
                tile_y: ty,
                water_count: counts[idx],
            });
        }
    }

    store
        .upsert_water_tiles(&map_version, &tiles)
        .context("store mysql water tiles")?;
    println!(
        "index-water-tiles-mysql: map_version={} tile_px={} tiles={}",
        map_version,
        tile_px,
        tiles.len()
    );
    Ok(())
}

fn run_index_zone_mask(
    db: PathBuf,
    map_version: String,
    zone_mask: PathBuf,
    overwrite: bool,
) -> Result<()> {
    if map_version.trim().is_empty() {
        bail!("map_version must be non-empty");
    }
    let mask = ZoneMask::load_png(&zone_mask).context("load zone mask")?;
    validate_zonemask_dims(&mask)?;
    let mut store = SqliteStore::open(&db).context("open db")?;

    let summary = index_zone_mask_with_mask(&mut store, &map_version, &mask, overwrite)?;
    println!(
        "index-zone-mask: total={} assigned={} skipped={} elapsed_ms={}",
        summary.total, summary.assigned, summary.skipped, summary.elapsed_ms
    );
    Ok(())
}

fn run_index_zone_mask_mysql(
    map_version: Option<String>,
    zone_mask_root: PathBuf,
    batch_size: usize,
    config: Option<&fishystuff_config::Config>,
) -> Result<()> {
    let layer_revision_id = resolve_map_version(map_version, config)?;
    run_build_event_zone_assignment_mysql(
        layer_revision_id,
        "zone_mask".to_string(),
        zone_mask_root,
        batch_size,
    )
}

fn run_build_event_zone_assignment_mysql(
    layer_revision_id: String,
    layer_id: String,
    zone_mask_root: PathBuf,
    batch_size: usize,
) -> Result<()> {
    if batch_size == 0 {
        bail!("batch_size must be > 0");
    }
    if layer_revision_id.trim().is_empty() {
        bail!("layer_revision_id must be non-empty");
    }
    if layer_id.trim().is_empty() {
        bail!("layer_id must be non-empty");
    }
    let database_url = resolve_database_url()?;
    let zone_mask = resolve_zone_mask_path(&zone_mask_root, &layer_revision_id)?;
    let mask = ZoneMask::load_png(&zone_mask).context("load zone mask")?;
    validate_zonemask_dims(&mask)?;
    let store = MySqlIngestStore::open(&database_url).context("open mysql store")?;
    store
        .ensure_layer_revision(&layer_revision_id, &layer_id)
        .context("ensure layer revision row")?;

    let start = std::time::Instant::now();
    let mut total = 0usize;
    let mut assigned_total = 0u64;
    let mut ring_support_total = 0u64;
    loop {
        let samples = store
            .load_events_missing_zone_support(&layer_revision_id, batch_size)
            .context("load events missing zone support rows")?;
        if samples.is_empty() {
            break;
        }
        total += samples.len();
        let mut rows = Vec::with_capacity(samples.len());
        let mut ring_rows = Vec::new();
        for sample in samples {
            let zone_rgb = mask.sample_rgb_u32_clamped(
                sample.assignment_sample_px_x,
                sample.assignment_sample_px_y,
            );
            rows.push(EventZoneInsertRow {
                event_id: sample.event_id,
                zone_rgb,
                sample_px_x: sample.assignment_sample_px_x,
                sample_px_y: sample.assignment_sample_px_y,
            });
            ring_rows.extend(build_event_zone_ring_support_rows(
                &mask,
                sample.event_id,
                sample.ring_center_px_x,
                sample.ring_center_px_y,
            ));
        }
        let assigned = store
            .insert_event_zones(&layer_revision_id, &rows)
            .context("insert event zone assignments")?;
        let ring_assigned = store
            .insert_event_zone_ring_support(&layer_revision_id, &ring_rows)
            .context("insert event zone ring support rows")?;
        assigned_total += assigned;
        ring_support_total += ring_assigned;
        if rows.len() < batch_size {
            break;
        }
    }
    let skipped = total.saturating_sub(assigned_total as usize);
    println!(
        "build-event-zone-assignment: layer_revision_id={} layer_id={} total={} assigned={} ring_support={} skipped={} elapsed_ms={}",
        layer_revision_id,
        layer_id,
        total,
        assigned_total,
        ring_support_total,
        skipped,
        start.elapsed().as_millis()
    );
    Ok(())
}

fn build_event_zone_ring_support_rows(
    mask: &ZoneMask,
    event_id: i64,
    ring_center_px_x: i32,
    ring_center_px_y: i32,
) -> Vec<EventZoneRingSupportInsertRow> {
    let touched_zones = ranking_ring_zone_overlaps(mask, ring_center_px_x, ring_center_px_y);
    if touched_zones.is_empty() {
        return Vec::new();
    }
    let fully_contained = touched_zones.len() == 1;
    touched_zones
        .into_iter()
        .map(|zone_rgb| EventZoneRingSupportInsertRow {
            event_id,
            zone_rgb,
            ring_fully_contained: fully_contained,
            ring_center_px_x,
            ring_center_px_y,
        })
        .collect()
}

fn ranking_ring_zone_overlaps(
    mask: &ZoneMask,
    ring_center_px_x: i32,
    ring_center_px_y: i32,
) -> Vec<u32> {
    let radius_px = RANKING_RING_RADIUS_WORLD_UNITS / DISTANCE_PER_PIXEL;
    let mut zones = BTreeSet::new();
    for idx in 0..RANKING_RING_SAMPLE_COUNT {
        let theta = (idx as f64 / RANKING_RING_SAMPLE_COUNT as f64) * std::f64::consts::TAU;
        let sample_px_x = ring_center_px_x as f64 + radius_px * theta.cos();
        let sample_px_y = ring_center_px_y as f64 + radius_px * theta.sin();
        let zone_rgb =
            mask.sample_rgb_u32_clamped(sample_px_x.round() as i32, sample_px_y.round() as i32);
        zones.insert(zone_rgb);
    }
    zones.into_iter().collect()
}

fn resolve_zone_mask_path(zone_mask_root: &Path, map_version: &str) -> Result<PathBuf> {
    if zone_mask_root.is_file() {
        return Ok(zone_mask_root.to_path_buf());
    }

    let candidates = [
        zone_mask_root.join(format!("zones_mask_{map_version}.png")),
        zone_mask_root.join(format!("{map_version}.png")),
        zone_mask_root.join("zones_mask.png"),
    ];
    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    bail!(
        "zone mask not found under {} (looked for zones_mask_{}.png, {}.png, zones_mask.png)",
        zone_mask_root.display(),
        map_version,
        map_version
    )
}

fn run_import_region_groups_mysql(
    map_version: Option<String>,
    geojson: PathBuf,
    regioninfo_bss: PathBuf,
    regiongroupinfo_bss: PathBuf,
    source: String,
    config: Option<&fishystuff_config::Config>,
) -> Result<()> {
    let database_url = resolve_database_url()?;
    let map_version = resolve_map_version(map_version, config)?;
    let store = MySqlIngestStore::open(&database_url).context("open mysql store")?;

    let (meta_rows, region_rows) = region_groups::load_region_group_inputs(
        &geojson,
        &regioninfo_bss,
        &regiongroupinfo_bss,
        &source,
    )?;
    store
        .replace_region_groups(&map_version, &meta_rows, &region_rows)
        .context("replace region-group metadata")?;

    println!(
        "import-region-groups-mysql: map_version={} groups={} regions={} source={}",
        map_version,
        meta_rows.len(),
        region_rows.len(),
        source.trim()
    );
    Ok(())
}

fn run_build_detailed_regions_geojson(
    regions_geojson: PathBuf,
    regioninfo_bss: PathBuf,
    regiongroupinfo_bss: PathBuf,
    loc: PathBuf,
    waypoint_xml: Vec<PathBuf>,
    out: PathBuf,
) -> Result<()> {
    let summary = region_layers::build_detailed_regions_geojson(
        &regions_geojson,
        &loc,
        &regioninfo_bss,
        &regiongroupinfo_bss,
        &waypoint_xml,
        &out,
    )?;
    println!(
        "build-detailed-regions-geojson: out={} features={} named={} resource_waypoints={}",
        out.display(),
        summary.feature_count,
        summary.named_feature_count,
        summary.resource_feature_count,
    );
    Ok(())
}

fn run_build_region_groups_geojson(
    region_groups_geojson: PathBuf,
    regioninfo_bss: PathBuf,
    regiongroupinfo_bss: PathBuf,
    loc: PathBuf,
    waypoint_xml: Vec<PathBuf>,
    out: PathBuf,
) -> Result<()> {
    let summary = region_layers::build_region_groups_geojson(
        &region_groups_geojson,
        &loc,
        &regioninfo_bss,
        &regiongroupinfo_bss,
        &waypoint_xml,
        &out,
    )?;
    println!(
        "build-region-groups-geojson: out={} features={} resource_waypoints={}",
        out.display(),
        summary.feature_count,
        summary.resource_feature_count,
    );
    Ok(())
}

fn run_build_region_nodes_geojson(
    regioninfo_bss: PathBuf,
    regiongroupinfo_bss: PathBuf,
    loc: PathBuf,
    waypoint_xml: Vec<PathBuf>,
    out: PathBuf,
) -> Result<()> {
    let summary = region_layers::build_region_nodes_geojson(
        &loc,
        &regioninfo_bss,
        &regiongroupinfo_bss,
        &waypoint_xml,
        &out,
    )?;
    println!(
        "build-region-nodes-geojson: out={} features={} named={} connections={}",
        out.display(),
        summary.feature_count,
        summary.named_feature_count,
        summary.connection_feature_count,
    );
    Ok(())
}

fn run_build_regions_field_metadata(
    field: PathBuf,
    regioninfo_bss: PathBuf,
    regiongroupinfo_bss: PathBuf,
    loc: PathBuf,
    waypoint_xml: Vec<PathBuf>,
    out: PathBuf,
) -> Result<()> {
    let summary = field_layers::build_regions_field_hover_metadata(
        &field,
        &loc,
        &regioninfo_bss,
        &regiongroupinfo_bss,
        &waypoint_xml,
        &out,
    )?;
    println!(
        "build-regions-field-metadata: out={} field_ids={} entries={}",
        out.display(),
        summary.field_id_count,
        summary.entry_count,
    );
    Ok(())
}

fn run_build_region_groups_field_metadata(
    field: PathBuf,
    regions_field: PathBuf,
    regioninfo_bss: PathBuf,
    regiongroupinfo_bss: PathBuf,
    loc: PathBuf,
    waypoint_xml: Vec<PathBuf>,
    out: PathBuf,
) -> Result<()> {
    let summary = field_layers::build_region_groups_field_hover_metadata(
        &field,
        &regions_field,
        &loc,
        &regioninfo_bss,
        &regiongroupinfo_bss,
        &waypoint_xml,
        &out,
    )?;
    println!(
        "build-region-groups-field-metadata: out={} field_ids={} entries={}",
        out.display(),
        summary.field_id_count,
        summary.entry_count,
    );
    Ok(())
}

fn run_build_zone_mask_field_metadata(field: PathBuf, out: PathBuf) -> Result<()> {
    let summary = field_layers::build_zone_mask_field_hover_metadata(&field, &out)?;
    println!(
        "build-zone-mask-field-metadata: out={} field_ids={} entries={}",
        out.display(),
        summary.field_id_count,
        summary.entry_count,
    );
    Ok(())
}

fn validate_zonemask_dims(mask: &ZoneMask) -> Result<()> {
    if mask.width() as i32 != MAP_WIDTH || mask.height() as i32 != MAP_HEIGHT {
        bail!(
            "zone mask dimensions mismatch: got {}x{}, expected {}x{}",
            mask.width(),
            mask.height(),
            MAP_WIDTH,
            MAP_HEIGHT
        );
    }
    Ok(())
}

fn full_map_tile_counts(tile_px: i32) -> (i32, i32, Vec<i32>) {
    let (tiles_x, tiles_y) = tile_dimensions(MAP_WIDTH, MAP_HEIGHT, tile_px);
    let mut counts = vec![0i32; (tiles_x * tiles_y) as usize];
    for ty in 0..tiles_y {
        let y0 = ty * tile_px;
        let y1 = (y0 + tile_px).min(MAP_HEIGHT);
        let h = (y1 - y0).max(0);
        for tx in 0..tiles_x {
            let x0 = tx * tile_px;
            let x1 = (x0 + tile_px).min(MAP_WIDTH);
            let w = (x1 - x0).max(0);
            counts[(ty * tiles_x + tx) as usize] = w * h;
        }
    }
    (tiles_x, tiles_y, counts)
}

#[derive(Debug, Clone)]
struct WatermapTransformArgs {
    kind: Option<String>,
    sx: Option<f64>,
    sy: Option<f64>,
    ox: Option<f64>,
    oy: Option<f64>,
    world_left: Option<f64>,
    world_right: Option<f64>,
    world_bottom: Option<f64>,
    world_top: Option<f64>,
    map_pixel_center_offset: Option<f64>,
}

impl WatermapTransformArgs {
    fn new(
        kind: Option<String>,
        sx: Option<f64>,
        sy: Option<f64>,
        ox: Option<f64>,
        oy: Option<f64>,
    ) -> Self {
        Self {
            kind,
            sx,
            sy,
            ox,
            oy,
            world_left: None,
            world_right: None,
            world_bottom: None,
            world_top: None,
            map_pixel_center_offset: None,
        }
    }
}

fn load_water_sampler(
    watermap: Option<PathBuf>,
    args: &WatermapTransformArgs,
    config: Option<&fishystuff_config::Config>,
) -> Result<WaterSampler> {
    let cfg_paths = config.map(|c| &c.paths);
    let cfg_water = config.map(|c| &c.watermap);

    let path = watermap
        .or_else(|| cfg_water.and_then(|w| w.path.as_ref().map(PathBuf::from)))
        .or_else(|| cfg_paths.and_then(|p| p.watermap.as_ref().map(PathBuf::from)))
        .ok_or_else(|| anyhow::anyhow!("watermap path required"))?;

    let img = image::ImageReader::open(&path)
        .with_context(|| format!("open watermap: {}", path.display()))?
        .with_guessed_format()
        .context("guess watermap format")?
        .decode()
        .context("decode watermap")?
        .into_rgb8();
    let water_w = img.width();
    let water_h = img.height();

    // Canonical projected watermap matches map pixel space exactly.
    // In that case, always use identity map-space sampling and ignore affine-like overrides.
    if water_w == MAP_WIDTH as u32 && water_h == MAP_HEIGHT as u32 {
        return Ok(WaterSampler::from_image(
            img,
            TransformKind::ScaleToFit {
                map_w: MAP_WIDTH as u32,
                map_h: MAP_HEIGHT as u32,
                water_w,
                water_h,
            },
        ));
    }

    let mut kind = args
        .kind
        .as_deref()
        .map(str::to_lowercase)
        .or_else(|| cfg_water.and_then(|w| w.transform.kind.clone()))
        .unwrap_or_else(|| "scale_to_fit".to_string());
    kind = kind.to_lowercase();
    let transform = build_transform(
        &kind,
        args,
        cfg_water.map(|w| &w.transform),
        water_w,
        water_h,
    )?;
    Ok(WaterSampler::from_image(img, transform))
}

fn build_transform(
    kind: &str,
    args: &WatermapTransformArgs,
    cfg: Option<&fishystuff_config::WatermapTransform>,
    water_w: u32,
    water_h: u32,
) -> Result<TransformKind> {
    match kind {
        "scale_to_fit" | "scaletofit" | "scale-to-fit" => Ok(TransformKind::ScaleToFit {
            map_w: MAP_WIDTH as u32,
            map_h: MAP_HEIGHT as u32,
            water_w,
            water_h,
        }),
        "scale_offset" | "scaleoffset" | "scale-offset" => {
            let sx = args
                .sx
                .or_else(|| cfg.and_then(|c| c.sx))
                .ok_or_else(|| anyhow::anyhow!("watermap sx required for scale_offset"))?;
            let sy = args
                .sy
                .or_else(|| cfg.and_then(|c| c.sy))
                .ok_or_else(|| anyhow::anyhow!("watermap sy required for scale_offset"))?;
            let ox = args.ox.or_else(|| cfg.and_then(|c| c.ox)).unwrap_or(0.0);
            let oy = args.oy.or_else(|| cfg.and_then(|c| c.oy)).unwrap_or(0.0);
            Ok(TransformKind::ScaleOffset { sx, sy, ox, oy })
        }
        "world_extent" | "worldextent" | "world-extent" => {
            let world_left = args
                .world_left
                .or_else(|| cfg.and_then(|c| c.world_left))
                .ok_or_else(|| anyhow::anyhow!("watermap world_left required for world_extent"))?;
            let world_right = args
                .world_right
                .or_else(|| cfg.and_then(|c| c.world_right))
                .ok_or_else(|| anyhow::anyhow!("watermap world_right required for world_extent"))?;
            let world_bottom = args
                .world_bottom
                .or_else(|| cfg.and_then(|c| c.world_bottom))
                .ok_or_else(|| {
                    anyhow::anyhow!("watermap world_bottom required for world_extent")
                })?;
            let world_top = args
                .world_top
                .or_else(|| cfg.and_then(|c| c.world_top))
                .ok_or_else(|| anyhow::anyhow!("watermap world_top required for world_extent"))?;
            let map_pixel_center_offset = args
                .map_pixel_center_offset
                .or_else(|| cfg.and_then(|c| c.map_pixel_center_offset))
                .unwrap_or(1.0);
            Ok(TransformKind::WorldExtent {
                world_left,
                world_right,
                world_bottom,
                world_top,
                map_pixel_center_offset,
                water_w,
                water_h,
            })
        }
        _ => bail!("unknown watermap transform kind: {}", kind),
    }
}

fn resolve_database_url() -> Result<String> {
    load_api_database_url_from_secretspec()
        .context("resolve database URL from SecretSpec `api` profile")
}

fn resolve_map_version(
    map_version: Option<String>,
    config: Option<&fishystuff_config::Config>,
) -> Result<String> {
    map_version
        .or_else(|| config.and_then(|cfg| cfg.defaults.map_version.clone()))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "map_version is required (pass --map-version or set defaults.map_version in config)"
            )
        })
}

fn run_debug_watermap_projection(
    watermap: Option<PathBuf>,
    out: PathBuf,
    landmarks_csv: Option<PathBuf>,
    projection_mode: String,
    mut water_xform: WatermapTransformArgs,
    config: Option<&fishystuff_config::Config>,
) -> Result<()> {
    if let Some(landmarks_csv) = landmarks_csv {
        let fit = fit_scale_offset_from_landmarks_csv(&landmarks_csv)?;
        eprintln!(
            "watermap landmark fit from {}: sx={:.12}, sy={:.12}, ox={:.6}, oy={:.6}, rmse={:.3}px, max_err={:.3}px (n={})",
            landmarks_csv.display(),
            fit.sx,
            fit.sy,
            fit.ox,
            fit.oy,
            fit.rmse,
            fit.max_error,
            fit.count
        );
        eprintln!(
            "watermap landmark affine map->water: a={:.12}, b={:.12}, tx={:.6}, c={:.12}, d={:.12}, ty={:.6}, rmse={:.3}px, max_err={:.3}px",
            fit.affine_a,
            fit.affine_b,
            fit.affine_tx,
            fit.affine_c,
            fit.affine_d,
            fit.affine_ty,
            fit.affine_rmse,
            fit.affine_max_error
        );
        water_xform.kind = Some("scale_offset".to_string());
        water_xform.sx = Some(fit.sx);
        water_xform.sy = Some(fit.sy);
        water_xform.ox = Some(fit.ox);
        water_xform.oy = Some(fit.oy);
        water_xform.world_left = None;
        water_xform.world_right = None;
        water_xform.world_bottom = None;
        water_xform.world_top = None;
        water_xform.map_pixel_center_offset = None;
    }

    let sampler = load_water_sampler(watermap, &water_xform, config)?;
    let mut img = image::RgbImage::new(MAP_WIDTH as u32, MAP_HEIGHT as u32);
    let mode = projection_mode.trim().to_lowercase();
    let binary_mode = match mode.as_str() {
        "rgb" => false,
        "binary" | "mask" => true,
        _ => bail!(
            "unknown projection_mode: {} (expected rgb|binary)",
            projection_mode
        ),
    };
    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let pixel = if binary_mode {
                if sampler.is_water_at_map_px(x, y) {
                    image::Rgb([0, 0, 255])
                } else {
                    image::Rgb([0, 0, 0])
                }
            } else {
                image::Rgb(sampler.sample_rgb_bilinear_at_map_px(x as f64, y as f64))
            };
            img.put_pixel(x as u32, y as u32, pixel);
        }
    }
    img.save(&out)
        .with_context(|| format!("save projected watermap: {}", out.display()))?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct WatermapLandmark {
    map_x: f64,
    map_y: f64,
    water_x: f64,
    water_y: f64,
}

#[derive(Debug, Clone, Copy)]
struct ScaleOffsetFit {
    sx: f64,
    sy: f64,
    ox: f64,
    oy: f64,
    rmse: f64,
    max_error: f64,
    count: usize,
    affine_a: f64,
    affine_b: f64,
    affine_tx: f64,
    affine_c: f64,
    affine_d: f64,
    affine_ty: f64,
    affine_rmse: f64,
    affine_max_error: f64,
}

fn fit_scale_offset_from_landmarks_csv(path: &PathBuf) -> Result<ScaleOffsetFit> {
    let points = load_watermap_landmarks(path)?;
    if points.len() < 2 {
        bail!(
            "landmarks csv requires at least 2 points, got {}",
            points.len()
        );
    }

    let (sx, ox) = fit_linear_axis(points.iter().map(|p| (p.map_x, p.water_x)))?;
    let (sy, oy) = fit_linear_axis(points.iter().map(|p| (p.map_y, p.water_y)))?;
    let (affine_a, affine_b, affine_tx, affine_c, affine_d, affine_ty) =
        fit_affine_map_to_water(&points)?;

    let (rmse, max_error) = map_to_water_residuals(&points, sx, 0.0, ox, 0.0, sy, oy);
    let (affine_rmse, affine_max_error) = map_to_water_residuals(
        &points, affine_a, affine_b, affine_tx, affine_c, affine_d, affine_ty,
    );

    Ok(ScaleOffsetFit {
        sx,
        sy,
        ox,
        oy,
        rmse,
        max_error,
        count: points.len(),
        affine_a,
        affine_b,
        affine_tx,
        affine_c,
        affine_d,
        affine_ty,
        affine_rmse,
        affine_max_error,
    })
}

fn load_watermap_landmarks(path: &PathBuf) -> Result<Vec<WatermapLandmark>> {
    let mut rdr = ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(path)
        .with_context(|| format!("open landmarks csv: {}", path.display()))?;

    let headers = rdr.headers().context("read landmarks headers")?.clone();
    let map_x_idx = header_index_optional(&headers, "map_x");
    let map_y_idx = header_index_optional(&headers, "map_y");
    let water_x_idx = header_index_optional(&headers, "water_x");
    let water_y_idx = header_index_optional(&headers, "water_y");

    let tile_name_idx = header_index_optional(&headers, "tilename");
    let minimap_x_idx = header_index_optional(&headers, "minimap_tile_x");
    let minimap_y_idx = header_index_optional(&headers, "minimap_tile_y");
    let watermap_x_idx = header_index_optional(&headers, "watermap_x");
    let watermap_y_idx = header_index_optional(&headers, "watermap_y");

    let mut points = Vec::new();
    if let (Some(map_x_idx), Some(map_y_idx), Some(water_x_idx), Some(water_y_idx)) =
        (map_x_idx, map_y_idx, water_x_idx, water_y_idx)
    {
        for (row_idx, rec) in rdr.records().enumerate() {
            let rec = rec.with_context(|| format!("read landmarks row {}", row_idx + 2))?;
            points.push(WatermapLandmark {
                map_x: parse_landmark_value(&rec, map_x_idx, row_idx + 2, "map_x")?,
                map_y: parse_landmark_value(&rec, map_y_idx, row_idx + 2, "map_y")?,
                water_x: parse_landmark_value(&rec, water_x_idx, row_idx + 2, "water_x")?,
                water_y: parse_landmark_value(&rec, water_y_idx, row_idx + 2, "water_y")?,
            });
        }
        return Ok(points);
    }

    if let (
        Some(tile_name_idx),
        Some(minimap_x_idx),
        Some(minimap_y_idx),
        Some(watermap_x_idx),
        Some(watermap_y_idx),
    ) = (
        tile_name_idx,
        minimap_x_idx,
        minimap_y_idx,
        watermap_x_idx,
        watermap_y_idx,
    ) {
        for (row_idx, rec) in rdr.records().enumerate() {
            let row_number = row_idx + 2;
            let rec = rec.with_context(|| format!("read landmarks row {}", row_number))?;
            let tile_name = rec
                .get(tile_name_idx)
                .ok_or_else(|| anyhow::anyhow!("row {} missing tilename", row_number))?;
            let (tile_x, tile_y) = parse_minimap_tile_name(tile_name, row_number)?;
            let minimap_x =
                parse_landmark_value(&rec, minimap_x_idx, row_number, "minimap_tile_x")?;
            let minimap_y =
                parse_landmark_value(&rec, minimap_y_idx, row_number, "minimap_tile_y")?;
            let (map_x, map_y) = minimap_tile_local_to_map_px(tile_x, tile_y, minimap_x, minimap_y);
            points.push(WatermapLandmark {
                map_x,
                map_y,
                water_x: parse_landmark_value(&rec, watermap_x_idx, row_number, "watermap_x")?,
                water_y: parse_landmark_value(&rec, watermap_y_idx, row_number, "watermap_y")?,
            });
        }
        return Ok(points);
    }

    bail!(
        "unsupported landmarks csv format in {} (expected map_x/map_y/water_x/water_y or tilename/minimap_tile_x/minimap_tile_y/watermap_x/watermap_y)",
        path.display()
    );
}

fn parse_minimap_tile_name(tile_name: &str, row_number: usize) -> Result<(i32, i32)> {
    let stem = tile_name
        .strip_suffix(".png")
        .ok_or_else(|| anyhow::anyhow!("row {} invalid tilename '{}'", row_number, tile_name))?;
    let rest = stem
        .strip_prefix("rader_")
        .ok_or_else(|| anyhow::anyhow!("row {} invalid tilename '{}'", row_number, tile_name))?;
    let (tile_x_raw, tile_y_raw) = rest
        .split_once('_')
        .ok_or_else(|| anyhow::anyhow!("row {} invalid tilename '{}'", row_number, tile_name))?;
    let tile_x = tile_x_raw.parse::<i32>().with_context(|| {
        format!(
            "row {} invalid tile x '{}' in '{}'",
            row_number, tile_x_raw, tile_name
        )
    })?;
    let tile_y = tile_y_raw.parse::<i32>().with_context(|| {
        format!(
            "row {} invalid tile y '{}' in '{}'",
            row_number, tile_y_raw, tile_name
        )
    })?;
    Ok((tile_x, tile_y))
}

fn minimap_tile_local_to_map_px(
    tile_x: i32,
    tile_y: i32,
    local_x: f64,
    local_y: f64,
) -> (f64, f64) {
    const TILE_PX: f64 = 128.0;
    let source_x = tile_x as f64 * TILE_PX + local_x;
    let source_y = tile_y as f64 * TILE_PX + (TILE_PX - 1.0 - local_y);
    let world_x = source_x * (SECTOR_SCALE / TILE_PX);
    let world_z = source_y * (SECTOR_SCALE / TILE_PX);
    world_to_pixel_f(world_x, world_z)
}

fn fit_affine_map_to_water(points: &[WatermapLandmark]) -> Result<(f64, f64, f64, f64, f64, f64)> {
    let (a, b, tx) = fit_affine_axis(points.iter().map(|p| (p.map_x, p.map_y, p.water_x)))?;
    let (c, d, ty) = fit_affine_axis(points.iter().map(|p| (p.map_x, p.map_y, p.water_y)))?;
    Ok((a, b, tx, c, d, ty))
}

fn fit_affine_axis(samples: impl Iterator<Item = (f64, f64, f64)>) -> Result<(f64, f64, f64)> {
    let mut n = 0.0_f64;
    let mut sum_x = 0.0_f64;
    let mut sum_y = 0.0_f64;
    let mut sum_xx = 0.0_f64;
    let mut sum_yy = 0.0_f64;
    let mut sum_xy = 0.0_f64;
    let mut sum_dx = 0.0_f64;
    let mut sum_dy = 0.0_f64;
    let mut sum_d = 0.0_f64;

    for (x, y, d) in samples {
        n += 1.0;
        sum_x += x;
        sum_y += y;
        sum_xx += x * x;
        sum_yy += y * y;
        sum_xy += x * y;
        sum_dx += d * x;
        sum_dy += d * y;
        sum_d += d;
    }
    if n < 3.0 {
        bail!("need at least 3 samples for affine least-squares fit");
    }

    let matrix = [
        [sum_xx, sum_xy, sum_x],
        [sum_xy, sum_yy, sum_y],
        [sum_x, sum_y, n],
    ];
    let rhs = [sum_dx, sum_dy, sum_d];
    solve_3x3(matrix, rhs)
}

fn solve_3x3(matrix: [[f64; 3]; 3], rhs: [f64; 3]) -> Result<(f64, f64, f64)> {
    let mut m = matrix;
    let mut b = rhs;
    for pivot in 0..3 {
        let mut best = pivot;
        let mut best_abs = m[pivot][pivot].abs();
        for (row, row_values) in m.iter().enumerate().skip(pivot + 1) {
            let value = row_values[pivot].abs();
            if value > best_abs {
                best = row;
                best_abs = value;
            }
        }
        if best_abs <= f64::EPSILON {
            bail!("degenerate landmarks for affine least-squares fit");
        }
        if best != pivot {
            m.swap(pivot, best);
            b.swap(pivot, best);
        }
        let pivot_value = m[pivot][pivot];
        for value in &mut m[pivot][pivot..] {
            *value /= pivot_value;
        }
        b[pivot] /= pivot_value;

        let pivot_row = m[pivot];
        for row in 0..3 {
            if row == pivot {
                continue;
            }
            let factor = m[row][pivot];
            for (value, pivot_value) in m[row][pivot..].iter_mut().zip(pivot_row[pivot..].iter()) {
                *value -= factor * *pivot_value;
            }
            b[row] -= factor * b[pivot];
        }
    }
    Ok((b[0], b[1], b[2]))
}

fn map_to_water_residuals(
    points: &[WatermapLandmark],
    a: f64,
    b: f64,
    tx: f64,
    c: f64,
    d: f64,
    ty: f64,
) -> (f64, f64) {
    let mut sq_sum = 0.0_f64;
    let mut max_error = 0.0_f64;
    for point in points {
        let pred_x = a * point.map_x + b * point.map_y + tx;
        let pred_y = c * point.map_x + d * point.map_y + ty;
        let err = ((pred_x - point.water_x).powi(2) + (pred_y - point.water_y).powi(2)).sqrt();
        sq_sum += err * err;
        max_error = max_error.max(err);
    }
    let rmse = (sq_sum / points.len() as f64).sqrt();
    (rmse, max_error)
}

fn header_index_optional(headers: &StringRecord, name: &str) -> Option<usize> {
    headers.iter().position(|h| h.eq_ignore_ascii_case(name))
}

fn parse_landmark_value(
    rec: &StringRecord,
    idx: usize,
    row_number: usize,
    column_name: &str,
) -> Result<f64> {
    let raw = rec
        .get(idx)
        .ok_or_else(|| anyhow::anyhow!("row {} missing {}", row_number, column_name))?;
    raw.parse::<f64>()
        .with_context(|| format!("row {} invalid {} value '{}'", row_number, column_name, raw))
}

fn fit_linear_axis(samples: impl Iterator<Item = (f64, f64)>) -> Result<(f64, f64)> {
    let mut n = 0.0_f64;
    let mut sum_x = 0.0_f64;
    let mut sum_y = 0.0_f64;
    let mut sum_xx = 0.0_f64;
    let mut sum_xy = 0.0_f64;

    for (x, y) in samples {
        n += 1.0;
        sum_x += x;
        sum_y += y;
        sum_xx += x * x;
        sum_xy += x * y;
    }

    if n < 2.0 {
        bail!("need at least 2 samples for least-squares fit");
    }

    let denom = n * sum_xx - sum_x * sum_x;
    if denom.abs() <= f64::EPSILON {
        bail!("degenerate landmarks for least-squares fit");
    }

    let slope = (n * sum_xy - sum_x * sum_y) / denom;
    let offset = (sum_y - slope * sum_x) / n;
    Ok((slope, offset))
}

fn run_zone_stats(command: ZoneStatsCommand) -> Result<()> {
    let ZoneStatsCommand {
        db,
        map_version,
        rgb,
        from_ts,
        to_ts,
        tile_px,
        sigma_tiles,
        fish_norm,
        alpha0,
        top_k,
        fish_names,
        dolt_repo,
        dolt_ref,
        half_life_days,
        drift_boundary_ts,
        config,
    } = command;
    let zone_rgb_u32 = parse_rgb_string(&rgb)?;
    let cfg_paths = config.as_ref().map(|c| &c.paths);
    let cfg_defaults = config.as_ref().map(|c| &c.defaults);

    let db = db
        .or_else(|| cfg_paths.and_then(|p| p.db.as_ref().map(PathBuf::from)))
        .ok_or_else(|| anyhow::anyhow!("--db is required (or config.paths.db)"))?;
    let fish_names = fish_names
        .or_else(|| cfg_paths.and_then(|p| p.fish_names.as_ref().map(PathBuf::from)))
        .ok_or_else(|| anyhow::anyhow!("--fish-names is required (or config.paths.fish_names)"))?;
    let map_version = map_version
        .or_else(|| cfg_defaults.and_then(|d| d.map_version.clone()))
        .ok_or_else(|| {
            anyhow::anyhow!("--map-version is required (or config.defaults.map_version)")
        })?;
    let tile_px = tile_px
        .or_else(|| cfg_defaults.and_then(|d| d.tile_px))
        .unwrap_or(32);
    let sigma_tiles = sigma_tiles
        .or_else(|| cfg_defaults.and_then(|d| d.sigma_tiles))
        .unwrap_or(3.0);
    let half_life_days = half_life_days.or_else(|| cfg_defaults.and_then(|d| d.half_life_days));

    let store = SqliteStore::open(&db).context("open db")?;
    let fish_names = load_fish_names(&fish_names)?;
    let zones_meta = load_zones_meta(
        dolt_repo.or_else(|| cfg_paths.and_then(|p| p.dolt_repo.as_ref().map(PathBuf::from))),
        dolt_ref.as_deref(),
    )?;
    let params = QueryParams {
        map_version,
        from_ts_utc: from_ts,
        to_ts_utc: to_ts,
        half_life_days,
        tile_px,
        sigma_tiles,
        fish_norm,
        alpha0,
        top_k,
        drift_boundary_ts,
    };
    let stats = if let Some(cfg) = &config {
        let mut status_cfg = ZoneStatusConfig::default();
        if let Some(v) = cfg.thresholds.stale_days {
            status_cfg.stale_days_threshold = v;
        }
        if let Some(v) = cfg.thresholds.ess {
            status_cfg.ess_threshold = v;
        }
        if let Some(v) = cfg.thresholds.drift_jsd {
            status_cfg.drift_jsd_threshold = v;
        }
        if let Some(v) = cfg.thresholds.drift_prob {
            status_cfg.drift_prob_threshold = v;
        }
        if let Some(v) = cfg.thresholds.drift_samples {
            status_cfg.drift_samples = v;
        }
        if let Some(v) = cfg.thresholds.drift_min_ess {
            status_cfg.drift_min_ess = v;
        }
        compute_zone_stats_with_config(
            &store,
            &zones_meta,
            &fish_names,
            &params,
            zone_rgb_u32,
            &status_cfg,
        )
        .context("compute zone stats")?
    } else {
        compute_zone_stats(&store, &zones_meta, &fish_names, &params, zone_rgb_u32)
            .context("compute zone stats")?
    };
    let json = zone_stats_to_json(&stats);
    println!("{json}");
    Ok(())
}

fn load_fish_names(path: &PathBuf) -> Result<std::collections::HashMap<i32, String>> {
    let data = std::fs::read_to_string(path)
        .with_context(|| format!("read fish names: {}", path.display()))?;
    let mut out = std::collections::HashMap::new();
    for (idx, line) in data.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut parts = line.split('\t');
        let id = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("missing fish_id on line {}", idx + 1))?;
        let name = parts
            .next()
            .ok_or_else(|| anyhow::anyhow!("missing fish_name on line {}", idx + 1))?;
        let fish_id: i32 = id
            .trim()
            .parse()
            .with_context(|| format!("parse fish_id on line {}", idx + 1))?;
        out.insert(fish_id, name.trim().to_string());
    }
    Ok(out)
}

fn load_zones_meta(
    dolt_repo: Option<PathBuf>,
    dolt_ref: Option<&str>,
) -> Result<std::collections::HashMap<u32, fishystuff_zones_meta::ZoneMeta>> {
    if let Some(repo) = dolt_repo {
        let provider = DoltZonesMetaProvider::new(repo);
        return provider.load(dolt_ref);
    }
    bail!("zones metadata not provided: pass --dolt-repo");
}

fn parse_rgb_string(input: &str) -> Result<u32> {
    let mut parts = input.split(',');
    let r: u8 = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("rgb missing red"))?
        .trim()
        .parse()
        .context("parse red")?;
    let g: u8 = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("rgb missing green"))?
        .trim()
        .parse()
        .context("parse green")?;
    let b: u8 = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("rgb missing blue"))?
        .trim()
        .parse()
        .context("parse blue")?;
    Ok(pack_rgb_u32(r, g, b))
}

// JSON helper lives in fishystuff_analytics.

struct IndexSummary {
    total: usize,
    assigned: usize,
    skipped: usize,
    elapsed_ms: u128,
}

fn index_zone_mask_with_mask(
    store: &mut SqliteStore,
    map_version: &str,
    mask: &ZoneMask,
    overwrite: bool,
) -> Result<IndexSummary> {
    let start = std::time::Instant::now();
    let events = store.load_water_events().context("load water events")?;
    let total = events.len();
    let mut rows = Vec::with_capacity(total);
    for ev in events {
        let rgb = mask.sample_rgb_u32_clamped(ev.water_px, ev.water_py);
        rows.push((ev.id, rgb));
    }
    let assigned = store
        .insert_event_zones(map_version, &rows, overwrite)
        .context("insert event zones")?;
    let skipped = total.saturating_sub(assigned);
    Ok(IndexSummary {
        total,
        assigned,
        skipped,
        elapsed_ms: start.elapsed().as_millis(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fishystuff_core::masks::pack_rgb_u32;

    fn set_pixel(data: &mut [u8], width: u32, px: u32, py: u32, r: u8, g: u8, b: u8) {
        let idx = ((py * width + px) * 3) as usize;
        data[idx] = r;
        data[idx + 1] = g;
        data[idx + 2] = b;
    }

    #[test]
    fn index_zone_mask_assigns_rgb() {
        let mut store = SqliteStore::open_in_memory().expect("db");
        let events = vec![
            Event {
                ts_utc: 0,
                fish_id: 1,
                world_x: 0.0,
                world_z: 0.0,
                px: Some(0),
                py: Some(0),
                water_px: Some(1),
                water_py: Some(1),
                tile_x: None,
                tile_y: None,
                water_ok: true,
            },
            Event {
                ts_utc: 0,
                fish_id: 2,
                world_x: 0.0,
                world_z: 0.0,
                px: Some(0),
                py: Some(0),
                water_px: Some(2),
                water_py: Some(2),
                tile_x: None,
                tile_y: None,
                water_ok: true,
            },
            Event {
                ts_utc: 0,
                fish_id: 3,
                world_x: 0.0,
                world_z: 0.0,
                px: Some(0),
                py: Some(0),
                water_px: None,
                water_py: None,
                tile_x: None,
                tile_y: None,
                water_ok: false,
            },
        ];
        store.insert_events(&events).expect("insert events");

        let width = 4u32;
        let height = 4u32;
        let mut data = vec![0u8; (width * height * 3) as usize];
        set_pixel(&mut data, width, 1, 1, 255, 0, 0);
        set_pixel(&mut data, width, 2, 2, 0, 255, 0);
        let mask = ZoneMask::from_rgb(width, height, data).expect("mask");

        let summary =
            index_zone_mask_with_mask(&mut store, "test", &mask, false).expect("index zone mask");
        assert_eq!(summary.total, 2);
        assert_eq!(summary.assigned, 2);

        let zones = store.load_event_zones("test").expect("load zones");
        let mut by_id = std::collections::HashMap::new();
        for (event_id, rgb) in zones {
            by_id.insert(event_id, rgb);
        }
        assert_eq!(by_id.get(&1), Some(&pack_rgb_u32(255, 0, 0)));
        assert_eq!(by_id.get(&2), Some(&pack_rgb_u32(0, 255, 0)));
        assert!(!by_id.contains_key(&3));
    }

    #[test]
    fn zonemask_dimension_mismatch() {
        let data = vec![0u8; 4 * 4 * 3];
        let mask = ZoneMask::from_rgb(4, 4, data).expect("mask");
        let err = validate_zonemask_dims(&mask).expect_err("should fail");
        let msg = format!("{err}");
        assert!(msg.contains("zone mask dimensions mismatch"));
    }

    #[test]
    fn ranking_uid_is_stable_for_identical_rows() {
        let uid1 = ranking_event_uid(1_700_000_000, 101, 12500, 100, 200, 300);
        let uid2 = ranking_event_uid(1_700_000_000, 101, 12500, 100, 200, 300);
        let uid3 = ranking_event_uid(1_700_000_001, 101, 12500, 100, 200, 300);
        assert_eq!(uid1.len(), 32);
        assert_eq!(uid1, uid2);
        assert_ne!(uid1, uid3);
    }

    #[test]
    fn ranking_dedupe_key_collapses_reimported_rows() {
        let row = RankingRow {
            date: "2025-04-25 23:57:51".to_string(),
            encyclopedia_key: 555,
            length: 12.345,
            x: 10.4,
            y: -20.2,
            z: 30.6,
        };

        let first = ranking_row_to_event_row(&row).expect("first event");
        let second = ranking_row_to_event_row(&row).expect("second event");
        let mut seen = std::collections::HashSet::new();
        assert!(seen.insert(first.event_uid.clone()));
        assert!(!seen.insert(second.event_uid.clone()));
        assert_eq!(seen.len(), 1);
    }

    #[test]
    fn ranking_dedupe_key_normalizes_equivalent_timestamp_formats() {
        let dotted = RankingRow {
            date: "12.04.2025 23:57".to_string(),
            encyclopedia_key: 21,
            length: 114.683,
            x: -249_904.0,
            y: -4_059.0,
            z: -47_175.0,
        };
        let am_pm = RankingRow {
            date: "2025-04-12 11:57:00 PM".to_string(),
            encyclopedia_key: dotted.encyclopedia_key,
            length: dotted.length,
            x: dotted.x,
            y: dotted.y,
            z: dotted.z,
        };

        let dotted_event = ranking_row_to_event_row(&dotted).expect("dotted event");
        let am_pm_event = ranking_row_to_event_row(&am_pm).expect("am/pm event");

        assert_eq!(dotted_event.ts_utc, am_pm_event.ts_utc);
        assert_eq!(dotted_event.event_uid, am_pm_event.event_uid);
    }

    #[test]
    fn ranking_dedupe_key_normalizes_subunit_coordinate_noise() {
        let precise = RankingRow {
            date: "24.04.2025 13:44".to_string(),
            encyclopedia_key: 60,
            length: 32.022,
            x: 369_908.38,
            y: -8_245.382,
            z: -23_714.15,
        };
        let rounded = RankingRow {
            date: precise.date.clone(),
            encyclopedia_key: precise.encyclopedia_key,
            length: precise.length,
            x: 369_908.0,
            y: -8_245.0,
            z: -23_714.0,
        };

        let precise_event = ranking_row_to_event_row(&precise).expect("precise event");
        let rounded_event = ranking_row_to_event_row(&rounded).expect("rounded event");

        assert_eq!(precise_event.world_x, rounded_event.world_x);
        assert_eq!(precise_event.world_y, rounded_event.world_y);
        assert_eq!(precise_event.world_z, rounded_event.world_z);
        assert_eq!(precise_event.event_uid, rounded_event.event_uid);
    }

    #[test]
    fn ranking_rows_deduped_excludes_skipped_rows() {
        assert_eq!(ranking_rows_deduped(10, 3, 2), 5);
    }

    #[test]
    fn ranking_row_maps_to_direct_evidence_semantics() {
        let row = RankingRow {
            date: "2025-04-25 23:57:51".to_string(),
            encyclopedia_key: 555,
            length: 12.345,
            x: 10.4,
            y: -20.2,
            z: 30.6,
        };
        let event = ranking_row_to_event_row(&row).expect("event");
        assert_eq!(event.snap_px_x, event.map_px_x);
        assert_eq!(event.snap_px_y, event.map_px_y);
        assert_eq!(event.snap_dist_px, 0);
        assert!(event.water_ok);
        assert_eq!(event.length_milli, 12345);
    }

    #[test]
    fn ranking_ring_zone_overlaps_marks_fully_contained_zone() {
        let width = 12u32;
        let height = 12u32;
        let mut data = vec![0u8; (width * height * 3) as usize];
        for y in 0..height {
            for x in 0..width {
                if x < 6 {
                    set_pixel(&mut data, width, x, y, 255, 0, 0);
                } else {
                    set_pixel(&mut data, width, x, y, 0, 255, 0);
                }
            }
        }
        let mask = ZoneMask::from_rgb(width, height, data).expect("mask");

        let rows = build_event_zone_ring_support_rows(&mask, 42, 3, 6);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].event_id, 42);
        assert_eq!(rows[0].zone_rgb, pack_rgb_u32(255, 0, 0));
        assert!(rows[0].ring_fully_contained);
    }

    #[test]
    fn ranking_ring_zone_overlaps_marks_partial_border_crossings() {
        let width = 12u32;
        let height = 12u32;
        let mut data = vec![0u8; (width * height * 3) as usize];
        for y in 0..height {
            for x in 0..width {
                if x < 6 {
                    set_pixel(&mut data, width, x, y, 255, 0, 0);
                } else {
                    set_pixel(&mut data, width, x, y, 0, 255, 0);
                }
            }
        }
        let mask = ZoneMask::from_rgb(width, height, data).expect("mask");

        let rows = build_event_zone_ring_support_rows(&mask, 77, 5, 6);

        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| !row.ring_fully_contained));
        let touched = rows.iter().map(|row| row.zone_rgb).collect::<BTreeSet<_>>();
        assert_eq!(
            touched,
            BTreeSet::from([pack_rgb_u32(255, 0, 0), pack_rgb_u32(0, 255, 0)])
        );
    }
}
