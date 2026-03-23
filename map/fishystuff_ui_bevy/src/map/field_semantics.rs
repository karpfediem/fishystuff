use fishystuff_api::Rgb;
use fishystuff_core::field_metadata::{
    FieldHoverMetadataAsset, FieldHoverMetadataEntry, FieldHoverRow, FieldHoverTarget,
};

use crate::map::exact_lookup::ExactLookupCache;
use crate::map::field_metadata::FieldMetadataCache;
use crate::map::field_view::{loaded_field_layer, FieldLayerView, LoadedFieldLayer};
use crate::map::layers::LayerSpec;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FieldSemanticSample {
    pub field_id: u32,
    pub rgb: Rgb,
    pub rgb_u32: u32,
    pub rows: Vec<FieldHoverRow>,
    pub targets: Vec<FieldHoverTarget>,
}

impl FieldSemanticSample {
    pub fn row_value(&self, key: &str) -> Option<&str> {
        self.rows
            .iter()
            .find(|row| row.key == key)
            .map(|row| row.value.trim())
            .filter(|value| !value.is_empty())
    }
}

pub trait SemanticFieldLayerView: FieldLayerView {
    fn metadata_entry_for_field_id(&self, field_id: u32) -> Option<&FieldHoverMetadataEntry>;

    fn semantic_sample_at_map_px(
        &self,
        map_px_x: i32,
        map_px_y: i32,
    ) -> Option<FieldSemanticSample> {
        let field_id = self.field_id_at_map_px(map_px_x, map_px_y)?;
        let rgb = self.rgb_at_map_px(map_px_x, map_px_y)?;
        let (rows, targets) = self
            .metadata_entry_for_field_id(field_id)
            .map(|entry| (entry.rows.clone(), entry.targets.clone()))
            .unwrap_or_else(|| (Vec::new(), Vec::new()));
        Some(FieldSemanticSample {
            field_id,
            rgb,
            rgb_u32: rgb.to_u32(),
            rows,
            targets,
        })
    }

    fn row_value_at_map_px(&self, map_px_x: i32, map_px_y: i32, key: &str) -> Option<String> {
        self.semantic_sample_at_map_px(map_px_x, map_px_y)?
            .row_value(key)
            .map(ToOwned::to_owned)
    }

    fn row_value_for_field_id(&self, field_id: u32, key: &str) -> Option<String> {
        let value = self
            .metadata_entry_for_field_id(field_id)?
            .row_value(key)?
            .trim();
        (!value.is_empty()).then(|| value.to_string())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LoadedSemanticFieldLayer<'a> {
    field: LoadedFieldLayer<'a>,
    metadata: Option<&'a FieldHoverMetadataAsset>,
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

pub fn field_row_value_for_id(
    layer: &LayerSpec,
    field_metadata: &FieldMetadataCache,
    field_id: u32,
    key: &str,
) -> Option<String> {
    let value = field_metadata_entry_for_id(layer, field_metadata, field_id)?
        .row_value(key)?
        .trim();
    (!value.is_empty()).then(|| value.to_string())
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
    use super::{field_row_value_for_id, loaded_semantic_field_layer, SemanticFieldLayerView};
    use crate::map::exact_lookup::ExactLookupCache;
    use crate::map::field_metadata::FieldMetadataCache;
    use crate::map::layers::LayerRegistry;
    use fishystuff_core::field::DiscreteFieldRows;
    use fishystuff_core::field_metadata::{
        FieldHoverMetadataAsset, FieldHoverMetadataEntry, FieldHoverRow, FieldHoverTarget,
    };

    fn field_layer_descriptor(layer_id: &str) -> fishystuff_api::models::layers::LayerDescriptor {
        fishystuff_api::models::layers::LayerDescriptor {
            layer_id: layer_id.to_string(),
            name: layer_id.to_string(),
            enabled: true,
            kind: fishystuff_api::models::layers::LayerKind::TiledRaster,
            transform: fishystuff_api::models::layers::LayerTransformDto::IdentityMapSpace,
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
            layers: vec![field_layer_descriptor("regions")],
        });
        registry
    }

    #[test]
    fn semantic_sample_collects_rgb_rows_and_targets() {
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
                        targets: vec![FieldHoverTarget {
                            key: "origin_node".to_string(),
                            label: "Origin: Tarif".to_string(),
                            world_x: 1.0,
                            world_z: 2.0,
                        }],
                    },
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
            semantic.rows,
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
        assert_eq!(
            semantic.targets,
            vec![FieldHoverTarget {
                key: "origin_node".to_string(),
                label: "Origin: Tarif".to_string(),
                world_x: 1.0,
                world_z: 2.0,
            }]
        );
    }

    #[test]
    fn row_value_for_id_reads_metadata_without_field_lookup() {
        let registry = test_registry();
        let layer = registry.get_by_key("regions").expect("regions layer");
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
        assert_eq!(
            field_row_value_for_id(layer, &field_metadata, 76, "origin"),
            Some("Tarif".to_string())
        );
    }
}
