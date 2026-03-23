use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::Result;
use fishystuff_core::loc::{scan_loc_records, LocRecord, LocScanSummary};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct LocInspectSummary {
    pub output_path: Option<PathBuf>,
    pub expected_uncompressed_size: u32,
    pub actual_uncompressed_size: usize,
    pub total_record_count: usize,
    pub layout_a_count: usize,
    pub layout_b_count: usize,
    pub namespace_count: usize,
    pub displayed_record_count: usize,
    pub missing_focus_key_count: usize,
}

#[derive(Debug, Clone)]
pub struct LocInspectResult {
    pub summary: LocInspectSummary,
    pub displayed_records: Vec<LocRecordSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocRecordSummary {
    pub format: String,
    pub key: u64,
    pub namespace: Option<u32>,
    pub text: String,
}

#[derive(Debug, Serialize)]
struct LocInspectReport {
    path: String,
    file_size: u64,
    expected_uncompressed_size: u32,
    actual_uncompressed_size: usize,
    total_record_count: usize,
    layout_a_count: usize,
    layout_b_count: usize,
    namespace_count: usize,
    displayed_records: Vec<LocRecordSummary>,
    missing_focus_keys: Vec<u64>,
}

pub fn inspect_loc(
    path: &Path,
    focus_namespaces: &[u32],
    focus_keys: &[u64],
    text_filters: &[String],
    limit: usize,
    max_len: usize,
    output_path: Option<&Path>,
) -> Result<LocInspectResult> {
    let namespace_filter = focus_namespaces.iter().copied().collect::<BTreeSet<_>>();
    let key_filter = focus_keys.iter().copied().collect::<BTreeSet<_>>();
    let text_filters = text_filters
        .iter()
        .map(|value| value.to_lowercase())
        .collect::<Vec<_>>();
    let has_filters =
        !namespace_filter.is_empty() || !key_filter.is_empty() || !text_filters.is_empty();

    let mut total_record_count = 0usize;
    let mut layout_a_count = 0usize;
    let mut layout_b_count = 0usize;
    let mut namespaces = BTreeSet::<u32>::new();
    let mut displayed_records = Vec::new();
    let mut matched_keys = BTreeSet::<u64>::new();

    let summary = scan_loc_records(path, max_len, |record| {
        total_record_count += 1;
        if let Some(namespace) = record.namespace {
            layout_b_count += 1;
            namespaces.insert(namespace);
        } else {
            layout_a_count += 1;
        }

        let namespace_matches = namespace_filter.is_empty()
            || record
                .namespace
                .is_some_and(|ns| namespace_filter.contains(&ns));
        let key_matches = key_filter.is_empty() || key_filter.contains(&record.key);
        let record_text_lower = (!text_filters.is_empty()).then(|| record.text.to_lowercase());
        let text_matches = text_filters.is_empty()
            || text_filters.iter().any(|needle| {
                record_text_lower
                    .as_deref()
                    .is_some_and(|text| text.contains(needle))
            });
        let matches = if has_filters {
            namespace_matches && key_matches && text_matches
        } else {
            displayed_records.len() < limit
        };

        if matches && displayed_records.len() < limit {
            displayed_records.push(record_summary(record));
        }
        if matches && key_filter.contains(&record.key) {
            matched_keys.insert(record.key);
        }
        Ok(())
    })?;

    let missing_focus_keys = key_filter
        .difference(&matched_keys)
        .copied()
        .collect::<Vec<_>>();

    if let Some(output_path) = output_path {
        let report = LocInspectReport {
            path: path.display().to_string(),
            file_size: summary.file_size,
            expected_uncompressed_size: summary.expected_uncompressed_size,
            actual_uncompressed_size: summary.actual_uncompressed_size,
            total_record_count,
            layout_a_count,
            layout_b_count,
            namespace_count: namespaces.len(),
            displayed_records: displayed_records.clone(),
            missing_focus_keys: missing_focus_keys.clone(),
        };
        super::write_json_report(output_path, &report)?;
    }

    Ok(LocInspectResult {
        summary: build_inspect_summary(
            &summary,
            output_path,
            total_record_count,
            layout_a_count,
            layout_b_count,
            namespaces.len(),
            displayed_records.len(),
            missing_focus_keys.len(),
        ),
        displayed_records,
    })
}

fn build_inspect_summary(
    summary: &LocScanSummary,
    output_path: Option<&Path>,
    total_record_count: usize,
    layout_a_count: usize,
    layout_b_count: usize,
    namespace_count: usize,
    displayed_record_count: usize,
    missing_focus_key_count: usize,
) -> LocInspectSummary {
    LocInspectSummary {
        output_path: output_path.map(Path::to_path_buf),
        expected_uncompressed_size: summary.expected_uncompressed_size,
        actual_uncompressed_size: summary.actual_uncompressed_size,
        total_record_count,
        layout_a_count,
        layout_b_count,
        namespace_count,
        displayed_record_count,
        missing_focus_key_count,
    }
}

fn record_summary(record: &LocRecord) -> LocRecordSummary {
    LocRecordSummary {
        format: record.format.as_str().to_string(),
        key: record.key,
        namespace: record.namespace,
        text: record.text.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::path::Path;

    use flate2::write::ZlibEncoder;
    use flate2::Compression;

    use super::inspect_loc;
    use fishystuff_core::loc::load_loc_namespaces_as_string_maps;

    fn encode_utf16le(value: &str) -> Vec<u8> {
        value.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    fn build_layout_a_record(key: u64, text: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(text.encode_utf16().count() as u64).to_le_bytes());
        bytes.extend_from_slice(&key.to_le_bytes());
        bytes.extend_from_slice(&encode_utf16le(text));
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        bytes
    }

    fn build_layout_b_record(key: u64, namespace: u32, text: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(text.encode_utf16().count() as u32).to_le_bytes());
        bytes.extend_from_slice(&namespace.to_le_bytes());
        bytes.extend_from_slice(&key.to_le_bytes());
        bytes.extend_from_slice(&encode_utf16le(text));
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        bytes
    }

    fn write_loc_file(path: &Path, payload: &[u8]) {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(payload).unwrap();
        let compressed = encoder.finish().unwrap();

        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&compressed);
        std::fs::write(path, bytes).unwrap();
    }

