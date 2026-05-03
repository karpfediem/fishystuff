use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::path::Path;

use anyhow::{bail, Context, Result};
use calamine::{open_workbook_auto, Data, Range, Reader};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const POINT_HEADERS: &[&str] = &[
    "PointKey",
    "PointSize",
    "StartPositionX",
    "StartPositionY",
    "StartPositionZ",
    "EndPositionX",
    "EndPositionZ",
    "FishingGroupKey",
    "SpawnRate",
    "SpawnCharacterKey",
    "SpawnActionIndex",
    "ContentsGroupKey",
];

const FISHING_HEADERS: &[&str] = &[
    "FishingGroupKey",
    "DropRate1",
    "DropID1",
    "DropRate2",
    "DropID2",
    "DropRate3",
    "DropID3",
    "DropRate4",
    "DropID4",
    "DropRate5",
    "DropID5",
    "MinWaitTime",
    "MaxWaitTime",
    "PointRemainTime",
    "MinFishCount",
    "MaxFishCount",
    "AvailableFishingLevel",
    "ObserveFishingLevel",
    "ContentsGroupKey",
];

#[derive(Debug, Clone, Copy)]
pub struct HotspotBuildSummary {
    pub hotspot_count: usize,
    pub source_point_rows: usize,
    pub source_fishing_group_rows: usize,
    pub source_loot_option_rows: usize,
    pub source_loot_variant_rows: usize,
    pub imported_metadata_rows: usize,
    pub matched_imported_metadata_rows: usize,
    pub expanded_degenerate_rectangles: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HotspotAsset {
    schema: &'static str,
    version: u32,
    coordinate_space: &'static str,
    sources: Vec<HotspotSourceDescriptor>,
    summary: HotspotAssetSummary,
    hotspots: Vec<HotspotAssetRecord>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HotspotSourceDescriptor {
    id: &'static str,
    file: String,
    role: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HotspotAssetSummary {
    hotspot_count: usize,
    source_point_rows: usize,
    source_fishing_group_rows: usize,
    source_loot_option_rows: usize,
    source_loot_variant_rows: usize,
    imported_metadata_rows: usize,
    matched_imported_metadata_rows: usize,
    expanded_degenerate_rectangles: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HotspotAssetRecord {
    id: u32,
    point_size: f64,
    start_x: f64,
    start_y: f64,
    start_z: f64,
    end_x: f64,
    end_z: f64,
    min_x: f64,
    min_z: f64,
    max_x: f64,
    max_z: f64,
    center_x: f64,
    center_z: f64,
    primary_fish_item_id: Option<u32>,
    primary_fish_name: Option<String>,
    primary_fish_icon_image: Option<String>,
    loot_items: Vec<HotspotLootItem>,
    loot_groups: Vec<HotspotLootGroup>,
    fishing_group_key: u32,
    spawn_rate: Option<u32>,
    spawn_character_key: Option<u32>,
    spawn_action_index: Option<u32>,
    point_contents_group_key: Option<u32>,
    fishing_contents_group_key: Option<u32>,
    drop_groups: Vec<HotspotDropGroup>,
    min_wait_time: Option<u32>,
    max_wait_time: Option<u32>,
    point_remain_time: Option<u32>,
    min_fish_count: Option<u32>,
    max_fish_count: Option<u32>,
    available_fishing_level: Option<u32>,
    observe_fishing_level: Option<u32>,
    source_stats: HotspotSourceStats,
    imported_metadata: Option<HotspotImportedMetadata>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HotspotDropGroup {
    slot: u8,
    drop_rate: Option<u32>,
    group_key: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HotspotSourceStats {
    min_wait_time: u32,
    max_wait_time: u32,
    point_remain_time: u32,
    min_fish_count: u32,
    max_fish_count: u32,
    available_fishing_level: u32,
    observe_fishing_level: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HotspotImportedMetadata {
    source_id: &'static str,
    source_hotspot_id: u32,
    min_wait_time: Option<u32>,
    max_wait_time: Option<u32>,
    point_remain_time: Option<u32>,
    min_fish_count: Option<u32>,
    max_fish_count: Option<u32>,
    available_fishing_level: Option<u32>,
    observe_fishing_level: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HotspotLootItem {
    item_id: u32,
    name: String,
    label: String,
    slot_idx: u8,
    group_label: String,
    select_rate: Option<u32>,
    grade_type: Option<u32>,
    icon_item_id: Option<u32>,
    icon_image: Option<String>,
    icon_grade_tone: String,
    drop_rate_text: String,
    drop_rate_source_kind: String,
    drop_rate_tooltip: String,
    raw_drop_rate_text: String,
    raw_drop_rate_tooltip: String,
    normalized_drop_rate_text: String,
    normalized_drop_rate_tooltip: String,
    catch_methods: Vec<String>,
    is_fish: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HotspotLootGroup {
    slot_idx: u8,
    label: String,
    condition_option_key: String,
    item_main_group_key: u32,
    drop_rate: Option<u32>,
    drop_rate_text: String,
    drop_rate_source_kind: String,
    drop_rate_tooltip: String,
    raw_drop_rate_text: String,
    raw_drop_rate_tooltip: String,
    normalized_drop_rate_text: String,
    normalized_drop_rate_tooltip: String,
    catch_methods: Vec<String>,
    condition_options: Vec<HotspotLootConditionOption>,
}

#[derive(Debug, Clone)]
struct FishingPointRecord {
    point_key: u32,
    point_size: f64,
    start_x: f64,
    start_y: f64,
    start_z: f64,
    end_x: f64,
    end_z: f64,
    fishing_group_key: u32,
    spawn_rate: Option<u32>,
    spawn_character_key: Option<u32>,
    spawn_action_index: Option<u32>,
    contents_group_key: Option<u32>,
}

#[derive(Debug, Clone)]
struct FishingGroupRecord {
    fishing_group_key: u32,
    drop_groups: Vec<HotspotDropGroup>,
    min_wait_time: Option<u32>,
    max_wait_time: Option<u32>,
    point_remain_time: Option<u32>,
    min_fish_count: Option<u32>,
    max_fish_count: Option<u32>,
    available_fishing_level: Option<u32>,
    observe_fishing_level: Option<u32>,
    contents_group_key: Option<u32>,
    source_stats: HotspotSourceStats,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HotspotLootConditionOption {
    option_idx: u32,
    condition_key: String,
    condition_text: String,
    condition_tooltip: String,
    item_sub_group_key: u32,
    select_rate: Option<u32>,
    drop_rate_text: String,
    drop_rate_source_kind: String,
    drop_rate_tooltip: String,
    raw_drop_rate_text: String,
    raw_drop_rate_tooltip: String,
    normalized_drop_rate_text: String,
    normalized_drop_rate_tooltip: String,
    active: bool,
    species_rows: Vec<HotspotLootItem>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct SourceLootRowsPayload {
    rows: Vec<SourceLootRow>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct BdolyticsHotspotsPayload {
    data: Vec<BdolyticsHotspotRecord>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct BdolyticsHotspotRecord {
    id: u32,
    min_wait_time: Option<u32>,
    max_wait_time: Option<u32>,
    point_remain_time: Option<u32>,
    min_fish_count: Option<u32>,
    max_fish_count: Option<u32>,
    available_fishing_level: Option<u32>,
    visible_fishing_level: Option<u32>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct SourceLootRow {
    item_main_group_key: Value,
    option_idx: Value,
    option_select_rate: Value,
    condition_raw: Option<String>,
    item_sub_group_key: Value,
    item_key: Value,
    item_select_rate: Value,
    item_name: Option<String>,
    icon_image: Option<String>,
    grade_type: Value,
    is_fish: Value,
    item_source_tooltip: Option<String>,
}

#[derive(Debug, Default)]
struct SourceLootLookup {
    options_by_main_group: BTreeMap<u32, Vec<SourceLootOption>>,
    option_rows: usize,
    variant_rows: usize,
}

#[derive(Debug, Clone)]
struct SourceLootOption {
    option_idx: u32,
    select_rate: Option<u32>,
    condition_raw: Option<String>,
    item_sub_group_key: u32,
    items: Vec<SourceLootItem>,
}

#[derive(Debug, Clone)]
struct SourceLootItem {
    item_id: u32,
    name: String,
    select_rate: u32,
    grade_type: Option<u32>,
    icon_item_id: Option<u32>,
    icon_image: Option<String>,
    is_fish: bool,
    source_tooltip: Option<String>,
}

#[derive(Debug, Clone)]
struct ExpandedSourceLootOption {
    branch_idx: u32,
    select_rate: Option<u32>,
    condition_raw: Option<String>,
    item_sub_group_key: u32,
    items: Vec<SourceLootItem>,
    lineage: Vec<SourceLootLineage>,
}

#[derive(Debug, Clone)]
struct SourceLootLineage {
    item_main_group_key: u32,
    option_idx: u32,
    item_sub_group_key: u32,
}

struct SourceTable {
    sheet_name: String,
    headers: BTreeMap<String, usize>,
    rows: Vec<Vec<Data>>,
}

pub fn build_hotspot_asset(
    float_fishing_point_xlsx: &Path,
    float_fishing_xlsx: &Path,
    source_loot_groups_json: &Path,
    bdolytics_hotspots_json: Option<&Path>,
    out_path: &Path,
) -> Result<HotspotBuildSummary> {
    let point_records = load_fishing_point_records(float_fishing_point_xlsx)?;
    let group_records = load_fishing_group_records(float_fishing_xlsx)?;
    let source_loot = load_source_loot_lookup(source_loot_groups_json)?;
    let imported_metadata = load_bdolytics_metadata_lookup(bdolytics_hotspots_json)?;
    let group_by_key = group_records
        .iter()
        .map(|group| (group.fishing_group_key, group))
        .collect::<BTreeMap<_, _>>();

    let mut expanded_degenerate_rectangles = 0usize;
    let mut matched_imported_metadata_rows = 0usize;
    let mut hotspots = Vec::with_capacity(point_records.len());
    for point in &point_records {
        let group = group_by_key
            .get(&point.fishing_group_key)
            .with_context(|| {
                format!(
                    "missing FloatFishing_Table group {} for point {}",
                    point.fishing_group_key, point.point_key
                )
            })?;
        let bounds = hotspot_bounds(
            point.start_x,
            point.start_z,
            point.end_x,
            point.end_z,
            point.point_size,
        );
        if bounds.expanded_degenerate_axis {
            expanded_degenerate_rectangles = expanded_degenerate_rectangles.saturating_add(1);
        }
        let loot_groups = source_loot.hotspot_loot_groups(point.point_key, &group.drop_groups);
        let loot_items = active_loot_items(&loot_groups);
        let primary_fish = primary_fish_identity(&loot_groups);
        let imported_metadata = imported_metadata.get(&point.point_key).cloned();
        if imported_metadata.is_some() {
            matched_imported_metadata_rows = matched_imported_metadata_rows.saturating_add(1);
        }
        hotspots.push(HotspotAssetRecord {
            id: point.point_key,
            point_size: point.point_size,
            start_x: point.start_x,
            start_y: point.start_y,
            start_z: point.start_z,
            end_x: point.end_x,
            end_z: point.end_z,
            min_x: bounds.min_x,
            min_z: bounds.min_z,
            max_x: bounds.max_x,
            max_z: bounds.max_z,
            center_x: (bounds.min_x + bounds.max_x) * 0.5,
            center_z: (bounds.min_z + bounds.max_z) * 0.5,
            primary_fish_item_id: primary_fish.as_ref().map(|identity| identity.item_id),
            primary_fish_name: primary_fish.as_ref().map(|identity| identity.name.clone()),
            primary_fish_icon_image: primary_fish.and_then(|identity| identity.icon_image),
            loot_items,
            loot_groups,
            fishing_group_key: point.fishing_group_key,
            spawn_rate: point.spawn_rate,
            spawn_character_key: point.spawn_character_key,
            spawn_action_index: point.spawn_action_index,
            point_contents_group_key: point.contents_group_key,
            fishing_contents_group_key: group.contents_group_key,
            drop_groups: group.drop_groups.clone(),
            min_wait_time: group.min_wait_time,
            max_wait_time: group.max_wait_time,
            point_remain_time: group.point_remain_time,
            min_fish_count: group.min_fish_count,
            max_fish_count: group.max_fish_count,
            available_fishing_level: group.available_fishing_level,
            observe_fishing_level: group.observe_fishing_level,
            source_stats: group.source_stats.clone(),
            imported_metadata,
        });
    }
    hotspots.sort_by_key(|hotspot| hotspot.id);

    let summary = HotspotBuildSummary {
        hotspot_count: hotspots.len(),
        source_point_rows: point_records.len(),
        source_fishing_group_rows: group_records.len(),
        source_loot_option_rows: source_loot.option_rows,
        source_loot_variant_rows: source_loot.variant_rows,
        imported_metadata_rows: imported_metadata.len(),
        matched_imported_metadata_rows,
        expanded_degenerate_rectangles,
    };
    let mut sources = vec![
        source_descriptor(
            "float_fishing_point_table",
            float_fishing_point_xlsx,
            "hotspot point bounds, spawn metadata, and fishing group linkage",
        ),
        source_descriptor(
            "float_fishing_table",
            float_fishing_xlsx,
            "hotspot drop group, count, level, wait, and contents-group metadata",
        ),
        source_descriptor_literal(
            "dolt_hotspot_loot_groups",
            "dolt:item_main_group_options,item_sub_group_item_variants,item_table,languagedata,fish_table",
            "source item main group options, subgroup variants, item metadata, and condition branches",
        ),
    ];
    if let Some(path) = bdolytics_hotspots_json {
        sources.push(source_descriptor(
            "bdolytics_community_hotspot_metadata",
            path,
            "one-off imported hotspot count, level, bite-time, and lifetime metadata; original FloatFishing source stat columns remain preserved separately",
        ));
    }
    let asset = HotspotAsset {
        schema: "fishystuff.hotspots",
        version: 1,
        coordinate_space: "bdo_world_xz",
        sources,
        summary: HotspotAssetSummary {
            hotspot_count: summary.hotspot_count,
            source_point_rows: summary.source_point_rows,
            source_fishing_group_rows: summary.source_fishing_group_rows,
            source_loot_option_rows: summary.source_loot_option_rows,
            source_loot_variant_rows: summary.source_loot_variant_rows,
            imported_metadata_rows: summary.imported_metadata_rows,
            matched_imported_metadata_rows: summary.matched_imported_metadata_rows,
            expanded_degenerate_rectangles: summary.expanded_degenerate_rectangles,
        },
        hotspots,
    };
    write_json(out_path, &asset)?;
    Ok(summary)
}

fn item_id_from_icon_image(icon_image: &str) -> Option<u32> {
    let file = icon_image
        .trim()
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .rsplit('/')
        .next()
        .unwrap_or_default();
    let stem = file
        .strip_suffix(".dds")
        .or_else(|| file.strip_suffix(".DDS"))
        .or_else(|| file.strip_suffix(".webp"))
        .or_else(|| file.strip_suffix(".WEBP"))
        .unwrap_or(file);
    let digits = stem
        .chars()
        .rev()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }
    digits.chars().rev().collect::<String>().parse().ok()
}

struct HotspotConditionFields {
    key: String,
    text: String,
    tooltip: String,
}

fn source_condition_fields(condition_raw: Option<&str>) -> HotspotConditionFields {
    let condition = condition_raw.unwrap_or_default().trim();
    if condition.is_empty() {
        return HotspotConditionFields {
            key: "default".to_string(),
            text: "Default".to_string(),
            tooltip: "Default".to_string(),
        };
    }

    let labels = condition
        .split(';')
        .map(str::trim)
        .filter(|predicate| !predicate.is_empty())
        .map(humanize_source_condition)
        .filter(|label| !label.is_empty())
        .collect::<Vec<_>>();
    HotspotConditionFields {
        key: condition.to_string(),
        text: labels.join(" · "),
        tooltip: condition.to_string(),
    }
}

fn humanize_source_condition(predicate: &str) -> String {
    if let Some(threshold) = predicate
        .strip_prefix("getLifeLevel(1)>")
        .and_then(|value| value.parse::<u32>().ok())
        .map(|value| value.saturating_add(1))
    {
        return format!("Fishing Level {}+", lifeskill_level_label(threshold));
    }
    if let Some(threshold) = predicate
        .strip_prefix("lifestat(1,1)>")
        .and_then(|value| value.parse::<u32>().ok())
        .map(|value| value.saturating_add(1))
    {
        return format!("Fishing Mastery {threshold}+");
    }
    if let Some(contents_group) = predicate
        .strip_prefix("isContentsGroupOpen(0,")
        .and_then(|value| value.strip_suffix(')'))
    {
        return format!("Contents Group {contents_group} Open");
    }
    if let Some(contents_group) = predicate
        .strip_prefix("!isContentsGroupOpen(0,")
        .and_then(|value| value.strip_suffix(')'))
    {
        return format!("Contents Group {contents_group} Closed");
    }
    predicate.to_string()
}

fn load_source_loot_lookup(path: &Path) -> Result<SourceLootLookup> {
    let file =
        File::open(path).with_context(|| format!("open source loot rows: {}", path.display()))?;
    let payload: SourceLootRowsPayload = serde_json::from_reader(file)
        .with_context(|| format!("parse source loot rows: {}", path.display()))?;
    let mut option_by_key = BTreeMap::<(u32, u32, u32), SourceLootOption>::new();
    let mut variant_rows = 0usize;

    for row in payload.rows {
        let Some(item_main_group_key) = value_u32(&row.item_main_group_key) else {
            continue;
        };
        let Some(option_idx) = value_u32(&row.option_idx) else {
            continue;
        };
        let Some(item_sub_group_key) = value_u32(&row.item_sub_group_key) else {
            continue;
        };
        let key = (item_main_group_key, option_idx, item_sub_group_key);
        let option = option_by_key
            .entry(key)
            .or_insert_with(|| SourceLootOption {
                option_idx,
                select_rate: value_u32(&row.option_select_rate),
                condition_raw: trimmed_optional_string(row.condition_raw.as_deref()),
                item_sub_group_key,
                items: Vec::new(),
            });

        let Some(item_id) = value_u32(&row.item_key) else {
            continue;
        };
        let Some(select_rate) = value_u32(&row.item_select_rate).filter(|rate| *rate > 0) else {
            continue;
        };
        let name = trimmed_optional_string(row.item_name.as_deref())
            .unwrap_or_else(|| item_id.to_string());
        let icon_image = trimmed_optional_string(row.icon_image.as_deref());
        option.items.push(SourceLootItem {
            item_id,
            name,
            select_rate,
            grade_type: value_u32(&row.grade_type),
            icon_item_id: icon_image
                .as_deref()
                .and_then(item_id_from_icon_image)
                .or(Some(item_id)),
            icon_image,
            is_fish: value_bool(&row.is_fish),
            source_tooltip: trimmed_optional_string(row.item_source_tooltip.as_deref()),
        });
        variant_rows = variant_rows.saturating_add(1);
    }

    let mut lookup = SourceLootLookup {
        option_rows: option_by_key.len(),
        variant_rows,
        ..Default::default()
    };
    for ((item_main_group_key, _, _), mut option) in option_by_key {
        option.items.sort_by(|left, right| {
            right
                .select_rate
                .cmp(&left.select_rate)
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
                .then_with(|| left.item_id.cmp(&right.item_id))
        });
        lookup
            .options_by_main_group
            .entry(item_main_group_key)
            .or_default()
            .push(option);
    }
    for options in lookup.options_by_main_group.values_mut() {
        options.sort_by_key(|option| option.option_idx);
    }
    Ok(lookup)
}

fn load_bdolytics_metadata_lookup(
    path: Option<&Path>,
) -> Result<BTreeMap<u32, HotspotImportedMetadata>> {
    let Some(path) = path else {
        return Ok(BTreeMap::new());
    };
    let file = File::open(path)
        .with_context(|| format!("open bdolytics hotspot metadata: {}", path.display()))?;
    let payload: BdolyticsHotspotsPayload = serde_json::from_reader(file)
        .with_context(|| format!("parse bdolytics hotspot metadata: {}", path.display()))?;
    let mut rows = BTreeMap::new();
    for row in payload.data {
        if row.id == 0 {
            continue;
        }
        rows.insert(
            row.id,
            HotspotImportedMetadata {
                source_id: "bdolytics_community_hotspot_metadata",
                source_hotspot_id: row.id,
                min_wait_time: row.min_wait_time,
                max_wait_time: row.max_wait_time,
                point_remain_time: row.point_remain_time,
                min_fish_count: row.min_fish_count,
                max_fish_count: row.max_fish_count,
                available_fishing_level: row.available_fishing_level,
                observe_fishing_level: row.visible_fishing_level,
            },
        );
    }
    Ok(rows)
}

impl SourceLootLookup {
    fn hotspot_loot_groups(
        &self,
        hotspot_id: u32,
        drop_groups: &[HotspotDropGroup],
    ) -> Vec<HotspotLootGroup> {
        drop_groups
            .iter()
            .enumerate()
            .map(|(index, drop_group)| {
                let label = format!("Group {}", index + 1);
                let mut condition_options = self
                    .expanded_options_for_main_group(drop_group.group_key)
                    .into_iter()
                    .map(|option| source_loot_condition_option(&option, drop_group.slot, &label))
                    .collect::<Vec<_>>();
                mark_active_condition_option(&mut condition_options);
                let drop_rate_text = drop_group
                    .drop_rate
                    .map(rate_text_from_micros)
                    .unwrap_or_default();
                let drop_rate_tooltip = format!(
                    "FloatFishing_Table DropID{} main group {}",
                    drop_group.slot, drop_group.group_key
                );
                HotspotLootGroup {
                    slot_idx: drop_group.slot,
                    label,
                    condition_option_key: format!(
                        "hotspot:{hotspot_id}:{}:{}",
                        drop_group.slot, drop_group.group_key
                    ),
                    item_main_group_key: drop_group.group_key,
                    drop_rate: drop_group.drop_rate,
                    drop_rate_text: drop_rate_text.clone(),
                    drop_rate_source_kind: "database".to_string(),
                    drop_rate_tooltip: drop_rate_tooltip.clone(),
                    raw_drop_rate_text: drop_rate_text.clone(),
                    raw_drop_rate_tooltip: drop_rate_tooltip.clone(),
                    normalized_drop_rate_text: drop_rate_text,
                    normalized_drop_rate_tooltip: drop_rate_tooltip,
                    catch_methods: vec!["rod".to_string()],
                    condition_options,
                }
            })
            .collect()
    }

    fn expanded_options_for_main_group(
        &self,
        item_main_group_key: u32,
    ) -> Vec<ExpandedSourceLootOption> {
        let mut branches = self.expand_main_group(
            item_main_group_key,
            &mut Vec::new(),
            Vec::new(),
            None,
            Vec::new(),
            0,
        );
        for (index, branch) in branches.iter_mut().enumerate() {
            branch.branch_idx = u32::try_from(index).unwrap_or(u32::MAX);
        }
        branches
    }

    fn expand_main_group(
        &self,
        item_main_group_key: u32,
        visited_main_groups: &mut Vec<u32>,
        inherited_conditions: Vec<String>,
        inherited_select_rate: Option<u32>,
        inherited_lineage: Vec<SourceLootLineage>,
        depth: usize,
    ) -> Vec<ExpandedSourceLootOption> {
        if depth > 8 || visited_main_groups.contains(&item_main_group_key) {
            return Vec::new();
        }
        let Some(options) = self.options_by_main_group.get(&item_main_group_key) else {
            return Vec::new();
        };
        visited_main_groups.push(item_main_group_key);
        let mut expanded = Vec::new();
        for option in options {
            let mut conditions = inherited_conditions.clone();
            if let Some(condition) = option.condition_raw.as_ref() {
                if !conditions.contains(condition) {
                    conditions.push(condition.clone());
                }
            }
            let select_rate = combine_select_rates(inherited_select_rate, option.select_rate);
            let mut lineage = inherited_lineage.clone();
            lineage.push(SourceLootLineage {
                item_main_group_key,
                option_idx: option.option_idx,
                item_sub_group_key: option.item_sub_group_key,
            });
            if option.item_sub_group_key != item_main_group_key
                && self
                    .options_by_main_group
                    .contains_key(&option.item_sub_group_key)
                && !visited_main_groups.contains(&option.item_sub_group_key)
            {
                let nested = self.expand_main_group(
                    option.item_sub_group_key,
                    visited_main_groups,
                    conditions.clone(),
                    select_rate,
                    lineage.clone(),
                    depth + 1,
                );
                if !nested.is_empty() {
                    expanded.extend(nested);
                    continue;
                }
            }
            expanded.push(ExpandedSourceLootOption {
                branch_idx: 0,
                select_rate,
                condition_raw: combine_condition_raw(&conditions),
                item_sub_group_key: option.item_sub_group_key,
                items: option.items.clone(),
                lineage,
            });
        }
        visited_main_groups.pop();
        expanded
    }
}

fn source_loot_condition_option(
    option: &ExpandedSourceLootOption,
    slot_idx: u8,
    group_label: &str,
) -> HotspotLootConditionOption {
    let condition = source_condition_fields(option.condition_raw.as_deref());
    let species_rows = source_loot_species_rows(option, slot_idx, group_label);
    let drop_rate_text = option
        .select_rate
        .map(rate_text_from_micros)
        .unwrap_or_default();
    let drop_rate_tooltip = format!(
        "item_main_group_options {}",
        source_loot_lineage_tooltip(&option.lineage)
    );
    HotspotLootConditionOption {
        option_idx: option.branch_idx,
        condition_key: condition.key,
        condition_text: condition.text,
        condition_tooltip: condition.tooltip,
        item_sub_group_key: option.item_sub_group_key,
        select_rate: option.select_rate,
        drop_rate_text: drop_rate_text.clone(),
        drop_rate_source_kind: "database".to_string(),
        drop_rate_tooltip: drop_rate_tooltip.clone(),
        raw_drop_rate_text: drop_rate_text.clone(),
        raw_drop_rate_tooltip: drop_rate_tooltip.clone(),
        normalized_drop_rate_text: drop_rate_text,
        normalized_drop_rate_tooltip: drop_rate_tooltip,
        active: false,
        species_rows,
    }
}

fn mark_active_condition_option(options: &mut [HotspotLootConditionOption]) {
    if options.is_empty() {
        return;
    }
    let active_index = options
        .iter()
        .position(|option| contents_group_open_condition(&option.condition_key))
        .or_else(|| {
            options.iter().position(|option| {
                contents_group_open_condition(&option.condition_key)
                    && !option.species_rows.is_empty()
            })
        })
        .or_else(|| {
            options.iter().position(|option| {
                option.condition_key == "default" && !option.species_rows.is_empty()
            })
        })
        .or_else(|| {
            options
                .iter()
                .position(|option| option.condition_key == "default")
        })
        .or_else(|| {
            options
                .iter()
                .position(|option| !option.species_rows.is_empty())
        })
        .unwrap_or(0);
    for (index, option) in options.iter_mut().enumerate() {
        option.active = index == active_index;
    }
}

fn combine_select_rates(parent: Option<u32>, child: Option<u32>) -> Option<u32> {
    match (parent, child) {
        (Some(parent), Some(child)) => {
            Some(((u64::from(parent) * u64::from(child)) / 1_000_000).min(1_000_000) as u32)
        }
        (Some(parent), None) => Some(parent),
        (None, Some(child)) => Some(child),
        (None, None) => None,
    }
}

fn combine_condition_raw(conditions: &[String]) -> Option<String> {
    let joined = conditions
        .iter()
        .map(|condition| condition.trim())
        .filter(|condition| !condition.is_empty())
        .collect::<Vec<_>>()
        .join("");
    if joined.is_empty() {
        None
    } else {
        Some(joined)
    }
}

fn contents_group_open_condition(condition_key: &str) -> bool {
    condition_key
        .split(';')
        .map(str::trim)
        .any(|predicate| predicate.starts_with("isContentsGroupOpen(0,"))
}

fn source_loot_lineage_tooltip(lineage: &[SourceLootLineage]) -> String {
    if lineage.is_empty() {
        return "unknown lineage".to_string();
    }
    lineage
        .iter()
        .map(|step| {
            format!(
                "main group {} -> subgroup {}, option {}",
                step.item_main_group_key, step.item_sub_group_key, step.option_idx
            )
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

#[derive(Debug, Clone)]
struct AggregatedLootItem {
    item: SourceLootItem,
    select_rate: u64,
}

fn source_loot_species_rows(
    option: &ExpandedSourceLootOption,
    slot_idx: u8,
    group_label: &str,
) -> Vec<HotspotLootItem> {
    let mut items_by_id = HashMap::<u32, AggregatedLootItem>::new();
    for item in &option.items {
        let entry = items_by_id
            .entry(item.item_id)
            .or_insert_with(|| AggregatedLootItem {
                item: item.clone(),
                select_rate: 0,
            });
        entry.select_rate = entry
            .select_rate
            .saturating_add(u64::from(item.select_rate));
    }
    let total = items_by_id
        .values()
        .map(|item| item.select_rate)
        .sum::<u64>();
    if total == 0 {
        return Vec::new();
    }

    let mut rows = items_by_id
        .into_values()
        .map(|aggregate| {
            let normalized_rate = ((aggregate.select_rate as f64 / total as f64) * 1_000_000.0)
                .round()
                .clamp(0.0, 1_000_000.0) as u32;
            let rate_text = rate_text_from_micros(normalized_rate);
            let item = aggregate.item;
            let tooltip = item.source_tooltip.clone().unwrap_or_else(|| {
                format!(
                    "item_sub_group_item_variants subgroup {} select rate {} / {}",
                    option.item_sub_group_key, aggregate.select_rate, total
                )
            });
            HotspotLootItem {
                item_id: item.item_id,
                name: item.name.clone(),
                label: item.name,
                slot_idx,
                group_label: group_label.to_string(),
                select_rate: Some(normalized_rate),
                grade_type: item.grade_type,
                icon_item_id: item.icon_item_id,
                icon_image: item.icon_image,
                icon_grade_tone: hotspot_grade_tone(item.grade_type).to_string(),
                drop_rate_text: rate_text.clone(),
                drop_rate_source_kind: "database".to_string(),
                drop_rate_tooltip: tooltip.clone(),
                raw_drop_rate_text: rate_text.clone(),
                raw_drop_rate_tooltip: tooltip.clone(),
                normalized_drop_rate_text: rate_text,
                normalized_drop_rate_tooltip: tooltip,
                catch_methods: vec!["rod".to_string()],
                is_fish: item.is_fish,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .select_rate
            .unwrap_or_default()
            .cmp(&left.select_rate.unwrap_or_default())
            .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
            .then_with(|| left.item_id.cmp(&right.item_id))
    });
    rows
}

fn active_loot_items(groups: &[HotspotLootGroup]) -> Vec<HotspotLootItem> {
    groups
        .iter()
        .flat_map(|group| &group.condition_options)
        .filter(|option| option.active)
        .flat_map(|option| option.species_rows.iter().cloned())
        .collect()
}

#[derive(Debug, Clone)]
struct PrimaryFishIdentity {
    item_id: u32,
    name: String,
    icon_image: Option<String>,
}

fn primary_fish_identity(groups: &[HotspotLootGroup]) -> Option<PrimaryFishIdentity> {
    let active_rows = groups
        .iter()
        .flat_map(|group| &group.condition_options)
        .filter(|option| option.active)
        .flat_map(|option| option.species_rows.iter())
        .collect::<Vec<_>>();
    best_primary_fish(active_rows.as_slice()).or_else(|| {
        let all_rows = groups
            .iter()
            .flat_map(|group| &group.condition_options)
            .flat_map(|option| option.species_rows.iter())
            .collect::<Vec<_>>();
        best_primary_fish(all_rows.as_slice())
    })
}

fn best_primary_fish(rows: &[&HotspotLootItem]) -> Option<PrimaryFishIdentity> {
    rows.iter()
        .copied()
        .filter(|row| row.is_fish)
        .max_by(|left, right| {
            left.select_rate
                .unwrap_or_default()
                .cmp(&right.select_rate.unwrap_or_default())
                .then_with(|| right.label.to_lowercase().cmp(&left.label.to_lowercase()))
        })
        .or_else(|| {
            rows.iter().copied().max_by(|left, right| {
                left.select_rate
                    .unwrap_or_default()
                    .cmp(&right.select_rate.unwrap_or_default())
                    .then_with(|| right.label.to_lowercase().cmp(&left.label.to_lowercase()))
            })
        })
        .map(|row| PrimaryFishIdentity {
            item_id: row.item_id,
            name: row.label.clone(),
            icon_image: row.icon_image.clone(),
        })
}

fn hotspot_grade_tone(grade_type: Option<u32>) -> &'static str {
    match grade_type {
        Some(4) => "red",
        Some(3) => "yellow",
        Some(2) => "blue",
        Some(1) => "green",
        Some(0) => "white",
        _ => "unknown",
    }
}

fn rate_text_from_micros(rate: u32) -> String {
    let mut text = format!("{:.4}", f64::from(rate) / 10_000.0);
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    format!("{text}%")
}

fn value_u32(value: &Value) -> Option<u32> {
    match value {
        Value::Number(number) => number.as_u64().and_then(|value| u32::try_from(value).ok()),
        Value::String(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                None
            } else {
                trimmed.parse::<u32>().ok()
            }
        }
        _ => None,
    }
}

fn value_bool(value: &Value) -> bool {
    match value {
        Value::Bool(value) => *value,
        Value::Number(number) => number.as_u64().is_some_and(|value| value > 0),
        Value::String(raw) => matches!(raw.trim(), "1" | "true" | "TRUE" | "True"),
        _ => false,
    }
}

fn lifeskill_level_label(level: u32) -> String {
    let tiers = [
        ("Beginner", 10u32),
        ("Apprentice", 10),
        ("Skilled", 10),
        ("Professional", 10),
        ("Artisan", 10),
        ("Master", 30),
    ];
    let mut remaining = level.max(1);
    for (name, count) in tiers {
        if remaining <= count {
            return format!("{name} {remaining}");
        }
        remaining = remaining.saturating_sub(count);
    }
    format!("Guru {remaining}")
}

fn trimmed_optional_string(value: Option<&str>) -> Option<String> {
    let trimmed = value?.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn load_fishing_point_records(path: &Path) -> Result<Vec<FishingPointRecord>> {
    let table = read_source_table(path, POINT_HEADERS)?;
    let mut records = Vec::new();
    for row in &table.rows {
        let Some(point_key) = table_cell_u32(&table, row, "PointKey")? else {
            continue;
        };
        records.push(FishingPointRecord {
            point_key,
            point_size: required_table_cell_f64(&table, row, "PointSize")?,
            start_x: required_table_cell_f64(&table, row, "StartPositionX")?,
            start_y: required_table_cell_f64(&table, row, "StartPositionY")?,
            start_z: required_table_cell_f64(&table, row, "StartPositionZ")?,
            end_x: required_table_cell_f64(&table, row, "EndPositionX")?,
            end_z: required_table_cell_f64(&table, row, "EndPositionZ")?,
            fishing_group_key: required_table_cell_u32(&table, row, "FishingGroupKey")?,
            spawn_rate: nonzero_table_cell_u32(&table, row, "SpawnRate")?,
            spawn_character_key: nonzero_table_cell_u32(&table, row, "SpawnCharacterKey")?,
            spawn_action_index: nonzero_table_cell_u32(&table, row, "SpawnActionIndex")?,
            contents_group_key: nonzero_table_cell_u32(&table, row, "ContentsGroupKey")?,
        });
    }
    Ok(records)
}

fn load_fishing_group_records(path: &Path) -> Result<Vec<FishingGroupRecord>> {
    let table = read_source_table(path, FISHING_HEADERS)?;
    let mut records = Vec::new();
    for row in &table.rows {
        let Some(fishing_group_key) = table_cell_u32(&table, row, "FishingGroupKey")? else {
            continue;
        };
        let mut drop_groups = Vec::new();
        for slot in 1..=5u8 {
            let drop_id_header = format!("DropID{slot}");
            let Some(group_key) = table_cell_u32(&table, row, drop_id_header.as_str())? else {
                continue;
            };
            if group_key == 0 {
                continue;
            }
            let drop_rate_header = format!("DropRate{slot}");
            drop_groups.push(HotspotDropGroup {
                slot,
                drop_rate: nonzero_table_cell_u32(&table, row, drop_rate_header.as_str())?,
                group_key,
            });
        }
        let source_stats = HotspotSourceStats {
            min_wait_time: table_cell_u32(&table, row, "MinWaitTime")?.unwrap_or_default(),
            max_wait_time: table_cell_u32(&table, row, "MaxWaitTime")?.unwrap_or_default(),
            point_remain_time: table_cell_u32(&table, row, "PointRemainTime")?.unwrap_or_default(),
            min_fish_count: table_cell_u32(&table, row, "MinFishCount")?.unwrap_or_default(),
            max_fish_count: table_cell_u32(&table, row, "MaxFishCount")?.unwrap_or_default(),
            available_fishing_level: table_cell_u32(&table, row, "AvailableFishingLevel")?
                .unwrap_or_default(),
            observe_fishing_level: table_cell_u32(&table, row, "ObserveFishingLevel")?
                .unwrap_or_default(),
        };
        records.push(FishingGroupRecord {
            fishing_group_key,
            drop_groups,
            min_wait_time: nonzero_table_cell_u32(&table, row, "MinWaitTime")?,
            max_wait_time: nonzero_table_cell_u32(&table, row, "MaxWaitTime")?,
            point_remain_time: nonzero_table_cell_u32(&table, row, "PointRemainTime")?,
            min_fish_count: nonzero_table_cell_u32(&table, row, "MinFishCount")?,
            max_fish_count: nonzero_table_cell_u32(&table, row, "MaxFishCount")?,
            available_fishing_level: nonzero_table_cell_u32(&table, row, "AvailableFishingLevel")?,
            observe_fishing_level: nonzero_table_cell_u32(&table, row, "ObserveFishingLevel")?,
            contents_group_key: nonzero_table_cell_u32(&table, row, "ContentsGroupKey")?,
            source_stats,
        });
    }
    Ok(records)
}

#[derive(Debug, Clone, Copy)]
struct HotspotBounds {
    min_x: f64,
    min_z: f64,
    max_x: f64,
    max_z: f64,
    expanded_degenerate_axis: bool,
}

fn hotspot_bounds(
    start_x: f64,
    start_z: f64,
    end_x: f64,
    end_z: f64,
    point_size: f64,
) -> HotspotBounds {
    let mut min_x = start_x.min(end_x);
    let mut max_x = start_x.max(end_x);
    let mut min_z = start_z.min(end_z);
    let mut max_z = start_z.max(end_z);
    let half_point_size = point_size.abs() * 0.5;
    let mut expanded_degenerate_axis = false;
    if (max_x - min_x).abs() <= f64::EPSILON {
        min_x -= half_point_size;
        max_x += half_point_size;
        expanded_degenerate_axis = true;
    }
    if (max_z - min_z).abs() <= f64::EPSILON {
        min_z -= half_point_size;
        max_z += half_point_size;
        expanded_degenerate_axis = true;
    }
    HotspotBounds {
        min_x,
        min_z,
        max_x,
        max_z,
        expanded_degenerate_axis,
    }
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
        .collect::<BTreeMap<_, _>>();
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

fn required_header_index(table: &SourceTable, header: &str) -> Result<usize> {
    table
        .headers
        .get(header)
        .copied()
        .with_context(|| format!("missing header '{header}' in sheet {}", table.sheet_name))
}

fn table_cell_u32(table: &SourceTable, row: &[Data], header: &str) -> Result<Option<u32>> {
    let idx = required_header_index(table, header)?;
    cell_to_u32(row.get(idx))
}

fn required_table_cell_u32(table: &SourceTable, row: &[Data], header: &str) -> Result<u32> {
    table_cell_u32(table, row, header)?
        .with_context(|| format!("missing required u32 value for {header}"))
}

fn nonzero_table_cell_u32(table: &SourceTable, row: &[Data], header: &str) -> Result<Option<u32>> {
    Ok(table_cell_u32(table, row, header)?.filter(|value| *value != 0))
}

fn table_cell_f64(table: &SourceTable, row: &[Data], header: &str) -> Result<Option<f64>> {
    let idx = required_header_index(table, header)?;
    cell_to_f64(row.get(idx))
}

fn required_table_cell_f64(table: &SourceTable, row: &[Data], header: &str) -> Result<f64> {
    table_cell_f64(table, row, header)?
        .with_context(|| format!("missing required f64 value for {header}"))
}

fn cell_to_u32(cell: Option<&Data>) -> Result<Option<u32>> {
    let Some(value) = cell_to_string(cell)? else {
        return Ok(None);
    };
    let normalized = value.replace('_', "");
    if let Ok(parsed) = normalized.parse::<u32>() {
        return Ok(Some(parsed));
    }
    let parsed = normalized
        .parse::<f64>()
        .with_context(|| format!("parse u32 cell value: {value}"))?;
    if parsed < 0.0 || parsed.fract() != 0.0 || parsed > f64::from(u32::MAX) {
        bail!("parse u32 cell value: {value}");
    }
    Ok(Some(parsed as u32))
}

fn cell_to_f64(cell: Option<&Data>) -> Result<Option<f64>> {
    let Some(value) = cell_to_string(cell)? else {
        return Ok(None);
    };
    value
        .replace('_', "")
        .parse::<f64>()
        .with_context(|| format!("parse f64 cell value: {value}"))
        .map(Some)
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

fn header_cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(value) => value.trim().to_string(),
        _ => cell.to_string().trim().to_string(),
    }
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

fn source_descriptor(id: &'static str, path: &Path, role: &'static str) -> HotspotSourceDescriptor {
    HotspotSourceDescriptor {
        id,
        file: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        role,
    }
}

fn source_descriptor_literal(
    id: &'static str,
    file: &'static str,
    role: &'static str,
) -> HotspotSourceDescriptor {
    HotspotSourceDescriptor {
        id,
        file: file.to_string(),
        role,
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

#[cfg(test)]
mod tests {
    use super::{
        hotspot_bounds, item_id_from_icon_image, load_bdolytics_metadata_lookup,
        source_condition_fields,
    };
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn hotspot_bounds_expand_degenerate_axis_from_point_size() {
        let bounds = hotspot_bounds(-4916.0, 101125.0, -2102.0, 101125.0, 2000.0);
        assert_eq!(bounds.min_x, -4916.0);
        assert_eq!(bounds.max_x, -2102.0);
        assert_eq!(bounds.min_z, 100125.0);
        assert_eq!(bounds.max_z, 102125.0);
        assert!(bounds.expanded_degenerate_axis);
    }

    #[test]
    fn hotspot_bounds_keep_non_degenerate_rectangle() {
        let bounds = hotspot_bounds(-73592.0, 253493.0, -33080.0, 198722.0, 2000.0);
        assert_eq!(bounds.min_x, -73592.0);
        assert_eq!(bounds.max_x, -33080.0);
        assert_eq!(bounds.min_z, 198722.0);
        assert_eq!(bounds.max_z, 253493.0);
        assert!(!bounds.expanded_degenerate_axis);
    }

    #[test]
    fn icon_image_suffix_resolves_fish_item_id() {
        assert_eq!(
            item_id_from_icon_image("New_Icon/03_ETC/07_ProductMaterial/00008452"),
            Some(8452)
        );
        assert_eq!(
            item_id_from_icon_image("New_Icon/03_ETC/10_Free_TradeItem/00800108.dds"),
            Some(800108)
        );
        assert_eq!(item_id_from_icon_image("New_Icon/no-item"), None);
    }

    #[test]
    fn source_life_level_condition_is_labeled() {
        let condition = source_condition_fields(Some("getLifeLevel(1)>80;"));

        assert_eq!(condition.key, "getLifeLevel(1)>80;");
        assert_eq!(condition.text, "Fishing Level Guru 1+");
        assert_eq!(condition.tooltip, "getLifeLevel(1)>80;");

        let default_condition = source_condition_fields(None);
        assert_eq!(default_condition.key, "default");
        assert_eq!(default_condition.text, "Default");
    }

    #[test]
    fn source_contents_group_condition_is_labeled() {
        let condition = source_condition_fields(Some("!isContentsGroupOpen(0,689);"));

        assert_eq!(condition.key, "!isContentsGroupOpen(0,689);");
        assert_eq!(condition.text, "Contents Group 689 Closed");
        assert_eq!(condition.tooltip, "!isContentsGroupOpen(0,689);");
    }

    #[test]
    fn bdolytics_metadata_import_keeps_external_provenance() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("fishystuff-bdolytics-hotspots-{unique}.json"));
        fs::write(
            &path,
            r#"{
              "data": [
                {
                  "id": 413,
                  "min_wait_time": 87950,
                  "max_wait_time": 117950,
                  "point_remain_time": 600000,
                  "min_fish_count": 2,
                  "max_fish_count": 4,
                  "available_fishing_level": 1,
                  "visible_fishing_level": 1
                },
                { "id": 0, "min_wait_time": 1 }
              ]
            }"#,
        )
        .unwrap();

        let rows = load_bdolytics_metadata_lookup(Some(path.as_path())).unwrap();
        let row = rows.get(&413).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(row.source_id, "bdolytics_community_hotspot_metadata");
        assert_eq!(row.source_hotspot_id, 413);
        assert_eq!(row.min_wait_time, Some(87950));
        assert_eq!(row.max_wait_time, Some(117950));
        assert_eq!(row.point_remain_time, Some(600000));
        assert_eq!(row.min_fish_count, Some(2));
        assert_eq!(row.max_fish_count, Some(4));
        assert_eq!(row.available_fishing_level, Some(1));
        assert_eq!(row.observe_fishing_level, Some(1));

        let _ = fs::remove_file(path);
    }
}
