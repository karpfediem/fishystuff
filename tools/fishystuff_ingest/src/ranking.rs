use std::io::Read;

use anyhow::{bail, Context, Result};
use chrono::{NaiveDateTime, TimeZone, Utc};
use csv::{Reader, ReaderBuilder, StringRecord};

#[derive(Debug, Clone, PartialEq)]
pub struct RankingRow {
    pub date: String,
    pub fish_id: i32,
    pub encyclopedia_key: i32,
    pub length: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RankingCsvLayout {
    DocumentedOrder,
    OriginalHeaderless,
}

pub struct RankingCsvReader<R: Read> {
    rdr: Reader<R>,
    layout: RankingCsvLayout,
    pending_record: Option<StringRecord>,
}

impl<R: Read> RankingCsvReader<R> {
    pub fn next_row(&mut self) -> Option<Result<RankingRow>> {
        if let Some(record) = self.pending_record.take() {
            return Some(parse_record(&record, self.layout));
        }

        let mut record = StringRecord::new();
        match self.rdr.read_record(&mut record) {
            Ok(true) => Some(parse_record(&record, self.layout)),
            Ok(false) => None,
            Err(err) => Some(Err(err).context("read ranking row")),
        }
    }
}

pub fn open_ranking_reader<R: Read>(reader: R) -> Result<RankingCsvReader<R>> {
    let mut rdr = ReaderBuilder::new()
        .delimiter(b';')
        .has_headers(false)
        .from_reader(reader);
    let mut first_record = StringRecord::new();
    if !rdr
        .read_record(&mut first_record)
        .context("read ranking csv first record")?
    {
        bail!("ranking csv is empty");
    }

    let (layout, pending_record) = if is_documented_header_record(&first_record) {
        (RankingCsvLayout::DocumentedOrder, None)
    } else if looks_like_documented_row(&first_record) {
        (RankingCsvLayout::DocumentedOrder, Some(first_record))
    } else if looks_like_original_row(&first_record) {
        (RankingCsvLayout::OriginalHeaderless, Some(first_record))
    } else {
        bail!(
            "unsupported ranking csv schema: expected documented header row, documented row order, or original headerless order"
        );
    };

    Ok(RankingCsvReader {
        rdr,
        layout,
        pending_record,
    })
}

fn parse_record(record: &StringRecord, layout: RankingCsvLayout) -> Result<RankingRow> {
    match layout {
        RankingCsvLayout::DocumentedOrder => parse_documented_record(record),
        RankingCsvLayout::OriginalHeaderless => parse_original_record(record),
    }
}

fn parse_documented_record(record: &StringRecord) -> Result<RankingRow> {
    ensure_record_len(record, 8)?;
    let encyclopedia_key = parse_i32_field(record, 1, "EncyclopediaKey")?;
    Ok(RankingRow {
        date: field(record, 0, "Date")?.to_string(),
        fish_id: encyclopedia_key,
        encyclopedia_key,
        length: parse_length(field(record, 2, "Length")?)?,
        x: parse_f64_field(record, 5, "X")?,
        y: parse_f64_field(record, 6, "Y")?,
        z: parse_f64_field(record, 7, "Z")?,
    })
}

fn parse_original_record(record: &StringRecord) -> Result<RankingRow> {
    ensure_record_len(record, 8)?;
    Ok(RankingRow {
        date: field(record, 7, "Date")?.to_string(),
        fish_id: parse_i32_field(record, 2, "FishId")?,
        encyclopedia_key: parse_i32_field(record, 1, "EncyclopediaKey")?,
        length: parse_length(field(record, 3, "Length")?)?,
        x: parse_f64_field(record, 4, "X")?,
        y: parse_f64_field(record, 5, "Y")?,
        z: parse_f64_field(record, 6, "Z")?,
    })
}

fn ensure_record_len(record: &StringRecord, expected_len: usize) -> Result<()> {
    if record.len() != expected_len {
        bail!(
            "expected {expected_len} ranking csv fields, got {}: {:?}",
            record.len(),
            record
        );
    }
    Ok(())
}

fn field<'a>(record: &'a StringRecord, index: usize, name: &str) -> Result<&'a str> {
    record
        .get(index)
        .map(str::trim)
        .with_context(|| format!("missing field `{name}` at index {index}"))
}

fn parse_i32_field(record: &StringRecord, index: usize, name: &str) -> Result<i32> {
    let value = field(record, index, name)?;
    value
        .parse::<i32>()
        .with_context(|| format!("parse field `{name}` as i32: {value}"))
}

fn parse_f64_field(record: &StringRecord, index: usize, name: &str) -> Result<f64> {
    let value = field(record, index, name)?;
    value
        .replace(',', ".")
        .parse::<f64>()
        .with_context(|| format!("parse field `{name}` as f64: {value}"))
}

fn parse_length(raw: &str) -> Result<f64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(0.0);
    }
    trimmed
        .replace(',', ".")
        .parse::<f64>()
        .with_context(|| format!("parse length: {trimmed}"))
}

fn is_documented_header_record(record: &StringRecord) -> bool {
    record.len() == 8
        && record.get(0).map(str::trim) == Some("Date")
        && record.get(1).map(str::trim) == Some("EncyclopediaKey")
        && record.get(2).map(str::trim) == Some("Length")
        && record.get(5).map(str::trim) == Some("X")
        && record.get(6).map(str::trim) == Some("Y")
        && record.get(7).map(str::trim) == Some("Z")
}

