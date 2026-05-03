use crate::public_assets::{normalize_public_base_url, resolve_public_asset_url};
use fishystuff_api::models::trade::{TRADE_NPC_MAP_LAYER_ID, TRADE_NPC_MAP_LAYER_NAME};

use super::{
    default_layer_filter_bindings_for_runtime_layer, FieldColorMode, FieldMetadataSourceSpec,
    FieldSourceSpec, GeometrySpace, LayerKind, LayerSpec, LayerTransform, LodPolicy, PickMode,
    WaypointSourceSpec, FISH_EVIDENCE_LAYER_KEY, HOTSPOTS_LAYER_KEY,
};

const LOCAL_LAYER_CATALOG_REVISION: &str = "local-layer-catalog-v6";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvailableLayerTemplate {
    Bookmarks,
    Minimap,
    ZoneMask,
    FishEvidence,
    RegionGroups,
    Regions,
    RegionNodes,
    Hotspots,
    TradeNpcs,
}

#[derive(Debug, Clone)]
pub struct AvailableLayerDefinition {
    pub layer_id: String,
    pub name: String,
    pub template: AvailableLayerTemplate,
    pub visible_default: bool,
    pub opacity_default: f32,
    pub z_base: f32,
    pub display_order: i32,
}

#[derive(bevy::prelude::Resource, Debug, Clone)]
pub struct AvailableLayerCatalog {
    entries: Vec<AvailableLayerDefinition>,
}

impl AvailableLayerCatalog {
    pub fn entries(&self) -> &[AvailableLayerDefinition] {
        &self.entries
    }

    pub fn set_entries(&mut self, entries: Vec<AvailableLayerDefinition>) {
        self.entries = entries;
    }

    pub fn push(&mut self, entry: AvailableLayerDefinition) {
        self.entries.push(entry);
    }

    pub fn remove_by_layer_id(&mut self, layer_id: &str) -> Option<AvailableLayerDefinition> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.layer_id == layer_id)?;
        Some(self.entries.remove(index))
    }
}

impl Default for AvailableLayerCatalog {
    fn default() -> Self {
        Self {
            entries: vec![
                AvailableLayerDefinition {
                    layer_id: "bookmarks".to_string(),
                    name: "Bookmarks".to_string(),
                    template: AvailableLayerTemplate::Bookmarks,
                    visible_default: true,
                    opacity_default: 1.0,
                    z_base: 40.4,
                    display_order: 40,
                },
                AvailableLayerDefinition {
                    layer_id: "minimap".to_string(),
                    name: "Minimap".to_string(),
                    template: AvailableLayerTemplate::Minimap,
                    visible_default: true,
                    opacity_default: 1.0,
                    z_base: 0.0,
                    display_order: 0,
                },
                AvailableLayerDefinition {
                    layer_id: "zone_mask".to_string(),
                    name: "Zone Mask".to_string(),
                    template: AvailableLayerTemplate::ZoneMask,
                    visible_default: true,
                    opacity_default: 0.35,
                    z_base: 10.0,
                    display_order: 10,
                },
                AvailableLayerDefinition {
                    layer_id: FISH_EVIDENCE_LAYER_KEY.to_string(),
                    name: "Fish Evidence".to_string(),
                    template: AvailableLayerTemplate::FishEvidence,
                    visible_default: true,
                    opacity_default: 1.0,
                    z_base: 40.0,
                    display_order: 39,
                },
                AvailableLayerDefinition {
                    layer_id: "region_groups".to_string(),
                    name: "Region Groups".to_string(),
                    template: AvailableLayerTemplate::RegionGroups,
                    visible_default: false,
                    opacity_default: 0.50,
                    z_base: 30.0,
                    display_order: 30,
                },
                AvailableLayerDefinition {
                    layer_id: "regions".to_string(),
                    name: "Regions".to_string(),
                    template: AvailableLayerTemplate::Regions,
                    visible_default: false,
                    opacity_default: 0.35,
                    z_base: 31.0,
                    display_order: 31,
                },
                AvailableLayerDefinition {
                    layer_id: "region_nodes".to_string(),
                    name: "Node Waypoints".to_string(),
                    template: AvailableLayerTemplate::RegionNodes,
                    visible_default: false,
                    opacity_default: 1.0,
                    z_base: 41.0,
                    display_order: 41,
                },
                AvailableLayerDefinition {
                    layer_id: HOTSPOTS_LAYER_KEY.to_string(),
                    name: "Hotspots".to_string(),
                    template: AvailableLayerTemplate::Hotspots,
                    visible_default: false,
                    opacity_default: 0.85,
                    z_base: 41.5,
                    display_order: 42,
                },
                AvailableLayerDefinition {
                    layer_id: TRADE_NPC_MAP_LAYER_ID.to_string(),
                    name: TRADE_NPC_MAP_LAYER_NAME.to_string(),
                    template: AvailableLayerTemplate::TradeNpcs,
                    visible_default: false,
                    opacity_default: 1.0,
                    z_base: 41.8,
                    display_order: 43,
                },
            ],
        }
    }
}

