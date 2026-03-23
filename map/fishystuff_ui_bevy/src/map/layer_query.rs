use fishystuff_api::Rgb;
use fishystuff_core::field_metadata::{FieldHoverRow, FieldHoverTarget};

use crate::map::exact_lookup::ExactLookupCache;
use crate::map::field_metadata::FieldMetadataCache;
use crate::map::field_semantics::{loaded_semantic_field_layer, SemanticFieldLayerView};
use crate::map::field_view::{loaded_field_layer, FieldLayerView};
use crate::map::layers::LayerSpec;
use crate::map::raster::{map_version_id, RasterTileCache, TileKey};
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::WorldPoint;
use crate::plugins::vector_layers::VectorLayerRuntime;

#[derive(Debug, Clone, PartialEq)]
pub struct LayerQuerySample {
    pub layer_id: String,
    pub layer_name: String,
    pub kind: String,
    pub rgb: Rgb,
    pub rgb_u32: u32,
    pub field_id: Option<u32>,
    pub rows: Vec<FieldHoverRow>,
    pub targets: Vec<FieldHoverTarget>,
}

pub struct LayerSamplingContext<'a> {
    pub exact_lookups: &'a ExactLookupCache,
    pub field_metadata: &'a FieldMetadataCache,
    pub tile_cache: &'a RasterTileCache,
    pub vector_runtime: &'a VectorLayerRuntime,
    pub world_point: WorldPoint,
    pub map_to_world: MapToWorld,
    pub map_version_id: Option<&'a str>,
}

pub fn sample_layers_at_world_point(
    layers: &[&LayerSpec],
    sampling: &LayerSamplingContext<'_>,
) -> Vec<LayerQuerySample> {
    layers
        .iter()
        .filter_map(|layer| sample_layer_at_world_point(layer, sampling))
        .collect()
}

pub fn sample_layer_at_world_point(
    layer: &LayerSpec,
    sampling: &LayerSamplingContext<'_>,
) -> Option<LayerQuerySample> {
    let (rgb, field_id, rows, targets, kind) = if layer.field_url().is_some() {
        let semantics =
            loaded_semantic_field_layer(layer, sampling.exact_lookups, sampling.field_metadata)?
                .semantic_sample_at_world_point(
                    layer,
                    sampling.map_to_world,
                    sampling.world_point,
                )?;
        (
            semantics.rgb,
            Some(semantics.field_id),
            semantics.rows,
            semantics.targets,
            "field".to_string(),
        )
    } else if layer.is_raster() {
        (
            sample_raster_layer_rgb(layer, sampling)?,
            None,
            Vec::new(),
            Vec::new(),
            "tiled-raster".to_string(),
        )
    } else if layer.is_vector() {
        (
            sample_vector_layer_rgb(layer, sampling)?,
            None,
            Vec::new(),
            Vec::new(),
            "vector-geojson".to_string(),
        )
    } else {
        return None;
    };
    Some(LayerQuerySample {
        layer_id: layer.key.clone(),
        layer_name: layer.name.clone(),
        kind,
        rgb,
        rgb_u32: rgb.to_u32(),
        field_id,
        rows,
        targets,
    })
}

