use std::collections::HashSet;

use bevy::prelude::*;
use fishystuff_api::models::events::EventPointCompact;

use crate::map::events::{EventZoneSetResolver, EventsSnapshotState};
use crate::map::layers::LayerRegistry;
use crate::plugins::api::{
    CommunityFishZoneSupportIndex, FishCatalog, FishFilterState, LayerEffectiveFilterState,
    LayerFilterBindingOverrideState, PatchFilterState, SemanticFieldFilterState,
};

use super::{normalized_time_and_fish_filters, EvidenceZoneFilter};

fn apply_zone_filter_state(
    filter: &mut EvidenceZoneFilter,
    next_active: bool,
    next_zone_rgbs: HashSet<u32>,
) {
    if filter.active != next_active || filter.zone_rgbs != next_zone_rgbs {
        filter.active = next_active;
        filter.zone_rgbs = next_zone_rgbs;
        filter.revision = filter.revision.wrapping_add(1);
    }
}

pub(super) fn collect_evidence_zone_rgbs(
    events: &[EventPointCompact],
    from_ts_utc: Option<i64>,
    to_ts_utc: Option<i64>,
    fish_ids: &[i32],
    resolver: &mut EventZoneSetResolver,
) -> (HashSet<u32>, bool, usize) {
    let mut zones = HashSet::new();
    let mut has_zone_data = false;
    let mut matched_events = 0usize;

    for event in events {
        if from_ts_utc.is_some_and(|from_ts_utc| event.ts_utc < from_ts_utc)
            || to_ts_utc.is_some_and(|to_ts_utc| event.ts_utc >= to_ts_utc)
        {
            continue;
        }
        if !fish_ids.is_empty() && fish_ids.binary_search(&event.fish_id).is_err() {
            continue;
        }
        matched_events = matched_events.saturating_add(1);
        let event_zone_rgbs = resolver.full_zone_rgbs(event);
        if !event_zone_rgbs.is_empty() {
            has_zone_data = true;
            zones.extend(event_zone_rgbs.iter().copied());
        }
    }

    (zones, has_zone_data, matched_events)
}

fn collect_community_zone_rgbs(
    fish_ids: &[i32],
    fish_catalog: &FishCatalog,
    community: &CommunityFishZoneSupportIndex,
) -> HashSet<u32> {
    let mut zones = HashSet::new();
    for fish_id in fish_ids {
        let Some(item_id) = fish_catalog.item_id_for_fish(*fish_id) else {
            continue;
        };
        zones.extend(community.zone_rgbs_for_item(item_id).iter().copied());
    }
    zones
}

pub(crate) fn sync_evidence_zone_filter(
    patch_filter: Res<PatchFilterState>,
    fish_filter: Res<FishFilterState>,
    fish_catalog: Res<FishCatalog>,
    community: Res<CommunityFishZoneSupportIndex>,
    snapshot: Res<EventsSnapshotState>,
    mut filter: ResMut<EvidenceZoneFilter>,
) {
    let active_terms = !fish_filter.selected_fish_ids.is_empty();
    if !active_terms {
        apply_zone_filter_state(filter.as_mut(), false, HashSet::new());
        return;
    }

    let Some((from_ts_utc, to_ts_utc, mut fish_ids)) =
        normalized_time_and_fish_filters(&patch_filter, &fish_filter)
    else {
        apply_zone_filter_state(filter.as_mut(), false, HashSet::new());
        return;
    };
    fish_ids.sort_unstable();
    fish_ids.dedup();
    let mut next_zone_rgbs = collect_community_zone_rgbs(&fish_ids, &fish_catalog, &community);

    let mut resolver = EventZoneSetResolver::new();
    if snapshot.loaded {
        let (ranking_zone_rgbs, _, _) = collect_evidence_zone_rgbs(
            &snapshot.events,
            from_ts_utc,
            to_ts_utc,
            &fish_ids,
            &mut resolver,
        );
        next_zone_rgbs.extend(ranking_zone_rgbs);
    }
    apply_zone_filter_state(filter.as_mut(), true, next_zone_rgbs);
}

