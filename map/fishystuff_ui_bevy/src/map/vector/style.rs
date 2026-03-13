use serde_json::{Map, Value};

use crate::map::layers::{StyleMode, VectorSourceSpec};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StyleBucketKey {
    pub rgba: [u8; 4],
}

impl StyleBucketKey {
    pub const fn new(rgba: [u8; 4]) -> Self {
        Self { rgba }
    }
}

pub fn style_bucket_key(
    source: &VectorSourceSpec,
    properties: &Map<String, Value>,
    feature_index: usize,
) -> StyleBucketKey {
    match source.style_mode {
        StyleMode::FeaturePropertyPalette => {
            let candidate_name = source
                .color_property
                .as_deref()
                .filter(|name| !name.trim().is_empty())
                .or(Some("c"));
            if let Some(name) = candidate_name {
                if let Some(color) = properties.get(name).and_then(color_from_value) {
                    return StyleBucketKey::new([color[0], color[1], color[2], 255]);
                }
            }
        }
    }

    StyleBucketKey::new(hash_color(feature_index))
}

fn color_from_value(value: &Value) -> Option<[u8; 3]> {
    match value {
        Value::Array(values) => color_from_array(values),
        Value::String(value) => color_from_string(value),
        Value::Number(value) => value.as_u64().map(color_from_u64),
        Value::Object(object) => {
            let r = object.get("r").and_then(channel_from_value)?;
            let g = object.get("g").and_then(channel_from_value)?;
            let b = object.get("b").and_then(channel_from_value)?;
            Some([r, g, b])
        }
        _ => None,
    }
}

fn color_from_array(values: &[Value]) -> Option<[u8; 3]> {
    if values.len() < 3 {
        return None;
    }
    let r = channel_from_value(&values[0])?;
    let g = channel_from_value(&values[1])?;
    let b = channel_from_value(&values[2])?;
    Some([r, g, b])
}

fn color_from_string(value: &str) -> Option<[u8; 3]> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some([r, g, b]);
        }
    }
    if let Some((r, g, b)) = parse_rgb_triplet(trimmed) {
        return Some([r, g, b]);
    }
    None
}

fn parse_rgb_triplet(value: &str) -> Option<(u8, u8, u8)> {
    let mut parts = value.split(',').map(str::trim);
    let r = parts.next()?.parse::<u8>().ok()?;
    let g = parts.next()?.parse::<u8>().ok()?;
    let b = parts.next()?.parse::<u8>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((r, g, b))
}

fn channel_from_value(value: &Value) -> Option<u8> {
    match value {
        Value::Number(value) => {
            if let Some(int_value) = value.as_u64() {
                u8::try_from(int_value).ok()
            } else {
                value.as_f64().map(|f| f.clamp(0.0, 255.0).round() as u8)
            }
        }
        Value::String(value) => value.trim().parse::<u8>().ok(),
        _ => None,
    }
}

fn color_from_u64(value: u64) -> [u8; 3] {
    let value = value as u32;
    [
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    ]
}

fn hash_color(feature_index: usize) -> [u8; 4] {
    let mut hash = 2166136261u32;
    for byte in feature_index.to_le_bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(16777619);
    }
    let r = (((hash >> 16) & 0x7f) + 96) as u8;
    let g = (((hash >> 8) & 0x7f) + 96) as u8;
    let b = ((hash & 0x7f) + 96) as u8;
    [r, g, b, 255]
}
