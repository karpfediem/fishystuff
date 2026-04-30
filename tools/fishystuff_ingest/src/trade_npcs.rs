use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::File;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use calamine::{open_workbook_auto, Data, Range, Reader};
use fishystuff_api::models::trade::{
    ExcludedTradeNpc, TradeNpcCatalogResponse, TradeNpcCatalogSummary, TradeNpcDestination,
    TradeNpcSourceDescriptor, TradeNpcSpawn, TradeNpcTradeInfo, TradeOriginRegion,
    TradeRegionWaypointRef, TradeZoneOriginRegion, TradeZoneOriginRegions,
};
use fishystuff_core::field::DiscreteFieldRows;
use fishystuff_core::gamecommondata::{
    load_original_region_layer_context, OriginalRegionLayerContext, RegionOriginInfo,
};
use fishystuff_core::loc::load_loc_namespaces_as_string_maps;
use serde::Serialize;

const TRADE_NAME_KO: &str = "무역";
const TRADE_MANAGER_TITLE_KO: &str = "<무역 관리>";

pub type TradeNpcDestinationsBuildSummary = TradeNpcCatalogSummary;

pub struct TradeNpcBuildInputs<'a> {
    pub character_function_xlsx: &'a Path,
    pub character_table_xlsx: &'a Path,
    pub selling_to_npc_xlsx: &'a Path,
    pub regionclientdata: &'a Path,
    pub regioninfo_bss: &'a Path,
    pub regiongroupinfo_bss: &'a Path,
    pub loc: &'a Path,
    pub waypoint_xml: &'a [PathBuf],
    pub zone_mask_field: Option<&'a Path>,
    pub regions_field: Option<&'a Path>,
}

#[derive(Debug, Default)]
struct TradeFunctionRecord {
    npc_key: u32,
    is_regular_trade: bool,
    is_barter: bool,
    item_main_group_key: Option<u32>,
    trade_group_type: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct CandidateTradeNpc {
    npc_key: u32,
    character_function_trade: bool,
    character_function_barter: bool,
    selling_to_npc: bool,
    character_title_trade_manager: bool,
    item_main_group_key: Option<u32>,
    trade_group_type: Option<String>,
}

impl CandidateTradeNpc {
    fn role_source(&self) -> &'static str {
        if self.character_function_trade {
            "character_function_trade"
        } else {
            "character_title_trade_manager"
        }
    }

    fn source_tags(&self, has_regionclientdata_spawn: bool) -> Vec<String> {
        let mut tags = Vec::new();
        if self.character_function_trade {
            tags.push("character_function_trade".to_string());
        }
        if self.selling_to_npc {
            tags.push("selling_to_npc".to_string());
        }
        if self.character_title_trade_manager {
            tags.push("character_title_trade_manager".to_string());
        }
        if self.character_function_barter {
            tags.push("character_function_barter".to_string());
        }
        if has_regionclientdata_spawn {
            tags.push("regionclientdata_spawn".to_string());
        }
        tags
    }
}

#[derive(Debug, Clone)]
struct SpawnInfo {
    region_id: u32,
    dialog_index: Option<u32>,
    world_x: f64,
    world_y: f64,
    world_z: f64,
}

struct SourceTable {
    sheet_name: String,
    headers: HashMap<String, usize>,
    rows: Vec<Vec<Data>>,
}

