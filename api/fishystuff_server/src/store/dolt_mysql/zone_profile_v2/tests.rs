use fishystuff_api::ids::RgbKey;
use fishystuff_api::models::zone_profile_v2::{
    ZoneBorderClass, ZoneClaimType, ZoneMetricAvailability, ZonePresenceState,
    ZoneProfileV2Request, ZonePublicState, ZoneRankingStatus, ZoneSourceFamily, ZoneSupportGrade,
};
use fishystuff_api::models::zone_stats::{
    ZoneConfidence, ZoneFishEvidence, ZoneStatsResponse, ZoneStatsWindow, ZoneStatus,
};

use crate::store::queries;

use super::community_support::{
    CommunitySupportStatus, CommunityZoneFishSupport, CommunityZoneSupportSummary,
};
use super::legacy_support::{LegacyZoneFishSupport, LegacyZoneSupportSummary};
use super::response::build_zone_profile_v2_response;

fn zone_profile_request() -> ZoneProfileV2Request {
    ZoneProfileV2Request {
        layer_revision_id: None,
        layer_id: None,
        patch_id: None,
        at_ts_utc: None,
        map_version_id: None,
        rgb: RgbKey("1,2,3".to_string()),
        map_px_x: None,
        map_px_y: None,
        from_ts_utc: 1_700_000_000,
        to_ts_utc: 1_700_086_400,
        tile_px: 32,
        sigma_tiles: 3.0,
        fish_norm: false,
        alpha0: 1.0,
        top_k: 30,
        half_life_days: None,
        drift_boundary_ts_utc: None,
        ref_id: None,
        lang: None,
    }
}

#[test]
fn zone_profile_v2_marks_missing_ranking_evidence_as_insufficient() {
    let request = zone_profile_request();
    let profile = build_zone_profile_v2_response(
        &request,
        "v1",
        ZoneStatsResponse {
            zone_rgb_u32: 0x010203,
            zone_rgb: RgbKey("1,2,3".to_string()),
            zone_name: Some("Test Zone".to_string()),
            window: ZoneStatsWindow::default(),
            confidence: ZoneConfidence {
                ess: 0.0,
                total_weight: 0.0,
                last_seen_ts_utc: None,
                age_days_last: None,
                status: ZoneStatus::Unknown,
                notes: vec!["no evidence in window".to_string()],
                drift: None,
            },
            distribution: Vec::new(),
        },
        LegacyZoneSupportSummary::default(),
        CommunityZoneSupportSummary::default(),
    );

    assert_eq!(
        profile.presence_support.state,
        ZonePresenceState::InsufficientEvidence
    );
    assert!(profile.presence_support.fish.is_empty());
    assert_eq!(
        profile.assignment.border.class,
        ZoneBorderClass::Unavailable
    );
    assert_eq!(
        profile.diagnostics.public_state,
        ZonePublicState::InsufficientEvidence
    );
    assert!(profile.diagnostics.insufficient_evidence);
    assert_eq!(
        profile.catch_rates.availability,
        ZoneMetricAvailability::PendingSource
    );
    assert!(profile
        .presence_support
        .notes
        .iter()
        .any(|note| note.contains("not evidence of absence")));
}

#[test]
fn zone_profile_v2_keeps_ranking_support_separate_from_catch_rates() {
    let request = zone_profile_request();
    let profile = build_zone_profile_v2_response(
        &request,
        "v1",
        ZoneStatsResponse {
            zone_rgb_u32: 0x010203,
            zone_rgb: RgbKey("1,2,3".to_string()),
            zone_name: Some("Test Zone".to_string()),
            window: ZoneStatsWindow::default(),
            confidence: ZoneConfidence {
                ess: 12.0,
                total_weight: 4.0,
                last_seen_ts_utc: Some(1_700_080_000),
                age_days_last: Some(0.07),
                status: ZoneStatus::Fresh,
                notes: Vec::new(),
                drift: None,
            },
            distribution: vec![ZoneFishEvidence {
                fish_id: 8201,
                item_id: 8201,
                encyclopedia_key: Some(821001),
                encyclopedia_id: Some(8501),
                fish_name: Some("Mudskipper".to_string()),
                evidence_weight: 4.0,
                p_mean: 0.6,
                ci_low: Some(0.4),
                ci_high: Some(0.8),
            }],
        },
        LegacyZoneSupportSummary::default(),
        CommunityZoneSupportSummary::default(),
    );

    assert_eq!(
        profile.presence_support.evaluated_sources,
        vec![ZoneSourceFamily::Ranking]
    );
    assert_eq!(
        profile.presence_support.fish[0].source_badges,
        vec![ZoneSourceFamily::Ranking]
    );
    assert_eq!(
        profile.presence_support.fish[0].claims[0].claim_type,
        ZoneClaimType::PresenceObserved
    );
    assert_eq!(profile.ranking_evidence.status, ZoneRankingStatus::Fresh);
    assert_eq!(
        profile.ranking_evidence.fish[0].evidence_share_mean,
        Some(0.6)
    );
    assert_eq!(
        profile.catch_rates.availability,
        ZoneMetricAvailability::PendingSource
    );
    assert!(profile.catch_rates.fish.is_empty());
    assert_eq!(profile.diagnostics.public_state, ZonePublicState::Supported);
    assert!(profile
        .ranking_evidence
        .notes
        .iter()
        .any(|note| note.contains("not a catch/drop rate")));
}

