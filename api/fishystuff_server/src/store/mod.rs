use std::borrow::Cow;

use async_trait::async_trait;

use fishystuff_api::models::calculator::CalculatorCatalogResponse;
use fishystuff_api::models::effort::{EffortGridRequest, EffortGridResponse};
use fishystuff_api::models::events::{EventsSnapshotMetaResponse, EventsSnapshotResponse};
use fishystuff_api::models::fish::{
    CommunityFishZoneSupportResponse, FishBestSpotsResponse, FishListResponse,
};
use fishystuff_api::models::meta::MetaResponse;
use fishystuff_api::models::region_groups::RegionGroupsResponse;
use fishystuff_api::models::trade::TradeNpcCatalogResponse;
use fishystuff_api::models::zone_profile_v2::{ZoneProfileV2Request, ZoneProfileV2Response};
use fishystuff_api::models::zone_stats::{ZoneStatsRequest, ZoneStatsResponse};
use fishystuff_api::models::zones::ZoneEntry;

use crate::config::ZoneStatusConfig;
use crate::error::{AppError, AppResult};

pub mod dolt_mysql;
pub mod queries;

pub use dolt_mysql::DoltMySqlStore;

#[derive(Debug, Clone, Default)]
pub struct CalculatorZoneLootEvidence {
    pub source_family: String,
    pub claim_kind: String,
    pub scope: String,
    pub rate: Option<f64>,
    pub normalized_rate: Option<f64>,
    pub status: Option<String>,
    pub claim_count: Option<u32>,
    pub source_id: Option<String>,
    pub source_label: Option<String>,
    pub source_drop_label: Option<String>,
    pub observed_at_utc: Option<String>,
    pub imported_at_utc: Option<String>,
    pub slot_idx: Option<u8>,
    pub item_main_group_key: Option<i64>,
    pub subgroup_key: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct CalculatorZoneLootRateContribution {
    pub source_family: String,
    pub source_id: Option<String>,
    pub source_label: Option<String>,
    pub source_drop_label: Option<String>,
    pub item_source_family: Option<String>,
    pub item_source_id: Option<String>,
    pub item_source_label: Option<String>,
    pub item_source_sheet: Option<String>,
    pub item_source_row: Option<u32>,
    pub item_source_added: Option<bool>,
    pub item_source_imported_at_utc: Option<String>,
    pub item_main_group_key: Option<i64>,
    pub option_idx: Option<u32>,
    pub subgroup_key: Option<i64>,
    pub group_conditions_raw: Vec<String>,
    pub weight: f64,
}

#[derive(Debug, Clone, Default)]
pub struct CalculatorZoneLootOverlayMeta {
    pub added: bool,
    pub slot_overlay_active: bool,
    pub explicit_rate_percent: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct CalculatorZoneLootEntry {
    pub slot_idx: u8,
    pub item_id: i32,
    pub name: String,
    pub icon: Option<String>,
    pub vendor_price: Option<i64>,
    pub fish_exp: Option<i64>,
    pub totem_exp: Option<i64>,
    pub grade: Option<String>,
    pub is_fish: bool,
    pub catch_methods: Vec<String>,
    pub group_conditions_raw: Vec<String>,
    pub within_group_rate: f64,
    pub rate_contributions: Vec<CalculatorZoneLootRateContribution>,
    pub evidence: Vec<CalculatorZoneLootEvidence>,
    pub overlay: CalculatorZoneLootOverlayMeta,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DataLang(Cow<'static, str>);

impl DataLang {
    #[allow(non_upper_case_globals)]
    pub const En: Self = Self(Cow::Borrowed("en"));

    pub fn from_param(lang: Option<&str>) -> AppResult<Self> {
        match lang {
            Some(value) => Self::from_code(value).ok_or_else(|| {
                AppError::invalid_argument(format!("invalid data language code: {value}"))
            }),
            None => Ok(Self::En),
        }
    }

    pub fn from_code(value: &str) -> Option<Self> {
        let code = value.trim();
        if code.is_empty()
            || !code
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
        {
            return None;
        }
        Some(Self(Cow::Owned(code.to_string())))
    }

    pub fn code(&self) -> &str {
        self.0.as_ref()
    }

    pub fn is_korean(&self) -> bool {
        self.code() == "kr"
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
        lang: DataLang,
        ref_id: Option<String>,
    ) -> AppResult<FishListResponse>;
    async fn community_fish_zone_support(
        &self,
        _ref_id: Option<String>,
    ) -> AppResult<CommunityFishZoneSupportResponse> {
        Ok(CommunityFishZoneSupportResponse::default())
    }
    async fn fish_best_spots(
        &self,
        _lang: DataLang,
        _ref_id: Option<String>,
        item_id: i32,
    ) -> AppResult<FishBestSpotsResponse> {
        Ok(FishBestSpotsResponse {
            item_id,
            ..FishBestSpotsResponse::default()
        })
    }
    async fn calculator_catalog(
        &self,
        lang: DataLang,
        ref_id: Option<String>,
    ) -> AppResult<CalculatorCatalogResponse>;
    async fn trade_npc_catalog(&self, ref_id: Option<String>)
        -> AppResult<TradeNpcCatalogResponse>;
    async fn calculator_zone_loot(
        &self,
        _lang: DataLang,
        _ref_id: Option<String>,
        _zone_rgb_key: String,
    ) -> AppResult<Vec<CalculatorZoneLootEntry>> {
        Ok(Vec::new())
    }
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

#[cfg(test)]
mod tests {
    use super::DataLang;

    #[test]
    fn data_lang_does_not_normalize_locale_tags() {
        assert_eq!(DataLang::from_code("kr").unwrap().code(), "kr");
        assert!(DataLang::from_code("KO").is_none());
        assert!(DataLang::from_code("ko-KR").is_none());
        assert!(DataLang::from_param(Some("ko-KR")).is_err());
    }
}
