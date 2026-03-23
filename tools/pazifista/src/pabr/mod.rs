mod geojson;
mod matching;
mod parse;
mod render;
mod util;

use std::collections::BTreeMap;
use std::path::PathBuf;

pub(crate) use util::color_rgb_for_value;

pub const DEFAULT_ROW_SHIFT: u32 = 0x0EF0;
pub(crate) const PABR_MAGIC: &[u8; 4] = b"PABR";
pub(crate) const RID_FOOTER_LEN: usize = 47;
pub(crate) const RID_FOOTER_SIGNATURE: [u8; 10] =
    [0x00, 0x00, 0x60, 0xFF, 0xFF, 0xFF, 0x78, 0x87, 0x00, 0x00];
pub(crate) const BKD_TRAILER_LEN: usize = 12;
pub(crate) const INDEX_SENTINEL: u16 = u16::MAX;
pub(crate) const BACKGROUND_BGR: [u8; 3] = [193, 154, 79];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Breakpoint {
    pub source_x: u16,
    pub dictionary_index: u16,
}

#[derive(Clone, Debug)]
pub struct RidFile {
    pub region_ids: Vec<u16>,
    pub native_width: u32,
    pub native_height: u32,
    pub trailer_prefix_len: usize,
}

#[derive(Clone, Debug)]
pub struct BkdFile {
    pub rows: Vec<Vec<Breakpoint>>,
    pub trailer_words: [u32; 3],
    pub max_source_x: u16,
}

#[derive(Clone, Debug)]
pub struct PabrMap {
    pub rid_path: PathBuf,
    pub bkd_path: PathBuf,
    pub rid: RidFile,
    pub bkd: BkdFile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PabrInspect {
    pub dictionary_entries: usize,
    pub scanline_rows: usize,
    pub native_width: u32,
    pub native_height: u32,
    pub wrapped_bands: usize,
    pub used_dictionary_entries: usize,
    pub used_region_ids: usize,
    pub transparent_breakpoints: usize,
    pub max_source_x: u16,
    pub rid_trailer_prefix_len: usize,
    pub bkd_trailer_words: [u32; 3],
}

#[derive(Clone, Debug)]
pub struct RenderSummary {
    pub output_path: PathBuf,
    pub dimensions: OutputDimensions,
}

#[derive(Clone, Debug)]
pub struct GeoJsonExportSummary {
    pub output_path: PathBuf,
    pub feature_count: usize,
    pub rectangle_count: usize,
}

#[derive(Clone, Debug)]
pub struct RegionMatchSummary {
    pub output_path: PathBuf,
    pub pabr_region_count: usize,
    pub current_region_count: usize,
    pub overlap_pair_count: usize,
    pub pabr_only_count: usize,
    pub current_only_count: usize,
    pub mutual_best_match_count: usize,
}

#[derive(Clone, Debug, Default)]
pub struct RegionGroupMapping {
    pub(crate) region_to_group: BTreeMap<u16, u16>,
    pub(crate) group_to_regions: BTreeMap<u16, Vec<u16>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct RowSegment {
    pub(crate) start_x: u32,
    pub(crate) end_x: u32,
    pub(crate) value: u32,
}