pub fn build_trade_npc_destinations(
    inputs: TradeNpcBuildInputs<'_>,
    catalog_out: &Path,
) -> Result<TradeNpcDestinationsBuildSummary> {
    let function_records = load_character_function_records(inputs.character_function_xlsx)?;
    let selling_to_npc_keys = load_selling_to_npc_keys(inputs.selling_to_npc_xlsx)?;
    let title_trade_manager_keys = load_trade_manager_title_keys(inputs.character_table_xlsx)?;
    let spawns_by_npc = load_regionclientdata_spawns(inputs.regionclientdata)?;
    let loc_maps = load_loc_namespaces_as_string_maps(inputs.loc, &[6, 17, 29], 10_000)
        .with_context(|| format!("load localization names: {}", inputs.loc.display()))?;
    let npc_names = loc_maps.get(&6);
    let context = load_original_region_layer_context(
        inputs.regioninfo_bss,
        inputs.regiongroupinfo_bss,
        inputs.waypoint_xml,
        inputs.loc,
    )?;

    let mut summary = TradeNpcDestinationsBuildSummary {
        character_function_trade_rows: function_records
            .iter()
            .filter(|record| record.is_regular_trade)
            .count(),
        character_function_barter_rows: function_records
            .iter()
            .filter(|record| record.is_barter)
            .count(),
        character_function_trade_barter_overlap_rows: function_records
            .iter()
            .filter(|record| record.is_regular_trade && record.is_barter)
            .count(),
        selling_to_npc_rows: selling_to_npc_keys.len(),
        title_trade_manager_rows: title_trade_manager_keys.len(),
        candidate_npcs: 0,
        origin_regions: 0,
        zone_origin_regions: 0,
        destinations: 0,
        excluded_missing_spawn: 0,
        excluded_missing_trade_origin: 0,
    };
    let origin_regions = build_origin_regions(&context);
    summary.origin_regions = origin_regions.len();
    let zone_origin_regions =
        build_zone_origin_regions(&context, inputs.zone_mask_field, inputs.regions_field)?;
    summary.zone_origin_regions = zone_origin_regions.len();

    let mut candidates = BTreeMap::<u32, CandidateTradeNpc>::new();
    for record in function_records {
        if !record.is_regular_trade {
            continue;
        }
        let candidate = candidates
            .entry(record.npc_key)
            .or_insert_with(|| CandidateTradeNpc {
                npc_key: record.npc_key,
                ..CandidateTradeNpc::default()
            });
        candidate.character_function_trade = true;
        candidate.character_function_barter |= record.is_barter;
        candidate.item_main_group_key =
            candidate.item_main_group_key.or(record.item_main_group_key);
        if candidate.trade_group_type.is_none() {
            candidate.trade_group_type = record.trade_group_type;
        }
    }
    for npc_key in title_trade_manager_keys {
        let candidate = candidates
            .entry(npc_key)
            .or_insert_with(|| CandidateTradeNpc {
                npc_key,
                ..CandidateTradeNpc::default()
            });
        candidate.character_title_trade_manager = true;
    }
    for npc_key in selling_to_npc_keys {
        if let Some(candidate) = candidates.get_mut(&npc_key) {
            candidate.selling_to_npc = true;
        }
    }
    summary.candidate_npcs = candidates.len();

    let mut destinations = Vec::new();
    let mut excluded = Vec::new();
    for candidate in candidates.values() {
        let npc_name = npc_display_name(npc_names, candidate.npc_key);
        let Some(spawns) = spawns_by_npc
            .get(&candidate.npc_key)
            .filter(|spawns| !spawns.is_empty())
        else {
            summary.excluded_missing_spawn += 1;
            excluded.push(ExcludedTradeNpc {
                npc_key: candidate.npc_key,
                npc_name,
                reason: "missing_regionclientdata_spawn".to_string(),
                source_tags: candidate.source_tags(false),
            });
            continue;
        };

        for (spawn_idx, spawn) in spawns.iter().enumerate() {
            let Some(trade_origin) = context
                .resolve_region_origin_info(spawn.region_id)
                .filter(|info| info.world_x.is_some() && info.world_z.is_some())
            else {
                summary.excluded_missing_trade_origin += 1;
                excluded.push(ExcludedTradeNpc {
                    npc_key: candidate.npc_key,
                    npc_name: npc_name.clone(),
                    reason: "missing_assigned_region_trade_origin".to_string(),
                    source_tags: candidate.source_tags(true),
                });
                continue;
            };

            let assigned_region = context.resolve_region_waypoint_info(spawn.region_id);
            let assigned_region_ref =
                region_waypoint_ref(assigned_region.as_ref(), Some(spawn.region_id));
            let trade_origin_ref = region_waypoint_ref(Some(&trade_origin), trade_origin.region_id);
            let region_name = assigned_region_ref.region_name.clone();
            destinations.push(TradeNpcDestination {
                id: destination_id(candidate.npc_key, spawn_idx, spawns.len()),
                npc_key: candidate.npc_key,
                npc_name: npc_name.clone(),
                role_source: candidate.role_source().to_string(),
                source_tags: candidate.source_tags(true),
                trade: TradeNpcTradeInfo {
                    item_main_group_key: candidate.item_main_group_key,
                    trade_group_type: candidate.trade_group_type.clone(),
                },
                npc_spawn: TradeNpcSpawn {
                    region_id: spawn.region_id,
                    region_name,
                    world_x: spawn.world_x,
                    world_y: spawn.world_y,
                    world_z: spawn.world_z,
                },
                assigned_region: assigned_region_ref,
                sell_destination_trade_origin: trade_origin_ref,
            });
        }
    }

    destinations.sort_by(|left, right| {
        left.assigned_region
            .region_name
            .cmp(&right.assigned_region.region_name)
            .then_with(|| left.npc_name.cmp(&right.npc_name))
            .then_with(|| left.npc_key.cmp(&right.npc_key))
            .then_with(|| left.id.cmp(&right.id))
    });
    excluded.sort_by(|left, right| {
        left.reason
            .cmp(&right.reason)
            .then_with(|| left.npc_name.cmp(&right.npc_name))
            .then_with(|| left.npc_key.cmp(&right.npc_key))
    });

    summary.destinations = destinations.len();

    let mut sources = vec![
        source_descriptor(
            "character_function_table",
            inputs.character_function_xlsx,
            "regular Trade NPC classification and trade item group metadata",
        ),
        source_descriptor(
            "selling_to_npc_table",
            inputs.selling_to_npc_xlsx,
            "cross-check for NPCs that accept regular trade item sales",
        ),
        source_descriptor(
            "character_table",
            inputs.character_table_xlsx,
            "exact Trade Manager title classification",
        ),
        source_descriptor(
            "regionclientdata",
            inputs.regionclientdata,
            "NPC spawn region and world coordinates",
        ),
        source_descriptor(
            "regioninfo_bss",
            inputs.regioninfo_bss,
            "assigned region tradeoriginregion linkage",
        ),
        source_descriptor(
            "waypoint_xml",
            inputs
                .waypoint_xml
                .first()
                .map(|path| path.as_path())
                .unwrap_or(Path::new("")),
            "region and trade-origin waypoint coordinates",
        ),
        source_descriptor(
            "languagedata_loc",
            inputs.loc,
            "localized NPC and region names",
        ),
    ];
    if let Some(path) = inputs.zone_mask_field {
        sources.push(source_descriptor(
            "zone_mask_field",
            path,
            "zone mask field used to precompute trade origins per fishing zone",
        ));
    }
    if let Some(path) = inputs.regions_field {
        sources.push(source_descriptor(
            "regions_field",
            path,
            "region field used to precompute trade origins per fishing zone",
        ));
    }

    let catalog = TradeNpcCatalogResponse {
        schema: "fishystuff.trade_npc_destinations".to_string(),
        version: 1,
        coordinate_space: "bdo_world_xz".to_string(),
        sources,
        summary,
        origin_regions,
        zone_origin_regions,
        destinations: destinations.clone(),
        excluded,
    };
    write_json(catalog_out, &catalog)?;

    Ok(summary)
}

