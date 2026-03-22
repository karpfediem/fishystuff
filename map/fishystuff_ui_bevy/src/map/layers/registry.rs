use std::collections::HashMap;

use crate::public_assets::{normalize_public_base_url, resolve_public_asset_url};
use bevy::prelude::Resource;
use fishystuff_api::models::layers::{
    GeometrySpace as GeometrySpaceDto, LayerDescriptor, LayerKind as LayerKindDto,
    LayerTransformDto, LayersResponse, StyleMode as StyleModeDto,
    VectorSourceRef as VectorSourceRefDto,
};
use fishystuff_core::asset_urls::normalize_site_asset_path;

use crate::map::spaces::affine::Affine2D;
use crate::map::spaces::layer_transform::{LayerTransform, WorldTransform};
use crate::map::spaces::world::MapToWorld;

use super::{
    GeometrySpace, LayerId, LayerKind, LayerSpec, LodPolicy, PickMode, StyleMode, VectorSourceSpec,
};

const ZONE_MASK_VISUAL_TILE_PX: u32 = 2048;
const MINIMAP_VISUAL_TILE_PX: u32 = 512;
const MINIMAP_VISUAL_MAX_LEVEL: u8 = 3;
const MINIMAP_TARGET_TILES: usize = 16;
const MINIMAP_HYSTERESIS_HI: f32 = 24.0;
const MINIMAP_HYSTERESIS_LO: f32 = 8.0;
const MINIMAP_MAX_RESIDENT_TILES: usize = 64;

#[derive(Resource, Debug, Clone, Default)]
pub struct LayerRegistry {
    ordered: Vec<LayerSpec>,
    index: HashMap<LayerId, usize>,
    id_by_key: HashMap<String, LayerId>,
    revision: Option<String>,
    map_version_id: Option<String>,
}

impl LayerRegistry {
    pub fn ordered(&self) -> &[LayerSpec] {
        &self.ordered
    }

    pub fn get(&self, id: LayerId) -> Option<&LayerSpec> {
        self.index.get(&id).map(|idx| &self.ordered[*idx])
    }

    pub fn id_by_key(&self, key: &str) -> Option<LayerId> {
        self.id_by_key.get(key).copied()
    }

    pub fn get_by_key(&self, key: &str) -> Option<&LayerSpec> {
        self.id_by_key(key).and_then(|id| self.get(id))
    }

    pub fn first_id_by_pick_mode(&self, mode: PickMode) -> Option<LayerId> {
        self.ordered
            .iter()
            .find(|layer| layer.pick_mode == mode)
            .map(|layer| layer.id)
    }

    pub fn label(&self, id: LayerId) -> &str {
        self.get(id)
            .map(|layer| layer.name.as_str())
            .unwrap_or("Layer")
    }

    pub fn revision(&self) -> Option<&str> {
        self.revision.as_deref()
    }

    pub fn map_version_id(&self) -> Option<&str> {
        self.map_version_id.as_deref()
    }

    pub fn apply_layers_response(&mut self, response: LayersResponse) {
        let mut descriptors = response.layers;
        descriptors.sort_by(|lhs, rhs| {
            lhs.ui
                .display_order
                .cmp(&rhs.ui.display_order)
                .then_with(|| lhs.layer_id.cmp(&rhs.layer_id))
        });

        let mut ordered = Vec::with_capacity(descriptors.len());
        let mut index = HashMap::new();
        let mut id_by_key = HashMap::new();

        for (idx, descriptor) in descriptors.into_iter().enumerate() {
            if !descriptor.enabled {
                continue;
            }
            let Ok(raw_id) = u16::try_from(idx) else {
                break;
            };
            let id = LayerId::from_raw(raw_id);
            let Some(spec) = layer_spec_from_descriptor(id, descriptor) else {
                continue;
            };
            id_by_key.insert(spec.key.clone(), id);
            index.insert(id, ordered.len());
            ordered.push(spec);
        }

        self.ordered = ordered;
        self.index = index;
        self.id_by_key = id_by_key;
        self.revision = Some(response.revision);
        self.map_version_id = response.map_version_id.map(|id| id.0);
    }

