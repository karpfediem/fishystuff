use std::collections::BTreeMap;

use fishystuff_api::models::zone_profile_v2::{
    ZoneAssignment, ZoneBorderAssessment, ZoneBorderClass, ZoneBorderMethod, ZoneCatchRateSummary,
    ZoneClaimType, ZoneDiagnostics, ZoneFishSupport, ZoneMetricAvailability, ZonePoint,
    ZonePresenceState, ZonePresenceSupport, ZoneProfileV2Request, ZoneProfileV2Response,
    ZonePublicState, ZoneRankingDrift, ZoneRankingEvidence, ZoneRankingFishEvidence,
    ZoneRankingShareKind, ZoneRankingStatus, ZoneSourceFamily, ZoneSupportClaim, ZoneSupportGrade,
};
use fishystuff_api::models::zone_stats::{DriftInfo, ZoneStatsResponse, ZoneStatus};

use super::community_support::{CommunitySupportStatus, CommunityZoneSupportSummary};
use super::legacy_support::LegacyZoneSupportSummary;

fn ranking_status_from_zone_status(status: ZoneStatus) -> ZoneRankingStatus {
    match status {
        ZoneStatus::Fresh => ZoneRankingStatus::Fresh,
        ZoneStatus::Stale => ZoneRankingStatus::Stale,
        ZoneStatus::Drifting => ZoneRankingStatus::Drifting,
        ZoneStatus::Unknown => ZoneRankingStatus::Unknown,
    }
}

fn support_grade_from_zone_status(status: ZoneStatus) -> ZoneSupportGrade {
    match status {
        ZoneStatus::Fresh | ZoneStatus::Drifting => ZoneSupportGrade::ObservedRecent,
        ZoneStatus::Stale => ZoneSupportGrade::ObservedHistorical,
        ZoneStatus::Unknown => ZoneSupportGrade::Unknown,
    }
}

fn support_grade_priority(grade: &ZoneSupportGrade) -> u8 {
    match grade {
        ZoneSupportGrade::ObservedRecent => 5,
        ZoneSupportGrade::ObservedHistorical => 4,
        ZoneSupportGrade::ReferenceSupported => 3,
        ZoneSupportGrade::WeakHint => 2,
        ZoneSupportGrade::InsufficientEvidence => 1,
        ZoneSupportGrade::Unknown => 0,
    }
}

fn merge_support_grade(
    current: &ZoneSupportGrade,
    incoming: &ZoneSupportGrade,
) -> ZoneSupportGrade {
    if support_grade_priority(incoming) > support_grade_priority(current) {
        incoming.clone()
    } else {
        current.clone()
    }
}

fn add_source_badge(target: &mut Vec<ZoneSourceFamily>, source_family: ZoneSourceFamily) {
    if !target.contains(&source_family) {
        target.push(source_family);
    }
}

fn public_state_from_zone_stats(
    zone_stats: &ZoneStatsResponse,
    has_nonranking_support: bool,
) -> ZonePublicState {
    if zone_stats.distribution.is_empty() || zone_stats.confidence.total_weight <= 0.0 {
        return if has_nonranking_support {
            ZonePublicState::Supported
        } else {
            ZonePublicState::InsufficientEvidence
        };
    }

    match zone_stats.confidence.status {
        ZoneStatus::Fresh => ZonePublicState::Supported,
        ZoneStatus::Stale => ZonePublicState::Stale,
        ZoneStatus::Drifting => ZonePublicState::Drifting,
        ZoneStatus::Unknown => ZonePublicState::Unknown,
    }
}

fn map_drift_info(drift: DriftInfo) -> ZoneRankingDrift {
    ZoneRankingDrift {
        boundary_ts_utc: drift.boundary_ts_utc,
        jsd_mean: drift.jsd_mean,
        p_drift: drift.p_drift,
        ess_old: drift.ess_old,
        ess_new: drift.ess_new,
        samples: drift.samples,
        jsd_threshold: drift.jsd_threshold,
    }
}

fn support_grade_from_community_status(status: CommunitySupportStatus) -> ZoneSupportGrade {
    match status {
        CommunitySupportStatus::Confirmed | CommunitySupportStatus::Guessed => {
            ZoneSupportGrade::ReferenceSupported
        }
        CommunitySupportStatus::Unconfirmed | CommunitySupportStatus::DataIncomplete => {
            ZoneSupportGrade::WeakHint
        }
    }
}

