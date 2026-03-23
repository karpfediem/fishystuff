use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use super::{
    BkdFile, Breakpoint, PabrInspect, PabrMap, RegionGroupMapping, RidFile, BKD_TRAILER_LEN,
    INDEX_SENTINEL, PABR_MAGIC, RID_FOOTER_LEN, RID_FOOTER_SIGNATURE,
};
use crate::pabr::util::{read_u16, read_u32};
use fishystuff_core::gamecommondata::load_region_group_mapping_from_regioninfo_bss;

impl PabrMap {
    pub fn paired_paths(input_path: &Path) -> Result<(PathBuf, PathBuf)> {
        let extension = input_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default();

        if extension.eq_ignore_ascii_case("rid") {
            Ok((input_path.to_path_buf(), input_path.with_extension("bkd")))
        } else if extension.eq_ignore_ascii_case("bkd") {
            Ok((input_path.with_extension("rid"), input_path.to_path_buf()))
        } else {
            bail!("PABR input must be a .rid or .bkd file; got .{}", extension);
        }
    }

    pub fn from_paths(rid_path: &Path, bkd_path: &Path) -> Result<Self> {
        let rid = RidFile::from_path(rid_path)?;
        let bkd = BkdFile::from_path(bkd_path)?;

        Ok(Self {
            rid_path: rid_path.to_path_buf(),
            bkd_path: bkd_path.to_path_buf(),
            rid,
            bkd,
        })
    }

    pub fn inspect(&self) -> Result<PabrInspect> {
        let (used_dictionary_entries, used_region_ids, transparent_breakpoints) =
            self.used_region_indices_and_ids()?;

        Ok(PabrInspect {
            dictionary_entries: self.rid.region_ids.len(),
            scanline_rows: self.bkd.rows.len(),
            native_width: self.rid.native_width,
            native_height: self.rid.native_height,
            wrapped_bands: self.band_count()?,
            used_dictionary_entries: used_dictionary_entries.len(),
            used_region_ids: used_region_ids.len(),
            transparent_breakpoints,
            max_source_x: self.bkd.max_source_x,
            rid_trailer_prefix_len: self.rid.trailer_prefix_len,
            bkd_trailer_words: self.bkd.trailer_words,
        })
    }

    pub fn used_region_ids(&self) -> Result<BTreeSet<u32>> {
        let (_, used_region_ids, _) = self.used_region_indices_and_ids()?;
        Ok(used_region_ids
            .into_iter()
            .map(u32::from)
            .collect::<BTreeSet<_>>())
    }

    pub(crate) fn region_id_for_dictionary_index(&self, dictionary_index: u16) -> Result<u16> {
        self.rid
            .region_ids
            .get(usize::from(dictionary_index))
            .copied()
            .with_context(|| {
                format!(
                    "dictionary index {} exceeds RID dictionary length {}",
                    dictionary_index,
                    self.rid.region_ids.len()
                )
            })
    }

    pub(crate) fn band_count(&self) -> Result<usize> {
        if self.rid.native_width == 0 {
            bail!("RID footer reported a zero native width");
        }

        Ok((u32::from(self.bkd.max_source_x) / self.rid.native_width) as usize + 1)
    }

    fn used_region_indices_and_ids(&self) -> Result<(BTreeSet<usize>, BTreeSet<u16>, usize)> {
        let mut used_dictionary_entries = BTreeSet::new();
        let mut used_region_ids = BTreeSet::new();
        let mut transparent_breakpoints = 0usize;

        for row in &self.bkd.rows {
            for breakpoint in row {
                if breakpoint.dictionary_index == INDEX_SENTINEL {
                    transparent_breakpoints += 1;
                    continue;
                }

                let dictionary_index = usize::from(breakpoint.dictionary_index);
                let region_id = self
                    .rid
                    .region_ids
                    .get(dictionary_index)
                    .copied()
                    .with_context(|| {
                        format!(
                            "dictionary index {} exceeds RID dictionary length {}",
                            dictionary_index,
                            self.rid.region_ids.len()
                        )
                    })?;

                used_dictionary_entries.insert(dictionary_index);
                used_region_ids.insert(region_id);
            }
        }

        Ok((
            used_dictionary_entries,
            used_region_ids,
            transparent_breakpoints,
        ))
    }
}

