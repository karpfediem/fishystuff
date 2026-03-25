use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::coord::world_to_pixel_f;
use crate::field::DiscreteFieldRows;
use crate::field_metadata::{
    FieldDetailFact, FieldDetailPaneRef, FieldDetailSection, FieldHoverMetadataEntry,
    FieldHoverTarget, FIELD_DETAIL_FACT_KEY_ORIGIN_NODE, FIELD_DETAIL_FACT_KEY_ORIGIN_REGION,
    FIELD_DETAIL_FACT_KEY_REGION, FIELD_DETAIL_FACT_KEY_REGION_NODE,
    FIELD_DETAIL_FACT_KEY_RESOURCE_GROUP, FIELD_DETAIL_FACT_KEY_RESOURCE_REGION,
    FIELD_DETAIL_FACT_KEY_RESOURCE_WAYPOINT, FIELD_DETAIL_PANE_ID_TERRITORY,
    FIELD_DETAIL_PANE_ID_ZONE_MASK, FIELD_DETAIL_SECTION_KIND_FACTS,
    FIELD_HOVER_TARGET_KEY_ORIGIN_NODE, FIELD_HOVER_TARGET_KEY_RESOURCE_NODE,
};
use crate::loc::load_loc_namespaces_as_string_maps;

const PABR_MAGIC: &[u8; 4] = b"PABR";
const REGIONINFO_ROW_SIGNATURE_PREFIX: [u8; 4] = [0x5A, 0x55, 0x00, 0x00];
const REGIONINFO_ROW_SIGNATURE_OFFSET: usize = 32;
const REGIONINFO_ROW_ACCESSIBLE_OFFSET: usize = 27;
const REGIONINFO_ROW_TRADE_ORIGIN_OFFSET: usize = 102;
const REGIONINFO_ROW_GROUP_OFFSET: usize = 104;
const REGIONINFO_ROW_WAYPOINT_PRIMARY_OFFSET: usize = 106;
const REGIONINFO_ROW_MIN_LEN: usize = 193;
const REGIONGROUPINFO_ROW_LEN: usize = 51;
const REGIONGROUPINFO_ROW_WAYPOINT_OFFSET: usize = 5;
const REGIONGROUPINFO_ROW_GRAPHX_OFFSET: usize = 12;
const REGIONGROUPINFO_ROW_GRAPHY_OFFSET: usize = 16;
const REGIONGROUPINFO_ROW_GRAPHZ_OFFSET: usize = 20;
const PABR_TRAILER_LEN: usize = 12;