fn build_origin_regions(context: &OriginalRegionLayerContext) -> Vec<TradeOriginRegion> {
    let mut origin_regions = BTreeMap::<u32, TradeOriginRegion>::new();
    for region_id in context.region_ids() {
        let Some(info) = context.resolve_region_origin_info(region_id) else {
            continue;
        };
        let Some(origin_region_id) = info.region_id else {
            continue;
        };
        let (Some(world_x), Some(world_z)) = (info.world_x, info.world_z) else {
            continue;
        };
        origin_regions
            .entry(origin_region_id)
            .or_insert_with(|| TradeOriginRegion {
                region_id: origin_region_id,
                region_name: info.region_name,
                waypoint_id: info.waypoint_id,
                waypoint_name: info.waypoint_name,
                world_x,
                world_z,
            });
    }
    let mut origin_regions = origin_regions.into_values().collect::<Vec<_>>();
    origin_regions.sort_by(|left, right| {
        left.region_name
            .cmp(&right.region_name)
            .then_with(|| left.region_id.cmp(&right.region_id))
    });
    origin_regions
}

fn build_zone_origin_regions(
    context: &OriginalRegionLayerContext,
    zone_mask_field: Option<&Path>,
    regions_field: Option<&Path>,
) -> Result<Vec<TradeZoneOriginRegions>> {
    let (Some(zone_mask_field), Some(regions_field)) = (zone_mask_field, regions_field) else {
        return Ok(Vec::new());
    };
    let zone_mask = load_discrete_field(zone_mask_field, "zone mask field")?;
    let regions = load_discrete_field(regions_field, "regions field")?;
    if zone_mask.width() != regions.width() || zone_mask.height() != regions.height() {
        bail!(
            "zone mask field dimensions {}x{} do not match regions field dimensions {}x{}",
            zone_mask.width(),
            zone_mask.height(),
            regions.width(),
            regions.height()
        );
    }

    let mut origin_cache = HashMap::<u32, Option<u32>>::new();
    let mut origins_by_zone = BTreeMap::<u32, BTreeMap<u32, u32>>::new();
    zone_mask.for_each_span(|y, start_x, end_x, zone_rgb_u32| {
        if zone_rgb_u32 == 0 {
            return;
        }
        for x in start_x..end_x {
            let region_id = regions
                .cell_id_u32(i32::from(x), i32::from(y))
                .unwrap_or_default();
            if region_id == 0 {
                continue;
            }
            let origin_region_id = if let Some(cached) = origin_cache.get(&region_id) {
                *cached
            } else {
                let resolved = context
                    .resolve_region_origin_info(region_id)
                    .filter(|info| info.world_x.is_some() && info.world_z.is_some())
                    .and_then(|info| info.region_id);
                origin_cache.insert(region_id, resolved);
                resolved
            };
            let Some(origin_region_id) = origin_region_id else {
                continue;
            };
            let count = origins_by_zone
                .entry(zone_rgb_u32)
                .or_default()
                .entry(origin_region_id)
                .or_default();
            *count = count.saturating_add(1);
        }
    });

    Ok(origins_by_zone
        .into_iter()
        .map(|(zone_rgb_u32, counts)| {
            let mut origins = counts
                .into_iter()
                .map(|(region_id, pixel_count)| TradeZoneOriginRegion {
                    region_id,
                    pixel_count,
                })
                .collect::<Vec<_>>();
            origins.sort_by(|left, right| {
                right
                    .pixel_count
                    .cmp(&left.pixel_count)
                    .then_with(|| left.region_id.cmp(&right.region_id))
            });
            TradeZoneOriginRegions {
                zone_rgb_key: rgb_key(zone_rgb_u32),
                zone_rgb_u32,
                origins,
            }
        })
        .collect())
}