pub fn load_region_group_mapping(path: &Path) -> Result<RegionGroupMapping> {
    load_region_group_mapping_from_regioninfo_bss(path)
}

impl RidFile {
    pub fn from_path(path: &Path) -> Result<Self> {
        let bytes = fs::read(path)
            .with_context(|| format!("failed to read RID file {}", path.display()))?;
        Self::from_bytes(&bytes)
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 8 + RID_FOOTER_LEN {
            bail!("RID file is too small");
        }
        if &bytes[0..4] != PABR_MAGIC {
            bail!("RID file is missing PABR magic");
        }

        let dictionary_entries = read_u32(bytes, 4)? as usize;
        let dictionary_bytes = dictionary_entries
            .checked_mul(2)
            .context("RID dictionary length overflow")?;
        let dictionary_end = 8usize
            .checked_add(dictionary_bytes)
            .context("RID dictionary offset overflow")?;
        if dictionary_end + RID_FOOTER_LEN > bytes.len() {
            bail!("RID dictionary exceeds file size");
        }

        let footer = &bytes[bytes.len() - RID_FOOTER_LEN..];
        if footer[0..RID_FOOTER_SIGNATURE.len()] != RID_FOOTER_SIGNATURE {
            bail!("RID footer signature does not match the known PABR region-map footer");
        }

        let mut region_ids = Vec::with_capacity(dictionary_entries);
        for offset in (8..dictionary_end).step_by(2) {
            region_ids.push(read_u16(bytes, offset)?);
        }

        Ok(Self {
            region_ids,
            native_width: u32::from(read_u16(footer, 10)?),
            native_height: u32::from(read_u16(footer, 14)?),
            trailer_prefix_len: bytes.len() - dictionary_end - RID_FOOTER_LEN,
        })
    }
}

