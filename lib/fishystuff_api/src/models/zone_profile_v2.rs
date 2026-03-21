use serde::{Deserialize, Serialize};

use crate::ids::{MapVersionId, RgbKey, Timestamp};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneProfileV2Request {
    #[serde(default)]
    pub layer_revision_id: Option<String>,
    #[serde(default)]
    pub layer_id: Option<String>,
    #[serde(default)]
    pub patch_id: Option<String>,
    #[serde(default)]
    pub at_ts_utc: Option<Timestamp>,
    #[serde(default)]
    pub map_version_id: Option<MapVersionId>,
    pub rgb: RgbKey,
    #[serde(default)]
    pub map_px_x: Option<i32>,
    #[serde(default)]
    pub map_px_y: Option<i32>,
    pub from_ts_utc: Timestamp,
    pub to_ts_utc: Timestamp,
    pub tile_px: u32,
    pub sigma_tiles: f64,
    #[serde(default)]
    pub fish_norm: bool,
    pub alpha0: f64,
    pub top_k: usize,
    pub half_life_days: Option<f64>,
    #[serde(default)]
    pub drift_boundary_ts_utc: Option<Timestamp>,
    #[serde(default)]
    pub ref_id: Option<String>,
    #[serde(default)]
    pub lang: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZoneProfileV2Response {
    pub assignment: ZoneAssignment,
    pub presence_support: ZonePresenceSupport,
    pub ranking_evidence: ZoneRankingEvidence,
    pub catch_rates: ZoneCatchRateSummary,
    pub diagnostics: ZoneDiagnostics,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZoneAssignment {
    pub zone_rgb_u32: u32,
    pub zone_rgb: RgbKey,
    #[serde(default)]
    pub zone_name: Option<String>,
    #[serde(default)]
    pub point: Option<ZonePoint>,
    pub border: ZoneBorderAssessment,
    #[serde(default)]
    pub neighboring_zones: Vec<ZoneNeighborCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZonePoint {
    pub map_px_x: i32,
    pub map_px_y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZoneBorderAssessment {
    pub class: ZoneBorderClass,
    #[serde(default)]
    pub nearest_border_distance_px: Option<f64>,
    pub method: ZoneBorderMethod,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ZoneBorderClass {
    Core,
    NearBorder,
    Ambiguous,
    #[default]
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ZoneBorderMethod {
    MaskDistance,
    LocalNeighborhood,
    #[default]
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZoneNeighborCandidate {
    pub zone_rgb_u32: u32,
    pub zone_rgb: RgbKey,
    #[serde(default)]
    pub zone_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZonePresenceSupport {
    pub state: ZonePresenceState,
    #[serde(default)]
    pub evaluated_sources: Vec<ZoneSourceFamily>,
    #[serde(default)]
    pub fish: Vec<ZoneFishSupport>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ZonePresenceState {
    Supported,
    InsufficientEvidence,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZoneFishSupport {
    pub fish_id: i32,
    pub item_id: i32,
    #[serde(default)]
    pub encyclopedia_key: Option<i32>,
    #[serde(default)]
    pub encyclopedia_id: Option<i32>,
    #[serde(default)]
    pub fish_name: Option<String>,
    pub support_grade: ZoneSupportGrade,
    #[serde(default)]
    pub source_badges: Vec<ZoneSourceFamily>,
    #[serde(default)]
    pub claims: Vec<ZoneSupportClaim>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZoneSupportClaim {
    pub source_family: ZoneSourceFamily,
    pub claim_type: ZoneClaimType,
    #[serde(default)]
    pub confidence_note: Option<String>,
    #[serde(default)]
    pub observed_at_ts_utc: Option<Timestamp>,
    #[serde(default)]
    pub source_revision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ZoneSupportGrade {
    ObservedRecent,
    ObservedHistorical,
    ReferenceSupported,
    WeakHint,
    InsufficientEvidence,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ZoneSourceFamily {
    Legacy,
    Community,
    #[default]
    Ranking,
    Logs,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ZoneClaimType {
    PresenceObserved,
    PresenceReferenced,
    GroupHint,
    CatchRateSummary,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZoneRankingEvidence {
    pub availability: ZoneMetricAvailability,
    pub source_family: ZoneSourceFamily,
    pub share_kind: ZoneRankingShareKind,
    pub total_weight: f64,
    pub ess: f64,
    #[serde(default)]
    pub raw_event_count: Option<u64>,
    #[serde(default)]
    pub last_seen_ts_utc: Option<Timestamp>,
    #[serde(default)]
    pub age_days_last: Option<f64>,
    pub status: ZoneRankingStatus,
    #[serde(default)]
    pub drift: Option<ZoneRankingDrift>,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub fish: Vec<ZoneRankingFishEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ZoneRankingShareKind {
    #[default]
    PosteriorMeanEvidenceShare,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ZoneRankingStatus {
    Fresh,
    Stale,
    Drifting,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZoneRankingDrift {
    pub boundary_ts_utc: Timestamp,
    pub jsd_mean: f64,
    pub p_drift: f64,
    pub ess_old: f64,
    pub ess_new: f64,
    pub samples: usize,
    pub jsd_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZoneRankingFishEvidence {
    pub fish_id: i32,
    pub item_id: i32,
    #[serde(default)]
    pub encyclopedia_key: Option<i32>,
    #[serde(default)]
    pub encyclopedia_id: Option<i32>,
    #[serde(default)]
    pub fish_name: Option<String>,
    pub evidence_weight: f64,
    #[serde(default)]
    pub evidence_share_mean: Option<f64>,
    #[serde(default)]
    pub ci_low: Option<f64>,
    #[serde(default)]
    pub ci_high: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZoneCatchRateSummary {
    pub source_family: ZoneSourceFamily,
    pub availability: ZoneMetricAvailability,
    #[serde(default)]
    pub fish: Vec<ZoneFishCatchRate>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZoneFishCatchRate {
    pub fish_id: i32,
    pub item_id: i32,
    #[serde(default)]
    pub encyclopedia_key: Option<i32>,
    #[serde(default)]
    pub encyclopedia_id: Option<i32>,
    #[serde(default)]
    pub fish_name: Option<String>,
    #[serde(default)]
    pub catch_rate_mean: Option<f64>,
    #[serde(default)]
    pub ci_low: Option<f64>,
    #[serde(default)]
    pub ci_high: Option<f64>,
    #[serde(default)]
    pub catches: Option<u64>,
    #[serde(default)]
    pub opportunities: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ZoneMetricAvailability {
    Available,
    PendingSource,
    #[default]
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZoneDiagnostics {
    pub public_state: ZonePublicState,
    #[serde(default)]
    pub insufficient_evidence: bool,
    #[serde(default)]
    pub border_sensitive: Option<bool>,
    #[serde(default)]
    pub border_stress: Option<ZoneBorderStressSummary>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ZonePublicState {
    Supported,
    Stale,
    Drifting,
    InsufficientEvidence,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZoneBorderStressSummary {
    #[serde(default)]
    pub near_border_weight_fraction: Option<f64>,
    #[serde(default)]
    pub core_vs_border_jsd: Option<f64>,
    #[serde(default)]
    pub per_neighbor: Vec<ZoneNeighborStress>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZoneNeighborStress {
    pub zone_rgb_u32: u32,
    pub zone_rgb: RgbKey,
    #[serde(default)]
    pub zone_name: Option<String>,
    #[serde(default)]
    pub shared_border_weight: Option<f64>,
    #[serde(default)]
    pub cross_border_jsd: Option<f64>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        ZoneAssignment, ZoneBorderAssessment, ZoneBorderClass, ZoneBorderMethod, ZoneDiagnostics,
        ZoneFishSupport, ZoneMetricAvailability, ZonePresenceState, ZonePresenceSupport,
        ZoneProfileV2Response, ZonePublicState, ZoneRankingEvidence, ZoneRankingFishEvidence,
        ZoneRankingShareKind, ZoneRankingStatus, ZoneSourceFamily, ZoneSupportClaim,
        ZoneSupportGrade,
    };
    use crate::ids::RgbKey;

    #[test]
    fn ranking_evidence_share_is_nested_and_explicitly_named() {
        let response = ZoneProfileV2Response {
            assignment: ZoneAssignment {
                zone_rgb_u32: 0x112233,
                zone_rgb: RgbKey("17,34,51".to_string()),
                zone_name: Some("Example".to_string()),
                point: None,
                border: ZoneBorderAssessment {
                    class: ZoneBorderClass::Unavailable,
                    nearest_border_distance_px: None,
                    method: ZoneBorderMethod::Unavailable,
                    warnings: vec!["point not provided".to_string()],
                },
                neighboring_zones: Vec::new(),
            },
            presence_support: ZonePresenceSupport {
                state: ZonePresenceState::Supported,
                evaluated_sources: vec![ZoneSourceFamily::Ranking],
                fish: Vec::new(),
                notes: vec!["ranking-backed support only".to_string()],
            },
            ranking_evidence: ZoneRankingEvidence {
                availability: ZoneMetricAvailability::Available,
                source_family: ZoneSourceFamily::Ranking,
                share_kind: ZoneRankingShareKind::PosteriorMeanEvidenceShare,
                total_weight: 12.5,
                ess: 9.0,
                raw_event_count: Some(12),
                last_seen_ts_utc: Some(123),
                age_days_last: Some(2.0),
                status: ZoneRankingStatus::Stale,
                drift: None,
                notes: vec!["ranking evidence share, not catch/drop rate".to_string()],
                fish: vec![ZoneRankingFishEvidence {
                    fish_id: 820115,
                    item_id: 820115,
                    encyclopedia_key: Some(821015),
                    encyclopedia_id: Some(9015),
                    fish_name: Some("Ancient Fish".to_string()),
                    evidence_weight: 4.0,
                    evidence_share_mean: Some(0.42),
                    ci_low: Some(0.20),
                    ci_high: Some(0.61),
                }],
            },
            catch_rates: super::ZoneCatchRateSummary {
                source_family: ZoneSourceFamily::Logs,
                availability: ZoneMetricAvailability::PendingSource,
                fish: Vec::new(),
                notes: vec!["player-log catch rates not yet available".to_string()],
            },
            diagnostics: ZoneDiagnostics {
                public_state: ZonePublicState::InsufficientEvidence,
                insufficient_evidence: true,
                border_sensitive: None,
                border_stress: None,
                notes: vec!["ranking evidence only".to_string()],
            },
        };

        let value = serde_json::to_value(response).expect("serialize zone profile");
        assert_eq!(
            value["ranking_evidence"]["share_kind"],
            json!("posterior_mean_evidence_share")
        );
        assert_eq!(
            value["ranking_evidence"]["fish"][0]["evidence_share_mean"],
            json!(0.42)
        );
        assert_eq!(
            value["ranking_evidence"]["availability"],
            json!("available")
        );
        assert!(value.get("p_mean").is_none());
        assert!(value.get("pMean").is_none());
        assert_eq!(
            value["catch_rates"]["availability"],
            json!("pending_source")
        );
    }

    #[test]
    fn support_and_border_unknown_states_are_explicit() {
        let support = ZoneFishSupport {
            fish_id: 1,
            item_id: 1,
            encyclopedia_key: None,
            encyclopedia_id: None,
            fish_name: Some("Test Fish".to_string()),
            support_grade: ZoneSupportGrade::Unknown,
            source_badges: vec![ZoneSourceFamily::Community],
            claims: vec![ZoneSupportClaim {
                source_family: ZoneSourceFamily::Community,
                claim_type: super::ZoneClaimType::GroupHint,
                confidence_note: Some("curated subgroup hint".to_string()),
                observed_at_ts_utc: None,
                source_revision: Some("sheet:v1".to_string()),
            }],
        };

        let value = serde_json::to_value(support).expect("serialize support");
        assert_eq!(value["support_grade"], json!("unknown"));
        assert_eq!(value["source_badges"][0], json!("community"));

        let border = ZoneBorderAssessment {
            class: ZoneBorderClass::Unavailable,
            nearest_border_distance_px: None,
            method: ZoneBorderMethod::Unavailable,
            warnings: vec!["distance transform not built".to_string()],
        };
        let border_value = serde_json::to_value(border).expect("serialize border");
        assert_eq!(border_value["class"], json!("unavailable"));
        assert_eq!(border_value["method"], json!("unavailable"));
    }

    #[test]
    fn presence_support_can_explicitly_encode_insufficient_evidence() {
        let presence = ZonePresenceSupport {
            state: ZonePresenceState::InsufficientEvidence,
            evaluated_sources: vec![ZoneSourceFamily::Ranking],
            fish: Vec::new(),
            notes: vec!["missing ranking evidence is not evidence of absence".to_string()],
        };

        let value = serde_json::to_value(presence).expect("serialize presence support");
        assert_eq!(value["state"], json!("insufficient_evidence"));
        assert_eq!(value["evaluated_sources"][0], json!("ranking"));
        assert_eq!(
            value["notes"][0],
            json!("missing ranking evidence is not evidence of absence")
        );
    }
}
