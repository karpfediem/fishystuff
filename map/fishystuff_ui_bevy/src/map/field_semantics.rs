use fishystuff_api::Rgb;
use fishystuff_core::field_metadata::{
    detail_fact_value, FieldDetailPaneRef, FieldDetailSection, FieldHoverMetadataAsset,
    FieldHoverMetadataEntry, FieldHoverTarget,
};

use crate::map::exact_lookup::ExactLookupCache;
use crate::map::field_metadata::FieldMetadataCache;
use crate::map::field_view::{loaded_field_layer, FieldLayerView, LoadedFieldLayer};
use crate::map::layers::{LayerRegistry, LayerSpec};
use crate::map::spaces::layer_transform::WorldTransform;
use crate::map::spaces::world::MapToWorld;
use crate::map::spaces::{LayerPoint, WorldPoint};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FieldSemanticSample {
    pub field_id: u32,
    pub rgb: Rgb,
    pub rgb_u32: u32,
    pub targets: Vec<FieldHoverTarget>,
    pub detail_pane: Option<FieldDetailPaneRef>,
    pub detail_sections: Vec<FieldDetailSection>,
}

impl FieldSemanticSample {
    pub fn fact_value(&self, key: &str) -> Option<&str> {
        detail_fact_value(
            self.detail_sections
                .iter()
                .flat_map(|section| section.facts.iter()),
            key,
        )
    }
}

pub trait SemanticFieldLayerView: FieldLayerView {
    fn metadata_entry_for_field_id(&self, field_id: u32) -> Option<&FieldHoverMetadataEntry>;

    fn semantic_sample_at_layer_point(
        &self,
        layer_point: LayerPoint,
    ) -> Option<FieldSemanticSample> {
        let field_id = self.field_id_at_layer_point(layer_point)?;
        let rgb = self.rgb_at_layer_point(layer_point)?;
        let (targets, detail_pane, detail_sections) = self
            .metadata_entry_for_field_id(field_id)
            .map(|entry| {
                (
                    entry.targets.clone(),
                    entry.detail_pane.clone(),
                    entry.detail_sections.clone(),
                )
            })
            .unwrap_or_else(|| (Vec::new(), None, Vec::new()));
        Some(FieldSemanticSample {
            field_id,
            rgb,
            rgb_u32: rgb.to_u32(),
            targets,
            detail_pane,
            detail_sections,
        })
    }

    fn semantic_sample_at_map_px(
        &self,
        map_px_x: i32,
        map_px_y: i32,
    ) -> Option<FieldSemanticSample> {
        self.semantic_sample_at_layer_point(LayerPoint::new(map_px_x as f64, map_px_y as f64))
    }

    fn semantic_sample_at_world_point(
        &self,
        layer: &LayerSpec,
        map_to_world: MapToWorld,
        world_point: WorldPoint,
    ) -> Option<FieldSemanticSample> {
        let world_transform = layer.world_transform(map_to_world)?;
        self.semantic_sample_at_world_point_with_transform(world_transform, world_point)
    }

    fn semantic_sample_at_world_point_with_transform(
        &self,
        world_transform: WorldTransform,
        world_point: WorldPoint,
    ) -> Option<FieldSemanticSample> {
        self.semantic_sample_at_layer_point(world_transform.world_to_layer(world_point))
    }

    fn fact_value_at_map_px(&self, map_px_x: i32, map_px_y: i32, key: &str) -> Option<String> {
        self.semantic_sample_at_map_px(map_px_x, map_px_y)?
            .fact_value(key)
            .map(ToOwned::to_owned)
    }

    fn fact_value_at_world_point(
        &self,
        layer: &LayerSpec,
        map_to_world: MapToWorld,
        world_point: WorldPoint,
        key: &str,
    ) -> Option<String> {
        self.semantic_sample_at_world_point(layer, map_to_world, world_point)?
            .fact_value(key)
            .map(ToOwned::to_owned)
    }

    fn fact_value_for_field_id(&self, field_id: u32, key: &str) -> Option<String> {
        let value = self
            .metadata_entry_for_field_id(field_id)?
            .fact_value(key)?
            .trim();
        (!value.is_empty()).then(|| value.to_string())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LoadedSemanticFieldLayer<'a> {
    field: LoadedFieldLayer<'a>,
    metadata: Option<&'a FieldHoverMetadataAsset>,
}

pub fn ordered_semantic_layers<'a>(layer_registry: &'a LayerRegistry) -> Vec<&'a LayerSpec> {
    layer_registry
        .ordered()
        .iter()
        .filter(|layer| {
            layer.key != "minimap"
                && layer.field_url().is_some()
                && layer.field_metadata_url().is_some()
        })
        .collect()
}

