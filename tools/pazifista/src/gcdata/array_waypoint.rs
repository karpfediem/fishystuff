use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use fishystuff_core::constants::{BOTTOM, LEFT, RIGHT, SECTOR_SCALE, TOP};
use serde::{Deserialize, Serialize};

use crate::pabr::color_rgb_for_value;

const ARRAY_WAYPOINT_HEADER_LEN: usize = 24;
const ARRAY_WAYPOINT_MICROCELLS_PER_SECTOR: u32 = 8;
const ARRAY_WAYPOINT_BLOCK_VALUE_COUNT: usize = 64;
const ARRAY_WAYPOINT_BLOCK_BYTES: usize = ARRAY_WAYPOINT_BLOCK_VALUE_COUNT * 2;
const ARRAY_WAYPOINT_HISTOGRAM_LIMIT: usize = 20;

#[derive(Debug, Clone)]
pub struct ArrayWaypointInspectSummary {
    pub output_path: Option<PathBuf>,
    pub preview_bmp_path: Option<PathBuf>,
    pub min_x_sector: i32,
    pub min_y_sector: i32,
    pub min_z_sector: i32,
    pub max_x_sector: i32,
    pub max_y_sector: i32,
    pub max_z_sector: i32,
    pub sector_width: u32,
    pub sector_height: u32,
    pub grid_width: u32,
    pub grid_height: u32,
    pub block_count: usize,
    pub uniform_block_count: usize,
    pub unique_value_count: usize,
    pub total_waypoint_count: Option<usize>,
    pub waypoints_inside_bounds: usize,
    pub focus_waypoint_sample_count: usize,
}

#[derive(Debug, Clone, Copy)]
struct ArrayWaypointHeader {
    min_x_sector: i32,
    min_y_sector: i32,
    min_z_sector: i32,
    max_x_sector: i32,
    max_y_sector: i32,
    max_z_sector: i32,
}

#[derive(Debug, Clone)]
struct ArrayWaypointGrid {
    header: ArrayWaypointHeader,
    grid_width: u32,
    grid_height: u32,
    cells: Vec<u16>,
    stats: ArrayWaypointStats,
}

