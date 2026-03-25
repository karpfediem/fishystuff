use fishystuff_api::models::layers::{
    FieldColorMode, FieldMetadataSourceRef, FieldSourceRef, GeometrySpace, LayerKind, StyleMode,
    VectorSourceRef,
};
use fishystuff_core::asset_urls::normalize_site_asset_path;

use crate::error::{AppError, AppResult};

use super::util::normalize_optional_string;

pub(super) struct VectorSourceFields {
    pub source_url: Option<String>,
    pub source_revision: Option<String>,
    pub geometry_space: Option<String>,
    pub style_mode: Option<String>,
    pub feature_id_property: Option<String>,
    pub color_property: Option<String>,
}

pub(super) struct FieldSourceFields {
    pub source_url: Option<String>,
    pub source_revision: Option<String>,
    pub color_mode: Option<String>,
}

pub(super) struct FieldMetadataSourceFields {
    pub source_url: Option<String>,
    pub source_revision: Option<String>,
}

pub(super) fn parse_layer_kind(layer_id: &str, value: &str) -> AppResult<LayerKind> {
    let normalized = value.trim();
    if normalized.eq_ignore_ascii_case("vector_geojson") {
        Ok(LayerKind::VectorGeoJson)
    } else if normalized.eq_ignore_ascii_case("tiled_raster") || normalized.is_empty() {
        Ok(LayerKind::TiledRaster)
    } else {
        Err(AppError::invalid_argument(format!(
            "layer '{}' has unsupported layer_kind '{}'",
            layer_id, value
        )))
    }
}

fn parse_geometry_space(layer_id: &str, value: Option<String>) -> AppResult<GeometrySpace> {
    let Some(value) = value else {
        return Ok(GeometrySpace::MapPixels);
    };
    let normalized = value.trim();
    if normalized.eq_ignore_ascii_case("world") {
        Ok(GeometrySpace::World)
    } else if normalized.eq_ignore_ascii_case("map_pixels") || normalized.is_empty() {
        Ok(GeometrySpace::MapPixels)
    } else {
        Err(AppError::invalid_argument(format!(
            "layer '{}' has unsupported vector_geometry_space '{}'",
            layer_id, value
        )))
    }
}

fn parse_style_mode(layer_id: &str, value: Option<String>) -> AppResult<StyleMode> {
    let Some(value) = value else {
        return Ok(StyleMode::FeaturePropertyPalette);
    };
    let normalized = value.trim();
    if normalized.eq_ignore_ascii_case("feature_property_palette") || normalized.is_empty() {
        Ok(StyleMode::FeaturePropertyPalette)
    } else {
        Err(AppError::invalid_argument(format!(
            "layer '{}' has unsupported vector_style_mode '{}'",
            layer_id, value
        )))
    }
}

pub(super) fn parse_vector_source(
    layer_id: &str,
    kind: LayerKind,
    source: VectorSourceFields,
    map_version_id: Option<&str>,
) -> AppResult<Option<VectorSourceRef>> {
    if kind != LayerKind::VectorGeoJson {
        return Ok(None);
    }
    let VectorSourceFields {
        source_url,
        source_revision,
        geometry_space,
        style_mode,
        feature_id_property,
        color_property,
    } = source;
    let source_url = resolve_layer_asset_url(&substitute_map_version(
        source_url.as_deref().unwrap_or(""),
        map_version_id,
    ));
    if source_url.trim().is_empty() {
        return Err(AppError::invalid_argument(format!(
            "layer '{}' is vector_geojson but vector_source_url is missing",
            layer_id
        )));
    }
    let source_revision =
        substitute_map_version(source_revision.as_deref().unwrap_or(""), map_version_id);
    Ok(Some(VectorSourceRef {
        url: source_url,
        revision: source_revision,
        geometry_space: parse_geometry_space(layer_id, geometry_space)?,
        style_mode: parse_style_mode(layer_id, style_mode)?,
        feature_id_property: normalize_optional_string(feature_id_property),
        color_property: normalize_optional_string(color_property),
    }))
}

