pub fn fish_item_icon_path(item_id: i32) -> String {
    format!("/images/FishIcons/{item_id:08}.png")
}

pub fn fish_encyclopedia_icon_path(encyclopedia_id: i32) -> String {
    format!("/images/FishIcons/IC_0{encyclopedia_id}.png")
}

pub fn parse_fish_icon_asset_id(value: &str) -> Option<i32> {
    let raw = value.trim();
    if raw.is_empty() {
        return None;
    }

    let file = raw.split(['?', '#']).next().unwrap_or(raw);
    let file = file.rsplit('/').next().unwrap_or(file);
    let stem = file.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(file);
    let digits = stem
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }

    digits.parse::<i32>().ok()
}

#[cfg(test)]
mod tests {
    use super::{fish_encyclopedia_icon_path, fish_item_icon_path, parse_fish_icon_asset_id};

    #[test]
    fn item_icon_paths_are_zero_padded() {
        assert_eq!(fish_item_icon_path(8475), "/images/FishIcons/00008475.png");
        assert_eq!(
            fish_item_icon_path(821295),
            "/images/FishIcons/00821295.png"
        );
    }

    #[test]
    fn encyclopedia_icon_paths_are_zero_padded() {
        assert_eq!(
            fish_encyclopedia_icon_path(8501),
            "/images/FishIcons/IC_08501.png"
        );
        assert_eq!(
            fish_encyclopedia_icon_path(9434),
            "/images/FishIcons/IC_09434.png"
        );
        assert_eq!(
            fish_encyclopedia_icon_path(11558),
            "/images/FishIcons/IC_011558.png"
        );
    }

    #[test]
    fn parses_numeric_icon_ids_from_known_filenames() {
        assert_eq!(parse_fish_icon_asset_id("00008475.png"), Some(8475));
        assert_eq!(parse_fish_icon_asset_id("IC_09434.png"), Some(9434));
        assert_eq!(
            parse_fish_icon_asset_id("https://cdn.example.com/images/FishIcons/IC_08588.png"),
            Some(8588)
        );
        assert_eq!(parse_fish_icon_asset_id("New_Icon/thing.dds"), None);
    }
}
