use async_channel::Receiver;
use fishystuff_api::models::layers::LayersResponse;
use fishystuff_api::models::meta::MetaResponse;
use fishystuff_api::models::zone_stats::ZoneStatsResponse;
use fishystuff_api::models::zones::ZonesResponse;

use crate::prelude::*;

use super::catalog::FishCatalogPayload;

#[derive(Resource, Default)]
pub struct PendingRequests {
    pub meta: Option<Receiver<Result<MetaResponse, String>>>,
    pub layers: Option<Receiver<Result<LayersResponse, String>>>,
    pub zones: Option<Receiver<Result<ZonesResponse, String>>>,
    pub zone_stats: Option<(u32, Receiver<Result<ZoneStatsResponse, String>>)>,
    pub(crate) fish_catalog: Option<Receiver<Result<FishCatalogPayload, String>>>,
}
