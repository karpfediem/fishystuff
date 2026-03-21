use std::collections::HashMap;

use fishystuff_api::ids::{Rgb, RgbKey};
use fishystuff_api::models::zone_profile_v2::{
    ZoneAssignment, ZoneBorderAssessment, ZoneBorderClass, ZoneBorderMethod, ZoneClaimType,
    ZoneMetricAvailability, ZonePresenceState, ZonePublicState, ZoneRankingStatus,
    ZoneSourceFamily, ZoneSupportGrade,
};
use fishystuff_api::models::zone_stats::{
    ZoneConfidence, ZoneFishEvidence, ZoneStatsResponse, ZoneStatsWindow, ZoneStatus,
};
use fishystuff_api::models::zones::ZoneEntry;
use fishystuff_core::masks::ZoneMask;

use crate::store::queries;

use super::assignment::compute_zone_assignment;
use super::community_support::{
    CommunitySupportStatus, CommunityZoneFishSupport, CommunityZoneSupportSummary,
};
use super::legacy_support::{LegacyZoneFishSupport, LegacyZoneSupportSummary};
use super::response::build_zone_profile_v2_response;

fn default_assignment() -> ZoneAssignment {
    ZoneAssignment {
        zone_rgb_u32: 0x010203,
        zone_rgb: RgbKey("1,2,3".to_string()),
        zone_name: Some("Test Zone".to_string()),
        point: None,
        border: ZoneBorderAssessment {
            class: ZoneBorderClass::Unavailable,
            nearest_border_distance_px: None,
            method: ZoneBorderMethod::Unavailable,
            warnings: vec!["point coordinates were not provided".to_string()],
        },
        neighboring_zones: Vec::new(),
    }
}

fn zone_entries(entries: &[(u32, &str)]) -> HashMap<u32, ZoneEntry> {
    entries
        .iter()
        .map(|(rgb_u32, name)| {
            let rgb = Rgb::from_u32(*rgb_u32);
            (
                *rgb_u32,
                ZoneEntry {
                    rgb_u32: *rgb_u32,
                    rgb,
                    rgb_key: rgb.key(),
                    name: Some((*name).to_string()),
                },
            )
        })
        .collect()
}

fn solid_mask_rgb(width: u32, height: u32, rgb: [u8; 3]) -> Vec<u8> {
    let mut data = Vec::with_capacity((width * height * 3) as usize);
    for _ in 0..(width * height) {
        data.extend_from_slice(&rgb);
    }
    data
}

#[test]
fn zone_profile_v2_marks_missing_ranking_evidence_as_insufficient() {
    let profile = build_zone_profile_v2_response(
        default_assignment(),
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
        "v1",
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
    let profile = build_zone_profile_v2_response(
        default_assignment(),
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
        "v1",
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
    let profile = build_zone_profile_v2_response(
        default_assignment(),
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
        "v1",
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
    let profile = build_zone_profile_v2_response(
        default_assignment(),
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
        "v1",
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
}

#[test]
fn zone_profile_v2_merges_confirmed_community_support_without_blurring_ranking() {
    let profile = build_zone_profile_v2_response(
        default_assignment(),
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
        "v1",
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
    let profile = build_zone_profile_v2_response(
        default_assignment(),
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
        "v1",
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

#[test]
fn zone_profile_assignment_marks_core_when_no_neighbor_zone_is_seen() {
    let target = 0x010203;
    let mask = ZoneMask::from_rgb(5, 5, solid_mask_rgb(5, 5, [1, 2, 3])).expect("mask");
    let assignment = compute_zone_assignment(
        target,
        Rgb::from_u32(target).key(),
        Some("Core Zone".to_string()),
        Some(2),
        Some(2),
        Some(&mask),
        None,
        &zone_entries(&[(target, "Core Zone")]),
    );

    assert_eq!(assignment.border.class, ZoneBorderClass::Core);
    assert_eq!(
        assignment.border.method,
        ZoneBorderMethod::LocalNeighborhood
    );
    assert!(assignment.neighboring_zones.is_empty());
    assert!(assignment.border.nearest_border_distance_px.is_none());
}

#[test]
fn zone_profile_assignment_marks_near_border_with_single_neighbor() {
    let target = 0x010203;
    let neighbor = 0x040506;
    let mut data = solid_mask_rgb(5, 5, [1, 2, 3]);
    let idx = (2 * 5 + 3) * 3;
    data[idx] = 4;
    data[idx + 1] = 5;
    data[idx + 2] = 6;
    let mask = ZoneMask::from_rgb(5, 5, data).expect("mask");

    let assignment = compute_zone_assignment(
        target,
        Rgb::from_u32(target).key(),
        Some("Target Zone".to_string()),
        Some(2),
        Some(2),
        Some(&mask),
        None,
        &zone_entries(&[(target, "Target Zone"), (neighbor, "Neighbor Zone")]),
    );

    assert_eq!(assignment.border.class, ZoneBorderClass::NearBorder);
    assert_eq!(
        assignment.border.method,
        ZoneBorderMethod::LocalNeighborhood
    );
    assert_eq!(assignment.neighboring_zones.len(), 1);
    assert_eq!(assignment.neighboring_zones[0].zone_rgb_u32, neighbor);
}

#[test]
fn zone_profile_assignment_marks_ambiguous_when_point_samples_different_zone() {
    let target = 0x010203;
    let center = 0x040506;
    let mut data = solid_mask_rgb(5, 5, [1, 2, 3]);
    let idx = (2 * 5 + 2) * 3;
    data[idx] = 4;
    data[idx + 1] = 5;
    data[idx + 2] = 6;
    let mask = ZoneMask::from_rgb(5, 5, data).expect("mask");

    let assignment = compute_zone_assignment(
        target,
        Rgb::from_u32(target).key(),
        Some("Target Zone".to_string()),
        Some(2),
        Some(2),
        Some(&mask),
        None,
        &zone_entries(&[(target, "Target Zone"), (center, "Center Zone")]),
    );

    assert_eq!(assignment.border.class, ZoneBorderClass::Ambiguous);
    assert_eq!(
        assignment.border.method,
        ZoneBorderMethod::LocalNeighborhood
    );
    assert_eq!(assignment.neighboring_zones[0].zone_rgb_u32, center);
    assert!(assignment
        .border
        .warnings
        .iter()
        .any(|warning| warning.contains("samples zone RGB")));
}

#[test]
fn zone_profile_assignment_keeps_terrain_named_neighbors_when_present_in_mask() {
    let target = 0x010203;
    let terrain = 0x3c3c96;
    let mut data = solid_mask_rgb(5, 5, [1, 2, 3]);
    let idx = (2 * 5 + 3) * 3;
    data[idx] = 60;
    data[idx + 1] = 60;
    data[idx + 2] = 150;
    let mask = ZoneMask::from_rgb(5, 5, data).expect("mask");

    let assignment = compute_zone_assignment(
        target,
        Rgb::from_u32(target).key(),
        Some("Target Zone".to_string()),
        Some(2),
        Some(2),
        Some(&mask),
        None,
        &zone_entries(&[(target, "Target Zone"), (terrain, "Calpheon - Terrain")]),
    );

    assert_eq!(assignment.border.class, ZoneBorderClass::NearBorder);
    assert_eq!(assignment.neighboring_zones.len(), 1);
    assert_eq!(assignment.neighboring_zones[0].zone_rgb_u32, terrain);
}
