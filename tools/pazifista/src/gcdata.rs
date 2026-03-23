mod array_waypoint;
mod loc;
mod stringtable;
mod waypoint_xml;

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use fishystuff_core::loc::load_loc_namespaces_as_string_maps;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::pabr::{PabrMap, RidFile, PABR_MAGIC};

#[allow(unused_imports)]
pub use array_waypoint::{inspect_arraywaypoint_bin, ArrayWaypointInspectSummary};
pub use loc::inspect_loc;
pub use stringtable::inspect_stringtable_bss;
pub use waypoint_xml::inspect_waypoint_xml;

const REGIONINFO_ROW_SIGNATURE_PREFIX: [u8; 4] = [0x5A, 0x55, 0x00, 0x00];
const REGIONINFO_ROW_SIGNATURE_OFFSET: usize = 32;
const REGIONINFO_ROW_TRADE_ORIGIN_OFFSET: usize = 102;
const REGIONINFO_ROW_GROUP_OFFSET: usize = 104;
const REGIONINFO_ROW_WAYPOINT_PRIMARY_OFFSET: usize = 106;
const REGIONINFO_ROW_WAYPOINT_SECONDARY_OFFSET: usize = 110;
const REGIONINFO_ROW_WAREHOUSE_CHARACTER_OFFSET: usize = 185;
const REGIONINFO_ROW_NPC_WORKER_CHARACTER_OFFSET: usize = 189;
const REGIONINFO_ROW_MIN_LEN: usize = 193;
const REGIONGROUPINFO_ROW_LEN: usize = 51;
const REGIONGROUPINFO_ROW_WAYPOINT_OFFSET: usize = 5;
const REGIONGROUPINFO_ROW_FLAGS_OFFSET: usize = 9;
const REGIONGROUPINFO_ROW_GRAPHX_OFFSET: usize = 12;
const REGIONGROUPINFO_ROW_GRAPHY_OFFSET: usize = 16;
const REGIONGROUPINFO_ROW_GRAPHZ_OFFSET: usize = 20;
const PABR_TRAILER_LEN: usize = 12;

#[derive(Debug, Clone)]
pub struct RegionSourceComparisonSummary {
    pub output_path: PathBuf,
    pub rid_dictionary_region_count: usize,
    pub pabr_used_region_count: usize,
    pub pabr_active_region_count: usize,
    pub current_region_count: usize,
    pub current_regioninfo_count: Option<usize>,
    pub unresolved_region_count: usize,
}

#[derive(Debug, Clone)]
pub struct RegionClientDataInspectSummary {
    pub variant_count: usize,
    pub total_unique_region_ids: usize,
    pub focus_region_count: usize,
}

#[derive(Debug, Clone)]
pub struct PabrTableInspectSummary {
    pub path: PathBuf,
    pub entry_count: u32,
    pub file_size: u64,
}

#[derive(Debug, Clone)]
pub struct RegionInfoBssInspectSummary {
    pub output_path: Option<PathBuf>,
    pub header_entry_count: u32,
    pub decoded_signature_row_count: usize,
    pub focus_row_count: usize,
    pub missing_focus_row_count: usize,
}

#[derive(Debug, Clone)]
pub struct RegionGroupInfoBssInspectSummary {
    pub output_path: Option<PathBuf>,
    pub header_entry_count: u32,
    pub decoded_group_row_count: usize,
    pub blank_placeholder_row_count: usize,
    pub current_group_count: Option<usize>,
    pub original_only_group_count: Option<usize>,
    pub current_only_group_count: Option<usize>,
    pub focus_row_count: usize,
    pub missing_focus_row_count: usize,
}

#[derive(Debug, Serialize)]
struct RegionSourceComparisonReport {
    rid_path: String,
    bkd_path: String,
    current_regions_path: String,
    row_shift: u32,
    current_regioninfo_path: Option<String>,
    regioninfo_bss: Option<PabrTableReport>,
    regionclientdata_variants: Vec<RegionClientDataVariantReport>,
    rid_dictionary_region_count: usize,
    pabr_used_region_count: usize,
    pabr_active_region_count: usize,
    current_region_count: usize,
    current_regioninfo_count: Option<usize>,
    rid_dictionary_only_unused_region_ids: Vec<u32>,
    bkd_referenced_only_zero_area_region_ids: Vec<u32>,
    pabr_active_only_region_ids: Vec<u32>,
    current_only_region_ids: Vec<u32>,
    pabr_active_region_ids_missing_from_current_regioninfo: Option<Vec<u32>>,
    regioninfo_bss_minus_current_regioninfo_count: Option<i64>,
    regioninfo_bss_gap_matches_missing_current_regioninfo_pabr_ids: Option<bool>,
    unresolved_region_ids: Vec<RegionSourcePresence>,
}

#[derive(Debug, Serialize)]
struct PabrTableReport {
    path: String,
    entry_count: u32,
    file_size: u64,
}