fn load_discrete_field(path: &Path, label: &str) -> Result<DiscreteFieldRows> {
    let bytes = std::fs::read(path).with_context(|| format!("read {label}: {}", path.display()))?;
    DiscreteFieldRows::from_bytes(&bytes)
        .with_context(|| format!("decode {label}: {}", path.display()))
}

fn rgb_key(rgb: u32) -> String {
    format!(
        "{},{},{}",
        (rgb >> 16) & 0xff,
        (rgb >> 8) & 0xff,
        rgb & 0xff
    )
}

fn load_character_function_records(path: &Path) -> Result<Vec<TradeFunctionRecord>> {
    let table = read_source_table(path, &["CharacterKey", "TradingName", "TradingNPCType"])?;
    let mut records = Vec::new();
    for row in &table.rows {
        let Some(npc_key) = table_cell_u32(&table, row, "CharacterKey")? else {
            continue;
        };
        let trading_name = table_cell_string(&table, row, "TradingName")?.unwrap_or_default();
        let trading_npc_type = table_cell_u32(&table, row, "TradingNPCType")?.unwrap_or_default();
        let item_main_group_key = optional_table_cell_u32(&table, row, "ItemMainGroupKey")?;
        let trade_group_type = optional_table_cell_string(&table, row, "TradeGroupType")?;
        let barter_name = optional_table_cell_string(&table, row, "BarterName")?;
        let condition_barter = optional_table_cell_string(&table, row, "ConditionBarter")?;
        let is_barter_flag = optional_table_cell_u32(&table, row, "isBarter")?.unwrap_or_default();
        let is_barter = is_barter_flag != 0 || barter_name.is_some() || condition_barter.is_some();
        records.push(TradeFunctionRecord {
            npc_key,
            is_regular_trade: trading_name == TRADE_NAME_KO && trading_npc_type == 1,
            is_barter,
            item_main_group_key,
            trade_group_type,
        });
    }
    Ok(records)
}

