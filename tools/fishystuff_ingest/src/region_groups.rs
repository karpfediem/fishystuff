use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::File;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::mysql_store::{RegionGroupMetaRow, RegionGroupRegionRow};

#[derive(Debug, Deserialize)]
struct RegionInfoRow {
    key: u32,
    #[serde(default)]
    is_accessible: i32,
    #[serde(default)]
    tradeoriginregion: u32,
    #[serde(default)]
    regiongroup: u32,
    #[serde(default)]
    waypoint: u32,
}

#[derive(Debug, Deserialize)]
struct DeckGraphRow {
    #[serde(default)]
    k: u32,
    graphx: f64,
    graphz: f64,
}

#[derive(Debug, Deserialize)]
struct GeoCollection {
    #[serde(default)]
    features: Vec<GeoFeature>,
}

#[derive(Debug, Deserialize)]
struct GeoFeature {
    properties: GeoProperties,
    geometry: GeoGeometry,
}

#[derive(Debug, Deserialize)]
struct GeoProperties {
    #[serde(default)]
    rg: u32,
    #[serde(default)]
    c: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "coordinates")]
enum GeoGeometry {
    Polygon(Vec<Vec<[f64; 2]>>),
    MultiPolygon(Vec<Vec<Vec<[f64; 2]>>>),
}

#[derive(Debug, Default)]
struct GroupAccumulator {
    color_rgb_u32: Option<u32>,
    feature_count: u32,
    region_ids: BTreeSet<u32>,
    accessible_region_count: u32,
    bbox_min_x: Option<f64>,
    bbox_min_y: Option<f64>,
    bbox_max_x: Option<f64>,
    bbox_max_y: Option<f64>,
    graph_world_x: Option<f64>,
    graph_world_z: Option<f64>,
}

pub fn load_region_group_inputs(
    geojson_path: &Path,
    regioninfo_path: &Path,
    deck_graphs_path: Option<&Path>,
    source: &str,
) -> Result<(Vec<RegionGroupMetaRow>, Vec<RegionGroupRegionRow>)> {
    let geo_file = File::open(geojson_path)
        .with_context(|| format!("open region-group geojson: {}", geojson_path.display()))?;
    let geo: GeoCollection =
        serde_json::from_reader(geo_file).context("parse region-group geojson")?;

    let region_file = File::open(regioninfo_path)
        .with_context(|| format!("open regioninfo json: {}", regioninfo_path.display()))?;
    let regioninfo: HashMap<String, RegionInfoRow> =
        serde_json::from_reader(region_file).context("parse regioninfo json")?;

    let mut accum: BTreeMap<u32, GroupAccumulator> = BTreeMap::new();
    let mut region_rows: Vec<RegionGroupRegionRow> = Vec::new();

    for row in regioninfo.into_values() {
        if row.regiongroup == 0 || row.key == 0 {
            continue;
        }
        let group = accum.entry(row.regiongroup).or_default();
        group.region_ids.insert(row.key);
        if row.is_accessible != 0 {
            group.accessible_region_count = group.accessible_region_count.saturating_add(1);
        }
        region_rows.push(RegionGroupRegionRow {
            region_group_id: row.regiongroup,
            region_id: row.key,
            trade_origin_region: if row.tradeoriginregion == 0 {
                None
            } else {
                Some(row.tradeoriginregion)
            },
            is_accessible: row.is_accessible != 0,
            waypoint: if row.waypoint == 0 {
                None
            } else {
                Some(row.waypoint)
            },
        });
    }

    for feature in geo.features {
        let group_id = feature.properties.rg;
        if group_id == 0 {
            continue;
        }
        let group = accum.entry(group_id).or_default();
        group.feature_count = group.feature_count.saturating_add(1);
        if group.color_rgb_u32.is_none() && feature.properties.c.len() >= 3 {
            group.color_rgb_u32 = Some(
                ((feature.properties.c[0] as u32) << 16)
                    | ((feature.properties.c[1] as u32) << 8)
                    | feature.properties.c[2] as u32,
            );
        }
        for (x, y) in iter_geometry_points(&feature.geometry) {
            group.bbox_min_x = Some(group.bbox_min_x.map_or(x, |v| v.min(x)));
            group.bbox_min_y = Some(group.bbox_min_y.map_or(y, |v| v.min(y)));
            group.bbox_max_x = Some(group.bbox_max_x.map_or(x, |v| v.max(x)));
            group.bbox_max_y = Some(group.bbox_max_y.map_or(y, |v| v.max(y)));
        }
    }

    if let Some(deck_path) = deck_graphs_path {
        let deck_file = File::open(deck_path)
            .with_context(|| format!("open deck rg graphs json: {}", deck_path.display()))?;
        let deck_rows: Vec<DeckGraphRow> =
            serde_json::from_reader(deck_file).context("parse deck rg graphs json")?;
        for row in deck_rows {
            if row.k == 0 {
                continue;
            }
            let group = accum.entry(row.k).or_default();
            if group.graph_world_x.is_none() {
                group.graph_world_x = Some(row.graphx);
                group.graph_world_z = Some(row.graphz);
            }
        }
    }

    region_rows.sort_by(|lhs, rhs| {
        lhs.region_group_id
            .cmp(&rhs.region_group_id)
            .then_with(|| lhs.region_id.cmp(&rhs.region_id))
    });
    region_rows.dedup_by(|lhs, rhs| {
        lhs.region_group_id == rhs.region_group_id && lhs.region_id == rhs.region_id
    });

    let source = source.trim().to_string();
    let mut meta_rows = Vec::with_capacity(accum.len());
    for (region_group_id, group) in accum {
        if region_group_id == 0 {
            continue;
        }
        meta_rows.push(RegionGroupMetaRow {
            region_group_id,
            color_rgb_u32: group.color_rgb_u32,
            feature_count: group.feature_count,
            region_count: u32::try_from(group.region_ids.len()).unwrap_or(0),
            accessible_region_count: group.accessible_region_count,
            bbox_min_x: group.bbox_min_x,
            bbox_min_y: group.bbox_min_y,
            bbox_max_x: group.bbox_max_x,
            bbox_max_y: group.bbox_max_y,
            graph_world_x: group.graph_world_x,
            graph_world_z: group.graph_world_z,
            source: source.clone(),
        });
    }
    meta_rows.sort_by_key(|row| row.region_group_id);

    Ok((meta_rows, region_rows))
}

fn iter_geometry_points(geometry: &GeoGeometry) -> Vec<(f64, f64)> {
    match geometry {
        GeoGeometry::Polygon(rings) => rings
            .iter()
            .flat_map(|ring| ring.iter().map(|point| (point[0], point[1])))
            .collect(),
        GeoGeometry::MultiPolygon(polygons) => polygons
            .iter()
            .flat_map(|rings| rings.iter())
            .flat_map(|ring| ring.iter().map(|point| (point[0], point[1])))
            .collect(),
    }
}
