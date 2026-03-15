use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use anyhow::{bail, Context, Result};
use csv::{ReaderBuilder, StringRecord};
use fishystuff_core::masks::pack_rgb_u32;

#[derive(Debug, Clone)]
pub struct ZoneMeta {
    pub rgb_u32: u32,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub name: Option<String>,
    pub active: Option<String>,
    pub confirmed: Option<String>,
    pub index: Option<String>,
    pub bite_time_min: Option<String>,
    pub bite_time_max: Option<String>,
}

pub trait ZonesMetaProvider {
    fn load(&self, ref_opt: Option<&str>) -> Result<HashMap<u32, ZoneMeta>>;
}

pub struct CsvZonesMetaProvider {
    path: PathBuf,
}

impl CsvZonesMetaProvider {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn load_from_reader<R: Read>(reader: R) -> Result<HashMap<u32, ZoneMeta>> {
        parse_zones_csv(reader)
    }
}

impl ZonesMetaProvider for CsvZonesMetaProvider {
    fn load(&self, _ref_opt: Option<&str>) -> Result<HashMap<u32, ZoneMeta>> {
        let file = File::open(&self.path)
            .with_context(|| format!("open zones csv: {}", self.path.display()))?;
        parse_zones_csv(file)
    }
}

pub struct DoltZonesMetaProvider {
    repo_path: PathBuf,
    cache: Mutex<HashMap<String, HashMap<u32, ZoneMeta>>>,
}

impl DoltZonesMetaProvider {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            repo_path: path.as_ref().to_path_buf(),
            cache: Mutex::new(HashMap::new()),
        }
    }
}

impl ZonesMetaProvider for DoltZonesMetaProvider {
    fn load(&self, ref_opt: Option<&str>) -> Result<HashMap<u32, ZoneMeta>> {
        let ref_key = ref_opt.unwrap_or("HEAD").to_string();
        if let Some(cached) = self
            .cache
            .lock()
            .ok()
            .and_then(|c| c.get(&ref_key).cloned())
        {
            return Ok(cached);
        }

        let csv_bytes = dolt_query_zones(&self.repo_path, &ref_key)?;
        let data = parse_zones_csv(&csv_bytes[..])?;
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(ref_key, data.clone());
        }
        Ok(data)
    }
}

