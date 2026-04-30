use std::collections::HashSet;

use fishystuff_core::field_metadata::{
    detail_fact_is_visible, preferred_detail_fact, FieldDetailFact,
    FIELD_DETAIL_FACT_KEY_ORIGIN_REGION, FIELD_DETAIL_FACT_KEY_RESOURCE_GROUP,
    FIELD_DETAIL_FACT_KEY_RESOURCE_REGION, FIELD_DETAIL_FACT_KEY_ZONE,
};

use crate::map::layer_query::LayerQuerySample;
use crate::plugins::api::SelectedInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionHeading {
    pub fact_key: Option<String>,
    pub value: String,
}

pub fn selection_heading(info: &SelectedInfo) -> Option<SelectionHeading> {
    preferred_title_fact(&info.layer_samples).map(|fact| SelectionHeading {
        fact_key: Some(fact.key.clone()),
        value: fact.value.trim().to_string(),
    })
}

pub fn selection_summary_text(info: &SelectedInfo) -> String {
    let heading = selection_heading(info);
    let lines = selection_overview_lines_with_heading(info, heading.as_ref());
    if !lines.is_empty() {
        return lines.into_iter().take(2).collect::<Vec<_>>().join(" · ");
    }
    heading
        .map(|heading| heading.value)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            format!(
                "Map {},{} · World {:.0},{:.0}",
                info.map_px, info.map_py, info.world_x, info.world_z
            )
        })
}

pub fn selection_overview_lines(info: &SelectedInfo) -> Vec<String> {
    let heading = selection_heading(info);
    selection_overview_lines_with_heading(info, heading.as_ref())
}

fn selection_overview_lines_with_heading(
    info: &SelectedInfo,
    heading: Option<&SelectionHeading>,
) -> Vec<String> {
    let lines = collect_overview_lines(flattened_facts(&info.layer_samples), heading);
    if !lines.is_empty() || heading.is_none() {
        return lines;
    }
    collect_overview_lines(flattened_facts(&info.layer_samples), None)
}

fn preferred_title_fact(samples: &[LayerQuerySample]) -> Option<&FieldDetailFact> {
    preferred_detail_fact(flattened_facts(samples).filter(|fact| summary_fact_is_visible(fact)))
}

fn flattened_facts<'a>(
    samples: &'a [LayerQuerySample],
) -> impl Iterator<Item = &'a FieldDetailFact> {
    samples
        .iter()
        .flat_map(|sample| sample.detail_sections.iter())
        .flat_map(|section| section.facts.iter())
}

fn should_skip_heading_row(
    fact: &FieldDetailFact,
    heading: Option<&SelectionHeading>,
    skipped_heading: &mut bool,
) -> bool {
    let Some(heading) = heading else {
        return false;
    };
    if *skipped_heading {
        return false;
    }
    let same_key = heading
        .fact_key
        .as_deref()
        .map(|key| key == fact.key)
        .unwrap_or(false);
    let same_value = fact.value.trim() == heading.value;
    if same_key && same_value {
        *skipped_heading = true;
        return true;
    }
    false
}

fn overview_line(fact: &FieldDetailFact) -> Option<String> {
    let value = nonempty(Some(fact.value.as_str()))?;
    let label = nonempty(summary_label_for_fact(fact))?;
    Some(format!("{label}: {value}"))
}

fn summary_fact_is_visible(fact: &FieldDetailFact) -> bool {
    if !detail_fact_is_visible(fact) {
        return false;
    }
    matches!(
        fact.key.as_str(),
        FIELD_DETAIL_FACT_KEY_ZONE
            | FIELD_DETAIL_FACT_KEY_RESOURCE_GROUP
            | FIELD_DETAIL_FACT_KEY_RESOURCE_REGION
            | FIELD_DETAIL_FACT_KEY_ORIGIN_REGION
    )
}