pub fn loaded_semantic_field_layer<'a>(
    layer: &'a LayerSpec,
    exact_lookups: &'a ExactLookupCache,
    field_metadata: &'a FieldMetadataCache,
) -> Option<LoadedSemanticFieldLayer<'a>> {
    let field = loaded_field_layer(layer, exact_lookups)?;
    let metadata = layer
        .field_metadata_url()
        .and_then(|url| field_metadata.get(layer.id, &url));
    Some(LoadedSemanticFieldLayer { field, metadata })
}

pub fn field_metadata_entry_for_id<'a>(
    layer: &'a LayerSpec,
    field_metadata: &'a FieldMetadataCache,
    field_id: u32,
) -> Option<&'a FieldHoverMetadataEntry> {
    let metadata_url = layer.field_metadata_url()?;
    field_metadata.entry(layer.id, &metadata_url, field_id)
}

pub fn field_fact_value_for_id(
    layer: &LayerSpec,
    field_metadata: &FieldMetadataCache,
    field_id: u32,
    key: &str,
) -> Option<String> {
    let value = field_metadata_entry_for_id(layer, field_metadata, field_id)?
        .fact_value(key)?
        .trim();
    (!value.is_empty()).then(|| value.to_string())
}

pub fn semantic_sample_for_field_id(
    layer: &LayerSpec,
    field_metadata: &FieldMetadataCache,
    field_id: u32,
    rgb: Rgb,
) -> FieldSemanticSample {
    let (targets, detail_pane, detail_sections) =
        field_metadata_entry_for_id(layer, field_metadata, field_id)
            .map(|entry| {
                (
                    entry.targets.clone(),
                    entry.detail_pane.clone(),
                    entry.detail_sections.clone(),
                )
            })
            .unwrap_or_else(|| (Vec::new(), None, Vec::new()));
    FieldSemanticSample {
        field_id,
        rgb,
        rgb_u32: rgb.to_u32(),
        targets,
        detail_pane,
        detail_sections,
    }
}

impl FieldLayerView for LoadedSemanticFieldLayer<'_> {
    fn width(&self) -> u16 {
        self.field.width()
    }

    fn height(&self) -> u16 {
        self.field.height()
    }

    fn field_id_at_map_px(&self, map_px_x: i32, map_px_y: i32) -> Option<u32> {
        self.field.field_id_at_map_px(map_px_x, map_px_y)
    }

    fn contains_at_map_px(&self, map_px_x: i32, map_px_y: i32) -> bool {
        self.field.contains_at_map_px(map_px_x, map_px_y)
    }

    fn rgb_at_map_px(&self, map_px_x: i32, map_px_y: i32) -> Option<Rgb> {
        self.field.rgb_at_map_px(map_px_x, map_px_y)
    }

    fn render_rgba_chunk(
        &self,
        source_origin_x: i32,
        source_origin_y: i32,
        source_width: u32,
        source_height: u32,
        output_width: u16,
        output_height: u16,
    ) -> fishystuff_core::field::FieldRgbaChunk {
        self.field.render_rgba_chunk(
            source_origin_x,
            source_origin_y,
            source_width,
            source_height,
            output_width,
            output_height,
        )
    }
}