pub fn build_local_layer_specs(
    entries: &[AvailableLayerDefinition],
    map_version_id: Option<&str>,
) -> (String, Vec<LayerSpec>) {
    let public_base = normalize_public_base_url(None);
    let layers = entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| {
            build_local_layer_spec(idx as u16, entry, map_version_id, public_base.as_deref())
        })
        .collect();
    (local_layer_catalog_revision(map_version_id), layers)
}

fn build_local_layer_spec(
    raw_id: u16,
    entry: &AvailableLayerDefinition,
    map_version_id: Option<&str>,
    public_base_url: Option<&str>,
) -> LayerSpec {
    match entry.template {
        AvailableLayerTemplate::Bookmarks => LayerSpec {
            id: super::LayerId::from_raw(raw_id),
            key: entry.layer_id.clone(),
            name: entry.name.clone(),
            visible_default: entry.visible_default,
            opacity_default: entry.opacity_default,
            z_base: entry.z_base,
            kind: LayerKind::Waypoints,
            tileset_url: String::new(),
            tile_url_template: String::new(),
            tileset_version: String::new(),
            vector_source: None,
            waypoint_source: None,
            transform: LayerTransform::IdentityMapSpace,
            tile_px: 0,
            max_level: 0,
            y_flip: false,
            field_source: None,
            field_metadata_source: None,
            filter_bindings: default_layer_filter_bindings_for_runtime_layer(
                &entry.layer_id,
                LayerKind::Waypoints,
                false,
            ),
            lod_policy: default_lod_policy(),
            request_weight: 1.0,
            pick_mode: PickMode::None,
            display_order: entry.display_order,
        },
        AvailableLayerTemplate::Minimap => LayerSpec {
            id: super::LayerId::from_raw(raw_id),
            key: entry.layer_id.clone(),
            name: entry.name.clone(),
            visible_default: entry.visible_default,
            opacity_default: entry.opacity_default,
            z_base: entry.z_base,
            kind: LayerKind::TiledRaster,
            tileset_url: resolve_public(
                "/images/tiles/minimap_visual/v1/tileset.json",
                public_base_url,
            ),
            tile_url_template: resolve_public(
                "/images/tiles/minimap_visual/v1/{z}/{x}_{y}.png",
                public_base_url,
            ),
            tileset_version: version_or_default(map_version_id),
            vector_source: None,
            waypoint_source: None,
            transform: LayerTransform::IdentityMapSpace,
            tile_px: 512,
            max_level: 2,
            y_flip: false,
            field_source: None,
            field_metadata_source: None,
            filter_bindings: default_layer_filter_bindings_for_runtime_layer(
                &entry.layer_id,
                LayerKind::TiledRaster,
                false,
            ),
            lod_policy: LodPolicy {
                target_tiles: 16,
                hysteresis_hi: 24.0,
                hysteresis_lo: 8.0,
                margin_tiles: 1,
                enable_refine: false,
                refine_debounce_ms: 0,
                max_detail_tiles: 0,
                max_resident_tiles: 128,
                pinned_coarse_levels: 0,
                coarse_pin_min_level: None,
                warm_margin_tiles: 1,
                protected_margin_tiles: 1,
                detail_eviction_weight: 4.0,
                max_detail_requests_while_camera_moving: 1,
                motion_suppresses_refine: true,
            },
            request_weight: 1.0,
            pick_mode: PickMode::None,
            display_order: entry.display_order,
        },
        AvailableLayerTemplate::ZoneMask => LayerSpec {
            id: super::LayerId::from_raw(raw_id),
            key: entry.layer_id.clone(),
            name: entry.name.clone(),
            visible_default: entry.visible_default,
            opacity_default: entry.opacity_default,
            z_base: entry.z_base,
            kind: LayerKind::Field,
            tileset_url: String::new(),
            tile_url_template: String::new(),
            tileset_version: String::new(),
            vector_source: None,
            waypoint_source: None,
            transform: LayerTransform::IdentityMapSpace,
            tile_px: 512,
            max_level: 0,
            y_flip: false,
            field_source: Some(FieldSourceSpec {
                url: cache_busted_public(
                    &format!(
                        "/fields/zone_mask.{}.bin",
                        version_or_default(map_version_id)
                    ),
                    "zone-lookup-v1",
                    public_base_url,
                ),
                revision: "zone-lookup-v1".to_string(),
                color_mode: FieldColorMode::RgbU24,
            }),
            field_metadata_source: Some(FieldMetadataSourceSpec {
                url: cache_busted_public(
                    &format!(
                        "/fields/zone_mask.{}.meta.json",
                        version_or_default(map_version_id)
                    ),
                    "zone-meta-v1",
                    public_base_url,
                ),
                revision: "zone-meta-v1".to_string(),
            }),
            filter_bindings: default_layer_filter_bindings_for_runtime_layer(
                &entry.layer_id,
                LayerKind::Field,
                true,
            ),
            lod_policy: default_lod_policy(),
            request_weight: 1.0,
            pick_mode: PickMode::ExactTilePixel,
            display_order: entry.display_order,
        },
        AvailableLayerTemplate::FishEvidence => LayerSpec {
            id: super::LayerId::from_raw(raw_id),
            key: entry.layer_id.clone(),
            name: entry.name.clone(),
            visible_default: entry.visible_default,
            opacity_default: entry.opacity_default,
            z_base: entry.z_base,
            kind: LayerKind::Waypoints,
            tileset_url: String::new(),
            tile_url_template: String::new(),
            tileset_version: String::new(),
            vector_source: None,
            waypoint_source: None,
            transform: LayerTransform::IdentityMapSpace,
            tile_px: 0,
            max_level: 0,
            y_flip: false,
            field_source: None,
            field_metadata_source: None,
            filter_bindings: default_layer_filter_bindings_for_runtime_layer(
                &entry.layer_id,
                LayerKind::Waypoints,
                false,
            ),
            lod_policy: default_lod_policy(),
            request_weight: 1.0,
            pick_mode: PickMode::None,
            display_order: entry.display_order,
        },
        AvailableLayerTemplate::RegionGroups => build_field_layer(
            raw_id,
            entry,
            map_version_id,
            public_base_url,
            "/fields/region_groups.{version}.bin",
            "rg-field-v1",
            "/fields/region_groups.{version}.meta.json",
            "rg-meta-v1",
        ),
        AvailableLayerTemplate::Regions => build_field_layer(
            raw_id,
            entry,
            map_version_id,
            public_base_url,
            "/fields/regions.{version}.bin",
            "r-field-v1",
            "/fields/regions.{version}.meta.json",
            "r-meta-v1",
        ),
        AvailableLayerTemplate::RegionNodes => LayerSpec {
            id: super::LayerId::from_raw(raw_id),
            key: entry.layer_id.clone(),
            name: entry.name.clone(),
            visible_default: entry.visible_default,
            opacity_default: entry.opacity_default,
            z_base: entry.z_base,
            kind: LayerKind::Waypoints,
            tileset_url: String::new(),
            tile_url_template: String::new(),
            tileset_version: String::new(),
            vector_source: None,
            waypoint_source: Some(WaypointSourceSpec {
                url: cache_busted_public(
                    &format!(
                        "/waypoints/region_nodes.{}.geojson",
                        version_or_default(map_version_id)
                    ),
                    "region-nodes-v1",
                    public_base_url,
                ),
                revision: "region-nodes-v1".to_string(),
                geometry_space: GeometrySpace::World,
                feature_id_property: Some("r".to_string()),
                label_property: Some("label".to_string()),
                name_property: Some("name".to_string()),
                supports_connections: true,
                supports_labels: true,
                show_connections_default: true,
                show_labels_default: true,
            }),
            transform: LayerTransform::IdentityMapSpace,
            tile_px: 0,
            max_level: 0,
            y_flip: false,
            field_source: None,
            field_metadata_source: None,
            filter_bindings: default_layer_filter_bindings_for_runtime_layer(
                &entry.layer_id,
                LayerKind::Waypoints,
                false,
            ),
            lod_policy: default_lod_policy(),
            request_weight: 1.0,
            pick_mode: PickMode::None,
            display_order: entry.display_order,
        },
        AvailableLayerTemplate::Hotspots => LayerSpec {
            id: super::LayerId::from_raw(raw_id),
            key: entry.layer_id.clone(),
            name: entry.name.clone(),
            visible_default: entry.visible_default,
            opacity_default: entry.opacity_default,
            z_base: entry.z_base,
            kind: LayerKind::Waypoints,
            tileset_url: String::new(),
            tile_url_template: String::new(),
            tileset_version: String::new(),
            vector_source: None,
            waypoint_source: Some(WaypointSourceSpec {
                url: cache_busted_public(
                    &format!(
                        "/hotspots/hotspots.{}.json",
                        version_or_default(map_version_id)
                    ),
                    "hotspots-v1-icons",
                    public_base_url,
                ),
                revision: "hotspots-v1-icons".to_string(),
                geometry_space: GeometrySpace::World,
                feature_id_property: None,
                label_property: None,
                name_property: None,
                supports_connections: false,
                supports_labels: false,
                show_connections_default: false,
                show_labels_default: false,
            }),
            transform: LayerTransform::IdentityMapSpace,
            tile_px: 0,
            max_level: 0,
            y_flip: false,
            field_source: None,
            field_metadata_source: None,
            filter_bindings: Vec::new(),
            lod_policy: default_lod_policy(),
            request_weight: 1.0,
            pick_mode: PickMode::None,
            display_order: entry.display_order,
        },
        AvailableLayerTemplate::TradeNpcs => LayerSpec {
            id: super::LayerId::from_raw(raw_id),
            key: entry.layer_id.clone(),
            name: entry.name.clone(),
            visible_default: entry.visible_default,
            opacity_default: entry.opacity_default,
            z_base: entry.z_base,
            kind: LayerKind::Waypoints,
            tileset_url: String::new(),
            tile_url_template: String::new(),
            tileset_version: String::new(),
            vector_source: None,
            waypoint_source: Some(WaypointSourceSpec {
                url: "/api/v1/trade_npcs/map".to_string(),
                revision: "trade-npcs-v1".to_string(),
                geometry_space: GeometrySpace::World,
                feature_id_property: Some("id".to_string()),
                label_property: Some("label".to_string()),
                name_property: Some("name".to_string()),
                supports_connections: false,
                supports_labels: true,
                show_connections_default: false,
                show_labels_default: false,
            }),
            transform: LayerTransform::IdentityMapSpace,
            tile_px: 0,
            max_level: 0,
            y_flip: false,
            field_source: None,
            field_metadata_source: None,
            filter_bindings: Vec::new(),
            lod_policy: default_lod_policy(),
            request_weight: 1.0,
            pick_mode: PickMode::None,
            display_order: entry.display_order,
        },
    }
}