fn summary_label_for_fact(fact: &FieldDetailFact) -> Option<&str> {
    match fact.key.as_str() {
        FIELD_DETAIL_FACT_KEY_ZONE => Some("Zone"),
        FIELD_DETAIL_FACT_KEY_RESOURCE_GROUP => Some("Resources"),
        FIELD_DETAIL_FACT_KEY_RESOURCE_REGION => Some("Resources"),
        FIELD_DETAIL_FACT_KEY_ORIGIN_REGION => Some("Origin"),
        _ => None,
    }
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn collect_overview_lines<'a>(
    facts: impl IntoIterator<Item = &'a FieldDetailFact>,
    heading: Option<&SelectionHeading>,
) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut skipped_heading = false;
    let mut lines = Vec::new();
    for fact in facts {
        if !summary_fact_is_visible(fact) {
            continue;
        }
        if should_skip_heading_row(fact, heading, &mut skipped_heading) {
            continue;
        }
        let Some(text) = overview_line(fact) else {
            continue;
        };
        if seen.insert(text.clone()) {
            lines.push(text);
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::{selection_heading, selection_overview_lines, selection_summary_text};
    use crate::map::layer_query::LayerQuerySample;
    use crate::plugins::api::SelectedInfo;
    use fishystuff_api::Rgb;
    use fishystuff_core::field_metadata::{
        FieldDetailFact, FieldDetailSection, FIELD_DETAIL_FACT_KEY_ORIGIN_REGION,
        FIELD_DETAIL_FACT_KEY_RESOURCE_REGION, FIELD_DETAIL_FACT_KEY_ZONE,
    };

    fn fact(key: &str, label: &str, value: &str) -> FieldDetailFact {
        FieldDetailFact {
            key: key.to_string(),
            label: label.to_string(),
            value: value.to_string(),
            icon: Some("hover".to_string()),
            status_icon: None,
            status_icon_tone: None,
        }
    }

    fn selection_info(layer_samples: Vec<LayerQuerySample>) -> SelectedInfo {
        SelectedInfo {
            map_px: 12,
            map_py: 34,
            world_x: 100.0,
            world_z: 200.0,
            sampled_world_point: true,
            point_kind: Some(crate::bridge::contract::FishyMapSelectionPointKind::Clicked),
            point_label: None,
            layer_samples,
            point_samples: Vec::new(),
        }
    }

    #[test]
    fn selection_heading_prefers_primary_zone_row_when_available() {
        let info = selection_info(vec![
            LayerQuerySample {
                layer_id: "zone_mask".to_string(),
                layer_name: "Zone Mask".to_string(),
                kind: "field".to_string(),
                rgb: Rgb::from_u32(0x010101),
                rgb_u32: 0x010101,
                field_id: Some(0x010101),
                targets: Vec::new(),
                detail_pane: None,
                detail_sections: vec![FieldDetailSection {
                    id: "zone".to_string(),
                    kind: "facts".to_string(),
                    title: Some("Zone".to_string()),
                    facts: vec![fact(FIELD_DETAIL_FACT_KEY_ZONE, "Zone", "Olvia Coast")],
                    targets: Vec::new(),
                }],
            },
            LayerQuerySample {
                layer_id: "region_groups".to_string(),
                layer_name: "Region Groups".to_string(),
                kind: "field".to_string(),
                rgb: Rgb::from_u32(0x111111),
                rgb_u32: 0x111111,
                field_id: Some(295),
                targets: Vec::new(),
                detail_pane: None,
                detail_sections: vec![FieldDetailSection {
                    id: "resource".to_string(),
                    kind: "facts".to_string(),
                    title: Some("Resource".to_string()),
                    facts: vec![fact(
                        FIELD_DETAIL_FACT_KEY_RESOURCE_REGION,
                        "Containing region",
                        "Olvia",
                    )],
                    targets: Vec::new(),
                }],
            },
        ]);
        assert_eq!(
            selection_heading(&info).map(|heading| heading.value),
            Some("Olvia Coast".to_string())
        );
    }

    #[test]
    fn selection_heading_falls_back_to_semantic_rows() {
        let info = selection_info(vec![
            LayerQuerySample {
                layer_id: "regions".to_string(),
                layer_name: "Regions".to_string(),
                kind: "field".to_string(),
                rgb: Rgb::from_u32(0x222222),
                rgb_u32: 0x222222,
                field_id: Some(76),
                targets: Vec::new(),
                detail_pane: None,
                detail_sections: vec![FieldDetailSection {
                    id: "origin".to_string(),
                    kind: "facts".to_string(),
                    title: Some("Origin".to_string()),
                    facts: vec![fact(
                        FIELD_DETAIL_FACT_KEY_ORIGIN_REGION,
                        "Region",
                        "Castle Ruins",
                    )],
                    targets: Vec::new(),
                }],
            },
            LayerQuerySample {
                layer_id: "region_groups".to_string(),
                layer_name: "Region Groups".to_string(),
                kind: "field".to_string(),
                rgb: Rgb::from_u32(0x333333),
                rgb_u32: 0x333333,
                field_id: Some(295),
                targets: Vec::new(),
                detail_pane: None,
                detail_sections: vec![FieldDetailSection {
                    id: "resource".to_string(),
                    kind: "facts".to_string(),
                    title: Some("Resource".to_string()),
                    facts: vec![fact(
                        FIELD_DETAIL_FACT_KEY_RESOURCE_REGION,
                        "Containing region",
                        "Olvia",
                    )],
                    targets: Vec::new(),
                }],
            },
        ]);
        assert_eq!(
            selection_heading(&info).map(|heading| heading.value),
            Some("Olvia".to_string())
        );
    }

    #[test]
    fn selection_summary_text_skips_the_heading_fact_and_uses_remaining_facts() {
        let info = selection_info(vec![
            LayerQuerySample {
                layer_id: "zone_mask".to_string(),
                layer_name: "Zone Mask".to_string(),
                kind: "field".to_string(),
                rgb: Rgb::from_u32(0x444444),
                rgb_u32: 0x444444,
                field_id: Some(0x444444),
                targets: Vec::new(),
                detail_pane: None,
                detail_sections: vec![FieldDetailSection {
                    id: "zone".to_string(),
                    kind: "facts".to_string(),
                    title: Some("Zone".to_string()),
                    facts: vec![fact(FIELD_DETAIL_FACT_KEY_ZONE, "Zone", "Demi River")],
                    targets: Vec::new(),
                }],
            },
            LayerQuerySample {
                layer_id: "region_groups".to_string(),
                layer_name: "Region Groups".to_string(),
                kind: "field".to_string(),
                rgb: Rgb::from_u32(0x555555),
                rgb_u32: 0x555555,
                field_id: Some(16),
                targets: Vec::new(),
                detail_pane: None,
                detail_sections: vec![FieldDetailSection {
                    id: "resource".to_string(),
                    kind: "facts".to_string(),
                    title: Some("Resource".to_string()),
                    facts: vec![fact(
                        FIELD_DETAIL_FACT_KEY_RESOURCE_REGION,
                        "Containing region",
                        "Tarif",
                    )],
                    targets: Vec::new(),
                }],
            },
            LayerQuerySample {
                layer_id: "regions".to_string(),
                layer_name: "Regions".to_string(),
                kind: "field".to_string(),
                rgb: Rgb::from_u32(0x666666),
                rgb_u32: 0x666666,
                field_id: Some(76),
                targets: Vec::new(),
                detail_pane: None,
                detail_sections: vec![FieldDetailSection {
                    id: "origin".to_string(),
                    kind: "facts".to_string(),
                    title: Some("Origin".to_string()),
                    facts: vec![fact(FIELD_DETAIL_FACT_KEY_ORIGIN_REGION, "Region", "Tarif")],
                    targets: Vec::new(),
                }],
            },
        ]);
        assert_eq!(
            selection_summary_text(&info),
            "Resources: Tarif · Origin: Tarif".to_string()
        );
        assert_eq!(
            selection_overview_lines(&info),
            vec!["Resources: Tarif".to_string(), "Origin: Tarif".to_string()]
        );
    }

    #[test]
    fn selection_summary_text_falls_back_to_coordinates_without_facts() {
        let info = selection_info(Vec::new());
        assert_eq!(
            selection_summary_text(&info),
            "Map 12,34 · World 100,200".to_string()
        );
    }
}