#[derive(Debug, Serialize)]
pub struct RegionClientDataVariantReport {
    pub variant: String,
    pub path: String,
    pub region_count: usize,
    pub focus_region_ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecodedRegionInfoBssRow {
    pub key: u32,
    pub row_start_offset: usize,
    pub tradeoriginregion: u32,
    pub regiongroup: u32,
    pub waypoint: Option<u32>,
    pub ware_house_character_key: Option<u32>,
    pub npc_worker_character_key: Option<u32>,
    pub origin_label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecodedRegionGroupInfoBssRow {
    pub key: u32,
    pub row_index: usize,
    pub row_start_offset: usize,
    pub waypoint: Option<u32>,
    pub graphx: f32,
    pub graphy: f32,
    pub graphz: f32,
    pub has_graph_position: bool,
    pub flag_bytes_hex: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CurrentRegionInfoRowSummary {
    pub key: u32,
    pub is_accessible: u32,
    pub tradeoriginregion: u32,
    pub regiongroup: u32,
    pub waypoint: u32,
    pub ware_house_character_key: u32,
    pub npc_worker_character_key: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct CurrentRegionGroupGraphRowSummary {
    pub key: u32,
    pub waypoint: u32,
    pub graphx: f64,
    pub graphy: f64,
    pub graphz: f64,
}

#[derive(Debug, Serialize)]
struct RegionInfoBssInspectReport {
    path: String,
    header_entry_count: u32,
    decoded_signature_row_count: usize,
    undecoded_header_row_count: i64,
    rows: Vec<DecodedRegionInfoBssRowWithCurrent>,
    missing_focus_region_ids: Vec<u32>,
}

#[derive(Debug, Serialize)]
struct RegionGroupInfoBssInspectReport {
    path: String,
    header_entry_count: u32,
    decoded_group_row_count: usize,
    blank_placeholder_row_count: usize,
    current_group_count: Option<usize>,
    original_only_group_ids: Option<Vec<u32>>,
    current_only_group_ids: Option<Vec<u32>>,
    rows: Vec<DecodedRegionGroupInfoBssRowWithCurrent>,
    missing_focus_group_ids: Vec<u32>,
}

#[derive(Debug, Serialize)]
struct DecodedRegionInfoBssRowWithCurrent {
    #[serde(flatten)]
    row: DecodedRegionInfoBssRow,
    current_regioninfo: Option<CurrentRegionInfoRowSummary>,
}

#[derive(Debug, Serialize)]
struct DecodedRegionGroupInfoBssRowWithCurrent {
    #[serde(flatten)]
    row: DecodedRegionGroupInfoBssRow,
    current_deck_rg_graphs: Option<CurrentRegionGroupGraphRowSummary>,
}

#[derive(Debug, Serialize)]
struct RegionSourcePresence {
    region_id: u32,
    current_label: Option<String>,
    in_pabr_rid_dictionary: bool,
    in_pabr_bkd_rows: bool,
    in_pabr_active_area: bool,
    in_current_regions_geojson: bool,
    in_current_regioninfo_json: Option<bool>,
    regionclientdata_variants: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CurrentFeatureCollection {
    #[serde(default)]
    features: Vec<CurrentFeature>,
}

#[derive(Debug, Deserialize)]
struct CurrentFeature {
    #[serde(default)]
    properties: Map<String, Value>,
}

#[derive(Debug, Deserialize)]
struct RegionInfoRow {
    #[serde(default)]
    key: u32,
    #[serde(default)]
    is_accessible: u32,
    #[serde(default)]
    tradeoriginregion: u32,
    #[serde(default)]
    regiongroup: u32,
    #[serde(default)]
    waypoint: u32,
    #[serde(default, rename = "wareHouseCharacterKey")]
    ware_house_character_key: u32,
    #[serde(default, rename = "npcWorkerCharacterKey")]
    npc_worker_character_key: u32,
}

#[derive(Debug, Clone, Deserialize)]
struct CurrentRegionGroupGraphRow {
    #[serde(default, rename = "k")]
    key: u32,
    #[serde(default, rename = "wp")]
    waypoint: u32,
    #[serde(default)]
    graphx: f64,
    #[serde(default)]
    graphy: f64,
    #[serde(default)]
    graphz: f64,
}

#[derive(Debug, Clone)]
struct RegionClientDataVariant {
    variant: String,
    path: PathBuf,
    region_ids: BTreeSet<u32>,
}

#[derive(Debug, Default)]
struct LocalizationFile {
    en: LocalizationTable,
}

#[derive(Debug, Default)]
struct LocalizationTable {
    node: BTreeMap<String, String>,
    town: BTreeMap<String, String>,
}

pub fn compare_region_sources(
    rid_path: &Path,
    current_regions_path: &Path,
    row_shift: u32,
    current_regioninfo_path: Option<&Path>,
    regioninfo_bss_path: Option<&Path>,
    regionclientdata_paths: &[PathBuf],
    output_path: &Path,
) -> Result<RegionSourceComparisonSummary> {
    let (rid_path, bkd_path) = PabrMap::paired_paths(rid_path)?;
    let map = PabrMap::from_paths(&rid_path, &bkd_path)?;
    let rid = RidFile::from_path(&rid_path)?;
    let rid_dictionary_region_ids: BTreeSet<u32> =
        rid.region_ids.iter().map(|&id| u32::from(id)).collect();
    let pabr_used_region_ids = map.used_region_ids()?;
    let pabr_active_region_ids: BTreeSet<u32> =
        map.native_region_areas(row_shift)?.into_keys().collect();
    let (current_region_ids, current_labels) = load_current_regions(current_regions_path)?;

    let current_regioninfo_ids = current_regioninfo_path
        .map(load_current_regioninfo_ids)
        .transpose()?;
    let regioninfo_bss = regioninfo_bss_path.map(inspect_pabr_table).transpose()?;
    let regionclientdata = load_regionclientdata_variants(regionclientdata_paths)?;

    let rid_dictionary_only_unused_region_ids: Vec<u32> = rid_dictionary_region_ids
        .difference(&pabr_used_region_ids)
        .copied()
        .collect();
    let bkd_referenced_only_zero_area_region_ids: Vec<u32> = pabr_used_region_ids
        .difference(&pabr_active_region_ids)
        .copied()
        .collect();
    let pabr_active_only_region_ids: Vec<u32> = pabr_active_region_ids
        .difference(&current_region_ids)
        .copied()
        .collect();
    let current_only_region_ids: Vec<u32> = current_region_ids
        .difference(&pabr_active_region_ids)
        .copied()
        .collect();

    let pabr_active_region_ids_missing_from_current_regioninfo = current_regioninfo_ids
        .as_ref()
        .map(|current_regioninfo_ids| {
            pabr_active_region_ids
                .difference(current_regioninfo_ids)
                .copied()
                .collect::<Vec<_>>()
        });

    let regioninfo_bss_minus_current_regioninfo_count = regioninfo_bss
        .as_ref()
        .zip(current_regioninfo_ids.as_ref())
        .map(|(regioninfo_bss, current_regioninfo_ids)| {
            i64::from(regioninfo_bss.entry_count) - current_regioninfo_ids.len() as i64
        });

    let regioninfo_bss_gap_matches_missing_current_regioninfo_pabr_ids =
        regioninfo_bss_minus_current_regioninfo_count
            .zip(pabr_active_region_ids_missing_from_current_regioninfo.as_ref())
            .map(|(count_gap, missing_ids)| count_gap == missing_ids.len() as i64);

    let unresolved_region_ids = build_unresolved_region_presence(
        &pabr_active_only_region_ids,
        &current_only_region_ids,
        &rid_dictionary_region_ids,
        &pabr_used_region_ids,
        &pabr_active_region_ids,
        &current_region_ids,
        current_regioninfo_ids.as_ref(),
        &current_labels,
        &regionclientdata,
    );

    let focus_region_ids: BTreeSet<u32> = unresolved_region_ids
        .iter()
        .map(|presence| presence.region_id)
        .collect();
    let regionclientdata_variants = regionclientdata
        .iter()
        .map(|variant| RegionClientDataVariantReport {
            variant: variant.variant.clone(),
            path: variant.path.display().to_string(),
            region_count: variant.region_ids.len(),
            focus_region_ids: focus_region_ids
                .iter()
                .copied()
                .filter(|region_id| variant.region_ids.contains(region_id))
                .collect(),
        })
        .collect::<Vec<_>>();

    let report = RegionSourceComparisonReport {
        rid_path: rid_path.display().to_string(),
        bkd_path: bkd_path.display().to_string(),
        current_regions_path: current_regions_path.display().to_string(),
        row_shift,
        current_regioninfo_path: current_regioninfo_path.map(|path| path.display().to_string()),
        regioninfo_bss: regioninfo_bss.map(|summary| PabrTableReport {
            path: summary.path.display().to_string(),
            entry_count: summary.entry_count,
            file_size: summary.file_size,
        }),
        regionclientdata_variants,
        rid_dictionary_region_count: rid_dictionary_region_ids.len(),
        pabr_used_region_count: pabr_used_region_ids.len(),
        pabr_active_region_count: pabr_active_region_ids.len(),
        current_region_count: current_region_ids.len(),
        current_regioninfo_count: current_regioninfo_ids.as_ref().map(BTreeSet::len),
        rid_dictionary_only_unused_region_ids,
        bkd_referenced_only_zero_area_region_ids,
        pabr_active_only_region_ids,
        current_only_region_ids,
        pabr_active_region_ids_missing_from_current_regioninfo,
        regioninfo_bss_minus_current_regioninfo_count,
        regioninfo_bss_gap_matches_missing_current_regioninfo_pabr_ids,
        unresolved_region_ids,
    };

    write_json_report(output_path, &report)?;
    Ok(RegionSourceComparisonSummary {
        output_path: output_path.to_path_buf(),
        rid_dictionary_region_count: report.rid_dictionary_region_count,
        pabr_used_region_count: report.pabr_used_region_count,
        pabr_active_region_count: report.pabr_active_region_count,
        current_region_count: report.current_region_count,
        current_regioninfo_count: report.current_regioninfo_count,
        unresolved_region_count: report.unresolved_region_ids.len(),
    })
}

pub fn inspect_regionclientdata(
    paths: &[PathBuf],
    focus_region_ids: &[u32],
) -> Result<(
    Vec<RegionClientDataVariantReport>,
    RegionClientDataInspectSummary,
)> {
    let variants = load_regionclientdata_variants(paths)?;
    let focus_region_ids: BTreeSet<u32> = focus_region_ids.iter().copied().collect();
    let total_unique_region_ids: BTreeSet<u32> = variants
        .iter()
        .flat_map(|variant| variant.region_ids.iter().copied())
        .collect();

    let reports = variants
        .iter()
        .map(|variant| RegionClientDataVariantReport {
            variant: variant.variant.clone(),
            path: variant.path.display().to_string(),
            region_count: variant.region_ids.len(),
            focus_region_ids: focus_region_ids
                .iter()
                .copied()
                .filter(|region_id| variant.region_ids.contains(region_id))
                .collect(),
        })
        .collect::<Vec<_>>();

    let focus_region_count = if focus_region_ids.is_empty() {
        0
    } else {
        total_unique_region_ids
            .intersection(&focus_region_ids)
            .count()
    };

    Ok((
        reports,
        RegionClientDataInspectSummary {
            variant_count: variants.len(),
            total_unique_region_ids: total_unique_region_ids.len(),
            focus_region_count,
        },
    ))
}

pub fn inspect_pabr_table(path: &Path) -> Result<PabrTableInspectSummary> {
    let bytes =
        fs::read(path).with_context(|| format!("failed to read PABR table {}", path.display()))?;
    if bytes.len() < 8 {
        bail!("PABR table {} is too small", path.display());
    }
    if &bytes[0..4] != PABR_MAGIC {
        bail!("PABR table {} is missing PABR magic", path.display());
    }

    Ok(PabrTableInspectSummary {
        path: path.to_path_buf(),
        entry_count: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        file_size: bytes.len() as u64,
    })
}

pub fn inspect_regioninfo_bss(
    path: &Path,
    loc_path: Option<&Path>,
    current_regioninfo_path: Option<&Path>,
    focus_region_ids: &[u32],
    output_path: Option<&Path>,
) -> Result<RegionInfoBssInspectSummary> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read regioninfo.bss {}", path.display()))?;
    let table = inspect_pabr_table(path)?;
    let localization = loc_path.map(load_localization).transpose()?;
    let current_regioninfo = current_regioninfo_path
        .map(load_current_regioninfo_rows_by_key)
        .transpose()?;
    let decoded_rows = decode_regioninfo_bss_signature_rows(&bytes, localization.as_ref())?;

    let requested_rows = if focus_region_ids.is_empty() {
        decoded_rows.values().cloned().collect::<Vec<_>>()
    } else {
        focus_region_ids
            .iter()
            .filter_map(|region_id| decoded_rows.get(region_id).cloned())
            .collect::<Vec<_>>()
    };
    let missing_focus_region_ids = if focus_region_ids.is_empty() {
        Vec::new()
    } else {
        focus_region_ids
            .iter()
            .copied()
            .filter(|region_id| !decoded_rows.contains_key(region_id))
            .collect::<Vec<_>>()
    };

    if let Some(output_path) = output_path {
        let report = RegionInfoBssInspectReport {
            path: path.display().to_string(),
            header_entry_count: table.entry_count,
            decoded_signature_row_count: decoded_rows.len(),
            undecoded_header_row_count: i64::from(table.entry_count) - decoded_rows.len() as i64,
            rows: requested_rows
                .into_iter()
                .map(|row| DecodedRegionInfoBssRowWithCurrent {
                    current_regioninfo: current_regioninfo
                        .as_ref()
                        .and_then(|rows| rows.get(&row.key))
                        .map(current_regioninfo_row_summary),
                    row,
                })
                .collect(),
            missing_focus_region_ids: missing_focus_region_ids.clone(),
        };
        write_json_report(output_path, &report)?;
    }

    Ok(RegionInfoBssInspectSummary {
        output_path: output_path.map(Path::to_path_buf),
        header_entry_count: table.entry_count,
        decoded_signature_row_count: decoded_rows.len(),
        focus_row_count: if focus_region_ids.is_empty() {
            decoded_rows.len()
        } else {
            focus_region_ids
                .len()
                .saturating_sub(missing_focus_region_ids.len())
        },
        missing_focus_row_count: missing_focus_region_ids.len(),
    })
}

pub fn inspect_regiongroupinfo_bss(
    path: &Path,
    current_deck_rg_graphs_path: Option<&Path>,
    focus_group_ids: &[u32],
    output_path: Option<&Path>,
) -> Result<RegionGroupInfoBssInspectSummary> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read regiongroupinfo.bss {}", path.display()))?;
    let table = inspect_pabr_table(path)?;
    let decoded_rows = decode_regiongroupinfo_bss_rows(&bytes)?;
    let blank_placeholder_row_count = usize::try_from(table.entry_count)
        .unwrap_or(0)
        .saturating_sub(decoded_rows.len());
    let current_rows = current_deck_rg_graphs_path
        .map(load_current_regiongroup_graph_rows_by_key)
        .transpose()?;

    let requested_rows = if focus_group_ids.is_empty() {
        decoded_rows.values().cloned().collect::<Vec<_>>()
    } else {
        focus_group_ids
            .iter()
            .filter_map(|group_id| decoded_rows.get(group_id).cloned())
            .collect::<Vec<_>>()
    };
    let missing_focus_group_ids = if focus_group_ids.is_empty() {
        Vec::new()
    } else {
        focus_group_ids
            .iter()
            .copied()
            .filter(|group_id| !decoded_rows.contains_key(group_id))
            .collect::<Vec<_>>()
    };

    let (current_group_count, original_only_group_ids, current_only_group_ids) =
        if let Some(current_rows) = current_rows.as_ref() {
            let original_group_ids: BTreeSet<u32> = decoded_rows.keys().copied().collect();
            let current_group_ids: BTreeSet<u32> = current_rows.keys().copied().collect();
            let original_only_group_ids = original_group_ids
                .difference(&current_group_ids)
                .copied()
                .collect::<Vec<_>>();
            let current_only_group_ids = current_group_ids
                .difference(&original_group_ids)
                .copied()
                .collect::<Vec<_>>();
            (
                Some(current_group_ids.len()),
                Some(original_only_group_ids),
                Some(current_only_group_ids),
            )
        } else {
            (None, None, None)
        };

    if let Some(output_path) = output_path {
        let report = RegionGroupInfoBssInspectReport {
            path: path.display().to_string(),
            header_entry_count: table.entry_count,
            decoded_group_row_count: decoded_rows.len(),
            blank_placeholder_row_count,
            current_group_count,
            original_only_group_ids: original_only_group_ids.clone(),
            current_only_group_ids: current_only_group_ids.clone(),
            rows: requested_rows
                .into_iter()
                .map(|row| DecodedRegionGroupInfoBssRowWithCurrent {
                    current_deck_rg_graphs: current_rows
                        .as_ref()
                        .and_then(|rows| rows.get(&row.key))
                        .map(current_regiongroup_graph_row_summary),
                    row,
                })
                .collect(),
            missing_focus_group_ids: missing_focus_group_ids.clone(),
        };
        write_json_report(output_path, &report)?;
    }

    Ok(RegionGroupInfoBssInspectSummary {
        output_path: output_path.map(Path::to_path_buf),
        header_entry_count: table.entry_count,
        decoded_group_row_count: decoded_rows.len(),
        blank_placeholder_row_count,
        current_group_count,
        original_only_group_count: original_only_group_ids.as_ref().map(std::vec::Vec::len),
        current_only_group_count: current_only_group_ids.as_ref().map(std::vec::Vec::len),
        focus_row_count: if focus_group_ids.is_empty() {
            decoded_rows.len()
        } else {
            focus_group_ids
                .len()
                .saturating_sub(missing_focus_group_ids.len())
        },
        missing_focus_row_count: missing_focus_group_ids.len(),
    })
}

fn build_unresolved_region_presence(
    pabr_active_only_region_ids: &[u32],
    current_only_region_ids: &[u32],
    rid_dictionary_region_ids: &BTreeSet<u32>,
    pabr_used_region_ids: &BTreeSet<u32>,
    pabr_active_region_ids: &BTreeSet<u32>,
    current_region_ids: &BTreeSet<u32>,
    current_regioninfo_ids: Option<&BTreeSet<u32>>,
    current_labels: &BTreeMap<u32, String>,
    regionclientdata: &[RegionClientDataVariant],
) -> Vec<RegionSourcePresence> {
    let focus_region_ids: BTreeSet<u32> = pabr_active_only_region_ids
        .iter()
        .copied()
        .chain(current_only_region_ids.iter().copied())
        .collect();

    focus_region_ids
        .into_iter()
        .map(|region_id| RegionSourcePresence {
            region_id,
            current_label: current_labels.get(&region_id).cloned(),
            in_pabr_rid_dictionary: rid_dictionary_region_ids.contains(&region_id),
            in_pabr_bkd_rows: pabr_used_region_ids.contains(&region_id),
            in_pabr_active_area: pabr_active_region_ids.contains(&region_id),
            in_current_regions_geojson: current_region_ids.contains(&region_id),
            in_current_regioninfo_json: current_regioninfo_ids
                .map(|current_regioninfo_ids| current_regioninfo_ids.contains(&region_id)),
            regionclientdata_variants: regionclientdata
                .iter()
                .filter(|variant| variant.region_ids.contains(&region_id))
                .map(|variant| variant.variant.clone())
                .collect(),
        })
        .collect()
}

fn load_current_regions(path: &Path) -> Result<(BTreeSet<u32>, BTreeMap<u32, String>)> {
    let file = File::open(path)
        .with_context(|| format!("failed to open current regions GeoJSON {}", path.display()))?;
    let collection: CurrentFeatureCollection = serde_json::from_reader(file)
        .with_context(|| format!("failed to parse current regions GeoJSON {}", path.display()))?;

    let mut region_ids = BTreeSet::new();
    let mut labels = BTreeMap::new();
    for feature in collection.features {
        let Some(region_id) = extract_u32_property(&feature.properties, "r") else {
            continue;
        };

        region_ids.insert(region_id);
        if let Some(label) = feature
            .properties
            .get("on")
            .and_then(Value::as_str)
            .map(str::to_owned)
        {
            labels.entry(region_id).or_insert(label);
        }
    }

    Ok((region_ids, labels))
}

fn load_current_regioninfo_ids(path: &Path) -> Result<BTreeSet<u32>> {
    let rows = load_current_regioninfo_rows(path)?;

    let mut region_ids = BTreeSet::new();
    for (key, row) in rows {
        if row.key != 0 {
            region_ids.insert(row.key);
            continue;
        }

        if let Ok(region_id) = key.parse::<u32>() {
            region_ids.insert(region_id);
        }
    }

    Ok(region_ids)
}

fn load_current_regioninfo_rows(path: &Path) -> Result<BTreeMap<String, RegionInfoRow>> {
    let file = File::open(path)
        .with_context(|| format!("failed to open current regioninfo JSON {}", path.display()))?;
    serde_json::from_reader(file)
        .with_context(|| format!("failed to parse current regioninfo JSON {}", path.display()))
}

fn load_current_regioninfo_rows_by_key(path: &Path) -> Result<BTreeMap<u32, RegionInfoRow>> {
    let rows = load_current_regioninfo_rows(path)?;
    let mut rows_by_key = BTreeMap::new();
    for (key, row) in rows {
        if row.key != 0 {
            rows_by_key.insert(row.key, row);
            continue;
        }
        if let Ok(region_id) = key.parse::<u32>() {
            rows_by_key.insert(region_id, row);
        }
    }
    Ok(rows_by_key)
}

fn load_current_regiongroup_graph_rows_by_key(
    path: &Path,
) -> Result<BTreeMap<u32, CurrentRegionGroupGraphRow>> {
    let file = File::open(path).with_context(|| {
        format!(
            "failed to open current deck_rg_graphs JSON {}",
            path.display()
        )
    })?;
    let rows: Vec<CurrentRegionGroupGraphRow> =
        serde_json::from_reader(file).with_context(|| {
            format!(
                "failed to parse current deck_rg_graphs JSON {}",
                path.display()
            )
        })?;
    let mut rows_by_key = BTreeMap::new();
    for row in rows {
        rows_by_key.insert(row.key, row);
    }
    Ok(rows_by_key)
}

fn load_localization(path: &Path) -> Result<LocalizationFile> {
    if !path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("loc"))
    {
        bail!(
            "expected original localization .loc file, got {}",
            path.display()
        );
    }

    let maps = load_loc_namespaces_as_string_maps(path, &[17, 29], 10_000).with_context(|| {
        format!(
            "failed to load localization namespaces from {}",
            path.display()
        )
    })?;
    Ok(LocalizationFile {
        en: LocalizationTable {
            node: maps.get(&29).cloned().unwrap_or_default(),
            town: maps.get(&17).cloned().unwrap_or_default(),
        },
    })
}

fn decode_regioninfo_bss_signature_rows(
    bytes: &[u8],
    localization: Option<&LocalizationFile>,
) -> Result<BTreeMap<u32, DecodedRegionInfoBssRow>> {
    if bytes.len() < 8 {
        bail!("regioninfo.bss is too small");
    }
    if &bytes[0..4] != PABR_MAGIC {
        bail!("regioninfo.bss is missing PABR magic");
    }

    let mut rows = BTreeMap::new();
    let mut search_from = 0usize;
    while let Some(relative_hit) = bytes[search_from..]
        .windows(REGIONINFO_ROW_SIGNATURE_PREFIX.len())
        .position(|window| window == REGIONINFO_ROW_SIGNATURE_PREFIX)
    {
        let signature_offset = search_from + relative_hit;
        let Some(row_start_offset) = signature_offset.checked_sub(REGIONINFO_ROW_SIGNATURE_OFFSET)
        else {
            search_from = signature_offset + 1;
            continue;
        };
        if row_start_offset + REGIONINFO_ROW_MIN_LEN > bytes.len() {
            break;
        }

        let key = u32::from(u16::from_le_bytes([
            bytes[row_start_offset],
            bytes[row_start_offset + 1],
        ]));
        if key == 0 {
            search_from = signature_offset + 1;
            continue;
        }

        let tradeoriginregion = u32::from(u16::from_le_bytes([
            bytes[row_start_offset + REGIONINFO_ROW_TRADE_ORIGIN_OFFSET],
            bytes[row_start_offset + REGIONINFO_ROW_TRADE_ORIGIN_OFFSET + 1],
        ]));
        let regiongroup = u32::from(u16::from_le_bytes([
            bytes[row_start_offset + REGIONINFO_ROW_GROUP_OFFSET],
            bytes[row_start_offset + REGIONINFO_ROW_GROUP_OFFSET + 1],
        ]));
        let waypoint = decode_shifted_u32_field(
            bytes,
            row_start_offset + REGIONINFO_ROW_WAYPOINT_PRIMARY_OFFSET,
        )?
        .or(decode_shifted_u32_field(
            bytes,
            row_start_offset + REGIONINFO_ROW_WAYPOINT_SECONDARY_OFFSET,
        )?);
        let ware_house_character_key = decode_unaligned_u32_field(
            bytes,
            row_start_offset + REGIONINFO_ROW_WAREHOUSE_CHARACTER_OFFSET,
        )?;
        let npc_worker_character_key = decode_unaligned_u32_field(
            bytes,
            row_start_offset + REGIONINFO_ROW_NPC_WORKER_CHARACTER_OFFSET,
        )?;
        let origin_label = localization
            .and_then(|localization| resolve_origin_label(localization, tradeoriginregion));

        rows.entry(key).or_insert(DecodedRegionInfoBssRow {
            key,
            row_start_offset,
            tradeoriginregion,
            regiongroup,
            waypoint,
            ware_house_character_key,
            npc_worker_character_key,
            origin_label,
        });
        search_from = signature_offset + 1;
    }

    Ok(rows)
}

fn decode_regiongroupinfo_bss_rows(
    bytes: &[u8],
) -> Result<BTreeMap<u32, DecodedRegionGroupInfoBssRow>> {
    if bytes.len() < 8 + PABR_TRAILER_LEN {
        bail!("regiongroupinfo.bss is too small");
    }
    if &bytes[0..4] != PABR_MAGIC {
        bail!("regiongroupinfo.bss is missing PABR magic");
    }

    let entry_count = usize::try_from(u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]))
        .context("regiongroupinfo.bss entry count does not fit usize")?;
    let payload_len = bytes
        .len()
        .checked_sub(8 + PABR_TRAILER_LEN)
        .context("regiongroupinfo.bss payload underflow")?;
    let expected_payload_len = entry_count
        .checked_mul(REGIONGROUPINFO_ROW_LEN)
        .context("regiongroupinfo.bss payload size overflow")?;
    if payload_len != expected_payload_len {
        bail!(
            "regiongroupinfo.bss payload length mismatch: expected {} bytes for {} rows, got {}",
            expected_payload_len,
            entry_count,
            payload_len
        );
    }

