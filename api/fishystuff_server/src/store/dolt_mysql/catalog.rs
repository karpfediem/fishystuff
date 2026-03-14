use std::collections::BTreeMap;

use super::util::normalize_optional_string;
use super::FishCatalogRow;

pub(super) fn fish_icon_url_from_db(value: Option<String>) -> Option<String> {
    let icon_file = normalize_optional_string(value)?;
    if !is_web_icon_path(&icon_file) {
        return None;
    }
    Some(format!("/images/FishIcons/{icon_file}"))
}

pub(super) fn preferred_item_icon_url(
    fish_table_icon: Option<String>,
    item_table_icon: Option<String>,
    encyclopedia_icon: Option<String>,
) -> Option<String> {
    fish_icon_url_from_db(fish_table_icon)
        .or_else(|| fish_icon_url_from_db(item_table_icon))
        .or_else(|| fish_icon_url_from_db(encyclopedia_icon))
}

pub(super) fn is_web_icon_path(path: &str) -> bool {
    let Some((_, ext)) = path.rsplit_once('.') else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "avif" | "svg"
    )
}

pub(super) fn merge_fish_catalog_row(
    rows: &mut BTreeMap<i32, FishCatalogRow>,
    candidate: FishCatalogRow,
) {
    use std::collections::btree_map::Entry;

    match rows.entry(candidate.fish_id) {
        Entry::Vacant(entry) => {
            entry.insert(candidate);
        }
        Entry::Occupied(mut entry) => {
            let existing = entry.get_mut();
            if existing.encyclopedia_key.is_none() {
                existing.encyclopedia_key = candidate.encyclopedia_key;
            }
            if candidate.name.to_lowercase() < existing.name.to_lowercase() {
                existing.name = candidate.name.clone();
            }

            let existing_rank = existing.grade_rank.unwrap_or_default();
            let candidate_rank = candidate.grade_rank.unwrap_or_default();
            if candidate_rank > existing_rank
                || (candidate_rank == existing_rank
                    && candidate.grade.as_deref().unwrap_or("")
                        < existing.grade.as_deref().unwrap_or(""))
            {
                existing.grade = candidate.grade.clone();
                existing.grade_rank = candidate.grade_rank;
            }

            existing.is_prize = match (existing.is_prize, candidate.is_prize) {
                (Some(left), Some(right)) => Some(left || right),
                (Some(left), None) => Some(left),
                (None, Some(right)) => Some(right),
                (None, None) => None,
            };

            existing.icon_url = match (existing.icon_url.take(), candidate.icon_url) {
                (Some(left), Some(right)) => Some(left.min(right)),
                (Some(left), None) => Some(left),
                (None, Some(right)) => Some(right),
                (None, None) => None,
            };

            existing.is_dried = existing.is_dried || candidate.is_dried;

            existing.catch_methods = merge_catch_methods(
                std::mem::take(&mut existing.catch_methods),
                candidate.catch_methods,
            );

            existing.vendor_price = match (existing.vendor_price, candidate.vendor_price) {
                (Some(left), Some(right)) => Some(left.max(right)),
                (Some(left), None) => Some(left),
                (None, Some(right)) => Some(right),
                (None, None) => None,
            };
        }
    }
}

pub(super) fn fish_grade_from_db(
    value: Option<String>,
) -> (Option<String>, Option<u8>, Option<bool>) {
    let normalized = normalize_optional_string(value);
    match normalized.as_deref() {
        Some("4") => (Some("Prize".to_string()), Some(4), Some(true)),
        Some("3") => (Some("Rare".to_string()), Some(3), Some(false)),
        Some("2") => (Some("HighQuality".to_string()), Some(2), Some(false)),
        Some("1") => (Some("General".to_string()), Some(1), Some(false)),
        Some("0") => (Some("Trash".to_string()), Some(0), Some(false)),
        _ => (None, None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::{fish_icon_url_from_db, preferred_item_icon_url};

    #[test]
    fn preferred_item_icon_url_uses_item_icon_before_encyclopedia_icon() {
        assert_eq!(
            preferred_item_icon_url(
                Some("00008477.png".to_string()),
                Some("00009999.png".to_string()),
                Some("IC_09507.png".to_string())
            )
            .as_deref(),
            Some("/images/FishIcons/00008477.png")
        );
        assert_eq!(
            preferred_item_icon_url(
                None,
                Some("00009999.png".to_string()),
                Some("IC_09507.png".to_string())
            )
            .as_deref(),
            Some("/images/FishIcons/00009999.png")
        );
        assert_eq!(
            preferred_item_icon_url(None, None, Some("IC_09507.png".to_string())).as_deref(),
            Some("/images/FishIcons/IC_09507.png")
        );
        assert_eq!(preferred_item_icon_url(None, None, None), None);
    }

    #[test]
    fn fish_icon_url_from_db_filters_non_web_assets() {
        assert_eq!(
            fish_icon_url_from_db(Some("00008475.png".to_string())).as_deref(),
            Some("/images/FishIcons/00008475.png")
        );
        assert_eq!(
            fish_icon_url_from_db(Some(
                "New_Icon/03_ETC/07_ProductMaterial/00008518.dds".to_string()
            )),
            None
        );
    }
}

pub(super) fn fish_catch_methods_from_description(value: Option<String>) -> Vec<String> {
    let Some(description) = normalize_optional_string(value) else {
        return vec!["rod".to_string()];
    };

    let mut methods = Vec::new();
    for raw_line in description.lines() {
        let line = raw_line.trim();
        if !line.starts_with("- ") {
            continue;
        }
        if line.contains("일반 어종")
            || line.contains("희귀 어종")
            || line.contains("대형 어종")
            || line.contains("보물 어종")
            || line.contains("바다 어종")
        {
            methods.push("rod".to_string());
        }
        if line.contains("작살 어종") {
            methods.push("harpoon".to_string());
        }
    }

    if methods.is_empty() {
        if description.contains("작살 어종") {
            methods.push("harpoon".to_string());
        } else {
            methods.push("rod".to_string());
        }
    }

    normalize_catch_methods(methods)
}

pub(super) fn fish_is_dried(name: Option<&str>, item_name: Option<&str>) -> bool {
    let normalized_name = name
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if normalized_name.starts_with("dried ") {
        return true;
    }

    item_name
        .map(str::trim)
        .is_some_and(|value| value.starts_with("말린 "))
}

fn normalize_catch_methods(methods: Vec<String>) -> Vec<String> {
    let mut has_rod = false;
    let mut has_harpoon = false;
    for method in methods {
        match method.trim().to_ascii_lowercase().as_str() {
            "rod" => has_rod = true,
            "harpoon" => has_harpoon = true,
            _ => {}
        }
    }

    let mut normalized = Vec::with_capacity(2);
    if has_rod {
        normalized.push("rod".to_string());
    }
    if has_harpoon {
        normalized.push("harpoon".to_string());
    }
    normalized
}

fn merge_catch_methods(left: Vec<String>, right: Vec<String>) -> Vec<String> {
    let mut merged = left;
    merged.extend(right);
    normalize_catch_methods(merged)
}

pub(super) fn parse_positive_i64(value: Option<String>) -> Option<i64> {
    let trimmed = normalize_optional_string(value)?;
    let parsed = trimmed.parse::<i64>().ok()?;
    (parsed > 0).then_some(parsed)
}
