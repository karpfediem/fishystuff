use async_channel::Receiver;
use bevy::tasks::IoTaskPool;
use fishystuff_api::models::layers::LayersResponse;
use fishystuff_api::models::meta::MetaResponse;
use fishystuff_api::models::zone_stats::{ZoneStatsRequest, ZoneStatsResponse};
use fishystuff_api::models::zones::ZonesResponse;
use fishystuff_client::{ClientError, FishyClient};

use super::super::state::FishCatalogPayload;

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

pub(super) fn spawn_layers_request(
    map_version: Option<String>,
) -> Receiver<Result<LayersResponse, String>> {
    let (sender, receiver) = async_channel::bounded(1);
    IoTaskPool::get()
        .spawn_local(async move {
            let client = FishyClient::new("");
            let result = client
                .layers(map_version.as_deref())
                .await
                .map_err(client_error_to_string);
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
    let (sender, receiver) = async_channel::bounded(1);
    IoTaskPool::get()
        .spawn_local(async move {
            let client = FishyClient::new("");
            let result = async {
                let fish = client.fish().await.map_err(client_error_to_string)?;
                let fish_table = client.fish_table().await.map_err(client_error_to_string)?;
                Ok(FishCatalogPayload { fish, fish_table })
            }
            .await;
            let _ = sender.send(result).await;
        })
        .detach();
    receiver
}

fn client_error_to_string(error: ClientError) -> String {
    match error {
        ClientError::Transport(message) => message,
        ClientError::Decode(message) => message,
        ClientError::Api(error) => error.message,
        ClientError::HttpStatus(status, body) => format!("http {status}: {body}"),
    }
}
