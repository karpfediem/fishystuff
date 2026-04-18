use fishystuff_api::error::ApiError;
#[cfg(target_arch = "wasm32")]
use fishystuff_api::error::ApiErrorEnvelope;
use fishystuff_api::models::effort::{EffortGridRequest, EffortGridResponse};
use fishystuff_api::models::events::{EventsSnapshotMetaResponse, EventsSnapshotResponse};
use fishystuff_api::models::fish::{CommunityFishZoneSupportResponse, FishListResponse};
use fishystuff_api::models::meta::MetaResponse;
use fishystuff_api::models::region_groups::RegionGroupsResponse;
use fishystuff_api::models::zone_stats::{ZoneStatsRequest, ZoneStatsResponse};
use fishystuff_api::models::zones::ZonesResponse;
#[cfg(target_arch = "wasm32")]
use fishystuff_core::public_endpoints::{
    derive_sibling_public_base_url, normalize_public_base_url, DEFAULT_PUBLIC_API_BASE_URL,
};
#[cfg(target_arch = "wasm32")]
use web_sys::{RequestCache, RequestMode};

#[derive(Debug, Clone)]
pub struct FishyClient {
    base_url: String,
}

#[derive(Debug, Clone)]
pub enum ClientError {
    Transport(String),
    Decode(String),
    Api(ApiError),
    HttpStatus(u16, String),
}

impl FishyClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        #[cfg(target_arch = "wasm32")]
        let base_url = if base_url.is_empty() {
            default_browser_base_url()
        } else {
            base_url
        };
        Self { base_url }
    }

    pub async fn meta(&self) -> Result<MetaResponse, ClientError> {
        self.get_json("/api/v1/meta").await
    }

    pub async fn region_groups(
        &self,
        map_version: Option<&str>,
    ) -> Result<RegionGroupsResponse, ClientError> {
        let path = if let Some(map_version) = map_version {
            format!("/api/v1/region_groups?map_version={map_version}")
        } else {
            "/api/v1/region_groups".to_string()
        };
        self.get_json(&path).await
    }

    pub async fn zones(&self) -> Result<ZonesResponse, ClientError> {
        self.get_json("/api/v1/zones").await
    }

    pub async fn fish(&self) -> Result<FishListResponse, ClientError> {
        self.get_json("/api/v1/fish").await
    }

    pub async fn community_fish_zone_support(
        &self,
    ) -> Result<CommunityFishZoneSupportResponse, ClientError> {
        self.get_json("/api/v1/fish/community_zone_support").await
    }

    pub async fn zone_stats(
        &self,
        request: &ZoneStatsRequest,
    ) -> Result<ZoneStatsResponse, ClientError> {
        self.post_json("/api/v1/zone_stats", request).await
    }

    pub async fn effort_grid(
        &self,
        request: &EffortGridRequest,
    ) -> Result<EffortGridResponse, ClientError> {
        self.post_json("/api/v1/effort_grid", request).await
    }

    pub async fn events_snapshot_meta(&self) -> Result<EventsSnapshotMetaResponse, ClientError> {
        self.get_json("/api/v1/events_snapshot_meta").await
    }

    pub async fn events_snapshot(
        &self,
        revision: &str,
    ) -> Result<EventsSnapshotResponse, ClientError> {
        self.get_json(&format!("/api/v1/events_snapshot?revision={revision}"))
            .await
    }

    fn join_url(&self, path: &str) -> String {
        if self.base_url.is_empty() {
            path.to_string()
        } else {
            format!("{}{}", self.base_url, path)
        }
    }

    #[cfg(target_arch = "wasm32")]
    async fn get_json<T>(&self, path: &str) -> Result<T, ClientError>
    where
        for<'de> T: serde::Deserialize<'de>,
    {
        let url = self.join_url(path);
        let response = gloo_net::http::Request::get(&url)
            .cache(RequestCache::NoStore)
            .mode(RequestMode::Cors)
            .send()
            .await
            .map_err(|err| ClientError::Transport(err.to_string()))?;

        if response.ok() {
            return response
                .json::<T>()
                .await
                .map_err(|err| ClientError::Decode(err.to_string()));
        }

        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        parse_error(status, text)
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn get_json<T>(&self, path: &str) -> Result<T, ClientError>
    where
        T: serde::de::DeserializeOwned,
    {
        Err(ClientError::Transport(format!(
            "non-wasm client transport is not enabled for {}",
            self.join_url(path)
        )))
    }

    #[cfg(target_arch = "wasm32")]
    async fn post_json<Req, Resp>(&self, path: &str, payload: &Req) -> Result<Resp, ClientError>
    where
        Req: serde::Serialize,
        for<'de> Resp: serde::Deserialize<'de>,
    {
        let url = self.join_url(path);
        let response = gloo_net::http::Request::post(&url)
            .json(payload)
            .map_err(|err| ClientError::Decode(err.to_string()))?
            .send()
            .await
            .map_err(|err| ClientError::Transport(err.to_string()))?;

        if response.ok() {
            return response
                .json::<Resp>()
                .await
                .map_err(|err| ClientError::Decode(err.to_string()));
        }

        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        parse_error(status, text)
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn post_json<Req, Resp>(&self, path: &str, payload: &Req) -> Result<Resp, ClientError>
    where
        Req: serde::Serialize,
        Resp: serde::de::DeserializeOwned,
    {
        let _ = payload;
        Err(ClientError::Transport(format!(
            "non-wasm client transport is not enabled for {}",
            self.join_url(path)
        )))
    }
}

#[cfg(target_arch = "wasm32")]
fn default_browser_base_url() -> String {
    browser_global_base_url("__fishystuffApiBaseUrl")
        .or_else(browser_location_api_base_url)
        .unwrap_or_else(|| DEFAULT_PUBLIC_API_BASE_URL.to_string())
}

#[cfg(target_arch = "wasm32")]
fn browser_global_base_url(name: &str) -> Option<String> {
    use wasm_bindgen::JsValue;

    let window = web_sys::window()?;
    let value = js_sys::Reflect::get(window.as_ref(), &JsValue::from_str(name)).ok()?;
    let value = value.as_string()?;
    normalize_public_base_url(Some(value.as_str()))
}

#[cfg(target_arch = "wasm32")]
fn browser_location_api_base_url() -> Option<String> {
    let window = web_sys::window()?;
    let origin = window.location().origin().ok()?;
    derive_sibling_public_base_url(Some(origin.as_str()), "api")
}

#[cfg(target_arch = "wasm32")]
fn parse_error<T>(status: u16, body: String) -> Result<T, ClientError> {
    if let Ok(envelope) = serde_json::from_str::<ApiErrorEnvelope>(&body) {
        return Err(ClientError::Api(envelope.error));
    }
    Err(ClientError::HttpStatus(status, body))
}