fn support_note_from_community_status(status: CommunitySupportStatus) -> &'static str {
    match status {
        CommunitySupportStatus::Confirmed => {
            "supported by curated community zone data for this zone"
        }
        CommunitySupportStatus::Guessed => {
            "supported by curated community guessed-rate data for this zone"
        }
        CommunitySupportStatus::Unconfirmed => {
            "listed in curated community zone data as an unconfirmed fish for this zone"
        }
        CommunitySupportStatus::DataIncomplete => {
            "listed in curated community zone data for an incomplete/uncertain zone row"
        }
    }
}

pub(super) fn build_zone_profile_v2_response(
    request: &ZoneProfileV2Request,
    layer_revision_id: &str,
    zone_stats: ZoneStatsResponse,
    legacy_support: LegacyZoneSupportSummary,
    community_support: CommunityZoneSupportSummary,
) -> ZoneProfileV2Response {
    struct PresenceAccumulator {
        fish: ZoneFishSupport,
        ranking_rank: Option<usize>,
        legacy_weight: f64,
    }

    let LegacyZoneSupportSummary {
        evaluated: legacy_evaluated,
        fish: legacy_fish_rows,
        notes: legacy_notes,
    } = legacy_support;
    let CommunityZoneSupportSummary {
        evaluated: community_evaluated,
        fish: community_fish_rows,
        notes: community_notes,
    } = community_support;

    let point = match (request.map_px_x, request.map_px_y) {
        (Some(map_px_x), Some(map_px_y)) => Some(ZonePoint { map_px_x, map_px_y }),
        _ => None,
    };
    let point_incomplete = request.map_px_x.is_some() ^ request.map_px_y.is_some();

    let mut border_warnings = vec![
        "border distance and neighboring-zone analysis are not yet implemented for zone_profile_v2"
            .to_string(),
    ];
    if point_incomplete {
        border_warnings
            .push("point coordinates were incomplete; assignment.point was omitted".to_string());
    } else if point.is_none() {
        border_warnings.push("point coordinates were not provided".to_string());
    }

    let ranking_support_grade =
        support_grade_from_zone_status(zone_stats.confidence.status.clone());
    let has_legacy_support = !legacy_fish_rows.is_empty();
    let has_community_support = !community_fish_rows.is_empty();
    let mut presence_notes = if zone_stats.distribution.is_empty() {
        vec![
            "ranking evidence was checked for the selected window, but no positive evidence was found"
                .to_string(),
            "missing ranking evidence is not evidence of absence".to_string(),
        ]
    } else {
        vec![
            "presence support in this slice combines ranking observations with legacy fishing-table references"
                .to_string(),
        ]
    };
    if has_legacy_support {
        presence_notes.push(
            "legacy fishing tables provide reference support for fish resolved from the zone RGB and group tables"
                .to_string(),
        );
    }
    if has_community_support {
        presence_notes.push(
            "community support provides curated zone/fish claims alongside ranking and legacy sources"
                .to_string(),
        );
    }
    presence_notes.extend(legacy_notes);
    presence_notes.extend(community_notes);
    if !community_evaluated {
        presence_notes.push(
            "community support is unavailable in the current runtime until the imported support table is populated"
                .to_string(),
        );
    }
    presence_notes.push("player-log source families remain placeholders in this slice".to_string());

    let mut presence_by_item: BTreeMap<i32, PresenceAccumulator> = BTreeMap::new();
    for (ranking_rank, fish) in zone_stats.distribution.iter().enumerate() {
        presence_by_item.insert(
            fish.item_id,
            PresenceAccumulator {
                fish: ZoneFishSupport {
                    fish_id: fish.fish_id,
                    item_id: fish.item_id,
                    encyclopedia_key: fish.encyclopedia_key,
                    encyclopedia_id: fish.encyclopedia_id,
                    fish_name: fish.fish_name.clone(),
                    support_grade: ranking_support_grade.clone(),
                    source_badges: vec![ZoneSourceFamily::Ranking],
                    claims: vec![ZoneSupportClaim {
                        source_family: ZoneSourceFamily::Ranking,
                        claim_type: ZoneClaimType::PresenceObserved,
                        confidence_note: Some(
                            "observed in ranking data for the selected time window".to_string(),
                        ),
                        observed_at_ts_utc: zone_stats.confidence.last_seen_ts_utc,
                        source_revision: Some(format!("layer_revision:{layer_revision_id}")),
                    }],
                },
                ranking_rank: Some(ranking_rank),
                legacy_weight: 0.0,
            },
        );
    }

    for legacy_fish in legacy_fish_rows {
        let legacy_claim = ZoneSupportClaim {
            source_family: ZoneSourceFamily::Legacy,
            claim_type: ZoneClaimType::PresenceReferenced,
            confidence_note: Some(
                "supported by legacy fishing tables for this zone via group-table resolution"
                    .to_string(),
            ),
            observed_at_ts_utc: None,
            source_revision: request
                .ref_id
                .as_ref()
                .map(|ref_id| format!("legacy_ref:{ref_id}"))
                .or_else(|| Some("legacy_runtime_tables".to_string())),
        };

        if let Some(accumulator) = presence_by_item.get_mut(&legacy_fish.item_id) {
            accumulator.legacy_weight += legacy_fish.aggregate_weight;
            accumulator.fish.support_grade = merge_support_grade(
                &accumulator.fish.support_grade,
                &ZoneSupportGrade::ReferenceSupported,
            );
            add_source_badge(
                &mut accumulator.fish.source_badges,
                ZoneSourceFamily::Legacy,
            );
            accumulator.fish.claims.push(legacy_claim);
            if accumulator.fish.fish_name.is_none() {
                accumulator.fish.fish_name = legacy_fish.fish_name.clone();
            }
            if accumulator.fish.encyclopedia_key.is_none() {
                accumulator.fish.encyclopedia_key = legacy_fish.encyclopedia_key;
            }
            if accumulator.fish.encyclopedia_id.is_none() {
                accumulator.fish.encyclopedia_id = legacy_fish.encyclopedia_id;
            }
        } else {
            presence_by_item.insert(
                legacy_fish.item_id,
                PresenceAccumulator {
                    fish: ZoneFishSupport {
                        fish_id: legacy_fish.item_id,
                        item_id: legacy_fish.item_id,
                        encyclopedia_key: legacy_fish.encyclopedia_key,
                        encyclopedia_id: legacy_fish.encyclopedia_id,
                        fish_name: legacy_fish.fish_name,
                        support_grade: ZoneSupportGrade::ReferenceSupported,
                        source_badges: vec![ZoneSourceFamily::Legacy],
                        claims: vec![legacy_claim],
                    },
                    ranking_rank: None,
                    legacy_weight: legacy_fish.aggregate_weight,
                },
            );
        }
    }

    for community_fish in community_fish_rows {
        let community_claim = ZoneSupportClaim {
            source_family: ZoneSourceFamily::Community,
            claim_type: ZoneClaimType::PresenceReferenced,
            confidence_note: Some(
                support_note_from_community_status(community_fish.status).to_string(),
            ),
            observed_at_ts_utc: None,
            source_revision: request
                .ref_id
                .as_ref()
                .map(|ref_id| format!("community_ref:{ref_id}"))
                .or_else(|| Some("community_zone_fish_support".to_string())),
        };

        if let Some(accumulator) = presence_by_item.get_mut(&community_fish.item_id) {
            accumulator.fish.support_grade = merge_support_grade(
                &accumulator.fish.support_grade,
                &support_grade_from_community_status(community_fish.status),
            );
            add_source_badge(
                &mut accumulator.fish.source_badges,
                ZoneSourceFamily::Community,
            );
            accumulator.fish.claims.push(community_claim);
            if accumulator.fish.fish_name.is_none() {
                accumulator.fish.fish_name = community_fish.fish_name.clone();
            }
        } else {
            presence_by_item.insert(
                community_fish.item_id,
                PresenceAccumulator {
                    fish: ZoneFishSupport {
                        fish_id: community_fish.item_id,
                        item_id: community_fish.item_id,
                        encyclopedia_key: None,
                        encyclopedia_id: None,
                        fish_name: community_fish.fish_name,
                        support_grade: support_grade_from_community_status(community_fish.status),
                        source_badges: vec![ZoneSourceFamily::Community],
                        claims: vec![community_claim],
                    },
                    ranking_rank: None,
                    legacy_weight: 0.0,
                },
            );
        }
    }

    let mut presence_fish = presence_by_item.into_values().collect::<Vec<_>>();
    presence_fish.sort_by(|left, right| {
        left.ranking_rank
            .unwrap_or(usize::MAX)
            .cmp(&right.ranking_rank.unwrap_or(usize::MAX))
            .then_with(|| right.legacy_weight.total_cmp(&left.legacy_weight))
            .then_with(|| left.fish.item_id.cmp(&right.fish.item_id))
    });
    let presence_fish = presence_fish
        .into_iter()
        .map(|entry| entry.fish)
        .collect::<Vec<_>>();

    let support_state = if presence_fish.is_empty() {
        ZonePresenceState::InsufficientEvidence
    } else {
        ZonePresenceState::Supported
    };

    let mut ranking_notes = zone_stats.confidence.notes.clone();
    ranking_notes.push("ranking evidence share is not a catch/drop rate".to_string());
    if zone_stats.distribution.is_empty() {
        ranking_notes.push("no ranking evidence in the selected window".to_string());
    }
    if has_legacy_support {
        ranking_notes.push(
            "legacy reference support is shown in presence_support, not in ranking_evidence"
                .to_string(),
        );
    }
    if has_community_support {
        ranking_notes.push(
            "community support is shown in presence_support, not in ranking_evidence".to_string(),
        );
    }

    let ranking_fish = zone_stats
        .distribution
        .iter()
        .map(|fish| ZoneRankingFishEvidence {
            fish_id: fish.fish_id,
            item_id: fish.item_id,
            encyclopedia_key: fish.encyclopedia_key,
            encyclopedia_id: fish.encyclopedia_id,
            fish_name: fish.fish_name.clone(),
            evidence_weight: fish.evidence_weight,
            evidence_share_mean: Some(fish.p_mean),
            ci_low: fish.ci_low,
            ci_high: fish.ci_high,
        })
        .collect();

    let public_state =
        public_state_from_zone_stats(&zone_stats, has_legacy_support || has_community_support);
    let insufficient_evidence = public_state == ZonePublicState::InsufficientEvidence;

    let mut evaluated_sources = vec![ZoneSourceFamily::Ranking];
    if legacy_evaluated {
        evaluated_sources.push(ZoneSourceFamily::Legacy);
    }
    if community_evaluated {
        evaluated_sources.push(ZoneSourceFamily::Community);
    }

    let mut diagnostics_notes = vec![
        "ranking evidence, border ambiguity, and catch-rate estimation are separate sections in zone_profile_v2".to_string(),
        "border ambiguity is intentionally unavailable in this slice rather than estimated from unsupported geometry".to_string(),
        "legacy and community support are populated in presence_support; player-log sources remain placeholders".to_string(),
    ];
    if insufficient_evidence {
        diagnostics_notes.push("missing ranking evidence is not evidence of absence".to_string());
    } else if zone_stats.distribution.is_empty() && (has_legacy_support || has_community_support) {
        diagnostics_notes.push(
            "ranking evidence is absent in the selected window, but non-ranking support exists for this zone"
                .to_string(),
        );
    }

    ZoneProfileV2Response {
        assignment: ZoneAssignment {
            zone_rgb_u32: zone_stats.zone_rgb_u32,
            zone_rgb: zone_stats.zone_rgb,
            zone_name: zone_stats.zone_name,
            point,
            border: ZoneBorderAssessment {
                class: ZoneBorderClass::Unavailable,
                nearest_border_distance_px: None,
                method: ZoneBorderMethod::Unavailable,
                warnings: border_warnings,
            },
            neighboring_zones: Vec::new(),
        },
        presence_support: ZonePresenceSupport {
            state: support_state,
            evaluated_sources,
            fish: presence_fish,
            notes: presence_notes,
        },
        ranking_evidence: ZoneRankingEvidence {
            availability: ZoneMetricAvailability::Available,
            source_family: ZoneSourceFamily::Ranking,
            share_kind: ZoneRankingShareKind::PosteriorMeanEvidenceShare,
            total_weight: zone_stats.confidence.total_weight,
            ess: zone_stats.confidence.ess,
            raw_event_count: None,
            last_seen_ts_utc: zone_stats.confidence.last_seen_ts_utc,
            age_days_last: zone_stats.confidence.age_days_last,
            status: ranking_status_from_zone_status(zone_stats.confidence.status),
            drift: zone_stats.confidence.drift.map(map_drift_info),
            notes: ranking_notes,
            fish: ranking_fish,
        },
        catch_rates: ZoneCatchRateSummary {
            source_family: ZoneSourceFamily::Logs,
            availability: ZoneMetricAvailability::PendingSource,
            fish: Vec::new(),
            notes: vec![
                "player-tracked catch-rate statistics are not yet available in zone_profile_v2"
                    .to_string(),
            ],
        },
        diagnostics: ZoneDiagnostics {
            public_state,
            insufficient_evidence,
            border_sensitive: None,
            border_stress: None,
            notes: diagnostics_notes,
        },
    }
}