fn sample_raster_layer_rgb(layer: &LayerSpec, sampling: &LayerSamplingContext<'_>) -> Option<Rgb> {
    if let Some(field) = loaded_field_layer(layer, sampling.exact_lookups) {
        return field.rgb_at_world_point(layer, sampling.map_to_world, sampling.world_point);
    }

    let world_transform = layer.world_transform(sampling.map_to_world)?;
    let layer_px = world_transform.world_to_layer(sampling.world_point);
    if layer_px.x < 0.0 || layer_px.y < 0.0 {
        return None;
    }
    let map_version = if layer.tile_url_template.contains("{map_version}") {
        sampling.map_version_id
    } else {
        None
    };
    if layer.tile_url_template.contains("{map_version}") && map_version.is_none() {
        return None;
    }
    let tile_px = layer.tile_px.max(1);
    let layer_ix = layer_px.x.floor() as u32;
    let layer_iy = layer_px.y.floor() as u32;
    let tx = layer_ix / tile_px;
    let ty = layer_iy / tile_px;
    let key = TileKey {
        layer: layer.id,
        map_version: map_version.map(map_version_id).unwrap_or(0),
        z: 0,
        tx: tx as i32,
        ty: ty as i32,
    };
    let tile = sampling.tile_cache.get_ready_pixel_data(&key)?;
    let local_x = layer_ix - tx * tile_px;
    let local_y = layer_iy - ty * tile_px;
    if local_x >= tile.width || local_y >= tile.height {
        return None;
    }
    let idx = ((local_y * tile.width + local_x) * 4) as usize;
    if idx + 3 >= tile.data.len() || tile.data[idx + 3] == 0 {
        return None;
    }
    Some(Rgb::new(
        tile.data[idx],
        tile.data[idx + 1],
        tile.data[idx + 2],
    ))
}

fn sample_vector_layer_rgb(layer: &LayerSpec, sampling: &LayerSamplingContext<'_>) -> Option<Rgb> {
    let source = layer.vector_source.as_ref()?;
    let revision = resolved_vector_revision(source, sampling.map_version_id);
    let bundle = sampling
        .vector_runtime
        .finished
        .get_ref(&(layer.id, revision))?;
    let rgba = bundle.sample_rgb(sampling.world_point.x as f32, sampling.world_point.z as f32)?;
    Some(Rgb::new(rgba[0], rgba[1], rgba[2]))
}

