use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use flate2::read::ZlibDecoder;
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

#[derive(Debug, Clone, Copy)]
enum LocRecordFormat {
    A,
    B,
}

#[derive(Debug, Clone)]
struct LocRecord {
    format: LocRecordFormat,
    key: u64,
    namespace: Option<u32>,
    text: String,
}

#[derive(Debug, Clone)]
struct DecompressedLoc {
    file_size: u64,
    expected_uncompressed_size: u32,
    blob: Vec<u8>,
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
    let loc = decompress_loc(path)?;
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

    scan_loc_records(&loc.blob, max_len, |record| {
        total_record_count += 1;
        match record.format {
            LocRecordFormat::A => layout_a_count += 1,
            LocRecordFormat::B => {
                layout_b_count += 1;
                if let Some(namespace) = record.namespace {
                    namespaces.insert(namespace);
                }
            }
        }

        let namespace_matches = namespace_filter.is_empty()
            || record
                .namespace
                .is_some_and(|ns| namespace_filter.contains(&ns));
        let key_matches = key_filter.is_empty() || key_filter.contains(&record.key);
        let text_matches = text_filters.is_empty()
            || text_filters
                .iter()
                .any(|needle| record.text.to_lowercase().contains(needle));
        let matches = if has_filters {
            namespace_matches && key_matches && text_matches
        } else {
            displayed_records.len() < limit
        };

        if matches && displayed_records.len() < limit {
            displayed_records.push(LocRecordSummary {
                format: format_name(record.format).to_string(),
                key: record.key,
                namespace: record.namespace,
                text: record.text.clone(),
            });
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
            file_size: loc.file_size,
            expected_uncompressed_size: loc.expected_uncompressed_size,
            actual_uncompressed_size: loc.blob.len(),
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
        summary: LocInspectSummary {
            output_path: output_path.map(Path::to_path_buf),
            expected_uncompressed_size: loc.expected_uncompressed_size,
            actual_uncompressed_size: loc.blob.len(),
            total_record_count,
            layout_a_count,
            layout_b_count,
            namespace_count: namespaces.len(),
            displayed_record_count: displayed_records.len(),
            missing_focus_key_count: missing_focus_keys.len(),
        },
        displayed_records,
    })
}

pub fn load_loc_namespaces_as_string_maps(
    path: &Path,
    focus_namespaces: &[u32],
    max_len: usize,
) -> Result<BTreeMap<u32, BTreeMap<String, String>>> {
    let loc = decompress_loc(path)?;
    let namespace_filter = focus_namespaces.iter().copied().collect::<BTreeSet<_>>();
    let mut maps = focus_namespaces
        .iter()
        .copied()
        .map(|namespace| (namespace, BTreeMap::<String, String>::new()))
        .collect::<BTreeMap<_, _>>();

    scan_loc_records(&loc.blob, max_len, |record| {
        let Some(namespace) = record.namespace else {
            return Ok(());
        };
        if !namespace_filter.contains(&namespace) {
            return Ok(());
        }
        let Ok(key) = u32::try_from(record.key) else {
            return Ok(());
        };
        maps.entry(namespace)
            .or_default()
            .entry(key.to_string())
            .or_insert(record.text.clone());
        Ok(())
    })?;

    Ok(maps)
}

fn decompress_loc(path: &Path) -> Result<DecompressedLoc> {
    let raw =
        fs::read(path).with_context(|| format!("failed to read .loc file {}", path.display()))?;
    if raw.len() < 5 {
        bail!(".loc file {} is too small", path.display());
    }

    let expected_uncompressed_size = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);
    let mut decoder = ZlibDecoder::new(&raw[4..]);
    let mut blob = Vec::new();
    decoder
        .read_to_end(&mut blob)
        .with_context(|| format!("failed to zlib-decompress .loc file {}", path.display()))?;

    Ok(DecompressedLoc {
        file_size: raw.len() as u64,
        expected_uncompressed_size,
        blob,
    })
}