    let mut rows = BTreeMap::new();
    for row_index in 0..entry_count {
        let row_start_offset = 8 + row_index * REGIONGROUPINFO_ROW_LEN;
        let row = &bytes[row_start_offset..row_start_offset + REGIONGROUPINFO_ROW_LEN];
        let key = u32::from(u16::from_le_bytes([row[0], row[1]]));
        if key == 0 {
            continue;
        }

        let waypoint = {
            let raw = u32::from_le_bytes([
                row[REGIONGROUPINFO_ROW_WAYPOINT_OFFSET],
                row[REGIONGROUPINFO_ROW_WAYPOINT_OFFSET + 1],
                row[REGIONGROUPINFO_ROW_WAYPOINT_OFFSET + 2],
                row[REGIONGROUPINFO_ROW_WAYPOINT_OFFSET + 3],
            ]);
            (raw != 0).then_some(raw)
        };
        let graphx = f32::from_le_bytes([
            row[REGIONGROUPINFO_ROW_GRAPHX_OFFSET],
            row[REGIONGROUPINFO_ROW_GRAPHX_OFFSET + 1],
            row[REGIONGROUPINFO_ROW_GRAPHX_OFFSET + 2],
            row[REGIONGROUPINFO_ROW_GRAPHX_OFFSET + 3],
        ]);
        let graphy = f32::from_le_bytes([
            row[REGIONGROUPINFO_ROW_GRAPHY_OFFSET],
            row[REGIONGROUPINFO_ROW_GRAPHY_OFFSET + 1],
            row[REGIONGROUPINFO_ROW_GRAPHY_OFFSET + 2],
            row[REGIONGROUPINFO_ROW_GRAPHY_OFFSET + 3],
        ]);
        let graphz = f32::from_le_bytes([
            row[REGIONGROUPINFO_ROW_GRAPHZ_OFFSET],
            row[REGIONGROUPINFO_ROW_GRAPHZ_OFFSET + 1],
            row[REGIONGROUPINFO_ROW_GRAPHZ_OFFSET + 2],
            row[REGIONGROUPINFO_ROW_GRAPHZ_OFFSET + 3],
        ]);
        let has_graph_position = graphx != 0.0 || graphy != 0.0 || graphz != 0.0;
        let flag_bytes_hex = row
            [REGIONGROUPINFO_ROW_FLAGS_OFFSET..REGIONGROUPINFO_ROW_FLAGS_OFFSET + 3]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();

        rows.insert(
            key,
            DecodedRegionGroupInfoBssRow {
                key,
                row_index,
                row_start_offset,
                waypoint,
                graphx,
                graphy,
                graphz,
                has_graph_position,
                flag_bytes_hex,
            },
        );
    }