fn build_field_layer(
    raw_id: u16,
    entry: &AvailableLayerDefinition,
    map_version_id: Option<&str>,
    public_base_url: Option<&str>,
    field_path_template: &str,
    field_revision: &str,
    metadata_path_template: &str,
    metadata_revision: &str,
) -> LayerSpec {
    let version = version_or_default(map_version_id);
    LayerSpec {
        id: super::LayerId::from_raw(raw_id),
        key: entry.layer_id.clone(),
        name: entry.name.clone(),
        visible_default: entry.visible_default,
        opacity_default: entry.opacity_default,
        z_base: entry.z_base,
        kind: LayerKind::Field,
        tileset_url: String::new(),
        tile_url_template: String::new(),
        tileset_version: String::new(),
        vector_source: None,
        waypoint_source: None,
        transform: LayerTransform::IdentityMapSpace,
        tile_px: 512,
        max_level: 0,
        y_flip: false,
        field_source: Some(FieldSourceSpec {
            url: cache_busted_public(
                &field_path_template.replace("{version}", &version),
                field_revision,
                public_base_url,
            ),
            revision: field_revision.to_string(),
            color_mode: FieldColorMode::DebugHash,
        }),
        field_metadata_source: Some(FieldMetadataSourceSpec {
            url: cache_busted_public(
                &metadata_path_template.replace("{version}", &version),
                metadata_revision,
                public_base_url,
            ),
            revision: metadata_revision.to_string(),
        }),
        filter_bindings: default_layer_filter_bindings_for_runtime_layer(
            &entry.layer_id,
            LayerKind::Field,
            true,
        ),
        lod_policy: default_lod_policy(),
        request_weight: 1.0,
        pick_mode: PickMode::None,
        display_order: entry.display_order,
    }
}