fn load_selling_to_npc_keys(path: &Path) -> Result<BTreeSet<u32>> {
    let table = read_source_table(path, &["NPCKey"])?;
    let mut keys = BTreeSet::new();
    for row in &table.rows {
        if let Some(npc_key) = table_cell_u32(&table, row, "NPCKey")? {
            keys.insert(npc_key);
        }
    }
    Ok(keys)
}

fn load_trade_manager_title_keys(path: &Path) -> Result<BTreeSet<u32>> {
    let table = read_source_table(path, &["Index", "CharacterTitle"])?;
    let mut keys = BTreeSet::new();
    for row in &table.rows {
        let Some(npc_key) = table_cell_u32(&table, row, "Index")? else {
            continue;
        };
        let title = table_cell_string(&table, row, "CharacterTitle")?.unwrap_or_default();
        if title == TRADE_MANAGER_TITLE_KO {
            keys.insert(npc_key);
        }
    }
    Ok(keys)
}

fn load_regionclientdata_spawns(path: &Path) -> Result<BTreeMap<u32, Vec<SpawnInfo>>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("read regionclientdata: {}", path.display()))?;
    let mut current_region_id = None;
    let mut spawns_by_npc = BTreeMap::<u32, Vec<SpawnInfo>>::new();

    for line in text.lines() {
        if line.contains("<RegionInfo ") {
            current_region_id = xml_attr(line, "Key")
                .or_else(|| xml_attr(line, "key"))
                .and_then(|value| value.parse::<u32>().ok());
        }
        if line.contains("<SpawnInfo ") {
            let Some(region_id) = current_region_id else {
                continue;
            };
            let Some(npc_key) = xml_attr(line, "key").and_then(|value| value.parse::<u32>().ok())
            else {
                continue;
            };
            let dialog_index = xml_attr(line, "dialogIndex").and_then(|value| value.parse().ok());
            let Some(position) = xml_attr(line, "position").and_then(parse_position) else {
                continue;
            };
            spawns_by_npc.entry(npc_key).or_default().push(SpawnInfo {
                region_id,
                dialog_index,
                world_x: position.0,
                world_y: position.1,
                world_z: position.2,
            });
        }
        if line.contains("</RegionInfo>") {
            current_region_id = None;
        }
    }

    for spawns in spawns_by_npc.values_mut() {
        spawns.sort_by(|left, right| {
            left.region_id
                .cmp(&right.region_id)
                .then_with(|| left.dialog_index.cmp(&right.dialog_index))
                .then_with(|| left.world_x.total_cmp(&right.world_x))
                .then_with(|| left.world_z.total_cmp(&right.world_z))
        });
    }

    Ok(spawns_by_npc)
}

