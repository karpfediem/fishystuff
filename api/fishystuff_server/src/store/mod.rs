use async_trait::async_trait;

use fishystuff_api::models::calculator::CalculatorCatalogResponse;
use fishystuff_api::models::effort::{EffortGridRequest, EffortGridResponse};
use fishystuff_api::models::events::{EventsSnapshotMetaResponse, EventsSnapshotResponse};
use fishystuff_api::models::fish::FishListResponse;
use fishystuff_api::models::meta::MetaResponse;
use fishystuff_api::models::region_groups::RegionGroupsResponse;
use fishystuff_api::models::zone_profile_v2::{ZoneProfileV2Request, ZoneProfileV2Response};
use fishystuff_api::models::zone_stats::{ZoneStatsRequest, ZoneStatsResponse};
use fishystuff_api::models::zones::ZoneEntry;

use crate::config::ZoneStatusConfig;
use crate::error::AppResult;

pub mod dolt_mysql;
pub mod queries;

pub use dolt_mysql::DoltMySqlStore;

#[derive(Debug, Clone, Copy)]
pub enum FishLang {
    En,
    Ko,
}

impl FishLang {
    pub fn from_param(lang: Option<&str>) -> Self {
        let value = lang.unwrap_or("en").trim().to_ascii_lowercase();
        if value.starts_with("ko") || value == "kr" || value == "korean" {
            Self::Ko
        } else {
            Self::En
        }
    }
}

#[async_trait]
pub trait Store: Send + Sync {
    async fn get_meta(&self) -> AppResult<MetaResponse>;
    async fn get_region_groups(
        &self,
        map_version_id: Option<String>,
    ) -> AppResult<RegionGroupsResponse>;
    async fn list_fish(
        &self,
        lang: FishLang,
        ref_id: Option<String>,
    ) -> AppResult<FishListResponse>;
    async fn calculator_catalog(
        &self,
        lang: FishLang,
        ref_id: Option<String>,
    ) -> AppResult<CalculatorCatalogResponse>;
    async fn list_zones(&self, ref_id: Option<String>) -> AppResult<Vec<ZoneEntry>>;
    async fn zone_stats(
        &self,
        request: ZoneStatsRequest,
        status_cfg: ZoneStatusConfig,
    ) -> AppResult<ZoneStatsResponse>;
    async fn zone_profile_v2(
        &self,
        request: ZoneProfileV2Request,
        status_cfg: ZoneStatusConfig,
    ) -> AppResult<ZoneProfileV2Response>;
    async fn effort_grid(&self, request: EffortGridRequest) -> AppResult<EffortGridResponse>;
    async fn events_snapshot_meta(&self) -> AppResult<EventsSnapshotMetaResponse>;
    async fn events_snapshot(
        &self,
        requested_revision: Option<String>,
    ) -> AppResult<EventsSnapshotResponse>;
    async fn healthcheck(&self) -> AppResult<()>;
}

pub fn validate_dolt_ref(value: &str) -> AppResult<()> {
    if value.is_empty() {
        return Err(crate::error::AppError::invalid_argument(
            "ref cannot be empty",
        ));
    }
    let ok = value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/' | '^' | '~'));
    if !ok {
        return Err(crate::error::AppError::invalid_argument(format!(
            "invalid ref: {value}"
        )));
    }
    Ok(())
}
