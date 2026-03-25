use async_channel::Receiver;
use bevy::tasks::IoTaskPool;
#[cfg(target_arch = "wasm32")]
use fishystuff_api::models::fish::FishListResponse;
use fishystuff_api::models::meta::MetaResponse;
use fishystuff_api::models::zone_stats::{ZoneStatsRequest, ZoneStatsResponse};
use fishystuff_api::models::zones::ZonesResponse;
use fishystuff_client::{ClientError, FishyClient};

use super::super::state::FishCatalogPayload;
#[cfg(target_arch = "wasm32")]
use super::util::resolve_api_request_url;
#[cfg(target_arch = "wasm32")]
use crate::runtime_io;

pub(super) fn spawn_zone_stats_request(
    request: ZoneStatsRequest,
) -> Receiver<Result<ZoneStatsResponse, String>> {
    let (sender, receiver) = async_channel::bounded(1);
    IoTaskPool::get()
        .spawn_local(async move {
            let client = FishyClient::new("");
            let result = client
                .zone_stats(&request)
                .await
                .map_err(client_error_to_string);
            let _ = sender.send(result).await;
        })
        .detach();
    receiver
}

pub(super) fn spawn_meta_request() -> Receiver<Result<MetaResponse, String>> {
    let (sender, receiver) = async_channel::bounded(1);
    IoTaskPool::get()
        .spawn_local(async move {
            let client = FishyClient::new("");
            let result = client.meta().await.map_err(client_error_to_string);
            let _ = sender.send(result).await;
        })
        .detach();
    receiver
}

pub(super) fn spawn_zones_request() -> Receiver<Result<ZonesResponse, String>> {
    let (sender, receiver) = async_channel::bounded(1);
    IoTaskPool::get()
        .spawn_local(async move {
            let client = FishyClient::new("");
            let result = client.zones().await.map_err(client_error_to_string);
            let _ = sender.send(result).await;
        })
        .detach();
    receiver
}

pub(super) fn spawn_fish_catalog_request() -> Receiver<Result<FishCatalogPayload, String>> {
    #[cfg(target_arch = "wasm32")]
    {
        let url = resolve_api_request_url("/api/v1/fish");
        let (sender, receiver) = async_channel::bounded(1);
        IoTaskPool::get()
            .spawn_local(async move {
                let result = runtime_io::load_json_async::<FishListResponse>(&url)
                    .await
                    .map(|fish| FishCatalogPayload { fish });
                let _ = sender.send(result).await;
            })
            .detach();
        return receiver;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let (sender, receiver) = async_channel::bounded(1);
        IoTaskPool::get()
            .spawn_local(async move {
                let client = FishyClient::new("");
                let result = client
                    .fish()
                    .await
                    .map(|fish| FishCatalogPayload { fish })
                    .map_err(client_error_to_string);
                let _ = sender.send(result).await;
            })
            .detach();
        receiver
    }
}

fn client_error_to_string(error: ClientError) -> String {
    match error {
        ClientError::Transport(message) => message,
        ClientError::Decode(message) => message,
        ClientError::Api(error) => error.message,
        ClientError::HttpStatus(status, body) => format!("http {status}: {body}"),
    }
}