fn dolt_query_zones(repo_path: &Path, ref_key: &str) -> Result<Vec<u8>> {
    let query = format!("SELECT * FROM `zones_merged` AS OF '{}';", ref_key);
    let output = Command::new("dolt")
        .current_dir(repo_path)
        .args(["sql", "--result-format", "csv", "-q", &query])
        .output()
        .context("run dolt sql (AS OF)")?;
    if output.status.success() {
        return Ok(output.stdout);
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let help = Command::new("dolt")
        .current_dir(repo_path)
        .args(["sql", "--help"])
        .output()
        .context("run dolt sql --help")?;
    let help_text = String::from_utf8_lossy(&help.stdout);
    let help_err = String::from_utf8_lossy(&help.stderr);
    let help_combined = format!("{help_text}\n{help_err}");

    if help_combined.contains("--ref") {
        let output = Command::new("dolt")
            .current_dir(repo_path)
            .args([
                "sql",
                "--result-format",
                "csv",
                "--ref",
                ref_key,
                "-q",
                "SELECT * FROM `zones_merged`;",
            ])
            .output()
            .context("run dolt sql --ref")?;
        if output.status.success() {
            return Ok(output.stdout);
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("dolt sql --ref failed: {stderr}");
    }

    bail!(
        "dolt does not support ref selection via AS OF or --ref; update dolt or use a ref-capable build. AS OF error: {stderr}"
    );
}

fn parse_zones_csv<R: Read>(reader: R) -> Result<HashMap<u32, ZoneMeta>> {
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(reader);
    let headers = rdr.headers().context("read zones csv headers")?.clone();
    let header_map = build_header_map(&headers);

    let r_idx = header_idx(&header_map, &["r", "red"])?;
    let g_idx = header_idx(&header_map, &["g", "green"])?;
    let b_idx = header_idx(&header_map, &["b", "blue"])?;

    let name_idx = header_idx_opt(&header_map, &["zone_name", "zone name", "name", "zone"]);
    let active_idx = header_idx_opt(&header_map, &["active"]);
    let confirmed_idx = header_idx_opt(&header_map, &["confirmed"]);
    let index_idx = header_idx_opt(&header_map, &["index"]);
    let bite_min_idx = header_idx_opt(
        &header_map,
        &["bite_time_min", "bite_time_start", "bite_time_from"],
    );
    let bite_max_idx = header_idx_opt(
        &header_map,
        &["bite_time_max", "bite_time_end", "bite_time_to"],
    );

    let mut out = HashMap::new();
    for result in rdr.records() {
        let record = result.context("read zones csv record")?;
        let r = parse_u8(&record, r_idx, "r")?;
        let g = parse_u8(&record, g_idx, "g")?;
        let b = parse_u8(&record, b_idx, "b")?;
        let rgb_u32 = pack_rgb_u32(r, g, b);
        let meta = ZoneMeta {
            rgb_u32,
            r,
            g,
            b,
            name: opt_field(&record, name_idx),
            active: opt_field(&record, active_idx),
            confirmed: opt_field(&record, confirmed_idx),
            index: opt_field(&record, index_idx),
            bite_time_min: opt_field(&record, bite_min_idx),
            bite_time_max: opt_field(&record, bite_max_idx),
        };
        out.insert(rgb_u32, meta);
    }
    Ok(out)
}

fn build_header_map(headers: &StringRecord) -> HashMap<String, usize> {
    let mut map = HashMap::new();
    for (idx, name) in headers.iter().enumerate() {
        let key = name.trim().to_lowercase();
        map.insert(key, idx);
    }
    map
}

fn header_idx(map: &HashMap<String, usize>, names: &[&str]) -> Result<usize> {
    header_idx_opt(map, names)
        .ok_or_else(|| anyhow::anyhow!("missing required column (one of): {}", names.join(", ")))
}

fn header_idx_opt(map: &HashMap<String, usize>, names: &[&str]) -> Option<usize> {
    names
        .iter()
        .filter_map(|name| map.get(&name.to_lowercase()).copied())
        .next()
}

fn opt_field(record: &StringRecord, idx: Option<usize>) -> Option<String> {
    let idx = idx?;
    let val = record.get(idx)?.trim();
    if val.is_empty() {
        return None;
    }
    let lower = val.to_lowercase();
    if lower == "null" || lower == "<null>" {
        return None;
    }
    Some(val.to_string())
}

fn parse_u8(record: &StringRecord, idx: usize, name: &str) -> Result<u8> {
    let raw = record
        .get(idx)
        .map(str::trim)
        .ok_or_else(|| anyhow::anyhow!("missing value for {}", name))?;
    raw.parse::<u8>()
        .with_context(|| format!("parse {} as u8: {}", name, raw))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::process::Command;

    #[test]
    fn parse_csv_basic() {
        let csv = "r,g,b,zone_name,active,bite_time_min,bite_time_max\n\
                   10,20,30,Test Zone,yes,0600,1800\n";
        let data = CsvZonesMetaProvider::load_from_reader(Cursor::new(csv)).expect("parse");
        let rgb = pack_rgb_u32(10, 20, 30);
        let meta = data.get(&rgb).expect("meta");
        assert_eq!(meta.name.as_deref(), Some("Test Zone"));
        assert_eq!(meta.active.as_deref(), Some("yes"));
        assert_eq!(meta.bite_time_min.as_deref(), Some("0600"));
        assert_eq!(meta.bite_time_max.as_deref(), Some("1800"));
    }

    #[test]
    fn parse_csv_nulls_as_none() {
        let csv = "r,g,b,name,active\n10,20,30,<null>,\n";
        let data = CsvZonesMetaProvider::load_from_reader(Cursor::new(csv)).expect("parse");
        let rgb = pack_rgb_u32(10, 20, 30);
        let meta = data.get(&rgb).expect("meta");
        assert!(meta.name.is_none());
        assert!(meta.active.is_none());
    }

    #[test]
    fn dolt_provider_head_optional() -> Result<()> {
        let Ok(repo_path) = std::env::var("DOLT_TEST_REPO_PATH") else {
            return Ok(());
        };
        if Command::new("dolt").arg("--version").output().is_err() {
            return Ok(());
        }
        let provider = DoltZonesMetaProvider::new(repo_path);
        let data = provider.load(None)?;
        assert!(!data.is_empty());
        Ok(())
    }
}
