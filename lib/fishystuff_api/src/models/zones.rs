use serde::{Deserialize, Serialize};

use crate::ids::{Rgb, RgbKey};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZoneEntry {
    pub rgb_u32: u32,
    pub rgb: Rgb,
    pub rgb_key: RgbKey,
    pub name: Option<String>,
    pub active: Option<bool>,
    pub confirmed: Option<bool>,
    pub index: Option<u32>,
    pub bite_time_min: Option<u32>,
    pub bite_time_max: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZonesResponse {
    #[serde(default)]
    pub zones: Vec<ZoneEntry>,
}