fn read_source_table(path: &Path, required_headers: &[&str]) -> Result<SourceTable> {
    let mut workbook =
        open_workbook_auto(path).with_context(|| format!("open workbook: {}", path.display()))?;
    let sheet_names = workbook.sheet_names().to_vec();
    for sheet_name in sheet_names {
        let range = workbook
            .worksheet_range(&sheet_name)
            .with_context(|| format!("read sheet '{sheet_name}' in {}", path.display()))?;
        if let Some(table) = source_table_from_range(&sheet_name, &range, required_headers) {
            return Ok(table);
        }
    }
    bail!(
        "no sheet in {} contains required headers: {}",
        path.display(),
        required_headers.join(", ")
    )
}

fn source_table_from_range(
    sheet_name: &str,
    range: &Range<Data>,
    required_headers: &[&str],
) -> Option<SourceTable> {
    let mut rows = range.rows();
    let header_row = rows.next()?;
    let headers = header_row
        .iter()
        .enumerate()
        .filter_map(|(idx, cell)| {
            let value = header_cell_to_string(cell);
            (!value.is_empty()).then_some((value, idx))
        })
        .collect::<HashMap<_, _>>();
    if required_headers
        .iter()
        .any(|header| !headers.contains_key(*header))
    {
        return None;
    }
    Some(SourceTable {
        sheet_name: sheet_name.to_string(),
        headers,
        rows: rows.map(|row| row.to_vec()).collect(),
    })
}

fn table_cell_string(table: &SourceTable, row: &[Data], header: &str) -> Result<Option<String>> {
    let idx = required_header_index(table, header)?;
    cell_to_string(row.get(idx))
}

fn table_cell_u32(table: &SourceTable, row: &[Data], header: &str) -> Result<Option<u32>> {
    let idx = required_header_index(table, header)?;
    cell_to_u32(row.get(idx))
}

fn optional_table_cell_string(
    table: &SourceTable,
    row: &[Data],
    header: &str,
) -> Result<Option<String>> {
    let Some(idx) = table.headers.get(header).copied() else {
        return Ok(None);
    };
    cell_to_string(row.get(idx))
}

fn optional_table_cell_u32(table: &SourceTable, row: &[Data], header: &str) -> Result<Option<u32>> {
    let Some(idx) = table.headers.get(header).copied() else {
        return Ok(None);
    };
    cell_to_u32(row.get(idx))
}

fn required_header_index(table: &SourceTable, header: &str) -> Result<usize> {
    table
        .headers
        .get(header)
        .copied()
        .with_context(|| format!("missing header '{header}' in sheet {}", table.sheet_name))
}

fn header_cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(value) => value.trim().to_string(),
        _ => cell.to_string().trim().to_string(),
    }
}

fn cell_to_string(cell: Option<&Data>) -> Result<Option<String>> {
    let Some(cell) = cell else {
        return Ok(None);
    };
    match cell {
        Data::Empty => Ok(None),
        Data::String(value) => normalized_string(value),
        Data::DateTimeIso(value) | Data::DurationIso(value) => normalized_string(value),
        Data::Float(value) => Ok(Some(format_float(*value))),
        Data::Int(value) => Ok(Some(value.to_string())),
        Data::Bool(value) => Ok(Some(if *value { "1" } else { "0" }.to_string())),
        Data::DateTime(value) => Ok(Some(format_float(value.as_f64()))),
        Data::Error(err) => bail!("cell error: {err:?}"),
    }
}

