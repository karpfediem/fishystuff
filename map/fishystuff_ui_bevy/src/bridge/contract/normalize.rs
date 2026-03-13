use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer};

pub(super) fn deserialize_nullable_string_field<'de, D>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Some(Option::<String>::deserialize(deserializer)?))
}

pub fn normalize_string_list(values: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        if out.iter().any(|existing| existing == trimmed) {
            continue;
        }
        out.push(trimmed.to_string());
    }
    out
}

pub fn normalize_i32_list(values: Vec<i32>) -> Vec<i32> {
    let mut out = Vec::new();
    for value in values {
        if out.contains(&value) {
            continue;
        }
        out.push(value);
    }
    out
}

pub fn normalize_layer_opacity_map(values: BTreeMap<String, f32>) -> BTreeMap<String, f32> {
    let mut out = BTreeMap::new();
    for (key, value) in values {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            continue;
        }
        out.insert(trimmed.to_string(), value.clamp(0.0, 1.0));
    }
    out
}

pub fn normalize_layer_clip_mask_map(values: BTreeMap<String, String>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (key, value) in values {
        let layer_id = key.trim();
        let mask_layer_id = value.trim();
        if layer_id.is_empty() || mask_layer_id.is_empty() {
            continue;
        }
        out.insert(layer_id.to_string(), mask_layer_id.to_string());
    }
    out
}