    pub fn set_transform(&mut self, id: LayerId, transform: LayerTransform) -> bool {
        let Some(idx) = self.index.get(&id).copied() else {
            return false;
        };
        if WorldTransform::new(transform, MapToWorld::default()).is_none() {
            bevy::log::warn!("ignoring non-invertible layer transform for {:?}", id);
            return false;
        }
        let same = self.ordered[idx].transform == transform;
        if same {
            return false;
        }
        self.ordered[idx].transform = transform;
        true
    }

    pub fn set_affine_transform(&mut self, id: LayerId, affine: Affine2D) -> bool {
        self.set_transform(id, LayerTransform::AffineToMap(affine))
    }
}

fn layer_spec_from_descriptor(id: LayerId, descriptor: LayerDescriptor) -> Option<LayerSpec> {
    let LayerDescriptor {
        layer_id,
        name,
        enabled: _,
        kind,
        transform,
        tileset,
        tile_px,
        max_level,
        y_flip,
        vector_source,
        lod_policy,
        ui,
        request_weight,
        pick_mode,
    } = descriptor;

    let mut transform = layer_transform_from_dto(transform);
    if WorldTransform::new(transform, MapToWorld::default()).is_none() {
        bevy::log::warn!(
            "dropping layer {} because transform is non-invertible",
            layer_id
        );
        return None;
    }

    let kind = layer_kind_from_dto(kind);
    let pick_mode = parse_pick_mode(&pick_mode);
    let vector_source = vector_source
        .and_then(vector_source_from_dto)
        .filter(|_| kind == LayerKind::VectorGeoJson);

    if kind == LayerKind::VectorGeoJson && vector_source.is_none() {
        bevy::log::warn!(
            "dropping layer {} because vector_source is missing",
            layer_id
        );
        return None;
    }

    let mut tileset_url = normalize_site_asset_path(&tileset.manifest_url);
    let mut tile_url_template = normalize_site_asset_path(&tileset.tile_url_template);
    let mut tile_px = tile_px.max(1);
    let mut max_level = max_level;
    let mut y_flip = y_flip;
    let mut lod_policy = LodPolicy {
        target_tiles: lod_policy.target_tiles.max(1),
        hysteresis_hi: lod_policy.hysteresis_hi.max(1.0),
        hysteresis_lo: lod_policy.hysteresis_lo.max(0.0),
        margin_tiles: lod_policy.margin_tiles,
        enable_refine: lod_policy.enable_refine,
        refine_debounce_ms: lod_policy.refine_debounce_ms,
        max_detail_tiles: lod_policy.max_detail_tiles,
        max_resident_tiles: lod_policy.max_resident_tiles.max(128),
        pinned_coarse_levels: lod_policy.pinned_coarse_levels,
        coarse_pin_min_level: lod_policy.coarse_pin_min_level,
        warm_margin_tiles: lod_policy.warm_margin_tiles.max(0),
        protected_margin_tiles: lod_policy.protected_margin_tiles.max(0),
        detail_eviction_weight: lod_policy.detail_eviction_weight.max(0.1),
        max_detail_requests_while_camera_moving: lod_policy
            .max_detail_requests_while_camera_moving
            .max(1),
        motion_suppresses_refine: lod_policy.motion_suppresses_refine,
    };
    if kind == LayerKind::TiledRaster
        && pick_mode == PickMode::ExactTilePixel
        && layer_id == "zone_mask"
    {
        let public_base = normalize_public_base_url(None);
        tileset_url = resolve_public_asset_url(
            Some("/images/tiles/zone_mask_visual/v1/tileset.json"),
            public_base.as_deref(),
        )
        .unwrap_or_else(|| "/images/tiles/zone_mask_visual/v1/tileset.json".to_string());
        tile_url_template = resolve_public_asset_url(
            Some("/images/tiles/zone_mask_visual/v1/{z}/{x}_{y}.png"),
            public_base.as_deref(),
        )
        .unwrap_or_else(|| "/images/tiles/zone_mask_visual/v1/{z}/{x}_{y}.png".to_string());
        tile_px = ZONE_MASK_VISUAL_TILE_PX;
        max_level = 0;
    }
    if kind == LayerKind::TiledRaster && layer_id == "minimap" {
        let public_base = normalize_public_base_url(None);
        tileset_url = resolve_public_asset_url(
            Some("/images/tiles/minimap_visual/v1/tileset.json"),
            public_base.as_deref(),
        )
        .unwrap_or_else(|| "/images/tiles/minimap_visual/v1/tileset.json".to_string());
        tile_url_template = resolve_public_asset_url(
            Some("/images/tiles/minimap_visual/v1/{z}/{x}_{y}.png"),
            public_base.as_deref(),
        )
        .unwrap_or_else(|| "/images/tiles/minimap_visual/v1/{z}/{x}_{y}.png".to_string());
        transform = LayerTransform::IdentityMapSpace;
        tile_px = MINIMAP_VISUAL_TILE_PX;
        max_level = MINIMAP_VISUAL_MAX_LEVEL;
        y_flip = false;
        lod_policy.target_tiles = MINIMAP_TARGET_TILES;
        lod_policy.hysteresis_hi = MINIMAP_HYSTERESIS_HI;
        lod_policy.hysteresis_lo = MINIMAP_HYSTERESIS_LO;
        lod_policy.margin_tiles = 0;
        lod_policy.enable_refine = false;
        lod_policy.refine_debounce_ms = 0;
        lod_policy.max_detail_tiles = 0;
        lod_policy.max_resident_tiles = MINIMAP_MAX_RESIDENT_TILES;
        lod_policy.pinned_coarse_levels = 0;
        lod_policy.coarse_pin_min_level = None;
        lod_policy.warm_margin_tiles = 0;
        lod_policy.protected_margin_tiles = 0;
        lod_policy.max_detail_requests_while_camera_moving = 1;
        lod_policy.motion_suppresses_refine = true;
    }

    Some(LayerSpec {
        id,
        key: layer_id,
        name,
        visible_default: ui.visible_default,
        opacity_default: ui.opacity_default.clamp(0.0, 1.0),
        z_base: ui.z_base,
        kind,
        tileset_url,
        tile_url_template,
        tileset_version: tileset.version,
        vector_source,
        transform,
        tile_px,
        max_level,
        y_flip,
        lod_policy,
        request_weight: request_weight.max(0.05),
        pick_mode,
        display_order: ui.display_order,
    })
}