#[derive(Debug, Clone)]
struct ArrayWaypointStats {
    file_size: u64,
    payload_bytes: usize,
    block_count: usize,
    unique_value_count: usize,
    unique_high_byte_count: usize,
    unique_low_byte_count: usize,
    top_values: Vec<HexCountReport>,
    top_high_bytes: Vec<ByteCountReport>,
    top_low_bytes: Vec<ByteCountReport>,
    block_unique_value_histogram: Vec<BlockUniqueValueBucket>,
    top_uniform_block_values: Vec<HexCountReport>,
    uniform_block_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ArrayWaypointInspectReport {
    path: String,
    file_size: u64,
    payload_bytes: usize,
    storage_order: &'static str,
    sector_size_world_units: u32,
    microcell_size_world_units: u32,
    header: ArrayWaypointHeaderReport,
    repo_map_alignment: RepoMapAlignmentReport,
    sector_width: u32,
    sector_height: u32,
    grid_width: u32,
    grid_height: u32,
    block_count: usize,
    unique_value_count: usize,
    unique_high_byte_count: usize,
    unique_low_byte_count: usize,
    top_values: Vec<HexCountReport>,
    top_high_bytes: Vec<ByteCountReport>,
    top_low_bytes: Vec<ByteCountReport>,
    block_unique_value_histogram: Vec<BlockUniqueValueBucket>,
    top_uniform_block_values: Vec<HexCountReport>,
    waypoints: Option<WaypointSamplingSummaryReport>,
    focus_waypoint_samples: Vec<FocusWaypointSampleReport>,
}

#[derive(Debug, Clone, Serialize)]
struct ArrayWaypointHeaderReport {
    min_x_sector: i32,
    min_y_sector: i32,
    min_z_sector: i32,
    max_x_sector: i32,
    max_y_sector: i32,
    max_z_sector: i32,
    min_world_x: f64,
    min_world_y: f64,
    min_world_z: f64,
    max_world_x: f64,
    max_world_y: f64,
    max_world_z: f64,
}

#[derive(Debug, Clone, Serialize)]
struct RepoMapAlignmentReport {
    repo_left_sector: i32,
    repo_right_sector: i32,
    repo_bottom_sector: i32,
    repo_top_sector: i32,
    left_inset_sectors: i32,
    right_inset_sectors: i32,
    bottom_inset_sectors: i32,
    top_inset_sectors: i32,
    matches_one_sector_inset: bool,
}

#[derive(Debug, Clone, Serialize)]
struct HexCountReport {
    value: u32,
    value_hex: String,
    count: u64,
}

#[derive(Debug, Clone, Serialize)]
struct ByteCountReport {
    value: u8,
    value_hex: String,
    count: u64,
}

#[derive(Debug, Clone, Serialize)]
struct BlockUniqueValueBucket {
    unique_value_count: usize,
    block_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct WaypointSamplingSummaryReport {
    path: String,
    total_waypoint_count: usize,
    inside_bounds_waypoint_count: usize,
    unique_sampled_value_count: usize,
    top_sampled_values: Vec<HexCountReport>,
}

#[derive(Debug, Clone, Serialize)]
struct FocusWaypointSampleReport {
    waypoint_id: u32,
    found_in_waypoints_json: bool,
    inside_bounds: bool,
    sample: Option<WaypointGridSample>,
}

#[derive(Debug, Clone, Serialize)]
struct WaypointGridSample {
    world_x: f64,
    world_y: f64,
    world_z: f64,
    sector_x: i32,
    sector_z: i32,
    sub_x: u8,
    sub_z: u8,
    grid_x: u32,
    grid_y: u32,
    value: u16,
    value_hex: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CurrentWaypointRow {
    #[serde(default)]
    key: u32,
    pos: CurrentWaypointPosition,
}

#[derive(Debug, Clone, Deserialize)]
struct CurrentWaypointPosition {
    x: f64,
    y: f64,
    z: f64,
}

pub fn inspect_arraywaypoint_bin(
    path: &Path,
    waypoints_path: Option<&Path>,
    focus_waypoint_ids: &[u32],
    output_path: Option<&Path>,
    preview_bmp_path: Option<&Path>,
) -> Result<ArrayWaypointInspectSummary> {
    let grid = ArrayWaypointGrid::from_path(path)?;
    let input_path = path.display().to_string();
    let waypoints = waypoints_path.map(load_current_waypoints).transpose()?;
    let waypoint_summary = waypoints.as_ref().map(|rows| {
        grid.sample_waypoints(
            rows,
            waypoints_path
                .map(|value| value.display().to_string())
                .unwrap_or_default(),
        )
    });
    let focus_waypoint_samples = if let Some(rows) = waypoints.as_ref() {
        build_focus_waypoint_samples(&grid, rows, focus_waypoint_ids)
    } else {
        focus_waypoint_ids
            .iter()
            .copied()
            .map(|waypoint_id| FocusWaypointSampleReport {
                waypoint_id,
                found_in_waypoints_json: false,
                inside_bounds: false,
                sample: None,
            })
            .collect()
    };

    if let Some(path) = output_path {
        let report = ArrayWaypointInspectReport {
            path: input_path,
            file_size: grid.stats.file_size,
            payload_bytes: grid.stats.payload_bytes,
            storage_order: "for z_sector { for x_sector { for sub_x { for sub_z { u16be }}}}",
            sector_size_world_units: SECTOR_SCALE as u32,
            microcell_size_world_units: (SECTOR_SCALE as u32)
                / ARRAY_WAYPOINT_MICROCELLS_PER_SECTOR,
            header: grid.header_report(),
            repo_map_alignment: grid.repo_map_alignment_report(),
            sector_width: grid.sector_width(),
            sector_height: grid.sector_height(),
            grid_width: grid.grid_width,
            grid_height: grid.grid_height,
            block_count: grid.stats.block_count,
            unique_value_count: grid.stats.unique_value_count,
            unique_high_byte_count: grid.stats.unique_high_byte_count,
            unique_low_byte_count: grid.stats.unique_low_byte_count,
            top_values: grid.stats.top_values.clone(),
            top_high_bytes: grid.stats.top_high_bytes.clone(),
            top_low_bytes: grid.stats.top_low_bytes.clone(),
            block_unique_value_histogram: grid.stats.block_unique_value_histogram.clone(),
            top_uniform_block_values: grid.stats.top_uniform_block_values.clone(),
            waypoints: waypoint_summary.as_ref().map(|summary| summary.to_report()),
            focus_waypoint_samples: focus_waypoint_samples.clone(),
        };
        super::write_json_report(path, &report)?;
    }

    if let Some(path) = preview_bmp_path {
        grid.render_bmp(path)?;
    }

    Ok(ArrayWaypointInspectSummary {
        output_path: output_path.map(Path::to_path_buf),
        preview_bmp_path: preview_bmp_path.map(Path::to_path_buf),
        min_x_sector: grid.header.min_x_sector,
        min_y_sector: grid.header.min_y_sector,
        min_z_sector: grid.header.min_z_sector,
        max_x_sector: grid.header.max_x_sector,
        max_y_sector: grid.header.max_y_sector,
        max_z_sector: grid.header.max_z_sector,
        sector_width: grid.sector_width(),
        sector_height: grid.sector_height(),
        grid_width: grid.grid_width,
        grid_height: grid.grid_height,
        block_count: grid.stats.block_count,
        uniform_block_count: grid.stats.uniform_block_count,
        unique_value_count: grid.stats.unique_value_count,
        total_waypoint_count: waypoint_summary
            .as_ref()
            .map(|summary| summary.total_waypoint_count),
        waypoints_inside_bounds: waypoint_summary
            .as_ref()
            .map(|summary| summary.inside_bounds_waypoint_count)
            .unwrap_or(0),
        focus_waypoint_sample_count: focus_waypoint_samples
            .iter()
            .filter(|sample| sample.inside_bounds)
            .count(),
    })
}

impl ArrayWaypointGrid {
    fn from_path(path: &Path) -> Result<Self> {
        let bytes = fs::read(path).with_context(|| {
            format!(
                "failed to read mapdata_arraywaypoint.bin {}",
                path.display()
            )
        })?;
        Self::from_bytes(&bytes)
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < ARRAY_WAYPOINT_HEADER_LEN {
            bail!("mapdata_arraywaypoint.bin is too small");
        }

        let header = ArrayWaypointHeader {
            min_x_sector: read_i32_le(bytes, 0)?,
            min_y_sector: read_i32_le(bytes, 4)?,
            min_z_sector: read_i32_le(bytes, 8)?,
            max_x_sector: read_i32_le(bytes, 12)?,
            max_y_sector: read_i32_le(bytes, 16)?,
            max_z_sector: read_i32_le(bytes, 20)?,
        };

        let sector_width = header.sector_width()?;
        let sector_height = header.sector_height()?;
        let expected_payload_bytes = sector_width
            .checked_mul(sector_height)
            .and_then(|value| value.checked_mul(ARRAY_WAYPOINT_BLOCK_BYTES))
            .context("mapdata_arraywaypoint.bin payload size overflow")?;
        let payload_bytes = bytes
            .len()
            .checked_sub(ARRAY_WAYPOINT_HEADER_LEN)
            .context("mapdata_arraywaypoint.bin payload underflow")?;
        if payload_bytes != expected_payload_bytes {
            bail!(
                "mapdata_arraywaypoint.bin payload length mismatch: expected {} bytes for {}x{} sectors, got {}",
                expected_payload_bytes,
                sector_width,
                sector_height,
                payload_bytes
            );
        }

        let grid_width = u32::try_from(sector_width)
            .context("sector width exceeds u32")?
            .checked_mul(ARRAY_WAYPOINT_MICROCELLS_PER_SECTOR)
            .context("grid width overflow")?;
        let grid_height = u32::try_from(sector_height)
            .context("sector height exceeds u32")?
            .checked_mul(ARRAY_WAYPOINT_MICROCELLS_PER_SECTOR)
            .context("grid height overflow")?;
        let grid_len = usize::try_from(u64::from(grid_width) * u64::from(grid_height))
            .context("grid allocation exceeds usize")?;
        let mut cells = vec![0u16; grid_len];

        let mut value_counts = BTreeMap::new();
        let mut high_byte_counts = [0u64; 256];
        let mut low_byte_counts = [0u64; 256];
        let mut unique_blocks = BTreeMap::new();
        let mut uniform_block_values = BTreeMap::new();
        let mut uniform_block_count = 0usize;

        let mut offset = ARRAY_WAYPOINT_HEADER_LEN;
        for sector_z in 0..sector_height {
            for sector_x in 0..sector_width {
                let mut block_counts = BTreeMap::new();
                for sub_x in 0..ARRAY_WAYPOINT_MICROCELLS_PER_SECTOR as usize {
                    for sub_z in 0..ARRAY_WAYPOINT_MICROCELLS_PER_SECTOR as usize {
                        let value = u16::from_be_bytes([bytes[offset], bytes[offset + 1]]);
                        offset += 2;

                        let grid_x =
                            sector_x * ARRAY_WAYPOINT_MICROCELLS_PER_SECTOR as usize + sub_x;
                        let bottom_up_y =
                            sector_z * ARRAY_WAYPOINT_MICROCELLS_PER_SECTOR as usize + sub_z;
                        let grid_y = grid_height as usize - 1 - bottom_up_y;
                        cells[grid_y * grid_width as usize + grid_x] = value;

                        *value_counts.entry(value).or_insert(0u64) += 1;
                        high_byte_counts[(value >> 8) as usize] += 1;
                        low_byte_counts[(value & 0xFF) as usize] += 1;
                        *block_counts.entry(value).or_insert(0usize) += 1;
                    }
                }

                let unique_value_count = block_counts.len();
                *unique_blocks.entry(unique_value_count).or_insert(0usize) += 1;
                if unique_value_count == 1 {
                    uniform_block_count += 1;
                    if let Some((&value, _)) = block_counts.first_key_value() {
                        *uniform_block_values.entry(value).or_insert(0u64) += 1;
                    }
                }
            }
        }

        let stats = ArrayWaypointStats {
            file_size: bytes.len() as u64,
            payload_bytes,
            block_count: sector_width * sector_height,
            unique_value_count: value_counts.len(),
            unique_high_byte_count: high_byte_counts.iter().filter(|count| **count > 0).count(),
            unique_low_byte_count: low_byte_counts.iter().filter(|count| **count > 0).count(),
            top_values: top_hex_counts(&value_counts, ARRAY_WAYPOINT_HISTOGRAM_LIMIT),
            top_high_bytes: top_byte_counts(&high_byte_counts, ARRAY_WAYPOINT_HISTOGRAM_LIMIT),
            top_low_bytes: top_byte_counts(&low_byte_counts, ARRAY_WAYPOINT_HISTOGRAM_LIMIT),
            block_unique_value_histogram: unique_blocks
                .into_iter()
                .map(|(unique_value_count, block_count)| BlockUniqueValueBucket {
                    unique_value_count,
                    block_count,
                })
                .collect(),
            top_uniform_block_values: top_hex_counts(
                &uniform_block_values,
                ARRAY_WAYPOINT_HISTOGRAM_LIMIT,
            ),
            uniform_block_count,
        };

        Ok(Self {
            header,
            grid_width,
            grid_height,
            cells,
            stats,
        })
    }

