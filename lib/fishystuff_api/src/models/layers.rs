use serde::{Deserialize, Serialize};

use crate::ids::MapVersionId;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LayersResponse {
    #[serde(default)]
    pub revision: String,
    pub map_version_id: Option<MapVersionId>,
    #[serde(default)]
    pub layers: Vec<LayerDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerDescriptor {
    pub layer_id: String,
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub kind: LayerKind,
    pub transform: LayerTransformDto,
    pub tileset: TilesetRef,
    pub tile_px: u32,
    pub max_level: u8,
    #[serde(default)]
    pub y_flip: bool,
    #[serde(default)]
    pub field_source: Option<FieldSourceRef>,
    #[serde(default)]
    pub field_metadata_source: Option<FieldMetadataSourceRef>,
    #[serde(default)]
    pub vector_source: Option<VectorSourceRef>,
    #[serde(default)]
    pub lod_policy: LodPolicyDto,
    #[serde(default)]
    pub ui: LayerUiInfo,
    #[serde(default = "default_request_weight")]
    pub request_weight: f32,
    #[serde(default = "default_pick_mode")]
    pub pick_mode: String,
}

impl Default for LayerDescriptor {
    fn default() -> Self {
        Self {
            layer_id: String::new(),
            name: String::new(),
            enabled: true,
            kind: LayerKind::default(),
            transform: LayerTransformDto::IdentityMapSpace,
            tileset: TilesetRef::default(),
            tile_px: 512,
            max_level: 0,
            y_flip: false,
            field_source: None,
            field_metadata_source: None,
            vector_source: None,
            lod_policy: LodPolicyDto::default(),
            ui: LayerUiInfo::default(),
            request_weight: default_request_weight(),
            pick_mode: default_pick_mode(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LayerKind {
    /// Static or dynamic tile pyramid rendered by the raster tile streamer.
    #[default]
    TiledRaster,
    /// Static GeoJSON overlay rendered via incremental triangulation.
    #[serde(rename = "vector_geojson")]
    VectorGeoJson,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GeometrySpace {
    /// GeoJSON coordinates are canonical map pixels and must be projected via `MapToWorld`.
    #[default]
    MapPixels,
    /// GeoJSON coordinates are already in WORLD coordinates (`x=world_x`, `y=world_z`).
    World,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StyleMode {
    /// Style features by inspecting a per-feature property (for example `c` for RGB arrays).
    #[default]
    FeaturePropertyPalette,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FieldColorMode {
    /// Interpret field ids as `0xRRGGBB` and render them directly.
    #[default]
    RgbU24,
    /// Render ids with a deterministic debug palette derived from the integer id.
    DebugHash,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FieldSourceRef {
    /// URL for the compact field asset.
    pub url: String,
    /// Revision identifier used to invalidate cached field state.
    #[serde(default)]
    pub revision: String,
    /// Declares how field ids should be visualized when a direct texture is requested.
    #[serde(default)]
    pub color_mode: FieldColorMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FieldMetadataSourceRef {
    /// URL for the hover/semantic metadata asset associated with a field.
    pub url: String,
    /// Revision identifier used to invalidate cached metadata state.
    #[serde(default)]
    pub revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorSourceRef {
    /// URL for the GeoJSON asset. This must be resolvable by the frontend.
    pub url: String,
    /// Revision identifier used to invalidate cached meshes (for example commit/hash/version).
    #[serde(default)]
    pub revision: String,
    /// Declares whether `coordinates` are map pixels or world coordinates.
    #[serde(default)]
    pub geometry_space: GeometrySpace,
    /// Declares how feature styling should be derived.
    #[serde(default)]
    pub style_mode: StyleMode,
    /// Optional feature identifier property name.
    #[serde(default)]
    pub feature_id_property: Option<String>,
    /// Optional feature color property name.
    #[serde(default)]
    pub color_property: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LayerTransformDto {
    #[default]
    IdentityMapSpace,
    AffineToMap {
        a: f64,
        b: f64,
        tx: f64,
        c: f64,
        d: f64,
        ty: f64,
    },
    AffineToWorld {
        a: f64,
        b: f64,
        tx: f64,
        c: f64,
        d: f64,
        ty: f64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TilesetRef {
    pub manifest_url: String,
    pub tile_url_template: String,
    #[serde(default)]
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LodPolicyDto {
    pub target_tiles: usize,
    pub hysteresis_hi: f32,
    pub hysteresis_lo: f32,
    pub margin_tiles: i32,
    pub enable_refine: bool,
    pub refine_debounce_ms: u32,
    pub max_detail_tiles: usize,
    #[serde(default = "default_max_resident_tiles")]
    pub max_resident_tiles: usize,
    #[serde(default = "default_pinned_coarse_levels")]
    pub pinned_coarse_levels: u8,
    #[serde(default = "default_coarse_pin_min_level")]
    pub coarse_pin_min_level: Option<i32>,
    #[serde(default = "default_warm_margin_tiles")]
    pub warm_margin_tiles: i32,
    #[serde(default = "default_protected_margin_tiles")]
    pub protected_margin_tiles: i32,
    #[serde(default = "default_detail_eviction_weight")]
    pub detail_eviction_weight: f32,
    #[serde(default = "default_max_detail_requests_while_camera_moving")]
    pub max_detail_requests_while_camera_moving: usize,
    #[serde(default = "default_motion_suppresses_refine")]
    pub motion_suppresses_refine: bool,
}

impl Default for LodPolicyDto {
    fn default() -> Self {
        Self {
            target_tiles: 220,
            hysteresis_hi: 280.0,
            hysteresis_lo: 160.0,
            margin_tiles: 1,
            enable_refine: false,
            refine_debounce_ms: 0,
            max_detail_tiles: 0,
            max_resident_tiles: default_max_resident_tiles(),
            pinned_coarse_levels: default_pinned_coarse_levels(),
            coarse_pin_min_level: default_coarse_pin_min_level(),
            warm_margin_tiles: default_warm_margin_tiles(),
            protected_margin_tiles: default_protected_margin_tiles(),
            detail_eviction_weight: default_detail_eviction_weight(),
            max_detail_requests_while_camera_moving:
                default_max_detail_requests_while_camera_moving(),
            motion_suppresses_refine: default_motion_suppresses_refine(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerUiInfo {
    pub visible_default: bool,
    pub opacity_default: f32,
    pub z_base: f32,
    pub display_order: i32,
}

impl Default for LayerUiInfo {
    fn default() -> Self {
        Self {
            visible_default: true,
            opacity_default: 1.0,
            z_base: 0.0,
            display_order: 0,
        }
    }
}

const fn default_request_weight() -> f32 {
    1.0
}

fn default_pick_mode() -> String {
    "none".to_string()
}

const fn default_max_resident_tiles() -> usize {
    4096
}

const fn default_pinned_coarse_levels() -> u8 {
    2
}

const fn default_coarse_pin_min_level() -> Option<i32> {
    None
}

const fn default_warm_margin_tiles() -> i32 {
    3
}

const fn default_protected_margin_tiles() -> i32 {
    1
}

const fn default_detail_eviction_weight() -> f32 {
    4.0
}

const fn default_max_detail_requests_while_camera_moving() -> usize {
    2
}

const fn default_motion_suppresses_refine() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::{
        FieldColorMode, FieldMetadataSourceRef, FieldSourceRef, GeometrySpace, LayerDescriptor,
        LayerKind, LayerTransformDto, LayerUiInfo, LayersResponse, LodPolicyDto, StyleMode,
        TilesetRef, VectorSourceRef,
    };

    #[test]
    fn vector_layer_metadata_serializes_with_expected_shape() {
        let response = LayersResponse {
            revision: "rev-1".to_string(),
            map_version_id: None,
            layers: vec![LayerDescriptor {
                layer_id: "region_groups".to_string(),
                name: "Region Groups".to_string(),
                enabled: true,
                kind: LayerKind::VectorGeoJson,
                transform: LayerTransformDto::IdentityMapSpace,
                tileset: TilesetRef::default(),
                tile_px: 512,
                max_level: 0,
                y_flip: false,
                field_source: None,
                field_metadata_source: None,
                vector_source: Some(VectorSourceRef {
                    url: "/region_groups/v1.geojson".to_string(),
                    revision: "rg-v1".to_string(),
                    geometry_space: GeometrySpace::MapPixels,
                    style_mode: StyleMode::FeaturePropertyPalette,
                    feature_id_property: Some("id".to_string()),
                    color_property: Some("c".to_string()),
                }),
                lod_policy: LodPolicyDto::default(),
                ui: LayerUiInfo::default(),
                request_weight: 1.0,
                pick_mode: "none".to_string(),
            }],
        };

        let json = serde_json::to_value(response).expect("serialize");
        assert_eq!(json["layers"][0]["kind"], "vector_geojson");
        assert_eq!(
            json["layers"][0]["vector_source"]["geometry_space"],
            "map_pixels"
        );
        assert_eq!(
            json["layers"][0]["vector_source"]["style_mode"],
            "feature_property_palette"
        );
        assert_eq!(
            json["layers"][0]["vector_source"]["url"],
            "/region_groups/v1.geojson"
        );
    }

    #[test]
    fn field_source_serializes_with_color_mode() {
        let response = LayersResponse {
            revision: "rev-1".to_string(),
            map_version_id: None,
            layers: vec![LayerDescriptor {
                layer_id: "regions".to_string(),
                name: "Regions".to_string(),
                enabled: true,
                kind: LayerKind::TiledRaster,
                transform: LayerTransformDto::IdentityMapSpace,
                tileset: TilesetRef::default(),
                tile_px: 512,
                max_level: 0,
                y_flip: false,
                field_source: Some(FieldSourceRef {
                    url: "/fields/regions.v1.bin".to_string(),
                    revision: "regions-field-v1".to_string(),
                    color_mode: FieldColorMode::DebugHash,
                }),
                field_metadata_source: Some(FieldMetadataSourceRef {
                    url: "/fields/regions.v1.meta.json".to_string(),
                    revision: "regions-meta-v1".to_string(),
                }),
                vector_source: None,
                lod_policy: LodPolicyDto::default(),
                ui: LayerUiInfo::default(),
                request_weight: 1.0,
                pick_mode: "none".to_string(),
            }],
        };

        let json = serde_json::to_value(response).expect("serialize");
        assert_eq!(
            json["layers"][0]["field_source"]["url"],
            "/fields/regions.v1.bin"
        );
        assert_eq!(
            json["layers"][0]["field_source"]["color_mode"],
            "debug_hash"
        );
        assert_eq!(
            json["layers"][0]["field_metadata_source"]["url"],
            "/fields/regions.v1.meta.json"
        );
    }
}