fn scan_loc_records<F>(blob: &[u8], max_len: usize, mut visitor: F) -> Result<()>
where
    F: FnMut(&LocRecord) -> Result<()>,
{
    let mut pos = 0usize;
    while pos + 16 <= blob.len() {
        if let Some((next_pos, record)) = try_parse_layout_a(blob, pos, max_len)? {
            visitor(&record)?;
            pos = next_pos;
            continue;
        }
        if let Some((next_pos, record)) = try_parse_layout_b(blob, pos, max_len)? {
            visitor(&record)?;
            pos = next_pos;
            continue;
        }
        break;
    }

    Ok(())
}

fn try_parse_layout_a(
    blob: &[u8],
    pos: usize,
    max_len: usize,
) -> Result<Option<(usize, LocRecord)>> {
    let len64 = u64::from_le_bytes(read_array::<8>(blob, pos)?);
    if len64 == 0 || len64 > max_len as u64 {
        return Ok(None);
    }

    let char_len = usize::try_from(len64).context("layout A char length does not fit usize")?;
    let end = pos
        .checked_add(16)
        .and_then(|offset| offset.checked_add(char_len.checked_mul(2)?))
        .context("layout A end offset overflow")?;
    if end + 4 > blob.len() || blob[end..end + 4] != [0, 0, 0, 0] {
        return Ok(None);
    }

    let key = u64::from_le_bytes(read_array::<8>(blob, pos + 8)?);
    let text = decode_utf16le(&blob[pos + 16..end])?;
    Ok(Some((
        end + 4,
        LocRecord {
            format: LocRecordFormat::A,
            key,
            namespace: None,
            text,
        },
    )))
}

fn try_parse_layout_b(
    blob: &[u8],
    pos: usize,
    max_len: usize,
) -> Result<Option<(usize, LocRecord)>> {
    let len32 = u32::from_le_bytes(read_array::<4>(blob, pos)?);
    if len32 == 0 || len32 > max_len as u32 {
        return Ok(None);
    }

    let char_len = usize::try_from(len32).context("layout B char length does not fit usize")?;
    let end = pos
        .checked_add(16)
        .and_then(|offset| offset.checked_add(char_len.checked_mul(2)?))
        .context("layout B end offset overflow")?;
    if end + 4 > blob.len() || blob[end..end + 4] != [0, 0, 0, 0] {
        return Ok(None);
    }

    let namespace = u32::from_le_bytes(read_array::<4>(blob, pos + 4)?);
    let key = u64::from_le_bytes(read_array::<8>(blob, pos + 8)?);
    let text = decode_utf16le(&blob[pos + 16..end])?;
    Ok(Some((
        end + 4,
        LocRecord {
            format: LocRecordFormat::B,
            key,
            namespace: Some(namespace),
            text,
        },
    )))
}

fn decode_utf16le(slice: &[u8]) -> Result<String> {
    if slice.len() % 2 != 0 {
        bail!("UTF-16LE slice has odd byte length {}", slice.len());
    }
    let units = slice
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect::<Vec<_>>();
    Ok(char::decode_utf16(units)
        .map(|value| value.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect())
}

fn read_array<const N: usize>(blob: &[u8], pos: usize) -> Result<[u8; N]> {
    let end = pos
        .checked_add(N)
        .with_context(|| format!("record offset overflow at {}", pos))?;
    let slice = blob
        .get(pos..end)
        .with_context(|| format!("record slice [{}..{}) is out of bounds", pos, end))?;
    let mut bytes = [0u8; N];
    bytes.copy_from_slice(slice);
    Ok(bytes)
}

fn format_name(format: LocRecordFormat) -> &'static str {
    match format {
        LocRecordFormat::A => "A",
        LocRecordFormat::B => "B",
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::path::Path;

    use flate2::write::ZlibEncoder;
    use flate2::Compression;

    use super::{inspect_loc, load_loc_namespaces_as_string_maps};

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