#[derive(Debug, Clone, Default)]
struct LocalizationTable {
    node: BTreeMap<String, String>,
    town: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct OriginalRegionInfoRow {
    pub key: u32,
    pub is_accessible: bool,
    pub tradeoriginregion: u32,
    pub regiongroup: u32,
    pub waypoint: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct OriginalRegionGroupInfoRow {
    pub key: u32,
    pub waypoint: Option<u32>,
    pub graphx: f64,
    pub graphy: f64,
    pub graphz: f64,
}

#[derive(Debug, Clone)]
pub struct OriginalWaypointRow {
    pub key: u32,
    pub raw_name: String,
    pub pos_x: f64,
    pub pos_y: f64,
    pub pos_z: f64,
    is_sub_waypoint: bool,
    source_index: usize,
}

#[derive(Debug, Clone)]
pub struct RegionGroupWaypointInfo {
    pub waypoint_id: Option<u32>,
    pub waypoint_name: Option<String>,
    pub world_x: Option<f64>,
    pub world_z: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct RegionOriginInfo {
    pub region_id: Option<u32>,
    pub waypoint_id: Option<u32>,
    pub world_x: Option<f64>,
    pub world_z: Option<f64>,
    pub region_name: Option<String>,
    pub waypoint_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaypointNodeType {
    City,
    Town,
    Village,
    TradingPost,
    WorkerNode,
    Gateway,
    Castle,
    Connection,
    Island,
    Sea,
    Danger,
}

impl WaypointNodeType {
    pub fn key(self) -> &'static str {
        match self {
            Self::City => "city",
            Self::Town => "town",
            Self::Village => "village",
            Self::TradingPost => "trading_post",
            Self::WorkerNode => "worker_node",
            Self::Gateway => "gateway",
            Self::Castle => "castle",
            Self::Connection => "connection",
            Self::Island => "island",
            Self::Sea => "sea",
            Self::Danger => "danger",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::City => "City",
            Self::Town => "Town",
            Self::Village => "Village",
            Self::TradingPost => "Trading Post",
            Self::WorkerNode => "Worker Node",
            Self::Gateway => "Gateway",
            Self::Castle => "Castle",
            Self::Connection => "Connection",
            Self::Island => "Island",
            Self::Sea => "Sea",
            Self::Danger => "Danger",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaypointDisplayClass {
    Main,
    Hidden,
    SubWaypoint,
}

impl WaypointDisplayClass {
    pub fn key(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Hidden => "hidden",
            Self::SubWaypoint => "subwaypoint",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Main => "Main",
            Self::Hidden => "Hidden",
            Self::SubWaypoint => "Sub-Waypoint",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaypointDisplayInfo {
    pub display_class: WaypointDisplayClass,
    pub referenced_by_region: bool,
    pub referenced_by_region_group: bool,
    pub default_visible: bool,
}

#[derive(Debug, Clone, Default)]
pub struct RegionGroupMapping {
    region_to_group: BTreeMap<u16, u16>,
    group_to_regions: BTreeMap<u16, Vec<u16>>,
}

impl RegionGroupMapping {
    pub fn region_group_for_region(&self, region_id: u16) -> Option<u16> {
        self.region_to_group.get(&region_id).copied()
    }

    pub fn region_ids_for_group(&self, group_id: u16) -> &[u16] {
        self.group_to_regions
            .get(&group_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

#[derive(Debug, Clone)]
pub struct OriginalRegionLayerContext {
    regioninfo: HashMap<u32, OriginalRegionInfoRow>,
    regiongroupinfo: HashMap<u32, OriginalRegionGroupInfoRow>,
    waypoints: HashMap<u32, OriginalWaypointRow>,
    waypoint_links: HashMap<u32, Vec<u32>>,
    localization: LocalizationTable,
}

#[derive(Debug, Clone)]
struct OriginalWaypointGraph {
    rows: HashMap<u32, OriginalWaypointRow>,
    links: HashMap<u32, Vec<u32>>,
}

impl OriginalRegionLayerContext {
    pub fn region_ids(&self) -> Vec<u32> {
        let mut ids: Vec<u32> = self.regioninfo.keys().copied().collect();
        ids.sort_unstable();
        ids
    }

    pub fn region_group_for_region(&self, region_id: u32) -> Option<u32> {
        self.regioninfo
            .get(&region_id)
            .and_then(|row| non_zero_u32(row.regiongroup))
    }

    pub fn region_node_connection_pairs(&self) -> Vec<(u32, u32)> {
        let mut waypoint_to_regions = BTreeMap::<u32, Vec<u32>>::new();
        for region_id in self.region_ids() {
            let Some(waypoint_id) = self
                .regioninfo
                .get(&region_id)
                .and_then(|row| row.waypoint)
                .filter(|waypoint_id| self.waypoints.contains_key(waypoint_id))
            else {
                continue;
            };
            waypoint_to_regions
                .entry(waypoint_id)
                .or_default()
                .push(region_id);
        }

        let main_waypoints = waypoint_to_regions.keys().copied().collect::<BTreeSet<_>>();
        let mut pairs = BTreeSet::new();
        for (waypoint_id, neighbors) in &self.waypoint_links {
            if !main_waypoints.contains(waypoint_id) {
                continue;
            }
            let Some(from_regions) = waypoint_to_regions.get(waypoint_id) else {
                continue;
            };
            for neighbor in neighbors {
                if !main_waypoints.contains(neighbor) || *waypoint_id >= *neighbor {
                    continue;
                }
                let Some(to_regions) = waypoint_to_regions.get(neighbor) else {
                    continue;
                };
                for from_region_id in from_regions {
                    for to_region_id in to_regions {
                        if from_region_id == to_region_id {
                            continue;
                        }
                        let pair = if from_region_id < to_region_id {
                            (*from_region_id, *to_region_id)
                        } else {
                            (*to_region_id, *from_region_id)
                        };
                        pairs.insert(pair);
                    }
                }
            }
        }

        pairs.into_iter().collect()
    }

    pub fn resolve_region_waypoint_info(&self, region_id: u32) -> Option<RegionOriginInfo> {
        let row = self.regioninfo.get(&region_id)?;
        let waypoint_id = row.waypoint;
        let waypoint = waypoint_id.and_then(|id| self.waypoints.get(&id));
        let info = RegionOriginInfo {
            region_id: Some(region_id),
            waypoint_id,
            world_x: waypoint.map(|waypoint| waypoint.pos_x),
            world_z: waypoint.map(|waypoint| waypoint.pos_z),
            region_name: resolve_region_name(&self.localization, Some(region_id)),
            waypoint_name: waypoint_id.and_then(|id| self.resolve_waypoint_name(id)),
        };
        info.has_value().then_some(info)
    }

    pub fn resolve_region_waypoint_node_type(&self, region_id: u32) -> Option<WaypointNodeType> {
        let waypoint_id = self.regioninfo.get(&region_id)?.waypoint?;
        self.resolve_waypoint_node_type(waypoint_id)
    }

    pub fn resolve_region_origin_info(&self, region_id: u32) -> Option<RegionOriginInfo> {
        let row = self.regioninfo.get(&region_id)?;
        let origin_region_id = non_zero_u32(row.tradeoriginregion);
        origin_region_id
            .and_then(|origin_region_id| self.resolve_region_waypoint_info(origin_region_id))
    }

    pub fn resolve_resource_waypoint(
        &self,
        region_group_id: u32,
    ) -> Option<RegionGroupWaypointInfo> {
        let row = self.regiongroupinfo.get(&region_group_id)?;
        let info = RegionGroupWaypointInfo {
            waypoint_id: row.waypoint,
            waypoint_name: row.waypoint.and_then(|id| self.resolve_waypoint_name(id)),
            world_x: Some(row.graphx).filter(|value| *value != 0.0),
            world_z: Some(row.graphz).filter(|value| *value != 0.0),
        };
        info.has_value().then_some(info)
    }

    pub fn resolve_region_hover_metadata(&self, region_id: u32) -> Option<FieldHoverMetadataEntry> {
        let region = self.resolve_region_waypoint_info(region_id);
        let origin = self.resolve_region_origin_info(region_id);
        let entry = FieldHoverMetadataEntry {
            targets: vec![build_origin_hover_target(origin.as_ref())]
                .into_iter()
                .flatten()
                .collect(),
            detail_pane: Some(territory_detail_pane_ref("hover-zone")),
            detail_sections: vec![
                build_region_detail_section(region_id, region.as_ref()),
                build_region_origin_detail_section(origin.as_ref()),
            ]
            .into_iter()
            .flatten()
            .collect(),
        };
        entry.has_value().then_some(entry)
    }

    pub fn resolve_region_group_hover_metadata(
        &self,
        region_group_id: u32,
        regions_field: &DiscreteFieldRows,
    ) -> Option<FieldHoverMetadataEntry> {
        let resource = self.resolve_resource_waypoint(region_group_id);
        let resource_region_id = resource
            .as_ref()
            .and_then(|info| self.resolve_resource_region_id(info, regions_field));
        let resource_region_info =
            resource_region_id.and_then(|region_id| self.resolve_region_waypoint_info(region_id));
        let entry = FieldHoverMetadataEntry {
            targets: vec![build_resource_hover_target(
                region_group_id,
                resource.as_ref(),
                resource_region_id,
                resource_region_info.as_ref(),
            )]
            .into_iter()
            .flatten()
            .collect(),
            detail_pane: Some(territory_detail_pane_ref("hover-resources")),
            detail_sections: vec![build_region_group_resource_detail_section(
                region_group_id,
                resource.as_ref(),
                resource_region_id,
                resource_region_info.as_ref(),
            )]
            .into_iter()
            .flatten()
            .collect(),
        };
        entry.has_value().then_some(entry)
    }

    pub fn resolve_waypoint_name(&self, waypoint_id: u32) -> Option<String> {
        localized_name(&self.localization.node, waypoint_id).or_else(|| {
            self.waypoints
                .get(&waypoint_id)
                .map(|row| row.raw_name.trim())
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
    }

    pub fn resolve_waypoint_position(&self, waypoint_id: u32) -> Option<(f64, f64)> {
        let waypoint = self.waypoints.get(&waypoint_id)?;
        Some((waypoint.pos_x, waypoint.pos_z))
    }

    pub fn resolve_waypoint_node_type(&self, waypoint_id: u32) -> Option<WaypointNodeType> {
        let waypoint = self.waypoints.get(&waypoint_id)?;
        classify_waypoint_node_type(&waypoint.raw_name)
    }

    pub fn resolve_waypoint_display_info(&self, waypoint_id: u32) -> Option<WaypointDisplayInfo> {
        let waypoint = self.waypoints.get(&waypoint_id)?;
        let referenced_by_region = self
            .regioninfo
            .values()
            .any(|row| row.waypoint == Some(waypoint_id));
        let referenced_by_region_group = self
            .regiongroupinfo
            .values()
            .any(|row| row.waypoint == Some(waypoint_id));
        let display_class = if waypoint.is_sub_waypoint {
            WaypointDisplayClass::SubWaypoint
        } else if is_hidden_waypoint_name(&waypoint.raw_name) {
            WaypointDisplayClass::Hidden
        } else {
            WaypointDisplayClass::Main
        };
        Some(WaypointDisplayInfo {
            display_class,
            referenced_by_region,
            referenced_by_region_group,
            default_visible: display_class == WaypointDisplayClass::Main
                || referenced_by_region
                || referenced_by_region_group,
        })
    }

    fn resolve_resource_region_id(
        &self,
        resource: &RegionGroupWaypointInfo,
        regions_field: &DiscreteFieldRows,
    ) -> Option<u32> {
        resource
            .waypoint_id
            .and_then(|waypoint_id| self.resolve_waypoint_position(waypoint_id))
            .and_then(|(world_x, world_z)| {
                sample_field_id_at_world(regions_field, world_x, world_z)
            })
            .or_else(|| match (resource.world_x, resource.world_z) {
                (Some(world_x), Some(world_z)) => {
                    sample_field_id_at_world(regions_field, world_x, world_z)
                }
                _ => None,
            })
    }
}

impl RegionOriginInfo {
    pub fn has_value(&self) -> bool {
        self.region_id.is_some()
            || self.waypoint_id.is_some()
            || self.world_x.is_some()
            || self.world_z.is_some()
            || self.region_name.is_some()
            || self.waypoint_name.is_some()
    }
}

impl RegionGroupWaypointInfo {
    pub fn has_value(&self) -> bool {
        self.waypoint_id.is_some()
            || self.waypoint_name.is_some()
            || self.world_x.is_some()
            || self.world_z.is_some()
    }
}

pub fn load_original_region_layer_context(
    regioninfo_bss_path: &Path,
    regiongroupinfo_bss_path: &Path,
    waypoint_xml_paths: &[PathBuf],
    loc_path: &Path,
) -> Result<OriginalRegionLayerContext> {
    if waypoint_xml_paths.is_empty() {
        bail!("at least one waypoint XML path is required");
    }

    let regioninfo = decode_regioninfo_bss_rows_from_path(regioninfo_bss_path)?
        .into_iter()
        .map(|row| (row.key, row))
        .collect();
    let regiongroupinfo = decode_regiongroupinfo_bss_rows_from_path(regiongroupinfo_bss_path)?
        .into_iter()
        .map(|row| (row.key, row))
        .collect();
    let waypoint_graph = load_waypoint_graph(waypoint_xml_paths)?;
    let localization = load_localization(loc_path)?;

    Ok(OriginalRegionLayerContext {
        regioninfo,
        regiongroupinfo,
        waypoints: waypoint_graph.rows,
        waypoint_links: waypoint_graph.links,
        localization,
    })
}

pub fn load_region_group_mapping_from_regioninfo_bss(path: &Path) -> Result<RegionGroupMapping> {
    let rows = load_original_regioninfo_rows(path)?;
    let mut region_to_group = BTreeMap::new();
    let mut group_to_regions: BTreeMap<u16, Vec<u16>> = BTreeMap::new();

    for row in rows {
        let region_id =
            u16::try_from(row.key).with_context(|| format!("region id {} exceeds u16", row.key))?;
        let group_id = u16::try_from(row.regiongroup)
            .with_context(|| format!("region-group id {} exceeds u16", row.regiongroup))?;
        region_to_group.insert(region_id, group_id);
        group_to_regions
            .entry(group_id)
            .or_default()
            .push(region_id);
    }

    for region_ids in group_to_regions.values_mut() {
        region_ids.sort_unstable();
        region_ids.dedup();
    }

    Ok(RegionGroupMapping {
        region_to_group,
        group_to_regions,
    })
}

pub fn load_original_regioninfo_rows(path: &Path) -> Result<Vec<OriginalRegionInfoRow>> {
    decode_regioninfo_bss_rows_from_path(path)
}

pub fn load_original_regiongroupinfo_rows(path: &Path) -> Result<Vec<OriginalRegionGroupInfoRow>> {
    decode_regiongroupinfo_bss_rows_from_path(path)
}

fn decode_regioninfo_bss_rows_from_path(path: &Path) -> Result<Vec<OriginalRegionInfoRow>> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read regioninfo.bss {}", path.display()))?;
    decode_regioninfo_bss_rows(&bytes)
}

fn decode_regiongroupinfo_bss_rows_from_path(
    path: &Path,
) -> Result<Vec<OriginalRegionGroupInfoRow>> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read regiongroupinfo.bss {}", path.display()))?;
    decode_regiongroupinfo_bss_rows(&bytes)
}

fn decode_regioninfo_bss_rows(bytes: &[u8]) -> Result<Vec<OriginalRegionInfoRow>> {
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
        )?;

        rows.entry(key).or_insert(OriginalRegionInfoRow {
            key,
            is_accessible: bytes[row_start_offset + REGIONINFO_ROW_ACCESSIBLE_OFFSET] == 1,
            tradeoriginregion,
            regiongroup,
            waypoint,
        });
        search_from = signature_offset + 1;
    }

    Ok(rows.into_values().collect())
}

fn decode_regiongroupinfo_bss_rows(bytes: &[u8]) -> Result<Vec<OriginalRegionGroupInfoRow>> {
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

    let mut rows = Vec::new();
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
        ]) as f64;
        let graphy = f32::from_le_bytes([
            row[REGIONGROUPINFO_ROW_GRAPHY_OFFSET],
            row[REGIONGROUPINFO_ROW_GRAPHY_OFFSET + 1],
            row[REGIONGROUPINFO_ROW_GRAPHY_OFFSET + 2],
            row[REGIONGROUPINFO_ROW_GRAPHY_OFFSET + 3],
        ]) as f64;
        let graphz = f32::from_le_bytes([
            row[REGIONGROUPINFO_ROW_GRAPHZ_OFFSET],
            row[REGIONGROUPINFO_ROW_GRAPHZ_OFFSET + 1],
            row[REGIONGROUPINFO_ROW_GRAPHZ_OFFSET + 2],
            row[REGIONGROUPINFO_ROW_GRAPHZ_OFFSET + 3],
        ]) as f64;

        rows.push(OriginalRegionGroupInfoRow {
            key,
            waypoint,
            graphx,
            graphy,
            graphz,
        });
    }

    Ok(rows)
}

fn load_waypoint_graph(paths: &[PathBuf]) -> Result<OriginalWaypointGraph> {
    let mut rows = HashMap::new();
    let mut links = HashMap::<u32, BTreeSet<u32>>::new();
    for (source_index, path) in paths.iter().enumerate() {
        let bytes = fs::read(path)
            .with_context(|| format!("failed to read waypoint XML {}", path.display()))?;
        let contents = String::from_utf8_lossy(&bytes);
        for raw_line in contents.lines() {
            let line = raw_line.trim();
            if line.starts_with("<Waypoint ") {
                let key = parse_attr_u32(line, "Key").with_context(|| {
                    format!("failed to parse waypoint key in {}", path.display())
                })?;
                let candidate = OriginalWaypointRow {
                    key,
                    raw_name: parse_attr_string(line, "Name").with_context(|| {
                        format!("failed to parse waypoint name in {}", path.display())
                    })?,
                    pos_x: parse_attr_f64(line, "PosX").with_context(|| {
                        format!("failed to parse waypoint PosX in {}", path.display())
                    })?,
                    pos_y: parse_attr_f64(line, "PosY").with_context(|| {
                        format!("failed to parse waypoint PosY in {}", path.display())
                    })?,
                    pos_z: parse_attr_f64(line, "PosZ").with_context(|| {
                        format!("failed to parse waypoint PosZ in {}", path.display())
                    })?,
                    is_sub_waypoint: parse_attr_bool(line, "IsSubWaypoint").with_context(|| {
                        format!(
                            "failed to parse waypoint IsSubWaypoint in {}",
                            path.display()
                        )
                    })?,
                    source_index,
                };
                rows.entry(key)
                    .and_modify(|existing| {
                        if should_replace_waypoint_row(existing, &candidate) {
                            *existing = candidate.clone();
                        }
                    })
                    .or_insert(candidate);
                continue;
            }
            if !line.starts_with("<Link ") {
                continue;
            }
            let source_waypoint = parse_attr_u32(line, "SourceWaypoint").with_context(|| {
                format!("failed to parse link SourceWaypoint in {}", path.display())
            })?;
            let target_waypoint = parse_attr_u32(line, "TargetWaypoint").with_context(|| {
                format!("failed to parse link TargetWaypoint in {}", path.display())
            })?;
            if source_waypoint == 0 || target_waypoint == 0 || source_waypoint == target_waypoint {
                continue;
            }
            links
                .entry(source_waypoint)
                .or_default()
                .insert(target_waypoint);
            links
                .entry(target_waypoint)
                .or_default()
                .insert(source_waypoint);
        }
    }
    Ok(OriginalWaypointGraph {
        rows,
        links: links
            .into_iter()
            .map(|(key, neighbors)| (key, neighbors.into_iter().collect()))
            .collect(),
    })
}

fn should_replace_waypoint_row(
    existing: &OriginalWaypointRow,
    candidate: &OriginalWaypointRow,
) -> bool {
    match (existing.is_sub_waypoint, candidate.is_sub_waypoint) {
        (true, false) => true,
        (false, true) => false,
        _ => candidate.source_index >= existing.source_index,
    }
}

fn load_localization(path: &Path) -> Result<LocalizationTable> {
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

    let maps = load_loc_namespaces_as_string_maps(path, &[17, 29], 10_000)
        .with_context(|| format!("load localization namespaces from {}", path.display()))?;
    Ok(LocalizationTable {
        node: maps.get(&29).cloned().unwrap_or_default(),
        town: maps.get(&17).cloned().unwrap_or_default(),
    })
}

fn build_region_detail_section(
    region_id: u32,
    region: Option<&RegionOriginInfo>,
) -> Option<FieldDetailSection> {
    let mut facts = Vec::new();
    let region_value = region
        .and_then(|info| info.region_name.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|name| format_region_name_with_id(name, region_id))
        .unwrap_or_else(|| format!("R{region_id}"));
    facts.push(FieldDetailFact {
        key: FIELD_DETAIL_FACT_KEY_REGION.to_string(),
        label: "Region".to_string(),
        value: region_value,
        icon: Some("hover-zone".to_string()),
        status_icon: region.and_then(|info| {
            let has_assignment = info.has_value();
            let has_name = info
                .region_name
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty());
            match (has_assignment, has_name) {
                (true, true) => None,
                (true, false) => Some("question-mark".to_string()),
                (false, _) => Some("question-mark".to_string()),
            }
        }),
        status_icon_tone: region.and_then(|info| {
            let has_assignment = info.has_value();
            let has_name = info
                .region_name
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty());
            match (has_assignment, has_name) {
                (true, false) => Some("subtle".to_string()),
                _ => None,
            }
        }),
    });

    if let Some(node_name) = region
        .and_then(|info| info.waypoint_name.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        facts.push(FieldDetailFact {
            key: FIELD_DETAIL_FACT_KEY_REGION_NODE.to_string(),
            label: "Node".to_string(),
            value: node_name.to_string(),
            icon: Some("map-pin".to_string()),
            status_icon: None,
            status_icon_tone: None,
        });
    }

    (!facts.is_empty()).then_some(FieldDetailSection {
        id: "region".to_string(),
        kind: FIELD_DETAIL_SECTION_KIND_FACTS.to_string(),
        title: Some("Region".to_string()),
        facts,
        targets: Vec::new(),
    })
}

fn build_region_origin_detail_section(
    origin: Option<&RegionOriginInfo>,
) -> Option<FieldDetailSection> {
    let mut facts = Vec::new();
    let origin_region_id = origin.and_then(|info| info.region_id);
    let origin_value = origin
        .and_then(|info| info.region_name.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .zip(origin_region_id)
        .map(|(name, region_id)| format_region_name_with_id(name, region_id))
        .or_else(|| origin_region_id.map(|region_id| format!("R{region_id}")))
        .unwrap_or_else(|| "Origin".to_string());
    facts.push(FieldDetailFact {
        key: FIELD_DETAIL_FACT_KEY_ORIGIN_REGION.to_string(),
        label: "Origin".to_string(),
        value: origin_value,
        icon: Some("trade-origin".to_string()),
        status_icon: origin.and_then(|info| {
            let has_assignment = info.has_value();
            let has_name = info
                .region_name
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty());
            match (has_assignment, has_name) {
                (true, true) => None,
                (true, false) => Some("question-mark".to_string()),
                (false, _) => Some("question-mark".to_string()),
            }
        }),
        status_icon_tone: origin.and_then(|info| {
            let has_assignment = info.has_value();
            let has_name = info
                .region_name
                .as_deref()
                .map(str::trim)
                .is_some_and(|value| !value.is_empty());
            match (has_assignment, has_name) {
                (true, false) => Some("subtle".to_string()),
                _ => None,
            }
        }),
    });

    if let Some(node_name) = origin
        .and_then(|info| info.waypoint_name.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        facts.push(FieldDetailFact {
            key: FIELD_DETAIL_FACT_KEY_ORIGIN_NODE.to_string(),
            label: "Node".to_string(),
            value: node_name.to_string(),
            icon: Some("map-pin".to_string()),
            status_icon: None,
            status_icon_tone: None,
        });
    }

    let targets = vec![build_origin_hover_target(origin)]
        .into_iter()
        .flatten()
        .collect();
    (!facts.is_empty()).then_some(FieldDetailSection {
        id: "trade-origin".to_string(),
        kind: FIELD_DETAIL_SECTION_KIND_FACTS.to_string(),
        title: Some("Trade Origin".to_string()),
        facts,
        targets,
    })
}

fn build_region_group_resource_detail_section(
    region_group_id: u32,
    resource: Option<&RegionGroupWaypointInfo>,
    resource_region_id: Option<u32>,
    resource_region_info: Option<&RegionOriginInfo>,
) -> Option<FieldDetailSection> {
    let mut facts = Vec::new();

    let resource_group_value =
        format_resource_group_value(region_group_id, resource_region_id, resource_region_info);
    facts.push(FieldDetailFact {
        key: FIELD_DETAIL_FACT_KEY_RESOURCE_GROUP.to_string(),
        label: "Region Group".to_string(),
        value: resource_group_value,
        icon: Some("hover-resources".to_string()),
        status_icon: None,
        status_icon_tone: None,
    });

    if let Some(resource_waypoint_name) = resource
        .as_ref()
        .and_then(|info| info.waypoint_name.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        facts.push(FieldDetailFact {
            key: FIELD_DETAIL_FACT_KEY_RESOURCE_WAYPOINT.to_string(),
            label: "Waypoint".to_string(),
            value: resource_waypoint_name.to_string(),
            icon: Some("map-pin".to_string()),
            status_icon: None,
            status_icon_tone: None,
        });
    }

    let containing_region_value = resource_region_info
        .and_then(|info| info.region_name.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .zip(resource_region_id)
        .map(|(name, region_id)| format_region_name_with_id(name, region_id))
        .or_else(|| resource_region_id.map(|region_id| format!("R{region_id}")))
        .unwrap_or_else(|| format!("RG{region_group_id}"));
    facts.push(FieldDetailFact {
        key: FIELD_DETAIL_FACT_KEY_RESOURCE_REGION.to_string(),
        label: "Region".to_string(),
        value: containing_region_value,
        icon: Some("hover-zone".to_string()),
        status_icon: resource.as_ref().and_then(|info| {
            let has_assignment = info.has_value();
            let has_name = resource_region_info
                .and_then(|origin| origin.region_name.as_deref())
                .map(str::trim)
                .is_some_and(|value| !value.is_empty());
            match (has_assignment, has_name) {
                (true, true) => None,
                (true, false) => Some("question-mark".to_string()),
                (false, _) => Some("question-mark".to_string()),
            }
        }),
        status_icon_tone: resource.as_ref().and_then(|info| {
            let has_assignment = info.has_value();
            let has_name = resource_region_info
                .and_then(|origin| origin.region_name.as_deref())
                .map(str::trim)
                .is_some_and(|value| !value.is_empty());
            match (has_assignment, has_name) {
                (true, false) => Some("subtle".to_string()),
                _ => None,
            }
        }),
    });

    let mut targets = Vec::new();
    if let Some(target) = build_resource_hover_target_from_resource(
        region_group_id,
        resource,
        resource_region_id,
        resource_region_info,
    ) {
        targets.push(target);
    }

    (!facts.is_empty()).then_some(FieldDetailSection {
        id: "resource-bar".to_string(),
        kind: FIELD_DETAIL_SECTION_KIND_FACTS.to_string(),
        title: Some("Resources".to_string()),
        facts,
        targets,
    })
}

fn build_origin_hover_target(origin: Option<&RegionOriginInfo>) -> Option<FieldHoverTarget> {
    let origin = origin?;
    let world_x = origin.world_x?;
    let world_z = origin.world_z?;
    let label_value = origin
        .region_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .zip(origin.region_id)
        .map(|(name, region_id)| format_region_name_with_id(name, region_id))
        .or_else(|| origin.region_id.map(|region_id| format!("R{region_id}")))
        .unwrap_or_else(|| "Origin region".to_string());
    let label = format!("Origin: {label_value}");
    Some(FieldHoverTarget {
        key: FIELD_HOVER_TARGET_KEY_ORIGIN_NODE.to_string(),
        label,
        world_x,
        world_z,
    })
}

fn build_resource_hover_target(
    region_group_id: u32,
    resource: Option<&RegionGroupWaypointInfo>,
    resource_region_id: Option<u32>,
    resource_region_info: Option<&RegionOriginInfo>,
) -> Option<FieldHoverTarget> {
    let resource = resource?;
    build_resource_hover_target_from_resource(
        region_group_id,
        Some(resource),
        resource_region_id,
        resource_region_info,
    )
}

fn build_resource_hover_target_from_resource(
    region_group_id: u32,
    resource: Option<&RegionGroupWaypointInfo>,
    resource_region_id: Option<u32>,
    resource_region_info: Option<&RegionOriginInfo>,
) -> Option<FieldHoverTarget> {
    let resource = resource?;
    let world_x = resource.world_x?;
    let world_z = resource.world_z?;
    let label = format!(
        "Resources: {}",
        format_resource_group_value(region_group_id, resource_region_id, resource_region_info)
    );
    Some(FieldHoverTarget {
        key: FIELD_HOVER_TARGET_KEY_RESOURCE_NODE.to_string(),
        label,
        world_x,
        world_z,
    })
}

fn format_region_name_with_id(name: &str, region_id: u32) -> String {
    format!("{name} (R{region_id})")
}

fn format_region_group_name_with_id(name: &str, region_group_id: u32) -> String {
    format!("{name} (RG{region_group_id})")
}

fn format_resource_group_value(
    region_group_id: u32,
    resource_region_id: Option<u32>,
    resource_region_info: Option<&RegionOriginInfo>,
) -> String {
    resource_region_info
        .and_then(|info| info.region_name.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|name| format_region_group_name_with_id(name, region_group_id))
        .or_else(|| {
            resource_region_id.map(|region_id| format!("R{region_id} (RG{region_group_id})"))
        })
        .unwrap_or_else(|| format!("RG{region_group_id}"))
}

fn territory_detail_pane_ref(icon: &str) -> FieldDetailPaneRef {
    FieldDetailPaneRef {
        id: FIELD_DETAIL_PANE_ID_TERRITORY.to_string(),
        label: "Territory".to_string(),
        icon: icon.to_string(),
        order: 200,
    }
}

pub fn zone_mask_detail_pane_ref() -> FieldDetailPaneRef {
    FieldDetailPaneRef {
        id: FIELD_DETAIL_PANE_ID_ZONE_MASK.to_string(),
        label: "Zone".to_string(),
        icon: "hover-zone".to_string(),
        order: 100,
    }
}

fn sample_field_id_at_world(field: &DiscreteFieldRows, world_x: f64, world_z: f64) -> Option<u32> {
    let (map_x, map_y) = world_to_pixel_f(world_x, world_z);
    if !map_x.is_finite() || !map_y.is_finite() {
        return None;
    }
    field
        .cell_id_u32(map_x.floor() as i32, map_y.floor() as i32)
        .filter(|id| *id != 0)
}

fn resolve_region_name(loc: &LocalizationTable, origin_region_id: Option<u32>) -> Option<String> {
    origin_region_id
        .and_then(|id| localized_name(&loc.town, id))
        .or_else(|| origin_region_id.and_then(|id| localized_name(&loc.node, id)))
}

fn classify_waypoint_node_type(raw_name: &str) -> Option<WaypointNodeType> {
    let normalized = raw_name.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    if normalized.contains("danger") {
        return Some(WaypointNodeType::Danger);
    }
    if normalized.starts_with("hidden_town_") {
        if normalized.contains("(trade)") || normalized.contains("post") {
            return Some(WaypointNodeType::TradingPost);
        }
        if normalized.contains("(worker)") {
            return Some(WaypointNodeType::WorkerNode);
        }
        return Some(WaypointNodeType::Town);
    }
    if let Some(token) = normalized
        .strip_prefix("town(")
        .and_then(|value| value.strip_suffix(')'))
    {
        if token.contains("city") {
            return Some(WaypointNodeType::City);
        }
        if token.contains("village") {
            return Some(WaypointNodeType::Village);
        }
        if token.contains("post") || token.contains("trade") || token.contains("specialty") {
            return Some(WaypointNodeType::TradingPost);
        }
        return Some(WaypointNodeType::Town);
    }
    if normalized.starts_with("gateway(") {
        return Some(WaypointNodeType::Gateway);
    }
    if normalized.starts_with("castle(") {
        return Some(WaypointNodeType::Castle);
    }
    if normalized.starts_with("island(") {
        return Some(WaypointNodeType::Island);
    }
    if normalized.starts_with("ocean(") || normalized.starts_with("seas(") {
        return Some(WaypointNodeType::Sea);
    }
    if normalized.starts_with("field(")
        || normalized.starts_with("filed(")
        || normalized.starts_with("hidden_field_")
        || normalized.starts_with("safe_field")
        || normalized.starts_with("workplace")
    {
        return Some(WaypointNodeType::Connection);
    }
    None
}

fn is_hidden_waypoint_name(raw_name: &str) -> bool {
    let normalized = raw_name.trim().to_ascii_lowercase();
    normalized.starts_with("hidden_")
}

fn localized_name(map: &BTreeMap<String, String>, key: u32) -> Option<String> {
    let value = map.get(&key.to_string())?;
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
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

fn read_unaligned_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let end = offset
        .checked_add(4)
        .context("u32 offset overflow while parsing regioninfo.bss")?;
    let slice = bytes
        .get(offset..end)
        .with_context(|| format!("u32 read at offset {} is out of bounds", offset))?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn parse_attr_u32(line: &str, attr: &str) -> Result<u32> {
    let raw = parse_attr(line, attr)?;
    raw.parse::<u32>()
        .with_context(|| format!("failed to parse `{raw}` as u32 for attribute {attr}"))
}

fn parse_attr_f64(line: &str, attr: &str) -> Result<f64> {
    let raw = parse_attr(line, attr)?;
    raw.parse::<f64>()
        .with_context(|| format!("failed to parse `{raw}` as f64 for attribute {attr}"))
}

fn parse_attr_string(line: &str, attr: &str) -> Result<String> {
    Ok(parse_attr(line, attr)?.to_string())
}

fn parse_attr_bool(line: &str, attr: &str) -> Result<bool> {
    let raw = parse_attr(line, attr)?;
    match raw {
        "True" => Ok(true),
        "False" => Ok(false),
        _ => bail!("failed to parse `{raw}` as bool for attribute {attr}"),
    }
}

fn parse_attr<'a>(line: &'a str, attr: &str) -> Result<&'a str> {
    let needle = format!(r#"{attr}=""#);
    let start = line
        .find(&needle)
        .with_context(|| format!("missing attribute {attr} in line `{line}`"))?
        + needle.len();
    let end = start
        + line[start..]
            .find('"')
            .with_context(|| format!("unterminated attribute {attr} in line `{line}`"))?;
    Ok(&line[start..end])
}

fn non_zero_u32(value: u32) -> Option<u32> {
    (value != 0).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::{
        classify_waypoint_node_type, decode_regiongroupinfo_bss_rows, decode_regioninfo_bss_rows,
        load_region_group_mapping_from_regioninfo_bss, load_waypoint_graph, parse_attr_f64,
        parse_attr_u32, LocalizationTable, OriginalRegionGroupInfoRow, OriginalRegionInfoRow,
        OriginalRegionLayerContext, OriginalWaypointRow, WaypointDisplayClass, WaypointNodeType,
        PABR_MAGIC, REGIONGROUPINFO_ROW_GRAPHX_OFFSET, REGIONGROUPINFO_ROW_GRAPHY_OFFSET,
        REGIONGROUPINFO_ROW_GRAPHZ_OFFSET, REGIONGROUPINFO_ROW_LEN,
        REGIONGROUPINFO_ROW_WAYPOINT_OFFSET, REGIONINFO_ROW_ACCESSIBLE_OFFSET,
        REGIONINFO_ROW_GROUP_OFFSET, REGIONINFO_ROW_SIGNATURE_OFFSET,
        REGIONINFO_ROW_SIGNATURE_PREFIX, REGIONINFO_ROW_TRADE_ORIGIN_OFFSET,
        REGIONINFO_ROW_WAYPOINT_PRIMARY_OFFSET,
    };
    use std::collections::HashMap;
    use std::fs;

    #[test]
    fn waypoint_attribute_parser_reads_values() {
        let line = r#"<Waypoint Key="2052" Name="town(olvia_academy)" PosX="-114942" PosY="-2674.33" PosZ="157114"/>"#;
        assert_eq!(parse_attr_u32(line, "Key").unwrap(), 2052);
        assert_eq!(parse_attr_f64(line, "PosY").unwrap(), -2674.33);
    }

    #[test]
    fn regioninfo_bss_decoder_finds_signature_rows() {
        let mut bytes = vec![0u8; 256];
        bytes[0..4].copy_from_slice(PABR_MAGIC);
        let row_start = 8usize;
        bytes[row_start..row_start + 2].copy_from_slice(&42u16.to_le_bytes());
        let signature_start = row_start + REGIONINFO_ROW_SIGNATURE_OFFSET;
        bytes[signature_start..signature_start + REGIONINFO_ROW_SIGNATURE_PREFIX.len()]
            .copy_from_slice(&REGIONINFO_ROW_SIGNATURE_PREFIX);
        bytes[row_start + REGIONINFO_ROW_TRADE_ORIGIN_OFFSET
            ..row_start + REGIONINFO_ROW_TRADE_ORIGIN_OFFSET + 2]
            .copy_from_slice(&88u16.to_le_bytes());
        bytes[row_start + REGIONINFO_ROW_GROUP_OFFSET..row_start + REGIONINFO_ROW_GROUP_OFFSET + 2]
            .copy_from_slice(&295u16.to_le_bytes());
        let encoded_waypoint = 2052u32 << 8;
        bytes[row_start + REGIONINFO_ROW_WAYPOINT_PRIMARY_OFFSET
            ..row_start + REGIONINFO_ROW_WAYPOINT_PRIMARY_OFFSET + 4]
            .copy_from_slice(&encoded_waypoint.to_le_bytes());

        let rows = decode_regioninfo_bss_rows(&bytes).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, 42);
        assert!(!rows[0].is_accessible);
        assert_eq!(rows[0].tradeoriginregion, 88);
        assert_eq!(rows[0].regiongroup, 295);
        assert_eq!(rows[0].waypoint, Some(2052));
    }

    #[test]
    fn regiongroupinfo_bss_decoder_reads_fixed_rows() {
        let mut bytes = vec![0u8; 8 + REGIONGROUPINFO_ROW_LEN + 12];
        bytes[0..4].copy_from_slice(PABR_MAGIC);
        bytes[4..8].copy_from_slice(&1u32.to_le_bytes());
        let row = &mut bytes[8..8 + REGIONGROUPINFO_ROW_LEN];
        row[0..2].copy_from_slice(&295u16.to_le_bytes());
        row[REGIONGROUPINFO_ROW_WAYPOINT_OFFSET..REGIONGROUPINFO_ROW_WAYPOINT_OFFSET + 4]
            .copy_from_slice(&2052u32.to_le_bytes());
        row[REGIONGROUPINFO_ROW_GRAPHX_OFFSET..REGIONGROUPINFO_ROW_GRAPHX_OFFSET + 4]
            .copy_from_slice(&(-114535.0f32).to_le_bytes());
        row[REGIONGROUPINFO_ROW_GRAPHY_OFFSET..REGIONGROUPINFO_ROW_GRAPHY_OFFSET + 4]
            .copy_from_slice(&(-2674.0f32).to_le_bytes());
        row[REGIONGROUPINFO_ROW_GRAPHZ_OFFSET..REGIONGROUPINFO_ROW_GRAPHZ_OFFSET + 4]
            .copy_from_slice(&(157512.0f32).to_le_bytes());

        let rows = decode_regiongroupinfo_bss_rows(&bytes).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, 295);
        assert_eq!(rows[0].waypoint, Some(2052));
        assert_eq!(rows[0].graphx, -114535.0);
        assert_eq!(rows[0].graphz, 157512.0);
    }

    #[test]
    fn load_waypoint_graph_prefers_non_subwaypoint_and_later_source() {
        let temp_dir = std::env::temp_dir().join("fishystuff_waypoint_xml_test");
        fs::create_dir_all(&temp_dir).unwrap();
        let primary = temp_dir.join("mapdata_realexplore.xml");
        let secondary = temp_dir.join("mapdata_realexplore2.xml");
        fs::write(
            &primary,
            concat!(
                "<Waypoint Key=\"1147\" Name=\"field(stonebeakshore)\" PosX=\"305927\" PosY=\"-414.348\" PosZ=\"-37179.1\" Property=\"ground\" IsSubWaypoint=\"True\" IsEscape=\"False\"/>\n",
                "<Waypoint Key=\"45\" Name=\"field(toscani's_farm)\" PosX=\"-29592.9\" PosY=\"-2909.08\" PosZ=\"26244.2\" Property=\"all\" IsSubWaypoint=\"False\" IsEscape=\"False\"/>\n",
                "<Link SourceWaypoint=\"1147\" TargetWaypoint=\"45\"/>\n"
            ),
        )
        .unwrap();
        fs::write(
            &secondary,
            concat!(
                "<Waypoint Key=\"1147\" Name=\"field(stonebeakshore)\" PosX=\"303191\" PosY=\"-322.014\" PosZ=\"-1694.35\" Property=\"none\" IsSubWaypoint=\"False\" IsEscape=\"False\"/>\n",
                "<Waypoint Key=\"45\" Name=\"field(toscani's_farm)\" PosX=\"-30432.3\" PosY=\"-3017.58\" PosZ=\"28481.4\" Property=\"ground\" IsSubWaypoint=\"False\" IsEscape=\"False\"/>\n"
            ),
        )
        .unwrap();

        let graph = load_waypoint_graph(&[primary.clone(), secondary.clone()]).unwrap();
        let stonebeak = graph.rows.get(&1147).unwrap();
        assert_eq!(stonebeak.pos_x, 303191.0);
        assert_eq!(stonebeak.pos_z, -1694.35);
        let toscani = graph.rows.get(&45).unwrap();
        assert_eq!(toscani.pos_x, -30432.3);
        assert_eq!(toscani.pos_z, 28481.4);
        assert_eq!(graph.links.get(&1147), Some(&vec![45]));
        assert_eq!(graph.links.get(&45), Some(&vec![1147]));

        fs::remove_file(primary).ok();
        fs::remove_file(secondary).ok();
        fs::remove_dir(temp_dir).ok();
    }

    #[test]
    fn classify_waypoint_node_type_uses_internal_waypoint_name_patterns() {
        assert_eq!(
            classify_waypoint_node_type("town(calpheoncity)"),
            Some(WaypointNodeType::City)
        );
        assert_eq!(
            classify_waypoint_node_type("town(trentvillage)"),
            Some(WaypointNodeType::Village)
        );
        assert_eq!(
            classify_waypoint_node_type("town(barhan_post)"),
            Some(WaypointNodeType::TradingPost)
        );
        assert_eq!(
            classify_waypoint_node_type("gateway(west_velia_gateway)"),
            Some(WaypointNodeType::Gateway)
        );
        assert_eq!(
            classify_waypoint_node_type("field(stonebeakshore)"),
            Some(WaypointNodeType::Connection)
        );
    }

    #[test]
    fn resolve_waypoint_display_info_combines_hidden_subwaypoint_and_reference_signals() {
        let context = OriginalRegionLayerContext {
            regioninfo: [(
                204,
                OriginalRegionInfoRow {
                    key: 204,
                    is_accessible: true,
                    tradeoriginregion: 204,
                    regiongroup: 55,
                    waypoint: Some(45),
                },
            )]
            .into_iter()
            .collect(),
            regiongroupinfo: [(
                58,
                OriginalRegionGroupInfoRow {
                    key: 58,
                    waypoint: Some(46),
                    graphx: 0.0,
                    graphy: 0.0,
                    graphz: 0.0,
                },
            )]
            .into_iter()
            .collect(),
            waypoints: [
                (
                    45,
                    OriginalWaypointRow {
                        key: 45,
                        raw_name: "town(velia)".to_string(),
                        pos_x: 0.0,
                        pos_y: 0.0,
                        pos_z: 0.0,
                        is_sub_waypoint: false,
                        source_index: 0,
                    },
                ),
                (
                    46,
                    OriginalWaypointRow {
                        key: 46,
                        raw_name: "hidden_field_test".to_string(),
                        pos_x: 0.0,
                        pos_y: 0.0,
                        pos_z: 0.0,
                        is_sub_waypoint: false,
                        source_index: 0,
                    },
                ),
                (
                    47,
                    OriginalWaypointRow {
                        key: 47,
                        raw_name: "hidden_field_unreferenced".to_string(),
                        pos_x: 0.0,
                        pos_y: 0.0,
                        pos_z: 0.0,
                        is_sub_waypoint: false,
                        source_index: 0,
                    },
                ),
                (
                    48,
                    OriginalWaypointRow {
                        key: 48,
                        raw_name: "road(subwaypoint)".to_string(),
                        pos_x: 0.0,
                        pos_y: 0.0,
                        pos_z: 0.0,
                        is_sub_waypoint: true,
                        source_index: 0,
                    },
                ),
            ]
            .into_iter()
            .collect(),
            waypoint_links: HashMap::new(),
            localization: LocalizationTable::default(),
        };

        let region_ref = context.resolve_waypoint_display_info(45).unwrap();
        assert_eq!(region_ref.display_class, WaypointDisplayClass::Main);
        assert!(region_ref.referenced_by_region);
        assert!(!region_ref.referenced_by_region_group);
        assert!(region_ref.default_visible);

        let hidden_group_ref = context.resolve_waypoint_display_info(46).unwrap();
        assert_eq!(hidden_group_ref.display_class, WaypointDisplayClass::Hidden);
        assert!(!hidden_group_ref.referenced_by_region);
        assert!(hidden_group_ref.referenced_by_region_group);
        assert!(hidden_group_ref.default_visible);

        let hidden_unreferenced = context.resolve_waypoint_display_info(47).unwrap();
        assert_eq!(
            hidden_unreferenced.display_class,
            WaypointDisplayClass::Hidden
        );
        assert!(!hidden_unreferenced.referenced_by_region);
        assert!(!hidden_unreferenced.referenced_by_region_group);
        assert!(!hidden_unreferenced.default_visible);

        let sub_waypoint = context.resolve_waypoint_display_info(48).unwrap();
        assert_eq!(
            sub_waypoint.display_class,
            WaypointDisplayClass::SubWaypoint
        );
        assert!(!sub_waypoint.default_visible);
    }

    #[test]
    fn region_group_mapping_builds_from_regioninfo_bss() {
        let mut bytes = vec![0u8; 512];
        bytes[0..4].copy_from_slice(PABR_MAGIC);

        for (index, (key, group)) in [(42u16, 295u16), (43u16, 295u16)].into_iter().enumerate() {
            let row_start = 8 + index * 200;
            bytes[row_start..row_start + 2].copy_from_slice(&key.to_le_bytes());
            let signature_start = row_start + REGIONINFO_ROW_SIGNATURE_OFFSET;
            bytes[signature_start..signature_start + REGIONINFO_ROW_SIGNATURE_PREFIX.len()]
                .copy_from_slice(&REGIONINFO_ROW_SIGNATURE_PREFIX);
            bytes[row_start + REGIONINFO_ROW_GROUP_OFFSET
                ..row_start + REGIONINFO_ROW_GROUP_OFFSET + 2]
                .copy_from_slice(&group.to_le_bytes());
        }

        let path = std::env::temp_dir().join("fishystuff_region_group_mapping_test.bss");
        fs::write(&path, &bytes).unwrap();
        let mapping = load_region_group_mapping_from_regioninfo_bss(&path).unwrap();
        fs::remove_file(&path).ok();

        assert_eq!(mapping.region_group_for_region(42), Some(295));
        assert_eq!(mapping.region_ids_for_group(295), &[42, 43]);
    }

    #[test]
    fn regioninfo_bss_decoder_accepts_alternate_signature_family() {
        let mut bytes = vec![0u8; 256];
        bytes[0..4].copy_from_slice(PABR_MAGIC);
        let row_start = 8usize;
        bytes[row_start..row_start + 2].copy_from_slice(&832u16.to_le_bytes());
        let signature_start = row_start + REGIONINFO_ROW_SIGNATURE_OFFSET;
        bytes[signature_start..signature_start + 8]
            .copy_from_slice(&[0x5A, 0x55, 0x00, 0x00, 0x00, 0x00, 0xBF, 0x06]);
        bytes[row_start + REGIONINFO_ROW_ACCESSIBLE_OFFSET] = 1;
        bytes[row_start + REGIONINFO_ROW_TRADE_ORIGIN_OFFSET
            ..row_start + REGIONINFO_ROW_TRADE_ORIGIN_OFFSET + 2]
            .copy_from_slice(&832u16.to_le_bytes());
        bytes[row_start + REGIONINFO_ROW_GROUP_OFFSET..row_start + REGIONINFO_ROW_GROUP_OFFSET + 2]
            .copy_from_slice(&218u16.to_le_bytes());
        let encoded_waypoint = 1417u32 << 8;
        bytes[row_start + REGIONINFO_ROW_WAYPOINT_PRIMARY_OFFSET
            ..row_start + REGIONINFO_ROW_WAYPOINT_PRIMARY_OFFSET + 4]
            .copy_from_slice(&encoded_waypoint.to_le_bytes());

        let rows = decode_regioninfo_bss_rows(&bytes).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, 832);
        assert!(rows[0].is_accessible);
        assert_eq!(rows[0].tradeoriginregion, 832);
        assert_eq!(rows[0].regiongroup, 218);
        assert_eq!(rows[0].waypoint, Some(1417));
    }

    #[test]
    fn regioninfo_bss_decoder_ignores_secondary_waypoint_only_rows() {
        let mut bytes = vec![0u8; 256];
        bytes[0..4].copy_from_slice(PABR_MAGIC);
        let row_start = 8usize;
        bytes[row_start..row_start + 2].copy_from_slice(&5u16.to_le_bytes());
        let signature_start = row_start + REGIONINFO_ROW_SIGNATURE_OFFSET;
        bytes[signature_start..signature_start + REGIONINFO_ROW_SIGNATURE_PREFIX.len()]
            .copy_from_slice(&REGIONINFO_ROW_SIGNATURE_PREFIX);
        bytes[row_start + REGIONINFO_ROW_ACCESSIBLE_OFFSET] = 1;
        bytes[row_start + REGIONINFO_ROW_TRADE_ORIGIN_OFFSET
            ..row_start + REGIONINFO_ROW_TRADE_ORIGIN_OFFSET + 2]
            .copy_from_slice(&5u16.to_le_bytes());
        bytes[row_start + REGIONINFO_ROW_GROUP_OFFSET..row_start + REGIONINFO_ROW_GROUP_OFFSET + 2]
            .copy_from_slice(&1u16.to_le_bytes());
        let encoded_secondary_waypoint = 1u32 << 8;
        bytes[row_start + 110..row_start + 114]
            .copy_from_slice(&encoded_secondary_waypoint.to_le_bytes());

        let rows = decode_regioninfo_bss_rows(&bytes).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, 5);
        assert_eq!(rows[0].waypoint, None);
    }

    #[test]
    fn resolve_region_origin_info_uses_origin_region_waypoint() {
        let context = OriginalRegionLayerContext {
            regioninfo: [
                (
                    216,
                    OriginalRegionInfoRow {
                        key: 216,
                        is_accessible: true,
                        tradeoriginregion: 221,
                        regiongroup: 58,
                        waypoint: Some(1172),
                    },
                ),
                (
                    221,
                    OriginalRegionInfoRow {
                        key: 221,
                        is_accessible: true,
                        tradeoriginregion: 221,
                        regiongroup: 58,
                        waypoint: Some(1141),
                    },
                ),
            ]
            .into_iter()
            .collect(),
            regiongroupinfo: HashMap::new(),
            waypoints: [
                (
                    1141,
                    OriginalWaypointRow {
                        key: 1141,
                        raw_name: "town(tarifcamp)".to_string(),
                        pos_x: 226814.0,
                        pos_y: -338.0,
                        pos_z: -73831.4,
                        is_sub_waypoint: false,
                        source_index: 1,
                    },
                ),
                (
                    1172,
                    OriginalWaypointRow {
                        key: 1172,
                        raw_name: "field(hathracliff)".to_string(),
                        pos_x: 189607.0,
                        pos_y: 16927.1,
                        pos_z: -160661.0,
                        is_sub_waypoint: false,
                        source_index: 1,
                    },
                ),
            ]
            .into_iter()
            .collect(),
            waypoint_links: HashMap::new(),
            localization: LocalizationTable {
                node: [
                    ("1141".to_string(), "Tarif".to_string()),
                    ("1172".to_string(), "Hasrah Cliff".to_string()),
                ]
                .into_iter()
                .collect(),
                town: [("221".to_string(), "Tarif".to_string())]
                    .into_iter()
                    .collect(),
            },
        };

        let origin = context.resolve_region_origin_info(216).unwrap();
        assert_eq!(origin.region_id, Some(221));
        assert_eq!(origin.region_name.as_deref(), Some("Tarif"));
        assert_eq!(origin.waypoint_id, Some(1141));
        assert_eq!(origin.waypoint_name.as_deref(), Some("Tarif"));
        assert_eq!(origin.world_x, Some(226814.0));
        assert_eq!(origin.world_z, Some(-73831.4));
    }

    #[test]
    fn resolve_region_waypoint_info_uses_selected_waypoint_xml_position() {
        let context = OriginalRegionLayerContext {
            regioninfo: [(
                204,
                OriginalRegionInfoRow {
                    key: 204,
                    is_accessible: true,
                    tradeoriginregion: 204,
                    regiongroup: 55,
                    waypoint: Some(1147),
                },
            )]
            .into_iter()
            .collect(),
            regiongroupinfo: HashMap::new(),
            waypoints: [(
                1147,
                OriginalWaypointRow {
                    key: 1147,
                    raw_name: "field(stonebeakshore)".to_string(),
                    pos_x: 303191.0,
                    pos_y: -322.014,
                    pos_z: -1694.35,
                    is_sub_waypoint: false,
                    source_index: 1,
                },
            )]
            .into_iter()
            .collect(),
            waypoint_links: HashMap::new(),
            localization: LocalizationTable {
                node: [("1147".to_string(), "Stonebeak Shore".to_string())]
                    .into_iter()
                    .collect(),
                town: [("204".to_string(), "Stonebeak Shore".to_string())]
                    .into_iter()
                    .collect(),
            },
        };

        let region = context.resolve_region_waypoint_info(204).unwrap();
        assert_eq!(region.region_id, Some(204));
        assert_eq!(region.region_name.as_deref(), Some("Stonebeak Shore"));
        assert_eq!(region.waypoint_id, Some(1147));
        assert_eq!(region.waypoint_name.as_deref(), Some("Stonebeak Shore"));
        assert_eq!(region.world_x, Some(303191.0));
        assert_eq!(region.world_z, Some(-1694.35));
    }

    #[test]
    fn region_hover_metadata_keeps_region_node_fact_and_only_targets_trade_origin() {
        let context = OriginalRegionLayerContext {
            regioninfo: [
                (
                    204,
                    OriginalRegionInfoRow {
                        key: 204,
                        is_accessible: true,
                        tradeoriginregion: 221,
                        regiongroup: 58,
                        waypoint: Some(1147),
                    },
                ),
                (
                    221,
                    OriginalRegionInfoRow {
                        key: 221,
                        is_accessible: true,
                        tradeoriginregion: 221,
                        regiongroup: 58,
                        waypoint: Some(1141),
                    },
                ),
            ]
            .into_iter()
            .collect(),
            regiongroupinfo: HashMap::new(),
            waypoints: [
                (
                    1141,
                    OriginalWaypointRow {
                        key: 1141,
                        raw_name: "town(tarifcamp)".to_string(),
                        pos_x: 226814.0,
                        pos_y: -338.0,
                        pos_z: -73831.4,
                        is_sub_waypoint: false,
                        source_index: 1,
                    },
                ),
                (
                    1147,
                    OriginalWaypointRow {
                        key: 1147,
                        raw_name: "field(stonebeakshore)".to_string(),
                        pos_x: 303191.0,
                        pos_y: -322.014,
                        pos_z: -1694.35,
                        is_sub_waypoint: false,
                        source_index: 1,
                    },
                ),
            ]
            .into_iter()
            .collect(),
            waypoint_links: HashMap::new(),
            localization: LocalizationTable {
                node: [
                    ("1141".to_string(), "Tarif".to_string()),
                    ("1147".to_string(), "Stonebeak Shore".to_string()),
                ]
                .into_iter()
                .collect(),
                town: [
                    ("204".to_string(), "Stonebeak Shore".to_string()),
                    ("221".to_string(), "Tarif".to_string()),
                ]
                .into_iter()
                .collect(),
            },
        };

        let entry = context.resolve_region_hover_metadata(204).unwrap();
        assert_eq!(entry.detail_sections.len(), 2);

        let region_section = entry
            .detail_sections
            .iter()
            .find(|section| section.id == "region")
            .unwrap();
        assert!(region_section
            .facts
            .iter()
            .any(|fact| fact.key == "region" && fact.value == "Stonebeak Shore (R204)"));
        assert!(region_section
            .facts
            .iter()
            .any(|fact| fact.key == "region_node" && fact.value == "Stonebeak Shore"));
        assert!(region_section.targets.is_empty());
        assert_eq!(entry.targets.len(), 1);
        assert_eq!(entry.targets[0].key, "origin_node");
        assert_eq!(entry.targets[0].label, "Origin: Tarif (R221)");

        let origin_section = entry
            .detail_sections
            .iter()
            .find(|section| section.id == "trade-origin")
            .unwrap();
        assert!(origin_section
            .facts
            .iter()
            .any(|fact| fact.key == "origin_region"
                && fact.label == "Origin"
                && fact.value == "Tarif (R221)"));
        assert!(origin_section
            .facts
            .iter()
            .any(|fact| fact.key == "origin_node" && fact.value == "Tarif"));
        assert!(origin_section.targets.iter().any(|target| {
            target.key == "origin_node"
                && target.label == "Origin: Tarif (R221)"
                && (target.world_x - 226814.0).abs() < f64::EPSILON
                && (target.world_z - (-73831.4)).abs() < f64::EPSILON
        }));
    }

    #[test]
    fn region_node_connection_pairs_use_direct_main_node_links() {
        let context = OriginalRegionLayerContext {
            regioninfo: [
                (
                    204,
                    OriginalRegionInfoRow {
                        key: 204,
                        is_accessible: true,
                        tradeoriginregion: 204,
                        regiongroup: 55,
                        waypoint: Some(1147),
                    },
                ),
                (
                    205,
                    OriginalRegionInfoRow {
                        key: 205,
                        is_accessible: true,
                        tradeoriginregion: 205,
                        regiongroup: 55,
                        waypoint: Some(1148),
                    },
                ),
            ]
            .into_iter()
            .collect(),
            regiongroupinfo: HashMap::new(),
            waypoints: [
                (
                    1147,
                    OriginalWaypointRow {
                        key: 1147,
                        raw_name: "field(stonebeakshore)".to_string(),
                        pos_x: 303191.0,
                        pos_y: -322.014,
                        pos_z: -1694.35,
                        is_sub_waypoint: false,
                        source_index: 1,
                    },
                ),
                (
                    1148,
                    OriginalWaypointRow {
                        key: 1148,
                        raw_name: "field(alejandrofarm)".to_string(),
                        pos_x: 301000.0,
                        pos_y: -320.0,
                        pos_z: 1200.0,
                        is_sub_waypoint: false,
                        source_index: 1,
                    },
                ),
                (
                    900001,
                    OriginalWaypointRow {
                        key: 900001,
                        raw_name: "road(tmp)".to_string(),
                        pos_x: 302200.0,
                        pos_y: -321.0,
                        pos_z: -300.0,
                        is_sub_waypoint: true,
                        source_index: 0,
                    },
                ),
            ]
            .into_iter()
            .collect(),
            waypoint_links: [
                (1147, vec![1148, 900001]),
                (900001, vec![1147, 1148]),
                (1148, vec![1147, 900001]),
            ]
            .into_iter()
            .collect(),
            localization: LocalizationTable::default(),
        };

        assert_eq!(context.region_node_connection_pairs(), vec![(204, 205)]);
    }

    #[test]
    fn region_node_connection_pairs_ignore_subwaypoint_bridge_only() {
        let context = OriginalRegionLayerContext {
            regioninfo: [
                (
                    204,
                    OriginalRegionInfoRow {
                        key: 204,
                        is_accessible: true,
                        tradeoriginregion: 204,
                        regiongroup: 55,
                        waypoint: Some(1147),
                    },
                ),
                (
                    205,
                    OriginalRegionInfoRow {
                        key: 205,
                        is_accessible: true,
                        tradeoriginregion: 205,
                        regiongroup: 55,
                        waypoint: Some(1148),
                    },
                ),
            ]
            .into_iter()
            .collect(),
            regiongroupinfo: HashMap::new(),
            waypoints: [
                (
                    1147,
                    OriginalWaypointRow {
                        key: 1147,
                        raw_name: "field(stonebeakshore)".to_string(),
                        pos_x: 303191.0,
                        pos_y: -322.014,
                        pos_z: -1694.35,
                        is_sub_waypoint: false,
                        source_index: 1,
                    },
                ),
                (
                    1148,
                    OriginalWaypointRow {
                        key: 1148,
                        raw_name: "field(alejandrofarm)".to_string(),
                        pos_x: 301000.0,
                        pos_y: -320.0,
                        pos_z: 1200.0,
                        is_sub_waypoint: false,
                        source_index: 1,
                    },
                ),
                (
                    900001,
                    OriginalWaypointRow {
                        key: 900001,
                        raw_name: "road(tmp)".to_string(),
                        pos_x: 302200.0,
                        pos_y: -321.0,
                        pos_z: -300.0,
                        is_sub_waypoint: true,
                        source_index: 0,
                    },
                ),
            ]
            .into_iter()
            .collect(),
            waypoint_links: [
                (1147, vec![900001]),
                (900001, vec![1147, 1148]),
                (1148, vec![900001]),
            ]
            .into_iter()
            .collect(),
            localization: LocalizationTable::default(),
        };

        assert!(context.region_node_connection_pairs().is_empty());
    }
}