fn parse_field_color_mode(layer_id: &str, value: Option<String>) -> AppResult<FieldColorMode> {
    let Some(value) = value else {
        return Ok(FieldColorMode::RgbU24);
    };
    let normalized = value.trim();
    if normalized.eq_ignore_ascii_case("rgb_u24") || normalized.is_empty() {
        Ok(FieldColorMode::RgbU24)
    } else if normalized.eq_ignore_ascii_case("debug_hash") {
        Ok(FieldColorMode::DebugHash)
    } else {
        Err(AppError::invalid_argument(format!(
            "layer '{}' has unsupported field_color_mode '{}'",
            layer_id, value
        )))
    }
}

pub(super) fn parse_field_source(
    layer_id: &str,
    source: FieldSourceFields,
    map_version_id: Option<&str>,
) -> AppResult<Option<FieldSourceRef>> {
    let FieldSourceFields {
        source_url,
        source_revision,
        color_mode,
    } = source;
    let source_url = resolve_layer_asset_url(&substitute_map_version(
        source_url.as_deref().unwrap_or(""),
        map_version_id,
    ));
    if source_url.trim().is_empty() {
        return Ok(None);
    }
    let source_revision =
        substitute_map_version(source_revision.as_deref().unwrap_or(""), map_version_id);
    Ok(Some(FieldSourceRef {
        url: source_url,
        revision: source_revision,
        color_mode: parse_field_color_mode(layer_id, color_mode)?,
    }))
}

pub(super) fn parse_field_metadata_source(
    source: FieldMetadataSourceFields,
    map_version_id: Option<&str>,
) -> AppResult<Option<FieldMetadataSourceRef>> {
    let FieldMetadataSourceFields {
        source_url,
        source_revision,
    } = source;
    let source_url = resolve_layer_asset_url(&substitute_map_version(
        source_url.as_deref().unwrap_or(""),
        map_version_id,
    ));
    if source_url.trim().is_empty() {
        return Ok(None);
    }
    let source_revision =
        substitute_map_version(source_revision.as_deref().unwrap_or(""), map_version_id);
    Ok(Some(FieldMetadataSourceRef {
        url: source_url,
        revision: source_revision,
    }))
}

pub(super) fn substitute_map_version(url: &str, map_version_id: Option<&str>) -> String {
    let Some(map_version_id) = map_version_id else {
        return url.to_string();
    };
    url.replace("{map_version}", map_version_id)
}

pub(super) fn resolve_layer_asset_url(url: &str) -> String {
    let normalized = normalize_site_asset_path(url);
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use fishystuff_api::models::layers::FieldColorMode;

    use super::{
        parse_field_metadata_source, parse_field_source, resolve_layer_asset_url,
        FieldMetadataSourceFields, FieldSourceFields,
    };

    #[test]
    fn resolve_layer_asset_url_normalizes_legacy_site_paths() {
        assert_eq!(
            resolve_layer_asset_url("/tiles/mask/v1/{level}/{x}_{y}.png"),
            "/images/tiles/mask/v1/{level}/{x}_{y}.png"
        );
        assert_eq!(
            resolve_layer_asset_url("/terrain/v1/manifest.json"),
            "/images/terrain/v1/manifest.json"
        );
        assert_eq!(
            resolve_layer_asset_url("https://example.com/images/tiles/mask/v1/{level}/{x}_{y}.png"),
            "https://example.com/images/tiles/mask/v1/{level}/{x}_{y}.png"
        );
    }

    #[test]
    fn parse_field_source_supports_map_version_and_color_mode() {
        let source = parse_field_source(
            "regions",
            FieldSourceFields {
                source_url: Some("/fields/regions.{map_version}.bin".to_string()),
                source_revision: Some("regions-field-{map_version}".to_string()),
                color_mode: Some("debug_hash".to_string()),
            },
            Some("v1"),
        )
        .expect("field source parse")
        .expect("field source");

        assert_eq!(source.url, "/fields/regions.v1.bin");
        assert_eq!(source.revision, "regions-field-v1");
        assert_eq!(source.color_mode, FieldColorMode::DebugHash);
    }

    #[test]
    fn parse_field_metadata_source_supports_map_version() {
        let source = parse_field_metadata_source(
            FieldMetadataSourceFields {
                source_url: Some("/fields/regions.{map_version}.meta.json".to_string()),
                source_revision: Some("regions-meta-{map_version}".to_string()),
            },
            Some("v1"),
        )
        .expect("field metadata parse")
        .expect("field metadata source");

        assert_eq!(source.url, "/fields/regions.v1.meta.json");
        assert_eq!(source.revision, "regions-meta-v1");
    }
}
