use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::field_metadata::FieldHoverMetadataEntry;
use crate::loc::load_loc_namespaces_as_string_maps;

const PABR_MAGIC: &[u8; 4] = b"PABR";
const REGIONINFO_ROW_SIGNATURE_PREFIX: [u8; 4] = [0x5A, 0x55, 0x00, 0x00];
const REGIONINFO_ROW_SIGNATURE_OFFSET: usize = 32;
const REGIONINFO_ROW_ACCESSIBLE_OFFSET: usize = 27;
const REGIONINFO_ROW_TRADE_ORIGIN_OFFSET: usize = 102;
const REGIONINFO_ROW_GROUP_OFFSET: usize = 104;
const REGIONINFO_ROW_WAYPOINT_PRIMARY_OFFSET: usize = 106;
const REGIONINFO_ROW_WAYPOINT_SECONDARY_OFFSET: usize = 110;
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
}

#[derive(Debug, Clone, Copy)]
pub struct RegionGroupWaypointInfo {
    pub waypoint_id: Option<u32>,
    pub world_x: Option<f64>,
    pub world_z: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct RegionOriginInfo {
    pub region_id: Option<u32>,
    pub waypoint_id: Option<u32>,
    pub world_x: Option<f64>,
    pub world_z: Option<f64>,
    pub name: Option<String>,
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
    localization: LocalizationTable,
}

impl OriginalRegionLayerContext {
    pub fn region_group_for_region(&self, region_id: u32) -> Option<u32> {
        self.regioninfo
            .get(&region_id)
            .and_then(|row| non_zero_u32(row.regiongroup))
    }

    pub fn resolve_region_origin_info(&self, region_id: u32) -> Option<RegionOriginInfo> {
        let row = self.regioninfo.get(&region_id)?;
        let origin_region_id = non_zero_u32(row.tradeoriginregion);
        let waypoint_id = row.waypoint;
        let waypoint = waypoint_id.and_then(|id| self.waypoints.get(&id));
        let info = RegionOriginInfo {
            region_id: origin_region_id,
            waypoint_id,
            world_x: waypoint.map(|waypoint| waypoint.pos_x),
            world_z: waypoint.map(|waypoint| waypoint.pos_z),
            name: resolve_origin_name(&self.localization, waypoint_id, origin_region_id),
        };
        info.has_value().then_some(info)
    }

    pub fn resolve_resource_waypoint(
        &self,
        region_group_id: u32,
    ) -> Option<RegionGroupWaypointInfo> {
        let row = self.regiongroupinfo.get(&region_group_id)?;
        let info = RegionGroupWaypointInfo {
            waypoint_id: row.waypoint,
            world_x: Some(row.graphx).filter(|value| *value != 0.0),
            world_z: Some(row.graphz).filter(|value| *value != 0.0),
        };
        info.has_value().then_some(info)
    }

    pub fn resolve_region_hover_metadata(&self, region_id: u32) -> Option<FieldHoverMetadataEntry> {
        let region_group = self.region_group_for_region(region_id);
        let origin = self.resolve_region_origin_info(region_id);
        let resource = region_group.and_then(|group_id| self.resolve_resource_waypoint(group_id));
        let entry = FieldHoverMetadataEntry {
            region_id: Some(region_id),
            region_group,
            region_name: origin.as_ref().and_then(|value| value.name.clone()),
            resource_bar_waypoint: resource.and_then(|value| value.waypoint_id),
            resource_bar_world_x: resource.and_then(|value| value.world_x),
            resource_bar_world_z: resource.and_then(|value| value.world_z),
            origin_waypoint: origin.as_ref().and_then(|value| value.waypoint_id),
            origin_world_x: origin.as_ref().and_then(|value| value.world_x),
            origin_world_z: origin.as_ref().and_then(|value| value.world_z),
        };
        entry.has_value().then_some(entry)
    }

    pub fn resolve_region_group_hover_metadata(
        &self,
        region_group_id: u32,
    ) -> Option<FieldHoverMetadataEntry> {
        let resource = self.resolve_resource_waypoint(region_group_id);
        let entry = FieldHoverMetadataEntry {
            region_id: None,
            region_group: Some(region_group_id),
            region_name: None,
            resource_bar_waypoint: resource.and_then(|value| value.waypoint_id),
            resource_bar_world_x: resource.and_then(|value| value.world_x),
            resource_bar_world_z: resource.and_then(|value| value.world_z),
            origin_waypoint: None,
            origin_world_x: None,
            origin_world_z: None,
        };
        entry.has_value().then_some(entry)
    }
}

impl RegionOriginInfo {
    pub fn has_value(&self) -> bool {
        self.region_id.is_some()
            || self.waypoint_id.is_some()
            || self.world_x.is_some()
            || self.world_z.is_some()
            || self.name.is_some()
    }
}

impl RegionGroupWaypointInfo {
    pub fn has_value(self) -> bool {
        self.waypoint_id.is_some() || self.world_x.is_some() || self.world_z.is_some()
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
    let waypoints = load_waypoint_rows(waypoint_xml_paths)?;
    let localization = load_localization(loc_path)?;

    Ok(OriginalRegionLayerContext {
        regioninfo,
        regiongroupinfo,
        waypoints,
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
        )?
        .or(decode_shifted_u32_field(
            bytes,
            row_start_offset + REGIONINFO_ROW_WAYPOINT_SECONDARY_OFFSET,
        )?);

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

fn load_waypoint_rows(paths: &[PathBuf]) -> Result<HashMap<u32, OriginalWaypointRow>> {
    let mut rows = HashMap::new();
    for path in paths {
        let bytes = fs::read(path)
            .with_context(|| format!("failed to read waypoint XML {}", path.display()))?;
        let contents = String::from_utf8_lossy(&bytes);
        for raw_line in contents.lines() {
            let line = raw_line.trim();
            if !line.starts_with("<Waypoint ") {
                continue;
            }

            let key = parse_attr_u32(line, "Key")
                .with_context(|| format!("failed to parse waypoint key in {}", path.display()))?;
            rows.entry(key).or_insert(OriginalWaypointRow {
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
            });
        }
    }
    Ok(rows)
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

fn resolve_origin_name(
    loc: &LocalizationTable,
    origin_waypoint_id: Option<u32>,
    origin_region_id: Option<u32>,
) -> Option<String> {
    origin_waypoint_id
        .and_then(|id| localized_name(&loc.node, id))
        .or_else(|| origin_region_id.and_then(|id| localized_name(&loc.town, id)))
        .or_else(|| origin_region_id.and_then(|id| localized_name(&loc.node, id)))
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
        decode_regiongroupinfo_bss_rows, decode_regioninfo_bss_rows,
        load_region_group_mapping_from_regioninfo_bss, parse_attr_f64, parse_attr_u32, PABR_MAGIC,
        REGIONGROUPINFO_ROW_GRAPHX_OFFSET, REGIONGROUPINFO_ROW_GRAPHY_OFFSET,
        REGIONGROUPINFO_ROW_GRAPHZ_OFFSET, REGIONGROUPINFO_ROW_LEN,
        REGIONGROUPINFO_ROW_WAYPOINT_OFFSET, REGIONINFO_ROW_ACCESSIBLE_OFFSET,
        REGIONINFO_ROW_GROUP_OFFSET, REGIONINFO_ROW_SIGNATURE_OFFSET,
        REGIONINFO_ROW_SIGNATURE_PREFIX, REGIONINFO_ROW_TRADE_ORIGIN_OFFSET,
        REGIONINFO_ROW_WAYPOINT_PRIMARY_OFFSET,
    };
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
}
