mod community_support;
mod legacy_support;
mod response;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use fishystuff_api::models::zone_profile_v2::{ZoneProfileV2Request, ZoneProfileV2Response};

use crate::config::ZoneStatusConfig;
use crate::error::{AppError, AppResult};
use crate::store::FishLang;

use super::{DoltMySqlStore, QueryParams};
use response::build_zone_profile_v2_response;

impl DoltMySqlStore {
    pub(super) fn compute_zone_profile_v2(
        &self,
        request: ZoneProfileV2Request,
        status_cfg: ZoneStatusConfig,
    ) -> AppResult<ZoneProfileV2Response> {
        let zone_rgb_u32 = request.rgb.to_u32().map_err(AppError::invalid_argument)?;
        let layer_id = request
            .layer_id
            .as_deref()
            .or(Some(super::ZONE_MASK_LAYER_ID));
        let layer_revision_id = self.resolve_layer_revision_id(
            request.layer_revision_id.as_deref(),
            request.map_version_id.as_ref(),
            layer_id,
            request.patch_id.as_deref(),
            request.at_ts_utc,
            request.to_ts_utc,
        )?;

        let params = QueryParams {
            map_version: layer_revision_id.clone(),
            from_ts_utc: request.from_ts_utc,
            to_ts_utc: request.to_ts_utc,
            half_life_days: request.half_life_days,
            tile_px: request.tile_px,
            sigma_tiles: request.sigma_tiles,
            fish_norm: request.fish_norm,
            alpha0: request.alpha0,
            top_k: request.top_k,
            drift_boundary_ts: request.drift_boundary_ts_utc,
        };
        params.validate()?;

        let lang = FishLang::from_param(request.lang.as_deref());
        let fish_names = self.query_fish_names(&lang, request.ref_id.as_deref())?;
        let fish_table = self.query_fish_identities(request.ref_id.as_deref())?;
        let zones_vec = self.query_zones(request.ref_id.as_deref())?;
        let zones: HashMap<u32, fishystuff_api::models::zones::ZoneEntry> = zones_vec
            .into_iter()
            .map(|zone| (zone.rgb_u32, zone))
            .collect();
        let event_fish_names = DoltMySqlStore::build_event_fish_names(&fish_names, &fish_table);
        let event_fish_identities = DoltMySqlStore::build_event_fish_identity_map(&fish_table);
        let zone_stats = self.compute_zone_stats(
            &zones,
            &event_fish_names,
            &event_fish_identities,
            &params,
            zone_rgb_u32,
            &status_cfg,
        )?;
        let legacy_support = self.query_legacy_zone_support(
            request.rgb.as_rgb().map_err(AppError::invalid_argument)?,
            request.ref_id.as_deref(),
            &event_fish_names,
            &event_fish_identities,
        )?;
        let community_support =
            self.query_community_zone_support(zone_rgb_u32, request.ref_id.as_deref())?;

        Ok(build_zone_profile_v2_response(
            &request,
            &layer_revision_id,
            zone_stats,
            legacy_support,
            community_support,
        ))
    }
}