fn looks_like_documented_row(record: &StringRecord) -> bool {
    record.len() == 8
        && parse_datetime_utc(record.get(0).map(str::trim).unwrap_or_default()).is_ok()
        && record
            .get(1)
            .map(str::trim)
            .unwrap_or_default()
            .parse::<i32>()
            .is_ok()
        && parse_length(record.get(2).map(str::trim).unwrap_or_default()).is_ok()
        && record
            .get(5)
            .map(str::trim)
            .unwrap_or_default()
            .replace(',', ".")
            .parse::<f64>()
            .is_ok()
        && record
            .get(6)
            .map(str::trim)
            .unwrap_or_default()
            .replace(',', ".")
            .parse::<f64>()
            .is_ok()
        && record
            .get(7)
            .map(str::trim)
            .unwrap_or_default()
            .replace(',', ".")
            .parse::<f64>()
            .is_ok()
}

fn looks_like_original_row(record: &StringRecord) -> bool {
    record.len() == 8
        && parse_datetime_utc(record.get(7).map(str::trim).unwrap_or_default()).is_ok()
        && record
            .get(1)
            .map(str::trim)
            .unwrap_or_default()
            .parse::<i32>()
            .is_ok()
        && parse_length(record.get(3).map(str::trim).unwrap_or_default()).is_ok()
        && record
            .get(4)
            .map(str::trim)
            .unwrap_or_default()
            .replace(',', ".")
            .parse::<f64>()
            .is_ok()
        && record
            .get(5)
            .map(str::trim)
            .unwrap_or_default()
            .replace(',', ".")
            .parse::<f64>()
            .is_ok()
        && record
            .get(6)
            .map(str::trim)
            .unwrap_or_default()
            .replace(',', ".")
            .parse::<f64>()
            .is_ok()
}

pub fn parse_datetime_utc(value: &str) -> Result<i64> {
    const FORMATS: [&str; 4] = [
        "%d.%m.%Y %H:%M",
        "%Y-%m-%d %I:%M:%S %p",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
    ];
    for fmt in FORMATS {
        if let Ok(dt) = NaiveDateTime::parse_from_str(value, fmt) {
            return Ok(Utc.from_utc_datetime(&dt).timestamp());
        }
    }
    Err(anyhow::anyhow!(
        "parse datetime: {} (supported: DD.MM.YYYY HH:MM, YYYY-MM-DD HH:MM[:SS], YYYY-MM-DD HH:MM:SS AM/PM)",
        value
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parse_datetime_utc_basic() {
        let ts = parse_datetime_utc("01.02.2024 12:34").expect("timestamp");
        let expected = Utc
            .with_ymd_and_hms(2024, 2, 1, 12, 34, 0)
            .unwrap()
            .timestamp();
        assert_eq!(ts, expected);
    }

    #[test]
    fn parse_datetime_utc_iso_ampm() {
        let ts = parse_datetime_utc("2025-04-25 11:57:51 PM").expect("timestamp");
        let expected = Utc
            .with_ymd_and_hms(2025, 4, 25, 23, 57, 51)
            .unwrap()
            .timestamp();
        assert_eq!(ts, expected);
    }

    #[test]
    fn parse_datetime_utc_iso_24h() {
        let ts = parse_datetime_utc("2025-04-25 23:57:51").expect("timestamp");
        let expected = Utc
            .with_ymd_and_hms(2025, 4, 25, 23, 57, 51)
            .unwrap()
            .timestamp();
        assert_eq!(ts, expected);
    }

    #[test]
    fn parse_csv_semicolon() {
        let input = "Date;EncyclopediaKey;Length;FamilyName;CharacterName;X;Y;Z\n\
                     01.02.2024 12:34;123;10.5;Fam;Char;1.0;2.0;3.0\n";
        let mut rdr = open_ranking_reader(Cursor::new(input)).expect("reader");
        let row = rdr.next_row().expect("row").expect("ok");
        assert_eq!(row.fish_id, 123);
        assert_eq!(row.encyclopedia_key, 123);
        assert!((row.length - 10.5).abs() < 1e-9);
        assert_eq!(row.x, 1.0);
        assert_eq!(row.y, 2.0);
        assert_eq!(row.z, 3.0);
        assert!(rdr.next_row().is_none());
    }

    #[test]
    fn parse_csv_without_headers_in_documented_order() {
        let input = "01.02.2024 12:34;123;10.5;Fam;Char;1.0;2.0;3.0\n";
        let mut rdr = open_ranking_reader(Cursor::new(input)).expect("reader");
        let row = rdr.next_row().expect("row").expect("ok");
        assert_eq!(
            row,
            RankingRow {
                date: "01.02.2024 12:34".to_string(),
                fish_id: 123,
                encyclopedia_key: 123,
                length: 10.5,
                x: 1.0,
                y: 2.0,
                z: 3.0,
            }
        );
        assert!(rdr.next_row().is_none());
    }

    #[test]
    fn parse_original_headerless_csv() {
        let input = "Salmon;8205;5;127.89;-245980.0;-3814.0;-48696.0;20.04.2025 11:17\n";
        let mut rdr = open_ranking_reader(Cursor::new(input)).expect("reader");
        let row = rdr.next_row().expect("row").expect("ok");
        assert_eq!(
            row,
            RankingRow {
                date: "20.04.2025 11:17".to_string(),
                fish_id: 5,
                encyclopedia_key: 8205,
                length: 127.89,
                x: -245980.0,
                y: -3814.0,
                z: -48696.0,
            }
        );
        assert!(rdr.next_row().is_none());
    }
}
