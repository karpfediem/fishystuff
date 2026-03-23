use std::collections::HashSet;

use fishystuff_core::field_metadata::{
    FieldHoverRow, FIELD_HOVER_ROW_KEY_ORIGIN, FIELD_HOVER_ROW_KEY_RESOURCES,
    FIELD_HOVER_ROW_KEY_ZONE,
};

use crate::map::layer_query::LayerQuerySample;
use crate::plugins::api::SelectedInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionHeading {
    pub row_key: Option<String>,
    pub value: String,
}

pub fn selection_heading(info: &SelectedInfo) -> Option<SelectionHeading> {
    nonempty(info.zone_name.as_deref())
        .map(|value| SelectionHeading {
            row_key: Some(FIELD_HOVER_ROW_KEY_ZONE.to_string()),
            value: value.to_string(),
        })
        .or_else(|| {
            preferred_title_row(&info.layer_samples).map(|row| SelectionHeading {
                row_key: Some(row.key.clone()),
                value: row.value.trim().to_string(),
            })
        })
}

pub fn selection_summary_text(info: &SelectedInfo) -> String {
    let heading = selection_heading(info);
    let lines = selection_overview_lines_with_heading(info, heading.as_ref());
    if !lines.is_empty() {
        return lines.into_iter().take(2).collect::<Vec<_>>().join(" · ");
    }
    format!(
        "Map {},{} · World {:.0},{:.0}",
        info.map_px, info.map_py, info.world_x, info.world_z
    )
}

pub fn selection_overview_lines(info: &SelectedInfo) -> Vec<String> {
    let heading = selection_heading(info);
    selection_overview_lines_with_heading(info, heading.as_ref())
}

fn selection_overview_lines_with_heading(
    info: &SelectedInfo,
    heading: Option<&SelectionHeading>,
) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut skipped_heading = false;
    let mut lines = Vec::new();
    for row in flattened_rows(&info.layer_samples) {
        if should_skip_heading_row(row, heading, &mut skipped_heading) {
            continue;
        }
        let Some(text) = overview_line(row) else {
            continue;
        };
        if seen.insert(text.clone()) {
            lines.push(text);
        }
    }
    lines
}

fn preferred_title_row(samples: &[LayerQuerySample]) -> Option<&FieldHoverRow> {
    for key in [
        FIELD_HOVER_ROW_KEY_ZONE,
        FIELD_HOVER_ROW_KEY_RESOURCES,
        FIELD_HOVER_ROW_KEY_ORIGIN,
    ] {
        if let Some(row) = flattened_rows(samples).find(|row| row.key == key && row_is_visible(row))
        {
            return Some(row);
        }
    }
    flattened_rows(samples).find(|row| row_is_visible(row))
}

fn flattened_rows<'a>(samples: &'a [LayerQuerySample]) -> impl Iterator<Item = &'a FieldHoverRow> {
    samples.iter().flat_map(|sample| sample.rows.iter())
}

fn should_skip_heading_row(
    row: &FieldHoverRow,
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
        .row_key
        .as_deref()
        .map(|key| key == row.key)
        .unwrap_or(false);
    let same_value = row.value.trim() == heading.value;
    if same_key && same_value {
        *skipped_heading = true;
        return true;
    }
    false
}

fn overview_line(row: &FieldHoverRow) -> Option<String> {
    let value = nonempty(Some(row.value.as_str()))?;
    if row.hide_label {
        return Some(value.to_string());
    }
    let label = nonempty(Some(row.label.as_str()))?;
    Some(format!("{label}: {value}"))
}

