use std::fs::File;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use fishystuff_core::gamecommondata::{
    load_original_region_layer_context, OriginalRegionLayerContext, RegionGroupWaypointInfo,
    RegionOriginInfo,
};
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

#[derive(Debug, Clone, Copy)]
pub struct RegionNodesBuildSummary {
    pub feature_count: usize,
    pub named_feature_count: usize,
}

pub fn build_region_groups_geojson(
    region_groups_geojson_path: &Path,
    loc_path: &Path,
    regioninfo_bss_path: &Path,
    regiongroupinfo_bss_path: &Path,
    waypoint_xml_paths: &[PathBuf],
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
    let context = load_context(
        loc_path,
        regioninfo_bss_path,
        regiongroupinfo_bss_path,
        waypoint_xml_paths,
    )?;

    let input_feature_count = region_groups.features.len();
    let mut resource_feature_count = 0usize;
    let mut features = Vec::with_capacity(input_feature_count);

    for mut feature in region_groups.features {
        let region_group_id = json_u32(feature.properties.get("rg"));
        let resource_info =
            region_group_id.and_then(|group_id| context.resolve_resource_waypoint(group_id));

        apply_resource_waypoint_info(&mut feature.properties, resource_info);
        if json_u32(feature.properties.get("rgwp")).is_some()
            || json_f64(feature.properties.get("rgx")).is_some()
            || json_f64(feature.properties.get("rgz")).is_some()
        {
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
    loc_path: &Path,
    regioninfo_bss_path: &Path,
    regiongroupinfo_bss_path: &Path,
    waypoint_xml_paths: &[PathBuf],
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
    let context = load_context(
        loc_path,
        regioninfo_bss_path,
        regiongroupinfo_bss_path,
        waypoint_xml_paths,
    )?;

    let input_feature_count = regions.features.len();
    let mut named_feature_count = 0usize;
    let mut resource_feature_count = 0usize;
    let mut features = Vec::with_capacity(input_feature_count);
    for mut feature in regions.features {
        let region_id = json_u32(feature.properties.get("r"));
        let region_group_id = json_u32(feature.properties.get("rg"))
            .or_else(|| region_id.and_then(|id| context.region_group_for_region(id)));
        let origin_info = region_id.and_then(|id| context.resolve_region_origin_info(id));
        let resource_info =
            region_group_id.and_then(|group_id| context.resolve_resource_waypoint(group_id));

        if let Some(region_group_id) = region_group_id {
            feature
                .properties
                .insert("rg".to_string(), Value::from(region_group_id));
        }
        apply_resource_waypoint_info(&mut feature.properties, resource_info);
        if json_u32(feature.properties.get("rgwp")).is_some()
            || json_f64(feature.properties.get("rgx")).is_some()
            || json_f64(feature.properties.get("rgz")).is_some()
        {
            resource_feature_count = resource_feature_count.saturating_add(1);
        }
        apply_origin_info(&mut feature.properties, origin_info.as_ref());
        if json_string(feature.properties.get("on")).is_some() {
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

pub fn build_region_nodes_geojson(
    loc_path: &Path,
    regioninfo_bss_path: &Path,
    regiongroupinfo_bss_path: &Path,
    waypoint_xml_paths: &[PathBuf],
    out_path: &Path,
) -> Result<RegionNodesBuildSummary> {
    let context = load_context(
        loc_path,
        regioninfo_bss_path,
        regiongroupinfo_bss_path,
        waypoint_xml_paths,
    )?;

    let mut named_feature_count = 0usize;
    let mut features = Vec::new();
    for region_id in context.region_ids() {
        let Some(info) = context.resolve_region_waypoint_info(region_id) else {
            continue;
        };
        let (Some(world_x), Some(world_z)) = (info.world_x, info.world_z) else {
            continue;
        };
        if info.region_name.is_some() {
            named_feature_count = named_feature_count.saturating_add(1);
        }
        features.push(build_region_node_feature(
            region_id, &info, world_x, world_z,
        ));
    }

    let feature_count = features.len();
    write_output_geojson(out_path, features)?;

    Ok(RegionNodesBuildSummary {
        feature_count,
        named_feature_count,
    })
}

fn default_feature_type() -> String {
    "Feature".to_string()
}

fn build_region_node_feature(
    region_id: u32,
    info: &RegionOriginInfo,
    world_x: f64,
    world_z: f64,
) -> Feature {
    let mut properties = Map::new();
    let region_name = info
        .region_name
        .clone()
        .or_else(|| info.waypoint_name.clone());
    let label = region_name
        .as_ref()
        .map(|name| format!("{name} (R{region_id})"))
        .unwrap_or_else(|| format!("R{region_id}"));
    properties.insert("kind".to_string(), Value::String("region_node".to_string()));
    properties.insert("r".to_string(), Value::from(region_id));
    properties.insert("label".to_string(), Value::String(label));
    if let Some(region_name) = region_name {
        properties.insert("name".to_string(), Value::String(region_name));
    }
    if let Some(waypoint_id) = info.waypoint_id {
        properties.insert("wp".to_string(), Value::from(waypoint_id));
    }
    Feature {
        feature_type: default_feature_type(),
        properties,
        geometry: serde_json::json!({
            "type": "Point",
            "coordinates": [world_x, world_z],
        }),
    }
}

fn load_context(
    loc_path: &Path,
    regioninfo_bss_path: &Path,
    regiongroupinfo_bss_path: &Path,
    waypoint_xml_paths: &[PathBuf],
) -> Result<OriginalRegionLayerContext> {
    load_original_region_layer_context(
        regioninfo_bss_path,
        regiongroupinfo_bss_path,
        waypoint_xml_paths,
        loc_path,
    )
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
    if let Some(origin_name) = origin_info.region_name.as_ref() {
        properties.insert("on".to_string(), Value::String(origin_name.clone()));
    }
    origin_info.region_name.is_some()
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

fn json_u32(value: Option<&Value>) -> Option<u32> {
    match value {
        Some(Value::Number(number)) => number.as_u64().and_then(|raw| u32::try_from(raw).ok()),
        Some(Value::String(text)) => text.trim().parse::<u32>().ok(),
        _ => None,
    }
}

fn json_f64(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Number(number)) => number.as_f64(),
        Some(Value::String(text)) => text.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn json_string(value: Option<&Value>) -> Option<String> {
    match value {
        Some(Value::String(text)) => {
            let trimmed = text.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        }
        _ => None,
    }
}