fn local_layer_catalog_revision(map_version_id: Option<&str>) -> String {
    match map_version_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(map_version_id) => format!("{LOCAL_LAYER_CATALOG_REVISION}:{map_version_id}"),
        None => LOCAL_LAYER_CATALOG_REVISION.to_string(),
    }
}

fn version_or_default(map_version_id: Option<&str>) -> String {
    map_version_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("v1")
        .to_string()
}

fn resolve_public(path: &str, public_base_url: Option<&str>) -> String {
    resolve_public_asset_url(Some(path), public_base_url).unwrap_or_else(|| path.to_string())
}

fn cache_busted_public(path: &str, revision: &str, public_base_url: Option<&str>) -> String {
    let resolved = resolve_public(path, public_base_url);
    let separator = if resolved.contains('?') { '&' } else { '?' };
    format!("{resolved}{separator}v={revision}")
}

fn default_lod_policy() -> LodPolicy {
    LodPolicy {
        target_tiles: 220,
        hysteresis_hi: 280.0,
        hysteresis_lo: 160.0,
        margin_tiles: 1,
        enable_refine: false,
        refine_debounce_ms: 0,
        max_detail_tiles: 0,
        max_resident_tiles: 4096,
        pinned_coarse_levels: 2,
        coarse_pin_min_level: None,
        warm_margin_tiles: 3,
        protected_margin_tiles: 1,
        detail_eviction_weight: 4.0,
        max_detail_requests_while_camera_moving: 2,
        motion_suppresses_refine: true,
    }
}