fn cell_to_u32(cell: Option<&Data>) -> Result<Option<u32>> {
    let Some(value) = cell_to_string(cell)? else {
        return Ok(None);
    };
    let normalized = value.replace('_', "");
    let parsed = normalized
        .parse::<u32>()
        .with_context(|| format!("parse u32 cell value: {value}"))?;
    Ok(Some(parsed))
}

fn normalized_string(value: &str) -> Result<Option<String>> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") || trimmed == "<null>" {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_string()))
    }
}

fn format_float(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

fn region_waypoint_ref(
    info: Option<&RegionOriginInfo>,
    fallback_region_id: Option<u32>,
) -> TradeRegionWaypointRef {
    TradeRegionWaypointRef {
        region_id: info.and_then(|info| info.region_id).or(fallback_region_id),
        region_name: info.and_then(|info| info.region_name.clone()),
        waypoint_id: info.and_then(|info| info.waypoint_id),
        waypoint_name: info.and_then(|info| info.waypoint_name.clone()),
        world_x: info.and_then(|info| info.world_x),
        world_z: info.and_then(|info| info.world_z),
    }
}

fn npc_display_name(npc_names: Option<&BTreeMap<String, String>>, npc_key: u32) -> String {
    npc_names
        .and_then(|names| names.get(&npc_key.to_string()))
        .cloned()
        .unwrap_or_else(|| format!("NPC {npc_key}"))
}

fn destination_id(npc_key: u32, spawn_idx: usize, spawn_count: usize) -> String {
    if spawn_count <= 1 {
        npc_key.to_string()
    } else {
        format!("{npc_key}:{}", spawn_idx + 1)
    }
}

fn source_descriptor(
    id: &'static str,
    path: &Path,
    role: &'static str,
) -> TradeNpcSourceDescriptor {
    TradeNpcSourceDescriptor {
        id: id.to_string(),
        file: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        role: role.to_string(),
    }
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create output directory: {}", parent.display()))?;
    }
    let file =
        File::create(path).with_context(|| format!("create output json: {}", path.display()))?;
    serde_json::to_writer_pretty(file, value)
        .with_context(|| format!("write output json: {}", path.display()))?;
    Ok(())
}

fn xml_attr<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("{name}=\"");
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

fn parse_position(value: &str) -> Option<(f64, f64, f64)> {
    let trimmed = value.trim().trim_start_matches('{').trim_end_matches('}');
    let mut parts = trimmed.split(',').map(str::trim);
    let x = parts.next()?.parse::<f64>().ok()?;
    let y = parts.next()?.parse::<f64>().ok()?;
    let z = parts.next()?.parse::<f64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((x, y, z))
}

#[cfg(test)]
mod tests {
    use super::{parse_position, source_table_from_range, xml_attr, TRADE_MANAGER_TITLE_KO};
    use calamine::{Data, Range};

    #[test]
    fn parses_regionclientdata_position() {
        let position = parse_position("{-258154.188,-7180.491,-391910.594}").unwrap();
        assert_eq!(position.0, -258154.188);
        assert_eq!(position.1, -7180.491);
        assert_eq!(position.2, -391910.594);
    }

    #[test]
    fn extracts_xml_attr() {
        let line = r#"<SpawnInfo key="40010" dialogIndex="1" position="{-1.0,2.0,3.0}" />"#;
        assert_eq!(xml_attr(line, "key"), Some("40010"));
        assert_eq!(xml_attr(line, "position"), Some("{-1.0,2.0,3.0}"));
    }

    #[test]
    fn source_table_requires_all_headers() {
        let mut range = Range::new((0, 0), (1, 2));
        range.set_value((0, 0), Data::String("Index".to_string()));
        range.set_value((0, 1), Data::String("CharacterTitle".to_string()));
        range.set_value((1, 0), Data::Int(47409));
        range.set_value((1, 1), Data::String(TRADE_MANAGER_TITLE_KO.to_string()));
        let table =
            source_table_from_range("Character_Table", &range, &["Index", "CharacterTitle"])
                .unwrap();
        assert_eq!(table.rows.len(), 1);
    }
}
