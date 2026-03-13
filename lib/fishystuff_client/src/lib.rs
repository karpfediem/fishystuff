use fishystuff_api::error::ApiError;
#[cfg(target_arch = "wasm32")]
use fishystuff_api::error::ApiErrorEnvelope;
use fishystuff_api::models::effort::{EffortGridRequest, EffortGridResponse};
use fishystuff_api::models::events::{EventsSnapshotMetaResponse, EventsSnapshotResponse};
use fishystuff_api::models::fish::{FishListResponse, FishMapResponse, FishTableResponse};
use fishystuff_api::models::layers::LayersResponse;
use fishystuff_api::models::meta::MetaResponse;
use fishystuff_api::models::region_groups::RegionGroupsResponse;
use fishystuff_api::models::zone_stats::{ZoneStatsRequest, ZoneStatsResponse};
use fishystuff_api::models::zones::ZonesResponse;

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

    pub async fn layers(&self, map_version: Option<&str>) -> Result<LayersResponse, ClientError> {
        let path = if let Some(map_version) = map_version {
            format!("/api/v1/layers?map_version={map_version}")
        } else {
            "/api/v1/layers".to_string()
        };
        self.get_json(&path).await
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

    pub async fn fish_table(&self) -> Result<FishTableResponse, ClientError> {
        self.get_json("/api/v1/fish_table").await
    }

    pub async fn fish_map(&self, query: &str) -> Result<FishMapResponse, ClientError> {
        self.get_json(&format!("/api/v1/fish_map?{query}")).await
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
    let hostname = web_sys::window()
        .and_then(|window| window.location().hostname().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if hostname == "localhost"
        || hostname == "127.0.0.1"
        || hostname == "::1"
        || hostname.ends_with(".localhost")
    {
        "http://localhost:8080".to_string()
    } else {
        "https://api.fishystuff.fish".to_string()
    }
}

#[cfg(target_arch = "wasm32")]
fn parse_error<T>(status: u16, body: String) -> Result<T, ClientError> {
    if let Ok(envelope) = serde_json::from_str::<ApiErrorEnvelope>(&body) {
        return Err(ClientError::Api(envelope.error));
    }
    Err(ClientError::HttpStatus(status, body))
}