pub(crate) fn sync_layer_effective_filters(
    layer_registry: Res<LayerRegistry>,
    binding_overrides: Res<LayerFilterBindingOverrideState>,
    semantic_filter: Res<SemanticFieldFilterState>,
    fish_selection_filter: Res<EvidenceZoneFilter>,
    mut effective_filters: ResMut<LayerEffectiveFilterState>,
) {
    effective_filters.sync_to_registry(&layer_registry);
    for layer in layer_registry.ordered() {
        effective_filters.resolve_zone_membership_filter_for_layer(
            layer,
            &binding_overrides,
            fish_selection_filter.as_ref(),
            &semantic_filter,
        );
        effective_filters.resolve_semantic_field_filter_for_layer(
            layer,
            &binding_overrides,
            &semantic_filter,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use fishystuff_api::models::events::EventPointCompact;
    use fishystuff_api::models::fish::{
        CommunityFishZoneSupportEntry, CommunityFishZoneSupportResponse,
    };

    use super::{collect_community_zone_rgbs, sync_evidence_zone_filter};
    use crate::map::events::EventsSnapshotState;
    use crate::plugins::api::{
        CommunityFishZoneSupportIndex, FishCatalog, FishEntry, FishFilterState, PatchFilterState,
    };
    use crate::plugins::points::EvidenceZoneFilter;
    use crate::prelude::*;

    #[test]
    fn collect_community_zone_rgbs_maps_selected_fish_to_item_ids() {
        let mut fish_catalog = FishCatalog::default();
        fish_catalog.replace(vec![FishEntry {
            id: 240,
            item_id: 820240,
            encyclopedia_key: Some(240),
            encyclopedia_id: Some(9240),
            name: "Blobfish".to_string(),
            name_lower: "blobfish".to_string(),
            grade: Some("Rare".to_string()),
            is_prize: false,
        }]);
        let mut community = CommunityFishZoneSupportIndex::default();
        community.replace_from_response(CommunityFishZoneSupportResponse {
            revision: "community-rev".to_string(),
            fish: vec![CommunityFishZoneSupportEntry {
                item_id: 820240,
                zone_rgbs: vec![0x112233, 0x445566],
            }],
            ..CommunityFishZoneSupportResponse::default()
        });

        let zones = collect_community_zone_rgbs(&[240], &fish_catalog, &community);

        assert_eq!(zones, HashSet::from([0x112233, 0x445566]));
    }

    #[test]
    fn sync_evidence_zone_filter_uses_community_support_without_snapshot() {
        let mut app = App::new();
        app.init_resource::<PatchFilterState>()
            .init_resource::<FishFilterState>()
            .init_resource::<FishCatalog>()
            .init_resource::<CommunityFishZoneSupportIndex>()
            .init_resource::<EventsSnapshotState>()
            .init_resource::<EvidenceZoneFilter>()
            .add_systems(Update, sync_evidence_zone_filter);

        {
            let mut fish_filter = app.world_mut().resource_mut::<FishFilterState>();
            fish_filter.selected_fish_ids = vec![240];
        }
        {
            let mut fish_catalog = app.world_mut().resource_mut::<FishCatalog>();
            fish_catalog.replace(vec![FishEntry {
                id: 240,
                item_id: 820240,
                encyclopedia_key: Some(240),
                encyclopedia_id: Some(9240),
                name: "Blobfish".to_string(),
                name_lower: "blobfish".to_string(),
                grade: Some("Rare".to_string()),
                is_prize: false,
            }]);
        }
        {
            let mut community = app
                .world_mut()
                .resource_mut::<CommunityFishZoneSupportIndex>();
            community.replace_from_response(CommunityFishZoneSupportResponse {
                revision: "community-rev".to_string(),
                fish: vec![CommunityFishZoneSupportEntry {
                    item_id: 820240,
                    zone_rgbs: vec![0x112233, 0x445566],
                }],
                ..CommunityFishZoneSupportResponse::default()
            });
        }

        app.update();

        let filter = app.world().resource::<EvidenceZoneFilter>();
        assert!(filter.active);
        assert_eq!(filter.zone_rgbs, HashSet::from([0x112233, 0x445566]));
    }

    #[test]
    fn sync_evidence_zone_filter_unions_community_and_ranking_support() {
        let mut app = App::new();
        app.init_resource::<PatchFilterState>()
            .init_resource::<FishFilterState>()
            .init_resource::<FishCatalog>()
            .init_resource::<CommunityFishZoneSupportIndex>()
            .init_resource::<EvidenceZoneFilter>();
        app.insert_resource(EventsSnapshotState {
            loaded: true,
            events: vec![EventPointCompact {
                event_id: 1,
                fish_id: 240,
                ts_utc: 100,
                map_px_x: 0,
                map_px_y: 0,
                length_milli: 1,
                world_x: None,
                world_z: None,
                zone_rgb_u32: None,
                zone_rgbs: vec![0x778899],
                full_zone_rgbs: vec![0x778899],
                source_kind: None,
                source_id: None,
            }],
            ..EventsSnapshotState::default()
        });
        app.add_systems(Update, sync_evidence_zone_filter);

        {
            let mut fish_filter = app.world_mut().resource_mut::<FishFilterState>();
            fish_filter.selected_fish_ids = vec![240];
        }
        {
            let mut fish_catalog = app.world_mut().resource_mut::<FishCatalog>();
            fish_catalog.replace(vec![FishEntry {
                id: 240,
                item_id: 820240,
                encyclopedia_key: Some(240),
                encyclopedia_id: Some(9240),
                name: "Blobfish".to_string(),
                name_lower: "blobfish".to_string(),
                grade: Some("Rare".to_string()),
                is_prize: false,
            }]);
        }
        {
            let mut community = app
                .world_mut()
                .resource_mut::<CommunityFishZoneSupportIndex>();
            community.replace_from_response(CommunityFishZoneSupportResponse {
                revision: "community-rev".to_string(),
                fish: vec![CommunityFishZoneSupportEntry {
                    item_id: 820240,
                    zone_rgbs: vec![0x112233],
                }],
                ..CommunityFishZoneSupportResponse::default()
            });
        }

        app.update();

        let filter = app.world().resource::<EvidenceZoneFilter>();
        assert!(filter.active);
        assert_eq!(filter.zone_rgbs, HashSet::from([0x112233, 0x778899]));
    }

    #[test]
    fn sync_evidence_zone_filter_omits_partial_only_ranking_zones() {
        let mut app = App::new();
        app.init_resource::<PatchFilterState>()
            .init_resource::<FishFilterState>()
            .init_resource::<FishCatalog>()
            .init_resource::<CommunityFishZoneSupportIndex>()
            .init_resource::<EvidenceZoneFilter>();
        app.insert_resource(EventsSnapshotState {
            loaded: true,
            events: vec![EventPointCompact {
                event_id: 1,
                fish_id: 240,
                ts_utc: 100,
                map_px_x: 0,
                map_px_y: 0,
                length_milli: 1,
                world_x: None,
                world_z: None,
                zone_rgb_u32: Some(0x778899),
                zone_rgbs: vec![0x778899],
                full_zone_rgbs: Vec::new(),
                source_kind: None,
                source_id: None,
            }],
            ..EventsSnapshotState::default()
        });
        app.add_systems(Update, sync_evidence_zone_filter);

        {
            let mut fish_filter = app.world_mut().resource_mut::<FishFilterState>();
            fish_filter.selected_fish_ids = vec![240];
        }
        {
            let mut fish_catalog = app.world_mut().resource_mut::<FishCatalog>();
            fish_catalog.replace(vec![FishEntry {
                id: 240,
                item_id: 820240,
                encyclopedia_key: Some(240),
                encyclopedia_id: Some(9240),
                name: "Blobfish".to_string(),
                name_lower: "blobfish".to_string(),
                grade: Some("Rare".to_string()),
                is_prize: false,
            }]);
        }

        app.update();

        let filter = app.world().resource::<EvidenceZoneFilter>();
        assert!(filter.active);
        assert!(filter.zone_rgbs.is_empty());
    }
}