    fn sector_width(&self) -> u32 {
        (self.header.max_x_sector - self.header.min_x_sector) as u32
    }

    fn sector_height(&self) -> u32 {
        (self.header.max_z_sector - self.header.min_z_sector) as u32
    }

    fn sample_world(&self, world_x: f64, world_y: f64, world_z: f64) -> Option<WaypointGridSample> {
        if !world_x.is_finite() || !world_z.is_finite() {
            return None;
        }

        let sector_x_f = world_x / SECTOR_SCALE;
        let sector_z_f = world_z / SECTOR_SCALE;
        let sector_x = sector_x_f.floor() as i32;
        let sector_z = sector_z_f.floor() as i32;
        if sector_x < self.header.min_x_sector
            || sector_x >= self.header.max_x_sector
            || sector_z < self.header.min_z_sector
            || sector_z >= self.header.max_z_sector
        {
            return None;
        }

        let sub_x = ((sector_x_f - f64::from(sector_x))
            * f64::from(ARRAY_WAYPOINT_MICROCELLS_PER_SECTOR))
        .floor()
        .clamp(0.0, f64::from(ARRAY_WAYPOINT_MICROCELLS_PER_SECTOR - 1)) as u8;
        let sub_z = ((sector_z_f - f64::from(sector_z))
            * f64::from(ARRAY_WAYPOINT_MICROCELLS_PER_SECTOR))
        .floor()
        .clamp(0.0, f64::from(ARRAY_WAYPOINT_MICROCELLS_PER_SECTOR - 1)) as u8;

        let grid_x = u32::try_from(sector_x - self.header.min_x_sector).ok()?
            * ARRAY_WAYPOINT_MICROCELLS_PER_SECTOR
            + u32::from(sub_x);
        let bottom_up_y = u32::try_from(sector_z - self.header.min_z_sector).ok()?
            * ARRAY_WAYPOINT_MICROCELLS_PER_SECTOR
            + u32::from(sub_z);
        let grid_y = self.grid_height.checked_sub(1 + bottom_up_y)?;
        let value = self.cell_at_grid(grid_x, grid_y)?;

        Some(WaypointGridSample {
            world_x,
            world_y,
            world_z,
            sector_x,
            sector_z,
            sub_x,
            sub_z,
            grid_x,
            grid_y,
            value,
            value_hex: format!("{value:04x}"),
        })
    }

