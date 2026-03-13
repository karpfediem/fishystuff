use serde::{Deserialize, Serialize};

use crate::ids::{Rgb, RgbKey};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZoneEntry {
    pub rgb_u32: u32,
    pub rgb: Rgb,
    pub rgb_key: RgbKey,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZonesResponse {
    #[serde(default)]
    pub zones: Vec<ZoneEntry>,
}
