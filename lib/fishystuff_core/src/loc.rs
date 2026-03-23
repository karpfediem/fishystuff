use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::Path;

use anyhow::{bail, Context, Result};
use flate2::read::ZlibDecoder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocRecordFormat {
    A,
    B,
}

impl LocRecordFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocRecord {
    pub format: LocRecordFormat,
    pub key: u64,
    pub namespace: Option<u32>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct LocScanSummary {
    pub file_size: u64,
    pub expected_uncompressed_size: u32,
    pub actual_uncompressed_size: usize,
    pub total_record_count: usize,
    pub layout_a_count: usize,
    pub layout_b_count: usize,
    pub namespace_count: usize,
}

#[derive(Debug, Clone)]
struct DecompressedLoc {
    file_size: u64,
    expected_uncompressed_size: u32,
    blob: Vec<u8>,
}

pub fn scan_loc_records<F>(path: &Path, max_len: usize, mut visitor: F) -> Result<LocScanSummary>
where
    F: FnMut(&LocRecord) -> Result<()>,
{
    let loc = decompress_loc(path)?;
    let mut total_record_count = 0usize;
    let mut layout_a_count = 0usize;
    let mut layout_b_count = 0usize;
    let mut namespaces = BTreeSet::<u32>::new();

    scan_decompressed_loc_records(&loc.blob, max_len, |record| {
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
        visitor(record)
    })?;

    Ok(LocScanSummary {
        file_size: loc.file_size,
        expected_uncompressed_size: loc.expected_uncompressed_size,
        actual_uncompressed_size: loc.blob.len(),
        total_record_count,
        layout_a_count,
        layout_b_count,
        namespace_count: namespaces.len(),
    })
}

pub fn load_loc_namespaces_as_string_maps(
    path: &Path,
    focus_namespaces: &[u32],
    max_len: usize,
) -> Result<BTreeMap<u32, BTreeMap<String, String>>> {
    let namespace_filter = focus_namespaces.iter().copied().collect::<BTreeSet<_>>();
    let mut maps = focus_namespaces
        .iter()
        .copied()
        .map(|namespace| (namespace, BTreeMap::<String, String>::new()))
        .collect::<BTreeMap<_, _>>();

    scan_loc_records(path, max_len, |record| {
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

fn scan_decompressed_loc_records<F>(blob: &[u8], max_len: usize, mut visitor: F) -> Result<()>
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

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::path::Path;

    use flate2::write::ZlibEncoder;
    use flate2::Compression;

    use super::{load_loc_namespaces_as_string_maps, scan_loc_records};

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
            "fishystuff-core-loc-test-{label}-{}.loc",
            std::process::id()
        ));
        path
    }

    #[test]
    fn scans_records_and_counts_layouts() {
        let path = temp_loc_path("scan");
        let mut payload = Vec::new();
        payload.extend_from_slice(&build_layout_a_record(
            860264,
            "Shining Adventure Support Pack",
        ));
        payload.extend_from_slice(&build_layout_b_record(2052, 29, "Olvia Academy"));
        payload.extend_from_slice(&build_layout_b_record(88, 17, "Olvia"));
        write_loc_file(&path, &payload);

        let mut seen = Vec::new();
        let summary = scan_loc_records(&path, 10_000, |record| {
            seen.push((record.namespace, record.key, record.text.clone()));
            Ok(())
        })
        .unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(summary.total_record_count, 3);
        assert_eq!(summary.layout_a_count, 1);
        assert_eq!(summary.layout_b_count, 2);
        assert_eq!(summary.namespace_count, 2);
        assert_eq!(seen[1], (Some(29), 2052, "Olvia Academy".to_string()));
        assert_eq!(seen[2], (Some(17), 88, "Olvia".to_string()));
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
