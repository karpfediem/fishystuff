use std::collections::HashMap;

use fishystuff_api::models::meta::{MetaDefaults, MetaResponse};

use crate::prelude::*;

#[derive(Resource)]
pub struct ApiBootstrapState {
    pub meta_status: String,
    pub layers_status: String,
    pub zones_status: String,
    pub meta: Option<MetaResponse>,
    pub defaults: Option<MetaDefaults>,
    pub map_version: Option<String>,
    pub zones: HashMap<u32, Option<String>>,
    pub map_version_dirty: bool,
    pub layers_loaded_map_version: Option<String>,
}

impl Default for ApiBootstrapState {
    fn default() -> Self {
        Self {
            meta_status: "meta: pending".to_string(),
            layers_status: "layers: pending".to_string(),
            zones_status: "zones: pending".to_string(),
            meta: None,
            defaults: None,
            map_version: None,
            zones: HashMap::new(),
            map_version_dirty: false,
            layers_loaded_map_version: None,
        }
    }
}
