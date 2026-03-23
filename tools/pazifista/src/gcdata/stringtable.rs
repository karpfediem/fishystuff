use std::char::decode_utf16;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct StringtableBssInspectSummary {
    pub output_path: Option<PathBuf>,
    pub header_section_count: usize,
    pub total_index_row_count: usize,
    pub text_entry_count: usize,
    pub focus_entry_count: usize,
    pub missing_focus_entry_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct StringtableIndexSectionSummary {
    pub section_index: usize,
    pub header_offset: usize,
    pub header_hash_hex: String,
    pub start_id: u32,
    pub row_count: usize,
    pub payload_offset: usize,
    pub payload_end_offset: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct StringtableRowReference {
    pub section_index: usize,
    pub row_index: usize,
    pub row_offset: usize,
    pub hash: u32,
    pub hash_hex: String,
    pub first_id: u32,
    pub second_id: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct StringtableTextEntrySummary {
    pub string_id: u32,
    pub offset: usize,
    pub tag: u8,
    pub byte_len: usize,
    pub text: String,
}

#[derive(Debug, Serialize)]
struct StringtableBssInspectReport {
    path: String,
    file_size: u64,
    header_section_count: usize,
    sections: Vec<StringtableIndexSectionSummary>,
    trailing_text_count_offset: usize,
    trailing_text_payload_offset: usize,
    trailing_text_entry_count: usize,
    trailing_text_end_offset: usize,
    focus_entries: Vec<StringtableFocusEntryReport>,
    missing_focus_string_ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
struct StringtableFocusEntryReport {
    string_id: u32,
    entry: Option<StringtableTextEntrySummary>,
    referenced_by_rows: Vec<StringtableRowReference>,
}

#[derive(Debug, Clone)]
struct StringtableIndexSection {
    summary: StringtableIndexSectionSummary,
    rows: Vec<StringtableIndexRow>,
}

#[derive(Debug, Clone)]
struct StringtableIndexRow {
    row_index: usize,
    row_offset: usize,
    hash: u32,
    first_id: u32,
    second_id: u32,
}

#[derive(Debug, Clone)]
struct StringtableTextEntry {
    string_id: u32,
    offset: usize,
    tag: u8,
    byte_len: usize,
    text: String,
}

pub fn inspect_stringtable_bss(
    path: &Path,
    focus_string_ids: &[u32],
    output_path: Option<&Path>,
) -> Result<StringtableBssInspectSummary> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read stringtable.bss {}", path.display()))?;
    if bytes.len() < 12 {
        bail!("stringtable.bss is too small");
    }
    if &bytes[0..4] != b"PABR" {
        bail!("stringtable.bss is missing PABR magic");
    }

    let header_section_count = read_u32(&bytes, 4)
        .context("failed to read stringtable.bss top-level section count")?
        as usize;
    let sections = parse_stringtable_index_sections(&bytes, header_section_count)?;
    let total_index_row_count = sections.iter().map(|section| section.rows.len()).sum();
    let trailing_text_count_offset = sections
        .last()
        .map(|section| section.summary.payload_end_offset)
        .unwrap_or(8);
    let trailing_text_entry_count = read_u32(&bytes, trailing_text_count_offset)
        .context("failed to read trailing stringtable entry count")?
        as usize;
    let trailing_text_payload_offset = trailing_text_count_offset
        .checked_add(4)
        .context("stringtable trailing text payload offset overflow")?;
    let text_entries = parse_stringtable_text_entries(
        &bytes,
        trailing_text_payload_offset,
        trailing_text_entry_count,
    )?;
    let trailing_text_end_offset = text_entries
        .last()
        .map(|entry| entry.offset + 5 + entry.byte_len)
        .unwrap_or(trailing_text_payload_offset);

    let focus_reports = focus_string_ids
        .iter()
        .copied()
        .map(|string_id| StringtableFocusEntryReport {
            string_id,
            entry: text_entries
                .get(
                    usize::try_from(string_id)
                        .with_context(|| format!("string id {} does not fit usize", string_id))
                        .unwrap_or(usize::MAX),
                )
                .cloned()
                .map(stringtable_text_entry_summary),
            referenced_by_rows: sections
                .iter()
                .flat_map(|section| {
                    section.rows.iter().filter_map(move |row| {
                        (row.first_id == string_id || row.second_id == string_id).then(|| {
                            StringtableRowReference {
                                section_index: section.summary.section_index,
                                row_index: row.row_index,
                                row_offset: row.row_offset,
                                hash: row.hash,
                                hash_hex: format!("0x{:08x}", row.hash),
                                first_id: row.first_id,
                                second_id: row.second_id,
                            }
                        })
                    })
                })
                .collect(),
        })
        .collect::<Vec<_>>();

    let missing_focus_string_ids = focus_string_ids
        .iter()
        .copied()
        .filter(|string_id| {
            usize::try_from(*string_id)
                .ok()
                .and_then(|index| text_entries.get(index))
                .is_none()
        })
        .collect::<Vec<_>>();

    if let Some(output_path) = output_path {
        let report = StringtableBssInspectReport {
            path: path.display().to_string(),
            file_size: bytes.len() as u64,
            header_section_count,
            sections: sections
                .iter()
                .map(|section| section.summary.clone())
                .collect(),
            trailing_text_count_offset,
            trailing_text_payload_offset,
            trailing_text_entry_count,
            trailing_text_end_offset,
            focus_entries: focus_reports.clone(),
            missing_focus_string_ids: missing_focus_string_ids.clone(),
        };
        write_json_report(output_path, &report)?;
    }

    Ok(StringtableBssInspectSummary {
        output_path: output_path.map(Path::to_path_buf),
        header_section_count,
        total_index_row_count,
        text_entry_count: text_entries.len(),
        focus_entry_count: focus_string_ids
            .len()
            .saturating_sub(missing_focus_string_ids.len()),
        missing_focus_entry_count: missing_focus_string_ids.len(),
    })
}

fn parse_stringtable_index_sections(
    bytes: &[u8],
    section_count: usize,
) -> Result<Vec<StringtableIndexSection>> {
    let mut cursor = 8usize;
    let mut sections = Vec::with_capacity(section_count);
    for section_index in 0..section_count {
        let header_offset = cursor;
        let header_hash = read_u32(bytes, cursor).with_context(|| {
            format!("failed to read stringtable section {} hash", section_index)
        })?;
        let start_id = read_u32(bytes, cursor + 4).with_context(|| {
            format!(
                "failed to read stringtable section {} start id",
                section_index
            )
        })?;
        let row_count = usize::try_from(read_u32(bytes, cursor + 8).with_context(|| {
            format!(
                "failed to read stringtable section {} row count",
                section_index
            )
        })?)
        .with_context(|| {
            format!(
                "stringtable section {} row count does not fit usize",
                section_index
            )
        })?;
        let payload_offset = cursor
            .checked_add(12)
            .context("stringtable section payload offset overflow")?;
        let payload_len = row_count
            .checked_mul(16)
            .context("stringtable section payload length overflow")?;
        let payload_end_offset = payload_offset
            .checked_add(payload_len)
            .context("stringtable section payload end overflow")?;
        if payload_end_offset > bytes.len() {
            bail!(
                "stringtable section {} payload exceeds file bounds: {} > {}",
                section_index,
                payload_end_offset,
                bytes.len()
            );
        }

        let mut rows = Vec::with_capacity(row_count);
        let mut row_cursor = payload_offset;
        for row_index in 0..row_count {
            let hash = read_u32(bytes, row_cursor).with_context(|| {
                format!(
                    "failed to read stringtable section {} row {} hash",
                    section_index, row_index
                )
            })?;
            let first_id = read_u32(bytes, row_cursor + 4).with_context(|| {
                format!(
                    "failed to read stringtable section {} row {} first id",
                    section_index, row_index
                )
            })?;
            let second_id = read_u32(bytes, row_cursor + 8).with_context(|| {
                format!(
                    "failed to read stringtable section {} row {} second id",
                    section_index, row_index
                )
            })?;
            let zero = read_u32(bytes, row_cursor + 12).with_context(|| {
                format!(
                    "failed to read stringtable section {} row {} trailer",
                    section_index, row_index
                )
            })?;
            if zero != 0 {
                bail!(
                    "stringtable section {} row {} expected zero trailer, got 0x{:08x}",
                    section_index,
                    row_index,
                    zero
                );
            }

            rows.push(StringtableIndexRow {
                row_index,
                row_offset: row_cursor,
                hash,
                first_id,
                second_id,
            });
            row_cursor += 16;
        }

        sections.push(StringtableIndexSection {
            summary: StringtableIndexSectionSummary {
                section_index,
                header_offset,
                header_hash_hex: format!("0x{:08x}", header_hash),
                start_id,
                row_count,
                payload_offset,
                payload_end_offset,
            },
            rows,
        });
        cursor = payload_end_offset;
    }

    Ok(sections)
}

fn parse_stringtable_text_entries(
    bytes: &[u8],
    start_offset: usize,
    entry_count: usize,
) -> Result<Vec<StringtableTextEntry>> {
    let mut entries = Vec::with_capacity(entry_count);
    let mut cursor = start_offset;
    for string_id in 0..entry_count {
        let tag = *bytes.get(cursor).with_context(|| {
            format!(
                "failed to read stringtable text entry {} tag at offset {}",
                string_id, cursor
            )
        })?;
        let byte_len = usize::try_from(read_u32(bytes, cursor + 1).with_context(|| {
            format!(
                "failed to read stringtable text entry {} byte length at offset {}",
                string_id,
                cursor + 1
            )
        })?)
        .with_context(|| {
            format!(
                "stringtable text entry {} byte length does not fit usize",
                string_id
            )
        })?;
        let text_offset = cursor
            .checked_add(5)
            .context("stringtable text entry offset overflow")?;
        let text_end = text_offset
            .checked_add(byte_len)
            .context("stringtable text entry end overflow")?;
        let slice = bytes.get(text_offset..text_end).with_context(|| {
            format!(
                "stringtable text entry {} payload [{}..{}) is out of bounds",
                string_id, text_offset, text_end
            )
        })?;
        if byte_len % 2 != 0 {
            bail!(
                "stringtable text entry {} has odd UTF-16LE byte length {}",
                string_id,
                byte_len
            );
        }

        let units = slice
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        let text = decode_utf16(units)
            .map(|result| result.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect::<String>();

        entries.push(StringtableTextEntry {
            string_id: u32::try_from(string_id).context("string id does not fit u32")?,
            offset: cursor,
            tag,
            byte_len,
            text,
        });
        cursor = text_end;
    }

    Ok(entries)
}

fn stringtable_text_entry_summary(entry: StringtableTextEntry) -> StringtableTextEntrySummary {
    StringtableTextEntrySummary {
        string_id: entry.string_id,
        offset: entry.offset,
        tag: entry.tag,
        byte_len: entry.byte_len,
        text: entry.text,
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let end = offset
        .checked_add(4)
        .context("u32 offset overflow while parsing stringtable.bss")?;
    let slice = bytes
        .get(offset..end)
        .with_context(|| format!("u32 read at offset {} is out of bounds", offset))?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn write_json_report<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let file =
        File::create(path).with_context(|| format!("failed to create {}", path.display()))?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, value)
        .with_context(|| format!("failed to write JSON report {}", path.display()))
}