    fn cell_at_grid(&self, grid_x: u32, grid_y: u32) -> Option<u16> {
        if grid_x >= self.grid_width || grid_y >= self.grid_height {
            return None;
        }

        let offset =
            usize::try_from(u64::from(grid_y) * u64::from(self.grid_width) + u64::from(grid_x))
                .ok()?;
        self.cells.get(offset).copied()
    }

    fn sample_waypoints(
        &self,
        waypoints: &BTreeMap<String, CurrentWaypointRow>,
        waypoints_path: String,
    ) -> WaypointSamplingSummary {
        let mut sampled_values = BTreeMap::new();
        let mut inside_bounds_waypoint_count = 0usize;

        for row in waypoints.values() {
            if let Some(sample) = self.sample_world(row.pos.x, row.pos.y, row.pos.z) {
                inside_bounds_waypoint_count += 1;
                *sampled_values.entry(sample.value).or_insert(0u64) += 1;
            }
        }

        WaypointSamplingSummary {
            waypoints_path,
            total_waypoint_count: waypoints.len(),
            inside_bounds_waypoint_count,
            sampled_values,
        }
    }

    fn header_report(&self) -> ArrayWaypointHeaderReport {
        ArrayWaypointHeaderReport {
            min_x_sector: self.header.min_x_sector,
            min_y_sector: self.header.min_y_sector,
            min_z_sector: self.header.min_z_sector,
            max_x_sector: self.header.max_x_sector,
            max_y_sector: self.header.max_y_sector,
            max_z_sector: self.header.max_z_sector,
            min_world_x: f64::from(self.header.min_x_sector) * SECTOR_SCALE,
            min_world_y: f64::from(self.header.min_y_sector) * SECTOR_SCALE,
            min_world_z: f64::from(self.header.min_z_sector) * SECTOR_SCALE,
            max_world_x: f64::from(self.header.max_x_sector) * SECTOR_SCALE,
            max_world_y: f64::from(self.header.max_y_sector) * SECTOR_SCALE,
            max_world_z: f64::from(self.header.max_z_sector) * SECTOR_SCALE,
        }
    }

