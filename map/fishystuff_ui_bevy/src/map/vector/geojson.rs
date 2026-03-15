use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Debug, Clone)]
pub struct ParsedGeoJson {
    pub features: Vec<ParsedFeature>,
    pub stats: ParsedGeoJsonStats,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ParsedGeoJsonStats {
    pub feature_count: u32,
    pub polygon_count: u32,
    pub multipolygon_count: u32,
    pub hole_ring_count: u32,
    pub vertex_count: u32,
}

#[derive(Debug, Clone)]
pub struct ParsedFeature {
    pub properties: Map<String, Value>,
    pub polygons: Vec<PolygonRings>,
}

#[derive(Debug, Clone)]
pub struct PolygonRings {
    pub rings: Vec<Vec<[f64; 2]>>,
}

#[derive(Debug, Deserialize)]
struct GeoJsonCollection {
    #[serde(default)]
    features: Vec<GeoJsonFeature>,
}

#[derive(Debug, Deserialize)]
struct GeoJsonFeature {
    #[serde(default)]
    properties: Map<String, Value>,
    #[serde(default)]
    geometry: Option<GeoJsonGeometry>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "coordinates")]
enum GeoJsonGeometry {
    Polygon(Vec<Vec<[f64; 2]>>),
    MultiPolygon(Vec<Vec<Vec<[f64; 2]>>>),
}

pub fn parse_geojson(bytes: &[u8]) -> Result<ParsedGeoJson, String> {
    crate::perf_scope!("vector.geojson_parse");
    let collection: GeoJsonCollection =
        serde_json::from_slice(bytes).map_err(|err| format!("parse geojson: {err}"))?;

    let mut features = Vec::with_capacity(collection.features.len());
    let mut stats = ParsedGeoJsonStats::default();
    for feature in collection.features {
        let Some(geometry) = feature.geometry else {
            continue;
        };
        let (polygons, was_multipolygon) = match geometry {
            GeoJsonGeometry::Polygon(rings) => {
                if rings.is_empty() {
                    (Vec::new(), false)
                } else {
                    (vec![PolygonRings { rings }], false)
                }
            }
            GeoJsonGeometry::MultiPolygon(polygons) => (
                polygons
                    .into_iter()
                    .filter(|rings| !rings.is_empty())
                    .map(|rings| PolygonRings { rings })
                    .collect(),
                true,
            ),
        };
        if polygons.is_empty() {
            continue;
        }

        stats.feature_count = stats.feature_count.saturating_add(1);
        if was_multipolygon {
            stats.multipolygon_count = stats.multipolygon_count.saturating_add(1);
        }
        for polygon in &polygons {
            stats.polygon_count = stats.polygon_count.saturating_add(1);
            stats.hole_ring_count = stats
                .hole_ring_count
                .saturating_add(polygon.rings.len().saturating_sub(1) as u32);
            for ring in &polygon.rings {
                stats.vertex_count = stats.vertex_count.saturating_add(ring.len() as u32);
            }
        }
        features.push(ParsedFeature {
            properties: feature.properties,
            polygons,
        });
    }

    Ok(ParsedGeoJson { features, stats })
}

#[cfg(test)]
mod tests {
    use super::parse_geojson;

    #[test]
    fn counts_polygon_multipolygon_and_holes() {
        let sample = br#"{
          "type":"FeatureCollection",
          "features":[
            {
              "type":"Feature",
              "properties":{"c":[1,2,3]},
              "geometry":{"type":"Polygon","coordinates":[[[0,0],[4,0],[4,4],[0,4],[0,0]],[[1,1],[2,1],[2,2],[1,2],[1,1]]]}
            },
            {
              "type":"Feature",
              "properties":{"c":[3,4,5]},
              "geometry":{"type":"MultiPolygon","coordinates":[
                [[[10,10],[12,10],[12,12],[10,12],[10,10]]],
                [[[20,20],[23,20],[23,23],[20,23],[20,20]]]
              ]}
            }
          ]
        }"#;
        let parsed = parse_geojson(sample).expect("parse");
        assert_eq!(parsed.stats.feature_count, 2);
        assert_eq!(parsed.stats.polygon_count, 3);
        assert_eq!(parsed.stats.multipolygon_count, 1);
        assert_eq!(parsed.stats.hole_ring_count, 1);
        assert!(parsed.stats.vertex_count >= 15);
    }
}