#[test]
fn zone_profile_v2_merges_legacy_reference_support_without_blurring_ranking() {
    let request = zone_profile_request();
    let profile = build_zone_profile_v2_response(
        &request,
        "v1",
        ZoneStatsResponse {
            zone_rgb_u32: 0x010203,
            zone_rgb: RgbKey("1,2,3".to_string()),
            zone_name: Some("Test Zone".to_string()),
            window: ZoneStatsWindow::default(),
            confidence: ZoneConfidence {
                ess: 0.0,
                total_weight: 0.0,
                last_seen_ts_utc: None,
                age_days_last: None,
                status: ZoneStatus::Unknown,
                notes: vec!["no evidence in window".to_string()],
                drift: None,
            },
            distribution: Vec::new(),
        },
        LegacyZoneSupportSummary {
            evaluated: true,
            fish: vec![LegacyZoneFishSupport {
                item_id: 8201,
                encyclopedia_key: Some(821001),
                encyclopedia_id: Some(8501),
                fish_name: Some("Mudskipper".to_string()),
                aggregate_weight: 0.62,
            }],
            notes: vec!["legacy support evaluated".to_string()],
        },
        CommunityZoneSupportSummary::default(),
    );

    assert_eq!(profile.presence_support.state, ZonePresenceState::Supported);
    assert_eq!(
        profile.presence_support.evaluated_sources,
        vec![ZoneSourceFamily::Ranking, ZoneSourceFamily::Legacy]
    );
    assert_eq!(profile.presence_support.fish.len(), 1);
    assert_eq!(
        profile.presence_support.fish[0].support_grade,
        ZoneSupportGrade::ReferenceSupported
    );
    assert_eq!(
        profile.presence_support.fish[0].source_badges,
        vec![ZoneSourceFamily::Legacy]
    );
    assert_eq!(
        profile.presence_support.fish[0].claims[0].claim_type,
        ZoneClaimType::PresenceReferenced
    );
    assert!(profile.ranking_evidence.fish.is_empty());
    assert_eq!(profile.diagnostics.public_state, ZonePublicState::Supported);
    assert!(profile
        .diagnostics
        .notes
        .iter()
        .any(|note| note.contains("non-ranking support exists")));
}

#[test]
fn zone_profile_v2_keeps_ranking_and_legacy_claims_separate_for_same_fish() {
    let request = zone_profile_request();
    let profile = build_zone_profile_v2_response(
        &request,
        "v1",
        ZoneStatsResponse {
            zone_rgb_u32: 0x010203,
            zone_rgb: RgbKey("1,2,3".to_string()),
            zone_name: Some("Test Zone".to_string()),
            window: ZoneStatsWindow::default(),
            confidence: ZoneConfidence {
                ess: 12.0,
                total_weight: 4.0,
                last_seen_ts_utc: Some(1_700_080_000),
                age_days_last: Some(0.07),
                status: ZoneStatus::Fresh,
                notes: Vec::new(),
                drift: None,
            },
            distribution: vec![ZoneFishEvidence {
                fish_id: 8201,
                item_id: 8201,
                encyclopedia_key: Some(821001),
                encyclopedia_id: Some(8501),
                fish_name: Some("Mudskipper".to_string()),
                evidence_weight: 4.0,
                p_mean: 0.6,
                ci_low: Some(0.4),
                ci_high: Some(0.8),
            }],
        },
        LegacyZoneSupportSummary {
            evaluated: true,
            fish: vec![LegacyZoneFishSupport {
                item_id: 8201,
                encyclopedia_key: Some(821001),
                encyclopedia_id: Some(8501),
                fish_name: Some("Mudskipper".to_string()),
                aggregate_weight: 0.62,
            }],
            notes: Vec::new(),
        },
        CommunityZoneSupportSummary::default(),
    );

    assert_eq!(profile.presence_support.fish.len(), 1);
    assert_eq!(
        profile.presence_support.fish[0].source_badges,
        vec![ZoneSourceFamily::Ranking, ZoneSourceFamily::Legacy]
    );
    let claim_types = profile.presence_support.fish[0]
        .claims
        .iter()
        .map(|claim| claim.claim_type.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        claim_types,
        vec![
            ZoneClaimType::PresenceObserved,
            ZoneClaimType::PresenceReferenced
        ]
    );
    assert_eq!(
        profile.presence_support.fish[0].support_grade,
        ZoneSupportGrade::ObservedRecent
    );
    assert_eq!(profile.ranking_evidence.fish.len(), 1);
    assert!(profile
        .ranking_evidence
        .notes
        .iter()
        .any(|note| note.contains("presence_support")));
}