    Ok(rows)
}

fn decode_shifted_u32_field(bytes: &[u8], offset: usize) -> Result<Option<u32>> {
    let raw = read_unaligned_u32(bytes, offset)?;
    if raw == 0 {
        return Ok(None);
    }
    if raw & 0xFF != 0 {
        return Ok(None);
    }
    Ok(Some(raw >> 8))
}

fn decode_unaligned_u32_field(bytes: &[u8], offset: usize) -> Result<Option<u32>> {
    let raw = read_unaligned_u32(bytes, offset)?;
    if raw == 0 {
        Ok(None)
    } else {
        Ok(Some(raw))
    }
}

fn read_unaligned_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let end = offset
        .checked_add(4)
        .context("u32 offset overflow while parsing regioninfo.bss")?;
    let slice = bytes
        .get(offset..end)
        .with_context(|| format!("u32 read at offset {} is out of bounds", offset))?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn resolve_origin_label(localization: &LocalizationFile, origin_region_id: u32) -> Option<String> {
    let key = origin_region_id.to_string();
    localization
        .en
        .town
        .get(&key)
        .cloned()
        .or_else(|| localization.en.node.get(&key).cloned())
}

fn current_regioninfo_row_summary(row: &RegionInfoRow) -> CurrentRegionInfoRowSummary {
    CurrentRegionInfoRowSummary {
        key: row.key,
        is_accessible: row.is_accessible,
        tradeoriginregion: row.tradeoriginregion,
        regiongroup: row.regiongroup,
        waypoint: row.waypoint,
        ware_house_character_key: row.ware_house_character_key,
        npc_worker_character_key: row.npc_worker_character_key,
    }
}

