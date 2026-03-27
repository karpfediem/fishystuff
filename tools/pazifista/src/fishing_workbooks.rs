use anyhow::{bail, Result};

use crate::archive::FileEntry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FishingWorkbookTarget {
    pub file_name: &'static str,
    pub required: bool,
}

#[derive(Clone, Debug)]
pub struct ResolvedFishingWorkbook {
    pub target: FishingWorkbookTarget,
    pub entry: FileEntry,
}

#[derive(Clone, Debug, Default)]
pub struct FishingWorkbookPlan {
    pub extracted: Vec<ResolvedFishingWorkbook>,
    pub missing_optional: Vec<&'static str>,
}

const REQUIRED_TARGETS: [FishingWorkbookTarget; 4] = [
    FishingWorkbookTarget {
        file_name: "Fishing_Table.xlsx",
        required: true,
    },
    FishingWorkbookTarget {
        file_name: "Item_Table.xlsx",
        required: true,
    },
    FishingWorkbookTarget {
        file_name: "ItemMainGroup_Table.xlsx",
        required: true,
    },
    FishingWorkbookTarget {
        file_name: "ItemSubGroup_Table.xlsx",
        required: true,
    },
];

const OPTIONAL_TARGETS: [FishingWorkbookTarget; 8] = [
    FishingWorkbookTarget {
        file_name: "FloatFishing_Table.xlsx",
        required: false,
    },
    FishingWorkbookTarget {
        file_name: "FloatFishingPoint_Table.xlsx",
        required: false,
    },
    FishingWorkbookTarget {
        file_name: "Water_Table.xlsx",
        required: false,
    },
    FishingWorkbookTarget {
        file_name: "FishingStatData.xlsx",
        required: false,
    },
    FishingWorkbookTarget {
        file_name: "DiscardFish.xlsx",
        required: false,
    },
    FishingWorkbookTarget {
        file_name: "LightStoneSetOption.xlsx",
        required: false,
    },
    FishingWorkbookTarget {
        file_name: "Pet_Table.xlsx",
        required: false,
    },
    FishingWorkbookTarget {
        file_name: "Pet_SetStats_Table.xlsx",
        required: false,
    },
];

pub fn fishing_workbook_targets(include_optional: bool) -> Vec<FishingWorkbookTarget> {
    let mut targets = REQUIRED_TARGETS.to_vec();
    if include_optional {
        targets.extend(OPTIONAL_TARGETS);
    }
    targets
}

pub fn build_fishing_workbook_plan<F>(
    include_optional: bool,
    mut resolve_matches: F,
) -> Result<FishingWorkbookPlan>
where
    F: FnMut(&FishingWorkbookTarget) -> Vec<FileEntry>,
{
    let mut plan = FishingWorkbookPlan::default();
    for target in fishing_workbook_targets(include_optional) {
        let matches = resolve_matches(&target);
        match select_workbook_entry(target, matches)? {
            Some(entry) => plan
                .extracted
                .push(ResolvedFishingWorkbook { target, entry }),
            None if target.required => {
                bail!(
                    "required workbook {} not found in archive",
                    target.file_name
                )
            }
            None => plan.missing_optional.push(target.file_name),
        }
    }
    Ok(plan)
}

fn select_workbook_entry(
    target: FishingWorkbookTarget,
    matches: Vec<FileEntry>,
) -> Result<Option<FileEntry>> {
    if matches.is_empty() {
        return Ok(None);
    }
    if matches.len() == 1 {
        return Ok(matches.into_iter().next());
    }

    let mut excel_matches = matches
        .iter()
        .filter(|entry| is_excel_workbook_path(&entry.file_path, target.file_name))
        .cloned()
        .collect::<Vec<_>>();

    if excel_matches.len() == 1 {
        return Ok(excel_matches.pop());
    }

    let paths = matches
        .iter()
        .map(|entry| entry.file_path.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    bail!(
        "workbook {} matched multiple archive entries: {}",
        target.file_name,
        paths
    )
}

fn is_excel_workbook_path(path: &str, file_name: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let needle = format!("/excel/{}", file_name.to_ascii_lowercase());
    lower.starts_with(&format!("excel/{}", file_name.to_ascii_lowercase()))
        || lower.contains(&needle)
}

#[cfg(test)]
mod tests {
    use super::{build_fishing_workbook_plan, FishingWorkbookTarget};
    use crate::archive::FileEntry;

    fn entry(path: &str, paz_num: u32) -> FileEntry {
        let file_name = path.rsplit('/').next().unwrap_or(path).to_string();
        FileEntry {
            paz_num,
            offset: 0,
            compressed_size: 0,
            original_size: 0,
            file_name,
            file_path: path.to_string(),
        }
    }

    #[test]
    fn prefers_excel_path_when_filename_matches_multiple_entries() {
        let plan = build_fishing_workbook_plan(false, |target| {
            if target.file_name == "Fishing_Table.xlsx" {
                vec![
                    entry("backup/Fishing_Table.xlsx", 1),
                    entry("gamecommondata/excel/Fishing_Table.xlsx", 2),
                ]
            } else if target.required {
                vec![entry(
                    &format!("gamecommondata/excel/{}", target.file_name),
                    2,
                )]
            } else {
                Vec::new()
            }
        })
        .expect("plan should resolve");

        let fishing = plan
            .extracted
            .iter()
            .find(|item| item.target.file_name == "Fishing_Table.xlsx")
            .expect("fishing workbook should be present");
        assert_eq!(fishing.entry.paz_num, 2);
        assert_eq!(
            fishing.entry.file_path,
            "gamecommondata/excel/Fishing_Table.xlsx"
        );
    }

    #[test]
    fn errors_when_required_workbook_is_missing() {
        let err = build_fishing_workbook_plan(false, |_target: &FishingWorkbookTarget| Vec::new())
            .expect_err("missing required workbook should fail");
        assert!(err
            .to_string()
            .contains("required workbook Fishing_Table.xlsx not found"));
    }

    #[test]
    fn tracks_missing_optional_workbooks_without_failing() {
        let plan = build_fishing_workbook_plan(true, |target| {
            if target.required {
                vec![entry(
                    &format!("gamecommondata/excel/{}", target.file_name),
                    9,
                )]
            } else {
                Vec::new()
            }
        })
        .expect("optional misses should not fail");

        assert!(plan.missing_optional.contains(&"FloatFishing_Table.xlsx"));
        assert!(plan.missing_optional.contains(&"LightStoneSetOption.xlsx"));
    }
}