fn layer_kind_from_dto(kind: LayerKindDto) -> LayerKind {
    match kind {
        LayerKindDto::TiledRaster => LayerKind::TiledRaster,
        LayerKindDto::VectorGeoJson => LayerKind::VectorGeoJson,
    }
}

fn geometry_space_from_dto(space: GeometrySpaceDto) -> GeometrySpace {
    match space {
        GeometrySpaceDto::MapPixels => GeometrySpace::MapPixels,
        GeometrySpaceDto::World => GeometrySpace::World,
    }
}

fn style_mode_from_dto(mode: StyleModeDto) -> StyleMode {
    match mode {
        StyleModeDto::FeaturePropertyPalette => StyleMode::FeaturePropertyPalette,
    }
}

fn vector_source_from_dto(dto: VectorSourceRefDto) -> Option<VectorSourceSpec> {
    let url = normalize_site_asset_path(&dto.url);
    if url.is_empty() {
        return None;
    }
    Some(VectorSourceSpec {
        url,
        revision: dto.revision.trim().to_string(),
        geometry_space: geometry_space_from_dto(dto.geometry_space),
        style_mode: style_mode_from_dto(dto.style_mode),
        feature_id_property: dto
            .feature_id_property
            .and_then(|value| (!value.trim().is_empty()).then_some(value)),
        color_property: dto
            .color_property
            .and_then(|value| (!value.trim().is_empty()).then_some(value)),
    })
}

fn layer_transform_from_dto(dto: LayerTransformDto) -> LayerTransform {
    match dto {
        LayerTransformDto::IdentityMapSpace => LayerTransform::IdentityMapSpace,
        LayerTransformDto::AffineToMap { a, b, tx, c, d, ty } => {
            LayerTransform::AffineToMap(Affine2D::new(a, b, tx, c, d, ty))
        }
        LayerTransformDto::AffineToWorld { a, b, tx, c, d, ty } => {
            LayerTransform::AffineToWorld(Affine2D::new(a, b, tx, c, d, ty))
        }
    }
}

fn parse_pick_mode(value: &str) -> PickMode {
    if value.eq_ignore_ascii_case("exact_tile_pixel") {
        PickMode::ExactTilePixel
    } else {
        PickMode::None
    }
}