    fn repo_map_alignment_report(&self) -> RepoMapAlignmentReport {
        let repo_left_sector = LEFT as i32;
        let repo_right_sector = RIGHT as i32;
        let repo_bottom_sector = BOTTOM as i32;
        let repo_top_sector = TOP as i32;
        let left_inset_sectors = self.header.min_x_sector - repo_left_sector;
        let right_inset_sectors = repo_right_sector - self.header.max_x_sector;
        let bottom_inset_sectors = self.header.min_z_sector - repo_bottom_sector;
        let top_inset_sectors = repo_top_sector - self.header.max_z_sector;

        RepoMapAlignmentReport {
            repo_left_sector,
            repo_right_sector,
            repo_bottom_sector,
            repo_top_sector,
            left_inset_sectors,
            right_inset_sectors,
            bottom_inset_sectors,
            top_inset_sectors,
            matches_one_sector_inset: left_inset_sectors == 1
                && right_inset_sectors == 1
                && bottom_inset_sectors == 1
                && top_inset_sectors == 1,
        }
    }

    fn render_bmp(&self, output_path: &Path) -> Result<()> {
        if let Some(parent) = output_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("failed to create output directory {}", parent.display())
                })?;
            }
        }

        let file = File::create(output_path)
            .with_context(|| format!("failed to create {}", output_path.display()))?;
        let mut writer = BufWriter::new(file);
        let row_padding = (4 - (self.grid_width * 3) % 4) % 4;
        let row_stride = self.grid_width as usize * 3 + row_padding as usize;
        let image_size = row_stride as u64 * u64::from(self.grid_height);
        let file_size = 54u64 + image_size;
        if file_size > u64::from(u32::MAX) {
            bail!(
                "BMP output would be too large for the format header: {} bytes",
                file_size
            );
        }

