use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::path::Path;

use anyhow::{bail, Context, Result};
use fishystuff_core::loc::load_loc_namespaces_as_string_maps;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Deserialize)]
struct FeatureCollection {
    #[serde(default)]
    features: Vec<Feature>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Feature {
    #[serde(rename = "type", default = "default_feature_type")]
    feature_type: String,
    #[serde(default)]
    properties: Map<String, Value>,
    geometry: Value,
}

#[derive(Debug, Serialize)]
struct OutputFeatureCollection {
    #[serde(rename = "type")]
    collection_type: &'static str,
    features: Vec<Feature>,
}

#[derive(Debug, Deserialize)]
struct RegionInfoRow {
    #[serde(default)]
    regiongroup: u32,
    #[serde(default)]
    tradeoriginregion: u32,
}

#[derive(Debug, Clone, Deserialize)]
struct DeckRegionOriginRow {
    #[serde(default)]
    r: u32,
    #[serde(default)]
    o: u32,
    #[serde(default)]
    owp: u32,
    x: Option<f64>,
    z: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct DeckRegionGroupGraphRow {
    #[serde(default)]
    k: u32,
    #[serde(default)]
    wp: u32,
    graphx: Option<f64>,
    graphz: Option<f64>,
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

#[derive(Debug, Clone, Copy)]
struct RegionGroupWaypointInfo {
    waypoint_id: Option<u32>,
    world_x: Option<f64>,
    world_z: Option<f64>,
}

#[derive(Debug, Clone)]
struct RegionOriginInfo {
    region_id: Option<u32>,
    waypoint_id: Option<u32>,
    world_x: Option<f64>,
    world_z: Option<f64>,
    name: Option<String>,
}

#[derive(Debug)]
struct RegionLayerContext {
    regioninfo: HashMap<String, RegionInfoRow>,
    loc: LocalizationFile,
    deck_by_region: HashMap<u32, DeckRegionOriginRow>,
    deck_by_group: HashMap<u32, DeckRegionGroupGraphRow>,
}

#[derive(Debug, Clone, Copy)]
pub struct DetailedRegionsBuildSummary {
    pub feature_count: usize,
    pub named_feature_count: usize,
    pub resource_feature_count: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct RegionGroupsBuildSummary {
    pub feature_count: usize,
    pub resource_feature_count: usize,
}

pub fn build_region_groups_geojson(
    region_groups_geojson_path: &Path,
    regioninfo_path: &Path,
    loc_path: &Path,
    deck_r_origins_path: &Path,
    deck_rg_graphs_path: &Path,
    out_path: &Path,
) -> Result<RegionGroupsBuildSummary> {
    let region_groups_file = File::open(region_groups_geojson_path).with_context(|| {
        format!(
            "open region-groups geojson: {}",
            region_groups_geojson_path.display()
        )
    })?;
    let region_groups: FeatureCollection =
        serde_json::from_reader(region_groups_file).context("parse region-groups geojson")?;
    let context = load_region_layer_context(
        regioninfo_path,
        loc_path,
        deck_r_origins_path,
        deck_rg_graphs_path,
    )?;

    let input_feature_count = region_groups.features.len();
    let mut resource_feature_count = 0usize;
    let mut features = Vec::with_capacity(input_feature_count);

    for mut feature in region_groups.features {
        let region_group_id = json_u32(feature.properties.get("rg"));
        let resource_info =
            region_group_id.and_then(|group_id| resolve_resource_waypoint(&context, group_id));

        if apply_resource_waypoint_info(&mut feature.properties, resource_info) {
            resource_feature_count = resource_feature_count.saturating_add(1);
        }

        features.push(feature);
    }

    write_output_geojson(out_path, features)?;

    Ok(RegionGroupsBuildSummary {
        feature_count: input_feature_count,
        resource_feature_count,
    })
}

pub fn build_detailed_regions_geojson(
    regions_geojson_path: &Path,
    regioninfo_path: &Path,
    loc_path: &Path,
    deck_r_origins_path: &Path,
    deck_rg_graphs_path: &Path,
    out_path: &Path,
) -> Result<DetailedRegionsBuildSummary> {
    let regions_file = File::open(regions_geojson_path).with_context(|| {
        format!(
            "open detailed-regions geojson: {}",
            regions_geojson_path.display()
        )
    })?;
    let regions: FeatureCollection =
        serde_json::from_reader(regions_file).context("parse detailed-regions geojson")?;
    let context = load_region_layer_context(
        regioninfo_path,
        loc_path,
        deck_r_origins_path,
        deck_rg_graphs_path,
    )?;

    let input_feature_count = regions.features.len();
    let mut named_feature_count = 0usize;
    let mut resource_feature_count = 0usize;
    let mut features = Vec::with_capacity(input_feature_count);
    for mut feature in regions.features {
        let region_id = json_u32(feature.properties.get("r"));
        let region_group_id = json_u32(feature.properties.get("rg")).or_else(|| {
            region_id.and_then(|id| {
                context
                    .regioninfo
                    .get(&id.to_string())
                    .and_then(|row| non_zero_u32(row.regiongroup))
            })
        });
        let origin_info = region_id.and_then(|id| resolve_region_origin_info(&context, id));
        let resource_info =
            region_group_id.and_then(|group_id| resolve_resource_waypoint(&context, group_id));

        if let Some(region_group_id) = region_group_id {
            feature
                .properties
                .insert("rg".to_string(), Value::from(region_group_id));
        }
        if apply_resource_waypoint_info(&mut feature.properties, resource_info) {
            resource_feature_count = resource_feature_count.saturating_add(1);
        }
        if apply_origin_info(&mut feature.properties, origin_info.as_ref()) {
            named_feature_count = named_feature_count.saturating_add(1);
        }

        features.push(feature);
    }

    write_output_geojson(out_path, features)?;

    Ok(DetailedRegionsBuildSummary {
        feature_count: input_feature_count,
        named_feature_count,
        resource_feature_count,
    })
}

fn default_feature_type() -> String {
    "Feature".to_string()
}

fn load_region_layer_context(
    regioninfo_path: &Path,
    loc_path: &Path,
    deck_r_origins_path: &Path,
    deck_rg_graphs_path: &Path,
) -> Result<RegionLayerContext> {
    let regioninfo_file = File::open(regioninfo_path)
        .with_context(|| format!("open regioninfo json: {}", regioninfo_path.display()))?;
    let regioninfo: HashMap<String, RegionInfoRow> =
        serde_json::from_reader(regioninfo_file).context("parse regioninfo json")?;

    let loc = load_localization(loc_path)?;

    let deck_file = File::open(deck_r_origins_path).with_context(|| {
        format!(
            "open deck_r_origins json: {}",
            deck_r_origins_path.display()
        )
    })?;
    let deck_rows: Vec<DeckRegionOriginRow> =
        serde_json::from_reader(deck_file).context("parse deck_r_origins json")?;
    let deck_by_region: HashMap<u32, DeckRegionOriginRow> =
        deck_rows.into_iter().map(|row| (row.r, row)).collect();

    let deck_rg_file = File::open(deck_rg_graphs_path).with_context(|| {
        format!(
            "open deck_rg_graphs json: {}",
            deck_rg_graphs_path.display()
        )
    })?;
    let deck_rg_rows: Vec<DeckRegionGroupGraphRow> =
        serde_json::from_reader(deck_rg_file).context("parse deck_rg_graphs json")?;
    let deck_by_group: HashMap<u32, DeckRegionGroupGraphRow> =
        deck_rg_rows.into_iter().map(|row| (row.k, row)).collect();

    Ok(RegionLayerContext {
        regioninfo,
        loc,
        deck_by_region,
        deck_by_group,
    })
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

    let maps = load_loc_namespaces_as_string_maps(path, &[17, 29], 10_000)
        .with_context(|| format!("load localization namespaces from {}", path.display()))?;
    Ok(LocalizationFile {
        en: LocalizationTable {
            node: maps.get(&29).cloned().unwrap_or_default(),
            town: maps.get(&17).cloned().unwrap_or_default(),
        },
    })
}

fn resolve_region_origin_info(
    context: &RegionLayerContext,
    region_id: u32,
) -> Option<RegionOriginInfo> {
    let info = context.regioninfo.get(&region_id.to_string());
    let deck = context.deck_by_region.get(&region_id);
    let origin_region_id = deck
        .and_then(|row| non_zero_u32(row.o))
        .or_else(|| info.and_then(|row| non_zero_u32(row.tradeoriginregion)));
    let origin_waypoint_id = deck.and_then(|row| non_zero_u32(row.owp));
    let world_x = deck.and_then(|row| row.x);
    let world_z = deck.and_then(|row| row.z);
    let name = resolve_origin_name(&context.loc.en, origin_waypoint_id, origin_region_id);
    let resolved = RegionOriginInfo {
        region_id: origin_region_id,
        waypoint_id: origin_waypoint_id,
        world_x,
        world_z,
        name,
    };
    resolved.has_value().then_some(resolved)
}

fn resolve_resource_waypoint(
    context: &RegionLayerContext,
    region_group_id: u32,
) -> Option<RegionGroupWaypointInfo> {
    let row = context.deck_by_group.get(&region_group_id)?;
    let resolved = RegionGroupWaypointInfo {
        waypoint_id: non_zero_u32(row.wp),
        world_x: row.graphx,
        world_z: row.graphz,
    };
    resolved.has_value().then_some(resolved)
}

fn apply_origin_info(
    properties: &mut Map<String, Value>,
    origin_info: Option<&RegionOriginInfo>,
) -> bool {
    let Some(origin_info) = origin_info else {
        return false;
    };
    if let Some(origin_region_id) = origin_info.region_id {
        properties.insert("o".to_string(), Value::from(origin_region_id));
    }
    if let Some(origin_waypoint_id) = origin_info.waypoint_id {
        properties.insert("owp".to_string(), Value::from(origin_waypoint_id));
    }
    if let Some(origin_world_x) = origin_info.world_x {
        properties.insert("ox".to_string(), Value::from(origin_world_x));
    }
    if let Some(origin_world_z) = origin_info.world_z {
        properties.insert("oz".to_string(), Value::from(origin_world_z));
    }
    if let Some(origin_name) = origin_info.name.as_ref() {
        properties.insert("on".to_string(), Value::String(origin_name.clone()));
    }
    origin_info.name.is_some()
}

fn apply_resource_waypoint_info(
    properties: &mut Map<String, Value>,
    resource_info: Option<RegionGroupWaypointInfo>,
) -> bool {
    let Some(resource_info) = resource_info else {
        return false;
    };
    if let Some(resource_waypoint_id) = resource_info.waypoint_id {
        properties.insert("rgwp".to_string(), Value::from(resource_waypoint_id));
    }
    if let Some(resource_world_x) = resource_info.world_x {
        properties.insert("rgx".to_string(), Value::from(resource_world_x));
    }
    if let Some(resource_world_z) = resource_info.world_z {
        properties.insert("rgz".to_string(), Value::from(resource_world_z));
    }
    resource_info.has_value()
}

fn write_output_geojson(out_path: &Path, features: Vec<Feature>) -> Result<()> {
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create output directory: {}", parent.display()))?;
    }
    let output_file = File::create(out_path)
        .with_context(|| format!("create output geojson: {}", out_path.display()))?;
    serde_json::to_writer_pretty(
        output_file,
        &OutputFeatureCollection {
            collection_type: "FeatureCollection",
            features,
        },
    )
    .with_context(|| format!("write output geojson: {}", out_path.display()))?;
    Ok(())
}

impl RegionGroupWaypointInfo {
    fn has_value(self) -> bool {
        self.waypoint_id.is_some() || self.world_x.is_some() || self.world_z.is_some()
    }
}

impl RegionOriginInfo {
    fn has_value(&self) -> bool {
        self.region_id.is_some()
            || self.waypoint_id.is_some()
            || self.world_x.is_some()
            || self.world_z.is_some()
            || self.name.is_some()
    }
}

fn non_zero_u32(value: u32) -> Option<u32> {
    (value != 0).then_some(value)
}

fn json_u32(value: Option<&Value>) -> Option<u32> {
    match value {
        Some(Value::Number(number)) => number.as_u64().and_then(|raw| u32::try_from(raw).ok()),
        Some(Value::String(text)) => text.trim().parse::<u32>().ok(),
        _ => None,
    }
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
