use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct WaypointXmlInspectSummary {
    pub output_path: Option<PathBuf>,
    pub waypoint_count: usize,
    pub link_count: usize,
    pub focus_waypoint_count: usize,
    pub missing_focus_waypoint_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct WaypointXmlWaypointSummary {
    pub key: u32,
    pub line_number: usize,
    pub raw_name: String,
    pub name_scope: Option<String>,
    pub name_token: Option<String>,
    pub pos_x: f64,
    pub pos_y: f64,
    pub pos_z: f64,
    pub property: String,
    pub is_sub_waypoint: bool,
    pub is_escape: bool,
}

#[derive(Debug, Serialize)]
struct WaypointXmlInspectReport {
    path: String,
    file_size: u64,
    waypoint_count: usize,
    link_count: usize,
    focus_waypoints: Vec<WaypointXmlFocusWaypointReport>,
    missing_focus_waypoint_ids: Vec<u32>,
}

#[derive(Debug, Serialize)]
struct WaypointXmlFocusWaypointReport {
    #[serde(flatten)]
    waypoint: WaypointXmlWaypointSummary,
    outgoing_links: Vec<u32>,
    incoming_links: Vec<u32>,
}

pub fn inspect_waypoint_xml(
    path: &Path,
    focus_waypoint_ids: &[u32],
    output_path: Option<&Path>,
) -> Result<WaypointXmlInspectSummary> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read waypoint XML {}", path.display()))?;
    let contents = String::from_utf8_lossy(&bytes);
    let focus_id_set = focus_waypoint_ids.iter().copied().collect::<BTreeSet<_>>();
    let mut focus_waypoints = BTreeMap::<u32, WaypointXmlWaypointSummary>::new();
    let mut outgoing_links = BTreeMap::<u32, BTreeSet<u32>>::new();
    let mut incoming_links = BTreeMap::<u32, BTreeSet<u32>>::new();
    let mut waypoint_count = 0usize;
    let mut link_count = 0usize;

    for (line_index, raw_line) in contents.lines().enumerate() {
        let line = raw_line.trim();
        if line.starts_with("<Waypoint ") {
            waypoint_count += 1;
            let key = parse_attr_u32(line, "Key").with_context(|| {
                format!("failed to parse Waypoint Key at line {}", line_index + 1)
            })?;
            if focus_id_set.is_empty() || focus_id_set.contains(&key) {
                let raw_name = parse_attr_string(line, "Name").with_context(|| {
                    format!("failed to parse Waypoint Name at line {}", line_index + 1)
                })?;
                let (name_scope, name_token) = split_waypoint_name(&raw_name);
                let waypoint = WaypointXmlWaypointSummary {
                    key,
                    line_number: line_index + 1,
                    raw_name,
                    name_scope,
                    name_token,
                    pos_x: parse_attr_f64(line, "PosX").with_context(|| {
                        format!("failed to parse Waypoint PosX at line {}", line_index + 1)
                    })?,
                    pos_y: parse_attr_f64(line, "PosY").with_context(|| {
                        format!("failed to parse Waypoint PosY at line {}", line_index + 1)
                    })?,
                    pos_z: parse_attr_f64(line, "PosZ").with_context(|| {
                        format!("failed to parse Waypoint PosZ at line {}", line_index + 1)
                    })?,
                    property: parse_attr_string(line, "Property").with_context(|| {
                        format!(
                            "failed to parse Waypoint Property at line {}",
                            line_index + 1
                        )
                    })?,
                    is_sub_waypoint: parse_attr_bool(line, "IsSubWaypoint").with_context(|| {
                        format!(
                            "failed to parse Waypoint IsSubWaypoint at line {}",
                            line_index + 1
                        )
                    })?,
                    is_escape: parse_attr_bool(line, "IsEscape").with_context(|| {
                        format!(
                            "failed to parse Waypoint IsEscape at line {}",
                            line_index + 1
                        )
                    })?,
                };
                focus_waypoints.insert(key, waypoint);
            }
        } else if line.starts_with("<Link ") {
            link_count += 1;
            if !focus_id_set.is_empty() {
                let source = parse_attr_u32(line, "SourceWaypoint").with_context(|| {
                    format!(
                        "failed to parse Link SourceWaypoint at line {}",
                        line_index + 1
                    )
                })?;
                let target = parse_attr_u32(line, "TargetWaypoint").with_context(|| {
                    format!(
                        "failed to parse Link TargetWaypoint at line {}",
                        line_index + 1
                    )
                })?;
                if focus_id_set.contains(&source) {
                    outgoing_links.entry(source).or_default().insert(target);
                }
                if focus_id_set.contains(&target) {
                    incoming_links.entry(target).or_default().insert(source);
                }
            }
        }
    }

    let focus_reports = focus_waypoint_ids
        .iter()
        .copied()
        .filter_map(|waypoint_id| {
            focus_waypoints.get(&waypoint_id).cloned().map(|waypoint| {
                WaypointXmlFocusWaypointReport {
                    waypoint,
                    outgoing_links: outgoing_links
                        .get(&waypoint_id)
                        .map(|values| values.iter().copied().collect())
                        .unwrap_or_default(),
                    incoming_links: incoming_links
                        .get(&waypoint_id)
                        .map(|values| values.iter().copied().collect())
                        .unwrap_or_default(),
                }
            })
        })
        .collect::<Vec<_>>();

    let missing_focus_waypoint_ids = focus_waypoint_ids
        .iter()
        .copied()
        .filter(|waypoint_id| !focus_waypoints.contains_key(waypoint_id))
        .collect::<Vec<_>>();

    if let Some(output_path) = output_path {
        let report = WaypointXmlInspectReport {
            path: path.display().to_string(),
            file_size: bytes.len() as u64,
            waypoint_count,
            link_count,
            focus_waypoints: focus_reports,
            missing_focus_waypoint_ids: missing_focus_waypoint_ids.clone(),
        };
        super::write_json_report(output_path, &report)?;
    }

    Ok(WaypointXmlInspectSummary {
        output_path: output_path.map(Path::to_path_buf),
        waypoint_count,
        link_count,
        focus_waypoint_count: focus_waypoint_ids
            .len()
            .saturating_sub(missing_focus_waypoint_ids.len()),
        missing_focus_waypoint_count: missing_focus_waypoint_ids.len(),
    })
}

