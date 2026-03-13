use anyhow::Result;
use chrono::{NaiveDateTime, TimeZone, Utc};
use serde::Deserialize;
use serde::Deserializer;

#[derive(Debug, Deserialize)]
pub struct RankingRow {
    #[serde(rename = "Date")]
    pub date: String,
    #[serde(rename = "EncyclopediaKey")]
    pub encyclopedia_key: i32,
    #[serde(rename = "Length", deserialize_with = "deserialize_length")]
    pub length: f64,
    #[serde(rename = "X")]
    pub x: f64,
    #[serde(rename = "Y")]
    pub y: f64,
    #[serde(rename = "Z")]
    pub z: f64,
}

fn deserialize_length<'de, D>(deserializer: D) -> std::result::Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(0.0);
    }
    let normalized = trimmed.replace(',', ".");
    normalized.parse::<f64>().map_err(serde::de::Error::custom)
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
    use csv::ReaderBuilder;
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
        let mut rdr = ReaderBuilder::new()
            .delimiter(b';')
            .from_reader(Cursor::new(input));
        let mut iter = rdr.deserialize::<RankingRow>();
        let row = iter.next().expect("row").expect("ok");
        assert_eq!(row.encyclopedia_key, 123);
        assert!((row.length - 10.5).abs() < 1e-9);
        assert_eq!(row.x, 1.0);
        assert_eq!(row.y, 2.0);
        assert_eq!(row.z, 3.0);
    }
}
