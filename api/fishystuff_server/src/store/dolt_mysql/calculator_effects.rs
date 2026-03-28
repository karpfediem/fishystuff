use std::collections::HashSet;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct CalculatorItemEffectValues {
    pub(super) afr: Option<f32>,
    pub(super) bonus_rare: Option<f32>,
    pub(super) bonus_big: Option<f32>,
    pub(super) item_drr: Option<f32>,
    pub(super) exp_fish: Option<f32>,
    pub(super) exp_life: Option<f32>,
}

fn add_effect_value(slot: &mut Option<f32>, value: Option<f32>) {
    let Some(value) = value else {
        return;
    };
    *slot = Some(slot.unwrap_or(0.0) + value);
}

fn strip_game_markup(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut angle_depth = 0usize;
    let mut brace_depth = 0usize;

    for ch in text.chars() {
        match ch {
            '<' if brace_depth == 0 => angle_depth += 1,
            '>' if angle_depth > 0 => angle_depth -= 1,
            '{' if angle_depth == 0 => brace_depth += 1,
            '}' if brace_depth > 0 => brace_depth -= 1,
            _ if angle_depth == 0 && brace_depth == 0 => out.push(ch),
            _ => {}
        }
    }

    out
}

fn normalize_effect_text(text: &str) -> String {
    strip_game_markup(text)
        .replace("\\r\\n", "\n")
        .replace("\\n", "\n")
        .replace("\r\n", "\n")
        .replace('\r', "\n")
}

pub(super) fn normalized_effect_lines(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::<String>::new();
    for line in normalize_effect_text(text).lines() {
        let normalized = line.trim().to_string();
        if normalized.is_empty() || !seen.insert(normalized.clone()) {
            continue;
        }
        out.push(normalized);
    }
    out
}

pub(super) fn extract_first_number(text: &str) -> Option<f32> {
    let sanitized = strip_game_markup(text);
    let chars: Vec<char> = sanitized.chars().collect();
    let mut idx = 0;
    while idx < chars.len() {
        if chars[idx] == '+' || chars[idx] == '-' || chars[idx].is_ascii_digit() {
            let start = idx;
            idx += 1;
            let mut seen_digit = chars[start].is_ascii_digit();
            while idx < chars.len() && (chars[idx].is_ascii_digit() || chars[idx] == '.') {
                seen_digit |= chars[idx].is_ascii_digit();
                idx += 1;
            }
            if seen_digit {
                let candidate = chars[start..idx].iter().collect::<String>();
                if let Ok(value) = candidate.parse::<f32>() {
                    return Some(value);
                }
            }
        } else {
            idx += 1;
        }
    }
    None
}

fn extract_first_number_after(text: &str, needle: &str) -> Option<f32> {
    let index = text.find(needle)?;
    extract_first_number(&text[index + needle.len()..])
}

fn extract_percent_ratio_after_any(text: &str, needles: &[&str]) -> Option<f32> {
    needles
        .iter()
        .find_map(|needle| extract_first_number_after(text, needle))
        .map(|value| value.abs() / 100.0)
}

fn parse_calculator_effect_line(values: &mut CalculatorItemEffectValues, line: &str) {
    let sanitized = strip_game_markup(line);
    let line = sanitized.trim();
    if line.is_empty() {
        return;
    }
    if line.contains("자동 낚시") {
        add_effect_value(
            &mut values.afr,
            extract_percent_ratio_after_any(line, &["자동 낚시 시간 감소", "자동 낚시 시간"]),
        );
    }
    if line.contains("희귀 어종") || line.contains("희귀 확률 증가") {
        add_effect_value(
            &mut values.bonus_rare,
            extract_percent_ratio_after_any(
                line,
                &["희귀 어종을 낚을 확률 증가", "희귀 어종", "희귀 확률 증가"],
            ),
        );
    }
    if line.contains("대형 어종") || line.contains("대어 확률 증가") || line.contains("고급 어종")
    {
        add_effect_value(
            &mut values.bonus_big,
            extract_percent_ratio_after_any(
                line,
                &[
                    "대형 어종을 낚을 확률 증가",
                    "대형 어종",
                    "대어 확률 증가",
                    "고급 어종을 낚을 확률 증가",
                    "고급 어종",
                ],
            ),
        );
    }
    if line.contains("내구도 소모 감소 저항") || line.contains("장비 내구도 감소 저항")
    {
        add_effect_value(
            &mut values.item_drr,
            extract_percent_ratio_after_any(
                line,
                &["내구도 소모 감소 저항", "장비 내구도 감소 저항"],
            ),
        );
    }
    if line.contains("낚시 경험치") {
        add_effect_value(
            &mut values.exp_fish,
            extract_percent_ratio_after_any(line, &["낚시 경험치 획득량", "낚시 경험치"]),
        );
    }
    if line.contains("생활 경험치") {
        add_effect_value(
            &mut values.exp_life,
            extract_percent_ratio_after_any(line, &["생활 경험치 획득량", "생활 경험치"]),
        );
    }
}

pub(super) fn parse_calculator_effect_text(values: &mut CalculatorItemEffectValues, text: &str) {
    let normalized = normalize_effect_text(text);
    for line in normalized.lines() {
        parse_calculator_effect_line(values, line);
    }
}

