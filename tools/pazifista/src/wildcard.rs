pub fn wild_match(pattern: &str, text: &str) -> bool {
    if pattern.is_empty() {
        return true;
    }
    if text.is_empty() {
        return pattern.is_empty();
    }

    let pattern = pattern.as_bytes();
    let text = text.as_bytes();

    let mut pat_idx = 0usize;
    let mut text_idx = 0usize;
    let mut last_star = None;
    let mut last_star_match = 0usize;

    while text_idx < text.len() {
        if pat_idx < pattern.len()
            && ((pattern[pat_idx] == b'?' && text[text_idx] != b'.')
                || pattern[pat_idx].to_ascii_lowercase() == text[text_idx].to_ascii_lowercase())
        {
            pat_idx += 1;
            text_idx += 1;
        } else if pat_idx < pattern.len() && pattern[pat_idx] == b'*' {
            last_star = Some(pat_idx);
            pat_idx += 1;
            last_star_match = text_idx;
        } else if let Some(star_idx) = last_star {
            pat_idx = star_idx + 1;
            last_star_match += 1;
            text_idx = last_star_match;
        } else {
            return false;
        }
    }

    while pat_idx < pattern.len() && pattern[pat_idx] == b'*' {
        pat_idx += 1;
    }

    pat_idx == pattern.len()
}

pub fn wild_match_any(patterns: &[String], text: &str) -> bool {
    patterns.is_empty() || patterns.iter().any(|pattern| wild_match(pattern, text))
}

#[cfg(test)]
mod tests {
    use super::{wild_match, wild_match_any};

    #[test]
    fn empty_pattern_matches_everything() {
        assert!(wild_match("", "abc.txt"));
    }

    #[test]
    fn star_and_question_follow_original_rules() {
        assert!(wild_match("*.txt", "folder/file.txt"));
        assert!(wild_match(
            "*languagedata_??.txt",
            "res/text/languagedata_en.txt"
        ));
        assert!(!wild_match("file?.txt", "file..txt"));
    }

    #[test]
    fn match_is_case_insensitive() {
        assert!(wild_match("PAD*.PAZ", "pad00001.paz"));
    }

    #[test]
    fn any_match_supports_multiple_filters() {
        let patterns = vec!["*.rid".to_string(), "*.bkd".to_string()];
        assert!(wild_match_any(
            &patterns,
            "ui_texture/minimap/area/map.bmp.rid"
        ));
        assert!(wild_match_any(
            &patterns,
            "ui_texture/minimap/area/map.bmp.bkd"
        ));
        assert!(!wild_match_any(
            &patterns,
            "ui_texture/minimap/area/map.bmp.png"
        ));
    }

    #[test]
    fn empty_filter_list_matches_everything() {
        assert!(wild_match_any(&[], "anything/goes.dat"));
    }
}
