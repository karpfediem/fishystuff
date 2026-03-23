use super::super::super::*;

pub(in crate::bridge::host::snapshot) fn effective_selection_snapshot(
    info: Option<&crate::plugins::api::SelectedInfo>,
    zone_stats: Option<&fishystuff_api::models::zone_stats::ZoneStatsResponse>,
) -> FishyMapSelectionSnapshot {
    let selected_world_point =
        info.and_then(crate::plugins::api::SelectedInfo::effective_world_point);
    FishyMapSelectionSnapshot {
        world_x: selected_world_point.map(|value| value.0),
        world_z: selected_world_point.map(|value| value.1),
        layer_samples: info
            .map(|value| hover_layer_samples_snapshot(&value.layer_samples))
            .unwrap_or_default(),
        zone_stats: zone_stats.map(zone_stats_snapshot),
    }
}

pub(in crate::bridge::host::snapshot) fn effective_hover_snapshot(
    info: Option<&HoverInfo>,
) -> FishyMapHoverSnapshot {
    FishyMapHoverSnapshot {
        world_x: info.map(|value| value.world_x),
        world_z: info.map(|value| value.world_z),
        layer_samples: info
            .map(|value| hover_layer_samples_snapshot(&value.layer_samples))
            .unwrap_or_default(),
    }
}

pub(in crate::bridge::host) fn hover_layer_samples_snapshot(
    samples: &[LayerQuerySample],
) -> Vec<crate::bridge::contract::FishyMapHoverLayerSampleSnapshot> {
    samples
        .iter()
        .map(
            |sample| crate::bridge::contract::FishyMapHoverLayerSampleSnapshot {
                layer_id: sample.layer_id.clone(),
                layer_name: sample.layer_name.clone(),
                kind: sample.kind.clone(),
                rgb: sample.rgb.as_array(),
                rgb_u32: sample.rgb_u32,
                field_id: sample.field_id,
                rows: sample.rows.clone(),
                targets: sample.targets.clone(),
            },
        )
        .collect()
}

fn zone_stats_snapshot(
    stats: &fishystuff_api::models::zone_stats::ZoneStatsResponse,
) -> FishyMapZoneStatsSnapshot {
    FishyMapZoneStatsSnapshot {
        zone_rgb: stats.zone_rgb_u32,
        zone_name: stats.zone_name.clone(),
        window: FishyMapZoneWindowSnapshot {
            from_ts_utc: stats.window.from_ts_utc,
            to_ts_utc: stats.window.to_ts_utc,
            half_life_days: stats.window.half_life_days,
            fish_norm: stats.window.fish_norm,
            tile_px: stats.window.tile_px,
            sigma_tiles: stats.window.sigma_tiles,
            alpha0: stats.window.alpha0,
        },
        confidence: FishyMapZoneConfidenceSnapshot {
            ess: stats.confidence.ess,
            total_weight: stats.confidence.total_weight,
            last_seen_ts_utc: stats.confidence.last_seen_ts_utc,
            age_days_last: stats.confidence.age_days_last,
            status: format!("{:?}", stats.confidence.status).to_uppercase(),
            notes: stats.confidence.notes.clone(),
            drift: stats
                .confidence
                .drift
                .as_ref()
                .map(|drift| FishyMapZoneDriftSnapshot {
                    boundary_ts_utc: drift.boundary_ts_utc,
                    jsd_mean: drift.jsd_mean,
                    p_drift: drift.p_drift,
                    ess_old: drift.ess_old,
                    ess_new: drift.ess_new,
                    samples: drift.samples,
                    jsd_threshold: drift.jsd_threshold,
                }),
        },
        distribution: stats
            .distribution
            .iter()
            .map(|entry| FishyMapZoneEvidenceEntrySnapshot {
                fish_id: entry.fish_id,
                item_id: entry.item_id,
                encyclopedia_key: entry.encyclopedia_key,
                encyclopedia_id: entry.encyclopedia_id,
                fish_name: entry.fish_name.clone(),
                evidence_weight: entry.evidence_weight,
                p_mean: entry.p_mean,
                ci_low: entry.ci_low,
                ci_high: entry.ci_high,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use crate::plugins::api::SelectedInfo;
    use fishystuff_api::Rgb;

    #[test]
    fn synthetic_zone_selection_does_not_emit_fake_world_point() {
        let info = SelectedInfo {
            map_px: 0,
            map_py: 0,
            world_x: f64::NAN,
            world_z: f64::NAN,
            sampled_world_point: false,
            layer_samples: vec![crate::map::layer_query::LayerQuerySample {
                layer_id: "zone_mask".to_string(),
                layer_name: "Zone Mask".to_string(),
                kind: "field".to_string(),
                rgb: Rgb::from_u32(0x123456),
                rgb_u32: 0x123456,
                field_id: Some(0x123456),
                rows: Vec::new(),
                targets: Vec::new(),
            }],
        };

        assert_eq!(info.effective_world_point(), None);
    }

    #[test]
    fn sampled_selection_keeps_world_point() {
        let info = SelectedInfo {
            map_px: 10,
            map_py: 20,
            world_x: 123.0,
            world_z: 456.0,
            sampled_world_point: true,
            layer_samples: vec![crate::map::layer_query::LayerQuerySample {
                layer_id: "zone_mask".to_string(),
                layer_name: "Zone Mask".to_string(),
                kind: "field".to_string(),
                rgb: Rgb::from_u32(0x654321),
                rgb_u32: 0x654321,
                field_id: Some(0x654321),
                rows: Vec::new(),
                targets: Vec::new(),
            }],
        };

        assert_eq!(info.effective_world_point(), Some((123.0, 456.0)));
    }
}