fn parse_attr_u32(line: &str, attr: &str) -> Result<u32> {
    let raw = parse_attr(line, attr)?;
    raw.parse::<u32>()
        .with_context(|| format!("failed to parse `{raw}` as u32 for attribute {attr}"))
}

fn parse_attr_f64(line: &str, attr: &str) -> Result<f64> {
    let raw = parse_attr(line, attr)?;
    raw.parse::<f64>()
        .with_context(|| format!("failed to parse `{raw}` as f64 for attribute {attr}"))
}

fn parse_attr_bool(line: &str, attr: &str) -> Result<bool> {
    let raw = parse_attr(line, attr)?;
    match raw {
        "True" => Ok(true),
        "False" => Ok(false),
        _ => anyhow::bail!("unexpected bool literal `{raw}` for attribute {attr}"),
    }
}

fn parse_attr_string(line: &str, attr: &str) -> Result<String> {
    Ok(parse_attr(line, attr)?.to_string())
}

fn parse_attr<'a>(line: &'a str, attr: &str) -> Result<&'a str> {
    let needle = format!(r#"{attr}=""#);
    let start = line
        .find(&needle)
        .with_context(|| format!("missing attribute {attr} in line `{line}`"))?
        + needle.len();
    let end = start
        + line[start..]
            .find('"')
            .with_context(|| format!("unterminated attribute {attr} in line `{line}`"))?;
    Ok(&line[start..end])
}

fn split_waypoint_name(raw_name: &str) -> (Option<String>, Option<String>) {
    if let Some(open) = raw_name.find('(') {
        if raw_name.ends_with(')') && open + 1 < raw_name.len() {
            let scope = &raw_name[..open];
            let token = &raw_name[open + 1..raw_name.len() - 1];
            return (
                (!scope.is_empty()).then(|| scope.to_string()),
                (!token.is_empty()).then(|| token.to_string()),
            );
        }
    }

    (None, None)
}

#[cfg(test)]
mod tests {
    use super::{parse_attr_bool, parse_attr_f64, parse_attr_u32, split_waypoint_name};

    #[test]
    fn parses_waypoint_attributes() {
        let line = r#"<Waypoint Key="2052" Name="town(olvia_academy)" PosX="-114942" PosY="-2674.33" PosZ="157114" Property="ground" IsSubWaypoint="True" IsEscape="False"/>"#;
        assert_eq!(parse_attr_u32(line, "Key").unwrap(), 2052);
        assert_eq!(parse_attr_f64(line, "PosY").unwrap(), -2674.33);
        assert!(parse_attr_bool(line, "IsSubWaypoint").unwrap());
        assert!(!parse_attr_bool(line, "IsEscape").unwrap());
    }

    #[test]
    fn splits_scope_and_token_from_waypoint_name() {
        assert_eq!(
            split_waypoint_name("town(olvia_academy)"),
            (Some("town".to_string()), Some("olvia_academy".to_string()))
        );
        assert_eq!(
            split_waypoint_name("hidden_town_velia(worker)"),
            (
                Some("hidden_town_velia".to_string()),
                Some("worker".to_string())
            )
        );
        assert_eq!(split_waypoint_name("plain_name"), (None, None));
    }
}