impl BkdFile {
    pub fn from_path(path: &Path) -> Result<Self> {
        let bytes = fs::read(path)
            .with_context(|| format!("failed to read BKD file {}", path.display()))?;
        Self::from_bytes(&bytes)
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 8 + BKD_TRAILER_LEN {
            bail!("BKD file is too small");
        }
        if &bytes[0..4] != PABR_MAGIC {
            bail!("BKD file is missing PABR magic");
        }

        let row_count = read_u32(bytes, 4)? as usize;
        let mut rows = Vec::with_capacity(row_count);
        let mut position = 8usize;
        let mut max_source_x = 0u16;

        for row_index in 0..row_count {
            if position + 4 > bytes.len() {
                bail!("BKD row {} header exceeds file size", row_index);
            }
            let pair_count = read_u32(bytes, position)? as usize;
            position += 4;

            let pair_bytes = pair_count
                .checked_mul(4)
                .context("BKD row length overflow")?;
            if position + pair_bytes > bytes.len() {
                bail!("BKD row {} payload exceeds file size", row_index);
            }

            let mut row = Vec::with_capacity(pair_count);
            let mut previous_x = 0u16;
            for pair_index in 0..pair_count {
                let pair_offset = position + pair_index * 4;
                let source_x = read_u16(bytes, pair_offset)?;
                let dictionary_index = read_u16(bytes, pair_offset + 2)?;
                if pair_index > 0 && source_x < previous_x {
                    bail!("BKD row {} is not sorted by x coordinate", row_index);
                }
                previous_x = source_x;
                max_source_x = max_source_x.max(source_x);
                row.push(Breakpoint {
                    source_x,
                    dictionary_index,
                });
            }

            position += pair_bytes;
            rows.push(row);
        }

        if position + BKD_TRAILER_LEN != bytes.len() {
            bail!(
                "BKD footer length mismatch: expected {} trailing bytes, found {}",
                BKD_TRAILER_LEN,
                bytes.len().saturating_sub(position)
            );
        }

        let trailer_words = [
            read_u32(bytes, position)?,
            read_u32(bytes, position + 4)?,
            read_u32(bytes, position + 8)?,
        ];
        if trailer_words[1] != position as u32 {
            bail!(
                "BKD footer offset {} does not match parsed row payload end {}",
                trailer_words[1],
                position
            );
        }

        Ok(Self {
            rows,
            trailer_words,
            max_source_x,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{BkdFile, Breakpoint, PabrMap, RidFile, RID_FOOTER_LEN, RID_FOOTER_SIGNATURE};

    fn make_rid_bytes(region_ids: &[u16], width: u16, height: u16, prefix: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"PABR");
        bytes.extend_from_slice(&(region_ids.len() as u32).to_le_bytes());
        for region_id in region_ids {
            bytes.extend_from_slice(&region_id.to_le_bytes());
        }
        bytes.extend_from_slice(prefix);

        let mut footer = [0u8; RID_FOOTER_LEN];
        footer[0..RID_FOOTER_SIGNATURE.len()].copy_from_slice(&RID_FOOTER_SIGNATURE);
        footer[10..12].copy_from_slice(&width.to_le_bytes());
        footer[14..16].copy_from_slice(&height.to_le_bytes());
        bytes.extend_from_slice(&footer);
        bytes
    }

    fn make_bkd_bytes(rows: &[Vec<Breakpoint>]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"PABR");
        bytes.extend_from_slice(&(rows.len() as u32).to_le_bytes());

        for row in rows {
            bytes.extend_from_slice(&(row.len() as u32).to_le_bytes());
            for breakpoint in row {
                bytes.extend_from_slice(&breakpoint.source_x.to_le_bytes());
                bytes.extend_from_slice(&breakpoint.dictionary_index.to_le_bytes());
            }
        }

        let payload_end = bytes.len() as u32;
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&payload_end.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes
    }

    #[test]
    fn rid_footer_dimensions_are_parsed_from_fixed_trailer() {
        let rid = RidFile::from_bytes(&make_rid_bytes(&[4, 17], 11560, 10540, &[1, 2, 3, 4]))
            .expect("RID should parse");

        assert_eq!(rid.region_ids, vec![4, 17]);
        assert_eq!(rid.native_width, 11560);
        assert_eq!(rid.native_height, 10540);
        assert_eq!(rid.trailer_prefix_len, 4);
    }

    #[test]
    fn bkd_rows_parse_sorted_breakpoints_and_footer_offset() {
        let rows = vec![
            vec![
                Breakpoint {
                    source_x: 0,
                    dictionary_index: 0,
                },
                Breakpoint {
                    source_x: 32768,
                    dictionary_index: 1,
                },
            ],
            vec![Breakpoint {
                source_x: 0,
                dictionary_index: 1,
            }],
        ];
        let bkd = BkdFile::from_bytes(&make_bkd_bytes(&rows)).expect("BKD should parse");

        assert_eq!(bkd.rows, rows);
        assert_eq!(bkd.max_source_x, 32768);
    }

    #[test]
    fn inspect_reports_wrapped_bands_and_used_ids() {
        let map = PabrMap {
            rid_path: "test.rid".into(),
            bkd_path: "test.bkd".into(),
            rid: RidFile {
                region_ids: vec![4, 17],
                native_width: 11560,
                native_height: 10540,
                trailer_prefix_len: 0,
            },
            bkd: BkdFile {
                rows: vec![
                    vec![
                        Breakpoint {
                            source_x: 0,
                            dictionary_index: 0,
                        },
                        Breakpoint {
                            source_x: 32768,
                            dictionary_index: 1,
                        },
                    ],
                    vec![Breakpoint {
                        source_x: 0,
                        dictionary_index: u16::MAX,
                    }],
                ],
                trailer_words: [0, 0, 0],
                max_source_x: 32768,
            },
        };

        let inspect = map.inspect().expect("inspect should succeed");
        assert_eq!(inspect.wrapped_bands, 3);
        assert_eq!(inspect.used_region_ids, 2);
        assert_eq!(inspect.transparent_breakpoints, 1);
    }
}
