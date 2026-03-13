use fishystuff_api::models::layers::{
    GeometrySpace, LayerKind, LayerTransformDto, StyleMode, VectorSourceRef,
};
use fishystuff_core::asset_urls::normalize_site_asset_path;

use crate::error::{AppError, AppResult};

use super::util::normalize_optional_string;

pub(super) fn parse_layer_transform(
    kind: &str,
    a: Option<f64>,
    b: Option<f64>,
    tx: Option<f64>,
    c: Option<f64>,
    d: Option<f64>,
    ty: Option<f64>,
) -> LayerTransformDto {
    match kind.trim().to_ascii_lowercase().as_str() {
        "identity_map_space" => LayerTransformDto::IdentityMapSpace,
        "affine_to_map" => LayerTransformDto::AffineToMap {
            a: a.unwrap_or(1.0),
            b: b.unwrap_or(0.0),
            tx: tx.unwrap_or(0.0),
            c: c.unwrap_or(0.0),
            d: d.unwrap_or(1.0),
            ty: ty.unwrap_or(0.0),
        },
        "affine_to_world" => LayerTransformDto::AffineToWorld {
            a: a.unwrap_or(1.0),
            b: b.unwrap_or(0.0),
            tx: tx.unwrap_or(0.0),
            c: c.unwrap_or(0.0),
            d: d.unwrap_or(1.0),
            ty: ty.unwrap_or(0.0),
        },
        _ => LayerTransformDto::IdentityMapSpace,
    }
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
    source_url: Option<String>,
    source_revision: Option<String>,
    geometry_space: Option<String>,
    style_mode: Option<String>,
    feature_id_property: Option<String>,
    color_property: Option<String>,
    map_version_id: Option<&str>,
    asset_base_url: Option<&str>,
) -> AppResult<Option<VectorSourceRef>> {
    if kind != LayerKind::VectorGeoJson {
        return Ok(None);
    }
    let source_url = resolve_layer_asset_url(
        &substitute_map_version(source_url.as_deref().unwrap_or(""), map_version_id),
        asset_base_url,
    );
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

pub(super) fn normalize_pick_mode(value: String) -> String {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "none" | "exact_tile_pixel" => normalized,
        _ => "none".to_string(),
    }
}

pub(super) fn substitute_map_version(url: &str, map_version_id: Option<&str>) -> String {
    let Some(map_version_id) = map_version_id else {
        return url.to_string();
    };
    url.replace("{map_version}", map_version_id)
}

pub(super) fn normalize_asset_base_url(value: Option<String>) -> Option<String> {
    normalize_optional_string(value).map(|base| base.trim_end_matches('/').to_string())
}

pub(super) fn resolve_layer_asset_url(url: &str, asset_base_url: Option<&str>) -> String {
    let normalized = normalize_site_asset_path(url);
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_string();
    }
    let Some(base) = asset_base_url
        .map(str::trim)
        .map(|raw| raw.trim_end_matches('/'))
    else {
        return trimmed.to_string();
    };
    if base.is_empty() {
        return trimmed.to_string();
    }
    if trimmed.starts_with('/') {
        return format!("{base}{trimmed}");
    }
    format!("{base}/{}", trimmed.trim_start_matches('/'))
}

#[cfg(test)]
mod tests {
    use super::resolve_layer_asset_url;

    #[test]
    fn resolve_layer_asset_url_normalizes_legacy_site_paths() {
        assert_eq!(
            resolve_layer_asset_url("/tiles/mask/v1/{level}/{x}_{y}.png", None),
            "/images/tiles/mask/v1/{level}/{x}_{y}.png"
        );
        assert_eq!(
            resolve_layer_asset_url("/terrain/v1/manifest.json", None),
            "/images/terrain/v1/manifest.json"
        );
        assert_eq!(
            resolve_layer_asset_url(
                "/tiles/mask/v1/{level}/{x}_{y}.png",
                Some("https://cdn.example.com")
            ),
            "https://cdn.example.com/images/tiles/mask/v1/{level}/{x}_{y}.png"
        );
    }
}
