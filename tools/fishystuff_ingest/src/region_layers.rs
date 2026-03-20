use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result};
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

#[derive(Debug, Deserialize)]
struct DeckRegionOriginRow {
    #[serde(default)]
    r: u32,
    #[serde(default)]
    o: u32,
    #[serde(default)]
    owp: u32,
}

#[derive(Debug, Default, Deserialize)]
struct LocalizationFile {
    #[serde(default)]
    en: LocalizationTable,
}

#[derive(Debug, Default, Deserialize)]
struct LocalizationTable {
    #[serde(default)]
    node: HashMap<String, String>,
    #[serde(default)]
    town: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy)]
struct RegionLayerInfo {
    region_group_id: Option<u32>,
    origin_region_id: Option<u32>,
    origin_waypoint_id: Option<u32>,
}

#[derive(Debug, Clone, Copy)]
pub struct DetailedRegionsBuildSummary {
    pub feature_count: usize,
    pub named_feature_count: usize,
}

pub fn build_detailed_regions_geojson(
    regions_geojson_path: &Path,
    regioninfo_path: &Path,
    loc_path: &Path,
    deck_r_origins_path: &Path,
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

    let regioninfo_file = File::open(regioninfo_path)
        .with_context(|| format!("open regioninfo json: {}", regioninfo_path.display()))?;
    let regioninfo: HashMap<String, RegionInfoRow> =
        serde_json::from_reader(regioninfo_file).context("parse regioninfo json")?;

    let loc_file =
        File::open(loc_path).with_context(|| format!("open loc json: {}", loc_path.display()))?;
    let loc: LocalizationFile =
        serde_json::from_reader(loc_file).context("parse localization json")?;

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

    let input_feature_count = regions.features.len();
    let mut named_feature_count = 0usize;
    let mut features = Vec::with_capacity(input_feature_count);
    for mut feature in regions.features {
        let region_id = json_u32(feature.properties.get("r"));
        let info = region_id.and_then(|id| {
            let key = id.to_string();
            regioninfo.get(&key).map(|row| RegionLayerInfo {
                region_group_id: non_zero_u32(row.regiongroup),
                origin_region_id: non_zero_u32(row.tradeoriginregion),
                origin_waypoint_id: None,
            })
        });
        let deck = region_id.and_then(|id| deck_by_region.get(&id));
        let region_layer_info = RegionLayerInfo {
            region_group_id: json_u32(feature.properties.get("rg"))
                .or(info.and_then(|row| row.region_group_id)),
            origin_region_id: json_u32(feature.properties.get("o"))
                .or(deck.and_then(|row| non_zero_u32(row.o)))
                .or(info.and_then(|row| row.origin_region_id)),
            origin_waypoint_id: json_u32(feature.properties.get("owp"))
                .or(deck.and_then(|row| non_zero_u32(row.owp))),
        };

        if let Some(region_group_id) = region_layer_info.region_group_id {
            feature
                .properties
                .insert("rg".to_string(), Value::from(region_group_id));
        }
        if let Some(origin_region_id) = region_layer_info.origin_region_id {
            feature
                .properties
                .insert("o".to_string(), Value::from(origin_region_id));
        }
        if let Some(origin_waypoint_id) = region_layer_info.origin_waypoint_id {
            feature
                .properties
                .insert("owp".to_string(), Value::from(origin_waypoint_id));
        }
        if let Some(origin_name) = resolve_origin_name(
            &loc.en,
            region_layer_info.origin_waypoint_id,
            region_layer_info.origin_region_id,
        ) {
            named_feature_count = named_feature_count.saturating_add(1);
            feature
                .properties
                .insert("on".to_string(), Value::String(origin_name));
        }

        features.push(feature);
    }

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
    .with_context(|| format!("write detailed-regions geojson: {}", out_path.display()))?;

    Ok(DetailedRegionsBuildSummary {
        feature_count: input_feature_count,
        named_feature_count,
    })
}

fn default_feature_type() -> String {
    "Feature".to_string()
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

fn localized_name(map: &HashMap<String, String>, key: u32) -> Option<String> {
    let value = map.get(&key.to_string())?;
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}