fn row_is_visible(row: &FieldHoverRow) -> bool {
    nonempty(Some(row.value.as_str())).is_some()
        && (row.hide_label || nonempty(Some(row.label.as_str())).is_some())
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{selection_heading, selection_overview_lines, selection_summary_text};
    use crate::map::layer_query::LayerQuerySample;
    use crate::plugins::api::SelectedInfo;
    use fishystuff_api::Rgb;
    use fishystuff_core::field_metadata::{
        FieldHoverRow, FIELD_HOVER_ROW_KEY_ORIGIN, FIELD_HOVER_ROW_KEY_RESOURCES,
        FIELD_HOVER_ROW_KEY_ZONE,
    };

    fn row(key: &str, label: &str, value: &str) -> FieldHoverRow {
        FieldHoverRow {
            key: key.to_string(),
            icon: "hover".to_string(),
            label: label.to_string(),
            value: value.to_string(),
            hide_label: false,
            status_icon: None,
            status_icon_tone: None,
        }
    }

    fn selection_info(
        zone_name: Option<&str>,
        layer_samples: Vec<LayerQuerySample>,
    ) -> SelectedInfo {
        SelectedInfo {
            map_px: 12,
            map_py: 34,
            rgb: Some(Rgb::from_u32(0x112233)),
            rgb_u32: Some(0x112233),
            zone_name: zone_name.map(ToOwned::to_owned),
            world_x: 100.0,
            world_z: 200.0,
            layer_samples,
        }
    }

    #[test]
    fn selection_heading_prefers_zone_name_when_available() {
        let info = selection_info(
            Some("Olvia Coast"),
            vec![LayerQuerySample {
                layer_id: "region_groups".to_string(),
                layer_name: "Region Groups".to_string(),
                kind: "field".to_string(),
                rgb: Rgb::from_u32(0x111111),
                rgb_u32: 0x111111,
                field_id: Some(295),
                rows: vec![row(FIELD_HOVER_ROW_KEY_RESOURCES, "Resources", "Olvia")],
                targets: Vec::new(),
            }],
        );
        assert_eq!(
            selection_heading(&info).map(|heading| heading.value),
            Some("Olvia Coast".to_string())
        );
    }

    #[test]
    fn selection_heading_falls_back_to_semantic_rows() {
        let info = selection_info(
            None,
            vec![
                LayerQuerySample {
                    layer_id: "regions".to_string(),
                    layer_name: "Regions".to_string(),
                    kind: "field".to_string(),
                    rgb: Rgb::from_u32(0x222222),
                    rgb_u32: 0x222222,
                    field_id: Some(76),
                    rows: vec![row(FIELD_HOVER_ROW_KEY_ORIGIN, "Origin", "Castle Ruins")],
                    targets: Vec::new(),
                },
                LayerQuerySample {
                    layer_id: "region_groups".to_string(),
                    layer_name: "Region Groups".to_string(),
                    kind: "field".to_string(),
                    rgb: Rgb::from_u32(0x333333),
                    rgb_u32: 0x333333,
                    field_id: Some(295),
                    rows: vec![row(FIELD_HOVER_ROW_KEY_RESOURCES, "Resources", "Olvia")],
                    targets: Vec::new(),
                },
            ],
        );
        assert_eq!(
            selection_heading(&info).map(|heading| heading.value),
            Some("Olvia".to_string())
        );
    }

    #[test]
    fn selection_summary_text_skips_the_heading_row_and_uses_remaining_rows() {
        let info = selection_info(
            Some("Demi River"),
            vec![
                LayerQuerySample {
                    layer_id: "zone_mask".to_string(),
                    layer_name: "Zone Mask".to_string(),
                    kind: "field".to_string(),
                    rgb: Rgb::from_u32(0x444444),
                    rgb_u32: 0x444444,
                    field_id: Some(0x444444),
                    rows: vec![row(FIELD_HOVER_ROW_KEY_ZONE, "Zone", "Demi River")],
                    targets: Vec::new(),
                },
                LayerQuerySample {
                    layer_id: "region_groups".to_string(),
                    layer_name: "Region Groups".to_string(),
                    kind: "field".to_string(),
                    rgb: Rgb::from_u32(0x555555),
                    rgb_u32: 0x555555,
                    field_id: Some(16),
                    rows: vec![row(FIELD_HOVER_ROW_KEY_RESOURCES, "Resources", "Tarif")],
                    targets: Vec::new(),
                },
                LayerQuerySample {
                    layer_id: "regions".to_string(),
                    layer_name: "Regions".to_string(),
                    kind: "field".to_string(),
                    rgb: Rgb::from_u32(0x666666),
                    rgb_u32: 0x666666,
                    field_id: Some(76),
                    rows: vec![row(FIELD_HOVER_ROW_KEY_ORIGIN, "Origin", "Tarif")],
                    targets: Vec::new(),
                },
            ],
        );
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
    fn selection_summary_text_falls_back_to_coordinates_without_rows() {
        let info = selection_info(None, Vec::new());
        assert_eq!(
            selection_summary_text(&info),
            "Map 12,34 · World 100,200".to_string()
        );
    }
}
