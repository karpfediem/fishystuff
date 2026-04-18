pub fn fish_item_icon_path(item_id: i32) -> String {
    format!("/images/items/{item_id:08}.webp")
}

pub fn fish_encyclopedia_icon_path(encyclopedia_id: i32) -> String {
    format!("/images/items/IC_0{encyclopedia_id}.webp")
}

pub fn fish_icon_asset_stem(value: &str) -> Option<String> {
    let raw = value.trim();
    if raw.is_empty() {
        return None;
    }

    let file = raw.split(['?', '#']).next().unwrap_or(raw);
    let file = file.rsplit('/').next().unwrap_or(file);
    let stem = file.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(file);
    let stem = stem.trim();
    (!stem.is_empty()).then(|| stem.to_string())
}

pub fn fish_icon_path_from_asset_stem(stem: &str) -> Option<String> {
    let stem = stem.trim();
    (!stem.is_empty()).then(|| format!("/images/items/{stem}.webp"))
}

pub fn fish_icon_path_from_asset_file(value: &str) -> Option<String> {
    fish_icon_asset_stem(value).and_then(|stem| fish_icon_path_from_asset_stem(&stem))
}

pub fn parse_fish_icon_asset_id(value: &str) -> Option<i32> {
    let stem = fish_icon_asset_stem(value)?;
    let stem = stem
        .rsplit_once('_')
        .and_then(|(base, suffix)| {
            (suffix.chars().all(|ch| ch.is_ascii_digit())
                && base.chars().all(|ch| ch.is_ascii_digit()))
            .then_some(base)
        })
        .unwrap_or(stem.as_str());

    if let Some(rest) = stem
        .strip_prefix("IC_")
        .or_else(|| stem.strip_prefix("ic_"))
    {
        return rest
            .chars()
            .all(|ch| ch.is_ascii_digit())
            .then(|| rest.parse::<i32>().ok())?;
    }

    stem.chars()
        .all(|ch| ch.is_ascii_digit())
        .then(|| stem.parse::<i32>().ok())?
}

#[cfg(test)]
mod tests {
    use super::{
        fish_encyclopedia_icon_path, fish_icon_path_from_asset_file, fish_item_icon_path,
        parse_fish_icon_asset_id,
    };

    #[test]
    fn item_icon_paths_are_zero_padded() {
        assert_eq!(fish_item_icon_path(8475), "/images/items/00008475.webp");
        assert_eq!(fish_item_icon_path(821295), "/images/items/00821295.webp");
    }

    #[test]
    fn encyclopedia_icon_paths_are_zero_padded() {
        assert_eq!(
            fish_encyclopedia_icon_path(8501),
            "/images/items/IC_08501.webp"
        );
        assert_eq!(
            fish_encyclopedia_icon_path(9434),
            "/images/items/IC_09434.webp"
        );
        assert_eq!(
            fish_encyclopedia_icon_path(11558),
            "/images/items/IC_011558.webp"
        );
    }

    #[test]
    fn builds_source_backed_icon_paths_from_asset_filenames() {
        assert_eq!(
            fish_icon_path_from_asset_file("00008475.png").as_deref(),
            Some("/images/items/00008475.webp")
        );
        assert_eq!(
            fish_icon_path_from_asset_file(
                "New_Icon/03_ETC/06_Housing/InHouse_DPFO_birthdayCake_01.dds"
            )
            .as_deref(),
            Some("/images/items/InHouse_DPFO_birthdayCake_01.webp")
        );
        assert_eq!(
            fish_icon_path_from_asset_file(
                "ui_texture/icon/new_icon/04_pc_skill/03_buff/event_item_00790580.dds"
            )
            .as_deref(),
            Some("/images/items/event_item_00790580.webp")
        );
        assert_eq!(fish_icon_path_from_asset_file(""), None);
    }

    #[test]
    fn parses_numeric_icon_ids_from_known_filenames() {
        assert_eq!(parse_fish_icon_asset_id("00008475.png"), Some(8475));
        assert_eq!(parse_fish_icon_asset_id("IC_09434.png"), Some(9434));
        assert_eq!(
            parse_fish_icon_asset_id("https://cdn.example.com/images/FishIcons/IC_08588.png"),
            Some(8588)
        );
        assert_eq!(
            parse_fish_icon_asset_id("New_Icon/03_ETC/11_Enchant_Material/00015229_2.dds"),
            Some(15229)
        );
        assert_eq!(
            parse_fish_icon_asset_id("New_Icon/03_ETC/11_Enchant_Material/00015647_11.dds"),
            Some(15647)
        );
        assert_eq!(
            parse_fish_icon_asset_id("New_Icon/03_ETC/000768425.dds"),
            Some(768425)
        );
        assert_eq!(parse_fish_icon_asset_id("New_Icon/thing.dds"), None);
        assert_eq!(
            parse_fish_icon_asset_id("New_Icon/03_ETC/06_Housing/InHouse_DPFO_birthdayCake_01.dds"),
            None
        );
        assert_eq!(
            parse_fish_icon_asset_id(
                "New_Icon/03_ETC/06_Housing/InHouse_DPFO_birthdayCake_3333.dds"
            ),
            None
        );
        assert_eq!(
            parse_fish_icon_asset_id(
                "ui_texture/icon/new_icon/04_pc_skill/03_buff/event_item_00790580.dds"
            ),
            None
        );
    }
}