#[cfg(test)]
mod tests {
    use super::{build_local_layer_specs, AvailableLayerDefinition, AvailableLayerTemplate};
    use crate::map::layers::LayerKind;

    #[test]
    fn duplicate_zone_mask_templates_keep_explicit_assets() {
        let entries = vec![
            AvailableLayerDefinition {
                layer_id: "zone_mask".to_string(),
                name: "Zone Mask".to_string(),
                template: AvailableLayerTemplate::ZoneMask,
                visible_default: true,
                opacity_default: 0.35,
                z_base: 10.0,
                display_order: 10,
            },
            AvailableLayerDefinition {
                layer_id: "zone_mask_variant".to_string(),
                name: "Zone Mask Variant".to_string(),
                template: AvailableLayerTemplate::ZoneMask,
                visible_default: false,
                opacity_default: 0.5,
                z_base: 11.0,
                display_order: 11,
            },
        ];

        let (_, layers) = build_local_layer_specs(&entries, Some("v1"));
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].kind, LayerKind::Field);
        assert_eq!(layers[1].kind, LayerKind::Field);
        assert_eq!(
            layers[0]
                .field_source
                .as_ref()
                .map(|source| source.url.as_str()),
            layers[1]
                .field_source
                .as_ref()
                .map(|source| source.url.as_str()),
            "zone-mask exact lookup source should come from the template, not the layer id"
        );
    }

    #[test]
    fn bookmarks_layer_is_a_waypoint_layer() {
        let (_, layers) = build_local_layer_specs(
            &[AvailableLayerDefinition {
                layer_id: "bookmarks".to_string(),
                name: "Bookmarks".to_string(),
                template: AvailableLayerTemplate::Bookmarks,
                visible_default: true,
                opacity_default: 1.0,
                z_base: 40.4,
                display_order: 40,
            }],
            Some("v1"),
        );

        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].kind, LayerKind::Waypoints);
        assert!(layers[0].waypoint_source.is_none());
    }

    #[test]
    fn region_group_field_layer_uses_field_assets() {
        let (_, layers) = build_local_layer_specs(
            &[AvailableLayerDefinition {
                layer_id: "region_groups".to_string(),
                name: "Region Groups".to_string(),
                template: AvailableLayerTemplate::RegionGroups,
                visible_default: false,
                opacity_default: 0.5,
                z_base: 30.0,
                display_order: 30,
            }],
            Some("v1"),
        );

        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].kind, LayerKind::Field);
        assert!(layers[0].vector_source.is_none());
        assert!(layers[0]
            .field_source
            .as_ref()
            .is_some_and(|source| source.url.starts_with("/fields/region_groups.v1.bin")));
        assert!(layers[0]
            .field_metadata_source
            .as_ref()
            .is_some_and(|source| {
                source.url.starts_with("/fields/region_groups.v1.meta.json")
            }));
    }

    #[test]
    fn trade_npc_layer_uses_api_waypoint_source() {
        let (_, layers) = build_local_layer_specs(
            &[AvailableLayerDefinition {
                layer_id: "trade_npcs".to_string(),
                name: "Trade NPCs".to_string(),
                template: AvailableLayerTemplate::TradeNpcs,
                visible_default: false,
                opacity_default: 1.0,
                z_base: 41.8,
                display_order: 42,
            }],
            Some("v1"),
        );

        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].kind, LayerKind::Waypoints);
        assert!(layers[0].filter_bindings.is_empty());
        let source = layers[0].waypoint_source.as_ref().expect("waypoint source");
        assert_eq!(source.url, "/api/v1/trade_npcs/map");
        assert_eq!(source.feature_id_property.as_deref(), Some("id"));
        assert!(!source.supports_connections);
        assert!(source.supports_labels);
        assert!(!source.show_labels_default);
    }

    #[test]
    fn hotspot_layer_uses_source_backed_hotspot_asset() {
        let (_, layers) = build_local_layer_specs(
            &[AvailableLayerDefinition {
                layer_id: "hotspots".to_string(),
                name: "Hotspots".to_string(),
                template: AvailableLayerTemplate::Hotspots,
                visible_default: false,
                opacity_default: 0.85,
                z_base: 41.5,
                display_order: 42,
            }],
            Some("v1"),
        );

        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].kind, LayerKind::Waypoints);
        let source = layers[0].waypoint_source.as_ref().expect("hotspot source");
        assert!(source.url.starts_with("/hotspots/hotspots.v1.json"));
        assert_eq!(source.revision, "hotspots-v1-icons");
        assert!(!source.supports_connections);
        assert!(!source.supports_labels);
    }
}