fn current_regiongroup_graph_row_summary(
    row: &CurrentRegionGroupGraphRow,
) -> CurrentRegionGroupGraphRowSummary {
    CurrentRegionGroupGraphRowSummary {
        key: row.key,
        waypoint: row.waypoint,
        graphx: row.graphx,
        graphy: row.graphy,
        graphz: row.graphz,
    }
}

fn load_regionclientdata_variants(paths: &[PathBuf]) -> Result<Vec<RegionClientDataVariant>> {
    let mut variants = paths
        .iter()
        .map(|path| load_regionclientdata_variant(path))
        .collect::<Result<Vec<_>>>()?;
    variants.sort_by(|left, right| left.variant.cmp(&right.variant));
    Ok(variants)
}

fn load_regionclientdata_variant(path: &Path) -> Result<RegionClientDataVariant> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read regionclientdata XML {}", path.display()))?;
    let region_ids = parse_regionclientdata_region_ids(&contents)?;

    Ok(RegionClientDataVariant {
        variant: infer_regionclientdata_variant(path),
        path: path.to_path_buf(),
        region_ids,
    })
}

fn parse_regionclientdata_region_ids(contents: &str) -> Result<BTreeSet<u32>> {
    const NEEDLE: &str = "<RegionInfo Key=\"";

    let mut region_ids = BTreeSet::new();
    let mut search_from = 0usize;
    while let Some(relative_start) = contents[search_from..].find(NEEDLE) {
        let id_start = search_from + relative_start + NEEDLE.len();
        let relative_end = contents[id_start..]
            .find('"')
            .context("unterminated RegionInfo Key attribute")?;
        let id_end = id_start + relative_end;
        let raw_id = &contents[id_start..id_end];
        let region_id = raw_id
            .parse::<u32>()
            .with_context(|| format!("failed to parse RegionInfo Key `{raw_id}` as u32"))?;
        region_ids.insert(region_id);
        search_from = id_end;
    }

    Ok(region_ids)
}