#[test]
fn ranking_events_query_is_source_filtered() {
    assert!(queries::RANKING_EVENTS_WITH_ZONE_SQL.contains("e.source_kind = ?"));
    assert!(queries::RANKING_EVENTS_WITH_RING_SUPPORT_SQL.contains("e.source_kind = ?"));
}

#[test]
fn zone_profile_v2_merges_confirmed_community_support_without_blurring_ranking() {
    let request = zone_profile_request();
    let profile = build_zone_profile_v2_response(
        &request,
        "v1",
        ZoneStatsResponse {
            zone_rgb_u32: 0x010203,
            zone_rgb: RgbKey("1,2,3".to_string()),
            zone_name: Some("Test Zone".to_string()),
            window: ZoneStatsWindow::default(),
            confidence: ZoneConfidence {
                ess: 0.0,
                total_weight: 0.0,
                last_seen_ts_utc: None,
                age_days_last: None,
                status: ZoneStatus::Unknown,
                notes: vec!["no evidence in window".to_string()],
                drift: None,
            },
            distribution: Vec::new(),
        },
        LegacyZoneSupportSummary::default(),
        CommunityZoneSupportSummary {
            evaluated: true,
            fish: vec![CommunityZoneFishSupport {
                item_id: 8201,
                fish_name: Some("Mudskipper".to_string()),
                status: CommunitySupportStatus::Confirmed,
                claim_count: 2,
            }],
            notes: vec!["community support evaluated".to_string()],
        },
    );

    assert_eq!(profile.presence_support.state, ZonePresenceState::Supported);
    assert_eq!(
        profile.presence_support.evaluated_sources,
        vec![ZoneSourceFamily::Ranking, ZoneSourceFamily::Community]
    );
    assert_eq!(
        profile.presence_support.fish[0].support_grade,
        ZoneSupportGrade::ReferenceSupported
    );
    assert_eq!(
        profile.presence_support.fish[0].source_badges,
        vec![ZoneSourceFamily::Community]
    );
    assert_eq!(
        profile.presence_support.fish[0].claims[0].claim_type,
        ZoneClaimType::PresenceReferenced
    );
    assert!(profile.ranking_evidence.fish.is_empty());
    assert_eq!(profile.diagnostics.public_state, ZonePublicState::Supported);
}

#[test]
fn zone_profile_v2_maps_unconfirmed_community_rows_to_weak_hint() {
    let request = zone_profile_request();
    let profile = build_zone_profile_v2_response(
        &request,
        "v1",
        ZoneStatsResponse {
            zone_rgb_u32: 0x010203,
            zone_rgb: RgbKey("1,2,3".to_string()),
            zone_name: Some("Test Zone".to_string()),
            window: ZoneStatsWindow::default(),
            confidence: ZoneConfidence {
                ess: 0.0,
                total_weight: 0.0,
                last_seen_ts_utc: None,
                age_days_last: None,
                status: ZoneStatus::Unknown,
                notes: vec!["no evidence in window".to_string()],
                drift: None,
            },
            distribution: Vec::new(),
        },
        LegacyZoneSupportSummary::default(),
        CommunityZoneSupportSummary {
            evaluated: true,
            fish: vec![CommunityZoneFishSupport {
                item_id: 8202,
                fish_name: Some("Sea Eel".to_string()),
                status: CommunitySupportStatus::Unconfirmed,
                claim_count: 1,
            }],
            notes: Vec::new(),
        },
    );

    assert_eq!(profile.presence_support.fish.len(), 1);
    assert_eq!(
        profile.presence_support.fish[0].support_grade,
        ZoneSupportGrade::WeakHint
    );
    assert_eq!(
        profile.presence_support.fish[0].source_badges,
        vec![ZoneSourceFamily::Community]
    );
    assert!(profile.presence_support.fish[0].claims[0]
        .confidence_note
        .as_deref()
        .is_some_and(|note| note.contains("unconfirmed")));
}
