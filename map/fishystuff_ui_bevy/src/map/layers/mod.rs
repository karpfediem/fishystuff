mod registry;
mod runtime;

use crate::map::spaces::layer_transform::{LayerTransform, WorldTransform};
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::{LayerPoint, LayerRect};
use crate::public_assets::{normalize_public_base_url, resolve_public_asset_url};

pub use registry::LayerRegistry;
pub use runtime::{LayerManifestStatus, LayerRuntime, LayerRuntimeState, LayerSettings};

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerKind {
    TiledRaster,
    VectorGeoJson,
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

#[derive(Debug, Clone)]
pub struct VectorSourceSpec {
    pub url: String,
    pub revision: String,
    pub geometry_space: GeometrySpace,
    pub style_mode: StyleMode,
    pub feature_id_property: Option<String>,
    pub color_property: Option<String>,
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
    pub transform: LayerTransform,
    pub tile_px: u32,
    pub max_level: u8,
    pub y_flip: bool,
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

    pub fn streams_raster_tiles(&self) -> bool {
        self.is_raster() && self.static_image_url().is_none()
    }

    pub fn is_vector(&self) -> bool {
        self.kind == LayerKind::VectorGeoJson
    }

    pub fn static_image_url(&self) -> Option<String> {
        if self.kind == LayerKind::TiledRaster
            && self.pick_mode == PickMode::ExactTilePixel
            && self.key == "zone_mask"
        {
            resolve_public_asset_url(
                Some("/images/zones_mask_v1.png"),
                normalize_public_base_url(None).as_deref(),
            )
        } else {
            None
        }
    }

    pub fn static_layer_bounds(&self, map_to_world: MapToWorld) -> Option<LayerRect> {
        let _ = self.static_image_url()?;
        match self.transform {
            LayerTransform::IdentityMapSpace | LayerTransform::AffineToMap(_) => {
                let world_transform = self.world_transform(map_to_world)?;
                let corners = map_to_world
                    .map_bounds()
                    .corners()
                    .map(|corner| world_transform.map_to_layer(corner));
                let mut min_x = f64::INFINITY;
                let mut min_y = f64::INFINITY;
                let mut max_x = f64::NEG_INFINITY;
                let mut max_y = f64::NEG_INFINITY;
                for corner in corners {
                    min_x = min_x.min(corner.x);
                    min_y = min_y.min(corner.y);
                    max_x = max_x.max(corner.x);
                    max_y = max_y.max(corner.y);
                }
                Some(LayerRect {
                    min: LayerPoint::new(min_x, min_y),
                    max: LayerPoint::new(max_x, max_y),
                })
            }
            LayerTransform::AffineToWorld(_) => None,
        }
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
    use super::{LayerKind, LayerRegistry, LayerRuntime, LayerVectorStatus, PickMode};
    use crate::map::spaces::layer_transform::LayerTransform;
    use fishystuff_api::models::layers::{
        GeometrySpace, LayerDescriptor, LayerKind as LayerKindDto, LayerTransformDto, LayerUiInfo,
        LayersResponse, LodPolicyDto, StyleMode, TilesetRef, VectorSourceRef,
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
                vector_source: None,
                lod_policy: LodPolicyDto::default(),
                ui: LayerUiInfo::default(),
                request_weight: 1.0,
                pick_mode: "exact_tile_pixel".to_string(),
            }],
        });

        let layer = registry.ordered().first().expect("zone mask layer");
        assert_eq!(layer.tileset_url, "/images/tiles/mask/v1/tileset.json");
        assert_eq!(
            layer.tile_url_template,
            "/images/tiles/mask/v1/{level}/{x}_{y}.png"
        );
        assert_eq!(
            layer.exact_lookup_url().as_deref(),
            Some("/images/exact_lookup/zone_mask.v1.bin")
        );
        assert_eq!(
            layer.static_image_url().as_deref(),
            Some("/images/zones_mask_v1.png")
        );
        assert!(!layer.streams_raster_tiles());
    }
}