fn infer_regionclientdata_variant(path: &Path) -> String {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if let Some(rest) = file_name.strip_prefix("regionclientdata_") {
        if let Some(rest) = rest.strip_suffix("_.xml") {
            return rest.to_string();
        }
    }

    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn extract_u32_property(properties: &Map<String, Value>, key: &str) -> Option<u32> {
    properties
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
}

fn write_json_report<T: Serialize>(path: &Path, report: &T) -> Result<()> {
    let file = File::create(path)
        .with_context(|| format!("failed to create report {}", path.display()))?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, report)
        .with_context(|| format!("failed to write report {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::{infer_regionclientdata_variant, parse_regionclientdata_region_ids};
    use std::path::Path;

    #[test]
    fn parses_regionclientdata_keys() {
        let contents = r#"
            <Root>
              <RegionInfo Key="1677"></RegionInfo>
              <RegionInfo Key="1150"><SpawnInfo key="1" /></RegionInfo>
              <RegionInfo Key="1677"></RegionInfo>
            </Root>
        "#;

        let region_ids = parse_regionclientdata_region_ids(contents).unwrap();
        assert_eq!(region_ids.into_iter().collect::<Vec<_>>(), vec![1150, 1677]);
    }

    #[test]
    fn infers_regionclientdata_variant_from_filename() {
        assert_eq!(
            infer_regionclientdata_variant(Path::new("regionclientdata_sa_.xml")),
            "sa"
        );
        assert_eq!(
            infer_regionclientdata_variant(Path::new("custom-name.xml")),
            "custom-name"
        );
    }
}