pub(super) fn parse_unique_calculator_effect_text(
    values: &mut CalculatorItemEffectValues,
    text: &str,
) {
    for normalized in normalized_effect_lines(text) {
        parse_calculator_effect_line(values, &normalized);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        extract_first_number, normalized_effect_lines, parse_calculator_effect_text,
        CalculatorItemEffectValues,
    };

    #[test]
    fn extract_first_number_handles_signed_percent_lines() {
        assert_eq!(extract_first_number("자동 낚시 시간 -15%"), Some(-15.0));
        assert_eq!(extract_first_number("낚시 경험치 획득량 +10%"), Some(10.0));
        assert_eq!(extract_first_number("생활 숙련도 +20"), Some(20.0));
        assert_eq!(extract_first_number("효과 없음"), None);
    }

    #[test]
    fn calculator_effect_text_parses_balacs_style_lines() {
        let mut values = CalculatorItemEffectValues::default();
        parse_calculator_effect_text(
            &mut values,
            "자동 낚시 시간 감소 7%\n낚시 경험치 획득량 +10%",
        );

        assert_eq!(
            values,
            CalculatorItemEffectValues {
                afr: Some(0.07),
                exp_fish: Some(0.10),
                ..CalculatorItemEffectValues::default()
            }
        );
    }

    #[test]
    fn calculator_effect_text_parses_event_food_and_housekeeper_lines() {
        let mut values = CalculatorItemEffectValues::default();
        parse_calculator_effect_text(&mut values, "생활 숙련도 +50\n생활 경험치 획득량 +20%");

        assert_eq!(
            values,
            CalculatorItemEffectValues {
                exp_life: Some(0.20),
                ..CalculatorItemEffectValues::default()
            }
        );

        let mut event_food = CalculatorItemEffectValues::default();
        parse_calculator_effect_text(
            &mut event_food,
            "자동 낚시 시간 -10%\n생활 경험치 획득량 +50%\n생활 숙련도 +100",
        );

        assert_eq!(
            event_food,
            CalculatorItemEffectValues {
                afr: Some(0.10),
                exp_life: Some(0.50),
                ..CalculatorItemEffectValues::default()
            }
        );
    }

    #[test]
    fn calculator_effect_text_parses_equipment_durability_phrase() {
        let mut values = CalculatorItemEffectValues::default();
        parse_calculator_effect_text(&mut values, "장비 내구도 감소 저항 +30%");

        assert_eq!(
            values,
            CalculatorItemEffectValues {
                item_drr: Some(0.30),
                ..CalculatorItemEffectValues::default()
            }
        );
    }

    #[test]
    fn calculator_effect_text_ignores_game_markup_when_parsing_numbers() {
        let mut values = CalculatorItemEffectValues::default();
        parse_calculator_effect_text(
            &mut values,
            "<PAColor0xffe9bd23>자동 낚시 시간 감소 +10%<PAOldColor>\n<PAColor0xffe9bd23>낚시 경험치 획득량 +25%<PAOldColor>",
        );

        assert_eq!(
            values,
            CalculatorItemEffectValues {
                afr: Some(0.10),
                exp_fish: Some(0.25),
                ..CalculatorItemEffectValues::default()
            }
        );
    }

    #[test]
    fn calculator_effect_text_binds_numbers_to_matching_effect_phrase() {
        let mut values = CalculatorItemEffectValues::default();
        parse_calculator_effect_text(
            &mut values,
            "[이벤트] 겉바속촉 붕어빵\n모든 생활 숙련도 +100\n생활 경험치 획득량 +50%\n자동 낚시 시간 감소 +10%\n희귀 어종을 낚을 확률 증가 +5%\n최대 소지 무게 +100LT",
        );

        assert_eq!(
            values,
            CalculatorItemEffectValues {
                afr: Some(0.10),
                bonus_rare: Some(0.05),
                exp_life: Some(0.50),
                ..CalculatorItemEffectValues::default()
            }
        );
    }

    #[test]
    fn calculator_effect_text_parses_atomic_rod_skill_buff_names() {
        let mut values = CalculatorItemEffectValues::default();
        parse_calculator_effect_text(
            &mut values,
            "자동 낚시 시간 감소(80%)\n희귀 확률 증가(5%)\n대어 확률 증가(11%)",
        );

        assert_eq!(
            values,
            CalculatorItemEffectValues {
                afr: Some(0.80),
                bonus_rare: Some(0.05),
                bonus_big: Some(0.11),
                ..CalculatorItemEffectValues::default()
            }
        );
    }

    #[test]
    fn normalized_effect_lines_strip_markup_and_deduplicate_lines() {
        let lines = normalized_effect_lines(
            "<PAColor0xffe9bd23>엔트의 눈물<PAOldColor>\n\n 생활 경험치 획득량 <PAColor0xffe9bd23>+30%<PAOldColor>\n 생활 경험치 획득량 +30%\n 자동 낚시 시간 감소 +10%",
        );

        assert_eq!(
            lines,
            vec![
                "엔트의 눈물".to_string(),
                "생활 경험치 획득량 +30%".to_string(),
                "자동 낚시 시간 감소 +10%".to_string(),
            ]
        );
    }

    #[test]
    fn normalized_effect_lines_split_escaped_newlines() {
        let lines = normalized_effect_lines(
            "엔트의 눈물\\n\\n 생활 경험치 획득량 +30%\\n 채집/낚시 속도 잠재력 +2단계\n생활 경험치 획득량 +30%",
        );

        assert_eq!(
            lines,
            vec![
                "엔트의 눈물".to_string(),
                "생활 경험치 획득량 +30%".to_string(),
                "채집/낚시 속도 잠재력 +2단계".to_string(),
            ]
        );
    }
}