fn resolved_vector_revision(
    source: &crate::map::layers::VectorSourceSpec,
    map_version_id: Option<&str>,
) -> String {
    let mut url = source.url.clone();
    if url.contains("{map_version}") {
        let version = map_version_id
            .filter(|value| !value.trim().is_empty() && *value != "0v0")
            .unwrap_or("v1");
        url = url.replace("{map_version}", version);
    }
    let revision = source.revision.trim();
    if revision.is_empty() {
        format!("url:{url}")
    } else {
        revision.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{sample_layer_at_world_point, LayerSamplingContext};
    use crate::map::exact_lookup::ExactLookupCache;
    use crate::map::field_metadata::FieldMetadataCache;
    use crate::map::layers::LayerRegistry;
    use crate::map::raster::RasterTileCache;
    use crate::map::spaces::affine::Affine2D;
    use crate::map::spaces::layer_transform::LayerTransform;
    use crate::map::spaces::world::MapToWorld;
    use crate::map::spaces::WorldPoint;
    use crate::plugins::vector_layers::VectorLayerRuntime;
    use fishystuff_core::field::DiscreteFieldRows;
    use fishystuff_core::field_metadata::{
        FieldHoverMetadataAsset, FieldHoverMetadataEntry, FieldHoverRow,
    };

    fn field_layer_descriptor(
        layer_id: &str,
        transform: fishystuff_api::models::layers::LayerTransformDto,
    ) -> fishystuff_api::models::layers::LayerDescriptor {
        fishystuff_api::models::layers::LayerDescriptor {
            layer_id: layer_id.to_string(),
            name: layer_id.to_string(),
            enabled: true,
            kind: fishystuff_api::models::layers::LayerKind::TiledRaster,
            transform,
            tileset: fishystuff_api::models::layers::TilesetRef::default(),
            tile_px: 512,
            max_level: 0,
            y_flip: false,
            field_source: Some(fishystuff_api::models::layers::FieldSourceRef {
                url: format!("/fields/{layer_id}.v1.bin"),
                revision: format!("{layer_id}-field-v1"),
                color_mode: fishystuff_api::models::layers::FieldColorMode::DebugHash,
            }),
            field_metadata_source: Some(fishystuff_api::models::layers::FieldMetadataSourceRef {
                url: format!("/fields/{layer_id}.v1.meta.json"),
                revision: format!("{layer_id}-meta-v1"),
            }),
            vector_source: None,
            lod_policy: fishystuff_api::models::layers::LodPolicyDto::default(),
            ui: fishystuff_api::models::layers::LayerUiInfo::default(),
            request_weight: 1.0,
            pick_mode: "none".to_string(),
        }
    }

    fn transformed_registry(transform: LayerTransform) -> LayerRegistry {
        let mut registry = LayerRegistry::default();
        let transform = match transform {
            LayerTransform::IdentityMapSpace => {
                fishystuff_api::models::layers::LayerTransformDto::IdentityMapSpace
            }
            LayerTransform::AffineToMap(affine) => {
                fishystuff_api::models::layers::LayerTransformDto::AffineToMap {
                    a: affine.a,
                    b: affine.b,
                    tx: affine.tx,
                    c: affine.c,
                    d: affine.d,
                    ty: affine.ty,
                }
            }
            LayerTransform::AffineToWorld(affine) => {
                fishystuff_api::models::layers::LayerTransformDto::AffineToWorld {
                    a: affine.a,
                    b: affine.b,
                    tx: affine.tx,
                    c: affine.c,
                    d: affine.d,
                    ty: affine.ty,
                }
            }
        };
        registry.apply_layers_response(fishystuff_api::models::layers::LayersResponse {
            revision: "rev".to_string(),
            map_version_id: None,
            layers: vec![field_layer_descriptor("regions", transform)],
        });
        registry
    }

    #[test]
    fn sample_layer_at_world_point_returns_field_semantics_for_transformed_layer() {
        let registry = transformed_registry(LayerTransform::AffineToWorld(Affine2D::IDENTITY));
        let layer = registry.get_by_key("regions").expect("regions layer");
        let mut exact_lookups = ExactLookupCache::default();
        exact_lookups.insert_ready(
            layer.id,
            "/fields/regions.v1.bin".to_string(),
            DiscreteFieldRows::from_u32_grid(2, 2, &[0, 76, 11, 13]).expect("field"),
        );
        let mut field_metadata = FieldMetadataCache::default();
        field_metadata.insert_ready(
            layer.id,
            "/fields/regions.v1.meta.json".to_string(),
            FieldHoverMetadataAsset {
                entries: std::collections::BTreeMap::from([(
                    76,
                    FieldHoverMetadataEntry {
                        rows: vec![FieldHoverRow {
                            key: "origin".to_string(),
                            icon: "hover-origin".to_string(),
                            label: "Origin".to_string(),
                            value: "Tarif".to_string(),
                            hide_label: false,
                            status_icon: None,
                            status_icon_tone: None,
                        }],
                        targets: Vec::new(),
                    },
                )]),
            },
        );

        let sample = sample_layer_at_world_point(
            layer,
            &LayerSamplingContext {
                exact_lookups: &exact_lookups,
                field_metadata: &field_metadata,
                tile_cache: &RasterTileCache::default(),
                vector_runtime: &VectorLayerRuntime::default(),
                world_point: WorldPoint::new(1.25, 0.25),
                map_to_world: MapToWorld::default(),
                map_version_id: None,
            },
        )
        .expect("sample");

        assert_eq!(sample.layer_id, "regions");
        assert_eq!(sample.layer_name, "regions");
        assert_eq!(sample.kind, "field");
        assert_eq!(sample.field_id, Some(76));
        assert_eq!(sample.rgb_u32, sample.rgb.to_u32());
        assert_eq!(
            sample.rows,
            vec![FieldHoverRow {
                key: "origin".to_string(),
                icon: "hover-origin".to_string(),
                label: "Origin".to_string(),
                value: "Tarif".to_string(),
                hide_label: false,
                status_icon: None,
                status_icon_tone: None,
            }]
        );
        assert!(sample.targets.is_empty());
    }
}