        write_bmp_header(
            &mut writer,
            self.grid_width,
            self.grid_height,
            image_size as u32,
            file_size as u32,
        )?;

        let mut row_buffer = vec![0u8; row_stride];
        for output_y in (0..self.grid_height).rev() {
            row_buffer.fill(0);
            let row_start = usize::try_from(u64::from(output_y) * u64::from(self.grid_width))
                .context("row offset overflow while rendering mapdata_arraywaypoint")?;
            for output_x in 0..self.grid_width as usize {
                let value = self.cells[row_start + output_x];
                let [red, green, blue] = color_rgb_for_value(u32::from(value));
                let pixel_offset = output_x * 3;
                row_buffer[pixel_offset] = blue;
                row_buffer[pixel_offset + 1] = green;
                row_buffer[pixel_offset + 2] = red;
            }
            writer
                .write_all(&row_buffer)
                .with_context(|| format!("failed writing {}", output_path.display()))?;
        }

        writer
            .flush()
            .with_context(|| format!("failed flushing {}", output_path.display()))?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct WaypointSamplingSummary {
    waypoints_path: String,
    total_waypoint_count: usize,
    inside_bounds_waypoint_count: usize,
    sampled_values: BTreeMap<u16, u64>,
}

impl WaypointSamplingSummary {
    fn to_report(&self) -> WaypointSamplingSummaryReport {
        WaypointSamplingSummaryReport {
            path: self.waypoints_path.clone(),
            total_waypoint_count: self.total_waypoint_count,
            inside_bounds_waypoint_count: self.inside_bounds_waypoint_count,
            unique_sampled_value_count: self.sampled_values.len(),
            top_sampled_values: top_hex_counts(
                &self.sampled_values,
                ARRAY_WAYPOINT_HISTOGRAM_LIMIT,
            ),
        }
    }
}

fn build_focus_waypoint_samples(
    grid: &ArrayWaypointGrid,
    waypoints: &BTreeMap<String, CurrentWaypointRow>,
    focus_waypoint_ids: &[u32],
) -> Vec<FocusWaypointSampleReport> {
    focus_waypoint_ids
        .iter()
        .copied()
        .map(|waypoint_id| {
            let found = waypoints
                .get(&waypoint_id.to_string())
                .filter(|row| row.key == waypoint_id || row.key == 0);
            if let Some(row) = found {
                let sample = grid.sample_world(row.pos.x, row.pos.y, row.pos.z);
                FocusWaypointSampleReport {
                    waypoint_id,
                    found_in_waypoints_json: true,
                    inside_bounds: sample.is_some(),
                    sample,
                }
            } else {
                FocusWaypointSampleReport {
                    waypoint_id,
                    found_in_waypoints_json: false,
                    inside_bounds: false,
                    sample: None,
                }
            }
        })
        .collect()
}

fn load_current_waypoints(path: &Path) -> Result<BTreeMap<String, CurrentWaypointRow>> {
    let file = File::open(path)
        .with_context(|| format!("failed to open current waypoints JSON {}", path.display()))?;
    serde_json::from_reader(file)
        .with_context(|| format!("failed to parse current waypoints JSON {}", path.display()))
}

fn top_hex_counts<T>(counts: &BTreeMap<T, u64>, limit: usize) -> Vec<HexCountReport>
where
    T: Copy + Ord + Into<u32>,
{
    let mut items = counts
        .iter()
        .map(|(value, count)| HexCountReport {
            value: (*value).into(),
            value_hex: format!("{:04x}", Into::<u32>::into(*value)),
            count: *count,
        })
        .collect::<Vec<_>>();
    items.sort_by(|lhs, rhs| {
        rhs.count
            .cmp(&lhs.count)
            .then_with(|| lhs.value.cmp(&rhs.value))
    });
    items.truncate(limit);
    items
}

fn top_byte_counts(counts: &[u64; 256], limit: usize) -> Vec<ByteCountReport> {
    let mut items = counts
        .iter()
        .enumerate()
        .filter_map(|(value, count)| {
            (*count > 0).then_some(ByteCountReport {
                value: value as u8,
                value_hex: format!("{value:02x}"),
                count: *count,
            })
        })
        .collect::<Vec<_>>();
    items.sort_by(|lhs, rhs| {
        rhs.count
            .cmp(&lhs.count)
            .then_with(|| lhs.value.cmp(&rhs.value))
    });
    items.truncate(limit);
    items
}

fn read_i32_le(bytes: &[u8], offset: usize) -> Result<i32> {
    let end = offset
        .checked_add(4)
        .context("i32 offset overflow while parsing mapdata_arraywaypoint.bin")?;
    let slice = bytes
        .get(offset..end)
        .with_context(|| format!("i32 read at offset {} is out of bounds", offset))?;
    Ok(i32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn write_bmp_header<W: Write>(
    writer: &mut W,
    width: u32,
    height: u32,
    image_size: u32,
    file_size: u32,
) -> Result<()> {
    writer.write_all(b"BM")?;
    writer.write_all(&file_size.to_le_bytes())?;
    writer.write_all(&[0, 0, 0, 0])?;
    writer.write_all(&54u32.to_le_bytes())?;
    writer.write_all(&40u32.to_le_bytes())?;
    writer.write_all(&(width as i32).to_le_bytes())?;
    writer.write_all(&(height as i32).to_le_bytes())?;
    writer.write_all(&1u16.to_le_bytes())?;
    writer.write_all(&24u16.to_le_bytes())?;
    writer.write_all(&0u32.to_le_bytes())?;
    writer.write_all(&image_size.to_le_bytes())?;
    writer.write_all(&2835i32.to_le_bytes())?;
    writer.write_all(&2835i32.to_le_bytes())?;
    writer.write_all(&0u32.to_le_bytes())?;
    writer.write_all(&0u32.to_le_bytes())?;
    Ok(())
}

impl ArrayWaypointHeader {
    fn sector_width(&self) -> Result<usize> {
        let width = self
            .max_x_sector
            .checked_sub(self.min_x_sector)
            .context("x-sector span overflow in mapdata_arraywaypoint.bin header")?;
        if width <= 0 {
            bail!("mapdata_arraywaypoint.bin x-sector span must be positive");
        }
        usize::try_from(width).context("x-sector span does not fit usize")
    }

    fn sector_height(&self) -> Result<usize> {
        let height = self
            .max_z_sector
            .checked_sub(self.min_z_sector)
            .context("z-sector span overflow in mapdata_arraywaypoint.bin header")?;
        if height <= 0 {
            bail!("mapdata_arraywaypoint.bin z-sector span must be positive");
        }
        usize::try_from(height).context("z-sector span does not fit usize")
    }
}

#[cfg(test)]
mod tests {
    use super::{ArrayWaypointGrid, ARRAY_WAYPOINT_BLOCK_BYTES};

    fn synthetic_bytes() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0i32.to_le_bytes());
        bytes.extend_from_slice(&0i32.to_le_bytes());
        bytes.extend_from_slice(&0i32.to_le_bytes());
        bytes.extend_from_slice(&1i32.to_le_bytes());
        bytes.extend_from_slice(&1i32.to_le_bytes());
        bytes.extend_from_slice(&1i32.to_le_bytes());

        assert_eq!(ARRAY_WAYPOINT_BLOCK_BYTES, 128);
        for sub_x in 0..8u16 {
            for sub_z in 0..8u16 {
                let value = sub_x * 10 + sub_z;
                bytes.extend_from_slice(&value.to_be_bytes());
            }
        }
        bytes
    }

    #[test]
    fn reconstructs_single_sector_grid_and_samples_world_positions() {
        let grid = ArrayWaypointGrid::from_bytes(&synthetic_bytes()).expect("grid");
        assert_eq!(grid.grid_width, 8);
        assert_eq!(grid.grid_height, 8);
        assert_eq!(grid.cell_at_grid(0, 7), Some(0));
        assert_eq!(grid.cell_at_grid(7, 0), Some(77));

        let sample = grid
            .sample_world(100.0, 0.0, 100.0)
            .expect("sample within bounds");
        assert_eq!(sample.grid_x, 0);
        assert_eq!(sample.grid_y, 7);
        assert_eq!(sample.value, 0);

        let sample = grid
            .sample_world(12_799.0, 0.0, 12_799.0)
            .expect("sample within bounds");
        assert_eq!(sample.grid_x, 7);
        assert_eq!(sample.grid_y, 0);
        assert_eq!(sample.value, 77);
    }
}
