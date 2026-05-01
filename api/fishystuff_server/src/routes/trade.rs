use axum::extract::{rejection::QueryRejection, Extension, Query, State};
use axum::http::{header, HeaderMap, HeaderValue};
use axum::Json;
use fishystuff_api::models::trade::{
    TradeNpcCatalogResponse, TradeNpcDestination, TradeRegionWaypointRef, TRADE_NPC_MAP_LAYER_ID,
    TRADE_NPC_MAP_LAYER_NAME,
};
use fishystuff_core::field_metadata::{
    FieldDetailFact, FieldDetailSection, FIELD_DETAIL_SECTION_KIND_FACTS,
};
use serde::Deserialize;
use serde::Serialize;
use serde_json::{json, Value};

use crate::error::{with_timeout, AppError, AppResult};
use crate::routes::meta::map_request_id;
use crate::state::{RequestId, SharedState};

#[derive(Debug, Deserialize)]
pub struct TradeNpcMapQuery {
    pub r#ref: Option<String>,
}

#[derive(Debug, Serialize)]
struct TradeNpcFeatureCollection {
    #[serde(rename = "type")]
    collection_type: &'static str,
    metadata: TradeNpcFeatureCollectionMetadata,
    features: Vec<TradeNpcFeature>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TradeNpcFeatureCollectionMetadata {
    layer_id: &'static str,
    layer_name: &'static str,
    schema: String,
    version: u32,
    coordinate_space: String,
    source_count: usize,
    destination_count: usize,
}

#[derive(Debug, Serialize)]
struct TradeNpcFeature {
    #[serde(rename = "type")]
    feature_type: &'static str,
    properties: Value,
    geometry: TradeNpcPointGeometry,
}

#[derive(Debug, Serialize)]
struct TradeNpcPointGeometry {
    #[serde(rename = "type")]
    geometry_type: &'static str,
    coordinates: [f64; 2],
}

pub async fn trade_npc_map_features(
    State(state): State<SharedState>,
    query: Result<Query<TradeNpcMapQuery>, QueryRejection>,
    Extension(request_id): Extension<RequestId>,
) -> AppResult<(HeaderMap, Json<Value>)> {
    let Query(query) = query.map_err(|err| {
        AppError::invalid_argument(err.to_string()).with_request_id(request_id.0.clone())
    })?;

    let catalog = with_timeout(
        state.config.request_timeout_secs,
        state.store.trade_npc_catalog(query.r#ref),
    )
    .await
    .map_err(|err| map_request_id(err, &request_id))?;

    let mut response_headers = HeaderMap::new();
    response_headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok((
        response_headers,
        Json(
            serde_json::to_value(trade_npc_feature_collection(catalog))
                .map_err(|err| AppError::internal(err.to_string()))?,
        ),
    ))
}

fn trade_npc_feature_collection(catalog: TradeNpcCatalogResponse) -> TradeNpcFeatureCollection {
    let destination_count = catalog.destinations.len();
    TradeNpcFeatureCollection {
        collection_type: "FeatureCollection",
        metadata: TradeNpcFeatureCollectionMetadata {
            layer_id: TRADE_NPC_MAP_LAYER_ID,
            layer_name: TRADE_NPC_MAP_LAYER_NAME,
            schema: catalog.schema,
            version: catalog.version,
            coordinate_space: catalog.coordinate_space,
            source_count: catalog.sources.len(),
            destination_count,
        },
        features: catalog
            .destinations
            .into_iter()
            .filter_map(trade_npc_feature)
            .collect(),
    }
}

fn trade_npc_feature(destination: TradeNpcDestination) -> Option<TradeNpcFeature> {
    if !destination.npc_spawn.world_x.is_finite() || !destination.npc_spawn.world_z.is_finite() {
        return None;
    }
    let npc_region = format_required_region(
        destination.npc_spawn.region_id,
        destination.npc_spawn.region_name.as_deref(),
    );
    let assigned_region = format_region_ref(&destination.assigned_region);
    let sell_origin = format_region_ref(&destination.sell_destination_trade_origin);
    let detail_sections = trade_npc_detail_sections(
        &destination,
        npc_region.as_deref(),
        assigned_region.as_deref(),
        sell_origin.as_deref(),
    );
    Some(TradeNpcFeature {
        feature_type: "Feature",
        properties: json!({
            "id": destination.id,
            "npcKey": destination.npc_key,
            "npcName": destination.npc_name,
            "name": destination.npc_name,
            "label": destination.npc_name,
            "roleSource": destination.role_source,
            "sourceTags": destination.source_tags,
            "trade": destination.trade,
            "npcSpawn": destination.npc_spawn,
            "assignedRegion": destination.assigned_region,
            "sellDestinationTradeOrigin": destination.sell_destination_trade_origin,
            "npcRegionLabel": npc_region,
            "assignedRegionLabel": assigned_region,
            "sellOriginLabel": sell_origin,
            "detailSections": detail_sections,
        }),
        geometry: TradeNpcPointGeometry {
            geometry_type: "Point",
            coordinates: [destination.npc_spawn.world_x, destination.npc_spawn.world_z],
        },
    })
}

fn trade_npc_detail_sections(
    destination: &TradeNpcDestination,
    npc_region: Option<&str>,
    assigned_region: Option<&str>,
    sell_origin: Option<&str>,
) -> Vec<FieldDetailSection> {
    let mut facts = vec![
        fact("trade_npc", "NPC", destination.npc_name.as_str()),
        fact("npc_key", "NPC Key", destination.npc_key.to_string()),
        fact(
            "role_source",
            "Role Source",
            destination.role_source.as_str(),
        ),
    ];
    if let Some(value) = npc_region {
        facts.push(fact("spawn_region", "Spawn Region", value));
    }
    if let Some(value) = assigned_region {
        facts.push(fact("assigned_region", "Assigned Region", value));
    }
    if let Some(value) = sell_origin {
        facts.push(fact("sell_origin", "Sell Origin", value));
    }
    if let Some(value) = destination.trade.item_main_group_key {
        facts.push(fact(
            "item_main_group",
            "Item Main Group",
            value.to_string(),
        ));
    }
    if let Some(value) = destination.trade.trade_group_type.as_deref() {
        facts.push(fact("trade_group", "Trade Group", value));
    }
    if !destination.source_tags.is_empty() {
        facts.push(fact(
            "sources",
            "Sources",
            destination.source_tags.join(", "),
        ));
    }

    vec![FieldDetailSection {
        id: "trade_npc".to_string(),
        kind: FIELD_DETAIL_SECTION_KIND_FACTS.to_string(),
        title: Some(TRADE_NPC_MAP_LAYER_NAME.to_string()),
        facts,
        targets: Vec::new(),
    }]
}

fn fact(key: &str, label: &str, value: impl Into<String>) -> FieldDetailFact {
    FieldDetailFact {
        key: key.to_string(),
        label: label.to_string(),
        value: value.into(),
        icon: None,
        status_icon: None,
        status_icon_tone: None,
    }
}

fn format_required_region(region_id: u32, region_name: Option<&str>) -> Option<String> {
    let name = region_name.map(str::trim).filter(|value| !value.is_empty());
    match name {
        Some(name) => Some(format!("{name} (R{region_id})")),
        None => Some(format!("R{region_id}")),
    }
}

fn format_region_ref(region: &TradeRegionWaypointRef) -> Option<String> {
    let name = region
        .region_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    match (name, region.region_id) {
        (Some(name), Some(region_id)) => Some(format!("{name} (R{region_id})")),
        (Some(name), None) => Some(name.to_string()),
        (None, Some(region_id)) => Some(format!("R{region_id}")),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use fishystuff_api::models::trade::{
        TradeNpcCatalogResponse, TradeNpcDestination, TradeNpcSpawn, TradeNpcTradeInfo,
        TradeRegionWaypointRef,
    };

    use super::trade_npc_feature_collection;

    #[test]
    fn trade_npc_feature_collection_contains_map_waypoint_details() {
        let collection = trade_npc_feature_collection(TradeNpcCatalogResponse {
            schema: "trade-npc-catalog".to_string(),
            version: 1,
            coordinate_space: "world_xz".to_string(),
            destinations: vec![TradeNpcDestination {
                id: "npc:1:region:5".to_string(),
                npc_key: 1,
                npc_name: "Crio".to_string(),
                role_source: "characterfunctiondata_trade".to_string(),
                source_tags: vec!["characterfunctiondata_trade".to_string()],
                trade: TradeNpcTradeInfo {
                    item_main_group_key: Some(100),
                    trade_group_type: Some("Fish".to_string()),
                },
                npc_spawn: TradeNpcSpawn {
                    region_id: 5,
                    region_name: Some("Velia".to_string()),
                    world_x: 10.0,
                    world_y: 0.0,
                    world_z: 20.0,
                },
                assigned_region: TradeRegionWaypointRef {
                    region_id: Some(5),
                    region_name: Some("Velia".to_string()),
                    waypoint_id: Some(1001),
                    waypoint_name: Some("Velia".to_string()),
                    world_x: Some(11.0),
                    world_z: Some(21.0),
                },
                sell_destination_trade_origin: TradeRegionWaypointRef {
                    region_id: Some(5),
                    region_name: Some("Velia".to_string()),
                    waypoint_id: Some(1001),
                    waypoint_name: Some("Velia".to_string()),
                    world_x: Some(11.0),
                    world_z: Some(21.0),
                },
            }],
            ..TradeNpcCatalogResponse::default()
        });

        assert_eq!(collection.features.len(), 1);
        let feature = &collection.features[0];
        assert_eq!(feature.geometry.coordinates, [10.0, 20.0]);
        assert_eq!(feature.properties["label"], "Crio");
        assert_eq!(feature.properties["npcRegionLabel"], "Velia (R5)");
        assert!(feature.properties["detailSections"].is_array());
    }
}
