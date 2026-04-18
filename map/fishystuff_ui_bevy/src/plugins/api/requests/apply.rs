use fishystuff_api::models::meta::MetaResponse;

use super::super::state::{ApiBootstrapState, PatchFilterState};
use super::util::pick_map_version;

pub(super) fn apply_meta_response(
    bootstrap: &mut ApiBootstrapState,
    patch_filter: &mut PatchFilterState,
    meta: MetaResponse,
) {
    let map_version = pick_map_version(&meta);
    if map_version != bootstrap.map_version {
        bootstrap.map_version_dirty = true;
        bootstrap.layers_loaded_map_version = None;
    }
    bootstrap.meta_status = "meta: loaded".to_string();
    bootstrap.defaults = Some(meta.defaults.clone());
    bootstrap.map_version = map_version;
    patch_filter.patches = meta.patches.clone();
    bootstrap.meta = Some(meta);
}

#[cfg(test)]
mod tests {
    use super::apply_meta_response;
    use crate::plugins::api::{ApiBootstrapState, PatchFilterState};
    use fishystuff_api::ids::PatchId;
    use fishystuff_api::models::meta::{CanonicalMapInfo, MetaDefaults, MetaResponse, PatchInfo};

    fn patch(patch_id: &str, start_ts_utc: i64) -> PatchInfo {
        PatchInfo {
            patch_id: PatchId(patch_id.to_string()),
            start_ts_utc,
            patch_name: None,
        }
    }

    fn meta_with_patches() -> MetaResponse {
        MetaResponse {
            canonical_map: CanonicalMapInfo {
                image_size_x: 4096,
                image_size_y: 2048,
                ..CanonicalMapInfo::default()
            },
            defaults: MetaDefaults::default(),
            patches: vec![patch("2026-03-12", 100), patch("2026-04-24", 200)],
            ..MetaResponse::default()
        }
    }

    #[test]
    fn apply_meta_response_does_not_seed_patch_range_defaults() {
        let mut bootstrap = ApiBootstrapState::default();
        let mut patch_filter = PatchFilterState::default();

        apply_meta_response(&mut bootstrap, &mut patch_filter, meta_with_patches());

        assert_eq!(patch_filter.from_ts, None);
        assert_eq!(patch_filter.to_ts, None);
        assert_eq!(patch_filter.selected_patch, None);
        assert_eq!(patch_filter.patches.len(), 2);
    }

    #[test]
    fn apply_meta_response_preserves_existing_explicit_patch_range() {
        let mut bootstrap = ApiBootstrapState::default();
        let mut patch_filter = PatchFilterState {
            from_ts: Some(150),
            to_ts: Some(250),
            selected_patch: Some("2026-03-12".to_string()),
            ..PatchFilterState::default()
        };

        apply_meta_response(&mut bootstrap, &mut patch_filter, meta_with_patches());

        assert_eq!(patch_filter.from_ts, Some(150));
        assert_eq!(patch_filter.to_ts, Some(250));
        assert_eq!(patch_filter.selected_patch.as_deref(), Some("2026-03-12"));
        assert_eq!(patch_filter.patches.len(), 2);
    }
}