impl SemanticFieldLayerView for LoadedSemanticFieldLayer<'_> {
    fn metadata_entry_for_field_id(&self, field_id: u32) -> Option<&FieldHoverMetadataEntry> {
        self.metadata.and_then(|metadata| metadata.entry(field_id))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        field_fact_value_for_id, loaded_semantic_field_layer, ordered_semantic_layers,
        SemanticFieldLayerView,
    };
    use crate::map::exact_lookup::ExactLookupCache;
    use crate::map::field_metadata::FieldMetadataCache;
    use crate::map::layers::LayerRegistry;
    use crate::map::spaces::affine::Affine2D;
    use crate::map::spaces::layer_transform::LayerTransform;
    use crate::map::spaces::world::MapToWorld;
    use crate::map::spaces::WorldPoint;
    use fishystuff_core::field::DiscreteFieldRows;
    use fishystuff_core::field_metadata::{
        FieldDetailFact, FieldDetailPaneRef, FieldDetailSection, FieldHoverMetadataAsset,
        FieldHoverMetadataEntry, FieldHoverTarget, FIELD_DETAIL_FACT_KEY_ORIGIN_REGION,
    };

    fn origin_metadata_entry(
        with_targets: bool,
        with_detail_pane: bool,
    ) -> FieldHoverMetadataEntry {
        FieldHoverMetadataEntry {
            targets: if with_targets {
                vec![FieldHoverTarget {
                    key: "origin_node".to_string(),
                    label: "Origin: Tarif".to_string(),
                    world_x: 1.0,
                    world_z: 2.0,
                }]
            } else {
                Vec::new()
            },
            detail_pane: with_detail_pane.then(|| FieldDetailPaneRef {
                id: "territory".to_string(),
                label: "Territory".to_string(),
                icon: "hover-origin".to_string(),
                order: 200,
            }),
            detail_sections: vec![FieldDetailSection {
                id: "trade-origin".to_string(),
                kind: "facts".to_string(),
                title: Some("Trade Origin".to_string()),
                facts: vec![FieldDetailFact {
                    key: FIELD_DETAIL_FACT_KEY_ORIGIN_REGION.to_string(),
                    label: "Region".to_string(),
                    value: "Tarif".to_string(),
                    icon: Some("hover-origin".to_string()),
                    status_icon: None,
                    status_icon_tone: None,
                }],
                targets: if with_targets {
                    vec![FieldHoverTarget {
                        key: "origin_node".to_string(),
                        label: "Origin: Tarif".to_string(),
                        world_x: 1.0,
                        world_z: 2.0,
                    }]
                } else {
                    Vec::new()
                },
            }],
        }
    }

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

    fn test_registry() -> LayerRegistry {
        let mut registry = LayerRegistry::default();
        registry.apply_layers_response(fishystuff_api::models::layers::LayersResponse {
            revision: "rev".to_string(),
            map_version_id: None,
            layers: vec![field_layer_descriptor(
                "regions",
                fishystuff_api::models::layers::LayerTransformDto::IdentityMapSpace,
            )],
        });
        registry
    }

    #[test]
    fn ordered_semantic_layers_follow_registry_order_and_skip_nonsemantic_layers() {
        let mut registry = LayerRegistry::default();
        let mut zone_mask = field_layer_descriptor(
            "zone_mask",
            fishystuff_api::models::layers::LayerTransformDto::IdentityMapSpace,
        );
        zone_mask.ui.display_order = 20;
        let mut regions = field_layer_descriptor(
            "regions",
            fishystuff_api::models::layers::LayerTransformDto::IdentityMapSpace,
        );
        regions.ui.display_order = 40;
        registry.apply_layers_response(fishystuff_api::models::layers::LayersResponse {
            revision: "rev".to_string(),
            map_version_id: None,
            layers: vec![
                fishystuff_api::models::layers::LayerDescriptor {
                    layer_id: "minimap".to_string(),
                    name: "Minimap".to_string(),
                    enabled: true,
                    kind: fishystuff_api::models::layers::LayerKind::TiledRaster,
                    transform: fishystuff_api::models::layers::LayerTransformDto::IdentityMapSpace,
                    tileset: fishystuff_api::models::layers::TilesetRef::default(),
                    tile_px: 512,
                    max_level: 0,
                    y_flip: false,
                    field_source: Some(fishystuff_api::models::layers::FieldSourceRef {
                        url: "/fields/minimap.v1.bin".to_string(),
                        revision: "minimap-field-v1".to_string(),
                        color_mode: fishystuff_api::models::layers::FieldColorMode::DebugHash,
                    }),
                    field_metadata_source: Some(
                        fishystuff_api::models::layers::FieldMetadataSourceRef {
                            url: "/fields/minimap.v1.meta.json".to_string(),
                            revision: "minimap-meta-v1".to_string(),
                        },
                    ),
                    vector_source: None,
                    lod_policy: fishystuff_api::models::layers::LodPolicyDto::default(),
                    ui: fishystuff_api::models::layers::LayerUiInfo {
                        display_order: 10,
                        ..Default::default()
                    },
                    request_weight: 1.0,
                    pick_mode: "none".to_string(),
                },
                zone_mask,
                regions,
                fishystuff_api::models::layers::LayerDescriptor {
                    layer_id: "vector".to_string(),
                    name: "Vector".to_string(),
                    enabled: true,
                    kind: fishystuff_api::models::layers::LayerKind::VectorGeoJson,
                    transform: fishystuff_api::models::layers::LayerTransformDto::IdentityMapSpace,
                    tileset: fishystuff_api::models::layers::TilesetRef::default(),
                    tile_px: 512,
                    max_level: 0,
                    y_flip: false,
                    field_source: None,
                    field_metadata_source: None,
                    vector_source: Some(fishystuff_api::models::layers::VectorSourceRef {
                        url: "/vector.geojson".to_string(),
                        revision: "vector-v1".to_string(),
                        geometry_space: fishystuff_api::models::layers::GeometrySpace::MapPixels,
                        style_mode:
                            fishystuff_api::models::layers::StyleMode::FeaturePropertyPalette,
                        feature_id_property: None,
                        color_property: None,
                    }),
                    lod_policy: fishystuff_api::models::layers::LodPolicyDto::default(),
                    ui: fishystuff_api::models::layers::LayerUiInfo {
                        display_order: 50,
                        ..Default::default()
                    },
                    request_weight: 1.0,
                    pick_mode: "none".to_string(),
                },
            ],
        });

        let ordered = ordered_semantic_layers(&registry)
            .into_iter()
            .map(|layer| layer.key.as_str())
            .collect::<Vec<_>>();

        assert_eq!(ordered, vec!["zone_mask", "regions"]);
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
    fn semantic_sample_collects_rgb_facts_and_targets() {
        let registry = test_registry();
        let layer = registry.get_by_key("regions").expect("regions layer");
        let mut exact_lookups = ExactLookupCache::default();
        exact_lookups.insert_ready(
            layer.id,
            "/fields/regions.v1.bin".to_string(),
            DiscreteFieldRows::from_u32_grid(1, 1, &[76]).expect("field"),
        );
        let mut field_metadata = FieldMetadataCache::default();
        field_metadata.insert_ready(
            layer.id,
            "/fields/regions.v1.meta.json".to_string(),
            FieldHoverMetadataAsset {
                entries: std::collections::BTreeMap::from([(
                    76,
                    origin_metadata_entry(true, true),
                )]),
            },
        );

        let semantic = loaded_semantic_field_layer(layer, &exact_lookups, &field_metadata)
            .expect("semantic layer")
            .semantic_sample_at_map_px(0, 0)
            .expect("semantic sample");
        assert_eq!(semantic.field_id, 76);
        assert_eq!(semantic.rgb_u32, semantic.rgb.to_u32());
        assert_eq!(
            semantic.targets,
            vec![FieldHoverTarget {
                key: "origin_node".to_string(),
                label: "Origin: Tarif".to_string(),
                world_x: 1.0,
                world_z: 2.0,
            }]
        );
        assert_eq!(
            semantic.detail_pane,
            Some(FieldDetailPaneRef {
                id: "territory".to_string(),
                label: "Territory".to_string(),
                icon: "hover-origin".to_string(),
                order: 200,
            })
        );
        assert_eq!(semantic.detail_sections.len(), 1);
        assert_eq!(semantic.detail_sections[0].id, "trade-origin");
    }

    #[test]
    fn semantic_sample_at_world_point_uses_layer_transform() {
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
                    origin_metadata_entry(false, false),
                )]),
            },
        );

        let semantic = loaded_semantic_field_layer(layer, &exact_lookups, &field_metadata)
            .expect("semantic layer")
            .semantic_sample_at_world_point(
                layer,
                MapToWorld::default(),
                WorldPoint::new(1.25, 0.25),
            )
            .expect("semantic sample");
        assert_eq!(semantic.field_id, 76);
        assert_eq!(
            semantic.fact_value(FIELD_DETAIL_FACT_KEY_ORIGIN_REGION),
            Some("Tarif")
        );
    }

    #[test]
    fn fact_value_for_id_reads_metadata_without_field_lookup() {
        let registry = test_registry();
        let layer = registry.get_by_key("regions").expect("regions layer");
        let mut field_metadata = FieldMetadataCache::default();
        field_metadata.insert_ready(
            layer.id,
            "/fields/regions.v1.meta.json".to_string(),
            FieldHoverMetadataAsset {
                entries: std::collections::BTreeMap::from([(
                    76,
                    origin_metadata_entry(false, false),
                )]),
            },
        );
        assert_eq!(
            field_fact_value_for_id(
                layer,
                &field_metadata,
                76,
                FIELD_DETAIL_FACT_KEY_ORIGIN_REGION
            ),
            Some("Tarif".to_string())
        );
    }
}
