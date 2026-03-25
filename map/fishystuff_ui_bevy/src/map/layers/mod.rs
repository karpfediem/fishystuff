mod catalog;
mod registry;
mod runtime;

use crate::map::spaces::layer_transform::{LayerTransform, WorldTransform};
use crate::map::spaces::world::MapToWorld;
use crate::public_assets::{normalize_public_base_url, resolve_public_asset_url};

pub use catalog::{
    build_local_layer_specs, AvailableLayerCatalog, AvailableLayerDefinition,
    AvailableLayerTemplate,
};
pub use registry::LayerRegistry;
pub use runtime::{LayerManifestStatus, LayerRuntime, LayerRuntimeState, LayerSettings};

pub const FISH_EVIDENCE_LAYER_KEY: &str = "fish_evidence";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LayerId(u16);

impl LayerId {
    pub const fn from_raw(raw: u16) -> Self {
        Self(raw)
    }

    pub const fn as_u16(self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct LodPolicy {
    pub target_tiles: usize,
    pub hysteresis_hi: f32,
    pub hysteresis_lo: f32,
    pub margin_tiles: i32,
    pub enable_refine: bool,
    pub refine_debounce_ms: u32,
    pub max_detail_tiles: usize,
    pub max_resident_tiles: usize,
    pub pinned_coarse_levels: u8,
    pub coarse_pin_min_level: Option<i32>,
    pub warm_margin_tiles: i32,
    pub protected_margin_tiles: i32,
    pub detail_eviction_weight: f32,
    pub max_detail_requests_while_camera_moving: usize,
    pub motion_suppresses_refine: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickMode {
    None,
    ExactTilePixel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerRenderKind {
    IdentitySprite,
    AffineQuad,
    VectorGeoJson,
    Waypoints,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerKind {
    TiledRaster,
    VectorGeoJson,
    Waypoints,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometrySpace {
    MapPixels,
    World,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleMode {
    FeaturePropertyPalette,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldColorMode {
    RgbU24,
    DebugHash,
}

#[derive(Debug, Clone)]
pub struct FieldSourceSpec {
    pub url: String,
    pub revision: String,
    pub color_mode: FieldColorMode,
}

#[derive(Debug, Clone)]
pub struct FieldMetadataSourceSpec {
    pub url: String,
    pub revision: String,
}

#[derive(Debug, Clone)]
pub struct VectorSourceSpec {
    pub url: String,
    pub revision: String,
    pub geometry_space: GeometrySpace,
    pub style_mode: StyleMode,
    pub feature_id_property: Option<String>,
    pub color_property: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WaypointSourceSpec {
    pub url: String,
    pub revision: String,
    pub geometry_space: GeometrySpace,
    pub feature_id_property: Option<String>,
    pub label_property: Option<String>,
    pub name_property: Option<String>,
    pub supports_connections: bool,
    pub supports_labels: bool,
    pub show_connections_default: bool,
    pub show_labels_default: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayerVectorStatus {
    #[default]
    Inactive,
    NotRequested,
    Fetching,
    Parsing,
    Building,
    Ready,
    Failed,
}

#[derive(Debug, Clone)]
pub struct LayerSpec {
    pub id: LayerId,
    pub key: String,
    pub name: String,
    pub visible_default: bool,
    pub opacity_default: f32,
    pub z_base: f32,
    pub kind: LayerKind,
    pub tileset_url: String,
    pub tile_url_template: String,
    pub tileset_version: String,
    pub vector_source: Option<VectorSourceSpec>,
    pub waypoint_source: Option<WaypointSourceSpec>,
    pub transform: LayerTransform,
    pub tile_px: u32,
    pub max_level: u8,
    pub y_flip: bool,
    pub field_source: Option<FieldSourceSpec>,
    pub field_metadata_source: Option<FieldMetadataSourceSpec>,
    pub lod_policy: LodPolicy,
    pub request_weight: f32,
    pub pick_mode: PickMode,
    pub display_order: i32,
}

impl LayerSpec {
    pub fn world_transform(&self, map_to_world: MapToWorld) -> Option<WorldTransform> {
        WorldTransform::new(self.transform, map_to_world)
    }

    pub fn render_kind(&self) -> LayerRenderKind {
        if self.kind == LayerKind::VectorGeoJson {
            return LayerRenderKind::VectorGeoJson;
        }
        if self.kind == LayerKind::Waypoints {
            return LayerRenderKind::Waypoints;
        }
        match self.transform {
            LayerTransform::IdentityMapSpace => LayerRenderKind::IdentitySprite,
            LayerTransform::AffineToMap(_) | LayerTransform::AffineToWorld(_) => {
                LayerRenderKind::AffineQuad
            }
        }
    }

    pub fn is_raster(&self) -> bool {
        self.kind == LayerKind::TiledRaster
    }

    pub fn is_vector(&self) -> bool {
        self.kind == LayerKind::VectorGeoJson
    }

    pub fn is_waypoints(&self) -> bool {
        self.kind == LayerKind::Waypoints
    }

    pub fn is_zone_mask_visual_layer(&self) -> bool {
        self.is_raster() && self.pick_mode == PickMode::ExactTilePixel && self.key == "zone_mask"
    }

    pub fn field_url(&self) -> Option<String> {
        if let Some(field_source) = self.field_source.as_ref() {
            let url = field_source.url.trim();
            return (!url.is_empty()).then(|| url.to_string());
        }
        self.exact_lookup_url()
    }

    pub fn field_revision(&self) -> Option<&str> {
        self.field_source.as_ref().and_then(|field_source| {
            let revision = field_source.revision.trim();
            (!revision.is_empty()).then_some(revision)
        })
    }

    pub fn field_color_mode(&self) -> Option<FieldColorMode> {
        self.field_source
            .as_ref()
            .map(|field_source| field_source.color_mode)
            .or_else(|| self.exact_lookup_url().map(|_| FieldColorMode::RgbU24))
    }

    pub fn field_metadata_url(&self) -> Option<String> {
        self.field_metadata_source.as_ref().and_then(|source| {
            let url = source.url.trim();
            (!url.is_empty()).then(|| url.to_string())
        })
    }

    pub fn exact_lookup_url(&self) -> Option<String> {
        if self.pick_mode != PickMode::ExactTilePixel {
            return None;
        }
        let version = self.tileset_version.trim();
        let version = if version.is_empty() { "v1" } else { version };
        resolve_public_asset_url(
            Some(&format!(
                "/images/exact_lookup/{}.{}.bin",
                self.key, version
            )),
            normalize_public_base_url(None).as_deref(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FieldColorMode, LayerKind, LayerRegistry, LayerRuntime, LayerVectorStatus, PickMode,
    };
    use crate::map::spaces::layer_transform::LayerTransform;
    use fishystuff_api::models::layers::{
        FieldColorMode as FieldColorModeDto, FieldSourceRef, GeometrySpace, LayerDescriptor,
        LayerKind as LayerKindDto, LayerTransformDto, LayerUiInfo, LayersResponse, LodPolicyDto,
        StyleMode, TilesetRef, VectorSourceRef,
    };

    fn vector_descriptor(with_source: bool) -> LayerDescriptor {
        LayerDescriptor {
            layer_id: "region_groups".to_string(),
            name: "Region Groups".to_string(),
            enabled: true,
            kind: LayerKindDto::VectorGeoJson,
            transform: LayerTransformDto::IdentityMapSpace,
            tileset: TilesetRef::default(),
            tile_px: 512,
            max_level: 0,
            y_flip: false,
            field_source: None,
            field_metadata_source: None,
            vector_source: with_source.then_some(VectorSourceRef {
                url: "/region_groups/v1.geojson".to_string(),
                revision: "rg-v1".to_string(),
                geometry_space: GeometrySpace::MapPixels,
                style_mode: StyleMode::FeaturePropertyPalette,
                feature_id_property: Some("id".to_string()),
                color_property: Some("c".to_string()),
            }),
            lod_policy: LodPolicyDto::default(),
            ui: LayerUiInfo {
                visible_default: true,
                opacity_default: 0.75,
                z_base: 30.0,
                display_order: 3,
            },
            request_weight: 1.0,
            pick_mode: "none".to_string(),
        }
    }

    #[test]
    fn vector_layer_without_source_is_dropped() {
        let mut registry = LayerRegistry::default();
        registry.apply_layers_response(LayersResponse {
            revision: "rev".to_string(),
            map_version_id: None,
            layers: vec![vector_descriptor(false)],
        });
        assert!(registry.ordered().is_empty());
    }

    #[test]
    fn opacity_and_visibility_updates_do_not_reset_vector_status() {
        let mut registry = LayerRegistry::default();
        registry.apply_layers_response(LayersResponse {
            revision: "rev".to_string(),
            map_version_id: None,
            layers: vec![vector_descriptor(true)],
        });
        let layer = registry.ordered().first().expect("vector layer").id;

        let mut runtime = LayerRuntime::default();
        runtime.sync_to_registry(&registry);
        runtime.get_mut(layer).expect("layer state").vector_status = LayerVectorStatus::Ready;

        runtime.set_opacity(layer, 0.42);
        runtime.set_visible(layer, false);

        let state = runtime.get(layer).expect("layer state");
        assert_eq!(state.vector_status, LayerVectorStatus::Ready);
        assert!(!state.visible);
        assert!((state.opacity - 0.42).abs() < f32::EPSILON);
    }

    #[test]
    fn vector_layer_uses_vector_render_kind() {
        let mut registry = LayerRegistry::default();
        registry.apply_layers_response(LayersResponse {
            revision: "rev".to_string(),
            map_version_id: None,
            layers: vec![vector_descriptor(true)],
        });
        let layer = registry.ordered().first().expect("layer");
        assert_eq!(layer.kind, LayerKind::VectorGeoJson);
        assert_eq!(layer.pick_mode, PickMode::None);
        assert!(matches!(layer.transform, LayerTransform::IdentityMapSpace));
    }

    #[test]
    fn legacy_static_asset_paths_are_normalized_when_layers_load() {
        let mut registry = LayerRegistry::default();
        registry.apply_layers_response(LayersResponse {
            revision: "rev".to_string(),
            map_version_id: None,
            layers: vec![LayerDescriptor {
                layer_id: "zone_mask".to_string(),
                name: "Zone Mask".to_string(),
                enabled: true,
                kind: LayerKindDto::TiledRaster,
                transform: LayerTransformDto::IdentityMapSpace,
                tileset: TilesetRef {
                    manifest_url: "/images/tiles/mask/v1/tileset.json".to_string(),
                    tile_url_template: "/tiles/mask/v1/{level}/{x}_{y}.png".to_string(),
                    version: "v1".to_string(),
                },
                tile_px: 512,
                max_level: 0,
                y_flip: false,
                field_source: None,
                field_metadata_source: None,
                vector_source: None,
                lod_policy: LodPolicyDto::default(),
                ui: LayerUiInfo::default(),
                request_weight: 1.0,
                pick_mode: "exact_tile_pixel".to_string(),
            }],
        });

        let layer = registry.ordered().first().expect("zone mask layer");
        assert_eq!(
            layer.tileset_url,
            "/images/tiles/zone_mask_visual/v1/tileset.json"
        );
        assert_eq!(
            layer.tile_url_template,
            "/images/tiles/zone_mask_visual/v1/{z}/{x}_{y}.png"
        );
        assert_eq!(layer.tile_px, 2048);
        assert_eq!(layer.max_level, 0);
        assert_eq!(
            layer.exact_lookup_url().as_deref(),
            Some("/images/exact_lookup/zone_mask.v1.bin")
        );
        assert_eq!(layer.field_color_mode(), Some(FieldColorMode::RgbU24));
    }

    #[test]
    fn minimap_visual_tiles_are_overridden_to_map_space_display_chunks() {
        let mut registry = LayerRegistry::default();
        registry.apply_layers_response(LayersResponse {
            revision: "rev".to_string(),
            map_version_id: None,
            layers: vec![LayerDescriptor {
                layer_id: "minimap".to_string(),
                name: "Minimap".to_string(),
                enabled: true,
                kind: LayerKindDto::TiledRaster,
                transform: LayerTransformDto::AffineToWorld {
                    a: 100.0,
                    b: 0.0,
                    tx: 0.0,
                    c: 0.0,
                    d: 100.0,
                    ty: 0.0,
                },
                tileset: TilesetRef {
                    manifest_url: "/images/tiles/minimap/v1/tileset.json".to_string(),
                    tile_url_template: "/images/tiles/minimap/v1/{level}/rader_{x}_{y}.png"
                        .to_string(),
                    version: "v1".to_string(),
                },
                tile_px: 128,
                max_level: 6,
                y_flip: true,
                field_source: None,
                field_metadata_source: None,
                vector_source: None,
                lod_policy: LodPolicyDto::default(),
                ui: LayerUiInfo::default(),
                request_weight: 1.0,
                pick_mode: "none".to_string(),
            }],
        });

        let layer = registry.ordered().first().expect("minimap layer");
        assert!(matches!(layer.transform, LayerTransform::IdentityMapSpace));
        assert_eq!(
            layer.tileset_url,
            "/images/tiles/minimap_visual/v1/tileset.json"
        );
        assert_eq!(
            layer.tile_url_template,
            "/images/tiles/minimap_visual/v1/{z}/{x}_{y}.png"
        );
        assert_eq!(layer.tile_px, 512);
        assert_eq!(layer.max_level, 2);
        assert!(!layer.y_flip);
        assert_eq!(layer.lod_policy.target_tiles, 16);
        assert_eq!(layer.lod_policy.hysteresis_hi, 24.0);
        assert_eq!(layer.lod_policy.hysteresis_lo, 8.0);
        assert_eq!(layer.lod_policy.margin_tiles, 1);
        assert_eq!(layer.lod_policy.max_resident_tiles, 128);
        assert_eq!(layer.lod_policy.pinned_coarse_levels, 0);
        assert_eq!(layer.lod_policy.warm_margin_tiles, 1);
        assert_eq!(layer.lod_policy.protected_margin_tiles, 1);
    }

    #[test]
    fn explicit_field_source_is_preferred_over_exact_lookup_convention() {
        let mut registry = LayerRegistry::default();
        registry.apply_layers_response(LayersResponse {
            revision: "rev".to_string(),
            map_version_id: None,
            layers: vec![LayerDescriptor {
                layer_id: "zone_mask".to_string(),
                name: "Zone Mask".to_string(),
                enabled: true,
                kind: LayerKindDto::TiledRaster,
                transform: LayerTransformDto::IdentityMapSpace,
                tileset: TilesetRef::default(),
                tile_px: 512,
                max_level: 0,
                y_flip: false,
                field_source: Some(FieldSourceRef {
                    url: "/fields/custom-zone-mask.bin".to_string(),
                    revision: "custom-v1".to_string(),
                    color_mode: FieldColorModeDto::DebugHash,
                }),
                field_metadata_source: None,
                vector_source: None,
                lod_policy: LodPolicyDto::default(),
                ui: LayerUiInfo::default(),
                request_weight: 1.0,
                pick_mode: "exact_tile_pixel".to_string(),
            }],
        });

        let layer = registry.ordered().first().expect("field-backed layer");
        assert_eq!(
            layer.field_url().as_deref(),
            Some("/fields/custom-zone-mask.bin")
        );
        assert_eq!(layer.field_revision(), Some("custom-v1"));
        assert_eq!(layer.field_color_mode(), Some(FieldColorMode::DebugHash));
        assert_eq!(
            layer.exact_lookup_url().as_deref(),
            Some("/images/exact_lookup/zone_mask.v1.bin")
        );
    }
}
