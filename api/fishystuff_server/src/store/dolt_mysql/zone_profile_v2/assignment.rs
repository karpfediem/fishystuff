use std::collections::{BTreeMap, HashMap};

use fishystuff_api::ids::RgbKey;
use fishystuff_api::models::zone_profile_v2::{
    ZoneAssignment, ZoneBorderAssessment, ZoneBorderClass, ZoneBorderMethod, ZoneNeighborCandidate,
    ZonePoint,
};
use fishystuff_api::models::zones::ZoneEntry;
use fishystuff_core::masks::ZoneMask;

const LOCAL_NEIGHBORHOOD_RADIUS_PX: i32 = 6;

pub(super) fn compute_zone_assignment(
    zone_rgb_u32: u32,
    zone_rgb: RgbKey,
    zone_name: Option<String>,
    map_px_x: Option<i32>,
    map_px_y: Option<i32>,
    zone_mask: Option<&ZoneMask>,
    zone_mask_warning: Option<&str>,
    zones: &HashMap<u32, ZoneEntry>,
) -> ZoneAssignment {
    let point = match (map_px_x, map_px_y) {
        (Some(map_px_x), Some(map_px_y)) => Some(ZonePoint { map_px_x, map_px_y }),
        _ => None,
    };
    let point_incomplete = map_px_x.is_some() ^ map_px_y.is_some();

    if point_incomplete {
        return ZoneAssignment {
            zone_rgb_u32,
            zone_rgb,
            zone_name,
            point: None,
            border: ZoneBorderAssessment {
                class: ZoneBorderClass::Unavailable,
                nearest_border_distance_px: None,
                method: ZoneBorderMethod::Unavailable,
                warnings: vec![
                    "point coordinates were incomplete; border classification was skipped"
                        .to_string(),
                ],
            },
            neighboring_zones: Vec::new(),
        };
    }

    let Some(point) = point else {
        return ZoneAssignment {
            zone_rgb_u32,
            zone_rgb,
            zone_name,
            point: None,
            border: ZoneBorderAssessment {
                class: ZoneBorderClass::Unavailable,
                nearest_border_distance_px: None,
                method: ZoneBorderMethod::Unavailable,
                warnings: vec!["point coordinates were not provided".to_string()],
            },
            neighboring_zones: Vec::new(),
        };
    };

    let Some(zone_mask) = zone_mask else {
        let mut warnings = vec![
            "zone mask was unavailable; border classification could not be computed".to_string(),
        ];
        if let Some(zone_mask_warning) = zone_mask_warning {
            warnings.push(zone_mask_warning.to_string());
        }
        return ZoneAssignment {
            zone_rgb_u32,
            zone_rgb,
            zone_name,
            point: Some(point),
            border: ZoneBorderAssessment {
                class: ZoneBorderClass::Unavailable,
                nearest_border_distance_px: None,
                method: ZoneBorderMethod::Unavailable,
                warnings,
            },
            neighboring_zones: Vec::new(),
        };
    };

    let Some(center_rgb) = zone_mask.rgb_u32(point.map_px_x, point.map_px_y) else {
        return ZoneAssignment {
            zone_rgb_u32,
            zone_rgb,
            zone_name,
            point: Some(point),
            border: ZoneBorderAssessment {
                class: ZoneBorderClass::Unavailable,
                nearest_border_distance_px: None,
                method: ZoneBorderMethod::Unavailable,
                warnings: vec!["point coordinates were outside the zone mask extent".to_string()],
            },
            neighboring_zones: Vec::new(),
        };
    };

    let mut neighbor_counts: BTreeMap<u32, u32> = BTreeMap::new();
    let mut target_seen_nearby = center_rgb == zone_rgb_u32;
    for dy in -LOCAL_NEIGHBORHOOD_RADIUS_PX..=LOCAL_NEIGHBORHOOD_RADIUS_PX {
        for dx in -LOCAL_NEIGHBORHOOD_RADIUS_PX..=LOCAL_NEIGHBORHOOD_RADIUS_PX {
            if dx == 0 && dy == 0 {
                continue;
            }
            let Some(sample_rgb) = zone_mask.rgb_u32(point.map_px_x + dx, point.map_px_y + dy)
            else {
                continue;
            };
            if sample_rgb == zone_rgb_u32 {
                target_seen_nearby = true;
                continue;
            }
            if !zones.contains_key(&sample_rgb) {
                continue;
            }
            *neighbor_counts.entry(sample_rgb).or_default() += 1;
        }
    }

    let mut warnings = vec![
        "border classification uses local zone-mask neighborhood sampling; exact border distance is not yet implemented"
            .to_string(),
    ];

    let center_zone_known = zones.contains_key(&center_rgb);
    let border_class = if center_rgb == zone_rgb_u32 {
        match neighbor_counts.len() {
            0 => ZoneBorderClass::Core,
            1 => ZoneBorderClass::NearBorder,
            _ => ZoneBorderClass::Ambiguous,
        }
    } else if center_zone_known {
        warnings.push(format!(
            "point samples zone RGB {} instead of requested zone RGB {}",
            format_rgb(center_rgb),
            format_rgb(zone_rgb_u32)
        ));
        if target_seen_nearby {
            ZoneBorderClass::Ambiguous
        } else {
            warnings.push(
                "requested zone RGB was not observed in the sampled local neighborhood".to_string(),
            );
            ZoneBorderClass::Ambiguous
        }
    } else {
        warnings.push(format!(
            "point samples unmapped zone RGB {} in the zone mask",
            format_rgb(center_rgb)
        ));
        return ZoneAssignment {
            zone_rgb_u32,
            zone_rgb,
            zone_name,
            point: Some(point),
            border: ZoneBorderAssessment {
                class: ZoneBorderClass::Unavailable,
                nearest_border_distance_px: None,
                method: ZoneBorderMethod::Unavailable,
                warnings,
            },
            neighboring_zones: Vec::new(),
        };
    };

    if center_rgb != zone_rgb_u32 && center_zone_known {
        *neighbor_counts.entry(center_rgb).or_default() += 1;
    }

    let mut neighboring_zones = neighbor_counts
        .into_iter()
        .filter(|(rgb, _)| *rgb != zone_rgb_u32)
        .filter_map(|(rgb, count)| {
            zones.get(&rgb).map(|zone| {
                (
                    count,
                    ZoneNeighborCandidate {
                        zone_rgb_u32: zone.rgb_u32,
                        zone_rgb: zone.rgb_key.clone(),
                        zone_name: zone.name.clone(),
                    },
                )
            })
        })
        .collect::<Vec<_>>();
    neighboring_zones.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.zone_rgb_u32.cmp(&right.1.zone_rgb_u32))
    });
    let neighboring_zones = neighboring_zones
        .into_iter()
        .map(|(_, zone)| zone)
        .collect::<Vec<_>>();

    ZoneAssignment {
        zone_rgb_u32,
        zone_rgb,
        zone_name,
        point: Some(point),
        border: ZoneBorderAssessment {
            class: border_class,
            nearest_border_distance_px: None,
            method: ZoneBorderMethod::LocalNeighborhood,
            warnings,
        },
        neighboring_zones,
    }
}

fn format_rgb(rgb_u32: u32) -> String {
    let r = (rgb_u32 >> 16) & 0xff;
    let g = (rgb_u32 >> 8) & 0xff;
    let b = rgb_u32 & 0xff;
    format!("{r},{g},{b}")
}
