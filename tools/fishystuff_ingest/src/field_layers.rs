use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use fishystuff_core::field::DiscreteFieldRows;
use fishystuff_core::field_metadata::{
    build_region_groups_hover_metadata, build_regions_hover_metadata, FieldHoverMetadataAsset,
    FieldHoverMetadataEntry,
};
use fishystuff_core::gamecommondata::{
    load_original_region_layer_context, zone_mask_detail_pane_ref, OriginalRegionLayerContext,
};

#[derive(Debug, Clone, Copy)]
pub struct FieldHoverMetadataBuildSummary {
    pub field_id_count: usize,
    pub entry_count: usize,
}

pub fn build_regions_field_hover_metadata(
    field_path: &Path,
    loc_path: &Path,
    regioninfo_bss_path: &Path,
    regiongroupinfo_bss_path: &Path,
    waypoint_xml_paths: &[PathBuf],
    out_path: &Path,
) -> Result<FieldHoverMetadataBuildSummary> {
    let field = load_field(field_path)?;
    let context = load_context(
        loc_path,
        regioninfo_bss_path,
        regiongroupinfo_bss_path,
        waypoint_xml_paths,
    )?;
    let field_id_count = field.unique_nonzero_ids().len();
    let metadata = build_regions_hover_metadata(&field, &context);
    write_field_hover_metadata(out_path, &metadata)?;
    Ok(FieldHoverMetadataBuildSummary {
        field_id_count,
        entry_count: metadata.entries.len(),
    })
}

pub fn build_region_groups_field_hover_metadata(
    field_path: &Path,
    regions_field_path: &Path,
    loc_path: &Path,
    regioninfo_bss_path: &Path,
    regiongroupinfo_bss_path: &Path,
    waypoint_xml_paths: &[PathBuf],
    out_path: &Path,
) -> Result<FieldHoverMetadataBuildSummary> {
    let field = load_field(field_path)?;
    let regions_field = load_field(regions_field_path)?;
    let context = load_context(
        loc_path,
        regioninfo_bss_path,
        regiongroupinfo_bss_path,
        waypoint_xml_paths,
    )?;
    let field_id_count = field.unique_nonzero_ids().len();
    let metadata = build_region_groups_hover_metadata(&field, &regions_field, &context);
    write_field_hover_metadata(out_path, &metadata)?;
    Ok(FieldHoverMetadataBuildSummary {
        field_id_count,
        entry_count: metadata.entries.len(),
    })
}

pub fn build_zone_mask_field_hover_metadata(
    field_path: &Path,
    _zones_csv_path: &Path,
    out_path: &Path,
) -> Result<FieldHoverMetadataBuildSummary> {
    let field = load_field(field_path)?;
    let field_id_count = field.unique_nonzero_ids().len();
    let mut entries = BTreeMap::new();

    for rgb_u32 in field.unique_nonzero_ids() {
        entries.insert(
            rgb_u32,
            FieldHoverMetadataEntry {
                targets: Vec::new(),
                detail_pane: Some(zone_mask_detail_pane_ref()),
                detail_sections: Vec::new(),
            },
        );
    }

    let metadata = FieldHoverMetadataAsset { entries };
    write_field_hover_metadata(out_path, &metadata)?;
    Ok(FieldHoverMetadataBuildSummary {
        field_id_count,
        entry_count: metadata.entries.len(),
    })
}

fn load_context(
    loc_path: &Path,
    regioninfo_bss_path: &Path,
    regiongroupinfo_bss_path: &Path,
    waypoint_xml_paths: &[PathBuf],
) -> Result<OriginalRegionLayerContext> {
    load_original_region_layer_context(
        regioninfo_bss_path,
        regiongroupinfo_bss_path,
        waypoint_xml_paths,
        loc_path,
    )
}

fn load_field(path: &Path) -> Result<DiscreteFieldRows> {
    let bytes = std::fs::read(path).with_context(|| format!("read field: {}", path.display()))?;
    DiscreteFieldRows::from_bytes(&bytes)
        .with_context(|| format!("decode field: {}", path.display()))
}

fn write_field_hover_metadata(out_path: &Path, metadata: &FieldHoverMetadataAsset) -> Result<()> {
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create output directory: {}", parent.display()))?;
    }
    let output_file = File::create(out_path)
        .with_context(|| format!("create output metadata: {}", out_path.display()))?;
    serde_json::to_writer_pretty(output_file, metadata)
        .with_context(|| format!("write output metadata: {}", out_path.display()))?;
    Ok(())
}