    fn temp_loc_path(label: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "pazifista-loc-test-{label}-{}.loc",
            std::process::id()
        ));
        path
    }

    #[test]
    fn inspects_filtered_namespace_and_keys() {
        let path = temp_loc_path("inspect");

        let mut payload = Vec::new();
        payload.extend_from_slice(&build_layout_a_record(
            860264,
            "Shining Adventure Support Pack",
        ));
        payload.extend_from_slice(&build_layout_b_record(2052, 29, "Olvia Academy"));
        payload.extend_from_slice(&build_layout_b_record(1739, 29, "Papua Crinea"));
        payload.extend_from_slice(&build_layout_b_record(88, 17, "Olvia"));
        write_loc_file(&path, &payload);

        let result = inspect_loc(&path, &[29], &[2052, 1739, 1746], &[], 10, 10_000, None).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(result.summary.total_record_count, 4);
        assert_eq!(result.summary.layout_a_count, 1);
        assert_eq!(result.summary.layout_b_count, 3);
        assert_eq!(result.summary.namespace_count, 2);
        assert_eq!(result.summary.displayed_record_count, 2);
        assert_eq!(result.summary.missing_focus_key_count, 1);
        assert_eq!(result.displayed_records[0].key, 2052);
        assert_eq!(result.displayed_records[0].namespace, Some(29));
        assert_eq!(result.displayed_records[0].text, "Olvia Academy");
        assert_eq!(result.displayed_records[1].key, 1739);
        assert_eq!(result.displayed_records[1].text, "Papua Crinea");
    }

    #[test]
    fn loads_selected_namespaces_as_string_maps() {
        let path = temp_loc_path("maps");

        let mut payload = Vec::new();
        payload.extend_from_slice(&build_layout_b_record(2052, 29, "Olvia Academy"));
        payload.extend_from_slice(&build_layout_b_record(1739, 29, "Papua Crinea"));
        payload.extend_from_slice(&build_layout_b_record(88, 17, "Olvia"));
        write_loc_file(&path, &payload);

        let maps = load_loc_namespaces_as_string_maps(&path, &[17, 29], 10_000).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(maps.get(&29).unwrap().get("2052").unwrap(), "Olvia Academy");
        assert_eq!(maps.get(&29).unwrap().get("1739").unwrap(), "Papua Crinea");
        assert_eq!(maps.get(&17).unwrap().get("88").unwrap(), "Olvia");
    }
}
